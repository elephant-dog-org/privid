# Feature Specification: PrivID v2 — Twitter Badges, Trust Network, Cross-Platform Registry

**Status**: Draft
**Created**: 2026-02-17
**Feature**: Cross-platform proof-of-personhood with individual trust signals

---

## Problem Statement

PrivID currently verifies Human Passport holders in Telegram group chats and Gmail inboxes. But the majority of crypto/web3 discourse happens on Twitter/X, where verification trust has eroded since the paid blue check era. Users have no way to know if a Twitter account belongs to a verified human with real credentials.

Additionally, knowing someone is verified is useful — but knowing that *people you trust* also trust that person is far more compelling. Individual trust signals ("4 verified people you follow also follow this person") provide social proof that a standalone badge cannot.

The current system also uses flat JSON files for persistence, which won't scale, and requires users to already have all prerequisites (wallet, ENS, Human Passport) with no guidance for those who don't.

---

## Users & Actors

| Actor | Description |
|-------|-------------|
| **Verified User** | Has a Human Passport (SBTs on Optimism), ENS name, and registered with PrivID |
| **Unverified User** | Doesn't yet have credentials; needs guidance through onboarding |
| **Viewer** | Anyone browsing Twitter/Gmail/Telegram who sees badges and trust signals |
| **Group Admin** | Telegram group admin who adds the bot |

---

## Functional Requirements

### FR-1: Twitter/X Badge Injection

- The browser extension injects verification badges next to display names on twitter.com / x.com
- Badges appear on tweet authors, profile pages, reply authors, and quoted tweets
- Badge lookup flow: extract @handle from DOM → check local registry → check ENS `com.twitter` text records → resolve wallet → verify Human Passport SBTs
- Results are cached locally with a reasonable TTL to avoid repeated lookups
- Badge includes a tooltip showing verification details (SBT types, ENS name)
- Works alongside existing Gmail and Bluesky badge injection

### FR-2: Cross-Platform Identity Registry

- A unified registry stores the mapping: platform identity → ENS name → wallet → verified SBT types
- Supports multiple platform identities per user (Twitter handle, Telegram username, email)
- Registration happens once via any entry point (extension popup, Telegram DM) and propagates to all platforms
- Extension popup provides a "Link Twitter" flow where users enter their @handle
- The bot provides `/register` which also captures Twitter if the ENS has a `com.twitter` text record
- Registry is stored in SQLite (replacing current JSON file persistence)

### FR-3: Individual Trust Network

- When viewing a Twitter profile or checking `/whois` in Telegram, show how many verified people the *viewer* follows/knows who also follow the target person
- Display format: "Trusted by 4 verified people you follow" or similar
- Trust data is gathered per-viewer: the extension reads the viewer's Twitter following list, cross-references with the PrivID registry
- Trust scores are computed locally in the extension (no server-side social graph needed for v1)
- In Telegram, `/whois` shows "N verified members of this group also verified" based on registry data

### FR-4: Onboarding Funnel

- When an unverified user encounters PrivID (via `/whois` nudge, extension popup, or Twitter badge click), they see a step-by-step guide:
  1. Get a wallet (link to popular options)
  2. Get your Human Passport at app.passport.xyz
  3. Get an ENS name at app.ens.domains
  4. Set text records (org.telegram, com.twitter) on your ENS
  5. Register with PrivID
- Each step shows whether the user has completed it (progressive checklist)
- The extension popup shows onboarding progress for connected wallets
- The Telegram bot guides users step-by-step when they DM `/register` without prerequisites

### FR-5: SQLite Migration

- Replace `data/registry.json` and `data/sessions.json` with a single SQLite database
- Maintain all existing functionality (register, deregister, lookup by user ID, lookup by username)
- Add indexes for wallet address and ENS name lookups
- Add a `platform_links` table for cross-platform identity mapping
- Support concurrent read/write access safely
- Migrate existing JSON data on first run if files exist

---

## User Scenarios & Testing

### Scenario 1: Twitter Badge — Verified User

