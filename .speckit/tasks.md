# Tasks: PrivID v2

**Spec**: .speckit/spec.md
**Plan**: .speckit/plan.md
**Created**: 2026-02-17

---

## Phase 1: SQLite Migration

### Task 1.1: Add SQLite dependencies and create db module
- Add `rusqlite` (bundled) and `tokio-rusqlite` to Cargo.toml
- Create `src/db.rs` with connection init, schema creation (identities, platform_links, sessions tables)
- Create indexes on wallet_address, ens_name, platform+handle
- Write migration logic: detect existing JSON files, import data, rename to `.migrated`
- **Tests**: Schema creation, migration from test JSON fixtures

### Task 1.2: Refactor registry to use SQLite
- Replace `RwLock<HashMap>` in registry.rs with SQLite queries
- Maintain same public API: register(), deregister(), lookup_by_user_id(), lookup_by_username()
- Add new methods: lookup_by_twitter_handle(), lookup_by_wallet(), link_platform()
- Remove old JSON persistence code
- **Tests**: All existing registry tests pass against SQLite, plus new lookup tests

### Task 1.3: Wire SQLite into main.rs
- Replace BotState + Registry initialization with Database
- Update dependency injection in dispatcher
- Verify all existing commands work unchanged
- **Tests**: cargo test passes, manual test of /register + /whois in mock mode

---

## Phase 2: Twitter Content Script

### Task 2.1: Create Twitter content script scaffold
- Create `content/twitter/injectTwitterBadge.ts` with MutationObserver pattern
- Create `content/twitter/injectTwitterBadge.css` with badge styling
- Create `content/twitter/twitterCache.ts` for handle→verification cache
- Create `vite.twitter.config.ts` (IIFE build, same pattern as gmail config)
- Add to manifest.json: content script for twitter.com/* and x.com/*
- Update package.json build script and package-extension.sh
- **Tests**: Extension builds successfully, content script loads on twitter.com

### Task 2.2: Implement Twitter handle lookup and badge injection
- DOM selectors for tweet authors, profile names, reply authors
- Lookup chain: cache → extension storage registry → (future: ENS reverse)
- Badge injection with SVG + tooltip showing SBT types and ENS name
- Debounced scanning (300ms) for Twitter's infinite scroll
- Cache with 1-hour TTL
- **Tests**: Manual test with mock registry data, badge appears on known handles

### Task 2.3: Handle Twitter DOM variations
- Test across: tweet view, profile page, reply threads, quoted tweets, search results
- Add fallback selectors for different Twitter layouts (old vs new)
- Handle dark mode / light mode styling
- Ensure badges don't break Twitter's layout or interaction (clicks, hover states)
- **Tests**: Visual verification across 5+ Twitter page types

---

## Phase 3: Cross-Platform Registry

### Task 3.1: Extend Telegram registration to capture Twitter
- During `/register name.eth`, also read `com.twitter` ENS text record
- Store Twitter handle in platform_links table
- `/status` shows all linked platforms
- **Tests**: Register with ENS that has com.twitter set, verify platform link stored

### Task 3.2: Add "Link Accounts" to extension popup
- New section in popup showing linked platforms (Twitter, Telegram, Email)
- "Link Twitter" input + button: stores @handle in extension registry
- "Link Telegram" instructions (DM the bot)
- Green checkmarks for linked platforms
- **Tests**: UI renders, linking stores data, re-opening popup shows linked state

### Task 3.3: Sync registry entries for badge injection
- Twitter content script reads from extension's local registry
- When user registers via popup, their handle becomes badgeable
- When user registers via Telegram bot (and has com.twitter), handle becomes badgeable on next extension sync
- **Tests**: Register via popup, see badge on twitter.com for that handle

---

## Phase 4: Trust Network

### Task 4.1: Passive following collection in Twitter content script
- As viewer browses, collect handles of verified users they see/interact with
- Store in extension storage: `viewerNetwork: { handle: lastSeen }[]`
- Cap at 1000 entries, LRU eviction
- **Tests**: Browse Twitter, verify handles accumulate in storage

### Task 4.2: Compute and display trust scores on Twitter
- When badge is injected, compute: how many of viewer's collected verified handles also appear as connections of the target
- Display in tooltip: "Trusted by N verified people you follow"
- For v1: "connections" = both viewer and target are in the verified registry (simple overlap)
- Cache trust scores per target handle
- **Tests**: Mock viewer network + registry, verify trust count displays correctly

### Task 4.3: Add trust signals to Telegram /whois
- `/whois @user` in a group: list other verified members in the same group
- "Also verified in this group: @alice, @bob"
- Uses registry data + badge tracker
- **Tests**: Register 3 users in a group, /whois shows mutual verified members

---

## Phase 5: Onboarding Funnel

### Task 5.1: Extension onboarding checklist
- New "Get Verified" section in popup (shown when wallet connected but incomplete)
- Progressive checklist: wallet, Human Passport, ENS, text records, registered
- Auto-detect completion by querying on-chain state
- Links to app.passport.xyz, app.ens.domains
- **Tests**: Connect wallet with no credentials → see full checklist. Connect with SBTs → see partial completion.

### Task 5.2: Telegram onboarding guidance
- Improve error messages in `/register` to give specific next steps
- New `/setup` command: full checklist with completion status
- Check wallet for SBTs, check ENS for text records, report what's missing
- **Tests**: DM /setup with incomplete credentials, verify step-by-step guidance

### Task 5.3: Unverified user nudge on Twitter
- When badge is NOT found for a profile, optionally show a subtle "Get verified" indicator
- Only on the viewer's own profile (not other people's profiles — that would be rude)
- Links to extension popup onboarding flow
- **Tests**: View own Twitter profile without PrivID registration, see CTA

---

## Task Dependencies

```
1.1 → 1.2 → 1.3
                 \
2.1 → 2.2 → 2.3 → 3.1 → 3.3
                    3.2 → 3.3 → 4.1 → 4.2
                                  4.3
                              5.1
                    1.3 ────→ 5.2
                              5.3 (after 2.2)
```

**Parallelizable**: Phase 1 (Rust) and Tasks 2.1-2.3 (TypeScript) can run simultaneously.

---

## Definition of Done

- [ ] All tests pass (cargo test + bun test + manual verification)
- [ ] Extension builds and loads in Chrome without errors
- [ ] Telegram bot runs in mock mode with all new commands working
- [ ] Twitter badges appear for registered users
- [ ] Trust signals display in badge tooltips
- [ ] Onboarding flow guides new users through all prerequisites
- [ ] SQLite migration preserves existing data
- [ ] No regression in existing Gmail/Bluesky/Telegram functionality
