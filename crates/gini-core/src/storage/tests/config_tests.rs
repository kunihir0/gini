use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::{tempdir, TempDir}; // Import TempDir for guard
use async_trait::async_trait;
use futures::executor::block_on; // Added for block_on

use crate::kernel::component::KernelComponent;
use crate::kernel::error::Result;
use crate::storage::local::LocalStorageProvider;
use crate::storage::manager::StorageManager; // Import StorageManager trait
use crate::storage::provider::StorageProvider;
use crate::storage::config::{
    ConfigManager, ConfigData, ConfigFormat, ConfigScope, PluginConfigScope, StorageScope
};

// --- Mock Storage Manager for ConfigManager Tests ---

#[derive(Clone, Debug)] // Added Debug derive
struct MockStorageManager {
    provider: Arc<LocalStorageProvider>,
    config_dir: PathBuf,
    data_dir: PathBuf,
    // Keep the tempdir guard alive
    _temp_dir_guard: Arc<TempDir>,
}

impl MockStorageManager {
    fn new() -> (Self, Arc<TempDir>) {
        let temp_dir = Arc::new(tempdir().expect("Failed to create temp directory for mock storage"));
        let root_path = temp_dir.path().to_path_buf();
        let config_dir = root_path.join("config");
        let data_dir = root_path.join("data");
        let provider = Arc::new(LocalStorageProvider::new(root_path)); // Provider base is root

        let manager = Self {
            provider,
            config_dir,
            data_dir,
            _temp_dir_guard: Arc::clone(&temp_dir),
        };
        (manager, temp_dir)
    }
}

// Implement necessary traits for the mock

#[async_trait]
impl KernelComponent for MockStorageManager {
    fn name(&self) -> &'static str { "MockStorageManager" }
    async fn initialize(&self) -> Result<()> {
        // Ensure base dirs exist within the temp dir
        self.provider.create_dir_all(&self.config_dir)?;
        self.provider.create_dir_all(&self.config_dir.join("plugins"))?; // Needed by ConfigManager structure
        self.provider.create_dir_all(&self.data_dir)?;
        Ok(())
    }
    async fn start(&self) -> Result<()> { Ok(()) }
    async fn stop(&self) -> Result<()> { Ok(()) }
}

impl StorageProvider for MockStorageManager {
    // Delegate all provider methods to the inner LocalStorageProvider
    fn name(&self) -> &str { self.provider.name() }
    fn exists(&self, path: &Path) -> bool { self.provider.exists(path) }
    fn is_file(&self, path: &Path) -> bool { self.provider.is_file(path) }
    fn is_dir(&self, path: &Path) -> bool { self.provider.is_dir(path) }
    fn create_dir(&self, path: &Path) -> Result<()> { self.provider.create_dir(path) }
    fn create_dir_all(&self, path: &Path) -> Result<()> { self.provider.create_dir_all(path) }
    fn read_to_string(&self, path: &Path) -> Result<String> { self.provider.read_to_string(path) }
    fn read_to_bytes(&self, path: &Path) -> Result<Vec<u8>> { self.provider.read_to_bytes(path) }
    fn write_string(&self, path: &Path, contents: &str) -> Result<()> { self.provider.write_string(path, contents) }
    fn write_bytes(&self, path: &Path, contents: &[u8]) -> Result<()> { self.provider.write_bytes(path, contents) }
    fn copy(&self, from: &Path, to: &Path) -> Result<()> { self.provider.copy(from, to) }
    fn rename(&self, from: &Path, to: &Path) -> Result<()> { self.provider.rename(from, to) }
    fn remove_file(&self, path: &Path) -> Result<()> { self.provider.remove_file(path) }
    fn remove_dir(&self, path: &Path) -> Result<()> { self.provider.remove_dir(path) }
    fn remove_dir_all(&self, path: &Path) -> Result<()> { self.provider.remove_dir_all(path) }
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> { self.provider.read_dir(path) }
    fn metadata(&self, path: &Path) -> Result<std::fs::Metadata> { self.provider.metadata(path) }
    fn open_read(&self, path: &Path) -> Result<Box<dyn std::io::Read>> { self.provider.open_read(path) }
    fn open_write(&self, path: &Path) -> Result<Box<dyn std::io::Write>> { self.provider.open_write(path) }
    fn open_append(&self, path: &Path) -> Result<Box<dyn std::io::Write>> { self.provider.open_append(path) }
}

