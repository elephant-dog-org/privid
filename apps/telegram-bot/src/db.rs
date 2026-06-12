use anyhow::Result;
use log::info;
use rusqlite::params;
use rusqlite::OptionalExtension;
use tokio_rusqlite::Connection;

use crate::blockchain::types::VerificationType;
use crate::registry::RegistrationEntry;

/// A platform link connecting a Telegram user to an external platform handle.
#[derive(Debug, Clone)]
pub struct PlatformLink {
    pub platform: String,
    pub handle: String,
    pub verified_via_ens: bool,
}

/// A verification challenge: a single-use token a "challenger" mints and sends to a
/// counterparty, who claims it by opening the bot via a deep link. The claim result
/// (pass/fail + assurance tier) is recorded and delivered back to the challenger.
#[derive(Debug, Clone)]
pub struct Challenge {
    pub token: String,
    pub challenger_user_id: i64,
    pub label: Option<String>,
    /// "pending" | "claimed". Expiry is derived from `expires_at`, not stored as a status.
    pub status: String,
    pub claimant_user_id: Option<i64>,
    pub claimant_username: Option<String>,
    pub claimant_tier: Option<String>,
    pub passed: Option<bool>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub claimed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Challenge {
    /// True if the token is still claimable (pending and not past expiry).
    pub fn is_open(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        self.status == "pending" && self.expires_at > now
    }

    /// Human-readable status for display, factoring in expiry.
    pub fn display_status(&self, now: chrono::DateTime<chrono::Utc>) -> &'static str {
        match self.status.as_str() {
            "claimed" => match self.passed {
                Some(true) => "passed",
                Some(false) => "failed",
                None => "claimed",
            },
            _ if self.expires_at <= now => "expired",
            _ => "pending",
        }
    }

    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        let parse = |s: String| -> chrono::DateTime<chrono::Utc> {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|d| d.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::DateTime::from_timestamp(0, 0).unwrap())
        };
        let claimed_at: Option<String> = row.get(9)?;
        Ok(Self {
            token: row.get(0)?,
            challenger_user_id: row.get(1)?,
            label: row.get(2)?,
            status: row.get(3)?,
            claimant_user_id: row.get(4)?,
            claimant_username: row.get(5)?,
            claimant_tier: row.get(6)?,
            passed: row.get::<_, Option<i32>>(7)?.map(|v| v != 0),
            created_at: parse(row.get(8)?),
            expires_at: parse(row.get(10)?),
            claimed_at: claimed_at.map(parse),
        })
    }
}

/// Standard SELECT column list for challenges (order must match `Challenge::from_row`).
const CHALLENGE_COLUMNS: &str = "token, challenger_user_id, label, status, \
     claimant_user_id, claimant_username, claimant_tier, passed, created_at, \
     claimed_at, expires_at";

/// Raw column data extracted from a rusqlite Row.
/// Used to shuttle data out of the synchronous `conn.call` closure
/// so that fallible parsing happens in async context.
#[derive(Debug, Clone)]
struct RawIdentityRow {
    telegram_user_id: i64,
    telegram_username: String,
    ens_name: String,
    wallet_address: String,
    verified_sbt_types_json: String,
    registered_at: String,
    last_verified: String,
}

