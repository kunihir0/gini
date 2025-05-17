//! # Gini Core Storage System Errors
//!
//! Defines error types specific to the Gini Storage System.
//!
//! This module includes [`StorageError`], the primary enum encompassing
//! various errors that can occur during storage operations. These include
//! issues related to file I/O, path resolution, configuration parsing,
//! provider interactions, and other storage-related failures.
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageSystemError {
    #[error("I/O error during operation '{operation}' on path '{path}': {source}")]
    Io {
        path: PathBuf,
        operation: String,
        #[source]
        source: std::io::Error,
    },

    #[error("File not found at path: {0}")]
    FileNotFound(PathBuf),

    #[error("Directory not found at path: {0}")]
    DirectoryNotFound(PathBuf),

    #[error("Access denied for path: {0} during operation: {1}")]
    AccessDenied(PathBuf, String),

    #[error("Path resolution failed for '{path}': {reason}")]
    PathResolutionFailed { path: PathBuf, reason: String },

    #[error("Serialization to '{format}' failed: {source}")]
    SerializationError {
        format: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    #[error("Deserialization from '{format}' failed: {source}")]
    DeserializationError {
        format: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    #[error("Unsupported configuration format: {0}")]
    UnsupportedConfigFormat(String),

    #[error("Configuration not found for scope '{scope}', name '{name}'")]
    ConfigNotFound { scope: String, name: String },

    #[error("Storage operation '{operation}' failed for path '{}': {message}", path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "<unknown>".into()))]
    OperationFailed {
        operation: String,
        path: Option<PathBuf>,
        message: String,
    },

    #[error("Attempted to write to a read-only resource: {0}")]
    ReadOnly(PathBuf),

    #[error("Resource already exists and overwrite is not permitted: {0}")]
    ResourceExists(PathBuf),

    #[error("Invalid path provided: '{path}': {reason}")]
    InvalidPath { path: PathBuf, reason: String },
}

// Helper for creating Io errors, ensuring path is always included.
impl StorageSystemError {
    pub fn io(source: std::io::Error, operation: impl Into<String>, path: PathBuf) -> Self {
        StorageSystemError::Io {
            source,
            operation: operation.into(),
            path,
        }
    }
}