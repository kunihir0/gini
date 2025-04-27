use tokio::fs; // Use tokio::fs
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet}; // Added HashSet for cycle detection later
use std::future::Future;
use std::pin::Pin;
use semver::{Version, VersionReq}; // Added semver imports
use thiserror::Error; // Added for custom error
// Remove the problematic import: use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt; // For stream methods like next()

use crate::kernel::error::{Error as KernelError, Result}; // Renamed Error to KernelError
use crate::plugin_system::traits::Plugin;
use crate::plugin_system::manifest::PluginManifest;
use crate::plugin_system::registry::PluginRegistry;
use crate::plugin_system::version::{ApiVersion, VersionRange}; // Added VersionRange

/// Error type specific to dependency resolution
#[derive(Error, Debug, Clone, PartialEq, Eq)] // Added derive(Error)
pub enum ResolutionError {
    #[error("Missing required dependency '{dependency_id}' for plugin '{plugin_id}'")]
    MissingDependency {
        plugin_id: String,
        dependency_id: String,
    },
    #[error("Version mismatch for dependency '{dependency_id}' required by plugin '{plugin_id}'. Required: '{required_version}', Found: '{found_version}'")]
    VersionMismatch {
        plugin_id: String,
        dependency_id: String,
        required_version: String, // Store as string for simplicity in error
        found_version: String,
    },
    #[error("Failed to parse version '{version}' for plugin '{plugin_id}': {error}")]
    VersionParseError {
        plugin_id: String,
        version: String,
        error: String,
    },
    #[error("Circular dependency detected involving plugin '{plugin_id}'. Cycle path: {cycle_path:?}")]
    CycleDetected {
        plugin_id: String,
        cycle_path: Vec<String>, // Store the path that formed the cycle
    },
}

/// Loads plugins from the filesystem or other sources
pub struct PluginLoader {
    /// Base directories to search for plugins
    plugin_dirs: Vec<PathBuf>,
    /// Cached plugin manifests
    manifests: HashMap<String, PluginManifest>,
}

impl PluginLoader {
    /// Create a new plugin loader
    pub fn new() -> Self {
        Self {
            plugin_dirs: Vec::new(),
            manifests: HashMap::new(),
        }
    }
    
    /// Add a plugin directory to search
    pub fn add_plugin_dir<P: AsRef<Path>>(&mut self, dir: P) {
        self.plugin_dirs.push(dir.as_ref().to_path_buf());
    }
    
    /// Scan for plugin manifests asynchronously
    pub async fn scan_for_manifests(&mut self) -> Result<Vec<PluginManifest>> {
        let mut manifests = Vec::new();
        
        // Search each plugin directory
        for dir in &self.plugin_dirs {
             // Check directory existence asynchronously
             let dir_exists = match fs::try_exists(dir).await {
                 Ok(exists) => exists,
                 Err(e) => {
                     eprintln!("Error checking existence of plugin directory {}: {}", dir.display(), e);
                     false // Assume doesn't exist on error
                 }
             };

             if !dir_exists {
                 continue;
             }

             // Check if it's a directory asynchronously
             let metadata = match fs::metadata(dir).await {
                 Ok(meta) => meta,
                 Err(e) => {
                     eprintln!("Failed to get metadata for plugin directory {}: {}", dir.display(), e);
                     continue; // Skip this directory
                 }
             };

             if !metadata.is_dir() {
                 continue;
             }
            
            // Scan the directory asynchronously - use the non-recursive function that returns a boxed future
            self.scan_directory_boxed(dir.clone(), &mut manifests).await?;
        }
        
        // Update the cache
        for manifest in &manifests {
            self.manifests.insert(manifest.id.clone(), manifest.clone());
        }
        
        Ok(manifests)
    }
    
