#![cfg(test)]

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::io::{Read, Write};
use async_trait::async_trait;
use std::fs; // Added for test setup
use tempfile::tempdir; // Added for test setup
// Use fully qualified paths instead of aliases
// use tokio::sync::Mutex as TokioMutex;
// use std::sync::Mutex as StdMutex;
use std::time::SystemTime;

use crate::kernel::bootstrap::Application;
use crate::kernel::component::KernelComponent;
use crate::kernel::error::{Error, Result as KernelResult};
use crate::plugin_system::dependency::PluginDependency;
use crate::plugin_system::traits::{Plugin, PluginError, PluginPriority};
use crate::plugin_system::version::VersionRange;
use crate::plugin_system::manager::DefaultPluginManager;
use crate::storage::config::{ConfigManager, ConfigFormat}; // Added for test setup
use crate::storage::local::LocalStorageProvider; // Added for test setup
use crate::stage_manager::{Stage, StageContext, StageResult};
use crate::stage_manager::manager::{StageManager, DefaultStageManager};
use crate::stage_manager::requirement::StageRequirement;
use crate::storage::manager::DefaultStorageManager;
use crate::storage::provider::StorageProvider;

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

    fn init(&self, _app: &mut Application) -> KernelResult<()> {
        println!("{}::init called", self.name());
        // Record initialization order
        let name_clone = self.name.clone();
        let order_tracker = self.execution_order.clone();
        order_tracker.lock().unwrap().push(name_clone);
        Ok(())
    }

    async fn preflight_check(&self, _context: &StageContext) -> Result<(), PluginError> {
        println!("{} preflight check: OK", self.name());
        Ok(())
    }

    fn stages(&self) -> Vec<Box<dyn Stage>> {
        vec![
            Box::new(TestStage::new(&format!("{}_Stage", self.name()), self.stages_executed.clone()))
        ]
    }

    fn shutdown(&self) -> KernelResult<()> { Ok(()) }
// Add default implementations for new trait methods
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

    fn init(&self, _app: &mut Application) -> KernelResult<()> {
        println!("{}::init called", self.name());
        // Record initialization order
        let name_clone = self.name.clone(); // Clone name for async block
        let order_tracker = self.execution_order.clone();
        // Use std::sync::Mutex lock() which returns a Result
        order_tracker.lock().unwrap().push(name_clone);
        Ok(())
    }

    async fn preflight_check(&self, _context: &StageContext) -> Result<(), PluginError> {
        match self.preflight_behavior {
            PreflightBehavior::Success => {
                println!("{} preflight check: OK", self.name());
                Ok(())
            },
            PreflightBehavior::Failure => {
                println!("{} preflight check: FAILED", self.name());
                Err(PluginError::PreflightCheckError(format!(
                    "Simulated preflight check failure for {}", self.name()
                )))
            }
        }
    }

    fn stages(&self) -> Vec<Box<dyn Stage>> {
        vec![
            Box::new(TestStage::new(&format!("{}_Stage", self.name()), self.stages_executed.clone()))
        ]
    }

    fn shutdown(&self) -> KernelResult<()> {
        println!("{}::shutdown called", self.name());
        // Record shutdown order
        let name_clone = self.name.clone();
        let order_tracker = self.shutdown_order.clone();
        // Use std::sync::Mutex lock() which returns a Result
        order_tracker.lock().unwrap().push(name_clone); // Record before returning result

        match self.shutdown_behavior {
            ShutdownBehavior::Success => Ok(()),
            ShutdownBehavior::Failure => Err(Error::Plugin(format!(
                "Simulated shutdown failure for {}", self.name()
            ))),
        }
    }
// Add default implementations for new trait methods
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

    fn init(&self, _app: &mut Application) -> KernelResult<()> { Ok(()) }

    async fn preflight_check(&self, _context: &StageContext) -> Result<(), PluginError> {
        Ok(())
    }

    fn stages(&self) -> Vec<Box<dyn Stage>> {
        vec![
            Box::new(TestStage::new(&format!("{}_Stage", self.name()), self.stages_executed.clone()))
        ]
    }

    fn shutdown(&self) -> KernelResult<()> { Ok(()) }
// Add default implementations for new trait methods
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

    async fn execute(&self, _context: &mut StageContext) -> KernelResult<()> {
        let mut tracker = self.execution_tracker.lock().await;
        tracker.insert(self.id.clone());
        println!("Executing stage: {}", self.id);
        Ok(())
    }
}

// ===== DEPENDENCY STAGES =====

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

    async fn execute(&self, _context: &mut StageContext) -> KernelResult<()> {
        // This stage uses std::sync::Mutex for its tracker, but execute is async.
        // Use spawn_blocking to avoid blocking the async executor.
        let order_tracker = self.execution_order.clone();
        let id_clone = self.id.clone();
        tokio::task::spawn_blocking(move || {
            let mut order_guard = order_tracker.lock().unwrap();
            order_guard.push(id_clone);
        }).await.map_err(|e| Error::Other(format!("Spawn blocking failed: {}", e)))?; // Handle potential join error

        println!("Executing dependent stage: {}", self.id);
        Ok(())
    }
}

