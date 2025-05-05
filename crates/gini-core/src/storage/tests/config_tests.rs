use crate::StorageProvider; // Add this import
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;

use crate::kernel::error::Result;
use crate::storage::local::LocalStorageProvider;
use crate::storage::config::{
    ConfigManager, ConfigData, ConfigFormat, ConfigScope, PluginConfigScope
};

fn create_test_config_manager() -> (ConfigManager, PathBuf) { // Remove generic
    // Create temp directory for test
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let root_path = temp_dir.path().to_path_buf();
    
    // Create paths
    let app_config_path = root_path.join("config");
    let plugin_config_path = root_path.join("plugins").join("config");
    
    // Create provider and manager
    let provider = Arc::new(LocalStorageProvider::new(root_path.clone())) as Arc<dyn StorageProvider>; // Cast to dyn trait
    let manager = ConfigManager::new( // Remove generic
        provider,
        app_config_path,
        plugin_config_path,
        ConfigFormat::Json,
    );
    
    // Create directories
    std::fs::create_dir_all(manager.app_config_path()).expect("Failed to create app config directory");
    std::fs::create_dir_all(manager.plugin_config_path().join("default"))
        .expect("Failed to create plugin default config directory");
    std::fs::create_dir_all(manager.plugin_config_path().join("user"))
        .expect("Failed to create plugin user config directory");
    
    (manager, root_path)
}

#[test]
fn test_config_data_basic() -> Result<()> {
    // Test basic ConfigData operations
    let mut config = ConfigData::new();
    
    // Test setting and getting values
    config.set("string_value", "hello")?;
    config.set("int_value", 42)?;
    config.set("bool_value", true)?;
    
    // Array and object values
    config.set("array", vec![1, 2, 3])?;
    
    // Test retrieving values
    assert_eq!(config.get::<String>("string_value").unwrap(), "hello");
    assert_eq!(config.get::<i32>("int_value").unwrap(), 42);
    assert_eq!(config.get::<bool>("bool_value").unwrap(), true);
    
    // Test default values
    assert_eq!(config.get_or("missing_key", "default".to_string()), "default");
    
    // Test removing values
    let removed = config.remove("int_value");
    assert!(removed.is_some());
    assert!(!config.contains_key("int_value"));
    
    // Test listing keys
    let keys = config.keys();
    assert!(keys.contains(&"string_value".to_string()));
    assert!(keys.contains(&"bool_value".to_string()));
    assert!(keys.contains(&"array".to_string()));
    assert!(!keys.contains(&"int_value".to_string()));
    
    Ok(())
}

#[test]
fn test_config_serialization() -> Result<()> {
    // Create a config with some data
    let mut config = ConfigData::new();
    config.set("string_value", "hello")?;
    config.set("int_value", 42)?;
    config.set("bool_value", true)?;
    
    // Test JSON serialization/deserialization
    let json_str = config.serialize(ConfigFormat::Json)?;
    let deserialized = ConfigData::deserialize(&json_str, ConfigFormat::Json)?;
    
    assert_eq!(deserialized.get::<String>("string_value").unwrap(), "hello");
    assert_eq!(deserialized.get::<i32>("int_value").unwrap(), 42);
    assert_eq!(deserialized.get::<bool>("bool_value").unwrap(), true);
    
    #[cfg(feature = "yaml-config")]
    {
        // Test YAML serialization/deserialization if the feature is enabled
        let yaml_str = config.serialize(ConfigFormat::Yaml)?;
        let yaml_deserialized = ConfigData::deserialize(&yaml_str, ConfigFormat::Yaml)?;
        
        assert_eq!(yaml_deserialized.get::<String>("string_value").unwrap(), "hello");
        assert_eq!(yaml_deserialized.get::<i32>("int_value").unwrap(), 42);
        assert_eq!(yaml_deserialized.get::<bool>("bool_value").unwrap(), true);
    }
    
    #[cfg(feature = "toml-config")]
    {
        // Test TOML serialization/deserialization if the feature is enabled
        let toml_str = config.serialize(ConfigFormat::Toml)?;
        let toml_deserialized = ConfigData::deserialize(&toml_str, ConfigFormat::Toml)?;
        
        assert_eq!(toml_deserialized.get::<String>("string_value").unwrap(), "hello");
        assert_eq!(toml_deserialized.get::<i32>("int_value").unwrap(), 42);
        assert_eq!(toml_deserialized.get::<bool>("bool_value").unwrap(), true);
    }
    
    Ok(())
}

