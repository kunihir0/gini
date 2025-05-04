use std::collections::HashMap;
use crate::kernel::error::{Error, Result};

/// Types of plugin conflicts
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictType {
    /// Two plugins provide the same functionality and are mutually exclusive
    MutuallyExclusive,
    /// Two plugins have conflicting versions of the same dependency
    DependencyVersion,
    /// A plugin requires a resource already claimed by another plugin
    ResourceConflict,
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
            ConflictType::DependencyVersion => true,
            ConflictType::ResourceConflict => true,
            ConflictType::ExplicitlyIncompatible => true,
            ConflictType::PartialOverlap => false,
            ConflictType::Custom(_) => false,
        }
    }
    
    /// Get a human-readable description of this conflict type
    pub fn description(&self) -> &str {
        match self {
            ConflictType::MutuallyExclusive => "Mutually exclusive plugins",
            ConflictType::DependencyVersion => "Conflicting dependency versions",
            ConflictType::ResourceConflict => "Resource conflict",
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
            return Err(Error::Plugin(format!("Conflict index out of bounds: {}", index)));
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
    
    /// Detect conflicts between plugins (stub implementation)
    pub fn detect_conflicts(&mut self, _plugin_ids: &[String]) -> Result<()> {
        // This is a stub implementation that would be expanded in a real system
        // In a real implementation, we would:
        // 1. Check for plugins that declare themselves as incompatible
        // 2. Check for plugins that provide the same resources
        // 3. Check for conflicting dependency versions
        // 4. Check for plugins that use the same extension points
        // etc.
        
        Ok(())
    }
}

impl Default for ConflictManager {
    fn default() -> Self {
        Self::new()
    }
}