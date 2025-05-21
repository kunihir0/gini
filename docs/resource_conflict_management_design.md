# Resource Conflict Management Design for Gini-Core Plugins

## 1. Introduction

This document outlines the design for a system within `gini-core` that allows plugins to declare the resources they use and for the `PluginRegistry` to detect and report conflicts if multiple active plugins attempt to claim exclusive or incompatible access to the same resource. This addresses the `TODO` in `crates/gini-core/src/plugin_system/registry.rs` regarding resource conflict checks.

## 2. Defining "Resource"

### 2.1. Resource Types

The system should be flexible enough to handle various resource types. Examples include:

*   **File Paths**: Absolute paths to files or directories (e.g., lock files, managed configuration files).
*   **Network Ports**: Specific TCP/UDP ports (e.g., "tcp:8080").
*   **Hardware Device IDs**: Unique identifiers for hardware (e.g., "/dev/ttyS0").
*   **Named Synchronization Primitives**: System-wide mutexes, semaphores, or shared memory regions identified by unique names.
*   **Unique Configuration Sections**: Specific keys or sections in shared configuration files managed by a plugin.
*   **Abstract Resources**: Logical resources identified by name (e.g., "PrimaryDatabaseConnection", "UniqueUIServiceEndpoint").

### 2.2. `ResourceIdentifier` Struct

To uniquely identify a resource, the following struct will be used:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceIdentifier {
    /// The kind of resource (e.g., "FilePath", "NetworkPort", "NamedMutex", "Abstract").
    pub kind: String,
    /// The unique ID of the resource within its kind (e.g., "/var/log/app.log", "tcp:8080", "my_app_global_lock").
    pub id: String,
}
```

### 2.3. `ResourceAccessType` Enum

This enum defines how a plugin intends to use a resource:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceAccessType {
    /// Exclusive read access. No other plugin can read or write this resource.
    ExclusiveRead,
    /// Exclusive write access. No other plugin can read or write this resource. Implies read capability.
    ExclusiveWrite,
    /// Shared read access. Multiple plugins can read this resource. No writes allowed by shared readers.
    SharedRead,
    /// Shared write access. Multiple plugins can write to this resource. Implies read capability.
    /// (Note: This requires careful management by plugins or the resource itself to avoid races).
    SharedWrite,
    /// Indicates the plugin defines or provides this resource uniquely.
    /// Examples: a plugin defining a specific stage ID, or a plugin that creates and manages a unique named pipe.
    /// Conflicts if another plugin also tries to provide the same ID or claims exclusive access.
    ProvidesUniqueId,
}
```

## 3. Plugin Resource Declaration

### 3.1. New `Plugin` Trait Method

A new method will be added to the `Plugin` trait in `crates/gini-core/src/plugin_system/traits.rs`:

```rust
// In crates/gini-core/src/plugin_system/traits.rs
pub trait Plugin: Send + Sync {
    // ... existing methods ...

    /// Declares the resources this plugin intends to use or provide.
    /// The plugin registry will use this information to detect potential conflicts
    /// between active plugins.
    ///
    /// The default implementation returns an empty vector, indicating that
    /// the plugin does not declare any specific resources.
    fn declared_resources(&self) -> Vec<ResourceClaim> {
        Vec::new()
    }
}
```

### 3.2. `ResourceClaim` Struct

