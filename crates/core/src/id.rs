//! Typed ID wrappers for domain entities.
//!
//! Provides newtype wrappers around UUIDs to give compile-time type safety
//! when passing identifiers around the system. Each ID type is Copy, Eq,
//! Hash, and serializable.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Error returned when parsing an ID from a string fails.
#[derive(Debug, Clone, thiserror::Error)]
#[error("invalid id: {reason}")]
pub struct IdParseError {
    /// Explanation of why the parse failed.
    pub reason: String,
}

/// Macro to generate a typed ID wrapper.
macro_rules! typed_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Create a new random ID.
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Create an ID from a raw UUID.
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Create an ID from raw bytes.
            pub fn from_bytes(bytes: [u8; 16]) -> Self {
                Self(Uuid::from_bytes(bytes))
            }

            /// Return the inner UUID.
            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }

            /// Return the nil (all-zeros) ID, useful as a sentinel value.
            pub fn nil() -> Self {
                Self(Uuid::nil())
            }

            /// Check whether this ID is the nil sentinel.
            pub fn is_nil(&self) -> bool {
                self.0.is_nil()
            }

            /// Return the ID as a hyphenated lowercase string.
            pub fn to_hyphenated(&self) -> String {
                self.0.to_string()
            }

            /// Return the ID as a compact string without hyphens.
            pub fn to_compact(&self) -> String {
                self.0.as_simple().to_string()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl std::ops::Deref for $name {
            type Target = Uuid;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = IdParseError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s)
                    .map(Self)
                    .map_err(|e| IdParseError { reason: e.to_string() })
            }
        }

        impl PartialEq<Uuid> for $name {
            fn eq(&self, other: &Uuid) -> bool {
                self.0 == *other
            }
        }
    };
}

typed_id!(
    /// Unique identifier for a conversation thread.
    ThreadId
);

typed_id!(
    /// Unique identifier for a project/workspace.
    ProjectId
);

typed_id!(
    /// Unique identifier for a user session.
    SessionId
);

typed_id!(
    /// Unique identifier for a single message within a thread.
    MessageId
);

typed_id!(
    /// Unique identifier for a user account.
    UserId
);

typed_id!(
    /// Unique identifier for a plugin installation.
    PluginId
);

typed_id!(
    /// Unique identifier for an agent task.
    TaskId
);

typed_id!(
    /// Unique identifier for a tool invocation.
    ToolCallId
);

/// Helper to generate a batch of IDs of the same type.
pub fn generate_ids<T: From<Uuid>>(count: usize) -> Vec<T> {
    (0..count).map(|_| T::from(Uuid::new_v4())).collect()
}

/// A pair of IDs representing a parent-child relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdPair<P, C> {
    /// The parent identifier.
    pub parent: P,
    /// The child identifier.
    pub child: C,
}

impl<P: fmt::Display, C: fmt::Display> fmt::Display for IdPair<P, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.parent, self.child)
    }
}

/// Map an ID to a short display form (first 8 hex chars).
pub fn short_id(id: &Uuid) -> String {
    let s = id.as_simple().to_string();
    s[..8].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_id_roundtrip() {
        let id = ThreadId::new();
        let s = id.to_string();
        let parsed: ThreadId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn nil_id() {
        let id = SessionId::nil();
        assert!(id.is_nil());
    }

    #[test]
    fn generate_batch() {
        let ids: Vec<MessageId> = generate_ids(5);
        assert_eq!(ids.len(), 5);
        // All should be unique.
        let set: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(set.len(), 5);
    }
}
