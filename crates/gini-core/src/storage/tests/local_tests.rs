use std::path::{Path, PathBuf};
use std::fs;
use tempfile::tempdir;

use crate::kernel::error::Result;
use crate::storage::provider::StorageProvider;
use crate::storage::local::LocalStorageProvider;

// Helper function to create PathBuf from str for tests
fn p(s: &str) -> PathBuf {
    PathBuf::from(s)
}

#[test]
fn test_write_and_read_bytes() -> Result<()> {
    // Create temp directory for test
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let root_path = temp_dir.path().to_path_buf();

    // Initialize provider
    let provider = LocalStorageProvider::new(root_path);

    // Write data using write_bytes
    let key_path = p("test.key");
    let data = b"test data".to_vec();
    provider.write_bytes(&key_path, &data)?;

    // Read data using read_to_bytes
    let retrieved = provider.read_to_bytes(&key_path)?;

    // Verify data was stored and retrieved correctly
    assert_eq!(retrieved, data);

    Ok(())
}

#[test]
fn test_remove_file() -> Result<()> {
    // Create temp directory for test
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let root_path = temp_dir.path().to_path_buf();

    // Initialize provider
    let provider = LocalStorageProvider::new(root_path);

    // Write data
    let key_path = p("test.key");
    let data = b"test data".to_vec();
    provider.write_bytes(&key_path, &data)?;

    // Verify data exists using the correct exists signature
    assert!(provider.exists(&key_path), "Data should exist after writing");

    // Remove file using remove_file
    provider.remove_file(&key_path)?;

    // Verify data was deleted
    assert!(!provider.exists(&key_path), "Data should not exist after deletion");

    Ok(())
}

#[test]
fn test_read_dir() -> Result<()> {
    // Create temp directory for test
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let root_path = temp_dir.path().to_path_buf();

    // Initialize provider
    let provider = LocalStorageProvider::new(root_path.clone());

    // Create a subdirectory and files within it
    let sub_dir = p("subdir");
    provider.create_dir_all(&sub_dir)?; // Use create_dir_all for nested paths

    let keys = vec![
        sub_dir.join("key1.txt"),
        sub_dir.join("key2.dat"),
        sub_dir.join("key3"),
    ];
    let data = b"test data".to_vec();

    for key_path in &keys {
        provider.write_bytes(key_path, &data)?;
    }

    // Create an empty directory as well
    let empty_sub_dir = p("empty_subdir");
    provider.create_dir(&empty_sub_dir)?;

    // List entries in the root directory
    let root_entries = provider.read_dir(&p(""))?; // Read root
    assert_eq!(root_entries.len(), 2, "Root should contain subdir and empty_subdir");
    assert!(root_entries.contains(&sub_dir));
    assert!(root_entries.contains(&empty_sub_dir));


    // List entries in the subdirectory using read_dir
    let listed_paths = provider.read_dir(&sub_dir)?;

    // Verify all keys are listed (read_dir returns relative paths from base)
    assert_eq!(listed_paths.len(), keys.len(), "Number of listed paths should match created files");
    for key_path in keys {
        // read_dir returns paths relative to the base_path
        assert!(listed_paths.contains(&key_path), "Listed paths should contain '{:?}'", key_path);
    }

    Ok(())
}

#[test]
fn test_nested_keys_paths() -> Result<()> {
    // Create temp directory for test
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let root_path = temp_dir.path().to_path_buf();

    // Initialize provider
    let provider = LocalStorageProvider::new(root_path);

    // Create nested directory structure first
    let nested_dir = p("nested/test");
    provider.create_dir_all(&nested_dir)?;

    // Store data with nested key path
    let key_path = nested_dir.join("key.file");
    let data = b"nested test data".to_vec();
    provider.write_bytes(&key_path, &data)?;

    // Verify data exists
    assert!(provider.exists(&key_path), "Data should exist with nested key path");
    assert!(provider.is_file(&key_path), "Path should be a file");

    // Retrieve data
    let retrieved = provider.read_to_bytes(&key_path)?;
    assert_eq!(retrieved, data, "Retrieved data should match stored data");

    // List keys in the nested directory
    let listed_paths = provider.read_dir(&nested_dir)?;
    assert_eq!(listed_paths.len(), 1);
    assert!(listed_paths.contains(&key_path), "Listed paths should contain nested key path");

    Ok(())
}

#[test]
fn test_is_file_and_is_dir() -> Result<()> {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let root_path = temp_dir.path().to_path_buf();
    let provider = LocalStorageProvider::new(root_path);

    let file_path = p("my_file.txt");
    let dir_path = p("my_dir");

    // Create file and directory
    provider.write_bytes(&file_path, b"hello")?;
    provider.create_dir(&dir_path)?;

    // Check types
    assert!(provider.is_file(&file_path));
    assert!(!provider.is_dir(&file_path));

    assert!(provider.is_dir(&dir_path));
    assert!(!provider.is_file(&dir_path));

    // Check non-existent path
    let non_existent = p("not_real");
    assert!(!provider.is_file(&non_existent));
    assert!(!provider.is_dir(&non_existent));

    Ok(())
}