This struct represents a single resource claim made by a plugin:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceClaim {
    pub resource: ResourceIdentifier,
    pub access_type: ResourceAccessType,
}
```

## 4. Conflict Detection Logic in `PluginRegistry`

The `detect_all_conflicts` method in `crates/gini-core/src/plugin_system/registry.rs` will be enhanced:

1.  Iterate through all pairs of *enabled* plugins.
2.  For each plugin, call `declared_resources()` to get its `Vec<ResourceClaim>`.
3.  Compare every `ResourceClaim` from the first plugin against every `ResourceClaim` from the second.
4.  Two `ResourceClaim`s, `claim_a` and `claim_b`, conflict if:
    *   `claim_a.resource == claim_b.resource`.
    *   AND their `access_type`s are incompatible (see conflict matrix below).
5.  If a conflict is detected, a `PluginConflict` instance is created and added to the `ConflictManager`.

### 4.1. Conflict Matrix (Initial - See Section 6 for Criticality)

| `claim_a.access_type` | `claim_b.access_type` | Conflict? |
| :-------------------- | :-------------------- | :-------- |
| `ExclusiveWrite`      | *Any*                 | Yes       |
| `ExclusiveRead`       | `ExclusiveWrite`      | Yes       |
| `ExclusiveRead`       | `ExclusiveRead`       | Yes       |
| `ExclusiveRead`       | `SharedWrite`         | Yes       |
| `ExclusiveRead`       | `SharedRead`          | Yes       |
| `ExclusiveRead`       | `ProvidesUniqueId`    | Yes       |
| `SharedWrite`         | `ExclusiveWrite`      | Yes       |
| `SharedWrite`         | `ExclusiveRead`       | Yes       |
| `SharedWrite`         | `SharedWrite`         | Yes\*     |
| `SharedWrite`         | `SharedRead`          | Yes\*     |
| `SharedWrite`         | `ProvidesUniqueId`    | Yes       |
| `SharedRead`          | `ExclusiveWrite`      | Yes       |
| `SharedRead`          | `ExclusiveRead`       | Yes       |
| `SharedRead`          | `SharedWrite`         | Yes\*     |
| `SharedRead`          | `SharedRead`          | No        |
| `SharedRead`          | `ProvidesUniqueId`    | No        |
| `ProvidesUniqueId`    | `ExclusiveWrite`      | Yes       |
| `ProvidesUniqueId`    | `ExclusiveRead`       | Yes       |
| `ProvidesUniqueId`    | `SharedWrite`         | Yes       |
| `ProvidesUniqueId`    | `SharedRead`          | No        |
| `ProvidesUniqueId`    | `ProvidesUniqueId`    | Yes       |

*(Yes\*): Conflicts involving `SharedWrite` might be treated as warnings or their criticality could depend on the resource `kind`.*

### 4.2. Mermaid Diagram for Conflict Detection Flow

```mermaid
graph TD
    A[PluginRegistry.detect_all_conflicts()] --> B{For each pair of enabled plugins (P1, P2)};
    B --> C1{claims1 = P1.declared_resources()};
    B --> C2{claims2 = P2.declared_resources()};
    C1 --> F{For each claimA in claims1};
    C2 --> F;
    F --> G{For each claimB in claims2};
    G --> H{claimA.resource == claimB.resource?};
    H -- Yes --> I{Are claimA.access_type & claimB.access_type incompatible? (See matrix)};
    I -- Yes --> J[Create PluginConflict with details: P1, P2, resource, access_types];
    J --> K[conflict_manager.add_conflict(new_conflict)];
    I -- No --> G;
    H -- No --> G;
    G -- Next claimB or Done --> F;
    F -- Next claimA or Done --> B;
    B -- Next pair or Done --> L[Return Result];
```

## 5. `PluginConflict` and `ConflictType` Enhancement

The `ConflictType` enum in `crates/gini-core/src/plugin_system/conflict.rs` will be enhanced:

```rust
// In crates/gini-core/src/plugin_system/conflict.rs
pub enum ConflictType {
    // ... other variants like MutuallyExclusive, DependencyVersion ...

    ResourceConflict {
        resource: ResourceIdentifier,
        first_plugin_access: ResourceAccessType,
        second_plugin_access: ResourceAccessType,
    },

