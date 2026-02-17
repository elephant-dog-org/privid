# Implementation Plan: PrivID v2

**Spec**: .speckit/spec.md
**Created**: 2026-02-17

---

## Architecture Overview

```
Browser Extension (TypeScript)
├── content/twitter/     ← NEW: Twitter badge injection
├── content/gmail/       ← Existing: Gmail badge injection
├── content/             ← Existing: Bluesky badge injection
├── popup/               ← Modified: onboarding funnel + link Twitter
└── blockchain/          ← Modified: shared lookup utils

Telegram Bot (Rust)
├── src/db/              ← NEW: SQLite (replaces storage.rs + registry.rs JSON)
├── src/ens/             ← Existing: ENS resolution
├── src/registry.rs      ← Modified: backed by SQLite
└── src/main.rs          ← Modified: cross-platform registration + trust signals
```

---

## Phase 1: SQLite Migration (Telegram Bot)

**Goal**: Replace JSON file storage with SQLite. Foundation for everything else.

### Changes

**New file: `src/db.rs`**
- SQLite connection pool using `rusqlite` with `tokio` wrapper (`tokio-rusqlite`)
- Schema: `identities` table (wallet, ens_name, sbt_types, registered_at, last_verified)
- Schema: `platform_links` table (identity_id, platform, handle, verified_via_ens)
- Schema: `sessions` table (migrated from current state.rs)
- Migration logic: detect existing JSON files, import, rename to `.migrated`
- Indexes on wallet_address, ens_name, platform+handle

**Modified: `src/registry.rs`**
- Replace `RwLock<HashMap>` with SQLite queries
- Same public API: register(), deregister(), lookup_by_user_id(), lookup_by_username()
- Add: lookup_by_twitter_handle(), lookup_by_wallet()
- Add: link_platform(), get_platform_links()

**Modified: `Cargo.toml`**
- Add `rusqlite` with `bundled` feature (compiles SQLite from source, no system dep)
- Add `tokio-rusqlite` for async wrapper

**Modified: `src/main.rs`**
- Pass `Database` instead of separate `BotState` + `Registry`
- Existing commands work unchanged

### New Dependencies
- `rusqlite = { version = "0.33", features = ["bundled"] }`
- `tokio-rusqlite = "0.6"`

---

## Phase 2: Twitter Content Script (Extension)

**Goal**: Inject verification badges on twitter.com / x.com.

### Changes

**New file: `content/twitter/injectTwitterBadge.ts`**
- MutationObserver pattern (same as Gmail content script)
- Target selectors: tweet author names, profile display names, reply authors
- Extract @handle from user link hrefs or `data-testid` attributes
- Lookup chain: local cache → extension storage registry → ENS `com.twitter` reverse lookup
- Badge injection: `<span class="privid-twitter-badge">` with SVG + tooltip
- Debounced scanning (300ms, same as Gmail pattern)
- Cache results in extension storage with 1-hour TTL

**New file: `content/twitter/injectTwitterBadge.css`**
- Badge styling consistent with Gmail badge (blue checkmark, drop shadow)
- Twitter-specific positioning (inline with display names)

**New file: `content/twitter/twitterCache.ts`**
- Same pattern as `gmail/emailCache.ts`
- Key: twitter handle → { verified, sbtTypes, ensName, walletAddress, timestamp }

**New file: `vite.twitter.config.ts`**
- IIFE build config (same pattern as vite.gmail.config.ts)
- Entry: `content/twitter/injectTwitterBadge.ts`
- Output: `dist/content/twitter/`

**Modified: `manifest.json`**
- Add content script entry for `https://twitter.com/*` and `https://x.com/*`

**Modified: `package.json`**
- Add twitter build to the `build` script chain

**Modified: `scripts/package-extension.sh`**
- Copy Twitter CSS to dist/content/twitter/

### Twitter Handle → Wallet Resolution

The extension uses this lookup chain for a Twitter @handle:

1. **Local registry** (extension storage): Check if handle is already mapped
2. **ENS reverse lookup**: This is the hard part. We can't easily go from @handle → ENS name since ENS doesn't index by text record values. Two approaches:
   - **Registry-first (v1)**: Only badge users who registered via PrivID (we know their handle). Fast, no extra RPC calls, but requires user action.
   - **ENS indexer (v2, future)**: Use an ENS indexing service to find names with matching `com.twitter` records. Deferred — adds complexity and potential costs.

For v1: badge injection works for registered PrivID users only. The onboarding funnel encourages others to register.

---

## Phase 3: Cross-Platform Registry (Extension + Bot)

**Goal**: Register once, verified everywhere.

### Changes

