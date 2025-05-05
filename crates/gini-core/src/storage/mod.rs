pub mod provider;
pub mod local;
pub mod manager; // Add manager module
pub mod config; // Add configuration module


/// Re-export key types
pub use provider::StorageProvider;
pub use local::LocalStorageProvider;
pub use manager::{StorageManager, DefaultStorageManager}; // Export manager types
pub use config::{
    ConfigManager, ConfigFormat, ConfigData, ConfigScope,
    PluginConfigScope, ConfigStorageExt,
}; // Export config types

// Keep the old StorageManager struct for now if needed for compatibility,
// or remove it if the new component replaces it entirely.
// For now, let's comment it out to avoid naming conflicts.

/*
/// Manager for storage operations (Old version)
pub struct OldStorageManager {
    providers: Vec<Box<dyn StorageProvider>>,
    default_provider_index: usize,
}

impl OldStorageManager {
    // ... (old methods) ...
    }
    */
    
    // Test module declaration
    #[cfg(test)]
    mod tests;