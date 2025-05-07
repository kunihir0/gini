#![cfg(test)]

// use crate::stage_manager::Stage; // Import the Stage trait
use crate::kernel::component::KernelComponent;
use crate::plugin_system::traits::Plugin;
use crate::stage_manager::{StageContext, StageResult};
use crate::stage_manager::manager::StageManager;
use crate::stage_manager::pipeline::PipelineBuilder;

// Destructure all trackers from setup
// Destructure all trackers from setup
use super::super::common::{setup_test_environment, TestPlugin};

#[tokio::test]
async fn test_plugin_loading_and_stage_execution() {
    // Destructure all trackers
    let (plugin_manager, stage_manager, _storage_manager, stages_executed, execution_order, _shutdown_order) = setup_test_environment().await;

    // Initialize components manually (they'd normally be initialized by the kernel)
    KernelComponent::initialize(&*stage_manager).await.expect("Failed to initialize stage manager");
    KernelComponent::initialize(&*plugin_manager).await.expect("Failed to initialize plugin manager");

    // Register test plugins, passing execution_order
    let plugin1 = TestPlugin::new("Plugin1", stages_executed.clone(), execution_order.clone());
    let plugin2 = TestPlugin::new("Plugin2", stages_executed.clone(), execution_order.clone());

    // Test plugin versions - this will call the version() method to address code coverage
    assert_eq!(plugin1.version(), "1.0.0", "TestPlugin should have version 1.0.0");
    assert_eq!(plugin2.version(), "1.0.0", "TestPlugin should have version 1.0.0");

    // Get the registry from the plugin manager
    let plugin_registry_arc = plugin_manager.registry();
    {
        let mut registry_lock = plugin_registry_arc.lock().await;
        registry_lock.register_plugin(Box::new(plugin1)).expect("Failed to register Plugin1");
        registry_lock.register_plugin(Box::new(plugin2)).expect("Failed to register Plugin2");
    }

    // Manually initialize plugins to register their stages
    let mut app_for_init = crate::kernel::bootstrap::Application::new().expect("Failed to create app for init");
    let stage_registry_arc_for_init = stage_manager.registry().registry.clone(); // Get the Arc<Mutex<StageRegistry>>
    {
        let mut plugin_registry_locked = plugin_registry_arc.lock().await;
        plugin_registry_locked.initialize_all(&mut app_for_init, &stage_registry_arc_for_init).await
            .expect("initialize_all failed for plugin loading and stage execution test");
    } // plugin_registry_locked is dropped here


    // Create a test context
    let mut context = StageContext::new_live(std::env::temp_dir());

    // Create a pipeline to execute the plugin-provided stages
    let plugin_stages = vec!["Plugin1_Stage".to_string(), "Plugin2_Stage".to_string()];
    let mut pipeline = StageManager::create_pipeline(
        &*stage_manager,
        "Test Pipeline",
        "Pipeline for plugin stages",
        plugin_stages
    ).await.expect("Failed to create pipeline");

    // Execute the pipeline
    let results = StageManager::execute_pipeline(&*stage_manager, &mut pipeline, &mut context).await
        .expect("Failed to execute pipeline");

    // Verify all stages were executed successfully
    for (stage_id, result) in &results {
        match result {
            StageResult::Success => {},
            _ => panic!("Stage {} did not succeed: {:?}", stage_id, result),
        }
    }

    // Check that our tracker contains the expected stage IDs
    let executed = stages_executed.lock().await;
    assert!(executed.contains("Plugin1_Stage"), "Plugin1_Stage was not executed");
    assert!(executed.contains("Plugin2_Stage"), "Plugin2_Stage was not executed");
}

#[tokio::test]
async fn test_plugin_stage_registration() {
    // Destructure all trackers
    let (plugin_manager, stage_manager, _storage_manager, stages_executed, execution_order, _shutdown_order) = setup_test_environment().await;

    // Initialize components
    KernelComponent::initialize(&*stage_manager).await.expect("Failed to initialize stage manager");
    KernelComponent::initialize(&*plugin_manager).await.expect("Failed to initialize plugin manager");

    // Create a plugin that provides stages, passing execution_order
    let plugin = TestPlugin::new("StageRegistrationPlugin", stages_executed.clone(), execution_order.clone());

    // Register the plugin
    {
        let mut registry_lock = plugin_manager.registry().lock().await;
        registry_lock.register_plugin(Box::new(plugin)).expect("Failed to register plugin");
    }

    // Track the number of stages before plugin initialization (core stages should be present)
    let stages_before = stage_manager.get_stage_ids().await.expect("Failed to get stage IDs").len();
    assert_eq!(stages_before, 3, "Expected 3 core stages before plugin init"); // core::plugin_preflight_check, core::plugin_initialization, core::plugin_post_initialization

    // Directly call initialize_all on the plugin registry
    let mut plugin_registry_locked = plugin_manager.registry().lock().await;
    let stage_registry_arc_for_init = stage_manager.registry().registry.clone(); // Get the Arc<Mutex<StageRegistry>>
    let mut app_for_init = crate::kernel::bootstrap::Application::new().expect("Failed to create app for init");

    // Manually run preflight logic (simplified for this test)
    // In a real scenario, PluginPreflightCheckStage would populate preflight_failures_key in context.
    // For this test, we assume TestPlugin passes preflight.
    // If preflight checks were more complex or essential here, we'd run that stage first.

    plugin_registry_locked.initialize_all(&mut app_for_init, &stage_registry_arc_for_init).await
        .expect("initialize_all failed");
    
    drop(plugin_registry_locked); // Release lock

    // Verify the plugin's stage was registered with the stage manager
    let stages_after = stage_manager.get_stage_ids().await.expect("Failed to get stage IDs");
    assert_eq!(stages_after.len(), stages_before + 1, "Plugin stage should be registered. Stages found: {:?}", stages_after);

    // Check that the specific stage exists
    let expected_stage_id = "StageRegistrationPlugin_Stage";
    let stage_exists = stages_after.iter().any(|id| id == expected_stage_id);
    assert!(stage_exists, "Plugin's stage '{}' should be registered with the stage manager. Stages found: {:?}", expected_stage_id, stages_after);
}

