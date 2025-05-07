// src/plugin_system/tests/preflight_tests.rs
#![cfg(test)]

use std::collections::HashSet;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::kernel::bootstrap::Application; // Mock or minimal version needed
use crate::kernel::error::{Result as KernelResult}; // Removed unused Error
use crate::plugin_system::dependency::PluginDependency;
use crate::plugin_system::traits::{Plugin, PluginError, PluginPriority};
use crate::plugin_system::version::{VersionRange, ApiVersion}; // Import ApiVersion
use crate::plugin_system::registry::PluginRegistry;
use crate::stage_manager::context::StageContext;
use crate::stage_manager::core_stages::{
    PluginPreflightCheckStage, PluginInitializationStage,
    PLUGIN_REGISTRY_KEY, PREFLIGHT_FAILURES_KEY, APPLICATION_KEY,
};
use crate::stage_manager::Stage; // Import the Stage trait
use crate::stage_manager::registry::{StageRegistry, SharedStageRegistry}; // Added for register_stages & SharedStageRegistry

// --- Mock Plugins ---

struct SuccessPlugin;
#[async_trait]
impl Plugin for SuccessPlugin {
    fn name(&self) -> &'static str { "SuccessPlugin" }
    fn version(&self) -> &str { "1.0.0" }
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(150) }
    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        // Declare compatibility with the test API version using parse()
        vec![">=0.1.0".parse().unwrap()]
    }
    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }
    fn required_stages(&self) -> Vec<crate::stage_manager::requirement::StageRequirement> { vec![] }
    fn shutdown(&self) -> KernelResult<()> { Ok(()) }

    // Mock init - tracks initialization
    fn init(&self, _app: &mut Application) -> KernelResult<()> {
        // This cast is tricky in tests. We'll use a simpler tracking mechanism.
        // For the real implementation, the context needs the actual Application.
        // For tests, we'll rely on the MockApplication tracking via context data.
        println!("SuccessPlugin::init called");
        Ok(())
    }

    async fn preflight_check(&self, _context: &StageContext) -> Result<(), PluginError> {
        println!("SuccessPlugin preflight check: OK");
        Ok(())
    }
    fn register_stages(&self, _registry: &mut StageRegistry) -> KernelResult<()> { Ok(()) } // Added
// Add default implementations for new trait methods
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
}

struct FailPlugin;
#[async_trait]
impl Plugin for FailPlugin {
    fn name(&self) -> &'static str { "FailPlugin" }
    fn version(&self) -> &str { "1.0.0" }
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(151) }
    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        vec![">=0.1.0".parse().unwrap()]
    }
    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }
    fn required_stages(&self) -> Vec<crate::stage_manager::requirement::StageRequirement> { vec![] }
    fn shutdown(&self) -> KernelResult<()> { Ok(()) }

    fn init(&self, _app: &mut Application) -> KernelResult<()> {
        println!("FailPlugin::init called (SHOULD NOT HAPPEN)");
        panic!("FailPlugin init should not be called after failed preflight!");
    }

    async fn preflight_check(&self, _context: &StageContext) -> Result<(), PluginError> {
        println!("FailPlugin preflight check: FAIL");
        Err(PluginError::PreflightCheckError("Simulated preflight failure".to_string()))
    }
    fn register_stages(&self, _registry: &mut StageRegistry) -> KernelResult<()> { Ok(()) } // Added

    // Add default implementations for new trait methods
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
}

struct DefaultPlugin;
#[async_trait]
impl Plugin for DefaultPlugin {
    fn name(&self) -> &'static str { "DefaultPlugin" }
    fn version(&self) -> &str { "1.0.0" }
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(152) }
    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        vec![">=0.1.0".parse().unwrap()]
    }
    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }
    fn required_stages(&self) -> Vec<crate::stage_manager::requirement::StageRequirement> { vec![] }
    fn shutdown(&self) -> KernelResult<()> { Ok(()) }

    fn init(&self, _app: &mut Application) -> KernelResult<()> {
        println!("DefaultPlugin::init called");
        Ok(())
    }
    // Uses default preflight_check implementation (always Ok)
    fn register_stages(&self, _registry: &mut StageRegistry) -> KernelResult<()> { Ok(()) } // Added

    // Add default implementations for new trait methods
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
}


