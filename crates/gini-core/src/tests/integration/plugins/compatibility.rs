#![cfg(test)]

use std::collections::HashSet;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;
use async_trait::async_trait;

use crate::kernel::bootstrap::Application;
use crate::kernel::component::KernelComponent;
use crate::kernel::constants::API_VERSION;
use crate::kernel::error::{Error, Result as KernelResult};
use crate::plugin_system::dependency::PluginDependency;
use crate::plugin_system::traits::{Plugin, PluginPriority, PluginError as TraitsPluginError};
use crate::plugin_system::version::VersionRange;
use crate::stage_manager::{Stage, StageContext, StageResult};
use crate::stage_manager::requirement::StageRequirement;
use crate::storage::manager::DefaultStorageManager;
use crate::plugin_system::manager::DefaultPluginManager;
use std::path::PathBuf;

use super::super::common::{setup_test_environment, TestPlugin, DependentPlugin, ShutdownBehavior, PreflightBehavior, VersionedPlugin};

// Define the incompatible plugin locally for this test
struct IncompatiblePlugin {
    id: String, // Keep id for potential future use or if name() needs it
    incompatible_api_version_range: VersionRange,
}

impl IncompatiblePlugin {
    // Use a static name to avoid lifetime issues with name() -> &'static str
    const PLUGIN_NAME: &'static str = "IncompatibleTestPlugin";

    fn new(id_suffix: &str, incompatible_version_req: &str) -> Self {
        IncompatiblePlugin {
            id: format!("{}_{}", Self::PLUGIN_NAME, id_suffix), // Create a unique ID
            incompatible_api_version_range: incompatible_version_req.parse().expect("Invalid version range for test"),
        }
    }
}

#[async_trait]
impl Plugin for IncompatiblePlugin {
    fn name(&self) -> &'static str {
        Self::PLUGIN_NAME // Return the static name
    }

    // fn id(&self) -> &str { &self.id } // Only needed if name() isn't static

    fn version(&self) -> &str { "0.1.0" }
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(151) }

    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        vec![self.incompatible_api_version_range.clone()]
    }

    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }
    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }

    fn init(&self, _app: &mut Application) -> crate::kernel::error::Result<()> { Ok(()) } // Use kernel::error::Result
    async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), TraitsPluginError> { Ok(()) } // Use aliased error
    fn stages(&self) -> Vec<Box<dyn Stage>> { vec![] } // Correct return type
    fn shutdown(&self) -> crate::kernel::error::Result<()> { Ok(()) } // Use kernel::error::Result
// Add default implementations for new trait methods
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
}

#[tokio::test]
async fn test_plugin_api_compatibility() {
    // Destructure the new shutdown_order tracker, even if unused here
    let (plugin_manager, _, _, _, _, _shutdown_order) = setup_test_environment().await;

    // Initialize components
    KernelComponent::initialize(&*plugin_manager).await.expect("Failed to initialize plugin manager");

    // Create an incompatible plugin (requires API ">2.0.0" when core is "0.1.0")
    let incompatible_plugin = IncompatiblePlugin::new("TestSuffix", ">2.0.0");
    let plugin_name = incompatible_plugin.name(); // Get the static name

    // Attempt to register the incompatible plugin
    let result = {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(incompatible_plugin))
    };

    // Verify registration failed
    assert!(result.is_err(), "Registration of incompatible plugin should fail");

    // Check for the specific kernel::error::Error::Plugin variant and message content
    match result.err().unwrap() {
        Error::Plugin(msg) => {
            eprintln!("API Compatibility Error Message: {}", msg); // Print message for debugging
            assert!(
                msg.contains("is not compatible with API version") || msg.contains("API version mismatch"),
                "Expected API incompatibility error message, but got: {}", msg
            );
            // Check that the message contains the plugin name and the core API version
            assert!(msg.contains(plugin_name), "Error message should contain plugin name '{}'", plugin_name);
            assert!(msg.contains(API_VERSION), "Error message should mention the core API version '{}'", API_VERSION);
        }
        e => panic!("Expected Error::Plugin for API incompatibility, but got {:?}", e),
    }

    // Verify the plugin was not actually added using its static name
    {
        let registry = plugin_manager.registry().lock().await;
        assert!(registry.get_plugin(plugin_name).is_none(), "Incompatible plugin should not be in the registry");
    }
}

