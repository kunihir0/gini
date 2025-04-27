use std::any::Any;
use std::fmt::Debug;
use std::path::Path;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex; // Use Tokio Mutex for async safety if needed

use crate::kernel::component::KernelComponent;
use crate::kernel::error::{Error, Result};
use crate::plugin_system::{Plugin, PluginManifest, ApiVersion, PluginPriority, PluginRegistry}; // Import PluginRegistry
use crate::kernel::constants; // Import constants

/// Plugin system component interface
#[async_trait]
pub trait PluginManager: KernelComponent {
    /// Load a plugin from a file
    async fn load_plugin(&self, path: &Path) -> Result<()>;

    /// Load all plugins from a directory
    async fn load_plugins_from_directory(&self, dir: &Path) -> Result<usize>;

    /// Get a plugin by ID
    async fn get_plugin(&self, id: &str) -> Result<Option<Arc<dyn Plugin>>>; // Return Result<Option<Arc>>

    /// Get all loaded plugins
    async fn get_plugins(&self) -> Result<Vec<Arc<dyn Plugin>>>; // Return Result<Vec<Arc>>

    /// Check if a plugin is loaded
    async fn is_plugin_loaded(&self, id: &str) -> Result<bool>; // Return Result<bool>

    /// Get plugin dependencies
    async fn get_plugin_dependencies(&self, id: &str) -> Result<Vec<String>>;

    /// Get plugins that depend on a plugin
    async fn get_dependent_plugins(&self, id: &str) -> Result<Vec<String>>;

    /// Enable a plugin (potentially async if it involves loading/init)
    async fn enable_plugin(&self, id: &str) -> Result<()>;

    /// Disable a plugin (potentially async if it involves unloading/shutdown)
    async fn disable_plugin(&self, id: &str) -> Result<()>;

    /// Check if a plugin is enabled
    async fn is_plugin_enabled(&self, id: &str) -> Result<bool>; // Return Result<bool>

    /// Get plugin manifest
    async fn get_plugin_manifest(&self, id: &str) -> Result<Option<PluginManifest>>; // Return Result<Option>
}

/// Default implementation of plugin manager
#[derive(Clone)] // Add Clone derive
pub struct DefaultPluginManager {
    name: &'static str,
    // Use Tokio Mutex for async safety with the registry
    registry: Arc<Mutex<PluginRegistry>>,
}

impl DefaultPluginManager {
    /// Create a new default plugin manager
    pub fn new() -> Result<Self> { // Return Result for parsing
        // Parse the API version from constants
        let api_version = ApiVersion::from_str(constants::API_VERSION)
            .map_err(|e| Error::Init(format!("Failed to parse API_VERSION constant: {}", e)))?;

        Ok(Self {
            name: "DefaultPluginManager",
            // Create PluginRegistry with the parsed ApiVersion
            registry: Arc::new(Mutex::new(PluginRegistry::new(api_version))),
        })
    }

    /// Get reference to the plugin registry Arc<Mutex>
    pub fn registry(&self) -> &Arc<Mutex<PluginRegistry>> {
        &self.registry
    }
}

impl Debug for DefaultPluginManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Avoid locking in Debug if possible, or show minimal info
        f.debug_struct("DefaultPluginManager")
            .field("name", &self.name)
            .finish_non_exhaustive() // Indicate registry state is omitted
    }
}

#[async_trait]
impl KernelComponent for DefaultPluginManager {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn initialize(&self) -> Result<()> {
        // Potentially load core plugins here
        println!("Initializing Plugin Manager...");
        // Example: Load plugins from a default directory
        // self.load_plugins_from_directory(Path::new(constants::CORE_PLUGINS_DIR)).await?;
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        // Initialize all loaded plugins
        println!("Starting Plugin Manager - Initializing plugins...");
        // This needs access to the Application/Kernel, which isn't directly available here.
        // Dependency injection or passing the kernel reference during init/start is needed.
        // For now, we skip the actual initialization call.
        // let mut registry = self.registry.lock().await;
        // registry.initialize_all(&mut app)?; // Needs app reference
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        // Shutdown all plugins
        println!("Stopping Plugin Manager - Shutting down plugins...");
        let mut registry = self.registry.lock().await;
        registry.shutdown_all()
    }
    // Removed as_any and as_any_mut
}

#[async_trait]
impl PluginManager for DefaultPluginManager {
    async fn load_plugin(&self, path: &Path) -> Result<()> {
        // Placeholder - Requires PluginLoader implementation
        println!("Placeholder: Load plugin from {:?}", path);
        Err(Error::Plugin("load_plugin not implemented".into()))
    }

    async fn load_plugins_from_directory(&self, dir: &Path) -> Result<usize> {
        // Placeholder - Requires PluginLoader implementation
        println!("Placeholder: Load plugins from directory {:?}", dir);
        Err(Error::Plugin("load_plugins_from_directory not implemented".into()))
    }

    async fn get_plugin(&self, id: &str) -> Result<Option<Arc<dyn Plugin>>> {
        let registry = self.registry.lock().await;
        // PluginRegistry::get_plugin returns &dyn Plugin, need to wrap in Arc if possible
        // This requires plugins to be stored as Arc<dyn Plugin> in the registry
        // For now, return None as the structure doesn't support Arc directly
        Ok(None) // Placeholder
    }

    async fn get_plugins(&self) -> Result<Vec<Arc<dyn Plugin>>> {
        let registry = self.registry.lock().await;
        // Similar to get_plugin, requires registry to store Arc<dyn Plugin>
        Ok(Vec::new()) // Placeholder
    }

    async fn is_plugin_loaded(&self, id: &str) -> Result<bool> {
        let registry = self.registry.lock().await;
        Ok(registry.has_plugin(id))
    }

    async fn get_plugin_dependencies(&self, id: &str) -> Result<Vec<String>> {
        let registry = self.registry.lock().await;
        if let Some(plugin) = registry.get_plugin(id) {
            Ok(plugin.dependencies().iter().map(|dep| dep.plugin_name.clone()).collect())
        } else {
            Err(Error::Plugin(format!("Plugin not found: {}", id)))
        }
    }

    async fn get_dependent_plugins(&self, id: &str) -> Result<Vec<String>> {
        let registry = self.registry.lock().await;
        let mut dependents = Vec::new();
        // Use the public iterator method
        for (name, plugin) in registry.iter_plugins() {
            if plugin.dependencies().iter().any(|dep| dep.plugin_name == id) {
                dependents.push(name.clone());
            }
        }
        Ok(dependents)
    }

    async fn enable_plugin(&self, id: &str) -> Result<()> {
        // Placeholder - Requires state management for enabled/disabled plugins
        println!("Placeholder: Enable plugin {}", id);
        Err(Error::Plugin("enable_plugin not implemented".into()))
    }

    async fn disable_plugin(&self, id: &str) -> Result<()> {
        // Placeholder - Requires state management and potentially unloading/shutdown
        println!("Placeholder: Disable plugin {}", id);
        Err(Error::Plugin("disable_plugin not implemented".into()))
    }

    async fn is_plugin_enabled(&self, id: &str) -> Result<bool> {
        // Placeholder - Requires state management
        println!("Placeholder: Check if plugin {} is enabled", id);
        // Assume loaded means enabled for now, if found
        self.is_plugin_loaded(id).await
    }

    async fn get_plugin_manifest(&self, id: &str) -> Result<Option<PluginManifest>> {
        // Placeholder - Manifests are usually loaded separately or part of the plugin trait
        println!("Placeholder: Get manifest for plugin {}", id);
        Ok(None) // Placeholder
    }
}

// Removed Default implementation as Self::new() now returns Result