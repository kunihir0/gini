use chrono::{DateTime, Utc};
use discord_presence::{
    client::Client as DiscordClient,
    error::DiscordError,
    event_handler::Context as EventContext,
    models::{
        // Removed: ErrorEvent, PartialUser, ReadyEvent
        rich_presence::{Activity, ActivityButton}, // Removed ActivityAssets, ActivityParty, ActivityTimestamps
        EventData,
        // Removed: Event as DiscordModelEvent,
    },
};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::thread;
use thiserror::Error;
use tokio::task::spawn_blocking;

#[derive(Error, Debug)]
pub enum WrapperError {
    #[error("Discord RPC client error: {0}")]
    Discord(#[from] DiscordError),
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
    fn apply_to_activity(&self, mut activity_builder: Activity) -> Activity {
        if let Some(details) = &self.details {
            activity_builder = activity_builder.details(details);
        }
        if let Some(state) = &self.state {
            activity_builder = activity_builder.state(state);
        }

        activity_builder = activity_builder.timestamps(|ts_builder| {
            let mut current_ts_builder = ts_builder;
            if let Some(start) = self.start_timestamp {
                current_ts_builder = current_ts_builder.start(start as u64);
            }
            if let Some(end) = self.end_timestamp {
                current_ts_builder = current_ts_builder.end(end as u64);
            }
            current_ts_builder
        });

        activity_builder = activity_builder.assets(|assets_builder| {
            let mut current_assets_builder = assets_builder;
            if let Some(key) = &self.large_image_key {
                current_assets_builder = current_assets_builder.large_image(key);
            }
            if let Some(text) = &self.large_image_text {
                current_assets_builder = current_assets_builder.large_text(text);
            }
            if let Some(key) = &self.small_image_key {
                current_assets_builder = current_assets_builder.small_image(key);
            }
            if let Some(text) = &self.small_image_text {
                current_assets_builder = current_assets_builder.small_text(text);
            }
            current_assets_builder
        });

        activity_builder = activity_builder.party(|party_builder| {
            let mut current_party_builder = party_builder;
            if let Some(id) = &self.party_id {
                current_party_builder = current_party_builder.id(id);
            }
            if let Some(size) = self.party_size {
                current_party_builder = current_party_builder.size((size.0 as u32, size.1 as u32));
            }
            current_party_builder
        });

        if let Some(buttons_data) = &self.buttons {
            if !buttons_data.is_empty() {
                let buttons_vec: Vec<ActivityButton> = buttons_data
                    .iter()
                    .map(|b_data| {
                        ActivityButton::new() 
                            .label(&b_data.label)
                            .url(&b_data.url)
                    })
                    .collect();
                activity_builder.buttons = buttons_vec;
            }
        }
        activity_builder
    }
}

pub struct DiscordRpcWrapper {
    client_id: String,
    pub(crate) client_handle: Option<DiscordClient>,
    pub(crate) current_presence_data: Arc<Mutex<Option<InternalRichPresenceData>>>,
    rpc_run_thread_handle: Option<thread::JoinHandle<()>>,
}

impl DiscordRpcWrapper {
    pub fn new(client_id: String) -> Self {
        DiscordRpcWrapper {
            client_id,
            client_handle: None,
            current_presence_data: Arc::new(Mutex::new(None)),
            rpc_run_thread_handle: None,
        }
    }

