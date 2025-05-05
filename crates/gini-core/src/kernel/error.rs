use std::error::Error as StdError;
use std::fmt;
use std::result::Result as StdResult;

use std::path::PathBuf; // Ensure PathBuf is imported correctly at the top

/// Custom error type for the OSX-Forge application
#[derive(Debug)] // This should be directly above the enum definition
pub enum Error {
    /// Initialization error
    Init(String),
    /// Plugin system error
    Plugin(String),
    /// Stage execution error
    Stage(String),
    
    // --- Storage & Config Errors ---
    /// I/O error during storage operation
    IoError {
        source: std::io::Error,
        path: Option<PathBuf>, // Optional path context
        operation: String, // Describe the operation (e.g., "read", "write", "create_dir")
    },
    /// Error serializing configuration data
    SerializationError {
        format: String, // e.g., "JSON", "YAML"
        source: Box<dyn StdError + Send + Sync>,
    },
    /// Error deserializing configuration data
    DeserializationError {
        format: String, // e.g., "JSON", "YAML"
        source: Box<dyn StdError + Send + Sync>,
    },
    /// Could not determine configuration format from path
    ConfigFormatError { path: PathBuf },
    /// Specific file not found
    FileNotFound { path: PathBuf },
    /// Specific directory not found
    DirectoryNotFound { path: PathBuf },
    /// Generic storage operation failure
    StorageOperationFailed {
        operation: String,
        path: Option<PathBuf>,
        message: String
    },
    // --- End Storage & Config Errors ---

    /// Event system error
    Event(String),
    /// Component error
    Component(String),
    /// Dependency injection error
    DependencyInjection(String),
    /// Generic error with message
    Other(String),
}

/// Shorthand for Result with our Error type
pub type Result<T> = StdResult<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Init(msg) => write!(f, "Initialization error: {}", msg),
            Error::Plugin(msg) => write!(f, "Plugin error: {}", msg),
            Error::Stage(msg) => write!(f, "Stage error: {}", msg),
            Error::IoError { source, path, operation } => {
                if let Some(p) = path {
                    write!(f, "I/O error during '{}' at path '{}': {}", operation, p.display(), source)
                } else {
                    write!(f, "I/O error during '{}': {}", operation, source)
                }
            },
            Error::SerializationError { format, source } => write!(f, "Failed to serialize to {}: {}", format, source),
            Error::DeserializationError { format, source } => write!(f, "Failed to deserialize from {}: {}", format, source),
            Error::ConfigFormatError { path } => write!(f, "Unknown or unsupported config format for path: {}", path.display()),
            Error::FileNotFound { path } => write!(f, "File not found: {}", path.display()),
            Error::DirectoryNotFound { path } => write!(f, "Directory not found: {}", path.display()),
            Error::StorageOperationFailed { operation, path, message } => {
                 if let Some(p) = path {
                    write!(f, "Storage operation '{}' failed for path '{}': {}", operation, p.display(), message)
                } else {
                    write!(f, "Storage operation '{}' failed: {}", operation, message)
                }
            },
            Error::Event(msg) => write!(f, "Event error: {}", msg),
            Error::Component(msg) => write!(f, "Component error: {}", msg),
            Error::DependencyInjection(msg) => write!(f, "Dependency injection error: {}", msg),
            Error::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::IoError { source, .. } => Some(source),
            Error::SerializationError { source, .. } => Some(source.as_ref()),
            Error::DeserializationError { source, .. } => Some(source.as_ref()),
            // Other variants don't wrap another error directly
            _ => None,
        }
    }
}

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
        Error::IoError {
            source: io_err,
            path: None, // No path context available here
            operation: "unknown".to_string(), // Operation context is lost
        }
    }
}

// Helper to create IoError with context
impl Error {
    pub fn io(source: std::io::Error, operation: impl Into<String>, path: Option<PathBuf>) -> Self {
        Error::IoError { source, operation: operation.into(), path }
    }
}