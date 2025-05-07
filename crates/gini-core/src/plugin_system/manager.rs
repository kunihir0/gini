// crates/gini-core/src/plugin_system/manager.rs
use std::fmt::Debug;
use std::future::Future; // Import Future
use std::pin::Pin; // Import Pin
use std::path::{Path, PathBuf};
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex;
use std::fs;
use libloading::{Library, Symbol};
use std::panic;
use std::ffi::{CStr, c_void};
use std::os::raw::c_char;

use crate::kernel::bootstrap::Application;
use crate::stage_manager::context::StageContext;
// Removed unused: use crate::stage_manager::Stage;
use crate::stage_manager::registry::StageRegistry; // Added for register_stages
use crate::stage_manager::requirement::StageRequirement;

use crate::kernel::component::KernelComponent;
use crate::storage::config::{ConfigManager, ConfigScope};
// Removed unused StorageProvider import
use crate::kernel::error::{Error, Result}; // Crate's Result alias
use crate::plugin_system::traits::{
    FfiResult, FfiSlice, FfiVersionRange, FfiPluginDependency,
    FfiStageRequirement, PluginVTable, PluginError,
};
use crate::plugin_system::{Plugin, PluginManifest, ApiVersion, PluginRegistry, PluginPriority, VersionRange, PluginDependency};
use crate::kernel::constants;


const CORE_SETTINGS_CONFIG_NAME: &str = "core_settings"; // Config file name for core settings


// --- Local FFI Helper Functions ---

/// Maps FfiResult to crate::kernel::error::Error
fn map_ffi_error(ffi_err: FfiResult, context: &str) -> Error {
    Error::Plugin(format!("FFI Error ({}) - {:?}: ", context, ffi_err))
}

/// Safely converts an FFI C string pointer to a Rust String.
/// # Safety
/// The caller must ensure that `ptr` is a valid pointer to a null-terminated
/// C string, and that it remains valid for the duration of this function call.
/// The string data is expected to be UTF-8 encoded.
unsafe fn ffi_string_from_ptr(ptr: *const c_char) -> std::result::Result<String, FfiResult> { unsafe { // Explicit std::result
    if ptr.is_null() {
        return Err(FfiResult::NullPointer);
    }
    CStr::from_ptr(ptr)
        .to_str()
        .map(|s| s.to_owned())
        .map_err(|_| FfiResult::Utf8Error)
}}

/// Safely converts an FFI C string pointer (which can be null) to an Option<String>.
/// # Safety
/// The caller must ensure that if `ptr` is non-null, it is a valid pointer to a
/// null-terminated C string, and that it remains valid for the duration of this
/// function call. The string data is expected to be UTF-8 encoded.
unsafe fn ffi_opt_string_from_ptr(ptr: *const c_char) -> std::result::Result<Option<String>, FfiResult> { unsafe { // Explicit std::result
    if ptr.is_null() {
        Ok(None)
    } else {
        CStr::from_ptr(ptr)
            .to_str()
            .map(|s| Some(s.to_owned()))
            .map_err(|_| FfiResult::Utf8Error)
    }
}}

// --- VTablePluginWrapper ---

#[derive(Debug, Clone, Copy)]
struct UnsafeVTablePtr(*const PluginVTable);
unsafe impl Send for UnsafeVTablePtr {}
unsafe impl Sync for UnsafeVTablePtr {}

// Wrapper needs to hold the Library to ensure it's dropped correctly
#[derive(Debug)]
struct VTablePluginWrapper {
    vtable: UnsafeVTablePtr,
    library: Option<Library>, // Store the library itself
    name_cache: &'static str, // Store as static str after leaking once
    version_cache: String,
    is_core_cache: bool,
    priority_cache: PluginPriority,
}

