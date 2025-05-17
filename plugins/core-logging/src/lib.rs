use gini_core::kernel::bootstrap::Application;
use gini_core::plugin_system::dependency::PluginDependency;
use gini_core::plugin_system::error::PluginSystemError;
use gini_core::plugin_system::traits::{Plugin, PluginPriority};
use gini_core::plugin_system::version::VersionRange;
use gini_core::stage_manager::registry::StageRegistry;
use gini_core::stage_manager::requirement::StageRequirement;

// Tracing specific imports
use tracing; // For macros like tracing::info!
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry,
};
// LogLayer import removed due to resolution issues. Will use LogTracer::init().
// use tracing_log::LogLayer;
// tracing_log import removed as LogTracer::init() is not called directly.
// The "fmt" feature of tracing-subscriber handles log bridging.

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

    fn init(&self, _app: &mut Application) -> Result<(), PluginSystemError> {
        // Placeholder: In the future, retrieve detailed configuration from `_app`
        // which would have been loaded by an earlier stage via StorageManager.
        // For example:
        // let logging_config = _app.get_kernel_component::<LoggingConfigService>()
        // .map(|service| service.get_config())
        // .unwrap_or_default(); // Assuming a default config struct

        // Configure EnvFilter:
        // 1. Try to read from RUST_LOG environment variable.
        // 2. If not set, fall back to a default (e.g., "info").
        // TODO: Allow this default to be overridden by `logging_config.default_level`.
        let default_filter = "info";
        let env_filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(default_filter))
            .map_err(|e| {
                PluginSystemError::InternalError(format!("Failed to create EnvFilter: {}", e))
            })?;

        // Configure Format Layer:
        // TODO: Allow format (compact, pretty, json) to be chosen via `logging_config.format`.
        let format_layer = fmt::layer()
            .compact(); // Default to compact. Alternatives: .pretty(), .json()

        // Build the subscriber
        let subscriber = Registry::default()
            .with(env_filter) // Apply filtering
            .with(format_layer); // Apply formatting
            // LogLayer is not added here.
            // LogTracer::init() is not called directly either.
            // The "fmt" feature of tracing-subscriber should handle log bridging.

        // Try to set the global default subscriber for tracing.
        // This should happen only once.
        // When "fmt" feature is enabled in tracing-subscriber, this also initializes log compatibility.
        subscriber.try_init().map_err(|e| {
            PluginSystemError::InternalError(format!(
                "Failed to set global default tracing subscriber: {}",
                e
            ))
        })?;

        tracing::info!(
            plugin_name = self.name(),
            plugin_version = self.version(),
            "Core Logging Plugin initialized with tracing"
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