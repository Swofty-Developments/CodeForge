//! Plugin API trait and host-to-plugin communication types.
//!
//! Defines the interface that the host exposes to plugins, including
//! capability negotiation, versioned API surface, and request/response
//! message types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// The API version for host-plugin communication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ApiVersion {
    /// Major version (breaking changes).
    pub major: u32,
    /// Minor version (additive changes).
    pub minor: u32,
}

impl ApiVersion {
    /// The current API version.
    pub const CURRENT: Self = Self { major: 1, minor: 0 };

    /// Create a new API version.
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Check if a plugin requiring `required` is compatible with this host version.
    pub fn is_compatible_with(&self, required: &ApiVersion) -> bool {
        self.major == required.major && self.minor >= required.minor
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// A capability that the host API can provide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApiCapability {
    /// Read files from the workspace.
    FileSystem,
    /// Execute shell commands.
    Shell,
    /// Access git operations.
    Git,
    /// Send notifications to the user.
    Notifications,
    /// Access the editor / UI.
    Editor,
    /// Access AI model inference.
    ModelInference,
    /// Access configuration.
    Configuration,
    /// Access logging.
    Logging,
    /// Access storage (key-value store for plugin data).
    Storage,
    /// Access network (make HTTP requests).
    Network,
}

impl ApiCapability {
    /// Return all available capabilities.
    pub fn all() -> &'static [ApiCapability] {
        &[
            ApiCapability::FileSystem,
            ApiCapability::Shell,
            ApiCapability::Git,
            ApiCapability::Notifications,
            ApiCapability::Editor,
            ApiCapability::ModelInference,
            ApiCapability::Configuration,
            ApiCapability::Logging,
            ApiCapability::Storage,
            ApiCapability::Network,
        ]
    }
}

impl fmt::Display for ApiCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiCapability::FileSystem => write!(f, "filesystem"),
            ApiCapability::Shell => write!(f, "shell"),
            ApiCapability::Git => write!(f, "git"),
            ApiCapability::Notifications => write!(f, "notifications"),
            ApiCapability::Editor => write!(f, "editor"),
            ApiCapability::ModelInference => write!(f, "model-inference"),
            ApiCapability::Configuration => write!(f, "configuration"),
            ApiCapability::Logging => write!(f, "logging"),
            ApiCapability::Storage => write!(f, "storage"),
            ApiCapability::Network => write!(f, "network"),
        }
    }
}

/// The result of capability negotiation between host and plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityNegotiation {
    /// Capabilities requested by the plugin.
    pub requested: Vec<ApiCapability>,
    /// Capabilities granted by the host.
    pub granted: Vec<ApiCapability>,
    /// Capabilities denied by the host.
    pub denied: Vec<ApiCapability>,
    /// The negotiated API version.
    pub api_version: ApiVersion,
}

impl CapabilityNegotiation {
    /// Negotiate capabilities from a request.
    pub fn negotiate(
        requested: Vec<ApiCapability>,
        available: &[ApiCapability],
    ) -> Self {
        let mut granted = Vec::new();
        let mut denied = Vec::new();
        for cap in &requested {
            if available.contains(cap) {
                granted.push(*cap);
            } else {
                denied.push(*cap);
            }
        }
        Self {
            requested,
            granted,
            denied,
            api_version: ApiVersion::CURRENT,
        }
    }

    /// Check if all requested capabilities were granted.
    pub fn fully_granted(&self) -> bool {
        self.denied.is_empty()
    }

    /// Check if a specific capability was granted.
    pub fn has_capability(&self, cap: ApiCapability) -> bool {
        self.granted.contains(&cap)
    }
}

/// A request from a plugin to the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRequest {
    /// Unique request ID for correlation.
    pub id: String,
    /// The method being called.
    pub method: String,
    /// The request parameters.
    pub params: HashMap<String, serde_json::Value>,
}

