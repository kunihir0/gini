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
use log; // Added for logging

use crate::kernel::bootstrap::Application;
use crate::stage_manager::context::StageContext;
// Removed unused: use crate::stage_manager::Stage;
use crate::stage_manager::registry::StageRegistry; // Added for register_stages
use crate::stage_manager::requirement::StageRequirement;
use crate::plugin_system::loader::PluginLoader; // Added for PluginLoader
use crate::plugin_system::conflict::ConflictManager; // Added for ConflictManager

use crate::kernel::component::KernelComponent;
use crate::storage::config::{ConfigManager, ConfigScope};
// Removed unused StorageProvider import
use crate::kernel::error::{Error, Result as KernelResult, KernelLifecyclePhase}; // Crate's Result alias, renamed to avoid conflict
use crate::plugin_system::error::{PluginSystemError, PluginSystemErrorSource}; // Import new error types
use crate::plugin_system::traits::{
    FfiResult, FfiSlice, FfiVersionRange, FfiPluginDependency,
    FfiStageRequirement, PluginVTable,
    // PluginError, // Removed old PluginError
};
use crate::plugin_system::{Plugin, PluginManifest, ApiVersion, PluginRegistry, PluginPriority, VersionRange, PluginDependency};
use crate::kernel::constants;


const CORE_SETTINGS_CONFIG_NAME: &str = "core_settings"; // Config file name for core settings


// --- Local FFI Helper Functions ---

/// Maps FfiResult to PluginSystemError
fn map_ffi_error(ffi_err: FfiResult, plugin_id: &str, operation: &str) -> PluginSystemError {
    PluginSystemError::FfiError {
        plugin_id: plugin_id.to_string(),
        operation: operation.to_string(),
        message: format!("{:?}", ffi_err),
    }
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
    unsafe fn new(vtable_ptr: *mut PluginVTable, library: Library, plugin_path_context: Option<PathBuf>) -> KernelResult<Self> {
        if vtable_ptr.is_null() {
            drop(library);
            return Err(Error::from(PluginSystemError::LoadingError {
                plugin_id: plugin_path_context.as_ref().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| "<unknown_on_null_vtable>".to_string()),
                path: plugin_path_context,
                source: Box::new(PluginSystemErrorSource::Other(
                    "Received null VTable pointer from plugin init.".to_string(),
                )),
            }));
        }

        let vtable_ref = unsafe { &*vtable_ptr };
        let instance_const = vtable_ref.instance as *const c_void;
        println!("[GINI_FFI_DEBUG] VTablePluginWrapper::new - instance_const: {:?}", instance_const);

        let name_ptr = (vtable_ref.name)(instance_const);
        let name_string = unsafe { ffi_string_from_ptr(name_ptr) }.map_err(|e| {
            Error::from(map_ffi_error(
                e,
                plugin_path_context.as_ref().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| "<unknown_during_name_fetch>".to_string()).as_str(),
                "getting plugin name",
            ))
        })?;
        (vtable_ref.free_name)(name_ptr as *mut c_char);
        let static_name: &'static str = Box::leak(name_string.into_boxed_str());
        println!("[GINI_FFI_DEBUG] Plugin name (static_name): {}", static_name);

        let version_ptr = (vtable_ref.version)(instance_const);
        let version = unsafe { ffi_string_from_ptr(version_ptr) }
            .map_err(|e| Error::from(map_ffi_error(e, static_name, "getting plugin version")))?;
        (vtable_ref.free_version)(version_ptr as *mut c_char);
        println!("[GINI_FFI_DEBUG] VTable 'free_version' called. Plugin version: {}", version);

        let is_core = (vtable_ref.is_core)(instance_const);
        println!("[GINI_FFI_DEBUG] VTable 'is_core' returned: {}", is_core);

        let ffi_priority = (vtable_ref.priority)(instance_const);
        let priority = ffi_priority.to_plugin_priority().ok_or_else(|| {
            Error::from(PluginSystemError::LoadingError {
                plugin_id: static_name.to_string(),
                path: plugin_path_context.clone(),
                source: Box::new(PluginSystemErrorSource::Other(format!(
                    "Invalid FFI priority value received: {:?}",
                    ffi_priority
                ))),
            })
        })?;
        println!("[GINI_FFI_DEBUG] Parsed priority: {:?}", priority);

        Ok(Self {
            vtable: UnsafeVTablePtr(vtable_ptr),
            library: Some(library),
            name_cache: static_name,
            version_cache: version,
            is_core_cache: is_core,
            priority_cache: priority,
        })
    }

    unsafe fn get_vector_from_ffi_slice<T, R, F>(
        &self,
        get_slice_fn: extern "C" fn(instance: *const c_void) -> FfiSlice<T>,
        free_slice_fn: extern "C" fn(slice: FfiSlice<T>),
        converter: F,
    ) -> std::result::Result<Vec<R>, PluginSystemError>
    where
        T: Copy,
        F: Fn(T) -> std::result::Result<R, PluginSystemError>,
    {
        let vtable = unsafe { &*self.vtable.0 };
        let instance_const = vtable.instance as *const c_void;
        let ffi_slice = (get_slice_fn)(instance_const);
        
        let result = unsafe { ffi_slice.as_slice() }.map_or_else(
            || Ok(Vec::new()),
            |slice_data| {
                slice_data.iter()
                    .map(|&item| converter(item))
                    .collect::<std::result::Result<Vec<R>, PluginSystemError>>()
            },
        );
        (free_slice_fn)(ffi_slice);
        result
    }
}

