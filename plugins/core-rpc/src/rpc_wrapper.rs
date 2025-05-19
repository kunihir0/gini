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
use tokio::sync::{Mutex as TokioMutex, Notify}; // Added Notify
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

#[derive(Debug)] // Added Debug derive
pub struct DiscordRpcWrapper {
    client_id: String,
    // pub(crate) client_handle: Option<DiscordClient>, // Removed
    pub(crate) raw_client_state: Arc<TokioMutex<Option<ipc_linux::RawDiscordClient>>>,
    pub(crate) current_presence_data: Arc<std::sync::Mutex<Option<InternalRichPresenceData>>>,
    rpc_run_thread_handle: Option<thread::JoinHandle<()>>,
    pub(crate) client_ready_signal: Arc<Notify>, // Added for signaling readiness
}

impl DiscordRpcWrapper {
    pub fn new(client_id: String) -> Self {
        DiscordRpcWrapper {
            client_id,
            // client_handle: None, // Removed
            raw_client_state: Arc::new(TokioMutex::new(None)),
            current_presence_data: Arc::new(std::sync::Mutex::new(None)),
            rpc_run_thread_handle: None,
            client_ready_signal: Arc::new(Notify::new()), // Initialize the signal
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
        let client_ready_signal_clone = Arc::clone(&self.client_ready_signal); // Clone signal for the thread

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
                        info!("[RPC OS Thread/async] RawDiscordClient stored in shared state.");
                        // Drop the guard before notifying and parking
                        drop(client_state_guard);

                        client_ready_signal_clone.notify_one(); // Signal that client is ready
                        info!("[RPC OS Thread/async] Client ready signal sent. Read loop should be running. Parking OS thread.");
                        
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

        let raw_client_state_clone = Arc::clone(&self.raw_client_state);
        let rpc_run_thread_handle_option = self.rpc_run_thread_handle.take();
        let current_presence_data_clone = Arc::clone(&self.current_presence_data);

        // Spawn a new OS thread to handle asynchronous shutdown operations.
        // This new thread can safely create and block on its own Tokio runtime.
        let shutdown_thread_join_handle = std::thread::spawn(move || -> Result<(), WrapperError> {
            info!("[shutdown_rpc_sync/dedicated_thread] Dedicated shutdown thread started.");

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    error!("[shutdown_rpc_sync/dedicated_thread] Failed to create Tokio runtime for shutdown: {}", e);
                    WrapperError::TokioRuntime(format!("Failed to create dedicated shutdown runtime: {}", e))
                })?;

            // Perform async shutdown of the client
            let client_shutdown_result: Result<(), WrapperError> = runtime.block_on(async {
                let mut guard = raw_client_state_clone.lock().await;
                if let Some(raw_client) = guard.take() {
                    info!("[shutdown_rpc_sync/dedicated_thread] RawDiscordClient instance taken. Calling its shutdown method.");
                    if let Err(e_client_shutdown) = raw_client.shutdown().await {
                        error!("[shutdown_rpc_sync/dedicated_thread] RawDiscordClient shutdown failed: {}", e_client_shutdown);
                        return Err(WrapperError::RawClient(e_client_shutdown));
                    } else {
                        info!("[shutdown_rpc_sync/dedicated_thread] RawDiscordClient shutdown successful.");
                    }
                } else {
                    warn!("[shutdown_rpc_sync/dedicated_thread] No active RawDiscordClient found in state.");
                }
                Ok(())
            });

            // Log client shutdown error if any (by borrowing)
            if let Err(ref e) = client_shutdown_result {
                 error!("[shutdown_rpc_sync/dedicated_thread] Error during async client shutdown part: {:?}", e);
            }

            // Handle OS thread joining
            let mut os_thread_join_failed = false;
            if let Some(handle) = rpc_run_thread_handle_option {
                info!("[shutdown_rpc_sync/dedicated_thread] Unparking and waiting for original RPC OS thread to join...");
                handle.thread().unpark();
                match handle.join() {
                    Ok(_) => info!("[shutdown_rpc_sync/dedicated_thread] Original RPC OS thread joined successfully."),
                    Err(e_join_panic) => {
                        error!("[shutdown_rpc_sync/dedicated_thread] Original RPC OS thread panicked during join: {:?}", e_join_panic);
                        os_thread_join_failed = true;
                    }
                }
            } else {
                info!("[shutdown_rpc_sync/dedicated_thread] No OS thread handle found for original RPC thread.");
            }
            
            // Clear presence data
            if let Ok(mut guard) = current_presence_data_clone.lock() { // std::sync::Mutex
                guard.take();
                info!("[shutdown_rpc_sync/dedicated_thread] Current presence data cleared.");
            } else {
                error!("[shutdown_rpc_sync/dedicated_thread] Failed to lock current_presence_data to clear it.");
            }

            info!("[shutdown_rpc_sync/dedicated_thread] Dedicated shutdown thread finished.");

            // Determine final result
            if os_thread_join_failed {
                if client_shutdown_result.is_ok() {
                    // OS thread panicked, but client shutdown was successful
                    return Err(WrapperError::TaskPanicked);
                }
                // If client shutdown also failed, client_shutdown_result (which is an Err) will be returned below,
                // effectively prioritizing the client shutdown error message.
            }
            
            client_shutdown_result
        });

        // Wait for the dedicated shutdown thread to complete.
        match shutdown_thread_join_handle.join() {
            Ok(Ok(_)) => {
                info!("[shutdown_rpc_sync] Dedicated shutdown thread completed successfully.");
                Ok(())
            }
            Ok(Err(e)) => { // Error returned by the dedicated shutdown thread's logic
                error!("[shutdown_rpc_sync] Dedicated shutdown thread returned an error: {:?}", e);
                Err(e)
            }
            Err(_panic_info) => { // Dedicated shutdown thread panicked
                error!("[shutdown_rpc_sync] Dedicated shutdown thread panicked.");
                Err(WrapperError::TaskPanicked)
            }
        }
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
            warn!("(Static) RawDiscordClient not yet initialized or unavailable in Arc<Mutex<Option<...>>>. Cannot update presence.");
            Err(WrapperError::Internal("RawDiscordClient not ready for static update".to_string()))
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