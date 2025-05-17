//! # Gini Core Storage System
//!
//! This module provides abstractions and utilities for managing application data,
//! configuration files, and other persistent storage needs within the Gini framework.
//! It aims to offer a flexible system for accessing and storing data, respecting
//! platform conventions (e.g., XDG base directories on Linux).
//!
//! ## Key Components & Submodules:
//!
//! - **[`config`]**: Handles typed configuration loading, parsing, and management.
//!   It includes [`ConfigManager`](config::ConfigManager) for application-wide
//!   configuration and [`Configurable`](config::Configurable) for components
//!   that require their own specific configurations.
//! - **[`error`]**: Defines storage-specific error types, such as [`StorageError`](error::StorageError).
//! - **[`local`]**: Provides implementations for local file system storage,
//!   including XDG-compliant path resolution through [`LocalFsProvider`](local::LocalFsProvider).
//! - **[`manager`]**: Contains the [`StorageManager`], which orchestrates access to
//!   different storage providers and offers a unified interface for storage operations.
//! - **[`provider`]**: Defines the [`StorageProvider`] trait, an abstraction for
//!   various storage backends (e.g., local file system, cloud storage).
//!   It also includes [`StoragePathCategory`] for classifying different types of
//!   storage locations (e.g., cache, config, data).
//!
//! The storage system allows `gini-core` and its plugins to manage their data
//! and settings in a consistent and organized manner.
pub mod provider;
pub mod local;
pub mod manager; // Add manager module
pub mod config; // Add configuration module
pub mod error; // Add error module


/// Re-export key types
pub use provider::StorageProvider;
pub use local::LocalStorageProvider;
pub use manager::{StorageManager, DefaultStorageManager}; // Export manager types
pub use config::{
    ConfigManager, ConfigFormat, ConfigData, ConfigScope,
    PluginConfigScope, ConfigStorageExt,
}; // Export config types
pub use error::StorageSystemError; // Export the new error type

    
    // Test module declaration
    #[cfg(test)]
    mod tests;