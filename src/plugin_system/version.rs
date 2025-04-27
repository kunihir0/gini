use std::str::FromStr;
use std::fmt;

/// Error type for version parsing
#[derive(Debug)]
pub enum VersionError {
    InvalidFormat,
    ParseError(String),
}

impl fmt::Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionError::InvalidFormat => write!(f, "Invalid version format"),
            VersionError::ParseError(msg) => write!(f, "Version parse error: {}", msg),
        }
    }
}

/// Represents a semantic version for the API
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApiVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ApiVersion {
    /// Creates a new API version
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
    
    /// Parses a version string like "1.2.3"
    pub fn from_str(version: &str) -> Result<Self, VersionError> {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return Err(VersionError::InvalidFormat);
        }
        
        let parse_part = |part: &str| -> Result<u32, VersionError> {
            part.parse::<u32>().map_err(|e| VersionError::ParseError(e.to_string()))
        };
        
        let major = parse_part(parts[0])?;
        let minor = parse_part(parts[1])?;
        let patch = parse_part(parts[2])?;
        
        Ok(Self::new(major, minor, patch))
    }
    
    /// Checks if this version is compatible with another version
    /// Based on semantic versioning rules: major versions must match
    pub fn is_compatible_with(&self, other: &ApiVersion) -> bool {
        self.major == other.major
    }
    
    /// Returns version as a string like "1.2.3"
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for ApiVersion {
    type Err = VersionError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ApiVersion::from_str(s)
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Represents a range of compatible API versions
#[derive(Debug, Clone)]
pub struct VersionRange {
    pub min: ApiVersion,
    pub max: ApiVersion,
}

impl VersionRange {
    /// Creates a new version range
    pub fn new(min: ApiVersion, max: ApiVersion) -> Self {
        Self { min, max }
    }
    
    /// Checks if a version is within this range
    pub fn includes(&self, version: &ApiVersion) -> bool {
        &self.min <= version && version <= &self.max
    }
    
    /// Creates a version range from a semver-style constraint like "^1.2.3"
    pub fn from_constraint(constraint: &str) -> Result<Self, VersionError> {
        if constraint.starts_with('^') {
            // Caret range: compatible with everything that has the same major version
            // ^1.2.3 becomes 1.2.3 <= version < 2.0.0
            let version_str = &constraint[1..];
            let version = ApiVersion::from_str(version_str)?;
            
            let min = version.clone();
            let max = ApiVersion::new(version.major + 1, 0, 0);
            
            Ok(Self { min, max })
        } else if constraint.starts_with('~') {
            // Tilde range: compatible with everything that has the same minor version
            // ~1.2.3 becomes 1.2.3 <= version < 1.3.0
            let version_str = &constraint[1..];
            let version = ApiVersion::from_str(version_str)?;
            
            let min = version.clone();
            let max = ApiVersion::new(version.major, version.minor + 1, 0);
            
            Ok(Self { min, max })
        } else if constraint.starts_with(">=") {
            // Greater than or equal: compatible with everything >= version
            let version_str = &constraint[2..];
            let version = ApiVersion::from_str(version_str)?;
            
            let min = version;
            // Set a very high max version
            let max = ApiVersion::new(999, 999, 999);
            
            Ok(Self { min, max })
        } else {
            // Default to exact match
            let version = ApiVersion::from_str(constraint)?;
            Ok(Self { min: version.clone(), max: version })
        }
    }
}