**Given** a verified user (@alice) has registered with PrivID and has Human Passport SBTs
**When** another user browses twitter.com and sees @alice's tweet
**Then** a verification badge appears next to @alice's display name
**And** hovering shows "Verified — Human Passport | KYC, Phone | ENS: alice.eth"

### Scenario 2: Twitter Badge — Trust Signal

**Given** viewer follows 10 verified PrivID users on Twitter
**And** 4 of those verified users also follow @alice
**When** viewer sees @alice's profile
**Then** badge tooltip includes "Trusted by 4 verified people you follow"

### Scenario 3: New User Onboarding via Extension

**Given** a user installs the PrivID extension and connects their wallet
**And** they have no Human Passport and no ENS
**When** they open the extension popup
**Then** they see an onboarding checklist showing steps to complete
**And** each step links to the relevant service (app.passport.xyz, app.ens.domains)

### Scenario 4: New User Onboarding via Telegram

**Given** a user DMs the bot with `/register`
**And** they provide an ENS name that has no `org.telegram` text record
**When** the bot checks their ENS
**Then** the bot responds with specific instructions for setting the text record
**And** includes a direct link to app.ens.domains

### Scenario 5: Cross-Platform Registration

**Given** a user registers via Telegram with `/register alice.eth`
**And** alice.eth has `com.twitter` set to `@alice_web3`
**When** registration completes
**Then** the registry stores both the Telegram and Twitter identity links
**And** @alice_web3 shows a badge on twitter.com for extension users

### Scenario 6: Telegram Trust Signal

**Given** a group has 3 registered PrivID members
**When** someone uses `/whois @newuser` for a registered user
**Then** the response includes "Also verified: @member1, @member2 in this group"

---

## Success Criteria

- Verification badges appear on twitter.com within 2 seconds of page load for cached users
- First-time lookups complete within 5 seconds (ENS resolution + SBT query)
- Users can complete registration from zero (no wallet) to fully verified badge in one session following the onboarding guide
- Trust signals display for at least 80% of verified profiles viewed (cache hit rate)
- Cross-platform registration works: register on Telegram, badge appears on Twitter
- SQLite migration preserves all existing registrations with zero data loss
- Extension size remains under 500KB (excluding ethers.js chunks)

---

## Scope & Boundaries

### In Scope

- Twitter/X content script with badge injection
- SQLite database for Telegram bot
- Individual trust signals (per-viewer, based on their follow list)
- Onboarding guide in extension popup and Telegram bot
- Cross-platform identity linking via ENS text records
- Caching layer for Twitter handle → verification lookups

### Out of Scope

- Full social graph analysis or visualization
- Server-side infrastructure (everything runs client-side or in the bot)
- Farcaster/Lens integration (future phase)
- Twitter API paid tier integration (use ENS records + local registry for v1)
- Mobile app
- Automated ENS text record setting (users do this manually)

---

## Assumptions

- ENS `com.twitter` text records are the standard way to link Twitter handles (widely adopted in crypto community)
- Twitter's DOM structure for tweet authors and profile names is stable enough for content script injection (may need periodic maintenance)
- The Human Passport Hub contract on Optimism remains at the same address with the same ABI
- Users are willing to set ENS text records as the trust anchor (vs wallet signature alternatives)
- The extension can read the viewer's Twitter following list from the DOM without needing Twitter API access
- SQLite via `rusqlite` crate is sufficient for the Telegram bot's concurrency needs

---

## Dependencies

- Human Passport (app.passport.xyz) — SBT verification source
- ENS (Ethereum Name Service) — identity linking via text records
- Optimism RPC — SBT queries
- Ethereum mainnet RPC — ENS resolution
- Chrome Extension APIs — storage, content scripts
- Twitter/X DOM structure — badge injection targets

---

## Key Entities

| Entity | Attributes |
|--------|------------|
| **Identity** | wallet_address, ens_name, verified_sbt_types[], registered_at, last_verified |
| **PlatformLink** | identity_id, platform (twitter/telegram/email), platform_handle, verified_via_ens |
| **TrustCache** | viewer_id, target_handle, mutual_verified_count, cached_at |
| **OnboardingState** | wallet_connected, has_human_passport, has_ens, has_text_records[], registered |
