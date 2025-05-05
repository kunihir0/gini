#![cfg(test)]

use std::sync::Arc;
use async_trait::async_trait;

use crate::kernel::bootstrap::Application;
use crate::kernel::component::KernelComponent;
use crate::kernel::error::{Error, Result as KernelResult};
use crate::plugin_system::dependency::PluginDependency;
use crate::plugin_system::traits::{Plugin, PluginPriority, PluginError as TraitsPluginError};
use crate::plugin_system::version::VersionRange;
use crate::stage_manager::{Stage, StageContext};
use crate::stage_manager::requirement::StageRequirement;
use crate::storage::manager::DefaultStorageManager;

use super::super::common::{setup_test_environment, TestPlugin, DependentPlugin, ShutdownBehavior, PreflightBehavior};

// Plugin with an unparsable version string
struct UnparsableVersionPlugin { name: String }
impl UnparsableVersionPlugin { fn new(name: &str) -> Self { Self { name: name.to_string() } } }
#[async_trait]
impl Plugin for UnparsableVersionPlugin {
    fn name(&self) -> &'static str { Box::leak(self.name.clone().into_boxed_str()) }
    fn version(&self) -> &str { "invalid-version" } // Unparsable version
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(100) }
    fn compatible_api_versions(&self) -> Vec<VersionRange> { vec![">=0.1.0".parse().unwrap()] }
    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }
    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }
    fn init(&self, _app: &mut Application) -> KernelResult<()> { Ok(()) }
    async fn preflight_check(&self, _context: &StageContext) -> Result<(), TraitsPluginError> { Ok(()) }
    fn stages(&self) -> Vec<Box<dyn Stage>> { vec![] }
    fn shutdown(&self) -> KernelResult<()> { Ok(()) }
// Add default implementations for new trait methods
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
}

#[tokio::test]
async fn test_plugin_dependency_unparsable_version() {
    let (plugin_manager, _, _, stages_executed, execution_order, shutdown_order) = setup_test_environment().await;
    KernelComponent::initialize(&*plugin_manager).await.expect("Init PluginManager");

    let dep_plugin = UnparsableVersionPlugin::new("UnparsableDep");
    let main_plugin = DependentPlugin::new(
        "MainUnparsableTest", "1.0.0",
        vec![PluginDependency::required("UnparsableDep", ">=1.0.0".parse().unwrap())], // Requires the dep
        ShutdownBehavior::Success, PreflightBehavior::Success,
        stages_executed.clone(), execution_order.clone(), shutdown_order.clone()
    );
    let _main_plugin_name = main_plugin.name().to_string();

    // Register both
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(dep_plugin)).expect("Register dep");
        registry.register_plugin(Box::new(main_plugin)).expect("Register main");
    }

    // Check dependencies - should fail due to unparsable version in the dependency
    let check_result = {
        let registry = plugin_manager.registry().lock().await;
        registry.check_dependencies()
    };

    assert!(check_result.is_err(), "check_dependencies should fail for unparsable version");
    match check_result.err().unwrap() {
        Error::Plugin(msg) => {
            // Check only for the core part of the parsing error message
            assert!(msg.contains("Failed to parse version string 'invalid-version'"), "Expected unparsable version error, got: {}", msg);
        }
        e => panic!("Expected Plugin error for unparsable version, got {:?}", e),
    }
}

