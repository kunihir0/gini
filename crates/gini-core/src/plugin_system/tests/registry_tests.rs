use super::super::registry::PluginRegistry;
use crate::plugin_system::version::{ApiVersion, VersionRange};
use crate::plugin_system::traits::{Plugin, PluginPriority, PluginError};
use crate::plugin_system::dependency::PluginDependency;
use crate::kernel::bootstrap::Application;
use crate::kernel::error::{Result as KernelResult, Error};
use crate::stage_manager::context::StageContext;
use crate::stage_manager::requirement::StageRequirement;
// Removed unused: use crate::stage_manager::Stage;
use crate::stage_manager::registry::StageRegistry;
use async_trait::async_trait;
use std::str::FromStr;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}}; // Add Arc here
use std::collections::HashSet;
use tokio::sync::Mutex; // Add Mutex

// --- Mock Plugin for Registry Tests ---
struct MockRegistryPlugin {
    id: String,
    version: String,
    deps: Vec<PluginDependency>,
    compatible_apis: Vec<VersionRange>,
    init_called: std::sync::Arc<AtomicBool>,
    shutdown_called: std::sync::Arc<AtomicBool>,
}

impl MockRegistryPlugin {
    fn new(id: &str, version: &str, deps: Vec<PluginDependency>, compatible_apis: Vec<VersionRange>) -> Self {
        Self {
            id: id.to_string(),
            version: version.to_string(),
            deps,
            compatible_apis,
            init_called: std::sync::Arc::new(AtomicBool::new(false)),
            shutdown_called: std::sync::Arc::new(AtomicBool::new(false)),
        }
    }
     fn default(id: &str) -> Self {
         Self::new(
             id,
             "1.0.0",
             vec![],
             vec![VersionRange::from_str(">=0.1.0").unwrap()]
         )
     }
}

#[async_trait]
impl Plugin for MockRegistryPlugin {
    fn name(&self) -> &'static str {
        Box::leak(self.id.clone().into_boxed_str())
    }
    fn version(&self) -> &str { &self.version }
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(151) }
    fn compatible_api_versions(&self) -> Vec<VersionRange> { self.compatible_apis.clone() }
    fn dependencies(&self) -> Vec<PluginDependency> { self.deps.clone() }
    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }
    fn init(&self, _app: &mut Application) -> KernelResult<()> {
        println!("MockRegistryPlugin '{}' init called.", self.id);
        self.init_called.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn shutdown(&self) -> KernelResult<()> {
         println!("MockRegistryPlugin '{}' shutdown called.", self.id);
         self.shutdown_called.store(true, Ordering::SeqCst);
         Ok(())
    }
    async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), PluginError> { Ok(()) }
    fn register_stages(&self, _registry: &mut StageRegistry) -> KernelResult<()> { Ok(()) } // Added
// Add default implementations for new trait methods
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
}

fn create_test_registry() -> PluginRegistry {
    PluginRegistry::new(ApiVersion::from_str("0.1.0").unwrap())
}

