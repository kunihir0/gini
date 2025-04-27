use crate::kernel::bootstrap::*;
use crate::kernel::error::Error;
// Import component traits to potentially test retrieval later
use crate::event::EventManager;
use crate::plugin_system::PluginManager;
use crate::stage_manager::manager::StageManager;
use crate::storage::StorageManager;

use std::env;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

// Helper function to set up a temporary directory for testing
fn setup_test_env() -> tempfile::TempDir {
    tempdir().expect("Failed to create temporary directory")
}

#[test]
fn test_application_new_creates_user_dir() {
    let temp_dir = setup_test_env();
    let base_path = temp_dir.path().to_path_buf();
    let expected_user_dir = base_path.join("user");

    assert!(!expected_user_dir.exists());

    // Application::new now returns Result
    let app = Application::new(Some(base_path)).expect("Application::new failed");

    assert!(expected_user_dir.exists());
    assert!(expected_user_dir.is_dir());
    assert_eq!(app.config_dir(), &expected_user_dir);
    assert!(!app.is_initialized()); // Should not be initialized yet
}

#[test]
fn test_application_new_uses_existing_user_dir() {
    let temp_dir = setup_test_env();
    let base_path = temp_dir.path().to_path_buf();
    let expected_user_dir = base_path.join("user");

    fs::create_dir(&expected_user_dir).expect("Failed to pre-create user dir");
    assert!(expected_user_dir.exists());

    // Application::new now returns Result
    let app = Application::new(Some(base_path)).expect("Application::new failed");

    assert!(expected_user_dir.exists());
    assert_eq!(app.config_dir(), &expected_user_dir);
    assert!(!app.is_initialized()); // Should not be initialized yet
}

#[tokio::test]
async fn test_application_run_lifecycle() {
    let temp_dir = setup_test_env();
    let base_path = temp_dir.path().to_path_buf();
    // Application::new now returns Result
    let mut app = Application::new(Some(base_path)).expect("Application::new failed");

    assert!(!app.is_initialized(), "App should not be initialized after new()");

    // Run the app (includes init, start, shutdown)
    let result1 = app.run().await;
    assert!(result1.is_ok(), "First run should succeed");

    // After run completes (including shutdown), initialized should be false again
    assert!(!app.is_initialized(), "App should be uninitialized after run() completes");

    // --- Test running again ---
    // We need a new instance because the old one shut down its components internally.
    // Re-running `run` on the same instance might have unintended side effects
    // depending on component stop logic. The current check prevents re-running anyway.
    let mut app2 = Application::new(Some(temp_dir.path().to_path_buf())).expect("App::new failed");
    let _ = app2.run().await; // Run the first time

    // Try running the *same instance* again
    let result2 = app2.run().await;
    assert!(result2.is_err(), "Second run on same instance should fail");

    // Check the specific error - the second run attempts re-initialization, which fails
    // because the StageManager tries to re-register core stages.
    match result2 {
        Err(Error::Stage(msg)) => {
            assert!(msg.contains("Stage already exists"), "Error message should indicate stage already exists");
            // Check for any of the core stages, as the order isn't guaranteed
            assert!(
                msg.contains("core::plugin_preflight_check") ||
                msg.contains("core::plugin_initialization") ||
                msg.contains("core::plugin_post_initialization"),
                "Error message should mention a core stage ID"
            );
        }
        _ => panic!("Expected Stage error due to re-registration attempt, got {:?}", result2),
    }

    // Even though the second run failed early, the app state remains from the first run's shutdown
    assert!(!app2.is_initialized(), "App should remain uninitialized after failed second run");
}


#[test]
fn test_config_dir_getter() {
     let temp_dir = setup_test_env();
     let base_path = temp_dir.path().to_path_buf();
     let expected_user_dir = base_path.join("user");

     // Application::new now returns Result
     let app = Application::new(Some(base_path)).expect("Application::new failed");
     assert_eq!(app.config_dir(), &expected_user_dir);
}

// TODO: Add tests for component retrieval (get_component)
// TODO: Add tests for component lifecycle methods (initialize, start, stop) indirectly via run
// TODO: Add tests for error handling during component lifecycle
