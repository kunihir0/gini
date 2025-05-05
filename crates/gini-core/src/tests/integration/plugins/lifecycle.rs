#![cfg(test)]


use crate::kernel::bootstrap::Application;
use crate::kernel::component::KernelComponent;
use crate::kernel::error::Error;
use crate::plugin_system::dependency::PluginDependency;
use crate::plugin_system::traits::{Plugin, PluginError as TraitsPluginError};
use crate::stage_manager::{StageContext};
use crate::stage_manager::manager::StageManager;
use crate::stage_manager::pipeline::PipelineBuilder;

use super::super::common::{setup_test_environment, DependentPlugin, ShutdownBehavior, PreflightBehavior};

#[tokio::test]
async fn test_lifecycle_management() {
    // Correctly destructure all 6 return values
    let (plugin_manager, stage_manager, _, stages_executed, execution_order, shutdown_order) = setup_test_environment().await;

    // Initialize components (represents system startup)
    KernelComponent::initialize(&*stage_manager).await.expect("Failed to initialize stage manager");
    KernelComponent::initialize(&*plugin_manager).await.expect("Failed to initialize plugin manager");

    // Track initialization steps (use std::sync::Mutex lock)
    execution_order.lock().unwrap().push("system_initialized".to_string());

    // Create plugins with dependencies to enforce a specific loading order
    let plugin_a = DependentPlugin::new(
        "LifecyclePluginA",
        "1.0.0",
        vec![],
        ShutdownBehavior::Success,
        PreflightBehavior::Success,
        stages_executed.clone(),
        execution_order.clone(), // Pass tracker
        shutdown_order.clone() // Pass shutdown tracker
    );

    let plugin_b = DependentPlugin::new(
        "LifecyclePluginB",
        "1.0.0",
        vec![PluginDependency::required("LifecyclePluginA", ">=1.0.0".parse().unwrap())],
        ShutdownBehavior::Success,
        PreflightBehavior::Success,
        stages_executed.clone(),
        execution_order.clone(), // Pass tracker
        shutdown_order.clone() // Pass shutdown tracker
    );

    // Register plugins
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(plugin_a)).expect("Failed to register plugin A");
        registry.register_plugin(Box::new(plugin_b)).expect("Failed to register plugin B");
        execution_order.lock().unwrap().push("plugins_registered".to_string());
    }

    // Initialize plugins
    {
        let registry = plugin_manager.registry();
        let mut reg = registry.lock().await;
        let mut app = Application::new(Some(std::env::temp_dir())).expect("Failed to create app");

        // Initialize plugins individually
        reg.initialize_plugin("LifecyclePluginA", &mut app).expect("Failed to initialize PluginA");
        reg.initialize_plugin("LifecyclePluginB", &mut app).expect("Failed to initialize PluginB");
        execution_order.lock().unwrap().push("plugins_initialized".to_string());
    }

    // Register plugin stages with stage manager
    {
        let registry = plugin_manager.registry();
        let plugins = {
            let reg = registry.lock().await;
            reg.get_plugins_arc()
        };

        for plugin in plugins {
            for stage in plugin.stages() {
                StageManager::register_stage(&*stage_manager, stage).await.expect("Failed to register plugin stage");
            }
        }
        execution_order.lock().unwrap().push("stages_registered".to_string());
    }

    // Create and execute a pipeline with plugin stages
    {
        let mut pipeline = PipelineBuilder::new("Lifecycle Pipeline", "Test system lifecycle")
            .add_stage("LifecyclePluginA_Stage")
            .add_stage("LifecyclePluginB_Stage")
            .add_dependency("LifecyclePluginB_Stage", "LifecyclePluginA_Stage") // B depends on A
            .build();

        let mut context = StageContext::new_live(std::env::temp_dir());
        StageManager::execute_pipeline(&*stage_manager, &mut pipeline, &mut context).await
            .expect("Failed to execute pipeline");

        execution_order.lock().unwrap().push("stages_executed".to_string());
    }

    // Shutdown components (represents system shutdown)
    {
        // Shutdown plugins in reverse order
        let mut registry = plugin_manager.registry().lock().await;
        registry.shutdown_all().expect("Failed to shutdown plugins");
        execution_order.lock().unwrap().push("plugins_shutdown".to_string());
    }

    // Verify the lifecycle order was correct (use std::sync::Mutex lock)
    let order = execution_order.lock().unwrap(); // No await needed

    // Check specific lifecycle phases
    let init_pos = order.iter().position(|s| s == "system_initialized").expect("Missing system_initialized");
    let reg_pos = order.iter().position(|s| s == "plugins_registered").expect("Missing plugins_registered");
    let plugins_init_pos = order.iter().position(|s| s == "plugins_initialized").expect("Missing plugins_initialized");
    let stages_reg_pos = order.iter().position(|s| s == "stages_registered").expect("Missing stages_registered");
    let stages_exec_pos = order.iter().position(|s| s == "stages_executed").expect("Missing stages_executed");
    let shutdown_pos = order.iter().position(|s| s == "plugins_shutdown").expect("Missing plugins_shutdown");

    // Verify correct order of lifecycle phases
    assert!(init_pos < reg_pos, "System should be initialized before plugins are registered");
    assert!(reg_pos < plugins_init_pos, "Plugins should be registered before initialized");
    assert!(plugins_init_pos < stages_reg_pos, "Plugins should be initialized before stages are registered");
    assert!(stages_reg_pos < stages_exec_pos, "Stages should be registered before executed");
    assert!(stages_exec_pos < shutdown_pos, "Stages should be executed before system shutdown");

    // Check that plugin stages were executed
    let executed = stages_executed.lock().await;
    assert!(executed.contains("LifecyclePluginA_Stage"), "Plugin A's stage should be executed");
    assert!(executed.contains("LifecyclePluginB_Stage"), "Plugin B's stage should be executed");
}

