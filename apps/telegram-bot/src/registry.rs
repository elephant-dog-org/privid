use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::sync::RwLock;
use log::info;
use anyhow::Result;

use crate::blockchain::types::VerificationType;

/// A registered user linking their Telegram identity to an ENS name and wallet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationEntry {
    pub telegram_user_id: u64,
    pub telegram_username: String,
    pub ens_name: String,
    pub wallet_address: String,
    pub verified_sbt_types: Vec<VerificationType>,
    pub registered_at: chrono::DateTime<chrono::Utc>,
    pub last_verified: chrono::DateTime<chrono::Utc>,
}

impl RegistrationEntry {
    /// Short summary of verified SBT types (e.g. "KYC, Phone").
    pub fn sbt_summary(&self) -> String {
        if self.verified_sbt_types.is_empty() {
            "None".to_string()
        } else {
            self.verified_sbt_types
                .iter()
                .map(|v| v.short_name())
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

/// Persistent registry mapping Telegram users to ENS names and wallets.
///
/// Maintains two indexes:
/// - `by_user_id`: telegram_user_id → RegistrationEntry
/// - `by_username`: lowercase telegram_username → telegram_user_id
pub struct Registry {
    by_user_id: RwLock<HashMap<u64, RegistrationEntry>>,
    by_username: RwLock<HashMap<String, u64>>,
    file_path: PathBuf,
}

impl Registry {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            by_user_id: RwLock::new(HashMap::new()),
            by_username: RwLock::new(HashMap::new()),
            file_path,
        }
    }

    /// Load registry from disk.
    pub async fn load(&self) -> Result<()> {
        if !self.file_path.exists() {
            info!("Registry file does not exist, starting empty");
            return Ok(());
        }

        let content = fs::read_to_string(&self.file_path).await?;
        let entries: Vec<RegistrationEntry> = serde_json::from_str(&content)?;

        let mut by_id = self.by_user_id.write().await;
        let mut by_name = self.by_username.write().await;

        for entry in entries {
            by_name.insert(entry.telegram_username.to_lowercase(), entry.telegram_user_id);
            by_id.insert(entry.telegram_user_id, entry);
        }

        info!("Loaded {} registrations from disk", by_id.len());
        Ok(())
    }

    /// Persist registry to disk.
    pub async fn save(&self) -> Result<()> {
        if let Some(parent) = self.file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await?;
            }
        }

        let by_id = self.by_user_id.read().await;
        let entries: Vec<&RegistrationEntry> = by_id.values().collect();
        let json = serde_json::to_string_pretty(&entries)?;
        fs::write(&self.file_path, json).await?;
        info!("Saved {} registrations to disk", entries.len());
        Ok(())
    }

    /// Register a user. Overwrites any existing registration for this user_id.
    pub async fn register(&self, entry: RegistrationEntry) {
        let user_id = entry.telegram_user_id;
        let username = entry.telegram_username.to_lowercase();

        // Remove old username mapping if the user was previously registered
        {
            let by_id = self.by_user_id.read().await;
            if let Some(old) = by_id.get(&user_id) {
                let mut by_name = self.by_username.write().await;
                by_name.remove(&old.telegram_username.to_lowercase());
            }
        }

        let mut by_name = self.by_username.write().await;
        by_name.insert(username, user_id);
        drop(by_name);

        let mut by_id = self.by_user_id.write().await;
        by_id.insert(user_id, entry);
    }

    /// Remove a user's registration.
    pub async fn deregister(&self, user_id: u64) -> bool {
        let mut by_id = self.by_user_id.write().await;
        if let Some(entry) = by_id.remove(&user_id) {
            let mut by_name = self.by_username.write().await;
            by_name.remove(&entry.telegram_username.to_lowercase());
            true
        } else {
            false
        }
    }

    /// Look up a registration by Telegram user ID.
    pub async fn lookup_by_user_id(&self, user_id: u64) -> Option<RegistrationEntry> {
        let by_id = self.by_user_id.read().await;
        by_id.get(&user_id).cloned()
    }

    /// Look up a registration by Telegram username (case-insensitive).
    pub async fn lookup_by_username(&self, username: &str) -> Option<RegistrationEntry> {
        let clean = username.strip_prefix('@').unwrap_or(username).to_lowercase();
        let by_name = self.by_username.read().await;
        let user_id = by_name.get(&clean)?;
        let by_id = self.by_user_id.read().await;
        by_id.get(user_id).cloned()
    }

    /// Get total number of registrations.
    pub async fn count(&self) -> usize {
        self.by_user_id.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_entry(user_id: u64, username: &str, ens: &str) -> RegistrationEntry {
        let now = chrono::Utc::now();
        RegistrationEntry {
            telegram_user_id: user_id,
            telegram_username: username.to_string(),
            ens_name: ens.to_string(),
            wallet_address: "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".to_string(),
            verified_sbt_types: vec![VerificationType::Kyc, VerificationType::Phone],
            registered_at: now,
            last_verified: now,
        }
    }

    #[tokio::test]
    async fn test_register_and_lookup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let registry = Registry::new(path);

        let entry = make_entry(123, "alice", "alice.eth");
        registry.register(entry).await;

        let found = registry.lookup_by_user_id(123).await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().ens_name, "alice.eth");
    }

    #[tokio::test]
    async fn test_lookup_by_username() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let registry = Registry::new(path);

        registry.register(make_entry(123, "Alice", "alice.eth")).await;

        // Case-insensitive lookup
        let found = registry.lookup_by_username("alice").await;
        assert!(found.is_some());

        // With @ prefix
        let found = registry.lookup_by_username("@Alice").await;
        assert!(found.is_some());

        // Not found
        let not_found = registry.lookup_by_username("bob").await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_deregister() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let registry = Registry::new(path);

        registry.register(make_entry(123, "alice", "alice.eth")).await;
        assert_eq!(registry.count().await, 1);

        let removed = registry.deregister(123).await;
        assert!(removed);
        assert_eq!(registry.count().await, 0);

        // Username index also cleared
        assert!(registry.lookup_by_username("alice").await.is_none());
    }

    #[tokio::test]
    async fn test_reregister_updates_username_index() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let registry = Registry::new(path);

        registry.register(make_entry(123, "oldname", "alice.eth")).await;
        registry.register(make_entry(123, "newname", "alice.eth")).await;

        assert!(registry.lookup_by_username("oldname").await.is_none());
        assert!(registry.lookup_by_username("newname").await.is_some());
        assert_eq!(registry.count().await, 1);
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");

        // Save
        {
            let registry = Registry::new(path.clone());
            registry.register(make_entry(1, "alice", "alice.eth")).await;
            registry.register(make_entry(2, "bob", "bob.eth")).await;
            registry.save().await.unwrap();
        }

        // Load in fresh instance
        {
            let registry = Registry::new(path);
            registry.load().await.unwrap();
            assert_eq!(registry.count().await, 2);
            assert!(registry.lookup_by_username("alice").await.is_some());
            assert!(registry.lookup_by_username("bob").await.is_some());
        }
    }

    #[tokio::test]
    async fn test_sbt_summary() {
        let entry = make_entry(1, "alice", "alice.eth");
        assert_eq!(entry.sbt_summary(), "KYC, Phone");

        let mut empty = make_entry(2, "bob", "bob.eth");
        empty.verified_sbt_types = vec![];
        assert_eq!(empty.sbt_summary(), "None");
    }
}
