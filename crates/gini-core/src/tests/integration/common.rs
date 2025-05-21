#![cfg(test)]

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::io::{Read, Write};
use std::error::Error as StdError; // For boxing
use async_trait::async_trait;
use std::fs; // Added for test setup
use tempfile::tempdir; // Added for test setup
// Use fully qualified paths instead of aliases
// use tokio::sync::Mutex as TokioMutex;
// use std::sync::Mutex as StdMutex;

use crate::kernel::bootstrap::Application;
// Removed: use crate::kernel::error::Error as KernelError;
// Removed: use crate::kernel::error::Result as KernelResult;
use crate::event::{EventManager, DefaultEventManager}; // Added for test setup
use crate::plugin_system::dependency::PluginDependency;
use crate::plugin_system::error::PluginSystemError; // Import PluginSystemError
use crate::plugin_system::traits::{Plugin, PluginPriority}; // Removed PluginError
use crate::plugin_system::version::VersionRange;
use crate::plugin_system::manager::DefaultPluginManager;
use crate::storage::config::{ConfigManager, ConfigFormat}; // Added for test setup
// Removed unused LocalStorageProvider import
use crate::stage_manager::{Stage, StageContext};
use crate::stage_manager::manager::DefaultStageManager;
use crate::stage_manager::registry::StageRegistry; // Added for register_stages
use crate::stage_manager::requirement::StageRequirement;
use crate::storage::manager::DefaultStorageManager;
use crate::storage::provider::StorageProvider;
use crate::storage::error::StorageSystemError; // Import StorageSystemError
use std::result::Result as StdResult; // Import StdResult for StorageProvider impl

// Type alias for Result with StorageSystemError for TestStorageProvider
type StorageProviderResult<T> = StdResult<T, StorageSystemError>;

// ===== MOCK PLUGINS =====

/// A test plugin that provides a simple stage
pub struct TestPlugin {
    name: String,
    stages_executed: Arc<tokio::sync::Mutex<HashSet<String>>>, // Use full path
    execution_order: Arc<std::sync::Mutex<Vec<String>>>, // Add execution order tracker
}

impl TestPlugin {
    // Update constructor to accept execution_order
    pub fn new(
        name: &str,
        stages_executed: Arc<tokio::sync::Mutex<HashSet<String>>>,
        execution_order: Arc<std::sync::Mutex<Vec<String>>> // Add parameter
    ) -> Self {
        Self {
            name: name.to_string(),
            stages_executed,
            execution_order, // Store it
        }
    }
}

#[async_trait]
impl Plugin for TestPlugin {
    fn name(&self) -> &'static str {
        // This is a hack for testing - in real code each plugin should have a unique static name
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn version(&self) -> &str { "1.0.0" }

    fn is_core(&self) -> bool { false }

    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(150) }

    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        vec![">=0.1.0".parse().unwrap()]
    }

    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }

    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }

    fn init(&self, _app: &mut Application) -> std::result::Result<(), PluginSystemError> {
        println!("{}::init called", self.name());
        // Record initialization order
        let name_clone = self.name.clone();
        let order_tracker = self.execution_order.clone();
        order_tracker.lock().unwrap().push(name_clone);
        Ok(())
    }

    async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), PluginSystemError> {
        println!("{} preflight check: OK", self.name());
        Ok(())
    }

    fn shutdown(&self) -> std::result::Result<(), PluginSystemError> { Ok(()) }
    fn register_stages(&self, registry: &mut StageRegistry) -> std::result::Result<(), PluginSystemError> {
        registry.register_stage(Box::new(TestStage::new(&format!("{}_Stage", self.name()), self.stages_executed.clone()))).map_err(|e| PluginSystemError::InternalError(e.to_string()))?;
        Ok(())
    }
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
}

// ===== MOCK PLUGINS WITH DEPENDENCIES =====

pub enum ShutdownBehavior {
    Success,
    Failure,
}

pub enum PreflightBehavior {
    Success,
    Failure,
}

pub struct DependentPlugin {
    name: String,
    version: String,
    dependencies: Vec<PluginDependency>,
    shutdown_behavior: ShutdownBehavior,
    preflight_behavior: PreflightBehavior,
    stages_executed: Arc<tokio::sync::Mutex<HashSet<String>>>, // Use full path
    execution_order: Arc<std::sync::Mutex<Vec<String>>>, // Use full path
    shutdown_order: Arc<std::sync::Mutex<Vec<String>>>, // Use full path
}

