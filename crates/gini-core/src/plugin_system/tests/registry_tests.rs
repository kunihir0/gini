use super::super::registry::PluginRegistry;
use crate::plugin_system::version::{ApiVersion, VersionRange};
use crate::plugin_system::traits::{Plugin, PluginPriority}; // Removed PluginError
use crate::plugin_system::dependency::PluginDependency;
use crate::plugin_system::error::PluginSystemError; // Import PluginSystemError
use crate::kernel::bootstrap::Application;
use crate::kernel::error::{Error}; // Removed unused Result as KernelResult
use crate::stage_manager::context::StageContext;
use crate::stage_manager::requirement::StageRequirement;
// Removed unused: use crate::stage_manager::Stage;
use crate::stage_manager::registry::StageRegistry;
use async_trait::async_trait;
use std::str::FromStr;
use std::sync::{Arc, Mutex as StdMutex, atomic::{AtomicBool, Ordering}}; // Add StdMutex
use std::collections::HashSet;
use tokio::sync::Mutex; // Add Mutex

// --- Mock Plugin for Registry Tests ---
struct MockRegistryPlugin {
    id: String,
    version: String,
    deps: Vec<PluginDependency>,
    compatible_apis: Vec<VersionRange>,
    priority_val: PluginPriority, // Added for priority testing
    init_called: std::sync::Arc<AtomicBool>,
    init_order_tracker: Option<Arc<StdMutex<Vec<String>>>>, // Added for init order testing
    shutdown_called: std::sync::Arc<AtomicBool>,
    shutdown_tracker: Option<Arc<StdMutex<Vec<String>>>>,
}

impl MockRegistryPlugin {
    #[allow(clippy::too_many_arguments)] // Common in test mocks
    fn new(
        id: &str,
        version: &str,
        deps: Vec<PluginDependency>,
        compatible_apis: Vec<VersionRange>,
        priority_val: PluginPriority,
        init_order_tracker: Option<Arc<StdMutex<Vec<String>>>>,
        shutdown_tracker: Option<Arc<StdMutex<Vec<String>>>>,
    ) -> Self {
        Self {
            id: id.to_string(),
            version: version.to_string(),
            deps,
            compatible_apis,
            priority_val,
            init_called: std::sync::Arc::new(AtomicBool::new(false)),
            init_order_tracker,
            shutdown_called: std::sync::Arc::new(AtomicBool::new(false)),
            shutdown_tracker,
        }
    }

    fn default(id: &str) -> Self {
        Self::new(
            id,
            "1.0.0",
            vec![],
            vec![VersionRange::from_str(">=0.1.0").unwrap()],
            PluginPriority::ThirdParty(151), // Default priority
            None, // No init tracker by default
            None, // No shutdown tracker by default
        )
    }
    
    // Constructor for init order testing
    fn with_init_tracker_and_priority(
        id: &str,
        priority: PluginPriority,
        deps: Vec<PluginDependency>,
        tracker: Arc<StdMutex<Vec<String>>>,
    ) -> Self {
        Self::new(
            id,
            "1.0.0",
            deps,
            vec![VersionRange::from_str(">=0.1.0").unwrap()],
            priority,
            Some(tracker),
            None,
        )
    }


