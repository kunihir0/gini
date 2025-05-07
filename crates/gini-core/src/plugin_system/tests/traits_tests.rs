// crates/gini-core/src/plugin_system/tests/traits_tests.rs
#![cfg(test)]

use crate::plugin_system::traits::{Plugin, PluginError, PluginPriority};
use crate::plugin_system::version::VersionRange;
use crate::plugin_system::dependency::PluginDependency;
use crate::stage_manager::context::StageContext;
use crate::stage_manager::requirement::StageRequirement;
// Removed unused: use crate::stage_manager::Stage;
use crate::stage_manager::registry::StageRegistry; // Added for register_stages
use crate::kernel::error::Result as KernelResult;
use crate::kernel::bootstrap::Application; // Needed for Plugin::init signature
use async_trait::async_trait;
use std::path::PathBuf; // For dummy StageContext

#[test]
fn test_priority_value() {
    assert_eq!(PluginPriority::Kernel(5).value(), 5);
    assert_eq!(PluginPriority::CoreCritical(30).value(), 30);
    assert_eq!(PluginPriority::Core(70).value(), 70);
    assert_eq!(PluginPriority::ThirdPartyHigh(120).value(), 120);
    assert_eq!(PluginPriority::ThirdParty(180).value(), 180);
    assert_eq!(PluginPriority::ThirdPartyLow(220).value(), 220);
}

#[test]
fn test_priority_from_str_valid() {
    assert_eq!(PluginPriority::from_str("kernel:5"), Some(PluginPriority::Kernel(5)));
    assert_eq!(PluginPriority::from_str("core_critical:30"), Some(PluginPriority::CoreCritical(30)));
    assert_eq!(PluginPriority::from_str("corecritical:45"), Some(PluginPriority::CoreCritical(45))); // Alias
    assert_eq!(PluginPriority::from_str("core:70"), Some(PluginPriority::Core(70)));
    assert_eq!(PluginPriority::from_str("third_party_high:120"), Some(PluginPriority::ThirdPartyHigh(120)));
    assert_eq!(PluginPriority::from_str("thirdpartyhigh:145"), Some(PluginPriority::ThirdPartyHigh(145))); // Alias
    assert_eq!(PluginPriority::from_str("third_party:180"), Some(PluginPriority::ThirdParty(180)));
    assert_eq!(PluginPriority::from_str("thirdparty:199"), Some(PluginPriority::ThirdParty(199))); // Alias
    assert_eq!(PluginPriority::from_str("third_party_low:220"), Some(PluginPriority::ThirdPartyLow(220)));
    assert_eq!(PluginPriority::from_str("thirdpartylow:255"), Some(PluginPriority::ThirdPartyLow(255))); // Alias
}

#[test]
fn test_priority_from_str_invalid() {
    // Wrong format
    assert_eq!(PluginPriority::from_str("kernel"), None);
    assert_eq!(PluginPriority::from_str("core:"), None);
    assert_eq!(PluginPriority::from_str(":50"), None);
    assert_eq!(PluginPriority::from_str("core: 70"), None); // Space
    assert_eq!(PluginPriority::from_str("core :70"), None); // Space

    // Unknown type
    assert_eq!(PluginPriority::from_str("unknown:50"), None);

    // Out-of-range value
    assert_eq!(PluginPriority::from_str("kernel:11"), None); // > 10
    assert_eq!(PluginPriority::from_str("core_critical:10"), None); // < 11
    assert_eq!(PluginPriority::from_str("core_critical:51"), None); // > 50
    assert_eq!(PluginPriority::from_str("core:50"), None); // < 51
    assert_eq!(PluginPriority::from_str("core:101"), None); // > 100
    assert_eq!(PluginPriority::from_str("third_party_high:100"), None); // < 101
    assert_eq!(PluginPriority::from_str("third_party_high:151"), None); // > 150
    assert_eq!(PluginPriority::from_str("third_party:150"), None); // < 151
    assert_eq!(PluginPriority::from_str("third_party:201"), None); // > 200
    assert_eq!(PluginPriority::from_str("third_party_low:200"), None); // < 201
    // 256 is not parsable as u8
    assert_eq!(PluginPriority::from_str("third_party_low:256"), None);
}

