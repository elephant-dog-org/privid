use async_trait::async_trait;
use log::{debug, info, warn};

use crate::blockchain::abi::{self, AbiError};
use crate::blockchain::rpc::{RpcClient, RpcError};
use crate::blockchain::types::VerificationType;
use crate::state::VerificationResult;
use crate::verification::provider::{VerificationError, VerificationProvider};

/// Real blockchain verification provider that queries the Holonym Hub contract
/// on Optimism to check Soulbound Token (SBT) ownership.
pub struct BlockchainVerificationProvider {
    rpc_client: RpcClient,
    hub_contract_address: String,
}

impl BlockchainVerificationProvider {
    /// Create a new blockchain verification provider.
    ///
    /// # Arguments
    /// - `rpc_url`: Optimism JSON-RPC endpoint URL
    /// - `hub_contract_address`: Holonym Hub contract address (hex with 0x prefix)
    pub fn new(rpc_url: String, hub_contract_address: String) -> Self {
        Self {
            rpc_client: RpcClient::new(rpc_url),
            hub_contract_address,
        }
    }

    /// Returns a reference to the underlying RPC client.
    pub fn rpc_client(&self) -> &RpcClient {
        &self.rpc_client
    }

    /// Returns the hub contract address.
    pub fn hub_contract_address(&self) -> &str {
        &self.hub_contract_address
    }
}

/// Map an `AbiError` into a `VerificationError`.
impl From<AbiError> for VerificationError {
    fn from(err: AbiError) -> Self {
        match err {
            AbiError::InvalidAddress(msg) => VerificationError::InvalidAddress(msg),
            other => VerificationError::AbiError(other.to_string()),
        }
    }
}

/// Map an `RpcError` into a `VerificationError`.
impl From<RpcError> for VerificationError {
    fn from(err: RpcError) -> Self {
        VerificationError::RpcError(err.to_string())
    }
}

/// Does this JSON-RPC revert message mean "no valid SBT for this wallet"?
///
/// Hub V3's `getSBT` reverts (rather than returning zeros) when the SBT is
/// missing or expired. The contract folds both cases into one message, so we
/// cannot distinguish "never verified" from "verified but lapsed" on-chain;
/// both map to `NotVerified`, which is the honest verdict either way.
fn is_no_sbt_revert(message: &str) -> bool {
    let m = message.to_ascii_lowercase();
    m.contains("does not exist") || m.contains("expired")
}

#[async_trait]
impl VerificationProvider for BlockchainVerificationProvider {
    async fn check_verification(
        &self,
        wallet_address: &str,
        verification_type: VerificationType,
    ) -> Result<VerificationResult, VerificationError> {
        info!(
            "Checking {} verification for {} on-chain",
            verification_type.description(),
            wallet_address
        );

        // 1. Parse wallet address
        let address_bytes = abi::parse_address(wallet_address)?;

        // 2. Get circuit ID for this verification type
        let circuit_id = verification_type.circuit_id();

        // 3. Encode calldata for getSBT(address, bytes32)
        let calldata = abi::encode_get_sbt(&address_bytes, &circuit_id);

        debug!(
            "Calling getSBT on {} with calldata length {}",
            self.hub_contract_address,
            calldata.len()
        );

        // 4. Execute eth_call with retry (3 retries for transient failures)
        //
        // Hub V3 does NOT return a zeroed struct for a missing SBT: `getSBT`
        // REVERTS with "SBT is expired or does not exist" (verified against the
        // live Optimism contract 2026-08-28 — see tests/live_rpc_test.rs). That
        // is the normal outcome for any unverified wallet, so it must surface as
        // `NotVerified`, not as an infrastructure `RpcError` that the bot would
        // show the user as a failure. Genuine RPC problems still propagate.
        let type_desc = verification_type.description().to_string();
        let response_bytes = match self
            .rpc_client
            .eth_call_with_retry(&self.hub_contract_address, &calldata, 3)
            .await
        {
            Ok(bytes) => bytes,
            Err(RpcError::JsonRpc { message, .. }) if is_no_sbt_revert(&message) => {
                debug!(
                    "Hub reverted for {} / {:?}: {} -> NotVerified",
                    wallet_address, verification_type, message
                );
                return Err(VerificationError::NotVerified(type_desc));
            }
            Err(e) => return Err(e.into()),
        };

        // 5. Decode the ABI response
        let sbt = abi::decode_sbt_response(&response_bytes)?;

        debug!(
            "SBT data for {} / {:?}: expiry={}, revoked={}, public_values={}",
            wallet_address,
            verification_type,
            sbt.expiry,
            sbt.revoked,
            sbt.public_values.len()
        );

        // 6. Check validity

        // No SBT exists
        if sbt.is_empty() {
            return Err(VerificationError::NotVerified(type_desc));
        }

        // SBT has been revoked
        if sbt.revoked {
            warn!(
                "SBT revoked for {} - {}",
                wallet_address,
                verification_type.description()
            );
            return Err(VerificationError::Revoked(type_desc));
        }

        // SBT has expired
        let now = chrono::Utc::now().timestamp() as u64;
        if sbt.expiry <= now {
            warn!(
                "SBT expired for {} - {} (expiry={}, now={})",
                wallet_address,
                verification_type.description(),
                sbt.expiry,
                now
            );
            return Err(VerificationError::Expired(type_desc));
        }

        // 7. Build successful verification result
        info!(
            "Verification confirmed for {} - {}",
            wallet_address,
            verification_type.description()
        );

        Ok(VerificationResult {
            verified: true,
            timestamp: chrono::Utc::now().to_rfc3339(),
            proof: format!(
                "sbt:optimism:{}:{}",
                hex::encode(circuit_id),
                wallet_address
            ),
            badge: format!("Verified: {}", verification_type.description()),
            verification_type: Some(type_desc),
            wallet_address: Some(wallet_address.to_string()),
            sbt_expiry: Some(sbt.expiry),
        })
    }