    // ... other variants like PartialOverlap, ExplicitlyIncompatible ...
}
```

The `PluginConflict` struct remains:
```rust
pub struct PluginConflict {
    pub first_plugin: String,
    pub second_plugin: String,
    pub conflict_type: ConflictType,
    pub description: String,
    pub resolved: bool,
    pub resolution: Option<ResolutionStrategy>,
}
```
The `description` field will be a human-readable summary derived from `ConflictType::ResourceConflict`.

## 6. Error Handling and Reporting

*   **Logging**: Detailed logs for detected resource conflicts.
*   **User Reporting**: Critical conflicts (see Section 7) will prevent initialization via `PluginSystemError::UnresolvedPluginConflicts`. Non-critical conflicts may be logged as warnings.

## 7. Definition of "Critical" for Resource Conflicts

A resource conflict's "criticality" determines if the system must halt.

### 7.1. Conflict Matrix with Criticality

| `claim_a.access_type` | `claim_b.access_type` | Conflict? | Critical? | Rationale for Criticality                                                                      |
| :-------------------- | :-------------------- | :-------- | :-------- | :--------------------------------------------------------------------------------------------- |
| `ExclusiveWrite`      | *Any*                 | Yes       | **Yes**   | `ExclusiveWrite` inherently cannot coexist.                                                    |
| `ExclusiveRead`       | `ExclusiveWrite`      | Yes       | **Yes**   |                                                                                                |
| `ExclusiveRead`       | `ExclusiveRead`       | Yes       | **Yes**   | Both demand exclusive read.                                                                    |
| `ExclusiveRead`       | `SharedWrite`         | Yes       | **Yes**   | Exclusive read vs. write.                                                                      |
| `ExclusiveRead`       | `SharedRead`          | Yes       | **Yes**   | Exclusivity violated.                                                                          |
| `ExclusiveRead`       | `ProvidesUniqueId`    | Yes       | **Yes**   | Exclusive read vs. provider control.                                                           |
| `SharedWrite`         | `ExclusiveWrite`      | Yes       | **Yes**   |                                                                                                |
| `SharedWrite`         | `ExclusiveRead`       | Yes       | **Yes**   |                                                                                                |
| `SharedWrite`         | `SharedWrite`         | Yes       | No (Warn) | Potential data corruption; warn by default.                                                    |
| `SharedWrite`         | `SharedRead`          | Yes       | No (Warn) | Potential inconsistent views; warn by default.                                                 |
| `SharedWrite`         | `ProvidesUniqueId`    | Yes       | **Yes**   | Writing to a resource another claims to uniquely provide.                                      |
| `SharedRead`          | `ExclusiveWrite`      | Yes       | **Yes**   |                                                                                                |
| `SharedRead`          | `ExclusiveRead`       | Yes       | **Yes**   |                                                                                                |
| `SharedRead`          | `SharedWrite`         | Yes       | No (Warn) |                                                                                                |
| `SharedRead`          | `SharedRead`          | No        | No        | Multiple shared reads are fine.                                                                |
| `SharedRead`          | `ProvidesUniqueId`    | No        | No        | Reading a provided resource is fine.                                                           |
| `ProvidesUniqueId`    | `ExclusiveWrite`      | Yes       | **Yes**   |                                                                                                |
| `ProvidesUniqueId`    | `ExclusiveRead`       | Yes       | **Yes**   |                                                                                                |
| `ProvidesUniqueId`    | `SharedWrite`         | Yes       | **Yes**   |                                                                                                |
| `ProvidesUniqueId`    | `SharedRead`          | No        | No        |                                                                                                |
| `ProvidesUniqueId`    | `ProvidesUniqueId`    | Yes       | **Yes**   | Both claim to define the same unique resource.                                                 |

### 7.2. `ConflictType::is_critical()` Update

```rust
// In crates/gini-core/src/plugin_system/conflict.rs
impl ConflictType {
    pub fn is_critical(&self) -> bool {
        match self {
            ConflictType::MutuallyExclusive => true,
            ConflictType::DependencyVersion { .. } => true,
            ConflictType::ExplicitlyIncompatible => true,
            ConflictType::ResourceConflict { resource: _, first_plugin_access, second_plugin_access } => {
                match (first_plugin_access, second_plugin_access) {
                    (ResourceAccessType::ExclusiveWrite, _) | (_, ResourceAccessType::ExclusiveWrite) => true,
                    (ResourceAccessType::ExclusiveRead, ResourceAccessType::ExclusiveRead) => true,
                    (ResourceAccessType::ExclusiveRead, ResourceAccessType::SharedWrite) | (ResourceAccessType::SharedWrite, ResourceAccessType::ExclusiveRead) => true,
                    (ResourceAccessType::ExclusiveRead, ResourceAccessType::SharedRead) | (ResourceAccessType::SharedRead, ResourceAccessType::ExclusiveRead) => true,
                    (ResourceAccessType::ExclusiveRead, ResourceAccessType::ProvidesUniqueId) | (ResourceAccessType::ProvidesUniqueId, ResourceAccessType::ExclusiveRead) => true,
                    (ResourceAccessType::ProvidesUniqueId, ResourceAccessType::ProvidesUniqueId) => true,
                    (ResourceAccessType::ProvidesUniqueId, ResourceAccessType::SharedWrite) | (ResourceAccessType::SharedWrite, ResourceAccessType::ProvidesUniqueId) => true,
                    _ => false, // Other combinations are conflicts but not critical by default
                }
            }
            ConflictType::PartialOverlap => false,
            ConflictType::Custom(_) => false,
        }
    }
    // ... rest of ConflictType impl ...
}
```

### 7.3. Summary of Critical Resource Conflicts

A resource conflict is **critical** if:
1.  Any plugin claims `ExclusiveWrite`.
2.  Any plugin claims `ExclusiveRead` and another plugin attempts any access that violates exclusivity.
3.  Two or more plugins claim `ProvidesUniqueId` for the same resource.
4.  One plugin claims `ProvidesUniqueId` and another attempts `SharedWrite`, `ExclusiveWrite`, or `ExclusiveRead`.