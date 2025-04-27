use gini_core::plugin_system::{
    Plugin, PluginDependency, PluginPriority, traits::PluginError, version::VersionRange,
};
use gini_core::stage_manager::{Stage, StageContext, requirement::StageRequirement};
use gini_core::kernel::bootstrap::Application; // For init method signature
use gini_core::kernel::Result as KernelResult; // Removed unused KernelError import
use async_trait::async_trait;
use std::env;

struct CompatCheckPlugin;

#[async_trait]
impl Plugin for CompatCheckPlugin {
    fn name(&self) -> &'static str {
        "CompatCheckExample"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn is_core(&self) -> bool {
        false // This is an example plugin, not core
    }

    fn priority(&self) -> PluginPriority {
        PluginPriority::ThirdParty(151) // Default third-party priority
    }

    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        // Example: Compatible with API version 0.1.x
        vec![VersionRange::from_constraint("~0.1.0").expect("Invalid version range constraint")]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![] // No dependencies for this simple example
    }

    fn required_stages(&self) -> Vec<StageRequirement> {
        vec![] // No specific stage requirements for this example
    }

    fn init(&self, _app: &mut Application) -> KernelResult<()> {
        // No complex initialization needed for this example
        println!("CompatCheckPlugin initialized (placeholder).");
        Ok(())
    }

    async fn preflight_check(&self, _context: &StageContext) -> Result<(), PluginError> {
        println!("Running preflight check for CompatCheckPlugin...");
        match env::var("GINI_COMPAT_CHECK_PASS") {
            Ok(val) if val == "1" => {
                println!("Preflight check passed (GINI_COMPAT_CHECK_PASS=1).");
                Ok(())
            }
            _ => {
                let err_msg = "Preflight check failed: GINI_COMPAT_CHECK_PASS environment variable not set to '1'.";
                println!("{}", err_msg);
                // Use the correct error variant
                Err(PluginError::PreflightCheckError(err_msg.to_string()))
            }
        }
    }

    fn stages(&self) -> Vec<Box<dyn Stage>> {
        vec![] // This plugin doesn't provide any stages
    }

    fn shutdown(&self) -> KernelResult<()> {
        // No complex shutdown needed for this example
        println!("CompatCheckPlugin shut down (placeholder).");
        Ok(())
    }
}

/// The entry point function for the plugin loader.
#[no_mangle]
pub extern "C" fn _plugin_init() -> *mut dyn Plugin {
    // Create the plugin instance boxed.
    let plugin = Box::new(CompatCheckPlugin);
    // Convert the Box into a raw pointer, leaking the memory.
    // The PluginManager is now responsible for this memory.
    Box::into_raw(plugin)
}

// Module for tests
#[cfg(test)]
mod tests;