// crates/gini-core/src/plugin_system/tests/conflict_tests.rs
#![cfg(test)]

use crate::plugin_system::conflict::{ConflictManager, ConflictType, PluginConflict, ResolutionStrategy};
use crate::plugin_system::{Plugin, PluginDependency, ApiVersion, VersionRange, PluginPriority, PluginRegistry};
use crate::plugin_system::error::PluginSystemError; // Import PluginSystemError
use crate::stage_manager::StageRequirement; // Removed unused Stage
use crate::stage_manager::registry::StageRegistry; // Added for register_stages
use crate::stage_manager::context::StageContext; // Added for preflight_check
// use crate::kernel::error::Result as KernelResult; // Removed unused import
use crate::kernel::bootstrap::Application; // Needed for Plugin trait
use std::str::FromStr; // Needed for VersionRange::from_str
use std::sync::Arc; // Added for Arc::new
use crate::plugin_system::manifest::{ManifestBuilder, ResourceAccessType}; // Added for new tests, removed PluginManifest


#[test]
fn test_conflict_type_is_critical() {
    // Updated variants based on conflict.rs
    assert!(ConflictType::MutuallyExclusive.is_critical());
    assert!(ConflictType::DependencyVersion.is_critical());
    assert!(ConflictType::ResourceConflict.is_critical());
    assert!(ConflictType::ExplicitlyIncompatible.is_critical());
    assert!(!ConflictType::PartialOverlap.is_critical());
    assert!(!ConflictType::Custom("test".to_string()).is_critical());
}

#[test]
fn test_conflict_type_description() {
    // Updated variants based on conflict.rs
    assert_eq!(ConflictType::MutuallyExclusive.description(), "Mutually exclusive plugins");
    assert_eq!(ConflictType::DependencyVersion.description(), "Conflicting dependency versions");
    assert_eq!(ConflictType::ResourceConflict.description(), "Resource conflict");
    assert_eq!(ConflictType::PartialOverlap.description(), "Partial functionality overlap");
    assert_eq!(ConflictType::ExplicitlyIncompatible.description(), "Explicitly marked as incompatible");
    assert_eq!(ConflictType::Custom("custom".to_string()).description(), "Custom conflict");
}

#[test]
fn test_plugin_conflict_new() {
    // PluginConflict::new takes &str IDs and description now
    let conflict = PluginConflict::new(
        "plugin_a",
        "plugin_b",
        ConflictType::DependencyVersion, // Use a current variant
        "Requires dep v1 vs v2",
    );

    assert_eq!(conflict.first_plugin, "plugin_a");
    assert_eq!(conflict.second_plugin, "plugin_b");
    assert_eq!(conflict.conflict_type, ConflictType::DependencyVersion);
    assert_eq!(conflict.description, "Requires dep v1 vs v2");
    assert!(!conflict.resolved);
    assert!(conflict.resolution.is_none()); // Resolution is Option<ResolutionStrategy>
}

#[test]
fn test_plugin_conflict_resolve() {
    let mut conflict = PluginConflict::new(
        "plugin_a",
        "plugin_b",
        ConflictType::PartialOverlap, // Use a non-critical variant for this test
        "Overlap",
    );

    assert!(!conflict.resolved);
    assert!(conflict.resolution.is_none());

    // Use a current ResolutionStrategy variant
    conflict.resolve(ResolutionStrategy::AllowWithWarning);

    assert!(conflict.resolved);
    assert_eq!(conflict.resolution, Some(ResolutionStrategy::AllowWithWarning));
}

#[test]
fn test_plugin_conflict_is_critical() {
    let critical_conflict = PluginConflict::new(
        "plugin_a",
        "plugin_b",
        ConflictType::MutuallyExclusive, // Use a critical variant
        "Critical",
    );
    let non_critical_conflict = PluginConflict::new(
        "plugin_c",
        "plugin_d",
        ConflictType::PartialOverlap, // Use a non-critical variant
        "Non-critical",
    );

    assert!(critical_conflict.is_critical());
    assert!(!non_critical_conflict.is_critical());
}

#[test]
fn test_conflict_manager_new_default() {
    let manager_new = ConflictManager::new();
    let manager_default = ConflictManager::default();

    assert!(manager_new.get_conflicts().is_empty());
    assert!(manager_default.get_conflicts().is_empty());
}

