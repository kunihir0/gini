#![cfg(test)]

use std::collections::HashSet;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;
use async_trait::async_trait;

use crate::kernel::bootstrap::Application;
use crate::kernel::component::KernelComponent;
use crate::kernel::error::{Error, Result as KernelResult};
use crate::plugin_system::dependency::PluginDependency;
use crate::plugin_system::traits::{Plugin, PluginPriority, PluginError as TraitsPluginError};
use crate::plugin_system::version::VersionRange;
use crate::stage_manager::{Stage, StageContext, StageResult};
use crate::stage_manager::requirement::StageRequirement;
use crate::storage::manager::DefaultStorageManager;
use crate::plugin_system::manager::DefaultPluginManager;
use std::path::PathBuf;

use super::super::common::{setup_test_environment, TestPlugin, DependentPlugin, ShutdownBehavior, PreflightBehavior};

#[tokio::test]
async fn test_plugin_enabling_disabling() {
    // Destructure all trackers
    let (plugin_manager, _stage_manager, _, stages_executed, execution_order, _shutdown_order) = setup_test_environment().await;

    // Initialize components
    KernelComponent::initialize(&*plugin_manager).await.expect("Failed to initialize plugin manager");

    // Create test plugins, passing execution_order
    let plugin1 = TestPlugin::new("EnableDisablePlugin1", stages_executed.clone(), execution_order.clone());
    let plugin2 = TestPlugin::new("EnableDisablePlugin2", stages_executed.clone(), execution_order.clone());
    let plugin3 = TestPlugin::new("EnableDisablePlugin3", stages_executed.clone(), execution_order.clone());

    // Register plugins
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(plugin1)).expect("Failed to register Plugin1");
        registry.register_plugin(Box::new(plugin2)).expect("Failed to register Plugin2");
        registry.register_plugin(Box::new(plugin3)).expect("Failed to register Plugin3");
    }

    // Disable Plugin2
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.disable_plugin("EnableDisablePlugin2").expect("Failed to disable Plugin2");
    }

    // Verify Plugin2 is disabled
    {
        let registry = plugin_manager.registry().lock().await;
        let enabled_plugins = registry.get_enabled_plugins_arc();
        
        // Check specific test plugins by name
        assert!(enabled_plugins.iter().any(|p| p.name() == "EnableDisablePlugin1"), "Plugin1 should be enabled");
        assert!(enabled_plugins.iter().any(|p| p.name() == "EnableDisablePlugin3"), "Plugin3 should be enabled");
        assert!(!enabled_plugins.iter().any(|p| p.name() == "EnableDisablePlugin2"), "Plugin2 should not be enabled");

        // Also check is_enabled
        assert!(registry.is_enabled("EnableDisablePlugin1"), "Plugin1 should be reported as enabled");
        assert!(!registry.is_enabled("EnableDisablePlugin2"), "Plugin2 should be reported as disabled");
        assert!(registry.is_enabled("EnableDisablePlugin3"), "Plugin3 should be reported as enabled");
        assert!(!registry.is_enabled("NonExistentPlugin"), "NonExistentPlugin should not be reported as enabled");
    }

    // Re-enable Plugin2
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.enable_plugin("EnableDisablePlugin2").expect("Failed to enable Plugin2");
    }

    // Verify all plugins are enabled again
    {
        let registry = plugin_manager.registry().lock().await;
        let enabled_plugins = registry.get_enabled_plugins_arc();
        
        // Check that all test plugins are now enabled
        assert!(enabled_plugins.iter().any(|p| p.name() == "EnableDisablePlugin1"), "Plugin1 should be enabled");
        assert!(enabled_plugins.iter().any(|p| p.name() == "EnableDisablePlugin2"), "Plugin2 should be enabled again");
        assert!(enabled_plugins.iter().any(|p| p.name() == "EnableDisablePlugin3"), "Plugin3 should be enabled");

        assert!(registry.is_enabled("EnableDisablePlugin2"), "Plugin2 should be reported as enabled again");
    }

    // Attempt to disable a non-existent plugin
    {
        let mut registry = plugin_manager.registry().lock().await;
        let result = registry.disable_plugin("NonExistentPlugin");
        // According to registry.rs implementation, disabling non-existent is a no-op Ok(())
        assert!(result.is_ok(), "Disabling a non-existent plugin should be Ok(())");
    }

    // Attempt to enable a non-existent plugin
    {
        let mut registry = plugin_manager.registry().lock().await;
        let result = registry.enable_plugin("NonExistentPlugin");
        assert!(result.is_err(), "Enabling a non-existent plugin should return an error");
        // Optionally check the error type/message if needed
        match result.err().unwrap() {
            Error::Plugin(msg) => assert!(msg.contains("Cannot enable non-existent plugin")),
            e => panic!("Expected Error::Plugin, got {:?}", e),
        }
    }
}

