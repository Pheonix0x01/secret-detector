use crate::models::scan::Finding;
use crate::models::github::Commit;
use crate::services::github::GitHubClient;
use crate::utils::patterns::{SECRET_PATTERNS, should_scan_file, is_likely_test_or_example};
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use log::{debug, error};

pub struct SecretScanner;

impl SecretScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan_content(&self, content: &str, file_path: &str, commit_sha: &str, commit_date: chrono::DateTime<chrono::Utc>) -> Vec<Finding> {
        let mut findings = Vec::new();

        for pattern in SECRET_PATTERNS.iter() {
            for (line_num, line) in content.lines().enumerate() {
                if let Some(mat) = pattern.pattern.find(line) {
                    let matched_text = mat.as_str();
                    let redacted_text = Self::redact_secret(matched_text);

                    findings.push(Finding {
                        secret_type: pattern.name.clone(),
                        severity: pattern.severity.clone(),
                        file_path: file_path.to_string(),
                        line_number: line_num + 1,
                        matched_text: redacted_text,
                        commit_sha: commit_sha.to_string(),
                        commit_date,
                        description: pattern.description.clone(),
                        remediation: pattern.remediation.clone(),
                    });
                }
            }
        }

        findings
    }

    pub async fn scan_commit(&self, commit: &Commit, github_client: &GitHubClient, owner: &str, repo: &str) -> Result<Vec<Finding>> {
        let mut all_findings = Vec::new();

        if let Some(files) = &commit.files {
            for file in files {
                if !should_scan_file(&file.filename) {
                    continue;
                }

                if is_likely_test_or_example(&file.filename) {
                    continue;
                }

                if let Some(patch) = &file.patch {
                    let findings = self.scan_content(
                        patch,
                        &file.filename,
                        &commit.sha,
                        commit.commit.author.date,
                    );
                    all_findings.extend(findings);
                }

                if file.status == "added" || file.status == "modified" {
                    match github_client.get_file_content(owner, repo, &file.filename, &commit.sha).await {
                        Ok(file_content) => {
                            let cleaned_content = file_content.content.replace("\n", "").replace("\r", "");
                            debug!("Original content length: {}, cleaned: {}", file_content.content.len(), cleaned_content.len());
                            
                            match general_purpose::STANDARD.decode(&cleaned_content) {
                                Ok(decoded) => {
                                    let content = String::from_utf8_lossy(&decoded);
                                    let findings = self.scan_content(
                                        &content,
                                        &file.filename,
                                        &commit.sha,
                                        commit.commit.author.date,
                                    );
                                    all_findings.extend(findings);
                                }
                                Err(e) => {
                                    error!("Failed to decode base64 content for {}: {}", file.filename, e);
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            debug!("Could not fetch file content for {}: {}", file.filename, e);
                            continue;
                        }
                    }
                }
            }
        }

        Ok(all_findings)
    }

    fn redact_secret(secret: &str) -> String {
        let len = secret.len();
        if len <= 8 {
            return "*".repeat(len);
        }
        
        let visible_chars = 4;
        let prefix = &secret[..visible_chars];
        let suffix = &secret[len - visible_chars..];
        format!("{}...{}", prefix, suffix)
    }
}