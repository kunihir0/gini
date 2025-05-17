mod cli; // Declare the cli module

use gini_core::kernel::bootstrap::Application;
// use gini_core::kernel::error::Error; // Import Error
// use gini_core::storage::DefaultStorageManager; // Import DefaultStorageManager
use gini_core::stage_manager::{StageManager, StageContext, StageResult}; // Remove unused StagePipeline
use gini_core::stage_manager::core_stages::STARTUP_ENV_CHECK_PIPELINE; // Added import
use clap::{Parser, Subcommand}; // Use clap for argument parsing
use std::sync::Arc; // Use Arc for shared ownership of the connector
use log::{info, error}; // Added logging imports

// --- Import Core Plugins for Static Registration ---
use core_environment_check::EnvironmentCheckPlugin; // Corrected name
use core_logging::LoggingPlugin; // Corrected name
// --- End Core Plugin Imports ---

/// Gini: A modular application framework
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// Simple ping command for testing - REMOVE LATER? Or keep for basic check?
    #[arg(long)]
    ping: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Manage plugins
    Plugin {
        #[command(subcommand)]
        command: PluginCommand,
    },
    /// Run a specific stage by its ID
    RunStage {
        /// The ID of the stage to run
        stage_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum PluginCommand {
    /// List registered plugins
    List {},
    /// Enable a plugin (persist setting)
    Enable {
        /// The name of the plugin to enable
        name: String
    },
    /// Disable a plugin (persist setting)
    Disable {
        /// The name of the plugin to disable
        name: String
    },
}


#[tokio::main]
async fn main() {
    println!("OSX-Forge: QEMU/KVM Deployment System");

    // Parse command-line arguments
    let args = CliArgs::parse();
    // println!("Parsed args: {:?}", args); // Remove debug print

    // Handle simple ping command
    if args.ping {
        println!("pong");
        return; // Exit after pong
    }

    println!("Initializing application...");
    
    // Create the application instance
    // Note: App initialization happens even for commands that might not need the full app running.
    // This is acceptable for now, as we need the initialized components (like PluginManager).
    // Could be optimized later if needed.
    let mut app = match Application::new() {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Failed to initialize application: {}", e);
            return;
        }
    };

    // --- Statically Register Core Plugins ---
    // This needs to happen after app init but before commands that might rely on these plugins.
    println!("Registering static core plugins...");
    let plugin_manager = app.plugin_manager(); // Get PluginManager Arc
    let registry_arc = plugin_manager.registry(); // Get Registry Arc<Mutex>
    { // Scope for the MutexGuard
        let mut registry = registry_arc.lock().await; // Lock the registry

        // Instantiate and register core-logging
        let logging_plugin = Arc::new(LoggingPlugin); // Corrected name
        if let Err(e) = registry.register_plugin(logging_plugin) {
            eprintln!("Fatal: Failed to register core-logging plugin: {}", e);
            // Decide if we should exit here. For core plugins, probably yes.
            return;
        }
        println!("  - Registered: core-logging");

        // Instantiate and register core-environment-check
        let env_check_plugin = Arc::new(EnvironmentCheckPlugin); // Corrected name
        if let Err(e) = registry.register_plugin(env_check_plugin) {
            eprintln!("Fatal: Failed to register core-environment-check plugin: {}", e);
            return;
        }
        println!("  - Registered: core-environment-check");
    } // MutexGuard dropped here
    println!("Static core plugins registered.");
    // --- End Static Plugin Registration ---

    // --- Initialize All Registered Plugins ---
    // This ensures register_stages is called for statically registered plugins
    // before any command tries to use those stages.
    println!("Initializing all registered plugins...");
    { // Scope for MutexGuard
        let mut registry = registry_arc.lock().await; // Lock plugin registry
        // Get the StageManager to retrieve its StageRegistry Arc
        let stage_manager = app.stage_manager(); // Assuming this returns Arc<impl StageManager>
        let stage_registry_arc = stage_manager.registry().registry(); // Assuming registry() -> SharedStageRegistry -> registry() -> Arc<Mutex<StageRegistry>>

        // initialize_all requires mutable app and the StageRegistry Arc
        if let Err(e) = registry.initialize_all(&mut app, &stage_registry_arc).await {
             eprintln!("Fatal: Failed to initialize plugins after static registration: {}", e);
             return; // Exit if initialization fails
        }
    } // MutexGuard dropped
    println!("All plugins initialized.");
    // --- End Plugin Initialization ---

    // --- Run Startup Pipeline ---
    info!("Running startup environment check pipeline...");
    let stage_manager = app.stage_manager();
    let stage_ids: Vec<String> = STARTUP_ENV_CHECK_PIPELINE.stages.iter().map(|s| s.to_string()).collect();
    match stage_manager.create_pipeline(
        STARTUP_ENV_CHECK_PIPELINE.name,
        STARTUP_ENV_CHECK_PIPELINE.description.unwrap_or("Startup Check"), // Provide default if None
        stage_ids
    ).await {
        Ok(mut startup_pipeline) => {
            // Get config_dir from StorageManager component
            let storage_manager_opt = app.get_component::<gini_core::storage::DefaultStorageManager>().await;
            let storage_manager = match storage_manager_opt {
                Some(sm) => sm,
                None => {
                    eprintln!("Fatal: StorageManager component not found during startup pipeline execution.");
                    return; // Exit if storage manager is missing
                }
            };
            let mut context = StageContext::new_live(storage_manager.config_dir().to_path_buf());
            match stage_manager.execute_pipeline(&mut startup_pipeline, &mut context).await {
                Ok(_) => info!("Startup environment check pipeline completed successfully."),
                Err(e) => error!("Startup environment check pipeline failed during execution: {:?}", e),
            }
        }
        Err(e) => {
            error!("Failed to create startup environment check pipeline: {:?}", e);
            // Decide if this is fatal. For now, log and continue.
        }
    }
    // --- End Startup Pipeline ---


    // --- Command Handling ---
    match args.command {
        Some(Commands::Plugin { command }) => {
            match command {
                PluginCommand::List {} => {
                    println!("Listing registered plugins:");
                    let plugin_manager = app.plugin_manager(); // Get PluginManager Arc
                    let registry_arc = plugin_manager.registry(); // Get Registry Arc<Mutex>
                    let registry = registry_arc.lock().await; // Lock the registry

                    if registry.iter_plugins().next().is_none() {
                        println!("  No plugins registered.");
                    } else {
                        for (id, plugin_arc) in registry.iter_plugins() {
                            let status = if registry.is_enabled(id) { "Enabled" } else { "Disabled" };
                            println!("  - Name: {}, Version: {}, Status: {}", plugin_arc.name(), plugin_arc.version(), status);
                        }
                    }
                    // Command handled, exit successfully
                    return;
                }
                PluginCommand::Enable { name } => {
                    println!("Attempting to enable plugin '{}'...", name);
                    let plugin_manager = app.plugin_manager(); // Get PluginManager Arc
                    match plugin_manager.persist_enable_plugin(&name).await {
                        Ok(_) => {
                            println!("Successfully marked plugin '{}' as enabled.", name);
                            // Note: Actual loading happens on next app start based on persisted state.
                        }
                        Err(e) => {
                            eprintln!("Error enabling plugin '{}': {}", name, e);
                        }
                    }
                    // Command handled, exit successfully
                    return;
                }
                PluginCommand::Disable { name } => {
                    println!("Attempting to disable plugin '{}'...", name);
                    let plugin_manager = app.plugin_manager(); // Get PluginManager Arc
                    match plugin_manager.persist_disable_plugin(&name).await {
                        Ok(_) => {
                            println!("Successfully marked plugin '{}' as disabled.", name);
                            // Note: Actual unloading/prevention happens on next app start based on persisted state.
                        }
                        Err(e) => {
                            eprintln!("Error disabling plugin '{}': {}", name, e);
                        }
                    }
                    // Command handled, exit successfully
                    return;
                }
            }
        }
        Some(Commands::RunStage { stage_id }) => {
            println!("Attempting to run stage '{}'...", stage_id);
            let stage_manager = app.stage_manager(); // Get StageManager Arc

            // Create a simple pipeline containing just the requested stage_id
            // create_pipeline validates the stage ID exists in the registry
            let pipeline_name = format!("run-{}", stage_id);
            let pipeline_desc = format!("Run single stage: {}", stage_id);
            let mut pipeline = match stage_manager.create_pipeline(&pipeline_name, &pipeline_desc, vec![stage_id.clone()]).await {
                 Ok(p) => p,
                 Err(e) => {
                     eprintln!("Error creating pipeline for stage '{}': {}", stage_id, e);
                     return; // Exit if pipeline creation fails (e.g., stage not found)
                 }
            };

            // Create a default context for execution in live mode
            // TODO: Allow context customization via CLI args later?
            // Get config_dir from StorageManager component
            let storage_manager_opt = app.get_component::<gini_core::storage::DefaultStorageManager>().await;
             let storage_manager = match storage_manager_opt {
                Some(sm) => sm,
                None => {
                    eprintln!("Fatal: StorageManager component not found when running stage '{}'.", stage_id);
                    return; // Exit if storage manager is missing
                }
            };
            let config_dir = storage_manager.config_dir().to_path_buf();
            let mut context = StageContext::new_live(config_dir); // Use new_live

            // Execute the pipeline
            // Note: execute_pipeline is async, so we need .await
            // Note: App initialization (which registers core stages) happens before this match block.
            match stage_manager.execute_pipeline(&mut pipeline, &mut context).await {
                Ok(results) => {
                    println!("Pipeline execution finished for stage '{}'. Results:", stage_id);
                    // Print results for clarity (iterate over reference)
                    for (id, result) in &results {
                        println!("  - {}: {}", id, result);
                    }
                    // Check if the specific stage succeeded (use reference)
                    if let Some(StageResult::Success) = pipeline.stages().first().and_then(|id| results.get(id)) {
                         println!("Stage '{}' completed successfully.", stage_id);
                    } else {
                         eprintln!("Stage '{}' did not complete successfully.", stage_id);
                         // Consider exiting with an error code?
                    }
                }
                Err(e) => {
                    eprintln!("Error executing pipeline for stage '{}': {}", stage_id, e);
                    // Consider exiting with an error code?
                }
            }
            // Command handled, exit successfully
            return;
        }
        None => {
            // No command specified, proceed with default app run
            println!("No command specified, running default application loop...");
            // Create and register the CLI interface
            let cli_interface = Box::new(cli::CliInterface); // Instantiate the interface
            if let Err(e) = app.ui_manager_mut().register_interface(cli_interface) {
                eprintln!("Failed to register CLI interface: {}", e);
                // Decide if this is fatal. For now, log and continue, but the UI might not work.
            } else {
                println!("CLI interface registered.");
            }

            // Run the application's main loop
            let run_result = app.run().await;
            if let Err(e) = run_result {
                eprintln!("Application error: {}", e);
            }
        }
        // Add other top-level Commands here later
    }

    println!("Shutting down application...");
}
