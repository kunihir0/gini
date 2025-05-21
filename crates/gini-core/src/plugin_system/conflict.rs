use crate::kernel::error::Result; // Removed unused Error
use crate::plugin_system::manifest::{PluginManifest, ResourceAccessType as ManifestResourceAccessType};
use crate::plugin_system::error::PluginSystemError; // Import PluginSystemError
use std::hash::Hash;

/// Unique identifier for a resource.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceIdentifier {
    /// The kind of resource (e.g., "FilePath", "NetworkPort", "NamedMutex", "Abstract").
    pub kind: String,
    /// The unique ID of the resource within its kind (e.g., "/var/log/app.log", "tcp:8080", "my_app_global_lock").
    pub id: String,
}

/// Defines how a plugin intends to use a resource.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)] // Added Copy
pub enum ResourceAccessType {
    /// Exclusive read access. No other plugin can read or write this resource.
    ExclusiveRead,
    /// Exclusive write access. No other plugin can read or write this resource. Implies read capability.
    ExclusiveWrite,
    /// Shared read access. Multiple plugins can read this resource. No writes allowed by shared readers.
    SharedRead,
    /// Shared write access. Multiple plugins can write to this resource. Implies read capability.
    SharedWrite,
    /// Indicates the plugin defines or provides this resource uniquely.
    ProvidesUniqueId,
}

/// Represents a single resource claim made by a plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceClaim {
    pub resource: ResourceIdentifier,
    pub access_type: ResourceAccessType,
}

/// Types of plugin conflicts
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictType {
    /// Two plugins provide the same functionality and are mutually exclusive
    MutuallyExclusive,
    /// Two plugins have conflicting versions of the same dependency
    DependencyVersion {
        /// The name of the dependency that has conflicting version requirements.
        dependency_name: String,
        /// The version range string required by the first plugin involved in the conflict.
        required_by_first: String,
        /// The version range string required by the second plugin involved in the conflict.
        required_by_second: String,
    },
    /// A plugin requires a resource already claimed by another plugin
    ResourceConflict {
        resource: ResourceIdentifier,
        first_plugin_access: ResourceAccessType,
        second_plugin_access: ResourceAccessType,
    },
    /// Plugin capabilities overlap but may be used together with caution
    PartialOverlap,
    /// A plugin has been marked as incompatible with another
    ExplicitlyIncompatible,
    /// Custom conflict type
    Custom(String),
}

impl ConflictType {
    /// Check if this conflict type is critical (must be resolved)
    pub fn is_critical(&self) -> bool {
        match self {
            ConflictType::MutuallyExclusive => true,
            ConflictType::DependencyVersion { .. } => true,
            ConflictType::ExplicitlyIncompatible => true,
            ConflictType::ResourceConflict { resource: _, first_plugin_access, second_plugin_access } => {
                // Determine criticality based on the access types involved
                match (first_plugin_access, second_plugin_access) {
                    (ResourceAccessType::ExclusiveWrite, _) | (_, ResourceAccessType::ExclusiveWrite) => true,

                    (ResourceAccessType::ExclusiveRead, ResourceAccessType::ExclusiveRead) => true,
                    (ResourceAccessType::ExclusiveRead, ResourceAccessType::SharedWrite) | (ResourceAccessType::SharedWrite, ResourceAccessType::ExclusiveRead) => true,
                    (ResourceAccessType::ExclusiveRead, ResourceAccessType::SharedRead) | (ResourceAccessType::SharedRead, ResourceAccessType::ExclusiveRead) => true,
                    (ResourceAccessType::ExclusiveRead, ResourceAccessType::ProvidesUniqueId) | (ResourceAccessType::ProvidesUniqueId, ResourceAccessType::ExclusiveRead) => true,

                    (ResourceAccessType::ProvidesUniqueId, ResourceAccessType::ProvidesUniqueId) => true,
                    (ResourceAccessType::ProvidesUniqueId, ResourceAccessType::SharedWrite) | (ResourceAccessType::SharedWrite, ResourceAccessType::ProvidesUniqueId) => true,
                    
                    // Other combinations are conflicts but not critical by default (warnings)
                    _ => false,
                }
            }
            ConflictType::PartialOverlap => false,
            ConflictType::Custom(_) => false,
        }
    }
    
    /// Get a human-readable description of this conflict type
    pub fn description(&self) -> &str {
        match self {
            ConflictType::MutuallyExclusive => "Mutually exclusive plugins",
            ConflictType::DependencyVersion { .. } => "Conflicting dependency versions",
            ConflictType::ResourceConflict { .. } => "Resource conflict", // Keep generic for the type itself
            ConflictType::PartialOverlap => "Partial functionality overlap",
            ConflictType::ExplicitlyIncompatible => "Explicitly marked as incompatible",
            ConflictType::Custom(_) => "Custom conflict",
        }
    }
}

