#![cfg(test)]

use tokio::test;
use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use std::time::SystemTime;

use crate::kernel::bootstrap::Application;
use crate::kernel::component::KernelComponent;
use crate::kernel::error::{Error, Result as KernelResult};
use crate::storage::DefaultStorageManager;
use crate::plugin_system::dependency::PluginDependency;
use crate::plugin_system::traits::{Plugin, PluginPriority, PluginError};
use crate::plugin_system::version::VersionRange;
use crate::stage_manager::{Stage, StageContext};
use crate::stage_manager::requirement::StageRequirement;
use crate::storage::config::{ConfigData, PluginConfigScope, ConfigStorageExt};

use super::common::setup_test_environment;

/// A plugin that uses configuration management
struct ConfigUsingPlugin {
    name: String,
}

impl ConfigUsingPlugin {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait]
impl Plugin for ConfigUsingPlugin {
    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }
    fn version(&self) -> &str { "1.0.0" }
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(100) }
    fn compatible_api_versions(&self) -> Vec<VersionRange> { vec![">=0.1.0".parse().unwrap()] }
    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }
    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }

    fn init(&self, app: &mut Application) -> KernelResult<()> {
        println!("Plugin {} initializing with config", self.name());
        let storage_manager = app.storage_manager();
        let config = match storage_manager.get_plugin_config(self.name()) {
            Ok(config) => config,
            Err(_) => {
                let mut default_config = ConfigData::new();
                default_config.set("plugin_name", self.name())?;
                default_config.set("initialized", true)?;
                default_config.set("setting1", "default_value")?;
                storage_manager.save_plugin_config(
                    self.name(),
                    &default_config,
                    PluginConfigScope::Default
                )?;
                default_config
            }
        };
        println!("Plugin {} loaded config: {:?}", self.name(), config);
        let initialized = config.get::<bool>("initialized").unwrap_or(false);
        if !initialized {
            return Err(Error::Plugin(format!(
                "Plugin {} not properly initialized", self.name()
            )));
        }
        let mut updated_config = config.clone();
        updated_config.set("last_used", SystemTime::now().elapsed().unwrap_or_default().as_secs())?;
        storage_manager.save_plugin_config(
            self.name(),
            &updated_config,
            PluginConfigScope::User
        )?;
        Ok(())
    }

    async fn preflight_check(&self, _context: &StageContext) -> Result<(), PluginError> {
        Ok(())
    }

    fn stages(&self) -> Vec<Box<dyn Stage>> {
        vec![]
    }

    fn shutdown(&self) -> KernelResult<()> {
        Ok(())
    }

    // Add default implementations for new trait methods
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
}

// SystemTime already imported above

#[test]
async fn test_plugin_configuration_management() {
    // Setup test environment - we don't need the storage_manager from here directly anymore
    let (plugin_manager, _stage_manager, _storage_manager_ignored, _, _, _) = setup_test_environment().await;

    // Create application instance - this will initialize its own storage manager
    let mut app = Application::new(Some(std::env::temp_dir().join("gini_test_config")))
        .expect("Failed to create application");
    
    // Create and register plugin
    let plugin = ConfigUsingPlugin::new("ConfigPlugin");
    let plugin_name = plugin.name().to_string();
    
    // Register the plugin
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(plugin))
            .expect("Failed to register config plugin");
    }

    // Get the storage manager FROM THE APPLICATION instance
    let app_storage_manager = app.storage_manager();

    // Create default plugin configuration
    let mut default_config = ConfigData::new();
    default_config.set("plugin_name", "ConfigPlugin").unwrap();
    default_config.set("initialized", true).unwrap();
    default_config.set("setting1", "default_value").unwrap();
    default_config.set("setting2", "another_value").unwrap();

    // Save default configuration USING THE APP'S STORAGE MANAGER
    app_storage_manager.save_plugin_config(
        &plugin_name,
        &default_config,
        PluginConfigScope::Default
    ).expect("Failed to save default config");

    // Initialize the plugin
    {
        let registry = plugin_manager.registry();
        let mut reg_lock = registry.lock().await;
        reg_lock.initialize_plugin(&plugin_name, &mut app)
            .expect("Failed to initialize plugin");
    }
    
    // Verify the plugin created a user configuration
    let scope = crate::storage::config::ConfigScope::Plugin(PluginConfigScope::User);
    let user_config = app_storage_manager.load_config(&plugin_name, scope)
        .expect("Failed to load user config");

    // Check that user config has last_used field (added by plugin)
    assert!(user_config.contains_key("last_used"));

    // Check that merged config has both default and user values
    let merged_config = app_storage_manager.get_plugin_config(&plugin_name)
        .expect("Failed to get plugin config");

    assert_eq!(merged_config.get::<String>("plugin_name").unwrap(), "ConfigPlugin");
    assert!(merged_config.get::<bool>("initialized").unwrap());
    assert_eq!(merged_config.get::<String>("setting1").unwrap(), "default_value");
    assert_eq!(merged_config.get::<String>("setting2").unwrap(), "another_value");
    assert!(merged_config.contains_key("last_used"));
}

