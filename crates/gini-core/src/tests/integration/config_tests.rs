#![cfg(test)]

use tokio::test;
use async_trait::async_trait;
use std::time::SystemTime;
use std::sync::Arc;
use tokio::sync::Mutex;
use tempfile::tempdir; 
use std::fs; 

use crate::kernel::bootstrap::Application;
// use crate::kernel::component::KernelComponent;
// use crate::kernel::error::{Error, Result as KernelResult}; // Removed unused imports
use crate::plugin_system::dependency::PluginDependency;
use crate::plugin_system::error::PluginSystemError; // Import PluginSystemError
use crate::plugin_system::traits::{Plugin, PluginPriority}; // Removed PluginError
use crate::plugin_system::version::VersionRange;
use crate::stage_manager::StageContext;
use crate::stage_manager::registry::StageRegistry;
use crate::stage_manager::requirement::StageRequirement;
use crate::storage::config::{ConfigData, PluginConfigScope, ConfigManager, ConfigFormat, ConfigScope};
use crate::storage::local::LocalStorageProvider;
// use crate::storage::manager::DefaultStorageManager; // Only if setup_test_environment's DSM is used

use super::common::setup_test_environment;

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

    fn init(&self, app: &mut Application) -> std::result::Result<(), PluginSystemError> {
        let storage_manager = app.storage_manager();
        let config = match storage_manager.get_plugin_config(self.name()) {
            Ok(config) => config,
            Err(_) => {
                let mut default_config = ConfigData::new();
                default_config.set("plugin_name", self.name()).map_err(|e| PluginSystemError::InternalError(e.to_string()))?;
                default_config.set("initialized", true).map_err(|e| PluginSystemError::InternalError(e.to_string()))?;
                default_config.set("setting1", "default_value").map_err(|e| PluginSystemError::InternalError(e.to_string()))?;
                storage_manager.save_plugin_config(
                    self.name(),
                    &default_config,
                    PluginConfigScope::Default
                ).map_err(|e| PluginSystemError::InternalError(e.to_string()))?;
                default_config
            }
        };
        let initialized = config.get::<bool>("initialized").unwrap_or(false);
        if !initialized {
            return Err(PluginSystemError::InitializationError {
                plugin_id: self.name().to_string(),
                message: "Plugin not properly initialized (config missing 'initialized' or false)".to_string(),
                source: None,
            });
        }
        let mut updated_config = config.clone();
        updated_config.set("last_used", SystemTime::now().elapsed().unwrap_or_default().as_secs()).map_err(|e| PluginSystemError::InternalError(e.to_string()))?;
        storage_manager.save_plugin_config(
            self.name(),
            &updated_config,
            PluginConfigScope::User
        ).map_err(|e| PluginSystemError::InternalError(e.to_string()))?;
        Ok(())
    }

    async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), PluginSystemError> { Ok(()) }
    fn shutdown(&self) -> std::result::Result<(), PluginSystemError> { Ok(()) }
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
    fn register_stages(&self, _registry: &mut StageRegistry) -> std::result::Result<(), PluginSystemError> { Ok(()) }
}

#[test]
async fn test_plugin_configuration_management() {
    let (plugin_manager, _stage_manager, _, _, _, _) = setup_test_environment().await;
    let mut app = Application::new().expect("Failed to create application");
    let plugin = ConfigUsingPlugin::new("ConfigPluginForTestMgmt"); 
    let plugin_name = plugin.name().to_string();
    
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Arc::new(plugin))
            .expect("Failed to register config plugin");
    }

    let app_storage_manager = app.storage_manager();
    let mut default_config = ConfigData::new();
    default_config.set("plugin_name", &plugin_name).unwrap();
    default_config.set("initialized", true).unwrap(); 
    default_config.set("setting1", "default_value_for_mgmt_test").unwrap();
    default_config.set("setting2", "another_value_for_mgmt_test").unwrap();

    app_storage_manager.save_plugin_config(
        &plugin_name,
        &default_config,
        PluginConfigScope::Default
    ).expect("Failed to save default config for mgmt test");

    {
        let registry = plugin_manager.registry();
        let stage_registry_arc = Arc::new(Mutex::new(StageRegistry::new())); 
        let mut reg_lock = registry.lock().await;
        reg_lock.initialize_plugin(&plugin_name, &mut app, &stage_registry_arc)
            .await.expect("Failed to initialize plugin for mgmt test");
    }

    let user_scope = ConfigScope::Plugin(PluginConfigScope::User);
    let user_config = app_storage_manager.load_config(&plugin_name, user_scope)
        .expect("Failed to load user config for mgmt test");
    assert!(user_config.contains_key("last_used"));

    let merged_config = app_storage_manager.get_plugin_config(&plugin_name)
        .expect("Failed to get plugin config for mgmt test");
    assert_eq!(merged_config.get::<String>("plugin_name").unwrap(), plugin_name);
    assert!(merged_config.get::<bool>("initialized").unwrap());
    assert_eq!(merged_config.get::<String>("setting1").unwrap(), "default_value_for_mgmt_test");
    assert_eq!(merged_config.get::<String>("setting2").unwrap(), "another_value_for_mgmt_test");
    assert!(merged_config.contains_key("last_used"));
}

