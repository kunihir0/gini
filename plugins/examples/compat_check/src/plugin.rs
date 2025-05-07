use gini_core::plugin_system::{
    Plugin, PluginDependency, PluginPriority, traits::PluginError, version::VersionRange,
};
use gini_core::stage_manager::{StageContext, requirement::StageRequirement}; // Removed unused Stage
use gini_core::stage_manager::registry::StageRegistry; // Added
use gini_core::kernel::bootstrap::Application;
use gini_core::kernel::Result as KernelResult; // Removed unused KernelError import
use async_trait::async_trait;
 // Keep for potential other uses, or remove if truly unused later

// Define a key for the context data
const COMPAT_CHECK_CONTEXT_KEY: &str = "compat_check_value";

struct CompatCheckPlugin;

#[async_trait]
impl Plugin for CompatCheckPlugin {
    fn name(&self) -> &'static str {
        "CompatCheckExample"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn is_core(&self) -> bool {
        false // This is an example plugin, not core
    }

    fn priority(&self) -> PluginPriority {
        PluginPriority::ThirdParty(151) // Default third-party priority
    }

    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        // Example: Compatible with API version 0.1.x
        vec![VersionRange::from_constraint("~0.1.0").expect("Invalid version range constraint")]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![] // No dependencies for this simple example
    }

    fn required_stages(&self) -> Vec<StageRequirement> {
        vec![] // No specific stage requirements for this example
    }

    fn init(&self, _app: &mut Application) -> KernelResult<()> {
        // No complex initialization needed for this example
        println!("CompatCheckPlugin initialized (placeholder).");
        Ok(())
    }

    async fn preflight_check(&self, context: &StageContext) -> Result<(), PluginError> {
        println!("Running preflight check for CompatCheckPlugin...");

        // Get the check value from the context
        let check_value = context.get_data::<String>(COMPAT_CHECK_CONTEXT_KEY);

        match check_value {
            Some(val) if val == "1" => {
                println!("Preflight check passed (Context Key '{}'='1').", COMPAT_CHECK_CONTEXT_KEY);
                Ok(())
            }
            Some(val) => {
                 let err_msg = format!(
                    "Preflight check failed: Context Key '{}' has incorrect value '{}' (expected '1').",
                    COMPAT_CHECK_CONTEXT_KEY, val
                );
                println!("{}", err_msg);
                Err(PluginError::PreflightCheckError(err_msg))
            }
            None => {
                 let err_msg = format!(
                    "Preflight check failed: Context Key '{}' not found.",
                    COMPAT_CHECK_CONTEXT_KEY
                );
                println!("{}", err_msg);
                Err(PluginError::PreflightCheckError(err_msg))
            }
        }
    }

    fn shutdown(&self) -> KernelResult<()> {
        // No complex shutdown needed for this example
        println!("CompatCheckPlugin shut down (placeholder).");
        Ok(())
    }

    // Add default implementations for new trait methods
    fn conflicts_with(&self) -> Vec<String> {
        vec![] // Default: no conflicts
    }

    fn incompatible_with(&self) -> Vec<PluginDependency> {
        vec![] // Default: no incompatibilities
    }

    // Add register_stages implementation
    fn register_stages(&self, _registry: &mut StageRegistry) -> KernelResult<()> {
        println!("CompatCheckPlugin provides no stages to register.");
        Ok(())
    }
}

use gini_core::plugin_system::traits::{
    FfiResult, FfiSlice, FfiVersionRange, FfiPluginDependency,
    FfiStageRequirement, PluginVTable, FfiPluginPriority,
};
use std::os::raw::{c_char, c_void};
use std::ffi::CString;
use std::ptr;
use std::panic;

// --- Static VTable Functions ---

// Note: Functions themselves are not unsafe, but their bodies might contain unsafe blocks.
// The signatures must match the VTable definition (safe extern "C").
extern "C" fn ffi_destroy(instance: *mut c_void) {
    if !instance.is_null() {
        // Reconstruct the box and let it drop, freeing the memory (unsafe operation)
        let _ = unsafe { Box::from_raw(instance as *mut CompatCheckPlugin) };
        println!("CompatCheckPlugin instance destroyed.");
    }
}

// Allocate and return a C string pointer. Host must free it.
extern "C" fn ffi_get_name(instance: *const c_void) -> *const c_char {
    let plugin = unsafe { &*(instance as *const CompatCheckPlugin) };
    match CString::new(plugin.name()) {
        Ok(s) => s.into_raw(), // Transfer ownership to host
        Err(_) => ptr::null(),
    }
}