#[test]
fn test_priority_display_format() {
    assert_eq!(format!("{}", PluginPriority::Kernel(5)), "kernel:5");
    assert_eq!(format!("{}", PluginPriority::CoreCritical(30)), "core_critical:30");
    assert_eq!(format!("{}", PluginPriority::Core(70)), "core:70");
    assert_eq!(format!("{}", PluginPriority::ThirdPartyHigh(120)), "third_party_high:120");
    assert_eq!(format!("{}", PluginPriority::ThirdParty(180)), "third_party:180");
    assert_eq!(format!("{}", PluginPriority::ThirdPartyLow(220)), "third_party_low:220");
}

#[test]
fn test_priority_ordering() {
    // Different types
    assert!(PluginPriority::Kernel(5) < PluginPriority::CoreCritical(30));
    assert!(PluginPriority::CoreCritical(30) < PluginPriority::Core(70));
    assert!(PluginPriority::Core(70) < PluginPriority::ThirdPartyHigh(120));
    assert!(PluginPriority::ThirdPartyHigh(120) < PluginPriority::ThirdParty(180));
    assert!(PluginPriority::ThirdParty(180) < PluginPriority::ThirdPartyLow(220));

    // Same type, different values (lower value = higher priority)
    assert!(PluginPriority::Kernel(5) < PluginPriority::Kernel(10));
    assert!(PluginPriority::Core(60) < PluginPriority::Core(70));
    assert!(PluginPriority::ThirdPartyLow(210) < PluginPriority::ThirdPartyLow(220));

    // Equality
    assert_eq!(PluginPriority::Core(70), PluginPriority::Core(70));
    assert_ne!(PluginPriority::Core(70), PluginPriority::Core(71));
    assert_ne!(PluginPriority::Core(70), PluginPriority::ThirdParty(70)); // Same value, different type
}

#[test]
fn test_plugin_error_display_format() {
    assert_eq!(format!("{}", PluginError::InitError("Failed init".to_string())), "Plugin initialization error: Failed init");
    assert_eq!(format!("{}", PluginError::LoadError("Failed load".to_string())), "Plugin loading error: Failed load");
    assert_eq!(format!("{}", PluginError::ExecutionError("Failed exec".to_string())), "Plugin execution error: Failed exec");
    assert_eq!(format!("{}", PluginError::DependencyError("Missing dep".to_string())), "Plugin dependency error: Missing dep");
    assert_eq!(format!("{}", PluginError::VersionError("Bad version".to_string())), "Plugin version error: Bad version");
    assert_eq!(format!("{}", PluginError::PreflightCheckError("Failed preflight".to_string())), "Plugin pre-flight check error: Failed preflight");
}

// --- Mock Plugin for Trait Test ---
struct MockTraitPlugin;

#[async_trait]
impl Plugin for MockTraitPlugin {
    fn name(&self) -> &'static str { "MockTraitPlugin" }
    fn version(&self) -> &str { "0.0.0" }
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(151) }
    fn compatible_api_versions(&self) -> Vec<VersionRange> { vec![] }
    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }
    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }
    fn init(&self, _app: &mut Application) -> KernelResult<()> { Ok(()) }
    // Default preflight_check is used
    fn shutdown(&self) -> KernelResult<()> { Ok(()) }
    fn register_stages(&self, _registry: &mut StageRegistry) -> KernelResult<()> { Ok(()) } // Added
// Add default implementations for new trait methods
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
}

#[tokio::test]
async fn test_plugin_trait_default_preflight() {
    let plugin = MockTraitPlugin;
    let context = StageContext::new_dry_run(PathBuf::new()); // Create dummy context
    let result = plugin.preflight_check(&context).await;
    assert!(result.is_ok());
}