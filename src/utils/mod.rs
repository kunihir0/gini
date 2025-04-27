pub mod fs;

use std::path::Path;
use std::io;

/// Check if a path exists
pub fn path_exists<P: AsRef<Path>>(path: P) -> bool {
    Path::new(path.as_ref()).exists()
}

/// Check if a path is a file
pub fn is_file<P: AsRef<Path>>(path: P) -> bool {
    Path::new(path.as_ref()).is_file()
}

/// Check if a path is a directory
pub fn is_dir<P: AsRef<Path>>(path: P) -> bool {
    Path::new(path.as_ref()).is_dir()
}

/// Get the file name from a path
pub fn file_name<P: AsRef<Path>>(path: P) -> Option<String> {
    path.as_ref()
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
}

/// Get the file stem (name without extension) from a path
pub fn file_stem<P: AsRef<Path>>(path: P) -> Option<String> {
    path.as_ref()
        .file_stem()
        .map(|name| name.to_string_lossy().to_string())
}

/// Get the file extension from a path
pub fn file_extension<P: AsRef<Path>>(path: P) -> Option<String> {
    path.as_ref()
        .extension()
        .map(|ext| ext.to_string_lossy().to_string())
}

/// Create a directory recursively
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    std::fs::create_dir_all(path)
}

/// Write string to a file
pub fn write_string<P: AsRef<Path>, C: AsRef<str>>(path: P, content: C) -> io::Result<()> {
    std::fs::write(path, content.as_ref())
}

/// Read a file to string
pub fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    std::fs::read_to_string(path)
}

/// Copy a file
pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<u64> {
    std::fs::copy(from, to)
}

/// Move/rename a file
pub fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    std::fs::rename(from, to)
}

/// Remove a file
pub fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    std::fs::remove_file(path)
}

/// Remove a directory
pub fn remove_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    std::fs::remove_dir(path)
}

/// Remove a directory recursively
pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    std::fs::remove_dir_all(path)
}