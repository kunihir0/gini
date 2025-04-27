use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::kernel::error::{Error, Result};
use crate::plugin_system::traits::{Plugin, PluginPriority};
use crate::plugin_system::version::ApiVersion;
use crate::plugin_system::dependency::PluginDependency;

/// Registry for managing plugins
pub struct PluginRegistry {
    /// Registered plugins
    plugins: HashMap<String, Box<dyn Plugin>>,
    /// Plugin initialization status
    initialized: HashSet<String>,
    /// Current API version
    api_version: ApiVersion,
}

impl PluginRegistry {
    /// Create a new plugin registry with the specified API version
    pub fn new(api_version: ApiVersion) -> Self {
        Self {
            plugins: HashMap::new(),
            initialized: HashSet::new(),
            api_version,
        }
    }
    
    /// Register a plugin
    pub fn register_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<()> {
        let name = plugin.name().to_string();
        
        if self.plugins.contains_key(&name) {
            return Err(Error::Plugin(format!("Plugin already registered: {}", name)));
        }
        
        // Check API compatibility
        let mut compatible = false;
        for version_range in plugin.compatible_api_versions() {
            if version_range.includes(&self.api_version) {
                compatible = true;
                break;
            }
        }
        
        if !compatible {
            return Err(Error::Plugin(format!(
                "Plugin {} is not compatible with API version {}",
                name,
                self.api_version
            )));
        }
        
        // All good, register the plugin
        self.plugins.insert(name, plugin);
        Ok(())
    }
    
    /// Unregister a plugin
    pub fn unregister_plugin(&mut self, name: &str) -> Result<Box<dyn Plugin>> {
        if let Some(plugin) = self.plugins.remove(name) {
            self.initialized.remove(name);
            Ok(plugin)
        } else {
            Err(Error::Plugin(format!("Plugin not found: {}", name)))
        }
    }
    
    /// Check if a plugin is registered
    pub fn has_plugin(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }
    
    /// Get a plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<&dyn Plugin> {
        self.plugins.get(name).map(AsRef::as_ref)
    }