fn create_mock_app() -> Application {
     // Create a minimal Application instance for testing init/shutdown
     Application::new().expect("Failed to create mock application")
 }
 
 // Helper to create a mock StageRegistry Arc for tests
 fn create_mock_stage_registry_arc() -> Arc<Mutex<StageRegistry>> {
     Arc::new(Mutex::new(StageRegistry::new()))
 }
 
 
 #[cfg(test)]
 mod tests {
    use super::*;
    // Removed unused: use tokio::test;

    #[tokio::test] // Use tokio::test
    async fn test_registry_new() { // Add async
        let registry = create_test_registry();
        assert_eq!(registry.plugin_count(), 0);
        assert_eq!(registry.initialized_count(), 0);
        assert!(registry.enabled.is_empty());
        assert_eq!(registry.api_version().to_string(), "0.1.0");
    }

    #[tokio::test] // Use tokio::test
    async fn test_register_plugin_success() { // Add async
        let mut registry = create_test_registry();
        let plugin = Box::new(MockRegistryPlugin::default("plugin1"));
        let plugin_id = plugin.name().to_string();

        let result = registry.register_plugin(plugin);
        assert!(result.is_ok());
        assert_eq!(registry.plugin_count(), 1);
        assert!(registry.has_plugin(&plugin_id));
        assert!(registry.is_enabled(&plugin_id), "Plugin should be enabled by default");
        assert!(!registry.initialized.contains(&plugin_id), "Plugin should not be initialized yet");
    }

    #[tokio::test] // Use tokio::test
    async fn test_register_duplicate_plugin() { // Add async
        let mut registry = create_test_registry();
        registry.register_plugin(Box::new(MockRegistryPlugin::default("plugin1"))).unwrap();
        let result = registry.register_plugin(Box::new(MockRegistryPlugin::default("plugin1")));
        assert!(result.is_err());
        assert_eq!(registry.plugin_count(), 1); // Count should remain 1
    }

     #[tokio::test] // Use tokio::test
     async fn test_register_incompatible_api() { // Add async
         let mut registry = PluginRegistry::new(ApiVersion::from_str("1.0.0").unwrap()); // API v1.0.0
         // Plugin compatible with >=0.1.0, <0.2.0
         let plugin = Box::new(MockRegistryPlugin::new(
             "incompatible_plugin",
             "1.0.0",
             vec![],
             vec![VersionRange::from_str(">=0.1.0, <0.2.0").unwrap()]
         ));
         let result = registry.register_plugin(plugin);
         assert!(result.is_err(), "Should fail due to API incompatibility");
         assert_eq!(registry.plugin_count(), 0);
     }

    #[tokio::test] // Use tokio::test
    async fn test_unregister_plugin() { // Add async
        let mut registry = create_test_registry();
        let plugin_id = "plugin_to_unregister";
        registry.register_plugin(Box::new(MockRegistryPlugin::default(plugin_id))).unwrap();
        assert!(registry.has_plugin(plugin_id));
        assert!(registry.is_enabled(plugin_id));

        let result = registry.unregister_plugin(plugin_id);
        assert!(result.is_ok());
        assert_eq!(registry.plugin_count(), 0);
        assert!(!registry.has_plugin(plugin_id));
        assert!(!registry.is_enabled(plugin_id)); // Should also be removed from enabled set

        // Try unregistering again
        let result2 = registry.unregister_plugin(plugin_id);
        assert!(result2.is_err());
    }

    #[tokio::test] // Use tokio::test
    async fn test_get_plugin() { // Add async
        let mut registry = create_test_registry();
        let plugin_id = "plugin_to_get";
        registry.register_plugin(Box::new(MockRegistryPlugin::default(plugin_id))).unwrap();

        let plugin_arc_opt = registry.get_plugin(plugin_id);
        assert!(plugin_arc_opt.is_some());
        assert_eq!(plugin_arc_opt.unwrap().name(), plugin_id);

        let non_existent = registry.get_plugin("non_existent");
        assert!(non_existent.is_none());
    }

    #[tokio::test] // Use tokio::test
    async fn test_get_plugins_arc() { // Add async
        let mut registry = create_test_registry();
        registry.register_plugin(Box::new(MockRegistryPlugin::default("plugin1"))).unwrap();
        registry.register_plugin(Box::new(MockRegistryPlugin::default("plugin2"))).unwrap();

        let plugins = registry.get_plugins_arc();
        assert_eq!(plugins.len(), 2);
        let names: HashSet<String> = plugins.iter().map(|p| p.name().to_string()).collect();
        assert!(names.contains("plugin1"));
        assert!(names.contains("plugin2"));
    }

    #[tokio::test] // Use tokio::test
    async fn test_enable_disable_is_enabled() { // Add async
        let mut registry = create_test_registry();
        let plugin_id = "plugin_enable_disable";
        registry.register_plugin(Box::new(MockRegistryPlugin::default(plugin_id))).unwrap();

        assert!(registry.is_enabled(plugin_id), "Should be enabled initially");

        // Disable
        let disable_res = registry.disable_plugin(plugin_id);
        assert!(disable_res.is_ok());
        assert!(!registry.is_enabled(plugin_id), "Should be disabled");

        // Disable again (no-op)
        let disable_res2 = registry.disable_plugin(plugin_id);
         assert!(disable_res2.is_ok()); // Should still be ok
         assert!(!registry.is_enabled(plugin_id), "Should remain disabled");


        // Enable
        let enable_res = registry.enable_plugin(plugin_id);
        assert!(enable_res.is_ok());
        assert!(registry.is_enabled(plugin_id), "Should be enabled again");

        // Enable non-existent
        let enable_non_existent = registry.enable_plugin("non_existent");
        assert!(enable_non_existent.is_err());

        // Disable non-existent (no-op)
        let disable_non_existent = registry.disable_plugin("non_existent");
        assert!(disable_non_existent.is_ok());
    }

    #[tokio::test] // Use tokio::test
    async fn test_get_enabled_plugins_arc() { // Add async
        let mut registry = create_test_registry();
        registry.register_plugin(Box::new(MockRegistryPlugin::default("plugin1"))).unwrap();
        registry.register_plugin(Box::new(MockRegistryPlugin::default("plugin2"))).unwrap();
        registry.register_plugin(Box::new(MockRegistryPlugin::default("plugin3"))).unwrap();

        registry.disable_plugin("plugin2").unwrap();

        let enabled_plugins = registry.get_enabled_plugins_arc();
        assert_eq!(enabled_plugins.len(), 2);
        let names: HashSet<String> = enabled_plugins.iter().map(|p| p.name().to_string()).collect();
        assert!(names.contains("plugin1"));
        assert!(!names.contains("plugin2"));
        assert!(names.contains("plugin3"));
    }

     #[tokio::test] // Make test async
     async fn test_initialize_plugin_basic() {
         let mut registry = create_test_registry();
         let mut app = create_mock_app();
         let stage_registry_arc = create_mock_stage_registry_arc(); // Create mock stage registry
         let plugin_id = "init_plugin";
         let plugin = Box::new(MockRegistryPlugin::default(plugin_id));
         let init_flag = plugin.init_called.clone();
         registry.register_plugin(plugin).unwrap();
 
         assert!(!init_flag.load(Ordering::SeqCst));
         assert!(!registry.initialized.contains(plugin_id));
 
         // Pass the stage_registry_arc
         let result = registry.initialize_plugin(plugin_id, &mut app, &stage_registry_arc).await;
         assert!(result.is_ok());
         assert!(init_flag.load(Ordering::SeqCst), "Plugin init method should have been called");
         assert!(registry.initialized.contains(plugin_id), "Plugin should be marked as initialized");
 
         // Initialize again (should be no-op)
         let init_flag_before = init_flag.load(Ordering::SeqCst);
         // Pass the stage_registry_arc again
         let result2 = registry.initialize_plugin(plugin_id, &mut app, &stage_registry_arc).await;
         assert!(result2.is_ok());
         assert_eq!(init_flag.load(Ordering::SeqCst), init_flag_before, "Init should not be called again");
     }

     #[tokio::test] // Make test async
     async fn test_initialize_disabled_plugin_skips() { // Make test async
         let mut registry = create_test_registry();
         let mut app = create_mock_app();
         let plugin_id = "init_disabled";
         let plugin = Box::new(MockRegistryPlugin::default(plugin_id));
         let init_flag = plugin.init_called.clone();
         registry.register_plugin(plugin).unwrap();

         registry.disable_plugin(plugin_id).unwrap(); // Disable it
         let stage_registry_arc = create_mock_stage_registry_arc(); // Create mock stage registry
 
         // Pass the stage_registry_arc
         let result = registry.initialize_plugin(plugin_id, &mut app, &stage_registry_arc).await;
         assert!(result.is_ok(), "Initializing a disabled plugin should succeed (but be a no-op)");
         assert!(!init_flag.load(Ordering::SeqCst), "Init should NOT be called for disabled plugin");
         assert!(!registry.initialized.contains(plugin_id), "Disabled plugin should not be marked initialized");
     }

     #[tokio::test] // Make test async
     async fn test_initialize_all_enabled_only() { // Make test async
         let mut registry = create_test_registry();
         let mut app = create_mock_app();
         let plugin1 = Box::new(MockRegistryPlugin::default("plugin1"));
         let plugin2 = Box::new(MockRegistryPlugin::default("plugin2"));
         let plugin3 = Box::new(MockRegistryPlugin::default("plugin3"));
         let init1 = plugin1.init_called.clone();
         let init2 = plugin2.init_called.clone();
         let init3 = plugin3.init_called.clone();
         registry.register_plugin(plugin1).unwrap();
         registry.register_plugin(plugin2).unwrap();
         registry.register_plugin(plugin3).unwrap();

         registry.disable_plugin("plugin2").unwrap(); // Disable plugin2
         let stage_registry_arc = create_mock_stage_registry_arc(); // Create mock stage registry
 
         // Pass the stage_registry_arc
         let result = registry.initialize_all(&mut app, &stage_registry_arc).await;
         assert!(result.is_ok());
 
         assert!(init1.load(Ordering::SeqCst), "Plugin1 should be initialized");
         assert!(!init2.load(Ordering::SeqCst), "Plugin2 should NOT be initialized");
         assert!(init3.load(Ordering::SeqCst), "Plugin3 should be initialized");

         assert!(registry.initialized.contains("plugin1"));
         assert!(!registry.initialized.contains("plugin2"));
         assert!(registry.initialized.contains("plugin3"));
         assert_eq!(registry.initialized_count(), 2);
     }


     #[tokio::test] // Make test async
     async fn test_shutdown_all() { // Make test async
         let mut registry = create_test_registry();
         let mut app = create_mock_app();
         let plugin1 = Box::new(MockRegistryPlugin::default("plugin1"));
         let plugin2 = Box::new(MockRegistryPlugin::default("plugin2"));
         let shutdown1 = plugin1.shutdown_called.clone();
         let shutdown2 = plugin2.shutdown_called.clone();
         registry.register_plugin(plugin1).unwrap();
         registry.register_plugin(plugin2).unwrap();
         let stage_registry_arc = create_mock_stage_registry_arc(); // Create mock stage registry
 
         // Initialize them, passing the stage_registry_arc
         registry.initialize_plugin("plugin1", &mut app, &stage_registry_arc).await.unwrap();
         registry.initialize_plugin("plugin2", &mut app, &stage_registry_arc).await.unwrap();
         assert_eq!(registry.initialized_count(), 2);
 
         // Shutdown
         let result = registry.shutdown_all();
         assert!(result.is_ok());

         assert!(shutdown1.load(Ordering::SeqCst), "Plugin1 shutdown should be called");
         assert!(shutdown2.load(Ordering::SeqCst), "Plugin2 shutdown should be called");
         assert_eq!(registry.initialized_count(), 0, "Initialized count should be zero after shutdown");
         assert!(registry.initialized.is_empty());
     }

     #[tokio::test] // Make test async
     async fn test_disable_initialized_plugin_fails() { // Make test async
         let mut registry = create_test_registry();
         let mut app = create_mock_app();
         let plugin_id = "disable_initialized";
         registry.register_plugin(Box::new(MockRegistryPlugin::default(plugin_id))).unwrap();
         let stage_registry_arc = create_mock_stage_registry_arc(); // Create mock stage registry
 
         // Initialize it, passing the stage_registry_arc
         registry.initialize_plugin(plugin_id, &mut app, &stage_registry_arc).await.unwrap();
         assert!(registry.initialized.contains(plugin_id));
         assert!(registry.is_enabled(plugin_id));

         // Try to disable
         let result = registry.disable_plugin(plugin_id);
         assert!(result.is_err(), "Should not be able to disable an initialized plugin");
         assert!(registry.is_enabled(plugin_id), "Plugin should remain enabled"); // State shouldn't change
         assert!(registry.initialized.contains(plugin_id), "Plugin should remain initialized"); // State shouldn't change
     }

     #[tokio::test] // Use tokio::test
     async fn test_check_dependencies_enabled_only() { // Add async
         let mut registry = create_test_registry();
         let dep_id = "dependency_plugin";
         let main_id = "main_plugin";

         // Main plugin requires dependency_plugin
         let main_plugin = Box::new(MockRegistryPlugin::new(
             main_id,
             "1.0.0",
             vec![PluginDependency::required_any(dep_id)],
             vec![VersionRange::from_str(">=0.1.0").unwrap()]
         ));
         // Dependency plugin
         let dep_plugin = Box::new(MockRegistryPlugin::default(dep_id));

         registry.register_plugin(main_plugin).unwrap();
         registry.register_plugin(dep_plugin).unwrap();

         // Initially, both are enabled, should pass
         assert!(registry.check_dependencies().is_ok());

         // Disable the dependency
         registry.disable_plugin(dep_id).unwrap();
         assert!(!registry.is_enabled(dep_id));

         // Now check_dependencies should fail because main_plugin is enabled but its required dependency is disabled
         let result = registry.check_dependencies();
         assert!(result.is_err());
         if let Err(Error::Plugin(msg)) = result {
             assert!(msg.contains("requires enabled plugin 'dependency_plugin', which is missing or disabled"));
         } else {
             panic!("Expected Plugin error");
         }

         // Disable the main plugin as well
         registry.disable_plugin(main_id).unwrap();

         // Now check_dependencies should pass again, because the main plugin is no longer enabled
         assert!(registry.check_dependencies().is_ok());
     }
// --- New Tests for Dependency Resolution ---

    #[tokio::test] // Make test async
    async fn test_initialize_all_linear_dependency() { // Make test async
        // A -> B -> C
        let mut registry = create_test_registry();
        let mut app = create_mock_app();
        let plugin_a = Box::new(MockRegistryPlugin::new("A", "1.0.0", vec![PluginDependency::required_any("B")], vec![VersionRange::from_str(">=0.1.0").unwrap()]));
        let plugin_b = Box::new(MockRegistryPlugin::new("B", "1.0.0", vec![PluginDependency::required_any("C")], vec![VersionRange::from_str(">=0.1.0").unwrap()]));
        let plugin_c = Box::new(MockRegistryPlugin::default("C")); // No deps

        let init_a = plugin_a.init_called.clone();
        let init_b = plugin_b.init_called.clone();
        let init_c = plugin_c.init_called.clone();

        registry.register_plugin(plugin_a).unwrap();
        registry.register_plugin(plugin_b).unwrap();
        registry.register_plugin(plugin_c).unwrap();
        let stage_registry_arc = create_mock_stage_registry_arc(); // Create mock stage registry

        // Pass the stage_registry_arc
        let result = registry.initialize_all(&mut app, &stage_registry_arc).await;
        assert!(result.is_ok(), "Initialization should succeed. Error: {:?}", result.err());

        assert!(init_a.load(Ordering::SeqCst), "A should be initialized");
        assert!(init_b.load(Ordering::SeqCst), "B should be initialized");
        assert!(init_c.load(Ordering::SeqCst), "C should be initialized");
        assert_eq!(registry.initialized_count(), 3);
        // Although we don't directly check the order here, success implies a valid order was found.
    }

    #[tokio::test] // Make test async
    async fn test_initialize_all_diamond_dependency() { // Make test async
        //   -> B --
        // A        -> D
        //   -> C --
        let mut registry = create_test_registry();
        let mut app = create_mock_app();
        let plugin_a = Box::new(MockRegistryPlugin::new("A", "1.0.0", vec![PluginDependency::required_any("B"), PluginDependency::required_any("C")], vec![VersionRange::from_str(">=0.1.0").unwrap()]));
        let plugin_b = Box::new(MockRegistryPlugin::new("B", "1.0.0", vec![PluginDependency::required_any("D")], vec![VersionRange::from_str(">=0.1.0").unwrap()]));
        let plugin_c = Box::new(MockRegistryPlugin::new("C", "1.0.0", vec![PluginDependency::required_any("D")], vec![VersionRange::from_str(">=0.1.0").unwrap()]));
        let plugin_d = Box::new(MockRegistryPlugin::default("D")); // No deps

        let init_a = plugin_a.init_called.clone();
        let init_b = plugin_b.init_called.clone();
        let init_c = plugin_c.init_called.clone();
        let init_d = plugin_d.init_called.clone();

        registry.register_plugin(plugin_a).unwrap();
        registry.register_plugin(plugin_b).unwrap();
        registry.register_plugin(plugin_c).unwrap();
        registry.register_plugin(plugin_d).unwrap();
        let stage_registry_arc = create_mock_stage_registry_arc(); // Create mock stage registry

        // Pass the stage_registry_arc
        let result = registry.initialize_all(&mut app, &stage_registry_arc).await;
        assert!(result.is_ok(), "Initialization should succeed. Error: {:?}", result.err());

        assert!(init_a.load(Ordering::SeqCst), "A should be initialized");
        assert!(init_b.load(Ordering::SeqCst), "B should be initialized");
        assert!(init_c.load(Ordering::SeqCst), "C should be initialized");
        assert!(init_d.load(Ordering::SeqCst), "D should be initialized");
        assert_eq!(registry.initialized_count(), 4);
    }

    #[tokio::test] // Make test async
    async fn test_initialize_all_simple_cycle() { // Make test async
        // A -> B -> A
        let mut registry = create_test_registry();
        let mut app = create_mock_app();
        let plugin_a = Box::new(MockRegistryPlugin::new("A", "1.0.0", vec![PluginDependency::required_any("B")], vec![VersionRange::from_str(">=0.1.0").unwrap()]));
        let plugin_b = Box::new(MockRegistryPlugin::new("B", "1.0.0", vec![PluginDependency::required_any("A")], vec![VersionRange::from_str(">=0.1.0").unwrap()]));

        registry.register_plugin(plugin_a).unwrap();
        registry.register_plugin(plugin_b).unwrap();
        let stage_registry_arc = create_mock_stage_registry_arc(); // Create mock stage registry

        // Pass the stage_registry_arc
        let result = registry.initialize_all(&mut app, &stage_registry_arc).await;
        assert!(result.is_err(), "Initialization should fail due to cycle");
        if let Err(Error::Plugin(msg)) = result {
            println!("Cycle error message: {}", msg); // Log for debugging
            assert!(msg.contains("Dependency resolution failed") && msg.contains("Circular dependency detected"), "Error message should indicate a cyclic dependency");
        } else {
            panic!("Expected Plugin error indicating a cycle");
        }
        assert_eq!(registry.initialized_count(), 0);
    }

     #[tokio::test] // Make test async
     async fn test_initialize_all_complex_cycle() { // Make test async
         // A -> B -> C -> A
         let mut registry = create_test_registry();
         let mut app = create_mock_app();
         let plugin_a = Box::new(MockRegistryPlugin::new("A", "1.0.0", vec![PluginDependency::required_any("B")], vec![VersionRange::from_str(">=0.1.0").unwrap()]));
         let plugin_b = Box::new(MockRegistryPlugin::new("B", "1.0.0", vec![PluginDependency::required_any("C")], vec![VersionRange::from_str(">=0.1.0").unwrap()]));
         let plugin_c = Box::new(MockRegistryPlugin::new("C", "1.0.0", vec![PluginDependency::required_any("A")], vec![VersionRange::from_str(">=0.1.0").unwrap()]));

         registry.register_plugin(plugin_a).unwrap();
         registry.register_plugin(plugin_b).unwrap();
         registry.register_plugin(plugin_c).unwrap();
         let stage_registry_arc = create_mock_stage_registry_arc(); // Create mock stage registry
 
         // Pass the stage_registry_arc
         let result = registry.initialize_all(&mut app, &stage_registry_arc).await;
         assert!(result.is_err(), "Initialization should fail due to cycle");
         if let Err(Error::Plugin(msg)) = result {
             println!("Cycle error message: {}", msg); // Log for debugging
             assert!(msg.contains("Dependency resolution failed") && msg.contains("Circular dependency detected"), "Error message should indicate a cyclic dependency");
         } else {
             panic!("Expected Plugin error indicating a cycle");
         }
         assert_eq!(registry.initialized_count(), 0);
     }

     #[tokio::test] // Make test async
     async fn test_initialize_all_missing_dependency() { // Make test async
         // A -> B (B is missing)
         let mut registry = create_test_registry();
         let mut app = create_mock_app();
         let plugin_a = Box::new(MockRegistryPlugin::new("A", "1.0.0", vec![PluginDependency::required_any("B")], vec![VersionRange::from_str(">=0.1.0").unwrap()]));

         registry.register_plugin(plugin_a).unwrap();

         // Note: The topological sort itself might succeed if B isn't in the 'enabled' set.
         // The error should occur during the recursive initialize_plugin call.
         let stage_registry_arc = create_mock_stage_registry_arc(); // Create mock stage registry
         // Pass the stage_registry_arc
         let result = registry.initialize_all(&mut app, &stage_registry_arc).await;
         assert!(result.is_err(), "Initialization should fail due to missing dependency B");
         if let Err(Error::Plugin(msg)) = result {
             println!("Missing dependency error: {}", msg);
             assert!(msg.contains("requires enabled dependency 'B', which is missing or disabled"), "Error message should indicate missing dependency B");
         } else {
             panic!("Expected Plugin error indicating missing dependency");
         }
         assert_eq!(registry.initialized_count(), 0);
     }

     #[tokio::test] // Make test async
     async fn test_initialize_all_disabled_dependency() { // Make test async
         // A -> B (B is registered but disabled)
         let mut registry = create_test_registry();
         let mut app = create_mock_app();
         let plugin_a = Box::new(MockRegistryPlugin::new("A", "1.0.0", vec![PluginDependency::required_any("B")], vec![VersionRange::from_str(">=0.1.0").unwrap()]));
         let plugin_b = Box::new(MockRegistryPlugin::default("B"));
         let init_b = plugin_b.init_called.clone();


         registry.register_plugin(plugin_a).unwrap();
         registry.register_plugin(plugin_b).unwrap();

         registry.disable_plugin("B").unwrap(); // Disable B
         let stage_registry_arc = create_mock_stage_registry_arc(); // Create mock stage registry
 
         // Pass the stage_registry_arc
         let result = registry.initialize_all(&mut app, &stage_registry_arc).await;
         assert!(result.is_err(), "Initialization should fail due to disabled dependency B");
         if let Err(Error::Plugin(msg)) = result {
             println!("Disabled dependency error: {}", msg);
             assert!(msg.contains("requires enabled dependency 'B', which is missing or disabled"), "Error message should indicate disabled dependency B");
         } else {
             panic!("Expected Plugin error indicating disabled dependency");
         }
         assert!(!init_b.load(Ordering::SeqCst), "Disabled plugin B should not be initialized");
         assert_eq!(registry.initialized_count(), 0);
     }

    // --- End New Tests ---
}