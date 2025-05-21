use std::fs;
use std::path::{Path, PathBuf};
use std::io::{self, Write};

/// Find files recursively in a directory that match a predicate
pub fn find_files<P, F>(path: P, predicate: &F) -> io::Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
    F: Fn(&Path) -> bool + ?Sized,
{
    let mut result = Vec::new();

    if !path.as_ref().exists() {
        return Ok(result);
    }

    if path.as_ref().is_file() {
        if predicate(path.as_ref()) {
            result.push(path.as_ref().to_path_buf());
        }
        return Ok(result);
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();

        if entry_path.is_file() {
            if predicate(&entry_path) {
                result.push(entry_path);
            }
        } else if entry_path.is_dir() {
            let mut sub_results = find_files(&entry_path, predicate)?; // Pass predicate directly
            result.append(&mut sub_results);
        }
    }
    
    Ok(result)
}

/// Find files with a specific extension
pub fn find_files_with_extension<P: AsRef<Path>>(path: P, extension: &str) -> io::Result<Vec<PathBuf>> {
    let extension_lower = extension.to_lowercase(); // Renamed to avoid conflict with the captured variable
    find_files(path, &move |p| { // Pass closure by reference
        match p.extension() {
            Some(ext) => ext.to_string_lossy().to_lowercase() == extension_lower,
            None => false,
        }
    })
}

/// Create a temporary directory with a prefix
pub fn create_temp_dir(prefix: &str) -> io::Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    let dir_name = format!("{}_{}", prefix, timestamp);
    let path = temp_dir.join(dir_name);
    
    fs::create_dir_all(&path)?;
    Ok(path)
}

/// Read a file line by line
pub fn read_lines<P: AsRef<Path>>(path: P) -> io::Result<Vec<String>> {
    use std::io::BufRead;
    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    let mut lines = Vec::new();
    
    for line in reader.lines() {
        lines.push(line?);
    }
    
    Ok(lines)
}

/// Append text to a file
pub fn append_to_file<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, content: C) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(path)?;
    
    file.write_all(content.as_ref())?;
    Ok(())
}

/// Get file size
pub fn file_size<P: AsRef<Path>>(path: P) -> io::Result<u64> {
    let metadata = fs::metadata(path)?;
    Ok(metadata.len())
}

/// Check if a file is newer than another file
pub fn is_file_newer<P: AsRef<Path>, Q: AsRef<Path>>(file: P, than: Q) -> io::Result<bool> {
    let file_meta = fs::metadata(file)?;
    let than_meta = fs::metadata(than)?;
    
    let file_time = file_meta.modified()?;
    let than_time = than_meta.modified()?;
    
    Ok(file_time > than_time)
}