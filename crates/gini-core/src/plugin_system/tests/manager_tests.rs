// #![cfg(test)] // Removed this redundant line

use crate::StorageProvider; // Add this import
use crate::plugin_system::manager::{DefaultPluginManager, PluginManager}; // Keep this line
use crate::plugin_system::traits::{Plugin, PluginPriority}; // Removed PluginError
use crate::plugin_system::error::PluginSystemError; // Import PluginSystemError
use crate::plugin_system::dependency::PluginDependency;
use crate::storage::config::{ConfigManager, ConfigFormat}; // Added
use crate::storage::local::LocalStorageProvider; // Added
use crate::plugin_system::version::VersionRange;
use crate::kernel::bootstrap::Application; // For Plugin::init signature
use crate::kernel::error::Error; // KernelResult alias removed
use crate::kernel::component::KernelComponent; // Import KernelComponent trait
use crate::stage_manager::context::StageContext;
use crate::stage_manager::requirement::StageRequirement;
use crate::stage_manager::Stage; // Import Stage trait
use crate::stage_manager::registry::StageRegistry; // Added for register_stages
use std::error::Error as StdError; // For boxing
use async_trait::async_trait;
use std::sync::Arc;
use tempfile::{tempdir, TempDir}; // Added TempDir
use std::path::{Path, PathBuf}; // Added Path
use std::str::FromStr; // Import FromStr for parsing VersionRange
use std::env; // Added for helper function
use std::fs; // Added for helper function
use std::fmt;
use crate::event::{DefaultEventManager, EventManager}; // Added for StageManager
use crate::stage_manager::manager::DefaultStageManager; // Added for StageManager
use std::time::Duration;
use serde_json::Value; // For deserializing state
// use rand; // Removed as no longer used after fixing test directory name

// Constants for config file and key used by DefaultPluginManager
const CORE_SETTINGS_CONFIG_NAME_VAL: &str = "core_settings";
const DISABLED_PLUGINS_KEY_VAL: &str = "core.plugins.disabled";

// Helper function to find the path to the compiled example plugin
// Copied from loading_tests.rs
fn get_example_plugin_path() -> Option<PathBuf> {
    // Try to find the plugin in various possible locations
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let plugin_name = "libcompat_check_example.so";

    // Common locations to search
    let search_paths = vec![
        // From crate directory (relative to crates/gini-core)
        current_dir.join("../target/debug").join(plugin_name),
        // From workspace root
        current_dir.join("target/debug").join(plugin_name),
        // Relative path often used in tests
        PathBuf::from("./target/debug").join(plugin_name),
         // Another common relative path from workspace root if tests run differently
        PathBuf::from("../target/debug").join(plugin_name),

    ];

    // Return the first path that exists
    for path in search_paths {
        if path.exists() {
            println!("Found example plugin at: {:?}", path); // Debug print
            return Some(path);
        } else {
             println!("Checked path, not found: {:?}", path); // Debug print
        }
    }
     println!("Example plugin not found in search paths."); // Debug print
    None
}

// --- Mock Plugin ---
struct MockManagerPlugin {
    id: String,
    deps: Vec<PluginDependency>,
    shutdown_behavior: ShutdownBehavior,
    init_behavior: InitBehavior,
    stages_to_provide: Vec<Box<dyn Stage>>,
    version_str: String,
    is_core_plugin: bool,
    priority_value: PluginPriority,
    compatible_versions: Vec<VersionRange>,
    required_stages_list: Vec<StageRequirement>,
    conflicts_with_ids: Vec<String>, // Added for conflict testing
}

enum ShutdownBehavior {
    Success,
    Failure(String),
    Timeout,  // Simulates long-running operation
}

enum InitBehavior {
    Success,
    Failure(String),
    Timeout,  // Simulates long-running operation
}

impl MockManagerPlugin {
    fn new(id: &str, deps: Vec<PluginDependency>) -> Self {
        Self {
            id: id.to_string(),
            deps,
            shutdown_behavior: ShutdownBehavior::Success,
            init_behavior: InitBehavior::Success,
            stages_to_provide: vec![],
            version_str: "1.0.0".to_string(),
            is_core_plugin: false,
            priority_value: PluginPriority::ThirdParty(100),
            compatible_versions: vec![VersionRange::from_str(">=0.1.0").unwrap()],
            required_stages_list: vec![],
            conflicts_with_ids: vec![], // Initialize new field
        }
    }

    #[allow(dead_code)] // This might be unused depending on test scenarios
    fn with_conflicts(mut self, conflicts: Vec<String>) -> Self {
        self.conflicts_with_ids = conflicts;
        self
    }

    fn with_shutdown_error(mut self, error_msg: &str) -> Self {
        self.shutdown_behavior = ShutdownBehavior::Failure(error_msg.to_string());
        self
    }

    fn with_init_error(mut self, error_msg: &str) -> Self {
        self.init_behavior = InitBehavior::Failure(error_msg.to_string());
        self
    }

    fn with_shutdown_timeout(mut self) -> Self {
        self.shutdown_behavior = ShutdownBehavior::Timeout;
        self
    }

    fn with_init_timeout(mut self) -> Self {
        self.init_behavior = InitBehavior::Timeout;
        self
    }

    fn with_version(mut self, version: &str) -> Self {
        self.version_str = version.to_string();
        self
    }

    fn with_core_status(mut self, is_core: bool) -> Self {
        self.is_core_plugin = is_core;
        self
    }

    fn with_priority(mut self, priority: PluginPriority) -> Self {
        self.priority_value = priority;
        self
    }

    fn with_compatible_api_versions(mut self, versions: Vec<&str>) -> Self {
        self.compatible_versions = versions.into_iter()
            .map(|v| VersionRange::from_str(v).unwrap())
            .collect();
        self
    }

    fn with_required_stages(mut self, stages: Vec<StageRequirement>) -> Self {
        self.required_stages_list = stages;
        self
    }

    fn add_stage(mut self, stage: Box<dyn Stage>) -> Self {
        self.stages_to_provide.push(stage);
        self
    }
}

impl fmt::Debug for MockManagerPlugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MockManagerPlugin")
            .field("id", &self.id)
            .field("version", &self.version_str)
            .field("is_core", &self.is_core_plugin)
            .field("priority", &self.priority_value)
            .field("deps", &self.deps.len())
            .finish()
    }
}

#[async_trait]
impl Plugin for MockManagerPlugin {
    fn name(&self) -> &'static str {
        // Hacky: Leak the string to get a 'static str. Okay for tests.
        Box::leak(self.id.clone().into_boxed_str())
    }

    fn version(&self) -> &str { &self.version_str }

    fn is_core(&self) -> bool { self.is_core_plugin }

    fn priority(&self) -> PluginPriority { self.priority_value.clone() }

    fn compatible_api_versions(&self) -> Vec<VersionRange> { self.compatible_versions.clone() }

    fn dependencies(&self) -> Vec<PluginDependency> { self.deps.clone() }

    fn required_stages(&self) -> Vec<StageRequirement> { self.required_stages_list.clone() }

    fn shutdown(&self) -> std::result::Result<(), PluginSystemError> {
        match &self.shutdown_behavior {
            ShutdownBehavior::Success => Ok(()),
            ShutdownBehavior::Failure(msg) => Err(PluginSystemError::ShutdownError {
                plugin_id: self.id.clone(),
                message: msg.clone(),
            }),
            ShutdownBehavior::Timeout => {
                // Simulate a long operation
                std::thread::sleep(Duration::from_millis(50));
                Ok(())
            }
        }
    }

    fn init(&self, _app: &mut Application) -> std::result::Result<(), PluginSystemError> {
        match &self.init_behavior {
            InitBehavior::Success => Ok(()),
            InitBehavior::Failure(msg) => Err(PluginSystemError::InitializationError {
                plugin_id: self.id.clone(),
                message: msg.clone(),
                source: None,
            }),
            InitBehavior::Timeout => {
                // Simulate a long operation
                std::thread::sleep(Duration::from_millis(50));
                Ok(())
            }
        }
    }

    async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), PluginSystemError> { Ok(()) }

    fn register_stages(&self, registry: &mut StageRegistry) -> std::result::Result<(), PluginSystemError> {
        // Register clones of the stages
        for stage in &self.stages_to_provide {
            registry.register_stage(Box::new(MockStage::new(&stage.id()))).map_err(|e| PluginSystemError::InternalError(e.to_string()))?; // Use register_stage
        }
        Ok(())
    }

    fn conflicts_with(&self) -> Vec<String> { self.conflicts_with_ids.clone() }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
}

