use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct A2ARequest {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: A2AParams,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct A2AParams {
    pub message: TelexMessage,
    
    #[serde(default)]
    pub configuration: Option<Configuration>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimpleMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelexMessage {
    pub kind: String,
    pub role: String,
    pub parts: Vec<MessagePart>,
    
    #[serde(rename = "messageId")]
    pub message_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "kind")]
pub enum MessagePart {
    #[serde(rename = "text")]
    Text { text: String },
    
    #[serde(rename = "data")]
    Data { data: Vec<Value> },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Configuration {
    #[serde(rename = "acceptedOutputModes")]
    pub accepted_output_modes: Vec<String>,
    
    #[serde(rename = "historyLength")]
    pub history_length: u32,
    
    #[serde(rename = "pushNotificationConfig", skip_serializing_if = "Option::is_none")]
    pub push_notification_config: Option<Value>,
    
    pub blocking: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct A2AResponse {
    pub jsonrpc: String,
    pub id: String,
    pub result: A2AResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct A2AResult {
    pub message: ResponseMessage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMessage {
    pub kind: String,
    pub role: String,
    pub parts: Vec<ResponsePart>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponsePart {
    pub kind: String,
    pub text: String,
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
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<A2AErrorData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct A2AErrorData {
    pub details: String,
}