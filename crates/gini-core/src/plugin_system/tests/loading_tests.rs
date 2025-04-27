#![cfg(test)]

use crate::kernel::error::Result;
use crate::plugin_system::manager::{DefaultPluginManager, PluginManager};
use std::fs;
use std::path::PathBuf;
use std::env; // Added for current_dir
use tempfile::tempdir; // Using tempfile crate for temporary directories

// Helper function to find the path to the compiled example plugin
fn get_example_plugin_path() -> Option<PathBuf> {
    // Try to find the plugin in various possible locations
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let plugin_name = "libcompat_check_example.so";
    
    // Common locations to search
    let search_paths = vec![
        // From crate directory
        current_dir.join("../../target/debug").join(plugin_name),
        // From workspace root
        current_dir.join("target/debug").join(plugin_name),
        // From various other relative positions
        PathBuf::from("./target/debug").join(plugin_name),
    ];
    
    // Return the first path that exists
    for path in search_paths {
        if path.exists() {
            return Some(path);
        }
    }
    
    None
}

#[tokio::test]
async fn test_load_valid_so_plugin_from_directory() -> Result<()> {
    // 1. Setup: Create temp dir and copy the valid .so file
    let tmp_dir = tempdir().expect("Failed to create temp directory");
    let plugin_dir = tmp_dir.path();
    
    // Get the path to the example plugin
    let example_plugin_src = match get_example_plugin_path() {
        Some(path) => path,
        None => {
            println!("Skipping test: Could not find the compiled example plugin.");
            println!("Please build the 'compat-check-example' plugin first with: cargo build -p compat-check-example");
            return Ok(());
        }
    };

    let plugin_dest = plugin_dir.join("libcompat_check_example.so");
    fs::copy(&example_plugin_src, &plugin_dest).expect("Failed to copy plugin to temp dir");

    // 2. Create Plugin Manager
    let manager = DefaultPluginManager::new()?;

    // 3. Load plugins from the temp directory
    let loaded_count = manager.load_plugins_from_directory(plugin_dir).await?;

    // 4. Assertions
    assert_eq!(loaded_count, 1, "Should have loaded exactly one plugin");

    let registry = manager.registry().lock().await;
    assert!(registry.has_plugin("CompatCheckExample"), "Plugin 'CompatCheckExample' should be registered");
    assert_eq!(registry.plugin_count(), 1, "Registry should contain exactly one plugin");

    Ok(())
}

#[tokio::test]
async fn test_ignore_non_so_files() -> Result<()> {
    // 1. Setup: Create temp dir and add a non-.so file
    let tmp_dir = tempdir().expect("Failed to create temp directory");
    let plugin_dir = tmp_dir.path();
    fs::write(plugin_dir.join("not_a_plugin.txt"), "hello").expect("Failed to write dummy file");

    // 2. Create Plugin Manager
    let manager = DefaultPluginManager::new()?;

    // 3. Load plugins
    let loaded_count = manager.load_plugins_from_directory(plugin_dir).await?;

    // 4. Assertions
    assert_eq!(loaded_count, 0, "Should have loaded zero plugins");
    let registry = manager.registry().lock().await;
    assert_eq!(registry.plugin_count(), 0, "Registry should be empty");

    Ok(())
}

#[tokio::test]
async fn test_load_from_non_existent_directory() -> Result<()> {
    // 1. Setup: Path to a non-existent directory
    let plugin_dir = PathBuf::from("./non_existent_plugin_dir_for_test");
    if plugin_dir.exists() {
        fs::remove_dir_all(&plugin_dir).expect("Failed to remove pre-existing test directory");
    }

    // 2. Create Plugin Manager
    let manager = DefaultPluginManager::new()?;

    // 3. Attempt to load plugins
    let result = manager.load_plugins_from_directory(&plugin_dir).await;

    // 4. Assertions
    assert!(result.is_err(), "Loading from non-existent directory should return an error");
    // Optionally check the specific error type/message if needed

    Ok(())
}

// TODO: Add tests for:
// - File exists but is not a valid library (e.g., corrupted .so or just random binary data)
// - Library loads but is missing the _plugin_init symbol
// - _plugin_init function panics during execution
// These might require compiling custom dummy .so files.