use std::any::Any;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex; // Use Tokio Mutex for async safety if needed
use std::fs; // Added for directory scanning
use libloading::{Library, Symbol}; // Added for dynamic loading
use std::panic; // Added for safe FFI calls

use crate::kernel::component::KernelComponent;
use crate::kernel::error::{Error, Result};
use crate::plugin_system::{Plugin, PluginManifest, ApiVersion, PluginPriority, PluginRegistry}; // Import PluginRegistry
use crate::kernel::constants; // Import constants

/// Plugin system component interface
#[async_trait]
pub trait PluginManager: KernelComponent {
    /// Load a plugin from a file
    async fn load_plugin(&self, path: &Path) -> Result<()>;

    /// Load all plugins from a directory
    async fn load_plugins_from_directory(&self, dir: &Path) -> Result<usize>;

    /// Get a plugin Arc by ID
    async fn get_plugin(&self, id: &str) -> Result<Option<Arc<dyn Plugin>>>;

    /// Get all loaded plugin Arcs
    async fn get_plugins(&self) -> Result<Vec<Arc<dyn Plugin>>>;

    /// Get all enabled plugin Arcs
    async fn get_enabled_plugins(&self) -> Result<Vec<Arc<dyn Plugin>>>;

    /// Check if a plugin is loaded
    async fn is_plugin_loaded(&self, id: &str) -> Result<bool>; // Return Result<bool>

    /// Get plugin dependencies
    async fn get_plugin_dependencies(&self, id: &str) -> Result<Vec<String>>;

    /// Get plugins that depend on a plugin
    async fn get_dependent_plugins(&self, id: &str) -> Result<Vec<String>>;

    /// Enable a plugin (potentially async if it involves loading/init)
    async fn enable_plugin(&self, id: &str) -> Result<()>;

    /// Disable a plugin (potentially async if it involves unloading/shutdown)
    async fn disable_plugin(&self, id: &str) -> Result<()>;

    /// Check if a plugin is enabled
    async fn is_plugin_enabled(&self, id: &str) -> Result<bool>; // Return Result<bool>

    /// Get plugin manifest
    async fn get_plugin_manifest(&self, id: &str) -> Result<Option<PluginManifest>>; // Return Result<Option>
}

/// Default implementation of plugin manager
#[derive(Clone)] // Add Clone derive
pub struct DefaultPluginManager {
    name: &'static str,
    // Use Tokio Mutex for async safety with the registry
    registry: Arc<Mutex<PluginRegistry>>,
}

impl DefaultPluginManager {
    /// Create a new default plugin manager
    pub fn new() -> Result<Self> { // Return Result for parsing
        // Parse the API version from constants
        let api_version = ApiVersion::from_str(constants::API_VERSION)
            .map_err(|e| Error::Init(format!("Failed to parse API_VERSION constant: {}", e)))?;

        Ok(Self {
            name: "DefaultPluginManager",
            // Create PluginRegistry with the parsed ApiVersion
            registry: Arc::new(Mutex::new(PluginRegistry::new(api_version))),
        })
    }

    /// Get reference to the plugin registry Arc<Mutex>
    pub fn registry(&self) -> &Arc<Mutex<PluginRegistry>> {
        &self.registry
    }

    /// Safely loads a single .so plugin file.
    /// Note: This is a blocking operation due to file I/O and library loading.
    /// Consider moving to a blocking thread pool if performance becomes an issue.
    fn load_so_plugin(&self, path: &Path) -> Result<Box<dyn Plugin>> {
        // Define the type of the plugin initialization function.
        // Note: Using *mut dyn Plugin directly for simplicity, even though it generates FFI safety warnings
        type PluginInitFn = unsafe extern "C" fn() -> *mut dyn Plugin;

        // 1. Load the library
        let library = unsafe { Library::new(path) }
            .map_err(|e| Error::Plugin(format!("Failed to load library {:?}: {}", path, e)))?;

        // 2. Get the initialization symbol
        let init_symbol: Symbol<PluginInitFn> = unsafe { library.get(b"_plugin_init\0") }
            .map_err(|e| Error::Plugin(format!("Failed to find _plugin_init symbol in {:?}: {}", path, e)))?;

        // 3. Call the initialization function safely
        // It's crucial to catch panics from FFI boundaries.
        let plugin_instance_ptr = match panic::catch_unwind(|| unsafe { init_symbol() }) {
            Ok(ptr) => ptr,
            Err(e) => {
                // Attempt to get a printable error message from the panic payload
                let panic_msg = if let Some(s) = e.downcast_ref::<&'static str>() {
                    *s
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.as_str()
                } else {
                    "Unknown panic reason"
                };
                return Err(Error::Plugin(format!(
                    "Plugin initialization panicked in {:?}: {}",
                    path, panic_msg
                )));
            }
        };

