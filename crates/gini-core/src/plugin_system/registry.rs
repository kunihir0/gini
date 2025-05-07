use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex; // Add back Mutex import
use std::pin::Pin;
use std::future::Future;

use crate::kernel::error::{Error, Result};
use crate::kernel::bootstrap::Application;
use crate::plugin_system::traits::Plugin;
use crate::plugin_system::version::ApiVersion;
use crate::plugin_system::conflict::{ConflictManager, ConflictType, PluginConflict};
use crate::stage_manager::registry::StageRegistry; // Keep StageRegistry, SharedStageRegistry not directly used in this file's signatures now
use semver::Version;

/// Registry for managing plugins
pub struct PluginRegistry {
    /// Registered plugins (using Arc for shared ownership)
    pub plugins: HashMap<String, Arc<dyn Plugin>>,
    /// Plugin initialization status
    pub initialized: HashSet<String>,
    /// Enabled plugin IDs
    pub enabled: HashSet<String>,
    /// Current API version
    api_version: ApiVersion,
    /// Conflict manager
    conflict_manager: ConflictManager, // Add ConflictManager field
}

impl PluginRegistry {
    /// Create a new plugin registry with the specified API version
    pub fn new(api_version: ApiVersion) -> Self {
        Self {
            plugins: HashMap::new(),
            initialized: HashSet::new(),
            enabled: HashSet::new(), // Initialize enabled set
            api_version,
            conflict_manager: ConflictManager::new(), // Initialize ConflictManager
        }
    }
    
    /// Register a plugin
    pub fn register_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<()> {
        let name = plugin.name().to_string();
        let id = name.clone(); // Use name as ID for now

        if self.plugins.contains_key(&id) {
            return Err(Error::Plugin(format!("Plugin already registered: {}", name)));
        }
        
        // Check API compatibility
        let mut compatible = false;
        // Convert ApiVersion to semver::Version for comparison
        let api_semver = match semver::Version::parse(&self.api_version.to_string()) {
            Ok(v) => v,
            Err(e) => {
                // Log error and consider it incompatible if internal API version fails parsing
                eprintln!("Failed to parse internal API version {}: {}", self.api_version, e);
                return Err(Error::Plugin(format!("Internal API version parse error: {}", e)));
            }
        };
        for version_range in plugin.compatible_api_versions() {
            if version_range.includes(&api_semver) { // Use includes with semver::Version
                compatible = true;
                break;
            }
        }
        
        if !compatible {
            return Err(Error::Plugin(format!(
                "Plugin {} is not compatible with API version {}",
                name,
                self.api_version
            )));
        }
        
        // All good, wrap in Arc and register the plugin
        let plugin_arc = Arc::from(plugin);
        self.plugins.insert(id.clone(), plugin_arc);
        // Newly registered plugins are enabled by default
        self.enabled.insert(id);
        Ok(())
    }
    
    /// Unregister a plugin by ID
    pub fn unregister_plugin(&mut self, id: &str) -> Result<Arc<dyn Plugin>> {
        if let Some(plugin) = self.plugins.remove(id) {
            self.initialized.remove(id);
            self.enabled.remove(id); // Also remove from enabled set
            Ok(plugin)
        } else {
            Err(Error::Plugin(format!("Plugin not found: {}", id)))
        }
    }
    
    /// Check if a plugin is registered by ID
    pub fn has_plugin(&self, id: &str) -> bool {
        self.plugins.contains_key(id)
    }
/// Checks if a plugin with the given name is already registered.
    pub fn is_registered(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }
    
    /// Get a plugin Arc by ID
    pub fn get_plugin(&self, id: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.get(id).cloned()
    }
    /// Get an iterator over registered plugin IDs and Arc references
    pub fn iter_plugins(&self) -> impl Iterator<Item = (&String, &Arc<dyn Plugin>)> {
        self.plugins.iter()
    }

    /// Get a Vec of all registered plugin Arcs
    pub fn get_plugins_arc(&self) -> Vec<Arc<dyn Plugin>> {
        self.plugins.values().cloned().collect()
    }

