use std::path::PathBuf;
use crate::plugin_system::version::VersionRange;
use crate::plugin_system::traits::PluginPriority;

/// Represents a plugin manifest that describes a plugin
#[derive(Debug, Clone)]
pub struct PluginManifest {
    /// Unique identifier for the plugin
    pub id: String,
    
    /// Human-readable name
    pub name: String,
    
    /// Plugin version
    pub version: String,
    
    /// Plugin description
    pub description: String,
    
    /// Plugin author
    pub author: String,
    
    /// Plugin website URL (optional)
    pub website: Option<String>,
    
    /// License information
    pub license: Option<String>,
    
    /// Compatible API versions
    pub api_versions: Vec<VersionRange>,
    
    /// Plugin dependencies
    pub dependencies: Vec<DependencyInfo>,
    
    /// Whether this is a core plugin
    pub is_core: bool,
    
    /// Plugin priority
    pub priority: Option<String>,
    
    /// Entry point (library name or script path)
    pub entry_point: String,
    
    /// Additional plugin files
    pub files: Vec<String>,
    
    /// Plugin configuration schema (optional)
    pub config_schema: Option<String>,
    
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Represents a dependency on another plugin
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    /// Plugin ID
    pub id: String,
    
    /// Required version range (optional)
    pub version_range: Option<VersionRange>,
    
    /// Whether this dependency is required
    pub required: bool,
}

impl PluginManifest {
    /// Create a new plugin manifest
    pub fn new(id: &str, name: &str, version: &str, description: &str, author: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            description: description.to_string(),
            author: author.to_string(),
            website: None,
            license: None,
            api_versions: Vec::new(),
            dependencies: Vec::new(),
            is_core: false,
            priority: None,
            entry_point: format!("lib{}.so", id),
            files: Vec::new(),
            config_schema: None,
            tags: Vec::new(),
        }
    }
    
    /// Add an API version range
    pub fn add_api_version(&mut self, version_range: VersionRange) -> &mut Self {
        self.api_versions.push(version_range);
        self
    }
    
    /// Add a dependency
    pub fn add_dependency(&mut self, id: &str, version_range: Option<VersionRange>, required: bool) -> &mut Self {
        self.dependencies.push(DependencyInfo {
            id: id.to_string(),
            version_range,
            required,
        });
        self
    }
    
    /// Set the plugin priority
    pub fn set_priority(&mut self, priority: PluginPriority) -> &mut Self {
        self.priority = Some(priority.to_string());
        self
    }
    
    /// Mark this as a core plugin
    pub fn set_core(&mut self, is_core: bool) -> &mut Self {
        self.is_core = is_core;
        self
    }
    
    /// Add a tag to the plugin
    pub fn add_tag(&mut self, tag: &str) -> &mut Self {
        self.tags.push(tag.to_string());
        self
    }
    
    /// Get the plugin priority
    pub fn get_priority(&self) -> Option<PluginPriority> {
        self.priority.as_ref().and_then(|p| PluginPriority::from_str(p))
    }
}

/// Builder for creating a plugin manifest
pub struct ManifestBuilder {
    manifest: PluginManifest,
}

impl ManifestBuilder {
    /// Create a new manifest builder
    pub fn new(id: &str, name: &str, version: &str) -> Self {
        Self {
            manifest: PluginManifest::new(
                id,
                name,
                version,
                "A plugin for OSX-Forge", // Default description
                "Unknown", // Default author
            ),
        }
    }
    
    /// Set the plugin description
    pub fn description(mut self, description: &str) -> Self {
        self.manifest.description = description.to_string();
        self
    }
    
    /// Set the plugin author
    pub fn author(mut self, author: &str) -> Self {
        self.manifest.author = author.to_string();
        self
    }
    
    /// Set the plugin website
    pub fn website(mut self, website: &str) -> Self {
        self.manifest.website = Some(website.to_string());
        self
    }
    
    /// Set the plugin license
    pub fn license(mut self, license: &str) -> Self {
        self.manifest.license = Some(license.to_string());
        self
    }
    
    /// Add an API version compatibility range
    pub fn api_version(mut self, version_range: VersionRange) -> Self {
        self.manifest.api_versions.push(version_range);
        self
    }
    
    /// Add a dependency
    pub fn dependency(mut self, id: &str, version_range: Option<VersionRange>, required: bool) -> Self {
        self.manifest.add_dependency(id, version_range, required);
        self
    }
    
    /// Set whether this is a core plugin
    pub fn core(mut self, is_core: bool) -> Self {
        self.manifest.is_core = is_core;
        self
    }
    
    /// Set the plugin priority
    pub fn priority(mut self, priority: PluginPriority) -> Self {
        self.manifest.set_priority(priority);
        self
    }
    
    /// Set the entry point
    pub fn entry_point(mut self, entry_point: &str) -> Self {
        self.manifest.entry_point = entry_point.to_string();
        self
    }
    
    /// Add a file to the plugin
    pub fn file(mut self, file: &str) -> Self {
        self.manifest.files.push(file.to_string());
        self
    }
    
    /// Add multiple files to the plugin
    pub fn files(mut self, files: &[&str]) -> Self {
        for file in files {
            self.manifest.files.push(file.to_string());
        }
        self
    }
    
    /// Add a tag to the plugin
    pub fn tag(mut self, tag: &str) -> Self {
        self.manifest.add_tag(tag);
        self
    }
    
    /// Add multiple tags to the plugin
    pub fn tags(mut self, tags: &[&str]) -> Self {
        for tag in tags {
            self.manifest.add_tag(tag);
        }
        self
    }
    
    /// Build the manifest
    pub fn build(self) -> PluginManifest {
        self.manifest
    }
}