#[test]
fn test_conflict_manager_add_conflict() {
    let mut manager = ConflictManager::new();
    let conflict1 = PluginConflict::new("plugin_a", "plugin_b", ConflictType::MutuallyExclusive, "Desc1");
    let conflict2 = PluginConflict::new("plugin_b", "plugin_c", ConflictType::PartialOverlap, "Desc2");

    manager.add_conflict(conflict1.clone());
    manager.add_conflict(conflict2.clone());

    let conflicts = manager.get_conflicts();
    assert_eq!(conflicts.len(), 2);
    // Check for presence by comparing relevant fields, as PluginConflict itself might not be PartialEq
    assert!(conflicts.iter().any(|c| c.first_plugin == "plugin_a" && c.second_plugin == "plugin_b"));
    assert!(conflicts.iter().any(|c| c.first_plugin == "plugin_b" && c.second_plugin == "plugin_c"));
}

#[test]
fn test_conflict_manager_get_unresolved() {
    let mut manager = ConflictManager::new();
    let mut conflict1 = PluginConflict::new("plugin_a", "plugin_b", ConflictType::MutuallyExclusive, "Desc1");
    let conflict2 = PluginConflict::new("plugin_b", "plugin_c", ConflictType::PartialOverlap, "Desc2");

    conflict1.resolve(ResolutionStrategy::DisableFirst); // Use current variant
    manager.add_conflict(conflict1.clone());
    manager.add_conflict(conflict2.clone());

    let unresolved = manager.get_unresolved_conflicts();
    assert_eq!(unresolved.len(), 1);
    // Compare by IDs and type, as the whole struct isn't PartialEq
    assert_eq!(unresolved[0].first_plugin, conflict2.first_plugin);
    assert_eq!(unresolved[0].second_plugin, conflict2.second_plugin);
    assert_eq!(unresolved[0].conflict_type, conflict2.conflict_type); // ConflictType derives PartialEq
}

#[test]
fn test_conflict_manager_get_critical_unresolved() {
     let mut manager = ConflictManager::new();
     let mut critical_resolved = PluginConflict::new("plugin_a", "plugin_b", ConflictType::DependencyVersion, "Desc1");
     let critical_unresolved = PluginConflict::new("plugin_a", "plugin_c", ConflictType::MutuallyExclusive, "Desc2");
     let mut non_critical_resolved = PluginConflict::new("plugin_b", "plugin_c", ConflictType::PartialOverlap, "Desc3");
     let non_critical_unresolved = PluginConflict::new("plugin_c", "plugin_d", ConflictType::Custom("custom".into()), "Desc4");

     critical_resolved.resolve(ResolutionStrategy::DisableSecond);
     non_critical_resolved.resolve(ResolutionStrategy::AllowWithWarning); // Use current variant

     manager.add_conflict(critical_resolved);
     manager.add_conflict(critical_unresolved.clone());
     manager.add_conflict(non_critical_resolved);
     manager.add_conflict(non_critical_unresolved);

     let critical_unresolved_list = manager.get_critical_unresolved_conflicts();
     assert_eq!(critical_unresolved_list.len(), 1);
     // Compare by IDs and type
     assert_eq!(critical_unresolved_list[0].first_plugin, critical_unresolved.first_plugin);
     assert_eq!(critical_unresolved_list[0].second_plugin, critical_unresolved.second_plugin);
     assert_eq!(critical_unresolved_list[0].conflict_type, critical_unresolved.conflict_type); // ConflictType derives PartialEq
}

#[test]
fn test_conflict_manager_resolve_conflict() {
    let mut manager = ConflictManager::new();
    let conflict = PluginConflict::new("plugin_a", "plugin_b", ConflictType::PartialOverlap, "Desc");
    manager.add_conflict(conflict);

    assert!(!manager.get_conflicts()[0].resolved);

    // Valid index
    let result = manager.resolve_conflict(0, ResolutionStrategy::ManualConfiguration); // Use current variant
    assert!(result.is_ok());
    assert!(manager.get_conflicts()[0].resolved);
    assert_eq!(manager.get_conflicts()[0].resolution, Some(ResolutionStrategy::ManualConfiguration));

    // Invalid index
    let result_invalid = manager.resolve_conflict(1, ResolutionStrategy::Merge); // Use current variant
    assert!(result_invalid.is_err());
    // Check the error type if possible, or the message format
    assert!(result_invalid.unwrap_err().to_string().contains("Conflict index out of bounds"));
}