/// Represents a conflict between plugins
#[derive(Debug, Clone)]
pub struct PluginConflict {
    /// First plugin ID
    pub first_plugin: String,
    /// Second plugin ID
    pub second_plugin: String,
    /// Type of conflict
    pub conflict_type: ConflictType,
    /// Detailed description of the conflict
    pub description: String,
    /// Whether the conflict has been resolved
    pub resolved: bool,
    /// Resolution strategy, if any
    pub resolution: Option<ResolutionStrategy>,
}

/// Strategies for resolving plugin conflicts
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionStrategy {
    /// Disable the first plugin
    DisableFirst,
    /// Disable the second plugin
    DisableSecond,
    /// Manually configure to avoid conflict
    ManualConfiguration,
    /// Use a compatibility layer
    CompatibilityLayer,
    /// Merge the plugins
    Merge,
    /// Allow both to run with awareness of potential issues
    AllowWithWarning,
    /// Custom resolution strategy
    Custom(String),
}

impl PluginConflict {
    /// Create a new plugin conflict
    pub fn new(
        first_plugin: &str,
        second_plugin: &str,
        conflict_type: ConflictType,
        description: &str,
    ) -> Self {
        Self {
            first_plugin: first_plugin.to_string(),
            second_plugin: second_plugin.to_string(),
            conflict_type,
            description: description.to_string(),
            resolved: false,
            resolution: None,
        }
    }
    
    /// Mark this conflict as resolved with the given strategy
    pub fn resolve(&mut self, strategy: ResolutionStrategy) {
        self.resolved = true;
        self.resolution = Some(strategy);
    }
    
    /// Check if this is a critical conflict that must be resolved
    pub fn is_critical(&self) -> bool {
        self.conflict_type.is_critical()
    }
}

/// Manager for detecting and resolving plugin conflicts
pub struct ConflictManager {
    conflicts: Vec<PluginConflict>,
}

impl ConflictManager {
    /// Create a new conflict manager
    pub fn new() -> Self {
        Self {
            conflicts: Vec::new(),
        }
    }
    
    /// Add a conflict
    pub fn add_conflict(&mut self, conflict: PluginConflict) {
        self.conflicts.push(conflict);
    }
    
    /// Get all conflicts
    pub fn get_conflicts(&self) -> &[PluginConflict] {
        &self.conflicts
    }
    
    /// Get unresolved conflicts
    pub fn get_unresolved_conflicts(&self) -> Vec<&PluginConflict> {
        self.conflicts.iter().filter(|c| !c.resolved).collect()
    }
    
    /// Get critical unresolved conflicts
    pub fn get_critical_unresolved_conflicts(&self) -> Vec<&PluginConflict> {
        self.conflicts
            .iter()
            .filter(|c| !c.resolved && c.is_critical())
            .collect()
    }
/// Check if a conflict already exists between two specific plugins (order doesn't matter).
    pub fn has_conflict_between(&self, id1: &str, id2: &str) -> bool {
        self.conflicts.iter().any(|c|
            (c.first_plugin == id1 && c.second_plugin == id2) ||
            (c.first_plugin == id2 && c.second_plugin == id1)
        )
    }
    
    /// Resolve a conflict
    pub fn resolve_conflict(&mut self, index: usize, strategy: ResolutionStrategy) -> Result<()> {
        if index >= self.conflicts.len() {
            return Err(PluginSystemError::ConflictError { message: format!("Conflict index out of bounds: {}", index) }.into());
        }
        
        self.conflicts[index].resolve(strategy);
        Ok(())
    }
    
    /// Check if all critical conflicts are resolved
    pub fn all_critical_conflicts_resolved(&self) -> bool {
        !self.conflicts
            .iter()
            .any(|c| !c.resolved && c.is_critical())
    }
    
    /// Get plugins that should be disabled based on conflict resolutions
    pub fn get_plugins_to_disable(&self) -> Vec<String> {
        let mut to_disable = Vec::new();
        
        for conflict in &self.conflicts {
            if !conflict.resolved {
                continue;
            }
            
            match &conflict.resolution {
                Some(ResolutionStrategy::DisableFirst) => {
                    to_disable.push(conflict.first_plugin.clone());
                }
                Some(ResolutionStrategy::DisableSecond) => {
                    to_disable.push(conflict.second_plugin.clone());
                }
                _ => {}
            }
        }
        
        // Remove duplicates
        to_disable.sort();
        to_disable.dedup();
        
        to_disable
    }
    
