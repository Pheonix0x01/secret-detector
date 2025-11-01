use crate::models::scan::Finding;
use crate::models::a2a::Message;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use log::{info, error};

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    temperature: f32,
    #[serde(rename = "topK")]
    top_k: u32,
    #[serde(rename = "topP")]
    top_p: f32,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: CandidateContent,
}

#[derive(Debug, Deserialize)]
struct CandidateContent {
    parts: Vec<CandidatePart>,
}

#[derive(Debug, Deserialize)]
struct CandidatePart {
    text: String,
}

#[derive(Debug, Deserialize)]
pub struct ScanCommand {
    pub scan_mode: String,
    pub repo_url: Option<String>,
    pub action: String,
}

pub struct GeminiClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl GeminiClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
        }
    }

    async fn generate_content(&self, prompt: &str) -> Result<String> {
        let url = format!(
            "{}/models/{}:generateContent",
            self.base_url, self.model
        );

        let request = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: GenerationConfig {
                temperature: 0.7,
                top_k: 40,
                top_p: 0.95,
                max_output_tokens: 2048,
            },
        };

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-goog-api-key", &self.api_key)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Gemini API error: {}", error_text);
            return Err(anyhow!("Gemini API error: {}", error_text));
        }

        let gemini_response: GeminiResponse = response.json().await?;
        
        let text = gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| anyhow!("No content in Gemini response"))?;

        Ok(text)
    }

    pub async fn parse_user_intent(&self, message: &str, history: &[Message]) -> Result<ScanCommand> {
        let history_context = history
            .iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            r#"Parse this user message and respond ONLY with valid JSON, nothing else.

Conversation history:
{}

User message: "{}"

Respond with this exact JSON structure:
{{
  "scan_mode": "quick",
  "repo_url": "https://github.com/octocat/Hello-World",
  "action": "start_scan"
}}

Rules:
- scan_mode: "quick", "running", or "deep"
- repo_url: full GitHub URL or null
- action: "start_scan", "continue_scan", "status", or "help"

JSON only, no markdown, no explanation:"#,
            history_context, message
        );

        info!("Sending prompt to Gemini for intent parsing");
        let response = self.generate_content(&prompt).await?;
        info!("Raw Gemini response: {}", response);
        
        let cleaned = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        info!("Cleaned response: {}", cleaned);

        match serde_json::from_str::<ScanCommand>(cleaned) {
            Ok(cmd) => {
                info!("Successfully parsed scan command: action={}, mode={}", cmd.action, cmd.scan_mode);
                Ok(cmd)
            }
            Err(e) => {
                error!("Failed to parse Gemini response: {}", e);
                error!("Response was: {}", cleaned);
                Err(anyhow!("Failed to parse Gemini response as JSON: {}. Response was: {}", e, cleaned))
            }
        }
    }

    pub async fn generate_response(&self, findings: &[Finding], repo_url: &str, scan_mode: &str, commit_count: usize) -> Result<String> {
        let findings_summary = if findings.is_empty() {
            "No secrets found.".to_string()
        } else {
            findings
                .iter()
                .map(|f| {
                    format!(
                        "- {} ({:?}) in {} at line {}",
                        f.secret_type, f.severity, f.file_path, f.line_number
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let prompt = format!(
            r#"You are a helpful GitHub security assistant. Generate a conversational response about the scan results.

Scan info:
- Repository: {}
- Scan mode: {}
- Commits scanned: {}
- Secrets found: {}

Findings:
{}

Generate a friendly, clear response that:
1. Summarizes what was scanned
2. Reports findings with severity
3. Provides actionable recommendations
4. Uses a conversational tone

Keep it concise but informative."#,
            repo_url, scan_mode, commit_count, findings.len(), findings_summary
        );

        info!("Generating final response with Gemini");
        let response = self.generate_content(&prompt).await?;
        Ok(response)
    }
}