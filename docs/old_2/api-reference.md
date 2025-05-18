# API Reference

## Overview

This document provides a reference for the main public APIs that are available to plugin developers and other users of the Gini system. This is not an exhaustive reference but covers the key interfaces and types you'll need to work with.

## Core APIs

### Application

The main application class that serves as the entry point.

```rust
pub struct Application {
    // ...
}

impl Application {
    pub fn new(base_path: PathBuf) -> Self;
    pub async fn run(&mut self) -> Result<()>;
    pub fn get_component<T: 'static>(&self) -> Option<&T>;
}
```

### KernelComponent

Interface for all kernel components.

```rust
#[async_trait]
pub trait KernelComponent: Send + Sync {
    fn name(&self) -> &'static str;
    async fn initialize(&self) -> Result<()>;
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
}
```

## Plugin System

### Plugin

The main trait that all plugins must implement.

```rust
pub trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn version(&self) -> &str;
    fn is_core(&self) -> bool;
    fn priority(&self) -> PluginPriority;
    fn compatible_api_versions(&self) -> Vec<VersionRange>;
    fn dependencies(&self) -> Vec<PluginDependency>;
    fn required_stages(&self) -> Vec<StageRequirement>;
    fn init(&self, app: &mut Application) -> Result<(), PluginError>;
    async fn preflight_check(&self, context: &StageContext) -> Result<(), PluginError>;
    fn stages(&self) -> Vec<Box<dyn Stage>>;
    fn shutdown(&self) -> Result<(), PluginError>;
}
```

### PluginManager

Interface for the plugin management system.

```rust
#[async_trait]
pub trait PluginManager: KernelComponent {
    async fn load_plugin(&self, path: &Path) -> Result<()>;
    async fn load_plugins_from_directory(&self, dir: &Path) -> Result<usize>;
    async fn get_plugin(&self, id: &str) -> Result<Option<Arc<dyn Plugin>>>;
    async fn get_plugins(&self) -> Result<Vec<Arc<dyn Plugin>>>;
    async fn get_enabled_plugins(&self) -> Result<Vec<Arc<dyn Plugin>>>;
    async fn is_plugin_loaded(&self, id: &str) -> Result<bool>;
    async fn get_plugin_dependencies(&self, id: &str) -> Result<Vec<String>>;
    async fn get_dependent_plugins(&self, id: &str) -> Result<Vec<String>>;
    async fn enable_plugin(&self, id: &str) -> Result<()>;
    async fn disable_plugin(&self, id: &str) -> Result<()>;
    async fn is_plugin_enabled(&self, id: &str) -> Result<bool>;
    async fn get_plugin_manifest(&self, id: &str) -> Result<Option<PluginManifest>>;
}
```

### PluginPriority

Determines the loading and initialization order of plugins.

```rust
pub enum PluginPriority {
    Kernel(u8),          // 0-10: Reserved for kernel
    CoreCritical(u8),    // 11-50: Critical core functionality
    Core(u8),            // 51-100: Standard core functionality
    ThirdPartyHigh(u8),  // 101-150: High-priority third-party
    ThirdParty(u8),      // 151-200: Standard third-party
    ThirdPartyLow(u8),   // 201-255: Low-priority third-party
}
```

### ApiVersion / VersionRange

Version compatibility checking.

```rust
pub struct ApiVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

pub struct VersionRange {
    pub min: ApiVersion,
    pub max: ApiVersion,
}
```

## Stage Manager

### Stage

The interface for stages that can be executed by the stage manager.

```rust
#[async_trait]
pub trait Stage: Send + Sync {
    fn name(&self) -> &str;
    fn dependencies(&self) -> Vec<StageDependency>;
    async fn execute(&self, context: &mut StageContext) -> StageResult;
    fn supports_dry_run(&self) -> bool { true }
    fn dry_run_description(&self) -> String;
}
```

### StageManager

The interface for the stage management system.

```rust
pub trait StageManager: KernelComponent {
    fn register_stage(&self, stage: Box<dyn Stage>) -> Result<()>;
    fn get_stages(&self) -> Vec<&dyn Stage>;
    fn build_pipeline(&self, stages: &[&str]) -> Result<Pipeline>;
    async fn execute_pipeline(&self, stages: &[&str], context: &mut StageContext) -> Result<()>;
    async fn dry_run_pipeline(&self, stages: &[&str]) -> Result<DryRunReport>;
    fn register_pre_stage_hook(&self, hook: Box<dyn StageHook>) -> HookId;
    fn register_post_stage_hook(&self, hook: Box<dyn StageHook>) -> HookId;
    fn unregister_hook(&self, id: HookId) -> Result<()>;
}
```

### StageContext

The context provided to stages during execution.

```rust
pub struct StageContext {
    // ...
}

impl StageContext {
    pub fn new(
        mode: ExecutionMode,
        storage: Arc<dyn StorageProvider>,
        events: Arc<dyn EventDispatcher>
    ) -> Self;
    
    pub fn execution_mode(&self) -> ExecutionMode;
    pub fn is_dry_run(&self) -> bool;
    pub fn storage_provider(&self) -> &Arc<dyn StorageProvider>;
    pub fn event_dispatcher(&self) -> &Arc<dyn EventDispatcher>;
    pub fn get_config<T: DeserializeOwned>(&self, path: &str) -> Option<T>;
    pub fn set_config<T: Serialize>(&mut self, path: &str, value: T) -> Result<()>;
    pub fn record_dry_run_operation(&mut self, description: impl Into<String>);
}
```

