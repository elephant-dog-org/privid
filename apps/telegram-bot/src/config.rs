use std::env;
use std::fmt;

/// Verification mode determines whether the bot uses mock data or real blockchain queries.
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationMode {
    /// Mock verification for development/testing (no blockchain calls)
    Mock,
    /// Real blockchain verification against the Holonym Hub contract on Optimism
    Blockchain,
}

impl fmt::Display for VerificationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerificationMode::Mock => write!(f, "mock"),
            VerificationMode::Blockchain => write!(f, "blockchain"),
        }
    }
}

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Telegram Bot API token (required)
    pub telegram_bot_token: String,
    /// Whether to use mock or real blockchain verification
    pub verification_mode: VerificationMode,
    /// Optimism JSON-RPC endpoint URL
    pub optimism_rpc_url: String,
    /// Holonym Hub contract address on Optimism
    pub hub_contract_address: String,
    /// Ethereum mainnet JSON-RPC endpoint URL (for ENS resolution)
    pub ethereum_rpc_url: String,
    /// Logging level filter
    pub rust_log: String,
    /// Port for the HTTP API server
    pub api_port: u16,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// # Required
    /// - `TELEGRAM_BOT_TOKEN` - must be set or this function panics
    ///
    /// # Optional (with defaults)
    /// - `VERIFICATION_MODE` - "mock" (default) or "blockchain"
    /// - `OPTIMISM_RPC_URL` - defaults to public Optimism RPC
    /// - `HUB_CONTRACT_ADDRESS` - defaults to Holonym Hub V3
    /// - `RUST_LOG` - defaults to "info"
    pub fn from_env() -> Self {
        let telegram_bot_token = env::var("TELEGRAM_BOT_TOKEN")
            .expect("TELEGRAM_BOT_TOKEN must be set in environment");

        let verification_mode = match env::var("VERIFICATION_MODE")
            .unwrap_or_else(|_| "mock".to_string())
            .to_lowercase()
            .as_str()
        {
            "blockchain" => VerificationMode::Blockchain,
            _ => VerificationMode::Mock,
        };

        let optimism_rpc_url = env::var("OPTIMISM_RPC_URL")
            .unwrap_or_else(|_| "https://optimism-rpc.publicnode.com".to_string());

        let hub_contract_address = env::var("HUB_CONTRACT_ADDRESS")
            .unwrap_or_else(|_| "0x2AA822e264F8cc31A2b9C22f39e5551241e94DfB".to_string());

        let ethereum_rpc_url = env::var("ETHEREUM_RPC_URL")
            .unwrap_or_else(|_| "https://ethereum-rpc.publicnode.com".to_string());

        let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

        let api_port = env::var("API_PORT")
            .unwrap_or_else(|_| "3141".to_string())
            .parse::<u16>()
            .unwrap_or(3141);

        Self {
            telegram_bot_token,
            verification_mode,
            optimism_rpc_url,
            hub_contract_address,
            ethereum_rpc_url,
            rust_log,
            api_port,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_mode_display() {
        assert_eq!(VerificationMode::Mock.to_string(), "mock");
        assert_eq!(VerificationMode::Blockchain.to_string(), "blockchain");
    }
}
