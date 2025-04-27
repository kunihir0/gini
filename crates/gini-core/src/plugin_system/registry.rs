use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::kernel::error::{Error, Result};
use crate::plugin_system::traits::{Plugin, PluginPriority};
use crate::plugin_system::version::ApiVersion;
use crate::plugin_system::dependency::PluginDependency;

/// Registry for managing plugins
pub struct PluginRegistry {
    /// Registered plugins (using Arc for shared ownership)
    pub plugins: HashMap<String, Arc<dyn Plugin>>,
    /// Plugin initialization status
    pub initialized: HashSet<String>,
    /// Enabled plugin IDs
    pub enabled: HashSet<String>,
    /// Current API version
    api_version: ApiVersion,
}

impl PluginRegistry {
    /// Create a new plugin registry with the specified API version
    pub fn new(api_version: ApiVersion) -> Self {
        Self {
            plugins: HashMap::new(),
            initialized: HashSet::new(),
            enabled: HashSet::new(), // Initialize enabled set
            api_version,
        }
    }
    
    /// Register a plugin
    pub fn register_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<()> {
        let name = plugin.name().to_string();
        let id = name.clone(); // Use name as ID for now

        if self.plugins.contains_key(&id) {
            return Err(Error::Plugin(format!("Plugin already registered: {}", name)));
        }
        
        // Check API compatibility
        let mut compatible = false;
        // Convert ApiVersion to semver::Version for comparison
        let api_semver = match semver::Version::parse(&self.api_version.to_string()) {
            Ok(v) => v,
            Err(e) => {
                // Log error and consider it incompatible if internal API version fails parsing
                eprintln!("Failed to parse internal API version {}: {}", self.api_version, e);
                return Err(Error::Plugin(format!("Internal API version parse error: {}", e)));
            }
        };
        for version_range in plugin.compatible_api_versions() {
            if version_range.includes(&api_semver) { // Use includes with semver::Version
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
        
        // All good, wrap in Arc and register the plugin
        let plugin_arc = Arc::from(plugin);
        self.plugins.insert(id.clone(), plugin_arc);
        // Newly registered plugins are enabled by default
        self.enabled.insert(id);
        Ok(())
    }
    
    /// Unregister a plugin by ID
    pub fn unregister_plugin(&mut self, id: &str) -> Result<Arc<dyn Plugin>> {
        if let Some(plugin) = self.plugins.remove(id) {
            self.initialized.remove(id);
            self.enabled.remove(id); // Also remove from enabled set
            Ok(plugin)
        } else {
            Err(Error::Plugin(format!("Plugin not found: {}", id)))
        }
    }
    
    /// Check if a plugin is registered by ID
    pub fn has_plugin(&self, id: &str) -> bool {
        self.plugins.contains_key(id)
    }
    
    /// Get a plugin Arc by ID
    pub fn get_plugin(&self, id: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.get(id).cloned()
    }
    /// Get an iterator over registered plugin IDs and Arc references
    pub fn iter_plugins(&self) -> impl Iterator<Item = (&String, &Arc<dyn Plugin>)> {
        self.plugins.iter()
    }

    /// Get a Vec of all registered plugin Arcs
    pub fn get_plugins_arc(&self) -> Vec<Arc<dyn Plugin>> {
        self.plugins.values().cloned().collect()
    }

    /// Get a Vec of enabled plugin Arcs
    pub fn get_enabled_plugins_arc(&self) -> Vec<Arc<dyn Plugin>> {
        self.plugins
            .iter()
            .filter(|(id, _)| self.enabled.contains(*id))
            .map(|(_, plugin)| plugin.clone())
            .collect()
    }
    
    /// Initialize a plugin by ID
    pub fn initialize_plugin(&mut self, id: &str, app: &mut crate::kernel::bootstrap::Application) -> Result<()> {
        // Ensure plugin exists and is enabled before initializing
        if !self.has_plugin(id) {
            return Err(Error::Plugin(format!("Plugin not found: {}", id)));
        }
        if !self.is_enabled(id) {
             println!("Plugin {} is disabled, skipping initialization.", id);
             return Ok(()); // Skip initialization if disabled
        }

        if self.initialized.contains(id) {
            return Ok(()); // Already initialized
        }

        // Get plugin dependencies (needs cloning the Arc to access methods)
        let plugin_arc = match self.plugins.get(id) {
             Some(p) => p.clone(),
             None => return Err(Error::Plugin(format!("Plugin {} disappeared unexpectedly", id))), // Should not happen if has_plugin passed
        };
        let dependencies = plugin_arc.dependencies().clone();


        // Initialize dependencies first
        for dep in dependencies {
            // Only initialize required dependencies that exist and are enabled
            if dep.required && self.has_plugin(&dep.plugin_name) && self.is_enabled(&dep.plugin_name) && !self.initialized.contains(&dep.plugin_name) {
                 self.initialize_plugin(&dep.plugin_name, app)?;
            } else if dep.required && (!self.has_plugin(&dep.plugin_name) || !self.is_enabled(&dep.plugin_name)) {
                // If a required dependency is missing or disabled, fail initialization
                return Err(Error::Plugin(format!(
                    "Plugin {} requires enabled dependency {}, which is missing or disabled.",
                    id, dep.plugin_name
                )));
            }
        }

        // Now initialize the plugin itself
        // We use the cloned Arc here. Assuming `init` doesn't need `&mut self`.
        // If `init` requires `&mut self`, the design needs rethinking (e.g., Arc<Mutex<dyn Plugin>>).
        plugin_arc.init(app)?;
        self.initialized.insert(id.to_string());
        Ok(())
    }
    
    /// Initialize all plugins in dependency order
    pub fn initialize_all(&mut self, app: &mut crate::kernel::bootstrap::Application) -> Result<()> {
        // Get IDs of enabled plugins
        let mut enabled_plugin_ids: Vec<String> = self.enabled.iter().cloned().collect();

        // Sort enabled plugins by priority
        enabled_plugin_ids.sort_by(|a, b| {
            let priority_a = self.plugins.get(a).map(|p| p.priority()).unwrap_or(PluginPriority::ThirdPartyLow(255));
            let priority_b = self.plugins.get(b).map(|p| p.priority()).unwrap_or(PluginPriority::ThirdPartyLow(255));
            priority_a.cmp(&priority_b)
        });

        // Initialize enabled plugins in order
        for id in enabled_plugin_ids {
            // Check if already initialized (might happen due to dependency resolution)
            if !self.initialized.contains(&id) {
                // initialize_plugin already checks for enabled status, but double-checking doesn't hurt
                 if self.is_enabled(&id) {
                    self.initialize_plugin(&id, app)?;
                 }
            }
        }
        Ok(())
    }
    
    /// Shutdown all plugins
    pub fn shutdown_all(&mut self) -> Result<()> {
        // Get IDs of initialized plugins
        let mut initialized_plugin_ids: Vec<String> = self.initialized.iter().cloned().collect();

        // Sort initialized plugins by priority in reverse for shutdown
        initialized_plugin_ids.sort_by(|a, b| {
            let priority_a = self.plugins.get(a).map(|p| p.priority()).unwrap_or(PluginPriority::ThirdPartyLow(255));
            let priority_b = self.plugins.get(b).map(|p| p.priority()).unwrap_or(PluginPriority::ThirdPartyLow(255));
            // Reverse the comparison for shutdown order
            priority_b.cmp(&priority_a)
        });

        // Shutdown each initialized plugin
        let mut shutdown_errors = Vec::new();
        for id in initialized_plugin_ids {
            if let Some(plugin) = self.plugins.get(&id) {
                 // Check if it's still marked as initialized before shutting down
                 if self.initialized.contains(&id) {
                    println!("Shutting down plugin: {}", id);
                    if let Err(e) = plugin.shutdown() {
                        let err_msg = format!("Error shutting down plugin {}: {}", id, e);
                        eprintln!("{}", err_msg);
                        shutdown_errors.push(err_msg);
                        // Continue shutting down others even if one fails
                    }
                    // Mark as uninitialized *after* attempting shutdown
                    self.initialized.remove(&id);
                 }
            }
        }

        if shutdown_errors.is_empty() {
            Ok(())
        } else {
            Err(Error::Plugin(format!(
                "Encountered errors during plugin shutdown: {}",
                shutdown_errors.join("; ")
            )))
        }
    }
    
    /// Get all registered plugin IDs
    pub fn get_plugin_ids(&self) -> Vec<String> {
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
        for (id, plugin) in &self.plugins {
             // Only check dependencies for enabled plugins
             if !self.is_enabled(id) {
                 continue;
             }
            for dep in plugin.dependencies() {
                // Check if the dependency exists and is enabled
                let dep_exists = self.has_plugin(&dep.plugin_name);
                let dep_enabled = self.is_enabled(&dep.plugin_name);

                if dep.required && (!dep_exists || !dep_enabled) {
                    return Err(Error::Plugin(format!(
                        "Enabled plugin '{}' requires enabled plugin '{}', which is missing or disabled.",
                        id, dep.plugin_name
                    )));
                }

                // Check version compatibility if the dependency exists (regardless of enabled status for version check)
                if dep_exists {
                    if let Some(dep_plugin) = self.get_plugin(&dep.plugin_name) { // get_plugin returns Option<Arc<dyn Plugin>>
                        if let Some(ref required_range) = dep.version_range {
                            // Parse the dependency's actual version string into semver::Version
                            match semver::Version::parse(dep_plugin.version()) {
                                Ok(dep_version) => {
                                    // Use the includes method which takes semver::Version
                                    if !required_range.includes(&dep_version) {
                                        return Err(Error::Plugin(format!(
                                            "Plugin '{}' requires dependency '{}' version '{}', but found version '{}'",
                                            id,
                                            dep.plugin_name,
                                            required_range.constraint_string(), // Use constraint_string() for display
                                            dep_plugin.version()
                                        )));
                                    }
                                },
                                Err(e) => {
                                    // Error parsing the dependency's version string
                                    return Err(Error::Plugin(format!(
                                        "Failed to parse version string '{}' for dependency plugin '{}': {}",
                                        dep_plugin.version(), dep.plugin_name, e
                                    )));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

     /// Enable a plugin by ID
     pub fn enable_plugin(&mut self, id: &str) -> Result<()> {
         if !self.has_plugin(id) {
             return Err(Error::Plugin(format!("Cannot enable non-existent plugin: {}", id)));
         }
         self.enabled.insert(id.to_string());
         println!("Plugin {} enabled.", id);
         Ok(())
     }

     /// Disable a plugin by ID
     pub fn disable_plugin(&mut self, id: &str) -> Result<()> {
         if !self.has_plugin(id) {
             // Disabling a non-existent plugin might be considered a no-op or an error.
             // Let's treat it as a no-op for now, but log it.
             println!("Attempted to disable non-existent plugin: {}", id);
             return Ok(());
         }
         if self.initialized.contains(id) {
             // Ideally, we should shut down the plugin if it's initialized before disabling.
             // However, shutdown logic is complex (dependency order).
             // For now, we prevent disabling an initialized plugin.
             // TODO: Implement safe disabling of initialized plugins (requires shutdown).
             return Err(Error::Plugin(format!(
                 "Cannot disable plugin '{}' while it is initialized. Stop the application first.",
                 id
             )));
         }

         if self.enabled.remove(id) {
             println!("Plugin {} disabled.", id);
         } else {
             println!("Plugin {} was already disabled.", id);
         }
         Ok(())
     }

     /// Check if a plugin is enabled by ID
     pub fn is_enabled(&self, id: &str) -> bool {
         self.enabled.contains(id)
     }
}
