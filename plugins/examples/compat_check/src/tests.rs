use super::*; // Import items from the parent module (plugin.rs)
use std::path::PathBuf; // Import PathBuf
use gini_core::plugin_system::error::PluginSystemError; // Import PluginSystemError
use gini_core::stage_manager::context::StageContext; // Import StageContext
// No need for std::env or serial_test anymore

// Helper to create a context for tests
fn create_test_context() -> StageContext {
    // Using new_live as new_dry_run might behave differently regarding data persistence
    // If dry_run behavior is specifically needed, adjust accordingly.
    let dummy_config_path = PathBuf::from("./dummy_config_for_test_context");
    StageContext::new_live(dummy_config_path)
}

#[tokio::test]
// Removed #[serial]
async fn preflight_check_pass() {
    let plugin = CompatCheckPlugin { _marker: 0 };
    let mut context = create_test_context();

    // Set the context data to make the check pass
    context.set_data(COMPAT_CHECK_CONTEXT_KEY, "1".to_string());

    let result = plugin.preflight_check(&context).await; // Pass context

    // Assert that the check passed
    assert!(result.is_ok());
}

#[tokio::test]
// Removed #[serial]
async fn preflight_check_fail_not_set() {
    let plugin = CompatCheckPlugin { _marker: 0 };
    let context = create_test_context(); // Context starts empty

    let result = plugin.preflight_check(&context).await; // Pass context

    // Assert that the check failed because the key wasn't set
    assert!(matches!(result, Err(PluginSystemError::PreflightCheckFailed{plugin_id: _, message}) if message.contains("not found")));
}

#[tokio::test]
// Removed #[serial]
async fn preflight_check_fail_wrong_value() {
    let plugin = CompatCheckPlugin { _marker: 0 };
    let mut context = create_test_context();

    // Set the context data to an incorrect value
    context.set_data(COMPAT_CHECK_CONTEXT_KEY, "0".to_string());

    let result = plugin.preflight_check(&context).await; // Pass context

    // Assert that the check failed with the correct error type and message
    assert!(matches!(result, Err(PluginSystemError::PreflightCheckFailed{plugin_id: _, message}) if message.contains("incorrect value '0'")));
}