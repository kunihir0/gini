//! # Gini Core Plugin System Errors
//!
//! Defines error types specific to the Gini Plugin System.
//!
//! This module includes [`PluginError`], the primary enum encompassing various
//! errors that can occur during plugin operations. These include issues related to
//! plugin loading, manifest parsing, dependency resolution, version conflicts,
//! FFI (Foreign Function Interface) problems, and general plugin management failures.
//! It also includes more specific error types like [`ManifestError`],
//! [`ResolutionError`], and [`ConflictError`].
// crates/gini-core/src/plugin_system/error.rs
use std::path::PathBuf;
use crate::plugin_system::version::VersionError;
use crate::plugin_system::dependency::DependencyError;

#[derive(Debug, thiserror::Error)]
pub enum PluginSystemError {
    #[error("Plugin loading failed for '{plugin_id}': {source}")]
    LoadingError {
        plugin_id: String,
        path: Option<PathBuf>,
        #[source]
        source: Box<PluginSystemErrorSource>,
    },

    #[error("FFI error in plugin '{plugin_id}' during operation '{operation}': {message}")]
    FfiError {
        plugin_id: String,
        operation: String,
        message: String,
    },

    #[error("Plugin manifest error for '{path}': {message}")]
    ManifestError {
        path: PathBuf,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Plugin registration error for '{plugin_id}': {message}")]
    RegistrationError {
        plugin_id: String,
        message: String,
    },

    #[error("Plugin initialization error for '{plugin_id}': {message}")]
    InitializationError {
        plugin_id: String,
        message: String,
        #[source]
        source: Option<Box<PluginSystemErrorSource>>,
    },

    #[error("Plugin preflight check failed for '{plugin_id}': {message}")]
    PreflightCheckFailed {
        plugin_id: String,
        message: String,
    },

    #[error("Plugin shutdown error for '{plugin_id}': {message}")]
    ShutdownError {
        plugin_id: String,
        message: String,
    },

    #[error("Dependency resolution failed: {0}")]
    DependencyResolution(#[from] DependencyError),

    #[error("Version parsing error: {0}")]
    VersionParsing(#[from] VersionError),

    #[error("Plugin conflict: {message}")]
    ConflictError {
        message: String,
    },

    #[error("Adapter error: {message}")]
    AdapterError {
        message: String,
    },

    #[error("Operation error in plugin '{plugin_id}': {message}", plugin_id = .plugin_id.as_deref().unwrap_or("<unknown>"))]
    OperationError {
        plugin_id: Option<String>,
        message: String,
    },

    #[error("Internal plugin system error: {0}")]
    InternalError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum PluginSystemErrorSource {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("Resolution failed: {0}")]
    Resolution(String),
    #[error("Other: {0}")]
    Other(String),
}