    async fn check_all_verifications(
        &self,
        wallet_address: &str,
    ) -> Vec<(VerificationType, Result<VerificationResult, VerificationError>)> {
        info!("Checking all verifications for {} on-chain", wallet_address);

        let mut results = Vec::with_capacity(5);

        for vt in VerificationType::all() {
            let result = self.check_verification(wallet_address, *vt).await;
            results.push((*vt, result));
        }

        results
    }

    fn is_mock(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blockchain_provider_new() {
        let provider = BlockchainVerificationProvider::new(
            "https://optimism-rpc.publicnode.com".to_string(),
            "0x2AA822e264F8cc31A2b9C22f39e5551241e94DfB".to_string(),
        );
        assert_eq!(
            provider.rpc_client().rpc_url(),
            "https://optimism-rpc.publicnode.com"
        );
        assert_eq!(
            provider.hub_contract_address(),
            "0x2AA822e264F8cc31A2b9C22f39e5551241e94DfB"
        );
    }

    #[test]
    fn test_no_sbt_revert_detection() {
        // The exact message Hub V3 emits on Optimism (captured live 2026-08-28).
        assert!(is_no_sbt_revert("execution reverted: SBT is expired or does not exist"));
        assert!(is_no_sbt_revert("SBT does not exist"));
        // Infrastructure failures must NOT be swallowed as "not verified".
        assert!(!is_no_sbt_revert("execution reverted"));
        assert!(!is_no_sbt_revert("rate limit exceeded"));
        assert!(!is_no_sbt_revert("out of gas"));
    }

    #[test]
    fn test_blockchain_provider_is_not_mock() {
        let provider = BlockchainVerificationProvider::new(
            "https://example.com".to_string(),
            "0x0000000000000000000000000000000000000000".to_string(),
        );
        assert!(!provider.is_mock());
    }

    #[test]
    fn test_abi_error_to_verification_error() {
        let abi_err = AbiError::InvalidAddress("bad address".to_string());
        let ver_err: VerificationError = abi_err.into();
        match ver_err {
            VerificationError::InvalidAddress(msg) => {
                assert_eq!(msg, "bad address");
            }
            other => panic!("Expected InvalidAddress, got: {:?}", other),
        }

        let abi_err = AbiError::ResponseTooShort {
            expected: 96,
            actual: 32,
        };
        let ver_err: VerificationError = abi_err.into();
        match ver_err {
            VerificationError::AbiError(msg) => {
                assert!(msg.contains("96"));
                assert!(msg.contains("32"));
            }
            other => panic!("Expected AbiError, got: {:?}", other),
        }
    }

    #[test]
    fn test_rpc_error_to_verification_error() {
        let rpc_err = RpcError::MissingResult;
        let ver_err: VerificationError = rpc_err.into();
        match ver_err {
            VerificationError::RpcError(msg) => {
                assert!(msg.contains("result"));
            }
            other => panic!("Expected RpcError, got: {:?}", other),
        }
    }
}