    /// Detect conflicts between plugins based on their manifests.
    /// This method will populate the internal list of conflicts.
    pub fn detect_conflicts(&mut self, manifests: &[PluginManifest]) -> Result<()> {
        self.conflicts.clear(); // Clear previous detections if any

        // Iterate through all pairs of manifests
        for i in 0..manifests.len() {
            for j in (i + 1)..manifests.len() {
                let m1 = &manifests[i];
                let m2 = &manifests[j];

                // Check for resource claim conflicts
                for claim1_manifest in &m1.resources {
                    for claim2_manifest in &m2.resources {
                        if claim1_manifest.resource_type == claim2_manifest.resource_type &&
                           claim1_manifest.identifier == claim2_manifest.identifier {
                            
                            // Map ManifestResourceAccessType to new ResourceAccessType
                            let access1 = match claim1_manifest.access {
                                ManifestResourceAccessType::ExclusiveWrite => ResourceAccessType::ExclusiveWrite,
                                ManifestResourceAccessType::SharedRead => ResourceAccessType::SharedRead,
                                ManifestResourceAccessType::ProvidesUniqueId => ResourceAccessType::ProvidesUniqueId,
                            };
                            let access2 = match claim2_manifest.access {
                                ManifestResourceAccessType::ExclusiveWrite => ResourceAccessType::ExclusiveWrite,
                                ManifestResourceAccessType::SharedRead => ResourceAccessType::SharedRead,
                                ManifestResourceAccessType::ProvidesUniqueId => ResourceAccessType::ProvidesUniqueId,
                            };

                            // Potential conflict on the same resource instance
                            // Use the conflict matrix logic from the design document (Section 4.1)
                            let conflict_exists = match (access1, access2) {
                                // ExclusiveWrite conflicts with anything
                                (ResourceAccessType::ExclusiveWrite, _) | (_, ResourceAccessType::ExclusiveWrite) => true,

                                // ExclusiveRead conflicts
                                (ResourceAccessType::ExclusiveRead, ResourceAccessType::ExclusiveRead) => true,
                                (ResourceAccessType::ExclusiveRead, ResourceAccessType::SharedWrite) => true,
                                (ResourceAccessType::ExclusiveRead, ResourceAccessType::SharedRead) => true,
                                (ResourceAccessType::ExclusiveRead, ResourceAccessType::ProvidesUniqueId) => true,
                                (ResourceAccessType::SharedWrite, ResourceAccessType::ExclusiveRead) => true, // Symmetric
                                (ResourceAccessType::SharedRead, ResourceAccessType::ExclusiveRead) => true,   // Symmetric
                                (ResourceAccessType::ProvidesUniqueId, ResourceAccessType::ExclusiveRead) => true, // Symmetric
                                
                                // SharedWrite conflicts (includes Yes* from design)
                                (ResourceAccessType::SharedWrite, ResourceAccessType::SharedWrite) => true,
                                (ResourceAccessType::SharedWrite, ResourceAccessType::SharedRead) => true,
                                (ResourceAccessType::SharedWrite, ResourceAccessType::ProvidesUniqueId) => true,
                                (ResourceAccessType::SharedRead, ResourceAccessType::SharedWrite) => true,   // Symmetric
                                (ResourceAccessType::ProvidesUniqueId, ResourceAccessType::SharedWrite) => true, // Symmetric

                                // ProvidesUniqueId conflicts
                                (ResourceAccessType::ProvidesUniqueId, ResourceAccessType::ProvidesUniqueId) => true,

                                // Non-conflicting cases
                                (ResourceAccessType::SharedRead, ResourceAccessType::SharedRead) => false,
                                (ResourceAccessType::SharedRead, ResourceAccessType::ProvidesUniqueId) => false,
                                (ResourceAccessType::ProvidesUniqueId, ResourceAccessType::SharedRead) => false,
                                // This match is exhaustive for all 5x5 combinations of ResourceAccessType
                            };

                            if conflict_exists {
                                let resource_id = ResourceIdentifier {
                                    kind: claim1_manifest.resource_type.clone(),
                                    id: claim1_manifest.identifier.clone(),
                                };
                                let description = format!(
                                    "Resource conflict on type '{}', identifier '{}'. Plugin '{}' claims access {:?}, and Plugin '{}' claims access {:?}.",
                                    resource_id.kind,
                                    resource_id.id,
                                    m1.id,
                                    access1, // Use the new mapped access type
                                    m2.id,
                                    access2  // Use the new mapped access type
                                );
                                self.add_conflict(PluginConflict::new(
                                    &m1.id,
                                    &m2.id,
                                    ConflictType::ResourceConflict {
                                        resource: resource_id,
                                        first_plugin_access: access1,
                                        second_plugin_access: access2,
                                    },
                                    &description,
                                ));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl Default for ConflictManager {
    fn default() -> Self {
        Self::new()
    }
}