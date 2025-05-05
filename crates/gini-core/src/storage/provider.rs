use std::fmt::Debug; // Add Debug import
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use crate::kernel::error::Result;

/// Trait for storage providers that can read and write data
pub trait StorageProvider: Send + Sync + Debug { // Add Debug requirement
    /// Get the name of this provider
    fn name(&self) -> &str;
    
    /// Check if a path exists
    fn exists(&self, path: &Path) -> bool;
    
    /// Check if a path is a file
    fn is_file(&self, path: &Path) -> bool;
    
    /// Check if a path is a directory
    fn is_dir(&self, path: &Path) -> bool;
    
    /// Create a directory
    fn create_dir(&self, path: &Path) -> Result<()>;
    
    /// Create a directory and all its parent directories
    fn create_dir_all(&self, path: &Path) -> Result<()>;
    
    /// Read a file to a string
    fn read_to_string(&self, path: &Path) -> Result<String>;
    
    /// Read a file to a vector of bytes
    fn read_to_bytes(&self, path: &Path) -> Result<Vec<u8>>;
    
    /// Write a string to a file
    fn write_string(&self, path: &Path, contents: &str) -> Result<()>;
    
    /// Write bytes to a file
    fn write_bytes(&self, path: &Path, contents: &[u8]) -> Result<()>;
    
    /// Copy a file from one path to another
    fn copy(&self, from: &Path, to: &Path) -> Result<()>;
    
    /// Move a file from one path to another
    fn rename(&self, from: &Path, to: &Path) -> Result<()>;
    
    /// Remove a file
    fn remove_file(&self, path: &Path) -> Result<()>;
    
    /// Remove a directory
    fn remove_dir(&self, path: &Path) -> Result<()>;
    
    /// Remove a directory and all its contents
    fn remove_dir_all(&self, path: &Path) -> Result<()>;
    
    /// List all entries in a directory
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
    
    /// Get file metadata (size, modification time, etc.)
    fn metadata(&self, path: &Path) -> Result<std::fs::Metadata>;
    
    /// Open a file for reading
    fn open_read(&self, path: &Path) -> Result<Box<dyn Read>>;
    
    /// Open a file for writing
    fn open_write(&self, path: &Path) -> Result<Box<dyn Write>>;
    
    /// Open a file for appending
    fn open_append(&self, path: &Path) -> Result<Box<dyn Write>>;
}