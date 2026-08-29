# Changelog

## Unreleased — 2026-08-28 (pre-live-test polish)
- **Fixed:** Hub V3 `getSBT` *reverts* for a missing/expired SBT rather than
  returning zeros. In `blockchain` mode every unverified wallet used to surface as
  an RPC error; now it is `NotVerified` (and `"SBT has been revoked"` → `Revoked`).
- **Fixed:** ePassport circuit ID was wrong; corrected against
  `holonym-api/src/constants/misc.js`. All five IDs verified.
- **Fixed:** Docker lost the SQLite registry on every restart (`./data` was not
  mounted). Added `.dockerignore`.
- **Fixed:** copy pointed users at Human Passport (passport.xyz); the bot reads
  Human ID / Holonym SBTs, minted at id.human.tech.
- Challenge verdicts and `/verifyme` re-check the chain live before judging.
- Verdicts state an assurance tier, not a bare pass/fail.
- HTTP API binds `127.0.0.1` by default (`API_BIND`); Docker sets `0.0.0.0`.
- `tests/live_rpc_test.rs`: opt-in tests against the real Optimism + mainnet RPCs.
- Build is warning-free (bin now uses the library crate instead of re-declaring modules).

## 2026-06-11
- Trust checkpoint: `/challenge`, `/start chg_<token>`, `/challenges`, `/verifyme`.

## v0.1.0
- Initial Telegram + Holonym scaffold.
