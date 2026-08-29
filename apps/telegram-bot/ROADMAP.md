# PrivID Telegram Bot — Roadmap

Status as of 2026-08-28. Spec: `../../.speckit/spec-telegram-challenge.md`.

## Done
- [x] Telegram bot (Rust / teloxide) with `/start /help /status /verify /register /deregister /whois`
- [x] Holonym / Human ID SBT reads straight from the Hub V3 contract on Optimism
      (KYC, phone, ePassport, clean hands, biometrics) — no API key needed
- [x] ENS binding: `/register name.eth` proves ownership via the `org.telegram`
      text record; no wallet signature
- [x] SQLite registry (identities, platform links) + HTTP lookup API for the extension
- [x] **Trust checkpoint**: `/challenge` mints a single-use, 30-min deep link; the
      counterparty taps it; the verdict is DM'd to the challenger. `/challenges`,
      `/verifyme`
- [x] Tiered verdicts (Telegram-only / ENS-linked / KYC'd human)
- [x] Live on-chain re-check at claim time (a lapsed SBT stops passing)
- [x] Live-RPC integration tests (`cargo test --test live_rpc_test -- --ignored`)

## Next — MVP for the friends test
- [ ] First live smoke test with real accounts (see `SMOKE_TEST.md`)
- [ ] Rung 1: one-tap social OAuth for claimants with no wallet (X/Twitter first —
      provider + scopes to be decided)
- [ ] Human Passport Stamps as a second credential source behind `VerificationProvider`
      (`api.passport.xyz/v2/stamps/{scorer}/score/{address}`, `X-API-KEY`)
- [ ] Group challenges (verdict in-thread) — default stays DM-only

## Later
- [ ] Signed verdicts (SEC-4) so a screenshot can't forge one
- [ ] EAS attestation write-back (consent-gated, off by default)
- [ ] Bluesky / AT Protocol linking
