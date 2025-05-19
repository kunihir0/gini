use std::collections::HashMap;
use std::fmt; // Add fmt import
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock}; // Replace RefCell with RwLock

use serde::{Serialize, Deserialize};
use serde_json;
#[cfg(feature = "yaml-config")]
use serde_yaml;
#[cfg(feature = "toml-config")]
use toml;

use crate::kernel::error::Error as KernelError; // Renamed for clarity
use std::result::Result as StdResult; // Import StdResult
use crate::storage::error::StorageSystemError; // Import StorageSystemError
// use crate::storage::manager::StorageManager; // Import StorageManager trait
use crate::storage::StorageProvider; // Keep for direct provider access if needed elsewhere

/// Type alias for Result with KernelError for public ConfigManager methods
type Result<T> = StdResult<T, KernelError>;
/// Type alias for Result with StorageSystemError for internal operations
type StorageResult<T> = StdResult<T, StorageSystemError>;


/// Defines the scope for storage operations, determining the base directory.
#[derive(Debug, Clone, PartialEq)]
pub enum StorageScope {
    /// Application-specific configuration files (e.g., `$XDG_CONFIG_HOME/gini`).
    Application,
    /// Plugin-specific configuration files (e.g., `$XDG_CONFIG_HOME/gini/plugins/<plugin_name>`).
    Plugin { plugin_name: String },
    /// Application data files (e.g., `$XDG_DATA_HOME/gini`).
    Data,
    // Potentially add Cache, Logs, etc. later
}


/// Supported configuration file formats
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigFormat {
    /// JSON format (.json)
    Json,
    /// YAML format (.yaml, .yml) - requires "yaml-config" feature
    #[cfg(feature = "yaml-config")]
    Yaml,
    /// TOML format (.toml) - requires "toml-config" feature
    #[cfg(feature = "toml-config")]
    Toml,
}

impl ConfigFormat {
    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            ConfigFormat::Json => "json",
            #[cfg(feature = "yaml-config")]
            ConfigFormat::Yaml => "yaml",
            #[cfg(feature = "toml-config")]
            ConfigFormat::Toml => "toml",
        }
    }
    
    /// Determine format from file extension
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext.to_lowercase().as_str() {
                "json" => Some(ConfigFormat::Json),
                #[cfg(feature = "yaml-config")]
                "yaml" | "yml" => Some(ConfigFormat::Yaml),
                #[cfg(feature = "toml-config")]
                "toml" => Some(ConfigFormat::Toml),
                _ => None,
            })
    }
}

/// In-memory representation of configuration data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigData {
    /// Raw configuration values
    #[serde(flatten)]
    values: HashMap<String, serde_json::Value>,
}

