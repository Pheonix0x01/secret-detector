use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScanState {
    pub repo_url: String,
    pub owner: String,
    pub repo: String,
    pub scan_mode: ScanMode,
    pub last_scanned_commit_sha: String,
    pub last_scan_timestamp: DateTime<Utc>,
    pub total_commits_scanned: usize,
    pub findings_count: usize,
    pub status: ScanStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ScanMode {
    Quick,
    Running,
    Deep,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ScanStatus {
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Finding {
    pub secret_type: String,
    pub severity: Severity,
    pub file_path: String,
    pub line_number: usize,
    pub matched_text: String,
    pub commit_sha: String,
    pub commit_date: DateTime<Utc>,
    pub description: String,
    pub remediation: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}