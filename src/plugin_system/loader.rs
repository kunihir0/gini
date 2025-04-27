use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

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
    
    /// Scan for plugin manifests
    pub fn scan_for_manifests(&mut self) -> Result<Vec<PluginManifest>> {
        let mut manifests = Vec::new();
        
        // Search each plugin directory
        for dir in &self.plugin_dirs {
            if !dir.exists() || !dir.is_dir() {
                continue;
            }
            
            // Use clone here to avoid borrow issues
            self.scan_directory(dir.clone(), &mut manifests)?;
        }
        
        // Update the cache
        for manifest in &manifests {
            self.manifests.insert(manifest.id.clone(), manifest.clone());
        }
        
        Ok(manifests)
    }
    
    /// Scan a directory recursively for plugin manifests
    fn scan_directory(&self, dir: PathBuf, manifests: &mut Vec<PluginManifest>) -> Result<()> {
        // Read directory entries
        let read_dir_result = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) => {
                return Err(Error::Storage(format!(
                    "Failed to read directory {}: {}", 
                    dir.display(), e
                )));
            }
        };
        
        // Process each entry
        for entry_result in read_dir_result {
            let entry = match entry_result {
                Ok(entry) => entry,
                Err(e) => {
                    return Err(Error::Storage(format!(
                        "Failed to read directory entry in {}: {}", 
                        dir.display(), e
                    )));
                }
            };
            
            // Get the path for this entry
            let entry_path = entry.path();
            
            // If it's a directory, check for manifest and scan recursively
            if entry_path.is_dir() {
                // Look for manifest.json in this directory
                let manifest_path = entry_path.join("manifest.json");
                if manifest_path.exists() && manifest_path.is_file() {
                    match self.load_manifest(&manifest_path) {
                        Ok(manifest) => manifests.push(manifest),
                        Err(e) => {
                            eprintln!(
                                "Error loading manifest from {}: {}", 
                                manifest_path.display(), e
                            );
                        }
                    }
                }
                
                // Recursively scan subdirectories
                // Use clone to avoid borrow issues
                if let Err(e) = self.scan_directory(entry_path.clone(), manifests) {
                    eprintln!(
                        "Error scanning subdirectory {}: {}", 
                        entry_path.display(), e
                    );
                }
            }
        }
        
        Ok(())
    }
    
    /// Load a plugin manifest from a file
    fn load_manifest<P: AsRef<Path>>(&self, path: P) -> Result<PluginManifest> {
        // Create a path reference once and use it throughout the function
        let path_ref = path.as_ref();
        
        // Read the file content
        let content = match fs::read_to_string(path_ref) {
            Ok(content) => content,
            Err(e) => return Err(Error::Storage(format!("Failed to read manifest file: {}", e))),
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
    
    /// Load a specific plugin
    pub fn load_plugin(&self, manifest: &PluginManifest) -> Result<Box<dyn Plugin>> {
        // This is a placeholder since actual dynamic loading would be complex
        // In a real implementation, we would:
        // 1. Load the dynamic library
        // 2. Look up the plugin creation function
        // 3. Call it to get the plugin instance
        
        Err(Error::Plugin(format!(
            "Dynamic plugin loading not implemented for plugin: {}", 
            manifest.id
        )))
    }
    
    /// Register all compatible plugins with the registry
    pub fn register_all_plugins(&self, registry: &mut PluginRegistry, api_version: &ApiVersion) -> Result<usize> {
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
            
            // Try to load the plugin
            match self.load_plugin(manifest) {
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