    pub fn start_client_loop(&mut self) -> Result<(), WrapperError> {
        if self.client_id.is_empty() {
            return Err(WrapperError::InvalidOrMissingClientId);
        }
        let client_id_u64 = self
            .client_id
            .parse::<u64>()
            .map_err(|_| WrapperError::InvalidOrMissingClientId)?;

        if self.rpc_run_thread_handle.is_some() {
            info!("RPC client thread is already running.");
            return Ok(());
        }

        let mut client_for_thread = DiscordClient::new(client_id_u64);

        client_for_thread.on_ready(move |ctx: EventContext| {
            match ctx.event {
                EventData::Ready(ready_event) => {
                    let user_name = ready_event.user.as_ref().and_then(|u| u.username.as_ref()).map_or("UnknownUser", |s| s.as_str());
                    info!("Discord RPC: Ready (User: {})", user_name);
                }
                _ => warn!("Received non-Ready EventData in on_ready callback."),
            }
        }).persist();

        client_for_thread.on_error(|ctx: EventContext| {
            match ctx.event {
                EventData::Error(error_event) => {
                    error!(
                        "Discord RPC Error (Callback): code {:?}, message '{}'",
                        error_event.code, error_event.message.as_deref().unwrap_or_default()
                    );
                }
                _ => error!("Received non-Error EventData in on_error callback. Raw: {:?}", ctx.event),
            }
        }).persist();

        client_for_thread.on_disconnected(|ctx: EventContext| {
             match ctx.event {
                EventData::Error(error_event) => {
                     warn!(
                        "Discord RPC Disconnected (Callback): code {:?}, message '{}'",
                        error_event.code, error_event.message.as_deref().unwrap_or_default()
                    );
                }
                EventData::None => {
                     warn!("Discord RPC Disconnected (Callback): No specific error data provided.");
                }
                _ => warn!("Received unexpected EventData in on_disconnected callback: {:?}", ctx.event),
            }
        }).persist();
        
        client_for_thread.on_connected(|_ctx: EventContext| {
            info!("Discord RPC: Successfully connected (or reconnected).");
        }).persist();
        
        self.client_handle = Some(client_for_thread.clone());

        let handle = thread::spawn(move || {
            info!("Discord client OS thread started for client ID: {}", client_id_u64);
            client_for_thread.start(); 
            info!("Discord client OS thread for client ID {} has finished.", client_id_u64);
        });
        self.rpc_run_thread_handle = Some(handle);
        info!("Discord RPC client loop initiated in a background OS thread.");
        Ok(())
    }

    // Make this synchronous as Plugin::shutdown is synchronous
    pub fn shutdown_rpc_sync(&mut self) -> Result<(), WrapperError> {
        info!("Attempting to shut down Discord RPC Wrapper (sync).");
        if let Some(client_to_shutdown) = self.client_handle.take() {
            // client.shutdown() is synchronous.
            // We are in a potentially async context (Plugin::shutdown called by async runtime)
            // but this specific method is now sync.
            // Directly calling client.shutdown() here.
            client_to_shutdown.shutdown().map_err(WrapperError::Discord)?;
            info!("Discord client shutdown signal sent.");

            if let Some(handle) = self.rpc_run_thread_handle.take() {
                info!("Waiting for RPC OS thread to join...");
                match handle.join() {
                    Ok(_) => info!("RPC OS thread joined successfully."),
                    Err(_) => {
                        error!("RPC OS thread panicked during shutdown or join.");
                        return Err(WrapperError::TaskPanicked);
                    }
                }
            }
            self.current_presence_data.lock().unwrap().take();
            info!("Discord RPC Wrapper shut down gracefully.");
            Ok(())
        } else {
            warn!("Shutdown called but no active client handle found.");
            Err(WrapperError::TaskNotRunning)
        }
    }
    
    pub fn take_task_handle(&mut self) -> Option<thread::JoinHandle<()>> {
        self.rpc_run_thread_handle.take()
    }

    pub async fn update_presence_activity(
        &self,
        details: Option<String>,
        state: Option<String>,
        start_timestamp: Option<DateTime<Utc>>,
        end_timestamp: Option<DateTime<Utc>>,
        large_image_key: Option<String>,
        large_image_text: Option<String>,
        small_image_key: Option<String>,
        small_image_text: Option<String>,
        party_id: Option<String>,
        party_size: Option<(i32, i32)>,
        buttons: Option<Vec<InternalButtonData>>,
    ) -> Result<(), WrapperError> {
        let new_presence_data = InternalRichPresenceData {
            details, state,
            start_timestamp: start_timestamp.map(|dt| dt.timestamp()),
            end_timestamp: end_timestamp.map(|dt| dt.timestamp()),
            large_image_key, large_image_text, small_image_key, small_image_text,
            party_id, party_size, buttons,
        };

        {
            let mut current_data_guard = self.current_presence_data.lock().unwrap();
            if *current_data_guard == Some(new_presence_data.clone()) {
                debug!("Presence data unchanged. No update sent.");
                return Ok(());
            }
            *current_data_guard = Some(new_presence_data.clone());
        }

        if let Some(client_handle) = &self.client_handle {
            let mut client_clone = client_handle.clone(); 
            let data_to_set = new_presence_data; 
            spawn_blocking(move || {
                debug!("Attempting to set activity via spawn_blocking.");
                client_clone.set_activity(|act_builder| data_to_set.apply_to_activity(act_builder))
            })
            .await?
            .map_err(WrapperError::Discord)?;
            debug!("Presence data updated via RPC.");
            Ok(())
        } else {
            Err(WrapperError::TaskNotRunning)
        }
    }

