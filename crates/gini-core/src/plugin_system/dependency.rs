use std::fmt;
use crate::plugin_system::version::VersionRange;
use serde::Serialize; // Added Serialize
use thiserror::Error; // Import thiserror

/// Represents a dependency on another plugin
#[derive(Debug, Clone, Serialize)] // Added Serialize
pub struct PluginDependency {
    /// The name of the required plugin
    pub plugin_name: String,
    
    /// The version range that is acceptable
    pub version_range: Option<VersionRange>,
    
    /// Whether this is a hard requirement or optional dependency
    pub required: bool,
}

/// Error that can occur when resolving dependencies
#[derive(Debug, Error)] // Add thiserror derive
pub enum DependencyError {
    /// The required plugin was not found
    #[error("Required plugin not found: {0}")]
    MissingPlugin(String),
    
    /// The plugin was found, but the version is incompatible
    #[error("Plugin version mismatch: '{plugin_name}' requires version '{required_range}' but found '{actual_version}'")]
    IncompatibleVersion {
        plugin_name: String,
        required_range: VersionRange, // VersionRange already implements Display
        actual_version: String,
    },
    
    /// Dependency cycle detected
    #[error("Circular dependency detected: {}", .0.join(" -> "))]
    CyclicDependency(Vec<String>),
    
    /// Other dependency resolution error
    #[error("Dependency error: {0}")]
    Other(String),
}

// Display is now handled by thiserror

impl PluginDependency {
    /// Create a new required dependency with a specific version range
    pub fn required(plugin_name: &str, version_range: VersionRange) -> Self {
        Self {
            plugin_name: plugin_name.to_string(),
            version_range: Some(version_range),
            required: true,
        }
    }
    
    /// Create a new required dependency with any version
    pub fn required_any(plugin_name: &str) -> Self {
        Self {
            plugin_name: plugin_name.to_string(),
            version_range: None,
            required: true,
        }
    }
    
    /// Create a new optional dependency with a specific version range
    pub fn optional(plugin_name: &str, version_range: VersionRange) -> Self {
        Self {
            plugin_name: plugin_name.to_string(),
            version_range: Some(version_range),
            required: false,
        }
    }
    
    /// Create a new optional dependency with any version
    pub fn optional_any(plugin_name: &str) -> Self {
        Self {
            plugin_name: plugin_name.to_string(),
            version_range: None,
            required: false,
        }
    }
    
    /// Check if this dependency is compatible with the given plugin version string
    pub fn is_compatible_with(&self, version_str: &str) -> bool {
        if let Some(ref range) = self.version_range {
            // Parse the provided version string into semver::Version
            match semver::Version::parse(version_str) {
                Ok(v) => range.includes(&v), // Use includes with semver::Version
                Err(_) => {
                    // If the provided version string is invalid, it's not compatible
                    eprintln!("Warning: Could not parse version string '{}' for compatibility check with plugin '{}'", version_str, self.plugin_name);
                    false
                }
            }
        } else {
            // No version range means any version is acceptable
            true
        }
    }
}

impl fmt::Display for PluginDependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let requirement_type = if self.required { "Requires" } else { "Optional" };
        match &self.version_range {
            // Use the constraint string from VersionRange for display
            Some(range) => write!(
                f,
                "{} plugin: {} (version: {})",
                requirement_type,
                self.plugin_name,
                range.constraint_string() // Use constraint_string()
            ),
            None => write!(f, "{} plugin: {} (any version)", requirement_type, self.plugin_name),
        }
    }
}