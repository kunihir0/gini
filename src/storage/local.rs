use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write, Error as IoError};
use std::path::{Path, PathBuf};
use crate::kernel::error::{Error, Result};
use crate::storage::provider::StorageProvider;

/// Local filesystem storage provider
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
    
    /// Convert an IO error to our custom error type
    fn io_error(e: IoError) -> Error {
        Error::Storage(e.to_string())
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
        fs::create_dir(self.resolve_path(path)).map_err(Self::io_error)
    }
    
    fn create_dir_all(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(self.resolve_path(path)).map_err(Self::io_error)
    }
    
    fn read_to_string(&self, path: &Path) -> Result<String> {
        fs::read_to_string(self.resolve_path(path)).map_err(Self::io_error)
    }
    
    fn read_to_bytes(&self, path: &Path) -> Result<Vec<u8>> {
        fs::read(self.resolve_path(path)).map_err(Self::io_error)
    }
    
    fn write_string(&self, path: &Path, contents: &str) -> Result<()> {
        fs::write(self.resolve_path(path), contents).map_err(Self::io_error)
    }
    
    fn write_bytes(&self, path: &Path, contents: &[u8]) -> Result<()> {
        fs::write(self.resolve_path(path), contents).map_err(Self::io_error)
    }
    
    fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        fs::copy(self.resolve_path(from), self.resolve_path(to))
            .map(|_| ())
            .map_err(Self::io_error)
    }
    
    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        fs::rename(self.resolve_path(from), self.resolve_path(to))
            .map_err(Self::io_error)
    }
    
    fn remove_file(&self, path: &Path) -> Result<()> {
        fs::remove_file(self.resolve_path(path)).map_err(Self::io_error)
    }
    
    fn remove_dir(&self, path: &Path) -> Result<()> {
        fs::remove_dir(self.resolve_path(path)).map_err(Self::io_error)
    }
    
    fn remove_dir_all(&self, path: &Path) -> Result<()> {
        fs::remove_dir_all(self.resolve_path(path)).map_err(Self::io_error)
    }
    
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let entries = fs::read_dir(self.resolve_path(path)).map_err(Self::io_error)?;
        let mut result = Vec::new();
        
        for entry in entries {
            let entry = entry.map_err(Self::io_error)?;
            let path = entry.path();
            
            // Convert back to a relative path if possible
            if let Ok(rel_path) = path.strip_prefix(&self.base_path) {
                result.push(rel_path.to_path_buf());
            } else {
                result.push(path);
            }
        }
        
        Ok(result)
    }
    
    fn metadata(&self, path: &Path) -> Result<std::fs::Metadata> {
        fs::metadata(self.resolve_path(path)).map_err(Self::io_error)
    }
    
    fn open_read(&self, path: &Path) -> Result<Box<dyn Read>> {
        let file = File::open(self.resolve_path(path)).map_err(Self::io_error)?;
        Ok(Box::new(file))
    }
    
    fn open_write(&self, path: &Path) -> Result<Box<dyn Write>> {
        let file = File::create(self.resolve_path(path)).map_err(Self::io_error)?;
        Ok(Box::new(file))
    }
    
    fn open_append(&self, path: &Path) -> Result<Box<dyn Write>> {
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(self.resolve_path(path))
            .map_err(Self::io_error)?;
        Ok(Box::new(file))
    }
}