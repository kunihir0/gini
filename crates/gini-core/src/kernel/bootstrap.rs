use std::any::TypeId; // Remove braces
// Removed unused std::path::PathBuf
// Removed unused std::env
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::kernel::error::{Error, Result};
use crate::kernel::constants;
use crate::kernel::component::{KernelComponent, DependencyRegistry};

// Import component traits and default implementations
use crate::event::DefaultEventManager; // Remove braces
use crate::stage_manager::manager::DefaultStageManager; // Remove braces
use crate::plugin_system::DefaultPluginManager; // Remove braces
use crate::storage::DefaultStorageManager; // Remove braces
use crate::ui_bridge::UIManager; // Added UIManager import

/// Main application struct coordinating components via dependency injection
pub struct Application {
    // base_path: PathBuf, // Removed: Path logic now handled by StorageManager
    // config_dir: PathBuf, // Removed: Path logic now handled by StorageManager
    initialized: bool,
    // Simplified Dependency registry
    dependencies: Arc<Mutex<DependencyRegistry>>,
    // Keep track of component initialization order (using concrete TypeIds)
    component_init_order: Vec<TypeId>,
    // UI Manager to handle UI connections
    ui_manager: UIManager, // Added ui_manager field
}

// Updated impl Application block using simplified DependencyRegistry
impl Application {
    /// Creates a new application instance with default components using XDG paths.
    pub fn new() -> Result<Self> { // Removed base_path_override
        println!("Initializing {} v{}", constants::APP_NAME, constants::APP_VERSION);

        // Base path and config dir logic removed - handled by StorageManager::new()

        let mut registry = DependencyRegistry::new();
        let mut init_order = Vec::new();

        // Register default components using their concrete types
        // Instantiate StorageManager using the new XDG-aware constructor
        let storage_manager = Arc::new(DefaultStorageManager::new()?); // Call new() which returns Result
        println!("Using config directory: {}", storage_manager.config_dir().display());
        println!("Using data directory: {}", storage_manager.data_dir().display());
        registry.register_instance(storage_manager.clone()); // Register Arc<DefaultStorageManager>, clone Arc
        init_order.push(TypeId::of::<DefaultStorageManager>()); // Store concrete TypeId

        let event_manager = Arc::new(DefaultEventManager::new());
        registry.register_instance(event_manager.clone()); // Register Arc<DefaultEventManager>, clone Arc
        init_order.push(TypeId::of::<DefaultEventManager>()); // Store concrete TypeId

        // Get the ConfigManager from the StorageManager to pass to PluginManager
        // Use the renamed public accessor method to get the ConfigManager Arc
        let config_manager_for_plugin = storage_manager.get_config_manager().clone();
        let plugin_manager = Arc::new(DefaultPluginManager::new(config_manager_for_plugin)?);
        registry.register_instance(plugin_manager.clone()); // Register Arc<DefaultPluginManager<LocalStorageProvider>>, clone Arc
        // Use the non-generic type for TypeId
        init_order.push(TypeId::of::<DefaultPluginManager>()); // Remove generic

        let stage_manager = Arc::new(DefaultStageManager::new());
        registry.register_instance(stage_manager.clone()); // Register Arc<DefaultStageManager>, clone Arc
        init_order.push(TypeId::of::<DefaultStageManager>()); // Store concrete TypeId

        // Instantiate UIManager
        let ui_manager = UIManager::new();

        Ok(Application {
            // base_path, // Removed
            // config_dir, // Removed
            initialized: false,
            dependencies: Arc::new(Mutex::new(registry)),
            component_init_order: init_order,
            ui_manager, // Assign ui_manager instance
        })
    }