    // Specific constructor for shutdown tests
    #[allow(dead_code)] // To allow this only for specific tests
    fn with_shutdown_tracker(id: &str, deps: Vec<PluginDependency>, tracker: Arc<StdMutex<Vec<String>>>) -> Self {
        Self::new(
            id,
            "1.0.0",
            deps,
            vec![VersionRange::from_str(">=0.1.0").unwrap()],
            PluginPriority::ThirdParty(151), // Default priority
            None,
            Some(tracker),
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
    fn priority(&self) -> PluginPriority { self.priority_val.clone() } // Use stored priority
    fn compatible_api_versions(&self) -> Vec<VersionRange> { self.compatible_apis.clone() }
    fn dependencies(&self) -> Vec<PluginDependency> { self.deps.clone() }
    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }
    fn init(&self, _app: &mut Application) -> std::result::Result<(), PluginSystemError> {
        println!("MockRegistryPlugin '{}' init called (Priority: {:?}).", self.id, self.priority_val);
        self.init_called.store(true, Ordering::SeqCst);
        if let Some(tracker_arc) = &self.init_order_tracker {
            let mut order = tracker_arc.lock().unwrap();
            order.push(self.id.clone());
        }
        Ok(())
    }
    fn shutdown(&self) -> std::result::Result<(), PluginSystemError> {
         println!("MockRegistryPlugin '{}' shutdown called.", self.id);
         self.shutdown_called.store(true, Ordering::SeqCst);
         if let Some(tracker_arc) = &self.shutdown_tracker {
             let mut order = tracker_arc.lock().unwrap();
             order.push(self.id.clone());
         }
         Ok(())
    }
    async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), PluginSystemError> { Ok(()) }
    fn register_stages(&self, _registry: &mut StageRegistry) -> std::result::Result<(), PluginSystemError> { Ok(()) }
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
        let plugin = Arc::new(MockRegistryPlugin::default("plugin1"));
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
        registry.register_plugin(Arc::new(MockRegistryPlugin::default("plugin1"))).unwrap();
        let result = registry.register_plugin(Arc::new(MockRegistryPlugin::default("plugin1")));
        assert!(result.is_err());
        assert_eq!(registry.plugin_count(), 1); // Count should remain 1
    }

     #[tokio::test] // Use tokio::test
     async fn test_register_incompatible_api() { // Add async
         let mut registry = PluginRegistry::new(ApiVersion::from_str("1.0.0").unwrap()); // API v1.0.0
         // Plugin compatible with >=0.1.0, <0.2.0
         let plugin = Arc::new(MockRegistryPlugin::new(
             "incompatible_plugin",
             "1.0.0",
             vec![],
             vec![VersionRange::from_str(">=0.1.0, <0.2.0").unwrap()],
             PluginPriority::ThirdParty(151), // Default priority
             None, // No init tracker
             None  // No shutdown tracker
         ));
         let result = registry.register_plugin(plugin);
         assert!(result.is_err(), "Should fail due to API incompatibility");
         assert_eq!(registry.plugin_count(), 0);
     }

    #[tokio::test] // Use tokio::test
    async fn test_unregister_plugin() { // Add async
        let mut registry = create_test_registry();
        let plugin_id = "plugin_to_unregister";
        registry.register_plugin(Arc::new(MockRegistryPlugin::default(plugin_id))).unwrap();
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
        registry.register_plugin(Arc::new(MockRegistryPlugin::default(plugin_id))).unwrap();

        let plugin_arc_opt = registry.get_plugin(plugin_id);
        assert!(plugin_arc_opt.is_some());
        assert_eq!(plugin_arc_opt.unwrap().name(), plugin_id);

        let non_existent = registry.get_plugin("non_existent");
        assert!(non_existent.is_none());
    }

    #[tokio::test] // Use tokio::test
    async fn test_get_plugins_arc() { // Add async
        let mut registry = create_test_registry();
        registry.register_plugin(Arc::new(MockRegistryPlugin::default("plugin1"))).unwrap();
        registry.register_plugin(Arc::new(MockRegistryPlugin::default("plugin2"))).unwrap();

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
        registry.register_plugin(Arc::new(MockRegistryPlugin::default(plugin_id))).unwrap();

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
        registry.register_plugin(Arc::new(MockRegistryPlugin::default("plugin1"))).unwrap();
        registry.register_plugin(Arc::new(MockRegistryPlugin::default("plugin2"))).unwrap();
        registry.register_plugin(Arc::new(MockRegistryPlugin::default("plugin3"))).unwrap();

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
         let plugin = Arc::new(MockRegistryPlugin::default(plugin_id));
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
         let plugin = Arc::new(MockRegistryPlugin::default(plugin_id));
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
         let plugin1 = Arc::new(MockRegistryPlugin::default("plugin1"));
         let plugin2 = Arc::new(MockRegistryPlugin::default("plugin2"));
         let plugin3 = Arc::new(MockRegistryPlugin::default("plugin3"));
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


     #[tokio::test]
     async fn test_shutdown_all_order_verification() {
         // Scenario 1: No dependencies
         {
             let mut registry = create_test_registry();
             let mut app = create_mock_app();
             let stage_registry_arc = create_mock_stage_registry_arc();
             let tracker = Arc::new(StdMutex::new(Vec::new()));
      
             let plugin_a = Arc::new(MockRegistryPlugin::with_shutdown_tracker("P_A", vec![], tracker.clone()));
             let plugin_b = Arc::new(MockRegistryPlugin::with_shutdown_tracker("P_B", vec![], tracker.clone()));
      
             registry.register_plugin(plugin_a.clone()).unwrap();
             registry.register_plugin(plugin_b.clone()).unwrap();
     
             registry.initialize_all(&mut app, &stage_registry_arc).await.unwrap();
             assert_eq!(registry.initialized_count(), 2);
     
             registry.shutdown_all().unwrap();
             let order = tracker.lock().unwrap();
             assert_eq!(order.len(), 2);
             // Order between P_A and P_B is not strictly defined as they have no mutual dependencies
             assert!(order.contains(&"P_A".to_string()));
             assert!(order.contains(&"P_B".to_string()));
             assert_eq!(registry.initialized_count(), 0);
             assert!(plugin_a.shutdown_called.load(Ordering::SeqCst));
             assert!(plugin_b.shutdown_called.load(Ordering::SeqCst));
         }
     
         // Scenario 2: Simple linear dependencies (A -> B -> C)
         // Expected shutdown: A, then B, then C
         {
             let mut registry = create_test_registry();
             let mut app = create_mock_app();
             let stage_registry_arc = create_mock_stage_registry_arc();
             let tracker = Arc::new(StdMutex::new(Vec::new()));
      
             let plugin_c_obj = Arc::new(MockRegistryPlugin::with_shutdown_tracker("C", vec![], tracker.clone()));
             let plugin_b_obj = Arc::new(MockRegistryPlugin::with_shutdown_tracker("B", vec![PluginDependency::required_any("C")], tracker.clone()));
             let plugin_a_obj = Arc::new(MockRegistryPlugin::with_shutdown_tracker("A", vec![PluginDependency::required_any("B")], tracker.clone()));
      
             registry.register_plugin(plugin_a_obj.clone()).unwrap();
             registry.register_plugin(plugin_b_obj.clone()).unwrap();
             registry.register_plugin(plugin_c_obj.clone()).unwrap();
     
             registry.initialize_all(&mut app, &stage_registry_arc).await.unwrap();
             assert_eq!(registry.initialized_count(), 3);
     
             registry.shutdown_all().unwrap();
             let order = tracker.lock().unwrap();
             assert_eq!(*order, vec!["A".to_string(), "B".to_string(), "C".to_string()], "Shutdown order for A->B->C should be A, B, C");
             assert_eq!(registry.initialized_count(), 0);
             assert!(plugin_a_obj.shutdown_called.load(Ordering::SeqCst));
             assert!(plugin_b_obj.shutdown_called.load(Ordering::SeqCst));
             assert!(plugin_c_obj.shutdown_called.load(Ordering::SeqCst));
         }
     
         // Scenario 3: Diamond dependencies (A -> B, A -> C, B -> D, C -> D)
         // Expected shutdown: A, then (B, C or C, B), then D.
         {
             let mut registry = create_test_registry();
             let mut app = create_mock_app();
             let stage_registry_arc = create_mock_stage_registry_arc();
             let tracker = Arc::new(StdMutex::new(Vec::new()));
      
             let plugin_d_obj = Arc::new(MockRegistryPlugin::with_shutdown_tracker("D_D", vec![], tracker.clone()));
             let plugin_b_obj = Arc::new(MockRegistryPlugin::with_shutdown_tracker("D_B", vec![PluginDependency::required_any("D_D")], tracker.clone()));
             let plugin_c_obj = Arc::new(MockRegistryPlugin::with_shutdown_tracker("D_C", vec![PluginDependency::required_any("D_D")], tracker.clone()));
             let plugin_a_obj = Arc::new(MockRegistryPlugin::with_shutdown_tracker("D_A", vec![PluginDependency::required_any("D_B"), PluginDependency::required_any("D_C")], tracker.clone()));
      
             registry.register_plugin(plugin_a_obj.clone()).unwrap();
             registry.register_plugin(plugin_b_obj.clone()).unwrap();
             registry.register_plugin(plugin_c_obj.clone()).unwrap();
             registry.register_plugin(plugin_d_obj.clone()).unwrap();
     
             registry.initialize_all(&mut app, &stage_registry_arc).await.unwrap();
             assert_eq!(registry.initialized_count(), 4);
     
             registry.shutdown_all().unwrap();
             let order = tracker.lock().unwrap();
             assert_eq!(order.len(), 4, "Diamond: Should shut down 4 plugins");
             assert_eq!(order[0], "D_A", "Diamond: D_A should be first");
             assert_eq!(order[3], "D_D", "Diamond: D_D should be last");
             assert!( (order[1] == "D_B" && order[2] == "D_C") || (order[1] == "D_C" && order[2] == "D_B"), "Diamond: D_B and D_C should be in the middle. Order: {:?}", order);
             assert_eq!(registry.initialized_count(), 0);
             assert!(plugin_a_obj.shutdown_called.load(Ordering::SeqCst));
             assert!(plugin_b_obj.shutdown_called.load(Ordering::SeqCst));
             assert!(plugin_c_obj.shutdown_called.load(Ordering::SeqCst));
             assert!(plugin_d_obj.shutdown_called.load(Ordering::SeqCst));
         }
         
         // Scenario 4: Multiple independent plugins and one with a dependency (M_P3 -> M_P1)
         // Expected: M_P3 before M_P1. M_P2 can be anywhere.
         {
             let mut registry = create_test_registry();
             let mut app = create_mock_app();
             let stage_registry_arc = create_mock_stage_registry_arc();
             let tracker = Arc::new(StdMutex::new(Vec::new()));
      
             let plugin_1_obj = Arc::new(MockRegistryPlugin::with_shutdown_tracker("M_P1", vec![], tracker.clone()));
             let plugin_2_obj = Arc::new(MockRegistryPlugin::with_shutdown_tracker("M_P2", vec![], tracker.clone()));
             let plugin_3_obj = Arc::new(MockRegistryPlugin::with_shutdown_tracker("M_P3", vec![PluginDependency::required_any("M_P1")], tracker.clone()));
      
             registry.register_plugin(plugin_1_obj.clone()).unwrap();
             registry.register_plugin(plugin_2_obj.clone()).unwrap();
             registry.register_plugin(plugin_3_obj.clone()).unwrap();
             
             registry.initialize_all(&mut app, &stage_registry_arc).await.unwrap();
             assert_eq!(registry.initialized_count(), 3);
     
             registry.shutdown_all().unwrap();
             let order = tracker.lock().unwrap();
             assert_eq!(order.len(), 3, "Mixed: Should shut down 3 plugins. Order: {:?}", order);
             
             let p1_pos = order.iter().position(|id| id == "M_P1").expect("M_P1 not found in shutdown order");
             let p3_pos = order.iter().position(|id| id == "M_P3").expect("M_P3 not found in shutdown order");
             assert!(p3_pos < p1_pos, "Mixed: M_P3 (dependent) must shut down before M_P1 (dependency). Order: {:?}", order);
             assert!(order.contains(&"M_P2".to_string()), "Mixed: M_P2 should be in the shutdown list. Order: {:?}", order);
             assert_eq!(registry.initialized_count(), 0);
             assert!(plugin_1_obj.shutdown_called.load(Ordering::SeqCst));
             assert!(plugin_2_obj.shutdown_called.load(Ordering::SeqCst));
             assert!(plugin_3_obj.shutdown_called.load(Ordering::SeqCst));
         }
     }
 
     #[tokio::test] // Make test async
     async fn test_disable_initialized_plugin_fails() { // Make test async
         let mut registry = create_test_registry();
         let mut app = create_mock_app();
         let plugin_id = "disable_initialized";
         registry.register_plugin(Arc::new(MockRegistryPlugin::default(plugin_id))).unwrap();
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
         let main_plugin = Arc::new(MockRegistryPlugin::new(
             main_id,
             "1.0.0",
             vec![PluginDependency::required_any(dep_id)],
             vec![VersionRange::from_str(">=0.1.0").unwrap()],
             PluginPriority::ThirdParty(151),
             None,
             None
         ));
         // Dependency plugin
         let dep_plugin = Arc::new(MockRegistryPlugin::default(dep_id));

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
         if let Err(PluginSystemError::DependencyResolution(dep_err)) = result {
             assert!(matches!(dep_err, crate::plugin_system::dependency::DependencyError::MissingPlugin(id) if id == dep_id));
         } else {
             panic!("Expected PluginSystemError::DependencyResolution error, got {:?}", result);
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
        let plugin_a = Arc::new(MockRegistryPlugin::new("A", "1.0.0", vec![PluginDependency::required_any("B")], vec![VersionRange::from_str(">=0.1.0").unwrap()], PluginPriority::ThirdParty(151), None, None));
        let plugin_b = Arc::new(MockRegistryPlugin::new("B", "1.0.0", vec![PluginDependency::required_any("C")], vec![VersionRange::from_str(">=0.1.0").unwrap()], PluginPriority::ThirdParty(151), None, None));
        let plugin_c = Arc::new(MockRegistryPlugin::default("C")); // No deps

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
        let plugin_a = Arc::new(MockRegistryPlugin::new("A", "1.0.0", vec![PluginDependency::required_any("B"), PluginDependency::required_any("C")], vec![VersionRange::from_str(">=0.1.0").unwrap()], PluginPriority::ThirdParty(151), None, None));
        let plugin_b = Arc::new(MockRegistryPlugin::new("B", "1.0.0", vec![PluginDependency::required_any("D")], vec![VersionRange::from_str(">=0.1.0").unwrap()], PluginPriority::ThirdParty(151), None, None));
        let plugin_c = Arc::new(MockRegistryPlugin::new("C", "1.0.0", vec![PluginDependency::required_any("D")], vec![VersionRange::from_str(">=0.1.0").unwrap()], PluginPriority::ThirdParty(151), None, None));
        let plugin_d = Arc::new(MockRegistryPlugin::default("D")); // No deps

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
        let plugin_a = Arc::new(MockRegistryPlugin::new("A", "1.0.0", vec![PluginDependency::required_any("B")], vec![VersionRange::from_str(">=0.1.0").unwrap()], PluginPriority::ThirdParty(151), None, None));
        let plugin_b = Arc::new(MockRegistryPlugin::new("B", "1.0.0", vec![PluginDependency::required_any("A")], vec![VersionRange::from_str(">=0.1.0").unwrap()], PluginPriority::ThirdParty(151), None, None));

        registry.register_plugin(plugin_a).unwrap();
        registry.register_plugin(plugin_b).unwrap();
        let stage_registry_arc = create_mock_stage_registry_arc(); // Create mock stage registry

        // Pass the stage_registry_arc
        let result = registry.initialize_all(&mut app, &stage_registry_arc).await;
        assert!(result.is_err(), "Initialization should fail due to cycle");
        if let Err(Error::PluginSystem(PluginSystemError::DependencyResolution(dep_err))) = result {
            println!("Cycle error message: {:?}", dep_err); // Log for debugging
            assert!(matches!(dep_err, crate::plugin_system::dependency::DependencyError::CyclicDependency(_)));
        } else {
            panic!("Expected PluginSystem(DependencyResolution(CyclicDependency)) error, got {:?}", result);
        }
        assert_eq!(registry.initialized_count(), 0);
    }

     #[tokio::test] // Make test async
     async fn test_initialize_all_complex_cycle() { // Make test async
         // A -> B -> C -> A
         let mut registry = create_test_registry();
         let mut app = create_mock_app();
         let plugin_a = Arc::new(MockRegistryPlugin::new("A", "1.0.0", vec![PluginDependency::required_any("B")], vec![VersionRange::from_str(">=0.1.0").unwrap()], PluginPriority::ThirdParty(151), None, None));
         let plugin_b = Arc::new(MockRegistryPlugin::new("B", "1.0.0", vec![PluginDependency::required_any("C")], vec![VersionRange::from_str(">=0.1.0").unwrap()], PluginPriority::ThirdParty(151), None, None));
         let plugin_c = Arc::new(MockRegistryPlugin::new("C", "1.0.0", vec![PluginDependency::required_any("A")], vec![VersionRange::from_str(">=0.1.0").unwrap()], PluginPriority::ThirdParty(151), None, None));
 
         registry.register_plugin(plugin_a).unwrap();
         registry.register_plugin(plugin_b).unwrap();
         registry.register_plugin(plugin_c).unwrap();
         let stage_registry_arc = create_mock_stage_registry_arc(); // Create mock stage registry
 
         // Pass the stage_registry_arc
         let result = registry.initialize_all(&mut app, &stage_registry_arc).await;
         assert!(result.is_err(), "Initialization should fail due to cycle");
         if let Err(Error::PluginSystem(PluginSystemError::DependencyResolution(dep_err))) = result {
             println!("Cycle error message: {:?}", dep_err); // Log for debugging
             assert!(matches!(dep_err, crate::plugin_system::dependency::DependencyError::CyclicDependency(_)));
         } else {
             panic!("Expected PluginSystem(DependencyResolution(CyclicDependency)) error, got {:?}", result);
         }
         assert_eq!(registry.initialized_count(), 0);
     }

     #[tokio::test] // Make test async
     async fn test_initialize_all_missing_dependency() { // Make test async
         // A -> B (B is missing)
         let mut registry = create_test_registry();
         let mut app = create_mock_app();
         let plugin_a = Arc::new(MockRegistryPlugin::new("A", "1.0.0", vec![PluginDependency::required_any("B")], vec![VersionRange::from_str(">=0.1.0").unwrap()], PluginPriority::ThirdParty(151), None, None));
 
         registry.register_plugin(plugin_a).unwrap();

         // Note: The topological sort itself might succeed if B isn't in the 'enabled' set.
         // The error should occur during the recursive initialize_plugin call.
         let stage_registry_arc = create_mock_stage_registry_arc(); // Create mock stage registry
         // Pass the stage_registry_arc
         let result = registry.initialize_all(&mut app, &stage_registry_arc).await;
         assert!(result.is_err(), "Initialization should fail due to missing dependency B");
         if let Err(Error::PluginSystem(PluginSystemError::DependencyResolution(dep_err))) = result {
             println!("Missing dependency error: {:?}", dep_err);
             assert!(matches!(dep_err, crate::plugin_system::dependency::DependencyError::MissingPlugin(id) if id == "B"));
         } else {
             panic!("Expected PluginSystem(DependencyResolution(MissingPlugin)) error, got {:?}", result);
         }
         assert_eq!(registry.initialized_count(), 0);
     }

     #[tokio::test] // Make test async
     async fn test_initialize_all_disabled_dependency() { // Make test async
         // A -> B (B is registered but disabled)
         let mut registry = create_test_registry();
         let mut app = create_mock_app();
         let plugin_a = Arc::new(MockRegistryPlugin::new("A", "1.0.0", vec![PluginDependency::required_any("B")], vec![VersionRange::from_str(">=0.1.0").unwrap()], PluginPriority::ThirdParty(151), None, None));
         let plugin_b = Arc::new(MockRegistryPlugin::default("B"));
         let init_b = plugin_b.init_called.clone();


         registry.register_plugin(plugin_a).unwrap();
         registry.register_plugin(plugin_b).unwrap();

         registry.disable_plugin("B").unwrap(); // Disable B
         let stage_registry_arc = create_mock_stage_registry_arc(); // Create mock stage registry
 
         // Pass the stage_registry_arc
         let result = registry.initialize_all(&mut app, &stage_registry_arc).await;
         assert!(result.is_err(), "Initialization should fail due to disabled dependency B");
         if let Err(Error::PluginSystem(PluginSystemError::DependencyResolution(dep_err))) = result {
             println!("Disabled dependency error: {:?}", dep_err);
             assert!(matches!(dep_err, crate::plugin_system::dependency::DependencyError::MissingPlugin(id) if id == "B"));
         } else {
             panic!("Expected PluginSystem(DependencyResolution(MissingPlugin)) error for disabled dependency, got {:?}", result);
         }
         assert!(!init_b.load(Ordering::SeqCst), "Disabled plugin B should not be initialized");
         assert_eq!(registry.initialized_count(), 0);
     }

    // --- End New Tests ---

    // --- Tests for Priority Sorting ---
    #[tokio::test]
    async fn test_initialize_all_priority_no_dependencies() {
        let mut registry = create_test_registry();
        let mut app = create_mock_app();
        let stage_registry_arc = create_mock_stage_registry_arc();
        let init_order_tracker = Arc::new(StdMutex::new(Vec::new()));

        // Lower numerical value = higher priority
        let plugin_high_priority = Arc::new(MockRegistryPlugin::with_init_tracker_and_priority(
            "P_High", PluginPriority::Core(51), vec![], init_order_tracker.clone()
        ));
        let plugin_mid_priority = Arc::new(MockRegistryPlugin::with_init_tracker_and_priority(
            "P_Mid", PluginPriority::ThirdParty(151), vec![], init_order_tracker.clone()
        ));
        let plugin_low_priority = Arc::new(MockRegistryPlugin::with_init_tracker_and_priority(
            "P_Low", PluginPriority::ThirdPartyLow(201), vec![], init_order_tracker.clone()
        ));
        // Another plugin with same priority as P_Mid, to test stable sort by name
        let plugin_mid_alt_priority = Arc::new(MockRegistryPlugin::with_init_tracker_and_priority(
            "P_Mid_Alt", PluginPriority::ThirdParty(151), vec![], init_order_tracker.clone()
        ));


        // Register in a mixed order
        registry.register_plugin(plugin_mid_priority.clone()).unwrap();
        registry.register_plugin(plugin_low_priority.clone()).unwrap();
        registry.register_plugin(plugin_high_priority.clone()).unwrap();
        registry.register_plugin(plugin_mid_alt_priority.clone()).unwrap();

        let result = registry.initialize_all(&mut app, &stage_registry_arc).await;
        assert!(result.is_ok(), "Initialization failed: {:?}", result.err());

        let order = init_order_tracker.lock().unwrap();
        println!("Initialization order (no deps): {:?}", order);

        assert_eq!(order.len(), 4, "All plugins should be initialized");
        // Expected order: P_High, then (P_Mid, P_Mid_Alt in alphabetical order), then P_Low
        assert_eq!(order[0], "P_High");
        // P_Mid and P_Mid_Alt have same priority, so their relative order depends on secondary sort (ID)
        if order[1] == "P_Mid" {
            assert_eq!(order[2], "P_Mid_Alt");
        } else {
            assert_eq!(order[1], "P_Mid_Alt");
            assert_eq!(order[2], "P_Mid");
        }
        assert_eq!(order[3], "P_Low");

        assert!(plugin_high_priority.init_called.load(Ordering::SeqCst));
        assert!(plugin_mid_priority.init_called.load(Ordering::SeqCst));
        assert!(plugin_mid_alt_priority.init_called.load(Ordering::SeqCst));
        assert!(plugin_low_priority.init_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_initialize_all_priority_with_dependencies() {
        let mut registry = create_test_registry();
        let mut app = create_mock_app();
        let stage_registry_arc = create_mock_stage_registry_arc();
        let init_order_tracker = Arc::new(StdMutex::new(Vec::new()));

        // P_Dep1 (Core 60)
        // P_Dep2 (Core 70)
        // P_Main_A depends on P_Dep1, P_Dep2. Priority ThirdParty(160)
        // P_Main_B depends on P_Dep1, P_Dep2. Priority ThirdParty(155) (higher than A)
        // P_Main_C depends on P_Dep1, P_Dep2. Priority ThirdParty(160) (same as A, name C > A)

        // Expected order: P_Dep1, P_Dep2 (order between them by priority: P_Dep1 then P_Dep2),
        // then P_Main_B, then P_Main_A, then P_Main_C (among mains, B is highest, then A then C by name)

        let p_dep1 = Arc::new(MockRegistryPlugin::with_init_tracker_and_priority(
            "P_Dep1", PluginPriority::Core(60), vec![], init_order_tracker.clone()
        ));
        let p_dep2 = Arc::new(MockRegistryPlugin::with_init_tracker_and_priority(
            "P_Dep2", PluginPriority::Core(70), vec![], init_order_tracker.clone()
        ));

        let common_deps = vec![
            PluginDependency::required_any("P_Dep1"),
            PluginDependency::required_any("P_Dep2"),
        ];

        let p_main_a = Arc::new(MockRegistryPlugin::with_init_tracker_and_priority(
            "P_Main_A", PluginPriority::ThirdParty(160), common_deps.clone(), init_order_tracker.clone()
        ));
        let p_main_b = Arc::new(MockRegistryPlugin::with_init_tracker_and_priority(
            "P_Main_B", PluginPriority::ThirdParty(155), common_deps.clone(), init_order_tracker.clone()
        ));
         let p_main_c = Arc::new(MockRegistryPlugin::with_init_tracker_and_priority(
            "P_Main_C", PluginPriority::ThirdParty(160), common_deps.clone(), init_order_tracker.clone()
        ));


        registry.register_plugin(p_dep1.clone()).unwrap();
        registry.register_plugin(p_main_a.clone()).unwrap(); // Register out of ideal order
        registry.register_plugin(p_dep2.clone()).unwrap();
        registry.register_plugin(p_main_c.clone()).unwrap();
        registry.register_plugin(p_main_b.clone()).unwrap();


        let result = registry.initialize_all(&mut app, &stage_registry_arc).await;
        assert!(result.is_ok(), "Initialization failed: {:?}", result.err());

        let order = init_order_tracker.lock().unwrap();
        println!("Initialization order (with deps & priority): {:?}", order);

        assert_eq!(order.len(), 5, "All 5 plugins should be initialized");

        // Check dependencies first
        assert_eq!(order[0], "P_Dep1", "P_Dep1 (higher prio dep) should be first");
        assert_eq!(order[1], "P_Dep2", "P_Dep2 (lower prio dep) should be second");

        // Then check the main plugins, sorted by priority, then name
        assert_eq!(order[2], "P_Main_B", "P_Main_B (highest prio main) should be third");
        assert_eq!(order[3], "P_Main_A", "P_Main_A (next prio main, A before C) should be fourth");
        assert_eq!(order[4], "P_Main_C", "P_Main_C (same prio as A, C after A) should be fifth");


        assert!(p_dep1.init_called.load(Ordering::SeqCst));
        assert!(p_dep2.init_called.load(Ordering::SeqCst));
        assert!(p_main_a.init_called.load(Ordering::SeqCst));
        assert!(p_main_b.init_called.load(Ordering::SeqCst));
        assert!(p_main_c.init_called.load(Ordering::SeqCst));
    }
}