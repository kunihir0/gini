use std::env;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use async_trait::async_trait;

use crate::kernel::component::KernelComponent;
use crate::kernel::error::Result; // Add Error import
use crate::storage::provider::StorageProvider;
use crate::storage::local::LocalStorageProvider; // Default provider
use crate::storage::config::{ConfigManager, ConfigFormat, ConfigData, ConfigStorageExt, StorageScope}; // Add StorageScope

/// Storage manager component interface
/// This simply wraps a StorageProvider for now
#[async_trait]
pub trait StorageManager: KernelComponent + StorageProvider {
    /// Returns the primary configuration directory for the application.
    fn config_dir(&self) -> &Path;
    /// Returns the primary data directory for the application.
    fn data_dir(&self) -> &Path;
    /// Resolves a path relative to a specific storage scope.
    fn resolve_path(&self, scope: StorageScope, relative_path: &Path) -> PathBuf;
}

/// Default implementation of StorageManager
#[derive(Clone)] // Add Clone derive
pub struct DefaultStorageManager {
    name: &'static str,
    provider: Arc<dyn StorageProvider>, // Holds the actual provider
    config_manager: Arc<ConfigManager>, // Use the non-generic ConfigManager
    config_dir: PathBuf, // XDG Config directory ($XDG_CONFIG_HOME/gini or $HOME/.config/gini)
    data_dir: PathBuf,   // XDG Data directory ($XDG_DATA_HOME/gini or $HOME/.local/share/gini)
}

// Helper function to get the base XDG config directory
fn get_xdg_config_base() -> Result<PathBuf> {
    match env::var("XDG_CONFIG_HOME") {
        Ok(val) if !val.is_empty() => Ok(PathBuf::from(val)),
        _ => match env::var("HOME") { // Use env::var("HOME") instead of home_dir()
            Ok(home) if !home.is_empty() => Ok(PathBuf::from(home).join(".config")),
            _ => {
                eprintln!("Warning: Could not determine home directory via $HOME. Falling back to CWD for config.");
                // Use a relative path as fallback, ensure it exists later
                Ok(PathBuf::from("./.gini/config"))
            }
        },
    }
}

// Helper function to get the base XDG data directory
fn get_xdg_data_base() -> Result<PathBuf> {
    match env::var("XDG_DATA_HOME") {
        Ok(val) if !val.is_empty() => Ok(PathBuf::from(val)),
        _ => match env::var("HOME") { // Use env::var("HOME") instead of home_dir()
            Ok(home) if !home.is_empty() => Ok(PathBuf::from(home).join(".local/share")),
            _ => {
                eprintln!("Warning: Could not determine home directory via $HOME. Falling back to CWD for data.");
                // Use a relative path as fallback, ensure it exists later
                Ok(PathBuf::from("./.gini/data"))
            }
        },
    }
}


impl DefaultStorageManager {
    /// Create a new default storage manager using XDG directories.
    pub fn new() -> Result<Self> { // Return Result for potential errors
        // Determine XDG paths
        let config_dir = get_xdg_config_base()?.join("gini");
        let data_dir = get_xdg_data_base()?.join("gini");

        // Define paths for ConfigManager
        let app_config_path = config_dir.clone(); // App config goes directly in config_dir
        let plugin_config_path = config_dir.join("plugins"); // Plugin config in config_dir/plugins

        // Create the provider - Use "." as base, paths will be absolute XDG paths
        // The provider needs to handle absolute paths correctly.
        let provider = Arc::new(LocalStorageProvider::new(PathBuf::from("."))) as Arc<dyn StorageProvider>;

        // Create the config manager using the SAME provider
        let config_manager = ConfigManager::new(
            Arc::clone(&provider), // Use the same provider instance
            app_config_path,       // Pass the determined config path
            plugin_config_path,    // Pass the determined plugin config path
            ConfigFormat::Json,    // Default to JSON format
        );

        Ok(Self {
            name: "DefaultStorageManager",
            provider,
            config_manager: Arc::new(config_manager), // Wrap in Arc
            config_dir,
            data_dir,
        })
    }

    // Removed `with_provider` as it's incompatible with XDG logic

    /// Get the underlying provider
    pub fn provider(&self) -> &Arc<dyn StorageProvider> {
        &self.provider
    }

    /// Get the configuration manager instance
    pub fn get_config_manager(&self) -> &Arc<ConfigManager> { // Update return type
        &self.config_manager
    }

    /// Get the application configuration directory path
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Get the application data directory path
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Resolves a path relative to a specific storage scope.
    pub fn resolve_path(&self, scope: StorageScope, relative_path: &Path) -> PathBuf {
         match scope {
            StorageScope::Application => self.config_dir.join(relative_path),
            StorageScope::Plugin { plugin_name } => self.config_dir.join("plugins").join(plugin_name).join(relative_path),
            StorageScope::Data => self.data_dir.join(relative_path),
            // Add other scopes if necessary, e.g., Cache
        }
    }


    /// Ensure all required base directories exist
    pub fn ensure_directories(&self) -> Result<()> {
        // Ensure base config directory exists
        self.create_dir_all(&self.config_dir)?;

        // Ensure base plugin config directory exists (individual plugin dirs created by ConfigManager)
        self.create_dir_all(&self.config_dir.join("plugins"))?;

        // Ensure base data directory exists
        self.create_dir_all(&self.data_dir)?;

        Ok(())
    }
}