#[tokio::test]
async fn test_plugin_dependency_no_version() {
    let (plugin_manager, _, _, _stages_executed, _execution_order, _shutdown_order) = setup_test_environment().await;
    KernelComponent::initialize(&*plugin_manager).await.expect("Init PluginManager");

    // Destructure execution_order as well
    let (plugin_manager, _, _, stages_executed, execution_order, shutdown_order) = setup_test_environment().await;
    KernelComponent::initialize(&*plugin_manager).await.expect("Init PluginManager");

    let dep_plugin = TestPlugin::new("NoVersionDep", stages_executed.clone(), execution_order.clone()); // Pass execution_order
    let main_plugin = DependentPlugin::new(
        "MainNoVersionTest", "1.0.0",
        vec![PluginDependency::required_any("NoVersionDep")], // Use required_any for no version check
        ShutdownBehavior::Success, PreflightBehavior::Success,
        stages_executed.clone(), execution_order.clone(), shutdown_order.clone()
    );
    let main_plugin_name = main_plugin.name().to_string();

    // Register both
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(dep_plugin)).expect("Register dep");
        registry.register_plugin(Box::new(main_plugin)).expect("Register main");
    }

    // Check dependencies - should succeed as version is not checked
    let check_result = {
        let registry = plugin_manager.registry().lock().await;
        registry.check_dependencies()
    };
    assert!(check_result.is_ok(), "check_dependencies should succeed when version is not required");

    // Initialize - should also succeed
    let mut app = Application::new(None).unwrap();
    let init_result = {
        let mut registry = plugin_manager.registry().lock().await;
        registry.initialize_plugin(&main_plugin_name, &mut app)
    };
    assert!(init_result.is_ok(), "Initialization should succeed when version is not required");
}

// Helper plugins for cycle test
#[allow(dead_code)] // Allow dead code for test helper struct
struct CyclePluginA { storage: Arc<DefaultStorageManager> } // Need storage for init
#[async_trait]
impl Plugin for CyclePluginA {
    fn name(&self) -> &'static str { "CycleA" }
    fn version(&self) -> &str { "1.0.0" }
    fn dependencies(&self) -> Vec<PluginDependency> { vec![PluginDependency::required_any("CycleC")] } // A -> C (Use required_any)
    fn init(&self, _app: &mut Application) -> KernelResult<()> { Ok(()) }
    fn shutdown(&self) -> KernelResult<()> { Ok(()) }
    // Other trait methods omitted for brevity...
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(100) }
    fn compatible_api_versions(&self) -> Vec<VersionRange> { vec![">=0.1.0".parse().unwrap()] }
// Add default implementations for new trait methods
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }
    async fn preflight_check(&self, _context: &StageContext) -> Result<(), TraitsPluginError> { Ok(()) }
    fn stages(&self) -> Vec<Box<dyn Stage>> { vec![] }
}

#[allow(dead_code)] // Allow dead code for test helper struct
struct CyclePluginB { storage: Arc<DefaultStorageManager> }
#[async_trait]
impl Plugin for CyclePluginB {
    fn name(&self) -> &'static str { "CycleB" }
    fn version(&self) -> &str { "1.0.0" }
    fn dependencies(&self) -> Vec<PluginDependency> { vec![PluginDependency::required_any("CycleA")] } // B -> A (Use required_any)
    fn init(&self, _app: &mut Application) -> KernelResult<()> { Ok(()) }
    fn shutdown(&self) -> KernelResult<()> { Ok(()) }
    // Other trait methods omitted...
// Add default implementations for new trait methods
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(100) }
    fn compatible_api_versions(&self) -> Vec<VersionRange> { vec![">=0.1.0".parse().unwrap()] }
    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }
    async fn preflight_check(&self, _context: &StageContext) -> Result<(), TraitsPluginError> { Ok(()) }
    fn stages(&self) -> Vec<Box<dyn Stage>> { vec![] }
}

#[allow(dead_code)] // Allow dead code for test helper struct
struct CyclePluginC { storage: Arc<DefaultStorageManager> }
#[async_trait]
impl Plugin for CyclePluginC {
    fn name(&self) -> &'static str { "CycleC" }
    fn version(&self) -> &str { "1.0.0" }
    fn dependencies(&self) -> Vec<PluginDependency> { vec![PluginDependency::required_any("CycleB")] } // C -> B (Use required_any)
// Add default implementations for new trait methods
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
    fn init(&self, _app: &mut Application) -> KernelResult<()> { Ok(()) }
    fn shutdown(&self) -> KernelResult<()> { Ok(()) }
    // Other trait methods omitted...
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(100) }
    fn compatible_api_versions(&self) -> Vec<VersionRange> { vec![">=0.1.0".parse().unwrap()] }
    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }
    async fn preflight_check(&self, _context: &StageContext) -> Result<(), TraitsPluginError> { Ok(()) }
    fn stages(&self) -> Vec<Box<dyn Stage>> { vec![] }
}


