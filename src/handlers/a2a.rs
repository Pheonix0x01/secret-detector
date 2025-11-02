use crate::models::a2a::{A2ARequest, A2AParams, A2AResponse, A2AResult, A2AErrorResponse, A2AError, A2AErrorData, TelexMessage, MessagePart, ResponseMessage, ResponsePart};
use crate::models::scan::{ScanMode, ScanState, ScanStatus};
use crate::services::github::GitHubClient;
use crate::services::scanner::SecretScanner;
use crate::services::gemini::GeminiClient;
use crate::services::state::StateManager;
use actix_web::{web, HttpResponse, HttpRequest, Result as ActixResult};
use reqwest::Client;
use chrono::Utc;
use std::sync::Arc;
use log::{info, error};
use serde_json::json;

pub struct AppState {
    pub github_client: GitHubClient,
    pub scanner: SecretScanner,
    pub gemini_client: GeminiClient,
    pub state_manager: Arc<StateManager>,
    pub max_scan_commits: usize,
}

pub async fn handle_a2a_request(
    req: HttpRequest,
    body: web::Bytes,
    data: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let body_str = String::from_utf8_lossy(&body);
    info!("Received request body: {}", body_str);
    
    let a2a_request: A2ARequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to parse A2A request: {}. Body was: {}", e, body_str);
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid request format: {}", e)
            })));
        }
    };
    
    let request_id = a2a_request.id.clone();
    let is_blocking = a2a_request.params.configuration
        .as_ref()
        .map(|c| c.blocking)
        .unwrap_or(true);
    
    if is_blocking {
        match process_request(&a2a_request, &data).await {
            Ok(response) => Ok(HttpResponse::Ok().json(response)),
            Err(e) => {
                error!("Request processing failed: {}", e);
                let error_response = A2AErrorResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request_id,
                    error: A2AError {
                        code: -32603,
                        message: "Internal error".to_string(),
                        data: Some(A2AErrorData {
                            details: e.to_string(),
                        }),
                    },
                };
                Ok(HttpResponse::Ok().json(error_response))
            }
        }
    } else {
        let webhook_url = a2a_request.params.configuration
            .as_ref()
            .and_then(|c| c.push_notification_config.as_ref())
            .and_then(|p| p.get("url"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());
        
        let webhook_token = a2a_request.params.configuration
            .as_ref()
            .and_then(|c| c.push_notification_config.as_ref())
            .and_then(|p| p.get("token"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());
        
        if let Some(url) = webhook_url {
            info!("Non-blocking request, will send response to webhook: {}", url);
            
            let data_clone = data.clone();
            let req_clone = a2a_request.clone();
            
            actix_web::rt::spawn(async move {
                match process_request(&req_clone, &data_clone).await {
                    Ok(response) => {
                        if let Err(e) = send_webhook_response(&url, webhook_token.as_deref(), &response, &req_clone.id).await {
                            error!("Failed to send webhook response: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Request processing failed: {}", e);
                        if let Err(e) = send_webhook_error(&url, webhook_token.as_deref(), &e.to_string(), &req_clone.id).await {
                            error!("Failed to send webhook error: {}", e);
                        }
                    }
                }
            });
            
            Ok(HttpResponse::Accepted().json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": {
                    "status": "processing"
                }
            })))
        } else {
            error!("Non-blocking request but no webhook URL provided");
            Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Non-blocking mode requires webhook URL"
            })))
        }
    }
}

async fn send_webhook_response(url: &str, token: Option<&str>, response: &A2AResponse, request_id: &str) -> anyhow::Result<()> {
    let webhook_payload = json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "message/send",
        "params": {
            "message": {
                "kind": "message",
                "role": "agent",
                "messageId": uuid::Uuid::new_v4().to_string(),
                "parts": response.result.message.parts
            }
        }
    });
    
    let client = Client::new();
    let mut request = client.post(url).json(&webhook_payload);
    
    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }
    
    info!("Sending webhook response to: {}", url);
    let resp = request.send().await?;
    
    if resp.status().is_success() {
        info!("Webhook response sent successfully");
        Ok(())
    } else {
        let status = resp.status();
        let error_text = resp.text().await.unwrap_or_default();
        error!("Webhook failed with status {}: {}", status, error_text);
        Err(anyhow::anyhow!("Webhook failed: {} - {}", status, error_text))
    }
}

