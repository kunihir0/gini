// crates/gini-core/src/plugin_system/manager.rs
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future; // Import Future
use std::pin::Pin; // Import Pin
use std::path::{Path, PathBuf};
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use std::fs;
use libloading::{Library, Symbol};
use std::panic;
use std::ffi::{CStr, c_void};
use std::os::raw::c_char;

use crate::kernel::bootstrap::Application;
use crate::stage_manager::context::StageContext;
use crate::stage_manager::Stage;
use crate::stage_manager::requirement::StageRequirement;

use crate::kernel::component::KernelComponent;
use crate::storage::config::{ConfigManager, ConfigScope, PluginConfigScope, ConfigData};
use crate::storage::StorageProvider;
use crate::kernel::error::{Error, Result}; // Crate's Result alias
use crate::plugin_system::traits::{
    FfiResult, FfiSlice, FfiVersionRange, FfiPluginDependency,
    FfiStageRequirement, PluginVTable, PluginError,
};
use crate::plugin_system::{Plugin, PluginManifest, ApiVersion, PluginRegistry, PluginPriority, VersionRange, PluginDependency};
use crate::kernel::constants;


// --- Local FFI Helper Functions ---

/// Maps FfiResult to crate::kernel::error::Error
fn map_ffi_error(ffi_err: FfiResult, context: &str) -> Error {
    Error::Plugin(format!("FFI Error ({}) - {:?}: ", context, ffi_err))
}

/// Safely converts an FFI C string pointer to a Rust String.
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
    unsafe fn new(vtable_ptr: *mut PluginVTable, library: Library) -> Result<Self> {
        if vtable_ptr.is_null() {
            // If pointer is null, drop the library as it's unusable
            drop(library);
            return Err(Error::Plugin("Received null VTable pointer from plugin init.".to_string()));
        }

        // Dereference the pointer once safely
        let vtable_ref = unsafe { &*vtable_ptr };
        let instance_const = vtable_ref.instance as *const c_void;

        // Attempt to read all necessary fields. If any fail, return Err.
        // The `library` variable will be dropped automatically in the Err case if we return early.

        // Get name, convert, and immediately free the FFI pointer
        let name_ptr = (vtable_ref.name)(instance_const); // Removed unnecessary unsafe block
        let name_string = unsafe { ffi_string_from_ptr(name_ptr) } // Keep unsafe for ffi_string_from_ptr
            .map_err(|e| map_ffi_error(e, "getting plugin name"))?;
        (vtable_ref.free_name)(name_ptr as *mut c_char); // Removed unnecessary unsafe block

        // Leak the name string once to get &'static str
        let static_name: &'static str = Box::leak(name_string.into_boxed_str());

        // Get version, convert, and immediately free the FFI pointer
        let version_ptr = (vtable_ref.version)(instance_const); // Removed unnecessary unsafe block
        let version = unsafe { ffi_string_from_ptr(version_ptr) } // Keep unsafe for ffi_string_from_ptr
            .map_err(|e| map_ffi_error(e, "getting plugin version"))?;
        (vtable_ref.free_version)(version_ptr as *mut c_char); // Removed unnecessary unsafe block

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
    { unsafe {
        // The outer function is already unsafe, so this inner block is redundant.
        let vtable = &*self.vtable.0; // Removed unnecessary unsafe block
        let instance_const = vtable.instance as *const c_void;
        let ffi_slice = (get_slice_fn)(instance_const);
        let result = ffi_slice.as_slice().map_or_else(
            || Ok(Vec::new()),
            |slice_data| {
                slice_data.iter()
                    .map(|&item| converter(item))
                    .collect::<Result<Vec<R>>>() // Collect into crate Result<Vec<R>>
            },
        );
        (free_slice_fn)(ffi_slice);
        result
    }}
}

