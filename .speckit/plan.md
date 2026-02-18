# Implementation Plan: PrivID v3 — Passive Verification (BYOK)

**Spec**: .speckit/spec.md
**Created**: 2026-02-17

---

## Architecture Overview

```
Browser Extension (TypeScript)
├── background/
│   ├── service-worker.ts          ← REWRITE: identity resolver engine
│   ├── identityResolver.ts        ← NEW: waterfall orchestrator
│   └── providers/                 ← NEW: one module per API
│       ├── nextid.ts              ← Free: twitter handle → wallet
│       ├── neynar.ts              ← BYOK: twitter handle → wallet (Farcaster)
│       ├── holonym.ts             ← Free: wallet → SBT check
│       ├── ensdata.ts             ← Free: wallet → ENS name + records
│       ├── passport.ts            ← BYOK: wallet → humanity score
│       └── ensSubgraph.ts         ← BYOK: reverse text record lookup
├── content/twitter/               ← MODIFIED: richer tooltips
├── content/gmail/                 ← Unchanged
├── popup/                         ← MODIFIED: provider settings UI
└── blockchain/                    ← Unchanged

Telegram Bot (Rust)
├── src/main.rs                    ← MODIFIED: /whois passive fallback
└── (rest unchanged)
```

---

## Phase 1: Provider Modules (Free Baseline)

**Goal**: Build the three free provider clients that require no API keys.

### New file: `background/providers/nextid.ts`
- Query: `GET https://proof-service.next.id/v1/proof?platform=twitter&identity={handle}&exact=true`
- Parse response: extract `ids` array → find proofs with `platform=ethereum` → return wallet addresses
- Handle: 404 (no proofs found), network errors, malformed responses
- Return type: `string[]` (array of wallet addresses, possibly empty)

### New file: `background/providers/holonym.ts`
- Check 4 credential endpoints on Optimism:
  - `GET https://api.holonym.io/sybil-resistance/gov-id/optimism?user={wallet}&action-id=123456789`
  - Same pattern for: `epassport`, `phone`, `biometrics`
- Each returns `{ "result": true/false }`
- Return type: `string[]` (array of verified credential type names, possibly empty)
- Use a fixed `action-id` value (Holonym uses this for app-scoping; any consistent value works)

### New file: `background/providers/ensdata.ts`
- Query: `GET https://ensdata.net/{wallet_address}`
- Parse response: extract `ens` (primary ENS name) and `records` object
- Return type: `{ ensName: string; records: Record<string, string> } | null`
- 72-hour cache on their end, but we cache locally too

### Tests
- Unit tests for response parsing (mock fetch responses)
- Integration note: real API calls can be tested manually since these are free

---

## Phase 2: Provider Modules (BYOK Enhanced)

**Goal**: Build provider clients for APIs that need user-supplied keys.

### New file: `background/providers/neynar.ts`
- Query: `GET https://api.neynar.com/v2/farcaster/user/by_x_username/?x_username={handle}`
- Header: `x-api-key: {user_key}`
- Parse response: extract `users[0].verified_addresses.eth_addresses[]`
- Return type: `string[]` (wallet addresses)
- Key validation: test query with a known handle (e.g., `vitalik`)

### New file: `background/providers/passport.ts`
- Query: `GET https://api.scorer.gitcoin.co/registry/v2/score/{scorer_id}/{wallet}`
- Header: `X-API-KEY: {user_key}`
- Parse response: extract `score` (string, parse to float) and `passing_score` (boolean)
- Return type: `{ score: number; passing: boolean } | null`
- Scorer ID provided by user alongside API key

### New file: `background/providers/ensSubgraph.ts`
- Query: GraphQL POST to `https://gateway.thegraph.com/api/{user_key}/subgraphs/id/5XqPmWe6gjyrJtFn9cLy237i4cWw2j9HcUJEXsP5qGtH`
- GraphQL query: find `textChangeds` events where `key=com.twitter` and `value` matches handle
- **Caveat**: TextChanged records events not current state — results may be stale
- Parse: extract resolver → domain → name, then forward-resolve to get current wallet
- Return type: `string[]` (wallet addresses)
- This is the least reliable provider — treat results as supplementary

### Tests
- Unit tests for response parsing
- Key validation test functions for each provider

---

## Phase 3: Identity Resolver Orchestrator

**Goal**: Single entry point that waterfalls through providers and returns a unified result.

### New file: `background/identityResolver.ts`

Core interface:
```typescript
interface IdentityResult {
    wallet: string;
    verified: boolean;
    sources: string[];       // which providers confirmed the link
    ensName?: string;
    sbtTypes?: string[];
    passportScore?: number;
    farcasterFid?: number;
}
```

Logic:
1. **Resolve handle → wallets**: Query all enabled handle-to-wallet providers in parallel (Next.ID always, Neynar if key exists, ENS subgraph if key exists)
2. **Deduplicate wallets**: Normalize to lowercase, remove duplicates
3. **Verify wallets → SBTs**: For each wallet, query Holonym (always free). Stop at first wallet with SBTs.
4. **Enrich**: Query ensdata.net (free) for ENS name. Query Gitcoin Passport (if key exists) for score.
5. **Return**: IdentityResult with all collected data, or null if no verified wallet found.

