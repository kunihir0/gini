pub mod provider;
pub mod local;

use std::path::{Path, PathBuf};
use crate::kernel::error::{Error, Result};

/// Re-export key types
pub use provider::StorageProvider;
pub use local::LocalStorageProvider;

/// Manager for storage operations
pub struct StorageManager {
    providers: Vec<Box<dyn StorageProvider>>,
    default_provider_index: usize,
}

impl StorageManager {
    /// Create a new storage manager with default local provider
    pub fn new(base_path: PathBuf) -> Self {
        let local_provider = Box::new(LocalStorageProvider::new(base_path));
        
        Self {
            providers: vec![local_provider],
            default_provider_index: 0,
        }
    }
    
    /// Add a provider to the manager
    pub fn add_provider(&mut self, provider: Box<dyn StorageProvider>) -> usize {
        let index = self.providers.len();
        self.providers.push(provider);
        index
    }
    
    /// Set the default provider index
    pub fn set_default_provider(&mut self, index: usize) -> Result<()> {
        if index >= self.providers.len() {
            return Err(Error::Storage(
                format!("Invalid provider index: {}", index)
            ));
        }
        
        self.default_provider_index = index;
        Ok(())
    }
    
    /// Get the name of a provider
    pub fn provider_name(&self, index: usize) -> Option<&str> {
        self.providers.get(index).map(|p| p.name())
    }
    
    /// Get the index of a provider by name
    pub fn find_provider(&self, name: &str) -> Option<usize> {
        self.providers.iter().position(|p| p.name() == name)
    }
    
    /// Get the number of providers
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }
    
    /// Get the default provider index
    pub fn default_provider_index(&self) -> usize {
        self.default_provider_index
    }
    
    // Forward common operations to the default provider
    
    /// Check if a path exists
    pub fn exists(&self, path: &Path) -> bool {
        self.providers[self.default_provider_index].exists(path)
    }
    
    /// Check if a path is a file
    pub fn is_file(&self, path: &Path) -> bool {
        self.providers[self.default_provider_index].is_file(path)
    }
    
    /// Check if a path is a directory
    pub fn is_dir(&self, path: &Path) -> bool {
        self.providers[self.default_provider_index].is_dir(path)
    }
    
    /// Create a directory
    pub fn create_dir(&self, path: &Path) -> Result<()> {
        self.providers[self.default_provider_index].create_dir(path)
    }
    
    /// Create a directory and all its parent directories
    pub fn create_dir_all(&self, path: &Path) -> Result<()> {
        self.providers[self.default_provider_index].create_dir_all(path)
    }
    
    /// Read a file to a string
    pub fn read_to_string(&self, path: &Path) -> Result<String> {
        self.providers[self.default_provider_index].read_to_string(path)
    }
    
    /// Read a file to a vector of bytes
    pub fn read_to_bytes(&self, path: &Path) -> Result<Vec<u8>> {
        self.providers[self.default_provider_index].read_to_bytes(path)
    }
    
    /// Write a string to a file
    pub fn write_string(&self, path: &Path, contents: &str) -> Result<()> {
        self.providers[self.default_provider_index].write_string(path, contents)
    }
    
    /// Write bytes to a file
    pub fn write_bytes(&self, path: &Path, contents: &[u8]) -> Result<()> {
        self.providers[self.default_provider_index].write_bytes(path, contents)
    }
    
    /// Execute an operation with a specific provider
    pub fn with_provider<F, T>(&self, index: usize, operation: F) -> Result<T>
    where
        F: FnOnce(&dyn StorageProvider) -> Result<T>,
    {
        if let Some(provider) = self.providers.get(index) {
            operation(provider.as_ref())
        } else {
            Err(Error::Storage(
                format!("Invalid provider index: {}", index)
            ))
        }
    }
}