pub mod api;
pub mod blockchain;
pub mod config;
pub mod db;
pub mod ens;
pub mod registry;
pub mod state;
pub mod storage;
pub mod verification;

// Re-export main types for easier access
pub use api::start_api_server;
pub use blockchain::{BlockchainVerificationProvider, RpcClient, SbtData, VerificationType};
pub use config::{Config, VerificationMode};
pub use db::{Database, PlatformLink};
pub use ens::{EnsError, EnsResolver};
pub use registry::{Registry, RegistrationEntry};
pub use state::{BotState, UserSession, VerificationResult, VerificationState};
pub use storage::FileStorage;
pub use verification::{MockVerificationProvider, VerificationError, VerificationProvider};
