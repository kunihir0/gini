#![cfg(test)]

use tempfile::tempdir;
use tokio::fs as tokio_fs;
use serde_json::json; // Use serde_json for better manifest creation
use std::path::PathBuf; // Added for PathBuf
use std::fs; // Added for fs::copy
use std::env; // Added for env::current_dir
use std::sync::Arc; // Added for Arc

use crate::kernel::bootstrap::Application;
use crate::kernel::component::KernelComponent;
use crate::kernel::error::Result as KernelResult;
use crate::plugin_system::error::PluginSystemError; // Import PluginSystemError
use crate::plugin_system::traits::Plugin;
use crate::plugin_system::version::ApiVersion; // Added ApiVersion
use crate::plugin_system::loader::PluginLoader; // Import PluginLoader
use crate::plugin_system::registry::PluginRegistry; // Added for tests
// Removed: use crate::plugin_system::manifest::PluginManifest;


use super::super::common::{setup_test_environment, TestPlugin};

// Helper function to find the path to the compiled example plugin
// Copied from manager_tests.rs and adapted
fn get_example_plugin_path_for_loading_tests() -> Option<PathBuf> {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    // Relative path from crates/gini-core/src/plugin_system/tests
    // to workspace_root/target/debug/libcompat_check_example.so
    // ../../../../target/debug/libcompat_check_example.so
    let plugin_name = if cfg!(target_os = "windows") {
        "compat_check_example.dll"
    } else if cfg!(target_os = "macos") {
        "libcompat_check_example.dylib"
    } else {
        "libcompat_check_example.so"
    };

    let paths_to_check = [
        // Path relative to crates/gini-core
        current_dir.join("../../target/debug").join(plugin_name),
        // Path relative to workspace root (if tests are run from there)
        current_dir.join("target/debug").join(plugin_name),
        // A common structure if OUT_DIR is used and linked from target/debug
        PathBuf::from("target/debug").join(plugin_name),
    ];

    for path in paths_to_check.iter() {
        if path.exists() {
            return Some(path.clone());
        }
    }
    None
}


#[tokio::test]
async fn test_scan_manifests_io_error_direct_loader() -> KernelResult<()> {
    // Setup environment is not needed here as we test the loader directly

    let tmp_dir = tempdir()?;
    let plugin_base_dir = tmp_dir.path().to_path_buf(); // Clone path

    // Create one valid plugin directory
    let valid_plugin_dir = plugin_base_dir.join("good_plugin");
    tokio_fs::create_dir_all(&valid_plugin_dir).await?;
    // Use serde_json to create valid manifest content
    let valid_manifest_content = json!({
        "id": "good_plugin", // ID should match directory name for current loader logic
        "name": "Good Plugin",
        "version": "1.0.0",
        "description": "A valid plugin",
        "author": "Test",
        "entry_point": "libgood_plugin.so" // Optional but good practice
    }).to_string();
    tokio_fs::write(valid_plugin_dir.join("manifest.json"), valid_manifest_content).await?;


    // Create one directory that will cause an I/O error during manifest loading
    // by naming a directory "manifest.json" where a file is expected.
    let io_error_plugin_dir = plugin_base_dir.join("io_error_plugin");
    tokio_fs::create_dir_all(io_error_plugin_dir.join("manifest.json")).await?; // Create dir named manifest.json

    // --- Test the loader directly ---
    let mut loader = PluginLoader::new();
    loader.add_plugin_dir(&plugin_base_dir); // Add our temp dir

    // Scan manifests - this should encounter the I/O error but continue
    let manifests = loader.scan_for_manifests().await?;

    // Assertions: Only the valid manifest should be loaded.
    assert_eq!(manifests.len(), 1, "Should load only the valid manifest, skipping the one with I/O error");
    assert_eq!(manifests[0].id, "good_plugin", "The ID of the loaded manifest should be 'good_plugin'");

    // We can't easily check stderr logs here, but we verified the function doesn't panic
    // and returns only the valid manifest.

    Ok(())
}