impl DependentPlugin {
    pub fn new(
        name: &str,
        version: &str,
        dependencies: Vec<PluginDependency>,
        shutdown_behavior: ShutdownBehavior,
        preflight_behavior: PreflightBehavior,
        stages_executed: Arc<tokio::sync::Mutex<HashSet<String>>>, // Use full path
        execution_order: Arc<std::sync::Mutex<Vec<String>>>, // Use full path
        shutdown_order: Arc<std::sync::Mutex<Vec<String>>>, // Use full path
    ) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            dependencies,
            shutdown_behavior,
            preflight_behavior,
            stages_executed,
            execution_order, // Store tracker
            shutdown_order, // Store shutdown tracker
        }
    }
}

#[async_trait]
impl Plugin for DependentPlugin {
    fn name(&self) -> &'static str {
        // This is a hack for testing - in real code each plugin should have a unique static name
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn version(&self) -> &str { &self.version }

    fn is_core(&self) -> bool { false }

    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(100) }

    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        vec![">=0.1.0".parse().unwrap()]
    }

    fn dependencies(&self) -> Vec<PluginDependency> { self.dependencies.clone() }

    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }

    fn init(&self, _app: &mut Application) -> std::result::Result<(), PluginSystemError> {
        println!("{}::init called", self.name());
        // Record initialization order
        let name_clone = self.name.clone(); // Clone name for async block
        let order_tracker = self.execution_order.clone();
        // Use std::sync::Mutex lock() which returns a Result
        order_tracker.lock().unwrap().push(name_clone);
        Ok(())
    }

    async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), PluginSystemError> {
        match self.preflight_behavior {
            PreflightBehavior::Success => {
                println!("{} preflight check: OK", self.name());
                Ok(())
            },
            PreflightBehavior::Failure => {
                println!("{} preflight check: FAILED", self.name());
                Err(PluginSystemError::PreflightCheckFailed {
                    plugin_id: self.name.clone(),
                    message: format!("Simulated preflight check failure for {}", self.name()),
                })
            }
        }
    }

    fn shutdown(&self) -> std::result::Result<(), PluginSystemError> {
        println!("{}::shutdown called", self.name());
        // Record shutdown order
        let name_clone = self.name.clone();
        let order_tracker = self.shutdown_order.clone();
        // Use std::sync::Mutex lock() which returns a Result
        order_tracker.lock().unwrap().push(name_clone); // Record before returning result

        match self.shutdown_behavior {
            ShutdownBehavior::Success => Ok(()),
            ShutdownBehavior::Failure => Err(PluginSystemError::ShutdownError {
                plugin_id: self.name.clone(),
                message: format!("Simulated shutdown failure for {}", self.name()),
            }),
        }
    }
    fn register_stages(&self, registry: &mut StageRegistry) -> std::result::Result<(), PluginSystemError> {
        registry.register_stage(Box::new(TestStage::new(&format!("{}_Stage", self.name()), self.stages_executed.clone()))).map_err(|e| PluginSystemError::InternalError(e.to_string()))?;
        Ok(())
    } // Add default implementations for new trait methods
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
}

// ===== PLUGIN WITH API VERSION COMPATIBILITY =====

pub struct VersionedPlugin {
    name: String,
    version: String,
    api_versions: Vec<VersionRange>,
    stages_executed: Arc<tokio::sync::Mutex<HashSet<String>>>, // Use full path
}

impl VersionedPlugin {
    pub fn new(
        name: &str,
        version: &str,
        api_versions: Vec<VersionRange>,
        stages_executed: Arc<tokio::sync::Mutex<HashSet<String>>> // Use full path
    ) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            api_versions,
            stages_executed,
        }
    }
}

#[async_trait]
impl Plugin for VersionedPlugin {
    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn version(&self) -> &str { &self.version }