#[test]
fn test_conflict_manager_all_critical_resolved() {
    let mut manager = ConflictManager::new();

    // Case 1: No critical conflicts
    manager.add_conflict(PluginConflict::new("p1", "p2", ConflictType::PartialOverlap, "Desc1"));
    assert!(manager.all_critical_conflicts_resolved());

    // Case 2: Unresolved critical conflict
    manager.add_conflict(PluginConflict::new("p1", "p3", ConflictType::MutuallyExclusive, "Desc2"));
    assert!(!manager.all_critical_conflicts_resolved());

    // Case 3: Resolve the critical conflict
    manager.resolve_conflict(1, ResolutionStrategy::DisableFirst).unwrap();
    assert!(manager.all_critical_conflicts_resolved());

     // Case 4: Only resolved critical conflicts
     let mut manager_resolved = ConflictManager::new();
     let mut critical_resolved = PluginConflict::new("p_a", "p_b", ConflictType::DependencyVersion, "Desc3");
     critical_resolved.resolve(ResolutionStrategy::DisableSecond);
     manager_resolved.add_conflict(critical_resolved);
     assert!(manager_resolved.all_critical_conflicts_resolved());
}

#[test]
fn test_conflict_manager_get_plugins_to_disable() {
    let mut manager = ConflictManager::new();

    let mut c1 = PluginConflict::new("plugin_a", "plugin_b", ConflictType::MutuallyExclusive, "Desc1");
    let mut c2 = PluginConflict::new("plugin_a", "plugin_c", ConflictType::DependencyVersion, "Desc2");
    let mut c3 = PluginConflict::new("plugin_b", "plugin_d", ConflictType::ResourceConflict, "Desc3");
    let mut c4 = PluginConflict::new("plugin_c", "plugin_d", ConflictType::PartialOverlap, "Desc4"); // Not disable strategy

    c1.resolve(ResolutionStrategy::DisableFirst); // Disable plugin_a
    c2.resolve(ResolutionStrategy::DisableSecond); // Disable plugin_c
    c3.resolve(ResolutionStrategy::DisableFirst); // Disable plugin_b
    c4.resolve(ResolutionStrategy::AllowWithWarning); // No disable

    manager.add_conflict(c1);
    manager.add_conflict(c2);
    manager.add_conflict(c3);
    manager.add_conflict(c4);

    // Add duplicate disable instruction
    let mut c5 = PluginConflict::new("plugin_a", "plugin_d", ConflictType::Custom("".into()), "Desc5");
    c5.resolve(ResolutionStrategy::DisableFirst); // Disable plugin_a again
    manager.add_conflict(c5);

    let mut expected: Vec<String> = vec!["plugin_a".to_string(), "plugin_b".to_string(), "plugin_c".to_string()];
    expected.sort(); // Function returns sorted list

    let to_disable = manager.get_plugins_to_disable();
    // The function sorts the result, so we compare directly
    assert_eq!(to_disable, expected);
}


#[test]
fn test_conflict_manager_detect_conflicts_no_claims() { // Renamed from stub
    let mut manager = ConflictManager::new();
    let manifest1 = ManifestBuilder::new("p1", "P1", "1.0.0").build();
    let manifest2 = ManifestBuilder::new("p2", "P2", "1.0.0").build();
    let manifests = vec![manifest1, manifest2];
    let result = manager.detect_conflicts(&manifests);
    assert!(result.is_ok());
    assert!(manager.get_conflicts().is_empty());
}

#[test]
fn test_no_conflicts_no_overlap() {
    let mut manager = ConflictManager::new();
    let manifest1 = ManifestBuilder::new("plugin_A", "Plugin A", "1.0.0")
        .resource("file_path", "/logs/a.log", ResourceAccessType::ExclusiveWrite)
        .build();
    let manifest2 = ManifestBuilder::new("plugin_B", "Plugin B", "1.0.0")
        .resource("file_path", "/logs/b.log", ResourceAccessType::ExclusiveWrite)
        .build();
    let manifests = vec![manifest1, manifest2];
    manager.detect_conflicts(&manifests).unwrap();
    assert!(manager.get_conflicts().is_empty());
}