impl VTablePluginWrapper {
    /// Creates a new wrapper from a VTable pointer and the loaded Library.
    /// Takes ownership of the Library.
    /// # Safety
    /// The `vtable_ptr` must be a valid pointer to a `PluginVTable` allocated by the plugin
    /// (e.g., via `Box::into_raw`) and returned by its `_plugin_init` function.
    /// The `PluginVTable` and its `instance` pointer must remain valid until `destroy` is called
    /// on the VTable, which happens in `VTablePluginWrapper::drop`.
    /// The `library` argument must be the `libloading::Library` from which `vtable_ptr` was obtained.
    unsafe fn new(vtable_ptr: *mut PluginVTable, library: Library) -> Result<Self> {
        if vtable_ptr.is_null() {
            // If pointer is null, drop the library as it's unusable
            drop(library);
            return Err(Error::Plugin("Received null VTable pointer from plugin init.".to_string()));
        }

        // Dereference the pointer once safely
        // SAFETY: `vtable_ptr` is assumed valid as per function contract.
        // The VTable's functions will be called using `vtable_ref`, and its `instance`
        // pointer will be passed to them. The `library` is stored to ensure
        // function pointers remain valid.
        let vtable_ref = unsafe { &*vtable_ptr };
        let instance_const = vtable_ref.instance as *const c_void;

        // Attempt to read all necessary fields. If any fail, return Err.
        // The `library` variable will be dropped automatically in the Err case if we return early.

        // Get name, convert, and immediately free the FFI pointer
        let name_ptr = (vtable_ref.name)(instance_const);
        // SAFETY: `name_ptr` is returned by the plugin's VTable `name` function.
        // It's assumed to be a valid C string pointer as per FFI contract.
        // `ffi_string_from_ptr` handles null checks and UTF-8 conversion.
        let name_string = unsafe { ffi_string_from_ptr(name_ptr) }
            .map_err(|e| map_ffi_error(e, "getting plugin name"))?;
        // SAFETY: `name_ptr` was returned by the plugin, and `free_name` is the
        // corresponding function from the plugin to free it.
        (vtable_ref.free_name)(name_ptr as *mut c_char);

        // Leak the name string once to get &'static str
        let static_name: &'static str = Box::leak(name_string.into_boxed_str());

        // Get version, convert, and immediately free the FFI pointer
        let version_ptr = (vtable_ref.version)(instance_const);
        // SAFETY: `version_ptr` is returned by the plugin's VTable `version` function.
        // Assumed valid as per FFI contract.
        let version = unsafe { ffi_string_from_ptr(version_ptr) }
            .map_err(|e| map_ffi_error(e, "getting plugin version"))?;
        // SAFETY: `version_ptr` was returned by the plugin, and `free_version` is its deallocator.
        (vtable_ref.free_version)(version_ptr as *mut c_char);

        let is_core = (vtable_ref.is_core)(instance_const);
        let ffi_priority = (vtable_ref.priority)(instance_const); // Unsafe block removed
        let priority = ffi_priority.to_plugin_priority()
            .ok_or_else(|| Error::Plugin(format!("Invalid FFI priority value received: {:?}", ffi_priority)))?;

        // All reads succeeded, construct Self and move the library into it.
        Ok(Self {
            vtable: UnsafeVTablePtr(vtable_ptr),
            library: Some(library), // Move library here
            name_cache: static_name, // Store the leaked static str
            version_cache: version,
            is_core_cache: is_core,
            priority_cache: priority,
        })
    }

    /// Helper to safely call VTable functions that return FfiSlice<T> and convert to Vec<R>.
    unsafe fn get_vector_from_ffi_slice<T, R, F>(
        &self,
        get_slice_fn: extern "C" fn(instance: *const c_void) -> FfiSlice<T>,
        free_slice_fn: extern "C" fn(slice: FfiSlice<T>),
        converter: F,
    ) -> Result<Vec<R>> // Uses crate Result
    where
        T: Copy,
        F: Fn(T) -> Result<R>, // Converter returns crate Result
    {
        // SAFETY: This function is marked `unsafe` because it dereferences `self.vtable.0`
        // and calls FFI functions. The caller must ensure `self.vtable.0` is valid.
        // The `get_slice_fn` is expected to return a valid `FfiSlice` (pointer and length)
        // or a slice with a null pointer if empty. The memory for the slice data is owned
        // by the plugin. `FfiSlice::as_slice` handles null pointer checks.
        // The `free_slice_fn` is called to ensure the plugin deallocates the slice memory.
        let vtable = unsafe { &*self.vtable.0 };
        let instance_const = vtable.instance as *const c_void;
        let ffi_slice = (get_slice_fn)(instance_const);
        // SAFETY: `FfiSlice::as_slice` is unsafe because it dereferences the raw pointer `ffi_slice.ptr`.
        // We rely on the FFI contract that `get_slice_fn` returns a valid pointer and length.
        // The function handles null pointers internally. Needs an unsafe block per Rust 2024 rules.
        let result = unsafe { ffi_slice.as_slice() }.map_or_else(
            || Ok(Vec::new()),
            |slice_data| {
                slice_data.iter()
                    .map(|&item| converter(item))
                    .collect::<Result<Vec<R>>>() // Collect into crate Result<Vec<R>>
            },
        );
        // SAFETY: `ffi_slice` was returned by the plugin, and `free_slice_fn` is its deallocator.
        (free_slice_fn)(ffi_slice);
        result
    }
}