#[tokio::test]
async fn test_static_plugin_initialization_succeeds() -> KernelResult<()> {
    // This test verifies that initializing a statically registered plugin succeeds
    // and does not incorrectly trigger the dynamic loading logic (which is currently stubbed).

    // Destructure all trackers
    let (plugin_manager, stage_manager_arc, _, stages_executed, execution_order, _) = setup_test_environment().await; // Get stage_manager too
    KernelComponent::initialize(&*plugin_manager).await?;
    KernelComponent::initialize(&*stage_manager_arc).await?; // Initialize stage_manager

    // Create a basic, compatible plugin (statically defined, doesn't need actual loading)
    let plugin = TestPlugin::new("StaticInitTestPlugin", stages_executed.clone(), execution_order.clone()); // Pass execution_order, new name
    let plugin_name = plugin.name().to_string();

    // Register the plugin
    let plugin_registry_arc = plugin_manager.registry();
    {
        let mut registry_lock = plugin_registry_arc.lock().await;
        registry_lock.register_plugin(Arc::new(plugin))?; // Use Arc::new
        // Ensure it's enabled but not initialized
        assert!(registry_lock.is_enabled(&plugin_name));
        assert!(!registry_lock.initialized.contains(&plugin_name));
    }

    // Initialize the plugin using initialize_all
    let mut app_for_init = Application::new().unwrap();
    // Get the actual Arc<Mutex<StageRegistry>> from the stage_manager provided by setup_test_environment
    let stage_registry_for_init = stage_manager_arc.registry().registry.clone();
    {
        let mut plugin_registry_locked = plugin_registry_arc.lock().await;
        plugin_registry_locked.initialize_all(&mut app_for_init, &stage_registry_for_init).await
            .expect("initialize_all failed for static plugin initialization test");
    } // plugin_registry_locked is dropped here
    
     // Verify plugin is marked as initialized
    let registry_check = plugin_manager.registry().lock().await;
    assert!(registry_check.initialized.contains(&plugin_name), "Plugin '{}' should be marked as initialized after successful init. Initialized: {:?}", plugin_name, registry_check.initialized);

    Ok(())
}
// --- Integration Tests from plugin_system_coverage_plan.md ---

// Use the actual PluginRegistry struct
use crate::kernel::error::Error as KernelError; // Alias to avoid conflict

// Helper to create a simple manifest JSON string using serde_json
fn create_manifest_json_string(id: &str, version: &str, deps: Option<Vec<(&str, &str, bool)>>, api_versions: Option<Vec<&str>>) -> String {
    let mut manifest_map = serde_json::Map::new();
    manifest_map.insert("id".to_string(), json!(id));
    manifest_map.insert("name".to_string(), json!(id)); // Use id as name for simplicity
    manifest_map.insert("version".to_string(), json!(version));
    manifest_map.insert("description".to_string(), json!("Test Description"));
    manifest_map.insert("author".to_string(), json!("Test Author"));
    manifest_map.insert("entry_point".to_string(), json!(format!("lib{}.so", id))); // Default entry point

    if let Some(deps_vec) = deps {
        let dep_json_array: Vec<_> = deps_vec.iter().map(|(dep_id, dep_ver_req, required)| {
            json!({
                "id": dep_id,
                "version_range": dep_ver_req, // Store the string requirement
                "required": required
            })
        }).collect();
        manifest_map.insert("dependencies".to_string(), json!(dep_json_array));
    }

    if let Some(apis_vec) = api_versions {
         // PluginManifest expects Vec<VersionRange>, but JSON usually stores strings
         // The loader/registry needs to handle parsing these strings.
         // For JSON, store the string representations.
        manifest_map.insert("api_versions".to_string(), json!(apis_vec));
    }

    serde_json::to_string(&manifest_map).unwrap_or_else(|_| "{}".to_string()) // Fallback to empty JSON on error
}