// Define two conflicting plugins (e.g., same name or provide same resource - using name here for simplicity)
struct ConflictingPlugin { name_val: String }
impl ConflictingPlugin { fn new(name: &str) -> Self { Self { name_val: name.to_string() } } }
#[async_trait]
impl Plugin for ConflictingPlugin {
    fn name(&self) -> &'static str { Box::leak(self.name_val.clone().into_boxed_str()) } // Use the same name
    fn version(&self) -> &str { "1.0.0" }
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(100) }
    fn compatible_api_versions(&self) -> Vec<VersionRange> { vec![">=0.1.0".parse().unwrap()] }
    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }
    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }
    fn init(&self, _app: &mut Application) -> KernelResult<()> { Ok(()) }
    async fn preflight_check(&self, _context: &StageContext) -> Result<(), TraitsPluginError> { Ok(()) }
    fn stages(&self) -> Vec<Box<dyn Stage>> { vec![] }
    fn shutdown(&self) -> KernelResult<()> { Ok(()) }
// Add default implementations for new trait methods
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
}

#[tokio::test]
async fn test_plugin_conflict_detection_and_resolution() {
    // Setup environment
    let (plugin_manager, _, _, _, _, _) = setup_test_environment().await;
    KernelComponent::initialize(&*plugin_manager).await.expect("Failed to initialize plugin manager");

    // Get initial plugin count
    let initial_count = {
        let registry = plugin_manager.registry().lock().await;
        registry.plugin_count()
    };

    let plugin1 = ConflictingPlugin::new("ConflictPlugin");
    let plugin2 = ConflictingPlugin::new("ConflictPlugin"); // Same name causes conflict

    // Register the first plugin - should succeed
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(plugin1)).expect("Failed to register first conflicting plugin");
    }

    // Attempt to register the second plugin - should fail due to name conflict
    let register_result = {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(plugin2)) // Register the second one
    };

    assert!(register_result.is_err(), "Registering a plugin with a conflicting name should fail");

    // Verify the error type and message
    match register_result.err().unwrap() {
        Error::Plugin(msg) => {
            assert!(msg.contains("Plugin already registered: ConflictPlugin"), "Expected name conflict error, got: {}", msg); // Match actual error
        }
        e => panic!("Expected Error::Plugin for conflict, got {:?}", e),
    }

    // Verify only the first plugin is actually registered
    {
        let registry = plugin_manager.registry().lock().await;
        assert_eq!(registry.plugin_count(), initial_count + 1, "Only one conflict plugin should be registered");
        assert!(registry.get_plugin("ConflictPlugin").is_some(), "The first plugin should be present");
    }

    // Note: This test assumes conflict detection happens at registration time based on name.
    // If conflict detection (e.g., based on provided resources) happens later (e.g., during dependency check or a specific stage),
    // the test would need to be adjusted to trigger that phase.
}

#[tokio::test]
async fn test_plugin_get_plugin_ids() {
    // Destructure all trackers
    let (plugin_manager, _, _, stages_executed, execution_order, _) = setup_test_environment().await;
    KernelComponent::initialize(&*plugin_manager).await.expect("Init PluginManager");

    // Get the initial number of plugins (if any)
    let initial_plugin_count = {
        let registry = plugin_manager.registry().lock().await;
        registry.plugin_count()
    };

    let plugin1 = TestPlugin::new("GetIdsPlugin1", stages_executed.clone(), execution_order.clone());
    let plugin2 = TestPlugin::new("GetIdsPlugin2", stages_executed.clone(), execution_order.clone());

    // Register plugins
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(plugin1)).expect("Register 1");
        registry.register_plugin(Box::new(plugin2)).expect("Register 2");
    }

    // Get IDs
    let ids = {
        let registry = plugin_manager.registry().lock().await;
        registry.get_plugin_ids() // Call the method
    };

    // Check if our specifically added plugins are present
    assert_eq!(ids.len(), initial_plugin_count + 2, "Should have added 2 plugin IDs");
    assert!(ids.contains(&"GetIdsPlugin1".to_string()), "GetIdsPlugin1 should be in the IDs");
    assert!(ids.contains(&"GetIdsPlugin2".to_string()), "GetIdsPlugin2 should be in the IDs");
}