#[tokio::test]
async fn test_plugin_shutdown_cycle() {
    let (plugin_manager, _, storage_manager, _, _, _) = setup_test_environment().await;
    KernelComponent::initialize(&*plugin_manager).await.expect("Init PluginManager");

    let plugin_a = CyclePluginA { storage: storage_manager.clone() };
    let plugin_b = CyclePluginB { storage: storage_manager.clone() };
    let plugin_c = CyclePluginC { storage: storage_manager.clone() };

    // Register plugins
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(plugin_a)).expect("Register A");
        registry.register_plugin(Box::new(plugin_b)).expect("Register B");
        registry.register_plugin(Box::new(plugin_c)).expect("Register C");
    }

    // Attempt to initialize all - this should now fail due to the cycle detection
    let mut app = Application::new(None).unwrap();
    let init_result = {
        let mut registry = plugin_manager.registry().lock().await;
        registry.initialize_all(&mut app) // Don't expect success
    };

    assert!(init_result.is_err(), "initialize_all should fail due to cyclic dependency");

    // Verify the specific error
    match init_result.err().unwrap() {
        Error::Plugin(msg) => {
            // Updated assertion to match the new error format from topological sort
            assert!(
                msg.contains("Dependency resolution failed: Circular dependency detected"),
                "Expected cycle detection error message, got: {}", msg
            );
            // Check if it mentions one of the cycle members (the specific nodes might vary based on map iteration order)
            assert!(
                msg.contains("CycleA") || msg.contains("CycleB") || msg.contains("CycleC"),
                "Error should mention a plugin in the cycle"
            );
        }
        e => panic!("Expected Plugin error for initialization cycle, got {:?}", e),
    }

    // Verify none were marked as initialized after the failed attempt
    {
        let registry = plugin_manager.registry().lock().await;
        assert!(!registry.initialized.contains("CycleA"));
        assert!(!registry.initialized.contains("CycleB"));
        assert!(!registry.initialized.contains("CycleC"));
    }
}