// --- Test Setup ---

async fn setup_test_environment() -> (StageContext, Arc<Mutex<PluginRegistry>>, SharedStageRegistry) {
    // Create registry and add plugins
    let api_version = ApiVersion::from_str("0.1.0").unwrap(); // Use imported ApiVersion
    let plugin_registry_arc = Arc::new(Mutex::new(PluginRegistry::new(api_version)));
    {
        let mut reg = plugin_registry_arc.lock().await;
        // Use expect for clearer panic messages in test setup
        reg.register_plugin(Box::new(SuccessPlugin)).expect("Failed to register SuccessPlugin");
        reg.register_plugin(Box::new(FailPlugin)).expect("Failed to register FailPlugin");
        reg.register_plugin(Box::new(DefaultPlugin)).expect("Failed to register DefaultPlugin");
    }

    // Create StageRegistry
    let stage_registry_arc = SharedStageRegistry::new(); // Create SharedStageRegistry

    // Create context and add registries
    let mut context = StageContext::new_live(std::env::temp_dir()); // Use temp dir for config
    context.set_data(PLUGIN_REGISTRY_KEY, plugin_registry_arc.clone());
    context.set_data("stage_registry_arc", stage_registry_arc.clone()); // Add StageRegistry to context

    // Add a placeholder Application to context for init stage signature
    // This is a simplification for the test. A real scenario needs the actual Application.
    let placeholder_app = Application::new().expect("Failed to create placeholder app"); // Create a dummy app
    context.set_data(APPLICATION_KEY, placeholder_app); // Add it to context

    (context, plugin_registry_arc, stage_registry_arc)
}


// --- Tests ---

#[tokio::test]
async fn test_preflight_check_stage_execution() {
    let (mut context, _plugin_registry, _stage_registry) = setup_test_environment().await;
    let stage = PluginPreflightCheckStage;

    // Execute the preflight stage
    let result = stage.execute(&mut context).await;

    // Assertions
    // The stage itself should now return Ok unless there's a fundamental stage execution error.
    // Individual plugin preflight failures are logged and recorded in the context.
    assert!(result.is_ok(), "Stage execute should return Ok even if plugins fail preflight. Error: {:?}", result.err());

    // Check context for failures
    let failures = context.get_data::<HashSet<String>>(PREFLIGHT_FAILURES_KEY);
    assert!(failures.is_some(), "Preflight failures should be stored in context");

    let failures_set = failures.unwrap();
    assert_eq!(failures_set.len(), 1, "Only FailPlugin should have failed preflight");
    assert!(failures_set.contains("FailPlugin"), "Failures set should contain FailPlugin ID");
    assert!(!failures_set.contains("SuccessPlugin"), "Failures set should not contain SuccessPlugin ID");
    assert!(!failures_set.contains("DefaultPlugin"), "Failures set should not contain DefaultPlugin ID");
}

#[tokio::test]
async fn test_initialization_stage_skips_failed_preflight() {
    let (mut context, _plugin_registry, _stage_registry) = setup_test_environment().await;
    let preflight_stage = PluginPreflightCheckStage;
    let init_stage = PluginInitializationStage;

    // --- Simulate Preflight Stage Execution ---
    // Run preflight first to populate the failures in the context
    let _preflight_result = preflight_stage.execute(&mut context).await;
    // We expect an error from preflight, but proceed to test init stage behavior

    // --- Execute Initialization Stage ---
    // Execute the init stage
    let init_result = init_stage.execute(&mut context).await;

    // Assertions
    // The main assertion is that the stage executes successfully.
    // If it tried to init FailPlugin (which panics), this would return Err or panic.
    // Successfully returning Ok implies FailPlugin was correctly skipped.
    assert!(init_result.is_ok(), "Initialization stage should succeed, implying failed plugins were skipped. Error: {:?}", init_result.err());

    // Cannot reliably check SuccessPlugin/DefaultPlugin init via tracker anymore,
    // as the tracker logic was correctly removed from core_stages.rs.
    // The lack of panic from FailPlugin::init is the key verification here.
}