#[test]
fn test_app_config_operations() -> Result<()> {
    let (config_manager, _root_path) = create_test_config_manager();
    
    // Create configuration
    let mut app_config = ConfigData::new();
    app_config.set("app_name", "Test App")?;
    app_config.set("version", "1.0.0")?;
    app_config.set("max_connections", 10)?;
    
    // Save the configuration
    config_manager.save_app_config("settings", &app_config)?;
    
    // Load the configuration
    let loaded_config = config_manager.get_app_config("settings")?;
    
    // Verify loaded values
    assert_eq!(loaded_config.get::<String>("app_name").unwrap(), "Test App");
    assert_eq!(loaded_config.get::<String>("version").unwrap(), "1.0.0");
    assert_eq!(loaded_config.get::<i32>("max_connections").unwrap(), 10);
    
    // Update the configuration
    let mut updated_config = loaded_config;
    updated_config.set("max_connections", 20)?;
    updated_config.set("new_setting", "new value")?;
    
    // Save the updated configuration
    config_manager.save_app_config("settings", &updated_config)?;
    
    // Load the updated configuration
    let reloaded_config = config_manager.get_app_config("settings")?;
    
    // Verify the updates
    assert_eq!(reloaded_config.get::<i32>("max_connections").unwrap(), 20);
    assert_eq!(reloaded_config.get::<String>("new_setting").unwrap(), "new value");
    assert_eq!(reloaded_config.get::<String>("app_name").unwrap(), "Test App");
    
    Ok(())
}
#[test]
#[cfg(feature = "toml-config")] // Only run if toml-config feature is enabled
fn test_toml_config_operations() -> Result<()> {
    let (mut config_manager, _root_path) = create_test_config_manager();
    
    // Set default format to TOML for this test
    config_manager.set_default_format(ConfigFormat::Toml);

    // Create configuration
    let mut app_config = ConfigData::new();
    app_config.set("app_name", "Test App TOML")?;
    app_config.set("version", "1.0.0-toml")?;
    app_config.set("max_connections", 15)?; // Use a different value

    // Save the configuration (should save as settings.toml due to default format)
    config_manager.save_app_config("settings", &app_config)?;

    // Invalidate cache to force reload from disk
    config_manager.invalidate_cache("settings", ConfigScope::Application);

    // Load the configuration (should load settings.toml)
    let loaded_config = config_manager.get_app_config("settings")?;

    // Verify loaded values
    assert_eq!(loaded_config.get::<String>("app_name").unwrap(), "Test App TOML");
    assert_eq!(loaded_config.get::<String>("version").unwrap(), "1.0.0-toml");
    assert_eq!(loaded_config.get::<i32>("max_connections").unwrap(), 15);

    // Test loading explicitly with .toml extension
    let loaded_explicit = config_manager.get_app_config("settings.toml")?;
    assert_eq!(loaded_explicit.get::<String>("app_name").unwrap(), "Test App TOML");

    // Test saving explicitly with .toml extension
    let mut updated_config = loaded_config;
    updated_config.set("max_connections", 25)?;
    config_manager.save_app_config("settings.toml", &updated_config)?;

    // Invalidate and reload to check explicit save
    config_manager.invalidate_cache("settings.toml", ConfigScope::Application); // Use full name for cache key
    let reloaded_config = config_manager.get_app_config("settings.toml")?;
    assert_eq!(reloaded_config.get::<i32>("max_connections").unwrap(), 25);

    Ok(())
}