#[async_trait]
impl StorageManager for MockStorageManager {
    fn config_dir(&self) -> &Path { &self.config_dir }
    fn data_dir(&self) -> &Path { &self.data_dir }
    fn resolve_path(&self, scope: StorageScope, relative_path: &Path) -> PathBuf {
        match scope {
            StorageScope::Application => self.config_dir.join(relative_path),
            // Mock resolves plugin path relative to its config dir
            StorageScope::Plugin { plugin_name } => self.config_dir.join("plugins").join(plugin_name).join(relative_path),
            StorageScope::Data => self.data_dir.join(relative_path),
        }
    }
}

// --- Updated Test Setup ---

// Returns the ConfigManager and the TempDir guard (to keep temp dir alive)
fn create_test_config_manager() -> (ConfigManager, Arc<TempDir>) {
    let (mock_storage_manager, temp_dir_guard) = MockStorageManager::new();

    // Initialize the mock storage manager (creates directories)
    block_on(mock_storage_manager.initialize()) // Use block_on directly
        .expect("Failed to initialize mock storage manager");

    // Call ConfigManager::new with the reverted signature (provider, app_path, plugin_path, format)
    let manager = ConfigManager::new(
        mock_storage_manager.provider.clone(), // Pass the provider Arc from the mock
        mock_storage_manager.config_dir.clone(), // Pass the app config path from the mock
        mock_storage_manager.config_dir.join("plugins"), // Construct the plugin config path
        ConfigFormat::Json, // Default format
    );

    (manager, temp_dir_guard)
}

// --- Tests ---

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
    let (config_manager, _temp_dir) = create_test_config_manager(); // Keep temp_dir guard alive

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
    let (config_manager, _temp_dir) = create_test_config_manager(); // Keep temp_dir guard alive

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
    let (config_manager, _temp_dir) = create_test_config_manager(); // Keep temp_dir guard alive

    // Create a malformed TOML file
    // Use the config manager's provider() method to get the provider
    let provider = config_manager.provider(); // Get the provider Arc
    // Resolve path using ConfigManager's method, which now uses stored paths
    let config_path = config_manager.resolve_config_path("malformed.toml", ConfigScope::Application);
    let malformed_content = r#"
        app_name = "Malformed"
        version = "1.0"
        invalid-syntax =
    "#;
    // Ensure parent directory exists before writing
    if let Some(parent_dir) = config_path.parent() {
        provider.create_dir_all(parent_dir).expect("Failed to create parent directory for malformed TOML"); // Use provider
    }
    provider.write_string(&config_path, malformed_content).expect("Failed to write malformed TOML"); // Use provider

    // Invalidate cache using the full filename before attempting load
    config_manager.invalidate_cache("malformed.toml", ConfigScope::Application);

    // Attempt to load the malformed config using the full filename
    let result = config_manager.load_config("malformed.toml", ConfigScope::Application);

    // Verify that loading failed with a deserialization error
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(matches!(e, crate::kernel::error::Error::DeserializationError { .. }));
        assert!(e.to_string().contains("Failed to deserialize from TOML"));
    } else {
        panic!("Expected an error but got Ok");
    }

    Ok(())
}
#[test]
#[cfg(feature = "yaml-config")] // Only run if yaml-config feature is enabled
fn test_yaml_config_operations() -> Result<()> {
    let (config_manager, _temp_dir) = create_test_config_manager(); // Keep temp_dir guard alive

    // Set default format to YAML for this test
    config_manager.set_default_format(ConfigFormat::Yaml);

    // Create configuration
    let mut app_config = ConfigData::new();
    app_config.set("app_name", "Test App YAML")?;
    app_config.set("version", "1.0.0-yaml")?;
    app_config.set("max_connections", 16)?; // Use a different value

    // Save the configuration (should save as settings.yaml due to default format)
    config_manager.save_app_config("settings", &app_config)?;

    // Invalidate cache to force reload from disk
    config_manager.invalidate_cache("settings", ConfigScope::Application);

    // Load the configuration (should load settings.yaml)
    let loaded_config = config_manager.get_app_config("settings")?;

    // Verify loaded values
    assert_eq!(loaded_config.get::<String>("app_name").unwrap(), "Test App YAML");
    assert_eq!(loaded_config.get::<String>("version").unwrap(), "1.0.0-yaml");
    assert_eq!(loaded_config.get::<i32>("max_connections").unwrap(), 16);

    // Test loading explicitly with .yaml extension
    let loaded_explicit = config_manager.get_app_config("settings.yaml")?;
    assert_eq!(loaded_explicit.get::<String>("app_name").unwrap(), "Test App YAML");

    // Test saving explicitly with .yaml extension
    let mut updated_config = loaded_config;
    updated_config.set("max_connections", 26)?;
    config_manager.save_app_config("settings.yaml", &updated_config)?;

    // Invalidate and reload to check explicit save
    config_manager.invalidate_cache("settings.yaml", ConfigScope::Application); // Use full name for cache key
    let reloaded_config = config_manager.get_app_config("settings.yaml")?;
    assert_eq!(reloaded_config.get::<i32>("max_connections").unwrap(), 26);

    Ok(())
}

