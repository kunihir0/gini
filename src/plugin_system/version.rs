use std::str::FromStr;
use std::fmt;
use semver::{Version, VersionReq}; // Import semver types

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

/// Represents a version requirement range using semver constraints.
#[derive(Debug, Clone)]
pub struct VersionRange {
    /// The original constraint string (e.g., "^1.2.3", ">=2.0")
    constraint: String,
    /// The parsed semver requirement
    req: VersionReq,
}

impl VersionRange {
    /// Creates a new version range from a constraint string.
    pub fn from_constraint(constraint: &str) -> Result<Self, VersionError> {
        let req = VersionReq::parse(constraint)
            .map_err(|e| VersionError::ParseError(format!("Invalid version constraint '{}': {}", constraint, e)))?;
        Ok(Self {
            constraint: constraint.to_string(),
            req,
        })
    }

    /// Checks if a specific `semver::Version` satisfies this range.
    /// Note: This now takes a `semver::Version`, not `ApiVersion`.
    /// The calling code in loader.rs needs to parse the dependency's version string into a `semver::Version`.
    pub fn includes(&self, version: &Version) -> bool {
        self.req.matches(version)
    }

    /// Returns a reference to the underlying `semver::VersionReq`.
    pub fn semver_req(&self) -> &VersionReq {
        &self.req
    }

    /// Returns the original constraint string.
    pub fn constraint_string(&self) -> &str {
        &self.constraint
    }
}

/// Implement Display to show the original constraint string.
impl fmt::Display for VersionRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.constraint)
    }
}

/// Allow parsing directly from a string slice.
impl FromStr for VersionRange {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        VersionRange::from_constraint(s)
    }
}