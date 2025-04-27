
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::kernel::bootstrap::Application; // Needed for Plugin::init
use crate::kernel::error::{Error, Result}; // Keep kernel::Result for stage execute return
use crate::plugin_system::registry::PluginRegistry;
use crate::plugin_system::traits::PluginError; // Ensure this path is correct
use crate::stage_manager::{Stage, StageContext, StageResult};

// Constants for context keys
pub const PLUGIN_REGISTRY_KEY: &str = "plugin_registry";
pub const PREFLIGHT_FAILURES_KEY: &str = "preflight_failures";
pub const APPLICATION_KEY: &str = "application"; // Key for Application reference

// --- Core Stage Definitions ---

// TODO: Define PluginDependencyResolution stage if needed as an explicit stage

/// Stage for running plugin pre-flight checks.
#[derive(Debug)]
pub struct PluginPreflightCheckStage;

#[async_trait]
impl Stage for PluginPreflightCheckStage {
    fn id(&self) -> &str { "core::plugin_preflight_check" }
    fn name(&self) -> &str { "Plugin Pre-flight Checks" }
    fn description(&self) -> &str { "Executes pre-initialization checks for all loaded plugins." }

    async fn execute(&self, context: &mut StageContext) -> Result<()> {
        println!("Executing Stage: {}", self.name());

        // Retrieve the plugin registry from the context
        let registry_arc_mutex = context
            .get_data::<Arc<Mutex<PluginRegistry>>>(PLUGIN_REGISTRY_KEY)
            .ok_or_else(|| Error::Stage(format!("'{}' not found in StageContext", PLUGIN_REGISTRY_KEY)))?
            .clone(); // Clone the Arc to work with it

        let registry = registry_arc_mutex.lock().await;
        let mut failures = HashSet::new();
        let mut overall_result: Result<()> = Ok(());

        println!("Running pre-flight checks for {} plugins...", registry.iter_plugins().count());

        for (id, plugin) in registry.iter_plugins() {
            println!("  - Checking plugin: {}", plugin.name());
            match plugin.preflight_check(context).await {
                Ok(_) => {
                    println!("    - Pre-flight check PASSED for {}", plugin.name());
                }
                Err(e) => {
                    eprintln!("    - Pre-flight check FAILED for {}: {}", plugin.name(), e);
                    // Store the specific error? For now, just mark as failed.
                    failures.insert(id.clone());
                    // Collect the first error to return if any fail
                    if overall_result.is_ok() {
                         overall_result = Err(Error::Plugin(format!(
                            "Pre-flight check failed for plugin '{}': {}", id, e
                        )));
                    }
                }
            }
        }

        // Store the set of failed plugin IDs in the context for the next stage
        context.set_data(PREFLIGHT_FAILURES_KEY, failures);

        println!("Pre-flight checks complete.");
        // Return Ok even if some checks failed; the initialization stage will handle skipping.
        // However, if a critical error occurred during the process (like context access), propagate that.
        // Let's adjust: return the first pre-flight error encountered, but still store all failures.
        overall_result
    }

    fn dry_run_description(&self, context: &StageContext) -> String {
        // Attempt to get plugin count for a more informative message
        let count = context
            .get_data::<Arc<Mutex<PluginRegistry>>>(PLUGIN_REGISTRY_KEY)
            .map(|reg_arc| {
                // Try to lock briefly, but don't block/panic in dry run description
                reg_arc.try_lock().map(|reg| reg.iter_plugins().count()).unwrap_or(0)
            })
            .unwrap_or(0);
        format!("Would execute pre-flight checks for {} loaded plugins.", count)
    }
}

/// Stage for initializing plugins (calling Plugin::init).
#[derive(Debug)]
pub struct PluginInitializationStage;

#[async_trait]
impl Stage for PluginInitializationStage {
    fn id(&self) -> &str { "core::plugin_initialization" }
    fn name(&self) -> &str { "Plugin Initialization" }
    fn description(&self) -> &str { "Initializes all plugins that passed previous checks." }

