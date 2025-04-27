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
async fn test_enable_disable_plugin() {
    let manager = DefaultPluginManager::new().unwrap();
    let plugin_id = "test_enable_disable";
    let plugin = Box::new(MockManagerPlugin::new(plugin_id, vec![]));

    // Register plugin
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin).unwrap();
    }

    // Should be enabled by default after registration
    let is_enabled_initial = manager.is_plugin_enabled(plugin_id).await.unwrap();
    assert!(is_enabled_initial, "Plugin should be enabled by default");

    // Disable the plugin
    let disable_result = manager.disable_plugin(plugin_id).await;
    assert!(disable_result.is_ok(), "Disabling should succeed");

    // Check if disabled
    let is_enabled_after_disable = manager.is_plugin_enabled(plugin_id).await.unwrap();
    assert!(!is_enabled_after_disable, "Plugin should be disabled");

    // Enable the plugin again
    let enable_result = manager.enable_plugin(plugin_id).await;
    assert!(enable_result.is_ok(), "Enabling should succeed");

    // Check if enabled
    let is_enabled_after_enable = manager.is_plugin_enabled(plugin_id).await.unwrap();
    assert!(is_enabled_after_enable, "Plugin should be enabled again");

    // Try disabling non-existent plugin (should be ok, no-op)
    let disable_non_existent = manager.disable_plugin("non_existent").await;
    assert!(disable_non_existent.is_ok(), "Disabling non-existent plugin should be a no-op");

    // Try enabling non-existent plugin (should fail)
    let enable_non_existent = manager.enable_plugin("non_existent").await;
    assert!(enable_non_existent.is_err(), "Enabling non-existent plugin should fail");
}

#[tokio::test]
async fn test_get_plugin_arc() {
    let manager = DefaultPluginManager::new().unwrap();
    let plugin_id = "test_get_arc";
    let plugin = Box::new(MockManagerPlugin::new(plugin_id, vec![]));

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
    let manager = DefaultPluginManager::new().unwrap();
    let plugin_id1 = "test_get_all_1";
    let plugin_id2 = "test_get_all_2";
    let plugin1 = Box::new(MockManagerPlugin::new(plugin_id1, vec![]));
    let plugin2 = Box::new(MockManagerPlugin::new(plugin_id2, vec![]));

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
    let manager = DefaultPluginManager::new().unwrap();
    let plugin_id1 = "enabled_1";
    let plugin_id2 = "enabled_2";
    let plugin_id3 = "disabled_1";
    let plugin1 = Box::new(MockManagerPlugin::new(plugin_id1, vec![]));
    let plugin2 = Box::new(MockManagerPlugin::new(plugin_id2, vec![]));
    let plugin3 = Box::new(MockManagerPlugin::new(plugin_id3, vec![]));

    // Register plugins
    {
        let mut registry = manager.registry().lock().await;
        registry.register_plugin(plugin1).unwrap();
        registry.register_plugin(plugin2).unwrap();
        registry.register_plugin(plugin3).unwrap();
    }

    // Disable one plugin
    manager.disable_plugin(plugin_id3).await.unwrap();

    // Get enabled plugin Arcs
    let enabled_plugins = manager.get_enabled_plugins().await.unwrap();
    assert_eq!(enabled_plugins.len(), 2, "Should retrieve two enabled plugins");

    let names: Vec<String> = enabled_plugins.iter().map(|p| p.name().to_string()).collect();
    assert!(names.contains(&plugin_id1.to_string()));
    assert!(names.contains(&plugin_id2.to_string()));
    assert!(!names.contains(&plugin_id3.to_string()));
}


#[tokio::test]
async fn test_get_plugin_manifest() {
    let manager = DefaultPluginManager::new().unwrap();
    let plugin_id = "test_manifest";
    let plugin = Box::new(MockManagerPlugin::new(plugin_id, vec![]));

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