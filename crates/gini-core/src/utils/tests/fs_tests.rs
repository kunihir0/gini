// Import the specific function from the parent module
use crate::utils::create_dir_all;
// Keep std::fs for file operations within the test
use std::fs::{self, File};
use tempfile::tempdir;
use crate::utils::fs::find_files_with_extension;

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
    let empty_dir = base_path.join("empty_dir");
    fs::create_dir(&empty_dir).unwrap();
    let dir_with_other_files = base_path.join("dir_with_other_files");
    fs::create_dir(&dir_with_other_files).unwrap();


    // Create files with different extensions
    File::create(base_path.join("root.txt")).unwrap();
    File::create(base_path.join("ROOT.TXT")).unwrap(); // For case-insensitivity test
    File::create(base_path.join("root.log")).unwrap();
    File::create(sub1.join("file1.txt")).unwrap();
    File::create(sub1.join("file2.dat")).unwrap();
    File::create(sub1_sub.join("nested.txt")).unwrap();
    File::create(sub1_sub.join("NESTED.TXT")).unwrap(); // For case-insensitivity test
    File::create(sub2.join("another.log")).unwrap();
    File::create(sub2.join("no_ext")).unwrap();
    File::create(dir_with_other_files.join("some.other")).unwrap();


    // 1. Finding files in a directory with a specific extension (recursive, case-insensitive)
    let txt_files = find_files_with_extension(base_path, "txt").unwrap();
    assert_eq!(txt_files.len(), 5, "Should find all .txt and .TXT files recursively");
    assert!(txt_files.contains(&base_path.join("root.txt")));
    assert!(txt_files.contains(&base_path.join("ROOT.TXT")));
    assert!(txt_files.contains(&sub1.join("file1.txt")));
    assert!(txt_files.contains(&sub1_sub.join("nested.txt")));
    assert!(txt_files.contains(&sub1_sub.join("NESTED.TXT")));


    // Find .log files
    let log_files = find_files_with_extension(base_path, "log").unwrap();
    assert_eq!(log_files.len(), 2);
    assert!(log_files.contains(&base_path.join("root.log")));
    assert!(log_files.contains(&sub2.join("another.log")));

    // Find .dat files
    let dat_files = find_files_with_extension(base_path, "dat").unwrap();
    assert_eq!(dat_files.len(), 1);
    assert!(dat_files.contains(&sub1.join("file2.dat")));

    // 2. Directory with no files of the specified extension
    let png_files = find_files_with_extension(base_path, "png").unwrap();
    assert!(png_files.is_empty(), "Should find no .png files");

    let result_other_files = find_files_with_extension(&dir_with_other_files, "txt").unwrap();
    assert!(result_other_files.is_empty(), "Should find no .txt files in dir_with_other_files");

    // 3. Empty directory
    let result_empty_dir = find_files_with_extension(&empty_dir, "txt").unwrap();
    assert!(result_empty_dir.is_empty(), "Should find no files in an empty directory");

    // 4. Recursive search is implicitly tested above with subdirectories.

    // 5. Case sensitivity of the extension (function is case-insensitive)
    let txt_files_case_insensitive_search_upper = find_files_with_extension(base_path, "TXT").unwrap();
    assert_eq!(txt_files_case_insensitive_search_upper.len(), 5, "Search with 'TXT' should yield same results");
    assert!(txt_files_case_insensitive_search_upper.contains(&base_path.join("root.txt")));
    assert!(txt_files_case_insensitive_search_upper.contains(&base_path.join("ROOT.TXT")));

    let txt_files_mixed_case_search = find_files_with_extension(base_path, "tXt").unwrap();
    assert_eq!(txt_files_mixed_case_search.len(), 5, "Search with 'tXt' should yield same results");
    assert!(txt_files_mixed_case_search.contains(&sub1.join("file1.txt")));
    assert!(txt_files_mixed_case_search.contains(&sub1_sub.join("NESTED.TXT")));

    // 6. Invalid directory path (non-existent path)
    // The function `find_files` (called by `find_files_with_extension`) returns Ok(Vec::new()) for non-existent paths.
    let non_existent_path = base_path.join("not_real_at_all");
    let result_non_existent = find_files_with_extension(&non_existent_path, "txt").unwrap();
    assert!(result_non_existent.is_empty(), "Should return empty vec for non-existent path");

    // Test with a path that is a file, not a directory
    // `find_files` handles this: if the path is a file, it checks if that single file matches the predicate.
    let file_as_path_txt = base_path.join("root.txt");
    let result_file_is_path_match = find_files_with_extension(&file_as_path_txt, "txt").unwrap();
    assert_eq!(result_file_is_path_match.len(), 1);
    assert!(result_file_is_path_match.contains(&file_as_path_txt));

    let file_as_path_log = base_path.join("root.log");
    let result_file_is_path_no_match = find_files_with_extension(&file_as_path_log, "txt").unwrap();
    assert!(result_file_is_path_no_match.is_empty());


    // Clean up is handled by temp_dir dropping.
}