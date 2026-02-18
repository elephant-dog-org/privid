# Tasks: PrivID v3 — Passive Verification (BYOK)

**Spec**: .speckit/spec.md
**Plan**: .speckit/plan.md
**Created**: 2026-02-17
**Amended**: 2026-02-17 (post-analyze fixes)

---

## Phase 1: Free Provider Modules

### Task 1.1: Next.ID provider [P]
- Create `background/providers/nextid.ts`
- Implement `queryNextId(handle: string): Promise<string[]>` (returns wallet addresses)
- Query: `GET https://proof-service.next.id/v1/proof?platform=twitter&identity={handle}&exact=true`
- Parse proof graph: find ethereum platform proofs, extract wallet addresses
- Handle: empty results, network errors, malformed responses
- Normalize wallet addresses to **lowercase** for internal comparison/dedup (display as EIP-55 checksummed when shown to users)
- Per-provider timeout: 3 seconds via `AbortSignal.timeout(3000)`
- Use raw `chrome.*` API, NOT `webextension-polyfill` (background context)
- **Tests**: Unit tests with mocked fetch responses (proof found, no proof, network error)

### Task 1.2: Holonym provider [P]
- Create `background/providers/holonym.ts`
- Implement `checkHolonymSBTs(wallet: string): Promise<string[]>` (returns human-readable credential type names)
- Check 4 endpoints: gov-id, epassport, phone, biometrics
- URL pattern: `https://api.holonym.io/sybil-resistance/{type}/optimism?user={wallet}&action-id=123456789`
- Run all 4 checks in parallel, collect those returning `{ "result": true }`
- **Credential name mapping**: Normalize API names to display names: `gov-id` → `KYC`, `epassport` → `ePassport`, `phone` → `Phone`, `biometrics` → `Biometrics`
- **Fallback**: If Holonym HTTP API is unreachable (all 4 calls fail), export a flag so the resolver can fall back to direct on-chain SBT checking via existing `blockchain/utils.ts` module (`getSBTByCircuitId`). This satisfies Constitution Principle 4 (Graceful Degradation).
- Handle: network errors, unexpected response shapes
- Per-provider timeout: 3 seconds
- Use raw `chrome.*` API (background context)
- **Tests**: Unit tests with mocked responses (all true, mixed, all false, network error, API unreachable)

### Task 1.3: ensdata provider [P]
- Create `background/providers/ensdata.ts`
- Implement `queryEnsdata(wallet: string): Promise<{ ensName: string; records: Record<string, string> } | null>`
- Query: `GET https://ensdata.net/{wallet_address}`
- Parse response: extract `ens` field and `records` object
- Handle: no ENS name found (return null), network errors
- Per-provider timeout: 3 seconds
- Use raw `chrome.*` API (background context)
- **Tests**: Unit tests with mocked responses (ENS found with records, no ENS, error)

---

## Phase 2: BYOK Provider Modules

### Task 2.1: Neynar/Farcaster provider [P]
- Create `background/providers/neynar.ts`
- Implement `queryNeynar(handle: string, apiKey: string): Promise<{ wallets: string[]; fid?: number }>` (returns wallet addresses AND Farcaster FID)
- Query: `GET https://api.neynar.com/v2/farcaster/user/by_x_username/?x_username={handle}`
- Header: `x-api-key: {apiKey}`
- Parse: extract `users[0].verified_addresses.eth_addresses` AND `users[0].fid`
- Implement `testNeynarKey(apiKey: string): Promise<boolean>` — test with a known handle
- Handle: 401 (bad key), 404 (user not found), rate limits
- Per-provider timeout: 3 seconds
- Use raw `chrome.*` API (background context)
- **Tests**: Unit tests with mocked responses (user found, not found, bad key)