#[tokio::test]
async fn test_plugin_shutdown_order() {
    // Set up environment, getting all trackers
    let (plugin_manager, _stage_manager, _, stages_executed, execution_order, shutdown_order) = setup_test_environment().await;

    // Initialize components
    KernelComponent::initialize(&*plugin_manager).await.expect("Failed to initialize plugin manager");

    // Create plugins with a dependency: B -> A
    let plugin_a = DependentPlugin::new(
        "DepShutdownA", "1.0.0", vec![],
        ShutdownBehavior::Success, PreflightBehavior::Success,
        stages_executed.clone(), execution_order.clone(), shutdown_order.clone()
    );
    let plugin_b = DependentPlugin::new(
        "DepShutdownB", "1.0.0",
        vec![PluginDependency::required("DepShutdownA", ">=1.0.0".parse().unwrap())],
        ShutdownBehavior::Success, PreflightBehavior::Success,
        stages_executed.clone(), execution_order.clone(), shutdown_order.clone()
    );

    // Register plugins
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(plugin_a)).expect("Failed to register A");
        registry.register_plugin(Box::new(plugin_b)).expect("Failed to register B");
    }

    // Initialize plugins (initialize B, which should trigger A first)
    let mut app = Application::new(Some(std::env::temp_dir())).expect("Failed to create app for test");
    {
        let registry = plugin_manager.registry();
        let mut reg_lock = registry.lock().await;
        reg_lock.initialize_plugin("DepShutdownB", &mut app).expect("Failed to initialize DepShutdownB");
    }

    // Verify initialization order first (A then B) (use std::sync::Mutex lock)
    {
        let init_order = execution_order.lock().unwrap(); // No await needed
        assert_eq!(*init_order, vec!["DepShutdownA", "DepShutdownB"], "Initialization order mismatch");
    }
     // Verify both are marked as initialized
    {
         let registry = plugin_manager.registry().lock().await;
         assert!(registry.initialized.contains("DepShutdownA"), "A should be initialized");
         assert!(registry.initialized.contains("DepShutdownB"), "B should be initialized");
    }


    // Shutdown all plugins
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.shutdown_all().expect("Failed to shutdown plugins");
    }

    // Verify the shutdown order (should be reverse of initialization: B then A) (use std::sync::Mutex lock)
    {
        let shut_order = shutdown_order.lock().unwrap(); // No await needed
        assert_eq!(shut_order.len(), 2, "Expected 2 plugins to be shut down");
        // Note: registry.shutdown_all() sorts by priority reverse. If priorities are equal,
        // the order might depend on HashMap iteration order unless explicitly sorted by name/ID too.
        // Assuming the priority sort in shutdown_all handles dependencies correctly (reverse topological).
        assert_eq!(*shut_order, vec!["DepShutdownB", "DepShutdownA"], "Shutdown order mismatch");
    }

    // Verify plugins are no longer marked as initialized
    {
        let registry = plugin_manager.registry().lock().await;
        assert!(!registry.initialized.contains("DepShutdownA"), "A should no longer be initialized");
        assert!(!registry.initialized.contains("DepShutdownB"), "B should no longer be initialized");
    }
}