impl Drop for VTablePluginWrapper {
    fn drop(&mut self) {
        println!("Dropping VTablePluginWrapper for '{}'", self.name_cache);
        // SAFETY: This block handles the deallocation of resources managed via FFI.
        // 1. `self.vtable.0` points to the `PluginVTable` provided by the plugin.
        //    Its `destroy` function is called to let the plugin clean up its `instance` data.
        //    This relies on `self.vtable.0` and `vtable_ref.instance` being valid, and
        //    `vtable_ref.destroy` being a valid function pointer.
        // 2. `Box::from_raw` reclaims the memory of the `PluginVTable` itself, assuming it
        //    was originally allocated via `Box::into_raw` by the plugin.
        // 3. Dropping `self.library` unloads the dynamic library.
        // These operations must occur in this order: plugin instance cleanup, VTable memory free, then library unload.
        unsafe {
            if !self.vtable.0.is_null() {
                let vtable_ref = &*self.vtable.0;
                println!("  - Calling FFI destroy function for plugin instance...");
                (vtable_ref.destroy)(vtable_ref.instance);
                println!("  - FFI destroy function called.");

                println!("  - Reconstructing VTable Box to free its memory...");
                let _vtable_box = Box::from_raw(self.vtable.0 as *mut PluginVTable);
                println!("  - VTable Box reconstructed and will be dropped.");
                self.vtable.0 = std::ptr::null_mut(); // Prevent double free
            } else {
                 println!("  - VTable pointer was null, skipping instance/VTable destruction.");
            }

            println!("  - Dropping Library Option (unloads .so file)...");
            drop(self.library.take());
            println!("  - Library Option dropped.");
        }
         println!("Finished dropping VTablePluginWrapper for '{}'", self.name_cache);
    }
}