// ===== MOCK STORAGE =====

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

    fn create_dir(&self, _path: &Path) -> KernelResult<()> {
        Ok(())
    }

    fn create_dir_all(&self, _path: &Path) -> KernelResult<()> {
        Ok(())
    }

    fn read_to_string(&self, path: &Path) -> KernelResult<String> {
        let binding = path.to_string_lossy().to_string();
        let storage = self.data.lock().unwrap(); // Use std lock
        match storage.get(&binding) {
            Some(bytes) => {
                String::from_utf8(bytes.clone())
                    .map_err(|e| Error::Storage(format!("UTF-8 conversion error: {}", e)))
            },
            None => Err(Error::Storage(format!("File not found: {:?}", path))),
        }
    }

    fn read_to_bytes(&self, path: &Path) -> KernelResult<Vec<u8>> {
        let binding = path.to_string_lossy().to_string();
        let storage = self.data.lock().unwrap(); // Use std lock
        match storage.get(&binding) {
            Some(bytes) => Ok(bytes.clone()),
            None => Err(Error::Storage(format!("File not found: {:?}", path))),
        }
    }

    fn write_string(&self, path: &Path, contents: &str) -> KernelResult<()> {
        self.write_bytes(path, contents.as_bytes())
    }

    fn write_bytes(&self, path: &Path, contents: &[u8]) -> KernelResult<()> {
        let binding = path.to_string_lossy().to_string();
        let mut storage = self.data.lock().unwrap(); // Use std lock
        storage.insert(binding, contents.to_vec());
        Ok(())
    }

    fn copy(&self, from: &Path, to: &Path) -> KernelResult<()> {
        let data = self.read_to_bytes(from)?;
        self.write_bytes(to, &data)
    }

    fn rename(&self, from: &Path, to: &Path) -> KernelResult<()> {
        let data = self.read_to_bytes(from)?;
        self.write_bytes(to, &data)?;
        self.remove_file(from)
    }

    fn remove_file(&self, path: &Path) -> KernelResult<()> {
        let binding = path.to_string_lossy().to_string();
        let mut storage = self.data.lock().unwrap(); // Use std lock
        storage.remove(&binding);
        Ok(())
    }

    fn remove_dir(&self, _path: &Path) -> KernelResult<()> {
        Ok(())
    }

    fn remove_dir_all(&self, _path: &Path) -> KernelResult<()> {
        Ok(())
    }

    fn read_dir(&self, _path: &Path) -> KernelResult<Vec<PathBuf>> {
        Ok(vec![])
    }

    fn metadata(&self, _path: &Path) -> KernelResult<std::fs::Metadata> {
        Err(Error::Storage("Metadata not supported for TestStorageProvider".to_string()))
    }

    fn open_read(&self, _path: &Path) -> KernelResult<Box<dyn Read>> {
        Err(Error::Storage("open_read not supported for TestStorageProvider".to_string()))
    }

    fn open_write(&self, _path: &Path) -> KernelResult<Box<dyn Write>> {
        Err(Error::Storage("open_write not supported for TestStorageProvider".to_string()))
    }

    fn open_append(&self, _path: &Path) -> KernelResult<Box<dyn Write>> {
        Err(Error::Storage("open_append not supported for TestStorageProvider".to_string()))
    }
}

// ===== TEST HELPER FUNCTIONS =====

pub async fn setup_test_environment() -> (
    // Update return type to include generic parameter
    Arc<DefaultPluginManager<LocalStorageProvider>>,
    Arc<DefaultStageManager>,
    Arc<DefaultStorageManager>,
    Arc<tokio::sync::Mutex<HashSet<String>>>, // stages_executed (tokio)
    Arc<std::sync::Mutex<Vec<String>>>, // execution_order (init) (std)
    Arc<std::sync::Mutex<Vec<String>>> // shutdown_order (std)
) {
    // Create storage manager with a test directory path
    let temp_path = std::env::temp_dir().join("gini_test_common"); // Use unique name
    let storage_manager = Arc::new(DefaultStorageManager::new(temp_path));

    // Create stage manager
    let stage_manager = Arc::new(DefaultStageManager::new());

    // Create ConfigManager for PluginManager
    let tmp_dir_config = tempdir().unwrap(); // Separate temp dir for config
    let app_config_path = tmp_dir_config.path().join("app_config");
    let plugin_config_path = tmp_dir_config.path().join("plugin_config");
    fs::create_dir_all(&app_config_path).unwrap();
    fs::create_dir_all(&plugin_config_path).unwrap();
    let provider = Arc::new(LocalStorageProvider::new(tmp_dir_config.path().to_path_buf()));
    let config_manager: Arc<ConfigManager<LocalStorageProvider>> = Arc::new(ConfigManager::new(
        provider,
        app_config_path,
        plugin_config_path,
        ConfigFormat::Json,
    ));

    // Create plugin manager, passing the ConfigManager
    let plugin_manager = match DefaultPluginManager::new(config_manager) { // Pass config_manager
        Ok(pm) => Arc::new(pm),
        Err(e) => panic!("Failed to create plugin manager: {}", e),
    };

    // Trackers for test assertions
    let stages_executed = Arc::new(tokio::sync::Mutex::new(HashSet::new())); // Use full path
    let execution_order = Arc::new(std::sync::Mutex::new(Vec::new())); // Use full path
    let shutdown_order = Arc::new(std::sync::Mutex::new(Vec::new())); // Use full path

    (plugin_manager, stage_manager, storage_manager, stages_executed, execution_order, shutdown_order) // Return shutdown tracker
}