impl RawIdentityRow {
    /// Extract from a rusqlite Row. Column order must match the SELECT.
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            telegram_user_id: row.get(0)?,
            telegram_username: row.get(1)?,
            ens_name: row.get(2)?,
            wallet_address: row.get(3)?,
            verified_sbt_types_json: row.get(4)?,
            registered_at: row.get(5)?,
            last_verified: row.get(6)?,
        })
    }

    /// Convert into a RegistrationEntry, parsing JSON and timestamps.
    fn into_entry(self) -> Result<RegistrationEntry> {
        let sbt_short_names: Vec<String> =
            serde_json::from_str(&self.verified_sbt_types_json)?;
        let verified_sbt_types: Vec<VerificationType> = sbt_short_names
            .iter()
            .filter_map(|name| match name.as_str() {
                "KYC" => Some(VerificationType::Kyc),
                "Phone" => Some(VerificationType::Phone),
                "Passport" => Some(VerificationType::Passport),
                "Clean Hands" => Some(VerificationType::CleanHands),
                "Biometrics" => Some(VerificationType::Biometrics),
                _ => None,
            })
            .collect();

        let registered_at = chrono::DateTime::parse_from_rfc3339(&self.registered_at)?
            .with_timezone(&chrono::Utc);
        let last_verified = chrono::DateTime::parse_from_rfc3339(&self.last_verified)?
            .with_timezone(&chrono::Utc);

        Ok(RegistrationEntry {
            telegram_user_id: self.telegram_user_id as u64,
            telegram_username: self.telegram_username,
            ens_name: self.ens_name,
            wallet_address: self.wallet_address,
            verified_sbt_types,
            registered_at,
            last_verified,
        })
    }
}

/// Standard SELECT column list for identities.
const IDENTITY_COLUMNS: &str =
    "telegram_user_id, telegram_username, ens_name, wallet_address, \
     verified_sbt_types, registered_at, last_verified";

