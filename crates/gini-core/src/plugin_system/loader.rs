use tokio::fs; // Use tokio::fs
// Removed unused: use tokio::task::spawn_blocking;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet, VecDeque}; // Added HashSet for cycle detection later, VecDeque for Kahn's
use std::future::Future;
use std::pin::Pin;
use semver::{Version}; // Removed VersionReq as VersionRange handles it
// use thiserror::Error; // Removed unused import, derive handles it
use std::str::FromStr; // For VersionRange::from_str
use serde::Deserialize; // Import Deserialize
use serde_json; // Added serde_json import
use std::sync::Arc;
use libloading::{Library, Symbol};
use std::ffi::{CStr, c_void};
use std::os::raw::c_char;
use std::panic;

use crate::kernel::error::{Error as KernelError, Result as KernelResult}; // Renamed Error to KernelError, added KernelResult alias
use crate::plugin_system::error::PluginSystemError; // Import new error type
use crate::plugin_system::traits::{
    Plugin, FfiResult, FfiSlice, FfiVersionRange, FfiPluginDependency,
    FfiStageRequirement, PluginVTable, PluginPriority, // Removed PluginError
};
use crate::kernel::bootstrap::Application; // For Plugin::init signature
use crate::stage_manager::context::StageContext; // For Plugin::preflight_check
use crate::stage_manager::registry::StageRegistry; // For Plugin::register_stages

// Import the final manifest structs
use crate::plugin_system::manifest::{PluginManifest, ResourceClaim};
use crate::plugin_system::registry::PluginRegistry;
use crate::plugin_system::version::{ApiVersion, VersionRange};
use crate::plugin_system::dependency::{PluginDependency, DependencyError}; // Import DependencyError


// --- Intermediate structs for deserialization ---

#[derive(Deserialize, Debug)]
struct RawDependencyInfo {
    id: String,
    #[serde(default)]
    version_range: Option<String>,
    #[serde(default)]
    required: bool,
}

#[derive(Deserialize, Debug)]
struct RawPluginManifest {
    id: String,
    name: String,
    version: String,
    description: String,
    author: String,
    #[serde(default)]
    website: Option<String>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    api_versions: Vec<String>,
    #[serde(default)]
    dependencies: Vec<RawDependencyInfo>,
    #[serde(default)]
    is_core: bool,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    entry_point: Option<String>,
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    config_schema: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    resources: Option<Vec<ResourceClaim>>,
}

// --- End Intermediate structs ---


// --- FFI Helper Functions (copied from manager.rs) ---

