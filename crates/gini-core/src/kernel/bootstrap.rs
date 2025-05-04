use std::any::{Any, TypeId};
use std::path::PathBuf;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::kernel::error::{Error, Result};
use crate::kernel::constants;
use crate::kernel::component::{KernelComponent, DependencyRegistry};

// Import component traits and default implementations
use crate::event::{EventManager, DefaultEventManager};
use crate::stage_manager::manager::{StageManager, DefaultStageManager};
use crate::plugin_system::{PluginManager, DefaultPluginManager};
use crate::storage::{StorageManager, DefaultStorageManager, local::LocalStorageProvider}; // Added LocalStorageProvider

/// Main application struct coordinating components via dependency injection
pub struct Application {
    base_path: PathBuf,
    config_dir: PathBuf,
    initialized: bool,
    // Simplified Dependency registry
    dependencies: Arc<Mutex<DependencyRegistry>>,
    // Keep track of component initialization order (using concrete TypeIds)
    component_init_order: Vec<TypeId>,
}

// Updated impl Application block using simplified DependencyRegistry
impl Application {
    /// Creates a new application instance with default components.
    pub fn new(base_path_override: Option<PathBuf>) -> Result<Self> {
        println!("Initializing {} v{}", constants::APP_NAME, constants::APP_VERSION);

        let base_path = base_path_override.unwrap_or_else(|| env::current_dir().unwrap_or_default());
        println!("Using base path: {}", base_path.display());

        let config_dir = base_path.join("user");
        if !config_dir.exists() {
            println!("Creating user data directory: {}", config_dir.display());
            std::fs::create_dir_all(&config_dir)
                .map_err(|e| Error::Init(format!("Failed to create user data directory: {}", e)))?;
        }

        let mut registry = DependencyRegistry::new();
        let mut init_order = Vec::new();

        // Register default components using their concrete types
        let storage_manager = Arc::new(DefaultStorageManager::new(config_dir.clone()));
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
        // Specify the generic parameter for TypeId
        init_order.push(TypeId::of::<DefaultPluginManager<LocalStorageProvider>>());

        let stage_manager = Arc::new(DefaultStageManager::new());
        registry.register_instance(stage_manager.clone()); // Register Arc<DefaultStageManager>, clone Arc
        init_order.push(TypeId::of::<DefaultStageManager>()); // Store concrete TypeId

        Ok(Application {
            base_path,
            config_dir,
            initialized: false,
            dependencies: Arc::new(Mutex::new(registry)),
            component_init_order: init_order,
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
        println!("User data directory: {}", self.config_dir.display());

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

    /// Returns the base path of the application.
    pub fn base_path(&self) -> &PathBuf {
        &self.base_path
    }

    /// Returns the config directory path.
    pub fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }

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
}