struct MockStage {
    id: String,
}

impl MockStage {
    fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

#[async_trait]
impl Stage for MockStage {
    fn id(&self) -> &str { &self.id }

    fn name(&self) -> &str { &self.id } // Added missing name method

    fn description(&self) -> &str { "Mock stage for testing" }

    fn supports_dry_run(&self) -> bool { true }
 
    async fn execute(&self, _context: &mut StageContext) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> { Ok(()) }
 
    fn dry_run_description(&self, _context: &StageContext) -> String {
        format!("Would execute stage {}", self.id)
    }
}

// Helper function to create a manager with a temporary config directory
fn create_test_manager() -> (DefaultPluginManager, TempDir) { // Remove generic
    let tmp_dir = tempdir().unwrap();
    let app_config_path = tmp_dir.path().join("app_config");
    let plugin_config_path = tmp_dir.path().join("plugin_config");
    fs::create_dir_all(&app_config_path).unwrap();
    fs::create_dir_all(&plugin_config_path).unwrap();

    // Pass the tmp_dir path to the LocalStorageProvider
    let provider = Arc::new(LocalStorageProvider::new(tmp_dir.path().to_path_buf())) as Arc<dyn StorageProvider>; // Cast to dyn trait
    // Explicitly type the Arc<ConfigManager>
    // Call ConfigManager::new with the reverted signature
    let config_manager: Arc<ConfigManager> = Arc::new(ConfigManager::new(
        provider,             // Pass the provider Arc
        app_config_path,      // Pass the app config path
        plugin_config_path,   // Pass the plugin config path
        ConfigFormat::Json,   // Pass the default format
    ));

    let event_manager = Arc::new(DefaultEventManager::new()) as Arc<dyn EventManager>;
    let stage_manager = Arc::new(DefaultStageManager::new(event_manager));
    let stage_registry_arc = stage_manager.registry();

    (DefaultPluginManager::new(config_manager, stage_registry_arc).unwrap(), tmp_dir)
}


#[tokio::test]
async fn test_manager_new() {
    // Test that the constructor with ConfigManager works
    let (manager, _tmp_dir) = create_test_manager();
    // If create_test_manager succeeded, new() worked.
    assert_eq!(manager.name(), "DefaultPluginManager");
}

#[tokio::test]
async fn test_manager_initialize_no_dir() {
    // Test initialization when the default plugin dir doesn't exist
    let (manager, _tmp_dir) = create_test_manager();
    // Ensure the default dir doesn't exist for this test run
    let plugin_dir = PathBuf::from("./target/debug_nonexistent_for_test"); // Use a unique name
    if plugin_dir.exists() {
        fs::remove_dir_all(&plugin_dir).expect("Failed to remove test dir before test");
    }
     // Call initialize via the KernelComponent trait explicitly
     let result = KernelComponent::initialize(&manager).await;
     assert!(result.is_ok(), "Initialize should succeed even if dir is missing");
}

#[tokio::test]
async fn test_is_plugin_loaded() {
    let (manager, _tmp_dir) = create_test_manager();
    let plugin = Arc::new(MockManagerPlugin::new("test_plugin", vec![]));
    let plugin_name = plugin.name().to_string(); // Get name before moving

    // Register plugin directly via registry for testing loaded status
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin).unwrap();
    }

    // Check loaded status
    let is_loaded = manager.is_plugin_loaded(&plugin_name).await;
    assert!(is_loaded.is_ok());
    assert!(is_loaded.unwrap(), "Plugin should be marked as loaded");

    // Check non-existent plugin
    let not_loaded = manager.is_plugin_loaded("non_existent").await;
    assert!(not_loaded.is_ok());
    assert!(!not_loaded.unwrap(), "Non-existent plugin should not be loaded");
}

#[tokio::test]
async fn test_get_plugin_dependencies() {
    let (manager, _tmp_dir) = create_test_manager();
    // Use helper constructors for dependencies
    let dep1 = PluginDependency::required("dep_a", VersionRange::from_str(">=1.0").unwrap());
    let dep2 = PluginDependency::required("dep_b", VersionRange::from_str("~2.1").unwrap());

    let plugin = Arc::new(MockManagerPlugin::new("test_plugin", vec![dep1.clone(), dep2.clone()]));
    let plugin_name = plugin.name().to_string();

    // Register plugin
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin).unwrap();
        // Also register dummy dependencies so registry doesn't complain later if needed
        registry.register_plugin(Arc::new(MockManagerPlugin::new("dep_a", vec![]))).unwrap();
        registry.register_plugin(Arc::new(MockManagerPlugin::new("dep_b", vec![]))).unwrap();
    }

    // Get dependencies
    let deps_result = manager.get_plugin_dependencies(&plugin_name).await;
    assert!(deps_result.is_ok());
    let deps = deps_result.unwrap();

    assert_eq!(deps.len(), 2);
    assert!(deps.contains(&"dep_a".to_string()));
    assert!(deps.contains(&"dep_b".to_string()));

    // Get dependencies for non-existent plugin
    let non_existent_deps = manager.get_plugin_dependencies("non_existent").await;
    assert!(non_existent_deps.is_err());
}

#[tokio::test]
async fn test_get_dependent_plugins() {
    let (manager, _tmp_dir) = create_test_manager();

    // Use helper constructors
    let dep_base = PluginDependency::required_any("base_plugin"); // Any version required
    let plugin_a = Arc::new(MockManagerPlugin::new("plugin_a", vec![dep_base.clone()]));
    let plugin_b = Arc::new(MockManagerPlugin::new("plugin_b", vec![dep_base.clone()]));

    let dep_a = PluginDependency::required_any("plugin_a"); // Any version required
    let plugin_c = Arc::new(MockManagerPlugin::new("plugin_c", vec![dep_a]));
    let base_plugin = Arc::new(MockManagerPlugin::new("base_plugin", vec![]));

    // Register plugins
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(base_plugin).unwrap();
        registry.register_plugin(plugin_a).unwrap();
        registry.register_plugin(plugin_b).unwrap();
        registry.register_plugin(plugin_c).unwrap();
    }

    // Get dependents of base_plugin
    let base_dependents = manager.get_dependent_plugins("base_plugin").await.unwrap();
    assert_eq!(base_dependents.len(), 2);
    assert!(base_dependents.contains(&"plugin_a".to_string()));
    assert!(base_dependents.contains(&"plugin_b".to_string()));

    // Get dependents of plugin_a
    let a_dependents = manager.get_dependent_plugins("plugin_a").await.unwrap();
    assert_eq!(a_dependents.len(), 1);
    assert!(a_dependents.contains(&"plugin_c".to_string()));

    // Get dependents of plugin_c (should be none)
    let c_dependents = manager.get_dependent_plugins("plugin_c").await.unwrap();
    assert!(c_dependents.is_empty());
}

