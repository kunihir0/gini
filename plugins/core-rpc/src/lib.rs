use gini_core::kernel::bootstrap::Application;
// use gini_core::kernel::KernelComponent; // Not directly used

// Event imports removed as they are unused after cleanup
// use gini_core::event::{EventPriority as GiniEventPriority, EventResult as GiniEventResult};
use gini_core::plugin_system::{
    error::PluginSystemError,
    error::PluginSystemErrorSource,
    traits::{Plugin, PluginPriority},
    version::VersionRange,
    dependency::PluginDependency,
};
use gini_core::stage_manager::{
    // registry::StageRegistry, // Not used
    context::StageContext,
    requirement::StageRequirement,
};
use log::{error, info, warn, debug};
use std::sync::Arc;
// use std::any::Any; // Not needed
use tokio::sync::Mutex as TokioMutex; // Single import
use thiserror::Error;
use tokio::runtime::Handle;
use chrono::Utc;
// use discord_presence::client::Client as DiscordClient; // No longer needed here
// use std::future::Future; // Not needed
// use std::pin::Pin;     // Not needed


// --- Event Definitions REMOVED ---
// All event-related structs (EnvironmentDataForRpc, EnvironmentInfoUpdatedEvent)
// and their `impl Event` have been removed to clean up the file.
// Event-based integration with core-environment-check will be revisited.
// --- End Event Definitions ---

mod rpc_wrapper;
mod settings;
mod ipc_linux;

use rpc_wrapper::{DiscordRpcWrapper, WrapperError};
use settings::{load_settings, RpcSettings, SettingsError};

const PLUGIN_ID_STR: &str = "core-rpc";

#[derive(Error, Debug)]
pub enum CoreRpcError {
    #[error("RPC Wrapper error: {0}")]
    Wrapper(#[from] WrapperError),
    #[error("Settings error: {0}")]
    Settings(#[from] SettingsError),
    #[error("Lifecycle error: {0}")]
    Lifecycle(String),
    #[error("Tokio runtime error: {0}")]
    TokioRuntime(String),
}

impl From<CoreRpcError> for PluginSystemError {
    fn from(err: CoreRpcError) -> Self {
        let original_message = err.to_string();
        let source_variant = PluginSystemErrorSource::Other(err.to_string());
        
        PluginSystemError::InitializationError {
            plugin_id: PLUGIN_ID_STR.to_string(),
            message: original_message,
            source: Some(Box::new(source_variant)),
        }
    }
}

pub struct CoreRpcPlugin {
    settings: Arc<std::sync::Mutex<Option<RpcSettings>>>,
    rpc_wrapper_handle: Arc<TokioMutex<Option<DiscordRpcWrapper>>>,
}

impl CoreRpcPlugin {
    pub fn new() -> Self {
        Self {
            settings: Arc::new(std::sync::Mutex::new(None)),
            rpc_wrapper_handle: Arc::new(TokioMutex::new(None)),
        }
    }
}

// Event handler struct and its impl AsyncEventHandler removed.

#[async_trait::async_trait]
impl Plugin for CoreRpcPlugin {
    fn name(&self) -> &'static str {
        "Core RPC Plugin"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn is_core(&self) -> bool {
        true
    }

    fn priority(&self) -> PluginPriority {
        PluginPriority::Core(2)
    }

    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        use std::str::FromStr;
        const COMPATIBLE_API_REQ: &str = "^0.1.0";
        match VersionRange::from_str(COMPATIBLE_API_REQ) {
            Ok(vr) => vec![vr],
            Err(e) => {
                log::error!(
                    "CoreRpcPlugin: Failed to parse API version requirement ('{}'): {}",
                    COMPATIBLE_API_REQ,
                    e
                );
                vec![]
            }
        }
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![
            PluginDependency {
                plugin_name: "core-environment-check".to_string(),
                version_range: Some(VersionRange::from_constraint("^0.1.0").expect("Valid version constraint")),
                required: true,
            }
        ]
    }

    fn required_stages(&self) -> Vec<StageRequirement> {
        vec![]
    }

    fn conflicts_with(&self) -> Vec<String> {
        vec![]
    }

    fn incompatible_with(&self) -> Vec<PluginDependency> {
        vec![]
    }

    async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), PluginSystemError> {
        Ok(())
    }
    
