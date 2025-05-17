#![cfg(test)]

use crate::plugin_system::loader::PluginLoader;
use crate::plugin_system::manifest::PluginManifest;
use crate::plugin_system::version::VersionRange; // Removed ApiVersion
use crate::kernel::error::Error as KernelError; // Corrected import
use crate::plugin_system::error::PluginSystemError; // Import PluginSystemError
use std::path::PathBuf; // Removed Path
use std::process::Command;
use tempfile::{tempdir, TempDir};
// Removed unused Arc, ConfigManager, EventManager, PluginRegistry, StageManager, StorageManager, UiBridge, RealFileSystemProvider
// Removed unused PluginSource, PluginManager, PluginLoadError, PluginRegistryError from kernel::error

// Helper function to compile a test plugin
// Returns the path to the compiled .so file, the library name, and the TempDir for the target (to keep it alive)
fn compile_test_plugin(plugin_crate_name: &str, plugin_project_subpath: &str) -> Result<(PathBuf, String, TempDir), String> {
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")); // Should be crates/gini-core
    let plugin_project_path = base_dir.join(plugin_project_subpath);

    let target_dir = tempdir().map_err(|e| format!("Failed to create temp dir for compilation: {}", e))?;

    let status = Command::new("cargo")
        .current_dir(&plugin_project_path)
        .arg("build")
        // .arg("--release") // Using debug builds for tests is faster and often sufficient
        .arg("--target-dir")
        .arg(target_dir.path())
        .status()
        .map_err(|e| format!("Failed to execute cargo build for {}: {}", plugin_crate_name, e))?;

    if !status.success() {
        // Attempt to read stderr for more detailed error output
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&plugin_project_path)
            .arg("build")
            // .arg("--release")
            .arg("--target-dir")
            .arg(target_dir.path());
        
        let output = cmd.output().map_err(|e| format!("Failed to execute cargo build (for error report) for {}: {}", plugin_crate_name, e))?;
        
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Failed to compile test plugin {}: cargo build exited with status {:?}.\nStderr:\n{}",
            plugin_crate_name, status, stderr
        ));
    }

    // Construct the path to the .so file
    // On Linux, it's lib<crate_name>.so. On macOS, lib<crate_name>.dylib. On Windows, <crate_name>.dll.
    // The crate name is specified in the [lib] name field of the plugin's Cargo.toml
    let lib_name_stem = plugin_crate_name; // This is the name from [lib] name = "..." in plugin's Cargo.toml
    
    // Determine library extension based on target OS
    let lib_extension = if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so" // Default to .so for Linux and other Unix-like systems
    };
    let lib_filename = format!("lib{}.{}", lib_name_stem, lib_extension);
    
    // Path will be target_dir/debug/lib<name>.so (or release)
    let build_profile = "debug"; // or "release" if --release is used
    let so_path = target_dir.path().join(build_profile).join(&lib_filename);

    if !so_path.exists() {
        // Try to list files in the target directory for debugging
        let entries = std::fs::read_dir(target_dir.path().join(build_profile))
            .map(|rd| rd.filter_map(Result::ok).map(|e| e.path().display().to_string()).collect::<Vec<_>>().join(", "))
            .unwrap_or_else(|e| format!("Could not read dir: {}", e));
        return Err(format!(
            "Compiled plugin library not found at {}. Project path: {}, crate_name: {}. Contents of {}: [{}]",
            so_path.display(),
            plugin_project_path.display(),
            plugin_crate_name,
            target_dir.path().join(build_profile).display(),
            entries
        ));
    }
    Ok((so_path, lib_filename, target_dir))
}

fn create_plugin_loader() -> PluginLoader {
    PluginLoader::new()
}

fn create_test_manifest(id: &str, entry_point: &str, plugin_base_dir: PathBuf) -> PluginManifest {
    PluginManifest {
        id: id.to_string(),
        name: format!("Test Plugin {}", id),
        version: "0.1.0".to_string(),
        description: "A test plugin".to_string(),
        author: "Gini Tests".to_string(),
        website: None,
        license: None,
        api_versions: vec![VersionRange::from_constraint("0.1.0").expect("Failed to parse default API version for test manifest")],
        dependencies: vec![],
        is_core: false,
        priority: None,
        entry_point: entry_point.to_string(),
        files: vec![],
        config_schema: None,
        tags: vec![],
        conflicts_with: vec![],
        incompatible_with: vec![],
        resources: Vec::new(),
        plugin_base_dir,
    }
}

