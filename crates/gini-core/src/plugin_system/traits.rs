use std::fmt;
use crate::kernel::error::Result;
use crate::plugin_system::version::VersionRange;
use crate::plugin_system::dependency::PluginDependency;
use crate::stage_manager::context::StageContext; // Added for preflight context if needed later
use async_trait::async_trait;
use crate::stage_manager::requirement::StageRequirement;

/// Priority levels for plugins
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginPriority {
    /// Reserved for kernel (0-10)
    Kernel(u8),
    /// Critical core functionality (11-50)
    CoreCritical(u8),
    /// Standard core functionality (51-100)
    Core(u8),
    /// High-priority third-party (101-150)
    ThirdPartyHigh(u8),
    /// Standard third-party (151-200)
    ThirdParty(u8),
    /// Low-priority third-party (201-255)
    ThirdPartyLow(u8),
}

impl PluginPriority {
    /// Get the numeric value of the priority
    pub fn value(&self) -> u8 {
        match self {
            PluginPriority::Kernel(val) => *val,
            PluginPriority::CoreCritical(val) => *val,
            PluginPriority::Core(val) => *val,
            PluginPriority::ThirdPartyHigh(val) => *val,
            PluginPriority::ThirdParty(val) => *val,
            PluginPriority::ThirdPartyLow(val) => *val,
        }
    }
    
    /// Parse a priority string like "core:80"
    pub fn from_str(priority_str: &str) -> Option<Self> {
        let parts: Vec<&str> = priority_str.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        
        let value = match parts[1].parse::<u8>() {
            Ok(val) => val,
            Err(_) => return None,
        };
        
        match parts[0].to_lowercase().as_str() {
            "kernel" => {
                if value > 10 {
                    return None;
                }
                Some(PluginPriority::Kernel(value))
            },
            "core_critical" | "corecritical" => {
                if value < 11 || value > 50 {
                    return None;
                }
                Some(PluginPriority::CoreCritical(value))
            },
            "core" => {
                if value < 51 || value > 100 {
                    return None;
                }
                Some(PluginPriority::Core(value))
            },
            "third_party_high" | "thirdpartyhigh" => {
                if value < 101 || value > 150 {
                    return None;
                }
                Some(PluginPriority::ThirdPartyHigh(value))
            },
            "third_party" | "thirdparty" => {
                if value < 151 || value > 200 {
                    return None;
                }
                Some(PluginPriority::ThirdParty(value))
            },
            "third_party_low" | "thirdpartylow" => {
                if value < 201 {
                    return None;
                }
                Some(PluginPriority::ThirdPartyLow(value))
            },
            _ => None,
        }
    }
}

impl fmt::Display for PluginPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginPriority::Kernel(val) => write!(f, "kernel:{}", val),
            PluginPriority::CoreCritical(val) => write!(f, "core_critical:{}", val),
            PluginPriority::Core(val) => write!(f, "core:{}", val),
            PluginPriority::ThirdPartyHigh(val) => write!(f, "third_party_high:{}", val),
            PluginPriority::ThirdParty(val) => write!(f, "third_party:{}", val),
            PluginPriority::ThirdPartyLow(val) => write!(f, "third_party_low:{}", val),
        }
    }
}