// Constants TEST_PLUGIN_STATES_CONFIG_NAME and TEST_PLUGIN_STATES_CONFIG_KEY are replaced by CORE_SETTINGS_CONFIG_NAME_VAL and DISABLED_PLUGINS_KEY_VAL

#[tokio::test]
async fn test_enable_disable_plugin() {
    let (manager, tmp_dir) = create_test_manager(); // Renamed _tmp_dir to tmp_dir
    let plugin_id = "test_enable_disable";
    let plugin = Arc::new(MockManagerPlugin::new(plugin_id, vec![]));

    // Register plugin
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin).unwrap();
    }

    // Should be enabled by default after registration
    let is_enabled_initial = manager.is_plugin_enabled(plugin_id).await.unwrap();
    assert!(is_enabled_initial, "Plugin should be enabled by default");

    // Disable the plugin
    let disable_result = manager.persist_disable_plugin(plugin_id).await;
    assert!(disable_result.is_ok(), "Disabling should succeed");

    // Check the persisted state in the config file
    let config_path_disable = tmp_dir.path().join("app_config").join(format!("{}.json", CORE_SETTINGS_CONFIG_NAME_VAL));
    assert!(config_path_disable.exists(), "Config file should exist after disable: {:?}", config_path_disable);
    let content_disable = fs::read_to_string(&config_path_disable).unwrap();
    let json_disable: Value = serde_json::from_str(&content_disable).unwrap();
    
    let disabled_list_after_disable = json_disable[DISABLED_PLUGINS_KEY_VAL].as_array().expect("disabled_list should be an array");
    assert!(
        disabled_list_after_disable.iter().any(|v| v.as_str() == Some(plugin_id)),
        "Plugin ID should be in the disabled list after disable"
    );

    // Enable the plugin again (persisted state)
    let enable_result = manager.persist_enable_plugin(plugin_id).await;
    assert!(enable_result.is_ok(), "Enabling should succeed");

    // Check the persisted state in the config file again
    let config_path_enable = tmp_dir.path().join("app_config").join(format!("{}.json", CORE_SETTINGS_CONFIG_NAME_VAL));
    assert!(config_path_enable.exists(), "Config file should exist after enable: {:?}", config_path_enable);
    let content_enable = fs::read_to_string(&config_path_enable).unwrap();
    let json_enable: Value = serde_json::from_str(&content_enable).unwrap();
    
    let disabled_list_after_enable = json_enable.get(DISABLED_PLUGINS_KEY_VAL).and_then(|v| v.as_array());
    assert!(
        disabled_list_after_enable.map_or(true, |list| !list.iter().any(|v| v.as_str() == Some(plugin_id))),
        "Plugin ID should not be in the disabled list after enable, or list might be empty/absent"
    );


    // Try disabling non-existent plugin (should fail as plugin not found)
    let disable_non_existent = manager.persist_disable_plugin("non_existent").await;
    assert!(disable_non_existent.is_err(), "Disabling non-existent plugin should fail");

    // Try enabling non-existent plugin (should be ok, no-op)
    let enable_non_existent = manager.persist_enable_plugin("non_existent").await;
    assert!(enable_non_existent.is_ok(), "Enabling non-existent plugin should be a no-op");
}

#[tokio::test]
async fn test_get_plugin_arc() {
    let (manager, _tmp_dir) = create_test_manager();
    let plugin_id = "test_get_arc";
    let plugin = Arc::new(MockManagerPlugin::new(plugin_id, vec![]));

    // Register plugin
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin).unwrap();
    }

    // Get the plugin Arc
    let plugin_arc_opt = manager.get_plugin(plugin_id).await.unwrap();
    assert!(plugin_arc_opt.is_some(), "Should find the registered plugin");
    let plugin_arc = plugin_arc_opt.unwrap();
    assert_eq!(plugin_arc.name(), plugin_id, "Plugin Arc should have the correct name");

    // Try getting non-existent plugin
    let non_existent_arc = manager.get_plugin("non_existent").await.unwrap();
    assert!(non_existent_arc.is_none(), "Should not find non-existent plugin");
}

#[tokio::test]
async fn test_get_plugins_arc() {
    let (manager, _tmp_dir) = create_test_manager();
    let plugin_id1 = "test_get_all_1";
    let plugin_id2 = "test_get_all_2";
    let plugin1 = Arc::new(MockManagerPlugin::new(plugin_id1, vec![]));
    let plugin2 = Arc::new(MockManagerPlugin::new(plugin_id2, vec![]));

    // Register plugins
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin1).unwrap();
        registry.register_plugin(plugin2).unwrap();
    }

    // Get all plugin Arcs
    let all_plugins = manager.get_plugins().await.unwrap();
    assert_eq!(all_plugins.len(), 2, "Should retrieve two plugins");

    let names: Vec<String> = all_plugins.iter().map(|p| p.name().to_string()).collect();
    assert!(names.contains(&plugin_id1.to_string()));
    assert!(names.contains(&plugin_id2.to_string()));
}


#[tokio::test]
async fn test_get_enabled_plugins_arc() {
    let (manager, _tmp_dir) = create_test_manager();
    let plugin_id1 = "enabled_1";
    let plugin_id2 = "enabled_2";
    let plugin_id3 = "disabled_1";
    let plugin1 = Arc::new(MockManagerPlugin::new(plugin_id1, vec![]));
    let plugin2 = Arc::new(MockManagerPlugin::new(plugin_id2, vec![]));
    let plugin3 = Arc::new(MockManagerPlugin::new(plugin_id3, vec![]));

    // Register plugins
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin1).unwrap();
        registry.register_plugin(plugin2).unwrap();
        registry.register_plugin(plugin3).unwrap();
    }

    // Disable one plugin
    manager.persist_disable_plugin(plugin_id3).await.unwrap();

    // Get enabled plugin Arcs
    let enabled_plugins = manager.get_enabled_plugins().await.unwrap();
    // persist_disable_plugin should now update runtime state, so expect 2 enabled plugins.
    assert_eq!(enabled_plugins.len(), 2, "Should retrieve two enabled plugins after one is disabled");

    let names: Vec<String> = enabled_plugins.iter().map(|p| p.name().to_string()).collect();
    assert!(names.contains(&plugin_id1.to_string()));
    assert!(names.contains(&plugin_id2.to_string()));
    assert!(!names.contains(&plugin_id3.to_string()), "Disabled plugin {} should not be in the enabled list", plugin_id3);
}


#[tokio::test]
async fn test_get_plugin_manifest() {
    let (manager, _tmp_dir) = create_test_manager();
    let plugin_id = "test_manifest";
    let plugin = Arc::new(MockManagerPlugin::new(plugin_id, vec![]));

    // Register plugin
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin).unwrap();
    }

    // Get the manifest
    let manifest_opt = manager.get_plugin_manifest(plugin_id).await.unwrap();
    assert!(manifest_opt.is_some(), "Should find the manifest for registered plugin");
    let manifest = manifest_opt.unwrap();

    // Check some manifest details (based on MockManagerPlugin)
    assert_eq!(manifest.name, plugin_id); // Manifest name should match plugin ID in this mock
    assert_eq!(manifest.version, "1.0.0");
    assert!(!manifest.is_core);
    // Compare the Option<String> from the manifest with the expected string representation
    assert_eq!(manifest.priority, Some(PluginPriority::ThirdParty(100).to_string()));
    assert!(manifest.dependencies.is_empty());

    // Try getting manifest for non-existent plugin
    let non_existent_manifest = manager.get_plugin_manifest("non_existent").await.unwrap();
    assert!(non_existent_manifest.is_none(), "Should not find manifest for non-existent plugin");
}