#[tokio::test]
async fn test_scan_manifests_empty_dirs_integration() -> KernelResult<()> {
    let tmp_dir = tempdir()?;
    let empty_dir = tmp_dir.path().join("empty_subdir");
    tokio_fs::create_dir_all(&empty_dir).await?;
    let non_existent_dir = tmp_dir.path().join("non_existent");

    let mut loader = PluginLoader::new();
    loader.add_plugin_dir(&empty_dir);
    loader.add_plugin_dir(&non_existent_dir); // Adding non-existent dir is allowed

    let manifests = loader.scan_for_manifests().await?;
    assert!(manifests.is_empty(), "Scanning empty or non-existent dirs should yield no manifests");

    Ok(())
}

#[tokio::test]
async fn test_scan_manifests_valid_integration() -> KernelResult<()> {
    let tmp_dir = tempdir()?;
    let plugin_base = tmp_dir.path();

    // Plugin 1
    let p1_dir = plugin_base.join("plugin_a");
    tokio_fs::create_dir_all(&p1_dir).await?;
    let p1_manifest_content = create_manifest_json_string("plugin_a", "1.0.0", None, None);
    tokio_fs::write(p1_dir.join("manifest.json"), p1_manifest_content).await?;

    // Plugin 2 (nested)
    let p2_dir = plugin_base.join("subdir/plugin_b");
    tokio_fs::create_dir_all(&p2_dir).await?;
    let p2_manifest_content = create_manifest_json_string("plugin_b", "0.1.0", None, None);
    tokio_fs::write(p2_dir.join("manifest.json"), p2_manifest_content).await?;

    let mut loader = PluginLoader::new();
    loader.add_plugin_dir(plugin_base);

    let manifests = loader.scan_for_manifests().await?;
    assert_eq!(manifests.len(), 2, "Should find two valid manifests");
    // Check IDs derived from path stems by the current loader implementation
    assert!(manifests.iter().any(|m| m.id == "plugin_a"));
    assert!(manifests.iter().any(|m| m.id == "plugin_b"));
    // Verify details (like version) now that the loader parses them correctly.
    assert_eq!(manifests.iter().find(|m| m.id == "plugin_a").unwrap().version, "1.0.0");
    assert_eq!(manifests.iter().find(|m| m.id == "plugin_b").unwrap().version, "0.1.0"); // Correct version from JSON


    Ok(())
}

