use crate::models::scan::ScanState;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tokio::sync::RwLock;

pub struct StateManager {
    file_path: String,
    states: RwLock<HashMap<String, ScanState>>,
}

impl StateManager {
    pub fn new(file_path: &str) -> Result<Self> {
        let states = if Path::new(file_path).exists() {
            let content = fs::read_to_string(file_path)?;
            serde_json::from_str(&content)?
        } else {
            HashMap::new()
        };

        Ok(Self {
            file_path: file_path.to_string(),
            states: RwLock::new(states),
        })
    }

    pub async fn load_state(&self, repo_url: &str) -> Result<Option<ScanState>> {
        let states = self.states.read().await;
        Ok(states.get(repo_url).cloned())
    }

    pub async fn save_state(&self, state: &ScanState) -> Result<()> {
        let mut states = self.states.write().await;
        states.insert(state.repo_url.clone(), state.clone());
        self.persist(&states)?;
        Ok(())
    }

    pub async fn list_all_states(&self) -> Result<Vec<ScanState>> {
        let states = self.states.read().await;
        Ok(states.values().cloned().collect())
    }

    fn persist(&self, states: &HashMap<String, ScanState>) -> Result<()> {
        let json = serde_json::to_string_pretty(states)?;
        fs::write(&self.file_path, json)?;
        Ok(())
    }
}