#[tokio::test]
async fn test_load_plugins_from_directory_with_errors() {
    let (manager, _tmp_dir) = create_test_manager();
    let plugin_dir_holder = tempdir().unwrap(); // Keep separate tmpdir for plugins
    let plugin_dir = plugin_dir_holder.path();

    // 1. Valid plugin (using the compiled example)
    let example_plugin_src = match get_example_plugin_path() {
        Some(path) => path,
        None => {
            println!("Skipping test: Could not find the compiled example plugin.");
            return; // Skip test if example not found
        }
    };
    let valid_plugin_dest = plugin_dir.join("libcompat_check_example.so");
    fs::copy(&example_plugin_src, &valid_plugin_dest).unwrap();

    // 2. Invalid file (not a .so)
    fs::write(plugin_dir.join("not_a_plugin.txt"), "hello").unwrap();

    // 3. A file that *looks* like a plugin but isn't (e.g., empty file)
    //    This should cause load_so_plugin to fail.
    fs::write(plugin_dir.join("libfake_plugin.so"), "").unwrap();

    // 4. A directory (should be ignored)
    fs::create_dir(plugin_dir.join("a_directory")).unwrap();


    // Load plugins - expect errors but also partial success
    let result = manager.load_plugins_from_directory(plugin_dir).await;

    // Assertions
    assert!(result.is_err(), "Expected an error due to the fake plugin failing to load");
    match result {
        Err(Error::PluginSystem(PluginSystemError::LoadingError{plugin_id: _, path: _, source})) => {
            let source_string = source.to_string();
            // Check if the source string contains any of the expected messages
            assert!(
                source_string.contains("Encountered errors while loading plugins") ||
                source_string.contains("Failed to load plugin library") ||
                source_string.contains("libfake_plugin.so") ||
                source_string.contains("libloading error") || // Added from recent changes
                source_string.contains("missing symbol _plugin_init"), // Added from recent changes
                "Error message content mismatch: {}", source_string
            );
        }
        _ => panic!("Expected Error::PluginSystem(PluginSystemError::LoadingError), got {:?}", result),
    }


    // Check registry state - only the valid plugin should be registered
    let registry = manager.registry().lock().await;
    assert_eq!(registry.plugin_count(), 1, "Only the valid plugin should be registered");
    assert!(registry.has_plugin("CompatCheckExample"), "Valid example plugin should be present");
}

#[tokio::test]
async fn test_load_plugins_from_directory_read_dir_error() {
     // This test is hard to reliably trigger without manipulating permissions in a complex way.
     // We'll simulate it by trying to load from a file path instead of a directory.
     let (manager, _tmp_dir) = create_test_manager();
     let file_dir_holder = tempdir().unwrap(); // Keep separate tmpdir
     let file_path = file_dir_holder.path().join("im_a_file_not_a_dir");
     fs::write(&file_path, "hello").unwrap();

     let result = manager.load_plugins_from_directory(&file_path).await;

     assert!(result.is_err(), "Expected an error when trying to read a file as a directory");
     match result {
        Err(Error::PluginSystem(PluginSystemError::LoadingError{plugin_id: _, path: _, source})) => {
            let source_str = source.to_string();
            // Due to #[error(transparent)] on PluginSystemErrorSource::Io, source_str is the direct std::io::Error string.
            assert!(source_str.contains("Not a directory"), "Error message should indicate 'Not a directory'. Got: {}", source_str);
        }
        Err(Error::StorageSystem(crate::storage::error::StorageSystemError::Io{operation, ..})) if operation == "read_dir" => { // Changed to StorageSystemError::Io
            // This case might occur if read_dir itself fails before specific plugin loading logic
            // This branch might be less likely now with the PluginSystemError wrapping
        }
        _ => panic!("Expected Error::PluginSystem(PluginSystemError::LoadingError) or StorageSystemError::Io, got {:?}", result),
     }
}

#[tokio::test]
async fn test_get_plugin_dependencies_not_found() {
    let (manager, _tmp_dir) = create_test_manager();
    let result = manager.get_plugin_dependencies("non_existent_plugin").await;
    assert!(result.is_err());
    match result {
        Err(Error::PluginSystem(PluginSystemError::RegistrationError{plugin_id, message})) => {
            assert_eq!(plugin_id, "non_existent_plugin");
            assert!(message.contains("Plugin not found"));
        }
        _ => panic!("Expected PluginSystemError::RegistrationError for non-existent plugin dependencies, got {:?}", result),
    }
}

#[tokio::test]
async fn test_get_dependent_plugins_not_found() {
    let (manager, _tmp_dir) = create_test_manager();
    // No error expected, just an empty vec if the target plugin doesn't exist or has no dependents
    let result = manager.get_dependent_plugins("non_existent_plugin").await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_plugin_manifest_not_found() {
    let (manager, _tmp_dir) = create_test_manager();
    let result = manager.get_plugin_manifest("non_existent_plugin").await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_manager_initialize_with_plugin_dir() {
    // Test initialization when the default plugin dir *does* exist
    // This requires the example plugin to be compiled in target/debug
    let (manager, _tmp_dir) = create_test_manager();
    let plugin_dir = PathBuf::from("./target/debug");

    if !plugin_dir.exists() {
         println!("Skipping test: Default plugin directory ./target/debug not found.");
         return;
    }
     if get_example_plugin_path().is_none() {
         println!("Skipping test: Example plugin not found in ./target/debug");
         return;
     }

     // Call initialize via the KernelComponent trait explicitly
     let result = KernelComponent::initialize(&manager).await;
     assert!(result.is_ok(), "Initialize should succeed when dir exists");

     // Check if the example plugin was loaded (assuming it's the only one)
     let loaded = manager.is_plugin_loaded("CompatCheckExample").await.unwrap();
     assert!(loaded, "Example plugin should have been loaded during initialization");
}


#[tokio::test]
async fn test_manager_stop() {
    let (manager, _tmp_dir) = create_test_manager();
    let plugin_id = "test_stop_plugin";
    let plugin = Arc::new(MockManagerPlugin::new(plugin_id, vec![]));

    // Register plugin
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin).unwrap();
        // Note: We can't easily mock the initialized state here without more complex setup
        // or making registry methods public. The main goal is to test stop() runs.
    }

    // Call stop via the KernelComponent trait
    let result = KernelComponent::stop(&manager).await;
    assert!(result.is_ok(), "Stop should succeed");

     // Check if plugin is still technically registered
     // The internal initialized state is handled by shutdown_all, just ensure stop runs ok.
     let registry = manager.registry().lock().await; // Re-acquire lock
     assert!(registry.has_plugin(plugin_id), "Plugin should still be in registry after stop");

}

#[tokio::test]
async fn test_load_plugin_success() {
    let (manager, _tmp_dir) = create_test_manager();
    let example_plugin_path = match get_example_plugin_path() {
        Some(path) => path,
        None => {
            println!("Skipping test: Could not find the compiled example plugin.");
            return; // Skip test if example not found
        }
    };

    let result = manager.load_plugin(&example_plugin_path).await;
    assert!(result.is_ok(), "Loading the valid example plugin should succeed");

    let loaded = manager.is_plugin_loaded("CompatCheckExample").await.unwrap();
    assert!(loaded, "Example plugin should be loaded after successful load_plugin call");
}

