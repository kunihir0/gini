use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write}; // Remove unused Error as IoError import
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile; // Import NamedTempFile
use crate::storage::error::StorageSystemError; // Changed import
use crate::storage::provider::StorageProvider;
use std::result::Result as StdResult; // Added for type alias

/// Type alias for Result with StorageSystemError
type Result<T> = StdResult<T, StorageSystemError>;

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
    
    // Helper to map std::io::Error to StorageSystemError::Io
    fn map_io_error(&self, err: std::io::Error, operation: &str, path: PathBuf) -> StorageSystemError {
        match err.kind() {
            std::io::ErrorKind::NotFound => StorageSystemError::FileNotFound(path),
            std::io::ErrorKind::PermissionDenied => StorageSystemError::AccessDenied(path, operation.to_string()),
            _ => StorageSystemError::io(err, operation, path),
        }
    }
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
        fs::create_dir(&full_path).map_err(|e| self.map_io_error(e, "create_dir", full_path))
    }
    
    fn create_dir_all(&self, path: &Path) -> Result<()> {
        let full_path = self.resolve_path(path);
        fs::create_dir_all(&full_path).map_err(|e| self.map_io_error(e, "create_dir_all", full_path))
    }
    
    fn read_to_string(&self, path: &Path) -> Result<String> {
        let full_path = self.resolve_path(path);
        fs::read_to_string(&full_path).map_err(|e| self.map_io_error(e, "read_to_string", full_path))
    }
    
    fn read_to_bytes(&self, path: &Path) -> Result<Vec<u8>> {
        let full_path = self.resolve_path(path);
        fs::read(&full_path).map_err(|e| self.map_io_error(e, "read_to_bytes", full_path))
    }
    
    fn write_string(&self, path: &Path, contents: &str) -> Result<()> {
        self.write_bytes(path, contents.as_bytes()) // Delegate to write_bytes
    }
    
    fn write_bytes(&self, path: &Path, contents: &[u8]) -> Result<()> {
        let full_path = self.resolve_path(path);
        
        let parent_dir = match full_path.parent() {
            Some(p) => p.to_path_buf(), // Convert to PathBuf for ownership
            None => return Err(StorageSystemError::InvalidPath {
                path: full_path.clone(),
                reason: "Cannot write to path without parent directory".to_string(),
            }),
        };

        // Ensure parent directory exists. create_dir_all expects a relative path to the provider's base.
        // We need to strip base_path if parent_dir is absolute, or handle it carefully.
        // For simplicity, assuming create_dir_all handles absolute paths correctly or paths are relative.
        // If parent_dir is already resolved (absolute), we pass it directly.
        // If create_dir_all is called on an already existing dir, it's a no-op.
        fs::create_dir_all(&parent_dir).map_err(|e| self.map_io_error(e, "create_dir_all_for_write", parent_dir.clone()))?;

        let temp_file = NamedTempFile::new_in(&parent_dir)
            .map_err(|e| self.map_io_error(e, "create_temp_file", parent_dir.clone()))?;

        temp_file.as_file().write_all(contents)
             .map_err(|e| self.map_io_error(e, "write_to_temp_file", temp_file.path().to_path_buf()))?;

        temp_file.persist(&full_path)
            .map_err(|e| self.map_io_error(e.error, "persist_temp_file", full_path.clone()))?;

        Ok(())
    }
    
    fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        let full_from = self.resolve_path(from);
        let full_to = self.resolve_path(to);
        fs::copy(&full_from, &full_to)
            .map(|_| ())
            .map_err(|e| self.map_io_error(e, "copy", full_from))
    }
    
    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        let full_from = self.resolve_path(from);
        let full_to = self.resolve_path(to);
        fs::rename(&full_from, &full_to)
            .map_err(|e| self.map_io_error(e, "rename", full_from))
    }
    
    fn remove_file(&self, path: &Path) -> Result<()> {
        let full_path = self.resolve_path(path);
        fs::remove_file(&full_path).map_err(|e| self.map_io_error(e, "remove_file", full_path))
    }
    
    fn remove_dir(&self, path: &Path) -> Result<()> {
        let full_path = self.resolve_path(path);
        fs::remove_dir(&full_path).map_err(|e| self.map_io_error(e, "remove_dir", full_path))
    }
    
    fn remove_dir_all(&self, path: &Path) -> Result<()> {
        let full_path = self.resolve_path(path);
        fs::remove_dir_all(&full_path).map_err(|e| self.map_io_error(e, "remove_dir_all", full_path))
    }
    
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let full_path = self.resolve_path(path);
        let entries = fs::read_dir(&full_path).map_err(|e| self.map_io_error(e, "read_dir", full_path.clone()))?;
        let mut result = Vec::new();
        
        for entry in entries {
            let entry = entry.map_err(|e| self.map_io_error(e, "read_dir_entry", full_path.clone()))?;
            let path = entry.path();
            
            if let Ok(rel_path) = path.strip_prefix(&self.base_path) {
                result.push(rel_path.to_path_buf());
            } else {
                result.push(path);
            }
        }
        
        Ok(result)
    }
    
    fn metadata(&self, path: &Path) -> Result<std::fs::Metadata> {
        let full_path = self.resolve_path(path);
        fs::metadata(&full_path).map_err(|e| self.map_io_error(e, "metadata", full_path))
    }
    
    fn open_read(&self, path: &Path) -> Result<Box<dyn Read>> {
        let full_path = self.resolve_path(path);
        File::open(&full_path)
            .map(|file| Box::new(file) as Box<dyn Read>)
            .map_err(|e| self.map_io_error(e, "open_read", full_path))
    }
    
    fn open_write(&self, path: &Path) -> Result<Box<dyn Write>> {
        let full_path = self.resolve_path(path);
        File::create(&full_path)
            .map(|file| Box::new(file) as Box<dyn Write>)
            .map_err(|e| self.map_io_error(e, "open_write", full_path))
    }
    
    fn open_append(&self, path: &Path) -> Result<Box<dyn Write>> {
        let full_path = self.resolve_path(path);
        OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(&full_path)
            .map(|file| Box::new(file) as Box<dyn Write>)
            .map_err(|e| self.map_io_error(e, "open_append", full_path))
    }
}

// Implement Debug for LocalStorageProvider (Moved outside impl StorageProvider)
impl fmt::Debug for LocalStorageProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalStorageProvider")
            .field("base_path", &self.base_path)
            .finish()
    }
}