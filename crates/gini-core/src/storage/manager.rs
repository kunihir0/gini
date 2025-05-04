use std::any::Any;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc; // Remove Mutex import
use async_trait::async_trait;
use std::env;

use crate::kernel::component::KernelComponent;
use crate::kernel::error::{Error, Result};
use crate::storage::provider::StorageProvider;
use crate::storage::local::LocalStorageProvider; // Default provider
use crate::storage::config::{ConfigManager, ConfigFormat, ConfigData, ConfigScope, PluginConfigScope, ConfigStorageExt};

/// Storage manager component interface
/// This simply wraps a StorageProvider for now
#[async_trait]
pub trait StorageManager: KernelComponent + StorageProvider {}

/// Default implementation of StorageManager
#[derive(Clone)] // Add Clone derive
pub struct DefaultStorageManager {
    name: &'static str,
    provider: Arc<dyn StorageProvider>, // Holds the actual provider
    config_manager: Arc<ConfigManager<LocalStorageProvider>>, // Wrap ConfigManager in Arc for Clone
    app_config_path: PathBuf, // Path to application configurations
    plugin_config_path: PathBuf, // Path to plugin configurations
    user_data_path: PathBuf, // Path to user data
}

impl DefaultStorageManager {
    /// Create a new default storage manager with a LocalStorageProvider
    pub fn new(base_path: PathBuf) -> Self {
        // Define standard paths
        let app_config_path = base_path.join("config");
        let plugin_config_path = base_path.join("plugins").join("config");
        let user_data_path = base_path.join("user");
        
        // Create the provider
        let local_provider = LocalStorageProvider::new(base_path.clone());
        let provider = Arc::new(local_provider) as Arc<dyn StorageProvider>;
        
        // Create the provider
        let provider = Arc::new(LocalStorageProvider::new(base_path.clone()));
        
        // Create a local provider specifically for config management
        let config_provider = LocalStorageProvider::new(base_path.clone());
        
        // Create the config manager
        let config_manager = ConfigManager::new(
            Arc::new(config_provider),
            app_config_path.clone(),
            plugin_config_path.clone(),
            ConfigFormat::Json, // Default to JSON format
        );
        
        Self {
            name: "DefaultStorageManager",
            provider,
            config_manager: Arc::new(config_manager), // Wrap in Arc
            app_config_path,
            plugin_config_path,
            user_data_path,
        }
    }

    /// Create a new storage manager with a custom provider
    pub fn with_provider(provider: Arc<dyn StorageProvider>) -> Self {
        // Get base path from provider or use default
        let base_path = PathBuf::from("."); // Default to current directory
        
        // Define standard paths
        let app_config_path = base_path.join("config");
        let plugin_config_path = base_path.join("plugins").join("config");
        let user_data_path = base_path.join("user");
        
        // Create a LocalStorageProvider for the config manager
        let config_provider = Arc::new(LocalStorageProvider::new(base_path.clone()));
        
        // Create a separate local provider for config management
        let config_provider = LocalStorageProvider::new(base_path.clone());
        
        // Create the config manager
        let config_manager = ConfigManager::new(
            Arc::new(config_provider),
            app_config_path.clone(),
            plugin_config_path.clone(),
            ConfigFormat::Json, // Default to JSON format
        );
        
        Self {
            name: "DefaultStorageManager", // Or derive from provider?
            provider,
            config_manager: Arc::new(config_manager), // Wrap in Arc
            app_config_path,
            plugin_config_path,
            user_data_path,
        }
    }

    /// Get the underlying provider
    pub fn provider(&self) -> &Arc<dyn StorageProvider> {
        &self.provider
    }

    /// Get the configuration manager instance (renamed to avoid potential conflicts)
    pub fn get_config_manager(&self) -> &Arc<ConfigManager<LocalStorageProvider>> {
        &self.config_manager
    }
    
    /// Get the application configuration path
    pub fn app_config_path(&self) -> &Path {
        &self.app_config_path
    }
    
    /// Get the plugin configuration path
    pub fn plugin_config_path(&self) -> &Path {
        &self.plugin_config_path
    }
    
    /// Get the user data path
    pub fn user_data_path(&self) -> &Path {
        &self.user_data_path
    }
    
    /// Ensure all required directories exist
    pub fn ensure_directories(&self) -> Result<()> {
        // Ensure application config directory exists
        self.create_dir_all(&self.app_config_path)?;
        
        // Ensure plugin config directories exist
        self.create_dir_all(&self.plugin_config_path.join("default"))?;
        self.create_dir_all(&self.plugin_config_path.join("user"))?;
        
        // Ensure user data directory exists
        self.create_dir_all(&self.user_data_path)?;
        
        Ok(())
    }
}

impl Debug for DefaultStorageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultStorageManager")
            .field("name", &self.name)
            .field("provider", &self.provider.name()) // Show provider name
            .finish()
    }
}

