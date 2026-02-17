use async_trait::async_trait;
use thiserror::Error;

use crate::blockchain::types::VerificationType;
use crate::state::VerificationResult;

/// Errors that can occur during the verification check process.
#[derive(Debug, Error)]
pub enum VerificationError {
    #[error("Invalid wallet address: {0}")]
    InvalidAddress(String),

    #[error("RPC call failed: {0}")]
    RpcError(String),

    #[error("ABI decoding failed: {0}")]
    AbiError(String),

    #[error("No valid SBT found for {0}")]
    NotVerified(String),

    #[error("SBT expired for {0}")]
    Expired(String),

    #[error("SBT revoked for {0}")]
    Revoked(String),
}

/// Abstraction over verification providers.
///
/// Implementations can check whether a wallet holds a valid Soulbound Token
/// for a given verification type. The two built-in implementations are:
///
/// - `MockVerificationProvider` for development/testing
/// - `BlockchainVerificationProvider` for real on-chain queries
#[async_trait]
pub trait VerificationProvider: Send + Sync {
    /// Check whether `wallet_address` has a valid SBT for the given verification type.
    ///
    /// Returns a `VerificationResult` on success, or a `VerificationError` describing
    /// why the check failed (invalid address, RPC failure, expired/revoked SBT, etc.).
    async fn check_verification(
        &self,
        wallet_address: &str,
        verification_type: VerificationType,
    ) -> Result<VerificationResult, VerificationError>;

    /// Check all five verification types for a given wallet address.
    ///
    /// Returns a vector of `(VerificationType, Result)` pairs so the caller can
    /// inspect successes and failures independently.
    async fn check_all_verifications(
        &self,
        wallet_address: &str,
    ) -> Vec<(VerificationType, Result<VerificationResult, VerificationError>)>;

    /// Returns `true` if this provider uses mock data rather than real blockchain queries.
    fn is_mock(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_error_display() {
        let err = VerificationError::InvalidAddress("bad-addr".to_string());
        assert_eq!(format!("{}", err), "Invalid wallet address: bad-addr");

        let err = VerificationError::RpcError("timeout".to_string());
        assert_eq!(format!("{}", err), "RPC call failed: timeout");

        let err = VerificationError::AbiError("short response".to_string());
        assert_eq!(format!("{}", err), "ABI decoding failed: short response");

        let err = VerificationError::NotVerified("KYC".to_string());
        assert_eq!(format!("{}", err), "No valid SBT found for KYC");

        let err = VerificationError::Expired("Phone".to_string());
        assert_eq!(format!("{}", err), "SBT expired for Phone");

        let err = VerificationError::Revoked("Passport".to_string());
        assert_eq!(format!("{}", err), "SBT revoked for Passport");
    }
}