#[test]
async fn test_app_configuration_management() {
    // Setup test environment
    let (_, _, storage_manager, _, _, _) = setup_test_environment().await;
    
    // Initialize storage manager (creates necessary directories)
    KernelComponent::initialize(&*storage_manager).await.expect("Failed to initialize storage manager");
    
    // Create app configuration
    let mut app_config = ConfigData::new();
    app_config.set("app_name", "Gini Test").unwrap();
    app_config.set("version", "1.0.0").unwrap();
    app_config.set("max_plugins", 100).unwrap();
    app_config.set("debug_mode", true).unwrap();
    
    // Save app configuration
    storage_manager.save_app_config("app_settings", &app_config)
        .expect("Failed to save app config");
    
    // Load app configuration
    let loaded_config = storage_manager.get_app_config("app_settings")
        .expect("Failed to load app config");
    
    // Verify values
    assert_eq!(loaded_config.get::<String>("app_name").unwrap(), "Gini Test");
    assert_eq!(loaded_config.get::<String>("version").unwrap(), "1.0.0");
    assert_eq!(loaded_config.get::<i32>("max_plugins").unwrap(), 100);
    assert!(loaded_config.get::<bool>("debug_mode").unwrap());
    
    // Update configuration
    let mut updated_config = loaded_config;
    updated_config.set("debug_mode", false).unwrap();
    updated_config.set("new_setting", "value").unwrap();
    
    // Save updated configuration
    storage_manager.save_app_config("app_settings", &updated_config)
        .expect("Failed to save updated app config");
    
    // Reload configuration
    let reloaded_config = storage_manager.get_app_config("app_settings")
        .expect("Failed to reload app config");
    
    // Verify updated values
    assert_eq!(reloaded_config.get::<String>("app_name").unwrap(), "Gini Test");
    assert!(!reloaded_config.get::<bool>("debug_mode").unwrap());
    assert_eq!(reloaded_config.get::<String>("new_setting").unwrap(), "value");
}