    fn is_core(&self) -> bool { false }

    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(100) }

    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        self.api_versions.clone()
    }

    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }

    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }

    fn init(&self, _app: &mut Application) -> std::result::Result<(), PluginSystemError> { Ok(()) }

    async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), PluginSystemError> {
        Ok(())
    }

    fn shutdown(&self) -> std::result::Result<(), PluginSystemError> { Ok(()) }
    fn register_stages(&self, registry: &mut StageRegistry) -> std::result::Result<(), PluginSystemError> {
        registry.register_stage(Box::new(TestStage::new(&format!("{}_Stage", self.name()), self.stages_executed.clone()))).map_err(|e| PluginSystemError::InternalError(e.to_string()))?;
        Ok(())
    }
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
}

// ===== TEST STAGE =====

pub struct TestStage {
    id: String,
    execution_tracker: Arc<tokio::sync::Mutex<HashSet<String>>>, // Use full path
}

impl TestStage {
    pub fn new(id: &str, tracker: Arc<tokio::sync::Mutex<HashSet<String>>>) -> Self { // Use full path
        Self {
            id: id.to_string(),
            execution_tracker: tracker,
        }
    }
}

#[async_trait]
impl Stage for TestStage {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.id
    }

    fn description(&self) -> &str {
        "Test stage for integration tests"
    }
 
    async fn execute(&self, _context: &mut StageContext) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        let mut tracker = self.execution_tracker.lock().await;
        tracker.insert(self.id.clone());
        println!("Executing stage: {}", self.id);
        Ok(())
    }
}

// ===== DEPENDENCY STAGES =====

#[allow(dead_code)] // Allow dead code for test helper struct
pub struct DependentStage {
    id: String,
    execution_order: Arc<std::sync::Mutex<Vec<String>>>, // Use full path
    dependency_id: Option<String>,
}

impl DependentStage {
    pub fn new(id: &str, execution_order: Arc<std::sync::Mutex<Vec<String>>>, dependency_id: Option<&str>) -> Self { // Use full path
        Self {
            id: id.to_string(),
            execution_order,
            dependency_id: dependency_id.map(|s| s.to_string()),
        }
    }
}

#[async_trait]
impl Stage for DependentStage {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.id
    }

    fn description(&self) -> &str {
        "Stage with dependencies for testing"
    }
 
    async fn execute(&self, _context: &mut StageContext) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        // This stage uses std::sync::Mutex for its tracker, but execute is async.
        // Use spawn_blocking to avoid blocking the async executor.
        let order_tracker = self.execution_order.clone();
        let id_clone = self.id.clone();
        tokio::task::spawn_blocking(move || {
            let mut order_guard = order_tracker.lock().unwrap();
            order_guard.push(id_clone);
        }).await.map_err(|e| Box::new(crate::kernel::error::Error::Other(format!("Spawn blocking failed: {}", e))) as Box<dyn StdError + Send + Sync + 'static>)?; // Handle potential join error
 
        println!("Executing dependent stage: {}", self.id);
        Ok(())
    }
}

// ===== MOCK STORAGE =====

#[derive(Debug)] // Add derive Debug
pub struct TestStorageProvider {
    data: Arc<std::sync::Mutex<HashMap<String, Vec<u8>>>>, // Use full path
}

impl TestStorageProvider {
    pub fn new() -> Self {
        Self {
            data: Arc::new(std::sync::Mutex::new(HashMap::new())), // Use full path
        }
    }
}

impl StorageProvider for TestStorageProvider {
    fn name(&self) -> &str {
        "TestStorageProvider"
    }

    fn exists(&self, path: &Path) -> bool {
        let binding = path.to_string_lossy().to_string();
        let storage = self.data.lock().unwrap(); // Use std lock
        storage.contains_key(&binding)
    }

    fn is_file(&self, path: &Path) -> bool {
        self.exists(path)
    }

    fn is_dir(&self, _path: &Path) -> bool {
        false
    }

    fn create_dir(&self, _path: &Path) -> StorageProviderResult<()> {
        Ok(())
    }

    fn create_dir_all(&self, _path: &Path) -> StorageProviderResult<()> {
        Ok(())
    }

    fn read_to_string(&self, path: &Path) -> StorageProviderResult<String> {
        let binding = path.to_string_lossy().to_string();
        let storage = self.data.lock().unwrap(); // Use std lock
        match storage.get(&binding) {
            Some(bytes) => {
                String::from_utf8(bytes.clone()).map_err(|e| StorageSystemError::OperationFailed {
                    operation: "read_to_string".to_string(),
                    path: Some(path.to_path_buf()),
                    message: format!("UTF-8 conversion error: {}", e),
                })
            },
            None => Err(StorageSystemError::FileNotFound(path.to_path_buf())),
        }
    }

