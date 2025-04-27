// Import the specific function from the parent module
use crate::utils::create_dir_all;
// Keep std::fs for file operations within the test
use std::fs::{self, File};
use std::path::Path;
use tempfile::tempdir;
// Commented out unused fs submodule import
// use crate::utils::fs::find_files_with_extension;

#[test]
fn test_create_dir_all_wrapper() { // Renamed test
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    // Test creating a new directory
    let new_dir = base_path.join("new_dir/nested"); // Test nested creation
    assert!(!new_dir.exists());
    let result = create_dir_all(&new_dir); // Use the wrapper from utils
    assert!(result.is_ok());
    assert!(new_dir.exists());
    assert!(new_dir.is_dir());

    // Test ensuring an existing directory exists
    let result_existing = create_dir_all(&new_dir);
    assert!(result_existing.is_ok()); // Should succeed if it already exists
    assert!(new_dir.exists());
    assert!(new_dir.is_dir());

    // Test ensuring a path where a file exists (should fail)
    let file_path = base_path.join("existing_file.txt");
    File::create(&file_path).unwrap();
    assert!(file_path.exists());
    assert!(file_path.is_file());
    let result_file = create_dir_all(&file_path); // Use the wrapper
    assert!(result_file.is_err()); // Should fail because a file exists there

    // Clean up
    fs::remove_file(file_path).unwrap();
    fs::remove_dir_all(base_path.join("new_dir")).unwrap(); // Remove the top level dir created
}

// TODO: Add tests for find_files_with_extension once its implementation is confirmed/added.
/*
#[test]
fn test_find_files_with_extension() {
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    // Create test structure
    let sub1 = base_path.join("sub1");
    let sub2 = base_path.join("sub2");
    let sub1_sub = sub1.join("sub1_sub");
    fs::create_dir_all(&sub1_sub).unwrap();
    fs::create_dir(&sub2).unwrap();

    // Create files with different extensions
    File::create(base_path.join("root.txt")).unwrap();
    File::create(base_path.join("root.log")).unwrap();
    File::create(sub1.join("file1.txt")).unwrap();
    File::create(sub1.join("file2.dat")).unwrap();
    File::create(sub1_sub.join("nested.txt")).unwrap();
    File::create(sub2.join("another.log")).unwrap();
    File::create(sub2.join("no_ext")).unwrap();

    // Find .txt files
    let txt_files = find_files_with_extension(base_path, "txt").unwrap();
    assert_eq!(txt_files.len(), 3);
    assert!(txt_files.contains(&base_path.join("root.txt")));
    assert!(txt_files.contains(&sub1.join("file1.txt")));
    assert!(txt_files.contains(&sub1_sub.join("nested.txt")));

    // Find .log files
    let log_files = find_files_with_extension(base_path, "log").unwrap();
    assert_eq!(log_files.len(), 2);
    assert!(log_files.contains(&base_path.join("root.log")));
    assert!(log_files.contains(&sub2.join("another.log")));

    // Find .dat files
    let dat_files = find_files_with_extension(base_path, "dat").unwrap();
    assert_eq!(dat_files.len(), 1);
    assert!(dat_files.contains(&sub1.join("file2.dat")));

    // Find non-existent extension
    let png_files = find_files_with_extension(base_path, "png").unwrap();
    assert!(png_files.is_empty());

    // Test on a non-existent directory
    let non_existent_path = base_path.join("not_real");
    let result_non_existent = find_files_with_extension(&non_existent_path, "txt");
    assert!(result_non_existent.is_err()); // Should return an error

    // Clean up
    fs::remove_dir_all(base_path).unwrap();
}
*/