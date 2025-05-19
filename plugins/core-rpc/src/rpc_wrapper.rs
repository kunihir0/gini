use chrono::{DateTime, Utc};
// discord_presence types are being replaced or are no longer directly used here.
// Some might be needed for InternalRichPresenceData if it's not fully replaced by JSON.
// For now, let's comment out what's clearly related to the old client.
// use discord_presence::{
//     client::Client as DiscordClient,
//     error::DiscordError,
//     event_handler::Context as EventContext,
//     models::{
//         rich_presence::{Activity, ActivityButton}, // Activity needed for apply_to_activity_old
//         EventData,
//     },
// };
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json; // Added for json! macro
use std::fmt;
use std::sync::Arc;
use std::thread;
use thiserror::Error;
use tokio::sync::Mutex as TokioMutex;
// use tokio::task::spawn_blocking; // May not be needed anymore for set_activity

// Assuming ipc_linux is in the same crate or a dependency
use crate::ipc_linux;

#[derive(Error, Debug)]
pub enum WrapperError {
    // #[error("Discord RPC client error: {0}")]
    // Discord(#[from] DiscordError), // This was for the old client
    #[error("RPC client thread is not running or already shut down.")]
    TaskNotRunning,
    #[error("RPC client thread panicked.")]
    TaskPanicked,
    #[error("Client ID is missing or invalid (must be u64).")]
    InvalidOrMissingClientId,
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Tokio task join error: {0}")]
    TokioJoinError(#[from] tokio::task::JoinError),
    #[error("Tokio runtime error: {0}")]
    TokioRuntime(String), // Added for runtime creation errors
    #[error("Raw Discord client error: {0}")]
    RawClient(String), // Changed from #[from] ipc_linux::RawClientError to String
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InternalRichPresenceData {
    pub details: Option<String>,
    pub state: Option<String>,
    pub start_timestamp: Option<i64>,
    pub end_timestamp: Option<i64>,
    pub large_image_key: Option<String>,
    pub large_image_text: Option<String>,
    pub small_image_key: Option<String>,
    pub small_image_text: Option<String>,
    pub party_id: Option<String>,
    pub party_size: Option<(i32, i32)>,
    pub buttons: Option<Vec<InternalButtonData>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InternalButtonData {
    pub label: String,
    pub url: String,
}

impl InternalRichPresenceData {
    // This method is no longer needed as we construct JSON payload directly.
    // fn apply_to_activity_old(&self, mut activity_builder: discord_presence::models::rich_presence::Activity) -> discord_presence::models::rich_presence::Activity { ... }
}

pub struct DiscordRpcWrapper {
    client_id: String,
    // pub(crate) client_handle: Option<DiscordClient>, // Removed
    pub(crate) raw_client_state: Arc<TokioMutex<Option<ipc_linux::RawDiscordClient>>>,
    pub(crate) current_presence_data: Arc<std::sync::Mutex<Option<InternalRichPresenceData>>>, // Keep std::sync::Mutex for now, assess if TokioMutex is needed here too
    rpc_run_thread_handle: Option<thread::JoinHandle<()>>,
}

impl DiscordRpcWrapper {
    pub fn new(client_id: String) -> Self {
        DiscordRpcWrapper {
            client_id,
            // client_handle: None, // Removed
            raw_client_state: Arc::new(TokioMutex::new(None)),
            current_presence_data: Arc::new(std::sync::Mutex::new(None)), // Keep std::sync::Mutex
            rpc_run_thread_handle: None,
        }
    }

    pub fn start_client_loop(&mut self) -> Result<(), WrapperError> {
        info!("[start_client_loop] Attempting to start client loop for client_id: {}", self.client_id);
        if self.client_id.is_empty() {
            error!("[start_client_loop] Client ID is empty.");
            return Err(WrapperError::InvalidOrMissingClientId);
        }
        // client_id is already a String, RawDiscordClient expects String. No parse to u64 needed.

        if self.rpc_run_thread_handle.is_some() {
            info!("[start_client_loop] RPC client thread is already running.");
            return Ok(());
        }

        let client_id_str = self.client_id.clone();
        let raw_client_state_clone_for_thread = Arc::clone(&self.raw_client_state);

        let handle = thread::spawn(move || {
            info!("[RPC OS Thread] Started for client ID: {}", client_id_str);

            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => {
                    info!("[RPC OS Thread] Tokio runtime created successfully.");
                    rt
                }
                Err(e) => {
                    error!("[RPC OS Thread] Failed to create Tokio runtime for RPC: {}", e);
                    return;
                }
            };
            
            info!("[RPC OS Thread] Entering runtime.block_on for RawDiscordClient::connect_and_run.");
            runtime.block_on(async {
                info!("[RPC OS Thread/async] Attempting RawDiscordClient::connect_and_run for client_id: {}", client_id_str);
                match ipc_linux::RawDiscordClient::connect_and_run(client_id_str.clone()).await {
                    Ok(client) => {
                        info!("[RPC OS Thread/async] RawDiscordClient::connect_and_run successful.");
                        let mut client_state_guard = raw_client_state_clone_for_thread.lock().await;
                        *client_state_guard = Some(client);
                        info!("[RPC OS Thread/async] RawDiscordClient stored in shared state. Read loop should be running. Parking OS thread.");
                        // Drop the guard before parking to avoid holding it indefinitely
                        drop(client_state_guard);
                        
                        // Park the OS thread to keep the RawDiscordClient (and its read_loop task) alive.
                        // The read_loop is spawned within connect_and_run.
                        // This thread will be unparked during shutdown.
                        loop {
                            std::thread::park();
                            // When unparked, we might check a shutdown flag if we had one,
                            // but for now, unparking is the signal to exit the loop.
                            // This assumes shutdown_rpc_sync will unpark and then join.
                            info!("[RPC OS Thread/async] Unparked. Likely for shutdown. Exiting parking loop.");
                            break;
                        }
                    }
                    Err(e) => {
                        error!("[RPC OS Thread/async] RawDiscordClient::connect_and_run failed: {}", e);
                        // client_state remains None
                    }
                }
            });
            info!("[RPC OS Thread] Exited runtime.block_on. OS Thread for client ID {} is now exiting.", client_id_str);
        });

        self.rpc_run_thread_handle = Some(handle);
        info!("[start_client_loop] Raw Discord RPC client loop initiated in a background OS thread for client_id: {}.", self.client_id);
        Ok(())
    }

    // Make this synchronous as Plugin::shutdown is synchronous
    pub fn shutdown_rpc_sync(&mut self) -> Result<(), WrapperError> {
        info!("[shutdown_rpc_sync] Attempting to shut down Raw Discord RPC Wrapper.");

        // Use the current Tokio runtime handle if available.
        // This method is called from Plugin::shutdown, which is synchronous but may be
        // running on a thread that is part of a Tokio runtime.
        let current_tokio_handle = match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle,
            Err(e) => {
                // Not in a Tokio runtime context, this is unexpected if called from main app shutdown.
                // However, if this happens, we can't block_on async calls easily without creating a new runtime,
                // which was the source of the original panic if we *were* in a runtime.
                // For now, log an error and attempt to unpark/join the thread without client shutdown.
                error!("[shutdown_rpc_sync] Failed to get current Tokio runtime handle: {}. RPC client may not be shut down cleanly.", e);
                if let Some(thread_handle) = self.rpc_run_thread_handle.take() {
                    thread_handle.thread().unpark();
                    match thread_handle.join() {
                        Ok(_) => info!("[shutdown_rpc_sync] RPC OS thread joined (no client shutdown)."),
                        Err(join_err) => error!("[shutdown_rpc_sync] RPC OS thread panicked during join (no client shutdown): {:?}", join_err),
                    }
                }
                return Err(WrapperError::TokioRuntime("Not in a Tokio runtime context during shutdown_rpc_sync".to_string()));
            }
        };

        // Take the client from raw_client_state
        let raw_client_option = current_tokio_handle.block_on(async {
            let mut guard = self.raw_client_state.lock().await;
            guard.take()
        });

        if let Some(raw_client) = raw_client_option { // Removed 'mut' as raw_client.shutdown takes &self
            info!("[shutdown_rpc_sync] RawDiscordClient instance taken. Calling its shutdown method.");
            match current_tokio_handle.block_on(raw_client.shutdown()) {
                Ok(_) => info!("[shutdown_rpc_sync] RawDiscordClient shutdown successful."),
                Err(e) => {
                    error!("[shutdown_rpc_sync] RawDiscordClient shutdown failed: {}", e);
                    // Continue with thread unparking and joining despite client shutdown error.
                }
            }
        } else {
            warn!("[shutdown_rpc_sync] No active RawDiscordClient found in state. Assuming already shut down or not started.");
        }

        if let Some(handle) = self.rpc_run_thread_handle.take() {
            info!("[shutdown_rpc_sync] Unparking and waiting for RPC OS thread to join...");
            handle.thread().unpark(); // Unpark the thread so it can exit its loop
            match handle.join() {
                Ok(_) => info!("[shutdown_rpc_sync] RPC OS thread joined successfully."),
                Err(e) => {
                    error!("[shutdown_rpc_sync] RPC OS thread panicked during shutdown or join: {:?}", e);
                    // Even if it panicked, we've attempted cleanup.
                    // Return TaskPanicked to indicate the issue.
                    return Err(WrapperError::TaskPanicked);
                }
            }
        } else {
            info!("[shutdown_rpc_sync] No OS thread handle found, thread might have already finished or was never started.");
        }

        // Clear presence data as well
        // Assuming current_presence_data uses std::sync::Mutex as per previous changes
        if let Ok(mut guard) = self.current_presence_data.lock() {
            guard.take();
            info!("[shutdown_rpc_sync] Current presence data cleared.");
        } else {
            error!("[shutdown_rpc_sync] Failed to lock current_presence_data to clear it.");
        }
        
        info!("[shutdown_rpc_sync] Raw Discord RPC Wrapper shutdown_rpc_sync completed.");
        Ok(())
    }

    // pub fn take_task_handle(&mut self) -> Option<thread::JoinHandle<()>> { // Method is unused
    //     self.rpc_run_thread_handle.take()
    // }

    // update_presence_activity, update_game_status, and clear_presence_activity are removed
    // as they were based on the old client. perform_update_activity_static will be the
    // primary method for updating presence with RawDiscordClient.
}

// Static-like method for use in spawned tasks to avoid holding MutexGuard across .await
impl DiscordRpcWrapper {
    #[allow(dead_code)] // May only be used by the lib.rs init's spawned task
    pub async fn perform_update_activity_static(
        raw_client_state_arc: Arc<TokioMutex<Option<ipc_linux::RawDiscordClient>>>,
        current_presence_data_arc: Arc<std::sync::Mutex<Option<InternalRichPresenceData>>>,
        details: Option<String>,
        state: Option<String>,
        start_timestamp: Option<DateTime<Utc>>,
        end_timestamp: Option<DateTime<Utc>>,
        large_image_key: Option<String>,
        large_image_text: Option<String>,
        small_image_key: Option<String>,
        small_image_text: Option<String>,
        party_id: Option<String>,
        party_size: Option<(i32, i32)>, // Discord expects (current_size, max_size)
        buttons: Option<Vec<InternalButtonData>>,
    ) -> Result<(), WrapperError> {
        let new_presence_data = InternalRichPresenceData {
            details: details.clone(), // Clone for comparison
            state: state.clone(),   // Clone for comparison
            start_timestamp: start_timestamp.map(|dt| dt.timestamp()),
            end_timestamp: end_timestamp.map(|dt| dt.timestamp()),
            large_image_key: large_image_key.clone(),
            large_image_text: large_image_text.clone(),
            small_image_key: small_image_key.clone(),
            small_image_text: small_image_text.clone(),
            party_id: party_id.clone(),
            party_size, // party_size is Copy if i32 is, but Option makes it not. Clone if necessary.
            buttons: buttons.clone(),
        };

        { // Scope for the std::sync::Mutex lock guard
            let mut current_data_guard = current_presence_data_arc.lock().unwrap(); // This is a std::sync::Mutex
            if *current_data_guard == Some(new_presence_data.clone()) {
                debug!("(Static) Presence data unchanged. No update sent.");
                return Ok(());
            }
            *current_data_guard = Some(new_presence_data.clone()); // Store the new data
        } // Guard dropped here

        let mut activity_payload_map = serde_json::Map::new();

        if let Some(d) = details {
            activity_payload_map.insert("details".to_string(), serde_json::Value::String(d));
        }
        if let Some(s) = state {
            activity_payload_map.insert("state".to_string(), serde_json::Value::String(s));
        }
        
        let mut timestamps_map = serde_json::Map::new();
        if let Some(start_ts) = start_timestamp.map(|dt| dt.timestamp()) {
            timestamps_map.insert("start".to_string(), serde_json::json!(start_ts));
        }
        if let Some(end_ts) = end_timestamp.map(|dt| dt.timestamp()) {
            timestamps_map.insert("end".to_string(), serde_json::json!(end_ts));
        }
        if !timestamps_map.is_empty() {
            activity_payload_map.insert("timestamps".to_string(), serde_json::Value::Object(timestamps_map));
        }

        let mut assets_map = serde_json::Map::new();
        if let Some(lik) = large_image_key {
            assets_map.insert("large_image".to_string(), serde_json::Value::String(lik));
        }
        if let Some(lit) = large_image_text {
            assets_map.insert("large_text".to_string(), serde_json::Value::String(lit));
        }
        if let Some(sik) = small_image_key {
            assets_map.insert("small_image".to_string(), serde_json::Value::String(sik));
        }
        if let Some(sit) = small_image_text {
            assets_map.insert("small_text".to_string(), serde_json::Value::String(sit));
        }
        if !assets_map.is_empty() {
            activity_payload_map.insert("assets".to_string(), serde_json::Value::Object(assets_map));
        }
        
        if let Some(pid) = party_id {
            let mut party_map = serde_json::Map::new();
            party_map.insert("id".to_string(), serde_json::Value::String(pid));
            if let Some(size_tuple) = party_size {
                 party_map.insert("size".to_string(), serde_json::json!([size_tuple.0, size_tuple.1]));
            }
            activity_payload_map.insert("party".to_string(), serde_json::Value::Object(party_map));
        }


        if let Some(button_data_vec) = buttons {
            if !button_data_vec.is_empty() {
                let json_buttons: Vec<serde_json::Value> = button_data_vec.iter().map(|b| {
                    serde_json::json!({
                        "label": b.label,
                        "url": b.url
                    })
                }).collect();
                activity_payload_map.insert("buttons".to_string(), serde_json::Value::Array(json_buttons));
            }
        }
        
        let activity_json_obj = serde_json::Value::Object(activity_payload_map);

        let final_payload = serde_json::json!({
            "pid": std::process::id() as u32, // Ensure PID is u32
            "activity": activity_json_obj
        });
        
        debug!("(Static) Constructed activity payload: {}", final_payload.to_string());

        let mut client_guard = raw_client_state_arc.lock().await;
        if let Some(raw_client) = client_guard.as_mut() {
            match raw_client.set_activity(final_payload).await {
                Ok(_) => {
                    debug!("(Static) Presence data updated via RawDiscordClient.");
                    Ok(())
                }
                Err(e) => {
                    error!("(Static) Failed to set activity via RawDiscordClient: {}", e);
                    Err(WrapperError::RawClient(e))
                }
            }
        } else {
            warn!("(Static) RawDiscordClient not available (None in Arc<Mutex<Option<...>>>). Cannot update presence.");
            Err(WrapperError::TaskNotRunning) // Or a more specific error
        }
    }
}


impl fmt::Display for DiscordRpcWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Check raw_client_state's inner Option to see if client is initialized
        // This requires an async block or try_lock if we want to be non-blocking here.
        // For simplicity in a Display impl, we might just indicate the potential.
        // A full check would be:
        // let is_initialized = self.raw_client_state.try_lock().map_or(false, |guard| guard.is_some());
        // However, try_lock might not be what we want if it's contended.
        // Let's just indicate the thread handle status for now.
        write!(
            f,
            "DiscordRpcWrapper (ClientID: {}, RawClient Potentially Initialized, Thread Running: {})",
            self.client_id,
            // self.raw_client_state.lock().await.is_some(), // Cannot await in sync fn
            self.rpc_run_thread_handle.as_ref().map_or(false, |h| !h.is_finished())
        )
    }
}