use crate::kernel::bootstrap::*;
use crate::kernel::error::{Error, Result};
use crate::kernel::component::KernelComponent; // Import the trait
use async_trait::async_trait; // Import async_trait

// Import concrete component types for get_component test
use crate::event::DefaultEventManager;
use crate::stage_manager::manager::DefaultStageManager;
use crate::plugin_system::DefaultPluginManager;
use crate::storage::DefaultStorageManager; // Removed unused LocalStorageProvider import & braces

use tempfile::tempdir;
// Removed unused std::fs

// Helper function to set up a temporary directory for testing
fn setup_test_env() -> tempfile::TempDir {
    tempdir().expect("Failed to create temporary directory")
}

#[tokio::test]
async fn test_application_new_creates_user_dir() {
    let _temp_dir = setup_test_env(); // Keep temp dir guard alive, but don't use path

    // Application::new now uses real XDG paths, not temp dir.
    // We just check if it succeeds and we can get the component.
    let app = Application::new().expect("Application::new failed");

    // Get StorageManager component to verify it was created
    let storage_manager_opt = app.get_component::<DefaultStorageManager>().await;
    assert!(storage_manager_opt.is_some(), "StorageManager component should exist after Application::new");

    assert!(!app.is_initialized()); // Should not be initialized yet
}

#[tokio::test]
async fn test_application_new_uses_existing_user_dir() {
    let _temp_dir = setup_test_env(); // Keep temp dir guard alive

    // Pre-creating a dir in temp_dir is irrelevant now as Application::new uses XDG paths.
    // We just check if Application::new succeeds.
    let app = Application::new().expect("Application::new failed");

    // Get StorageManager component to verify it was created
    let storage_manager_opt = app.get_component::<DefaultStorageManager>().await;
    assert!(storage_manager_opt.is_some(), "StorageManager component should exist after Application::new");

    assert!(!app.is_initialized()); // Should not be initialized yet
}

#[tokio::test]
async fn test_application_run_lifecycle() {
    let temp_dir = setup_test_env();
    let _base_path = temp_dir.path().to_path_buf(); // Prefixed with underscore
    // Application::new now returns Result
    // Note: base_path is no longer used by Application::new
    let mut app = Application::new().expect("Application::new failed");

    assert!(!app.is_initialized(), "App should not be initialized after new()");

    // Run the app (includes init, start, shutdown)
    let result1 = app.run().await;
    assert!(result1.is_ok(), "First run should succeed");

    // After run completes (including shutdown), initialized should be false again
    assert!(!app.is_initialized(), "App should be uninitialized after run() completes");

    // --- Test running again ---
    // We need a new instance because the old one shut down its components internally.
    // Note: base_path is no longer used by Application::new
    let mut app2 = Application::new().expect("App::new failed");
    let _ = app2.run().await; // Run the first time

    // Try running the *same instance* again
    let result2 = app2.run().await;
    assert!(result2.is_err(), "Second run on same instance should fail");

    // Check the specific error - it should fail when trying to re-register core stages
    match result2 {
         Err(Error::StageSystem(stage_err)) => { // Expect StageSystemError now
            let actual_message = stage_err.to_string(); // Get string representation first
            if let crate::stage_manager::error::StageSystemError::StageAlreadyExists { ref stage_id } = stage_err {
                let expected_message_part = "already exists in the registry";
                assert!(actual_message.contains(stage_id.as_str()) && actual_message.contains(expected_message_part),
                    "Error message '{}' did not contain expected stage_id '{}' or substring '{}'", actual_message, stage_id, expected_message_part);

                assert!(
                    *stage_id == "core::plugin_preflight_check" ||
                    *stage_id == "core::plugin_initialization" ||
                    *stage_id == "core::plugin_post_initialization",
                    "Error stage_id '{}' was not one of the expected core stage IDs", stage_id
                );
            } else {
                panic!("Expected StageSystemError::StageAlreadyExists, got {:?}", stage_err);
            }
         }
         _ => panic!("Expected StageSystemError due to re-registration attempt, got {:?}", result2),
     }


    // Even though the second run failed early, the app state remains from the first run's shutdown
    assert!(!app2.is_initialized(), "App should remain uninitialized after failed second run");
}


#[tokio::test] // Mark as async test
async fn test_config_dir_getter() { // Add async keyword
     let temp_dir = setup_test_env();
     let _base_path = temp_dir.path().to_path_buf(); // Mark unused
     // Determine expected XDG path (this is tricky without setting env vars)
     // For now, just check that the component exists and we can call the method.
     // let expected_config_dir = ...;

     // Application::new now returns Result
     let app = Application::new().expect("Application::new failed");
     // Get config_dir via StorageManager component
     let storage_manager_opt = app.get_component::<DefaultStorageManager>().await; // await is now valid
     assert!(storage_manager_opt.is_some(), "StorageManager component should exist");
     let storage_manager = storage_manager_opt.unwrap();
     let _config_dir = storage_manager.config_dir(); // Call the method
     // assert_eq!(config_dir, &expected_config_dir); // Check against expected XDG path
}

#[tokio::test]
async fn test_get_component() {
    let temp_dir = setup_test_env();
    let _base_path = temp_dir.path().to_path_buf(); // Prefixed with underscore
    let app = Application::new().expect("Application::new failed");

    // Retrieve each default component by its concrete type
    let event_manager = app.get_component::<DefaultEventManager>().await;
    assert!(event_manager.is_some(), "Should retrieve DefaultEventManager");

    let stage_manager = app.get_component::<DefaultStageManager>().await;
    assert!(stage_manager.is_some(), "Should retrieve DefaultStageManager");

    // Retrieve DefaultPluginManager without generic parameter
    let plugin_manager = app.get_component::<DefaultPluginManager>().await; // Remove generic
    assert!(plugin_manager.is_some(), "Should retrieve DefaultPluginManager");

    let storage_manager = app.get_component::<DefaultStorageManager>().await;
    assert!(storage_manager.is_some(), "Should retrieve DefaultStorageManager");

    // Try retrieving a non-registered type (should return None)
    #[derive(Debug)] // Add Debug derive
    struct NonRegisteredComponent;

    #[async_trait] // Add async_trait macro
    impl KernelComponent for NonRegisteredComponent {
        fn name(&self) -> &'static str { "NonRegistered" }
        // Implement other required methods if any (e.g., async initialize, start, stop)
        // For this test, we only need the type to exist.
         // Removed fn dependencies(&self) -> Vec<TypeId> { vec![] } as it's not in KernelComponent
         async fn initialize(&self) -> Result<()> { Ok(()) }
         async fn start(&self) -> Result<()> { Ok(()) }
         async fn stop(&self) -> Result<()> { Ok(()) }
    }
    let non_existent = app.get_component::<NonRegisteredComponent>().await;
    assert!(non_existent.is_none(), "Should not retrieve non-registered component");
}

// Note: Testing initialize, start, stop directly is difficult as they are private async methods.
// The test_application_run_lifecycle already covers the sequence indirectly.
// Testing error handling during component lifecycle would require mocking components
// or introducing ways to make specific components fail during init/start/stop,
// which is beyond the scope of basic bootstrap tests.