    /// Gets a specific component instance by its concrete type T.
    /// Returns Option<Arc<T>>.
    pub async fn get_component<T: KernelComponent + 'static>(&self) -> Option<Arc<T>> {
         let registry = self.dependencies.lock().await;
         registry.get_concrete::<T>()
    }

    // Removed register_component for now

    /// Runs the application initialization and main loop (placeholder).
    pub async fn run(&mut self) -> Result<()> {
        if self.initialized {
            return Err(Error::Init("Application already running".to_string()));
        }

        self.initialize().await?;
        self.start().await?;

        self.initialized = true;
        println!("Application initialized and started successfully.");
        // Print XDG paths from StorageManager
        if let Some(sm) = self.get_component::<DefaultStorageManager>().await {
             println!("Config directory: {}", sm.config_dir().display());
             println!("Data directory: {}", sm.data_dir().display());
        } else {
             println!("Warning: Could not retrieve StorageManager to display paths.");
        }


        println!("Application running... (Simulating work)");
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        self.shutdown().await?;
        Ok(())
    }

    /// Initialize all registered components in the predefined order.
    async fn initialize(&mut self) -> Result<()> {
        println!("Initializing components...");
        let registry = self.dependencies.lock().await; // Lock registry

        for type_id in &self.component_init_order {
            // Use get_component_by_id which returns Option<Arc<dyn KernelComponent>>
            if let Some(component_arc) = registry.get_component_by_id(type_id) {
                 println!("Initializing component: {}", component_arc.name());
                 component_arc.initialize().await?; // Call method on Arc<dyn KernelComponent>
            } else {
                 // This indicates a logic error
                 eprintln!("Error: Component instance not found in registry for TypeId {:?} during initialization.", type_id);
                 return Err(Error::Component(format!("Initialization failed: Instance missing for component {:?}", type_id)));
            }
        }
        println!("Component initialization complete.");
        Ok(())
    }

     /// Start all initialized components in the predefined order.
    async fn start(&mut self) -> Result<()> {
        println!("Starting components...");
        let registry = self.dependencies.lock().await; // Lock registry

        for type_id in &self.component_init_order {
             // Use get_component_by_id
            if let Some(component_arc) = registry.get_component_by_id(type_id) {
                 println!("Starting component: {}", component_arc.name());
                 component_arc.start().await?; // Call method on Arc<dyn KernelComponent>
            } else {
                 eprintln!("Error: Component instance not found in registry for TypeId {:?} during start.", type_id);
                 return Err(Error::Component(format!("Start failed: Instance missing for component {:?}", type_id)));
            }
        }
        println!("Component start complete.");
        Ok(())
    }

    /// Shutdown all components in reverse order of initialization.
    async fn shutdown(&mut self) -> Result<()> {
        println!("Shutting down components...");
        let registry = self.dependencies.lock().await; // Lock registry

        // Shutdown in reverse initialization order
        for type_id in self.component_init_order.iter().rev() {
             // Use get_component_by_id
             if let Some(component_arc) = registry.get_component_by_id(type_id) {
                 println!("Stopping component: {}", component_arc.name());
                 if let Err(e) = component_arc.stop().await { // Call method on Arc<dyn KernelComponent>
                     eprintln!("Error stopping component {}: {}", component_arc.name(), e);
                 }
             } else {
                 eprintln!("Warning: Component instance not found in registry for TypeId {:?} during stop.", type_id);
             }
         }
        self.initialized = false; // Mark as not running
        println!("Component shutdown complete.");
        Ok(())
    }

    // Removed base_path() method
    // Removed config_dir() method

    /// Returns whether the application has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    /// Get the storage manager instance (synchronous convenience accessor)
    pub fn storage_manager(&self) -> Arc<DefaultStorageManager> {
        // For testing purposes, this simplified approach is sufficient
        // In a real implementation, we should handle the potential absence gracefully
        self.dependencies.try_lock()
            .ok()
            .and_then(|reg| reg.get_concrete::<DefaultStorageManager>())
            .expect("Storage manager not found in registry")
    }

    /// Get the plugin manager instance (synchronous convenience accessor)
    /// Note: Uses try_lock for sync access, similar to storage_manager. May need async if required elsewhere.
    pub fn plugin_manager(&self) -> Arc<DefaultPluginManager> {
        // Similar simplified approach as storage_manager
        self.dependencies.try_lock()
            .ok()
            .and_then(|reg| reg.get_concrete::<DefaultPluginManager>())
            .expect("Plugin manager not found in registry")
    }

    /// Get the stage manager instance (synchronous convenience accessor)
    /// Note: Uses try_lock for sync access.
    pub fn stage_manager(&self) -> Arc<DefaultStageManager> {
        // Similar simplified approach as other managers
        self.dependencies.try_lock()
            .ok()
            .and_then(|reg| reg.get_concrete::<DefaultStageManager>())
            .expect("Stage manager not found in registry")
    }

    /// Returns a mutable reference to the UI manager.
    pub fn ui_manager_mut(&mut self) -> &mut UIManager {
        &mut self.ui_manager
    }
}