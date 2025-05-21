use gini_core::kernel::bootstrap::Application;
use gini_core::plugin_system::dependency::PluginDependency;
use gini_core::plugin_system::error::PluginSystemError;
use gini_core::plugin_system::traits::{Plugin, PluginPriority};
use gini_core::plugin_system::version::VersionRange;
use gini_core::stage_manager::registry::StageRegistry;
use gini_core::stage_manager::requirement::StageRequirement;
use gini_core::storage::error::StorageSystemError as GiniStorageError; // Corrected Alias
// use gini_core::storage::DefaultStorageManager; // Import concrete type - Removed as unused
use serde::{Deserialize, Serialize};
use serde_json; // For ConfigData to LoggingConfig conversion
use std::error::Error as StdError; // Import the standard Error trait

// Tracing specific imports
use tracing; // For macros like tracing::info!
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry,
};
// LogLayer import removed due to resolution issues. Will use LogTracer::init().
// use tracing_log::LogLayer;
// tracing_log import removed as LogTracer::init() is not called directly.
// The "fmt" feature of tracing-subscriber handles log bridging.

#[derive(Serialize, Deserialize, Debug, Default)]
struct LoggingConfig {
    default_level: Option<String>,
    format: Option<String>,
    // per_module_levels: Option<HashMap<String, String>>, // Example for future extension
}

// Define the main plugin struct
#[derive(Default)]
#[allow(dead_code)] // Suppress warning as it might be loaded implicitly
pub struct LoggingPlugin;