**Modified: `src/main.rs` (Telegram bot)**
- During `/register name.eth`, also read `com.twitter` and `com.email` ENS text records
- Store all discovered platform links in `platform_links` table
- `/status` shows all linked platforms

**New section in `popup/popup.html` + `popup/popup.ts`**
- "Link Accounts" section in extension popup
- Shows current linked platforms (Twitter, Telegram, Email)
- "Link Twitter" button: user enters @handle, stored in extension registry
- "Link Telegram" button: instructs user to DM the bot
- Progressive display: green checkmarks for linked platforms

**Shared registry format** (extension storage):
```typescript
interface RegistryEntry {
  walletAddress: string;
  ensName: string;
  sbtTypes: string[];
  platforms: {
    twitter?: string;    // @handle
    telegram?: string;   // username
    email?: string;      // hashed
  };
  registeredAt: string;
  lastVerified: string;
}
```

**Note**: For v1, the extension and bot maintain separate registries (extension in browser storage, bot in SQLite). They sync indirectly via ENS text records. A shared backend is out of scope.

---

## Phase 4: Individual Trust Network

**Goal**: Show per-viewer trust signals ("N verified people you follow also follow this person").

### Twitter Trust Signals

**Modified: `content/twitter/injectTwitterBadge.ts`**
- After injecting badge, compute trust score for that profile
- Read viewer's cached following list from extension storage
- Cross-reference: how many of viewer's followees are (a) in PrivID registry AND (b) follow the target user
- Display in tooltip: "Trusted by N verified people you follow"

**New file: `content/twitter/trustNetwork.ts`**
- `getViewerFollowing()`: Scrape viewer's following list from Twitter DOM (lazy, cached)
  - Navigate to twitter.com/following programmatically? No — too invasive
  - Instead: build following list passively as viewer browses (observe tweet authors, profile visits)
  - Store in extension storage, grows over time
- `computeTrustScore(targetHandle, viewerFollowing, registry)`: Count mutual verified connections
- Cache trust scores with TTL

**Key design decision**: We do NOT scrape Twitter's following list aggressively. Instead:
1. As the viewer browses Twitter, we passively collect handles they interact with
2. When they visit someone's profile, we check: "of the verified PrivID users we've seen this viewer engage with, how many also engage with this target?"
3. This is an approximation, not an exact follower count — but it's privacy-respecting and doesn't require API access

### Telegram Trust Signals

**Modified: `src/main.rs`**
- `/whois` in a group: after showing verification info, also list other verified members in the same group
- "Also verified in this group: @alice, @bob, @charlie"
- Uses existing registry data + badge tracker to know who's in the group

---

## Phase 5: Onboarding Funnel

**Goal**: Guide unverified users through getting credentials.

### Extension Onboarding

**Modified: `popup/popup.html` + `popup/popup.ts`**
- New "Get Verified" section (shown when wallet connected but not fully registered)
- Progressive checklist:
  - [ ] Connect wallet (already done if showing this)
  - [ ] Human Passport — link to app.passport.xyz + auto-check SBTs
  - [ ] ENS name — link to app.ens.domains + auto-check reverse resolution
  - [ ] Text records set — auto-check org.telegram / com.twitter
  - [ ] Registered with PrivID
- Each step auto-detects completion by querying on-chain state
- CTA button advances to next incomplete step

### Telegram Onboarding

**Modified: `src/main.rs`**
- When `/register` fails (no ENS, no text record, no SBTs), provide specific next-step guidance
- New `/setup` command: shows full onboarding checklist with status
- Links to app.passport.xyz, app.ens.domains with instructions

---

## Implementation Order

| Phase | Effort | Dependencies |
|-------|--------|--------------|
| Phase 1: SQLite Migration | Medium | None |
| Phase 2: Twitter Content Script | Medium | Phase 1 (registry format) |
| Phase 3: Cross-Platform Registry | Small | Phase 1 + Phase 2 |
| Phase 4: Trust Network | Medium | Phase 2 + Phase 3 |
| Phase 5: Onboarding Funnel | Small | Phase 3 (knows what's missing) |

Phases 1 and 2 can be worked in parallel since they're in different codebases (Rust vs TypeScript).

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Twitter DOM changes break badge injection | Badges stop appearing | Use semantic selectors where possible, add version-specific fallbacks |
| ENS text record adoption is low | Few users to badge | Registry-first approach (users register explicitly), onboarding funnel |
| Twitter anti-extension measures | Extension blocked | Content script is read-only (no API calls to Twitter), low detection risk |
| Human Passport contract changes | SBT queries break | Abstract behind provider trait (already done), easy to update |
| SQLite file locking under load | Bot hangs on writes | WAL mode, connection pooling, async wrapper |
