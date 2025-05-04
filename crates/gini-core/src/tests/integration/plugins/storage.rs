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
use crate::storage::provider::StorageProvider;
use crate::plugin_system::manager::DefaultPluginManager;
use std::path::PathBuf;

use super::super::common::{setup_test_environment, TestPlugin, DependentPlugin, ShutdownBehavior, PreflightBehavior};

// Define a plugin that interacts with storage
struct StorageInteractingPlugin {
    name: String,
    storage: Arc<DefaultStorageManager>, // Store the storage manager Arc
}
impl StorageInteractingPlugin {
    // Modify constructor to accept storage manager
    fn new(name: &str, storage_manager: Arc<DefaultStorageManager>) -> Self { // Imports added at top
        Self { name: name.to_string(), storage: storage_manager }
    }
}
#[async_trait]
impl Plugin for StorageInteractingPlugin {
    fn name(&self) -> &'static str { Box::leak(self.name.clone().into_boxed_str()) }
    fn version(&self) -> &str { "1.0.0" }
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(100) }
    fn compatible_api_versions(&self) -> Vec<VersionRange> { vec![">=0.1.0".parse().unwrap()] }
    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }
    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }

    // init no longer needs to get the storage manager from app
    fn init(&self, app: &mut Application) -> KernelResult<()> { // Use app parameter
        println!("Plugin {} init: Interacting with storage", self.name());
        // Use the stored storage manager Arc directly
        let storage = &self.storage;
        // Write relative to the application's config directory
        let config_dir = app.config_dir(); // Get config dir from app
        let test_path = config_dir.join("plugin_storage_test.txt"); // Write inside config dir
        let content = format!("Data written by {}", self.name());

        // Write data using the StorageProvider trait methods (trait import added at top)
        storage.write_string(&test_path, &content)?;
        println!("Plugin {} wrote to {:?}", self.name(), test_path);

        // Read data back
        let read_content = storage.read_to_string(&test_path)?;
        println!("Plugin {} read back: {}", self.name(), read_content);

        // Verify content
        assert_eq!(read_content, content, "Read content should match written content");

        // Clean up
        storage.remove_file(&test_path)?;
        println!("Plugin {} cleaned up {:?}", self.name(), test_path);

        Ok(())
    }

    async fn preflight_check(&self, _context: &StageContext) -> Result<(), TraitsPluginError> { Ok(()) }
    fn stages(&self) -> Vec<Box<dyn Stage>> { vec![] }
    fn shutdown(&self) -> KernelResult<()> { Ok(()) } // Imports added at top
// Add default implementations for new trait methods
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
}

#[tokio::test]
async fn test_plugin_interaction_with_storage() {
    // Setup environment - we need the real storage manager from setup
    let (plugin_manager, _stage_manager, storage_manager, _, _, _) = setup_test_environment().await;
    KernelComponent::initialize(&*plugin_manager).await.expect("Failed to initialize plugin manager");
    // Ensure the storage directory exists (use the path configured in Application::new)
    // We need the path that the Application instance *inside* the test will use.
    // Let's assume Application::new uses a predictable path or we configure it.
    // Re-creating app with a known path for the test:
    let test_base_path = std::env::temp_dir().join("gini_test_plugin_storage");
     if !test_base_path.exists() {
        std::fs::create_dir_all(&test_base_path).expect("Failed to create base storage directory for test");
     }


     // Create the plugin instance, passing the storage manager Arc
     let plugin = StorageInteractingPlugin::new("StoragePlugin", storage_manager.clone()); // Pass storage_manager
     let plugin_name = plugin.name().to_string();

     // Register the plugin
    {
        let mut registry = plugin_manager.registry().lock().await;
        registry.register_plugin(Box::new(plugin)).expect("Failed to register storage plugin");
    }

    // Create a mock Application instance containing the necessary managers
    // Note: Application::new might require more setup depending on its implementation.
    // We pass `None` for config dir assuming the plugin doesn't need it directly.
    let mut app = Application::new(None).expect("Failed to create mock Application");
    // Manually add the storage manager instance we got from setup_test_environment
    // This depends on Application having a way to set/replace managers, which might not be standard.
    // If Application::new sets up its own managers, we need to ensure the test uses the correct one.
    // Let's assume Application::new creates its own, and the plugin will access *that* one.
    // We need to ensure the path used by the Application's storage manager is known or controllable.
    // Re-creating app with the known storage path for this test specifically:
    let mut app = Application::new(Some(test_base_path.clone())).expect("Failed to create Application with storage path");


    // Initialize the plugin - this will trigger the storage interaction in its init method
    let init_result = {
        let registry = plugin_manager.registry();
        let mut reg_lock = registry.lock().await;
        reg_lock.initialize_plugin(&plugin_name, &mut app)
    };

    // Verify initialization (and thus storage interaction) succeeded
    assert!(init_result.is_ok(), "Plugin initialization (with storage interaction) failed: {:?}", init_result.err());

    // The assertions are inside the plugin's init method in this setup.
    // We could also have the plugin set a flag in shared context or return data
    // to verify interaction from the test function itself.
}