#[tokio::test]
async fn test_register_all_plugins_api_compatibility_detailed() -> KernelResult<()> {
    // Setup environment - Need stages_executed tracker for VersionedPlugin
    let (plugin_manager, _, _, stages_executed, _, _) = setup_test_environment().await;
    KernelComponent::initialize(&*plugin_manager).await?;

    // Core API version (assuming it's parseable, e.g., "0.1.0")
    let core_api_semver = semver::Version::parse(API_VERSION).expect("Failed to parse core API_VERSION");

    // Create plugins with different API compatibilities
    let compatible_plugin = super::super::common::VersionedPlugin::new( // Adjusted path
        "ApiCompatPlugin",
        "1.0.0",
        vec![API_VERSION.parse().unwrap()], // Exactly matches core API
        stages_executed.clone() // Pass the correct tracker type (Arc<tokio::sync::Mutex<HashSet<String>>>)
    );
    let incompatible_plugin_newer = super::super::common::VersionedPlugin::new( // Adjusted path
        "ApiIncompatNewer",
        "1.0.0",
        vec![format!(">{}.{}.{}", core_api_semver.major, core_api_semver.minor, core_api_semver.patch).parse().unwrap()], // Requires newer API
        stages_executed.clone() // Pass the correct tracker type
    );
     let incompatible_plugin_older = super::super::common::VersionedPlugin::new( // Adjusted path
        "ApiIncompatOlder",
        "1.0.0",
        vec![format!("<{}.{}.{}", core_api_semver.major, core_api_semver.minor, core_api_semver.patch).parse().unwrap()], // Requires older API
        stages_executed.clone() // Pass the correct tracker type
    );
    let compatible_plugin_range = super::super::common::VersionedPlugin::new( // Adjusted path
        "ApiCompatRange",
        "1.0.0",
        vec![format!("^{}.{}", core_api_semver.major, core_api_semver.minor).parse().unwrap()], // Compatible range (e.g., ^0.1)
        stages_executed.clone() // Pass the correct tracker type
    );

    let compat_name = compatible_plugin.name().to_string();
    let incompat_newer_name = incompatible_plugin_newer.name().to_string();
    let incompat_older_name = incompatible_plugin_older.name().to_string();
    let compat_range_name = compatible_plugin_range.name().to_string();

    // Register all plugins - registration should succeed even for incompatible ones
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(compatible_plugin))?;
        // Registration for incompatible plugins should fail based on current registry logic
        assert!(registry.register_plugin(Box::new(incompatible_plugin_newer)).is_err(), "Registering newer incompatible should fail");
        assert!(registry.register_plugin(Box::new(incompatible_plugin_older)).is_err(), "Registering older incompatible should fail");
        registry.register_plugin(Box::new(compatible_plugin_range))?;
    }

    // Attempt to initialize all registered and enabled plugins
    let mut app = Application::new(None).unwrap();
    let init_result = {
        let mut registry = plugin_manager.registry().lock().await;
        // initialize_all checks compatibility before initializing
        registry.initialize_all(&mut app)
    };

    // initialize_all should succeed overall, but only initialize compatible plugins
    assert!(init_result.is_ok(), "initialize_all should succeed even if some plugins are skipped");

    // Verify which plugins were actually initialized
    {
        let registry = plugin_manager.registry().lock().await;
        assert!(registry.initialized.contains(&compat_name), "Compatible plugin should be initialized");
        assert!(registry.initialized.contains(&compat_range_name), "Compatible range plugin should be initialized");
        // Incompatible plugins shouldn't be in the registry at all if registration failed
        assert!(!registry.plugins.contains_key(&incompat_newer_name), "Incompatible newer plugin should not be in registry");
        assert!(!registry.plugins.contains_key(&incompat_older_name), "Incompatible older plugin should not be in registry");
        assert!(!registry.initialized.contains(&incompat_newer_name), "Incompatible newer plugin should not be initialized");
        assert!(!registry.initialized.contains(&incompat_older_name), "Incompatible older plugin should not be initialized");
    }

    Ok(())
}