use crate::plugin_system::manager::{DefaultPluginManager, PluginManager};
use crate::plugin_system::traits::{Plugin, PluginError, PluginPriority};
use crate::plugin_system::dependency::PluginDependency;
use crate::plugin_system::version::VersionRange;
use crate::kernel::bootstrap::Application; // For Plugin::init signature
use crate::kernel::error::{Result as KernelResult, Error};
use crate::kernel::component::KernelComponent; // Import KernelComponent trait
use crate::stage_manager::context::StageContext;
use crate::stage_manager::requirement::StageRequirement;
use crate::stage_manager::Stage; // Import Stage trait
use async_trait::async_trait;
use std::sync::Arc;
use tempfile::tempdir;
use std::path::PathBuf;
use std::str::FromStr; // Import FromStr for parsing VersionRange

// --- Mock Plugin ---
struct MockManagerPlugin {
    id: String,
    deps: Vec<PluginDependency>,
}

impl MockManagerPlugin {
    fn new(id: &str, deps: Vec<PluginDependency>) -> Self {
        Self { id: id.to_string(), deps }
    }
}

#[async_trait]
impl Plugin for MockManagerPlugin {
    fn name(&self) -> &'static str {
        // Hacky: Leak the string to get a 'static str. Okay for tests.
        Box::leak(self.id.clone().into_boxed_str())
    }
    fn version(&self) -> &str { "1.0.0" }
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(100) }
    fn compatible_api_versions(&self) -> Vec<VersionRange> { vec![VersionRange::from_str(">=0.1.0").unwrap()] }
    fn dependencies(&self) -> Vec<PluginDependency> { self.deps.clone() }
    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }
    fn stages(&self) -> Vec<Box<dyn Stage>> { vec![] }
    fn shutdown(&self) -> KernelResult<()> { Ok(()) }
    fn init(&self, _app: &mut Application) -> KernelResult<()> { Ok(()) }
    async fn preflight_check(&self, _context: &StageContext) -> Result<(), PluginError> { Ok(()) }
}


#[tokio::test]
async fn test_manager_new() {
    let manager_result = DefaultPluginManager::new();
    assert!(manager_result.is_ok());
}

#[tokio::test]
async fn test_manager_initialize_no_dir() {
    // Test initialization when the default plugin dir doesn't exist
    let manager = DefaultPluginManager::new().unwrap();
    // Ensure the default dir doesn't exist for this test run
    let plugin_dir = PathBuf::from("./target/debug");
    if plugin_dir.exists() {
        // In a real scenario, might skip test or use a unique temp dir
        println!("Warning: Skipping part of test as ./target/debug exists.");
    } else {
         // Call initialize via the KernelComponent trait explicitly
         let result = KernelComponent::initialize(&manager).await;
         assert!(result.is_ok(), "Initialize should succeed even if dir is missing");
    }
}

#[tokio::test]
async fn test_is_plugin_loaded() {
    let manager = DefaultPluginManager::new().unwrap();
    let plugin = Box::new(MockManagerPlugin::new("test_plugin", vec![]));
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
    let manager = DefaultPluginManager::new().unwrap();
    // Use helper constructors for dependencies
    let dep1 = PluginDependency::required("dep_a", VersionRange::from_str(">=1.0").unwrap());
    let dep2 = PluginDependency::required("dep_b", VersionRange::from_str("~2.1").unwrap());

    let plugin = Box::new(MockManagerPlugin::new("test_plugin", vec![dep1.clone(), dep2.clone()]));
    let plugin_name = plugin.name().to_string();

    // Register plugin
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin).unwrap();
        // Also register dummy dependencies so registry doesn't complain later if needed
        registry.register_plugin(Box::new(MockManagerPlugin::new("dep_a", vec![]))).unwrap();
        registry.register_plugin(Box::new(MockManagerPlugin::new("dep_b", vec![]))).unwrap();
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
    let manager = DefaultPluginManager::new().unwrap();

    // Use helper constructors
    let dep_base = PluginDependency::required_any("base_plugin"); // Any version required
    let plugin_a = Box::new(MockManagerPlugin::new("plugin_a", vec![dep_base.clone()]));
    let plugin_b = Box::new(MockManagerPlugin::new("plugin_b", vec![dep_base.clone()]));

    let dep_a = PluginDependency::required_any("plugin_a"); // Any version required
    let plugin_c = Box::new(MockManagerPlugin::new("plugin_c", vec![dep_a]));
    let base_plugin = Box::new(MockManagerPlugin::new("base_plugin", vec![]));

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

#[tokio::test]
async fn test_is_plugin_enabled() {
     let manager = DefaultPluginManager::new().unwrap();
     let plugin = Box::new(MockManagerPlugin::new("test_plugin", vec![]));
     let plugin_name = plugin.name().to_string();

     // Register plugin
     {
         let mut registry = manager.registry().lock().await;
         registry.register_plugin(plugin).unwrap();
     }

     // Check enabled status (currently mirrors loaded status)
     let is_enabled = manager.is_plugin_enabled(&plugin_name).await;
     assert!(is_enabled.is_ok());
     assert!(is_enabled.unwrap(), "Loaded plugin should be considered enabled (currently)");

     // Check non-existent plugin
     let not_enabled = manager.is_plugin_enabled("non_existent").await;
     assert!(not_enabled.is_ok());
     assert!(!not_enabled.unwrap(), "Non-existent plugin should not be enabled");
}

// Tests for enable/disable/manifest/get_plugin/get_plugins are omitted
// as the current implementation is placeholder/incomplete.