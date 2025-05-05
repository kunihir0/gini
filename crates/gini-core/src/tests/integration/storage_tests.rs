#![cfg(test)]

use tokio::test;
use std::fs;
use crate::storage::provider::StorageProvider;

use super::common::setup_test_environment;

#[test]
async fn test_storage_file_operations() {
    // Destructure all 6 return values, ignoring unused ones
    let (_, _, storage_manager, _, _, _) = setup_test_environment().await;
    
    // Create a proper temp path
    let temp_dir = std::env::temp_dir().join("gini_test");
    fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");
    let test_path = temp_dir.join("test_file.txt");
    let test_data = "Test storage data";
    
    // Save data
    storage_manager.write_string(&test_path, test_data)
        .expect("Failed to save data");
    
    // Verify data exists
    let exists = storage_manager.exists(&test_path);
    assert!(exists, "Data should exist after saving");
    
    // Load and verify data
    let loaded = storage_manager.read_to_string(&test_path)
        .expect("Failed to load data");
    assert_eq!(loaded, test_data, "Loaded data does not match saved data");
    
    // Remove data
    storage_manager.remove_file(&test_path)
        .expect("Failed to delete data");
    
    // Verify data no longer exists
    let exists = storage_manager.exists(&test_path);
    assert!(!exists, "Data should not exist after deleting");

    // Clean up
    let _ = fs::remove_dir_all(temp_dir);
}

// Add necessary imports if not already present
use std::io::{Write}; // For append test

// --- Test: Storage Directory Operations ---
#[test]
async fn test_storage_directory_operations() {
    let (_, _, storage_manager, _, _, _) = setup_test_environment().await;
    let base_dir = std::env::temp_dir().join("gini_test_dir_ops");
    let test_dir = base_dir.join("my_test_dir");
    let file_in_dir = test_dir.join("file1.txt");
    let nested_dir = test_dir.join("nested");
    let file_in_nested = nested_dir.join("file2.txt");

    // Cleanup previous runs if any
    let _ = storage_manager.remove_dir_all(&base_dir);

    // Ensure base directory exists before creating subdirectory
    storage_manager.create_dir_all(&base_dir).expect("Failed to create base directory for dir ops test");

    // 1. Create Directory
    storage_manager.create_dir(&test_dir).expect("Failed to create test directory");
    assert!(storage_manager.is_dir(&test_dir), "Test directory should exist and be a directory");

    // 2. Create files/nested dirs inside
    storage_manager.write_string(&file_in_dir, "content1").expect("Failed to write file in dir");
    storage_manager.create_dir_all(&nested_dir).expect("Failed to create nested directory"); // Use create_dir_all for nested
    storage_manager.write_string(&file_in_nested, "content2").expect("Failed to write file in nested dir");
    assert!(storage_manager.is_file(&file_in_dir), "File in directory should exist");
    assert!(storage_manager.is_dir(&nested_dir), "Nested directory should exist");
    assert!(storage_manager.is_file(&file_in_nested), "File in nested directory should exist");


    // 3. Read Directory
    let entries = storage_manager.read_dir(&test_dir).expect("Failed to read directory");
    assert_eq!(entries.len(), 2, "Should contain the file and the nested directory");
    // Note: read_dir might not guarantee order. Check for presence.
    assert!(entries.iter().any(|p| p.file_name().unwrap() == "file1.txt"), "read_dir should list file1.txt");
    assert!(entries.iter().any(|p| p.file_name().unwrap() == "nested"), "read_dir should list nested dir");

    // 4. Remove Empty Directory (should fail as test_dir is not empty)
    let remove_empty_res = storage_manager.remove_dir(&test_dir);
    assert!(remove_empty_res.is_err(), "remove_dir should fail on non-empty directory");

    // 5. Remove Directory Recursively
    storage_manager.remove_dir_all(&test_dir).expect("Failed to remove directory recursively");
    assert!(!storage_manager.exists(&test_dir), "Directory should not exist after remove_dir_all");
    assert!(!storage_manager.exists(&file_in_dir), "File inside should also be removed");
    assert!(!storage_manager.exists(&nested_dir), "Nested dir inside should also be removed");
    assert!(!storage_manager.exists(&file_in_nested), "File in nested dir should also be removed");

    // Cleanup base
    let _ = storage_manager.remove_dir_all(&base_dir);
}


