pub mod blockchain;
pub mod config;
pub mod state;
pub mod storage;
pub mod verification;

// Re-export main types for easier access
pub use blockchain::{BlockchainVerificationProvider, RpcClient, SbtData, VerificationType};
pub use config::{Config, VerificationMode};
pub use state::{BotState, UserSession, VerificationResult, VerificationState};
pub use storage::FileStorage;
pub use verification::{MockVerificationProvider, VerificationError, VerificationProvider};
