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
use chrono::{Utc, DateTime}; // Added DateTime
use gini_core::event::{Event, AsyncEventHandler, EventResult}; // Ensure EventError is not imported if not used
use serde::{Serialize, Deserialize}; // For event serialization if needed

// use discord_presence::client::Client as DiscordClient; // No longer needed here
// use std::future::Future; // Not needed
// use std::pin::Pin;     // Not needed


// --- New Event Definitions for Stage Updates ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StageActivityStatus {
    Started,
    InProgress, // Could carry progress percentage in the future
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageStatusUpdateEventPayload {
    pub stage_id: String,
    pub stage_name: String,
    pub status: StageActivityStatus,
    pub timestamp: DateTime<Utc>,
    pub message: Option<String>, // For progress details or error messages
}

#[derive(Debug, Clone)]
pub struct StageStatusUpdateEvent {
    pub payload: StageStatusUpdateEventPayload,
}

impl StageStatusUpdateEvent {
    pub fn new(payload: StageStatusUpdateEventPayload) -> Self {
        Self { payload }
    }
}

// Corrected Event trait implementation based on compiler errors
impl Event for StageStatusUpdateEvent {
    fn name(&self) -> &'static str {
        "StageStatusUpdateEvent"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

// The EventPayload trait and its import were removed as they were causing unresolved import errors
// and the primary issue was with the `Event` trait implementation.
// If EventPayload is a necessary concept in Gini for event data, its correct usage/definition
// will need to be ascertained from Gini's core event system. For now, data will be accessed
// via `as_any` and downcasting.


// --- Stage Update Event Handler ---

#[allow(dead_code)] // Silencing warning as registration is deferred due to Gini API constraints
#[derive(Debug)]
struct StageUpdateEventHandler {
    rpc_wrapper_handle: Arc<TokioMutex<Option<DiscordRpcWrapper>>>,
}

impl StageUpdateEventHandler {
    #[allow(dead_code)] // Silencing warning as construction is deferred
    fn new(rpc_wrapper_handle: Arc<TokioMutex<Option<DiscordRpcWrapper>>>) -> Self {
        Self { rpc_wrapper_handle }
    }
}

// AsyncEventHandler is NOT generic.
// It does NOT have event_name().
// Its handle method takes &(dyn Event + 'static) and returns EventResult.
#[async_trait::async_trait]
impl AsyncEventHandler for StageUpdateEventHandler {
    // event_name() method removed entirely

    async fn handle(&self, event: &(dyn Event + 'static)) -> EventResult { // Corrected signature
        // Downcast the trait object to our specific event type.
        // The payload is inside StageStatusUpdateEvent.
        if let Some(concrete_event) = event.as_any().downcast_ref::<StageStatusUpdateEvent>() {
            let payload = &concrete_event.payload;
            info!("[StageUpdateEventHandler] Received StageStatusUpdateEvent: ID='{}', Name='{}', Status='{:?}'",
                payload.stage_id, payload.stage_name, payload.status);

            let details: Option<String>;
            let state: Option<String>;
            let start_timestamp: Option<DateTime<Utc>> = Some(payload.timestamp); // Default to event time, removed mut
            let mut end_timestamp: Option<DateTime<Utc>> = None; // end_timestamp is mutable as it's set conditionally

            match payload.status {
                StageActivityStatus::Started => {
                    details = Some(format!("Stage: {}", payload.stage_name));
                    state = Some(payload.message.clone().unwrap_or_else(|| "Starting...".to_string()));
                    // In a more advanced version, we'd store payload.timestamp as the start time for this stage_id
                }
                StageActivityStatus::InProgress => {
                    details = Some(format!("Stage: {}", payload.stage_name));
                    state = Some(payload.message.clone().unwrap_or_else(|| "In Progress...".to_string()));
                    // If we had stored start time: start_timestamp = Some(get_stored_start_time(payload.stage_id));
                }
                StageActivityStatus::Completed => {
                    details = Some(format!("Stage: {}", payload.stage_name));
                    state = Some(payload.message.clone().unwrap_or_else(|| "Completed".to_string()));
                    end_timestamp = Some(payload.timestamp); // Mark end time
                    // If we had stored start time: start_timestamp = Some(get_stored_start_time(payload.stage_id));
                    // Then clear stored start time.
                }
                StageActivityStatus::Failed => {
                    details = Some(format!("Stage Failed: {}", payload.stage_name));
                    state = Some(payload.message.clone().unwrap_or_else(|| "Error".to_string()));
                    end_timestamp = Some(payload.timestamp); // Mark end time
                }
            }
            
            debug!("[StageUpdateEventHandler] Formatted presence: Details='{:?}', State='{:?}', Start='{:?}', End='{:?}'",
                   details, state, start_timestamp, end_timestamp);

            let rpc_wrapper_guard = self.rpc_wrapper_handle.lock().await;
            if let Some(wrapper_instance) = rpc_wrapper_guard.as_ref() {
                let raw_client_state_arc = Arc::clone(&wrapper_instance.raw_client_state);
                let current_presence_data_arc = Arc::clone(&wrapper_instance.current_presence_data);
                let client_ready_signal = Arc::clone(&wrapper_instance.client_ready_signal);
                
                drop(rpc_wrapper_guard); // Release lock before await

                // Ensure client is ready before attempting to update
                client_ready_signal.notified().await;

                if let Err(e) = DiscordRpcWrapper::perform_update_activity_static(
                    raw_client_state_arc,
                    current_presence_data_arc,
                    details,
                    state,
                    start_timestamp,
                    end_timestamp,
                    None, None, None, None, None, None, None, // Other fields not used for now
                ).await {
                    warn!("[StageUpdateEventHandler] Failed to update Discord presence for stage event: {}", e);
                } else {
                    info!("[StageUpdateEventHandler] Discord presence updated for stage event: ID='{}'", payload.stage_id);
                }
            } else {
                warn!("[StageUpdateEventHandler] DiscordRpcWrapper not available, cannot update presence for stage event ID='{}'.", payload.stage_id);
            }
            EventResult::Continue
        } else {
             error!("[StageUpdateEventHandler] Failed to downcast event. Expected StageStatusUpdateEvent.");
             EventResult::Continue
        }
    }
}

// --- End Stage Update Event Handler ---


// --- End New Event Definitions ---

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
        info!("Core RPC Plugin: Initializing...");
        let tokio_handle = Handle::current();

