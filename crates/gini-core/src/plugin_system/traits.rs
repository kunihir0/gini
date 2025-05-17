use std::fmt;
use crate::plugin_system::error::PluginSystemError; // Import new error type
use crate::plugin_system::version::VersionRange;
use crate::plugin_system::dependency::PluginDependency;
use crate::stage_manager::context::StageContext; // Added for preflight context if needed later
use async_trait::async_trait;
use crate::stage_manager::registry::StageRegistry; // Added for register_stages
// Removed incorrect import: use crate::plugin_system::error::PluginError;
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
            (PluginPriority::CoreCritical(_), _) => std::cmp::Ordering::Less,
            
            (PluginPriority::Core(_), PluginPriority::Core(_)) => std::cmp::Ordering::Equal,
            (PluginPriority::Core(_), PluginPriority::CoreCritical(_)) => std::cmp::Ordering::Greater,
            (PluginPriority::Core(_), _) => std::cmp::Ordering::Less,
            
            (PluginPriority::ThirdPartyHigh(_), PluginPriority::ThirdPartyHigh(_)) => std::cmp::Ordering::Equal,
            (PluginPriority::ThirdPartyHigh(_), PluginPriority::CoreCritical(_) | PluginPriority::Core(_)) => std::cmp::Ordering::Greater,
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

// Deprecate and remove the old PluginError enum
// /// Error type for plugin operations
// #[derive(Debug)]
// pub enum PluginError {
//     InitError(String),
//     LoadError(String),
//     ExecutionError(String),
//     DependencyError(String),
//     VersionError(String),
//     PreflightCheckError(String), // Added for preflight check failures
// }
//
// impl fmt::Display for PluginError {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             PluginError::InitError(msg) => write!(f, "Plugin initialization error: {}", msg),
//             PluginError::LoadError(msg) => write!(f, "Plugin loading error: {}", msg),
//             PluginError::ExecutionError(msg) => write!(f, "Plugin execution error: {}", msg),
//             PluginError::DependencyError(msg) => write!(f, "Plugin dependency error: {}", msg),
//             PluginError::VersionError(msg) => write!(f, "Plugin version error: {}", msg),
//             PluginError::PreflightCheckError(msg) => write!(f, "Plugin pre-flight check error: {}", msg),
//         }
//     }
// }

use std::os::raw::{c_char, c_void};
use std::slice;

/// Represents the result of an FFI call.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiResult {
    /// Operation was successful.
    Ok = 0,
    /// A general error occurred.
    Err = 1,
    /// A null pointer was encountered unexpectedly.
    NullPointer = 2,
    /// Failed to decode UTF-8 string.
    Utf8Error = 3,
    /// Invalid data format or value.
    InvalidData = 4,
    // Add more specific codes as needed
}

/// FFI-safe representation of PluginPriority.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FfiPluginPriority {
    /// Category: 0: Kernel, 1: CoreCritical, 2: Core, 3: ThirdPartyHigh, 4: ThirdParty, 5: ThirdPartyLow
    pub category: u8,
    /// Value within the category.
    pub value: u8,
}

/// FFI-safe representation for returning slices (pointer + length).
/// The memory pointed to by `ptr` is managed by the plugin side
/// and must be freed using the corresponding `free_` function in the VTable.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiSlice<T> {
    pub ptr: *const T,
    pub len: usize,
}

/// FFI-safe representation of VersionRange (constraint string).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiVersionRange {
    /// Pointer to a null-terminated UTF-8 string (e.g., "^1.0", ">=2.1.0").
    /// Memory managed by the plugin, freed via `free_compatible_api_versions`.
    pub constraint: *const c_char,
}

/// FFI-safe representation of PluginDependency.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiPluginDependency {
    /// Pointer to a null-terminated UTF-8 string for the plugin name.
    /// Memory managed by the plugin, freed via `free_dependencies` or `free_incompatible_with`.
    pub plugin_name: *const c_char,
    /// Pointer to a null-terminated UTF-8 string for the version constraint.
    /// Can be null if any version is acceptable.
    /// Memory managed by the plugin, freed via `free_dependencies` or `free_incompatible_with`.
    pub version_constraint: *const c_char, // Null if no specific version required
    /// True if the dependency is required, false if optional.
    pub required: bool,
}

/// FFI-safe representation of StageRequirement.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiStageRequirement {
    /// Pointer to a null-terminated UTF-8 string for the stage ID.
    /// Memory managed by the plugin, freed via `free_required_stages`.
    pub stage_id: *const c_char,
    /// True if the stage is required, false if optional.
    pub required: bool,
    /// True if the stage is provided by the plugin.
    pub provided: bool,
}

