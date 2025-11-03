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
pub struct TelexMessage {
    pub kind: String,
    pub role: String,
    pub parts: Vec<MessagePart>,
    
    #[serde(rename = "messageId")]
    pub message_id: String,
    
    #[serde(rename = "taskId", skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum MessagePart {
    Text { 
        kind: String,
        text: String 
    },
    Data { 
        kind: String,
        data: Vec<Value> 
    },
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<TaskResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<A2AError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskResult {
    pub kind: String,
    pub id: String,
    #[serde(rename = "contextId")]
    pub context_id: String,
    pub status: TaskStatus,
    pub artifacts: Vec<Artifact>,
    pub history: Vec<TelexMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskStatus {
    pub state: String,
    pub timestamp: String,
    pub message: TelexMessage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Artifact {
    #[serde(rename = "artifactId")]
    pub artifact_id: String,
    pub name: String,
    pub parts: Vec<MessagePart>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct A2AError {
    pub code: i32,
    pub message: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl A2AResponse {
    pub fn success(
        request_id: String,
        task_id: String,
        context_id: String,
        response_text: String,
        request_message: &TelexMessage,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        let response_message_id = uuid::Uuid::new_v4().to_string();
        
        let response_message = TelexMessage {
            kind: "message".to_string(),
            role: "agent".to_string(),
            message_id: response_message_id,
            task_id: Some(task_id.clone()),
            parts: vec![MessagePart::Text {
                kind: "text".to_string(),
                text: response_text.clone(),
            }],
        };
        
        let artifact_id = uuid::Uuid::new_v4().to_string();
        
        Self {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            result: Some(TaskResult {
                kind: "task".to_string(),
                id: task_id,
                context_id,
                status: TaskStatus {
                    state: "completed".to_string(),
                    timestamp: now,
                    message: response_message.clone(),
                },
                artifacts: vec![Artifact {
                    artifact_id,
                    name: "secretDetectorResponse".to_string(),
                    parts: vec![MessagePart::Text {
                        kind: "text".to_string(),
                        text: response_text,
                    }],
                }],
                history: vec![request_message.clone(), response_message],
            }),
            error: None,
        }
    }
    
    pub fn error(request_id: String, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            result: None,
            error: Some(A2AError {
                code,
                message,
                data: None,
            }),
        }
    }
}