// Implement the Plugin trait
impl Plugin for LoggingPlugin {
    fn name(&self) -> &'static str {
        "core-logging"
    }

    fn version(&self) -> &str {
        "0.1.0" // Version can be updated if features change significantly
    }

    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        const COMPATIBLE_API_REQ: &str = "^0.1";
        match VersionRange::from_constraint(COMPATIBLE_API_REQ) {
            Ok(vr) => vec![vr],
            Err(e) => {
                // Use tracing::error! here. It will be a no-op if no subscriber is set,
                // but it won't prematurely initialize the `log` crate's default logger.
                // If a subscriber is later initialized, this event might be lost,
                // but it prevents the logger conflict.
                tracing::error!(
                    plugin_name = self.name(),
                    api_requirement = COMPATIBLE_API_REQ,
                    error = %e,
                    "Failed to parse API version requirement"
                );
                vec![]
            }
        }
    }

    fn is_core(&self) -> bool {
        true
    }

    fn priority(&self) -> PluginPriority {
        PluginPriority::Core(1) // Highest priority to ensure it initializes before any other plugin that might log.
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![]
    }

    fn required_stages(&self) -> Vec<StageRequirement> {
        vec![] // No specific stages required by this plugin itself for now
    }

    fn conflicts_with(&self) -> Vec<String> {
        vec![]
    }

    fn incompatible_with(&self) -> Vec<PluginDependency> {
        vec![]
    }

    fn init(&self, app: &mut Application) -> Result<(), PluginSystemError> {
        let storage_manager = app.storage_manager(); // Arc<DefaultStorageManager>

        let logging_config: LoggingConfig = match storage_manager.get_plugin_config(self.name()) {
            Ok(config_data) => {
                // ConfigData holds values as HashMap<String, serde_json::Value>.
                // We need to convert this to our LoggingConfig struct.
                // A common way is to serialize ConfigData to a serde_json::Value,
                // then deserialize that Value into LoggingConfig.
                match serde_json::to_value(config_data) {
                    Ok(json_val) => {
                        match serde_json::from_value::<LoggingConfig>(json_val.clone()) {
                            Ok(cfg) => {
                                tracing::debug!(
                                    plugin_name = self.name(),
                                    config_name = self.name(),
                                    "Logging configuration loaded and parsed successfully."
                                );
                                cfg
                            }
                            Err(e) => {
                                // This case means the file was found and read by ConfigManager,
                                // but its structure doesn't match LoggingConfig.
                                tracing::warn!(
                                    plugin_name = self.name(),
                                    config_name = self.name(),
                                    error = %e,
                                    loaded_config_json = %json_val, // Log the problematic JSON
                                    "Failed to parse loaded logging configuration into LoggingConfig struct. Using default settings."
                                );
                                LoggingConfig::default()
                            }
                        }
                    }
                    Err(e) => {
                        // Error serializing ConfigData to serde_json::Value
                        tracing::warn!(
                            plugin_name = self.name(),
                            config_name = self.name(),
                            error = %e,
                            "Failed to convert ConfigData to JSON for parsing. Using default settings."
                        );
                        LoggingConfig::default()
                    }
                }
            }
            Err(kernel_error) => {
                // get_plugin_config returns KernelError. We need to check if it's due to NotFound.
                // KernelError can wrap a StorageSystemError.
                let mut is_not_found = false;
                // Use a loop to traverse the source chain, as the direct source might not be GiniStorageError
                let mut current_err: Option<&dyn StdError> = Some(&kernel_error);
                while let Some(err) = current_err {
                    if let Some(storage_err) = err.downcast_ref::<GiniStorageError>() {
                        if matches!(storage_err, GiniStorageError::ConfigNotFound { .. } | GiniStorageError::FileNotFound(_)) {
                            is_not_found = true;
                            break; // Found the relevant error
                        }
                    }
                    current_err = err.source();
                }

                if is_not_found {
                    tracing::info!(
                        plugin_name = self.name(),
                        config_name = self.name(),
                        "Logging configuration file ('{}.toml' or similar) not found. Using default settings.", self.name()
                    );
                } else {
                    tracing::warn!(
                        plugin_name = self.name(),
                        config_name = self.name(),
                        error = %kernel_error,
                        "Failed to load logging configuration. Using default settings."
                    );
                }
                LoggingConfig::default()
            }
        };

        // Configure EnvFilter:
        // 1. Try to read from RUST_LOG environment variable.
        // 2. If not set, fall back to `logging_config.default_level` or "info".
        let default_log_level_from_config = logging_config
            .default_level
            .as_deref()
            .unwrap_or("info");

        let env_filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(default_log_level_from_config))
            .map_err(|e| {
                PluginSystemError::InternalError(format!(
                    "Failed to create EnvFilter with effective level '{}': {}",
                    default_log_level_from_config, e
                ))
            })?;

        // Configure Format Layer:
        // Use `logging_config.format` or default to "compact".
        let format_str = logging_config.format.as_deref().unwrap_or("compact");

        // Build the subscriber and initialize
        let subscriber_builder = Registry::default().with(env_filter);

        match format_str {
            "pretty" => subscriber_builder.with(fmt::layer().pretty()).try_init(),
            "json" => subscriber_builder.with(fmt::layer().json()).try_init(),
            "compact" | _ => {
                if format_str != "compact" && logging_config.format.is_some() {
                    // This warning uses the 'compact' logger as it's about to be set.
                    // It will be emitted after try_init if we log it there.
                    // Logging it here means it's queued for the new logger.
                    tracing::warn!(
                        plugin_name = self.name(),
                        invalid_format = %logging_config.format.as_ref().unwrap(),
                        "Unrecognized log format specified. Defaulting to 'compact'."
                    );
                }
                subscriber_builder.with(fmt::layer().compact()).try_init()
            }
        }
        .map_err(|e| {
            PluginSystemError::InternalError(format!(
                "Failed to set global default tracing subscriber: {}",
                e
            ))
        })?;

        // Determine source of effective log level for clarity in logs
        let (level_source_msg, final_effective_level_str) =
            if std::env::var("RUST_LOG").is_ok() {
                ("RUST_LOG environment variable", std::env::var("RUST_LOG").unwrap_or_default())
            } else if logging_config.default_level.is_some() {
                ("logging_config.default_level", default_log_level_from_config.to_string())
            } else {
                ("hardcoded_default", default_log_level_from_config.to_string())
            };

        tracing::info!(
            plugin_name = self.name(),
            plugin_version = self.version(),
            effective_log_level_source = %level_source_msg,
            effective_log_level = %final_effective_level_str, // RUST_LOG can be complex, this shows the fallback or actual RUST_LOG string
            log_format = %format_str,
            "Core Logging Plugin initialized with tracing."
        );
        Ok(())
    }

    fn register_stages(&self, _registry: &mut StageRegistry) -> Result<(), PluginSystemError> {
        tracing::info!(
            plugin_name = self.name(),
            "Core Logging Plugin provides no stages to register."
        );
        Ok(())
    }

    fn shutdown(&self) -> Result<(), PluginSystemError> {
        tracing::info!(
            plugin_name = self.name(),
            "Shutting down Core Logging Plugin"
        );
        // Tracing shutdown is typically handled globally when the application exits
        // or when the dispatcher is dropped. No explicit shutdown needed here for the subscriber itself.
        Ok(())
    }
}