/// SQLite-backed database for PrivID identity data.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) a database at the given file path.
    pub async fn new(path: &str) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        let conn = Connection::open(path).await?;
        Self::initialize_schema(&conn).await?;
        Ok(Self { conn })
    }

    /// Create an in-memory database (for testing).
    pub async fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().await?;
        Self::initialize_schema(&conn).await?;
        Ok(Self { conn })
    }

    /// Create all tables and indexes.
    async fn initialize_schema(conn: &Connection) -> Result<()> {
        conn.call(|conn| {
            conn.execute_batch(
                "PRAGMA journal_mode=WAL;
                 PRAGMA foreign_keys = ON;

                 CREATE TABLE IF NOT EXISTS identities (
                     telegram_user_id INTEGER PRIMARY KEY,
                     telegram_username TEXT NOT NULL,
                     ens_name TEXT NOT NULL,
                     wallet_address TEXT NOT NULL,
                     verified_sbt_types TEXT NOT NULL,
                     registered_at TEXT NOT NULL,
                     last_verified TEXT NOT NULL
                 );

                 CREATE INDEX IF NOT EXISTS idx_wallet ON identities(wallet_address);
                 CREATE INDEX IF NOT EXISTS idx_ens ON identities(ens_name);
                 CREATE INDEX IF NOT EXISTS idx_username ON identities(telegram_username COLLATE NOCASE);

                 CREATE TABLE IF NOT EXISTS platform_links (
                     id INTEGER PRIMARY KEY AUTOINCREMENT,
                     telegram_user_id INTEGER NOT NULL,
                     platform TEXT NOT NULL,
                     handle TEXT NOT NULL,
                     verified_via_ens INTEGER NOT NULL DEFAULT 0,
                     FOREIGN KEY (telegram_user_id) REFERENCES identities(telegram_user_id) ON DELETE CASCADE,
                     UNIQUE(telegram_user_id, platform)
                 );

                 CREATE INDEX IF NOT EXISTS idx_platform_handle ON platform_links(platform, handle);

                 CREATE TABLE IF NOT EXISTS challenges (
                     token TEXT PRIMARY KEY,
                     challenger_user_id INTEGER NOT NULL,
                     label TEXT,
                     status TEXT NOT NULL DEFAULT 'pending',
                     claimant_user_id INTEGER,
                     claimant_username TEXT,
                     claimant_tier TEXT,
                     passed INTEGER,
                     created_at TEXT NOT NULL,
                     claimed_at TEXT,
                     expires_at TEXT NOT NULL
                 );

                 CREATE INDEX IF NOT EXISTS idx_challenge_challenger ON challenges(challenger_user_id);",
            )?;
            Ok(())
        })
        .await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Identity operations
    // -----------------------------------------------------------------------

    /// Insert or replace an identity registration.
    pub async fn register_identity(&self, entry: &RegistrationEntry) -> Result<()> {
        let entry = entry.clone();
        self.conn
            .call(move |conn| {
                let sbt_json = serde_json::to_string(
                    &entry
                        .verified_sbt_types
                        .iter()
                        .map(|v| v.short_name())
                        .collect::<Vec<_>>(),
                )
                .unwrap_or_else(|_| "[]".to_string());

                conn.execute(
                    "INSERT OR REPLACE INTO identities
                     (telegram_user_id, telegram_username, ens_name, wallet_address,
                      verified_sbt_types, registered_at, last_verified)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        entry.telegram_user_id as i64,
                        entry.telegram_username,
                        entry.ens_name,
                        entry.wallet_address,
                        sbt_json,
                        entry.registered_at.to_rfc3339(),
                        entry.last_verified.to_rfc3339(),
                    ],
                )?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    /// Remove an identity by Telegram user ID. Returns true if a row was deleted.
    pub async fn deregister_identity(&self, telegram_user_id: u64) -> Result<bool> {
        let deleted = self
            .conn
            .call(move |conn| {
                let count = conn.execute(
                    "DELETE FROM identities WHERE telegram_user_id = ?1",
                    params![telegram_user_id as i64],
                )?;
                Ok(count > 0)
            })
            .await?;
        Ok(deleted)
    }

    /// Look up an identity by Telegram user ID.
    pub async fn lookup_by_user_id(
        &self,
        user_id: u64,
    ) -> Result<Option<RegistrationEntry>> {
        let sql = format!(
            "SELECT {} FROM identities WHERE telegram_user_id = ?1",
            IDENTITY_COLUMNS
        );
        let raw = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare(&sql)?;
                let row = stmt
                    .query_row(params![user_id as i64], RawIdentityRow::from_row)
                    .optional()?;
                Ok(row)
            })
            .await?;
        match raw {
            Some(r) => Ok(Some(r.into_entry()?)),
            None => Ok(None),
        }
    }

    /// Look up an identity by Telegram username (case-insensitive, strips leading @).
    pub async fn lookup_by_username(
        &self,
        username: &str,
    ) -> Result<Option<RegistrationEntry>> {
        let clean = username
            .strip_prefix('@')
            .unwrap_or(username)
            .to_lowercase();
        let sql = format!(
            "SELECT {} FROM identities WHERE telegram_username COLLATE NOCASE = ?1",
            IDENTITY_COLUMNS
        );
        let raw = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare(&sql)?;
                let row = stmt
                    .query_row(params![clean], RawIdentityRow::from_row)
                    .optional()?;
                Ok(row)
            })
            .await?;
        match raw {
            Some(r) => Ok(Some(r.into_entry()?)),
            None => Ok(None),
        }
    }

    /// Look up an identity by Twitter handle (via platform_links table).
    pub async fn lookup_by_twitter_handle(
        &self,
        handle: &str,
    ) -> Result<Option<RegistrationEntry>> {
        let handle = handle
            .strip_prefix('@')
            .unwrap_or(handle)
            .to_lowercase();
        let cols = IDENTITY_COLUMNS
            .split(", ")
            .map(|c| format!("i.{}", c.trim()))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT {} FROM identities i
             JOIN platform_links p ON i.telegram_user_id = p.telegram_user_id
             WHERE p.platform = 'twitter' AND LOWER(p.handle) = ?1",
            cols
        );
        let raw = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare(&sql)?;
                let row = stmt
                    .query_row(params![handle], RawIdentityRow::from_row)
                    .optional()?;
                Ok(row)
            })
            .await?;
        match raw {
            Some(r) => Ok(Some(r.into_entry()?)),
            None => Ok(None),
        }
    }

    /// Look up an identity by wallet address (case-insensitive).
    pub async fn lookup_by_wallet(
        &self,
        wallet: &str,
    ) -> Result<Option<RegistrationEntry>> {
        let wallet = wallet.to_lowercase();
        let sql = format!(
            "SELECT {} FROM identities WHERE LOWER(wallet_address) = ?1",
            IDENTITY_COLUMNS
        );
        let raw = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare(&sql)?;
                let row = stmt
                    .query_row(params![wallet], RawIdentityRow::from_row)
                    .optional()?;
                Ok(row)
            })
            .await?;
        match raw {
            Some(r) => Ok(Some(r.into_entry()?)),
            None => Ok(None),
        }
    }

    /// Count all registered identities.
    pub async fn count(&self) -> Result<usize> {
        let count = self
            .conn
            .call(|conn| {
                let count: i64 =
                    conn.query_row("SELECT COUNT(*) FROM identities", [], |row| row.get(0))?;
                Ok(count as usize)
            })
            .await?;
        Ok(count)
    }

    // -----------------------------------------------------------------------
    // Platform link operations
    // -----------------------------------------------------------------------

    /// Link (or update) a platform handle for a user.
    pub async fn link_platform(
        &self,
        telegram_user_id: u64,
        platform: &str,
        handle: &str,
        verified_via_ens: bool,
    ) -> Result<()> {
        let platform = platform.to_string();
        let handle = handle.to_string();
        self.conn
            .call(move |conn| {
                conn.execute(
                    "INSERT OR REPLACE INTO platform_links
                     (telegram_user_id, platform, handle, verified_via_ens)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        telegram_user_id as i64,
                        platform,
                        handle,
                        verified_via_ens as i32,
                    ],
                )?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    /// Get all platform links for a user.
    pub async fn get_platform_links(
        &self,
        telegram_user_id: u64,
    ) -> Result<Vec<PlatformLink>> {
        let links = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT platform, handle, verified_via_ens
                     FROM platform_links WHERE telegram_user_id = ?1",
                )?;
                let rows = stmt
                    .query_map(params![telegram_user_id as i64], |row| {
                        Ok(PlatformLink {
                            platform: row.get(0)?,
                            handle: row.get(1)?,
                            verified_via_ens: row.get::<_, i32>(2)? != 0,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await?;
        Ok(links)
    }

    // -----------------------------------------------------------------------
    // Challenge operations
    // -----------------------------------------------------------------------

    /// Create a new pending challenge token.
    pub async fn create_challenge(
        &self,
        token: &str,
        challenger_user_id: i64,
        label: Option<String>,
        created_at: chrono::DateTime<chrono::Utc>,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let token = token.to_string();
        self.conn
            .call(move |conn| {
                conn.execute(
                    "INSERT INTO challenges
                     (token, challenger_user_id, label, status, created_at, expires_at)
                     VALUES (?1, ?2, ?3, 'pending', ?4, ?5)",
                    params![
                        token,
                        challenger_user_id,
                        label,
                        created_at.to_rfc3339(),
                        expires_at.to_rfc3339(),
                    ],
                )?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    /// Fetch a challenge by token.
    pub async fn get_challenge(&self, token: &str) -> Result<Option<Challenge>> {
        let token = token.to_string();
        let sql = format!("SELECT {} FROM challenges WHERE token = ?1", CHALLENGE_COLUMNS);
        let challenge = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare(&sql)?;
                let row = stmt
                    .query_row(params![token], Challenge::from_row)
                    .optional()?;
                Ok(row)
            })
            .await?;
        Ok(challenge)
    }

    /// Atomically claim a pending challenge. The `WHERE status = 'pending'` guard makes
    /// this single-use even under concurrent claims: only the first claim updates a row.
    /// Returns `true` if this call is the one that claimed it.
    pub async fn claim_challenge(
        &self,
        token: &str,
        claimant_user_id: i64,
        claimant_username: Option<String>,
        claimant_tier: &str,
        passed: bool,
        claimed_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<bool> {
        let token = token.to_string();
        let claimant_tier = claimant_tier.to_string();
        let claimed = self
            .conn
            .call(move |conn| {
                let count = conn.execute(
                    "UPDATE challenges
                     SET status = 'claimed', claimant_user_id = ?2, claimant_username = ?3,
                         claimant_tier = ?4, passed = ?5, claimed_at = ?6
                     WHERE token = ?1 AND status = 'pending'",
                    params![
                        token,
                        claimant_user_id,
                        claimant_username,
                        claimant_tier,
                        passed as i32,
                        claimed_at.to_rfc3339(),
                    ],
                )?;
                Ok(count > 0)
            })
            .await?;
        Ok(claimed)
    }

    /// List a challenger's recent challenges, newest first.
    pub async fn list_challenges_for(
        &self,
        challenger_user_id: i64,
        limit: usize,
    ) -> Result<Vec<Challenge>> {
        let sql = format!(
            "SELECT {} FROM challenges WHERE challenger_user_id = ?1 \
             ORDER BY created_at DESC LIMIT ?2",
            CHALLENGE_COLUMNS
        );
        let rows = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare(&sql)?;
                let rows = stmt
                    .query_map(params![challenger_user_id, limit as i64], Challenge::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await?;
        Ok(rows)
    }

    // -----------------------------------------------------------------------
    // Migration from JSON
    // -----------------------------------------------------------------------

    /// Migrate existing registry data from a JSON file into SQLite.
    ///
    /// Handles both array format (`[{...}, {...}]`) and map format
    /// (`{"123": {...}, "456": {...}}`), since the old registry serialization
    /// may have used either depending on the version.
    ///
    /// Reads the JSON file, inserts all entries, and renames the file to .migrated.
    /// Returns the number of entries migrated.
    pub async fn migrate_from_json(&self, registry_path: &str) -> Result<usize> {
        let path = std::path::PathBuf::from(registry_path);
        if !path.exists() {
            return Ok(0);
        }

        let content = tokio::fs::read_to_string(&path).await?;

        // Try array format first, then map format
        let entries: Vec<RegistrationEntry> =
            if let Ok(vec) = serde_json::from_str::<Vec<RegistrationEntry>>(&content) {
                vec
            } else if let Ok(map) =
                serde_json::from_str::<std::collections::HashMap<String, RegistrationEntry>>(
                    &content,
                )
            {
                map.into_values().collect()
            } else {
                anyhow::bail!(
                    "registry.json is not a recognized format (expected array or map of entries)"
                );
            };
        let count = entries.len();

        for entry in &entries {
            self.register_identity(entry).await?;
        }

        // Rename to .migrated so we don't re-import on next startup
        let migrated_path = path.with_extension("json.migrated");
        tokio::fs::rename(&path, &migrated_path).await?;
        info!(
            "Migrated {} entries from {} -> {}",
            count,
            path.display(),
            migrated_path.display()
        );

        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
    async fn test_register_and_lookup_by_user_id() {
        let db = Database::new_in_memory().await.unwrap();
        let entry = make_entry(123, "alice", "alice.eth");
        db.register_identity(&entry).await.unwrap();

        let found = db.lookup_by_user_id(123).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.ens_name, "alice.eth");
        assert_eq!(found.telegram_username, "alice");
    }

    #[tokio::test]
    async fn test_lookup_by_username_case_insensitive() {
        let db = Database::new_in_memory().await.unwrap();
        db.register_identity(&make_entry(123, "Alice", "alice.eth"))
            .await
            .unwrap();

        // lowercase
        let found = db.lookup_by_username("alice").await.unwrap();
        assert!(found.is_some());

        // with @ prefix
        let found = db.lookup_by_username("@Alice").await.unwrap();
        assert!(found.is_some());

        // not found
        let not_found = db.lookup_by_username("bob").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_deregister() {
        let db = Database::new_in_memory().await.unwrap();
        db.register_identity(&make_entry(123, "alice", "alice.eth"))
            .await
            .unwrap();
        assert_eq!(db.count().await.unwrap(), 1);

        let removed = db.deregister_identity(123).await.unwrap();
        assert!(removed);
        assert_eq!(db.count().await.unwrap(), 0);
        assert!(db.lookup_by_username("alice").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_lookup_by_wallet() {
        let db = Database::new_in_memory().await.unwrap();
        db.register_identity(&make_entry(123, "alice", "alice.eth"))
            .await
            .unwrap();

        let found = db
            .lookup_by_wallet("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045")
            .await
            .unwrap();
        assert!(found.is_some());

        // case-insensitive
        let found = db
            .lookup_by_wallet("0xd8da6bf26964af9d7eed9e03e53415d37aa96045")
            .await
            .unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_platform_links() {
        let db = Database::new_in_memory().await.unwrap();
        db.register_identity(&make_entry(123, "alice", "alice.eth"))
            .await
            .unwrap();

        db.link_platform(123, "twitter", "alice_x", false)
            .await
            .unwrap();
        db.link_platform(123, "github", "alice-gh", true)
            .await
            .unwrap();

        let links = db.get_platform_links(123).await.unwrap();
        assert_eq!(links.len(), 2);

        // Twitter lookup
        let found = db.lookup_by_twitter_handle("alice_x").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().telegram_user_id, 123);
    }

    #[tokio::test]
    async fn test_platform_links_cascade_delete() {
        let db = Database::new_in_memory().await.unwrap();
        db.register_identity(&make_entry(123, "alice", "alice.eth"))
            .await
            .unwrap();
        db.link_platform(123, "twitter", "alice_x", false)
            .await
            .unwrap();

        // Deregister should cascade-delete platform links
        db.deregister_identity(123).await.unwrap();
        let links = db.get_platform_links(123).await.unwrap();
        assert!(links.is_empty());
    }

    #[tokio::test]
    async fn test_reregister_updates_entry() {
        let db = Database::new_in_memory().await.unwrap();
        db.register_identity(&make_entry(123, "oldname", "alice.eth"))
            .await
            .unwrap();
        db.register_identity(&make_entry(123, "newname", "alice.eth"))
            .await
            .unwrap();

        assert!(db.lookup_by_username("oldname").await.unwrap().is_none());
        assert!(db.lookup_by_username("newname").await.unwrap().is_some());
        assert_eq!(db.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_sbt_types_roundtrip() {
        let db = Database::new_in_memory().await.unwrap();
        let mut entry = make_entry(123, "alice", "alice.eth");
        entry.verified_sbt_types = vec![
            VerificationType::Kyc,
            VerificationType::Passport,
            VerificationType::Biometrics,
        ];
        db.register_identity(&entry).await.unwrap();

        let found = db.lookup_by_user_id(123).await.unwrap().unwrap();
        assert_eq!(found.verified_sbt_types.len(), 3);
        assert!(found.verified_sbt_types.contains(&VerificationType::Kyc));
        assert!(found
            .verified_sbt_types
            .contains(&VerificationType::Passport));
        assert!(found
            .verified_sbt_types
            .contains(&VerificationType::Biometrics));
    }

    #[tokio::test]
    async fn test_migrate_from_json() {
        let dir = tempfile::tempdir().unwrap();
        let json_path = dir.path().join("registry.json");

        // Write test JSON
        let entries = vec![
            make_entry(1, "alice", "alice.eth"),
            make_entry(2, "bob", "bob.eth"),
        ];
        let json = serde_json::to_string_pretty(&entries).unwrap();
        tokio::fs::write(&json_path, &json).await.unwrap();

        let db = Database::new_in_memory().await.unwrap();
        let count = db
            .migrate_from_json(json_path.to_str().unwrap())
            .await
            .unwrap();
        assert_eq!(count, 2);
        assert_eq!(db.count().await.unwrap(), 2);

        // Original file should be renamed
        assert!(!json_path.exists());
        assert!(json_path.with_extension("json.migrated").exists());
    }

    #[tokio::test]
    async fn test_migrate_missing_file() {
        let db = Database::new_in_memory().await.unwrap();
        let count = db
            .migrate_from_json("/tmp/nonexistent_privid_test.json")
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_challenge_create_and_get() {
        let db = Database::new_in_memory().await.unwrap();
        let now = chrono::Utc::now();
        let expires = now + chrono::Duration::minutes(30);
        db.create_challenge("chg_abc", 111, Some("a deal".to_string()), now, expires)
            .await
            .unwrap();

        let c = db.get_challenge("chg_abc").await.unwrap().unwrap();
        assert_eq!(c.challenger_user_id, 111);
        assert_eq!(c.label.as_deref(), Some("a deal"));
        assert_eq!(c.status, "pending");
        assert!(c.is_open(now));
        assert_eq!(c.display_status(now), "pending");

        assert!(db.get_challenge("nope").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_challenge_claim_is_single_use() {
        let db = Database::new_in_memory().await.unwrap();
        let now = chrono::Utc::now();
        let expires = now + chrono::Duration::minutes(30);
        db.create_challenge("chg_xyz", 111, None, now, expires)
            .await
            .unwrap();

        // First claim succeeds.
        let first = db
            .claim_challenge("chg_xyz", 222, Some("bob".to_string()), "KYC'd human", true, now)
            .await
            .unwrap();
        assert!(first);

        // Second claim is rejected (single-use guard).
        let second = db
            .claim_challenge("chg_xyz", 333, Some("eve".to_string()), "Unknown", false, now)
            .await
            .unwrap();
        assert!(!second);

        // Recorded result reflects the first claimant only.
        let c = db.get_challenge("chg_xyz").await.unwrap().unwrap();
        assert_eq!(c.status, "claimed");
        assert_eq!(c.claimant_user_id, Some(222));
        assert_eq!(c.passed, Some(true));
        assert_eq!(c.display_status(now), "passed");
        assert!(!c.is_open(now));
    }

    #[tokio::test]
    async fn test_challenge_expiry_blocks_claim_open() {
        let db = Database::new_in_memory().await.unwrap();
        let now = chrono::Utc::now();
        let expired = now - chrono::Duration::minutes(1);
        db.create_challenge("chg_old", 111, None, now - chrono::Duration::minutes(31), expired)
            .await
            .unwrap();

        let c = db.get_challenge("chg_old").await.unwrap().unwrap();
        assert!(!c.is_open(now));
        assert_eq!(c.display_status(now), "expired");
    }

    #[tokio::test]
    async fn test_list_challenges_for() {
        let db = Database::new_in_memory().await.unwrap();
        let now = chrono::Utc::now();
        let expires = now + chrono::Duration::minutes(30);
        db.create_challenge("chg_1", 111, None, now, expires).await.unwrap();
        db.create_challenge("chg_2", 111, None, now + chrono::Duration::seconds(1), expires)
            .await
            .unwrap();
        db.create_challenge("chg_other", 999, None, now, expires).await.unwrap();

        let list = db.list_challenges_for(111, 10).await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|c| c.challenger_user_id == 111));
    }

    #[tokio::test]
    async fn test_migrate_from_json_map_format() {
        let dir = tempfile::tempdir().unwrap();
        let json_path = dir.path().join("registry.json");

        // Write test JSON as a HashMap (keyed by user_id string)
        let mut map = std::collections::HashMap::new();
        map.insert("1".to_string(), make_entry(1, "alice", "alice.eth"));
        map.insert("2".to_string(), make_entry(2, "bob", "bob.eth"));
        let json = serde_json::to_string_pretty(&map).unwrap();
        tokio::fs::write(&json_path, &json).await.unwrap();

        let db = Database::new_in_memory().await.unwrap();
        let count = db
            .migrate_from_json(json_path.to_str().unwrap())
            .await
            .unwrap();
        assert_eq!(count, 2);
        assert_eq!(db.count().await.unwrap(), 2);
    }
}