#[tokio::test]
async fn test_scan_manifests_invalid_json_integration() -> KernelResult<()> {
    let tmp_dir = tempdir()?;
    let plugin_base = tmp_dir.path();

    // Valid Plugin
    let p1_dir = plugin_base.join("plugin_good");
    tokio_fs::create_dir_all(&p1_dir).await?;
    let p1_manifest_content = create_manifest_json_string("plugin_good", "1.0.0", None, None);
    tokio_fs::write(p1_dir.join("manifest.json"), p1_manifest_content).await?;

    // Invalid JSON Plugin
    let p2_dir = plugin_base.join("plugin_bad_json");
    tokio_fs::create_dir_all(&p2_dir).await?;
    tokio_fs::write(p2_dir.join("manifest.json"), r#"{"id": "bad", "version": "1.0.0", invalid json"#).await?;

    // IO Error Plugin (directory named manifest.json) - reusing test from above
    let p3_dir = plugin_base.join("plugin_io_error");
    tokio_fs::create_dir_all(p3_dir.join("manifest.json")).await?;


    let mut loader = PluginLoader::new();
    loader.add_plugin_dir(plugin_base);

    // Scan should log errors but continue and return only valid manifests
    let manifests = loader.scan_for_manifests().await?;

    // It *should* only return the valid one.
    assert_eq!(manifests.len(), 1, "Should only load the one valid manifest");
    assert_eq!(manifests[0].id, "plugin_good"); // ID derived from path stem

    // Cannot easily check stderr logs here, but verify function returns valid subset.

    Ok(())
}

// Removed MockRegistry struct and impl block

#[tokio::test]
async fn test_register_all_plugins_success_integration() -> KernelResult<()> {
    let tmp_dir = tempdir().expect("Failed to create temp directory for test_register_all_plugins_success_integration");
    let plugin_dir_path = tmp_dir.path();

    // Create a manifest for a plugin that can be loaded
    let plugin_id = "test_plugin_for_register";
    let manifest_content = format!(r#"
        {{
            "id": "{}",
            "name": "Test Plugin for Register",
            "version": "1.0.0",
            "description": "A test plugin.",
            "author": "Test Author",
            "api_versions": [">=0.1.0"],
            "entry_point": "libcompat_check_example.so"
        }}
    "#, plugin_id);

    let plugin_subdir = plugin_dir_path.join(plugin_id);
    tokio_fs::create_dir_all(&plugin_subdir).await?;
    tokio_fs::write(plugin_subdir.join("manifest.json"), manifest_content).await?;

    // Copy the actual compiled example plugin to the location specified by entry_point
    let example_plugin_src = get_example_plugin_path_for_loading_tests()
        .expect("Example plugin .so not found. Build 'compat-check-example' first with: cargo build -p compat-check-example");
    let so_dest_path = plugin_subdir.join("libcompat_check_example.so");
    fs::copy(&example_plugin_src, &so_dest_path)?; // Use std::fs for sync copy


    let mut loader = PluginLoader::new();
    loader.add_plugin_dir(plugin_dir_path);
    let scanned_manifests = loader.scan_for_manifests().await?;
    assert_eq!(scanned_manifests.len(), 1, "Should find one manifest");
    assert_eq!(scanned_manifests[0].id, plugin_id);

    let mut registry = PluginRegistry::new(ApiVersion::from_str("0.1.0").unwrap());
    
    // This will call loader.load_plugin internally
    let count = loader.register_all_plugins(&mut registry, &ApiVersion::from_str("0.1.0").unwrap()).await?;
    
    // With PluginLoader::load_plugin implemented, we expect 1 plugin to be loaded.
    assert_eq!(count, 1, "Expected 1 plugin to be loaded successfully");
    assert_eq!(registry.plugin_count(), 1);
    // The actual plugin name from the .so file is "CompatCheckExample"
    assert!(registry.has_plugin("CompatCheckExample"));

    Ok(())
}


#[tokio::test]
async fn test_register_all_plugins_api_incompatible_integration() -> KernelResult<()> {
    let tmp_dir = tempdir()?;
    let plugin_base = tmp_dir.path();

    // Plugin A (compatible API)
    let pa_dir = plugin_base.join("plugin_a");
    tokio_fs::create_dir_all(&pa_dir).await?;
    let pa_manifest_content = create_manifest_json_string("plugin_a", "1.0.0", None, Some(vec!["^1.0"]));
    tokio_fs::write(pa_dir.join("manifest.json"), pa_manifest_content).await?;
    // Create a dummy .so for plugin_a to allow it to attempt loading
    let so_a_path = pa_dir.join("libplugin_a.so");
    tokio_fs::write(&so_a_path, "").await?;


    // Plugin B (incompatible API)
    let pb_dir = plugin_base.join("plugin_b");
    tokio_fs::create_dir_all(&pb_dir).await?;
    let pb_manifest_content = create_manifest_json_string("plugin_b", "1.0.0", None, Some(vec!["^2.0"])); // Requires API v2
    tokio_fs::write(pb_dir.join("manifest.json"), pb_manifest_content).await?;
    // Create a dummy .so for plugin_b
    let so_b_path = pb_dir.join("libplugin_b.so");
    tokio_fs::write(&so_b_path, "").await?;


    let mut loader = PluginLoader::new();
    loader.add_plugin_dir(plugin_base);
    let _ = loader.scan_for_manifests().await?;

    let api_version = crate::plugin_system::version::ApiVersion::new(1, 0, 0); // Define kernel API version (v1)
    // Use the real PluginRegistry
    let mut registry = crate::plugin_system::registry::PluginRegistry::new(api_version.clone());

    // Plugin B should be skipped due to API incompatibility.
    // Plugin A should pass compatibility but fail at the load_plugin stage if libplugin_a.so is empty/invalid.
    // If libplugin_a.so was a *real* plugin, it would load.
    // Since load_plugin is now implemented, an empty .so will cause a loading error.
    let result = loader.register_all_plugins(&mut registry, &api_version).await;

    assert!(result.is_ok(), "register_all_plugins should return Ok even if individual loads fail");
    let registered_count = result.unwrap();
    // Plugin A will attempt to load but fail because libplugin_a.so is empty.
    // Plugin B will be skipped due to API incompatibility.
    assert_eq!(registered_count, 0, "Expected 0 plugins registered (A fails load, B incompatible)");
    assert_eq!(registry.plugin_count(), 0, "Registry count should be 0");


    Ok(())
}

#[tokio::test]
async fn test_register_all_plugins_dep_resolution_fail_integration() -> KernelResult<()> {
    let tmp_dir = tempdir()?;
    let plugin_base = tmp_dir.path();

    // Plugin A depends on missing Plugin B
    let pa_dir = plugin_base.join("plugin_a");
    tokio_fs::create_dir_all(&pa_dir).await?;
    // Ensure the JSON includes the dependency structure register_all_plugins expects
    let pa_manifest_content = create_manifest_json_string("plugin_a", "1.0.0", Some(vec![("plugin_b", "^1.0", true)]), Some(vec!["^1.0"]));
    tokio_fs::write(pa_dir.join("manifest.json"), pa_manifest_content).await?;

    let mut loader = PluginLoader::new();
    loader.add_plugin_dir(plugin_base);
    let _ = loader.scan_for_manifests().await?; // Scan finds plugin_a

    let api_version = crate::plugin_system::version::ApiVersion::new(1, 0, 0); // Define kernel API version
    // Use the real PluginRegistry
    let mut registry = crate::plugin_system::registry::PluginRegistry::new(api_version.clone());

    // Registration should fail during dependency resolution
    let result = loader.register_all_plugins(&mut registry, &api_version).await; // Pass real registry

    assert!(result.is_err(), "Expected registration to fail due to missing dependency");
    match result.err().unwrap() {
        KernelError::PluginSystem(PluginSystemError::DependencyResolution(dep_err)) => {
            let msg = dep_err.to_string();
            // The DependencyError::MissingPlugin format is "Required plugin not found: {0}"
            assert!(msg.contains("Required plugin not found: plugin_b"), "Error message mismatch. Got: {}", msg);
        },
        other_err => panic!("Expected KernelError::PluginSystem(PluginSystemError::DependencyResolution), but got {:?}", other_err),
    }
    assert_eq!(registry.plugin_count(), 0, "Registry should be empty after resolution failure");

    Ok(())
}


#[tokio::test]
async fn test_register_all_plugins_load_fail_integration() -> KernelResult<()> {
    let tmp_dir = tempdir()?;
    let plugin_base = tmp_dir.path();

    // Plugin A (no deps, compatible API, but its .so file will be empty/invalid)
    let pa_dir = plugin_base.join("plugin_a");
    tokio_fs::create_dir_all(&pa_dir).await?;
    let pa_manifest_content = create_manifest_json_string("plugin_a", "1.0.0", None, Some(vec!["^1.0"]));
    tokio_fs::write(pa_dir.join("manifest.json"), pa_manifest_content).await?;
    // Create an empty .so file, which will cause libloading to fail
    let so_a_path = pa_dir.join("libplugin_a.so");
    tokio_fs::write(&so_a_path, "").await?;


    let mut loader = PluginLoader::new();
    loader.add_plugin_dir(plugin_base);
    let _ = loader.scan_for_manifests().await?;

    let api_version = crate::plugin_system::version::ApiVersion::new(1, 0, 0); // Define kernel API version
    // Use the real PluginRegistry
    let mut registry = crate::plugin_system::registry::PluginRegistry::new(api_version.clone());

    // Resolution and compatibility checks should pass for plugin_a.
    // Loading should fail because libplugin_a.so is empty.
    let result = loader.register_all_plugins(&mut registry, &api_version).await;

    assert!(result.is_ok(), "register_all_plugins should return Ok even if loading fails");
    let registered_count = result.unwrap();
    assert_eq!(registered_count, 0, "Expected 0 plugins registered due to load_plugin failure");
    assert_eq!(registry.plugin_count(), 0, "Registry count should be 0");

    Ok(())
}
#[tokio::test]
async fn test_register_all_plugins_dep_version_mismatch() -> KernelResult<()> {
    let tmp_dir = tempdir()?;
    let plugin_base = tmp_dir.path();

    // Plugin A v1.0.0
    let pa_dir = plugin_base.join("plugin_a");
    tokio_fs::create_dir_all(&pa_dir).await?;
    let pa_manifest = create_manifest_json_string("plugin_a", "1.0.0", None, Some(vec!["^1.0"]));
    tokio_fs::write(pa_dir.join("manifest.json"), pa_manifest).await?;

    // Plugin B requires Plugin A v2.0.0
    let pb_dir = plugin_base.join("plugin_b");
    tokio_fs::create_dir_all(&pb_dir).await?;
    let pb_manifest = create_manifest_json_string("plugin_b", "1.0.0", Some(vec![("plugin_a", "^2.0", true)]), Some(vec!["^1.0"]));
    tokio_fs::write(pb_dir.join("manifest.json"), pb_manifest).await?;

    let mut loader = PluginLoader::new();
    loader.add_plugin_dir(plugin_base);
    let _ = loader.scan_for_manifests().await?;

    let api_version = crate::plugin_system::version::ApiVersion::new(1, 0, 0);
    let mut registry = crate::plugin_system::registry::PluginRegistry::new(api_version.clone());

    // Registration should fail during dependency resolution (version mismatch)
    let result = loader.register_all_plugins(&mut registry, &api_version).await;

    assert!(result.is_err(), "Expected registration to fail due to version mismatch");
    match result.err().unwrap() {
        KernelError::PluginSystem(PluginSystemError::DependencyResolution(dep_err)) => {
           let msg = dep_err.to_string();
           // The DependencyError::IncompatibleVersion format is "Plugin version mismatch: '{plugin_name}' requires version '{required_range}' but found '{actual_version}'"
           assert!(msg.contains("Plugin version mismatch: 'plugin_a' requires version '^2.0' but found '1.0.0'"), "Error message mismatch. Got: {}", msg);
       },
       other_err => panic!("Expected KernelError::PluginSystem(PluginSystemError::DependencyResolution), but got {:?}", other_err),
    }
    assert_eq!(registry.plugin_count(), 0, "Registry should be empty after resolution failure");

    Ok(())
}

#[tokio::test]
async fn test_register_all_plugins_dep_version_parse_err() -> KernelResult<()> {
    let tmp_dir = tempdir()?;
    let plugin_base = tmp_dir.path();

    // Plugin A with invalid version
    let pa_dir = plugin_base.join("plugin_a");
    tokio_fs::create_dir_all(&pa_dir).await?;
    let pa_manifest = create_manifest_json_string("plugin_a", "invalid-version", None, Some(vec!["^1.0"]));
    tokio_fs::write(pa_dir.join("manifest.json"), pa_manifest).await?;

    // Plugin B depends on A
    let pb_dir = plugin_base.join("plugin_b");
    tokio_fs::create_dir_all(&pb_dir).await?;
    let pb_manifest = create_manifest_json_string("plugin_b", "1.0.0", Some(vec![("plugin_a", "*", true)]), Some(vec!["^1.0"]));
    tokio_fs::write(pb_dir.join("manifest.json"), pb_manifest).await?;


    let mut loader = PluginLoader::new();
    loader.add_plugin_dir(plugin_base);
    let _ = loader.scan_for_manifests().await?;

    let api_version = crate::plugin_system::version::ApiVersion::new(1, 0, 0);
    let mut registry = crate::plugin_system::registry::PluginRegistry::new(api_version.clone());

    // Registration should fail during dependency resolution (version parse error)
    let result = loader.register_all_plugins(&mut registry, &api_version).await;

    assert!(result.is_err(), "Expected registration to fail due to version parse error");
     match result.err().unwrap() {
        KernelError::PluginSystem(PluginSystemError::DependencyResolution(dep_err)) => {
           let msg = dep_err.to_string();
           let expected_msg = "Dependency error: Failed to parse version for dependency plugin_a: unexpected character 'i' while parsing major version number";
           assert_eq!(msg, expected_msg, "Error message mismatch.");
       },
       other_err => panic!("Expected KernelError::PluginSystem(PluginSystemError::DependencyResolution), but got {:?}", other_err),
    }
    assert_eq!(registry.plugin_count(), 0, "Registry should be empty after resolution failure");

    Ok(())
}


#[tokio::test]
async fn test_register_all_plugins_cycle_detected() -> KernelResult<()> {
    let tmp_dir = tempdir()?;
    let plugin_base = tmp_dir.path();

    // Plugin A depends on B
    let pa_dir = plugin_base.join("plugin_a");
    tokio_fs::create_dir_all(&pa_dir).await?;
    let pa_manifest = create_manifest_json_string("plugin_a", "1.0.0", Some(vec![("plugin_b", "*", true)]), Some(vec!["^1.0"]));
    tokio_fs::write(pa_dir.join("manifest.json"), pa_manifest).await?;

    // Plugin B depends on A (cycle)
    let pb_dir = plugin_base.join("plugin_b");
    tokio_fs::create_dir_all(&pb_dir).await?;
    let pb_manifest = create_manifest_json_string("plugin_b", "1.0.0", Some(vec![("plugin_a", "*", true)]), Some(vec!["^1.0"]));
    tokio_fs::write(pb_dir.join("manifest.json"), pb_manifest).await?;

    let mut loader = PluginLoader::new();
    loader.add_plugin_dir(plugin_base);
    let _ = loader.scan_for_manifests().await?;

    let api_version = crate::plugin_system::version::ApiVersion::new(1, 0, 0);
    let mut registry = crate::plugin_system::registry::PluginRegistry::new(api_version.clone());

    // Registration should fail during dependency resolution (cycle detected)
    let result = loader.register_all_plugins(&mut registry, &api_version).await;

    assert!(result.is_err(), "Expected registration to fail due to cycle detection");
     match result.err().unwrap() {
        KernelError::PluginSystem(PluginSystemError::DependencyResolution(dep_err)) => {
           let msg = dep_err.to_string();
           // Updated Display format is "Circular dependency detected: A -> B -> ..."
           assert!(msg.starts_with("Circular dependency detected:"), "Error message mismatch. Got: {}", msg);
           // Check if cycle path is mentioned (specific path might vary based on HashMap iteration order)
           assert!(msg.contains("plugin_a") && msg.contains("plugin_b"), "Error message should mention cycle participants. Got: {}", msg);
        },
        other_err => panic!("Expected KernelError::PluginSystem(PluginSystemError::DependencyResolution), but got {:?}", other_err),
    }
    assert_eq!(registry.plugin_count(), 0, "Registry should be empty after resolution failure");

    Ok(())
}