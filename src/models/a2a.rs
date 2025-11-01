use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct A2ARequest {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: A2AParams,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct A2AParams {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    pub task: Task,
    #[serde(rename = "userMessage")]
    pub user_message: Message,
    pub context: Context,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Context {
    pub history: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct A2AResponse {
    pub jsonrpc: String,
    pub id: String,
    pub result: A2AResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct A2AResult {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    pub text: String,
    pub artifacts: Vec<Artifact>,
    pub history: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Artifact {
    #[serde(rename = "type")]
    pub artifact_type: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct A2AErrorResponse {
    pub jsonrpc: String,
    pub id: String,
    pub error: A2AError,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct A2AError {
    pub code: i32,
    pub message: String,
    pub data: Option<A2AErrorData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct A2AErrorData {
    pub details: String,
}