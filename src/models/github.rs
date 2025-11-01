use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub owner: Owner,
    pub html_url: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pushed_at: DateTime<Utc>,
    pub size: u64,
    pub stargazers_count: u32,
    pub default_branch: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Owner {
    pub login: String,
    pub id: u64,
    pub avatar_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Commit {
    pub sha: String,
    pub commit: CommitDetail,
    pub html_url: String,
    pub author: Option<Author>,
    pub files: Option<Vec<CommitFile>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitDetail {
    pub author: CommitAuthor,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitAuthor {
    pub name: String,
    pub email: String,
    pub date: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Author {
    pub login: String,
    pub id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitFile {
    pub filename: String,
    pub status: String,
    pub additions: u32,
    pub deletions: u32,
    pub changes: u32,
    pub patch: Option<String>,
    pub raw_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileContent {
    pub name: String,
    pub path: String,
    pub sha: String,
    pub size: u64,
    pub content: String,
    pub encoding: String,
}