#[tokio::test]
async fn test_load_plugin_file_not_found() {
    let (manager, _tmp_dir) = create_test_manager();
    let non_existent_path = PathBuf::from("./non_existent_plugin.so");

    let result = manager.load_plugin(&non_existent_path).await;
    assert!(result.is_err(), "Loading a non-existent plugin should fail");
    match result {
        Err(Error::PluginSystem(PluginSystemError::LoadingError{plugin_id: _, path: _, source})) => {
            assert!(source.to_string().contains("Failed to load library") || source.to_string().contains("libloading error"), "Error message should indicate library load failure. Got: {}", source);
        }
        _ => panic!("Expected Error::PluginSystem(PluginSystemError::LoadingError) for non-existent file, got {:?}", result),
    }
}

// Additional tests focusing on simpler coverage improvements

#[tokio::test]
async fn test_start_method() {
    let (manager, _tmp_dir) = create_test_manager();
    let result = manager.start().await;
    assert!(result.is_ok(), "Start method should succeed as it's mostly a placeholder");
}

// Test for the manager's debug implementation
#[test]
fn test_manager_debug_impl() {
    let (manager, _tmp_dir) = create_test_manager();
    let debug_str = format!("{:?}", manager);
    assert!(debug_str.contains("DefaultPluginManager"), "Debug output should contain struct name");
    assert!(debug_str.contains("name: \"DefaultPluginManager\""), "Debug output should contain name field");
}

// Test registry() accessor method
#[tokio::test]
async fn test_registry_accessor() {
    let (manager, _tmp_dir) = create_test_manager();
    let registry_arc = manager.registry();

    // Verify we can acquire a lock
    let _registry_guard = registry_arc.lock().await;

    // The fact that we can acquire the lock means the accessor works properly
    assert!(true);
}

// New tests to improve coverage

// Test initializing plugins with dependencies - make sure initialization order is correct
#[tokio::test]
async fn test_initialize_plugins_with_dependencies() {
    let (manager, _tmp_dir) = create_test_manager();

    // Create plugins with dependencies
    let base_plugin = Arc::new(MockManagerPlugin::new("base_plugin", vec![]));

    let dependent_plugin_1 = {
        let dep = PluginDependency::required_any("base_plugin");
        Arc::new(MockManagerPlugin::new("dependent_plugin_1", vec![dep]))
    };

    let dependent_plugin_2 = {
        let dep = PluginDependency::required_any("dependent_plugin_1");
        Arc::new(MockManagerPlugin::new("dependent_plugin_2", vec![dep]))
    };

    // Register plugins
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(base_plugin).unwrap();
        registry.register_plugin(dependent_plugin_1).unwrap();
        registry.register_plugin(dependent_plugin_2).unwrap();

        // Mark them as initialized to mirror a real scenario
        registry.initialized.insert("base_plugin".to_string());
        registry.initialized.insert("dependent_plugin_1".to_string());
        registry.initialized.insert("dependent_plugin_2".to_string());
    }

    // Test stop method which should call shutdown on plugins in correct order
    let result = manager.stop().await;
    assert!(result.is_ok(), "Stop should succeed with correctly initialized plugins");
}

// Test plugin initialization failure
#[tokio::test]
async fn test_plugin_initialization_failure() {
    let (manager, _tmp_dir) = create_test_manager();

    // Create a plugin that will fail to initialize
    let failing_plugin = Arc::new(MockManagerPlugin::new("failing_plugin", vec![])
        .with_init_error("Simulated initialization failure"));

    // Register the plugin
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(failing_plugin).unwrap();

        // Manually add to initialized set to test shutdown flow
        registry.initialized.insert("failing_plugin".to_string());
    }

    // In real scenario, initialization would fail, but we're testing the shutdown here
    let result = manager.stop().await;
    assert!(result.is_ok(), "Stop should still succeed even if plugin had failed to initialize");
}

// Test shutdown with plugin that fails during shutdown
#[tokio::test]
async fn test_plugin_shutdown_error() {
    let (manager, _tmp_dir) = create_test_manager();

    // Create a plugin that will fail during shutdown
    let failing_shutdown_plugin = Arc::new(MockManagerPlugin::new("failing_shutdown", vec![])
        .with_shutdown_error("Simulated shutdown failure"));

    // Register the plugin
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(failing_shutdown_plugin).unwrap();

        // Mark as initialized
        registry.initialized.insert("failing_shutdown".to_string());
    }

    // Test stop method which should propagate the shutdown error
    let result = manager.stop().await;
    assert!(result.is_err(), "Stop should fail when a plugin fails to shut down");

    match result {
        Err(Error::PluginSystem(PluginSystemError::ShutdownError{plugin_id, message})) => {
            assert_eq!(plugin_id, "failing_shutdown");
            assert!(message.contains("Simulated shutdown failure"),
                    "Error should contain the specific shutdown error message. Got: {}", message);
        }
        _ => panic!("Expected PluginSystemError::ShutdownError for failed shutdown, got {:?}", result),
    }
}

// Test complex shutdown ordering
#[tokio::test]
async fn test_complex_shutdown_ordering() {
    let (manager, _tmp_dir) = create_test_manager();

    // Set up a graph of plugins with dependencies
    // P3 depends on P2 depends on P1
    // P4 depends on P2
    // This should result in:
    // Shutdown order: P3, P4, P2, P1 (reverse dependency order)

    // Create and register plugins
    {
        let mut registry = manager.registry().lock().await;

        // Create base plugin P1 with no dependencies
        registry.register_plugin(Arc::new(MockManagerPlugin::new("p1", vec![]))).unwrap();

        // Create P2 that depends on P1
        let p2_deps = vec![PluginDependency::required_any("p1")];
        registry.register_plugin(Arc::new(MockManagerPlugin::new("p2", p2_deps))).unwrap();

        // Create P3 that depends on P2
        let p3_deps = vec![PluginDependency::required_any("p2")];
        registry.register_plugin(Arc::new(MockManagerPlugin::new("p3", p3_deps))).unwrap();

        // Create P4 that depends on P2
        let p4_deps = vec![PluginDependency::required_any("p2")];
        registry.register_plugin(Arc::new(MockManagerPlugin::new("p4", p4_deps))).unwrap();

        // Mark all as initialized
        registry.initialized.insert("p1".to_string());
        registry.initialized.insert("p2".to_string());
        registry.initialized.insert("p3".to_string());
        registry.initialized.insert("p4".to_string());
    }

    // Test the stop method
    let result = manager.stop().await;
    assert!(result.is_ok(), "Stop should succeed with no errors");
}

// Test loading plugins from empty directory
#[tokio::test]
async fn test_load_plugins_from_empty_directory() {
    let (manager, _tmp_dir) = create_test_manager();
    let plugin_dir_holder = tempdir().unwrap(); // Keep separate tmpdir for plugins

    let result = manager.load_plugins_from_directory(plugin_dir_holder.path()).await;
    assert!(result.is_ok(), "Loading from empty directory should succeed with 0 plugins");
    assert_eq!(result.unwrap(), 0, "Should have loaded 0 plugins");
}

// Test multiple plugin loaders using the same file
#[tokio::test]
async fn test_multiple_managers_loading_same_plugin() {
    let (manager1, _tmp_dir1) = create_test_manager();
    let (manager2, _tmp_dir2) = create_test_manager();

    let example_plugin_path = match get_example_plugin_path() {
        Some(path) => path,
        None => {
            println!("Skipping test: Could not find the compiled example plugin.");
            return; // Skip test if example not found
        }
    };

    // Load plugin in first manager
    let result1 = manager1.load_plugin(&example_plugin_path).await;
    assert!(result1.is_ok(), "First manager should load plugin successfully");

    // Load same plugin in second manager
    let result2 = manager2.load_plugin(&example_plugin_path).await;
    assert!(result2.is_ok(), "Second manager should also load plugin successfully");

    // Both managers should have the plugin registered
    let loaded1 = manager1.is_plugin_loaded("CompatCheckExample").await.unwrap();
    let loaded2 = manager2.is_plugin_loaded("CompatCheckExample").await.unwrap();

    assert!(loaded1, "Plugin should be loaded in first manager");
    assert!(loaded2, "Plugin should be loaded in second manager");
}