#[test]
fn test_no_conflicts_shared_read() {
    let mut manager = ConflictManager::new();
    let manifest1 = ManifestBuilder::new("plugin_A", "Plugin A", "1.0.0")
        .resource("file_path", "/config/settings.toml", ResourceAccessType::SharedRead)
        .build();
    let manifest2 = ManifestBuilder::new("plugin_B", "Plugin B", "1.0.0")
        .resource("file_path", "/config/settings.toml", ResourceAccessType::SharedRead)
        .build();
    let manifests = vec![manifest1, manifest2];
    manager.detect_conflicts(&manifests).unwrap();
    assert!(manager.get_conflicts().is_empty());
}

#[test]
fn test_exclusive_write_conflict_on_file_path() {
    let mut manager = ConflictManager::new();
    let manifest1 = ManifestBuilder::new("plugin_A", "Plugin A", "1.0.0")
        .resource("file_path", "/var/log/app.log", ResourceAccessType::ExclusiveWrite)
        .build();
    let manifest2 = ManifestBuilder::new("plugin_B", "Plugin B", "1.0.0")
        .resource("file_path", "/var/log/app.log", ResourceAccessType::ExclusiveWrite)
        .build();
    let manifests = vec![manifest1, manifest2];
    manager.detect_conflicts(&manifests).unwrap();
    let conflicts = manager.get_conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].conflict_type, ConflictType::ResourceConflict);
    assert!(conflicts[0].description.contains("Resource conflict on type 'file_path', identifier '/var/log/app.log'"));
    assert!(conflicts[0].description.contains("Plugin 'plugin_A' claims access 'ExclusiveWrite'"));
    assert!(conflicts[0].description.contains("Plugin 'plugin_B' claims access 'ExclusiveWrite'"));
}

#[test]
fn test_provides_unique_id_conflict_on_stage() {
    let mut manager = ConflictManager::new();
    let manifest1 = ManifestBuilder::new("plugin_C", "Plugin C", "1.0.0")
        .resource("stage_id", "my_custom_stage", ResourceAccessType::ProvidesUniqueId)
        .build();
    let manifest2 = ManifestBuilder::new("plugin_D", "Plugin D", "1.0.0")
        .resource("stage_id", "my_custom_stage", ResourceAccessType::ProvidesUniqueId)
        .build();
    let manifests = vec![manifest1, manifest2];
    manager.detect_conflicts(&manifests).unwrap();
    let conflicts = manager.get_conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].conflict_type, ConflictType::ResourceConflict);
    assert!(conflicts[0].description.contains("Resource conflict on type 'stage_id', identifier 'my_custom_stage'"));
    assert!(conflicts[0].description.contains("Plugin 'plugin_C' claims access 'ProvidesUniqueId'"));
    assert!(conflicts[0].description.contains("Plugin 'plugin_D' claims access 'ProvidesUniqueId'"));
}

#[test]
fn test_mixed_access_conflict_exclusive_vs_provides() {
    let mut manager = ConflictManager::new();
    let manifest1 = ManifestBuilder::new("plugin_E", "Plugin E", "1.0.0")
        .resource("unique_resource", "id_123", ResourceAccessType::ExclusiveWrite)
        .build();
    let manifest2 = ManifestBuilder::new("plugin_F", "Plugin F", "1.0.0")
        .resource("unique_resource", "id_123", ResourceAccessType::ProvidesUniqueId)
        .build();
    let manifests = vec![manifest1, manifest2];
    manager.detect_conflicts(&manifests).unwrap();
    let conflicts = manager.get_conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].conflict_type, ConflictType::ResourceConflict);
    assert!(conflicts[0].description.contains("Plugin 'plugin_E' claims access 'ExclusiveWrite'"));
    assert!(conflicts[0].description.contains("Plugin 'plugin_F' claims access 'ProvidesUniqueId'"));
}

#[test]
fn test_mixed_access_conflict_exclusive_vs_shared_read() {
    let mut manager = ConflictManager::new();
    let manifest1 = ManifestBuilder::new("plugin_G", "Plugin G", "1.0.0")
        .resource("shared_file", "/data/critical.dat", ResourceAccessType::ExclusiveWrite)
        .build();
    let manifest2 = ManifestBuilder::new("plugin_H", "Plugin H", "1.0.0")
        .resource("shared_file", "/data/critical.dat", ResourceAccessType::SharedRead)
        .build();
    let manifests = vec![manifest1, manifest2];
    manager.detect_conflicts(&manifests).unwrap();
    let conflicts = manager.get_conflicts();
    assert_eq!(conflicts.len(), 1, "Expected conflict between ExclusiveWrite and SharedRead");
    assert_eq!(conflicts[0].conflict_type, ConflictType::ResourceConflict);
    assert!(conflicts[0].description.contains("Plugin 'plugin_G' claims access 'ExclusiveWrite'"));
    assert!(conflicts[0].description.contains("Plugin 'plugin_H' claims access 'SharedRead'"));
}

