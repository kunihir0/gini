use std::fmt;
use crate::plugin_system::version::VersionRange;

/// Represents a dependency on another plugin
#[derive(Debug, Clone)]
pub struct PluginDependency {
    /// The name of the required plugin
    pub plugin_name: String,
    
    /// The version range that is acceptable
    pub version_range: Option<VersionRange>,
    
    /// Whether this is a hard requirement or optional dependency
    pub required: bool,
}

/// Error that can occur when resolving dependencies
#[derive(Debug)]
pub enum DependencyError {
    /// The required plugin was not found
    MissingPlugin(String),
    
    /// The plugin was found, but the version is incompatible
    IncompatibleVersion {
        plugin_name: String,
        required_range: VersionRange,
        actual_version: String,
    },
    
    /// Dependency cycle detected
    CyclicDependency(Vec<String>),
    
    /// Other dependency resolution error
    Other(String),
}

impl fmt::Display for DependencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DependencyError::MissingPlugin(name) => {
                write!(f, "Required plugin not found: {}", name)
            }
            DependencyError::IncompatibleVersion { 
                plugin_name, 
                required_range, 
                actual_version 
            } => {
                write!(
                    f, 
                    "Plugin version mismatch: {} required version {} but found {}", 
                    plugin_name, 
                    required_range.min.to_string(), 
                    actual_version
                )
            }
            DependencyError::CyclicDependency(cycle) => {
                write!(f, "Circular dependency detected: {}", cycle.join(" -> "))
            }
            DependencyError::Other(msg) => {
                write!(f, "Dependency error: {}", msg)
            }
        }
    }
}

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
    
    /// Check if this dependency is compatible with the given plugin version
    pub fn is_compatible_with(&self, version: &str) -> bool {
        if let Some(ref range) = self.version_range {
            // Parse the version
            match crate::plugin_system::version::ApiVersion::from_str(version) {
                Ok(v) => range.includes(&v),
                Err(_) => false,
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
            Some(range) => write!(
                f, 
                "{} plugin: {} (version range: {} to {})", 
                requirement_type, 
                self.plugin_name, 
                range.min.to_string(), 
                range.max.to_string()
            ),
            None => write!(f, "{} plugin: {} (any version)", requirement_type, self.plugin_name),
        }
    }
}