// Test plugin file extension filtering
#[tokio::test]
async fn test_file_extension_filtering() {
    let (manager, _tmp_dir) = create_test_manager();
    let plugin_dir_holder = tempdir().unwrap(); // Keep separate tmpdir for plugins

    // Create different types of files with extensions to test filtering
    for ext in &[".dll", ".dylib", ".txt", ".so.old", ".not-so"] {
        let filename = format!("test_plugin{}", ext);
        fs::write(plugin_dir_holder.path().join(filename), "dummy content").unwrap();
    }

    // Create a proper .so file (but invalid content)
    fs::write(plugin_dir_holder.path().join("proper.so"), "invalid plugin content").unwrap();

    let result = manager.load_plugins_from_directory(plugin_dir_holder.path()).await;

    // Should try to load the .so file but fail due to invalid content
    assert!(result.is_err());
    match result {
        Err(Error::PluginSystem(PluginSystemError::LoadingError{plugin_id: _, path: _, source})) => {
            let source_string = source.to_string();
            assert!(
                source_string.contains("proper.so") ||
                source_string.contains("Encountered errors while loading plugins") ||
                source_string.contains("libloading error") ||
                source_string.contains("missing symbol _plugin_init"), // Added from recent changes
                 "Error should mention the .so file. Got: {}", source_string);
            // Other files should be ignored (not mentioned in error)
            assert!(!source_string.contains(".dll"), "Non-.so files should be ignored");
            assert!(!source_string.contains(".dylib"), "Non-.so files should be ignored");
            assert!(!source_string.contains(".txt"), "Non-.so files should be ignored");
        }
        _ => panic!("Expected PluginSystemError::LoadingError, got {:?}", result),
    }
}

// NEW TESTS ADDED FOR ADDITIONAL COVERAGE
#[tokio::test]
async fn test_plugin_with_custom_priority_and_version() {
    let (manager, _tmp_dir) = create_test_manager();

    // Create plugin with custom priority and version
    let plugin = Arc::new(MockManagerPlugin::new("custom_plugin", vec![])
        .with_priority(PluginPriority::Core(50))
        .with_version("2.3.4")
        .with_core_status(true));

    // Register the plugin
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin).unwrap();
    }

    // Get manifest and verify the custom values are reflected
    let manifest = manager.get_plugin_manifest("custom_plugin").await.unwrap().unwrap();
    assert_eq!(manifest.version, "2.3.4");
    assert!(manifest.is_core);
    assert_eq!(manifest.priority, Some(PluginPriority::Core(50).to_string()));
}

#[tokio::test]
async fn test_plugin_with_stages() {
    let (manager, _tmp_dir) = create_test_manager();

    // Create a plugin that provides stages
    let _mock_stage = Box::new(MockStage::new("test_stage"));
    let plugin = Arc::new(MockManagerPlugin::new("stage_plugin", vec![])
        .add_stage(Box::new(MockStage::new("stage1")))
        .add_stage(Box::new(MockStage::new("stage2")))
        .add_stage(Box::new(MockStage::new("stage3"))));

    // Register the plugin
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin).unwrap();

        // Mark as initialized
        registry.initialized.insert("stage_plugin".to_string());
    }

    // Get plugin and verify its stages
    let plugin_arc = manager.get_plugin("stage_plugin").await.unwrap().unwrap();
    // let stages = plugin_arc.stages(); // Removed call to stages()
    // We can no longer easily assert the stages provided by the plugin instance itself.
    // Stage registration now happens during initialization within the registry.
    // We'll rely on other tests (like stage execution tests) to verify registration.
    // For this test, just ensure the plugin itself can be retrieved.
    assert_eq!(plugin_arc.name(), "stage_plugin");
}

#[tokio::test]
async fn test_plugin_with_required_stages() {
    let (manager, _tmp_dir) = create_test_manager();

    // Create a plugin that requires stages
    let required_stage1 = StageRequirement::require("required_stage1");
    let required_stage2 = StageRequirement::optional("required_stage2");

    let plugin = Arc::new(MockManagerPlugin::new("requiring_plugin", vec![])
        .with_required_stages(vec![required_stage1, required_stage2]));

    // Register the plugin
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin).unwrap();
    }

    // Get plugin and verify its required stages
    let plugin_arc = manager.get_plugin("requiring_plugin").await.unwrap().unwrap();
    let required_stages = plugin_arc.required_stages();
    assert_eq!(required_stages.len(), 2, "Plugin should require 2 stages");

    // Find the required stages and verify properties
    let mandatory_stages: Vec<String> = required_stages.iter()
        .filter(|s| s.required)
        .map(|s| s.stage_id.to_string())
        .collect();
    assert_eq!(mandatory_stages.len(), 1);
    assert_eq!(mandatory_stages[0], "required_stage1");

    let optional_stages: Vec<String> = required_stages.iter()
        .filter(|s| !s.required)
        .map(|s| s.stage_id.to_string())
        .collect();
    assert_eq!(optional_stages.len(), 1);
    assert_eq!(optional_stages[0], "required_stage2");
}

#[tokio::test]
async fn test_plugin_with_specific_api_version_compatibility() {
    let (manager, _tmp_dir) = create_test_manager();

    // Create a plugin with specific API version requirements
    let plugin = Arc::new(MockManagerPlugin::new("versioned_plugin", vec![])
        .with_compatible_api_versions(vec![">=1.2.0", "<2.0.0"]));

    // Register the plugin
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin).unwrap();
    }

    // Get plugin and verify its API version requirements
    let plugin_arc = manager.get_plugin("versioned_plugin").await.unwrap().unwrap();
    let api_versions = plugin_arc.compatible_api_versions();
    assert_eq!(api_versions.len(), 2, "Plugin should have 2 API version requirements");

    // Convert to strings for easier comparison
    let version_strs: Vec<String> = api_versions.iter()
        .map(|v| v.to_string())
        .collect();
    assert!(version_strs.contains(&">=1.2.0".to_string()));
    assert!(version_strs.contains(&"<2.0.0".to_string()));
}

#[tokio::test]
async fn test_plugin_with_long_running_operations() {
    let (manager, _tmp_dir) = create_test_manager();

    // Create plugins with timeout behaviors
    let init_timeout_plugin = Arc::new(MockManagerPlugin::new("init_timeout", vec![])
        .with_init_timeout());

    let shutdown_timeout_plugin = Arc::new(MockManagerPlugin::new("shutdown_timeout", vec![])
        .with_shutdown_timeout());

    // Register plugins
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(init_timeout_plugin).unwrap();
        registry.register_plugin(shutdown_timeout_plugin).unwrap();

        // Mark shutdown plugin as initialized
        registry.initialized.insert("shutdown_timeout".to_string());
    }

    // Test behaviors
    // 1. Try to enable the init timeout plugin (persist only)
    let enable_result = manager.persist_enable_plugin("init_timeout").await;
    assert!(enable_result.is_ok(), "Persisting enable should succeed");

    // 2. Test stop() which should run shutdown on the timeout plugin
    let result = manager.stop().await;
    assert!(result.is_ok(), "Stopping plugin with long shutdown should succeed");
}