// --- Test: Storage Copy and Rename ---
#[test]
async fn test_storage_copy_and_rename() {
    let (_, _, storage_manager, _, _, _) = setup_test_environment().await;
    let base_dir = std::env::temp_dir().join("gini_test_copy_rename");
    let file_a = base_dir.join("file_a.txt");
    let file_b = base_dir.join("file_b.txt");
    let file_c = base_dir.join("file_c.txt");
    let content_a = "Original content for copy/rename";

    // Cleanup previous runs if any
    let _ = storage_manager.remove_dir_all(&base_dir);
    storage_manager.create_dir_all(&base_dir).expect("Failed to create base dir");

    // Create initial file A
    storage_manager.write_string(&file_a, content_a).expect("Failed to write file A");
    assert!(storage_manager.exists(&file_a), "File A should exist");

    // 1. Copy A to B
    storage_manager.copy(&file_a, &file_b).expect("Failed to copy A to B");
    assert!(storage_manager.exists(&file_a), "File A should still exist after copy");
    assert!(storage_manager.exists(&file_b), "File B should exist after copy");
    let content_b = storage_manager.read_to_string(&file_b).expect("Failed to read file B");
    assert_eq!(content_b, content_a, "Content of file B should match file A after copy");

    // 2. Rename B to C
    storage_manager.rename(&file_b, &file_c).expect("Failed to rename B to C");
    assert!(storage_manager.exists(&file_a), "File A should still exist after rename");
    assert!(!storage_manager.exists(&file_b), "File B should NOT exist after rename");
    assert!(storage_manager.exists(&file_c), "File C should exist after rename");
    let content_c = storage_manager.read_to_string(&file_c).expect("Failed to read file C");
    assert_eq!(content_c, content_a, "Content of file C should match original content after rename");

    // Cleanup base
    let _ = storage_manager.remove_dir_all(&base_dir);
}


// --- Test: Storage Metadata Retrieval ---
#[test]
async fn test_storage_metadata_retrieval() {
    let (_, _, storage_manager, _, _, _) = setup_test_environment().await;
    let base_dir = std::env::temp_dir().join("gini_test_metadata");
    let test_file = base_dir.join("metadata_test.txt");
    let test_content = "Some data for metadata test";

    // Cleanup previous runs if any
    let _ = storage_manager.remove_dir_all(&base_dir);
    storage_manager.create_dir_all(&base_dir).expect("Failed to create base dir");

    // Create file
    storage_manager.write_string(&test_file, test_content).expect("Failed to write metadata test file");

    // Get metadata
    let metadata = storage_manager.metadata(&test_file).expect("Failed to get metadata");

    // Verify metadata properties
    assert!(metadata.is_file(), "Metadata should indicate it's a file");
    assert!(!metadata.is_dir(), "Metadata should not indicate it's a directory");
    assert_eq!(metadata.len(), test_content.len() as u64, "Metadata size should match content length");
    // Check modification time is somewhat recent (allow some tolerance)
    let modified = metadata.modified().expect("Failed to get modification time");
    let now = std::time::SystemTime::now();
    let duration_since_modified = now.duration_since(modified).expect("Modification time is in the future?");
    assert!(duration_since_modified.as_secs() < 5, "Modification time seems too old"); // Allow 5 seconds difference

    // Cleanup base
    let _ = storage_manager.remove_dir_all(&base_dir);
}


// --- Test: Storage Byte Level Operations ---
#[test]
async fn test_storage_byte_level_operations() {
    let (_, _, storage_manager, _, _, _) = setup_test_environment().await;
    let base_dir = std::env::temp_dir().join("gini_test_bytes");
    let test_file = base_dir.join("byte_test.bin");
    let test_bytes: Vec<u8> = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04];

    // Cleanup previous runs if any
    let _ = storage_manager.remove_dir_all(&base_dir);
    storage_manager.create_dir_all(&base_dir).expect("Failed to create base dir");

    // Write bytes
    storage_manager.write_bytes(&test_file, &test_bytes).expect("Failed to write bytes");
    assert!(storage_manager.exists(&test_file), "Byte file should exist after writing");

    // Read bytes back
    let read_bytes = storage_manager.read_to_bytes(&test_file).expect("Failed to read bytes");

    // Verify bytes match
    assert_eq!(read_bytes, test_bytes, "Read bytes do not match written bytes");

    // Cleanup base
    let _ = storage_manager.remove_dir_all(&base_dir);
}


// --- Test: Storage Append Operations ---
#[test]
async fn test_storage_append_operations() {
    let (_, _, storage_manager, _, _, _) = setup_test_environment().await;
    let base_dir = std::env::temp_dir().join("gini_test_append");
    let test_file = base_dir.join("append_test.log");
    let initial_content = "Initial line.\n";
    let appended_content = "Appended line.\n";
    let expected_content = "Initial line.\nAppended line.\n";

    // Cleanup previous runs if any
    let _ = storage_manager.remove_dir_all(&base_dir);
    storage_manager.create_dir_all(&base_dir).expect("Failed to create base dir");

    // Write initial content
    storage_manager.write_string(&test_file, initial_content).expect("Failed to write initial content");

    // Open in append mode and write
    {
        let mut file_handle = storage_manager.open_append(&test_file).expect("Failed to open file for appending");
        file_handle.write_all(appended_content.as_bytes()).expect("Failed to write appended content");
        // Handle is dropped here, ensuring flush/close
    }

    // Read the whole file back
    let final_content = storage_manager.read_to_string(&test_file).expect("Failed to read file after append");

    // Verify combined content
    assert_eq!(final_content, expected_content, "File content after append is incorrect");

    // Cleanup base
    let _ = storage_manager.remove_dir_all(&base_dir);
}
