use std::env;
use std::fs;
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue; // Alias for clarity
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor; // Removed Read, Write
use log; // Added for logging macros
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Mutex, Notify};

// Discord RPC Data Structures

#[derive(Serialize, Debug)]
pub struct HandshakeRequest {
    pub v: i32,
    pub client_id: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DiscordHandshakeResponse {
    #[serde(rename = "cmd")] // Keep original name for deserialization
    pub _cmd: String,
    #[serde(default)]
    pub evt: Option<String>, // This field IS used
    #[serde(default)]
    pub data: Option<JsonValue>,
    #[serde(default, rename = "nonce")] // Keep original name for deserialization
    pub _nonce: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct DiscordErrorData {
    pub code: i32,
    pub message: String,
}

#[derive(Deserialize, Debug)]
pub struct DiscordErrorResponse {
    // evt is typically "ERROR" but let's capture it if present
    #[serde(default, rename = "evt")] // Keep original name for deserialization
    pub _evt: Option<String>, // Making this optional as per common error structures
    pub data: DiscordErrorData, // This field IS used
    #[serde(default, rename = "cmd")] // Keep original name for deserialization
    pub _cmd: Option<String>, // Sometimes errors might also echo a command
    #[serde(default, rename = "nonce")] // Keep original name for deserialization
    pub _nonce: Option<String>,
}

// Framing functions

/// Packs the opcode, payload length, and JSON payload into a byte vector.
/// Opcode: 4 bytes, little-endian
/// Length: 4 bytes, little-endian (length of the JSON string payload)
/// Payload: JSON string (UTF-8 encoded)
pub fn frame_message(opcode: u32, payload_json: &str) -> std::io::Result<Vec<u8>> {
    let payload_bytes = payload_json.as_bytes();
    let payload_len = payload_bytes.len() as u32;

    let mut frame = Vec::new();
    // Use WriteBytesExt for Vec<u8>
    WriteBytesExt::write_u32::<LittleEndian>(&mut frame, opcode)?;
    WriteBytesExt::write_u32::<LittleEndian>(&mut frame, payload_len)?;
    // Use std::io::Write for Vec<u8>
    std::io::Write::write_all(&mut frame, payload_bytes)?;

    Ok(frame)
}

/// Reads a framed message from the UnixStream.
/// Returns the opcode and the JSON payload string.
pub async fn read_framed_message(stream: &mut tokio::net::UnixStream) -> std::io::Result<(u32, String)> {
    let mut header_buf = [0u8; 8]; // 4 bytes for opcode, 4 bytes for length

    // Read the 8-byte header
    // stream is an AsyncRead, so stream.read_exact is fine.
    stream.read_exact(&mut header_buf).await?;
    
    let mut cursor = Cursor::new(header_buf);
    // Use ReadBytesExt for Cursor
    let opcode = ReadBytesExt::read_u32::<LittleEndian>(&mut cursor)?;
    let length = ReadBytesExt::read_u32::<LittleEndian>(&mut cursor)?;

    if length == 0 {
        // No payload, just opcode
        return Ok((opcode, String::new()));
    }

    // Read the payload
    let mut payload_buf = vec![0u8; length as usize];
    stream.read_exact(&mut payload_buf).await?;

    let payload_str = String::from_utf8(payload_buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Payload is not valid UTF-8: {}", e)))?;
    
    Ok((opcode, payload_str))
}


// Helper function to get current user's UID
fn get_current_uid() -> Option<u32> {
    // In a real scenario, you might use a crate like `users`
    // For this example, we'll try to parse it from `id -u` or simply return None
    // if not easily available without adding a dependency.
    // However, the prompt implies `users::get_current_uid()` so we'll assume it's available
    // For now, let's use a placeholder that would require the `users` crate.
    // To make this runnable without adding `users` crate immediately,
    // we can use a common default or an environment variable if available.
    // For a robust solution, `users::get_current_uid()` is preferred.
    // Let's simulate its presence for the logic.
    // In a real environment, you would add `users = "0.4"` to Cargo.toml
    // and uncomment the line below.
    // Some systems might not have the `users` crate available or it might be complex to get.
    // As a fallback, we can try to get it from `env::var("UID")` if set,
    // or use `unsafe { libc::getuid() }` if `libc` is a dependency.
    // Given the constraints, we'll proceed with a simplified approach.
    if let Ok(uid_str) = env::var("UID") {
        if let Ok(uid) = uid_str.parse::<u32>() {
            return Some(uid);
        }
    }
    // Fallback for when UID env var is not set or not a number
    // On most Linux systems, `id -u` would work, but executing commands is not ideal here.
    // The `users` crate is the idiomatic way: `users::get_current_uid()`
    // For now, let's assume we can get it. If not, this part of path search will be skipped.
    // This is a simplification as `users::get_current_uid()` is the proper way.
    // We'll proceed as if we can get it, to fulfill the logic requirement.
    // A more direct way without adding `users` crate immediately:
    #[cfg(unix)]
    {
        Some(unsafe { libc::getuid() })
    }
    #[cfg(not(unix))]
    {
        None // Placeholder for non-Unix where this logic isn't applicable
    }
}

/// Performs the initial handshake with the Discord client.
pub async fn perform_handshake(
    stream: &mut tokio::net::UnixStream,
    client_id_str: &str,
) -> Result<DiscordHandshakeResponse, String> {
    log::debug!("[ipc_linux::perform_handshake] Starting handshake with client_id: {}", client_id_str);

    // 1. Create HandshakeRequest payload
    let handshake_request = HandshakeRequest {
        v: 1,
        client_id: client_id_str.to_string(),
    };
    log::trace!("[ipc_linux::perform_handshake] Handshake request payload created: {:?}", handshake_request);

    // 2. Serialize to JSON
    let request_json = match serde_json::to_string(&handshake_request) {
        Ok(json) => json,
        Err(e) => {
            let err_msg = format!("Failed to serialize handshake request: {}", e);
            log::error!("[ipc_linux::perform_handshake] {}", err_msg);
            return Err(err_msg);
        }
    };
    log::trace!("[ipc_linux::perform_handshake] Handshake request serialized to JSON: {}", request_json);

    // 3. Frame with opcode 0
    let framed_message = match frame_message(0, &request_json) {
        Ok(frame) => frame,
        Err(e) => {
            let err_msg = format!("Failed to frame handshake message: {}", e);
            log::error!("[ipc_linux::perform_handshake] {}", err_msg);
            return Err(err_msg);
        }
    };
    log::trace!("[ipc_linux::perform_handshake] Handshake message framed, length: {}", framed_message.len());

    // 4. Write to UnixStream
    if let Err(e) = stream.write_all(&framed_message).await {
        let err_msg = format!("Failed to write handshake message to stream: {}", e);
        log::error!("[ipc_linux::perform_handshake] {}", err_msg);
        return Err(err_msg);
    }
    log::debug!("[ipc_linux::perform_handshake] Handshake message sent to stream.");

    // 5. Read response frame
    let (_opcode, response_json) = match read_framed_message(stream).await {
        Ok((op, json)) => {
            log::debug!("[ipc_linux::perform_handshake] Received response frame. Opcode: {}, JSON: {}", op, json);
            (op, json)
        }
        Err(e) => {
            let err_msg = format!("Failed to read handshake response from stream: {}", e);
            log::error!("[ipc_linux::perform_handshake] {}", err_msg);
            return Err(err_msg);
        }
    };

    // 6. Deserialize response
    log::trace!("[ipc_linux::perform_handshake] Attempting to deserialize response as DiscordHandshakeResponse: {}", response_json);
    match serde_json::from_str::<DiscordHandshakeResponse>(&response_json) {
        Ok(mut response) => {
            // Check if it's an ERROR event wrapped in a standard response structure
            if response.evt.as_deref() == Some("ERROR") {
                log::warn!("[ipc_linux::perform_handshake] Received response with evt: ERROR. Attempting to parse as DiscordErrorResponse. Raw JSON: {}", response_json);
                // Attempt to deserialize the same payload as DiscordErrorResponse
                match serde_json::from_str::<DiscordErrorResponse>(&response_json) {
                    Ok(error_response) => {
                        let err_msg = format!(
                            "Handshake failed with Discord error. Code: {}, Message: '{}'. Raw: {}",
                            error_response.data.code, error_response.data.message, response_json
                        );
                        log::error!("[ipc_linux::perform_handshake] {}", err_msg);
                        return Err(err_msg);
                    }
                    Err(e_parse_err) => {
                        // This case is tricky: evt was "ERROR" but it didn't fit DiscordErrorResponse.
                        // We will take the 'data' field from the initial parse if available.
                        if let Some(data_val) = response.data.take() {
                             // Try to get code and message from the generic data if possible
                            let code = data_val.get("code").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
                            let message = data_val.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown error content").to_string();
                             let err_msg_generic = format!(
                                "Handshake failed with Discord error (generic parsing from evt:ERROR data). Code: {}, Message: '{}'. Raw: {}",
                                code, message, response_json
                            );
                            log::error!("[ipc_linux::perform_handshake] {}", err_msg_generic);
                            return Err(err_msg_generic);
                        } else {
                            let err_msg_fallback = format!(
                                "Received evt: ERROR, but failed to parse as DiscordErrorResponse and no 'data' field in initial parse. Error: {}. Raw JSON: {}",
                                e_parse_err, response_json
                            );
                            log::error!("[ipc_linux::perform_handshake] {}", err_msg_fallback);
                            return Err(err_msg_fallback);
                        }
                    }
                }
            } else {
                // Not an evt: "ERROR", so consider it a successful handshake response for now.
                // The actual "READY" event will come later.
                log::info!("[ipc_linux::perform_handshake] Handshake successful (initial response received): {:?}", response);
                Ok(response)
            }
        }
        Err(e_handshake_parse) => {
            // Initial deserialization as DiscordHandshakeResponse failed.
            // Try deserializing as DiscordErrorResponse directly.
            log::warn!("[ipc_linux::perform_handshake] Failed to parse as DiscordHandshakeResponse: {}. Attempting to parse as DiscordErrorResponse. Raw JSON: {}", e_handshake_parse, response_json);
            match serde_json::from_str::<DiscordErrorResponse>(&response_json) {
                Ok(error_response) => {
                    let err_msg = format!(
                        "Handshake failed with Discord error. Code: {}, Message: '{}'. Raw: {}",
                        error_response.data.code, error_response.data.message, response_json
                    );
                    log::error!("[ipc_linux::perform_handshake] {}", err_msg);
                    Err(err_msg)
                }
                Err(e_error_parse) => {
                    let err_msg = format!(
                        "Failed to deserialize handshake response as DiscordHandshakeResponse ({}) and as DiscordErrorResponse ({}). Raw JSON: {}",
                        e_handshake_parse, e_error_parse, response_json
                    );
                    log::error!("[ipc_linux::perform_handshake] {}", err_msg);
                    Err(err_msg)
                }
            }
        }
    }
}


/// Searches for Discord IPC sockets on Linux.
///
/// Search locations (prioritized):
/// 1. Contents of the directory specified by the `$XDG_RUNTIME_DIR` environment variable.
/// 2. Contents of the directory `/run/user/{uid}` (where `{uid}` is the current user's ID).
/// 3. Contents of the directory specified by `$TMPDIR`.
/// 4. Contents of `/tmp`.
///
/// Within these directories, also checks common subdirectories like `snap.discord/`,
/// `app/com.discordapp.Discord/`, and `app/com.discordapp.DiscordCanary/` if they exist.
///
/// Returns the `PathBuf` of the first valid and existing socket found.
pub fn find_discord_ipc_path() -> Option<PathBuf> {
    let mut potential_base_paths: Vec<PathBuf> = Vec::new();

    // 1. $XDG_RUNTIME_DIR
    if let Ok(xdg_runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        potential_base_paths.push(PathBuf::from(xdg_runtime_dir));
    }

    // 2. /run/user/{uid}
    if let Some(uid) = get_current_uid() {
        potential_base_paths.push(PathBuf::from(format!("/run/user/{}", uid)));
    } else {
        eprintln!("[ipc_linux] Could not determine current UID for /run/user/{{uid}} path.");
    }

    // 3. $TMPDIR
    if let Ok(tmpdir) = env::var("TMPDIR") {
        potential_base_paths.push(PathBuf::from(tmpdir));
    }

    // 4. /tmp
    potential_base_paths.push(PathBuf::from("/tmp"));

    let subdirs_to_check = [
        "", // Check the base path itself
        "snap.discord/",
        "app/com.discordapp.Discord/",
        "app/com.discordapp.DiscordCanary/",
    ];

    for base_path in &potential_base_paths {
        for subdir in &subdirs_to_check {
            let current_search_path = base_path.join(subdir);
            if !current_search_path.exists() || !current_search_path.is_dir() {
                if !subdir.is_empty() { // Don't log for the base path itself if it doesn't exist
                    // eprintln!("[ipc_linux] Subdirectory not found or not a dir: {:?}", current_search_path);
                }
                continue;
            }
            eprintln!("[ipc_linux] Searching in: {:?}", current_search_path);

            for i in 0..=9 {
                let socket_name = format!("discord-ipc-{}", i);
                let potential_socket_path = current_search_path.join(&socket_name);
                eprintln!("[ipc_linux] Attempting to check socket: {:?}", potential_socket_path);

                match fs::metadata(&potential_socket_path) {
                    Ok(metadata) => {
                        if metadata.file_type().is_socket() {
                            eprintln!("[ipc_linux] Found Discord IPC socket at: {:?}", potential_socket_path);
                            return Some(potential_socket_path);
                        } else {
                            eprintln!("[ipc_linux] Path exists but is not a socket: {:?}", potential_socket_path);
                        }
                    }
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::NotFound {
                             eprintln!("[ipc_linux] Error checking path {:?}: {}", potential_socket_path, e);
                        }
                        // If not found, just continue to the next one.
                    }
                }
            }
        }
    }

    eprintln!("[ipc_linux] Discord IPC socket not found in any specified locations.");
    None
}

/// Asynchronously connects to the Unix domain socket at the given path.
pub async fn connect_ipc(path: &Path) -> std::io::Result<tokio::net::UnixStream> {
    eprintln!("[ipc_linux] Attempting to connect to IPC socket: {:?}", path);
    match tokio::net::UnixStream::connect(path).await {
        Ok(stream) => {
            eprintln!("[ipc_linux] Successfully connected to IPC socket: {:?}", path);
            Ok(stream)
        }
        Err(e) => {
            eprintln!("[ipc_linux] Failed to connect to IPC socket {:?}: {}", path, e);
            Err(e)
        }
    }
}

// New RawDiscordClient implementation

#[derive(Debug)]
pub struct RawDiscordClient {
    client_id: String,
    stream_arc: Arc<Mutex<Option<tokio::net::UnixStream>>>,
    is_connected: Arc<AtomicBool>,
    shutdown_signal: Arc<Notify>,
}

impl RawDiscordClient {
    pub async fn connect_and_run(client_id: String) -> Result<Self, String> {
        log::info!("[RawDiscordClient] Attempting to connect and run for client_id: {}", client_id);

        let ipc_path = find_discord_ipc_path()
            .ok_or_else(|| "Failed to find Discord IPC path".to_string())?;
        log::debug!("[RawDiscordClient] Found IPC path: {:?}", ipc_path);

        let mut stream = connect_ipc(&ipc_path)
            .await
            .map_err(|e| format!("Failed to connect to IPC socket: {}", e))?;
        log::debug!("[RawDiscordClient] Connected to IPC socket.");

        match perform_handshake(&mut stream, &client_id).await {
            Ok(_) => {
                log::info!("[RawDiscordClient] Handshake successful.");
            }
            Err(e) => {
                log::error!("[RawDiscordClient] Handshake failed: {}", e);
                return Err(format!("Handshake failed: {}", e));
            }
        }

        let stream_arc = Arc::new(Mutex::new(Some(stream)));
        let is_connected = Arc::new(AtomicBool::new(true));
        let shutdown_signal = Arc::new(Notify::new());

        let client = Self {
            client_id: client_id.clone(),
            stream_arc: stream_arc.clone(),
            is_connected: is_connected.clone(),
            shutdown_signal: shutdown_signal.clone(),
        };

        tokio::spawn(Self::read_loop(stream_arc, is_connected, shutdown_signal));
        log::info!("[RawDiscordClient] Read loop spawned.");

        Ok(client)
    }

    async fn read_loop(
        stream_arc: Arc<Mutex<Option<tokio::net::UnixStream>>>,
        is_connected: Arc<AtomicBool>,
        shutdown_signal: Arc<Notify>,
    ) {
        log::debug!("[RawDiscordClient::read_loop] Starting read loop.");
        loop {
            let mut stream_guard = stream_arc.lock().await;
            let stream_opt = stream_guard.as_mut();

            if stream_opt.is_none() {
                log::info!("[RawDiscordClient::read_loop] Stream is None, exiting loop.");
                break;
            }
            let stream = stream_opt.unwrap();

            tokio::select! {
                _ = shutdown_signal.notified() => {
                    log::info!("[RawDiscordClient::read_loop] Shutdown signal received, exiting loop.");
                    break;
                }
                read_result = read_framed_message(stream) => {
                    match read_result {
                        Ok((opcode, payload_json)) => {
                            log::info!("[RawDiscordClient::read_loop] Received message - Opcode: {}, Payload: {}", opcode, payload_json);
                            // Handle specific opcodes if necessary, e.g., CLOSE
                            if opcode == 2 { // Opcode for CLOSE
                                log::info!("[RawDiscordClient::read_loop] Received CLOSE frame from Discord. Payload: {}", payload_json);
                                // The server is closing the connection or has closed it.
                                // We should probably signal shutdown or at least mark as disconnected.
                                is_connected.store(false, Ordering::SeqCst);
                                shutdown_signal.notify_one(); // Notify other parts if they depend on this signal
                                break; // Exit loop as connection is closed
                            }
                        }
                        Err(e) => {
                            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                                log::info!("[RawDiscordClient::read_loop] Connection closed by peer (EOF). Error: {}", e);
                            } else {
                                log::error!("[RawDiscordClient::read_loop] Error reading framed message: {}", e);
                            }
                            is_connected.store(false, Ordering::SeqCst);
                            // Notify other parts that might be waiting for a clean shutdown
                            shutdown_signal.notify_one();
                            break;
                        }
                    }
                }
            }
        }
        is_connected.store(false, Ordering::SeqCst);
        log::info!("[RawDiscordClient::read_loop] Exited.");
    }

    pub async fn set_activity(&self, activity_payload: JsonValue) -> Result<(), String> {
        log::debug!("[RawDiscordClient::set_activity] Attempting to set activity.");
        if !self.is_connected.load(Ordering::SeqCst) {
            log::warn!("[RawDiscordClient::set_activity] Not connected, cannot set activity.");
            return Err("Not connected".to_string());
        }

        let mut stream_guard = self.stream_arc.lock().await;
        if let Some(stream) = stream_guard.as_mut() {
            let payload_str = match serde_json::to_string(&activity_payload) {
                Ok(s) => s,
                Err(e) => {
                    let err_msg = format!("Failed to serialize activity payload: {}", e);
                    log::error!("[RawDiscordClient::set_activity] {}", err_msg);
                    return Err(err_msg);
                }
            };

            let framed_msg = match frame_message(1, &payload_str) { // Opcode 1 for SET_ACTIVITY
                Ok(fm) => fm,
                Err(e) => {
                    let err_msg = format!("Failed to frame SET_ACTIVITY message: {}", e);
                    log::error!("[RawDiscordClient::set_activity] {}", err_msg);
                    return Err(err_msg);
                }
            };

            match stream.write_all(&framed_msg).await {
                Ok(_) => {
                    log::info!("[RawDiscordClient::set_activity] Successfully sent SET_ACTIVITY command. Payload: {}", payload_str);
                    Ok(())
                }
                Err(e) => {
                    let err_msg = format!("Failed to write SET_ACTIVITY to stream: {}", e);
                    log::error!("[RawDiscordClient::set_activity] {}", err_msg);
                    // If write fails, connection might be broken
                    self.is_connected.store(false, Ordering::SeqCst);
                    self.shutdown_signal.notify_one(); // Signal read loop
                    Err(err_msg)
                }
            }
        } else {
            log::warn!("[RawDiscordClient::set_activity] Stream is None, cannot set activity.");
            self.is_connected.store(false, Ordering::SeqCst); // Should already be false if stream is None after connect
            Err("Stream not available".to_string())
        }
    }

    pub async fn shutdown(&self) -> Result<(), String> { // Changed to take &self as per typical usage
        log::info!("[RawDiscordClient::shutdown] Initiating shutdown.");
        if !self.is_connected.load(Ordering::SeqCst) && self.stream_arc.lock().await.is_none() {
            log::info!("[RawDiscordClient::shutdown] Already disconnected or shut down.");
            return Ok(());
        }

        self.shutdown_signal.notify_one(); // Signal the read_loop to exit
        log::debug!("[RawDiscordClient::shutdown] Shutdown signal sent to read_loop.");

        let mut stream_guard = self.stream_arc.lock().await;
        if let Some(stream) = stream_guard.as_mut() {
            // Attempt to send a "close" frame (opcode 2)
            let close_payload = serde_json::json!({
                "v": 1,
                "client_id": self.client_id
            });
            let close_payload_str = serde_json::to_string(&close_payload)
                .map_err(|e| format!("Failed to serialize close payload: {}", e))?;
            
            match frame_message(2, &close_payload_str) {
                Ok(framed_close_msg) => {
                    log::debug!("[RawDiscordClient::shutdown] Attempting to send CLOSE frame.");
                    if let Err(e) = stream.write_all(&framed_close_msg).await {
                        log::warn!("[RawDiscordClient::shutdown] Failed to send CLOSE frame: {}. Connection might already be closed.", e);
                    } else {
                        log::info!("[RawDiscordClient::shutdown] CLOSE frame sent.");
                    }
                }
                Err(e) => {
                    log::warn!("[RawDiscordClient::shutdown] Failed to frame CLOSE message: {}", e);
                }
            }

            // Shutdown the write half of the stream.
            // The read half will be closed when read_loop exits or by dropping the stream.
            log::debug!("[RawDiscordClient::shutdown] Attempting to shutdown stream (write half).");
            if let Err(e) = stream.shutdown().await { // This is tokio::io::AsyncShutdown
                log::warn!("[RawDiscordClient::shutdown] Error during stream shutdown: {}", e);
            } else {
                log::info!("[RawDiscordClient::shutdown] Stream shutdown (write half) successful.");
            }
        }

        // Set stream to None
        *stream_guard = None;
        log::debug!("[RawDiscordClient::shutdown] Stream option set to None.");

        // Ensure is_connected is false
        self.is_connected.store(false, Ordering::SeqCst);
        log::info!("[RawDiscordClient::shutdown] Shutdown process complete. Client disconnected.");
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use tokio::net::UnixListener;

    // Helper to create a dummy socket file for testing
    async fn create_dummy_socket(path: &Path) -> std::io::Result<UnixListener> {
        if path.exists() {
            fs::remove_file(path)?;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        UnixListener::bind(path)
    }

    #[tokio::test]
    async fn test_connect_ipc_success() {
        let test_dir = env::temp_dir().join("test_discord_ipc");
        fs::create_dir_all(&test_dir).unwrap();
        let socket_path = test_dir.join("test-ipc-0");

        let listener = create_dummy_socket(&socket_path).await.expect("Failed to create dummy socket");

        let connect_result = connect_ipc(&socket_path).await;
        assert!(connect_result.is_ok(), "Should connect to dummy socket: {:?}", connect_result.err());

        drop(listener); // Explicitly drop to release the socket file
        fs::remove_file(&socket_path).unwrap_or_default(); // Clean up
        fs::remove_dir_all(&test_dir).unwrap_or_default();
    }

    #[tokio::test]
    async fn test_connect_ipc_failure() {
        let non_existent_path = PathBuf::from("/tmp/non_existent_socket_for_sure_12345.sock");
        if non_existent_path.exists() {
            fs::remove_file(&non_existent_path).unwrap(); // Clean up if it somehow exists
        }
        let result = connect_ipc(&non_existent_path).await;
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().kind(), ErrorKind::NotFound);
    }

    // Note: Testing `find_discord_ipc_path` is more complex as it depends on environment variables
    // and file system state. It would require mocking or setting up a specific test environment.
    // For this phase, we'll focus on `connect_ipc` tests.
    // A basic test could ensure it doesn't panic and returns None if nothing is found.

    #[test]
    fn test_find_discord_ipc_path_runs_without_panic() {
        // This test mainly ensures the function executes without panicking.
        // It doesn't verify finding a socket, as that's environment-dependent.
        let _ = find_discord_ipc_path(); // Call it and ignore result for this basic test
    }

    #[test]
    fn test_find_discord_ipc_path_with_mocked_env() {
        // More comprehensive test for find_discord_ipc_path
        let base_test_dir = env::temp_dir().join("test_find_ipc");
        fs::create_dir_all(&base_test_dir).expect("Failed to create base test dir");

        let xdg_runtime_val = base_test_dir.join("xdg_runtime");
        fs::create_dir_all(&xdg_runtime_val).expect("Failed to create xdg_runtime_val dir");

        // Create a dummy socket in the mocked XDG_RUNTIME_DIR
        let socket_path_in_xdg = xdg_runtime_val.join("discord-ipc-0");
        let _listener_xdg = UnixListener::bind(&socket_path_in_xdg).expect("Failed to bind dummy socket in xdg");


        env::set_var("XDG_RUNTIME_DIR", xdg_runtime_val.to_str().unwrap());
        // Unset other vars to ensure XDG_RUNTIME_DIR is prioritized
        env::remove_var("TMPDIR");


        let found_path = find_discord_ipc_path();
        assert!(found_path.is_some(), "Should find the socket in mocked XDG_RUNTIME_DIR");
        assert_eq!(found_path.unwrap(), socket_path_in_xdg);

        // Cleanup
        drop(_listener_xdg);
        fs::remove_file(&socket_path_in_xdg).unwrap_or_default();
        fs::remove_dir_all(&xdg_runtime_val).unwrap_or_default();


        // Test with TMPDIR
        let tmpdir_val = base_test_dir.join("my_tmp");
        fs::create_dir_all(&tmpdir_val).expect("Failed to create tmpdir_val");
        let socket_path_in_tmp = tmpdir_val.join("discord-ipc-1");
        let _listener_tmp = UnixListener::bind(&socket_path_in_tmp).expect("Failed to bind dummy socket in tmpdir");

        env::remove_var("XDG_RUNTIME_DIR"); // Remove XDG to test TMPDIR
        env::set_var("TMPDIR", tmpdir_val.to_str().unwrap());

        let found_path_tmp = find_discord_ipc_path();
        assert!(found_path_tmp.is_some(), "Should find the socket in mocked TMPDIR");
        assert_eq!(found_path_tmp.unwrap(), socket_path_in_tmp);

        drop(_listener_tmp);
        fs::remove_file(&socket_path_in_tmp).unwrap_or_default();
        fs::remove_dir_all(&tmpdir_val).unwrap_or_default();


        // Test with /tmp (by ensuring other vars are not set or point to non-existent paths)
        // This is harder to isolate perfectly without root or complex mocks for /tmp itself.
        // We can simulate it by creating a socket in a known /tmp subdirectory if possible,
        // or rely on the fact that if other paths fail, it will check /tmp.

        // Test with subdirectories
        let snap_discord_dir = xdg_runtime_val.join("snap.discord");
        fs::create_dir_all(&snap_discord_dir).expect("Failed to create snap.discord dir");
        let socket_in_snap = snap_discord_dir.join("discord-ipc-2");
        let _listener_snap = UnixListener::bind(&socket_in_snap).expect("Failed to bind in snap.discord");

        env::set_var("XDG_RUNTIME_DIR", xdg_runtime_val.to_str().unwrap());
        env::remove_var("TMPDIR");


        let found_path_snap = find_discord_ipc_path();
        assert!(found_path_snap.is_some(), "Should find socket in snap.discord subdir");
        assert!(
            found_path_snap.unwrap().starts_with(&snap_discord_dir),
            "Path should be inside snap.discord"
        );


        drop(_listener_snap);
        fs::remove_file(&socket_in_snap).unwrap_or_default();
        fs::remove_dir_all(&snap_discord_dir).unwrap_or_default();
        fs::remove_dir_all(&xdg_runtime_val).unwrap_or_default();
        fs::remove_dir_all(&base_test_dir).unwrap_or_default();

        // Clear env vars
        env::remove_var("XDG_RUNTIME_DIR");
        env::remove_var("TMPDIR");
    }
}