impl Drop for VTablePluginWrapper {
    fn drop(&mut self) {
        println!("[GINI_FFI_DEBUG] Dropping VTablePluginWrapper for '{}'", self.name_cache);
        unsafe {
            if !self.vtable.0.is_null() {
                let vtable_ref = &*self.vtable.0;
                let instance_ptr = vtable_ref.instance;
                println!("[GINI_FFI_DEBUG] Drop: Calling FFI destroy function for plugin instance ({:?}) via VTable ({:?})", instance_ptr, self.vtable.0);
                (vtable_ref.destroy)(instance_ptr);
                println!("[GINI_FFI_DEBUG] Drop: FFI destroy function called.");

                println!("[GINI_FFI_DEBUG] Drop: Reconstructing VTable Box from raw ptr ({:?}) to free its memory...", self.vtable.0);
                let _vtable_box = Box::from_raw(self.vtable.0 as *mut PluginVTable);
                println!("[GINI_FFI_DEBUG] Drop: VTable Box reconstructed and will be dropped.");
                self.vtable.0 = std::ptr::null_mut();
            } else {
                 println!("[GINI_FFI_DEBUG] Drop: VTable pointer was null, skipping instance/VTable destruction.");
            }
            println!("[GINI_FFI_DEBUG] Drop: Dropping Library Option (unloads .so file)...");
            drop(self.library.take());
            println!("[GINI_FFI_DEBUG] Drop: Library Option dropped.");
        }
        println!("[GINI_FFI_DEBUG] Finished dropping VTablePluginWrapper for '{}'", self.name_cache);
    }
}

