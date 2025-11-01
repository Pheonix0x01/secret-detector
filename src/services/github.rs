use crate::models::github::{Repository, Commit, FileContent};
use anyhow::{Result, anyhow};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, AUTHORIZATION, ACCEPT};
use regex::Regex;
use lazy_static::lazy_static;
use log::{error, debug};

lazy_static! {
    static ref GITHUB_URL_REGEX: Regex = Regex::new(
        r"github\.com/([^/]+)/([^/\s]+)"
    ).unwrap();
}

pub struct GitHubClient {
    client: reqwest::Client,
    base_url: String,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("github-secret-scanner"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github+json"));
        
        if let Some(t) = token {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", t))?
            );
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            base_url: "https://api.github.com".to_string(),
        })
    }

    pub fn parse_repo_url(url: &str) -> Result<(String, String)> {
        let caps = GITHUB_URL_REGEX.captures(url)
            .ok_or_else(|| anyhow!("Invalid GitHub URL format"))?;
        
        let owner = caps.get(1)
            .ok_or_else(|| anyhow!("Could not extract owner"))?
            .as_str()
            .to_string();
        
        let repo = caps.get(2)
            .ok_or_else(|| anyhow!("Could not extract repo"))?
            .as_str()
            .trim_end_matches(".git")
            .to_string();

        Ok((owner, repo))
    }

    pub async fn get_repository(&self, owner: &str, repo: &str) -> Result<Repository> {
        let url = format!("{}/repos/{}/{}", self.base_url, owner, repo);
        debug!("GET {}", url);
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("GitHub API error {}: {}", status, error_text);
            return Err(anyhow!("GitHub API error: {}", status));
        }

        let repository: Repository = response.json().await?;
        Ok(repository)
    }

    pub async fn list_commits(
        &self,
        owner: &str,
        repo: &str,
        since: Option<&str>,
        per_page: u32,
    ) -> Result<Vec<Commit>> {
        let mut url = format!(
            "{}/repos/{}/{}/commits?per_page={}",
            self.base_url, owner, repo, per_page
        );

        if let Some(since_date) = since {
            url.push_str(&format!("&since={}", since_date));
        }

        debug!("GET {}", url);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("GitHub API error {}: {}", status, error_text);
            return Err(anyhow!("GitHub API error: {}", status));
        }

        let commits: Vec<Commit> = response.json().await?;
        Ok(commits)
    }

    pub async fn get_commit(&self, owner: &str, repo: &str, sha: &str) -> Result<Commit> {
        let url = format!("{}/repos/{}/{}/commits/{}", self.base_url, owner, repo, sha);
        debug!("GET {}", url);
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("GitHub API error {}: {}", status, error_text);
            return Err(anyhow!("GitHub API error: {}", status));
        }

        let response_text = response.text().await?;
        debug!("Raw response length: {} bytes", response_text.len());
        
        match serde_json::from_str::<Commit>(&response_text) {
            Ok(commit) => Ok(commit),
            Err(e) => {
                error!("Failed to parse commit JSON: {}", e);
                error!("Response preview: {}", &response_text[..response_text.len().min(500)]);
                Err(anyhow!("Failed to parse GitHub commit response: {}", e))
            }
        }
    }

    pub async fn get_file_content(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
        ref_sha: &str,
    ) -> Result<FileContent> {
        let url = format!(
            "{}/repos/{}/{}/contents/{}?ref={}",
            self.base_url, owner, repo, path, ref_sha
        );
        debug!("GET {}", url);
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("GitHub API error {}: {}", status, error_text);
            return Err(anyhow!("GitHub API error: {}", status));
        }

        let content: FileContent = response.json().await?;
        Ok(content)
    }
}