#[tokio::test]
async fn test_plugin_shutdown_error_handling() {
    // Set up environment
    let (plugin_manager, _stage_manager, _, stages_executed, execution_order, shutdown_order) = setup_test_environment().await;

    // Initialize components
    KernelComponent::initialize(&*plugin_manager).await.expect("Failed to initialize plugin manager");

    // Create plugins: one that fails shutdown, one that succeeds
    let error_plugin = DependentPlugin::new(
        "ShutdownErrorPlugin", "1.0.0", vec![],
        ShutdownBehavior::Failure, // Configure to fail shutdown
        PreflightBehavior::Success,
        stages_executed.clone(), execution_order.clone(), shutdown_order.clone()
    );
    let success_plugin = DependentPlugin::new(
        "ShutdownSuccessPlugin", "1.0.0", vec![],
        ShutdownBehavior::Success, // Configure to succeed shutdown
        PreflightBehavior::Success,
        stages_executed.clone(), execution_order.clone(), shutdown_order.clone()
    );
    let error_plugin_name = error_plugin.name().to_string();
    let success_plugin_name = success_plugin.name().to_string();


    // Register plugins
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(error_plugin)).expect("Failed to register error plugin");
        registry.register_plugin(Box::new(success_plugin)).expect("Failed to register success plugin");
    }

    // Initialize plugins
    let mut app = Application::new(Some(std::env::temp_dir())).expect("Failed to create app for test");
    {
        let registry = plugin_manager.registry();
        let mut reg_lock = registry.lock().await;
        // Initialize both (order doesn't strictly matter here as they have no deps on each other)
        reg_lock.initialize_plugin(&error_plugin_name, &mut app).expect("Failed to initialize error plugin");
        reg_lock.initialize_plugin(&success_plugin_name, &mut app).expect("Failed to initialize success plugin");
    }
     // Verify both are marked as initialized
    {
         let registry = plugin_manager.registry().lock().await;
         assert!(registry.initialized.contains(&error_plugin_name), "Error plugin should be initialized");
         assert!(registry.initialized.contains(&success_plugin_name), "Success plugin should be initialized");
    }

    // Shutdown all plugins - expect an error because one fails
    let shutdown_result = {
        let mut registry = plugin_manager.registry().lock().await;
        registry.shutdown_all()
    };

    // Verify that shutdown_all returned an error
    assert!(shutdown_result.is_err(), "shutdown_all should return an error when a plugin fails");

    // Verify the error message contains the failing plugin's info
    match shutdown_result.err().unwrap() {
        Error::Plugin(msg) => {
            eprintln!("Shutdown Error Message: {}", msg); // Debug print
            assert!(msg.contains("Encountered errors during plugin shutdown"), "Error message prefix mismatch");
            assert!(msg.contains("ShutdownErrorPlugin") && msg.contains("Simulated shutdown failure"), "Error message content mismatch");
        }
        e => panic!("Expected Error::Plugin for shutdown failure, but got {:?}", e),
    }

    // Verify that *both* plugins were attempted to be shut down (check the tracker)
    // The order might depend on internal sorting (e.g., by priority, then maybe name/hash) (use std::sync::Mutex lock)
    {
        let shut_order = shutdown_order.lock().unwrap(); // No await needed
        assert_eq!(shut_order.len(), 2, "Expected both plugins to be attempted for shutdown");
        assert!(shut_order.contains(&error_plugin_name), "Error plugin should be in shutdown order");
        assert!(shut_order.contains(&success_plugin_name), "Success plugin should be in shutdown order");
    }

    // Verify both plugins are marked as uninitialized, even though one failed shutdown
    {
        let registry = plugin_manager.registry().lock().await;
        assert!(!registry.initialized.contains(&error_plugin_name), "Error plugin should be uninitialized after shutdown attempt");
        assert!(!registry.initialized.contains(&success_plugin_name), "Success plugin should be uninitialized after shutdown");
    }
}