#[tokio::test]
async fn test_register_all_plugins_dep_resolution_fail_via_manager() -> KernelResult<()> {
    let (plugin_manager, _, _, stages_executed, execution_order, shutdown_order) = setup_test_environment().await;
    KernelComponent::initialize(&*plugin_manager).await?;

    // Create Plugin A depending on non-existent Plugin B
    let plugin_a = DependentPlugin::new(
        "DepFailA", "1.0.0",
        vec![PluginDependency::required("NonExistentB", ">=1.0.0".parse().unwrap())],
        ShutdownBehavior::Success, PreflightBehavior::Success,
        stages_executed.clone(), execution_order.clone(), shutdown_order.clone()
    );
    let plugin_a_name = plugin_a.name().to_string();

    // Register Plugin A
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(plugin_a))?;
    }

    // Attempt to initialize Plugin A - should fail dependency check
    let mut app = Application::new(None).unwrap();
    let init_result = {
        let mut registry = plugin_manager.registry().lock().await;
        registry.initialize_plugin(&plugin_a_name, &mut app)
    };

    assert!(init_result.is_err(), "Initialization should fail due to missing dependency");
    match init_result.err().unwrap() {
        Error::Plugin(msg) => {
             assert!(
                msg.contains("requires enabled dependency") && msg.contains("NonExistentB") && msg.contains("which is missing or disabled"),
                "Expected missing dependency error, got: {}", msg
            );
        }
        e => panic!("Expected Plugin error for missing dependency, got {:?}", e),
    }

    // Verify plugin A is not initialized
    let registry = plugin_manager.registry().lock().await;
    assert!(!registry.initialized.contains(&plugin_a_name));

    Ok(())
}
#[tokio::test]
async fn test_initialize_all_diamond_dependency_order() {
    // Test diamond: A -> B, A -> C, B -> D, C -> D
    // Expected init order: D, then B/C (any order), then A
    let (plugin_manager, _, _, stages_executed, execution_order, shutdown_order) = setup_test_environment().await;
    KernelComponent::initialize(&*plugin_manager).await.expect("Init PluginManager");

    let plugin_d = TestPlugin::new("DiamondD", stages_executed.clone(), execution_order.clone()); // Pass execution_order
    let plugin_b = DependentPlugin::new(
        "DiamondB", "1.0.0", vec![PluginDependency::required_any("DiamondD")],
        ShutdownBehavior::Success, PreflightBehavior::Success,
        stages_executed.clone(), execution_order.clone(), shutdown_order.clone()
    );
    let plugin_c = DependentPlugin::new(
        "DiamondC", "1.0.0", vec![PluginDependency::required_any("DiamondD")],
        ShutdownBehavior::Success, PreflightBehavior::Success,
        stages_executed.clone(), execution_order.clone(), shutdown_order.clone()
    );
    let plugin_a = DependentPlugin::new(
        "DiamondA", "1.0.0", vec![PluginDependency::required_any("DiamondB"), PluginDependency::required_any("DiamondC")],
        ShutdownBehavior::Success, PreflightBehavior::Success,
        stages_executed.clone(), execution_order.clone(), shutdown_order.clone()
    );

    // Register in a non-initialization order
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(plugin_a)).expect("Register A");
        registry.register_plugin(Box::new(plugin_c)).expect("Register C");
        registry.register_plugin(Box::new(plugin_d)).expect("Register D");
        registry.register_plugin(Box::new(plugin_b)).expect("Register B");
    }

    // Initialize all
    let mut app = Application::new(None).unwrap();
    let init_result = {
        let mut registry = plugin_manager.registry().lock().await;
        registry.initialize_all(&mut app)
    };

    assert!(init_result.is_ok(), "Initialization should succeed for diamond dependency. Error: {:?}", init_result.err());

    // Verify all were initialized
    {
        let registry = plugin_manager.registry().lock().await;
        assert!(registry.initialized.contains("DiamondA"));
        assert!(registry.initialized.contains("DiamondB"));
        assert!(registry.initialized.contains("DiamondC"));
        assert!(registry.initialized.contains("DiamondD"));
        assert_eq!(registry.initialized_count(), 4);
    }

    // Verify initialization order (D before B/C, B/C before A)
    let order = execution_order.lock().unwrap();
    println!("Execution order: {:?}", *order); // Debug log

    let pos_d = order.iter().position(|name| name == "DiamondD");
    let pos_b = order.iter().position(|name| name == "DiamondB");
    let pos_c = order.iter().position(|name| name == "DiamondC");
    let pos_a = order.iter().position(|name| name == "DiamondA");

    assert!(pos_d.is_some(), "D should be in execution order");
    assert!(pos_b.is_some(), "B should be in execution order");
    assert!(pos_c.is_some(), "C should be in execution order");
    assert!(pos_a.is_some(), "A should be in execution order");

    let pos_d = pos_d.unwrap();
    let pos_b = pos_b.unwrap();
    let pos_c = pos_c.unwrap();
    let pos_a = pos_a.unwrap();

    assert!(pos_d < pos_b, "D should be initialized before B (D={}, B={})", pos_d, pos_b);
    assert!(pos_d < pos_c, "D should be initialized before C (D={}, C={})", pos_d, pos_c);
    assert!(pos_b < pos_a, "B should be initialized before A (B={}, A={})", pos_b, pos_a);
    assert!(pos_c < pos_a, "C should be initialized before A (C={}, A={})", pos_c, pos_a);
}