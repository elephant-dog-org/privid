use std::sync::Arc;
use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::blockchain::types::VerificationType;
use crate::db::{Database, PlatformLink};

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

/// Persistent registry backed by SQLite via Database.
pub struct Registry {
    db: Arc<Database>,
}

impl Registry {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Register a user. Overwrites any existing registration for this user_id.
    pub async fn register(&self, entry: RegistrationEntry) {
        if let Err(e) = self.db.register_identity(&entry).await {
            log::warn!("Failed to register identity: {}", e);
        }
    }

    /// Remove a user's registration.
    pub async fn deregister(&self, user_id: u64) -> bool {
        match self.db.deregister_identity(user_id).await {
            Ok(removed) => removed,
            Err(e) => {
                log::warn!("Failed to deregister identity: {}", e);
                false
            }
        }
    }

    /// Look up a registration by Telegram user ID.
    pub async fn lookup_by_user_id(&self, user_id: u64) -> Option<RegistrationEntry> {
        match self.db.lookup_by_user_id(user_id).await {
            Ok(entry) => entry,
            Err(e) => {
                log::warn!("Failed to lookup by user_id: {}", e);
                None
            }
        }
    }

    /// Look up a registration by Telegram username (case-insensitive).
    pub async fn lookup_by_username(&self, username: &str) -> Option<RegistrationEntry> {
        match self.db.lookup_by_username(username).await {
            Ok(entry) => entry,
            Err(e) => {
                log::warn!("Failed to lookup by username: {}", e);
                None
            }
        }
    }

    /// Look up a registration by Twitter handle (via platform links).
    pub async fn lookup_by_twitter_handle(&self, handle: &str) -> Option<RegistrationEntry> {
        match self.db.lookup_by_twitter_handle(handle).await {
            Ok(entry) => entry,
            Err(e) => {
                log::warn!("Failed to lookup by twitter handle: {}", e);
                None
            }
        }
    }

    /// Look up a registration by wallet address.
    pub async fn lookup_by_wallet(&self, wallet: &str) -> Option<RegistrationEntry> {
        match self.db.lookup_by_wallet(wallet).await {
            Ok(entry) => entry,
            Err(e) => {
                log::warn!("Failed to lookup by wallet: {}", e);
                None
            }
        }
    }

    /// Link a platform handle to a user.
    pub async fn link_platform(
        &self,
        user_id: u64,
        platform: &str,
        handle: &str,
        verified_via_ens: bool,
    ) -> Result<()> {
        self.db
            .link_platform(user_id, platform, handle, verified_via_ens)
            .await
    }

    /// Get all platform links for a user.
    pub async fn get_platform_links(&self, user_id: u64) -> Result<Vec<PlatformLink>> {
        self.db.get_platform_links(user_id).await
    }

    /// Get total number of registrations.
    pub async fn count(&self) -> usize {
        match self.db.count().await {
            Ok(n) => n,
            Err(e) => {
                log::warn!("Failed to count registrations: {}", e);
                0
            }
        }
    }

    /// No-op for backward compatibility. SQLite handles persistence automatically.
    pub async fn save(&self) -> Result<()> {
        // SQLite writes are immediate; nothing to flush.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let db = Arc::new(Database::new_in_memory().await.unwrap());
        let registry = Registry::new(db);

        let entry = make_entry(123, "alice", "alice.eth");
        registry.register(entry).await;

        let found = registry.lookup_by_user_id(123).await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().ens_name, "alice.eth");
    }

    #[tokio::test]
    async fn test_lookup_by_username() {
        let db = Arc::new(Database::new_in_memory().await.unwrap());
        let registry = Registry::new(db);

        registry
            .register(make_entry(123, "Alice", "alice.eth"))
            .await;

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
        let db = Arc::new(Database::new_in_memory().await.unwrap());
        let registry = Registry::new(db);

        registry
            .register(make_entry(123, "alice", "alice.eth"))
            .await;
        assert_eq!(registry.count().await, 1);

        let removed = registry.deregister(123).await;
        assert!(removed);
        assert_eq!(registry.count().await, 0);

        // Username index also cleared
        assert!(registry.lookup_by_username("alice").await.is_none());
    }

    #[tokio::test]
    async fn test_reregister_updates_username_index() {
        let db = Arc::new(Database::new_in_memory().await.unwrap());
        let registry = Registry::new(db);

        registry
            .register(make_entry(123, "oldname", "alice.eth"))
            .await;
        registry
            .register(make_entry(123, "newname", "alice.eth"))
            .await;

        assert!(registry.lookup_by_username("oldname").await.is_none());
        assert!(registry.lookup_by_username("newname").await.is_some());
        assert_eq!(registry.count().await, 1);
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
