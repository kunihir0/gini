// crates/gini-core/src/plugin_system/tests/manifest_tests.rs
#![cfg(test)]

use crate::plugin_system::{
    manifest::{DependencyInfo, ManifestBuilder, PluginManifest},
    traits::PluginPriority,
    version::VersionRange,
};
use std::str::FromStr;

#[test]
fn test_manifest_new_defaults() {
    // PluginManifest::new requires id, name, version, description, author
    let manifest = PluginManifest::new(
        "test_plugin",
        "Test Plugin",
        "0.1.0",
        "Default Description",
        "Default Author",
    );
    assert_eq!(manifest.id, "test_plugin");
    assert_eq!(manifest.name, "Test Plugin");
    assert_eq!(manifest.version, "0.1.0");
    assert_eq!(manifest.entry_point, "libtest_plugin.so"); // Default entry point format
    assert_eq!(manifest.description, "Default Description"); // Now String, not Option<String>
    assert_eq!(manifest.author, "Default Author"); // Now String, not Option<String>
    assert!(manifest.dependencies.is_empty());
    assert!(manifest.api_versions.is_empty());
    assert!(manifest.tags.is_empty());
    assert!(manifest.priority.is_none());
    assert!(!manifest.is_core); // Replaced core_component
}

#[test]
fn test_manifest_builder_methods() {
    let version_range = VersionRange::from_str("^1.0").unwrap();
    let api_version_range = VersionRange::from_str("~1").unwrap();
    let priority = PluginPriority::Core(50);

    // ManifestBuilder::new requires id, name, version
    let manifest = ManifestBuilder::new("builder_test", "Builder Test Plugin", "1.2.3")
        .entry_point("my_plugin.dll") // Use builder method
        .description("A test plugin built with the builder.") // Use builder method
        .author("Test Author") // Use builder method
        .dependency("core", Some(version_range.clone()), true) // Use builder method
        .api_version(api_version_range.clone()) // Use builder method
        .tag("test") // Use builder method
        .priority(priority.clone()) // Clone priority to avoid move
        .core(true) // Use builder method
        .build();

    assert_eq!(manifest.id, "builder_test");
    assert_eq!(manifest.name, "Builder Test Plugin");
    assert_eq!(manifest.version, "1.2.3");
    assert_eq!(manifest.entry_point, "my_plugin.dll");
    assert_eq!(manifest.description, "A test plugin built with the builder."); // String
    assert_eq!(manifest.author, "Test Author"); // String
    assert_eq!(manifest.dependencies.len(), 1);
    assert_eq!(manifest.dependencies[0].id, "core"); // Check id field
    // Compare Option<VersionRange> via string representation
    assert_eq!(manifest.dependencies[0].version_range.as_ref().map(|vr| vr.to_string()), Some(version_range.to_string()));
    assert!(manifest.dependencies[0].required);
    assert_eq!(manifest.api_versions.len(), 1);
    // Check if the Vec contains the expected VersionRange by comparing string representations
    let api_version_present = manifest.api_versions.iter().any(|vr| vr.to_string() == api_version_range.to_string());
    assert!(api_version_present, "Expected API version range {:?} not found", api_version_range.to_string());
    assert_eq!(manifest.tags, vec!["test".to_string()]);
    assert_eq!(manifest.priority, Some(priority.to_string()));
    assert!(manifest.is_core); // Check is_core field
}

#[test]
fn test_manifest_get_priority() {
    // Need to provide all required args for PluginManifest::new
    let mut manifest = PluginManifest::new("priority_test", "P Test", "1.0", "Desc", "Auth");

    // None - default state
    assert!(manifest.get_priority().is_none());

    // Valid
    manifest.priority = Some("core_critical:30".to_string());
    assert_eq!(manifest.get_priority(), Some(PluginPriority::CoreCritical(30)));

    // Invalid format
    manifest.priority = Some("invalid-priority".to_string());
    assert!(manifest.get_priority().is_none());

    // Invalid type
    manifest.priority = Some("unknown:50".to_string());
    assert!(manifest.get_priority().is_none());

    // Invalid value (out of range)
    manifest.priority = Some("core:260".to_string());
    assert!(manifest.get_priority().is_none());
}

#[test]
fn test_manifest_builder_chaining() {
    // ManifestBuilder::new requires id, name, version
    let manifest = ManifestBuilder::new("chain_test", "Chain Test", "0.5.0")
        .description("Chained description.") // Use builder method
        .author("Chained Author") // Use builder method
        .tag("chained") // Use builder method
        // 150 falls into ThirdPartyHigh range (101-150) according to from_str logic
        .priority(PluginPriority::ThirdPartyHigh(150))
        .build();

    assert_eq!(manifest.id, "chain_test");
    assert_eq!(manifest.name, "Chain Test"); // Set in new()
    assert_eq!(manifest.version, "0.5.0"); // Set in new()
    assert_eq!(manifest.description, "Chained description."); // String
    assert_eq!(manifest.author, "Chained Author"); // String
    assert_eq!(manifest.tags, vec!["chained".to_string()]);
    // The builder stores the string representation of the enum passed.
    // Since we passed ThirdPartyHigh(150), the stored string should be "third_party_high:150"
    assert_eq!(manifest.priority, Some("third_party_high:150".to_string()));
    // get_priority() should now correctly parse the stored string back to the enum variant
    assert_eq!(manifest.get_priority(), Some(PluginPriority::ThirdPartyHigh(150)));
}

