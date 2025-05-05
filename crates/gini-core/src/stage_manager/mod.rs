pub mod registry;
pub mod pipeline;
pub mod context;
pub mod dry_run;
pub mod dependency;
pub mod manager;
pub mod requirement;
pub mod core_stages; // Make the new module public

use crate::kernel::error::Result;
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
    async fn execute(&self, context: &mut context::StageContext) -> Result<()>; // Make execute async
    
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