#[test]
#[cfg(feature = "yaml-config")]
fn test_yaml_malformed_load() -> Result<()> {
    let (config_manager, _temp_dir) = create_test_config_manager(); // Keep temp_dir guard alive

    // Create a malformed YAML file
    let provider = config_manager.provider(); // Get the provider Arc
    let config_path = config_manager.resolve_config_path("malformed.yaml", ConfigScope::Application);
    // Malformed YAML: inconsistent indentation
    let malformed_content = r#"
app_name: "Malformed"
version: "1.0"
  invalid-indent: true
"#;
    // Ensure parent directory exists before writing
    if let Some(parent_dir) = config_path.parent() {
        provider.create_dir_all(parent_dir).expect("Failed to create parent directory for malformed YAML"); // Use provider
    }
    provider.write_string(&config_path, malformed_content).expect("Failed to write malformed YAML"); // Use provider

    // Invalidate cache using the full filename before attempting load
    config_manager.invalidate_cache("malformed.yaml", ConfigScope::Application);

    // Attempt to load the malformed config using the full filename
    let result = config_manager.load_config("malformed.yaml", ConfigScope::Application);

    // Verify that loading failed with a deserialization error
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(matches!(e, crate::kernel::error::Error::DeserializationError { .. }));
        // The exact error message from serde_yaml might vary, but it should indicate deserialization failure.
        assert!(e.to_string().contains("Failed to deserialize from YAML"));
    } else {
        panic!("Expected an error but got Ok");
    }

    Ok(())
}

#[test]
fn test_plugin_config_with_overrides() -> Result<()> {
    let (config_manager, _temp_dir) = create_test_config_manager(); // Keep temp_dir guard alive

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
    let (config_manager, _temp_dir) = create_test_config_manager(); // Keep temp_dir guard alive
    let provider = config_manager.provider(); // Get the provider Arc

    // Create and save initial config
    let mut config = ConfigData::new();
    config.set("value", "initial")?;
    config_manager.save_app_config("cached", &config)?;

    // Load the config (should be cached)
    let loaded1 = config_manager.get_app_config("cached")?;
    assert_eq!(loaded1.get::<String>("value").unwrap(), "initial");

    // Modify the file directly (bypassing cache)
    // Use provider to resolve path and write
    let config_path = config_manager.resolve_config_path("cached.json", ConfigScope::Application);
    let new_content = r#"{"value":"modified"}"#;
    provider.write_string(&config_path, new_content).expect("Failed to write file"); // Use provider

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
    let (config_manager, _temp_dir) = create_test_config_manager(); // Keep temp_dir guard alive

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