impl ConfigData {
    /// Create a new empty configuration
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }
    
    /// Create a configuration from a HashMap
    pub fn from_hashmap(values: HashMap<String, serde_json::Value>) -> Self {
        Self { values }
    }
    
    /// Get a configuration value
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.values.get(key)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }
    
    /// Get a configuration value with default
    pub fn get_or<T: for<'de> Deserialize<'de>>(&self, key: &str, default: T) -> T {
        self.get(key).unwrap_or(default)
    }
    
    /// Set a configuration value
    pub fn set<T: Serialize>(&mut self, key: &str, value: T) -> StorageResult<()> { // Changed to StorageResult
        match serde_json::to_value(value) {
            Ok(json_value) => {
                self.values.insert(key.to_string(), json_value);
                Ok(())
            },
            Err(e) => Err(StorageSystemError::SerializationError { // Changed to StorageSystemError
                format: "JSON".to_string(),
                source: Box::new(e),
            }),
        }
    }
    
    /// Remove a configuration value
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.values.remove(key)
    }
    
    /// Check if key exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }
    
    /// Get all keys
    pub fn keys(&self) -> Vec<String> {
        self.values.keys().cloned().collect()
    }
    
    /// Merge with another config, overriding existing values
    pub fn merge(&mut self, other: &ConfigData) {
        for (key, value) in &other.values {
            self.values.insert(key.clone(), value.clone());
        }
    }
    
    /// Serialize to string based on format
    pub fn serialize(&self, format: ConfigFormat) -> StorageResult<String> { // Changed to StorageResult
        match format {
            ConfigFormat::Json => {
                serde_json::to_string_pretty(&self).map_err(|e| StorageSystemError::SerializationError { // Changed to StorageSystemError
                    format: "JSON".to_string(),
                    source: Box::new(e),
                })
            },
            #[cfg(feature = "yaml-config")]
            ConfigFormat::Yaml => {
                serde_yaml::to_string(&self).map_err(|e| StorageSystemError::SerializationError { // Changed to StorageSystemError
                    format: "YAML".to_string(),
                    source: Box::new(e),
                })
            },
            #[cfg(feature = "toml-config")]
            ConfigFormat::Toml => {
                // Filter out serde_json::Value::Null before serializing to TOML,
                // as TOML does not support null values directly (None is represented by absence).
                let filtered_values: HashMap<String, serde_json::Value> = self.values.iter()
                    .filter(|(_, v)| !v.is_null()) // Filter out null values
                    .map(|(k, v)| (k.clone(), v.clone())) // Clone to create owned values for the new map
                    .collect();
                
                // If after filtering, all values were null and the map is empty,
                // toml::to_string_pretty on an empty map produces an empty string, which is valid TOML.
                // If there are non-null values, they will be serialized.
                toml::to_string_pretty(&filtered_values).map_err(|e| StorageSystemError::SerializationError {
                    format: "TOML".to_string(),
                    source: Box::new(e),
                })
            },
        }
    }
    
    /// Deserialize from string based on format
    pub fn deserialize(data: &str, format: ConfigFormat) -> StorageResult<Self> { // Changed to StorageResult
        match format {
            ConfigFormat::Json => {
                serde_json::from_str(data).map_err(|e| StorageSystemError::DeserializationError { // Changed to StorageSystemError
                    format: "JSON".to_string(),
                    source: Box::new(e),
                })
            },
            #[cfg(feature = "yaml-config")]
            ConfigFormat::Yaml => {
                serde_yaml::from_str(data).map_err(|e| StorageSystemError::DeserializationError { // Changed to StorageSystemError
                    format: "YAML".to_string(),
                    source: Box::new(e),
                })
            },
            #[cfg(feature = "toml-config")]
            ConfigFormat::Toml => {
                // Note: toml::Value doesn't directly support flatten, so we deserialize into the inner map
                let values: HashMap<String, toml::Value> = toml::from_str(data).map_err(|e| StorageSystemError::DeserializationError { // Changed to StorageSystemError
                    format: "TOML".to_string(),
                    source: Box::new(e),
                })?;
                // Convert toml::Value to serde_json::Value
                let json_values = values.into_iter().map(|(k, v)| {
                    // Consider if this unwrap_or could mask an important error.
                    // If toml::Value -> serde_json::Value conversion is critical,
                    // this might need to return a Result and be propagated.
                    (k, serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
                }).collect();
                Ok(ConfigData { values: json_values })
            },
        }
    }
}

