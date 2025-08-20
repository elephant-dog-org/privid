pub mod state;
pub mod storage;

// Re-export main types for easier access
pub use state::{BotState, UserSession, VerificationResult, VerificationState};
pub use storage::FileStorage;
