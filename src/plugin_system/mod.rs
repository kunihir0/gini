pub mod registry;
pub mod loader;
pub mod traits;
pub mod dependency;
pub mod version;
pub mod adapter;
pub mod manifest;
pub mod conflict;
pub mod manager;

pub use registry::PluginRegistry;
pub use traits::{Plugin, PluginPriority};
pub use version::{ApiVersion, VersionRange};
pub use dependency::PluginDependency;
pub use manifest::PluginManifest;
pub use manager::{PluginManager, DefaultPluginManager};