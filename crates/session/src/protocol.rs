use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A JSON-RPC request (has `method`, `id`, and optional `params`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(default)]
    pub params: Value,
}

/// A JSON-RPC response (has `id`, and either `result` or `error`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// A JSON-RPC error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

/// A JSON-RPC notification (has `method` and `params`, but no `id`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// Parsed incoming JSON-RPC message.
#[derive(Debug, Clone)]
pub enum JsonRpcMessage {
    /// A request from the server (e.g., approval requests). Has an `id` that
    /// must be responded to.
    Request(JsonRpcRequest),
    /// A response to a previously sent request.
    Response(JsonRpcResponse),
    /// A notification (no response expected).
    Notification(JsonRpcNotification),
}

/// Parse a single NDJSON line into the appropriate JSON-RPC variant.
///
/// Heuristic:
/// - If the object has `method` and a numeric `id` field, it is a **Request**.
/// - If the object has `id` but no `method`, it is a **Response**.
/// - If the object has `method` but no `id`, it is a **Notification**.
pub fn parse_jsonrpc_line(line: &str) -> Result<JsonRpcMessage, serde_json::Error> {
    let v: Value = serde_json::from_str(line)?;

    let has_method = v.get("method").and_then(|m| m.as_str()).is_some();
    let has_id = v.get("id").is_some();

    if has_method && has_id {
        let req: JsonRpcRequest = serde_json::from_value(v)?;
        Ok(JsonRpcMessage::Request(req))
    } else if has_id && !has_method {
        let resp: JsonRpcResponse = serde_json::from_value(v)?;
        Ok(JsonRpcMessage::Response(resp))
    } else if has_method {
        let notif: JsonRpcNotification = serde_json::from_value(v)?;
        Ok(JsonRpcMessage::Notification(notif))
    } else {
        // Treat as notification with empty method as fallback.
        let notif: JsonRpcNotification = serde_json::from_value(v)?;
        Ok(JsonRpcMessage::Notification(notif))
    }
}