    /// Get a Vec of enabled plugin Arcs
    pub fn get_enabled_plugins_arc(&self) -> Vec<Arc<dyn Plugin>> {
        self.plugins
            .iter()
            .filter(|(id, _)| self.enabled.contains(*id))
            .map(|(_, plugin)| plugin.clone())
            .collect()
    }

    /// Builds the dependency graph for a given set of plugin IDs.
    /// Returns the adjacency list (plugin -> dependencies) and reverse adjacency list (plugin -> dependents).
    fn build_dependency_graph(
        &self,
        plugin_ids: &HashSet<String>,
    ) -> (HashMap<String, Vec<String>>, HashMap<String, Vec<String>>) {
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        let mut reverse_adj: HashMap<String, Vec<String>> = HashMap::new();

        for id in plugin_ids {
            // Ensure all plugins in the set are keys in the maps
            adj.entry(id.clone()).or_default();
            reverse_adj.entry(id.clone()).or_default();

            if let Some(plugin) = self.plugins.get(id) {
                for dep in plugin.dependencies() {
                    // Only consider dependencies that are also in the target set (e.g., enabled plugins)
                    if plugin_ids.contains(&dep.plugin_name) {
                        // Standard adjacency list: id depends on dep.plugin_name
                        adj.entry(id.clone()).or_default().push(dep.plugin_name.clone());
                        // Reverse adjacency list: dep.plugin_name is depended on by id
                        reverse_adj.entry(dep.plugin_name.clone()).or_default().push(id.clone());
                    }
                    // We don't add dependencies outside the set to the graph itself,
                    // but version/existence checks later will handle them.
                }
            }
        }
        (adj, reverse_adj)
    }

    /// Performs a topological sort (Kahn's algorithm) on a given graph.
    /// Returns the sorted list of plugin IDs or a DependencyError::CyclicDependency.
    fn topological_sort(
        &self,
        plugin_ids: &HashSet<String>,
        adj: &HashMap<String, Vec<String>>, // plugin -> dependencies
    ) -> std::result::Result<Vec<String>, crate::plugin_system::dependency::DependencyError> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut sorted_list = Vec::new();
        let mut queue = VecDeque::new();

        // Calculate initial in-degrees (number of incoming edges/dependencies)
        for id in plugin_ids {
            in_degree.insert(id.clone(), 0);
        }
        for dependencies in adj.values() {
            for dep_id in dependencies {
                if plugin_ids.contains(dep_id) { // Only count edges within the set
                    *in_degree.entry(dep_id.clone()).or_insert(0) += 1;
                }
            }
        }

        // Initialize queue with nodes having in-degree 0
        for id in plugin_ids {
            if *in_degree.get(id).unwrap_or(&1) == 0 { // Use unwrap_or to handle potential missing entries defensively
                queue.push_back(id.clone());
            }
        }

        while let Some(id) = queue.pop_front() {
            sorted_list.push(id.clone());

            // For each neighbor (dependency) of the current node `id`
            if let Some(dependencies) = adj.get(&id) {
                for dep_id in dependencies {
                     if plugin_ids.contains(dep_id) { // Process only dependencies within the set
                        if let Some(degree) = in_degree.get_mut(dep_id) {
                            *degree -= 1;
                            if *degree == 0 {
                                queue.push_back(dep_id.clone());
                            }
                        }
                     }
                }
            }
        }

