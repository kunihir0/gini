use std::collections::{HashMap, HashSet};

// Removed unused: use crate::kernel::error::Error as KernelError;
use crate::stage_manager::requirement::StageRequirement;
use crate::stage_manager::error::StageSystemError; // Import StageSystemError

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
    
    /// Check if the graph contains cycles. Returns Ok(cycle_path) if a cycle is detected.
    fn find_cycle_path(&self) -> Option<Vec<String>> {
        let mut visited: HashSet<&str> = HashSet::new(); // Store &str
        let mut recursion_stack: HashSet<&str> = HashSet::new(); // Store &str
        let mut path = Vec::new();

        for node_string in &self.nodes { // node_string is &String
            let node_str: &str = node_string.as_str(); // Convert to &str for consistent use
            if !visited.contains(node_str) {
                if self.detect_cycle_dfs(node_str, &mut visited, &mut recursion_stack, &mut path) {
                    // A cycle was detected, path is populated by detect_cycle_dfs
                    // The path needs to be trimmed to the actual cycle
                    // node_str is the starting point of this DFS traversal that found a cycle
                    if let Some(_cycle_start_index) = path.iter().rposition(|n_in_path| n_in_path == node_str) { // Prefixed with underscore
                         // The cycle actually starts where the back edge points to an element in the recursion_stack
                         // The 'path' at this point contains the full DFS path leading to the cycle.
                         // The last element added to path in detect_cycle_dfs is the one completing the cycle.
                        if let Some(last_node_in_path) = path.last() {
                            if let Some(actual_cycle_start_index) = path.iter().position(|n| n == last_node_in_path) {
                                if actual_cycle_start_index < path.len() -1 { // Ensure it's not just the last element itself
                                    return Some(path[actual_cycle_start_index..].to_vec());
                                }
                            }
                        }
                        // Fallback or if the above logic isn't perfect for trimming, return the current path.
                        // This part might need more robust cycle path extraction.
                        return Some(path);
                    }
                    return Some(path);
                }
            }
        }
        None
    }

    // Helper DFS function to detect cycle and populate path
    // Returns true if a cycle is detected
    fn detect_cycle_dfs<'a>(&'a self, node: &'a str, visited: &mut HashSet<&'a str>, recursion_stack: &mut HashSet<&'a str>, path: &mut Vec<String>) -> bool {
        visited.insert(node);
        recursion_stack.insert(node);
        path.push(node.to_string());

        if let Some(dependencies) = self.edges.get(node) { // node is &str, self.edges keys are String
            for dependency_string in dependencies { // dependency_string is &String
                let dependency_str: &str = dependency_string.as_str(); // Convert to &str
                if !visited.contains(dependency_str) {
                    if self.detect_cycle_dfs(dependency_str, visited, recursion_stack, path) {
                        return true;
                    }
                } else if recursion_stack.contains(dependency_str) {
                    // Cycle detected
                    path.push(dependency_string.to_string()); // Add the node that closes the cycle
                    return true;
                }
            }
        }

        path.pop();
        recursion_stack.remove(node);
        false
    }
    
    /// Get a topologically sorted list of nodes
    pub fn topological_sort(&self) -> std::result::Result<Vec<String>, StageSystemError> {
        if let Some(cycle_path) = self.find_cycle_path() {
            return Err(StageSystemError::DependencyCycleDetected {
                // Assuming DependencyGraph doesn't have a name, use a generic one or leave it out
                pipeline_name: "DependencyGraph".to_string(),
                cycle_path,
            });
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
    pub fn validate(&self) -> std::result::Result<(), StageSystemError> {
        if let Some(cycle_path) = self.find_cycle_path() {
            return Err(StageSystemError::DependencyCycleDetected {
                pipeline_name: "DependencyGraph".to_string(), // Or a more specific identifier if available
                cycle_path,
            });
        }
        
        let missing = self.missing_requirements();
        if !missing.is_empty() {
            return Err(StageSystemError::MissingStageDependencies {
                entity_name: "DependencyGraph".to_string(), // Or a more specific identifier
                missing_stages: missing,
            });
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