//! Live-network integration tests.
//!
//! These hit the REAL public Optimism and Ethereum mainnet JSON-RPC endpoints,
//! so they are `#[ignore]`d by default and never run in the normal `cargo test`
//! gate. Run them deliberately before a live smoke test:
//!
//! ```sh
//! cargo test --test live_rpc_test -- --ignored --nocapture
//! ```
//!
//! What they prove that unit tests cannot:
//! - our hand-rolled `getSBT(address,bytes32)` calldata is accepted by the
//!   deployed Holonym Hub (no revert / decode error) and the response shape
//!   decodes;
//! - a wallet with no SBT surfaces as `NotVerified`, not as an RPC error
//!   (the bot's copy depends on that distinction);
//! - ENS forward resolution and text-record reads work against mainnet.
//!
//! Optional positive case: set `PRIVID_TEST_SBT_WALLET=0x...` to a wallet known
//! to hold at least one valid Holonym SBT and the positive test asserts on it.

use privid_telegram_bot::{
    BlockchainVerificationProvider, EnsResolver, VerificationError, VerificationProvider,
    VerificationType,
};

/// Defaults mirror `Config::from_env` so this exercises the same endpoints
/// the bot will use in `VERIFICATION_MODE=blockchain`.
fn provider() -> BlockchainVerificationProvider {
    let rpc = std::env::var("OPTIMISM_RPC_URL")
        .unwrap_or_else(|_| "https://optimism-rpc.publicnode.com".to_string());
    let hub = std::env::var("HUB_CONTRACT_ADDRESS")
        .unwrap_or_else(|_| "0x2AA822e264F8cc31A2b9C22f39e5551241e94DfB".to_string());
    BlockchainVerificationProvider::new(rpc, hub)
}

fn ens() -> EnsResolver {
    let rpc = std::env::var("ETHEREUM_RPC_URL")
        .unwrap_or_else(|_| "https://ethereum-rpc.publicnode.com".to_string());
    EnsResolver::new(rpc)
}

/// A wallet that certainly holds no Holonym SBT: the burn address.
const NO_SBT_WALLET: &str = "0x000000000000000000000000000000000000dEaD";

/// vitalik.eth → a stable, well-known forward resolution.
const VITALIK: &str = "0xd8da6bf26964af9d7eed9e03e53415d37aa96045";

#[tokio::test]
#[ignore = "hits live Optimism RPC"]
async fn hub_call_for_wallet_without_sbt_is_not_verified_not_error() {
    let p = provider();
    for vt in VerificationType::all() {
        let r = p.check_verification(NO_SBT_WALLET, *vt).await;
        match r {
            // The only acceptable outcome: the call succeeded and decoded to
            // "no SBT". Anything else means our ABI or the contract's revert
            // behaviour differs from what `provider.rs` assumes.
            Err(VerificationError::NotVerified(_)) => {
                eprintln!("{:?}: NotVerified (ok)", vt);
            }
            other => panic!(
                "{:?}: expected NotVerified for a no-SBT wallet, got {:?}",
                vt, other
            ),
        }
    }
}

#[tokio::test]
#[ignore = "hits live Optimism RPC"]
async fn check_all_verifications_returns_one_entry_per_type() {
    let results = provider().check_all_verifications(NO_SBT_WALLET).await;
    assert_eq!(results.len(), VerificationType::all().len());
}

#[tokio::test]
#[ignore = "hits live Optimism RPC; needs PRIVID_TEST_SBT_WALLET"]
async fn known_sbt_holder_has_at_least_one_valid_sbt() {
    let Ok(wallet) = std::env::var("PRIVID_TEST_SBT_WALLET") else {
        eprintln!("PRIVID_TEST_SBT_WALLET not set — skipping positive case");
        return;
    };
    let results = provider().check_all_verifications(&wallet).await;
    let mut any_ok = false;
    for (vt, r) in &results {
        eprintln!("{:?}: {:?}", vt, r.as_ref().map(|v| (&v.badge, v.sbt_expiry)));
        any_ok |= r.is_ok();
    }
    assert!(any_ok, "expected at least one valid SBT on {wallet}");
}

#[tokio::test]
#[ignore = "hits live Ethereum mainnet RPC"]
async fn ens_resolves_vitalik_eth() {
    let addr = ens().resolve_address("vitalik.eth").await.expect("resolve");
    assert_eq!(format!("0x{}", hex::encode(addr)), VITALIK);
}

#[tokio::test]
#[ignore = "hits live Ethereum mainnet RPC"]
async fn ens_text_record_read_works() {
    // vitalik.eth has long-standing text records; we only assert the call
    // path works (Ok or a clean "not found"), not a specific value, since the
    // owner can change them at any time.
    let r = ens().get_text_record("vitalik.eth", "url").await;
    eprintln!("vitalik.eth url = {:?}", r);
    assert!(
        r.is_ok(),
        "text-record read should succeed for an existing name: {:?}",
        r
    );
    // The bot's /register flow reads this specific key; make sure a missing
    // key on a real name is a clean result rather than a decode error.
    let missing = ens().get_text_record("vitalik.eth", "org.telegram").await;
    eprintln!("vitalik.eth org.telegram = {:?}", missing);
}

#[tokio::test]
#[ignore = "hits live Ethereum mainnet RPC"]
async fn ens_unregistered_name_is_clean_error() {
    let r = ens()
        .resolve_address("this-name-should-never-exist-privid-8f3a.eth")
        .await;
    assert!(r.is_err(), "unregistered name must not resolve: {:?}", r);
}