#[test]
async fn test_app_configuration_management() {
    let temp_dir = tempdir().expect("Failed to create temp dir for app config test");
    let base_path = temp_dir.path();
    
    let local_provider = Arc::new(LocalStorageProvider::new(base_path.to_path_buf())); // Corrected
    
    let app_config_dir = base_path.join("app_cfg_dir"); // Unique name for this test's config
    let plugin_config_dir = base_path.join("plugin_cfg_dir"); // Unique name
    fs::create_dir_all(&app_config_dir).unwrap();
    fs::create_dir_all(&plugin_config_dir).unwrap();

    let config_manager = Arc::new(ConfigManager::new(
        local_provider.clone(), 
        app_config_dir.clone(), 
        plugin_config_dir.clone(), 
        ConfigFormat::Json,
    ));

    let mut app_config_data = ConfigData::new();
    let app_settings_name = "app_settings_isolated"; 
    app_config_data.set("app_name", "Gini Test AppMgmt Isolated").unwrap();
    app_config_data.set("version", "1.0.0").unwrap();
    
    config_manager.save_app_config(app_settings_name, &app_config_data)
        .expect("Failed to save app config");
    
    let loaded_config = config_manager.get_app_config(app_settings_name)
        .expect("Failed to load app config");
    
    assert_eq!(loaded_config.get::<String>("app_name").unwrap(), "Gini Test AppMgmt Isolated");
    
    let mut updated_config = loaded_config;
    updated_config.set("version", "1.0.1").unwrap();
    
    config_manager.save_app_config(app_settings_name, &updated_config)
        .expect("Failed to save updated app config");
    
    let reloaded_config = config_manager.get_app_config(app_settings_name)
        .expect("Failed to reload app config");
    
    assert_eq!(reloaded_config.get::<String>("version").unwrap(), "1.0.1");
}

#[test]
#[cfg(feature = "toml-config")]
async fn test_app_configuration_toml_format() {
    let temp_dir = tempdir().expect("Failed to create temp dir for app config toml test");
    let base_path = temp_dir.path();
    let local_provider = Arc::new(LocalStorageProvider::new(base_path.to_path_buf())); // Corrected

    let app_config_dir = base_path.join("app_cfg_toml"); 
    let plugin_config_dir = base_path.join("plugin_cfg_toml");
    fs::create_dir_all(&app_config_dir).unwrap();
    fs::create_dir_all(&plugin_config_dir).unwrap();
    
    let config_manager_instance = ConfigManager::new(
        local_provider.clone(),
        app_config_dir.clone(), 
        plugin_config_dir.clone(), 
        ConfigFormat::Json, 
    );
    config_manager_instance.set_default_format(crate::storage::config::ConfigFormat::Toml);
    let config_manager = Arc::new(config_manager_instance);

    let app_settings_name_toml = "app_settings_isolated_toml";
    let mut app_config = ConfigData::new();
    app_config.set("app_name", "Gini Test TOML Isolated").unwrap();

    config_manager.save_app_config(app_settings_name_toml, &app_config)
        .expect("Failed to save app config as TOML");

    config_manager.invalidate_cache(app_settings_name_toml, ConfigScope::Application);

    let loaded_config = config_manager.get_app_config(app_settings_name_toml)
        .expect("Failed to load app config from TOML");
    assert_eq!(loaded_config.get::<String>("app_name").unwrap(), "Gini Test TOML Isolated");
    
    config_manager.set_default_format(crate::storage::config::ConfigFormat::Json); 
}