async fn send_webhook_error(url: &str, token: Option<&str>, error_msg: &str, request_id: &str) -> anyhow::Result<()> {
    let webhook_payload = json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "message/send",
        "params": {
            "message": {
                "kind": "message",
                "role": "agent",
                "messageId": uuid::Uuid::new_v4().to_string(),
                "parts": [
                    {
                        "kind": "text",
                        "text": format!("Sorry, an error occurred while processing your request: {}", error_msg)
                    }
                ]
            }
        }
    });
    
    let client = Client::new();
    let mut request = client.post(url).json(&webhook_payload);
    
    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }
    
    info!("Sending webhook error to: {}", url);
    let resp = request.send().await?;
    
    if resp.status().is_success() {
        info!("Webhook error sent successfully");
        Ok(())
    } else {
        let status = resp.status();
        let error_text = resp.text().await.unwrap_or_default();
        error!("Webhook failed with status {}: {}", status, error_text);
        Err(anyhow::anyhow!("Webhook failed: {} - {}", status, error_text))
    }
}

async fn process_request(
    req: &A2ARequest,
    data: &web::Data<AppState>,
) -> anyhow::Result<A2AResponse> {
    let user_message = extract_user_message(&req.params.message)?;
    
    info!("Processing request: {}", user_message);
    
    let command = data.gemini_client.parse_user_intent(&user_message, &[]).await?;
    
    info!("Parsed command - action: {}, mode: {}", command.action, command.scan_mode);
    
    let response_text = match command.action.as_str() {
        "start_scan" => {
            if let Some(ref repo_url) = command.repo_url {
                info!("Starting scan for: {}", repo_url);
                execute_scan(repo_url, &command.scan_mode, data).await?
            } else {
                "Please provide a GitHub repository URL to scan.".to_string()
            }
        }
        "continue_scan" => {
            if let Some(ref repo_url) = command.repo_url {
                continue_scan(repo_url, data).await?
            } else {
                "Please specify which repository to continue scanning.".to_string()
            }
        }
        "status" => {
            get_scan_status(data).await?
        }
        "help" => {
            get_help_message()
        }
        _ => {
            "I can help you scan GitHub repositories for exposed secrets. Try 'scan <repo-url>' or 'help' for more info.".to_string()
        }
    };
    
    info!("Generated response text, length: {}", response_text.len());
    
    Ok(A2AResponse {
        jsonrpc: "2.0".to_string(),
        id: req.id.clone(),
        result: A2AResult {
            message: ResponseMessage {
                kind: "message".to_string(),
                role: "assistant".to_string(),
                parts: vec![ResponsePart {
                    kind: "text".to_string(),
                    text: response_text,
                }],
            },
        },
    })
}

fn extract_user_message(message: &TelexMessage) -> anyhow::Result<String> {
    info!("Extracting message from {} parts", message.parts.len());
    
    for (i, part) in message.parts.iter().enumerate() {
        match part {
            MessagePart::Text { text } => {
                info!("Part {}: Text = '{}'", i, text);
                if !text.trim().is_empty() {
                    return Ok(text.clone());
                }
            }
            MessagePart::Data { data } => {
                info!("Part {}: Data with {} items", i, data.len());
                for (j, item) in data.iter().enumerate() {
                    info!("  Data[{}]: {}", j, item);
                    if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                        let text = text.trim();
                        if !text.is_empty() 
                            && !text.starts_with("<p>") 
                            && !text.starts_with("Scanning")
                            && !text.starts_with("Here")
                            && text.len() > 5 {
                            info!("✓ Found message in data: '{}'", text);
                            return Ok(text.to_string());
                        }
                    }
                }
            }
        }
    }
    
    Err(anyhow::anyhow!("No valid user message found"))
}

