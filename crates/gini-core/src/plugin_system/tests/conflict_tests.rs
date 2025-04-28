// crates/gini-core/src/plugin_system/tests/conflict_tests.rs
#![cfg(test)]

use crate::plugin_system::conflict::{ConflictManager, ConflictType, PluginConflict, ResolutionStrategy};
// No longer need PluginManifest here directly, but keep HashSet if needed later
// use std::collections::HashSet;

// Helper to create a dummy manifest ID (string) as PluginConflict uses IDs now
fn dummy_id(id: &str) -> String {
    id.to_string()
}

// Updated dummy_manifest to match the new signature (if needed elsewhere, currently not)
// fn dummy_manifest(id: &str) -> PluginManifest {
//     PluginManifest::new(id, id, "0.1.0", "Desc", "Auth")
// }


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
fn test_conflict_manager_detect_conflicts_stub() {
    let mut manager = ConflictManager::new();
    // Pass a slice of String IDs
    let plugin_ids = vec![dummy_id("p1"), dummy_id("p2")];
    let result = manager.detect_conflicts(&plugin_ids); // Pass only the required argument
    assert!(result.is_ok());
    // Ensure no conflicts were added by the stub
    assert!(manager.get_conflicts().is_empty());
}