impl Plugin for VTablePluginWrapper {
    fn name(&self) -> &'static str {
        self.name_cache
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
                |ffi_range: FfiVersionRange| {
                    let constraint_str_ptr = ffi_range.constraint;
                    let constraint = ffi_string_from_ptr(constraint_str_ptr).map_err(|e| {
                        map_ffi_error(
                            e,
                            self.name_cache,
                            "getting compatible_api_versions constraint",
                        )
                    })?;
                    VersionRange::from_constraint(&constraint).map_err(PluginSystemError::from)
                },
            )
            .unwrap_or_else(|e| {
                eprintln!(
                    "Error getting compatible API versions for plugin '{}': {}",
                    self.name_cache, e
                );
                Vec::new()
            })
        }
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        unsafe {
            self.get_vector_from_ffi_slice(
                (*self.vtable.0).dependencies,
                (*self.vtable.0).free_dependencies,
                |ffi_dep: FfiPluginDependency| {
                    let name = ffi_string_from_ptr(ffi_dep.plugin_name).map_err(|e| {
                        map_ffi_error(e, self.name_cache, "getting dependency name")
                    })?;
                    let version_constraint_str = ffi_opt_string_from_ptr(ffi_dep.version_constraint)
                        .map_err(|e| {
                            map_ffi_error(
                                e,
                                self.name_cache,
                                "getting dependency version constraint",
                            )
                        })?;
                    let version_range = version_constraint_str
                        .map(|s| VersionRange::from_constraint(s.as_str()))
                        .transpose()
                        .map_err(PluginSystemError::from)?;

                    Ok(PluginDependency {
                        plugin_name: name,
                        version_range: version_range,
                        required: ffi_dep.required,
                    })
                },
            )
            .unwrap_or_else(|e| {
                eprintln!("Error getting dependencies for plugin '{}': {}", self.name_cache, e);
                Vec::new()
            })
        }
    }

     fn required_stages(&self) -> Vec<StageRequirement> {
        unsafe {
            self.get_vector_from_ffi_slice(
                (*self.vtable.0).required_stages,
                (*self.vtable.0).free_required_stages,
                |ffi_req: FfiStageRequirement| {
                    let id = ffi_string_from_ptr(ffi_req.stage_id).map_err(|e| {
                        map_ffi_error(e, self.name_cache, "getting required_stages id")
                    })?;
                    Ok(StageRequirement {
                        stage_id: id,
                        required: ffi_req.required,
                        provided: ffi_req.provided,
                    })
                },
            )
            .unwrap_or_else(|e| {
                eprintln!("Error getting required stages for plugin '{}': {}", self.name_cache, e);
                Vec::new()
            })
        }
    }

    fn conflicts_with(&self) -> Vec<String> {
        unsafe {
            self.get_vector_from_ffi_slice(
                (*self.vtable.0).conflicts_with,
                (*self.vtable.0).free_conflicts_with,
                |ffi_str_ptr: *const c_char| {
                    ffi_string_from_ptr(ffi_str_ptr).map_err(|e| {
                        map_ffi_error(e, self.name_cache, "getting conflicts_with string")
                    })
                },
            )
            .unwrap_or_else(|e| {
                eprintln!("Error getting conflicts_with for plugin '{}': {}", self.name_cache, e);
                Vec::new()
            })
        }
    }

    fn incompatible_with(&self) -> Vec<PluginDependency> {
        unsafe {
            self.get_vector_from_ffi_slice(
                (*self.vtable.0).incompatible_with,
                (*self.vtable.0).free_incompatible_with,
                |ffi_dep: FfiPluginDependency| {
                    let name = ffi_string_from_ptr(ffi_dep.plugin_name).map_err(|e| {
                        map_ffi_error(e, self.name_cache, "getting incompatible_with name")
                    })?;
                    let version_constraint_str =
                        ffi_opt_string_from_ptr(ffi_dep.version_constraint).map_err(|e| {
                            map_ffi_error(
                                e,
                                self.name_cache,
                                "getting incompatible_with version constraint",
                            )
                        })?;
                    let version_range = version_constraint_str
                        .map(|s| VersionRange::from_constraint(s.as_str()))
                        .transpose()
                        .map_err(PluginSystemError::from)?;

                    Ok(PluginDependency {
                        plugin_name: name,
                        version_range: version_range,
                        required: ffi_dep.required,
                    })
                },
            )
            .unwrap_or_else(|e| {
                eprintln!(
                    "Error getting incompatible_with for plugin '{}': {}",
                    self.name_cache, e
                );
                Vec::new()
            })
        }
    }

    fn init(&self, app: &mut Application) -> std::result::Result<(), PluginSystemError> {
        let vtable_ptr = self.vtable.0;
        let plugin_name_for_err = self.name_cache;

        let result = panic::catch_unwind(std::panic::AssertUnwindSafe(move || unsafe {
            let vtable = &*vtable_ptr;
            let app_ptr = app as *mut _ as *mut c_void;
            (vtable.init)(vtable.instance, app_ptr)
        }));

        match result {
            Ok(ffi_res) => {
                if ffi_res == FfiResult::Ok {
                    Ok(())
                } else {
                    Err(map_ffi_error(ffi_res, plugin_name_for_err, "init"))
                }
            }
            Err(panic_obj) => {
                let panic_msg = if let Some(s_ref) = panic_obj.downcast_ref::<&'static str>() {
                    (*s_ref).to_string()
                } else if let Some(s_obj) = panic_obj.downcast_ref::<String>() {
                    s_obj.clone()
                } else {
                    "Unknown panic reason".to_string()
                };
                eprintln!("[GINI_FFI_DEBUG] Panic in FFI 'init' for plugin '{}': {}", plugin_name_for_err, panic_msg);
                Err(PluginSystemError::FfiError {
                    plugin_id: plugin_name_for_err.to_string(),
                    operation: "init".to_string(),
                    message: format!("panic: {}", panic_msg),
                })
            }
        }
    }

    // #[must_use = "The result of preflight_check should be handled"] // Removed as per compiler warning
    fn preflight_check<'life0, 'life1, 'async_trait>(
        &'life0 self,
        context: &'life1 StageContext
    ) -> Pin<Box<dyn Future<Output = std::result::Result<(), PluginSystemError>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        let vtable_ptr = self.vtable.0;
        let _plugin_name_for_err = self.name_cache; // Prefixed as it's unused in this specific function after recent changes

        let ffi_call_result = panic::catch_unwind(std::panic::AssertUnwindSafe(move || unsafe {
            let vtable = &*vtable_ptr;
            let context_ptr = context as *const _ as *const c_void;
            (vtable.preflight_check)(vtable.instance, context_ptr)
        }));

        let plugin_name = self.name_cache;

        Box::pin(async move {
            match ffi_call_result {
                Ok(ffi_res) => {
                    if ffi_res == FfiResult::Ok {
                        Ok(())
                    } else {
                        Err(PluginSystemError::PreflightCheckFailed {
                            plugin_id: plugin_name.to_string(),
                            message: format!("FFI Error: {:?}", ffi_res),
                        })
                    }
                }
                Err(panic_obj) => {
                    let panic_msg = if let Some(s_ref) = panic_obj.downcast_ref::<&'static str>() {
                        (*s_ref).to_string()
                    } else if let Some(s_obj) = panic_obj.downcast_ref::<String>() {
                        s_obj.clone()
                    } else {
                        "Unknown panic reason".to_string()
                    };
                    eprintln!("[GINI_FFI_DEBUG] Panic in FFI 'preflight_check' for plugin '{}': {}", plugin_name, panic_msg);
                    Err(PluginSystemError::PreflightCheckFailed {
                        plugin_id: plugin_name.to_string(),
                        message: format!("FFI panic: {}", panic_msg),
                    })
                }
            }
        })
    }

    fn register_stages(&self, registry: &mut StageRegistry) -> std::result::Result<(), PluginSystemError> {
        let vtable_ptr = self.vtable.0;
        let plugin_name_for_err = self.name_cache;

        let result = panic::catch_unwind(std::panic::AssertUnwindSafe(move || unsafe {
            let vtable = &*vtable_ptr;
            let registry_ptr = registry as *mut _ as *mut c_void;
            (vtable.register_stages)(vtable.instance, registry_ptr)
        }));

        match result {
            Ok(ffi_res) => {
                if ffi_res == FfiResult::Ok {
                    Ok(())
                } else {
                    Err(map_ffi_error(ffi_res, plugin_name_for_err, "register_stages"))
                }
            }
            Err(panic_obj) => {
                let panic_msg = if let Some(s_ref) = panic_obj.downcast_ref::<&'static str>() {
                    (*s_ref).to_string()
                } else if let Some(s_obj) = panic_obj.downcast_ref::<String>() {
                    s_obj.clone()
                } else {
                    "Unknown panic reason".to_string()
                };
                eprintln!("[GINI_FFI_DEBUG] Panic in FFI 'register_stages' for plugin '{}': {}", plugin_name_for_err, panic_msg);
                Err(PluginSystemError::FfiError {
                    plugin_id: plugin_name_for_err.to_string(),
                    operation: "register_stages".to_string(),
                    message: format!("panic: {}", panic_msg),
                })
            }
        }
    }

    fn shutdown(&self) -> std::result::Result<(), PluginSystemError> {
        let vtable_ptr = self.vtable.0;
        let plugin_name_for_err = self.name_cache;

        let result = panic::catch_unwind(std::panic::AssertUnwindSafe(move || unsafe {
            let vtable = &*vtable_ptr;
            (vtable.shutdown)(vtable.instance)
        }));

        match result {
            Ok(ffi_res) => {
                if ffi_res == FfiResult::Ok {
                    println!("Plugin '{}' FFI shutdown method executed successfully.", plugin_name_for_err);
                    Ok(())
                } else {
                    eprintln!("Error during FFI shutdown for plugin '{}': {:?}", plugin_name_for_err, ffi_res);
                    Err(map_ffi_error(ffi_res, plugin_name_for_err, "shutdown"))
                }
            }
            Err(panic_obj) => {
                let panic_msg = if let Some(s_ref) = panic_obj.downcast_ref::<&'static str>() {
                    (*s_ref).to_string()
                } else if let Some(s_obj) = panic_obj.downcast_ref::<String>() {
                    s_obj.clone()
                } else {
                    "Unknown panic reason".to_string()
                };
                eprintln!("[GINI_FFI_DEBUG] Panic in FFI 'shutdown' for plugin '{}': {}", plugin_name_for_err, panic_msg);
                Err(PluginSystemError::FfiError {
                    plugin_id: plugin_name_for_err.to_string(),
                    operation: "shutdown".to_string(),
                    message: format!("panic: {}", panic_msg),
                })
            }
        }
    }
}