/// Maps FfiResult to PluginSystemError
fn map_ffi_error_loader(ffi_err: FfiResult, plugin_id: &str, operation: &str) -> PluginSystemError {
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
unsafe fn ffi_string_from_ptr(ptr: *const c_char) -> std::result::Result<String, FfiResult> { // Explicit std::result
    if ptr.is_null() {
        return Err(FfiResult::NullPointer);
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(|s| s.to_owned())
        .map_err(|_| FfiResult::Utf8Error)
}

/// Safely converts an FFI C string pointer (which can be null) to an Option<String>.
/// # Safety
/// The caller must ensure that if `ptr` is non-null, it is a valid pointer to a
/// null-terminated C string, and that it remains valid for the duration of this
/// function call. The string data is expected to be UTF-8 encoded.
unsafe fn ffi_opt_string_from_ptr(ptr: *const c_char) -> std::result::Result<Option<String>, FfiResult> { // Explicit std::result
    if ptr.is_null() {
        Ok(None)
    } else {
        unsafe { CStr::from_ptr(ptr) }
            .to_str()
            .map(|s| Some(s.to_owned()))
            .map_err(|_| FfiResult::Utf8Error)
    }
}

// --- VTablePluginWrapper (copied and adapted from manager.rs) ---

#[derive(Debug, Clone, Copy)]
struct UnsafeVTablePtr(*const PluginVTable);
unsafe impl Send for UnsafeVTablePtr {}
unsafe impl Sync for UnsafeVTablePtr {}

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
    unsafe fn new(vtable_ptr: *mut PluginVTable, library: Library) -> std::result::Result<Self, PluginSystemError> {
        if vtable_ptr.is_null() {
            drop(library);
            return Err(PluginSystemError::LoadingError {
                plugin_id: "<unknown_on_null_vtable_loader>".to_string(),
                path: None,
                source: Box::new(crate::plugin_system::error::PluginSystemErrorSource::Other(
                    "Received null VTable pointer from plugin init (loader).".to_string(),
                )),
            });
        }

        let vtable_ref = unsafe { &*vtable_ptr };
        let instance_const = vtable_ref.instance as *const c_void;

        let name_ptr = (vtable_ref.name)(instance_const);
        let name_string = unsafe { ffi_string_from_ptr(name_ptr) }
            .map_err(|e| map_ffi_error_loader(e, "<unknown_loader>", "getting plugin name"))?;
        (vtable_ref.free_name)(name_ptr as *mut c_char);
        let static_name: &'static str = Box::leak(name_string.into_boxed_str());

        let version_ptr = (vtable_ref.version)(instance_const);
        let version = unsafe { ffi_string_from_ptr(version_ptr) }
            .map_err(|e| map_ffi_error_loader(e, static_name, "getting plugin version"))?;
        (vtable_ref.free_version)(version_ptr as *mut c_char);

        let is_core = (vtable_ref.is_core)(instance_const);
        let ffi_priority = (vtable_ref.priority)(instance_const);
        let priority = ffi_priority.to_plugin_priority()
            .ok_or_else(|| PluginSystemError::LoadingError {
                plugin_id: static_name.to_string(),
                path: None,
                source: Box::new(crate::plugin_system::error::PluginSystemErrorSource::Other(
                    format!("Invalid FFI priority value received: {:?}", ffi_priority)
                )),
            })?;

        Ok(Self {
            vtable: UnsafeVTablePtr(vtable_ptr),
            library: Some(library),
            name_cache: static_name,
            version_cache: version,
            is_core_cache: is_core,
            priority_cache: priority,
        })
    }

    unsafe fn get_vector_from_ffi_slice<T, R, FVal>(
        &self,
        get_slice_fn: extern "C" fn(instance: *const c_void) -> FfiSlice<T>,
        free_slice_fn: extern "C" fn(slice: FfiSlice<T>),
        converter: FVal,
    ) -> std::result::Result<Vec<R>, PluginSystemError>
    where
        T: Copy,
        FVal: Fn(T) -> std::result::Result<R, PluginSystemError>, // Converter returns PluginSystemError
    {
        let vtable = unsafe { &*self.vtable.0 };
        let instance_const = vtable.instance as *const c_void;
        let ffi_slice = (get_slice_fn)(instance_const);
        let result = unsafe { ffi_slice.as_slice() }
            .map_or_else(
                || Ok(Vec::new()),
                |slice_data| {
                    slice_data.iter()
                        .map(|&item| converter(item)) // item is T, converter returns Result<R, PluginSystemError>
                        .collect::<std::result::Result<Vec<R>, PluginSystemError>>() // Collect into Result<Vec<R>, PluginSystemError>
                },
            );
        (free_slice_fn)(ffi_slice);
        result
    }
}

impl Drop for VTablePluginWrapper {
    fn drop(&mut self) {
        unsafe {
            if !self.vtable.0.is_null() {
                let vtable_ref = &*self.vtable.0;
                (vtable_ref.destroy)(vtable_ref.instance);
                let _vtable_box = Box::from_raw(self.vtable.0 as *mut PluginVTable);
                self.vtable.0 = std::ptr::null_mut();
            }
            drop(self.library.take());
        }
    }
}

#[async_trait::async_trait]
impl Plugin for VTablePluginWrapper {
    fn name(&self) -> &'static str { self.name_cache }
    fn version(&self) -> &str { &self.version_cache }
    fn is_core(&self) -> bool { self.is_core_cache }
    fn priority(&self) -> PluginPriority { self.priority_cache.clone() }

    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        unsafe {
            self.get_vector_from_ffi_slice(
                (*self.vtable.0).compatible_api_versions,
                (*self.vtable.0).free_compatible_api_versions,
                |ffi_range: FfiVersionRange| {
                    let constraint = ffi_string_from_ptr(ffi_range.constraint)
                        .map_err(|e| map_ffi_error_loader(e, self.name_cache, "getting compatible_api_versions constraint"))?;
                    VersionRange::from_constraint(&constraint).map_err(PluginSystemError::from)
                },
            )
            .unwrap_or_else(|e| {
                eprintln!("Error getting compatible API versions for plugin '{}': {}", self.name_cache, e);
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
                    let name = ffi_string_from_ptr(ffi_dep.plugin_name)
                        .map_err(|e| map_ffi_error_loader(e, self.name_cache, "getting dependency name"))?;
                    let version_constraint_str = ffi_opt_string_from_ptr(ffi_dep.version_constraint)
                         .map_err(|e| map_ffi_error_loader(e, self.name_cache, "getting dependency version constraint"))?;
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

     fn required_stages(&self) -> Vec<crate::stage_manager::requirement::StageRequirement> {
        unsafe {
            self.get_vector_from_ffi_slice(
                (*self.vtable.0).required_stages,
                (*self.vtable.0).free_required_stages,
                |ffi_req: FfiStageRequirement| {
                    let id = ffi_string_from_ptr(ffi_req.stage_id)
                        .map_err(|e| map_ffi_error_loader(e, self.name_cache, "getting required_stages id"))?;
                    Ok(crate::stage_manager::requirement::StageRequirement {
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
                    ffi_string_from_ptr(ffi_str_ptr)
                        .map_err(|e| map_ffi_error_loader(e, self.name_cache, "getting conflicts_with string"))
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
                     let name = ffi_string_from_ptr(ffi_dep.plugin_name)
                        .map_err(|e| map_ffi_error_loader(e, self.name_cache, "getting incompatible_with name"))?;
                    let version_constraint_str = ffi_opt_string_from_ptr(ffi_dep.version_constraint)
                         .map_err(|e| map_ffi_error_loader(e, self.name_cache, "getting incompatible_with version constraint"))?;
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
                eprintln!("Error getting incompatible_with for plugin '{}': {}", self.name_cache, e);
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
                if ffi_res == FfiResult::Ok { Ok(()) }
                else { Err(map_ffi_error_loader(ffi_res, plugin_name_for_err, "init")) }
            }
            Err(panic_obj) => {
                let panic_msg = if let Some(s_ref) = panic_obj.downcast_ref::<&'static str>() { (*s_ref).to_string() }
                                else if let Some(s_obj) = panic_obj.downcast_ref::<String>() { s_obj.clone() }
                                else { "Unknown panic reason".to_string() };
                Err(PluginSystemError::FfiError{ plugin_id: plugin_name_for_err.to_string(), operation: "init".to_string(), message: format!("panic: {}", panic_msg) })
            }
        }
    }

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
        let plugin_name_for_err = self.name_cache;
        let ffi_call_result = panic::catch_unwind(std::panic::AssertUnwindSafe(move || unsafe {
            let vtable = &*vtable_ptr;
            let context_ptr = context as *const _ as *const c_void;
            (vtable.preflight_check)(vtable.instance, context_ptr)
        }));
        
        Box::pin(async move {
            match ffi_call_result {
                Ok(ffi_res) => {
                    if ffi_res == FfiResult::Ok { Ok(()) }
                    else { Err(PluginSystemError::PreflightCheckFailed{ plugin_id: plugin_name_for_err.to_string(), message: format!("FFI Error: {:?}", ffi_res) }) }
                }
                Err(panic_obj) => {
                    let panic_msg = if let Some(s_ref) = panic_obj.downcast_ref::<&'static str>() { (*s_ref).to_string() }
                                    else if let Some(s_obj) = panic_obj.downcast_ref::<String>() { s_obj.clone() }
                                    else { "Unknown panic reason".to_string() };
                    Err(PluginSystemError::PreflightCheckFailed{ plugin_id: plugin_name_for_err.to_string(), message: format!("FFI panic: {}", panic_msg) })
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
                if ffi_res == FfiResult::Ok { Ok(()) }
                else { Err(map_ffi_error_loader(ffi_res, plugin_name_for_err, "register_stages")) }
            }
            Err(panic_obj) => {
                let panic_msg = if let Some(s_ref) = panic_obj.downcast_ref::<&'static str>() { (*s_ref).to_string() }
                                else if let Some(s_obj) = panic_obj.downcast_ref::<String>() { s_obj.clone() }
                                else { "Unknown panic reason".to_string() };
                Err(PluginSystemError::FfiError{ plugin_id: plugin_name_for_err.to_string(), operation: "register_stages".to_string(), message: format!("panic: {}", panic_msg) })
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
                if ffi_res == FfiResult::Ok { Ok(()) }
                else { Err(map_ffi_error_loader(ffi_res, plugin_name_for_err, "shutdown")) }
            }
            Err(panic_obj) => {
                let panic_msg = if let Some(s_ref) = panic_obj.downcast_ref::<&'static str>() { (*s_ref).to_string() }
                                else if let Some(s_obj) = panic_obj.downcast_ref::<String>() { s_obj.clone() }
                                else { "Unknown panic reason".to_string() };
                Err(PluginSystemError::FfiError{ plugin_id: plugin_name_for_err.to_string(), operation: "shutdown".to_string(), message: format!("panic: {}", panic_msg) })
            }
        }
    }
}

// --- End VTablePluginWrapper ---


/// Loads plugins from the filesystem or other sources
#[derive(Clone)]
pub struct PluginLoader {
    /// Base directories to search for plugins
    plugin_dirs: Vec<PathBuf>,
    /// Cached plugin manifests (using the final struct)
    manifests: HashMap<String, PluginManifest>,
}

impl PluginLoader {
    // Synchronous helper function for loading .so plugin
    fn load_so_plugin_sync_helper(lib_path: &Path) -> KernelResult<Box<dyn Plugin>> {
        type PluginInitFn = unsafe extern "C-unwind" fn() -> *mut PluginVTable;
        let library = unsafe { Library::new(lib_path) }
            .map_err(|e| PluginSystemError::LoadingError {
                plugin_id: lib_path.to_string_lossy().into_owned(),
                path: Some(lib_path.to_path_buf()),
                source: Box::new(crate::plugin_system::error::PluginSystemErrorSource::Other(format!("libloading error: {}", e))),
            })?; // ? will convert PluginSystemError to KernelError
        
        let init_symbol: Symbol<PluginInitFn> = unsafe { library.get(b"_plugin_init\0") }
            .map_err(|e| PluginSystemError::LoadingError {
                plugin_id: lib_path.to_string_lossy().into_owned(),
                path: Some(lib_path.to_path_buf()),
                source: Box::new(crate::plugin_system::error::PluginSystemErrorSource::Other(format!("missing symbol _plugin_init: {}", e))),
            })?; // ? will convert PluginSystemError to KernelError
        
        // Dereference the Symbol to get the raw function pointer.
        // The function pointer type PluginInitFn (unsafe extern "C-unwind" fn()) is UnwindSafe.
        let func_to_call: PluginInitFn = *init_symbol;
        
        // The closure now captures func_to_call, which is UnwindSafe.
        // Thus, AssertUnwindSafe is not strictly needed here if the closure only captures UnwindSafe types.
        // However, the operation `func_to_call()` is unsafe, so the closure is unsafe.
        // `catch_unwind` itself can take an unsafe fn if the closure is unsafe.
        // For maximum clarity and safety, let's keep AssertUnwindSafe if there's any doubt,
        // or ensure the closure is structured such that it's clearly UnwindSafe.
        // Since `func_to_call` is `UnwindSafe`, the closure `|| unsafe { func_to_call() }` is also `UnwindSafe`.
        let vtable_ptr_result = panic::catch_unwind(|| unsafe {
            func_to_call()
        });
        
        let vtable_ptr = match vtable_ptr_result {
            Ok(ptr) if !ptr.is_null() => ptr,
            Err(e) => {
                let panic_msg = if let Some(s_ref) = e.downcast_ref::<&'static str>() { (*s_ref).to_string() }
                                else if let Some(s_obj) = e.downcast_ref::<String>() { s_obj.clone() }
                                else { "Unknown panic reason".to_string() };
                return Err(KernelError::from(PluginSystemError::FfiError{ plugin_id: lib_path.to_string_lossy().into_owned(), operation: "_plugin_init_loader".to_string(), message: format!("panic: {}", panic_msg) }));
            }
            Ok(ptr) if ptr.is_null() => {
                 return Err(KernelError::from(PluginSystemError::LoadingError{ plugin_id: lib_path.to_string_lossy().into_owned(), path: Some(lib_path.to_path_buf()), source: Box::new(crate::plugin_system::error::PluginSystemErrorSource::Other("Plugin init returned null VTable (loader)".to_string())) }));
            }
             _ => unreachable!(),
        };
        let plugin_wrapper = unsafe { VTablePluginWrapper::new(vtable_ptr, library).map_err(KernelError::from)? };
        Ok(Box::new(plugin_wrapper))
    }

    /// Create a new plugin loader
    pub fn new() -> Self {
        Self {
            plugin_dirs: Vec::new(),
            manifests: HashMap::new(),
        }
    }

    /// Add a plugin directory to search
    pub fn add_plugin_dir<P: AsRef<Path>>(&mut self, dir: P) {
        self.plugin_dirs.push(dir.as_ref().to_path_buf());
    }

    /// Scan for plugin manifests asynchronously
    pub async fn scan_for_manifests(&mut self) -> KernelResult<Vec<PluginManifest>> {
        let mut manifests = Vec::new();

        // Search each plugin directory
        for dir in &self.plugin_dirs {
             // Check directory existence asynchronously
             let dir_exists = match fs::try_exists(dir).await {
                 Ok(exists) => exists,
                 Err(e) => {
                     eprintln!("Error checking existence of plugin directory {}: {}", dir.display(), e);
                     false // Assume doesn't exist on error
                 }
             };

             if !dir_exists {
                 continue;
             }

             // Check if it's a directory asynchronously
             let metadata = match fs::metadata(dir).await {
                 Ok(meta) => meta,
                 Err(e) => {
                     eprintln!("Failed to get metadata for plugin directory {}: {}", dir.display(), e);
                     continue; // Skip this directory
                 }
             };

             if !metadata.is_dir() {
                 continue;
             }

            // Scan the directory asynchronously - use the non-recursive function that returns a boxed future
            self.scan_directory_boxed(dir.clone(), &mut manifests).await?;
        }

        // Update the cache
        for manifest_ref in &manifests { // manifest_ref is &PluginManifest
            self.manifests.insert(manifest_ref.id.clone(), manifest_ref.clone());
        }

        Ok(manifests)
    }

    /// Helper function that returns a boxed future for recursive scanning
    fn scan_directory_boxed<'a>(
        &'a self,
        dir: PathBuf,
        manifests: &'a mut Vec<PluginManifest>
    ) -> Pin<Box<dyn Future<Output = KernelResult<()>> + Send + 'a>> { // Return KernelResult and ensure Send
        Box::pin(self.scan_directory_inner(dir, manifests))
    }

    /// Inner async function that implements the directory scanning logic
    async fn scan_directory_inner(&self, dir: PathBuf, manifests: &mut Vec<PluginManifest>) -> KernelResult<()> { // Return KernelResult
        // Read directory entries asynchronously
        let mut read_dir_result = fs::read_dir(&dir).await
            .map_err(|e| KernelError::io(e, "read_dir", dir.clone()))?;

        // Process each entry asynchronously using the ReadDir directly
        while let Some(entry) = read_dir_result.next_entry().await? {
            // Get the path for this entry
            let entry_path = entry.path();

            // Check if it's a directory asynchronously
            let metadata = match fs::metadata(&entry_path).await {
                 Ok(meta) => meta,
                 Err(e) => {
                     eprintln!("Failed to get metadata for {}: {}", entry_path.display(), e);
                     continue; // Skip this entry
                 }
            };

            if metadata.is_dir() {
                // Look for manifest.json in this directory
                let manifest_path = entry_path.join("manifest.json");

                // Check existence and if it's a file asynchronously
                let manifest_exists = match fs::try_exists(&manifest_path).await {
                    Ok(exists) => exists,
                    Err(e) => {
                        eprintln!("Error checking existence of {}: {}", manifest_path.display(), e);
                        false // Assume not found on error
                    }
                };

                if manifest_exists {
                     // Double check it's a file (try_exists doesn't guarantee)
                     let manifest_meta = match fs::metadata(&manifest_path).await {
                         Ok(meta) => Some(meta),
                         Err(_) => None, // Ignore error if we can't get metadata
                     };

                     if manifest_meta.map_or(false, |m| m.is_file()) {
                        match self.load_manifest(&manifest_path).await {
                            Ok(manifest) => manifests.push(manifest),
                            Err(e) => {
                                eprintln!(
                                    "Error loading manifest from {}: {}",
                                    manifest_path.display(), e
                                );
                            }
                        }
                     }
                }

                // Recursively scan subdirectories asynchronously
                // Use the boxed version to handle recursive async calls
                if let Err(e) = self.scan_directory_boxed(entry_path.clone(), manifests).await {
                    eprintln!(
                        "Error scanning subdirectory {}: {}",
                        entry_path.display(), e
                    );
                    // Decide whether to continue or propagate the error
                }
            }
        }

        Ok(())
    }

    /// Load a plugin manifest from a file asynchronously
    async fn load_manifest<P: AsRef<Path>>(&self, path: P) -> KernelResult<PluginManifest> { // Return KernelResult
        let path_ref = path.as_ref();
        let _path_display = path_ref.display().to_string(); // For error messages, prefixed with underscore

        // Read the file content asynchronously
        let content = fs::read_to_string(path_ref).await
            .map_err(|e| KernelError::io(e, "read_manifest", path_ref.to_path_buf()))?; // Use Error::io

        // Parse the JSON content into the intermediate raw struct
        // Explicitly type the variable receiving the result of from_str
        let raw_manifest: RawPluginManifest = serde_json::from_str(&content)
            .map_err(|e| PluginSystemError::ManifestError {
                path: path_ref.to_path_buf(),
                message: format!("Failed to parse manifest JSON: {}", e),
                source: Some(Box::new(e)),
            })?; // ? will convert PluginSystemError to KernelError

        // Convert RawPluginManifest to PluginManifest, parsing versions
        let plugin_base_dir = path_ref.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
        let mut final_manifest = PluginManifest {
            id: raw_manifest.id.clone(), // Clone ID
            name: raw_manifest.name,
            version: raw_manifest.version,
            description: raw_manifest.description,
            author: raw_manifest.author,
            website: raw_manifest.website,
            license: raw_manifest.license,
            api_versions: Vec::new(), // Initialize empty, fill below
            dependencies: Vec::new(), // Initialize empty, fill below
            is_core: raw_manifest.is_core,
            priority: raw_manifest.priority,
            // Handle entry point: use provided or default
            entry_point: raw_manifest.entry_point.unwrap_or_else(|| format!("lib{}.so", raw_manifest.id)), // Assign String to String
            files: raw_manifest.files,
            config_schema: raw_manifest.config_schema,
            tags: raw_manifest.tags,
            // Add default empty values for new fields
            conflicts_with: Vec::new(),
            incompatible_with: Vec::new(),
            resources: raw_manifest.resources.unwrap_or_default(),
            plugin_base_dir,
        };

        // Parse API version strings
        for api_ver_str in raw_manifest.api_versions {
            match VersionRange::from_str(api_ver_str.as_str()) { // Use .as_str()
                Ok(vr) => final_manifest.api_versions.push(vr),
                Err(e) => return Err(PluginSystemError::ManifestError{
                    path: path_ref.to_path_buf(),
                    message: format!("Failed to parse API version range '{}': {}", api_ver_str, e),
                    source: None, // VersionError is not Error, so can't box directly
                }.into()), // Convert to KernelError
            }
        }

        // Parse dependency version strings
        for raw_dep in raw_manifest.dependencies {
            let version_range = match raw_dep.version_range {
                Some(vr_str) => { // vr_str is String
                    match VersionRange::from_str(vr_str.as_str()) { // Use .as_str()
                        Ok(vr) => Some(vr),
                        Err(e) => return Err(PluginSystemError::ManifestError{
                            path: path_ref.to_path_buf(),
                            message: format!("Failed to parse dependency version range '{}' for dep '{}': {}", vr_str, raw_dep.id, e),
                            source: None, // VersionError is not Error
                        }.into()), // Convert to KernelError
                    }
                }
                None => None,
            };
            // Use PluginDependency struct literal directly
            final_manifest.dependencies.push(crate::plugin_system::dependency::PluginDependency {
                plugin_name: raw_dep.id, // Use plugin_name field
                version_range,
                required: raw_dep.required,
            });
        }

        Ok(final_manifest)
    }

    // Removed unused load_so_plugin_sync method (it's now load_so_plugin_sync_helper)

    /// Load a specific plugin asynchronously
    pub async fn load_plugin(&self, manifest: &PluginManifest) -> KernelResult<Arc<dyn Plugin>> { // Return KernelResult
        let entry_point_name = &manifest.entry_point;
        let base_dir = manifest.plugin_base_dir.clone();
        let owned_entry_point_name = entry_point_name.clone();
        let plugin_id_clone = manifest.id.clone(); // Used for error reporting
        
        let library_path = base_dir.join(&owned_entry_point_name);

        if owned_entry_point_name.contains("..") || Path::new(&owned_entry_point_name).is_absolute() {
            return Err(PluginSystemError::LoadingError {
                plugin_id: plugin_id_clone,
                path: Some(library_path),
                source: Box::new(crate::plugin_system::error::PluginSystemErrorSource::Other(
                    format!("Invalid entry_point path '{}': must be relative and not traverse upwards.", owned_entry_point_name)
                )),
            }.into()); // Convert to KernelError
        }
        
        // Call the synchronous helper directly.
        // This is generally okay for quick operations like FFI loading that might fail fast.
        // For potentially long-running synchronous CPU-bound work, spawn_blocking is preferred.
        // The `AssertUnwindSafe` is inside `load_so_plugin_sync_helper` around the FFI call.
        match Self::load_so_plugin_sync_helper(&library_path) {
            Ok(boxed_plugin) => Ok(Arc::from(boxed_plugin)),
            Err(e) => Err(e),
        }
    }

    /// Resolves dependencies between loaded manifests, including cycle detection.
    /// Returns Ok(()) if all dependencies are met, otherwise returns a DependencyError.
    fn resolve_dependencies(&self) -> std::result::Result<(), DependencyError> {
        let manifests = &self.manifests;
        let mut visiting = HashSet::<&str>::new();
        let mut visited = HashSet::<&str>::new(); // Use &str with lifetime tied to manifests

        // Helper function for DFS-based cycle detection
        fn detect_cycle_dfs<'a>( // Add lifetime 'a
            plugin_id: &'a str, // Use &'a str
            manifests: &'a HashMap<String, PluginManifest>, // Use &'a
            visiting: &mut HashSet<&'a str>,
            visited: &mut HashSet<&'a str>,
            path: &mut Vec<&'a str>,
        ) -> std::result::Result<(), DependencyError> {
            visiting.insert(plugin_id);
            path.push(plugin_id);

            if let Some(manifest) = manifests.get(plugin_id) {
                // Access the dependencies field directly
                for dep in &manifest.dependencies { // Iterate over Vec<DependencyInfo>
                    // Only consider required dependencies for cycle detection that blocks loading
                    if !dep.required {
                        continue;
                    }
                    // Get the dependency ID as &str from the manifests map keys if possible
                    // This avoids cloning the String just for the check.
                    let dep_id_str = manifests.keys()
                        .find(|k| **k == dep.plugin_name) // Use dep.plugin_name
                        .map(|s| s.as_str());

                    if let Some(dep_id) = dep_id_str {
                        // Check if the dependency exists (basic check, full check happens later)
                        // This check is slightly redundant now but kept for clarity
                        if !manifests.contains_key(dep_id) {
                            continue;
                        }

                        if visiting.contains(dep_id) {
                            // Cycle detected! Find the start of the cycle in the path
                            let cycle_start_index = path.iter().position(|&p| p == dep_id).unwrap_or(0);
                            let cycle_path_slice = &path[cycle_start_index..];
                            return Err(DependencyError::CyclicDependency(
                                cycle_path_slice.iter().map(|s| s.to_string()).collect()
                            ));
                        }

                        if !visited.contains(dep_id) {
                            // Pass the borrowed str for the recursive call
                            detect_cycle_dfs(dep_id, manifests, visiting, visited, path)?;
                        }
                    } else {
                        // Dependency ID not found in manifests map keys, handle as missing
                        // This case should ideally be caught by the later check, but handle defensively
                        continue;
                    }
                }
            }

            path.pop(); // Backtrack: remove current node from path
            visiting.remove(plugin_id);
            visited.insert(plugin_id);
            Ok(())
        }

        // --- Start Cycle Detection ---
        for plugin_id in manifests.keys() {
            if !visited.contains(plugin_id.as_str()) {
                let mut path = Vec::new(); // Path tracker for this DFS run
                // Pass borrowed str for the initial call
                detect_cycle_dfs(plugin_id.as_str(), manifests, &mut visiting, &mut visited, &mut path)?;
            }
        }
        // --- End Cycle Detection ---

        // --- Existing Dependency Checks (Missing/Version) ---
        for (_plugin_id, manifest) in manifests { // Prefixed plugin_id with _
            // Parse the plugin's own version once
            let _plugin_version_str = &manifest.version; // Prefixed plugin_version_str with _
            // Parsing plugin's own version is not strictly needed for this check if not used further
            // let _plugin_version = Version::parse(plugin_version_str).map_err(|e| {
            //     DependencyError::Other(format!("Failed to parse version for plugin {}: {}", _plugin_id, e))
            // })?;

            // Access the dependencies field directly
            for dep in &manifest.dependencies {
                if !dep.required {
                    continue; // Skip optional dependencies for now
                }

                let dep_id = &dep.plugin_name; // Use plugin_name field

                // 1. Check if dependency exists
                let dep_manifest = manifests.get(dep_id).ok_or_else(|| DependencyError::MissingPlugin(dep_id.clone()))?;

                // 2. Check version constraint (if specified)
                if let Some(version_range) = &dep.version_range {
                    let dep_version_str = &dep_manifest.version;
                    let dep_version = Version::parse(dep_version_str).map_err(|e| {
                        DependencyError::Other(format!("Failed to parse version for dependency {}: {}", dep_id, e))
                    })?;

                    if !version_range.includes(&dep_version) {
                        return Err(DependencyError::IncompatibleVersion {
                            plugin_name: dep_id.clone(),
                            required_range: version_range.clone(),
                            actual_version: dep_version_str.clone(),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Register all compatible plugins with the registry asynchronously
    /// This now includes dependency resolution before loading.
    pub async fn register_all_plugins(&self, registry: &mut PluginRegistry, api_version: &ApiVersion) -> KernelResult<usize> { // Return KernelResult
        // --- Dependency Resolution Step ---
        if let Err(e) = self.resolve_dependencies() {
            // Convert ResolutionError to the main kernel Error type
            return Err(KernelError::from(PluginSystemError::DependencyResolution(e)));
        }
        // --- End Dependency Resolution ---

        // --- Topological Sort and Priority Sort ---
        // 1. Perform topological sort (Kahn's algorithm)
        let mut adj: HashMap<String, Vec<String>> = HashMap::new(); // B -> A if A depends on B
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        
        for id in self.manifests.keys() {
            adj.entry(id.clone()).or_default();
            in_degree.entry(id.clone()).or_insert(0);
        }

        for (id_a, manifest_a) in &self.manifests { // A is manifest_a, id_a
            for dep_b in &manifest_a.dependencies { // A depends on B (dep_b.plugin_name)
                if !dep_b.required { continue; } // Only consider required dependencies for topological sort
                let id_b = &dep_b.plugin_name;

                if self.manifests.contains_key(id_b) { // Only add edges for known plugins
                    adj.entry(id_b.clone()).or_default().push(id_a.clone()); // Edge B -> A
                    *in_degree.entry(id_a.clone()).or_default() += 1;
                }
                // If id_b is not in self.manifests, resolve_dependencies should have caught it.
            }
        }

        let mut queue: VecDeque<String> = self.manifests.keys()
            .filter(|id| *in_degree.get(*id).expect("All manifest IDs should be in in_degree map") == 0)
            .cloned()
            .collect();

        let mut topological_order_ids: Vec<String> = Vec::new();
        while let Some(id_b) = queue.pop_front() { // B is processed (dependency)
            topological_order_ids.push(id_b.clone());
            if let Some(dependents_a) = adj.get(&id_b) { // For each A that depends on B
                for id_a in dependents_a {
                    if let Some(degree) = in_degree.get_mut(id_a) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(id_a.clone());
                        }
                    }
                }
            }
        }

        if topological_order_ids.len() != self.manifests.len() {
            // This indicates a cycle or other issue not caught by resolve_dependencies.
            // resolve_dependencies should ideally prevent this state.
            return Err(KernelError::from(PluginSystemError::DependencyResolution(
                DependencyError::Other(
                    "Topological sort failed: processed plugin count does not match total. Possible cycle or unresolved dependency.".to_string()
                )
            )));
        }

        // 2. Get manifests in topological order
        let mut manifests_in_topo_order: Vec<PluginManifest> = topological_order_ids.iter()
            .map(|id| self.manifests.get(id).expect("ID from topological sort must be in manifests").clone())
            .collect();

        // 3. Stable sort by priority. PluginPriority implements Ord (lower value = higher priority).
        // sort_by_key is stable.
        manifests_in_topo_order.sort_by_key(|manifest| {
            manifest.priority.as_ref()
                .and_then(|s| PluginPriority::from_str(s))
                .unwrap_or_else(|| PluginPriority::ThirdPartyLow(u8::MAX)) // Default to lowest priority
        });
        // --- End Topological Sort and Priority Sort ---

        let mut count = 0;
        // Convert the kernel's ApiVersion to semver::Version once for comparisons
        let api_semver = match semver::Version::parse(&api_version.to_string()) {
            Ok(v) => v,
            Err(e) => {
                // If the internal API version fails to parse, something is very wrong.
                return Err(PluginSystemError::VersionParsing(
                    crate::plugin_system::version::VersionError::ParseError(format!("Internal API version parse error: {}", e))
                ).into()); // Convert to KernelError
            }
        };

        // Load and register each plugin using the sorted list
        for manifest in manifests_in_topo_order { // Iterate over topologically and priority sorted manifests
            // Check API compatibility using semver::Version
            let mut compatible = false;
            // Access the api_versions field directly
            for version_range in &manifest.api_versions { // version_range is now VersionRange
                if version_range.includes(&api_semver) { // Compare against api_semver
                    compatible = true;
                    break;
                }
            }

            if !compatible {
                println!("Skipping incompatible plugin: {}", manifest.id);
                continue;
            }

            // Try to load the plugin asynchronously
            match self.load_plugin(&manifest).await { // Pass reference to manifest
                Ok(plugin_arc) => { // Changed from plugin to plugin_arc
                    if let Err(e) = registry.register_plugin(plugin_arc) { // Pass Arc<dyn Plugin>
                        eprintln!("Failed to register plugin {}: {}", manifest.id, e);
                    } else {
                        count += 1;
                    }
                }
                Err(e) => {
                    // e is already a KernelError here. If it's PluginSystem, print that, else print KernelError.
                    match e {
                        KernelError::PluginSystem(pse) => eprintln!("Failed to load plugin {}: {}", manifest.id, pse),
                        other_ke => eprintln!("Failed to load plugin {}: {}", manifest.id, other_ke),
                    }
                }
            }
        }

        Ok(count)
    }

    /// Get a manifest by plugin ID
    pub fn get_manifest(&self, id: &str) -> Option<&PluginManifest> {
        self.manifests.get(id)
    }

    /// Get all loaded manifests
    pub fn get_all_manifests(&self) -> Vec<&PluginManifest> {
        self.manifests.values().collect()
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}