impl Drop for VTablePluginWrapper {
    fn drop(&mut self) {
        println!("Dropping VTablePluginWrapper for '{}'", self.name_cache);
        unsafe {
            if !self.vtable.0.is_null() {
                // 1. Destroy the plugin instance via the VTable function pointer
                let vtable_ref = &*self.vtable.0;
                println!("  - Calling destroy function pointer...");
                (vtable_ref.destroy)(vtable_ref.instance);
                println!("  - destroy function called.");

                // 2. Free the VTable struct itself (reconstruct the Box)
                println!("  - Reconstructing VTable Box...");
                let _vtable_box = Box::from_raw(self.vtable.0 as *mut PluginVTable); // Unsafe block removed (outer function is unsafe)
                println!("  - VTable Box reconstructed and will be dropped.");
                // Set pointer to null after freeing to prevent double free if drop is called again somehow
                self.vtable.0 = std::ptr::null();
            } else {
                 println!("  - VTable pointer was null, skipping instance/VTable destruction.");
            }

            // 3. Drop the Library Option, which unloads the library if Some(library)
            println!("  - Dropping Library Option...");
            // This happens automatically when `self.library` goes out of scope after this function,
            // or we can be explicit:
            drop(self.library.take()); // Take ownership and drop it
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

    // --- Methods NOT handled by the simple VTable ---

    fn init(&self, _app: &mut Application) -> Result<()> {
        println!("Warning: VTablePluginWrapper::init called for '{}', but not implemented via VTable.", self.name_cache);
        Ok(())
    }

    // Manually implement the async fn to match async_trait's expected signature
    #[must_use = "The result of preflight_check should be handled"]
    fn preflight_check<'life0, 'life1, 'async_trait>(
        &'life0 self,
        _context: &'life1 StageContext
    ) -> Pin<Box<dyn Future<Output = std::result::Result<(), PluginError>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        let name_cache = self.name_cache; // Clone to move into the async block
        Box::pin(async move {
            println!("Warning: VTablePluginWrapper::preflight_check called for '{}', but not implemented via VTable.", name_cache);
            // The actual logic would go here if preflight_check was supported via FFI
            Ok(())
        })
    }


    fn stages(&self) -> Vec<Box<dyn Stage>> {
        println!("Warning: VTablePluginWrapper::stages called for '{}', but not implemented via VTable.", self.name_cache);
        Vec::new()
    }

    fn shutdown(&self) -> Result<()> {
        println!("VTablePluginWrapper::shutdown called for '{}' (handled by Drop).", self.name_cache);
        Ok(())
    }
}

// --- End VTablePluginWrapper ---


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
    async fn enable_plugin(&self, id: &str) -> Result<()>;
    async fn disable_plugin(&self, id: &str) -> Result<()>;
    async fn is_plugin_enabled(&self, id: &str) -> Result<bool>;
    async fn get_plugin_manifest(&self, id: &str) -> Result<Option<PluginManifest>>;
}

/// Default implementation of plugin manager
pub struct DefaultPluginManager<P: StorageProvider + ?Sized + 'static> {
    name: &'static str,
    registry: Arc<Mutex<PluginRegistry>>,
    config_manager: Arc<ConfigManager<P>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct PluginStates {
    #[serde(default)]
    enabled_map: HashMap<String, bool>,
}

const PLUGIN_STATES_CONFIG_NAME: &str = "plugin_states";
const PLUGIN_STATES_CONFIG_KEY: &str = "enabled_map";
const PLUGIN_STATES_SCOPE: ConfigScope = ConfigScope::Plugin(PluginConfigScope::User);

impl<P: StorageProvider + ?Sized + 'static> DefaultPluginManager<P> {
    pub fn new(config_manager: Arc<ConfigManager<P>>) -> Result<Self> {
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

        let library = unsafe { Library::new(path) }
            .map_err(|e| Error::Plugin(format!("Failed to load library {:?}: {}", path, e)))?;

        let init_symbol: Symbol<PluginInitFn> = unsafe { library.get(b"_plugin_init\0") }
            .map_err(|e| Error::Plugin(format!("Failed to find _plugin_init symbol in {:?}: {}", path, e)))?;

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
             _ => unreachable!(),
        };

        // Pass the library ownership to the wrapper
        let plugin_wrapper = unsafe { VTablePluginWrapper::new(vtable_ptr, library)? }; // Pass library here

        Ok(Box::new(plugin_wrapper))
    }
}