// Note: async_trait is applied to the Plugin trait definition, not the impl
impl Plugin for VTablePluginWrapper {
    fn name(&self) -> &'static str {
        self.name_cache // Return the cached static str directly
    }

    fn version(&self) -> &str {
        &self.version_cache
    }

    fn is_core(&self) -> bool {
        self.is_core_cache
    }

    fn priority(&self) -> PluginPriority {
        self.priority_cache.clone()
    }

    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        unsafe {
            self.get_vector_from_ffi_slice(
                (*self.vtable.0).compatible_api_versions,
                (*self.vtable.0).free_compatible_api_versions,
                |ffi_range: FfiVersionRange| { // Returns crate Result<VersionRange>
                    let constraint = ffi_string_from_ptr(ffi_range.constraint)
                        .map_err(|e| map_ffi_error(e, "getting compatible_api_versions constraint"))?;
                    VersionRange::from_constraint(&constraint)
                        .map_err(|e| Error::Plugin(format!("Failed to parse version constraint '{}': {}", constraint, e)))
                },
            )
            .unwrap_or_else(|e| {
                eprintln!("Error getting compatible API versions: {}", e);
                Vec::new()
            })
        }
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        unsafe {
            self.get_vector_from_ffi_slice(
                (*self.vtable.0).dependencies,
                (*self.vtable.0).free_dependencies,
                |ffi_dep: FfiPluginDependency| { // Returns crate Result<PluginDependency>
                    let name = ffi_string_from_ptr(ffi_dep.plugin_name)
                        .map_err(|e| map_ffi_error(e, "getting dependency name"))?;
                    let version_constraint_str = ffi_opt_string_from_ptr(ffi_dep.version_constraint)
                         .map_err(|e| map_ffi_error(e, "getting dependency version constraint"))?;
                    let version_range = version_constraint_str
                        .map(|s| VersionRange::from_constraint(s.as_str()))
                        .transpose()
                        .map_err(|e| Error::Plugin(format!("Failed to parse dependency version constraint: {}", e)))?;

                    Ok(PluginDependency {
                        plugin_name: name,
                        version_range: version_range,
                        required: ffi_dep.required,
                    })
                },
            )
            .unwrap_or_else(|e| {
                eprintln!("Error getting dependencies: {}", e);
                Vec::new()
            })
        }
    }

     fn required_stages(&self) -> Vec<StageRequirement> {
        unsafe {
            self.get_vector_from_ffi_slice(
                (*self.vtable.0).required_stages,
                (*self.vtable.0).free_required_stages,
                |ffi_req: FfiStageRequirement| { // Returns crate Result<StageRequirement>
                    let id = ffi_string_from_ptr(ffi_req.stage_id)
                        .map_err(|e| map_ffi_error(e, "getting required_stages id"))?;
                    Ok(StageRequirement {
                        stage_id: id,
                        required: ffi_req.required,
                        provided: ffi_req.provided,
                    })
                },
            )
            .unwrap_or_else(|e| {
                eprintln!("Error getting required stages: {}", e);
                Vec::new()
            })
        }
    }

    fn conflicts_with(&self) -> Vec<String> {
        unsafe {
            self.get_vector_from_ffi_slice(
                (*self.vtable.0).conflicts_with,
                (*self.vtable.0).free_conflicts_with,
                |ffi_str_ptr: *const c_char| { // Returns crate Result<String>
                    ffi_string_from_ptr(ffi_str_ptr)
                        .map_err(|e| map_ffi_error(e, "getting conflicts_with string"))
                },
            )
            .unwrap_or_else(|e| {
                eprintln!("Error getting conflicts_with: {}", e);
                Vec::new()
            })
        }
    }

    fn incompatible_with(&self) -> Vec<PluginDependency> {
        unsafe {
            self.get_vector_from_ffi_slice(
                (*self.vtable.0).incompatible_with,
                (*self.vtable.0).free_incompatible_with,
                |ffi_dep: FfiPluginDependency| { // Returns crate Result<PluginDependency>
                     let name = ffi_string_from_ptr(ffi_dep.plugin_name)
                        .map_err(|e| map_ffi_error(e, "getting incompatible_with name"))?;
                    let version_constraint_str = ffi_opt_string_from_ptr(ffi_dep.version_constraint)
                         .map_err(|e| map_ffi_error(e, "getting incompatible_with version constraint"))?;
                    let version_range = version_constraint_str
                        .map(|s| VersionRange::from_constraint(s.as_str()))
                        .transpose()
                        .map_err(|e| Error::Plugin(format!("Failed to parse incompatible_with version constraint: {}", e)))?;

                    Ok(PluginDependency {
                        plugin_name: name,
                        version_range: version_range,
                        required: ffi_dep.required,
                    })
                },
            )
            .unwrap_or_else(|e| {
                eprintln!("Error getting incompatible_with: {}", e);
                Vec::new()
            })
        }
    }

    // --- Lifecycle Methods implemented via VTable ---

    fn init(&self, app: &mut Application) -> Result<()> {
        // SAFETY: `self.vtable.0` is dereferenced to call the FFI `init` function.
        // Assumes `self.vtable.0` is valid (checked in `new`).
        // `vtable.instance` must be valid for the plugin.
        // `app` is cast to `*mut c_void`; the FFI function must handle this correctly
        // and not misuse the pointer (e.g., store it beyond the call if `Application` is not 'static).
        // The call is synchronous, so `app` remains valid.
        unsafe {
            let vtable = &*self.vtable.0;
            let app_ptr = app as *mut _ as *mut c_void;
            let result = (vtable.init)(vtable.instance, app_ptr);
            if result == FfiResult::Ok {
                Ok(())
            } else {
                Err(map_ffi_error(result, &format!("initializing plugin '{}'", self.name_cache)))
            }
        }
    }

    // Manually implement the async fn to match async_trait's expected signature
    #[must_use = "The result of preflight_check should be handled"]
    fn preflight_check<'life0, 'life1, 'async_trait>(
        &'life0 self,
        context: &'life1 StageContext
    ) -> Pin<Box<dyn Future<Output = std::result::Result<(), PluginError>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        // Perform the synchronous FFI call here, outside the async block.
        // `self` is borrowed for `'life0`, ensuring pointers are valid during this call.
        // SAFETY: Dereferences `self.vtable.0` to call FFI `preflight_check`.
        // Assumes `self.vtable.0` and `vtable.instance` are valid.
        // `context` is cast to `*const c_void`; FFI must handle correctly.
        // `context` remains valid for the synchronous FFI call.
        let ffi_call_result: FfiResult = unsafe {
            let vtable = &*self.vtable.0;
            let context_ptr = context as *const _ as *const c_void;
            (vtable.preflight_check)(vtable.instance, context_ptr)
        };

        // Capture the result and other Send-able data for the async block.
        let plugin_name = self.name_cache; // &'static str is Send

        Box::pin(async move {
            if ffi_call_result == FfiResult::Ok {
                Ok(())
            } else {
                Err(PluginError::PreflightCheckError(format!(
                    "FFI Error during preflight_check for plugin '{}': {:?}",
                    plugin_name, ffi_call_result
                )))
            }
        })
    }

    fn register_stages(&self, registry: &mut StageRegistry) -> Result<()> { // Use kernel Result alias
        // SAFETY: Dereferences `self.vtable.0` to call FFI `register_stages`.
        // Assumes `self.vtable.0` and `vtable.instance` are valid.
        // `registry` is cast to `*mut c_void`; FFI must handle correctly.
        // `registry` remains valid for the synchronous FFI call.
        unsafe {
            let vtable = &*self.vtable.0;
            let registry_ptr = registry as *mut _ as *mut c_void;
            let result = (vtable.register_stages)(vtable.instance, registry_ptr);
            if result == FfiResult::Ok {
                Ok(())
            } else {
                Err(map_ffi_error(result, &format!("registering stages for plugin '{}'", self.name_cache)))
            }
        }
    }

    fn shutdown(&self) -> Result<()> {
        // The primary shutdown logic (destroying instance, freeing VTable, unloading library)
        // is handled by the `Drop` implementation of VTablePluginWrapper.
        // This `shutdown` method in the `Plugin` trait is for any *additional*
        // graceful shutdown logic the plugin might want to perform *before* its
        // memory is reclaimed.
        // SAFETY: Dereferences `self.vtable.0` to call FFI `shutdown`.
        // Assumes `self.vtable.0` and `vtable.instance` are valid.
        unsafe {
            let vtable = &*self.vtable.0;
            let result = (vtable.shutdown)(vtable.instance);
            if result == FfiResult::Ok {
                println!("Plugin '{}' FFI shutdown method executed successfully.", self.name_cache);
                Ok(())
            } else {
                eprintln!("Error during FFI shutdown for plugin '{}': {:?}", self.name_cache, result);
                Err(map_ffi_error(result, &format!("shutting down plugin '{}'", self.name_cache)))
            }
        }
    }
}

