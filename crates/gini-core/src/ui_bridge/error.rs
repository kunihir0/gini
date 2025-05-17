//! # Gini Core UI Bridge Errors
//!
//! Defines error types specific to the Gini UI Bridge system.
//!
//! This module includes [`UiBridgeError`], the primary enum encompassing
//! various errors that can occur during UI interactions. These can include
//! issues with UI provider initialization, message sending or receiving failures,
//! problems with user input handling, or general communication breakdowns
//! between the core and the UI.
// In a new file: crates/gini-core/src/ui_bridge/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UiBridgeError {
    #[error("UI Interface '{interface_name}' failed to handle message of type '{message_type}': {source}")]
    InterfaceHandlingFailed {
        interface_name: String,
        message_type: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    #[error("Failed to register UI Interface: {0}")]
    RegistrationFailed(String),

    #[error("UI Interface '{0}' not found")]
    InterfaceNotFound(String),

    #[error("Failed to send input to UI: {0}")]
    InputError(String), // Could be enhanced with a source error if applicable

    #[error("UI Interface '{interface_name}' failed during lifecycle method '{method}': {source}")]
    LifecycleMethodFailed {
        interface_name: String,
        method: String, // e.g., "initialize", "update", "finalize"
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    #[error("Failed to acquire lock for '{entity}' during operation '{operation}'")]
    LockError { // For Mutex poisoning or other lock issues
        entity: String, // e.g., "UnifiedUiInterface", "MessageBuffer"
        operation: String,
    },

    #[error("Message buffer operation failed: {0}")]
    MessageBufferError(String), // For errors specific to UnifiedUiManager's message_buffer

    #[error("Operation failed for UI Interface '{interface_name}': {message}")]
    InterfaceOperationFailed { // Generic failure for an interface
        interface_name: String,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    },

    #[error("Multiple UI interfaces failed during operation")]
    MultipleInterfaceFailures(Vec<UiBridgeError>), // To aggregate errors from multiple interfaces

    #[error("UI Bridge internal error: {0}")]
    InternalError(String),
}