/// The VTable struct passed across the FFI boundary.
/// Contains function pointers to access plugin metadata and manage its lifecycle.
#[repr(C)]
pub struct PluginVTable {
    /// Opaque pointer to the plugin's internal instance data.
    /// Passed as the first argument to all VTable functions.
    pub instance: *mut c_void,

    /// Destroys the plugin instance data. Called by the host when unloading the plugin.
    /// The implementation must free all resources associated with the instance.
    pub destroy: extern "C" fn(instance: *mut c_void),

    // --- Metadata Accessors ---

    /// Gets the plugin's name.
    /// Returns a pointer to a null-terminated UTF-8 string.
    /// The string data is owned by the plugin and must remain valid for the lifetime of the plugin instance.
    /// The host MUST call `free_name` on the returned pointer when done.
    pub name: extern "C" fn(instance: *const c_void) -> *const c_char,
    /// Frees the memory allocated for the string returned by `name`.
    pub free_name: extern "C" fn(name_ptr: *mut c_char),

    /// Gets the plugin's version.
    /// Returns a pointer to a null-terminated UTF-8 string.
    /// The string data is owned by the plugin and must remain valid for the lifetime of the plugin instance.
    /// The host MUST call `free_version` on the returned pointer when done.
    pub version: extern "C" fn(instance: *const c_void) -> *const c_char,
    /// Frees the memory allocated for the string returned by `version`.
    pub free_version: extern "C" fn(version_ptr: *mut c_char),

    /// Checks if the plugin is a core plugin.
    pub is_core: extern "C" fn(instance: *const c_void) -> bool,

    /// Gets the plugin's priority.
    pub priority: extern "C" fn(instance: *const c_void) -> FfiPluginPriority,

    /// Gets the compatible API versions.
    /// Returns an FfiSlice containing FfiVersionRange structs.
    /// The host MUST call `free_compatible_api_versions` on the returned slice
    /// when done to prevent memory leaks.
    pub compatible_api_versions: extern "C" fn(instance: *const c_void) -> FfiSlice<FfiVersionRange>,
    /// Frees the memory allocated for the slice returned by `compatible_api_versions`.
    pub free_compatible_api_versions: extern "C" fn(slice: FfiSlice<FfiVersionRange>),

    /// Gets the plugin's dependencies.
    /// Returns an FfiSlice containing FfiPluginDependency structs.
    /// The host MUST call `free_dependencies` on the returned slice when done.
    pub dependencies: extern "C" fn(instance: *const c_void) -> FfiSlice<FfiPluginDependency>,
    /// Frees the memory allocated for the slice returned by `dependencies`.
    pub free_dependencies: extern "C" fn(slice: FfiSlice<FfiPluginDependency>),

    /// Gets the plugin's required/provided stages.
    /// Returns an FfiSlice containing FfiStageRequirement structs.
    /// The host MUST call `free_required_stages` on the returned slice when done.
    pub required_stages: extern "C" fn(instance: *const c_void) -> FfiSlice<FfiStageRequirement>,
    /// Frees the memory allocated for the slice returned by `required_stages`.
    pub free_required_stages: extern "C" fn(slice: FfiSlice<FfiStageRequirement>),

    /// Gets the list of plugin IDs this plugin conflicts with.
    /// Returns an FfiSlice containing C strings (`*const c_char`).
    /// The host MUST call `free_conflicts_with` on the returned slice when done.
    pub conflicts_with: extern "C" fn(instance: *const c_void) -> FfiSlice<*const c_char>,
    /// Frees the memory allocated for the slice returned by `conflicts_with`.
    pub free_conflicts_with: extern "C" fn(slice: FfiSlice<*const c_char>),

    /// Gets the list of plugins/versions this plugin is incompatible with.
    /// Returns an FfiSlice containing FfiPluginDependency structs.
    /// The host MUST call `free_incompatible_with` on the returned slice when done.
    pub incompatible_with: extern "C" fn(instance: *const c_void) -> FfiSlice<FfiPluginDependency>,
    /// Frees the memory allocated for the slice returned by `incompatible_with`.
    pub free_incompatible_with: extern "C" fn(slice: FfiSlice<FfiPluginDependency>),

    // --- Lifecycle Methods ---