// Function for the host to free the name string
extern "C" fn ffi_free_name(name_ptr: *mut c_char) {
    if !name_ptr.is_null() {
        unsafe {
            // Retake ownership and drop the CString
            let _ = CString::from_raw(name_ptr);
        }
    }
}

// Allocate and return a C string pointer. Host must free it.
extern "C" fn ffi_get_version(instance: *const c_void) -> *const c_char {
    let plugin = unsafe { &*(instance as *const CompatCheckPlugin) };
    match CString::new(plugin.version()) {
        Ok(s) => s.into_raw(), // Transfer ownership to host
        Err(_) => ptr::null(),
    }
}

// Function for the host to free the version string
extern "C" fn ffi_free_version(version_ptr: *mut c_char) {
    if !version_ptr.is_null() {
        unsafe {
            // Retake ownership and drop the CString
            let _ = CString::from_raw(version_ptr);
        }
    }
}

extern "C" fn ffi_is_core(instance: *const c_void) -> bool {
    // Dereferencing the raw pointer is unsafe
    let plugin = unsafe { &*(instance as *const CompatCheckPlugin) };
    plugin.is_core()
}

extern "C" fn ffi_get_priority(instance: *const c_void) -> FfiPluginPriority {
    // Dereferencing the raw pointer is unsafe
    let plugin = unsafe { &*(instance as *const CompatCheckPlugin) };
    let priority = plugin.priority();
    // Convert PluginPriority to FfiPluginPriority
    match priority {
        PluginPriority::Kernel(v) => FfiPluginPriority { category: 0, value: v },
        PluginPriority::CoreCritical(v) => FfiPluginPriority { category: 1, value: v },
        PluginPriority::Core(v) => FfiPluginPriority { category: 2, value: v },
        PluginPriority::ThirdPartyHigh(v) => FfiPluginPriority { category: 3, value: v },
        PluginPriority::ThirdParty(v) => FfiPluginPriority { category: 4, value: v },
        PluginPriority::ThirdPartyLow(v) => FfiPluginPriority { category: 5, value: v },
    }
}

// --- Slice Functions ---

// Compatible API Versions
extern "C" fn ffi_get_compatible_api_versions(instance: *const c_void) -> FfiSlice<FfiVersionRange> {
    let plugin = unsafe { &*(instance as *const CompatCheckPlugin) };
    let ranges = plugin.compatible_api_versions();
    let mut ffi_ranges: Vec<FfiVersionRange> = Vec::with_capacity(ranges.len());

    for range in ranges {
        // Convert the VersionRange constraint string to a CString
        match CString::new(range.to_string()) {
            Ok(c_str) => {
                ffi_ranges.push(FfiVersionRange {
                    constraint: c_str.into_raw(), // Transfer ownership
                });
            }
            Err(_) => {
                // Handle error: maybe log, skip, or return an error indicator?
                // For now, skip this range if conversion fails.
                eprintln!("Error converting version range constraint to CString");
            }
        }
    }

    // Convert Vec to FfiSlice, transferring ownership of the Vec's buffer
    let ptr = ffi_ranges.as_mut_ptr();
    let len = ffi_ranges.len();
    std::mem::forget(ffi_ranges); // Prevent Vec from dropping the buffer

    FfiSlice { ptr, len }
}

extern "C" fn ffi_free_compatible_api_versions(slice: FfiSlice<FfiVersionRange>) {
    if !slice.ptr.is_null() {
        unsafe {
            // Reconstruct the Vec to manage memory, casting ptr to mutable
            let ffi_ranges = Vec::from_raw_parts(slice.ptr as *mut FfiVersionRange, slice.len, slice.len);
            // Free the CString for each constraint
            for ffi_range in ffi_ranges {
                if !ffi_range.constraint.is_null() {
                    // Retake ownership of the CString and drop it
                    let _ = CString::from_raw(ffi_range.constraint as *mut c_char);
                }
            }
            // The Vec itself will be dropped here, freeing the main buffer
        }
    }
}


// Dependencies (Empty)
extern "C" fn ffi_get_empty_dependencies(_instance: *const c_void) -> FfiSlice<FfiPluginDependency> {
    FfiSlice { ptr: ptr::null(), len: 0 }
}
extern "C" fn ffi_free_empty_dependencies(_slice: FfiSlice<FfiPluginDependency>) {}

extern "C" fn ffi_get_empty_stage_requirements(_instance: *const c_void) -> FfiSlice<FfiStageRequirement> {
    FfiSlice { ptr: ptr::null(), len: 0 }
}
extern "C" fn ffi_free_empty_stage_requirements(_slice: FfiSlice<FfiStageRequirement>) {}