        if sorted_list.len() == plugin_ids.len() {
            // The result should be in initialization order (dependencies first).
            // Kahn's algorithm naturally produces this order.
            // However, the standard algorithm adds nodes with 0 in-degree first.
            // If A depends on B (A -> B), B has in-degree 0 initially. B gets added.
            // Then A's in-degree becomes 0. A gets added. Order: [B, A]. This is correct for init.
             // Do NOT reverse the list. Kahn's algorithm naturally produces the correct init order.
             Ok(sorted_list)
        } else {
            // Cycle detected. Find the nodes involved in the cycle (nodes not in sorted_list).
            let cycle_nodes: Vec<String> = plugin_ids
                .iter()
                .filter(|id| !sorted_list.contains(id))
                .cloned()
                .collect();
            // Note: This doesn't give the exact path, just the nodes involved.
            // More complex cycle finding algorithms exist but add complexity.
            Err(crate::plugin_system::dependency::DependencyError::CyclicDependency(cycle_nodes))
        }
    }
    
    /// Public entry point for initializing a single plugin.
    /// Handles the initial call and sets up cycle detection.
    /// Returns a pinned, boxed future.
    // Make the public function async again
    pub async fn initialize_plugin(
        &mut self,
        id: &str,
        app: &mut Application, // Keep app for Plugin::init for now
        stage_registry_arc: &Arc<Mutex<StageRegistry>>, // Expect &Arc<Mutex<StageRegistry>>
    ) -> Result<()> {
        let mut currently_initializing = HashSet::new();
        // Pass the stage_registry_arc down
        self.initialize_plugin_recursive(id, app, stage_registry_arc, &mut currently_initializing).await
    }

    /// Internal recursive function for plugin initialization with cycle detection.
    /// Returns a pinned, boxed future.
    fn initialize_plugin_recursive<'a>(
        &'a mut self,
        id: &'a str,
        app: &'a mut Application, // Keep app for Plugin::init
        stage_registry_arc: &'a Arc<Mutex<StageRegistry>>, // Expect &Arc<Mutex<StageRegistry>>
        currently_initializing: &'a mut HashSet<String>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        // Wrap the async logic in Box::pin
        Box::pin(async move {
        // Ensure plugin exists and is enabled before initializing
        if !self.has_plugin(id) {
            return Err(Error::Plugin(format!("Plugin not found: {}", id)));
        }
        if !self.is_enabled(id) {
             println!("Plugin {} is disabled, skipping initialization.", id);
             return Ok(()); // Skip initialization if disabled
        }

        if self.initialized.contains(id) {
            return Ok(()); // Already initialized
        }

        // --- Cycle Detection ---
        if currently_initializing.contains(id) {
            return Err(Error::Plugin(format!(
                "Cyclic dependency detected during initialization involving plugin '{}'", id
            )));
        }
        currently_initializing.insert(id.to_string());
        // --- End Cycle Detection ---

        // Get plugin dependencies (needs cloning the Arc to access methods)
        let plugin_arc = match self.plugins.get(id) {
             Some(p) => p.clone(),
             None => return Err(Error::Plugin(format!("Plugin {} disappeared unexpectedly", id))), // Should not happen if has_plugin passed
        };
        let dependencies = plugin_arc.dependencies().clone();


        // Check and initialize dependencies first
        for dep in dependencies {
            let dep_exists = self.has_plugin(&dep.plugin_name);
            let dep_enabled = self.is_enabled(&dep.plugin_name);

            // Check for missing/disabled required dependencies
            if dep.required && (!dep_exists || !dep_enabled) {
                return Err(Error::Plugin(format!(
                    "Plugin '{}' requires enabled dependency '{}', which is missing or disabled.",
                    id, dep.plugin_name
                )));
            }

            // Check version constraint if dependency exists
            if dep_exists {
                 if let Some(dep_plugin) = self.get_plugin(&dep.plugin_name) {
                     if let Some(ref required_range) = dep.version_range {
                         match semver::Version::parse(dep_plugin.version()) {
                             Ok(dep_version) => {
                                 if !required_range.includes(&dep_version) {
                                     return Err(Error::Plugin(format!(
                                         "Plugin '{}' requires dependency '{}' version '{}', but found version '{}'",
                                         id,
                                         dep.plugin_name,
                                         required_range.constraint_string(),
                                         dep_plugin.version()
                                     )));
                                 }
                             },
                             Err(e) => {
                                 return Err(Error::Plugin(format!(
                                     "Failed to parse version string '{}' for dependency plugin '{}': {}",
                                     dep_plugin.version(), dep.plugin_name, e
                                 )));
                             }
                         }
                     }
                 }
            }

            // Initialize dependency if required, enabled, exists, and not already initialized
            if dep.required && dep_exists && dep_enabled && !self.initialized.contains(&dep.plugin_name) {
                 // Recursive call: Pass app and the stage_registry_arc_clone
                 self.initialize_plugin_recursive(
                     &dep.plugin_name,
                     app, // Pass app reference down
                     stage_registry_arc, // Pass the original Arc reference down
                     currently_initializing
                 ).await?;
            }
       }

       // --- Initialize the plugin itself ---
       println!("[PluginRegistry] Initializing plugin: {}", id);

       // Call init with mutable borrow of app (still required by Plugin::init signature)
       plugin_arc.init(app)?;
       println!("[PluginRegistry] Plugin initialized: {}", id);

       // --- Register stages immediately after successful initialization ---
       println!("[PluginRegistry] Attempting to register stages for plugin: {}...", id);
       // Lock the registry using the Arc<Mutex<StageRegistry>> directly
       let mut registry_guard = stage_registry_arc.lock().await;
       println!("[PluginRegistry] StageRegistry locked for plugin: {}", id);
       match plugin_arc.register_stages(&mut *registry_guard) { // Call the plugin's method
            Ok(_) => {
                println!("[PluginRegistry] plugin.register_stages call succeeded for plugin: {}", id);
             }
             Err(e) => {
                 println!("[PluginRegistry] plugin.register_stages call FAILED for plugin: {}: {}", id, e);
                 // Propagate the error
                 return Err(e);
             }
        }
        // Drop the guard explicitly *before* marking as initialized, although it would drop at end of scope anyway.
        drop(registry_guard);
        println!("[PluginRegistry] StageRegistry unlocked for plugin: {}", id);


        // Mark as initialized *after* successful init and stage registration
        self.initialized.insert(id.to_string());
        // --- Cycle Detection Cleanup ---
        currently_initializing.remove(id);
        // --- End Cycle Detection Cleanup ---

        Ok(()) }) // Close the async move block and Box::pin
    }

    /// Initialize all enabled plugins in dependency order, after checking for conflicts.
    /// Requires the Application instance (for Plugin::init) and the correct StageRegistry Arc.
    pub async fn initialize_all(
        &mut self,
        app: &mut Application,
        stage_registry_arc: &Arc<Mutex<StageRegistry>>, // Expect &Arc<Mutex<StageRegistry>>
    ) -> Result<()> {
        // 1. Detect conflicts among enabled plugins
        self.detect_all_conflicts()?;
        println!("[Init] Detected conflicts: {:?}", self.conflict_manager.get_conflicts());

        // 2. Check for critical unresolved conflicts
        if !self.conflict_manager.all_critical_conflicts_resolved() {
            let critical_conflicts = self.conflict_manager.get_critical_unresolved_conflicts();
            let conflict_details: Vec<String> = critical_conflicts
                .iter()
                .map(|c| format!("  - {} vs {}: {}", c.first_plugin, c.second_plugin, c.description))
                .collect();
            return Err(Error::Plugin(format!(
                "Cannot initialize plugins due to unresolved critical conflicts:\n{}",
                conflict_details.join("\n")
            )));
        }

        // 3. Get IDs of enabled plugins
        // TODO: Factor in plugins disabled by conflict resolution?
        let enabled_plugin_ids: HashSet<String> = self.enabled.iter().cloned().collect();
        if enabled_plugin_ids.is_empty() {
             println!("[Init] No enabled plugins to initialize.");
             return Ok(());
        }

        // 4. Build dependency graph for enabled plugins
        println!("[Init] Building dependency graph for enabled plugins: {:?}", enabled_plugin_ids);
        let (adj, _reverse_adj) = self.build_dependency_graph(&enabled_plugin_ids);
        println!("[Init] Adjacency List: {:?}", adj);


        // 5. Perform Topological Sort and Cycle Detection
        println!("[Init] Performing topological sort...");
        let sorted_plugin_ids = match self.topological_sort(&enabled_plugin_ids, &adj) {
             Ok(sorted_ids) => {
                 println!("[Init] Topological sort successful. Order: {:?}", sorted_ids);
                 sorted_ids
             }
             Err(dep_err) => {
                 // Map DependencyError to kernel::Error::Plugin
                 let err_msg = format!("Dependency resolution failed: {}", dep_err); // Format the error
                 eprintln!("[Init] {}", err_msg);
                 return Err(Error::Plugin(err_msg)); // Wrap the formatted string
             }
        };

        // 6. Perform Enhanced Version Compatibility Check (Placeholder - Needs Implementation)
        // TODO: Implement check_transitive_version_constraints(&enabled_plugin_ids, &adj)?;
        println!("[Init] Skipping transitive version constraint check (TODO).");


        // 7. Initialize plugins in topological order
        println!("[Init] Initializing plugins in topological order...");
        for id in sorted_plugin_ids {
            // Check if already initialized (might happen due to dependency resolution within recursive calls)
            if !self.initialized.contains(&id) {
                 println!("[Init] Calling initialize_plugin for: {}", id);
                 // Call the public method, passing the app and the stage_registry_arc
                 match self.initialize_plugin(&id, app, stage_registry_arc).await {
                     Ok(_) => println!("[Init] Successfully initialized plugin: {}", id),
                     Err(e) => {
                         // If initialization fails for one plugin, stop the whole process?
                         // Or collect errors and report at the end? Let's stop for now.
                         eprintln!("[Init] Failed to initialize plugin {}: {}", id, e);
                         return Err(e);
                     }
                 }
            } else {
                 println!("[Init] Plugin {} already initialized (likely by a dependency), skipping.", id);
            }
        }

        println!("[Init] All enabled plugins initialized successfully.");
        Ok(())
    }
    
    /// Shutdown all plugins
    pub fn shutdown_all(&mut self) -> Result<()> {
        // Build dependency graph for initialized plugins
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        let mut reverse_adj: HashMap<String, Vec<String>> = HashMap::new(); // For reverse topological sort
        let initialized_plugin_ids: HashSet<String> = self.initialized.iter().cloned().collect();

        for id in &initialized_plugin_ids {
            adj.entry(id.clone()).or_default(); // Ensure all initialized plugins are in the graph
            reverse_adj.entry(id.clone()).or_default();
            if let Some(plugin) = self.plugins.get(id) {
                for dep in plugin.dependencies() {
                    // Only consider dependencies that are also initialized
                    if initialized_plugin_ids.contains(&dep.plugin_name) {
                        // Standard adjacency list: id depends on dep.plugin_name
                        adj.entry(id.clone()).or_default().push(dep.plugin_name.clone());
                        // Reverse adjacency list: dep.plugin_name is depended on by id
                        reverse_adj.entry(dep.plugin_name.clone()).or_default().push(id.clone());
                    }
                }
            }
        }

        // Perform topological sort (Kahn's algorithm)
        // We want to shut down plugins with *no dependents* first.
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for id in &initialized_plugin_ids {
             // Calculate in-degree based on the *reverse* graph (how many plugins depend on this one)
             in_degree.insert(id.clone(), reverse_adj.get(id).map_or(0, |dependents| dependents.len()));
        }

        // Queue contains nodes with in-degree 0 (in the reverse graph, meaning no other initialized plugin depends on them)
        let mut queue: std::collections::VecDeque<String> = initialized_plugin_ids
            .iter()
            .filter(|id| *in_degree.get(*id).unwrap_or(&1) == 0)
            .cloned()
            .collect();

        let mut shutdown_order = Vec::new(); // This will store the order: least dependent first
        while let Some(id) = queue.pop_front() {
// DEBUG LOGGING FOR CYCLE
            println!("[shutdown_all] Processing ID: {}", id);
            println!("[shutdown_all] Current Queue: {:?}", queue);
            println!("[shutdown_all] Current In-Degrees: {:?}", in_degree);
            println!("[shutdown_all] Current Shutdown Order: {:?}", shutdown_order);
            // END DEBUG LOGGING
            shutdown_order.push(id.clone());

            // For each plugin `dep_id` that the current plugin `id` depends on (using original adj list)
             if let Some(dependencies) = adj.get(&id) {
                 for dep_id in dependencies {
                     if let Some(degree) = in_degree.get_mut(dep_id) {
                         *degree -= 1;
                         if *degree == 0 {
                             queue.push_back(dep_id.clone());
                         }
                     }
                 }
             }
        }


        // Check if topological sort included all initialized plugins (cycle detection)
        if shutdown_order.len() != initialized_plugin_ids.len() {
            // This indicates a cycle among initialized plugins, which ideally shouldn't happen
            // if initialization succeeded, but handle defensively.
             return Err(Error::Plugin(
                 "Cyclic dependency detected among initialized plugins during shutdown sort.".to_string()
             ));
        }

        // Shutdown plugins in the calculated topological order.
        // The `shutdown_order` Vec contains plugins where dependencies appear *before*
        // the plugins that depend on them (e.g., A before B if B depends on A).
        // For shutdown, we need to process this list *as is* to ensure dependents
        // are shut down before their dependencies.
        let mut shutdown_errors = Vec::new();
        for id in shutdown_order { // Iterate normally
            // Explicitly convert id (&String) to &str using as_str()
            if let Some(plugin) = self.plugins.get(id.as_str()) {
                 // Check if it's still marked as initialized before shutting down
                 if self.initialized.contains(id.as_str()) { // Use as_str()
                    println!("Shutting down plugin: {}", id);
                    if let Err(e) = plugin.shutdown() {
                        let err_msg = format!("Error shutting down plugin {}: {}", id, e);
                        eprintln!("{}", err_msg);
                        shutdown_errors.push(err_msg);
                        // Continue shutting down others even if one fails
                    }
                    // Mark as uninitialized *after* attempting shutdown
                    self.initialized.remove(id.as_str()); // Use as_str()
                 }
            }
        }

        if shutdown_errors.is_empty() {
            Ok(())
        } else {
            Err(Error::Plugin(format!(
                "Encountered errors during plugin shutdown: {}",
                shutdown_errors.join("; ")
            )))
        }
    }
    
    /// Get all registered plugin IDs
    pub fn get_plugin_ids(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }
    
    /// Get the number of registered plugins
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
    
    /// Get the number of initialized plugins
    pub fn initialized_count(&self) -> usize {
        self.initialized.len()
    }
    
    /// Get the current API version
    pub fn api_version(&self) -> &ApiVersion {
        &self.api_version
    }
    
    /// Check for plugin dependencies
    pub fn check_dependencies(&self) -> Result<()> {
        for (id, plugin) in &self.plugins {
             // Only check dependencies for enabled plugins
             if !self.is_enabled(id) {
                 continue;
             }
            for dep in plugin.dependencies() {
                // Check if the dependency exists and is enabled
                let dep_exists = self.has_plugin(&dep.plugin_name);
                let dep_enabled = self.is_enabled(&dep.plugin_name);

                if dep.required && (!dep_exists || !dep_enabled) {
                    return Err(Error::Plugin(format!(
                        "Enabled plugin '{}' requires enabled plugin '{}', which is missing or disabled.",
                        id, dep.plugin_name
                    )));
                }

                // Check version compatibility if the dependency exists (regardless of enabled status for version check)
                if dep_exists {
                    if let Some(dep_plugin) = self.get_plugin(&dep.plugin_name) { // get_plugin returns Option<Arc<dyn Plugin>>
                        if let Some(ref required_range) = dep.version_range {
                            // Parse the dependency's actual version string into semver::Version
                            match semver::Version::parse(dep_plugin.version()) {
                                Ok(dep_version) => {
                                    // Use the includes method which takes semver::Version
                                    if !required_range.includes(&dep_version) {
                                        return Err(Error::Plugin(format!(
                                            "Plugin '{}' requires dependency '{}' version '{}', but found version '{}'",
                                            id,
                                            dep.plugin_name,
                                            required_range.constraint_string(), // Use constraint_string() for display
                                            dep_plugin.version()
                                        )));
                                    }
                                },
                                Err(e) => {
                                    // Error parsing the dependency's version string
                                    return Err(Error::Plugin(format!(
                                        "Failed to parse version string '{}' for dependency plugin '{}': {}",
                                        dep_plugin.version(), dep.plugin_name, e
                                    )));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

     /// Enable a plugin by ID
     pub fn enable_plugin(&mut self, id: &str) -> Result<()> {
         if !self.has_plugin(id) {
             return Err(Error::Plugin(format!("Cannot enable non-existent plugin: {}", id)));
         }
         self.enabled.insert(id.to_string());
         println!("Plugin {} enabled.", id);
         Ok(())
     }

     /// Disable a plugin by ID
     pub fn disable_plugin(&mut self, id: &str) -> Result<()> {
         if !self.has_plugin(id) {
             // Disabling a non-existent plugin might be considered a no-op or an error.
             // Let's treat it as a no-op for now, but log it.
             println!("Attempted to disable non-existent plugin: {}", id);
             return Ok(());
         }
         if self.initialized.contains(id) {
             // Ideally, we should shut down the plugin if it's initialized before disabling.
             // However, shutdown logic is complex (dependency order).
             // For now, we prevent disabling an initialized plugin.
             // TODO: Implement safe disabling of initialized plugins (requires shutdown).
             return Err(Error::Plugin(format!(
                 "Cannot disable plugin '{}' while it is initialized. Stop the application first.",
                 id
             )));
         }

         if self.enabled.remove(id) {
             println!("Plugin {} disabled.", id);
         } else {
             println!("Plugin {} was already disabled.", id);
         }
         Ok(())
     }

     /// Check if a plugin is enabled by ID
     pub fn is_enabled(&self, id: &str) -> bool {
         self.enabled.contains(id)
     }

    /// Get a reference to the conflict manager
    pub fn conflict_manager(&self) -> &ConflictManager {
        &self.conflict_manager
    }

    /// Get a mutable reference to the conflict manager
    pub fn conflict_manager_mut(&mut self) -> &mut ConflictManager {
        &mut self.conflict_manager
    }

    /// Detect all conflicts between currently enabled plugins.
    /// This clears previous conflicts and re-evaluates based on the current state.
    ///
    // Removed problematic line: (e.g., resource claims).
    pub fn detect_all_conflicts(&mut self) -> Result<()> {
        // Clear previous conflicts before re-detecting
        self.conflict_manager = ConflictManager::new();

        let enabled_plugin_ids: Vec<String> = self.enabled.iter().cloned().collect();
        let mut plugins_to_check: Vec<Arc<dyn Plugin>> = enabled_plugin_ids // Make mutable
            .iter()
            .filter_map(|id| self.plugins.get(id).cloned())
            .collect();

        // Sort plugins by ID for deterministic conflict checking order
        plugins_to_check.sort_by(|a, b| a.name().cmp(b.name()));

        if plugins_to_check.len() < 2 {
            return Ok(()); // No conflicts possible with less than 2 enabled plugins
        }

        for i in 0..plugins_to_check.len() {
            for j in (i + 1)..plugins_to_check.len() {
                let plugin_a = &plugins_to_check[i];
                let plugin_b = &plugins_to_check[j];
                let id_a = plugin_a.name();
                let id_b = plugin_b.name();

                // --- Specific Conflict Checks ---

                // 1. Check for Declared Mutual Exclusion (conflicts_with)
                if plugin_a.conflicts_with().contains(&id_b.to_string()) ||
                   plugin_b.conflicts_with().contains(&id_a.to_string()) {
                    self.conflict_manager.add_conflict(
                        PluginConflict::new(
                            id_a,
                            id_b,
                            ConflictType::MutuallyExclusive,
                            "Plugins are explicitly declared as conflicting.",
                        ),
                    );
                    // Continue to next pair if mutually exclusive, as incompatibility is implied
                    continue;
                }

                // 2. Check for Declared Incompatibilities (incompatible_with)
                // Check if A declares incompatibility with B
                for incompatibility in plugin_a.incompatible_with() {
                    if incompatibility.plugin_name == id_b { // Use plugin_name
                        let version_match = match &incompatibility.version_range {
                            Some(range) => {
                                match Version::parse(plugin_b.version()) {
                                    Ok(ver_b) => range.includes(&ver_b),
                                    Err(e) => {
                                        // Log error but potentially still flag as incompatible if version unknown/unparsable?
                                        // For now, let's log and skip the version check for this rule.
                                        eprintln!("Warning: Could not parse version '{}' for plugin '{}' during incompatibility check: {}", plugin_b.version(), id_b, e);
                                        false // Treat as non-matching if version parse fails
                                    }
                                }
                            },
                            None => true, // No version range means incompatible with *any* version
                        };

                        if version_match {
                            let description = match &incompatibility.version_range {
                                Some(range) => format!(
                                    "Plugin '{}' is explicitly incompatible with '{}' version '{}' (found version '{}').",
                                    id_a, id_b, range.constraint_string(), plugin_b.version()
                                ),
                                None => format!(
                                    "Plugin '{}' is explicitly incompatible with any version of plugin '{}'.",
                                    id_a, id_b
                                ),
                            };
                            self.conflict_manager.add_conflict(
                                PluginConflict::new(
                                    id_a,
                                    id_b,
                                    ConflictType::ExplicitlyIncompatible,
                                    &description,
                                ),
                            );
                            // Found incompatibility, no need to check B vs A for this specific rule type
                            // But we might still find other conflict types, so don't 'continue' outer loop yet.
                            break; // Move to next incompatibility rule for plugin_a
                        }
                    }
                }

                // Check if B declares incompatibility with A (avoid adding duplicate conflict entry if A already declared it)
                if !self.conflict_manager.has_conflict_between(id_a, id_b) {
                    for incompatibility in plugin_b.incompatible_with() {
                        if incompatibility.plugin_name == id_a { // Use plugin_name
                            let version_match = match &incompatibility.version_range {
                                Some(range) => {
                                    match Version::parse(plugin_a.version()) {
                                        Ok(ver_a) => range.includes(&ver_a),
                                        Err(e) => {
                                            eprintln!("Warning: Could not parse version '{}' for plugin '{}' during incompatibility check: {}", plugin_a.version(), id_a, e);
                                            false
                                        }
                                    }
                                },
                                None => true,
                            };

                            if version_match {
                                let description = match &incompatibility.version_range {
                                    Some(range) => format!(
                                        "Plugin '{}' is explicitly incompatible with '{}' version '{}' (found version '{}').",
                                        id_b, id_a, range.constraint_string(), plugin_a.version()
                                    ),
                                    None => format!(
                                        "Plugin '{}' is explicitly incompatible with any version of plugin '{}'.",
                                        id_b, id_a
                                    ),
                                };
                                self.conflict_manager.add_conflict(
                                    PluginConflict::new(
                                        id_a, // Keep order consistent (A, B)
                                        id_b,
                                        ConflictType::ExplicitlyIncompatible,
                                        &description,
                                    ),
                                );
                                break; // Move to next incompatibility rule for plugin_b
                            }
                        }
                    }
                }

                // 3. Placeholder: Resource Conflict (Keep for now, needs trait extension)
                //    This would ideally check a `resources()` method on the Plugin trait.
                //    Simulating based on common resource names in plugin IDs.
                if (id_a.contains("database") && id_b.contains("database")) ||
                   (id_a.contains("logger") && id_b.contains("logger")) {
                     // Avoid flagging the *exact* same plugin
                     if id_a != id_b && !self.conflict_manager.has_conflict_between(id_a, id_b) { // Avoid double-flagging
                        self.conflict_manager.add_conflict(
                            PluginConflict::new(
                                id_a,
                                id_b,
                                ConflictType::ResourceConflict, // Assuming this type exists
                                "Plugins might claim the same resource type (placeholder check).",
                            ),
                        );
                     }
                }

                // --- End Specific Conflict Checks ---

                // TODO: Add checks for DependencyVersion conflicts (e.g., A requires Dep X v1, B requires Dep X v2) - requires dependency graph analysis
                // TODO: Add checks for PartialOverlap conflicts (if applicable)
                // TODO: Add checks for Resource conflicts based on a future `resources()` trait method
            }
        }

        Ok(())
    }
} // Close impl PluginRegistry