#[test]
fn test_no_conflict_different_identifiers() {
    let mut manager = ConflictManager::new();
    let manifest1 = ManifestBuilder::new("plugin_I", "Plugin I", "1.0.0")
        .resource("file_path", "/var/log/app_i.log", ResourceAccessType::ExclusiveWrite)
        .build();
    let manifest2 = ManifestBuilder::new("plugin_J", "Plugin J", "1.0.0")
        .resource("file_path", "/var/log/app_j.log", ResourceAccessType::ExclusiveWrite)
        .build();
    let manifests = vec![manifest1, manifest2];
    manager.detect_conflicts(&manifests).unwrap();
    assert!(manager.get_conflicts().is_empty());
}

#[test]
fn test_no_conflict_different_resource_types() {
    let mut manager = ConflictManager::new();
    let manifest1 = ManifestBuilder::new("plugin_K", "Plugin K", "1.0.0")
        .resource("file_path", "my_resource", ResourceAccessType::ExclusiveWrite)
        .build();
    let manifest2 = ManifestBuilder::new("plugin_L", "Plugin L", "1.0.0")
        .resource("network_port", "my_resource", ResourceAccessType::ExclusiveWrite)
        .build();
    let manifests = vec![manifest1, manifest2];
    manager.detect_conflicts(&manifests).unwrap();
    assert!(manager.get_conflicts().is_empty());
}

#[test]
fn test_multiple_resources_multiple_conflicts() {
    let mut manager = ConflictManager::new();
    let manifest1 = ManifestBuilder::new("plugin_M", "Plugin M", "1.0.0")
        .resource("file", "/data/file1.txt", ResourceAccessType::ExclusiveWrite)
        .resource("port", "8080", ResourceAccessType::ProvidesUniqueId)
        .resource("id", "common_id", ResourceAccessType::SharedRead) // No conflict here
        .build();
    let manifest2 = ManifestBuilder::new("plugin_N", "Plugin N", "1.0.0")
        .resource("file", "/data/file1.txt", ResourceAccessType::ExclusiveWrite) // Conflict with M on file1
        .resource("port", "9090", ResourceAccessType::ProvidesUniqueId) // No conflict with M on port
        .build();
    let manifest3 = ManifestBuilder::new("plugin_O", "Plugin O", "1.0.0")
        .resource("port", "8080", ResourceAccessType::ProvidesUniqueId) // Conflict with M on port 8080
        .resource("id", "common_id", ResourceAccessType::SharedRead) // No conflict here
        .build();

    let manifests = vec![manifest1.clone(), manifest2.clone(), manifest3.clone()];
    manager.detect_conflicts(&manifests).unwrap();
    let conflicts = manager.get_conflicts();
    assert_eq!(conflicts.len(), 2);

    let has_conflict = |p1_id: &str, p2_id: &str, res_type: &str, res_id: &str| {
        conflicts.iter().any(|c| {
            ((c.first_plugin == p1_id && c.second_plugin == p2_id) || (c.first_plugin == p2_id && c.second_plugin == p1_id)) &&
            c.description.contains(&format!("type '{}', identifier '{}'", res_type, res_id))
        })
    };

    assert!(has_conflict("plugin_M", "plugin_N", "file", "/data/file1.txt"), "Missing conflict between M and N on file /data/file1.txt");
    assert!(has_conflict("plugin_M", "plugin_O", "port", "8080"), "Missing conflict between M and O on port 8080");
}


// --- Tests for PluginRegistry::detect_all_conflicts ---
mod registry_conflict_tests {
    use super::*; // Import items from the parent module

