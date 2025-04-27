use std::any::Any;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use async_trait::async_trait;

use crate::kernel::component::KernelComponent;
use crate::kernel::error::Result;
use crate::storage::provider::StorageProvider;
use crate::storage::local::LocalStorageProvider; // Default provider

/// Storage manager component interface
/// This simply wraps a StorageProvider for now
#[async_trait]
pub trait StorageManager: KernelComponent + StorageProvider {}

/// Default implementation of StorageManager
#[derive(Clone)] // Add Clone derive
pub struct DefaultStorageManager {
    name: &'static str,
    provider: Arc<dyn StorageProvider>, // Holds the actual provider
}

impl DefaultStorageManager {
    /// Create a new default storage manager with a LocalStorageProvider
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            name: "DefaultStorageManager",
            provider: Arc::new(LocalStorageProvider::new(base_path)),
        }
    }

    /// Create a new storage manager with a custom provider
    pub fn with_provider(provider: Arc<dyn StorageProvider>) -> Self {
        Self {
            name: "DefaultStorageManager", // Or derive from provider?
            provider,
        }
    }

    /// Get the underlying provider
    pub fn provider(&self) -> &Arc<dyn StorageProvider> {
        &self.provider
    }
}

impl Debug for DefaultStorageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultStorageManager")
            .field("name", &self.name)
            .field("provider", &self.provider.name()) // Show provider name
            .finish()
    }
}

#[async_trait]
impl KernelComponent for DefaultStorageManager {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn initialize(&self) -> Result<()> {
        // Delegate to provider if it has an init method (currently doesn't)
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        // Delegate to provider if it has a start method (currently doesn't)
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        // Delegate to provider if it has a stop method (currently doesn't)
        Ok(())
    }
    // Removed as_any and as_any_mut
}

// Implement StorageProvider by delegating to the internal provider
impl StorageProvider for DefaultStorageManager {
    fn name(&self) -> &str {
        self.provider.name()
    }

    fn exists(&self, path: &Path) -> bool {
        self.provider.exists(path)
    }

    fn is_file(&self, path: &Path) -> bool {
        self.provider.is_file(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        self.provider.is_dir(path)
    }

    fn create_dir(&self, path: &Path) -> Result<()> {
        self.provider.create_dir(path)
    }

    fn create_dir_all(&self, path: &Path) -> Result<()> {
        self.provider.create_dir_all(path)
    }

    fn read_to_string(&self, path: &Path) -> Result<String> {
        self.provider.read_to_string(path)
    }

    fn read_to_bytes(&self, path: &Path) -> Result<Vec<u8>> {
        self.provider.read_to_bytes(path)
    }

    fn write_string(&self, path: &Path, contents: &str) -> Result<()> {
        self.provider.write_string(path, contents)
    }

    fn write_bytes(&self, path: &Path, contents: &[u8]) -> Result<()> {
        self.provider.write_bytes(path, contents)
    }

    fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        self.provider.copy(from, to)
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        self.provider.rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> Result<()> {
        self.provider.remove_file(path)
    }

    fn remove_dir(&self, path: &Path) -> Result<()> {
        self.provider.remove_dir(path)
    }

    fn remove_dir_all(&self, path: &Path) -> Result<()> {
        self.provider.remove_dir_all(path)
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        self.provider.read_dir(path)
    }

    fn metadata(&self, path: &Path) -> Result<std::fs::Metadata> {
        self.provider.metadata(path)
    }

    // Note: open_read, open_write, open_append return Box<dyn Read/Write>
    // which might not be Send/Sync. This could be an issue if the manager
    // needs to be Send/Sync. For now, we delegate directly.
    fn open_read(&self, path: &Path) -> Result<Box<dyn std::io::Read>> {
        self.provider.open_read(path)
    }

    fn open_write(&self, path: &Path) -> Result<Box<dyn std::io::Write>> {
        self.provider.open_write(path)
    }

    fn open_append(&self, path: &Path) -> Result<Box<dyn std::io::Write>> {
        self.provider.open_append(path)
    }
}

// Implement the marker trait
impl StorageManager for DefaultStorageManager {}

// Default using current directory (or appropriate default)
impl Default for DefaultStorageManager {
    fn default() -> Self {
        // Determine a sensible default base path, e.g., current dir or user data dir
        let default_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::new(default_path)
    }
}