    fn init(&self, app: &mut Application) -> Result<(), PluginSystemError> {
        info!("Core RPC Plugin: Initializing and queuing async setup.");
        let tokio_handle = Handle::current();

        let storage_manager_arc = app.storage_manager();
        let settings_for_task = Arc::clone(&self.settings);
        let rpc_wrapper_handle_for_task = Arc::clone(&self.rpc_wrapper_handle);
        
        // TODO: Event Handler Registration for dynamic presence updates using core-environment-check.
        // This functionality is deferred due to complexities in accessing EventManager synchronously
        // from Plugin::init or adapting the plugin initialization to be async.
        warn!("Core RPC Plugin: Dynamic presence updates via events (e.g., from core-environment-check) are currently deferred.");

        tokio_handle.spawn(async move {
            info!("Core RPC Plugin: Async initialization task started (main logic).");
            match load_settings(storage_manager_arc).await {
                Ok(final_settings) => {
                    debug!("RPC Settings loaded in async task: {:?}", final_settings);
                    *settings_for_task.lock().unwrap() = Some(final_settings.clone());

                    if final_settings.enabled {
                        if let Some(client_id) = &final_settings.client_id {
                            if !client_id.is_empty() {
                                info!("RPC is enabled with Client ID: {}. Starting DiscordRpcWrapper (async task).", client_id);
                                let mut wrapper = DiscordRpcWrapper::new(client_id.clone());
                                
                                match wrapper.start_client_loop() {
                                    Ok(_) => {
                                        // The thread_handle is now managed internally by DiscordRpcWrapper
                                        let mut rpc_wrapper_guard = rpc_wrapper_handle_for_task.lock().await;
                                        *rpc_wrapper_guard = Some(wrapper);
                                        // Important: Keep the guard only as long as needed.
                                        // For perform_update_activity_static, we need Arcs from the wrapper.
                                        // Let's re-acquire the lock or pass the necessary Arcs.
                                        // For simplicity, we'll re-acquire the lock to get the Arcs.
                                        drop(rpc_wrapper_guard); // Release lock before potential .await in perform_update
                                        
                                        info!("DiscordRpcWrapper started and stored successfully (async task).");

                                        let presence_details_from_settings = final_settings.default_details.clone();
                                        let presence_state_from_settings = final_settings.default_state.clone();

                                        if presence_details_from_settings.is_some() || presence_state_from_settings.is_some() {
                                            let rpc_wrapper_guard = rpc_wrapper_handle_for_task.lock().await;
                                            if let Some(wrapper_instance) = rpc_wrapper_guard.as_ref() {
                                                // Clone the Arcs needed for perform_update_activity_static
                                                let raw_client_state_arc = Arc::clone(&wrapper_instance.raw_client_state);
                                                let current_presence_data_arc = Arc::clone(&wrapper_instance.current_presence_data);
                                                let client_ready_signal_for_update = Arc::clone(&wrapper_instance.client_ready_signal);
                                                
                                                // Drop the lock guard *before* awaiting the ready signal
                                                drop(rpc_wrapper_guard);

                                                info!("Waiting for client ready signal before initial presence update...");
                                                client_ready_signal_for_update.notified().await; // Wait for the signal
                                                info!("Client ready signal received. Proceeding with initial presence update.");

                                                debug!("Setting initial presence via static call using settings defaults: Details='{:?}', State='{:?}'",
                                                       presence_details_from_settings, presence_state_from_settings);
                                            
                                                if let Err(e) = DiscordRpcWrapper::perform_update_activity_static(
                                                    raw_client_state_arc,
                                                    current_presence_data_arc,
                                                    presence_details_from_settings,
                                                    presence_state_from_settings,
                                                    Some(Utc::now()), // start_timestamp
                                                    None,             // end_timestamp
                                                    None,             // large_image_key
                                                    None,             // large_image_text
                                                    None,             // small_image_key
                                                    None,             // small_image_text
                                                    None,             // party_id
                                                    None,             // party_size
                                                    None              // buttons
                                                ).await {
                                                    warn!("Failed to set initial Discord presence in async task (static call) after ready signal: {}", e);
                                                }
                                            } else {
                                                 // This case should ideally not happen if wrapper was stored successfully
                                                 drop(rpc_wrapper_guard);
                                                 warn!("Cannot set initial presence: DiscordRpcWrapper not found in handle for static call even after start_client_loop.");
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to start DiscordRpcWrapper in async task: {}", e);
                                    }
                                }
                            } else {
                                info!("RPC is enabled, but Client ID is empty. RPC will not start (async task).");
                            }
                        } else {
                            info!("RPC is enabled, but no Client ID is configured. RPC will not start (async task).");
                        }
                    } else {
                        info!("Discord RPC is disabled in settings (async task).");
                    }
                }
                Err(e) => {
                    error!("Failed to load RPC settings in async task: {}", e);
                }
            }
        });

        info!("Core RPC Plugin: Synchronous part of init complete. Async initialization task spawned.");
        Ok(())
    }

    fn shutdown(&self) -> Result<(), PluginSystemError> {
        info!("Core RPC Plugin: Attempting shutdown.");
        // Use try_lock to avoid blocking the main thread if the async task in init still holds the lock.
        // The shutdown of the wrapper itself is synchronous internally now.
        match self.rpc_wrapper_handle.try_lock() {
            Ok(mut guard) => {
                if let Some(mut wrapper) = guard.take() {
                    info!("Core RPC Plugin: Acquired wrapper instance. Shutting down DiscordRpcWrapper synchronously.");
                    // The shutdown_rpc_sync method is synchronous and handles thread joining.
                    if let Err(e) = wrapper.shutdown_rpc_sync() {
                        error!("Core RPC Plugin: Error during DiscordRpcWrapper shutdown: {}", e);
                    } else {
                        info!("Core RPC Plugin: DiscordRpcWrapper shutdown_rpc_sync completed.");
                    }
                } else {
                    info!("Core RPC Plugin: Wrapper already shut down or not initialized.");
                }
            }
            Err(_) => {
                warn!("Core RPC Plugin: Could not acquire lock on rpc_wrapper_handle during shutdown (try_lock failed). RPC client might not be shut down cleanly if init task is still running.");
                // If we can't get the lock, it implies the init task might still be holding it or
                // it's being accessed elsewhere. Forcing a shutdown here could be problematic.
                // The `DiscordRpcWrapper`'s Drop implementation (if any) or OS might eventually clean up.
            }
        }
        
        info!("Core RPC Plugin: Synchronous part of shutdown method complete.");
        Ok(())
    }
    
    fn register_stages(&self, _registry: &mut gini_core::stage_manager::registry::StageRegistry) -> Result<(), PluginSystemError> {
        Ok(())
    }
}