#[test]
async fn test_multiple_plugins_configuration() {
    let temp_dir = tempdir().expect("Failed to create temp dir for multiple plugins test");
    let base_path = temp_dir.path();

    let local_provider = Arc::new(LocalStorageProvider::new(base_path.to_path_buf())); // Corrected
    
    let app_config_base = base_path.join("app_cfg_multi"); 
    let plugin_config_base = base_path.join("plugin_cfg_multi"); 
    
    fs::create_dir_all(&app_config_base).unwrap();
    fs::create_dir_all(&plugin_config_base).unwrap();
    fs::create_dir_all(plugin_config_base.join("default")).unwrap();
    fs::create_dir_all(plugin_config_base.join("user")).unwrap();

    let config_manager = ConfigManager::new(
        local_provider.clone(), 
        app_config_base,      
        plugin_config_base,     
        ConfigFormat::Json,
    );
    
    for i in 1..=3 {
        let plugin_name = format!("Plugin{}", i);
        let mut default_config = ConfigData::new();
        default_config.set("plugin_name", plugin_name.clone()).unwrap();
        default_config.set("initialized", true).unwrap();
        default_config.set("setting1", format!("default_value_{}", i)).unwrap();
        
        config_manager.save_plugin_config(
            &plugin_name,
            &default_config,
            PluginConfigScope::Default
        ).expect("Failed to save default config");
        
        if i % 2 == 1 {
            let mut user_config = ConfigData::new();
            user_config.set("setting1", format!("user_value_{}", i)).unwrap();
            user_config.set("user_specific", format!("user_data_{}", i)).unwrap();
            
            config_manager.save_plugin_config(
                &plugin_name,
                &user_config,
                PluginConfigScope::User
            ).expect("Failed to save user config");
        }
    }
    
    let plugin1_config = config_manager.get_plugin_config("Plugin1")
        .expect("Failed to get Plugin1 config");
    
    assert_eq!(plugin1_config.get::<String>("plugin_name").unwrap(), "Plugin1");
    assert!(plugin1_config.get::<bool>("initialized").unwrap());
    assert_eq!(plugin1_config.get::<String>("setting1").unwrap(), "user_value_1");
    assert_eq!(plugin1_config.get::<String>("user_specific").unwrap(), "user_data_1");
    
    let plugin2_config = config_manager.get_plugin_config("Plugin2")
        .expect("Failed to get Plugin2 config");
    
    assert_eq!(plugin2_config.get::<String>("plugin_name").unwrap(), "Plugin2");
    assert!(plugin2_config.get::<bool>("initialized").unwrap());
    assert_eq!(plugin2_config.get::<String>("setting1").unwrap(), "default_value_2");
    assert!(plugin2_config.get::<String>("user_specific").is_none());
    
    let default_scope = ConfigScope::Plugin(PluginConfigScope::Default);
    let default_plugins = config_manager.list_configs(default_scope)
        .expect("Failed to list default plugin configs");
    
    let user_scope = ConfigScope::Plugin(PluginConfigScope::User);
    let user_plugins = config_manager.list_configs(user_scope)
        .expect("Failed to list user plugin configs");
    
    assert_eq!(default_plugins.len(), 3, "Expected 3 default plugin configs, found: {:?}", default_plugins);
    assert_eq!(user_plugins.len(), 2, "Expected 2 user plugin configs, found: {:?}", user_plugins);
    
    assert!(default_plugins.contains(&"Plugin1".to_string()));
    assert!(default_plugins.contains(&"Plugin2".to_string()));
    assert!(default_plugins.contains(&"Plugin3".to_string()));
    
    assert!(user_plugins.contains(&"Plugin1".to_string()));
    assert!(!user_plugins.contains(&"Plugin2".to_string()));
    assert!(user_plugins.contains(&"Plugin3".to_string()));
}