    fn read_to_bytes(&self, path: &Path) -> StorageProviderResult<Vec<u8>> {
        let binding = path.to_string_lossy().to_string();
        let storage = self.data.lock().unwrap(); // Use std lock
        match storage.get(&binding) {
            Some(bytes) => Ok(bytes.clone()),
            None => Err(StorageSystemError::FileNotFound(path.to_path_buf())),
        }
    }

    fn write_string(&self, path: &Path, contents: &str) -> StorageProviderResult<()> {
        self.write_bytes(path, contents.as_bytes())
    }

    fn write_bytes(&self, path: &Path, contents: &[u8]) -> StorageProviderResult<()> {
        let binding = path.to_string_lossy().to_string();
        let mut storage = self.data.lock().unwrap(); // Use std lock
        storage.insert(binding, contents.to_vec());
        Ok(())
    }

    fn copy(&self, from: &Path, to: &Path) -> StorageProviderResult<()> {
        let data = self.read_to_bytes(from)?;
        self.write_bytes(to, &data)
    }

    fn rename(&self, from: &Path, to: &Path) -> StorageProviderResult<()> {
        let data = self.read_to_bytes(from)?;
        self.write_bytes(to, &data)?;
        self.remove_file(from)
    }

    fn remove_file(&self, path: &Path) -> StorageProviderResult<()> {
        let binding = path.to_string_lossy().to_string();
        let mut storage = self.data.lock().unwrap(); // Use std lock
        storage.remove(&binding);
        Ok(())
    }

    fn remove_dir(&self, _path: &Path) -> StorageProviderResult<()> {
        Ok(())
    }

    fn remove_dir_all(&self, _path: &Path) -> StorageProviderResult<()> {
        Ok(())
    }

    fn read_dir(&self, _path: &Path) -> StorageProviderResult<Vec<PathBuf>> {
        Ok(vec![])
    }

    fn metadata(&self, path: &Path) -> StorageProviderResult<std::fs::Metadata> {
        // This is tricky to mock correctly. Return an error for now.
        Err(StorageSystemError::OperationFailed {
            operation: "metadata".to_string(),
            path: Some(path.to_path_buf()),
            message: "Metadata not supported for TestStorageProvider".to_string(),
        })
    }

    fn open_read(&self, path: &Path) -> StorageProviderResult<Box<dyn Read>> {
        // Mock reading by returning bytes from the map wrapped in a Cursor
        let files = self.data.lock().unwrap(); // Use self.data
        let binding = path.to_string_lossy().to_string();
        if let Some(data) = files.get(&binding) {
            Ok(Box::new(std::io::Cursor::new(data.clone())))
        } else {
            Err(StorageSystemError::FileNotFound(path.to_path_buf()))
        }
    }

    fn open_write(&self, path: &Path) -> StorageProviderResult<Box<dyn Write>> {
        // Mock writing is complex. For now, return an error.
        Err(StorageSystemError::OperationFailed {
            operation: "open_write".to_string(),
            path: Some(path.to_path_buf()),
            message: "open_write not fully supported for TestStorageProvider".to_string(),
        })
    }

    fn open_append(&self, path: &Path) -> StorageProviderResult<Box<dyn Write>> {
        // Mock appending is complex. For now, return an error.
        Err(StorageSystemError::OperationFailed {
            operation: "open_append".to_string(),
            path: Some(path.to_path_buf()),
            message: "open_append not fully supported for TestStorageProvider".to_string(),
        })
    }
}

// ===== TEST HELPER FUNCTIONS =====

