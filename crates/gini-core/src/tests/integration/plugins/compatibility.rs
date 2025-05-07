#![cfg(test)]

use async_trait::async_trait;

use crate::kernel::bootstrap::Application;
use crate::kernel::component::KernelComponent;
use crate::kernel::constants::API_VERSION;
use crate::kernel::error::{Error, Result as KernelResult};
use crate::plugin_system::dependency::PluginDependency;
use crate::plugin_system::traits::{Plugin, PluginPriority, PluginError as TraitsPluginError};
use crate::plugin_system::version::VersionRange;
use crate::stage_manager::StageContext;
use crate::stage_manager::registry::StageRegistry;
use crate::stage_manager::requirement::StageRequirement;

use super::super::common::setup_test_environment; // Assuming VersionedPlugin is here
use super::super::common::VersionedPlugin; // Explicit import for VersionedPlugin

// Define the incompatible plugin locally for this test
#[allow(dead_code)] // Allow dead code for test helper struct
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

    fn version(&self) -> &str { "0.1.0" }
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(151) }

    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        vec![self.incompatible_api_version_range.clone()]
    }

    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }
    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }

    fn init(&self, _app: &mut Application) -> crate::kernel::error::Result<()> { Ok(()) }
    async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), TraitsPluginError> { Ok(()) }
    fn shutdown(&self) -> crate::kernel::error::Result<()> { Ok(()) }
    fn register_stages(&self, _registry: &mut StageRegistry) -> KernelResult<()> { Ok(()) }
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
}

#[tokio::test]
async fn test_plugin_api_compatibility() {
    let (plugin_manager, _, _, _, _, _shutdown_order) = setup_test_environment().await;
    KernelComponent::initialize(&*plugin_manager).await.expect("Failed to initialize plugin manager");

    let incompatible_plugin = IncompatiblePlugin::new("TestSuffix", ">2.0.0");
    let plugin_name = incompatible_plugin.name();

    let result = {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(incompatible_plugin))
    };

    assert!(result.is_err(), "Registration of incompatible plugin should fail");
    match result.err().unwrap() {
        Error::Plugin(msg) => {
            eprintln!("API Compatibility Error Message: {}", msg);
            assert!(
                msg.contains("is not compatible with API version") || msg.contains("API version mismatch"),
                "Expected API incompatibility error message, but got: {}", msg
            );
            assert!(msg.contains(plugin_name), "Error message should contain plugin name '{}'", plugin_name);
            assert!(msg.contains(API_VERSION), "Error message should mention the core API version '{}'", API_VERSION);
        }
        e => panic!("Expected Error::Plugin for API incompatibility, but got {:?}", e),
    }

    {
        let registry = plugin_manager.registry().lock().await;
        assert!(registry.get_plugin(plugin_name).is_none(), "Incompatible plugin should not be in the registry");
    }
}

#[tokio::test]
async fn test_register_all_plugins_api_compatibility_detailed() -> KernelResult<()> {
    let (plugin_manager, stage_manager, _, stages_executed, _, _) = setup_test_environment().await;
    KernelComponent::initialize(&*plugin_manager).await?;
    KernelComponent::initialize(&*stage_manager).await?; // Initialize StageManager

    let core_api_semver = semver::Version::parse(API_VERSION).expect("Failed to parse core API_VERSION");

    let compatible_plugin = VersionedPlugin::new(
        "ApiCompatPlugin",
        "1.0.0",
        vec![API_VERSION.parse().unwrap()],
        stages_executed.clone()
    );
    let incompatible_plugin_newer = VersionedPlugin::new(
        "ApiIncompatNewer",
        "1.0.0",
        vec![format!(">{}.{}.{}", core_api_semver.major, core_api_semver.minor, core_api_semver.patch + 1).parse().unwrap()], // Requires newer API (e.g. >0.1.1 if core is 0.1.0)
        stages_executed.clone()
    );
     let incompatible_plugin_older = VersionedPlugin::new(
        "ApiIncompatOlder",
        "1.0.0",
        vec![format!("<{}.{}.{}", core_api_semver.major, core_api_semver.minor, core_api_semver.patch).parse().unwrap()],
        stages_executed.clone()
    );
    let compatible_plugin_range = VersionedPlugin::new(
        "ApiCompatRange",
        "1.0.0",
        vec![format!("^{}.{}", core_api_semver.major, core_api_semver.minor).parse().unwrap()],
        stages_executed.clone()
    );

    let compat_name = compatible_plugin.name().to_string();
    let incompat_newer_name = incompatible_plugin_newer.name().to_string();
    let incompat_older_name = incompatible_plugin_older.name().to_string();
    let compat_range_name = compatible_plugin_range.name().to_string();

    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(compatible_plugin))?;
        assert!(registry.register_plugin(Box::new(incompatible_plugin_newer)).is_err(), "Registering newer incompatible should fail");
        assert!(registry.register_plugin(Box::new(incompatible_plugin_older)).is_err(), "Registering older incompatible should fail");
        registry.register_plugin(Box::new(compatible_plugin_range))?;
    }

    let mut app = Application::new().unwrap();
    let init_result = {
        let mut registry = plugin_manager.registry().lock().await;
        let stage_registry_arc_clone = stage_manager.registry().registry.clone(); // stage_manager.registry() is SharedStageRegistry, then .registry is Arc<Mutex<StageRegistry>>
        registry.initialize_all(&mut app, &stage_registry_arc_clone).await
    };
    assert!(init_result.is_ok(), "initialize_all should succeed. Error: {:?}", init_result.err());

    // Verify which plugins were actually initialized
    // A plugin is initialized if:
    // 1. It's API compatible (and thus successfully registered).
    // 2. It's enabled (by default after registration, unless pre-flight fails).
    // 3. Its pre-flight check passes (all test plugins here pass pre-flight by default).
    // 4. Its dependencies are met and initialized (none here).
    // 5. Its init() method succeeds (all test plugins here succeed).
    {
        let registry = plugin_manager.registry().lock().await;
        assert!(registry.is_enabled(&compat_name), "Compatible plugin should be enabled before init check");
        assert!(registry.initialized.contains(&compat_name), "Compatible plugin should be initialized");

        assert!(registry.is_enabled(&compat_range_name), "Compatible range plugin should be enabled before init check");
        assert!(registry.initialized.contains(&compat_range_name), "Compatible range plugin should be initialized");
        
        // Incompatible plugins shouldn't be in the registry at all if registration failed as per current PluginRegistry::register_plugin logic
        assert!(!registry.plugins.contains_key(&incompat_newer_name), "Incompatible newer plugin should not be in registry");
        assert!(!registry.plugins.contains_key(&incompat_older_name), "Incompatible older plugin should not be in registry");
        
        // Consequently, they should not be initialized
        assert!(!registry.initialized.contains(&incompat_newer_name), "Incompatible newer plugin should not be initialized");
        assert!(!registry.initialized.contains(&incompat_older_name), "Incompatible older plugin should not be initialized");
    }

    Ok(())
}