use crate::utils::{
    path_exists, is_file, is_dir, file_name, file_stem, file_extension,
    create_dir_all, write_string, remove_file, remove_dir_all,
};
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_path_existence_checks() {
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    let existing_file = base_path.join("exists.txt");
    let existing_dir = base_path.join("exists_dir");
    let non_existent = base_path.join("not_real");

    // Create file and dir
    write_string(&existing_file, "hello").unwrap();
    create_dir_all(&existing_dir).unwrap();

    // Test path_exists
    assert!(path_exists(&existing_file));
    assert!(path_exists(&existing_dir));
    assert!(!path_exists(&non_existent));

    // Test is_file
    assert!(is_file(&existing_file));
    assert!(!is_file(&existing_dir));
    assert!(!is_file(&non_existent));

    // Test is_dir
    assert!(!is_dir(&existing_file));
    assert!(is_dir(&existing_dir));
    assert!(!is_dir(&non_existent));

    // Clean up
    remove_file(&existing_file).unwrap();
    remove_dir_all(&existing_dir).unwrap();
}

#[test]
fn test_file_name_extraction() {
    assert_eq!(file_name("/some/path/to/file.txt"), Some("file.txt".to_string()));
    assert_eq!(file_name("another_file.rs"), Some("another_file.rs".to_string()));
    assert_eq!(file_name(".config"), Some(".config".to_string()));
    assert_eq!(file_name("/"), None);
    assert_eq!(file_name("."), None);
    assert_eq!(file_name(".."), None);
    assert_eq!(file_name("no_extension"), Some("no_extension".to_string()));
}

#[test]
fn test_file_stem_extraction() {
    assert_eq!(file_stem("/some/path/to/file.txt"), Some("file".to_string()));
    assert_eq!(file_stem("another_file.rs"), Some("another_file".to_string()));
    assert_eq!(file_stem(".config"), Some(".config".to_string())); // Stem includes dot
    assert_eq!(file_stem("archive.tar.gz"), Some("archive.tar".to_string()));
    assert_eq!(file_stem("/"), None);
    assert_eq!(file_stem("."), None);
    assert_eq!(file_stem(".."), None);
    assert_eq!(file_stem("no_extension"), Some("no_extension".to_string()));
}

#[test]
fn test_file_extension_extraction() {
    assert_eq!(file_extension("/some/path/to/file.txt"), Some("txt".to_string()));
    assert_eq!(file_extension("another_file.rs"), Some("rs".to_string()));
    assert_eq!(file_extension(".config"), None); // No extension
    assert_eq!(file_extension("archive.tar.gz"), Some("gz".to_string()));
    assert_eq!(file_extension("/"), None);
    assert_eq!(file_extension("."), None);
    assert_eq!(file_extension(".."), None);
    assert_eq!(file_extension("no_extension"), None);
}

// Note: Tests for create_dir_all, write_string, etc., are implicitly covered
// by other tests (like storage tests) that use these utilities or std::fs directly.
// Adding specific tests here would be redundant unless there's complex logic
// within the utils wrappers themselves (which there isn't currently).