        // 4. Reconstruct the Box from the raw pointer. This takes ownership of the memory.
        let plugin_instance = unsafe { Box::from_raw(plugin_instance_ptr) };

        // 5. Important: Forget the library. If we drop it, the plugin code is unloaded.
        // This means the plugin's code remains loaded as long as the PluginManager exists.
        // Proper unloading would require more complex lifetime management.
        std::mem::forget(library);

        Ok(plugin_instance)
    }
}

impl Debug for DefaultPluginManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Avoid locking in Debug if possible, or show minimal info
        f.debug_struct("DefaultPluginManager")
            .field("name", &self.name)
            .finish_non_exhaustive() // Indicate registry state is omitted
    }
}

#[async_trait]
impl KernelComponent for DefaultPluginManager {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn initialize(&self) -> Result<()> {
        // Potentially load core plugins here
        println!("Initializing Plugin Manager...");
        // Load plugins from the default directory (e.g., target/debug for development)
        // TODO: Make this configurable or use a more standard location like ./plugins
        let plugin_dir = PathBuf::from("./target/debug"); // Or constants::PLUGINS_DIR
        if plugin_dir.exists() && plugin_dir.is_dir() {
            match self.load_plugins_from_directory(&plugin_dir).await {
                Ok(count) => println!("Loaded {} external plugins from {:?}", count, plugin_dir),
                Err(e) => eprintln!("Error loading plugins from {:?}: {}", plugin_dir, e), // Log error but continue
            }
        } else {
            println!("Plugin directory {:?} not found, skipping external plugin loading.", plugin_dir);
        }
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        // Initialize all loaded plugins
        println!("Starting Plugin Manager - Initializing plugins...");
        // This needs access to the Application/Kernel, which isn't directly available here.
        // Dependency injection or passing the kernel reference during init/start is needed.
        // For now, we skip the actual initialization call.
        // Initialize all *enabled* plugins (handled by registry.initialize_all)
        println!("Starting Plugin Manager - Initializing enabled plugins...");
        // This still needs the Application/Kernel reference passed in somehow.
        // For now, we assume it would be passed to initialize_all if available.
        // let mut registry = self.registry.lock().await;
        // registry.initialize_all(&mut app)?;
        println!("Plugin initialization skipped (needs Application reference).");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        // Shutdown all plugins
        println!("Stopping Plugin Manager - Shutting down plugins...");
        // Shutdown all *initialized* plugins (handled by registry.shutdown_all)
        println!("Stopping Plugin Manager - Shutting down initialized plugins...");
        let mut registry = self.registry.lock().await;
        registry.shutdown_all() // Returns Result
    }
    // Removed as_any and as_any_mut
}