#[async_trait]
impl KernelComponent for DefaultStorageManager {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn initialize(&self) -> Result<()> {
        // Create required directories
        self.ensure_directories()?;
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        // Delegate to provider if it has a start method (currently doesn't)
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        // Delegate to provider if it has a stop method (currently doesn't)
        Ok(())
    }
    // Removed as_any and as_any_mut
}

// Implement StorageProvider by delegating to the internal provider
impl StorageProvider for DefaultStorageManager {
    fn name(&self) -> &str {
        self.provider.name()
    }

    fn exists(&self, path: &Path) -> bool {
        self.provider.exists(path)
    }

    fn is_file(&self, path: &Path) -> bool {
        self.provider.is_file(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        self.provider.is_dir(path)
    }

    fn create_dir(&self, path: &Path) -> Result<()> {
        self.provider.create_dir(path)
    }

    fn create_dir_all(&self, path: &Path) -> Result<()> {
        self.provider.create_dir_all(path)
    }

    fn read_to_string(&self, path: &Path) -> Result<String> {
        self.provider.read_to_string(path)
    }

    fn read_to_bytes(&self, path: &Path) -> Result<Vec<u8>> {
        self.provider.read_to_bytes(path)
    }

    fn write_string(&self, path: &Path, contents: &str) -> Result<()> {
        self.provider.write_string(path, contents)
    }

    fn write_bytes(&self, path: &Path, contents: &[u8]) -> Result<()> {
        self.provider.write_bytes(path, contents)
    }

    fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        self.provider.copy(from, to)
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        self.provider.rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> Result<()> {
        self.provider.remove_file(path)
    }

    fn remove_dir(&self, path: &Path) -> Result<()> {
        self.provider.remove_dir(path)
    }

    fn remove_dir_all(&self, path: &Path) -> Result<()> {
        self.provider.remove_dir_all(path)
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        self.provider.read_dir(path)
    }

    fn metadata(&self, path: &Path) -> Result<std::fs::Metadata> {
        self.provider.metadata(path)
    }

    // Note: open_read, open_write, open_append return Box<dyn Read/Write>
    // which might not be Send/Sync. This could be an issue if the manager
    // needs to be Send/Sync. For now, we delegate directly.
    fn open_read(&self, path: &Path) -> Result<Box<dyn std::io::Read>> {
        self.provider.open_read(path)
    }

    fn open_write(&self, path: &Path) -> Result<Box<dyn std::io::Write>> {
        self.provider.open_write(path)
    }

    fn open_append(&self, path: &Path) -> Result<Box<dyn std::io::Write>> {
        self.provider.open_append(path)
    }
}

// Implement the marker trait
impl StorageManager for DefaultStorageManager {}

// Default using current directory (or appropriate default)
impl Default for DefaultStorageManager {
    fn default() -> Self {
        // Determine a sensible default base path, e.g., current dir or user data dir
        let default_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::new(default_path)
    }
}

// Implement ConfigStorageExt trait for DefaultStorageManager
impl ConfigStorageExt for DefaultStorageManager {
    fn config_manager<P: StorageProvider + ?Sized>(&self) -> &ConfigManager<P> {
        // Transmute is still problematic here, especially with Arc.
        // The type is now Arc<ConfigManager<LocalStorageProvider>>, not &ConfigManager<...>.
        // This needs a proper redesign. For now, keeping the potentially incorrect transmute
        // to focus on the Clone + Send + Sync fix. It will likely fail if P is not LocalStorageProvider.
        // TODO: Redesign ConfigStorageExt trait or its usage.
        unsafe { std::mem::transmute(&*self.config_manager) } // Deref Arc before transmute (still likely wrong)
    }
}

// Implement additional configuration methods directly on DefaultStorageManager for convenience
impl DefaultStorageManager {
    // Make load_config method directly accessible from DefaultStorageManager
    pub fn load_config(&self, name: &str, scope: crate::storage::config::ConfigScope) -> Result<ConfigData> {
        self.config_manager.load_config(name, scope) // Reverted
    }

    // Make list_configs method directly accessible from DefaultStorageManager
    pub fn list_configs(&self, scope: crate::storage::config::ConfigScope) -> Result<Vec<String>> {
        self.config_manager.list_configs(scope) // Reverted
    }
    
    // Get plugin configuration with user override support
    pub fn get_plugin_config(&self, plugin_name: &str) -> Result<ConfigData> {
        self.config_manager.get_plugin_config(plugin_name) // Reverted
    }
    
    // Save plugin-specific configuration
    pub fn save_plugin_config(
        &self,
        plugin_name: &str,
        config: &ConfigData,
        scope: crate::storage::config::PluginConfigScope
    ) -> Result<()> {
        self.config_manager.save_plugin_config(plugin_name, config, scope) // Reverted
    }
    
    // Get application configuration
    pub fn get_app_config(&self, name: &str) -> Result<ConfigData> {
        self.config_manager.get_app_config(name) // Reverted
    }
    
    // Save application configuration
    pub fn save_app_config(&self, name: &str, config: &ConfigData) -> Result<()> {
        self.config_manager.save_app_config(name, config) // Reverted
    }
}