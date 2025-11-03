use crate::models::a2a::{A2ARequest, A2AResponse, TelexMessage, MessagePart};
use crate::models::scan::{ScanState, ScanStatus};
use crate::services::github::GitHubClient;
use crate::services::scanner::SecretScanner;
use crate::services::gemini::GeminiClient;
use crate::services::state::StateManager;
use actix_web::{web, HttpResponse, HttpRequest, Result as ActixResult};
use chrono::Utc;
use uuid::Uuid;
use std::sync::Arc;
use log::{info, error};

pub struct AppState {
    pub gemini_client: Arc<GeminiClient>,
    pub github_client: Arc<GitHubClient>,
    pub state_manager: Arc<StateManager>,
    pub scanner: Arc<SecretScanner>,
    pub max_scan_commits: u32,
}

pub async fn handle_a2a_request(
    _req: HttpRequest,
    body: web::Bytes,
    data: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let body_str = String::from_utf8_lossy(&body);
    info!("Received request body: {}", body_str);
    
    let a2a_request: A2ARequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to parse A2A request: {}. Body was: {}", e, body_str);
            return Ok(HttpResponse::BadRequest().json(
                A2AResponse::error("unknown".to_string(), -32700, format!("Parse error: {}", e))
            ));
        }
    };
    
    if a2a_request.jsonrpc != "2.0" {
        return Ok(HttpResponse::BadRequest().json(
            A2AResponse::error(a2a_request.id, -32602, "Invalid jsonrpc version".to_string())
        ));
    }
    
    let request_id = a2a_request.id.clone();
    let task_id = Uuid::new_v4().to_string();
    let context_id = Uuid::new_v4().to_string();
    
    match process_request(&a2a_request, &data).await {
        Ok(response_text) => {
            let response = A2AResponse::success(
                request_id,
                task_id,
                context_id,
                response_text,
                &a2a_request.params.message,
            );
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("Request processing failed: {}", e);
            Ok(HttpResponse::Ok().json(
                A2AResponse::error(request_id, -32603, format!("Internal error: {}", e))
            ))
        }
    }
}

async fn process_request(
    req: &A2ARequest,
    data: &web::Data<AppState>,
) -> anyhow::Result<String> {
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
    
    Ok(response_text)
}

fn extract_user_message(message: &TelexMessage) -> anyhow::Result<String> {
    info!("Extracting message from {} parts", message.parts.len());
    
    for (i, part) in message.parts.iter().enumerate() {
        match part {
            MessagePart::Text { text, .. } => {
                info!("Part {}: Text = '{}'", i, text);
                if !text.trim().is_empty() {
                    return Ok(text.clone());
                }
            }
            MessagePart::Data { data, .. } => {
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
                            info!("Found message in data: '{}'", text);
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
    
    info!("Listing commits for {}/{}", owner, repo);
    let commits = data.github_client.list_commits(
        &owner,
        &repo,
        None,
        data.max_scan_commits,  // Use from AppState
    ).await?;
    
    info!("Found {} commits to scan", commits.len());
    
    let mut all_findings = Vec::new();
    
    for (idx, commit) in commits.iter().enumerate() {
        info!("Scanning commit {}/{}: {}", idx + 1, commits.len(), commit.sha);
        let commit_details = data.github_client.get_commit(&owner, &repo, &commit.sha).await?;
        let findings = data.scanner.scan_commit(&commit_details, &data.github_client, &owner, &repo).await?;  // Use from AppState
        info!("Found {} secrets in commit {}", findings.len(), commit.sha);
        all_findings.extend(findings);
    }
    
    info!("Total findings: {}", all_findings.len());
    
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
        data.max_scan_commits,
    ).await?;
    
    if commits.is_empty() {
        return Ok("No new commits to scan since last scan.".to_string());
    }
    
    let mut all_findings = Vec::new();
    
    for commit in &commits {
        let commit_details = data.github_client.get_commit(&state.owner, &state.repo, &commit.sha).await?;
        let findings = data.scanner.scan_commit(&commit_details, &data.github_client, &state.owner, &state.repo).await?;  // Use from AppState
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