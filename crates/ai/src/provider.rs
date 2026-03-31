//! AI provider trait defining the interface for model backends.

use crate::context::ContextWindow;
use crate::message::Message;
use crate::streaming::StreamEvent;

/// Trait implemented by AI provider backends (Anthropic, Bedrock, Vertex, etc.).
///
/// Implementations handle the details of communicating with specific AI APIs,
/// managing sessions, and translating between the CodeForge message format
/// and the provider's native format.
pub trait AiProvider: Send + Sync {
    /// The error type returned by provider operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a new session with the given configuration.
    fn create_session(
        &self,
        config: SessionConfig,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<SessionHandle, Self::Error>> + Send + '_>,
    >;

    /// Send a message and receive streaming events.
    fn send_message(
        &self,
        handle: &SessionHandle,
        message: Message,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<StreamEvent>, Self::Error>> + Send + '_>,
    >;

    /// Interrupt the current generation.
    fn interrupt(
        &self,
        handle: &SessionHandle,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send + '_>,
    >;

    /// Resume a previously interrupted or saved session.
    fn resume(
        &self,
        session_id: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<SessionHandle, Self::Error>> + Send + '_>,
    >;

    /// Get the current context window usage for a session.
    fn context_usage(
        &self,
        handle: &SessionHandle,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ContextWindow, Self::Error>> + Send + '_>,
    >;
}

/// Configuration for creating a new AI session.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionConfig {
    /// The model identifier to use.
    pub model: String,
    /// System prompt to set the assistant's behavior.
    pub system_prompt: Option<String>,
    /// Maximum tokens in the response.
    pub max_tokens: u32,
    /// Temperature for response randomness (0.0 to 1.0).
    pub temperature: Option<f32>,
    /// Working directory for tool execution.
    pub working_directory: Option<String>,
    /// Permission mode for tool approvals.
    pub permission_mode: String,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            model: "sonnet".to_string(),
            system_prompt: None,
            max_tokens: 16384,
            temperature: None,
            working_directory: None,
            permission_mode: "normal".to_string(),
        }
    }
}

/// Handle representing an active AI session.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionHandle {
    /// Unique session identifier.
    pub session_id: String,
    /// The model being used.
    pub model: String,
    /// Whether the session is currently generating a response.
    pub is_active: bool,
}