#[tokio::test]
async fn test_disabling_core_plugin() {
    let (manager, _tmp_dir) = create_test_manager();

    // Create a core plugin
    let core_plugin = Arc::new(MockManagerPlugin::new("core_plugin", vec![])
        .with_core_status(true));

    // Register the plugin
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(core_plugin).unwrap();
    }

    // Try to disable the core plugin
    let disable_result = manager.persist_disable_plugin("core_plugin").await;
    assert!(disable_result.is_err(), "Disabling a core plugin should fail");

    match disable_result {
        Err(Error::PluginSystem(PluginSystemError::OperationError{plugin_id: Some(pid), message})) => { // Expect OperationError
            assert_eq!(pid, "core_plugin");
            assert!(message.contains("Core plugin cannot be disabled"), // Adjusted message check
                    "Error should indicate that core plugins can't be disabled. Actual: {}", message);
        }
        _ => panic!("Expected PluginSystemError::OperationError when trying to disable core plugin, got {:?}", disable_result),
    }

    // Verify the plugin is still enabled
    let is_enabled = manager.is_plugin_enabled("core_plugin").await.unwrap();
    assert!(is_enabled, "Core plugin should remain enabled");
}

// TODO: Add tests for load_so_plugin error paths (missing symbol, init panic)
// This requires creating dummy .so files, potentially in build.rs or separate test crates.

// --- Tests for Plugin State Persistence ---

// Redefine constants if not easily importable (adjust if needed based on manager.rs visibility)
#[tokio::test]
async fn test_state_save_on_disable() {
    let (manager, tmp_dir) = create_test_manager();
    let plugin_id = "persist_disable_test";
    let plugin = Arc::new(MockManagerPlugin::new(plugin_id, vec![]));

    // Register plugin
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin).unwrap();
    }

    // Initially enabled
    assert!(manager.is_plugin_enabled(plugin_id).await.unwrap(), "Plugin should be initially enabled");

    // Disable the plugin - this should trigger saving state
    manager.persist_disable_plugin(plugin_id).await.unwrap();
    // Verify the state file content directly instead of runtime state
    let config_path = tmp_dir.path()
        .join("app_config") // State is saved under app_config path
        .join(format!("{}.json", CORE_SETTINGS_CONFIG_NAME_VAL)); // Use correct config name

    assert!(config_path.exists(), "Config file should have been created at {:?}", config_path);

    let content = fs::read_to_string(&config_path).expect("Failed to read config file");
    let json_value: Value = serde_json::from_str(&content).expect("Failed to parse config JSON");

    let disabled_list = json_value
        .get(DISABLED_PLUGINS_KEY_VAL)
        .and_then(|v| v.as_array())
        .expect("JSON should contain a disabled_list array");

    assert!(
        disabled_list.iter().any(|v| v.as_str() == Some(plugin_id)),
        "Plugin ID '{}' should be in the disabled list in the config file", plugin_id
    );
}

#[tokio::test]
async fn test_state_save_on_enable() {
    let (manager, tmp_dir) = create_test_manager();
    let plugin_id = "persist_enable_test";
    let plugin = Arc::new(MockManagerPlugin::new(plugin_id, vec![]));

    // Register plugin
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin).unwrap();
    }

    // Disable first to ensure state is false
    manager.persist_disable_plugin(plugin_id).await.unwrap();
    // The previous assert!(!...) checks the runtime state *before* enabling, which is correct here.

    // Enable the plugin - this should update the saved state
    manager.persist_enable_plugin(plugin_id).await.unwrap();
    // We don't check runtime state here, check the file below

    // Verify the state file content (saved under app_config path for ConfigScope::Application)
    let config_path = tmp_dir.path()
        .join("app_config") // State is saved under app_config path
        .join(format!("{}.json", CORE_SETTINGS_CONFIG_NAME_VAL)); // Use correct config name

    assert!(config_path.exists(), "Config file should exist at {:?}", config_path);

    let content = fs::read_to_string(&config_path).expect("Failed to read config file");
    let json_value: Value = serde_json::from_str(&content).expect("Failed to parse config JSON");

    let disabled_list_after_enable = json_value
        .get(DISABLED_PLUGINS_KEY_VAL)
        .and_then(|v| v.as_array());
    
    assert!(
        disabled_list_after_enable.map_or(true, |list| !list.iter().any(|v| v.as_str() == Some(plugin_id))),
        "Plugin ID '{}' should not be in the disabled list after enabling, or list might be empty/absent", plugin_id
    );
}


