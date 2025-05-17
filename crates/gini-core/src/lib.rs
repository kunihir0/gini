//! # Gini Core Library
//!
//! `gini-core` is the central engine of the Gini application framework.
//! It provides the foundational capabilities for building modular and extensible applications.
//!
//! ## Key Features:
//!
//! - **Plugin System:** Dynamically load, manage, and interact with plugins.
//!   See the [`plugin_system`] module.
//! - **Stage Management:** Define and execute a series of stages or tasks in a configurable pipeline.
//!   See the [`stage_manager`] module.
//! - **Event Handling:** A flexible system for inter-module communication via events.
//!   See the [`event`] module.
//! - **Kernel Services:** Core application lifecycle management and essential services.
//!   See the [`kernel`] module.
//! - **Storage Abstraction:** Provides a way to manage application data and configuration.
//!   See the [`storage`] module.
//! - **UI Bridge:** Facilitates communication between the core and user interface components.
//!   See the [`ui_bridge`] module.
//! - **Utilities:** Common helper functions and structures.
//!   See the [`utils`] module.
//!
//! The library is designed to be highly modular, allowing developers to use only the
//! components they need and to extend functionality through custom plugins and stages.
// Declare modules moved from the old src/
pub mod event;
pub mod kernel;
pub mod plugin_system;
pub mod stage_manager;
pub mod storage;
pub mod ui_bridge;
pub mod utils;
// Potentially declare top-level error types if they exist
// pub mod error; // Assuming error types are within modules for now

// Re-export key public types/traits for easier use by the binary and plugins
// These are examples based on typical usage and filenames, adjust as needed
// after reviewing the actual code structure and public interfaces.
pub use kernel::Application; // Corrected: kernel/mod.rs exports Application from bootstrap
pub use kernel::error::Error as KernelError; // Assuming error::Error is the main kernel error type
pub use plugin_system::{Plugin, PluginManifest, PluginManager}; // Assuming PluginManager is public in plugin_system/mod.rs or plugin_system/manager.rs
// pub use plugin_system::error::Error as PluginError; // Add if plugin_system has a public error type
pub use stage_manager::StageManager; // Assuming StageManager is public in stage_manager/mod.rs or stage_manager/manager.rs
pub use event::{EventDispatcher, Event}; // Assuming these are public in event/mod.rs or submodules
pub use storage::StorageProvider; // Assuming this is the primary public trait/struct in storage/mod.rs or storage/provider.rs

// Example of how tests within modules are declared (no change needed if already like this)
// Inside kernel/mod.rs:
// #[cfg(test)]
// mod tests;

// Inside plugin_system/mod.rs:
// #[cfg(test)]
// mod tests;

// Integration tests across multiple modules
#[cfg(test)]
pub mod tests;