    async fn execute(&self, context: &mut StageContext) -> Result<()> {
        println!("Executing Stage: {}", self.name());

        // Retrieve the plugin registry and failure set
        let registry_arc_mutex = context
            .get_data::<Arc<Mutex<PluginRegistry>>>(PLUGIN_REGISTRY_KEY)
            .ok_or_else(|| Error::Stage(format!("'{}' not found in StageContext", PLUGIN_REGISTRY_KEY)))?
            .clone();
        let failures = context
            .get_data::<HashSet<String>>(PREFLIGHT_FAILURES_KEY)
            .cloned() // Clone the HashSet if found
            .unwrap_or_default(); // Default to empty set if not found

        // TODO: This stage needs mutable access to the Application object to pass to init.
        // Assuming it's stored in the context for now. This needs proper implementation.
        let app_ref_maybe = context.get_data_mut::<Application>(APPLICATION_KEY);
        let app = app_ref_maybe.ok_or_else(|| Error::Stage(format!("'{}' not found or wrong type in StageContext", APPLICATION_KEY)))?;


        let registry = registry_arc_mutex.lock().await;
        let mut overall_result: Result<()> = Ok(());

        println!("Initializing plugins (skipping {} failed pre-flight checks)...", failures.len());

        for (id, plugin) in registry.iter_plugins() {
            if failures.contains(id) {
                println!("  - Skipping initialization for {} (failed pre-flight check)", plugin.name());
                continue;
            }

            println!("  - Initializing plugin: {}", plugin.name());
            match plugin.init(app) { // Pass the mutable app reference
                Ok(_) => {
                    println!("    - Initialization PASSED for {}", plugin.name());
                    // Test-specific tracker logic removed from main code.
                }
                Err(e) => {
                    eprintln!("    - Initialization FAILED for {}: {}", plugin.name(), e);
                    // How to handle init failures? Mark plugin as disabled? Stop pipeline?
                    // For now, log and collect the first error.
                     if overall_result.is_ok() {
                         overall_result = Err(Error::Plugin(format!(
                            "Initialization failed for plugin '{}': {}", id, e
                        )));
                    }
                    // Consider adding to a different failure set? e.g., "init_failures"
                }
            }
        }

        println!("Plugin initialization complete.");
        overall_result // Return the first initialization error encountered
    }

     fn dry_run_description(&self, context: &StageContext) -> String {
         let total_count = context
            .get_data::<Arc<Mutex<PluginRegistry>>>(PLUGIN_REGISTRY_KEY)
            .map(|reg_arc| reg_arc.try_lock().map(|reg| reg.iter_plugins().count()).unwrap_or(0))
            .unwrap_or(0);
         let failure_count = context
            .get_data::<HashSet<String>>(PREFLIGHT_FAILURES_KEY)
            .map(|f| f.len())
            .unwrap_or(0);
        format!("Would initialize {} plugins (skipping {} due to failed pre-flight checks).", total_count - failure_count, failure_count)
    }
}

/// Stage for running post-initialization logic.
#[derive(Debug)]
pub struct PluginPostInitializationStage;

#[async_trait]
impl Stage for PluginPostInitializationStage {
    fn id(&self) -> &str { "core::plugin_post_initialization" }
    fn name(&self) -> &str { "Plugin Post-Initialization" }
    fn description(&self) -> &str { "Executes logic after all plugins have been initialized." }

    async fn execute(&self, _context: &mut StageContext) -> Result<()> {
        println!("Executing Stage: {}", self.name());
        // TODO: Implement logic if any core post-init actions are needed,
        // or allow plugins to hook into this stage if necessary.
        // This might involve iterating through successfully initialized plugins.
        Ok(())
    }

     fn dry_run_description(&self, _context: &StageContext) -> String {
        // TODO: Potentially list plugins that would run post-init hooks.
        format!("Would run post-initialization hooks for successfully initialized plugins.")
    }
}