#[async_trait]
impl PluginManager for DefaultPluginManager {
    async fn load_plugin(&self, path: &Path) -> Result<()> {
        println!("Attempting to load plugin from {:?}", path);
        match self.load_so_plugin(path) {
            Ok(plugin) => {
                let name = plugin.name().to_string();
                let mut registry = self.registry.lock().await;
                match registry.register_plugin(plugin) {
                    Ok(_) => {
                        println!("Successfully loaded and registered plugin: {}", name);
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("Failed to register plugin from {:?}: {}", path, e);
                        Err(e)
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to load plugin from {:?}: {}", path, e);
                Err(e)
            }
        }
    }

    async fn load_plugins_from_directory(&self, dir: &Path) -> Result<usize> {
        println!("Scanning for plugins in directory {:?}", dir);
        let mut loaded_count = 0;
        let mut errors = Vec::new();

        match fs::read_dir(dir) {
            Ok(entries) => {
                let mut registry = self.registry.lock().await; // Lock once outside the loop

                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            // Check if it's a file and has the .so extension (Linux specific)
                            if path.is_file() && path.extension().map_or(false, |ext| ext == "so") {
                                println!("Found potential plugin: {:?}", path);
                                match self.load_so_plugin(&path) {
                                    Ok(plugin) => {
                                        let name = plugin.name().to_string();
                                        match registry.register_plugin(plugin) {
                                            Ok(_) => {
                                                println!("Successfully loaded and registered plugin: {}", name);
                                                loaded_count += 1;
                                            }
                                            Err(e) => {
                                                let err_msg = format!("Failed to register plugin from {:?}: {}", path, e);
                                                eprintln!("{}", err_msg);
                                                errors.push(err_msg);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let err_msg = format!("Failed to load plugin library {:?}: {}", path, e);
                                        eprintln!("{}", err_msg);
                                        errors.push(err_msg);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let err_msg = format!("Failed to read directory entry in {:?}: {}", dir, e);
                            eprintln!("{}", err_msg);
                            errors.push(err_msg);
                        }
                    }
                }
            }
            Err(e) => {
                return Err(Error::Plugin(format!("Failed to read plugin directory {:?}: {}", dir, e)));
            }
        }

        if errors.is_empty() {
            Ok(loaded_count)
        } else {
            // Combine errors into a single error message if any occurred
            Err(Error::Plugin(format!(
                "Encountered errors while loading plugins from {:?}: {}",
                dir,
                errors.join("; ")
            )))
        }
    }

    async fn get_plugin(&self, id: &str) -> Result<Option<Arc<dyn Plugin>>> {
        let registry = self.registry.lock().await;
        Ok(registry.get_plugin(id)) // Directly use the registry method
    }

    async fn get_plugins(&self) -> Result<Vec<Arc<dyn Plugin>>> {
        let registry = self.registry.lock().await;
        Ok(registry.get_plugins_arc()) // Use the new registry method
    }

    async fn get_enabled_plugins(&self) -> Result<Vec<Arc<dyn Plugin>>> {
        let registry = self.registry.lock().await;
        Ok(registry.get_enabled_plugins_arc()) // Use the new registry method
    }

    async fn is_plugin_loaded(&self, id: &str) -> Result<bool> {
        let registry = self.registry.lock().await;
        Ok(registry.has_plugin(id))
    }

    async fn get_plugin_dependencies(&self, id: &str) -> Result<Vec<String>> {
        let registry = self.registry.lock().await;
        // Use the get_plugin method which returns Option<Arc<dyn Plugin>>
        match registry.get_plugin(id) {
            Some(plugin_arc) => {
                // Access dependencies via the Arc
                Ok(plugin_arc.dependencies().iter().map(|dep| dep.plugin_name.clone()).collect())
            }
            None => Err(Error::Plugin(format!("Plugin not found: {}", id))),
        }
    }

    async fn get_dependent_plugins(&self, id: &str) -> Result<Vec<String>> {
        let registry = self.registry.lock().await;
        let mut dependents = Vec::new();
        // Use the updated iterator method which yields (&String, &Arc<dyn Plugin>)
        for (plugin_id, plugin_arc) in registry.iter_plugins() {
            // Access dependencies via the Arc
            if plugin_arc.dependencies().iter().any(|dep| dep.plugin_name == id) {
                dependents.push(plugin_id.clone());
            }
        }
        Ok(dependents)
    }

    async fn enable_plugin(&self, id: &str) -> Result<()> {
        let mut registry = self.registry.lock().await;
        registry.enable_plugin(id) // Delegate to registry
    }

    async fn disable_plugin(&self, id: &str) -> Result<()> {
        // First check if plugin is a core plugin
        let plugin_opt = self.get_plugin(id).await?;
        
        if let Some(plugin) = plugin_opt {
            // Don't allow disabling core plugins
            if plugin.is_core() {
                return Err(Error::Plugin(format!(
                    "Cannot disable core plugin '{}'", id
                )));
            }
        }
        
        let mut registry = self.registry.lock().await;
        registry.disable_plugin(id) // Delegate to registry
    }

    async fn is_plugin_enabled(&self, id: &str) -> Result<bool> {
        let registry = self.registry.lock().await;
        Ok(registry.is_enabled(id)) // Delegate to registry
    }

    async fn get_plugin_manifest(&self, id: &str) -> Result<Option<PluginManifest>> {
        let registry = self.registry.lock().await;
        match registry.get_plugin(id) {
            Some(plugin_arc) => {
                // Construct the manifest from the plugin trait methods
                let plugin_ref = &*plugin_arc; // Dereference Arc to get &dyn Plugin
                let manifest = PluginManifest {
                    // Fields directly from Plugin trait
                    id: plugin_ref.name().to_string(), // Use name as ID for now
                    name: plugin_ref.name().to_string(),
                    version: plugin_ref.version().to_string(),
                    api_versions: plugin_ref.compatible_api_versions(),
                    dependencies: plugin_ref.dependencies().iter().map(|dep| {
                        // Convert PluginDependency to manifest's DependencyInfo
                        crate::plugin_system::manifest::DependencyInfo {
                            id: dep.plugin_name.clone(),
                            version_range: dep.version_range.clone(),
                            required: dep.required,
                        }
                    }).collect(),
                    is_core: plugin_ref.is_core(),
                    priority: Some(plugin_ref.priority().to_string()), // Convert enum to string

                    // Fields *not* directly available from Plugin trait - use defaults/placeholders
                    description: format!("Manifest generated from plugin '{}'", plugin_ref.name()), // Placeholder
                    author: "Unknown".to_string(), // Placeholder
                    website: None, // Placeholder
                    license: None, // Placeholder
                    entry_point: format!("lib{}.so", plugin_ref.name()), // Default assumption
                    files: vec![], // Placeholder
                    config_schema: None, // Placeholder
                    tags: vec![], // Placeholder
                };
                Ok(Some(manifest))
            }
            None => Ok(None), // Plugin not found
        }
    }
}

// Removed Default implementation as Self::new() now returns Result