    // Mock Plugin Implementation for testing registry conflict detection
    #[derive(Debug, Clone)] // Added Clone
    struct MockPlugin {
    id: String,
    version: String,
    dependencies: Vec<PluginDependency>,
    api_versions: Vec<VersionRange>, // Added for compatibility check
    priority: PluginPriority, // Added for sorting
    is_core: bool, // Added
    // Fields for explicit conflict/incompatibility declarations
    conflicts_with_ids: Vec<String>,
    incompatible_with_deps: Vec<PluginDependency>, // Use PluginDependency for consistency
}

impl MockPlugin {
    // Helper constructor
    fn new(id: &str, version: &str) -> Self {
        Self {
            id: id.to_string(),
            version: version.to_string(),
            dependencies: vec![],
            // Assume compatible with current API for simplicity in tests
            // Use VersionRange::from_str
            api_versions: vec![<VersionRange as FromStr>::from_str(">=0.1.0").unwrap()],
            priority: PluginPriority::ThirdPartyLow(100),
            is_core: false,
            conflicts_with_ids: vec![], // Initialize new fields
            incompatible_with_deps: vec![], // Initialize new fields
        }
    }

    // Builder-style methods to set conflicts/incompatibilities for tests
    fn conflicts_with(mut self, ids: &[&str]) -> Self {
        self.conflicts_with_ids = ids.iter().map(|s| s.to_string()).collect();
        self
    }

    fn incompatible_with(mut self, deps: Vec<PluginDependency>) -> Self {
        self.incompatible_with_deps = deps;
        self
    }
}

#[async_trait::async_trait] // Add async_trait because preflight_check is async
impl Plugin for MockPlugin {
    // name returns &'static str
    fn name(&self) -> &'static str {
        // Leak the string to get a 'static reference for testing.
        // WARNING: Leaks memory.
        Box::leak(self.id.clone().into_boxed_str())
    }
    // version returns &str (NOT &'static str as per trait)
    fn version(&self) -> &str { &self.version }

    // Return owned Vecs as required by the trait
    fn dependencies(&self) -> Vec<PluginDependency> { self.dependencies.clone() }
    fn compatible_api_versions(&self) -> Vec<VersionRange> { self.api_versions.clone() }

    // Clone priority as it doesn't implement Copy
    fn priority(&self) -> PluginPriority { self.priority.clone() }
    fn is_core(&self) -> bool { self.is_core }

    // Add missing stage methods with dummy implementations
    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }

    // Add missing async preflight_check
    async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), PluginSystemError> {
        Ok(())
    }

    // Dummy implementations for init/shutdown
    fn init(&self, _app: &mut Application) -> std::result::Result<(), PluginSystemError> { Ok(()) }
    fn shutdown(&self) -> std::result::Result<(), PluginSystemError> { Ok(()) }
    fn register_stages(&self, _registry: &mut StageRegistry) -> std::result::Result<(), PluginSystemError> { Ok(()) }

// Implement new trait methods
    fn conflicts_with(&self) -> Vec<String> { self.conflicts_with_ids.clone() }
    fn incompatible_with(&self) -> Vec<PluginDependency> { self.incompatible_with_deps.clone() }
}

// Helper to create a registry with mock plugins
fn create_registry_with_plugins(plugins: Vec<MockPlugin>) -> PluginRegistry {
    // Use a realistic API version for registry creation
    let api_version = ApiVersion::from_str("0.1.0").unwrap();
    let mut registry = PluginRegistry::new(api_version);
    for plugin in plugins {
        let plugin_id = plugin.id.clone(); // Clone ID before moving plugin
        registry.register_plugin(Arc::new(plugin)).unwrap();
        // Ensure all plugins are marked as enabled for conflict detection
        // Use the cloned ID here to avoid borrow checker issue
        registry.enable_plugin(&plugin_id).unwrap();
    }
    registry
}

// --- New Tests for Specific Conflict Logic ---

#[test]
fn test_registry_detect_declared_mutual_exclusion() {
    let plugin_a = MockPlugin::new("plugin-a", "1.0.0").conflicts_with(&["plugin-b"]);
    let plugin_b = MockPlugin::new("plugin-b", "1.0.0"); // Doesn't need to declare back
    let plugin_c = MockPlugin::new("plugin-c", "1.0.0");

    let mut registry = create_registry_with_plugins(vec![plugin_a.clone(), plugin_b.clone(), plugin_c]);

    registry.detect_all_conflicts().unwrap();
    let conflicts = registry.conflict_manager().get_conflicts();

    assert_eq!(conflicts.len(), 1, "Expected exactly one conflict");
    assert_eq!(conflicts[0].first_plugin, plugin_a.id);
    assert_eq!(conflicts[0].second_plugin, plugin_b.id);
    assert_eq!(conflicts[0].conflict_type, ConflictType::MutuallyExclusive);
    assert_eq!(conflicts[0].description, "Plugins are explicitly declared as conflicting.");
}