#[test]
fn test_manifest_builder_defaults() {
    // ManifestBuilder::new requires id, name, version and sets default desc/author
    let manifest = ManifestBuilder::new("default_builder_test", "Default Builder", "1.0.0")
        // No description or author methods called, should use defaults from new()
        .build();

    assert_eq!(manifest.id, "default_builder_test");
    assert_eq!(manifest.name, "Default Builder");
    assert_eq!(manifest.version, "1.0.0");
    // Check the default values set by ManifestBuilder::new
    assert_eq!(manifest.description, "A plugin for OSX-Forge");
    assert_eq!(manifest.author, "Unknown");
}

#[test]
fn test_dependency_info_creation() {
    let version_range = VersionRange::from_str(">=1.0, <2.0").unwrap();
    // Use id field
    let dep_info = DependencyInfo {
        id: "another_plugin".to_string(),
        version_range: Some(version_range.clone()),
        required: false,
    };

    assert_eq!(dep_info.id, "another_plugin");
    // VersionRange doesn't implement PartialEq, compare Option<String> representations
    assert_eq!(dep_info.version_range.map(|vr| vr.to_string()), Some(version_range.to_string()));
    assert!(!dep_info.required);
}

#[test]
fn test_manifest_add_multiple_items() {
    let vr1 = VersionRange::from_str("*").unwrap();
    let vr_api1 = VersionRange::from_str("1.0").unwrap();
    let vr_api2 = VersionRange::from_str("~1.1").unwrap();

    // Use builder methods
    let manifest = ManifestBuilder::new("multi_item_test", "Multi Test", "1.0")
        .dependency("dep1", None, true) // Required, no version range
        .dependency("dep2", Some(vr1.clone()), false) // Optional, any version
        .api_version(vr_api1.clone())
        .api_version(vr_api2.clone())
        .tag("tag1")
        .tag("tag2")
        .tag("tag1") // Test duplicate tag add
        .build();

    assert_eq!(manifest.dependencies.len(), 2);
    // Check dependency details using find or iteration
    let dep1 = manifest.dependencies.iter().find(|d| d.id == "dep1").unwrap();
    assert!(dep1.required);
    assert!(dep1.version_range.is_none());

    let dep2 = manifest.dependencies.iter().find(|d| d.id == "dep2").unwrap();
    assert!(!dep2.required);
    assert_eq!(dep2.version_range.as_ref().map(|v| v.to_string()), Some(vr1.to_string()));


    assert_eq!(manifest.api_versions.len(), 2);
    // Check for presence by comparing string representations
    let api1_present = manifest.api_versions.iter().any(|vr| vr.to_string() == vr_api1.to_string());
    let api2_present = manifest.api_versions.iter().any(|vr| vr.to_string() == vr_api2.to_string());
    assert!(api1_present, "Expected API version range {:?} not found", vr_api1.to_string());
    assert!(api2_present, "Expected API version range {:?} not found", vr_api2.to_string());

    // Tags are added to a Vec, duplicates are allowed by default Vec::push
    assert_eq!(manifest.tags.len(), 3);
    assert_eq!(manifest.tags.iter().filter(|&t| t == "tag1").count(), 2);
    assert!(manifest.tags.contains(&"tag2".to_string()));
}


#[test]
fn test_manifest_priority_parsing_edge() {
    // Need to provide all required args for PluginManifest::new
    let mut manifest = PluginManifest::new("priority_edge_test", "Edge Test", "1.0", "Desc", "Auth");

    // Edge case strings for PluginPriority::from_str
    manifest.priority = Some("kernel:0".to_string()); // Min kernel
    assert_eq!(manifest.get_priority(), Some(PluginPriority::Kernel(0)));

    manifest.priority = Some("kernel:49".to_string()); // Value 49 is outside the valid range (0-10) for Kernel
    assert!(manifest.get_priority().is_none(), "Parsing 'kernel:49' should fail (value > 10)"); // Expect None

    manifest.priority = Some("core_critical:50".to_string()); // Min core_critical is 50 (range 11-50)
    assert_eq!(manifest.get_priority(), Some(PluginPriority::CoreCritical(50))); // This should pass

    manifest.priority = Some("core:99".to_string()); // Max core
    assert_eq!(manifest.get_priority(), Some(PluginPriority::Core(99))); // 99 is within 51-100

    manifest.priority = Some("third_party_high:101".to_string()); // Min third_party_high is 101
    assert_eq!(manifest.get_priority(), Some(PluginPriority::ThirdPartyHigh(101)));

    manifest.priority = Some("third_party_low:255".to_string()); // Max third_party_low
    assert_eq!(manifest.get_priority(), Some(PluginPriority::ThirdPartyLow(255)));

    // Out of bounds
    manifest.priority = Some("kernel:50".to_string());
    assert!(manifest.get_priority().is_none()); // kernel:50 is invalid ( > 10 )

    manifest.priority = Some("third_party_low:256".to_string()); // 256 is > 255
    assert!(manifest.get_priority().is_none());

    // Add check for kernel:49 explicitly, expecting None
    manifest.priority = Some("kernel:49".to_string());
    assert!(manifest.get_priority().is_none(), "kernel:49 should be invalid");

    // Malformed
    manifest.priority = Some("core: 70".to_string()); // Space
    assert!(manifest.get_priority().is_none());
    manifest.priority = Some(":70".to_string()); // Missing type
    assert!(manifest.get_priority().is_none());
    manifest.priority = Some("core:".to_string()); // Missing value
    assert!(manifest.get_priority().is_none());
     manifest.priority = Some("core".to_string()); // Missing colon and value
    assert!(manifest.get_priority().is_none());
}