// --- End VTablePluginWrapper ---


const DISABLED_PLUGINS_KEY: &str = "core.plugins.disabled";


/// Plugin system component interface
#[async_trait]
pub trait PluginManager: KernelComponent {
    async fn load_plugin(&self, path: &Path) -> Result<()>;
    async fn load_plugins_from_directory(&self, dir: &Path) -> Result<usize>;
    async fn get_plugin(&self, id: &str) -> Result<Option<Arc<dyn Plugin>>>;
    async fn get_plugins(&self) -> Result<Vec<Arc<dyn Plugin>>>;
    async fn get_enabled_plugins(&self) -> Result<Vec<Arc<dyn Plugin>>>;
    async fn is_plugin_loaded(&self, id: &str) -> Result<bool>;
    async fn get_plugin_dependencies(&self, id: &str) -> Result<Vec<String>>;
    async fn get_dependent_plugins(&self, id: &str) -> Result<Vec<String>>;
    async fn is_plugin_enabled(&self, id: &str) -> Result<bool>; // Note: This reflects runtime state, not persisted config
    async fn get_plugin_manifest(&self, id: &str) -> Result<Option<PluginManifest>>;
}

/// Default implementation of plugin manager
pub struct DefaultPluginManager { // Remove generic <P>
    name: &'static str,
    registry: Arc<Mutex<PluginRegistry>>,
    config_manager: Arc<ConfigManager>, // Remove generic <P>
}

impl DefaultPluginManager { // Remove generic <P>
    pub fn new(config_manager: Arc<ConfigManager>) -> Result<Self> {
        let api_version = ApiVersion::from_str(constants::API_VERSION)
            .map_err(|e| Error::Init(format!("Failed to parse API_VERSION constant: {}", e)))?;
        Ok(Self {
            name: "DefaultPluginManager",
            registry: Arc::new(Mutex::new(PluginRegistry::new(api_version))),
            config_manager,
        })
    }

    pub fn registry(&self) -> &Arc<Mutex<PluginRegistry>> {
        &self.registry
    }