        let storage_manager_arc = app.storage_manager();
        let settings_for_task = Arc::clone(&self.settings);
        let rpc_wrapper_handle_clone = Arc::clone(&self.rpc_wrapper_handle);
        
        // Event handler registration is deferred.
        // let plugin_manager_for_async = app.plugin_manager(); // This Arc could be passed to the async task.

        info!("Core RPC Plugin: Queuing async setup for RPC client and initial presence.");

        tokio_handle.spawn(async move {
            let rpc_wrapper_handle = rpc_wrapper_handle_clone; 
            
            info!("Core RPC Plugin: Async task started.");

            // 1. Load Settings
            let final_settings = match load_settings(storage_manager_arc).await {
                Ok(s) => {
                    debug!("RPC Settings loaded: {:?}", s);
                    *settings_for_task.lock().unwrap() = Some(s.clone());
                    s
                }
                Err(e) => {
                    error!("Failed to load RPC settings: {}", e);
                    return; 
                }
            }; // Correctly closes match load_settings

            // 2. Start RPC Client if enabled
            if final_settings.enabled {
                if let Some(client_id) = &final_settings.client_id {
                    if !client_id.is_empty() {
                        info!("RPC is enabled with Client ID: {}. Starting DiscordRpcWrapper.", client_id);
                        let mut wrapper = DiscordRpcWrapper::new(client_id.clone());
                        
                        if let Err(e) = wrapper.start_client_loop() {
                            error!("Failed to start DiscordRpcWrapper: {}", e);
                            return; 
                        }
                        
                        let client_ready_signal_for_initial_update = Arc::clone(&wrapper.client_ready_signal);
                        
                        // Store the wrapper
                        let mut rpc_wrapper_guard = rpc_wrapper_handle.lock().await;
                        *rpc_wrapper_guard = Some(wrapper);
                        drop(rpc_wrapper_guard);
                        info!("DiscordRpcWrapper started and stored.");

                        // 3. Register Event Handler (Deferred)
                        // TODO: Implement event handler registration here.
                        warn!("Core RPC Plugin: Event handler registration for dynamic updates is deferred.");

                        // 4. Initial Presence Update
                        if final_settings.default_details.is_some() || final_settings.default_state.is_some() {
                            info!("Waiting for client ready signal before initial presence update...");
                            client_ready_signal_for_initial_update.notified().await;
                            info!("Client ready signal received. Proceeding with initial presence update.");

                            let rpc_wrapper_guard = rpc_wrapper_handle.lock().await;
                            if let Some(wrapper_instance) = rpc_wrapper_guard.as_ref() {
                                let raw_client_state_arc = Arc::clone(&wrapper_instance.raw_client_state);
                                let current_presence_data_arc = Arc::clone(&wrapper_instance.current_presence_data);
                                drop(rpc_wrapper_guard);

                                debug!("Setting initial presence: Details='{:?}', State='{:?}'", 
                                    final_settings.default_details, final_settings.default_state);
                                if let Err(e) = DiscordRpcWrapper::perform_update_activity_static(
                                    raw_client_state_arc,
                                    current_presence_data_arc,
                                    final_settings.default_details, 
                                    final_settings.default_state,   
                                    Some(Utc::now()), None, None, None, None, None, None, None, None
                                ).await {
                                    warn!("Failed to set initial Discord presence: {}", e);
                                }
                            } else {
                                 warn!("Cannot set initial presence: DiscordRpcWrapper became None unexpectedly after being stored.");
                            }
                        } // Correctly closes if for initial presence update
                    } else { // client_id is empty
                        info!("RPC is enabled, but Client ID is empty. RPC will not start.");
                    }
                } else { // no client_id
                    info!("RPC is enabled, but no Client ID is configured. RPC will not start.");
                }
            } else { // RPC not enabled
                info!("Discord RPC is disabled in settings.");
            } // Correctly closes if final_settings.enabled
        }); // Correctly closes tokio_handle.spawn(async move { ... })

        info!("Core RPC Plugin: Synchronous part of init complete. Async initialization task spawned.");
        Ok(())
    }

    fn shutdown(&self) -> Result<(), PluginSystemError> {
        info!("Core RPC Plugin: Attempting shutdown.");
        match self.rpc_wrapper_handle.try_lock() {
            Ok(mut guard) => {
                if let Some(mut wrapper) = guard.take() {
                    info!("Core RPC Plugin: Acquired wrapper instance. Shutting down DiscordRpcWrapper synchronously.");
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
            }
        }
        
        info!("Core RPC Plugin: Synchronous part of shutdown method complete.");
        Ok(())
    }
    
    fn register_stages(&self, _registry: &mut gini_core::stage_manager::registry::StageRegistry) -> Result<(), PluginSystemError> {
        Ok(())
    }
}