async fn execute_scan(
    repo_url: &str,
    scan_mode: &str,
    data: &web::Data<AppState>,
) -> anyhow::Result<String> {
    info!("Parsing repo URL: {}", repo_url);
    let (owner, repo) = GitHubClient::parse_repo_url(repo_url)?;
    
    info!("Fetching repository info for {}/{}", owner, repo);
    let _repository = data.github_client.get_repository(&owner, &repo).await?;
    
    let mode = match scan_mode {
        "running" => ScanMode::Running,
        "deep" => ScanMode::Deep,
        _ => ScanMode::Quick,
    };
    
    info!("Listing commits for {}/{}", owner, repo);
    let commits = data.github_client.list_commits(
        &owner,
        &repo,
        None,
        data.max_scan_commits as u32,
    ).await?;
    
    info!("Found {} commits to scan", commits.len());
    
    let mut all_findings = Vec::new();
    
    for (idx, commit) in commits.iter().enumerate() {
        info!("Scanning commit {}/{}: {}", idx + 1, commits.len(), commit.sha);
        let commit_details = data.github_client.get_commit(&owner, &repo, &commit.sha).await?;
        let findings = data.scanner.scan_commit(&commit_details, &data.github_client, &owner, &repo).await?;
        info!("Found {} secrets in commit {}", findings.len(), commit.sha);
        all_findings.extend(findings);
    }
    
    info!("Total findings: {}", all_findings.len());
    
    if matches!(mode, ScanMode::Running) {
        let state = ScanState {
            repo_url: repo_url.to_string(),
            owner: owner.clone(),
            repo: repo.clone(),
            scan_mode: mode,
            last_scanned_commit_sha: commits.first().map(|c| c.sha.clone()).unwrap_or_default(),
            last_scan_timestamp: Utc::now(),
            total_commits_scanned: commits.len(),
            findings_count: all_findings.len(),
            status: ScanStatus::Completed,
        };
        data.state_manager.save_state(&state).await?;
    }
    
    info!("Generating response with Gemini");
    let response = data.gemini_client.generate_response(
        &all_findings,
        repo_url,
        scan_mode,
        commits.len(),
    ).await?;
    
    info!("Response generated successfully");
    Ok(response)
}

async fn continue_scan(
    repo_url: &str,
    data: &web::Data<AppState>,
) -> anyhow::Result<String> {
    let state = data.state_manager.load_state(repo_url).await?
        .ok_or_else(|| anyhow::anyhow!("No previous scan found for this repository"))?;
    
    let commits = data.github_client.list_commits(
        &state.owner,
        &state.repo,
        Some(&state.last_scan_timestamp.to_rfc3339()),
        data.max_scan_commits as u32,
    ).await?;
    
    if commits.is_empty() {
        return Ok("No new commits to scan since last scan.".to_string());
    }
    
    let mut all_findings = Vec::new();
    
    for commit in &commits {
        let commit_details = data.github_client.get_commit(&state.owner, &state.repo, &commit.sha).await?;
        let findings = data.scanner.scan_commit(&commit_details, &data.github_client, &state.owner, &state.repo).await?;
        all_findings.extend(findings);
    }
    
    let updated_state = ScanState {
        last_scanned_commit_sha: commits.first().map(|c| c.sha.clone()).unwrap_or(state.last_scanned_commit_sha),
        last_scan_timestamp: Utc::now(),
        total_commits_scanned: state.total_commits_scanned + commits.len(),
        findings_count: state.findings_count + all_findings.len(),
        status: ScanStatus::Completed,
        ..state
    };
    
    data.state_manager.save_state(&updated_state).await?;
    
    let response = data.gemini_client.generate_response(
        &all_findings,
        repo_url,
        "running",
        commits.len(),
    ).await?;
    
    Ok(response)
}

async fn get_scan_status(data: &web::Data<AppState>) -> anyhow::Result<String> {
    let states = data.state_manager.list_all_states().await?;
    
    if states.is_empty() {
        return Ok("No active scans found.".to_string());
    }
    
    let mut status_text = String::from("Active scans:\n\n");
    
    for state in states {
        status_text.push_str(&format!(
            "- {}: {} commits scanned, {} findings\n",
            state.repo_url, state.total_commits_scanned, state.findings_count
        ));
    }
    
    Ok(status_text)
}

fn get_help_message() -> String {
    r#"I can help you scan GitHub repositories for exposed secrets!

Commands:
- "scan <repo-url>" - Quick scan (last 100 commits)
- "start running scan <repo-url>" - Begin incremental scanning
- "continue scan" - Continue previous running scan
- "deep scan <repo-url>" - Full history scan
- "status" - Check current scan states

I detect:
- AWS credentials
- API keys (OpenAI, Stripe, SendGrid, etc.)
- Database credentials
- OAuth tokens
- Private keys
- And more!

Just provide a GitHub repository URL and I'll get started!"#.to_string()
}