    /// Safely loads a single .so plugin file using the VTable approach.
    fn load_so_plugin(&self, path: &Path) -> Result<Box<dyn Plugin>> { // Uses crate Result
        type PluginInitFn = unsafe extern "C" fn() -> *mut PluginVTable;

        // SAFETY: `Library::new` is unsafe as it loads foreign code.
        // The path must point to a valid shared library compatible with the host.
        let library = unsafe { Library::new(path) }
            .map_err(|e| Error::Plugin(format!("Failed to load library {:?}: {}", path, e)))?;

        // SAFETY: `library.get` is unsafe as it retrieves a symbol by name.
        // `_plugin_init` is the agreed-upon symbol name for the plugin's entry point.
        // The symbol must have the `PluginInitFn` signature.
        let init_symbol: Symbol<PluginInitFn> = unsafe { library.get(b"_plugin_init\0") }
            .map_err(|e| Error::Plugin(format!("Failed to find _plugin_init symbol in {:?}: {}", path, e)))?;

        // SAFETY: Calling the FFI function `init_symbol()` is unsafe.
        // It's wrapped in `panic::catch_unwind` to handle panics within the FFI call.
        // The FFI function is expected to return a valid `*mut PluginVTable` or null.
        let vtable_ptr = match panic::catch_unwind(|| unsafe { init_symbol() }) {
            Ok(ptr) if !ptr.is_null() => ptr,
            Err(e) => {
                let panic_msg = if let Some(s) = e.downcast_ref::<&'static str>() { *s }
                                else if let Some(s) = e.downcast_ref::<String>() { s.as_str() }
                                else { "Unknown panic reason" };
                return Err(Error::Plugin(format!("Plugin initialization panicked in {:?}: {}", path, panic_msg)));
            }
            Ok(ptr) if ptr.is_null() => {
                 return Err(Error::Plugin(format!("Plugin initialization function in {:?} returned a null pointer.", path)));
            }
             _ => unreachable!(), // Should be covered by the two Ok(ptr) cases
        };

        // Pass the library ownership to the wrapper.
        // SAFETY: `vtable_ptr` is the pointer returned by the plugin's `_plugin_init`.
        // `VTablePluginWrapper::new`'s safety contract must be upheld here.
        let plugin_wrapper = unsafe { VTablePluginWrapper::new(vtable_ptr, library)? };

        Ok(Box::new(plugin_wrapper))
    }

    /// Persists the intent to enable a plugin by removing it from the disabled list.
    /// This method is intended for CLI/external management, not runtime enabling.
    pub async fn persist_enable_plugin(&self, name: &str) -> Result<()> {
        // Load the core settings config data
        let mut config_data = self.config_manager.load_config(crate::plugin_system::manager::CORE_SETTINGS_CONFIG_NAME, ConfigScope::Application)?;

        // Get the current list of disabled plugins, defaulting to an empty Vec if not present
        let mut disabled_list: Vec<String> = config_data.get_or(DISABLED_PLUGINS_KEY, Vec::new());

        // Remove the plugin from the disabled list if it's there
        disabled_list.retain(|disabled_name| disabled_name != name);

        // Update the config data with the modified list
        config_data.set(DISABLED_PLUGINS_KEY, &disabled_list)?;

        // Save the updated config data back
        self.config_manager.save_config(crate::plugin_system::manager::CORE_SETTINGS_CONFIG_NAME, &config_data, ConfigScope::Application)?;

        println!("Persisted state: Plugin '{}' marked as enabled (removed from disabled list).", name);

        // Also update runtime state in registry
        let mut registry = self.registry.lock().await;
        if registry.has_plugin(name) {
            registry.enable_plugin(name)?; // Propagate actual errors if enabling a known plugin fails
        } else {
            // If plugin doesn't exist in registry, it's a no-op for runtime state, matching test expectation.
            println!("Attempted to enable non-existent plugin '{}' in registry (no runtime state change).", name);
        }

        Ok(())
    }

    /// Persists the intent to disable a plugin by adding it to the disabled list.
    /// This method is intended for CLI/external management, not runtime disabling.
    pub async fn persist_disable_plugin(&self, name: &str) -> Result<()> {
        // Verify plugin exists before disabling
        let registry = self.registry.lock().await;
        if !registry.has_plugin(name) {
            // Use the correct error variant
            return Err(Error::Plugin(format!("Plugin not found: {}", name)));
        }
        // Core plugins cannot be disabled via this mechanism (they are always loaded)
        if let Some(plugin) = registry.get_plugin(name) {
            if plugin.is_core() {
                return Err(Error::Plugin(format!("Core plugin '{}' cannot be disabled.", name)));
            }
        }
        drop(registry); // Release lock before async I/O

        // Load the core settings config data
        let mut config_data = self.config_manager.load_config(crate::plugin_system::manager::CORE_SETTINGS_CONFIG_NAME, ConfigScope::Application)?;

        // Get the current list of disabled plugins, defaulting to an empty Vec if not present
        let mut disabled_list: Vec<String> = config_data.get_or(DISABLED_PLUGINS_KEY, Vec::new());

        // Add the plugin to the disabled list if it's not already there
        if !disabled_list.contains(&name.to_string()) {
            disabled_list.push(name.to_string());
        }

        // Update the config data with the modified list
        config_data.set(DISABLED_PLUGINS_KEY, &disabled_list)?;

        // Save the updated config data back
        self.config_manager.save_config(crate::plugin_system::manager::CORE_SETTINGS_CONFIG_NAME, &config_data, ConfigScope::Application)?;

        println!("Persisted state: Plugin '{}' marked as disabled.", name);

        // Also update runtime state in registry
        let mut registry = self.registry.lock().await;
        registry.disable_plugin(name)?; // Propagate potential errors like "not found" or "is core"

        Ok(())
    }
}