extern "C" fn ffi_get_empty_conflicts_with(_instance: *const c_void) -> FfiSlice<*const c_char> {
    FfiSlice { ptr: ptr::null(), len: 0 }
}
extern "C" fn ffi_free_empty_conflicts_with(_slice: FfiSlice<*const c_char>) {}

extern "C" fn ffi_get_empty_incompatible_with(_instance: *const c_void) -> FfiSlice<FfiPluginDependency> {
    FfiSlice { ptr: ptr::null(), len: 0 }
}
extern "C" fn ffi_free_empty_incompatible_with(_slice: FfiSlice<FfiPluginDependency>) {}

// --- New FFI Lifecycle Stubs ---

extern "C" fn ffi_init(instance: *mut c_void, _app_ptr: *mut c_void) -> FfiResult {
    let plugin = unsafe { &*(instance as *const CompatCheckPlugin) };
    println!("FFI: CompatCheckPlugin ('{}') ffi_init called.", plugin.name());
    // In a real plugin, cast app_ptr and interact with Application.
    // Here, we just call the Rust method for demonstration if needed,
    // but the host already calls the Rust trait method.
    // plugin.init() // This would require app_ptr to be correctly cast and passed.
    FfiResult::Ok
}

extern "C" fn ffi_preflight_check(instance: *const c_void, _context_ptr: *const c_void) -> FfiResult {
    let plugin = unsafe { &*(instance as *const CompatCheckPlugin) };
    println!("FFI: CompatCheckPlugin ('{}') ffi_preflight_check called.", plugin.name());
    // In a real plugin, cast context_ptr and perform checks.
    // The Rust trait method `preflight_check` is already called by the host.
    // This FFI function would be for plugins written purely in C/C++ etc.
    // For this example, we can simulate its logic or just return Ok.
    // Let's assume the host relies on the trait's async preflight_check.
    FfiResult::Ok
}

extern "C" fn ffi_register_stages(instance: *const c_void, _registry_ptr: *mut c_void) -> FfiResult {
    let plugin = unsafe { &*(instance as *const CompatCheckPlugin) };
    println!("FFI: CompatCheckPlugin ('{}') ffi_register_stages called.", plugin.name());
    // In a real plugin, cast registry_ptr and register stages.
    // The host already calls the Rust trait method.
    FfiResult::Ok
}

extern "C" fn ffi_shutdown(instance: *mut c_void) -> FfiResult {
    let plugin = unsafe { &*(instance as *const CompatCheckPlugin) };
    println!("FFI: CompatCheckPlugin ('{}') ffi_shutdown called.", plugin.name());
    // Perform any FFI-specific shutdown before destroy.
    FfiResult::Ok
}


/// The entry point function for the plugin loader.
#[no_mangle]
pub extern "C" fn _plugin_init() -> *mut PluginVTable {
     // Use catch_unwind to prevent panics from crossing FFI boundaries.
    let result = panic::catch_unwind(|| {
        // Create the plugin instance
        let plugin_instance = Box::new(CompatCheckPlugin);

        // Create the VTable
        let vtable = PluginVTable {
            instance: Box::into_raw(plugin_instance) as *mut c_void,
            destroy: ffi_destroy,
            name: ffi_get_name,
            free_name: ffi_free_name, // Add free function
            version: ffi_get_version,
            free_version: ffi_free_version, // Add free function
            is_core: ffi_is_core,
            priority: ffi_get_priority,
            // Use actual slice functions
            compatible_api_versions: ffi_get_compatible_api_versions,
            free_compatible_api_versions: ffi_free_compatible_api_versions,
            dependencies: ffi_get_empty_dependencies,
            free_dependencies: ffi_free_empty_dependencies,
            required_stages: ffi_get_empty_stage_requirements,
            free_required_stages: ffi_free_empty_stage_requirements,
            conflicts_with: ffi_get_empty_conflicts_with,
            free_conflicts_with: ffi_free_empty_conflicts_with,
            incompatible_with: ffi_get_empty_incompatible_with,
            free_incompatible_with: ffi_free_empty_incompatible_with,
            // New lifecycle functions
            init: ffi_init,
            preflight_check: ffi_preflight_check,
            register_stages: ffi_register_stages,
            shutdown: ffi_shutdown,
        };

        // Box the VTable and return the raw pointer
        Box::into_raw(Box::new(vtable))
    });

    match result {
        Ok(ptr) => ptr,
        Err(_) => {
            eprintln!("Panic occurred during _plugin_init of CompatCheckPlugin!");
            ptr::null_mut() // Return null if initialization panicked
        }
    }
}

// Module for tests
#[cfg(test)]
mod tests;