impl<P: StorageProvider + ?Sized + 'static> Debug for DefaultPluginManager<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultPluginManager")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl<P: StorageProvider + ?Sized + 'static> KernelComponent for DefaultPluginManager<P> {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn initialize(&self) -> Result<()> {
        println!("Initializing Plugin Manager...");
        let loaded_states = match self.load_plugin_states() {
            Ok(states) => { println!("Successfully loaded plugin states."); states }
            Err(e) => { eprintln!("Warning: Failed to load plugin states: {}. Proceeding with defaults.", e); HashMap::new() }
        };
        if !loaded_states.is_empty() {
            let mut registry = self.registry.lock().await;
            for (plugin_id, should_be_enabled) in loaded_states.iter() {
                if registry.has_plugin(plugin_id) {
                    if *should_be_enabled { if !registry.is_enabled(plugin_id) { registry.enabled.insert(plugin_id.to_string()); println!("Applied loaded state: Enabled plugin '{}'", plugin_id); } }
                    else { if registry.is_enabled(plugin_id) { registry.enabled.remove(plugin_id); println!("Applied loaded state: Disabled plugin '{}'", plugin_id); } }
                } else { println!("Plugin '{}' found in state file but not currently registered. State ignored.", plugin_id); }
            }
        }
        let plugin_dir = PathBuf::from("./target/debug");
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
impl<P: StorageProvider + ?Sized + 'static> PluginManager for DefaultPluginManager<P> {
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
                                match self.load_so_plugin(&path) {
                                    Ok(plugin) => {
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

    async fn enable_plugin(&self, id: &str) -> Result<()> {
        { let mut registry = self.registry.lock().await; registry.enable_plugin(id)?; }
        let mut states = self.load_plugin_states()?;
        states.insert(id.to_string(), true);
        self.save_plugin_states(&states)?;
        Ok(())
    }

    async fn disable_plugin(&self, id: &str) -> Result<()> {
        let plugin_opt = self.get_plugin(id).await?;
        if let Some(plugin) = plugin_opt { if plugin.is_core() { return Err(Error::Plugin(format!("Cannot disable core plugin '{}'", id))); } }
        { let mut registry = self.registry.lock().await; registry.disable_plugin(id)?; }
        let mut states = self.load_plugin_states()?;
        states.insert(id.to_string(), false);
        self.save_plugin_states(&states)?;
        Ok(())
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

// Helper methods for loading/saving state
impl<P: StorageProvider + ?Sized + 'static> DefaultPluginManager<P> {
    fn load_plugin_states(&self) -> Result<HashMap<String, bool>> {
        match self.config_manager.load_config(PLUGIN_STATES_CONFIG_NAME, PLUGIN_STATES_SCOPE) {
            Ok(config_data) => {
                let states_map: HashMap<String, bool> = config_data.get(PLUGIN_STATES_CONFIG_KEY)
                    .unwrap_or_else(|| { println!("No existing plugin states found under key '{}', using defaults.", PLUGIN_STATES_CONFIG_KEY); HashMap::new() });
                Ok(states_map)
            }
            Err(Error::Storage(msg)) if msg.contains("Unknown config format") || msg.contains("Failed to read") => {
                println!("Plugin state file '{}' not found or invalid format, using defaults.", PLUGIN_STATES_CONFIG_NAME);
                Ok(HashMap::new())
            }
            Err(e) => Err(Error::Storage(format!("Failed to load plugin states config: {}", e))),
        }
    }

    fn save_plugin_states(&self, states: &HashMap<String, bool>) -> Result<()> {
        let mut config_data = match self.config_manager.load_config(PLUGIN_STATES_CONFIG_NAME, PLUGIN_STATES_SCOPE) {
            Ok(data) => data,
            Err(Error::Storage(msg)) if msg.contains("Unknown config format") || msg.contains("Failed to read") => {
                 println!("Plugin state file '{}' not found or invalid, creating new one.", PLUGIN_STATES_CONFIG_NAME);
                 ConfigData::default() // Use ConfigData::default()
            }
            Err(e) => return Err(e),
        };
        config_data.set(PLUGIN_STATES_CONFIG_KEY, states)?;
        self.config_manager.save_config(PLUGIN_STATES_CONFIG_NAME, &config_data, PLUGIN_STATES_SCOPE)?;
        println!("Successfully saved plugin states.");
        Ok(())
    }
}

impl<P: StorageProvider + ?Sized + 'static> Clone for DefaultPluginManager<P> {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            registry: Arc::clone(&self.registry),
            config_manager: Arc::clone(&self.config_manager),
        }
    }
}