// --- End VTablePluginWrapper ---


const DISABLED_PLUGINS_KEY: &str = "core.plugins.disabled";


/// Plugin system component interface
#[async_trait]
pub trait PluginManager: KernelComponent {
    async fn load_plugin(&self, path: &Path) -> KernelResult<()>;
    async fn load_plugins_from_directory(&self, dir: &Path) -> KernelResult<usize>;
    async fn get_plugin(&self, id: &str) -> KernelResult<Option<Arc<dyn Plugin>>>;
    async fn get_plugins(&self) -> KernelResult<Vec<Arc<dyn Plugin>>>;
    async fn get_enabled_plugins(&self) -> KernelResult<Vec<Arc<dyn Plugin>>>;
    async fn is_plugin_loaded(&self, id: &str) -> KernelResult<bool>;
    async fn get_plugin_dependencies(&self, id: &str) -> KernelResult<Vec<String>>;
    async fn get_dependent_plugins(&self, id: &str) -> KernelResult<Vec<String>>;
    async fn is_plugin_enabled(&self, id: &str) -> KernelResult<bool>;
    async fn get_plugin_manifest(&self, id: &str) -> KernelResult<Option<PluginManifest>>;
}

/// Default implementation of plugin manager
pub struct DefaultPluginManager {
    name: &'static str,
    registry: Arc<Mutex<PluginRegistry>>,
    config_manager: Arc<ConfigManager>,
    plugin_loader: PluginLoader, // Added PluginLoader
    stage_registry_arc: Arc<Mutex<StageRegistry>>, // Added StageRegistry Arc
}

impl DefaultPluginManager {
    pub fn new(
        config_manager: Arc<ConfigManager>,
        stage_registry_arc: Arc<Mutex<StageRegistry>>,
    ) -> KernelResult<Self> {
        let api_version = ApiVersion::from_str(constants::API_VERSION)
            .map_err(|e| Error::KernelLifecycleError {
                phase: KernelLifecyclePhase::Bootstrap,
                component_name: Some("DefaultPluginManager".to_string()),
                type_id_str: None,
                message: format!("Failed to parse API_VERSION constant: {}", e),
                source: None,
            })?;
        Ok(Self {
            name: "DefaultPluginManager",
            registry: Arc::new(Mutex::new(PluginRegistry::new(api_version))),
            config_manager,
            plugin_loader: PluginLoader::new(), // Initialize PluginLoader
            stage_registry_arc, // Store StageRegistry Arc
        })
    }

    pub fn registry(&self) -> &Arc<Mutex<PluginRegistry>> {
        &self.registry
    }

