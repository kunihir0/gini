//! # Gini Core Plugin System
//!
//! This module provides the comprehensive infrastructure for extending Gini's
//! functionality through dynamically loaded or statically registered plugins.
//! It is responsible for the entire lifecycle of plugins, including discovery,
//! manifest parsing, dependency resolution, version compatibility checks,
//! conflict management, loading, and integration into the core application.
//!
//! ## Key Submodules and Responsibilities:
//!
//! - **[`adapter`]**: Facilitates the interaction between the core system and
//!   plugin-defined components, often involving FFI (Foreign Function Interface)
//!   abstractions.
//! - **[`conflict`]**: Handles the detection and resolution of conflicts between
//!   plugins, such as incompatible dependencies or duplicate provisions.
//! - **[`dependency`]**: Manages plugin dependencies, ensuring that required
//!   plugins and their versions are available.
//! - **[`error`]**: Defines specific error types (e.g., [`PluginError`](error::PluginError))
//!   related to plugin operations.
//! - **[`loader`]**: Responsible for finding, parsing plugin manifests, and loading
//!   plugin libraries into memory.
//! - **[`manager`]**: The central orchestrator ([`PluginManager`]) for the plugin system,
//!   coordinating all aspects of plugin lifecycle and interaction.
//! - **[`manifest`]**: Defines the structure of plugin metadata ([`PluginManifest`]),
//!   which includes information like plugin name, version, dependencies, and capabilities.
//! - **[`registry`]**: Maintains a collection ([`PluginRegistry`]) of all known, loaded,
//!   and active plugins.
//! - **[`traits`]**: Contains essential traits that plugins must implement, most notably
//!   the [`Plugin`] trait, which defines the core interface for all plugins.
//! - **[`version`]**: Provides utilities for parsing, comparing, and managing
//!   plugin versions and version requirements.
//!
//! The plugin system is designed to be robust and flexible, allowing for a rich
//! ecosystem of extensions that can enhance and customize the Gini application.
pub mod registry;
pub mod loader;
pub mod traits;
pub mod dependency;
pub mod version;
pub mod adapter;
pub mod manifest;
pub mod conflict;
pub mod manager;
pub mod error; // Add the new error module

pub use registry::PluginRegistry;
pub use traits::{Plugin, PluginPriority};
pub use version::{ApiVersion, VersionRange};
pub use dependency::PluginDependency;
pub use manifest::PluginManifest;
pub use manager::{PluginManager, DefaultPluginManager};
// Test module declaration
#[cfg(test)]
mod tests;