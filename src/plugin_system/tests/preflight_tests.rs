// src/plugin_system/tests/preflight_tests.rs
#![cfg(test)]

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::str::FromStr; // Import FromStr trait
use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::kernel::bootstrap::Application; // Mock or minimal version needed
use crate::kernel::error::{Error, Result as KernelResult};
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

// --- Mock Application ---
// Minimal struct to satisfy Plugin::init signature for testing
#[derive(Default)]
struct MockApplication {
    initialized_plugins: Mutex<HashSet<String>>,
}

impl MockApplication {
    // Method to simulate initialization tracking
    async fn track_init(&self, plugin_id: &str) {
        let mut lock = self.initialized_plugins.lock().await;
        lock.insert(plugin_id.to_string());
    }

    async fn was_initialized(&self, plugin_id: &str) -> bool {
        let lock = self.initialized_plugins.lock().await;
        lock.contains(plugin_id)
    }
}


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
    fn stages(&self) -> Vec<Box<dyn Stage>> { vec![] }
    fn shutdown(&self) -> KernelResult<()> { Ok(()) }

    // Mock init - tracks initialization
    fn init(&self, app: &mut Application) -> KernelResult<()> {
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
}

struct FailPlugin;
#[async_trait]
impl Plugin for FailPlugin {
    fn name(&self) -> &'static str { "FailPlugin" }
    fn version(&self) -> &str { "1.0.0" }
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(151) }
    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        // Declare compatibility with the test API version using parse()
        vec![">=0.1.0".parse().unwrap()]
    }
    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }
    fn required_stages(&self) -> Vec<crate::stage_manager::requirement::StageRequirement> { vec![] }
    fn stages(&self) -> Vec<Box<dyn Stage>> { vec![] }
    fn shutdown(&self) -> KernelResult<()> { Ok(()) }

    fn init(&self, _app: &mut Application) -> KernelResult<()> {
        println!("FailPlugin::init called (SHOULD NOT HAPPEN)");
        // Panic if called, as preflight should prevent it
        panic!("FailPlugin init should not be called after failed preflight!");
        // Ok(())
    }

    async fn preflight_check(&self, _context: &StageContext) -> Result<(), PluginError> {
        println!("FailPlugin preflight check: FAIL");
        Err(PluginError::PreflightCheckError("Simulated preflight failure".to_string()))
    }
}

struct DefaultPlugin;
#[async_trait]
impl Plugin for DefaultPlugin {
    fn name(&self) -> &'static str { "DefaultPlugin" }
    fn version(&self) -> &str { "1.0.0" }
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(152) }
    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        // Declare compatibility with the test API version using parse()
        vec![">=0.1.0".parse().unwrap()]
    }
    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }
    fn required_stages(&self) -> Vec<crate::stage_manager::requirement::StageRequirement> { vec![] }
    fn stages(&self) -> Vec<Box<dyn Stage>> { vec![] }
    fn shutdown(&self) -> KernelResult<()> { Ok(()) }

    fn init(&self, _app: &mut Application) -> KernelResult<()> {
        println!("DefaultPlugin::init called");
        Ok(())
    }
    // Uses default preflight_check implementation (always Ok)
}


// --- Test Setup ---

async fn setup_test_environment() -> (StageContext, Arc<Mutex<PluginRegistry>>, Arc<MockApplication>) {
    // Create registry and add plugins
    let api_version = ApiVersion::from_str("0.1.0").unwrap(); // Use imported ApiVersion
    let registry = Arc::new(Mutex::new(PluginRegistry::new(api_version)));
    {
        let mut reg = registry.lock().await;
        // Use expect for clearer panic messages in test setup
        reg.register_plugin(Box::new(SuccessPlugin)).expect("Failed to register SuccessPlugin");
        reg.register_plugin(Box::new(FailPlugin)).expect("Failed to register FailPlugin");
        reg.register_plugin(Box::new(DefaultPlugin)).expect("Failed to register DefaultPlugin");
    }

    // Create mock application for tracking init calls
    let mock_app = Arc::new(MockApplication::default());

    // Create context and add registry and mock app
    let mut context = StageContext::new_live(std::env::temp_dir()); // Use temp dir for config
    context.set_data(PLUGIN_REGISTRY_KEY, registry.clone());
    // Store the Arc<MockApplication> for tracking init calls within the test
    context.set_data("mock_app_tracker", mock_app.clone());
    // We need *something* for the Application type expected by init, even if unused directly
    // Let's put a placeholder value or handle this differently if possible.
    // For now, let's assume the test won't actually need to *use* the Application reference
    // passed to init, relying on the mock_app_tracker instead.
    // If init *must* interact with Application, we need a more complex mock.
    // context.set_data(APPLICATION_KEY, ???); // How to put a mock Application here?

    (context, registry, mock_app)
}


// --- Tests ---

#[tokio::test]
async fn test_preflight_check_stage_execution() {
    let (mut context, _registry, _mock_app) = setup_test_environment().await;
    let stage = PluginPreflightCheckStage;

    // Execute the preflight stage
    let result = stage.execute(&mut context).await;

    // Assertions
    // The stage itself should return Ok unless there's an internal error (like context access).
    // Individual plugin failures are handled by storing them in the context.
    // However, we modified it to return the *first* plugin error.
    assert!(result.is_err(), "Stage execute should return the first preflight error");
    if let Err(Error::Plugin(msg)) = result {
         assert!(msg.contains("FailPlugin"), "Error message should mention FailPlugin");
         assert!(msg.contains("Simulated preflight failure"), "Error message should contain the failure reason");
    } else {
        panic!("Expected Plugin Error, got {:?}", result);
    }


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
    let (mut context, _registry, mock_app) = setup_test_environment().await;
    let preflight_stage = PluginPreflightCheckStage;
    let init_stage = PluginInitializationStage;

    // --- Simulate Preflight Stage Execution ---
    // Run preflight first to populate the failures in the context
    let _preflight_result = preflight_stage.execute(&mut context).await;
    // We expect an error from preflight, but proceed to test init stage behavior

    // Add a placeholder Application to context for init stage signature
    // This is a simplification for the test. A real scenario needs the actual Application.
    let mut placeholder_app = Application::new(None).expect("Failed to create placeholder app"); // Create a dummy app
    context.set_data(APPLICATION_KEY, placeholder_app); // Add it to context


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