impl Debug for DefaultPluginManager { // Remove generic <P>
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultPluginManager")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl KernelComponent for DefaultPluginManager { // Remove generic <P>
    fn name(&self) -> &'static str {
        self.name
    }

    async fn initialize(&self) -> Result<()> {
        println!("Initializing Plugin Manager...");
        // Load persisted disabled state
        match self.config_manager.load_config(CORE_SETTINGS_CONFIG_NAME, ConfigScope::Application) {
            Ok(config_data) => {
                let disabled_list: Vec<String> = config_data.get_or(DISABLED_PLUGINS_KEY, Vec::new());
                if !disabled_list.is_empty() {
                    println!("Applying persisted disabled state for plugins: {:?}", disabled_list);
                    let mut registry = self.registry.lock().await;
                    for plugin_name in disabled_list {
                        // Attempt to disable in registry, ignoring errors like "not found" or "is core"
                        // as these states might be inconsistent until full initialization.
                        // Core plugins are handled by persist_disable_plugin check anyway.
                        let _ = registry.disable_plugin(&plugin_name);
                    }
                }
            }
            Err(e) => {
                // Log error but continue initialization - perhaps config file is missing/corrupt
                eprintln!("Warning: Failed to load plugin disabled states from config: {}. Proceeding with defaults.", e);
            }
        }

        // Load external plugins from directory
        let plugin_dir = PathBuf::from("./target/debug"); // TODO: Make this configurable
        if plugin_dir.exists() && plugin_dir.is_dir() {
            match self.load_plugins_from_directory(&plugin_dir).await {
                Ok(count) => println!("Loaded {} external plugins from {:?}", count, plugin_dir),
                Err(e) => eprintln!("Error loading plugins from {:?}: {}", plugin_dir, e),
            }
        } else { println!("Plugin directory {:?} not found, skipping external plugin loading.", plugin_dir); }
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        println!("Starting Plugin Manager - Initializing enabled plugins...");
        println!("Plugin initialization skipped (needs Application reference).");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        println!("Stopping Plugin Manager - Shutting down initialized plugins...");
        let mut registry = self.registry.lock().await;
        registry.shutdown_all()
    }
}

#[async_trait]
impl PluginManager for DefaultPluginManager { // Remove generic <P>
    async fn load_plugin(&self, path: &Path) -> Result<()> {
        println!("Attempting to load plugin from {:?}", path);
        match self.load_so_plugin(path) {
            Ok(plugin) => {
                let name = plugin.name().to_string(); // Leaks memory
                let mut registry = self.registry.lock().await;
                match registry.register_plugin(plugin) {
                    Ok(_) => { println!("Successfully loaded and registered plugin: {}", name); Ok(()) }
                    Err(e) => { eprintln!("Failed to register plugin from {:?}: {}", path, e); Err(e) }
                }
            }
            Err(e) => { eprintln!("Failed to load plugin from {:?}: {}", path, e); Err(e) }
        }
    }

