//! Semantic versioning parsing, comparison, and constraint matching.
//!
//! Implements a subset of the SemVer 2.0 specification with support for
//! version constraints using `>=`, `~`, `^`, and exact match operators.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// A parsed semantic version.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SemVer {
    /// Major version (breaking changes).
    pub major: u64,
    /// Minor version (new features, backwards-compatible).
    pub minor: u64,
    /// Patch version (bug fixes).
    pub patch: u64,
    /// Optional pre-release label (e.g., "alpha.1", "rc.2").
    pub pre_release: Option<String>,
    /// Optional build metadata (e.g., "build.123").
    pub build_metadata: Option<String>,
}

impl SemVer {
    /// Create a new version with only major.minor.patch.
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: None,
            build_metadata: None,
        }
    }

    /// Attach a pre-release label.
    pub fn with_pre_release(mut self, pre: impl Into<String>) -> Self {
        self.pre_release = Some(pre.into());
        self
    }

    /// Attach build metadata.
    pub fn with_build_metadata(mut self, meta: impl Into<String>) -> Self {
        self.build_metadata = Some(meta.into());
        self
    }

    /// Return the version with only major.minor.patch, stripping extras.
    pub fn base(&self) -> Self {
        Self::new(self.major, self.minor, self.patch)
    }

    /// Check whether this is a pre-release version.
    pub fn is_pre_release(&self) -> bool {
        self.pre_release.is_some()
    }

    /// Check whether this is a stable release (>= 1.0.0, no pre-release).
    pub fn is_stable(&self) -> bool {
        self.major >= 1 && self.pre_release.is_none()
    }

    /// Bump the major version, resetting minor and patch.
    pub fn bump_major(&self) -> Self {
        Self::new(self.major + 1, 0, 0)
    }

    /// Bump the minor version, resetting patch.
    pub fn bump_minor(&self) -> Self {
        Self::new(self.major, self.minor + 1, 0)
    }

    /// Bump the patch version.
    pub fn bump_patch(&self) -> Self {
        Self::new(self.major, self.minor, self.patch + 1)
    }

    /// Check whether `other` is API-compatible with `self` (same major, >= minor).
    pub fn is_compatible_with(&self, other: &SemVer) -> bool {
        if self.major == 0 && other.major == 0 {
            // In 0.x.y, only same minor is compatible.
            self.minor == other.minor && other.patch >= self.patch
        } else {
            self.major == other.major
                && (other.minor > self.minor
                    || (other.minor == self.minor && other.patch >= self.patch))
        }
    }

    /// Compare two versions ignoring build metadata (per SemVer spec).
    fn cmp_precedence(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {}
            ord => return ord,
        }
        // Pre-release versions have lower precedence than release.
        match (&self.pre_release, &other.pre_release) {
            (None, None) => Ordering::Equal,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (Some(a), Some(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp_precedence(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cmp_precedence(other)
    }
}

impl Default for SemVer {
    fn default() -> Self {
        Self::new(0, 1, 0)
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.pre_release {
            write!(f, "-{pre}")?;
        }
        if let Some(ref meta) = self.build_metadata {
            write!(f, "+{meta}")?;
        }
        Ok(())
    }
}

/// Error returned when parsing a version string fails.
#[derive(Debug, Clone, thiserror::Error)]
#[error("invalid version string: {reason}")]
pub struct VersionParseError {
    /// Explanation of why the parse failed.
    pub reason: String,
}

impl FromStr for SemVer {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix('v').unwrap_or(s);

        // Split off build metadata first.
        let (version_pre, build_metadata) = match s.split_once('+') {
            Some((vp, bm)) => (vp, Some(bm.to_string())),
            None => (s, None),
        };

        // Split off pre-release.
        let (version, pre_release) = match version_pre.split_once('-') {
            Some((v, pr)) => (v, Some(pr.to_string())),
            None => (version_pre, None),
        };

        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return Err(VersionParseError {
                reason: format!("expected 3 dot-separated components, got {}", parts.len()),
            });
        }

        let major = parts[0].parse::<u64>().map_err(|e| VersionParseError {
            reason: format!("invalid major version: {e}"),
        })?;
        let minor = parts[1].parse::<u64>().map_err(|e| VersionParseError {
            reason: format!("invalid minor version: {e}"),
        })?;
        let patch = parts[2].parse::<u64>().map_err(|e| VersionParseError {
            reason: format!("invalid patch version: {e}"),
        })?;

        Ok(Self {
            major,
            minor,
            patch,
            pre_release,
            build_metadata,
        })
    }
}

/// An operator used in version constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintOp {
    /// Exact match: `=1.2.3`.
    Exact,
    /// Greater than or equal: `>=1.2.3`.
    GreaterEqual,
    /// Greater than: `>1.2.3`.
    Greater,
    /// Less than: `<1.2.3`.
    Less,
    /// Less than or equal: `<=1.2.3`.
    LessEqual,
    /// Tilde: `~1.2.3` means `>=1.2.3, <1.3.0`.
    Tilde,
    /// Caret: `^1.2.3` means `>=1.2.3, <2.0.0`.
    Caret,
}