#[test]
#[cfg(feature = "toml-config")]
fn test_toml_malformed_load() -> Result<()> {
    let (config_manager, root_path) = create_test_config_manager();
    
    // Create a malformed TOML file
    let config_path = config_manager.resolve_config_path("malformed", ConfigScope::Application)
                                    .with_extension("toml"); // Ensure .toml extension
    let malformed_content = r#"
        app_name = "Malformed"
        version = "1.0"
        invalid-syntax =
    "#;
    // Ensure parent directory exists before writing
    if let Some(parent_dir) = config_path.parent() {
        std::fs::create_dir_all(parent_dir).expect("Failed to create parent directory for malformed TOML");
    }
    std::fs::write(&config_path, malformed_content).expect("Failed to write malformed TOML");

    // Attempt to load the malformed config
    let result = config_manager.load_config("malformed.toml", ConfigScope::Application);

    // Verify that loading failed with a storage error (indicating parsing failure)
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(matches!(e, crate::kernel::error::Error::Storage(_)));
        assert!(e.to_string().contains("Failed to deserialize from TOML"));
    } else {
        panic!("Expected an error but got Ok");
    }

    Ok(())
}

#[test]
fn test_plugin_config_with_overrides() -> Result<()> {
    let (config_manager, _root_path) = create_test_config_manager();
    
    // Create default plugin configuration
    let mut default_config = ConfigData::new();
    default_config.set("plugin_name", "Test Plugin")?;
    default_config.set("version", "1.0.0")?;
    default_config.set("setting1", "default_value1")?;
    default_config.set("setting2", "default_value2")?;
    
    // Save default plugin configuration
    config_manager.save_plugin_config("test_plugin", &default_config, PluginConfigScope::Default)?;
    
    // Create user-specific plugin configuration (partial override)
    let mut user_config = ConfigData::new();
    user_config.set("setting1", "user_value1")?; // Override
    user_config.set("setting3", "user_value3")?; // New setting
    
    // Save user plugin configuration
    config_manager.save_plugin_config("test_plugin", &user_config, PluginConfigScope::User)?;
    
    // Get merged configuration
    let merged_config = config_manager.get_plugin_config("test_plugin")?;
    
    // Verify merged values
    assert_eq!(merged_config.get::<String>("plugin_name").unwrap(), "Test Plugin"); // From default
    assert_eq!(merged_config.get::<String>("version").unwrap(), "1.0.0"); // From default
    assert_eq!(merged_config.get::<String>("setting1").unwrap(), "user_value1"); // Overridden
    assert_eq!(merged_config.get::<String>("setting2").unwrap(), "default_value2"); // From default
    assert_eq!(merged_config.get::<String>("setting3").unwrap(), "user_value3"); // New from user
    
    Ok(())
}

#[test]
fn test_config_cache() -> Result<()> {
    let (config_manager, root_path) = create_test_config_manager();
    
    // Create and save initial config
    let mut config = ConfigData::new();
    config.set("value", "initial")?;
    config_manager.save_app_config("cached", &config)?;
    
    // Load the config (should be cached)
    let loaded1 = config_manager.get_app_config("cached")?;
    assert_eq!(loaded1.get::<String>("value").unwrap(), "initial");
    
    // Modify the file directly (bypassing cache)
    let config_path = root_path.join("config").join("cached.json");
    let new_content = r#"{"value":"modified"}"#;
    std::fs::write(config_path, new_content).expect("Failed to write file");
    
    // Load again (should still give cached version)
    let loaded2 = config_manager.get_app_config("cached")?;
    assert_eq!(loaded2.get::<String>("value").unwrap(), "initial");
    
    // Invalidate the cache
    config_manager.invalidate_cache("cached", ConfigScope::Application);
    
    // Load again (should get updated version)
    let loaded3 = config_manager.get_app_config("cached")?;
    assert_eq!(loaded3.get::<String>("value").unwrap(), "modified");
    
    Ok(())
}