#[tokio::test]
async fn test_load_missing_symbol_plugin() {
    let plugin_loader = create_plugin_loader();
    let plugin_crate_name = "missing_symbol_plugin";
    let (so_path, lib_filename, _target_dir) = compile_test_plugin(plugin_crate_name, "tests/test_plugins/failing_ffi/missing_symbol_plugin")
        .expect("Failed to compile missing_symbol_plugin");

    let manifest = create_test_manifest(plugin_crate_name, &lib_filename, so_path.parent().unwrap().to_path_buf());
    let result = plugin_loader.load_plugin(&manifest).await;

    if result.is_ok() {
        panic!("Expected loading to fail for plugin '{}', but it succeeded.", manifest.id);
    }
    match result.err().unwrap() {
        KernelError::PluginSystem(PluginSystemError::LoadingError { plugin_id, path: _, source }) => { // Bind plugin_id
            let source_string = source.to_string();
            assert!(source_string.contains("missing symbol _plugin_init"), "Unexpected error message for missing symbol: {}", source_string);
            assert!(source_string.contains(&lib_filename) || plugin_id.contains(&lib_filename), "Error message should contain plugin filename: {}. Actual plugin_id: {}", source_string, plugin_id);
        }
        other_error => panic!("Expected KernelError::PluginSystem(PluginSystemError::LoadingError), got {:?}", other_error),
    }
}

#[tokio::test]
#[ignore = "Currently, panics in `extern \"C-unwind\"` FFI functions loaded via libloading can abort the test process despite std::panic::catch_unwind, due to complexities in FFI panic handling. This test documents that behavior."]
async fn test_load_init_panic_plugin() {
    let plugin_loader = create_plugin_loader();
    let plugin_crate_name = "init_panic_plugin";
    let (so_path, lib_filename, _target_dir) = compile_test_plugin(plugin_crate_name, "tests/test_plugins/failing_ffi/init_panic_plugin")
        .expect("Failed to compile init_panic_plugin");

    let manifest = create_test_manifest(plugin_crate_name, &lib_filename, so_path.parent().unwrap().to_path_buf());
    let result = plugin_loader.load_plugin(&manifest).await;
    
    if result.is_ok() {
        panic!("Expected loading to fail due to panic for plugin '{}', but it succeeded.", manifest.id);
    }
    match result.err().unwrap() {
        KernelError::PluginSystem(PluginSystemError::FfiError { plugin_id, operation: _, message }) => {
            assert_eq!(plugin_id, plugin_crate_name);
            assert!(message.contains("panic: Plugin initialization deliberately failed!"), "Unexpected error message for init panic: {}", message);
        }
        other_error => panic!("Expected KernelError::PluginSystem(PluginSystemError::FfiError) for InitPanic, got {:?}", other_error),
    }
}

#[tokio::test]
async fn test_load_invalid_vtable_plugin() {
    let plugin_loader = create_plugin_loader();
    let plugin_crate_name = "invalid_vtable_plugin";
    let (so_path, lib_filename, _target_dir) = compile_test_plugin(plugin_crate_name, "tests/test_plugins/failing_ffi/invalid_vtable_plugin")
        .expect("Failed to compile invalid_vtable_plugin");

    let manifest = create_test_manifest(plugin_crate_name, &lib_filename, so_path.parent().unwrap().to_path_buf());
    let result = plugin_loader.load_plugin(&manifest).await;

    if result.is_ok() {
        panic!("Expected loading to fail due to invalid VTable for plugin '{}', but it succeeded.", manifest.id);
    }
    match result.err().unwrap() {
        KernelError::PluginSystem(PluginSystemError::LoadingError { plugin_id, path: _, source }) => {
            // plugin_id in LoadingError is the path when VTable loading fails early
            assert!(plugin_id.contains(plugin_crate_name), "Expected plugin_id ('{}') to contain crate name ('{}')", plugin_id, plugin_crate_name);
            let source_string = source.to_string();
            assert!(source_string.contains("Plugin init returned null VTable") || source_string.contains("Received null VTable pointer"), "Unexpected error message for invalid VTable: {}", source_string);
        }
        other_error => panic!("Expected KernelError::PluginSystem(PluginSystemError::LoadingError) for Invalid VTable, got {:?}", other_error),
    }
}