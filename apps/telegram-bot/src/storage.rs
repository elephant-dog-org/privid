use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use log::{info, warn, error};

use crate::state::UserSession;

#[derive(Debug, Serialize, Deserialize)]
struct StorageData {
    sessions: HashMap<u64, UserSession>,
    version: String,
}

#[derive(Debug)]
pub struct FileStorage {
    file_path: PathBuf,
}

impl FileStorage {
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }

    pub async fn load_sessions(&self) -> Result<HashMap<u64, UserSession>> {
        if !self.file_path.exists() {
            info!("Storage file does not exist, starting with empty sessions");
            return Ok(HashMap::new());
        }

        match fs::read_to_string(&self.file_path).await {
            Ok(content) => {
                match serde_json::from_str::<StorageData>(&content) {
                    Ok(data) => {
                        info!("Loaded {} sessions from storage", data.sessions.len());
                        Ok(data.sessions)
                    }
                    Err(e) => {
                        warn!("Failed to parse storage file: {}, starting fresh", e);
                        Ok(HashMap::new())
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read storage file: {}, starting fresh", e);
                Ok(HashMap::new())
            }
        }
    }

    pub async fn save_sessions(&self, sessions: &HashMap<u64, UserSession>) -> Result<()> {
        let data = StorageData {
            sessions: sessions.clone(),
            version: "1.0".to_string(),
        };

        // Create directory if it doesn't exist
        if let Some(parent) = self.file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await?;
            }
        }

        let json = serde_json::to_string_pretty(&data)?;
        
        match fs::write(&self.file_path, json).await {
            Ok(_) => {
                info!("Saved {} sessions to storage", sessions.len());
                Ok(())
            }
            Err(e) => {
                error!("Failed to save sessions: {}", e);
                Err(e.into())
            }
        }
    }

    pub async fn backup_sessions(&self, sessions: &HashMap<u64, UserSession>) -> Result<()> {
        let backup_path = self.file_path.with_extension("backup");
        let data = StorageData {
            sessions: sessions.clone(),
            version: "1.0".to_string(),
        };

        let json = serde_json::to_string_pretty(&data)?;
        fs::write(backup_path, json).await?;
        
        info!("Created backup of {} sessions", sessions.len());
        Ok(())
    }
} 