use super::*; // Import items from the parent module (plugin.rs)
use std::env;
use std::path::PathBuf; // Import PathBuf
use gini_core::plugin_system::traits::PluginError; // Correct path for PluginError
use gini_core::stage_manager::context::StageContext; // Import StageContext

#[tokio::test]
async fn preflight_check_pass() {
    // Set the environment variable to make the check pass
    env::set_var("GINI_COMPAT_CHECK_PASS", "1");

    let plugin = CompatCheckPlugin;
    // Create a dry run context with a dummy path
    let dummy_config_path = PathBuf::from("./dummy_config_for_test");
    let context = StageContext::new_dry_run(dummy_config_path);
    let result = plugin.preflight_check(&context).await; // Pass context

    // Assert that the check passed
    assert!(result.is_ok());

    // Clean up the environment variable
    env::remove_var("GINI_COMPAT_CHECK_PASS");
}

#[tokio::test]
async fn preflight_check_fail_not_set() {
    // Ensure the environment variable is not set
    env::remove_var("GINI_COMPAT_CHECK_PASS");

    let plugin = CompatCheckPlugin;
    let dummy_config_path = PathBuf::from("./dummy_config_for_test");
    let context = StageContext::new_dry_run(dummy_config_path); // Create context
    let result = plugin.preflight_check(&context).await; // Pass context

    // Assert that the check failed with the correct error type and variant
    assert!(matches!(result, Err(PluginError::PreflightCheckError(_)))); // Check specific variant

    // No need to clean up env var here as it wasn't set
}

#[tokio::test]
async fn preflight_check_fail_wrong_value() {
    // Set the environment variable to a value other than "1"
    env::set_var("GINI_COMPAT_CHECK_PASS", "0");

    let plugin = CompatCheckPlugin;
    let dummy_config_path = PathBuf::from("./dummy_config_for_test");
    let context = StageContext::new_dry_run(dummy_config_path); // Create context
    let result = plugin.preflight_check(&context).await; // Pass context

    // Assert that the check failed with the correct error type and variant
    assert!(matches!(result, Err(PluginError::PreflightCheckError(_)))); // Check specific variant

    // Clean up the environment variable
    env::remove_var("GINI_COMPAT_CHECK_PASS");
}