### Task 2.2: Gitcoin Passport provider [P]
- Create `background/providers/passport.ts`
- Implement `queryPassport(wallet: string, apiKey: string, scorerId: string): Promise<{ score: number; passing: boolean } | null>`
- Query: `GET https://api.scorer.gitcoin.co/registry/v2/score/{scorerId}/{wallet}`
- Header: `X-API-KEY: {apiKey}`
- Parse: extract `score` (string → number) and `passing_score` (boolean)
- Implement `testPassportKey(apiKey: string, scorerId: string): Promise<boolean>`
- **Note**: This provider requires TWO user inputs: API Key AND Scorer ID. Task 4.1 must provide two input fields for this provider.
- Handle: 401, 404, rate limits
- Per-provider timeout: 3 seconds
- Use raw `chrome.*` API (background context)
- **Tests**: Unit tests with mocked responses (score found, not found, bad key)

### Task 2.3: ENS Subgraph provider [P]
- Create `background/providers/ensSubgraph.ts`
- Implement `queryEnsSubgraph(handle: string, apiKey: string): Promise<string[]>` (returns wallet addresses)
- GraphQL POST to `https://gateway.thegraph.com/api/{apiKey}/subgraphs/id/5XqPmWe6gjyrJtFn9cLy237i4cWw2j9HcUJEXsP5qGtH`
- Query: `textChangeds(where: { key: "com.twitter", value: "@{handle}" })` → get resolver → domain name
- Then forward-resolve domain name to current wallet (verify it's not stale)
- Implement `testGraphKey(apiKey: string): Promise<boolean>`
- Handle: stale data, missing resolvers, rate limits
- Per-provider timeout: 3 seconds
- Use raw `chrome.*` API (background context)
- **Tests**: Unit tests with mocked GraphQL responses

---

## Phase 3: Identity Resolver + Service Worker

### Task 3.0: Update manifest.json host_permissions [CRITICAL]
- Modify `apps/extension/manifest.json`
- Add `host_permissions` for all external API domains:
  ```json
  "host_permissions": [
      "https://proof-service.next.id/*",
      "https://api.holonym.io/*",
      "https://ensdata.net/*",
      "https://api.neynar.com/*",
      "https://api.scorer.gitcoin.co/*",
      "https://gateway.thegraph.com/*"
  ]
  ```
- **Without this, ALL service worker fetch calls to external APIs will be silently blocked by Chrome MV3.**
- This MUST be done before any provider testing.
- **Tests**: Extension loads without permission errors

### Task 3.1: Identity resolver orchestrator
- Create `background/identityResolver.ts`
- Define `IdentityResult` interface:
  ```typescript
  interface IdentityResult {
      wallet: string;           // lowercase for internal use
      verified: boolean;
      sources: string[];        // e.g. ['nextid', 'neynar']
      ensName?: string;
      sbtTypes?: string[];      // human-readable: ['KYC', 'Phone']
      passportScore?: number;
      farcasterFid?: number;
  }
  ```
- Implement `resolveTwitterIdentity(handle: string): Promise<IdentityResult | null>`
- **Execution strategy**: Run all free providers in parallel (`Promise.allSettled`), then run all configured BYOK providers in parallel. Two tiers, each parallel internally. Overall pipeline timeout: 5 seconds.
- Waterfall logic:
  1. Query all enabled handle→wallet providers in parallel
  2. Deduplicate wallets — normalize to **lowercase** for comparison
  3. For each unique wallet: check Holonym SBTs (free API first, fall back to on-chain `blockchain/utils.ts` if API unreachable)
  4. For first verified wallet: enrich with ensdata + passport
  5. Track `sources[]` — include ALL providers that returned this wallet, even if deduplicated
  6. Return IdentityResult or null
- Bot API fallback: if `prividApiUrl` is configured in storage, query it as an additional source. Convert bot API response `{ verified, sbt_types, ens_name, wallet_address }` to IdentityResult format with `sources: ['privid-bot']`. Merge: prefer the richer result.
- API key helpers: `getApiKey(provider)`, `setApiKey(provider, key)`, `testApiKey(provider, key)` — use raw `chrome.storage.local` API (background context)
- **Tests**: Unit tests with mocked providers (all succeed, some fail, none return data, multiple wallets returning same address, wallet found but no SBTs)

### Task 3.2: Rewrite service worker to use identity resolver
- Modify `background/service-worker.ts`
- Import `resolveTwitterIdentity` from identityResolver (Vite will bundle all imported provider modules into the single IIFE output — no config change needed, but verify imports are correct)
- Replace `fetchFromApi()` with `resolveTwitterIdentity()`
- Update cache:
  - Rename storage key from `twitterRegistry` to `privid_identity_cache` to avoid v2 data conflicts
  - Store full `IdentityResult` objects keyed by normalized handle
  - **Dual TTL**: Use `POSITIVE_CACHE_TTL = 5 * 60 * 1000` and `NEGATIVE_CACHE_TTL = 30 * 60 * 1000`. Select TTL based on `verified` boolean on the cached entry.
- Add `testProviderKey` message handler: accepts `{ action: 'testProviderKey', provider: string, key: string, scorerId?: string }`, delegates to the appropriate provider's test function, returns `{ success: boolean, error?: string }`
- Add `clearIdentityCache` message handler: clears all cached entries (called when user adds/removes API key)
- Keep existing `lookupTwitterHandle` and `openPopup` message handlers
- **Tests**: Build passes, service worker loads without errors

### Task 3.3: Update content script to handle new response format
- Modify `content/twitter/injectTwitterBadge.ts`
- Update `lookupHandle()` to expect IdentityResult shape from service worker
- Update `TwitterCacheEntry` in `twitterCache.ts` to include ALL new fields:
  - Add: `sources: string[]`, `passportScore?: number`, `farcasterFid?: number`
  - Note: existing field is `walletAddress` — keep this name (don't rename to `wallet`) for backward compat with in-memory cache consumers
  - Map `IdentityResult.wallet` → `TwitterCacheEntry.walletAddress` in the message handler
- Update in-memory cache TTL to support dual TTLs (check `verified` field)
- Badge still shows/hides based on `verified` boolean — no functional change to injection logic
- **Tests**: Build passes, badges still appear for cached verified entries

---

## Phase 4: Popup Settings UI

### Task 4.1: Provider settings HTML [P after 3.0]
- Modify `popup/popup.html`
- Replace the "API Settings" `<details>` section with an "Identity Providers" panel
- Place BELOW the existing Link Accounts section, visible regardless of mock mode state (provider config is independent of wallet auth)
- Layout per provider: name, description, free/paid badge, key input (if paid), test button, status indicator
- Provider list:
  1. Next.ID — free, always enabled, no config (show green status dot)
  2. Holonym — free, always enabled, no config (show green status dot)
  3. ensdata.net — free, always enabled, no config (show green status dot)
  4. Neynar/Farcaster — needs API key (1 input field)
  5. Gitcoin Passport — needs API Key AND Scorer ID (**2 input fields**)
  6. The Graph ENS — needs API key (1 input field)
  7. Self-hosted PrivID Bot — optional, URL input (relabel from "Bot API URL" to make clear this is user-operated)
- Use collapsible `<details>` to keep compact
- Update onboarding checklist: remove "Register with PrivID" step (no longer primary path). Replace with "Configure providers" or simplify to a status display.

### Task 4.2: Provider settings logic
- Modify `popup/popup.ts`
- Load/save API keys using **`browser.storage.local`** (webextension-polyfill — matches existing popup code pattern, NOT raw `chrome.*`)
- Store under `apiKeys` object: `{ neynar: string, passport: string, passportScorerId: string, theGraph: string }`
- Test button handlers: use `browser.runtime.sendMessage({ action: 'testProviderKey', provider, key, scorerId? })` to delegate test to service worker
- On key add/remove: call `browser.runtime.sendMessage({ action: 'clearIdentityCache' })` to bust cache so new key takes effect immediately
- Visual feedback: green dot (working), gray dot (unconfigured), red dot (failed test)
- **Tests**: Build passes, keys persist across popup open/close, cache clears on key change

---

## Phase 5: Badge Tooltip Enhancement

### Task 5.1: Rich tooltip content
- Modify `content/twitter/injectTwitterBadge.ts`
- Update `createBadgeElement()` to accept full IdentityResult (via the updated TwitterCacheEntry)
- Tooltip lines:
  - "Verified via Holonym" (always)
  - Credential types: "KYC, Phone" (from sbtTypes, using human-readable names from Task 1.2 mapping)
  - ENS: "alice.eth" (if ensName present)
  - Passport: "Score: 24.5" (if passportScore present)
  - Sources: "Confirmed by: Next.ID, Farcaster" (from sources array)
  - Trust: "Trusted by N verified people you know" (existing trustNetwork, unchanged)
- Style the tooltip for readability (existing CSS + minor additions)
- **Tests**: Visual verification with mock data showing all tooltip lines

---

## Phase 6: Telegram Bot Passive Lookup

### Task 6.1: Next.ID fallback in /whois [P]
- Modify `apps/telegram-bot/src/main.rs`
- In `/whois` handler: after local registry miss, query Next.ID
  - `GET https://proof-service.next.id/v1/proof?platform=telegram&identity={username}&exact=true`
- Parse response (same structure as Twitter lookup — extract ethereum wallet proofs)
- If wallet found: check Holonym SBTs using existing verification provider
- Format response: "Discovered via Next.ID (not registered with bot)"
- If nothing found: existing "not registered" message
- **Best-effort**: If Next.ID does not support `telegram` as a platform (returns empty or error), degrade gracefully to existing registry-only behavior. This is a no-op fallback, not a failure.
- Uses existing `reqwest` — no new dependencies
- **Tests**: cargo test passes, manual test with a known Next.ID user

---

## Task Dependencies

```
                    3.0 (manifest — do FIRST)
                     │
1.1 ─┐               │
1.2 ─┤               │
1.3 ─┼─→ 3.1 → 3.2 ─┼─→ 3.3 ─→ 5.1
2.1 ─┤              │
2.2 ─┤              └─→ 4.1 → 4.2
2.3 ─┘

6.1 (independent, parallel with any phase)
```

**Parallelizable**:
- Task 3.0 (manifest) can be done first as a quick prerequisite
- All Phase 1 tasks (1.1, 1.2, 1.3) are independent of each other
- All Phase 2 tasks (2.1, 2.2, 2.3) are independent of each other
- Phases 1 and 2 are independent of each other (all 6 provider modules can be built in parallel)
- Task 6.1 is independent of everything else
- Task 4.1 (HTML only) can start in parallel with Phase 3 since it only needs the provider list from the spec
- Tasks 4.2 and 5.1 can run in parallel after 3.2

---

## Definition of Done

- [ ] `manifest.json` includes `host_permissions` for all external API domains
- [ ] Extension builds without errors (all vite configs)
- [ ] Free baseline works: Next.ID + Holonym resolve at least one known verified handle
- [ ] BYOK providers work when keys are configured
- [ ] Holonym API failure falls back to on-chain SBT checking
- [ ] Provider failures are silent — no console errors, no broken UI
- [ ] API keys are stored locally and only sent to their respective endpoints
- [ ] Cache clears when user adds/removes API keys
- [ ] Badge tooltips show all available verification details
- [ ] Telegram bot `/whois` tries Next.ID fallback for unregistered users
- [ ] cargo test passes for Telegram bot
- [ ] No regression in existing Gmail/Bluesky/Telegram functionality
- [ ] Background modules use raw `chrome.*` API; popup modules use `webextension-polyfill`
