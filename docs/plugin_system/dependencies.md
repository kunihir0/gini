# Plugin Dependencies Design

This document outlines the design for handling inter-plugin dependencies within the Gini kernel.

## 1. Declaration

Plugin dependencies are declared within the `PluginManifest` struct located in `src/plugin_system/manifest.rs`. The existing `dependencies` field, which is a `Vec<DependencyInfo>`, will continue to be used.

```rust
// src/plugin_system/manifest.rs (Relevant Structures)

/// Represents a dependency on another plugin
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    /// Plugin ID
    pub id: String,

    /// Required version range (optional)
    /// Uses the VersionRange struct for semantic version constraints.
    pub version_range: Option<VersionRange>,

    /// Whether this dependency is required for the plugin to function.
    /// If true, the plugin will fail to load if the dependency is not met.
    /// If false, the dependency is optional, and the plugin might adapt its functionality.
    pub required: bool,
}

/// Represents a plugin manifest that describes a plugin
#[derive(Debug, Clone)]
pub struct PluginManifest {
    // ... other fields ...

    /// Plugin dependencies
    pub dependencies: Vec<DependencyInfo>,

    // ... other fields ...
}
```

This existing structure is sufficient for declaring dependencies, including optional dependencies and specific version constraints using semantic versioning. No changes to the manifest structure itself are required for this phase.

## 2. Resolution Process

The responsibility for resolving these declared dependencies lies within the plugin loading mechanism, likely handled by the `PluginLoader` or during the `PluginManager`'s initialization phase.

The resolution process must occur *after* all available plugin manifests have been discovered and parsed, but *before* the actual `Plugin` objects are initialized (i.e., before their `init` method is called).

The process involves the following steps:

1.  **Build Dependency Graph:** Construct a directed graph where nodes represent plugins and edges represent dependencies.
2.  **Check for Missing Dependencies:** Iterate through all plugins and verify that all `required = true` dependencies exist in the set of discovered plugins.
3.  **Verify Version Constraints:** For each dependency with a `version_range` specified, check if the version of the discovered dependency plugin satisfies the range requirement.
4.  **Detect Circular Dependencies:** Analyze the graph to identify any circular dependencies, which would prevent a valid load order.
5.  **Determine Load Order:** Perform a topological sort on the dependency graph to determine a valid initialization order for the plugins (if required, though initialization might happen concurrently after checks pass).

## 3. Error Handling

*   If any required dependency is missing, the depending plugin(s) should fail to load.
*   If a version constraint is not met for a required dependency, the depending plugin(s) should fail to load.
*   If a circular dependency is detected involving required dependencies, all plugins within the cycle should fail to load.
*   Clear and informative error messages must be logged or reported, indicating which plugin failed, the specific dependency issue (missing, version mismatch, cycle), and the required dependency details.
*   Optional dependencies (`required = false`) that are missing or have version mismatches should likely log a warning but allow the depending plugin to load, assuming the plugin is designed to handle the absence of the optional dependency.

This resolution logic ensures that plugins are only initialized if their prerequisites are met, contributing to a more robust and predictable system.