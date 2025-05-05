use crate::kernel::bootstrap::*;
use crate::kernel::error::{Error, Result};
use crate::kernel::component::KernelComponent; // Import the trait
use async_trait::async_trait; // Import async_trait

// Import concrete component types for get_component test
use crate::event::DefaultEventManager;
use crate::stage_manager::manager::DefaultStageManager;
use crate::plugin_system::DefaultPluginManager;
use crate::storage::{DefaultStorageManager, local::LocalStorageProvider}; // Added LocalStorageProvider

use tempfile::tempdir;
use std::fs; // Import std::fs

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

    // Use std::fs here
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
    let mut app = Application::new(Some(base_path.clone())).expect("Application::new failed"); // Clone base_path

    assert!(!app.is_initialized(), "App should not be initialized after new()");

    // Run the app (includes init, start, shutdown)
    let result1 = app.run().await;
    assert!(result1.is_ok(), "First run should succeed");

    // After run completes (including shutdown), initialized should be false again
    assert!(!app.is_initialized(), "App should be uninitialized after run() completes");

    // --- Test running again ---
    // We need a new instance because the old one shut down its components internally.
    let mut app2 = Application::new(Some(base_path)).expect("App::new failed"); // Use original base_path
    let _ = app2.run().await; // Run the first time

    // Try running the *same instance* again
    let result2 = app2.run().await;
    assert!(result2.is_err(), "Second run on same instance should fail");

    // Check the specific error - it should fail when trying to re-register core stages
    match result2 {
         Err(Error::Stage(msg)) => { // Expect Stage error now
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

#[tokio::test]
async fn test_get_component() {
    let temp_dir = setup_test_env();
    let base_path = temp_dir.path().to_path_buf();
    let app = Application::new(Some(base_path)).expect("Application::new failed");

    // Retrieve each default component by its concrete type
    let event_manager = app.get_component::<DefaultEventManager>().await;
    assert!(event_manager.is_some(), "Should retrieve DefaultEventManager");

    let stage_manager = app.get_component::<DefaultStageManager>().await;
    assert!(stage_manager.is_some(), "Should retrieve DefaultStageManager");

    // Specify the generic parameter when retrieving DefaultPluginManager
    let plugin_manager = app.get_component::<DefaultPluginManager<LocalStorageProvider>>().await;
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