    /// Initializes the plugin.
    /// `app_ptr` is a raw pointer to `gini_core::kernel::bootstrap::Application`.
    /// The plugin should cast this pointer appropriately to interact with the application.
    /// Returns `FfiResult::Ok` on success, or an error code on failure.
    pub init: extern "C" fn(instance: *mut c_void, app_ptr: *mut c_void) -> FfiResult,

    /// Performs pre-flight checks for the plugin.
    /// `context_ptr` is a raw pointer to `gini_core::stage_manager::context::StageContext`.
    /// The plugin should cast this pointer to perform checks.
    /// Returns `FfiResult::Ok` if checks pass, or an error code if they fail.
    /// Note: This is a synchronous FFI call. If async operations are needed,
    /// the plugin must manage its own runtime or use a blocking approach.
    pub preflight_check: extern "C" fn(instance: *const c_void, context_ptr: *const c_void) -> FfiResult,

    /// Registers stages provided by this plugin.
    /// `registry_ptr` is a raw pointer to `gini_core::stage_manager::registry::StageRegistry`.
    /// The plugin should cast this pointer and use it to register its stages.
    /// Returns `FfiResult::Ok` on success, or an error code on failure.
    pub register_stages: extern "C" fn(instance: *const c_void, registry_ptr: *mut c_void) -> FfiResult,
    
    /// Shuts down the plugin.
    /// Returns `FfiResult::Ok` on success, or an error code on failure.
    pub shutdown: extern "C" fn(instance: *mut c_void) -> FfiResult,
}

// Helper functions `ffi_string_from_ptr` and `ffi_opt_string_from_ptr` moved to manager.rs

impl FfiPluginPriority {
    /// Converts FFI priority to the internal PluginPriority enum.
    pub fn to_plugin_priority(&self) -> Option<PluginPriority> {
        match self.category {
            0 => Some(PluginPriority::Kernel(self.value.min(10))), // Clamp to valid range
            1 => Some(PluginPriority::CoreCritical(self.value.max(11).min(50))),
            2 => Some(PluginPriority::Core(self.value.max(51).min(100))),
            3 => Some(PluginPriority::ThirdPartyHigh(self.value.max(101).min(150))),
            4 => Some(PluginPriority::ThirdParty(self.value.max(151).min(200))),
            5 => Some(PluginPriority::ThirdPartyLow(self.value.max(201))),
            _ => None, // Invalid category
        }
    }
}

// Conversion logic for FfiVersionRange, FfiPluginDependency, FfiStageRequirement
// would involve calling the unsafe string conversion helpers and then parsing/constructing
// the target Rust types (VersionRange, PluginDependency, StageRequirement).
// This requires access to those type definitions and their constructors/parsers.

// Example (conceptual - needs actual type definitions available)
// impl FfiPluginDependency {
//     pub unsafe fn to_plugin_dependency(&self) -> Result<PluginDependency, FfiResult> {
//         let name = ffi_string_from_ptr(self.plugin_name)?;
//         let version_constraint_str = ffi_opt_string_from_ptr(self.version_constraint)?;
//         let version_range = version_constraint_str
//             .map(|s| VersionRange::from_constraint(&s)) // Assuming VersionRange::from_constraint exists
//             .transpose()
//             .map_err(|_| FfiResult::InvalidData)?; // Map VersionError to FfiResult
//
//         Ok(PluginDependency {
//             plugin_name: name,
//             version_range: version_range,
//             required: self.required,
//         })
//     }
// }

// Helper for processing FfiSlice<T>
impl<T: Copy> FfiSlice<T> {
    /// Provides safe access to the slice data.
    /// Returns None if the pointer is null.
    pub unsafe fn as_slice(&self) -> Option<&[T]> { unsafe {
        if self.ptr.is_null() {
            None
        } else {
            Some(slice::from_raw_parts(self.ptr, self.len))
        }
    }}
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
    fn init(&self, app: &mut crate::kernel::bootstrap::Application) -> std::result::Result<(), PluginSystemError>;

    /// Perform pre-flight checks.
    /// This method is called during the `PluginPreflightCheck` stage.
    /// Plugins can override this to perform checks before their main initialization.
    /// The default implementation does nothing and succeeds.
    /// The `context` provides access to shared resources if needed for checks.
    async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), PluginSystemError> {
        // Default: No pre-flight check needed
        Ok(())
    }

    /// Register stages provided by this plugin with the StageRegistry.
    fn register_stages(&self, registry: &mut StageRegistry) -> std::result::Result<(), PluginSystemError>;

    /// Shutdown the plugin
    fn shutdown(&self) -> std::result::Result<(), PluginSystemError>;
}