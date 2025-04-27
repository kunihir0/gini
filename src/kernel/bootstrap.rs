use std::path::PathBuf;
use std::env;

use crate::kernel::error::{Error, Result};
use crate::kernel::constants;

/// Main application struct that coordinates all components
pub struct Application {
    config_dir: PathBuf,
    initialized: bool,
    // These will be added as we implement the other modules
    // plugin_registry: PluginRegistry,
    // stage_manager: StageManager,
    // storage_manager: StorageManager,
    // event_dispatcher: EventDispatcher,
    // ui_bridge: UiBridge,
}

impl Application {
    /// Creates a new application instance
    pub fn new() -> Result<Self> {
        println!("Initializing {} v{}", constants::APP_NAME, constants::APP_VERSION);
        
        // Setup config directory
        let home_dir = match env::var("HOME") {
            Ok(path) => PathBuf::from(path),
            Err(e) => return Err(Error::Init(format!("Failed to get HOME directory: {}", e))),
        };
        
        let config_dir = home_dir.join(constants::CONFIG_DIR_NAME);
        
        // Ensure config directory exists
        if !config_dir.exists() {
            println!("Creating configuration directory: {}", config_dir.display());
            match std::fs::create_dir_all(&config_dir) {
                Ok(_) => {},
                Err(e) => return Err(Error::Init(format!("Failed to create config directory: {}", e))),
            }
        }
        
        Ok(Application {
            config_dir,
            initialized: false,
            // Other components will be initialized here
        })
    }
    
    /// Runs the application
    pub fn run(&mut self) -> Result<()> {
        if self.initialized {
            return Err(Error::Init("Application already running".to_string()));
        }
        
        // Initialize components
        self.initialize()?;
        
        // Set initialized flag
        self.initialized = true;
        
        println!("Application initialized successfully.");
        println!("Config directory: {}", self.config_dir.display());
        
        // In a real application, we would have a main loop here
        // For now, just return Ok
        Ok(())
    }
    
    /// Initialize all application components
    fn initialize(&mut self) -> Result<()> {
        // Here we would initialize:
        // 1. Plugin Registry
        // 2. Stage Manager
        // 3. Storage Manager
        // 4. Event Dispatcher
        // 5. UI Bridge
        
        // For now, just return Ok
        Ok(())
    }
    
    /// Returns the config directory
    pub fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }
}