impl PluginRequest {
    /// Create a new request.
    pub fn new(id: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            method: method.into(),
            params: HashMap::new(),
        }
    }

    /// Add a parameter.
    pub fn with_param(
        mut self,
        key: impl Into<String>,
        value: serde_json::Value,
    ) -> Self {
        self.params.insert(key.into(), value);
        self
    }

    /// Get a string parameter.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.params.get(key).and_then(|v| v.as_str())
    }

    /// Get a boolean parameter.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.params.get(key).and_then(|v| v.as_bool())
    }
}

/// A response from the host to a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResponse {
    /// The request ID this is responding to.
    pub id: String,
    /// Whether the request succeeded.
    pub success: bool,
    /// The response data.
    pub data: Option<serde_json::Value>,
    /// Error message if the request failed.
    pub error: Option<PluginApiError>,
}

impl PluginResponse {
    /// Create a successful response.
    pub fn ok(id: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Create an error response.
    pub fn err(id: impl Into<String>, error: PluginApiError) -> Self {
        Self {
            id: id.into(),
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

/// An error returned from the plugin API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginApiError {
    /// Error code.
    pub code: ApiErrorCode,
    /// Human-readable message.
    pub message: String,
    /// Additional details.
    pub details: Option<serde_json::Value>,
}

impl fmt::Display for PluginApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

/// Standard API error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiErrorCode {
    /// The request was invalid.
    InvalidRequest,
    /// The method was not found.
    MethodNotFound,
    /// Missing required parameters.
    MissingParams,
    /// The plugin lacks the required capability.
    PermissionDenied,
    /// The operation timed out.
    Timeout,
    /// An internal error occurred.
    Internal,
    /// The resource was not found.
    NotFound,
    /// Rate limit exceeded.
    RateLimited,
    /// API version mismatch.
    VersionMismatch,
}

/// An event pushed from the host to a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEvent {
    /// The event type.
    pub event_type: String,
    /// Event payload.
    pub payload: serde_json::Value,
    /// Timestamp (epoch millis).
    pub timestamp_ms: u64,
}

impl PluginEvent {
    /// Create a new event.
    pub fn new(event_type: impl Into<String>, payload: serde_json::Value) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        Self {
            event_type: event_type.into(),
            payload,
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

/// Standard host event types that plugins can subscribe to.
pub const EVENT_FILE_CHANGED: &str = "file.changed";
/// File was created.
pub const EVENT_FILE_CREATED: &str = "file.created";
/// File was deleted.
pub const EVENT_FILE_DELETED: &str = "file.deleted";
/// A message was sent in the chat.
pub const EVENT_MESSAGE_SENT: &str = "message.sent";
/// A tool was invoked.
pub const EVENT_TOOL_USED: &str = "tool.used";
/// A session started.
pub const EVENT_SESSION_START: &str = "session.start";
/// A session ended.
pub const EVENT_SESSION_END: &str = "session.end";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_version_compatibility() {
        let host = ApiVersion::new(1, 2);
        assert!(host.is_compatible_with(&ApiVersion::new(1, 0)));
        assert!(host.is_compatible_with(&ApiVersion::new(1, 2)));
        assert!(!host.is_compatible_with(&ApiVersion::new(1, 3)));
        assert!(!host.is_compatible_with(&ApiVersion::new(2, 0)));
    }

    #[test]
    fn capability_negotiation() {
        let available = vec![ApiCapability::FileSystem, ApiCapability::Logging];
        let negotiation = CapabilityNegotiation::negotiate(
            vec![ApiCapability::FileSystem, ApiCapability::Shell],
            &available,
        );
        assert!(negotiation.has_capability(ApiCapability::FileSystem));
        assert!(!negotiation.has_capability(ApiCapability::Shell));
        assert!(!negotiation.fully_granted());
    }

    #[test]
    fn request_response() {
        let req = PluginRequest::new("req-1", "fs.readFile")
            .with_param("path", serde_json::json!("/src/main.rs"));
        assert_eq!(req.get_str("path"), Some("/src/main.rs"));

        let resp = PluginResponse::ok("req-1", serde_json::json!({"content": "fn main(){}"}));
        assert!(resp.success);
    }
}
