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