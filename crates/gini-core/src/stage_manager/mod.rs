//! # Gini Core Stage Manager
//!
//! This module provides the framework for defining, organizing, and executing
//! sequences of operations, known as "stages," within the Gini application.
//! It allows for the creation of complex workflows or pipelines by managing
//! stage dependencies, execution order, and providing a shared context.
//!
//! ## Core Concepts & Components:
//!
//! - **[`Stage`] Trait**: The fundamental trait that all stages must implement.
//!   It defines methods for stage metadata (name, description), execution logic
//!   ([`execute`](Stage::execute)), and optional dry-run capabilities.
//! - **[`StageContext`](context::StageContext)**: An object passed to each stage during execution,
//!   providing access to shared resources, application state, and inter-stage
//!   communication mechanisms.
//! - **[`StageManager`](manager::StageManager)**: The central orchestrator responsible for managing
//!   the lifecycle of stages, including registration, dependency resolution,
//!   pipeline construction, and execution.
//! - **[`StagePipeline`](pipeline::StagePipeline)**: Represents an ordered sequence of stages
//!   to be executed. Pipelines can be dynamically built and configured.
//! - **[`StageRegistry`](registry::StageRegistry)**: A collection of all available stages,
//!   allowing them to be discovered and utilized by the `StageManager`.
//! - **[`StageResult`]**: An enum indicating the outcome of a stage's execution
//!   (e.g., success, failure, skipped).
//! - **Submodules**:
//!     - `context`: Defines the `StageContext`.
//!     - `core_stages`: Provides common, built-in stage implementations.
//!     - `dependency`: Handles stage dependency definition and resolution.
//!     - `dry_run`: Logic related to dry-run execution of stages.
//!     - `error`: Defines error types specific to the stage manager ([`StageError`](error::StageError)).
//!     - `manager`: Contains the `StageManager`.
//!     - `pipeline`: Defines the `StagePipeline`.
//!     - `registry`: Contains the `StageRegistry`.
//!     - `requirement`: Logic for stage requirements and capabilities.
//!
//! The stage manager enables a modular and extensible approach to defining
//! application workflows, promoting separation of concerns and reusability of
//! operational units.
pub mod error;
pub mod registry;
pub mod pipeline;
pub mod context;
pub mod dry_run;
pub mod dependency;
pub mod manager;
pub mod requirement;
pub mod core_stages; // Make the new module public

// Removed: use crate::kernel::error::Result;
use std::fmt;
use async_trait::async_trait; // Import async_trait

/// Core trait that all stages must implement
#[async_trait] // Apply async_trait
pub trait Stage: Send + Sync {
    /// The unique identifier of the stage
    fn id(&self) -> &str;
    
    /// The human-readable name of the stage
    fn name(&self) -> &str;
    
    /// The description of what this stage does
    fn description(&self) -> &str;
    
    /// Whether this stage supports dry run mode
    fn supports_dry_run(&self) -> bool {
        true // Most stages should support dry run by default
    }
    
    /// Execute the stage with the given context
    async fn execute(&self, context: &mut context::StageContext) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;
    
    /// Generate a description of what this stage would do in dry run mode
    fn dry_run_description(&self, _context: &context::StageContext) -> String {
        format!("Would execute stage: {}", self.name())
    }
}

/// Result of a stage execution
#[derive(Clone, Debug)]
pub enum StageResult {
    /// Stage executed successfully
    Success,
    /// Stage failed with error
    Failure(String),
    /// Stage was skipped
    Skipped(String),
}

impl fmt::Display for StageResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StageResult::Success => write!(f, "Success"),
            StageResult::Failure(msg) => write!(f, "Failure: {}", msg),
            StageResult::Skipped(reason) => write!(f, "Skipped: {}", reason),
        }
    }
}

// Re-export important types
pub use context::StageContext;
pub use requirement::StageRequirement;
pub use registry::StageRegistry;
pub use pipeline::StagePipeline;
pub use manager::StageManager;

// Test module declaration
#[cfg(test)]
mod tests;