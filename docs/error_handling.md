# Error Handling Documentation

This document outlines the error handling patterns and practices used across the Gini application.

## Error Handling Architecture

The Gini application implements a comprehensive error handling strategy across its various modules. Each subsystem defines its own error types that implement the standard `Error` trait, allowing for consistent error propagation throughout the application.

## Module-Specific Error Types

### Kernel Errors

The kernel module defines its error types in `crates/gini-core/src/kernel/error.rs`, representing failures that can occur during the kernel bootstrapping and component lifecycle management.

```rust
// Key error types:
pub enum Error { // Note: The actual enum in kernel/error.rs is named `Error`
    PluginSystem(PluginSystemError), // Example of wrapping a subsystem error
    StageSystem(StageSystemError),
    StorageSystem(StorageSystemError),
    EventSystem(EventSystemError),
    UiBridge(UiBridgeError),
    KernelLifecycleError { phase: KernelLifecyclePhase, message: String, ... },
    ComponentRegistryError { operation: String, message: String, ... },
    // ...other variants including deprecated ones and specific errors
}
```

### Plugin System Errors

Plugin system errors are defined in `crates/gini-core/src/plugin_system/error.rs` and represent failures related to plugin loading, dependency resolution, and execution.

```rust
// Key error types:
pub enum PluginSystemError {
    LoadingError { plugin_id: String, path: Option<PathBuf>, source: Box<PluginSystemErrorSource> },
    FfiError { plugin_id: String, operation: String, message: String },
    ManifestError { path: PathBuf, message: String, source: Option<Box<dyn std::error::Error + Send + Sync>> },
    RegistrationError { plugin_id: String, message: String },
    InitializationError { plugin_id: String, message: String, source: Option<Box<PluginSystemErrorSource>> },
    PreflightCheckFailed { plugin_id: String, message: String },
    DependencyResolution(#[from] DependencyError),
    VersionParsing(#[from] VersionError),
    ConflictError { message: String },
    // ...other variants like ShutdownError, AdapterError, OperationError, InternalError
}
```

### Event System Errors

Event system errors in `crates/gini-core/src/event/error.rs` represent failures during event registration, dispatch, and handling.

```rust
// Key error types:
pub enum EventSystemError {
    HandlerRegistrationFailedByName { event_name: String, reason: String },
    HandlerRegistrationFailedByType { type_id: TypeId, reason: String },
    HandlerUnregistrationFailed { id: EventId, reason: String },
    DispatchError { event_name: String, reason: String },
    QueueOperationFailed { operation: String, reason: String },
    InvalidEventData { event_name: String, details: String },
    DispatcherPoisoned { component: String },
    InternalError(String),
    // ...other error variants
}
```

### Storage System Errors

Storage errors in `crates/gini-core/src/storage/error.rs` represent failures related to data storage, configuration, and persistence.

```rust
// Key error types:
pub enum StorageSystemError {
    Io { path: PathBuf, operation: String, source: std::io::Error },
    FileNotFound(PathBuf),
    DirectoryNotFound(PathBuf),
    AccessDenied(PathBuf, String),
    SerializationError { format: String, source: Box<dyn std::error::Error + Send + Sync + 'static> },
    DeserializationError { format: String, source: Box<dyn std::error::Error + Send + Sync + 'static> },
    ConfigNotFound { scope: String, name: String },
    OperationFailed { operation: String, path: Option<PathBuf>, message: String },
    // ...other variants like PathResolutionFailed, UnsupportedConfigFormat, ReadOnly, ResourceExists, InvalidPath
}
```

### UI Bridge Errors

UI bridge errors in `crates/gini-core/src/ui_bridge/error.rs` represent failures in the communication between the core application and the UI layer.

```rust
// Key error types:
pub enum UiBridgeError {
    InterfaceHandlingFailed { interface_name: String, message_type: String, source: Box<dyn std::error::Error + Send + Sync + 'static> },
    RegistrationFailed(String),
    InterfaceNotFound(String),
    InputError(String),
    LifecycleMethodFailed { interface_name: String, method: String, source: Box<dyn std::error::Error + Send + Sync + 'static> },
    LockError { entity: String, operation: String },
    MessageBufferError(String),
    // ...other variants like InterfaceOperationFailed, MultipleInterfaceFailures, InternalError
}
```

## Error Propagation Patterns

Gini uses the following patterns for error handling and propagation:

1. **Result Return Values**: Functions that can fail return `Result<T, E>` types
2. **Error Context**: The `?` operator is used with context-adding combinators
3. **Error Translation**: Module boundaries include error translation layers
4. **Centralized Logging**: Error logging is centralized through the event system

## Error Recovery Strategies

The application employs several strategies for error recovery:

1. **Graceful Degradation**: When a plugin fails to load, the system continues without it
2. **Retry Logic**: For transient errors, particularly in storage operations
3. **Fallback Mechanisms**: Default configurations are used when config files are missing or corrupt
4. **User Notification**: Critical errors are reported to users via the UI bridge

## Error Handling Best Practices

1. **Be Specific**: Use specific error types rather than generic strings
2. **Add Context**: Include relevant context with each error
3. **Log Appropriately**: Log errors at appropriate levels
4. **Recover When Possible**: Implement recovery strategies where feasible
5. **User-Friendly Messages**: Translate technical errors to user-friendly messages

## Testing Error Conditions

The testing strategy includes:

1. **Unit Tests**: Each error case is tested at the unit level
2. **Integration Tests**: Error propagation is verified in integration tests
3. **Fault Injection**: Tests deliberately inject failures to verify correct error handling

## Future Improvements

1. **Enhanced Error Reporting**: More detailed error reporting with stack traces
2. **User-Configurable Error Handling**: Allow users to configure how certain errors are handled
3. **Error Telemetry**: Anonymous error reporting for application improvement