//! # Gini Core Kernel Errors
//!
//! Defines error types specific to the Gini Kernel.
//!
//! This module includes [`KernelError`], the primary enum encompassing various
//! errors that can occur during kernel operations, such as application
//! bootstrapping failures, component lifecycle issues, or problems with
//! the dependency registry.
use std::error::Error as StdError;
// use std::fmt; // Removed unused import
use std::result::Result as StdResult;

use std::path::PathBuf; // Ensure PathBuf is imported correctly at the top
// Import the new PluginSystemError
use crate::plugin_system::error::PluginSystemError;
use crate::event::error::EventSystemError; // New import
use crate::stage_manager::error::StageSystemError; // New import for StageSystemError
use crate::storage::error::StorageSystemError; // New import for StorageSystemError
use crate::ui_bridge::error::UiBridgeError; // New import for UiBridgeError
use thiserror::Error as ThisError; // Import ThisError

/// Custom error type for the OSX-Forge application
#[derive(Debug, ThisError)] // Add ThisError derive
pub enum Error {
    /// Deprecated: Generic initialization error. Use `KernelLifecycleError` instead.
    #[deprecated(since = "0.6.0", note = "Use `Error::KernelLifecycleError` with appropriate phase instead.")]
    #[error("Initialization error (deprecated): {0}")]
    Init(String),
    /// Deprecated: Generic plugin system error. Use `PluginSystem` instead.
    #[deprecated(since = "0.2.0", note = "Use `Error::PluginSystem` instead for typed plugin errors")]
    #[error("Plugin error (deprecated): {0}")]
    Plugin(String),
    /// Specific, typed plugin system error
    #[error("Plugin system error: {0}")]
    PluginSystem(#[from] PluginSystemError),
    /// Deprecated: Generic stage system error. Use `StageSystem` instead.
    #[deprecated(since = "0.4.0", note = "Use `Error::StageSystem` instead for typed stage system errors")]
    #[error("Stage error (deprecated): {0}")]
    Stage(String),
    /// Specific, typed stage system error
    #[error("Stage system error: {0}")]
    StageSystem(#[from] StageSystemError),
    
    // --- Storage & Config Errors ---
    #[deprecated(since = "0.5.0", note = "Use `Error::StorageSystem(StorageSystemError::Io)` instead.")]
    #[error("I/O error (deprecated, use StorageSystem): during operation '{operation}' on path '{}': {source}", path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "<unknown>".into()))]
    IoError { #[source] source: std::io::Error, path: Option<PathBuf>, operation: String },

    #[deprecated(since = "0.5.0", note = "Use `Error::StorageSystem(StorageSystemError::SerializationError)` instead.")]
    #[error("Serialization error (deprecated, use StorageSystem): Failed to serialize to {format}: {source}")]
    SerializationError { format: String, #[source] source: Box<dyn StdError + Send + Sync> },

    #[deprecated(since = "0.5.0", note = "Use `Error::StorageSystem(StorageSystemError::DeserializationError)` instead.")]
    #[error("Deserialization error (deprecated, use StorageSystem): Failed to deserialize from {format}: {source}")]
    DeserializationError { format: String, #[source] source: Box<dyn StdError + Send + Sync> },

    #[deprecated(since = "0.5.0", note = "Use `Error::StorageSystem(StorageSystemError::UnsupportedConfigFormat)` or similar instead.")]
    #[error("Config format error (deprecated, use StorageSystem): Unknown or unsupported config format for path: {path}")]
    ConfigFormatError { path: PathBuf },

    #[deprecated(since = "0.5.0", note = "Use `Error::StorageSystem(StorageSystemError::FileNotFound)` instead.")]
    #[error("File not found (deprecated, use StorageSystem): {path}")]
    FileNotFound { path: PathBuf },

    #[deprecated(since = "0.5.0", note = "Use `Error::StorageSystem(StorageSystemError::DirectoryNotFound)` instead.")]
    #[error("Directory not found (deprecated, use StorageSystem): {path}")]
    DirectoryNotFound { path: PathBuf },

    #[deprecated(since = "0.5.0", note = "Use `Error::StorageSystem(StorageSystemError::OperationFailed)` instead.")]
    #[error("Storage operation failed (deprecated, use StorageSystem): operation '{operation}' failed for path '{}': {message}", path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "<unknown>".into()))]
    StorageOperationFailed { operation: String, path: Option<PathBuf>, message: String },

    /// Specific, typed storage system error
    #[error("Storage system error: {0}")]
    StorageSystem(#[from] StorageSystemError),
    // --- End Storage & Config Errors ---

    /// Event system error
    #[error("Event error: {0}")]
    #[deprecated(since = "0.3.0", note = "Use `Error::EventSystem` instead for typed event system errors")]
    Event(String),
    #[error("Event system error: {0}")]
    EventSystem(#[from] EventSystemError), // New variant

    /// UI Bridge system error
    #[error("UI Bridge system error: {0}")]
    UiBridge(#[from] UiBridgeError),

    /// Deprecated: Generic component error. Use `KernelLifecycleError` or `ComponentRegistryError` instead.
    #[deprecated(since = "0.6.0", note = "Use `Error::KernelLifecycleError` or `Error::ComponentRegistryError` instead.")]
    #[error("Component error (deprecated): {0}")]
    Component(String),
    /// Deprecated: Generic dependency injection error. Use `ComponentRegistryError` or `KernelLifecycleError` instead.
    #[deprecated(since = "0.6.0", note = "Use `Error::ComponentRegistryError` or `Error::KernelLifecycleError` for more specific DI-related errors.")]
    #[error("Dependency injection error (deprecated): {0}")]
    DependencyInjection(String),

    /// Error occurring during a specific kernel lifecycle phase.
    #[error("Kernel lifecycle error during {phase:?}: {message}")]
    KernelLifecycleError {
        phase: KernelLifecyclePhase,
        component_name: Option<String>,
        type_id_str: Option<String>, // To store formatted TypeId if relevant
        message: String,
        #[source]
        source: Option<Box<Error>>, // Can wrap another KernelError or a subsystem error
    },

    /// Error related to the DependencyRegistry operations or component lookup failures.
    #[error("Component registry error during operation '{operation}': {message}")]
    ComponentRegistryError {
        operation: String, // e.g., "RetrieveForInitialize", "RegisterComponent"
        component_name: Option<String>,
        type_id_str: Option<String>,
        message: String,
    },

    /// Generic error with message
    #[error("Error: {0}")]
    Other(String),
}

/// Represents a specific phase in the kernel's lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, ThisError)]
pub enum KernelLifecyclePhase {
    #[error("Bootstrap")]
    Bootstrap,
    #[error("Initialize")]
    Initialize,
    #[error("Start")]
    Start,
    #[error("RunPreCheck")]
    RunPreCheck,
    #[error("Shutdown")]
    Shutdown,
}


/// Shorthand for Result with our Error type
pub type Result<T> = StdResult<T, Error>;

// The #[derive(ThisError)] handles the Display trait implementation based on #[error(...)] attributes.
// However, for variants like IoError and StorageOperationFailed that had conditional formatting
// based on `Option<PathBuf>`, we might need to customize their Display or accept simpler messages.
// For now, thiserror will generate Display based on the #[error] attributes.
// If more complex Display logic is needed, the manual `impl fmt::Display for Error` can be kept,
// and the `#[error(...)]` attributes would primarily serve `thiserror` for `source()` and `From` impls.
// Let's keep the manual Display for now to preserve the detailed conditional messages.

// The manual `impl fmt::Display for Error` is no longer needed as `thiserror` handles it.
// The manual `impl StdError for Error` is also no longer needed.

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Error::Other(msg.to_string())
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Error::Other(msg)
    }
}

// Note: We keep the generic From<std::io::Error> but specific code should ideally
// create the more detailed Error::IoError variant with path and operation context.
impl From<std::io::Error> for Error {
    fn from(io_err: std::io::Error) -> Self {
        Error::StorageSystem(StorageSystemError::Io {
            source: io_err,
            path: PathBuf::new(), // Path is unknown here, provide a default or consider removing this From impl
            operation: "unknown".to_string(),
        })
    }
}

// Helper to create IoError with context, now wraps StorageSystemError::Io
impl Error {
    pub fn io(source: std::io::Error, operation: impl Into<String>, path: PathBuf) -> Self { // path is no longer Option
        Error::StorageSystem(StorageSystemError::Io {
            source,
            operation: operation.into(),
            path,
        })
    }
}