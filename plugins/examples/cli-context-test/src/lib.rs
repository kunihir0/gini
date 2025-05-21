use async_trait::async_trait;
use gini_core::kernel::Application;
use gini_core::kernel::constants::API_VERSION; // Ensure this is the correct path
use gini_core::plugin_system::{
    conflict::ResourceClaim,
    dependency::PluginDependency,
    error::PluginSystemError, // Ensure this is the correct path
    // manifest::PluginManifest, // Not directly used for trait methods
    traits::{Plugin, PluginPriority},
    version::VersionRange, // Ensure this is the correct path
};
use gini_core::stage_manager::{
    error::StageSystemError, // Ensure this is the correct path
    requirement::StageRequirement,
    Stage, StageContext, StageRegistry, // StageResult removed
};
use log::{info, error};
// std::sync::Arc removed

const PLUGIN_ID: &str = "cli-context-test-plugin";
const PLUGIN_NAME: &str = "CLI Context Test Plugin";
const PLUGIN_VERSION_MAJOR: u16 = 0;
const PLUGIN_VERSION_MINOR: u16 = 1;
const PLUGIN_VERSION_PATCH: u16 = 0;
// PLUGIN_DESCRIPTION removed
// PLUGIN_AUTHOR removed


const STAGE_ID: &str = "cli_context_test_stage";
const STAGE_NAME: &str = "CLI Context Test Stage";
const STAGE_DESCRIPTION: &str = "Reads 'test_message' from context (set via CLI) and logs it.";
const CONTEXT_VAR_NAME: &str = "test_message";

#[derive(Debug)]
pub struct CliContextTestPlugin {
    version_str: String,
}

impl CliContextTestPlugin {
    pub fn new() -> Self {
        CliContextTestPlugin {
            version_str: format!("{}.{}.{}", PLUGIN_VERSION_MAJOR, PLUGIN_VERSION_MINOR, PLUGIN_VERSION_PATCH),
        }
    }
}

// Note: Plugin trait methods are NOT async
impl Plugin for CliContextTestPlugin {
    fn name(&self) -> &'static str {
        PLUGIN_NAME
    }

    fn version(&self) -> &str {
        &self.version_str
    }

    fn is_core(&self) -> bool {
        false
    }

    fn priority(&self) -> PluginPriority {
        PluginPriority::ThirdParty(151) // Default priority for a third-party plugin
    }

    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        // Example: Compatible with the current API_VERSION
        // Assuming VersionRange has a constructor like this or similar helper
        // For now, let's parse from the string constant.
        // This might need adjustment based on actual VersionRange API.
        // A common pattern is to be compatible with the major version of the API.
        let api_ver_req = format!("^{}", API_VERSION); // e.g. "^0.1.0"
        match VersionRange::from_constraint(&api_ver_req) {
            Ok(vr) => vec![vr],
            Err(_) => {
                error!("Failed to parse API_VERSION constraint: {}", api_ver_req);
                Vec::new() // Fallback to empty or handle error appropriately
            }
        }
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        Vec::new()
    }

    fn required_stages(&self) -> Vec<StageRequirement> {
        Vec::new()
    }

    fn conflicts_with(&self) -> Vec<String> {
        Vec::new()
    }

    fn incompatible_with(&self) -> Vec<PluginDependency> {
        Vec::new()
    }

    fn declared_resources(&self) -> Vec<ResourceClaim> {
        Vec::new()
    }

    fn init(&self, _app: &mut Application) -> Result<(), PluginSystemError> {
        info!("Plugin '{}' initialized.", PLUGIN_NAME);
        Ok(())
    }

    fn register_stages(&self, registry: &mut StageRegistry) -> Result<(), PluginSystemError> {
        let stage = Box::new(CliContextTestStage); // Use Box instead of Arc
        match registry.register_stage(stage) {
            Ok(_) => {
                info!("Stage '{}' registered by plugin '{}'", STAGE_ID, PLUGIN_NAME);
                Ok(())
            }
            Err(e) => {
                let err_msg = format!("Failed to register stage '{}' for plugin '{}': {}", STAGE_ID, PLUGIN_NAME, e);
                error!("{}", err_msg);
                // Use the RegistrationError variant
                Err(PluginSystemError::RegistrationError {
                    plugin_id: PLUGIN_ID.to_string(), // Use PLUGIN_ID for consistency
                    message: err_msg,
                })
            }
        }
    }

    fn shutdown(&self) -> Result<(), PluginSystemError> {
        info!("Plugin '{}' shutting down.", PLUGIN_NAME);
        Ok(())
    }

    // async fn preflight_check - not overriding, default is Ok(())
}

#[derive(Debug)]
struct CliContextTestStage;

#[async_trait]
impl Stage for CliContextTestStage {
    fn id(&self) -> &str {
        STAGE_ID
    }

    fn name(&self) -> &str {
        STAGE_NAME
    }

    fn description(&self) -> &str {
        STAGE_DESCRIPTION
    }

    // supports_dry_run() -> bool { true } // Default implementation is fine

    async fn execute(&self, context: &mut StageContext) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        info!("Executing stage: {}", self.id());

        if context.is_dry_run() {
            info!("Dry run: Would attempt to read context variable '{}'", CONTEXT_VAR_NAME);
            return Ok(());
        }

        match context.get_data::<String>(CONTEXT_VAR_NAME) {
            Some(message) => {
                info!("Successfully read context variable '{}': {}", CONTEXT_VAR_NAME, message);
                // For demonstration, we can set an output in the context
                let output_key = format!("{}_output", self.id());
                // Create a String to own the data before setting it in context
                let output_value = format!("Successfully read: {}", message);
                context.set_data(&output_key, output_value); // value is String, moved here
                Ok(())
            }
            None => {
                let error_msg = format!("Context variable '{}' not found or not a String.", CONTEXT_VAR_NAME);
                error!("{}", error_msg);
                // Return an error that fits Box<dyn Error + Send + Sync + 'static>
                Err(Box::new(StageSystemError::ContextError {
                    key: CONTEXT_VAR_NAME.to_string(),
                    reason: error_msg
                }))
            }
        }
    }
}

// FFI glue removed as this plugin is statically linked