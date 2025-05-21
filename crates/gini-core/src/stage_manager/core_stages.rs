use std::collections::HashSet;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex; // Already present, ensure it's used

use crate::kernel::bootstrap::Application; // Needed for Plugin::init
use crate::kernel::error::Error as KernelError; // Renamed for clarity, KernelResult removed
use crate::plugin_system::registry::PluginRegistry;
use crate::stage_manager::{Stage, StageContext};
use crate::stage_manager::error::StageSystemError; // Import StageSystemError
use crate::stage_manager::registry::SharedStageRegistry; // Add SharedStageRegistry import

// Constants for context keys
pub const PLUGIN_REGISTRY_KEY: &str = "plugin_registry";
pub const PREFLIGHT_FAILURES_KEY: &str = "preflight_failures";
pub const APPLICATION_KEY: &str = "application"; // Key for Application reference

// --- Core Stage Definitions ---

// Note: Plugin dependency resolution is handled within the PluginInitializationStage.

/// Stage for running plugin pre-flight checks.
#[derive(Debug)]
pub struct PluginPreflightCheckStage;

#[async_trait]
impl Stage for PluginPreflightCheckStage {
    fn id(&self) -> &str { "core::plugin_preflight_check" }
    fn name(&self) -> &str { "Plugin Pre-flight Checks" }
    fn description(&self) -> &str { "Executes pre-initialization checks for all loaded plugins." }

    async fn execute(&self, context: &mut StageContext) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        println!("Executing Stage: {}", self.name());

        // Retrieve the plugin registry from the context
        let registry_arc_mutex = context
            .get_data::<Arc<Mutex<PluginRegistry>>>(PLUGIN_REGISTRY_KEY)
            .ok_or_else(|| StageSystemError::ContextError {
                key: PLUGIN_REGISTRY_KEY.to_string(),
                reason: "PluginRegistry not found".to_string()
            })?
            .clone(); // Clone the Arc to work with it

        let registry = registry_arc_mutex.lock().await;
        let mut failures = HashSet::new();
        // If a fundamental error occurs (e.g. context access), this will be set.
        // Individual plugin preflight failures are logged and added to `failures` set.
        // let stage_execution_error: Option<KernelError> = None; // Removed as per plan, direct error or Ok(())

        println!("Running pre-flight checks for {} plugins...", registry.iter_plugins().count());

        for (id, plugin) in registry.iter_plugins() {
            // Skip already failed plugins if this stage were to be re-run (though not typical)
            if failures.contains(id) {
                continue;
            }
            println!("  - Checking plugin: {}", plugin.name());
            match plugin.preflight_check(context).await {
                Ok(_) => {
                    println!("    - Pre-flight check PASSED for {}", plugin.name());
                }
                Err(e) => {
                    eprintln!("    - Pre-flight check FAILED for {}: {}", plugin.name(), e);
                    failures.insert(id.clone());
                    // We log the error and add to failures, but the stage itself doesn't fail here.
                    // The PluginInitializationStage will use the `failures` set.
                }
            }
        }

        // Store the set of failed plugin IDs in the context for the next stage
        context.set_data(PREFLIGHT_FAILURES_KEY, failures.clone()); // Store a clone

        println!("Pre-flight checks complete. {} plugins failed pre-flight.", failures.len());
        
        // The stage itself succeeds if it could iterate through plugins.
        // Individual preflight failures are handled by the next stage.
        // The stage itself succeeds if it could iterate through plugins.
        // Individual preflight failures are handled by the next stage.
        // Critical context errors are returned above.
        Ok(())
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

    async fn execute(&self, context: &mut StageContext) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        println!("Executing Stage: {}", self.name());

        // Retrieve immutable data first
        let registry_arc_mutex = context
            .get_data::<Arc<Mutex<PluginRegistry>>>(PLUGIN_REGISTRY_KEY)
            .ok_or_else(|| StageSystemError::ContextError {
                key: PLUGIN_REGISTRY_KEY.to_string(),
                reason: "PluginRegistry not found".to_string()
            })?
            .clone();
        let preflight_failures = context
            .get_data::<HashSet<String>>(PREFLIGHT_FAILURES_KEY)
            .map(|set_ref| set_ref.clone())
            .unwrap_or_default();
        let stage_registry_arc = context
             .get_data::<SharedStageRegistry>("stage_registry_arc") // Use SharedStageRegistry type alias
             .ok_or_else(|| StageSystemError::ContextError {
                key: "stage_registry_arc".to_string(),
                reason: "SharedStageRegistry not found".to_string()
            })?
             .clone();

        // Now get mutable access to Application
        let app = context.get_data_mut::<Application>(APPLICATION_KEY)
            .ok_or_else(|| StageSystemError::ContextError {
                key: APPLICATION_KEY.to_string(),
                reason: "Application not found or wrong type".to_string()
            })?;

        // Lock the plugin registry
        let mut registry = registry_arc_mutex.lock().await;

        // Disable plugins that failed pre-flight checks
        if !preflight_failures.is_empty() {
            println!("Disabling plugins that failed pre-flight checks: {:?}", preflight_failures);
            for failed_plugin_id in preflight_failures {
                // Pass the stage_registry_arc.registry (which is Arc<Mutex<StageRegistry>>)
                // The disable_plugin method expects &Arc<Mutex<StageRegistry>>
                match registry.disable_plugin(&failed_plugin_id, &stage_registry_arc.registry).await {
                    Ok(_) => println!("  - Plugin '{}' disabled due to pre-flight failure.", failed_plugin_id),
                    Err(e) => eprintln!("  - Warning: Failed to disable plugin '{}' after pre-flight failure: {}", failed_plugin_id, e),
                }
            }
        }

        // Call the registry's initialize_all method, passing the app and the StageRegistry Arc.
        // This method internally checks if plugins are enabled before initializing them.
        // It returns Result<_, PluginSystemError>, which needs to be boxed if we want to propagate it directly.
        // Or, map it to KernelError then box, or handle it here.
        // For now, let's map to KernelError then box.
        registry.initialize_all(app, &stage_registry_arc.registry).await.map_err(|e| Box::new(KernelError::from(e)) as Box<dyn std::error::Error + Send + Sync + 'static>)?;

        println!("Plugin initialization complete.");
        Ok(())
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

    async fn execute(&self, _context: &mut StageContext) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        println!("Executing Stage: {}", self.name());
        // This stage serves as a hook for any system-wide actions after all plugins are initialized.
        // or allow plugins to hook into this stage if necessary.
        // This might involve iterating through successfully initialized plugins.
        Ok(())
    }

     fn dry_run_description(&self, _context: &StageContext) -> String {
        format!("Would run post-initialization hooks for successfully initialized plugins.")
    }
}
// --- Core Pipeline Definitions ---