pub async fn setup_test_environment() -> (
    // Update return type to remove generic parameter
    Arc<DefaultPluginManager>, // Remove <LocalStorageProvider>
    Arc<DefaultStageManager>,
    Arc<DefaultStorageManager>,
    Arc<tokio::sync::Mutex<HashSet<String>>>, // stages_executed (tokio)
    Arc<std::sync::Mutex<Vec<String>>>, // execution_order (init) (std)
    Arc<std::sync::Mutex<Vec<String>>> // shutdown_order (std)
) {
    // Create storage manager with a test directory path
    let _temp_path = std::env::temp_dir().join("gini_test_common"); // Path no longer needed for constructor
    // DefaultStorageManager::new now takes no arguments and returns Result
    // Handle the Result before creating the Arc
    let storage_manager_instance = DefaultStorageManager::new().expect("Failed to create test storage manager instance");
    let storage_manager = Arc::new(storage_manager_instance); // Create Arc from the instance
 
    // Create event manager for the stage manager
    let event_manager = Arc::new(DefaultEventManager::new()) as Arc<dyn EventManager>;
 
    // Create stage manager
    let stage_manager = Arc::new(DefaultStageManager::new(event_manager.clone()));
 
    // Create ConfigManager for PluginManager
    // Ensure a unique directory for each call to setup_test_environment for config paths
    let unique_config_dir = tempdir().expect("Failed to create unique temp dir for config manager");
    let app_config_path = unique_config_dir.path().join("app_config");
    let plugin_config_path = unique_config_dir.path().join("plugin_config");
    fs::create_dir_all(&app_config_path).expect("Failed to create app_config dir");
    fs::create_dir_all(&plugin_config_path).expect("Failed to create plugin_config dir");
    
    // The DefaultStorageManager created earlier determines its own XDG paths for general storage.
    // For the ConfigManager used by DefaultPluginManager, we want it to use these specific, isolated temp paths.
    // We need a StorageProvider instance that operates on these temp paths for the ConfigManager.
    // However, DefaultPluginManager takes an Arc<ConfigManager>, and ConfigManager is constructed with a StorageProvider.
    // The existing `storage_manager` (DefaultStorageManager) from setup_test_environment uses XDG paths.
    // We need a *new* ConfigManager instance for the PluginManager that uses a provider scoped to these unique temp dirs.
    // This implies that the DefaultPluginManager should perhaps be constructed with paths, or the ConfigManager it uses
    // should be configurable to use specific paths rather than deriving from a global StorageManager's provider.

    // For now, let's create a new LocalStorageProvider specifically for this ConfigManager,
    // which is not ideal as it diverges from the main storage_manager's provider, but will isolate configs.
    // A better long-term solution would be to make ConfigManager's paths more flexible or
    // allow DefaultPluginManager to be configured with specific config paths.

    // Create a new LocalStorageProvider instance. This is a simplified approach for test isolation.
    // Note: LocalStorageProvider::new() might not be public or might require a base path.
    // Assuming DefaultStorageManager's provider() method gives an Arc<dyn StorageProvider> that can be used.
    // The DefaultStorageManager's provider is already created and uses XDG paths.
    // What we need is for the ConfigManager to operate on the unique_config_dir.
    // The ConfigManager takes a StorageProvider. We can't easily change the base path of the existing provider.

    // Let's stick to the original approach of creating a ConfigManager with the unique paths,
    // but acknowledge this ConfigManager will use the *same provider type* as the main storage_manager,
    // but operate on different *base paths* passed to its constructor.
    let provider_for_plugin_config = Arc::clone(storage_manager.provider()); // Use the same provider type

    let config_manager: Arc<ConfigManager> = Arc::new(ConfigManager::new(
        provider_for_plugin_config, // This provider will be used with the paths below
        app_config_path,      // Isolated app config path
        plugin_config_path,   // Isolated plugin config path
        ConfigFormat::Json,
    ));

    // Create plugin manager, passing the ConfigManager and StageRegistry Arc
    let stage_registry_arc = stage_manager.registry();
    let plugin_manager = match DefaultPluginManager::new(config_manager, stage_registry_arc) { // Pass config_manager and stage_registry_arc
        Ok(pm) => Arc::new(pm),
        Err(e) => panic!("Failed to create plugin manager: {}", e),
    };

    // Trackers for test assertions
    let stages_executed = Arc::new(tokio::sync::Mutex::new(HashSet::new())); // Use full path
    let execution_order = Arc::new(std::sync::Mutex::new(Vec::new())); // Use full path
    let shutdown_order = Arc::new(std::sync::Mutex::new(Vec::new())); // Use full path

    // Return the Arc<DefaultStorageManager>
    (plugin_manager, stage_manager, storage_manager, stages_executed, execution_order, shutdown_order)
}