    /// Helper function that returns a boxed future for recursive scanning
    fn scan_directory_boxed<'a>(
        &'a self, 
        dir: PathBuf,
        manifests: &'a mut Vec<PluginManifest>
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
        Box::pin(self.scan_directory_inner(dir, manifests))
    }
    
    /// Inner async function that implements the directory scanning logic
    async fn scan_directory_inner(&self, dir: PathBuf, manifests: &mut Vec<PluginManifest>) -> Result<()> {
        // Read directory entries asynchronously
        let mut read_dir_result = fs::read_dir(&dir).await
            .map_err(|e| KernelError::Storage(format!( // Use KernelError
                "Failed to read directory {}: {}",
                dir.display(), e
            )))?;
        
        // Process each entry asynchronously using the ReadDir directly
        while let Some(entry) = read_dir_result.next_entry().await? {
            // Get the path for this entry
            let entry_path = entry.path();
            
            // Check if it's a directory asynchronously
            let metadata = match fs::metadata(&entry_path).await {
                 Ok(meta) => meta,
                 Err(e) => {
                     eprintln!("Failed to get metadata for {}: {}", entry_path.display(), e);
                     continue; // Skip this entry
                 }
            };

            if metadata.is_dir() {
                // Look for manifest.json in this directory
                let manifest_path = entry_path.join("manifest.json");
                
                // Check existence and if it's a file asynchronously
                let manifest_exists = match fs::try_exists(&manifest_path).await {
                    Ok(exists) => exists,
                    Err(e) => {
                        eprintln!("Error checking existence of {}: {}", manifest_path.display(), e);
                        false // Assume not found on error
                    }
                };

                if manifest_exists {
                     // Double check it's a file (try_exists doesn't guarantee)
                     let manifest_meta = match fs::metadata(&manifest_path).await {
                         Ok(meta) => Some(meta),
                         Err(_) => None, // Ignore error if we can't get metadata
                     };
                     
                     if manifest_meta.map_or(false, |m| m.is_file()) {
                        match self.load_manifest(&manifest_path).await {
                            Ok(manifest) => manifests.push(manifest),
                            Err(e) => {
                                eprintln!(
                                    "Error loading manifest from {}: {}",
                                    manifest_path.display(), e
                                );
                            }
                        }
                     }
                }
                
                // Recursively scan subdirectories asynchronously
                // Use the boxed version to handle recursive async calls
                if let Err(e) = self.scan_directory_boxed(entry_path.clone(), manifests).await {
                    eprintln!(
                        "Error scanning subdirectory {}: {}",
                        entry_path.display(), e
                    );
                    // Decide whether to continue or propagate the error
                }
            }
        }
        
        Ok(())
    }
    
    /// Load a plugin manifest from a file asynchronously
    async fn load_manifest<P: AsRef<Path>>(&self, path: P) -> Result<PluginManifest> {
        // Create a path reference once and use it throughout the function
        let path_ref = path.as_ref();
        
        // Read the file content asynchronously
        let content = match fs::read_to_string(path_ref).await {
            Ok(content) => content,
            Err(e) => return Err(KernelError::Storage(format!("Failed to read manifest file {}: {}", path_ref.display(), e))), // Use KernelError
        };

        // In a real implementation, we would parse JSON here
        // For now, just create a basic manifest from the file name
        let path_display = path_ref.display().to_string();
        
        let id = match path_ref.file_stem() {
            Some(stem) => stem.to_string_lossy().to_string(),
            None => "unknown".to_string(),
        };
            
        let name = id.clone();
        
        Ok(PluginManifest::new(
            &id,
            &name,
            "1.0.0", 
            &format!("Plugin loaded from {}", path_display),
            "System"
        ))
    }
    
    /// Load a specific plugin asynchronously (placeholder)
    pub async fn load_plugin(&self, manifest: &PluginManifest) -> Result<Box<dyn Plugin>> {
        // This is a placeholder since actual dynamic loading would be complex
        // In a real implementation, we would:
        // 1. Load the dynamic library asynchronously
        // 2. Look up the plugin creation function
        // 3. Call it to get the plugin instance

        Err(KernelError::Plugin(format!( // Use KernelError
            "Dynamic plugin loading not implemented for plugin: {}",
            manifest.id
        )))
    }

    /// Resolves dependencies between loaded manifests, including cycle detection.
    /// Returns Ok(()) if all dependencies are met, otherwise returns a ResolutionError.
    fn resolve_dependencies(&self) -> std::result::Result<(), ResolutionError> {
        let manifests = &self.manifests;
        let mut visiting = HashSet::new(); // Nodes currently in the recursion stack for DFS
        let mut visited = HashSet::new(); // Nodes that have been fully processed

        // Helper function for DFS-based cycle detection
        fn detect_cycle_dfs<'a>(
            plugin_id: &'a str,
            manifests: &'a HashMap<String, PluginManifest>,
            visiting: &mut HashSet<&'a str>,
            visited: &mut HashSet<&'a str>,
            path: &mut Vec<&'a str>, // Keep track of the current path
        ) -> std::result::Result<(), ResolutionError> {
            visiting.insert(plugin_id);
            path.push(plugin_id);

            if let Some(manifest) = manifests.get(plugin_id) {
                for dep_info in &manifest.dependencies {
                    // Only consider required dependencies for cycle detection that blocks loading
                    if !dep_info.required {
                        continue;
                    }
                    let dep_id = &dep_info.id;

                    // Check if the dependency exists (basic check, full check happens later)
                    if !manifests.contains_key(dep_id) {
                        // This error will be caught by the main loop, but good to be defensive
                        continue;
                    }

                    if visiting.contains(dep_id.as_str()) {
                        // Cycle detected! Find the start of the cycle in the path
                        let cycle_start_index = path.iter().position(|&p| p == dep_id).unwrap_or(0);
                        let cycle_path_slice = &path[cycle_start_index..];
                        return Err(ResolutionError::CycleDetected {
                            plugin_id: plugin_id.to_string(), // The node where the cycle was detected
                            cycle_path: cycle_path_slice.iter().map(|s| s.to_string()).collect(),
                        });
                    }

                    if !visited.contains(dep_id.as_str()) {
                        detect_cycle_dfs(dep_id, manifests, visiting, visited, path)?;
                    }
                }
            }

            path.pop(); // Backtrack: remove current node from path
            visiting.remove(plugin_id);
            visited.insert(plugin_id);
            Ok(())
        }

        // --- Start Cycle Detection ---
        for plugin_id in manifests.keys() {
            if !visited.contains(plugin_id.as_str()) {
                let mut path = Vec::new(); // Path tracker for this DFS run
                detect_cycle_dfs(plugin_id, manifests, &mut visiting, &mut visited, &mut path)?;
            }
        }
        // --- End Cycle Detection ---

        // --- Existing Dependency Checks (Missing/Version) ---
        for (plugin_id, manifest) in manifests {
            // Parse the plugin's own version once
            let plugin_version_str = &manifest.version;
            let _plugin_version = Version::parse(plugin_version_str).map_err(|e| {
                ResolutionError::VersionParseError {
                    plugin_id: plugin_id.clone(),
                    version: plugin_version_str.clone(),
                    error: e.to_string(),
                }
            })?;

            for dep_info in &manifest.dependencies {
                if !dep_info.required {
                    continue; // Skip optional dependencies for now
                }

                let dep_id = &dep_info.id;

                // 1. Check if dependency exists
                let dep_manifest = manifests.get(dep_id).ok_or_else(|| {
                    ResolutionError::MissingDependency {
                        plugin_id: plugin_id.clone(),
                        dependency_id: dep_id.clone(),
                    }
                })?;

                // 2. Check version constraint (if specified)
                if let Some(version_range) = &dep_info.version_range {
                    let dep_version_str = &dep_manifest.version;
                    let dep_version = Version::parse(dep_version_str).map_err(|e| {
                        ResolutionError::VersionParseError {
                            plugin_id: dep_id.clone(), // Error is in the dependency's version
                            version: dep_version_str.clone(),
                            error: e.to_string(),
                        }
                    })?;

                    // Use the VersionRange's internal semver::VersionReq
                    if !version_range.semver_req().matches(&dep_version) {
                        return Err(ResolutionError::VersionMismatch {
                            plugin_id: plugin_id.clone(),
                            dependency_id: dep_id.clone(),
                            required_version: version_range.to_string(),
                            found_version: dep_version_str.clone(),
                        });
                    }
                }
            }
        }

        Ok(())
    }
    
    /// Register all compatible plugins with the registry asynchronously
    /// This now includes dependency resolution before loading.
    pub async fn register_all_plugins(&self, registry: &mut PluginRegistry, api_version: &ApiVersion) -> Result<usize> {
        // --- Dependency Resolution Step ---
        if let Err(e) = self.resolve_dependencies() {
            // Convert ResolutionError to the main kernel Error type
            return Err(KernelError::Plugin(format!("Dependency resolution failed: {}", e)));
        }
        // --- End Dependency Resolution ---

        let mut count = 0;
        // Get all manifests
        let manifests: Vec<_> = self.manifests.values().collect();
        
        // Sort by priority
        // TODO: Implement proper sorting by priority
        
        // Convert the kernel's ApiVersion to semver::Version once for comparisons
        let api_semver = match semver::Version::parse(&api_version.to_string()) {
            Ok(v) => v,
            Err(e) => {
                // If the internal API version fails to parse, something is very wrong.
                return Err(KernelError::Plugin(format!("Internal API version parse error: {}", e)));
            }
        };

        // Load and register each plugin
        for manifest in manifests {
            // Check API compatibility using semver::Version
            let mut compatible = false;
            for version_range in &manifest.api_versions {
                if version_range.includes(&api_semver) { // Compare against api_semver
                    compatible = true;
                    break;
                }
            }
            
            if !compatible {
                println!("Skipping incompatible plugin: {}", manifest.id);
                continue;
            }
            
            // Try to load the plugin asynchronously
            match self.load_plugin(manifest).await {
                Ok(plugin) => {
                    if let Err(e) = registry.register_plugin(plugin) {
                        eprintln!("Failed to register plugin {}: {}", manifest.id, e);
                    } else {
                        count += 1;
                    }
                }
                Err(e) => {
                    // Use KernelError::Plugin for consistency
                    eprintln!("Failed to load plugin {}: {}", manifest.id, KernelError::Plugin(e.to_string()));
                }
            }
        }
        
        Ok(count)
    }
    
    /// Get a manifest by plugin ID
    pub fn get_manifest(&self, id: &str) -> Option<&PluginManifest> {
        self.manifests.get(id)
    }
    
    /// Get all loaded manifests
    pub fn get_all_manifests(&self) -> Vec<&PluginManifest> {
        self.manifests.values().collect()
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}