#[test]
fn test_list_configs() -> Result<()> {
    let (config_manager, _root_path) = create_test_config_manager();
    
    // Create several app configs
    for name in &["settings1", "settings2", "settings3"] {
        let mut config = ConfigData::new();
        config.set("name", name)?;
        config_manager.save_app_config(name, &config)?;
    }
    
    // List app configs
    let app_configs = config_manager.list_configs(ConfigScope::Application)?;
    
    // Verify the list contains our configs
    assert!(app_configs.contains(&"settings1".to_string()));
    assert!(app_configs.contains(&"settings2".to_string()));
    assert!(app_configs.contains(&"settings3".to_string()));
    assert_eq!(app_configs.len(), 3);
    
    // Create several plugin configs
    for name in &["plugin1", "plugin2"] {
        let mut config = ConfigData::new();
        config.set("name", name)?;
        config_manager.save_plugin_config(name, &config, PluginConfigScope::Default)?;
    }
    
    // Create a user plugin config
    let mut user_config = ConfigData::new();
    user_config.set("name", "plugin1_user")?;
    config_manager.save_plugin_config("plugin1", &user_config, PluginConfigScope::User)?;
    
    // List plugin default configs
    let plugin_default_configs = config_manager.list_configs(ConfigScope::Plugin(PluginConfigScope::Default))?;
    assert!(plugin_default_configs.contains(&"plugin1".to_string()));
    assert!(plugin_default_configs.contains(&"plugin2".to_string()));
    assert_eq!(plugin_default_configs.len(), 2);
    
    // List plugin user configs
    let plugin_user_configs = config_manager.list_configs(ConfigScope::Plugin(PluginConfigScope::User))?;
    assert!(plugin_user_configs.contains(&"plugin1".to_string()));
    assert_eq!(plugin_user_configs.len(), 1);
    
    Ok(())
}

#[test]
fn test_config_format_detection() {
    // Test extension to format mapping
    assert_eq!(
        ConfigFormat::from_path(&PathBuf::from("test.json")).unwrap(),
        ConfigFormat::Json
    );
    
    #[cfg(feature = "yaml-config")]
    {
        assert_eq!(
            ConfigFormat::from_path(&PathBuf::from("test.yaml")).unwrap(),
            ConfigFormat::Yaml
        );
        assert_eq!(
            ConfigFormat::from_path(&PathBuf::from("test.yml")).unwrap(),
            ConfigFormat::Yaml
        );
    }
    
    #[cfg(feature = "toml-config")]
    {
        assert_eq!(
            ConfigFormat::from_path(&PathBuf::from("test.toml")).unwrap(),
            ConfigFormat::Toml
        );
    }
    
    // Unknown extension
    assert!(ConfigFormat::from_path(&PathBuf::from("test.unknown")).is_none());
    
    // No extension
    assert!(ConfigFormat::from_path(&PathBuf::from("test")).is_none());
}

#[test]
fn test_config_merge() -> Result<()> {
    // Create two configurations
    let mut config1 = ConfigData::new();
    config1.set("shared", "original")?;
    config1.set("only_in_1", "value1")?;
    
    let mut config2 = ConfigData::new();
    config2.set("shared", "overridden")?;
    config2.set("only_in_2", "value2")?;
    
    // Merge config2 into config1
    config1.merge(&config2);
    
    // Verify result
    assert_eq!(config1.get::<String>("shared").unwrap(), "overridden");
    assert_eq!(config1.get::<String>("only_in_1").unwrap(), "value1");
    assert_eq!(config1.get::<String>("only_in_2").unwrap(), "value2");
    
    Ok(())
}