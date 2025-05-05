use std::collections::{HashMap, HashSet};

use crate::kernel::error::{Error, Result};
use crate::stage_manager::requirement::StageRequirement;

/// Represents a node in the dependency graph
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DependencyNode {
    /// Stage ID
    pub id: String,
    /// Whether this stage is required
    pub required: bool,
    /// Whether this stage is provided by a plugin
    pub provided: bool,
}

impl From<&StageRequirement> for DependencyNode {
    fn from(req: &StageRequirement) -> Self {
        Self {
            id: req.stage_id.clone(),
            required: req.required,
            provided: req.provided,
        }
    }
}

/// Dependency graph for stages
pub struct DependencyGraph {
    /// Nodes in the graph (stage IDs)
    nodes: HashSet<String>,
    /// Edges in the graph (stage_id -> dependencies)
    edges: HashMap<String, Vec<String>>,
    /// Required stages
    required: HashSet<String>,
    /// Provided stages
    provided: HashSet<String>,
}

impl DependencyGraph {
    /// Create a new dependency graph
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            edges: HashMap::new(),
            required: HashSet::new(),
            provided: HashSet::new(),
        }
    }
    
    /// Add a node to the graph
    pub fn add_node(&mut self, node: &DependencyNode) {
        self.nodes.insert(node.id.clone());
        
        if node.required {
            self.required.insert(node.id.clone());
        }
        
        if node.provided {
            self.provided.insert(node.id.clone());
        }
    }
    
    /// Add an edge to the graph (stage_id depends on dependency)
    pub fn add_edge(&mut self, stage_id: &str, dependency: &str) {
        self.nodes.insert(stage_id.to_string());
        self.nodes.insert(dependency.to_string());
        
        self.edges
            .entry(stage_id.to_string())
            .or_default()
            .push(dependency.to_string());
    }
    
    /// Add multiple edges from stage_id to dependencies
    pub fn add_edges(&mut self, stage_id: &str, dependencies: &[&str]) {
        for dep in dependencies {
            self.add_edge(stage_id, dep);
        }
    }
    
    /// Check if the graph contains a node
    pub fn contains(&self, node_id: &str) -> bool {
        self.nodes.contains(node_id)
    }
    
    /// Get the dependencies of a node
    pub fn dependencies_of(&self, node_id: &str) -> Vec<String> {
        self.edges.get(node_id).cloned().unwrap_or_default()
    }
    
    /// Check if the graph contains cycles
    pub fn has_cycles(&self) -> bool {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        
        for node in &self.nodes {
            if !visited.contains(node) {
                if self.has_cycle_dfs(node, &mut visited, &mut stack) {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// DFS to check for cycles
    fn has_cycle_dfs(&self, node: &str, visited: &mut HashSet<String>, stack: &mut HashSet<String>) -> bool {
        visited.insert(node.to_string());
        stack.insert(node.to_string());
        
        if let Some(deps) = self.edges.get(node) {
            for dep in deps {
                if !visited.contains(dep) {
                    if self.has_cycle_dfs(dep, visited, stack) {
                        return true;
                    }
                } else if stack.contains(dep) {
                    return true;
                }
            }
        }
        
        stack.remove(node);
        false
    }
    
    /// Get a topologically sorted list of nodes
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        if self.has_cycles() {
            return Err(Error::Stage("Cannot sort: dependency graph contains cycles".to_string()));
        }
        
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        
        // Start with nodes that have no dependencies
        for node in &self.nodes {
            if !visited.contains(node) {
                self.visit_topsort(node, &mut visited, &mut result);
            }
        }
        
        // Reverse the result to get the correct order
        result.reverse();
        
        Ok(result)
    }
    
    /// DFS for topological sort
    fn visit_topsort(&self, node: &str, visited: &mut HashSet<String>, result: &mut Vec<String>) {
        visited.insert(node.to_string());
        
        if let Some(deps) = self.edges.get(node) {
            for dep in deps {
                if !visited.contains(dep) {
                    self.visit_topsort(dep, visited, result);
                }
            }
        }
        
        result.push(node.to_string());
    }
    
    /// Check if all required nodes are provided
    pub fn all_required_provided(&self) -> bool {
        for req in &self.required {
            if !self.provided.contains(req) {
                return false;
            }
        }
        true
    }
    
    /// Get missing requirements (required but not provided)
    pub fn missing_requirements(&self) -> Vec<String> {
        self.required
            .iter()
            .filter(|req| !self.provided.contains(*req))
            .cloned()
            .collect()
    }
    
    /// Validate that all requirements are met
    pub fn validate(&self) -> Result<()> {
        if self.has_cycles() {
            return Err(Error::Stage("Dependency graph contains cycles".to_string()));
        }
        
        let missing = self.missing_requirements();
        if !missing.is_empty() {
            return Err(Error::Stage(format!(
                "Missing required stages: {}",
                missing.join(", ")
            )));
        }
        
        Ok(())
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating a dependency graph from stage requirements
pub struct DependencyGraphBuilder {
    graph: DependencyGraph,
}

impl DependencyGraphBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            graph: DependencyGraph::new(),
        }
    }
    
    /// Add a stage requirement to the graph
    pub fn add_requirement(&mut self, req: &StageRequirement) -> &mut Self {
        let node = DependencyNode::from(req);
        self.graph.add_node(&node);
        self
    }
    
    /// Add multiple stage requirements
    pub fn add_requirements(&mut self, reqs: &[StageRequirement]) -> &mut Self {
        for req in reqs {
            self.add_requirement(req);
        }
        self
    }
    
    /// Add a dependency between stages
    pub fn add_dependency(&mut self, stage_id: &str, dependency: &str) -> &mut Self {
        self.graph.add_edge(stage_id, dependency);
        self
    }
    
    /// Build the dependency graph
    pub fn build(self) -> DependencyGraph {
        self.graph
    }
}

impl Default for DependencyGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}