### API key storage helper
- `getApiKey(provider: string): Promise<string | null>` — reads from `chrome.storage.local` key `apiKeys.{provider}`
- `setApiKey(provider: string, key: string): Promise<void>`
- `testApiKey(provider: string, key: string): Promise<boolean>` — makes a test query

### Modified file: `background/service-worker.ts`
- Replace `fetchFromApi()` with `resolveTwitterIdentity()` from identityResolver
- Keep the existing bot API as an optional additional source (if `prividApiUrl` is configured)
- Cache structure unchanged: `twitterRegistry` in chrome.storage.local
- Adjust TTLs: 5 minutes positive, 30 minutes negative (since we're querying external APIs)

---

## Phase 4: Popup Settings UI

**Goal**: Let users configure their API keys and see provider status.

### Modified: `popup/popup.html`
- Replace the "API Settings" `<details>` section with a proper "Identity Providers" panel
- For each provider, show:
  - Provider name + description
  - Free/Paid indicator
  - API key input (for paid providers)
  - "Test" button → shows "Connected" or "Failed"
  - Enabled/disabled toggle
- Providers listed:
  1. Next.ID (free, always enabled, no config needed)
  2. Holonym (free, always enabled, no config needed)
  3. ensdata.net (free, always enabled, no config needed)
  4. Neynar/Farcaster (needs key, input field)
  5. Gitcoin Passport (needs key + scorer ID, two input fields)
  6. The Graph ENS (needs key, input field)
  7. PrivID Bot API (optional, URL input — legacy fallback)

### Modified: `popup/popup.ts`
- Load/save API keys from chrome.storage.local
- Test connection handlers for each paid provider
- Visual feedback: green dot for working providers, gray for unconfigured, red for failed

---

## Phase 5: Badge Tooltip Enhancement

**Goal**: Show richer verification details in Twitter badge tooltips.

### Modified: `content/twitter/injectTwitterBadge.ts`
- Update `createBadgeElement()` to accept the full IdentityResult
- Tooltip now shows:
  - Line 1: "Verified via Holonym" (always)
  - Line 2: Credential types — "KYC, Phone" (from sbtTypes)
  - Line 3: ENS name — "ENS: alice.eth" (if available)
  - Line 4: Passport score — "Passport: 24.5" (if available)
  - Line 5: Sources — "Confirmed by: Next.ID, Farcaster" (from sources array)
  - Line 6: Trust — "Trusted by N verified people you know" (existing trust network)
- Service worker response format updated to include all IdentityResult fields

---

## Phase 6: Telegram Bot Passive Lookup

**Goal**: Bot's `/whois` tries Next.ID as fallback for unregistered users.

### Modified: `apps/telegram-bot/src/main.rs`
- In `/whois` handler: if user not found in local registry, query Next.ID by Telegram username
  - `GET https://proof-service.next.id/v1/proof?platform=telegram&identity={username}&exact=true`
- If Next.ID returns a wallet, check Holonym SBTs
- If verified, show result with note: "Discovered via Next.ID (not registered)"
- No new Rust dependencies needed — uses existing `reqwest`

---

## Implementation Order

| Phase | Effort | Dependencies | Parallelizable |
|-------|--------|--------------|----------------|
| Phase 1: Free providers | Small | None | Yes (all 3 independent) |
| Phase 2: BYOK providers | Small | None | Yes (all 3 independent, parallel with Phase 1) |
| Phase 3: Identity resolver | Medium | Phase 1 + 2 | No (needs providers) |
| Phase 4: Popup settings UI | Small | Phase 3 (needs key storage) | No |
| Phase 5: Badge tooltip | Small | Phase 3 (needs IdentityResult) | Parallel with Phase 4 |
| Phase 6: Telegram fallback | Small | None (independent codebase) | Parallel with any phase |

Phases 1 + 2 can be built in parallel (6 independent modules).
Phase 6 is independent and can be built at any time.
Phases 4 + 5 can be built in parallel once Phase 3 is done.

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Next.ID has low coverage for target audience | Few badges on free baseline | Neynar BYOK provides broader Farcaster coverage; ENS subgraph provides another path |
| Next.ID rate limits or goes down | Free baseline stops working | Cache aggressively; fall back to BYOK providers; negative cache prevents hammering |
| Holonym API changes or goes offline | Can't verify SBTs | Already have direct on-chain SBT checking in the extension (existing blockchain/ module) — can fall back to RPC |
| CORS blocks service worker → API calls | Lookups fail in browser | Service workers are not subject to CORS (they make fetch requests like a server); this is a non-issue |
| ENS subgraph TextChanged is unreliable | Stale reverse lookups | Document as supplementary source; always verify forward resolution matches |
| Too many API calls per page load | Slow page, rate limits | Batch lookups, aggressive caching (5min positive, 30min negative), skip if already cached |

---

## No New Dependencies

All HTTP calls use the built-in `fetch` API available in Chrome MV3 service workers.
No new npm packages required for the extension.
No new Rust crates required for the Telegram bot.
