use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub verified: bool,
    pub timestamp: String,
    pub proof: String,
    pub badge: String,
    /// The type of verification (e.g., "KYC (Know Your Customer)")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_type: Option<String>,
    /// The wallet address that was verified
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
    /// Unix timestamp when the SBT expires
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sbt_expiry: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationState {
    NotStarted,
    InProgress { verification_id: String },
    Completed { verification_result: VerificationResult },
    Failed { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub user_id: u64,
    pub verification_state: VerificationState,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl UserSession {
    pub fn new(user_id: u64) -> Self {
        let now = chrono::Utc::now();
        Self {
            user_id,
            verification_state: VerificationState::NotStarted,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_verification_state(&mut self, state: VerificationState) {
        self.verification_state = state;
        self.updated_at = chrono::Utc::now();
    }

    pub fn generate_mock_verification_result() -> VerificationResult {
        let badge_strings = ["Identity Verified via Holonym",
            "Verified Person", 
            "Authenticated by ZK Proof"];
        
        let random_index = (rand::random::<f64>() * badge_strings.len() as f64) as usize;
        let random_proof_id = rand::random::<u64>().to_string();
        
        VerificationResult {
            verified: true,
            timestamp: chrono::Utc::now().to_rfc3339(),
            proof: format!("mock-zk-proof-{}", random_proof_id),
            badge: badge_strings[random_index].to_string(),
            verification_type: None,
            wallet_address: None,
            sbt_expiry: None,
        }
    }
}

#[derive(Debug)]
pub struct BotState {
    sessions: Arc<RwLock<HashMap<u64, UserSession>>>,
    storage: Option<Arc<crate::storage::FileStorage>>,
}

impl Default for BotState {
    fn default() -> Self {
        Self::new()
    }
}

impl BotState {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            storage: None,
        }
    }

    pub fn with_storage(storage: crate::storage::FileStorage) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            storage: Some(Arc::new(storage)),
        }
    }

    pub async fn get_or_create_session(&self, user_id: u64) -> UserSession {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get(&user_id) {
            session.clone()
        } else {
            let session = UserSession::new(user_id);
            sessions.insert(user_id, session.clone());
            session
        }
    }

    pub async fn update_session(&self, user_id: u64, session: UserSession) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(user_id, session);
    }

    pub async fn get_session(&self, user_id: u64) -> Option<UserSession> {
        let sessions = self.sessions.read().await;
        sessions.get(&user_id).cloned()
    }

    pub async fn load_from_storage(&self) -> Result<()> {
        if let Some(storage) = &self.storage {
            let sessions_data = storage.load_sessions().await?;
            let mut sessions = self.sessions.write().await;
            *sessions = sessions_data;
        }
        Ok(())
    }

    pub async fn save_to_storage(&self) -> Result<()> {
        if let Some(storage) = &self.storage {
            let sessions = self.sessions.read().await;
            storage.save_sessions(&sessions).await?;
        }
        Ok(())
    }

    pub async fn backup_to_storage(&self) -> Result<()> {
        if let Some(storage) = &self.storage {
            let sessions = self.sessions.read().await;
            storage.backup_sessions(&sessions).await?;
        }
        Ok(())
    }
} 