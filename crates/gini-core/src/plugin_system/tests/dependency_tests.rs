// crates/gini-core/src/plugin_system/tests/dependency_tests.rs
#![cfg(test)]

use crate::plugin_system::{
    dependency::{PluginDependency, DependencyError},
    version::VersionRange,
};
use std::str::FromStr;

#[test]
fn test_dependency_constructors() {
    let vr = VersionRange::from_str("^1.0").unwrap();

    // Required with version
    let dep_req = PluginDependency::required("core", vr.clone());
    assert_eq!(dep_req.plugin_name, "core");
    assert_eq!(dep_req.version_range.as_ref().map(|v| v.to_string()), Some(vr.to_string()));
    assert!(dep_req.required);

    // Required any version
    let dep_req_any = PluginDependency::required_any("utils");
    assert_eq!(dep_req_any.plugin_name, "utils");
    assert!(dep_req_any.version_range.is_none());
    assert!(dep_req_any.required);

    // Optional with version
    let dep_opt = PluginDependency::optional("logger", vr.clone());
    assert_eq!(dep_opt.plugin_name, "logger");
    assert_eq!(dep_opt.version_range.as_ref().map(|v| v.to_string()), Some(vr.to_string()));
    assert!(!dep_opt.required);

    // Optional any version
    let dep_opt_any = PluginDependency::optional_any("ui");
    assert_eq!(dep_opt_any.plugin_name, "ui");
    assert!(dep_opt_any.version_range.is_none());
    assert!(!dep_opt_any.required);
}

#[test]
fn test_dependency_is_compatible_no_range() {
    let dep = PluginDependency::required_any("any_version_plugin");
    assert!(dep.is_compatible_with("1.0.0"));
    assert!(dep.is_compatible_with("0.1.0-alpha"));
    assert!(dep.is_compatible_with("invalid-version")); // Should still return true as no range is specified
}

#[test]
fn test_dependency_is_compatible_with_range() {
    let vr = VersionRange::from_str(">=1.0.0, <2.0.0").unwrap();
    let dep = PluginDependency::required("ranged_plugin", vr);

    // Compatible versions
    assert!(dep.is_compatible_with("1.0.0"));
    assert!(dep.is_compatible_with("1.5.0"));
    assert!(dep.is_compatible_with("1.9.9"));

    // Incompatible versions
    assert!(!dep.is_compatible_with("0.9.9"));
    assert!(!dep.is_compatible_with("2.0.0"));
    assert!(!dep.is_compatible_with("2.1.0"));
}

#[test]
fn test_dependency_is_compatible_invalid_version() {
    let vr = VersionRange::from_str("^1.0").unwrap();
    let dep = PluginDependency::required("invalid_version_test", vr);

    // is_compatible_with should return false for unparsable versions
    assert!(!dep.is_compatible_with("abc"));
    assert!(!dep.is_compatible_with("1.0-beta+extra")); // Example of potentially complex but invalid semver
    assert!(!dep.is_compatible_with(""));

    // Check a valid version still works
    assert!(dep.is_compatible_with("1.2.3"));
}

#[test]
fn test_dependency_display_format() {
    let vr = VersionRange::from_str("~1.2").unwrap(); // Constraint string is "~1.2"

    let dep_req = PluginDependency::required("display_req", vr.clone());
    assert_eq!(
        format!("{}", dep_req),
        "Requires plugin: display_req (version: ~1.2)"
    );

    let dep_req_any = PluginDependency::required_any("display_req_any");
    assert_eq!(
        format!("{}", dep_req_any),
        "Requires plugin: display_req_any (any version)"
    );

    let dep_opt = PluginDependency::optional("display_opt", vr);
    assert_eq!(
        format!("{}", dep_opt),
        "Optional plugin: display_opt (version: ~1.2)"
    );

    let dep_opt_any = PluginDependency::optional_any("display_opt_any");
     assert_eq!(
        format!("{}", dep_opt_any),
        "Optional plugin: display_opt_any (any version)"
    );
}

#[test]
fn test_dependency_error_display_format() {
    let missing_err = DependencyError::MissingPlugin("missing_core".to_string());
    assert_eq!(format!("{}", missing_err), "Required plugin not found: missing_core");

    let vr = VersionRange::from_str("^2.0").unwrap(); // Constraint string is "^2.0"
    let incompatible_err = DependencyError::IncompatibleVersion {
        plugin_name: "my_plugin".to_string(),
        required_range: vr,
        actual_version: "1.5.0".to_string(),
    };
    assert_eq!(
        format!("{}", incompatible_err),
        "Plugin version mismatch: 'my_plugin' requires version '^2.0' but found '1.5.0'"
    );

    let cycle_err = DependencyError::CyclicDependency(vec!["A".to_string(), "B".to_string(), "A".to_string()]);
    assert_eq!(format!("{}", cycle_err), "Circular dependency detected: A -> B -> A");

    let other_err = DependencyError::Other("Something else went wrong".to_string());
    assert_eq!(format!("{}", other_err), "Dependency error: Something else went wrong");
}