    async fn load_plugins_from_directory(&self, dir: &Path) -> Result<usize> {
        println!("Scanning for plugins in directory {:?}", dir);
        let mut loaded_count = 0;
        let mut errors = Vec::new();
        match fs::read_dir(dir) {
            Ok(entries) => {
                let mut registry = self.registry.lock().await;
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            if path.is_file() && path.extension().map_or(false, |ext| ext == "so") {
                                println!("Found potential plugin: {:?}", path);

                                // Derive plugin name from filename (e.g., libcore_logging.so -> core-logging)
                                let derived_plugin_name = path.file_stem()
                                    .and_then(|stem| stem.to_str()) // Get stem as &str
                                    .map(|stem| stem.strip_prefix("lib").unwrap_or(stem)) // Remove "lib" prefix
                                    .map(|name_part| name_part.replace('_', "-")) // Replace underscores
                                    .unwrap_or_else(|| path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string()); // Fallback

                                // Check if the derived plugin name is already registered
                                if !derived_plugin_name.is_empty() && registry.is_registered(&derived_plugin_name) {
                                    println!("Plugin '{}' (derived from {:?}) is already registered (likely static), skipping dynamic load.", derived_plugin_name, path);
                                    continue; // Skip to the next file
                                }

                                // Attempt to load the .so file
                                match self.load_so_plugin(&path) {
                                    Ok(plugin) => {
                                        // Note: The plugin's actual name() might differ from derived_plugin_name.
                                        // Registration uses the name() reported by the plugin itself.
                                        let name = plugin.name().to_string(); // Leaks memory
                                        match registry.register_plugin(plugin) {
                                            Ok(_) => { println!("Successfully loaded and registered plugin: {}", name); loaded_count += 1; }
                                            Err(e) => { let err_msg = format!("Failed to register plugin from {:?}: {}", path, e); eprintln!("{}", err_msg); errors.push(err_msg); }
                                        }
                                    }
                                    Err(e) => { let err_msg = format!("Failed to load plugin library {:?}: {}", path, e); eprintln!("{}", err_msg); errors.push(err_msg); }
                                }
                            }
                        }
                        Err(e) => { let err_msg = format!("Failed to read directory entry in {:?}: {}", dir, e); eprintln!("{}", err_msg); errors.push(err_msg); }
                    }
                }
            }
            Err(e) => { return Err(Error::Plugin(format!("Failed to read plugin directory {:?}: {}", dir, e))); }
        }
        if errors.is_empty() { Ok(loaded_count) }
        else { Err(Error::Plugin(format!("Encountered errors while loading plugins from {:?}: {}", dir, errors.join("; ")))) }
    }

    async fn get_plugin(&self, id: &str) -> Result<Option<Arc<dyn Plugin>>> {
        let registry = self.registry.lock().await;
        Ok(registry.get_plugin(id))
    }

    async fn get_plugins(&self) -> Result<Vec<Arc<dyn Plugin>>> {
        let registry = self.registry.lock().await;
        Ok(registry.get_plugins_arc())
    }

    async fn get_enabled_plugins(&self) -> Result<Vec<Arc<dyn Plugin>>> {
        let registry = self.registry.lock().await;
        Ok(registry.get_enabled_plugins_arc())
    }

    async fn is_plugin_loaded(&self, id: &str) -> Result<bool> {
        let registry = self.registry.lock().await;
        Ok(registry.has_plugin(id))
    }

    async fn get_plugin_dependencies(&self, id: &str) -> Result<Vec<String>> {
        let registry = self.registry.lock().await;
        match registry.get_plugin(id) {
            Some(plugin_arc) => Ok(plugin_arc.dependencies().iter().map(|dep| dep.plugin_name.clone()).collect()),
            None => Err(Error::Plugin(format!("Plugin not found: {}", id))),
        }
    }

    async fn get_dependent_plugins(&self, id: &str) -> Result<Vec<String>> {
        let registry = self.registry.lock().await;
        let mut dependents = Vec::new();
        for (plugin_id, plugin_arc) in registry.iter_plugins() {
            if plugin_arc.dependencies().iter().any(|dep| dep.plugin_name == id) {
                dependents.push(plugin_id.clone());
            }
        }
        Ok(dependents)
    }

    async fn is_plugin_enabled(&self, id: &str) -> Result<bool> {
        let registry = self.registry.lock().await;
        Ok(registry.is_enabled(id))
    }

    async fn get_plugin_manifest(&self, id: &str) -> Result<Option<PluginManifest>> {
        let registry = self.registry.lock().await;
        match registry.get_plugin(id) {
            Some(plugin_arc) => {
                let plugin_ref = &*plugin_arc;
                let manifest = PluginManifest {
                    id: plugin_ref.name().to_string(), // Leaks memory
                    name: plugin_ref.name().to_string(), // Leaks memory
                    version: plugin_ref.version().to_string(),
                    api_versions: plugin_ref.compatible_api_versions(),
                    dependencies: plugin_ref.dependencies(),
                    is_core: plugin_ref.is_core(),
                    priority: Some(plugin_ref.priority().to_string()),
                    description: format!("Manifest generated from plugin '{}'", plugin_ref.name()), // Leaks memory
                    author: "Unknown".to_string(),
                    website: None,
                    license: None,
                    entry_point: format!("lib{}.so", plugin_ref.name()), // Leaks memory
                    files: vec![],
                    config_schema: None,
                    tags: vec![],
                    conflicts_with: plugin_ref.conflicts_with(),
                    incompatible_with: plugin_ref.incompatible_with(),
                };
                Ok(Some(manifest))
            }
            None => Ok(None),
        }
    }
}

impl Clone for DefaultPluginManager { // Remove generic <P>
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            registry: Arc::clone(&self.registry),
            config_manager: Arc::clone(&self.config_manager),
        }
    }
}