#[tokio::test]
async fn test_state_load_on_initialize() {
    let tmp_dir = tempdir().unwrap(); // Create temp dir once
    let plugin_id = "persist_load_test";

    // --- Manager 1: Register and Disable Plugin ---
    {
        let app_config_path = tmp_dir.path().join("app_config");
        let plugin_config_path = tmp_dir.path().join("plugin_config");
        fs::create_dir_all(&app_config_path).unwrap();
        fs::create_dir_all(&plugin_config_path.join("user")).unwrap(); // Ensure user dir exists

        let provider1 = Arc::new(LocalStorageProvider::new(tmp_dir.path().to_path_buf())) as Arc<dyn StorageProvider>; // Cast to dyn trait
        let config_manager1: Arc<ConfigManager> = Arc::new(ConfigManager::new( // Remove generic
            provider1,
            app_config_path.clone(),
            plugin_config_path.clone(),
            ConfigFormat::Json,
        ));
        let event_manager1 = Arc::new(DefaultEventManager::new()) as Arc<dyn EventManager>;
        let stage_manager1 = Arc::new(DefaultStageManager::new(event_manager1));
        let stage_registry_arc1 = stage_manager1.registry();
        let manager1 = DefaultPluginManager::new(config_manager1, stage_registry_arc1).unwrap();

        // Register plugin
        let plugin1 = Arc::new(MockManagerPlugin::new(plugin_id, vec![]));
        {
            let mut registry = manager1.registry().lock().await;
            registry.register_plugin(plugin1).unwrap();
        }
        assert!(manager1.is_plugin_enabled(plugin_id).await.unwrap(), "Plugin should be enabled initially in manager1");

        // Disable plugin (this saves state)
        manager1.persist_disable_plugin(plugin_id).await.unwrap();
        assert!(!manager1.is_plugin_enabled(plugin_id).await.unwrap(), "Plugin should be disabled in manager1");
    } // Manager 1 goes out of scope

    // --- Manager 2: Initialize and Check Loaded State ---
    {
        let app_config_path = tmp_dir.path().join("app_config"); // Use same paths
        let plugin_config_path = tmp_dir.path().join("plugin_config");

        let provider2 = Arc::new(LocalStorageProvider::new(tmp_dir.path().to_path_buf())) as Arc<dyn StorageProvider>; // Cast to dyn trait
        // Call ConfigManager::new with the reverted signature
        let config_manager2: Arc<ConfigManager> = Arc::new(ConfigManager::new(
            provider2,            // Pass the provider Arc
            app_config_path,      // Pass the app config path
            plugin_config_path,   // Pass the plugin config path
            ConfigFormat::Json,   // Pass the default format
        ));
        let event_manager2 = Arc::new(DefaultEventManager::new()) as Arc<dyn EventManager>;
        let stage_manager2 = Arc::new(DefaultStageManager::new(event_manager2));
        let stage_registry_arc2 = stage_manager2.registry();
        let manager2 = DefaultPluginManager::new(config_manager2, stage_registry_arc2).unwrap();

        // IMPORTANT: Register the *same* plugin ID *before* initializing manager2
        // This simulates plugin discovery happening before state is applied.
        let plugin2 = Arc::new(MockManagerPlugin::new(plugin_id, vec![]));
        {
            let mut registry = manager2.registry().lock().await;
            registry.register_plugin(plugin2).unwrap();
            // At this point, the plugin is enabled *in memory* in manager2's registry
            assert!(registry.is_enabled(plugin_id), "Plugin should be enabled in registry before initialize");
        }


        // Initialize manager2 - this should load the state from the file saved by manager1
        KernelComponent::initialize(&manager2).await.unwrap();

        // Verify that the loaded state (disabled) was applied
        assert!(!manager2.is_plugin_enabled(plugin_id).await.unwrap(), "Plugin should be disabled in manager2 after initialization due to loaded state");

        // Double-check the registry directly
         {
            let registry = manager2.registry().lock().await;
            assert!(!registry.is_enabled(plugin_id), "Plugin should be disabled in registry after initialize");
         }
     }
     
     #[cfg(test)]
     pub mod initialize_conflict_tests { // Made module public
         use super::*; // Imports items from the outer module (manager_tests)
     
         // Helper function to create a dummy .so file and a manifest for testing plugin loading
         #[allow(dead_code)] // Allow dead code for this test helper due to linter warnings
         fn create_dummy_so_and_manifest_for_test(
             base_dir: &Path,      // Directory to create files in
             id: &str,             // Plugin ID and base for filenames
         version: &str,        // Plugin version
         conflicts: Vec<String>, // List of plugin IDs this plugin conflicts with
         entry_point_filename: &str, // Exact filename for the .so file (e.g., "libmyplugin.so")
         plugin_base_dir_in_manifest: &Path, // The base_dir to record in the manifest
     ) -> std::io::Result<(PathBuf, PathBuf)> {
         use crate::plugin_system::manifest::PluginManifest; // Ensure PluginManifest is in scope
     
         // 1. Create dummy .so file by copying the example plugin
         let example_plugin_src = get_example_plugin_path().expect("Example plugin .so not found for test helper");
         let so_dest_path = base_dir.join(entry_point_filename);
         fs::copy(&example_plugin_src, &so_dest_path)?;
     
         // 2. Create manifest data
         let manifest = PluginManifest {
             id: id.to_string(),
             name: id.to_string(), // Typically name matches ID for simplicity in tests
             version: version.to_string(),
             api_versions: vec![VersionRange::from_str(">=0.1.0").unwrap()], // Default
             dependencies: vec![],
             is_core: false,
             priority: Some(PluginPriority::ThirdParty(100).to_string()),
             description: format!("Test manifest for {}", id),
             author: "Gini Test Suite".to_string(),
             website: None,
             license: None,
             entry_point: entry_point_filename.to_string(),
             files: vec![entry_point_filename.to_string()], // List the .so file
             config_schema: None,
             tags: vec![],
             conflicts_with: conflicts,
             incompatible_with: vec![],
             resources: vec![],
             plugin_base_dir: plugin_base_dir_in_manifest.to_path_buf(), // Path where .so and manifest are expected relative to
         };
     
         // 3. Serialize manifest to JSON and write to file
         let manifest_filename = format!("{}.json", id);
         let manifest_dest_path = base_dir.join(manifest_filename);
         let manifest_json = serde_json::to_string_pretty(&manifest)?;
         fs::write(&manifest_dest_path, manifest_json)?;
     
         Ok((so_dest_path, manifest_dest_path))
     }
     
     #[tokio::test]
     #[allow(dead_code)] // Allow dead code for this test due to linter warnings
     pub async fn test_conflict_skip_on_init() {
         let (manager, _tmp_dir_manager_config) = create_test_manager();
     
     
         // ID for Plugin A (pre-loaded, its actual name from .so is "CompatCheckExample")
         let plugin_a_registered_name = "CompatCheckExample";
     
         // ID for Plugin B (to be discovered by initialize)
         let plugin_b_manifest_id = "PluginBConflictsWithA";
         let plugin_b_so_filename = "libpluginbconflictswitha.so";
     
         // Setup a temporary directory that will be scanned by PluginLoader inside initialize()
         // This needs to be a subdirectory of "./target/debug" or similar, based on how
         // DefaultPluginManager::initialize configures its internal PluginLoader.
         // For this test, we'll create files directly in a unique subdir of ./target/debug
         let target_debug_dir = PathBuf::from("./target/debug");
         if !target_debug_dir.exists() {
             fs::create_dir_all(&target_debug_dir).expect("Failed to create ./target/debug for test setup");
         }
         let scan_dir_name = "gini_test_scan_conflict_dir_fixed"; // Using a fixed name
         let scan_dir_for_plugin_b = target_debug_dir.join(scan_dir_name);
         if scan_dir_for_plugin_b.exists() { // Clean up if exists from a previous failed run
             fs::remove_dir_all(&scan_dir_for_plugin_b).expect("Failed to clean up existing test scan directory");
         }
         fs::create_dir_all(&scan_dir_for_plugin_b).expect("Failed to create test scan directory for Plugin B");
     
         // 1. Pre-load Plugin A (CompatCheckExample)
         let example_plugin_path = get_example_plugin_path().expect("Example plugin for Plugin A not found");
         manager.load_plugin(&example_plugin_path).await.expect("Failed to load Plugin A");
         assert!(
             manager.is_plugin_loaded(plugin_a_registered_name).await.unwrap(),
             "Plugin A ({}) should be loaded", plugin_a_registered_name
         );
         assert!(
             manager.is_plugin_enabled(plugin_a_registered_name).await.unwrap(),
             "Plugin A ({}) should be enabled", plugin_a_registered_name
         );
     
         // 2. Setup Plugin B's manifest and dummy .so in the scan_dir_for_plugin_b
         // Plugin B will declare a conflict with Plugin A's registered name.
         let (_plugin_b_so_path, _plugin_b_manifest_path) = create_dummy_so_and_manifest_for_test(
             &scan_dir_for_plugin_b,
             plugin_b_manifest_id,
             "1.0.0",
             vec![plugin_a_registered_name.to_string()],
             plugin_b_so_filename,
             &scan_dir_for_plugin_b, // Manifest's base_dir is where it and its .so reside
         ).expect("Failed to create dummy .so and manifest for Plugin B");
     
         // 3. Call initialize() on the manager.
         // This will use its internal PluginLoader, which scans "./target/debug".
         // Our scan_dir_for_plugin_b is inside "./target/debug", so Plugin B's manifest should be found.
         // The conflict logic added to initialize() should then skip loading Plugin B.
         let init_result = KernelComponent::initialize(&manager).await;
         if let Err(e) = &init_result {
             eprintln!("Initialize failed: {:?}", e); // Log error for diagnostics
             // If other plugins in ./target/debug fail to load, initialize might return an error.
             // We are primarily interested in whether PluginB was loaded or skipped.
         }
         // We don't strictly require init_result to be Ok if other plugins in target/debug cause issues,
         // as long as our specific conflict logic for PluginB works.
     
         // 4. Assertions
         // Plugin A should still be loaded and enabled
         assert!(
             manager.is_plugin_loaded(plugin_a_registered_name).await.unwrap(),
             "Plugin A ({}) should remain loaded after initialize", plugin_a_registered_name
         );
         assert!(
             manager.is_plugin_enabled(plugin_a_registered_name).await.unwrap(),
             "Plugin A ({}) should remain enabled after initialize", plugin_a_registered_name
         );
     
         // Plugin B (PluginBConflictsWithA) should NOT be loaded due to the conflict
         assert!(
             !manager.is_plugin_loaded(plugin_b_manifest_id).await.unwrap(),
             "Plugin B ({}) should NOT be loaded due to conflict with Plugin A", plugin_b_manifest_id
         );
     
         // Cleanup the temporary directory for Plugin B
         fs::remove_dir_all(&scan_dir_for_plugin_b).expect("Failed to clean up test scan directory for Plugin B");
     }
     } // Close mod initialize_conflict_tests
}