#[test]
#[cfg(feature = "toml-config")] // Only run if toml-config feature is enabled
async fn test_app_configuration_toml_format() {
    // Setup test environment
    let (_, _, mut storage_manager, _, _, _) = setup_test_environment().await;

    // Initialize storage manager (creates necessary directories)
    KernelComponent::initialize(&*storage_manager).await.expect("Failed to initialize storage manager");

    // Explicitly set default format to TOML
    // Get the inner ConfigManager
    let config_manager = storage_manager.get_config_manager();

    // Explicitly set default format to TOML
    config_manager.set_default_format(crate::storage::config::ConfigFormat::Toml);

    // Create app configuration
    let mut app_config = ConfigData::new();
    app_config.set("app_name", "Gini Test TOML").unwrap();
    app_config.set("version", "1.0.0-toml").unwrap();
    app_config.set("max_plugins", 150).unwrap(); // Different value
    app_config.set("debug_mode", false).unwrap(); // Different value

    // Save app configuration (should save as app_settings.toml)
    storage_manager.save_app_config("app_settings", &app_config)
        .expect("Failed to save app config as TOML");

    // Invalidate cache to ensure loading from disk
    // Invalidate cache to ensure loading from disk
    config_manager.invalidate_cache("app_settings", crate::storage::config::ConfigScope::Application);

    // Load app configuration (should load app_settings.toml)
    let loaded_config = storage_manager.get_app_config("app_settings")
        .expect("Failed to load app config from TOML");

    // Verify values
    assert_eq!(loaded_config.get::<String>("app_name").unwrap(), "Gini Test TOML");
    assert_eq!(loaded_config.get::<String>("version").unwrap(), "1.0.0-toml");
    assert_eq!(loaded_config.get::<i32>("max_plugins").unwrap(), 150);
    assert!(!loaded_config.get::<bool>("debug_mode").unwrap());

    // Test saving/loading with explicit .toml extension
    let mut updated_config = loaded_config;
    updated_config.set("max_plugins", 200).unwrap();
    storage_manager.save_app_config("app_settings.toml", &updated_config)
        .expect("Failed to save app config with explicit .toml");

    // Invalidate and reload
    // Invalidate and reload
    config_manager.invalidate_cache("app_settings.toml", crate::storage::config::ConfigScope::Application);
    let reloaded_config = storage_manager.get_app_config("app_settings.toml")
        .expect("Failed to reload app config with explicit .toml");

    assert_eq!(reloaded_config.get::<i32>("max_plugins").unwrap(), 200);
}
#[test]
async fn test_multiple_plugins_configuration() {
    // Setup test environment
    let (_, _, storage_manager, _, _, _) = setup_test_environment().await;
    
    // Initialize storage manager (creates necessary directories)
    KernelComponent::initialize(&*storage_manager).await.expect("Failed to initialize storage manager");
    
    // Create configurations for multiple plugins
    for i in 1..=3 {
        let plugin_name = format!("Plugin{}", i);
        
        // Create default configuration
        let mut default_config = ConfigData::new();
        default_config.set("plugin_name", plugin_name.clone()).unwrap();
        default_config.set("initialized", true).unwrap();
        default_config.set("setting1", format!("default_value_{}", i)).unwrap();
        
        // Save default configuration
        storage_manager.save_plugin_config(
            &plugin_name,
            &default_config,
            PluginConfigScope::Default
        ).expect("Failed to save default config");
        
        // Create user configuration for odd-numbered plugins
        if i % 2 == 1 {
            let mut user_config = ConfigData::new();
            user_config.set("setting1", format!("user_value_{}", i)).unwrap();
            user_config.set("user_specific", format!("user_data_{}", i)).unwrap();
            
            // Save user configuration
            storage_manager.save_plugin_config(
                &plugin_name,
                &user_config,
                PluginConfigScope::User
            ).expect("Failed to save user config");
        }
    }
    
    // Verify plugin1 configuration (has user override)
    let plugin1_config = storage_manager.get_plugin_config("Plugin1")
        .expect("Failed to get Plugin1 config");
    
    assert_eq!(plugin1_config.get::<String>("plugin_name").unwrap(), "Plugin1");
    assert!(plugin1_config.get::<bool>("initialized").unwrap());
    assert_eq!(plugin1_config.get::<String>("setting1").unwrap(), "user_value_1"); // Overridden
    assert_eq!(plugin1_config.get::<String>("user_specific").unwrap(), "user_data_1");
    
    // Verify plugin2 configuration (no user override)
    let plugin2_config = storage_manager.get_plugin_config("Plugin2")
        .expect("Failed to get Plugin2 config");
    
    assert_eq!(plugin2_config.get::<String>("plugin_name").unwrap(), "Plugin2");
    assert!(plugin2_config.get::<bool>("initialized").unwrap());
    assert_eq!(plugin2_config.get::<String>("setting1").unwrap(), "default_value_2"); // Default
    assert!(plugin2_config.get::<String>("user_specific").is_none()); // Doesn't exist
    
    // List all plugin configurations
    let default_scope = crate::storage::config::ConfigScope::Plugin(PluginConfigScope::Default);
    let default_plugins = storage_manager.list_configs(default_scope)
        .expect("Failed to list default plugin configs");
    
    let user_scope = crate::storage::config::ConfigScope::Plugin(PluginConfigScope::User);
    let user_plugins = storage_manager.list_configs(user_scope)
        .expect("Failed to list user plugin configs");
    
    // Verify counts
    assert_eq!(default_plugins.len(), 3); // All plugins have default configs
    assert_eq!(user_plugins.len(), 2); // Only odd-numbered plugins have user configs
    
    // Verify specific plugins
    assert!(default_plugins.contains(&"Plugin1".to_string()));
    assert!(default_plugins.contains(&"Plugin2".to_string()));
    assert!(default_plugins.contains(&"Plugin3".to_string()));
    
    assert!(user_plugins.contains(&"Plugin1".to_string()));
    assert!(!user_plugins.contains(&"Plugin2".to_string()));
    assert!(user_plugins.contains(&"Plugin3".to_string()));
}