    fn load_so_plugin(&self, path: &Path) -> KernelResult<Box<dyn Plugin>> {
        type PluginInitFn = unsafe extern "C" fn() -> *mut PluginVTable;

        let library = unsafe { Library::new(path) }.map_err(|e| {
            Error::from(PluginSystemError::LoadingError {
                plugin_id: path.to_string_lossy().into_owned(),
                path: Some(path.to_path_buf()),
                source: Box::new(PluginSystemErrorSource::Other(format!("Failed to load library: {}", e))),
            })
        })?;

        let init_symbol: Symbol<PluginInitFn> = unsafe { library.get(b"_plugin_init\0") }.map_err(|e| {
            Error::from(PluginSystemError::LoadingError {
                plugin_id: path.to_string_lossy().into_owned(),
                path: Some(path.to_path_buf()),
                source: Box::new(PluginSystemErrorSource::Other(format!("Failed to find _plugin_init symbol: {}", e))),
            })
        })?;
        
        println!("[GINI_FFI_DEBUG] DefaultPluginManager::load_so_plugin: Calling _plugin_init symbol for {:?}", path);
        let vtable_ptr_result = panic::catch_unwind(|| unsafe { init_symbol() });
        
        let vtable_ptr = match vtable_ptr_result {
            Ok(ptr) if !ptr.is_null() => {
                println!("[GINI_FFI_DEBUG] DefaultPluginManager::load_so_plugin: _plugin_init for {:?} returned valid ptr: {:?}", path, ptr);
                ptr
            }
            Err(e) => {
                let panic_msg = if let Some(s_ref) = e.downcast_ref::<&'static str>() {
                    (*s_ref).to_string()
                } else if let Some(s_obj) = e.downcast_ref::<String>() {
                    s_obj.clone()
                } else {
                    "Unknown panic reason".to_string()
                };
                eprintln!("[GINI_FFI_DEBUG] DefaultPluginManager::load_so_plugin: _plugin_init for {:?} panicked: {}", path, panic_msg);
                return Err(Error::from(PluginSystemError::FfiError {
                    plugin_id: path.to_string_lossy().into_owned(),
                    operation: "_plugin_init".to_string(),
                    message: format!("panic: {}", panic_msg),
                }));
            }
            Ok(ptr) if ptr.is_null() => {
                 eprintln!("[GINI_FFI_DEBUG] DefaultPluginManager::load_so_plugin: _plugin_init for {:?} returned a null pointer.", path);
                 return Err(Error::from(PluginSystemError::LoadingError {
                    plugin_id: path.to_string_lossy().into_owned(),
                    path: Some(path.to_path_buf()),
                    source: Box::new(PluginSystemErrorSource::Other(
                        "Plugin initialization function returned a null VTable pointer.".to_string()
                    )),
                }));
            }
             _ => unreachable!(),
        };
        println!("[GINI_FFI_DEBUG] DefaultPluginManager::load_so_plugin: _plugin_init symbol raw returned ptr: {:?}", vtable_ptr);

        println!("[GINI_FFI_DEBUG] DefaultPluginManager::load_so_plugin: Calling VTablePluginWrapper::new with vtable_ptr: {:?}", vtable_ptr);
        let plugin_wrapper = unsafe { VTablePluginWrapper::new(vtable_ptr, library, Some(path.to_path_buf()))? };

        Ok(Box::new(plugin_wrapper))
    }

    pub async fn persist_enable_plugin(&self, name: &str) -> KernelResult<()> {
        let mut config_data = self.config_manager.load_config(CORE_SETTINGS_CONFIG_NAME, ConfigScope::Application)?;
        let mut disabled_list: Vec<String> = config_data.get_or(DISABLED_PLUGINS_KEY, Vec::new());
        disabled_list.retain(|disabled_name| disabled_name != name);
        config_data.set(DISABLED_PLUGINS_KEY, &disabled_list)?;
        self.config_manager.save_config(CORE_SETTINGS_CONFIG_NAME, &config_data, ConfigScope::Application)?;
        println!("Persisted state: Plugin '{}' marked as enabled (removed from disabled list).", name);

        let mut registry = self.registry.lock().await;
        if registry.has_plugin(name) {
            registry.enable_plugin(name).map_err(Error::from)?;
        } else {
            println!("Attempted to enable non-existent plugin '{}' in registry (no runtime state change).", name);
        }
        Ok(())
    }

    pub async fn persist_disable_plugin(&self, name: &str) -> KernelResult<()> {
        let registry = self.registry.lock().await;
        if !registry.has_plugin(name) {
            return Err(Error::from(PluginSystemError::RegistrationError {
                plugin_id: name.to_string(),
                message: "Plugin not found".to_string(),
            }));
        }
        if let Some(plugin) = registry.get_plugin(name) {
            if plugin.is_core() {
                return Err(Error::from(PluginSystemError::OperationError {
                    plugin_id: Some(name.to_string()),
                    message: "Core plugin cannot be disabled".to_string(),
                }));
            }
        }
        drop(registry);

        let mut config_data = self.config_manager.load_config(CORE_SETTINGS_CONFIG_NAME, ConfigScope::Application)?;
        let mut disabled_list: Vec<String> = config_data.get_or(DISABLED_PLUGINS_KEY, Vec::new());
        if !disabled_list.contains(&name.to_string()) {
            disabled_list.push(name.to_string());
        }
        config_data.set(DISABLED_PLUGINS_KEY, &disabled_list)?;
        self.config_manager.save_config(CORE_SETTINGS_CONFIG_NAME, &config_data, ConfigScope::Application)?;
        println!("Persisted state: Plugin '{}' marked as disabled.", name);

        let mut registry = self.registry.lock().await;
        registry.disable_plugin(name, &self.stage_registry_arc).await.map_err(Error::from)?;
        Ok(())
    }
}

