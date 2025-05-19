use gini_core::storage::manager::DefaultStorageManager; // Removed StorageManager
use gini_core::storage::error::StorageSystemError;
use gini_core::storage::config::{ConfigManager, ConfigScope, ConfigData, PluginConfigScope};
use gini_core::kernel::error::Error as KernelError;
use serde::{Deserialize, Serialize};
use serde_json;
use thiserror::Error;
use log::{debug, warn, error, info}; // Add info macro
// use toml; // Removed unused import, toml::from_str and toml::to_string_pretty are used via ConfigData
use std::sync::Arc;
// Removed: use std::collections::HashMap; // No longer directly used after ConfigData conversion changes

const SETTINGS_CONFIG_NAME: &str = "core-rpc.toml"; // Specify .toml extension

#[derive(Error, Debug)]
pub enum SettingsError {
    #[error("Failed to deserialize settings: {0}")]
    Deserialization(String),
    #[error("Failed to serialize settings: {0}")]
    Serialization(String),
    #[error("Storage operation failed: {0}")]
    Storage(#[from] StorageSystemError),
    #[error("Kernel error during storage operation: {0}")]
    Kernel(#[from] KernelError),
    #[error("Settings structure conversion error: {0}")]
    Conversion(#[from] serde_json::Error),
    #[error("Failed to save default settings, but using defaults for this session. Original error: {0}")]
    SaveDefaultFailed(Box<SettingsError>), // Keep this if it's a distinct scenario
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RpcSettings {
    pub enabled: bool,
    pub client_id: Option<String>,
    pub default_details: Option<String>,
    pub default_state: Option<String>,
    // Add any other RPC specific settings here
}

impl Default for RpcSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            client_id: Some("".to_string()), // Default to empty string, will appear in TOML
            default_details: Some("Using Gini Framework".to_string()),
            default_state: Some("Idle".to_string()),
        }
    }
}

// Helper to get the plugin's specific config scope.
// Assuming core-rpc is a plugin, its config might be under its own name.
// Or, if it's truly "core" to Gini, it might use Application scope.
// For now, let's treat it as a plugin's config.
fn get_rpc_config_scope() -> ConfigScope {
    // This assumes the plugin's ID or a known name is used for its config directory.
    // If core-rpc is special, it might use ConfigScope::Application.
    // For a generic plugin "core-rpc":
    ConfigScope::Plugin(PluginConfigScope::User) // User scope for plugin "core-rpc"
    // The name "core-rpc" will be passed to load_config/save_config.
    // ConfigManager will resolve to something like: <plugin_config_path>/user/core-rpc.toml (or .json)
}


pub async fn load_settings(storage_manager: Arc<DefaultStorageManager>) -> Result<RpcSettings, SettingsError> {
    let config_mngr = storage_manager.get_config_manager();

    match config_mngr.load_config(SETTINGS_CONFIG_NAME, get_rpc_config_scope()) {
        Ok(loaded_config_data) => {
            // Serialize ConfigData to a JSON string to inspect its content and parse from it.
            // ConfigData internally holds values as serde_json::Value, so serializing to JSON is natural.
            let json_string = loaded_config_data.serialize(gini_core::storage::config::ConfigFormat::Json)
                .map_err(|e| SettingsError::Serialization(format!("Failed to serialize loaded ConfigData to JSON: {}", e)))?;

            // If json_string is empty or just "{}", it means ConfigManager returned a default/empty ConfigData
            // (e.g., because the file didn't exist or was empty).
            if json_string.trim().is_empty() || json_string.trim() == "{}" {
                info!("No existing settings found or settings file was empty for '{}'. Using and saving defaults.", SETTINGS_CONFIG_NAME);
                let defaults = RpcSettings::default();
                // Attempt to save these defaults. If this fails, the original situation (no file) persists for next load.
                if let Err(save_err) = save_settings_internal(config_mngr.as_ref(), &defaults, get_rpc_config_scope()).await {
                    warn!("Failed to save default settings for '{}' after finding none: {}", SETTINGS_CONFIG_NAME, save_err);
                }
                Ok(defaults)
            } else {
                // Try to parse the JSON string into RpcSettings
                match serde_json::from_str::<RpcSettings>(&json_string) {
                    Ok(settings) => {
                        debug!("Successfully loaded and parsed RPC settings for '{}' from existing file.", SETTINGS_CONFIG_NAME);
                        Ok(settings)
                    }
                    Err(e) => {
                        warn!("Failed to parse existing settings for '{}' (content: '{}'): {}. Using defaults and attempting to overwrite.", SETTINGS_CONFIG_NAME, json_string, e);
                        let defaults = RpcSettings::default();
                        if let Err(save_err) = save_settings_internal(config_mngr.as_ref(), &defaults, get_rpc_config_scope()).await {
                             warn!("Additionally, failed to save default settings after parsing failure of existing file: {}", save_err);
                        }
                        Ok(defaults)
                    }
                }
            }
        }
        // These specific errors from StorageSystem might occur if ConfigManager itself fails before returning ConfigData
        Err(KernelError::StorageSystem(StorageSystemError::FileNotFound(_))) |
        Err(KernelError::StorageSystem(StorageSystemError::ConfigNotFound { .. })) => {
            debug!("No RPC settings file found for '{}'. Using and saving default settings.", SETTINGS_CONFIG_NAME);
            let defaults = RpcSettings::default();
            save_settings_internal(config_mngr.as_ref(), &defaults, get_rpc_config_scope()).await?;
            Ok(defaults)
        }
        Err(e) => {
            error!("Failed to load RPC settings for '{}': {}. Using default settings.", SETTINGS_CONFIG_NAME, e);
            let defaults = RpcSettings::default();
            if let Err(save_err) = save_settings_internal(config_mngr.as_ref(), &defaults, get_rpc_config_scope()).await {
                 warn!("Additionally, failed to save default settings after load failure: {}", save_err);
            }
            Err(SettingsError::Kernel(e))
        }
    }
}

async fn save_settings_internal(config_mngr: &ConfigManager, settings: &RpcSettings, scope: ConfigScope) -> Result<(), SettingsError> {
    // Convert RpcSettings directly to ConfigData via serde_json::Value
    let settings_json_value = serde_json::to_value(settings)
        .map_err(|e| SettingsError::Serialization(format!("Failed to serialize RpcSettings to JSON value: {}", e)))?;
    
    let config_data = match settings_json_value {
        serde_json::Value::Object(map) => {
            // ConfigData::from_hashmap expects HashMap<String, serde_json::Value>
            // The map from serde_json::to_value is serde_json::Map.
            let hash_map: std::collections::HashMap<String, serde_json::Value> = map.into_iter().collect();
            ConfigData::from_hashmap(hash_map)
        },
        _ => return Err(SettingsError::Serialization("RpcSettings did not serialize to a JSON object".to_string())),
    };
    
    config_mngr.save_config(SETTINGS_CONFIG_NAME, &config_data, scope)?;
    debug!("RPC settings saved for '{}' in scope {:?}", SETTINGS_CONFIG_NAME, scope);
    Ok(())
}

#[allow(dead_code)] // This is part of the public API, may be used by other plugins/core
pub async fn save_settings(storage_manager: Arc<DefaultStorageManager>, settings: &RpcSettings) -> Result<(), SettingsError> {
    let config_mngr = storage_manager.get_config_manager();
    save_settings_internal(config_mngr.as_ref(), settings, get_rpc_config_scope()).await
}