impl Debug for DefaultStorageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultStorageManager")
            .field("name", &self.name)
            .field("provider", &self.provider.name()) // Show provider name
            .field("config_dir", &self.config_dir)
            .field("data_dir", &self.data_dir)
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

    // These methods now correctly return Result<T, StorageSystemError>
    // as per the StorageProvider trait. The conversion to KernelError
    // will happen at call sites if DefaultStorageManager is used
    // where a KernelError is expected, thanks to From<StorageSystemError> for KernelError.
    fn create_dir(&self, path: &Path) -> std::result::Result<(), crate::storage::error::StorageSystemError> {
        self.provider.create_dir(path)
    }

    fn create_dir_all(&self, path: &Path) -> std::result::Result<(), crate::storage::error::StorageSystemError> {
        self.provider.create_dir_all(path)
    }

    fn read_to_string(&self, path: &Path) -> std::result::Result<String, crate::storage::error::StorageSystemError> {
        self.provider.read_to_string(path)
    }

    fn read_to_bytes(&self, path: &Path) -> std::result::Result<Vec<u8>, crate::storage::error::StorageSystemError> {
        self.provider.read_to_bytes(path)
    }

    fn write_string(&self, path: &Path, contents: &str) -> std::result::Result<(), crate::storage::error::StorageSystemError> {
        self.provider.write_string(path, contents)
    }

    fn write_bytes(&self, path: &Path, contents: &[u8]) -> std::result::Result<(), crate::storage::error::StorageSystemError> {
        self.provider.write_bytes(path, contents)
    }

    fn copy(&self, from: &Path, to: &Path) -> std::result::Result<(), crate::storage::error::StorageSystemError> {
        self.provider.copy(from, to)
    }

    fn rename(&self, from: &Path, to: &Path) -> std::result::Result<(), crate::storage::error::StorageSystemError> {
        self.provider.rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> std::result::Result<(), crate::storage::error::StorageSystemError> {
        self.provider.remove_file(path)
    }

    fn remove_dir(&self, path: &Path) -> std::result::Result<(), crate::storage::error::StorageSystemError> {
        self.provider.remove_dir(path)
    }

    fn remove_dir_all(&self, path: &Path) -> std::result::Result<(), crate::storage::error::StorageSystemError> {
        self.provider.remove_dir_all(path)
    }

    fn read_dir(&self, path: &Path) -> std::result::Result<Vec<PathBuf>, crate::storage::error::StorageSystemError> {
        self.provider.read_dir(path)
    }

    fn metadata(&self, path: &Path) -> std::result::Result<std::fs::Metadata, crate::storage::error::StorageSystemError> {
        self.provider.metadata(path)
    }

    fn open_read(&self, path: &Path) -> std::result::Result<Box<dyn std::io::Read>, crate::storage::error::StorageSystemError> {
        self.provider.open_read(path)
    }

    fn open_write(&self, path: &Path) -> std::result::Result<Box<dyn std::io::Write>, crate::storage::error::StorageSystemError> {
        self.provider.open_write(path)
    }

    fn open_append(&self, path: &Path) -> std::result::Result<Box<dyn std::io::Write>, crate::storage::error::StorageSystemError> {
        self.provider.open_append(path)
    }
}

// Implement the extended StorageManager trait
#[async_trait] // Keep async_trait if other methods become async later
impl StorageManager for DefaultStorageManager {
    fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    fn resolve_path(&self, scope: StorageScope, relative_path: &Path) -> PathBuf {
        match scope {
            StorageScope::Application => self.config_dir.join(relative_path),
            StorageScope::Plugin { plugin_name } => self.config_dir.join("plugins").join(plugin_name).join(relative_path),
            StorageScope::Data => self.data_dir.join(relative_path),
            // Add other scopes if necessary, e.g., Cache
        }
    }
}


// Default implementation using XDG paths
impl Default for DefaultStorageManager {
    fn default() -> Self {
        // Attempt to create using XDG paths, panic on failure for Default trait
        // In real application setup, handle the Result properly.
        Self::new().expect("Failed to initialize default storage manager")
    }
}

// Implement ConfigStorageExt trait for DefaultStorageManager
impl ConfigStorageExt for DefaultStorageManager {
    fn config_manager(&self) -> &ConfigManager { // Remove generic P, return correct type
        // Simply return a reference to the config_manager field
        &self.config_manager
    }
}

// Implement additional configuration methods directly on DefaultStorageManager for convenience
// These methods delegate to ConfigManager, which uses the StorageManager's resolve_path.
// No changes needed here as long as ConfigManager is updated correctly.
impl DefaultStorageManager {
    // Make load_config method directly accessible from DefaultStorageManager
    pub fn load_config(&self, name: &str, scope: crate::storage::config::ConfigScope) -> Result<ConfigData> {
        self.config_manager.load_config(name, scope)
    }

    // Make list_configs method directly accessible from DefaultStorageManager
    pub fn list_configs(&self, scope: crate::storage::config::ConfigScope) -> Result<Vec<String>> {
        self.config_manager.list_configs(scope)
    }

    // Get plugin configuration with user override support
    pub fn get_plugin_config(&self, plugin_name: &str) -> Result<ConfigData> {
        self.config_manager.get_plugin_config(plugin_name)
    }

    // Save plugin-specific configuration
    pub fn save_plugin_config(
        &self,
        plugin_name: &str,
        config: &ConfigData,
        scope: crate::storage::config::PluginConfigScope
    ) -> Result<()> {
        self.config_manager.save_plugin_config(plugin_name, config, scope)
    }

    // Get application configuration
    pub fn get_app_config(&self, name: &str) -> Result<ConfigData> {
        self.config_manager.get_app_config(name)
    }

    // Save application configuration
    pub fn save_app_config(&self, name: &str, config: &ConfigData) -> Result<()> {
        self.config_manager.save_app_config(name, config)
    }
}