impl Debug for DefaultPluginManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultPluginManager")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl KernelComponent for DefaultPluginManager {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn initialize(&self) -> KernelResult<()> {
        println!("Initializing Plugin Manager...");

        // 1. Create a mutable PluginLoader instance (or make self mutable if appropriate)
        // For now, let's assume we might need to add dirs to it, so make it mutable.
        // However, DefaultPluginManager holds an immutable plugin_loader.
        // This suggests plugin_dirs should be configured on the loader at construction or via a method.
        // For this step, we'll assume the loader is pre-configured or we configure it here.
        let mut loader = PluginLoader::new(); // Create a new loader instance for scanning
        
        // Configure plugin directories for the loader.
        // This should ideally come from configuration or a more robust mechanism.
        // For now, using the same hardcoded path as before.
        let plugin_dir_to_scan = PathBuf::from("./target/debug");
        if plugin_dir_to_scan.exists() && plugin_dir_to_scan.is_dir() {
            loader.add_plugin_dir(&plugin_dir_to_scan);
        } else {
            println!("Plugin directory {:?} not found, no directories added to loader.", plugin_dir_to_scan);
        }
        // Add other standard plugin directories if necessary
        // loader.add_plugin_dir(PathBuf::from("/usr/lib/gini/plugins"));


        // 2. Scan for all manifests
        let all_manifests = match loader.scan_for_manifests().await {
            Ok(manifests) => {
                println!("Found {} plugin manifests.", manifests.len());
                manifests
            }
            Err(e) => {
                eprintln!("Error scanning for plugin manifests: {}", e);
                return Err(e); // Propagate error
            }
        };

        // 3. Perform conflict detection
        let mut conflict_manager = ConflictManager::new();
        if let Err(e) = conflict_manager.detect_conflicts(&all_manifests) {
            eprintln!("Error during conflict detection: {}", e);
            // Depending on policy, we might continue or halt. For now, log and continue.
        }

        let conflicts = conflict_manager.get_conflicts();
        if !conflicts.is_empty() {
            println!("Detected {} plugin conflicts:", conflicts.len());
            for conflict in conflicts {
                println!(
                    "  - Conflict between '{}' and '{}': {} (Description: {})",
                    conflict.first_plugin, conflict.second_plugin, conflict.conflict_type.description(), conflict.description
                );
            }
            // Further logic to handle critical conflicts might go here (e.g., prevent loading)
            if conflict_manager.get_critical_unresolved_conflicts().is_empty() {
                 println!("No critical unresolved conflicts detected.");
            } else {
                eprintln!("Critical unresolved conflicts detected! Review plugin configurations.");
                // Potentially return an error or prevent further loading
            }
        } else {
            println!("No plugin conflicts detected.");
        }

        // 4. Load plugins (simplified for now, actual loading logic needs refinement)
        // This part needs to be integrated with the existing plugin loading mechanism,
        // considering the manifests that are deemed loadable after conflict checks.
        // The original `load_plugins_from_directory` directly loads .so files.
        // We now have manifests, so we should iterate these and load them.

        let mut loaded_count = 0;
        let mut registry_locked = self.registry.lock().await; // Lock registry once

        for manifest in &all_manifests {
            // Check for conflicts with already registered and enabled plugins
            let mut conflict_found_and_enabled = false;
            if !manifest.conflicts_with.is_empty() { // Optimization: only iterate if there are potential conflicts
                for conflicting_plugin_name in &manifest.conflicts_with {
                    // We have registry_locked: MutexGuard<'_, PluginRegistry>
                    if registry_locked.is_registered(conflicting_plugin_name) && registry_locked.is_enabled(conflicting_plugin_name) {
                        log::warn!(
                            "Skipping manifest for plugin '{}' (from file: {:?}) due to declared conflict with already registered and enabled plugin '{}'",
                            manifest.id,
                            manifest.plugin_base_dir.join(&manifest.entry_point),
                            conflicting_plugin_name
                        );
                        conflict_found_and_enabled = true;
                        break; // Found a conflict, no need to check further for this manifest
                    }
                }
            }

            if conflict_found_and_enabled {
                continue; // Skip to the next manifest
            }

            // Check if plugin is already registered (e.g. static plugins)
            if registry_locked.is_registered(&manifest.id) {
                 println!("Plugin '{}' is already registered (likely static), skipping dynamic load.", manifest.id);
                 continue;
            }

            // Determine the path to the .so file from the manifest's entry_point and plugin_base_dir
            let entry_point_path = manifest.plugin_base_dir.join(&manifest.entry_point);
            println!("Attempting to load plugin '{}' from {:?}", manifest.id, entry_point_path);

            match self.load_so_plugin(&entry_point_path) {
                Ok(plugin_instance) => {
                    let plugin_name = plugin_instance.name().to_string();
                    match registry_locked.register_plugin(Arc::from(plugin_instance)) {
                        Ok(_) => {
                            println!("Successfully loaded and registered plugin: {}", plugin_name);
                            loaded_count += 1;
                        }
                        Err(e) => {
                            eprintln!("Failed to register plugin {}: {}", plugin_name, e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load plugin library for {}: {}", manifest.id, e);
                }
            }
        }
        drop(registry_locked); // Release lock

        println!("Loaded {} plugins after manifest scanning and conflict detection.", loaded_count);


        // Apply persisted disabled states (moved after loading all potential plugins)
        match self.config_manager.load_config(CORE_SETTINGS_CONFIG_NAME, ConfigScope::Application) {
            Ok(config_data) => {
                let disabled_list: Vec<String> = config_data.get_or(DISABLED_PLUGINS_KEY, Vec::new());
                if !disabled_list.is_empty() {
                    println!("Applying persisted disabled state for plugins: {:?}", disabled_list);
                    let mut registry = self.registry.lock().await;
                    for plugin_name in disabled_list {
                        if let Err(e) = registry.disable_plugin(&plugin_name, &self.stage_registry_arc).await {
                            eprintln!("Failed to apply persisted disable for {}: {}", plugin_name, e);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to load plugin disabled states from config: {}. Proceeding with defaults.", e);
            }
        }
        Ok(())
    }

    async fn start(&self) -> KernelResult<()> {
        println!("PluginManager component started.");
        // Dynamic plugin loading and registration occur during the PluginManager's 'initialize' phase.
        // Individual plugin `init()` methods are called by the Application via the PluginRegistry
        // after all plugins are registered and before kernel components are started.
        // No further plugin initialization actions are performed by PluginManager::start().
        Ok(())
    }

    async fn stop(&self) -> KernelResult<()> {
        println!("Stopping Plugin Manager - Shutting down initialized plugins...");
        let mut registry = self.registry.lock().await;
        registry.shutdown_all().map_err(Error::from)
    }
}

#[async_trait]
impl PluginManager for DefaultPluginManager {
    async fn load_plugin(&self, path: &Path) -> KernelResult<()> {
        println!("Attempting to load plugin from {:?}", path);
        match self.load_so_plugin(path) {
            Ok(plugin) => {
                let name = plugin.name().to_string();
                let mut registry = self.registry.lock().await;
                match registry.register_plugin(Arc::from(plugin)) {
                    Ok(_) => { println!("Successfully loaded and registered plugin: {}", name); Ok(()) }
                    Err(e) => { eprintln!("Failed to register plugin from {:?}: {}", path, e); Err(Error::from(e)) }
                }
            }
            Err(e) => { eprintln!("Failed to load plugin from {:?}: {}", path, e); Err(e) }
        }
    }

    async fn load_plugins_from_directory(&self, dir: &Path) -> KernelResult<usize> {
        println!("Scanning for plugins in directory {:?}", dir);
        let mut loaded_count = 0;
        let mut errors: Vec<PluginSystemError> = Vec::new();
        match fs::read_dir(dir) {
            Ok(entries) => {
                let mut registry = self.registry.lock().await;
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            if path.is_file() && path.extension().map_or(false, |ext| ext == "so") {
                                println!("Found potential plugin: {:?}", path);
                                let derived_plugin_name = path.file_stem()
                                    .and_then(|stem| stem.to_str())
                                    .map(|stem| stem.strip_prefix("lib").unwrap_or(stem))
                                    .map(|name_part| name_part.replace('_', "-"))
                                    .unwrap_or_else(|| path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string());

                                if !derived_plugin_name.is_empty() && registry.is_registered(&derived_plugin_name) {
                                    println!("Plugin '{}' (derived from {:?}) is already registered (likely static), skipping dynamic load.", derived_plugin_name, path);
                                    continue;
                                }

                                match self.load_so_plugin(&path) {
                                    Ok(plugin) => {
                                        let name = plugin.name().to_string();
                                        match registry.register_plugin(Arc::from(plugin)) {
                                            Ok(_) => { println!("Successfully loaded and registered plugin: {}", name); loaded_count += 1; }
                                            Err(plugin_system_err) => { // This is PluginSystemError
                                                let err_msg = format!("Failed to register plugin from {:?}: {}", path, plugin_system_err);
                                                eprintln!("{}", err_msg);
                                                errors.push(plugin_system_err);
                                            }
                                        }
                                    }
                                    Err(kernel_err) => { // This is KernelError
                                        let err_msg = format!("Failed to load plugin library {:?}: {}", path, kernel_err);
                                        eprintln!("{}", err_msg);
                                        errors.push(PluginSystemError::LoadingError{
                                            plugin_id: path.to_string_lossy().into_owned(),
                                            path: Some(path.to_path_buf()),
                                            source: Box::new(PluginSystemErrorSource::Other(kernel_err.to_string()))
                                        });
                                    }
                                }
                            }
                        }
                        Err(io_err) => {
                            let err_msg = format!("Failed to read directory entry in {:?}: {}", dir, io_err);
                            eprintln!("{}", err_msg);
                            errors.push(PluginSystemError::LoadingError{
                                plugin_id: dir.to_string_lossy().into_owned(),
                                path: Some(dir.to_path_buf()),
                                source: Box::new(PluginSystemErrorSource::Io(io_err))
                            });
                        }
                    }
                }
            }
            Err(io_err) => {
                 return Err(Error::from(PluginSystemError::LoadingError{
                    plugin_id: dir.to_string_lossy().into_owned(),
                    path: Some(dir.to_path_buf()),
                    source: Box::new(PluginSystemErrorSource::Io(io_err))
                 }));
            }
        }
        if errors.is_empty() { Ok(loaded_count) }
        else {
            let combined_errors = errors.iter().map(|e| e.to_string()).collect::<Vec<String>>().join("; ");
            Err(Error::from(PluginSystemError::LoadingError {
                plugin_id: dir.to_string_lossy().into_owned(),
                path: Some(dir.to_path_buf()),
                source: Box::new(PluginSystemErrorSource::Other(format!("Multiple errors loading plugins from directory: {}", combined_errors)))
            }))
        }
   }

   async fn get_plugin(&self, id: &str) -> KernelResult<Option<Arc<dyn Plugin>>> {
       let registry = self.registry.lock().await;
       Ok(registry.get_plugin(id))
   }

   async fn get_plugins(&self) -> KernelResult<Vec<Arc<dyn Plugin>>> {
       let registry = self.registry.lock().await;
       Ok(registry.get_plugins_arc())
   }

   async fn get_enabled_plugins(&self) -> KernelResult<Vec<Arc<dyn Plugin>>> {
       let registry = self.registry.lock().await;
       Ok(registry.get_enabled_plugins_arc())
   }

   async fn is_plugin_loaded(&self, id: &str) -> KernelResult<bool> {
       let registry = self.registry.lock().await;
       Ok(registry.has_plugin(id))
   }

   async fn get_plugin_dependencies(&self, id: &str) -> KernelResult<Vec<String>> {
       let registry = self.registry.lock().await;
       match registry.get_plugin(id) {
           Some(plugin_arc) => Ok(plugin_arc.dependencies().iter().map(|dep| dep.plugin_name.clone()).collect()),
           None => Err(Error::from(PluginSystemError::RegistrationError {
               plugin_id: id.to_string(),
               message: "Plugin not found".to_string(),
           }))
       }
   }

   async fn get_dependent_plugins(&self, id: &str) -> KernelResult<Vec<String>> {
       let registry = self.registry.lock().await;
       let mut dependents = Vec::new();
        for (plugin_id, plugin_arc) in registry.iter_plugins() {
            if plugin_arc.dependencies().iter().any(|dep| dep.plugin_name == id) {
                dependents.push(plugin_id.clone());
            }
        }
        Ok(dependents)
    }

    async fn is_plugin_enabled(&self, id: &str) -> KernelResult<bool> {
        let registry = self.registry.lock().await;
        Ok(registry.is_enabled(id))
    }

    async fn get_plugin_manifest(&self, id: &str) -> KernelResult<Option<PluginManifest>> {
        let registry = self.registry.lock().await;
        match registry.get_plugin(id) {
            Some(plugin_arc) => {
                let plugin_ref = &*plugin_arc;
                let manifest = PluginManifest {
                    id: plugin_ref.name().to_string(),
                    name: plugin_ref.name().to_string(),
                    version: plugin_ref.version().to_string(),
                    api_versions: plugin_ref.compatible_api_versions(),
                    dependencies: plugin_ref.dependencies(),
                    is_core: plugin_ref.is_core(),
                    priority: Some(plugin_ref.priority().to_string()),
                    description: format!("Manifest generated from plugin '{}'", plugin_ref.name()),
                    author: "Unknown".to_string(),
                    website: None,
                    license: None,
                    entry_point: format!("lib{}.so", plugin_ref.name()),
                    files: vec![],
                    config_schema: None,
                    tags: vec![],
                    conflicts_with: plugin_ref.conflicts_with(),
                    incompatible_with: plugin_ref.incompatible_with(),
                    resources: Vec::new(), // Add empty resources for now
                    plugin_base_dir: std::path::PathBuf::new(), // This should ideally be the actual path if known
                };
                Ok(Some(manifest))
            }
            None => Ok(None),
        }
    }
}

impl Clone for DefaultPluginManager {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            registry: Arc::clone(&self.registry),
            config_manager: Arc::clone(&self.config_manager),
            plugin_loader: self.plugin_loader.clone(), // Clone PluginLoader
            stage_registry_arc: Arc::clone(&self.stage_registry_arc), // Clone StageRegistry Arc
        }
    }
}

// Add Clone for PluginLoader if it's not already there (it's not by default)
// For now, PluginLoader doesn't have fields that prevent a simple derive.
// If PluginLoader becomes non-Clone, DefaultPluginManager cannot be Clone.
// For this exercise, we assume PluginLoader can be made Clone or this is handled.
// In loader.rs, PluginLoader is:
// pub struct PluginLoader {
//     plugin_dirs: Vec<PathBuf>,
//     manifests: HashMap<String, PluginManifest>,
// }
// This is clonable. We'll need to add `#[derive(Clone)]` to `PluginLoader` in `loader.rs`.
// This will be a separate step if `apply_diff` fails due to this.
// For now, proceeding with the assumption it will be made Clone.