#[test]
fn test_registry_detect_declared_incompatibility_any_version() {
    let plugin_a = MockPlugin::new("plugin-a", "1.0.0")
        .incompatible_with(vec![PluginDependency::required_any("plugin-b")]); // Incompatible with any version of B
    let plugin_b = MockPlugin::new("plugin-b", "2.5.0");
    let plugin_c = MockPlugin::new("plugin-c", "1.0.0");

    let mut registry = create_registry_with_plugins(vec![plugin_a.clone(), plugin_b.clone(), plugin_c]);

    registry.detect_all_conflicts().unwrap();
    let conflicts = registry.conflict_manager().get_conflicts();

    assert_eq!(conflicts.len(), 1, "Expected exactly one conflict");
    assert_eq!(conflicts[0].first_plugin, plugin_a.id);
    assert_eq!(conflicts[0].second_plugin, plugin_b.id);
    assert_eq!(conflicts[0].conflict_type, ConflictType::ExplicitlyIncompatible);
    assert!(conflicts[0].description.contains("any version"));
}

#[test]
fn test_registry_detect_declared_incompatibility_version_range_match() {
    let range = VersionRange::from_str("<1.0.0").unwrap();
    let plugin_a = MockPlugin::new("plugin-a", "1.0.0")
        .incompatible_with(vec![PluginDependency::required("plugin-b", range.clone())]); // Incompatible with B < 1.0.0
    let plugin_b = MockPlugin::new("plugin-b", "0.9.0"); // Matches incompatibility range
    let plugin_c = MockPlugin::new("plugin-c", "1.0.0");

    let mut registry = create_registry_with_plugins(vec![plugin_a.clone(), plugin_b.clone(), plugin_c]);

    registry.detect_all_conflicts().unwrap();
    let conflicts = registry.conflict_manager().get_conflicts();

    assert_eq!(conflicts.len(), 1, "Expected exactly one conflict");
    assert_eq!(conflicts[0].first_plugin, plugin_a.id);
    assert_eq!(conflicts[0].second_plugin, plugin_b.id);
    assert_eq!(conflicts[0].conflict_type, ConflictType::ExplicitlyIncompatible);
    assert!(conflicts[0].description.contains(&format!("version '{}'", range.constraint_string())));
    assert!(conflicts[0].description.contains(&format!("found version '{}'", plugin_b.version)));
}

#[test]
fn test_registry_detect_declared_incompatibility_version_range_no_match() {
    let range = VersionRange::from_str("<1.0.0").unwrap();
    let plugin_a = MockPlugin::new("plugin-a", "1.0.0")
        .incompatible_with(vec![PluginDependency::required("plugin-b", range)]); // Incompatible with B < 1.0.0
    let plugin_b = MockPlugin::new("plugin-b", "1.1.0"); // Does NOT match incompatibility range
    let plugin_c = MockPlugin::new("plugin-c", "1.0.0");

    let mut registry = create_registry_with_plugins(vec![plugin_a, plugin_b, plugin_c]);

    registry.detect_all_conflicts().unwrap();
    let conflicts = registry.conflict_manager().get_conflicts();

    assert!(conflicts.is_empty(), "Expected no conflicts");
}

#[test]
fn test_registry_detect_declared_incompatibility_bidirectional() {
    // A incompatible with B < 1.0, B incompatible with A >= 2.0
    let range_a_incompat = VersionRange::from_str("<1.0.0").unwrap();
    let range_b_incompat = VersionRange::from_str(">=2.0.0").unwrap();

    let plugin_a = MockPlugin::new("plugin-a", "2.1.0") // Version matches B's incompatibility rule
        .incompatible_with(vec![PluginDependency::required("plugin-b", range_a_incompat.clone())]);
    let plugin_b = MockPlugin::new("plugin-b", "0.8.0") // Version matches A's incompatibility rule
        .incompatible_with(vec![PluginDependency::required("plugin-a", range_b_incompat.clone())]);
    let plugin_c = MockPlugin::new("plugin-c", "1.0.0");

    let mut registry = create_registry_with_plugins(vec![plugin_a.clone(), plugin_b.clone(), plugin_c]);

    registry.detect_all_conflicts().unwrap();
    let conflicts = registry.conflict_manager().get_conflicts();

    // Should detect *both* incompatibilities as separate checks, but only add *one* conflict entry
    assert_eq!(conflicts.len(), 1, "Expected exactly one conflict entry");
    assert_eq!(conflicts[0].first_plugin, plugin_a.id);
    assert_eq!(conflicts[0].second_plugin, plugin_b.id);
    assert_eq!(conflicts[0].conflict_type, ConflictType::ExplicitlyIncompatible);
    // The description might come from either rule, check for key parts
    assert!(conflicts[0].description.contains("explicitly incompatible"));
}

