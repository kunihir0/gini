use gini_core::kernel::bootstrap::Application;
// use gini_core::kernel::KernelComponent; // Marked as unused
use gini_core::plugin_system::{
    error::PluginSystemError,
    error::PluginSystemErrorSource, 
    traits::{Plugin, PluginPriority},
    version::VersionRange,
    dependency::PluginDependency,
};
use gini_core::stage_manager::{
    registry::StageRegistry,
    context::StageContext,
    requirement::StageRequirement,
};
// use gini_core::storage::manager::DefaultStorageManager; // This was the unused import
use log::{error, info, warn, debug};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex; // Add import for TokioMutex
use thiserror::Error;
use tokio::runtime::Handle;
// Removed: use std::error::Error as StdError;
use chrono::Utc;
use discord_presence::client::Client as DiscordClient; // Add import for DiscordClient

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
    #[error("StorageManager component is unavailable in the application.")]
    StorageManagerUnavailable,
    #[error("Tokio runtime error: {0}")]
    TokioRuntime(String),
    #[error("Plugin operation failed: {0}")]
    OperationFailed(String),
}

impl From<CoreRpcError> for PluginSystemError {
    fn from(err: CoreRpcError) -> Self {
        let original_message = err.to_string();
        // Convert CoreRpcError into a variant of PluginSystemErrorSource
        let source_variant = PluginSystemErrorSource::Other(err.to_string());
        
        PluginSystemError::InitializationError {
            plugin_id: PLUGIN_ID_STR.to_string(),
            message: original_message,
            source: Some(Box::new(source_variant)),
        }
    }
}

pub struct CoreRpcPlugin {
    settings: Arc<std::sync::Mutex<Option<RpcSettings>>>, // Explicitly std::sync::Mutex
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

#[async_trait::async_trait] // Add async_trait to the impl block
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
        use std::str::FromStr; // Required for VersionRange::from_str
        const COMPATIBLE_API_REQ: &str = "^0.1.0";
        match VersionRange::from_str(COMPATIBLE_API_REQ) {
            Ok(vr) => vec![vr],
            Err(e) => {
                log::error!(
                    "CoreRpcPlugin: Failed to parse API version requirement ('{}'): {}",
                    COMPATIBLE_API_REQ,
                    e
                );
                vec![] // Fallback to empty if parsing fails, will cause registration failure
            }
        }
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![]
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

    async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), PluginSystemError> { // Explicitly using std::result::Result
        Ok(())
    }
    
    fn init(&self, app: &mut Application) -> Result<(), PluginSystemError> {
        info!("Core RPC Plugin: Queuing asynchronous initialization.");
        let tokio_handle = Handle::current();

        // Get StorageManager Arc synchronously using the convenience accessor
        let storage_manager_arc = app.storage_manager();

        let settings_arc = Arc::clone(&self.settings);
        let wrapper_state_arc = Arc::clone(&self.wrapper_state);
        
        tokio_handle.spawn(async move {
            info!("Core RPC Plugin: Async initialization task started.");
            match load_settings(storage_manager_arc).await { // Pass the Arc directly
                Ok(final_settings) => {
                    debug!("RPC Settings loaded in async task: {:?}", final_settings);
                    // Store settings (settings_arc uses std::sync::Mutex, so .lock().unwrap() is fine)
                    *settings_arc.lock().unwrap() = Some(final_settings.clone());

                    if final_settings.enabled {
                        if let Some(client_id) = &final_settings.client_id {
                            if !client_id.is_empty() {
                                info!("RPC is enabled with Client ID: {}. Starting DiscordRpcWrapper (async task).", client_id);
                                let mut wrapper = DiscordRpcWrapper::new(client_id.clone());
                                
                                match wrapper.start_client_loop() { // This is sync
                                    Ok(_) => {
                                        let thread_handle = wrapper.take_task_handle();
                                        // Hold the wrapper and its OS thread handle
                                        // Now use .await for TokioMutex
                                        let mut w_state_guard = wrapper_state_arc.lock().await;
                                        *w_state_guard = Some((wrapper, thread_handle));
                                        drop(w_state_guard); // Explicitly drop guard after modification
                                        
                                        info!("DiscordRpcWrapper started successfully (async task).");

                                        // Initial presence update
                                        if let (Some(details), Some(state)) = (&final_settings.default_details, &final_settings.default_state) {
                                            let extracted_data_for_update: Option<(DiscordClient, Arc<std::sync::Mutex<Option<rpc_wrapper::InternalRichPresenceData>>>)> = {
                                                // Use .await for TokioMutex
                                                let guard = wrapper_state_arc.lock().await;
                                                if let Some((wrapper_instance, _)) = guard.as_ref() {
                                                    if let Some(client_handle) = wrapper_instance.client_handle.clone() {
                                                        Some((client_handle, Arc::clone(&wrapper_instance.current_presence_data)))
                                                    } else {
                                                        warn!("Initial presence update: client_handle is None after start_client_loop.");
                                                        None
                                                    }
                                                } else {
                                                    warn!("Initial presence update: wrapper_instance is None in wrapper_state_arc.");
                                                    None
                                                }
                                                // guard is dropped here (implicitly by scope if not explicitly)
                                            };
                        
                                            if let Some((cloned_client_handle, cloned_presence_data_arc)) = extracted_data_for_update {
                                                debug!("Setting initial presence via static call: Details='{}', State='{}'", details, state);
                                                let details_clone = details.clone();
                                                let state_clone = state.clone();
                                            
                                                if let Err(e) = DiscordRpcWrapper::perform_update_activity_static(
                                                    cloned_client_handle,
                                                    cloned_presence_data_arc, // This is Arc<std::sync::Mutex<...>>
                                                    Some(details_clone), Some(state_clone), Some(Utc::now()), None,
                                                    None, None, None, None, None, None, None
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
                                        // Consider how to propagate this error if critical. For now, just log.
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

        // Try to acquire the lock without blocking the current (Tokio worker) thread.
        // If it's contended, we might skip the detailed shutdown to avoid deadlocks/panics.
        match self.wrapper_state.try_lock() {
            Ok(mut state_guard) => {
                if let Some((mut wrapper, thread_handle_opt)) = state_guard.take() {
                    info!("Core RPC Plugin: Acquired wrapper state, proceeding with detailed shutdown in a new OS thread.");
                    // Spawn a new OS thread to perform blocking operations
                    std::thread::spawn(move || {
                        info!("Core RPC Plugin (shutdown thread): Shutting down DiscordRpcWrapper...");
                        if let Err(e) = wrapper.shutdown_rpc_sync() { // Call the synchronous version
                            error!("Core RPC Plugin (shutdown thread): Error during DiscordRpcWrapper shutdown: {}", e);
                        } else {
                            info!("Core RPC Plugin (shutdown thread): DiscordRpcWrapper shutdown signal processed.");
                        }

                        if let Some(os_thread_handle) = thread_handle_opt {
                            info!("Core RPC Plugin (shutdown thread): Waiting for RPC OS thread to join...");
                            if let Err(_e) = os_thread_handle.join() {
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
                // If try_lock fails, it means it's locked elsewhere.
                // We can't block here. The async task in init might still be holding it if shutdown is too quick,
                // or another part of the code. For now, we just log and move on.
            }
        }
        
        info!("Core RPC Plugin: Synchronous part of shutdown method complete.");
        Ok(())
    }
    
    fn register_stages(&self, _registry: &mut StageRegistry) -> Result<(), PluginSystemError> {
        Ok(())
    }

    
}