// --- Test: Plugin Preflight Check Failure Handling ---
#[tokio::test]
async fn test_plugin_preflight_check_failure_handling() {
    // Setup environment
    let (plugin_manager, _stage_manager, _, stages_executed, execution_order, shutdown_order) = setup_test_environment().await;
    KernelComponent::initialize(&*plugin_manager).await.expect("Failed to initialize plugin manager");

    // Create a plugin configured to fail its preflight check
    let failing_plugin = DependentPlugin::new(
        "PreflightFailPlugin",
        "1.0.0",
        vec![],
        ShutdownBehavior::Success,
        PreflightBehavior::Failure, // Configure to fail
        stages_executed.clone(),
        execution_order.clone(),
        shutdown_order.clone()
    );
    let plugin_name = failing_plugin.name().to_string(); // Get name before moving

    // Register the plugin
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(failing_plugin)).expect("Failed to register failing plugin");
    }

    // Attempt to run the preflight check directly on the plugin instance for this test.
    // In a real scenario, this would likely be orchestrated by the PluginManager or Kernel during initialization or a specific stage.
    let preflight_result = {
         let registry = plugin_manager.registry();
         let reg_lock = registry.lock().await;
         if let Some(plugin_arc) = reg_lock.get_plugin(&plugin_name) {
             // Need a StageContext for the check method
             let context = StageContext::new_dry_run(std::env::temp_dir()); // Use dry run context
             plugin_arc.preflight_check(&context).await // Call the method directly
         } else {
             panic!("Plugin {} not found after registration", plugin_name);
         }
    };

     // Verify the preflight check itself failed with the correct error type from traits::PluginError
     assert!(preflight_result.is_err(), "Preflight check should have failed");
     match preflight_result.err().unwrap() {
         TraitsPluginError::PreflightCheckError(msg) => { // Use aliased error type
             assert!(msg.contains("Simulated preflight check failure"), "Error message mismatch: {}", msg);
         }
         e => panic!("Expected PreflightCheckError, got {:?}", e),
     }

    // Verify the plugin is not marked as initialized (assuming preflight is checked before/during init)
    {
        let registry = plugin_manager.registry().lock().await;
        // The `initialized` field might not be the best check if init wasn't called.
        // Instead, check if the plugin is still present and enabled.
        assert!(registry.get_plugin(&plugin_name).is_some(), "Plugin should still be registered");
        assert!(registry.is_enabled(&plugin_name), "Plugin should still be enabled even if preflight failed");
        // If initialization *was* attempted and failed due to preflight, then check initialized:
        // assert!(!registry.initialized.contains(&plugin_name), "Plugin should not be marked as initialized after failed preflight");
    }
}