    #[allow(dead_code)] // This is part of the public API, may be used by other plugins/core
    pub async fn update_game_status(&self, app_name: &str, current_task: &str) -> Result<(), WrapperError> {
        self.update_presence_activity(
            Some(format!("Playing {}", app_name)),
            Some(current_task.to_string()),
            Some(Utc::now()),
            None,
            Some("gini_logo_large".to_string()), // Assuming this asset key exists
            Some(format!("Gini Framework - {}", app_name)),
            None, None, None, None, None,
        ).await
    }

    #[allow(dead_code)] // This is part of the public API, may be used by other plugins/core
    pub async fn clear_presence_activity(&self) -> Result<(), WrapperError> {
        {
            let mut current_data_guard = self.current_presence_data.lock().unwrap();
            if current_data_guard.is_none() {
                debug!("Presence already clear. No update sent.");
                return Ok(());
            }
            *current_data_guard = None;
        }

        if let Some(client_handle) = &self.client_handle {
            let mut client_clone = client_handle.clone();
            spawn_blocking(move || {
                debug!("Attempting to clear activity via spawn_blocking.");
                client_clone.clear_activity()
            })
            .await?
            .map_err(WrapperError::Discord)?;
            debug!("Presence cleared via RPC.");
            Ok(())
        } else {
            Err(WrapperError::TaskNotRunning)
        }
    }
}

// Static-like method for use in spawned tasks to avoid holding MutexGuard across .await
impl DiscordRpcWrapper {
    #[allow(dead_code)] // May only be used by the lib.rs init's spawned task
    pub async fn perform_update_activity_static(
        mut client_handle: DiscordClient, // Takes ownership of a cloned client
        current_presence_data_arc: Arc<Mutex<Option<InternalRichPresenceData>>>,
        details: Option<String>,
        state: Option<String>,
        start_timestamp: Option<DateTime<Utc>>,
        end_timestamp: Option<DateTime<Utc>>,
        large_image_key: Option<String>,
        large_image_text: Option<String>,
        small_image_key: Option<String>,
        small_image_text: Option<String>,
        party_id: Option<String>,
        party_size: Option<(i32, i32)>,
        buttons: Option<Vec<InternalButtonData>>,
    ) -> Result<(), WrapperError> {
        let new_presence_data = InternalRichPresenceData {
            details, state,
            start_timestamp: start_timestamp.map(|dt| dt.timestamp()),
            end_timestamp: end_timestamp.map(|dt| dt.timestamp()),
            large_image_key, large_image_text, small_image_key, small_image_text,
            party_id, party_size, buttons,
        };

        { // Scope for the lock guard
            let mut current_data_guard = current_presence_data_arc.lock().unwrap();
            if *current_data_guard == Some(new_presence_data.clone()) {
                debug!("(Static) Presence data unchanged. No update sent.");
                return Ok(());
            }
            *current_data_guard = Some(new_presence_data.clone());
        } // Guard dropped here

        // Use the passed client_handle
        let data_to_set = new_presence_data; // new_presence_data is cloned above, re-clone for spawn_blocking
        spawn_blocking(move || {
            debug!("(Static) Attempting to set activity via spawn_blocking.");
            // client_handle is moved into this closure
            client_handle.set_activity(|act_builder| data_to_set.apply_to_activity(act_builder))
        })
        .await? // JoinError
        .map_err(WrapperError::Discord)?; // DiscordError
        debug!("(Static) Presence data updated via RPC.");
        Ok(())
    }
}


impl fmt::Display for DiscordRpcWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DiscordRpcWrapper (ClientID: {}, ClientHandle Initialized: {}, Thread Running: {})",
            self.client_id,
            self.client_handle.is_some(),
            self.rpc_run_thread_handle.as_ref().map_or(false, |h| !h.is_finished())
        )
    }
}