impl PartialOrd for PluginPriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PluginPriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // First compare by priority type
        let type_order = match (self, other) {
            (PluginPriority::Kernel(_), PluginPriority::Kernel(_)) => std::cmp::Ordering::Equal,
            (PluginPriority::Kernel(_), _) => std::cmp::Ordering::Less,
            (_, PluginPriority::Kernel(_)) => std::cmp::Ordering::Greater,
            
            (PluginPriority::CoreCritical(_), PluginPriority::CoreCritical(_)) => std::cmp::Ordering::Equal,
            (PluginPriority::CoreCritical(_), PluginPriority::Kernel(_)) => std::cmp::Ordering::Greater,
            (PluginPriority::CoreCritical(_), _) => std::cmp::Ordering::Less,
            
            (PluginPriority::Core(_), PluginPriority::Core(_)) => std::cmp::Ordering::Equal,
            (PluginPriority::Core(_), PluginPriority::Kernel(_) | PluginPriority::CoreCritical(_)) => std::cmp::Ordering::Greater,
            (PluginPriority::Core(_), _) => std::cmp::Ordering::Less,
            
            (PluginPriority::ThirdPartyHigh(_), PluginPriority::ThirdPartyHigh(_)) => std::cmp::Ordering::Equal,
            (PluginPriority::ThirdPartyHigh(_), PluginPriority::Kernel(_) | PluginPriority::CoreCritical(_) | PluginPriority::Core(_)) => std::cmp::Ordering::Greater,
            (PluginPriority::ThirdPartyHigh(_), _) => std::cmp::Ordering::Less,
            
            (PluginPriority::ThirdParty(_), PluginPriority::ThirdParty(_)) => std::cmp::Ordering::Equal,
            (PluginPriority::ThirdParty(_), PluginPriority::ThirdPartyLow(_)) => std::cmp::Ordering::Less,
            (PluginPriority::ThirdParty(_), _) => std::cmp::Ordering::Greater,
            
            (PluginPriority::ThirdPartyLow(_), PluginPriority::ThirdPartyLow(_)) => std::cmp::Ordering::Equal,
            (PluginPriority::ThirdPartyLow(_), _) => std::cmp::Ordering::Greater,
        };
        
        if type_order != std::cmp::Ordering::Equal {
            return type_order;
        }
        
        // If the priority type is the same, compare by value
        // Note: Lower values have higher priority (1 is higher priority than 2)
        self.value().cmp(&other.value())
    }
}

/// Error type for plugin operations
#[derive(Debug)]
pub enum PluginError {
    InitError(String),
    LoadError(String),
    ExecutionError(String),
    DependencyError(String),
    VersionError(String),
    PreflightCheckError(String), // Added for preflight check failures
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginError::InitError(msg) => write!(f, "Plugin initialization error: {}", msg),
            PluginError::LoadError(msg) => write!(f, "Plugin loading error: {}", msg),
            PluginError::ExecutionError(msg) => write!(f, "Plugin execution error: {}", msg),
            PluginError::DependencyError(msg) => write!(f, "Plugin dependency error: {}", msg),
            PluginError::VersionError(msg) => write!(f, "Plugin version error: {}", msg),
            PluginError::PreflightCheckError(msg) => write!(f, "Plugin pre-flight check error: {}", msg),
        }
    }
}

/// Core trait that all plugins must implement
#[async_trait]
pub trait Plugin: Send + Sync {
    /// The name of the plugin
    fn name(&self) -> &'static str;
    
    /// The version of the plugin
    fn version(&self) -> &str;
    
    /// Whether this is a core plugin
    fn is_core(&self) -> bool;
    
    /// The priority of the plugin
    fn priority(&self) -> PluginPriority;
    
    /// Compatible API versions
    fn compatible_api_versions(&self) -> Vec<VersionRange>;
    
    /// Plugin dependencies
    fn dependencies(&self) -> Vec<PluginDependency>;
    
    /// Stage requirements
    fn required_stages(&self) -> Vec<StageRequirement>;

    /// List of plugin IDs this plugin conflicts with (cannot run together)
    /// Typically sourced from the manifest.
    fn conflicts_with(&self) -> Vec<String>;

    /// List of plugins/versions this plugin is incompatible with.
    /// Typically sourced from the manifest.
    fn incompatible_with(&self) -> Vec<PluginDependency>; // Use PluginDependency from dependency.rs
    
    /// Initialize the plugin
    fn init(&self, app: &mut crate::kernel::bootstrap::Application) -> Result<()>;

    /// Perform pre-flight checks.
    /// This method is called during the `PluginPreflightCheck` stage.
    /// Plugins can override this to perform checks before their main initialization.
    /// The default implementation does nothing and succeeds.
    /// The `context` provides access to shared resources if needed for checks.
    async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), crate::plugin_system::traits::PluginError> {
        // Default: No pre-flight check needed
        Ok(())
    }

    /// Get the stages provided by this plugin
    fn stages(&self) -> Vec<Box<dyn crate::stage_manager::Stage>>;
    
    /// Shutdown the plugin
    fn shutdown(&self) -> Result<()>;
}