/// Get an iterator over registered plugin names and references
    pub fn iter_plugins(&self) -> impl Iterator<Item = (&String, &Box<dyn Plugin>)> {
        self.plugins.iter()
    }
    
    /// Initialize a plugin
    pub fn initialize_plugin(&mut self, name: &str, app: &mut crate::kernel::bootstrap::Application) -> Result<()> {
        if !self.has_plugin(name) {
            return Err(Error::Plugin(format!("Plugin not found: {}", name)));
        }
        
        if self.initialized.contains(name) {
            return Ok(()); // Already initialized
        }
        
        // Get plugin dependencies
        let mut dependencies = Vec::new();
        if let Some(plugin) = self.plugins.get(name) {
            dependencies = plugin.dependencies().clone();
        }
        
        // Initialize dependencies first
        for dep in dependencies {
            if dep.required && !self.initialized.contains(&dep.plugin_name) {
                self.initialize_plugin(&dep.plugin_name, app)?;
            }
        }
        
        // Now initialize the plugin
        if let Some(plugin) = self.plugins.get_mut(name) {
            plugin.init(app)?;
            self.initialized.insert(name.to_string());
            Ok(())
        } else {
            Err(Error::Plugin(format!("Plugin not found: {}", name)))
        }
    }
    
    /// Initialize all plugins in dependency order
    pub fn initialize_all(&mut self, app: &mut crate::kernel::bootstrap::Application) -> Result<()> {
        // Sort plugins by priority
        let mut plugin_names: Vec<String> = self.plugins.keys().cloned().collect();
        plugin_names.sort_by(|a, b| {
            let priority_a = self.plugins.get(a).map(|p| p.priority()).unwrap_or(PluginPriority::ThirdPartyLow(255));
            let priority_b = self.plugins.get(b).map(|p| p.priority()).unwrap_or(PluginPriority::ThirdPartyLow(255));
            
            priority_a.cmp(&priority_b)
        });
        
        // Initialize in order
        for name in plugin_names {
            if !self.initialized.contains(&name) {
                self.initialize_plugin(&name, app)?;
            }
        }
        
        Ok(())
    }
    
    /// Shutdown all plugins
    pub fn shutdown_all(&mut self) -> Result<()> {
        // Shutdown in reverse initialization order (stack unwinding)
        let mut plugin_names: Vec<String> = self.initialized.iter().cloned().collect();
        
        // Sort plugins by priority in reverse
        plugin_names.sort_by(|a, b| {
            let priority_a = self.plugins.get(a).map(|p| p.priority()).unwrap_or(PluginPriority::ThirdPartyLow(255));
            let priority_b = self.plugins.get(b).map(|p| p.priority()).unwrap_or(PluginPriority::ThirdPartyLow(255));
            
            // Reverse the comparison
            priority_b.cmp(&priority_a)
        });
        
        // Shutdown each plugin
        for name in plugin_names {
            if let Some(plugin) = self.plugins.get(&name) {
                if let Err(e) = plugin.shutdown() {
                    eprintln!("Error shutting down plugin {}: {}", name, e);
                }
            }
            self.initialized.remove(&name);
        }
        
        Ok(())
    }
    
    /// Get all registered plugin names
    pub fn get_plugin_names(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }
    
    /// Get the number of registered plugins
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
    
    /// Get the number of initialized plugins
    pub fn initialized_count(&self) -> usize {
        self.initialized.len()
    }
    
    /// Get the current API version
    pub fn api_version(&self) -> &ApiVersion {
        &self.api_version
    }
    
    /// Check for plugin dependencies
    pub fn check_dependencies(&self) -> Result<()> {
        for (name, plugin) in &self.plugins {
            for dep in plugin.dependencies() {
                if dep.required && !self.has_plugin(&dep.plugin_name) {
                    return Err(Error::Plugin(format!(
                        "Plugin {} requires missing plugin {}",
                        name,
                        dep.plugin_name
                    )));
                }
                
                // Check version compatibility if required
                if let Some(plugin) = self.get_plugin(&dep.plugin_name) {
                    if let Some(ref version_range) = dep.version_range {
                        match crate::plugin_system::version::ApiVersion::from_str(plugin.version()) {
                            Ok(version) => {
                                if !version_range.includes(&version) {
                                    return Err(Error::Plugin(format!(
                                        "Plugin {} requires {} version {}, but found {}",
                                        name,
                                        dep.plugin_name,
                                        version_range.min,
                                        plugin.version()
                                    )));
                                }
                            },
                            Err(e) => {
                                return Err(Error::Plugin(format!(
                                    "Invalid version format in plugin {}: {}",
                                    dep.plugin_name, e
                                )));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// Thread-safe plugin registry
pub struct SharedPluginRegistry {
    registry: Arc<Mutex<PluginRegistry>>,
}

impl SharedPluginRegistry {
    /// Create a new shared plugin registry
    pub fn new(api_version: ApiVersion) -> Self {
        Self {
            registry: Arc::new(Mutex::new(PluginRegistry::new(api_version))),
        }
    }
    
    /// Get a reference to the registry
    pub fn registry(&self) -> Arc<Mutex<PluginRegistry>> {
        self.registry.clone()
    }
    
    /// Register a plugin
    pub fn register_plugin(&self, plugin: Box<dyn Plugin>) -> Result<()> {
        let mut registry = self.registry.lock().map_err(|e| {
            Error::Plugin(format!("Failed to lock plugin registry: {}", e))
        })?;
        
        registry.register_plugin(plugin)
    }
    
    /// Initialize all plugins
    pub fn initialize_all(&self, app: &mut crate::kernel::bootstrap::Application) -> Result<()> {
        let mut registry = self.registry.lock().map_err(|e| {
            Error::Plugin(format!("Failed to lock plugin registry: {}", e))
        })?;
        
        registry.initialize_all(app)
    }
    
    /// Shutdown all plugins
    pub fn shutdown_all(&self) -> Result<()> {
        let mut registry = self.registry.lock().map_err(|e| {
            Error::Plugin(format!("Failed to lock plugin registry: {}", e))
        })?;
        
        registry.shutdown_all()
    }
    
    /// Check plugin dependencies
    pub fn check_dependencies(&self) -> Result<()> {
        let registry = self.registry.lock().map_err(|e| {
            Error::Plugin(format!("Failed to lock plugin registry: {}", e))
        })?;
        
        registry.check_dependencies()
    }
    
    /// Get all plugin names
    pub fn get_plugin_names(&self) -> Result<Vec<String>> {
        let registry = self.registry.lock().map_err(|e| {
            Error::Plugin(format!("Failed to lock plugin registry: {}", e))
        })?;
        
        Ok(registry.get_plugin_names())
    }
}