#[tokio::test]
async fn test_plugin_stage_execution() {
    // Destructure all trackers
    let (plugin_manager, stage_manager, _storage_manager, stages_executed, execution_order, _shutdown_order) = setup_test_environment().await;

    // Initialize components
    KernelComponent::initialize(&*stage_manager).await.expect("Failed to initialize stage manager");
    KernelComponent::initialize(&*plugin_manager).await.expect("Failed to initialize plugin manager");

    // Create plugins that provide stages, passing execution_order
    let plugin_a = TestPlugin::new("StageExecPluginA", stages_executed.clone(), execution_order.clone());
    let plugin_b = TestPlugin::new("StageExecPluginB", stages_executed.clone(), execution_order.clone());

    // Register the plugins
    let plugin_registry_arc = plugin_manager.registry();
    {
        let mut registry_lock = plugin_registry_arc.lock().await;
        registry_lock.register_plugin(Box::new(plugin_a)).expect("Failed to register plugin A");
        registry_lock.register_plugin(Box::new(plugin_b)).expect("Failed to register plugin B");
    }

    // Manually initialize plugins to register their stages
    let mut app_for_init = crate::kernel::bootstrap::Application::new().expect("Failed to create app for init");
    let stage_registry_arc_for_init = stage_manager.registry().registry.clone(); // Get the Arc<Mutex<StageRegistry>>
    {
        let mut plugin_registry_locked = plugin_registry_arc.lock().await;
        plugin_registry_locked.initialize_all(&mut app_for_init, &stage_registry_arc_for_init).await
            .expect("initialize_all failed for plugin stage execution test");
    } // plugin_registry_locked is dropped here


    // Create a pipeline using the plugin-provided stages
    let mut pipeline = PipelineBuilder::new("Plugin Stage Execution", "Test executing plugin stages")
        .add_stage("StageExecPluginA_Stage")
        .add_stage("StageExecPluginB_Stage")
        .build();

    // Execute the pipeline
    let mut context = StageContext::new_live(std::env::temp_dir());
    let results = StageManager::execute_pipeline(&*stage_manager, &mut pipeline, &mut context).await
        .expect("Failed to execute pipeline");

    // Verify that both plugin stages executed successfully
    for (stage_id, result) in &results {
        assert!(matches!(result, StageResult::Success),
            "Stage {} should succeed", stage_id);
    }

    // Check that the tracker contains the expected stages
    let executed = stages_executed.lock().await;
    assert!(executed.contains("StageExecPluginA_Stage"), "Plugin A's stage should be executed");
    assert!(executed.contains("StageExecPluginB_Stage"), "Plugin B's stage should be executed");
}

#[tokio::test]
async fn test_plugin_system_stage_manager_integration() {
    // Destructure all trackers
    let (plugin_manager, stage_manager, _storage_manager, stages_executed, execution_order, _shutdown_order) = setup_test_environment().await;

    // Initialize components
    KernelComponent::initialize(&*stage_manager).await.expect("Failed to initialize stage manager");
    KernelComponent::initialize(&*plugin_manager).await.expect("Failed to initialize plugin manager");

    // Create a test plugin that will register its own stages, passing execution_order
    let test_plugin = TestPlugin::new("IntegrationPlugin", stages_executed.clone(), execution_order.clone());

    // Register the plugin
    let plugin_registry_arc = plugin_manager.registry();
    {
        let mut registry_lock = plugin_registry_arc.lock().await;
        registry_lock.register_plugin(Box::new(test_plugin)).expect("Failed to register test plugin");
    }

    // Manually initialize plugins to register their stages
    let mut app_for_init = crate::kernel::bootstrap::Application::new().expect("Failed to create app for init");
    let stage_registry_arc_for_init = stage_manager.registry().registry.clone(); // Get the Arc<Mutex<StageRegistry>>
    {
        let mut plugin_registry_locked = plugin_registry_arc.lock().await;
        plugin_registry_locked.initialize_all(&mut app_for_init, &stage_registry_arc_for_init).await
            .expect("initialize_all failed for plugin system stage manager integration test");
    } // plugin_registry_locked is dropped here


    // Create a pipeline with the plugin's stage
    let mut pipeline = StageManager::create_pipeline(
        &*stage_manager,
        "Integration Pipeline",
        "Test plugin and stage manager integration",
        vec!["IntegrationPlugin_Stage".to_string()]
    ).await.expect("Failed to create pipeline");

    // Create a context for execution
    let mut context = StageContext::new_live(std::env::temp_dir());

    // Execute the pipeline
    let results = StageManager::execute_pipeline(&*stage_manager, &mut pipeline, &mut context).await
        .expect("Failed to execute pipeline");

    // Verify the stage executed successfully
    let stage_result = results.get("IntegrationPlugin_Stage").expect("Missing result for plugin stage");
    match stage_result {
        StageResult::Success => {},
        _ => panic!("Plugin stage execution failed: {:?}", stage_result),
    }

    // Check that our tracker contains the expected stage ID
    let executed = stages_executed.lock().await;
    assert!(executed.contains("IntegrationPlugin_Stage"), "Plugin stage was not executed");
}