impl Default for ConfigData {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration scope determines where configuration is stored
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigScope {
    /// Global application configuration
    Application,
    /// Plugin-specific configuration
    Plugin(PluginConfigScope),
}

/// Plugin configuration scope determines which plugin configuration to access
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PluginConfigScope {
    /// Default plugin configuration (shared across all plugins)
    Default,
    /// User-specific plugin configuration (overrides default)
    User,
}

/// Configuration manager that handles loading, saving, and caching configurations
// #[derive(Debug)] // Manual Debug implementation below
pub struct ConfigManager {
    // Store provider and paths directly again
    provider: Arc<dyn StorageProvider>,
    app_config_path: PathBuf,
    plugin_config_path: PathBuf,
    /// Default format for new configurations
    default_format: Arc<RwLock<ConfigFormat>>, // Use Arc<RwLock> for shared mutability
    /// In-memory cache of loaded configurations (Thread-safe and Cloneable via Arc)
    cache: Arc<RwLock<HashMap<String, ConfigData>>>,
}

impl ConfigManager {
    /// Create a new configuration manager
    // Revert to taking provider and paths, store them directly.
    // StorageManager will be accessed via methods when needed.
    pub fn new(
        provider: Arc<dyn StorageProvider>, // Use dynamic dispatch provider
        app_config_path: PathBuf,           // App config base path
        plugin_config_path: PathBuf,        // Plugin config base path
        default_format: ConfigFormat,
    ) -> Self {
        Self {
            // storage_manager: storage_manager, // Removed field
            provider, // Store provider directly
            app_config_path, // Store path directly
            plugin_config_path, // Store path directly
            default_format: Arc::new(RwLock::new(default_format)),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the underlying storage provider Arc.
    pub fn provider(&self) -> &Arc<dyn StorageProvider> {
        &self.provider
    }

    // Removed storage_manager() getter, access provider directly if needed
    // pub fn storage_manager(&self) -> &Arc<dyn StorageManager> {
    //     &self.storage_manager
    // }

    // Removed app_config_path() and plugin_config_path() methods

    /// Get the default format
    pub fn default_format(&self) -> ConfigFormat {
        *self.default_format.read().unwrap() // Read from RwLock
    }
    
    /// Set the default format
    pub fn set_default_format(&self, format: ConfigFormat) { // Takes &self now
        *self.default_format.write().unwrap() = format; // Write to RwLock
    }
    
    /// Resolve the complete path for a configuration file
    /// Resolve the complete path for a configuration file using stored paths
    pub fn resolve_config_path(&self, name: &str, scope: ConfigScope) -> PathBuf {
         // Ensure name has appropriate extension
        let file_name = if Path::new(name).extension().is_some() {
            name.to_string()
        } else {
            format!("{}.{}", name, self.default_format().extension()) // Use method to read lock
        };

        match scope {
            ConfigScope::Application => self.app_config_path.join(file_name),
            ConfigScope::Plugin(plugin_scope) => {
                // Use the stored plugin_config_path
                match plugin_scope {
                    // Note: This reverts to the original structure: <plugin_config_path>/<scope>/<name>.<ext>
                    // Example: <root>/plugins/config/default/my_plugin.json
                    // If the XDG structure (<config_dir>/plugins/<plugin_name>/<scope>/config.json) is desired,
                    // this needs adjustment, likely requiring the StorageManager reference again.
                    PluginConfigScope::Default => self.plugin_config_path.join("default").join(file_name),
                    PluginConfigScope::User => self.plugin_config_path.join("user").join(file_name),
                }
            },
        }
    }

    /// Load configuration from disk
    pub fn load_config(&self, name: &str, scope: ConfigScope) -> Result<ConfigData> {
        // Generate a cache key to identify this configuration
        let cache_key = match scope {
            ConfigScope::Application => format!("app:{}", name),
            ConfigScope::Plugin(PluginConfigScope::Default) => format!("plugin:default:{}", name),
            ConfigScope::Plugin(PluginConfigScope::User) => format!("plugin:user:{}", name),
        };
        
        // Check if we have this config in cache (read lock)
        {
            let cache = self.cache.read().unwrap(); // Use read lock
            if let Some(config) = cache.get(&cache_key) {
                return Ok(config.clone());
            }
        }
        
        // Not in cache, need to load from disk
        let path = self.resolve_config_path(name, scope);
        
        // If file doesn't exist, return default empty config (no error)
        // Use the stored provider for checks
        if !self.provider.exists(&path) {
             // Check if the *directory* exists before returning empty.
             // If the dir doesn't exist, it might indicate a setup issue.
             if let Some(parent_dir) = path.parent() {
                 if !self.provider.is_dir(parent_dir) {
                     // Optionally, log a warning here or return a specific error?
                     // For now, return empty config as per original logic.
                     // Let's create it to be safe, as before.
                     self.provider.create_dir_all(parent_dir).map_err(KernelError::from)?;
                 }
             }
 
             let empty_config = ConfigData::new();
 
             // Cache the empty config (write lock)
             self.cache.write().unwrap().insert(cache_key, empty_config.clone()); // Use write lock
 
             return Ok(empty_config);
         }
 
         // Ensure it's actually a file before trying to read
         if !self.provider.is_file(&path) {
             return Err(KernelError::from(StorageSystemError::OperationFailed {
                 operation: "load_config".to_string(),
                 path: Some(path.clone()),
                 message: format!("Path exists but is not a file: {}", path.display()),
             }));
         }
 
         // Determine format from file extension
         let format = ConfigFormat::from_path(&path)
             .ok_or_else(|| KernelError::from(StorageSystemError::UnsupportedConfigFormat(path.to_string_lossy().into_owned())))?;
 
         // Load the file content using the stored provider
         let content = self.provider.read_to_string(&path).map_err(KernelError::from)?;
 
         // Parse based on format
         let config = ConfigData::deserialize(&content, format).map_err(KernelError::from)?;
         
         // Cache the loaded config (write lock)
         self.cache.write().unwrap().insert(cache_key, config.clone()); // Use write lock

         Ok(config)
     }
     
     /// Save configuration to disk
     pub fn save_config(&self, name: &str, config: &ConfigData, scope: ConfigScope) -> Result<()> {
         let path = self.resolve_config_path(name, scope);
         
         // Ensure the directory exists using the stored provider
         if let Some(parent) = path.parent() {
             self.provider.create_dir_all(parent).map_err(KernelError::from)?;
         }
 
         // Determine format from file extension or use default
         let format = ConfigFormat::from_path(&path).unwrap_or(self.default_format());
 
         // Serialize based on format
         let content = config.serialize(format).map_err(KernelError::from)?;
 
         // Write to disk using the stored provider
         self.provider.write_string(&path, &content).map_err(KernelError::from)?;
 
         // Update cache
        let cache_key = match scope {
            ConfigScope::Application => format!("app:{}", name),
            ConfigScope::Plugin(PluginConfigScope::Default) => format!("plugin:default:{}", name),
            ConfigScope::Plugin(PluginConfigScope::User) => format!("plugin:user:{}", name),
        };

        self.cache.write().unwrap().insert(cache_key, config.clone()); // Use write lock

        Ok(())
    }
    
    /// Get plugin configuration with user override support
    /// This will first load the user-specific configuration, then fall back to defaults
    pub fn get_plugin_config(&self, plugin_name: &str) -> Result<ConfigData> {
        // First try to load user-specific configuration
        let user_config = self.load_config(plugin_name, ConfigScope::Plugin(PluginConfigScope::User))?;
        
        // Then load default configuration
        let default_config = self.load_config(plugin_name, ConfigScope::Plugin(PluginConfigScope::Default))?;
        
        // Merge them with user config taking priority
        let mut merged_config = default_config;
        merged_config.merge(&user_config);
        
        Ok(merged_config)
    }
    
    /// Save plugin-specific configuration
    pub fn save_plugin_config(
        &self, 
        plugin_name: &str, 
        config: &ConfigData, 
        scope: PluginConfigScope
    ) -> Result<()> {
        self.save_config(plugin_name, config, ConfigScope::Plugin(scope))
    }
    
    /// Get application configuration
    pub fn get_app_config(&self, name: &str) -> Result<ConfigData> {
        self.load_config(name, ConfigScope::Application)
    }
    
    /// Save application configuration
    pub fn save_app_config(&self, name: &str, config: &ConfigData) -> Result<()> {
        self.save_config(name, config, ConfigScope::Application)
    }
    
    /// Invalidate the cache for a specific configuration
    pub fn invalidate_cache(&self, name: &str, scope: ConfigScope) {
        let cache_key = match scope {
            ConfigScope::Application => format!("app:{}", name),
            ConfigScope::Plugin(PluginConfigScope::Default) => format!("plugin:default:{}", name),
            ConfigScope::Plugin(PluginConfigScope::User) => format!("plugin:user:{}", name),
        };

        self.cache.write().unwrap().remove(&cache_key); // Use write lock
    }

    /// Clear the entire configuration cache
    pub fn clear_cache(&self) {
        self.cache.write().unwrap().clear(); // Use write lock
    }

    /// List available configuration files
    pub fn list_configs(&self, scope: ConfigScope) -> Result<Vec<String>> {
        // Get the appropriate directory path using stored paths
        let dir_path = match scope {
            ConfigScope::Application => self.app_config_path.clone(),
            ConfigScope::Plugin(plugin_scope) => match plugin_scope {
                PluginConfigScope::Default => self.plugin_config_path.join("default"),
                PluginConfigScope::User => self.plugin_config_path.join("user"),
            },
        };

        // Ensure directory exists using stored provider
        if !self.provider.exists(&dir_path) {
            // Attempt to create the directory if it doesn't exist? Or just return empty?
            // Let's create it for consistency with load/save.
            self.provider.create_dir_all(&dir_path).map_err(KernelError::from)?;
            return Ok(vec![]); // Return empty if just created
        }
        if !self.provider.is_dir(&dir_path) {
            return Err(KernelError::from(StorageSystemError::OperationFailed {
                operation: "list_configs".to_string(),
                path: Some(dir_path.clone()),
                message: "Path exists but is not a directory".to_string(),
            }));
        }
        println!("[list_configs] Listing in directory: {:?}", dir_path); // DEBUG

        // List files in directory using stored provider
        let entries = self.provider.read_dir(&dir_path).map_err(KernelError::from)?;
        println!("[list_configs] Found entries: {:?}", entries); // DEBUG

        // Filter for configuration files
        let config_files = entries.into_iter()
            .filter_map(|path| {
                // Check if it's a file using stored provider
                if self.provider.is_file(&path) {
                    // Check if it has a recognized extension
                    if ConfigFormat::from_path(&path).is_some() {
                        // Extract the file name without extension
                        path.file_stem().and_then(|stem| stem.to_str().map(String::from))
                    } else {
                        None // Not a recognized config format
                    }
                } else {
                    None // Not a file
                }
            })
            .collect();
        
        Ok(config_files)
    }
}

// Manual Debug implementation for ConfigManager
// Update Debug impl to use stored provider/paths
impl fmt::Debug for ConfigManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConfigManager")
            .field("provider", &self.provider.name()) // Use provider name
            .field("app_config_path", &self.app_config_path)
            .field("plugin_config_path", &self.plugin_config_path)
            .field("default_format", &*self.default_format.read().unwrap()) // Read the format
            .field("cache", &format!("{} items", self.cache.read().unwrap().len()))
            .finish()
    }
}

/// Extension trait for StorageManager to provide configuration capabilities
pub trait ConfigStorageExt { // Remove StorageProvider bound and 'static
    /// Get the configuration manager
    fn config_manager(&self) -> &ConfigManager; // Remove generic P
}

// Manual Clone implementation for ConfigManager
// Update Clone impl
impl Clone for ConfigManager {
    fn clone(&self) -> Self {
        Self {
            provider: Arc::clone(&self.provider), // Clone provider Arc
            app_config_path: self.app_config_path.clone(),
            plugin_config_path: self.plugin_config_path.clone(),
            default_format: Arc::clone(&self.default_format),
            cache: Arc::clone(&self.cache),
        }
    }
}