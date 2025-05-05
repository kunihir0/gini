use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write}; // Remove unused Error as IoError import
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile; // Import NamedTempFile
use crate::kernel::error::{Error, Result};
use crate::storage::provider::StorageProvider;

/// Local filesystem storage provider
#[derive(Clone)]
pub struct LocalStorageProvider {
    base_path: PathBuf,
}

impl LocalStorageProvider {
    /// Create a new local storage provider with the given base path
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
    
    /// Resolve a relative path against the base path
    fn resolve_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.base_path.join(path)
    }
    
    // Remove the old io_error helper, use Error::io directly
}

impl StorageProvider for LocalStorageProvider {
    fn name(&self) -> &str {
        "local"
    }
    
    fn exists(&self, path: &Path) -> bool {
        self.resolve_path(path).exists()
    }
    
    fn is_file(&self, path: &Path) -> bool {
        self.resolve_path(path).is_file()
    }
    
    fn is_dir(&self, path: &Path) -> bool {
        self.resolve_path(path).is_dir()
    }
    
    fn create_dir(&self, path: &Path) -> Result<()> {
        let full_path = self.resolve_path(path);
        fs::create_dir(&full_path).map_err(|e| Error::io(e, "create_dir", Some(full_path)))
    }
    
    fn create_dir_all(&self, path: &Path) -> Result<()> {
        let full_path = self.resolve_path(path);
        fs::create_dir_all(&full_path).map_err(|e| Error::io(e, "create_dir_all", Some(full_path)))
    }
    
    fn read_to_string(&self, path: &Path) -> Result<String> {
        let full_path = self.resolve_path(path);
        fs::read_to_string(&full_path).map_err(|e| Error::io(e, "read_to_string", Some(full_path)))
    }
    
    fn read_to_bytes(&self, path: &Path) -> Result<Vec<u8>> {
        let full_path = self.resolve_path(path);
        fs::read(&full_path).map_err(|e| Error::io(e, "read_to_bytes", Some(full_path)))
    }
    
    fn write_string(&self, path: &Path, contents: &str) -> Result<()> {
        self.write_bytes(path, contents.as_bytes()) // Delegate to write_bytes
    }
    
    fn write_bytes(&self, path: &Path, contents: &[u8]) -> Result<()> {
        let full_path = self.resolve_path(path);
        
        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            if !self.is_dir(parent) { // Use self.is_dir to check relative path
                 self.create_dir_all(parent)?; // Use self.create_dir_all for relative path
            }
        } else {
            // Handle cases where path has no parent (e.g., root directory, unlikely for configs)
             return Err(Error::StorageOperationFailed {
                operation: "write_bytes".to_string(),
                path: Some(full_path.clone()),
                message: "Cannot write to path without parent directory".to_string(),
            });
        }

        // Create a named temporary file in the same directory as the target file
        let temp_file = NamedTempFile::new_in(full_path.parent().unwrap()) // unwrap is safe due to check above
            .map_err(|e| Error::io(e, "create_temp_file", Some(full_path.parent().unwrap().to_path_buf())))?;

        // Write contents to the temporary file
        // Use write_all for robustness
        temp_file.as_file().write_all(contents)
             .map_err(|e| Error::io(e, "write_to_temp_file", Some(temp_file.path().to_path_buf())))?;

        // Persist the temporary file, atomically replacing the target file
        temp_file.persist(&full_path)
            .map_err(|e| Error::io(e.error, "persist_temp_file", Some(full_path.clone())))?; // e is PersistError

        Ok(())
    }
    
    fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        let full_from = self.resolve_path(from);
        let full_to = self.resolve_path(to);
        fs::copy(&full_from, &full_to)
            .map(|_| ())
            .map_err(|e| Error::io(e, "copy", Some(full_from))) // Report error with source path
    }
    
    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        let full_from = self.resolve_path(from);
        let full_to = self.resolve_path(to);
        fs::rename(&full_from, &full_to)
            .map_err(|e| Error::io(e, "rename", Some(full_from))) // Report error with source path
    }
    
    fn remove_file(&self, path: &Path) -> Result<()> {
        let full_path = self.resolve_path(path);
        fs::remove_file(&full_path).map_err(|e| Error::io(e, "remove_file", Some(full_path)))
    }
    
    fn remove_dir(&self, path: &Path) -> Result<()> {
        let full_path = self.resolve_path(path);
        fs::remove_dir(&full_path).map_err(|e| Error::io(e, "remove_dir", Some(full_path)))
    }
    
    fn remove_dir_all(&self, path: &Path) -> Result<()> {
        let full_path = self.resolve_path(path);
        fs::remove_dir_all(&full_path).map_err(|e| Error::io(e, "remove_dir_all", Some(full_path)))
    }
    
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let full_path = self.resolve_path(path);
        let entries = fs::read_dir(&full_path).map_err(|e| Error::io(e, "read_dir", Some(full_path.clone())))?;
        let mut result = Vec::new();
        
        for entry in entries {
            // Map error for individual entry reading
            let entry = entry.map_err(|e| Error::io(e, "read_dir_entry", Some(full_path.clone())))?;
            let path = entry.path();
            
            // Convert back to a relative path if possible
            if let Ok(rel_path) = path.strip_prefix(&self.base_path) {
                result.push(rel_path.to_path_buf());
            } else {
                // This case should ideally not happen if entry path is within base_path
                result.push(path);
            }
        } // End of for loop
        
        Ok(result)
    } // End of read_dir method
    
    fn metadata(&self, path: &Path) -> Result<std::fs::Metadata> {
        let full_path = self.resolve_path(path);
        fs::metadata(&full_path).map_err(|e| Error::io(e, "metadata", Some(full_path)))
    }
    
    fn open_read(&self, path: &Path) -> Result<Box<dyn Read>> {
        let full_path = self.resolve_path(path);
        let file = File::open(&full_path).map_err(|e| Error::io(e, "open_read", Some(full_path)))?;
        Ok(Box::new(file))
    }
    
    fn open_write(&self, path: &Path) -> Result<Box<dyn Write>> {
        let full_path = self.resolve_path(path);
        let file = File::create(&full_path).map_err(|e| Error::io(e, "open_write", Some(full_path)))?;
        Ok(Box::new(file))
    }
    
    fn open_append(&self, path: &Path) -> Result<Box<dyn Write>> {
        let full_path = self.resolve_path(path);
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(&full_path)
            .map_err(|e| Error::io(e, "open_append", Some(full_path)))?;
        Ok(Box::new(file))
    }
} // End of impl StorageProvider

// Implement Debug for LocalStorageProvider (Moved outside impl StorageProvider)
impl fmt::Debug for LocalStorageProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalStorageProvider")
            .field("base_path", &self.base_path)
            .finish()
    }
}