## Event System

### Event

The trait for all events in the system.

```rust
pub trait Event: Send + Sync + Debug {
    fn event_type(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
}
```

### EventDispatcher

The component that dispatches events to registered handlers.

```rust
pub struct EventDispatcher {
    // ...
}

impl EventDispatcher {
    pub fn new() -> Self;
    pub fn register_handler<F>(&mut self, event_type: &str, handler: F) -> HandlerId
        where F: Fn(&dyn Event) -> EventResult + Send + 'static;
    pub fn unregister_handler(&mut self, id: HandlerId) -> bool;
    pub async fn dispatch(&self, event: &dyn Event) -> Vec<EventResult>;
}
```

### EventManager

The kernel component that provides access to the event system.

```rust
pub struct EventManager {
    // ...
}

impl EventManager {
    pub fn new() -> Self;
    pub async fn register_handler<F>(&self, event_type: &str, handler: F) -> HandlerId
        where F: Fn(&dyn Event) -> EventResult + Send + 'static;
    pub async fn unregister_handler(&self, id: HandlerId) -> Result<()>;
    pub async fn dispatch(&self, event: &dyn Event) -> Vec<EventResult>;
}
```

## Storage System

### StorageProvider

The interface for storage access.

```rust
pub trait StorageProvider: Send + Sync {
    fn file_exists(&self, path: &Path) -> Result<bool>;
    fn read_file_to_string(&self, path: &Path) -> Result<String>;
    fn read_file_to_bytes(&self, path: &Path) -> Result<Vec<u8>>;
    fn write_file_from_string(&self, path: &Path, content: &str) -> Result<()>;
    fn write_file_from_bytes(&self, path: &Path, bytes: &[u8]) -> Result<()>;
    fn create_dir_all(&self, path: &Path) -> Result<()>;
    fn remove_file(&self, path: &Path) -> Result<()>;
    fn remove_dir(&self, path: &Path) -> Result<()>;
    fn remove_dir_all(&self, path: &Path) -> Result<()>;
    fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
    fn copy_file(&self, src: &Path, dst: &Path) -> Result<()>;
    fn move_file(&self, src: &Path, dst: &Path) -> Result<()>;
}
```

### StorageManager

The kernel component that manages storage access.

```rust
pub struct StorageManager {
    // ...
}

impl StorageManager {
    pub fn new(base_path: PathBuf) -> Result<Self>;
    pub fn provider(&self) -> &dyn StorageProvider;
    pub fn base_path(&self) -> &Path;
    pub fn user_path(&self) -> &Path;
    pub fn user_config_path(&self) -> PathBuf;
    pub fn user_data_path(&self) -> PathBuf;
    pub fn user_plugins_path(&self) -> PathBuf;
}
```

## UI Bridge

### UiMessage

Message types for UI communication.

```rust
#[derive(Debug, Clone)]
pub enum UiMessage {
    Info(String),
    Warning(String),
    Error(String),
    ProgressStart { id: String, total: usize },
    ProgressUpdate { id: String, current: usize },
    ProgressComplete { id: String },
    DataUpdate { key: String, value: Value },
    StatusChange { status: SystemStatus },
    DashboardUpdate(DashboardData),
    RequestInput { prompt: String, options: Vec<String> },
}
```

### UiConnector

Interface for UI implementations.

```rust
pub trait UiConnector: Send + Sync {
    fn handle_message(&self, message: &UiMessage) -> Result<()>;
    fn send_input(&self, input: UserInput) -> Result<()>;
    fn name(&self) -> &str;
    fn supports_feature(&self, feature: UiFeature) -> bool;
}
```

### UIManager

The kernel component that manages UI communication.

```rust
pub struct UIManager {
    // ...
}

impl UIManager {
    pub fn new() -> Self;
    pub fn send_message(&self, message: UiMessage) -> Result<()>;
    pub fn register_connector(&self, connector: Box<dyn UiConnector>) -> Result<ConnectorId>;
    pub fn unregister_connector(&self, id: ConnectorId) -> Result<()>;
}
```

## Common Types

### Error Handling

```rust
// Kernel errors
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Init(String),
    Plugin(String),
    Stage(String),
    Storage(String),
    Event(String),
    UI(String),
    IO(String),
    Other(String),
}

// Component-specific errors
pub type EventResult<T> = std::result::Result<T, EventError>;
pub type PluginResult<T> = std::result::Result<T, PluginError>;
pub type StageResult = std::result::Result<(), StageError>;
pub type StorageResult<T> = std::result::Result<T, StorageError>;
```

## More Information

For more detailed information on specific components, please refer to their respective documentation:

- [Kernel System](kernel-system.md)
- [Event System](event-system.md)
- [Plugin System](plugin-system.md)
- [Stage Manager](stage-manager.md)
- [Storage System](storage-system.md)
- [UI Bridge](ui-bridge.md)

For a guide on creating plugins, see the [Plugin Creation Guide](plugin-creation-guide.md).