#[test]
fn test_registry_detect_mutual_exclusion_and_incompatibility() {
    let range = VersionRange::from_str(">=1.0.0").unwrap();
    let plugin_a = MockPlugin::new("plugin-a", "1.0.0")
        .conflicts_with(&["plugin-b"]); // A conflicts with B
    let plugin_b = MockPlugin::new("plugin-b", "1.0.0");
    let plugin_c = MockPlugin::new("plugin-c", "1.0.0")
        .incompatible_with(vec![PluginDependency::required("plugin-d", range)]); // C incompatible with D >= 1.0
    let plugin_d = MockPlugin::new("plugin-d", "1.1.0"); // Matches C's incompatibility

    let mut registry = create_registry_with_plugins(vec![
        plugin_a.clone(),
        plugin_b.clone(),
        plugin_c.clone(),
        plugin_d.clone(),
    ]);

    registry.detect_all_conflicts().unwrap();
    let conflicts = registry.conflict_manager().get_conflicts();

    assert_eq!(conflicts.len(), 2, "Expected two conflicts");

    let conflict_exists = |p1: &str, p2: &str, ctype: ConflictType| {
        conflicts.iter().any(|c|
            ((c.first_plugin == p1 && c.second_plugin == p2) || (c.first_plugin == p2 && c.second_plugin == p1))
            && c.conflict_type == ctype
        )
    };

    assert!(conflict_exists(&plugin_a.id, &plugin_b.id, ConflictType::MutuallyExclusive), "Missing A vs B (Mutual)");
    assert!(conflict_exists(&plugin_c.id, &plugin_d.id, ConflictType::ExplicitlyIncompatible), "Missing C vs D (Incompatible)");
}

// --- Updated Tests (Previously relied on placeholder logic) ---

#[test]
fn test_registry_detect_resource_conflict_placeholder() {
    // This test remains valid as it tests the placeholder logic which hasn't been removed yet.
    let plugin_a = MockPlugin::new("main-database-connector", "1.0.0");
    let plugin_b = MockPlugin::new("alt-database-logger", "1.0.0");
    let plugin_c = MockPlugin::new("utility-plugin", "1.0.0"); // Non-conflicting

    let mut registry = create_registry_with_plugins(vec![plugin_a.clone(), plugin_b.clone(), plugin_c]);

    registry.detect_all_conflicts().unwrap();
    let conflicts = registry.conflict_manager().get_conflicts();

    assert_eq!(conflicts.len(), 1);
     // Check that the correct plugins are involved, regardless of order
    assert!(
        (conflicts[0].first_plugin == plugin_a.id && conflicts[0].second_plugin == plugin_b.id) ||
        (conflicts[0].first_plugin == plugin_b.id && conflicts[0].second_plugin == plugin_a.id)
    );
    assert_eq!(conflicts[0].conflict_type, ConflictType::ResourceConflict);
}

#[test]
fn test_registry_detect_no_conflicts_declared() {
    // Renamed from test_registry_detect_no_conflicts to be clearer
    let plugin_a = MockPlugin::new("plugin-a", "1.0.0");
    let plugin_b = MockPlugin::new("plugin-b", "1.0.0");
    let plugin_c = MockPlugin::new("plugin-c", "1.0.0");

    // Ensure no conflicts/incompatibilities are declared
    let mut registry = create_registry_with_plugins(vec![plugin_a, plugin_b, plugin_c]);

    registry.detect_all_conflicts().unwrap();
    let conflicts = registry.conflict_manager().get_conflicts();

    assert!(conflicts.is_empty());
}

// Note: The old `test_registry_detect_mutually_exclusive` and
// `test_registry_detect_explicitly_incompatible` tests relied on placeholder
// name matching. They are superseded by the new tests above that use explicit declarations.
// The old `test_registry_detect_multiple_conflict_types` is also removed as it
// tested combinations of placeholder logic. The new test
// `test_registry_detect_mutual_exclusion_and_incompatibility` covers multiple *real* types.

} // Close mod registry_conflict_tests