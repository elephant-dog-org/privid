use async_trait::async_trait;
use log::info;
use tokio::time::{sleep, Duration};

use crate::blockchain::types::VerificationType;
use crate::state::VerificationResult;

use super::provider::{VerificationError, VerificationProvider};

/// Mock verification provider for development and testing.
///
/// Simulates blockchain verification with a 2-second delay and always
/// returns a successful result. Useful for local development without
/// needing a real RPC endpoint.
pub struct MockVerificationProvider;

impl MockVerificationProvider {
    pub fn new() -> Self {
        Self
    }

    /// Build a mock `VerificationResult` for the given verification type.
    fn mock_result(
        wallet_address: &str,
        verification_type: VerificationType,
    ) -> VerificationResult {
        let random_proof_id = rand::random::<u64>();

        VerificationResult {
            verified: true,
            timestamp: chrono::Utc::now().to_rfc3339(),
            proof: format!("mock-zk-proof-{}", random_proof_id),
            badge: format!("Verified: {}", verification_type.description()),
            verification_type: Some(verification_type.description().to_string()),
            wallet_address: Some(wallet_address.to_string()),
            sbt_expiry: Some(chrono::Utc::now().timestamp() as u64 + 365 * 24 * 3600),
        }
    }
}

impl Default for MockVerificationProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VerificationProvider for MockVerificationProvider {
    async fn check_verification(
        &self,
        wallet_address: &str,
        verification_type: VerificationType,
    ) -> Result<VerificationResult, VerificationError> {
        info!(
            "[MOCK] Checking {} verification for {}",
            verification_type.description(),
            wallet_address
        );

        // Simulate network latency
        sleep(Duration::from_secs(2)).await;

        info!(
            "[MOCK] Verification complete for {} - {}",
            wallet_address,
            verification_type.description()
        );

        Ok(Self::mock_result(wallet_address, verification_type))
    }

    async fn check_all_verifications(
        &self,
        wallet_address: &str,
    ) -> Vec<(VerificationType, Result<VerificationResult, VerificationError>)> {
        info!("[MOCK] Checking all verifications for {}", wallet_address);

        // Simulate network latency (once, not per-type)
        sleep(Duration::from_secs(2)).await;

        VerificationType::all()
            .iter()
            .map(|vt| {
                let result = Self::mock_result(wallet_address, *vt);
                (*vt, Ok(result))
            })
            .collect()
    }

    fn is_mock(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_check_verification() {
        let provider = MockVerificationProvider::new();
        let result = provider
            .check_verification(
                "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
                VerificationType::Kyc,
            )
            .await;

        let result = result.expect("Mock verification should succeed");
        assert!(result.verified);
        assert!(result.proof.starts_with("mock-zk-proof-"));
        assert!(result.badge.contains("KYC"));
        assert_eq!(
            result.verification_type,
            Some("KYC (Know Your Customer)".to_string())
        );
        assert_eq!(
            result.wallet_address,
            Some("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".to_string())
        );
        assert!(result.sbt_expiry.is_some());
    }

    #[tokio::test]
    async fn test_mock_check_all_verifications() {
        let provider = MockVerificationProvider::new();
        let results = provider
            .check_all_verifications("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045")
            .await;

        assert_eq!(results.len(), 5, "Should return results for all 5 types");

        for (vt, result) in &results {
            let result = result.as_ref().expect("All mock verifications should succeed");
            assert!(result.verified);
            assert!(
                result.badge.contains(vt.description()),
                "Badge should contain verification type description"
            );
        }
    }

    #[test]
    fn test_mock_is_mock() {
        let provider = MockVerificationProvider::new();
        assert!(provider.is_mock());
    }

    #[test]
    fn test_mock_default() {
        let provider = MockVerificationProvider;
        assert!(provider.is_mock());
    }
}
