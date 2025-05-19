use gini_core::kernel::bootstrap::Application;
// use gini_core::kernel::KernelComponent; // Not directly used

// Minimal event imports, some might become unused after full cleanup
use gini_core::event::{EventPriority as GiniEventPriority, EventResult as GiniEventResult}; 
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
// use std::any::Any; // Not needed after removing event impl
use tokio::sync::Mutex as TokioMutex; // Single import
use thiserror::Error;
use tokio::runtime::Handle;
use chrono::Utc;
use discord_presence::client::Client as DiscordClient; 
// use std::future::Future; // Not needed
// use std::pin::Pin;     // Not needed


// --- Event Definitions REMOVED ---
// All event-related structs (EnvironmentDataForRpc, EnvironmentInfoUpdatedEvent)
// and their `impl Event` have been removed to clean up the file.
// Event-based integration with core-environment-check will be revisited.
// --- End Event Definitions ---

mod rpc_wrapper;
mod settings;

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
    wrapper_state: Arc<TokioMutex<Option<(DiscordRpcWrapper, Option<std::thread::JoinHandle<()>>)>>>,
}

impl CoreRpcPlugin {
    pub fn new() -> Self {
        Self {
            settings: Arc::new(std::sync::Mutex::new(None)),
            wrapper_state: Arc::new(TokioMutex::new(None)),
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
        let wrapper_state_for_task = Arc::clone(&self.wrapper_state);
        
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
                                        let thread_handle = wrapper.take_task_handle();
                                        let mut w_state_guard = wrapper_state_for_task.lock().await;
                                        *w_state_guard = Some((wrapper, thread_handle));
                                        drop(w_state_guard);
                                        
                                        info!("DiscordRpcWrapper started successfully (async task).");

                                        let presence_details_from_settings = final_settings.default_details.clone();
                                        let presence_state_from_settings = final_settings.default_state.clone();

                                        if presence_details_from_settings.is_some() || presence_state_from_settings.is_some() {
                                            let extracted_data_for_update: Option<(DiscordClient, Arc<std::sync::Mutex<Option<rpc_wrapper::InternalRichPresenceData>>>)> = {
                                                let guard = wrapper_state_for_task.lock().await; 
                                                if let Some((wrapper_instance, _)) = guard.as_ref() {
                                                    if let Some(client_handle) = wrapper_instance.client_handle.clone() {
                                                        Some((client_handle, Arc::clone(&wrapper_instance.current_presence_data)))
                                                    } else { 
                                                        warn!("Initial presence update: client_handle is None after start_client_loop (inside lock).");
                                                        None 
                                                    }
                                                } else { 
                                                    warn!("Initial presence update: wrapper_instance is None in wrapper_state_arc (inside lock).");
                                                    None 
                                                }
                                            };
                        
                                            if let Some((cloned_client_handle, cloned_presence_data_arc)) = extracted_data_for_update {
                                                debug!("Setting initial presence via static call using settings defaults: Details='{:?}', State='{:?}'", 
                                                       presence_details_from_settings, presence_state_from_settings);
                                            
                                                if let Err(e) = DiscordRpcWrapper::perform_update_activity_static(
                                                    cloned_client_handle,
                                                    cloned_presence_data_arc,
                                                    presence_details_from_settings, 
                                                    presence_state_from_settings,   
                                                    Some(Utc::now()), None, None, None, None, None, None, None, None
                                                ).await {
                                                    warn!("Failed to set initial Discord presence in async task (static call): {}", e);
                                                }
                                            } else {
                                                warn!("Cannot set initial presence: Necessary data not available from wrapper_state for static call.");
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
        match self.wrapper_state.try_lock() {
            Ok(mut state_guard) => {
                if let Some((mut wrapper, thread_handle_opt)) = state_guard.take() {
                    info!("Core RPC Plugin: Acquired wrapper state, proceeding with detailed shutdown in a new OS thread.");
                    std::thread::spawn(move || {
                        info!("Core RPC Plugin (shutdown thread): Shutting down DiscordRpcWrapper...");
                        if let Err(e) = wrapper.shutdown_rpc_sync() {
                            error!("Core RPC Plugin (shutdown thread): Error during DiscordRpcWrapper shutdown: {}", e);
                        } else {
                            info!("Core RPC Plugin (shutdown thread): DiscordRpcWrapper shutdown signal processed.");
                        }

                        if let Some(os_thread_handle) = thread_handle_opt {
                            info!("Core RPC Plugin (shutdown thread): Waiting for RPC OS thread to join...");
                            if os_thread_handle.join().is_err() {
                                error!("Core RPC Plugin (shutdown thread): RPC OS thread panicked or failed to join.");
                            } else {
                                info!("Core RPC Plugin (shutdown thread): RPC OS thread joined successfully.");
                            }
                        }
                        info!("Core RPC Plugin (shutdown thread): Detailed shutdown complete.");
                    });
                } else {
                    info!("Core RPC Plugin: Wrapper already shut down or not initialized.");
                }
            }
            Err(_) => {
                warn!("Core RPC Plugin: Could not acquire lock on wrapper state during shutdown (try_lock failed). Skipping detailed RPC shutdown to avoid blocking. The RPC client OS thread might not be joined cleanly.");
            }
        }
        
        info!("Core RPC Plugin: Synchronous part of shutdown method complete.");
        Ok(())
    }
    
    fn register_stages(&self, _registry: &mut gini_core::stage_manager::registry::StageRegistry) -> Result<(), PluginSystemError> {
        Ok(())
    }
}