/// A single version constraint like `^1.2.3`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionConstraint {
    /// The comparison operator.
    pub op: ConstraintOp,
    /// The version to compare against.
    pub version: SemVer,
}

impl VersionConstraint {
    /// Create a new constraint.
    pub fn new(op: ConstraintOp, version: SemVer) -> Self {
        Self { op, version }
    }

    /// Check whether the given version satisfies this constraint.
    pub fn matches(&self, v: &SemVer) -> bool {
        match self.op {
            ConstraintOp::Exact => v == &self.version,
            ConstraintOp::GreaterEqual => v >= &self.version,
            ConstraintOp::Greater => v > &self.version,
            ConstraintOp::Less => v < &self.version,
            ConstraintOp::LessEqual => v <= &self.version,
            ConstraintOp::Tilde => {
                v >= &self.version
                    && v.major == self.version.major
                    && v.minor == self.version.minor
            }
            ConstraintOp::Caret => {
                if self.version.major == 0 {
                    if self.version.minor == 0 {
                        v == &self.version
                    } else {
                        v >= &self.version
                            && v.major == 0
                            && v.minor == self.version.minor
                    }
                } else {
                    v >= &self.version && v.major == self.version.major
                }
            }
        }
    }
}

impl fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.op {
            ConstraintOp::Exact => "=",
            ConstraintOp::GreaterEqual => ">=",
            ConstraintOp::Greater => ">",
            ConstraintOp::Less => "<",
            ConstraintOp::LessEqual => "<=",
            ConstraintOp::Tilde => "~",
            ConstraintOp::Caret => "^",
        };
        write!(f, "{prefix}{}", self.version)
    }
}

impl FromStr for VersionConstraint {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (op, rest) = if let Some(r) = s.strip_prefix(">=") {
            (ConstraintOp::GreaterEqual, r)
        } else if let Some(r) = s.strip_prefix("<=") {
            (ConstraintOp::LessEqual, r)
        } else if let Some(r) = s.strip_prefix('>') {
            (ConstraintOp::Greater, r)
        } else if let Some(r) = s.strip_prefix('<') {
            (ConstraintOp::Less, r)
        } else if let Some(r) = s.strip_prefix('~') {
            (ConstraintOp::Tilde, r)
        } else if let Some(r) = s.strip_prefix('^') {
            (ConstraintOp::Caret, r)
        } else if let Some(r) = s.strip_prefix('=') {
            (ConstraintOp::Exact, r)
        } else {
            (ConstraintOp::Exact, s)
        };

        let version = rest.trim().parse::<SemVer>()?;
        Ok(Self { op, version })
    }
}

/// A set of version constraints that must all be satisfied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionReq {
    /// Individual constraints that are ANDed together.
    pub constraints: Vec<VersionConstraint>,
}

impl VersionReq {
    /// Check whether a version satisfies all constraints in this requirement.
    pub fn matches(&self, v: &SemVer) -> bool {
        self.constraints.iter().all(|c| c.matches(v))
    }

    /// Parse a comma-separated constraint string like `>=1.0.0, <2.0.0`.
    pub fn parse(s: &str) -> Result<Self, VersionParseError> {
        let constraints = s
            .split(',')
            .map(|part| part.trim().parse::<VersionConstraint>())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { constraints })
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.constraints.iter().map(|c| c.to_string()).collect();
        write!(f, "{}", parts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let v: SemVer = "1.2.3".parse().unwrap();
        assert_eq!(v, SemVer::new(1, 2, 3));
    }

    #[test]
    fn parse_with_v_prefix() {
        let v: SemVer = "v2.0.0-beta.1+build.42".parse().unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.pre_release.as_deref(), Some("beta.1"));
        assert_eq!(v.build_metadata.as_deref(), Some("build.42"));
    }

    #[test]
    fn ordering() {
        let v1: SemVer = "1.0.0-alpha".parse().unwrap();
        let v2: SemVer = "1.0.0".parse().unwrap();
        assert!(v1 < v2);
    }

    #[test]
    fn caret_constraint() {
        let c: VersionConstraint = "^1.2.3".parse().unwrap();
        assert!(c.matches(&"1.9.0".parse().unwrap()));
        assert!(!c.matches(&"2.0.0".parse().unwrap()));
        assert!(!c.matches(&"1.2.2".parse().unwrap()));
    }

    #[test]
    fn tilde_constraint() {
        let c: VersionConstraint = "~1.2.3".parse().unwrap();
        assert!(c.matches(&"1.2.5".parse().unwrap()));
        assert!(!c.matches(&"1.3.0".parse().unwrap()));
    }

    #[test]
    fn version_req() {
        let req = VersionReq::parse(">=1.0.0, <2.0.0").unwrap();
        assert!(req.matches(&"1.5.0".parse().unwrap()));
        assert!(!req.matches(&"2.0.0".parse().unwrap()));
        assert!(!req.matches(&"0.9.0".parse().unwrap()));
    }
}
