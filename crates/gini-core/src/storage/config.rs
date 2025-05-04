use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::cell::RefCell;

use serde::{Serialize, Deserialize};
use serde_json;
#[cfg(feature = "yaml-config")]
use serde_yaml;
#[cfg(feature = "toml-config")]
use toml;

use crate::kernel::error::{Error, Result};
use crate::storage::StorageProvider;

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
    pub fn set<T: Serialize>(&mut self, key: &str, value: T) -> Result<()> {
        match serde_json::to_value(value) {
            Ok(json_value) => {
                self.values.insert(key.to_string(), json_value);
                Ok(())
            },
            Err(e) => Err(Error::Storage(format!("Failed to serialize config value: {}", e))),
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
    pub fn serialize(&self, format: ConfigFormat) -> Result<String> {
        match format {
            ConfigFormat::Json => {
                serde_json::to_string_pretty(&self)
                    .map_err(|e| Error::Storage(format!("Failed to serialize to JSON: {}", e)))
            },
            #[cfg(feature = "yaml-config")]
            ConfigFormat::Yaml => {
                serde_yaml::to_string(&self)
                    .map_err(|e| Error::Storage(format!("Failed to serialize to YAML: {}", e)))
            },
            #[cfg(feature = "toml-config")]
            ConfigFormat::Toml => {
                toml::to_string_pretty(&self)
                    .map_err(|e| Error::Storage(format!("Failed to serialize to TOML: {}", e)))
            },
        }
    }
    
    /// Deserialize from string based on format
    pub fn deserialize(data: &str, format: ConfigFormat) -> Result<Self> {
        match format {
            ConfigFormat::Json => {
                serde_json::from_str(data)
                    .map_err(|e| Error::Storage(format!("Failed to deserialize from JSON: {}", e)))
            },
            #[cfg(feature = "yaml-config")]
            ConfigFormat::Yaml => {
                serde_yaml::from_str(data)
                    .map_err(|e| Error::Storage(format!("Failed to deserialize from YAML: {}", e)))
            },
            #[cfg(feature = "toml-config")]
            ConfigFormat::Toml => {
                toml::from_str(data)
                    .map_err(|e| Error::Storage(format!("Failed to deserialize from TOML: {}", e)))
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
#[derive(Debug, Clone)]
pub struct ConfigManager<P: StorageProvider + ?Sized> {
    /// Storage provider for reading/writing configs
    provider: Arc<P>,
    /// Base path for application configurations
    app_config_path: PathBuf,
    /// Base path for plugin configurations
    plugin_config_path: PathBuf,
    /// Default format for new configurations
    default_format: ConfigFormat,
    /// In-memory cache of loaded configurations
    cache: RefCell<HashMap<String, ConfigData>>,
}

impl<P: StorageProvider + ?Sized + 'static> ConfigManager<P> {
    /// Create a new configuration manager
    pub fn new(
        provider: Arc<P>, 
        app_config_path: PathBuf,
        plugin_config_path: PathBuf,
        default_format: ConfigFormat,
    ) -> Self {
        Self {
            provider,
            app_config_path,
            plugin_config_path,
            default_format,
            cache: RefCell::new(HashMap::new()),
        }
    }
    
    /// Get the app configuration path
    pub fn app_config_path(&self) -> &Path {
        &self.app_config_path
    }
    
    /// Get the plugin configuration path
    pub fn plugin_config_path(&self) -> &Path {
        &self.plugin_config_path
    }
    
    /// Get the default format
    pub fn default_format(&self) -> ConfigFormat {
        self.default_format
    }
    
    /// Set the default format
    pub fn set_default_format(&mut self, format: ConfigFormat) {
        self.default_format = format;
    }
    
    /// Resolve the complete path for a configuration file
    pub fn resolve_config_path(&self, name: &str, scope: ConfigScope) -> PathBuf {
        let base_path = match scope {
            ConfigScope::Application => &self.app_config_path,
            ConfigScope::Plugin(_) => &self.plugin_config_path,
        };
        
        // Ensure name has appropriate extension
        let file_name = if Path::new(name).extension().is_some() {
            name.to_string()
        } else {
            format!("{}.{}", name, self.default_format.extension())
        };
        
        match scope {
            ConfigScope::Application => base_path.join(file_name),
            ConfigScope::Plugin(plugin_scope) => {
                match plugin_scope {
                    PluginConfigScope::Default => base_path.join("default").join(file_name),
                    PluginConfigScope::User => base_path.join("user").join(file_name),
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
        
        // Check if we have this config in cache
        {
            let cache = self.cache.borrow();
            if let Some(config) = cache.get(&cache_key) {
                return Ok(config.clone());
            }
        }
        
        // Not in cache, need to load from disk
        let path = self.resolve_config_path(name, scope);
        
        // If file doesn't exist, return default empty config
        if !self.provider.exists(&path) {
            let empty_config = ConfigData::new();
            
            // Cache the empty config
            self.cache.borrow_mut().insert(cache_key, empty_config.clone());
            
            return Ok(empty_config);
        }
        
        // Determine format from file extension
        let format = ConfigFormat::from_path(&path)
            .ok_or_else(|| Error::Storage(format!("Unknown config format for path: {:?}", path)))?;
        
        // Load the file content
        let content = self.provider.read_to_string(&path)?;
        
        // Parse based on format
        let config = ConfigData::deserialize(&content, format)?;
        
        // Cache the loaded config
        self.cache.borrow_mut().insert(cache_key, config.clone());
        
        Ok(config)
    }
    
    /// Save configuration to disk
    pub fn save_config(&self, name: &str, config: &ConfigData, scope: ConfigScope) -> Result<()> {
        let path = self.resolve_config_path(name, scope);
        
        // Ensure the directory exists
        if let Some(parent) = path.parent() {
            self.provider.create_dir_all(parent)?;
        }
        
        // Determine format from file extension or use default
        let format = ConfigFormat::from_path(&path).unwrap_or(self.default_format);
        
        // Serialize based on format
        let content = config.serialize(format)?;
        
        // Write to disk
        self.provider.write_string(&path, &content)?;
        
        // Update cache
        let cache_key = match scope {
            ConfigScope::Application => format!("app:{}", name),
            ConfigScope::Plugin(PluginConfigScope::Default) => format!("plugin:default:{}", name),
            ConfigScope::Plugin(PluginConfigScope::User) => format!("plugin:user:{}", name),
        };
        
        self.cache.borrow_mut().insert(cache_key, config.clone());
        
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
        
        self.cache.borrow_mut().remove(&cache_key);
    }
    
    /// Clear the entire configuration cache
    pub fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
    }
    
    /// List available configuration files
    pub fn list_configs(&self, scope: ConfigScope) -> Result<Vec<String>> {
        // Get the appropriate directory path
        let dir_path = match scope {
            ConfigScope::Application => self.app_config_path.clone(),
            ConfigScope::Plugin(plugin_scope) => match plugin_scope {
                PluginConfigScope::Default => self.plugin_config_path.join("default"),
                PluginConfigScope::User => self.plugin_config_path.join("user"),
            },
        };
        
        // Ensure directory exists
        if !self.provider.exists(&dir_path) {
            return Ok(vec![]);
        }
        
        // List files in directory
        let entries = self.provider.read_dir(&dir_path)?;
        
        // Filter for configuration files
        let config_files = entries.into_iter()
            .filter_map(|path| {
                // Check if it's a file
                if self.provider.is_file(&path) {
                    // Check if it has a recognized extension
                    if ConfigFormat::from_path(&path).is_some() {
                        // Extract the file name without extension
                        path.file_stem().and_then(|stem| stem.to_str().map(String::from))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        
        Ok(config_files)
    }
}

/// Extension trait for StorageManager to provide configuration capabilities
pub trait ConfigStorageExt: StorageProvider where Self: 'static {
    /// Get the configuration manager
    fn config_manager<P: StorageProvider + ?Sized>(&self) -> &ConfigManager<P>;
}