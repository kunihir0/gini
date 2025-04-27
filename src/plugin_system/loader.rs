use tokio::fs; // Use tokio::fs
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
// Remove the problematic import: use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt; // For stream methods like next()

use crate::kernel::error::{Error, Result};
use crate::plugin_system::traits::Plugin;
use crate::plugin_system::manifest::PluginManifest;
use crate::plugin_system::registry::PluginRegistry;
use crate::plugin_system::version::ApiVersion;

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
            .map_err(|e| Error::Storage(format!(
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
            Err(e) => return Err(Error::Storage(format!("Failed to read manifest file {}: {}", path_ref.display(), e))),
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
        
        Err(Error::Plugin(format!(
            "Dynamic plugin loading not implemented for plugin: {}", 
            manifest.id
        )))
    }
    
    /// Register all compatible plugins with the registry asynchronously
    pub async fn register_all_plugins(&self, registry: &mut PluginRegistry, api_version: &ApiVersion) -> Result<usize> {
        let mut count = 0;
        
        // Get all manifests
        let manifests: Vec<_> = self.manifests.values().collect();
        
        // Sort by priority
        // TODO: Implement proper sorting by priority
        
        // Load and register each plugin
        for manifest in manifests {
            // Check API compatibility
            let mut compatible = false;
            for version_range in &manifest.api_versions {
                if version_range.includes(api_version) {
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
                    eprintln!("Failed to load plugin {}: {}", manifest.id, e);
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