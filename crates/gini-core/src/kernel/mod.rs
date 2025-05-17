//! # Gini Core Kernel
//!
//! The `kernel` module forms the heart of the `gini-core` application framework.
//! It is responsible for the fundamental operations of the application, including
//! bootstrapping, managing the lifecycle of core components, and providing
//! essential services and constants.
//!
//! ## Key Responsibilities & Components:
//!
//! - **Application Bootstrapping**: Initializes and starts the application.
//!   Managed by the [`Application`](bootstrap::Application) struct from the `bootstrap` submodule.
//! - **Component Lifecycle**: Defines and manages core components through the
//!   [`KernelComponent`](component::KernelComponent) trait and a [`DependencyRegistry`](component::DependencyRegistry)
//!   for shared component access, both found in the `component` submodule.
//! - **Core Constants**: Provides system-wide constants via the `constants` submodule.
//! - **Error Handling**: Defines kernel-specific error types ([`Error`](error::Error)) and
//!   a `Result` type alias in the `error` submodule.
//!
//! This module orchestrates the core functionalities, ensuring that different parts of
//! the Gini framework can initialize, interact, and terminate in a coordinated manner.
pub mod bootstrap;
pub mod component;
pub mod constants;
pub mod error;

pub use bootstrap::Application;
pub use component::{KernelComponent, DependencyRegistry}; // Removed ComponentDependency
pub use error::{Error, Result};
// Test module declaration
#[cfg(test)]
mod tests;