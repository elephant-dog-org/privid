# Feature Specification: PrivID v3 — Zero-Registration Passive Verification (BYOK)

**Status**: Draft
**Created**: 2026-02-17
**Feature**: Client-side passive identity verification with bring-your-own-key model

---

## Problem Statement

PrivID v2 requires every user to register with the system before they can be identified as verified. This creates unacceptable friction: the people who would benefit most from being badged (verified humans on Twitter/X) are exactly the people who won't go through a multi-step registration flow.

Meanwhile, the data needed to verify someone already exists publicly: wallet addresses linked to social handles (via Next.ID, Farcaster), identity attestations on-chain (Holonym SBTs), and ENS records. The system should read this public data directly rather than building its own registration silo.

Additionally, running a centralized API server to serve verification lookups creates operational costs and a single point of failure. The extension should function entirely client-side, querying public APIs directly from the user's browser.

---

## Users & Actors

| Actor | Description |
|-------|-------------|
| **Extension User** | Installs the PrivID browser extension; browses Twitter/X and sees verification badges on verified humans. May optionally configure API keys for enhanced coverage. |
| **Verified Human** | Holds Holonym SBT credentials and has linked their social identity via Next.ID, Farcaster, or ENS text records. Does NOT need to register with PrivID — discovered automatically. |
| **Bot Operator** | Runs the PrivID Telegram bot on their own machine. The bot serves group chat verification independently from the extension. |
| **Group Member** | Participates in Telegram groups where the PrivID bot operates. |

---

## Functional Requirements

### FR-1: Passive Twitter/X Identity Resolution

- The extension automatically identifies verified humans on Twitter/X without requiring them to register with PrivID
- Resolution pipeline: extract @handle from DOM → query identity providers for linked wallet → check wallet for Holonym SBTs → show badge
- A free baseline MUST work with zero configuration using providers that require no API keys
- Enhanced coverage MUST be available when the user provides their own API keys for additional providers
- Results are cached locally with time-based expiry to minimize repeated queries
- Failed or unavailable providers are skipped silently — the system tries all available sources and shows a badge if any source confirms verification

### FR-2: Multi-Provider Identity Resolution

- The system queries multiple identity providers in a waterfall pattern (free sources first, then paid if keys are configured):
  - **Free, no key required**: Next.ID ProofService (twitter handle → wallet), Holonym API (wallet → SBT check), ensdata.net (wallet → ENS records)
  - **Optional, user provides key**: Neynar/Farcaster API (twitter handle → wallet via Farcaster verification), The Graph ENS subgraph (ENS text record reverse lookup), Gitcoin Passport (wallet → humanity score)
- Each provider is a self-contained module that can fail independently without affecting others
- Provider results are merged: if multiple sources return different wallets, each wallet is checked for SBTs
- The system MUST be extensible — adding a new identity provider should not require changing the core resolution logic

### FR-3: BYOK API Key Management

- The extension popup provides a settings panel where users can enter their own API keys for paid providers
- Keys are stored in the user's local browser storage only
- Keys are sent only to the respective API endpoint they authenticate against
- Each provider has a "Test" function to verify the key works before relying on it
- The settings panel clearly indicates which providers are free and which require a key
- Users who provide no keys still get the free baseline functionality

### FR-4: Rich Verification Badge

- Badges on Twitter/X show richer information than v2:
  - Verification source ("Verified via Holonym")
  - Credential types held (KYC, Phone, ePassport, Biometrics)
  - ENS name if available
  - Gitcoin Passport score if available and user has configured the key
  - Which identity sources confirmed the link ("Confirmed by: Next.ID, Farcaster")
- Trust network signals from v2 continue to work ("Trusted by N verified people you know")

### FR-5: Telegram Bot Enhancement

- The bot's `/whois` command gains a passive lookup fallback: if a user is not in the local registry, query Next.ID by Telegram username as a secondary source
- Existing registration flow (`/register name.eth`) continues to work for users who want explicit registration
- The bot remains a self-contained, self-hosted tool with no dependency on the extension or any PrivID-operated service

---

## User Scenarios & Testing

### Scenario 1: Zero-Config Badge Discovery

**Given** a user installs the PrivID extension with no configuration
**And** browses to a Twitter profile of someone who has a Holonym SBT and a Next.ID proof linking their Twitter handle to their wallet
**When** the page loads
**Then** a verification badge appears next to that person's display name
**And** no API keys, registration, or setup was required

### Scenario 2: Enhanced Coverage with BYOK

**Given** a user has configured their Neynar API key in the extension settings
**And** browses to a Twitter profile of someone who verified their X handle on Farcaster but does NOT have a Next.ID proof
**When** the page loads
**Then** the Neynar provider resolves the handle via Farcaster
**And** a verification badge appears (that would NOT have appeared without the key)

### Scenario 3: Badge Tooltip Details

**Given** a verified user's badge is visible on Twitter
**When** the viewer hovers over the badge
**Then** they see: verification source, credential types, ENS name (if any), confirmation sources
**And** optionally: Passport score (if viewer configured that key), trust network count

### Scenario 4: Provider Graceful Failure

**Given** Next.ID is temporarily unavailable
**And** the user has a Neynar API key configured
**When** the extension tries to resolve a Twitter handle
**Then** the Next.ID lookup fails silently
**And** the Neynar lookup succeeds and returns the badge
**And** no errors are visible to the user

### Scenario 5: API Key Configuration

**Given** a user opens the extension popup and navigates to provider settings
**When** they enter their Neynar API key and click "Test"
**Then** the extension makes a test query to Neynar
**And** shows "Connected" with a success indicator
**And** the key is saved to local browser storage

### Scenario 6: Telegram Passive Whois

**Given** a user in a Telegram group is NOT registered with the PrivID bot
**But** they have a Next.ID proof linking their Telegram username to a wallet with Holonym SBTs
**When** someone uses `/whois @thatuser`
**Then** the bot queries Next.ID as a fallback
**And** shows verification details even though the user never registered

---

## Success Criteria

- Verified humans on Twitter/X are automatically badged within 3 seconds of page load with zero registration required
- The free baseline (no API keys configured) successfully resolves and badges at least some verified users via Next.ID + Holonym
- Users who configure additional API keys see broader coverage (more profiles badged)
- Badge tooltips display verification source, credential types, and at least one identity confirmation source
- All provider failures are handled silently — no user-visible errors, no broken page layouts
- Extension functions entirely client-side with no dependency on any PrivID-operated server
- API keys entered by users are never sent anywhere except to the specific API they authenticate against
- Telegram bot's `/whois` resolves unregistered users via passive lookup when possible

---

## Scope & Boundaries

### In Scope

- Multi-provider identity resolver (Next.ID, Neynar, Holonym, ensdata.net, Gitcoin Passport, ENS subgraph)
- BYOK API key management in extension popup
- Service worker rewrite: query public APIs directly instead of bot API
- Rich badge tooltips with source attribution
- Telegram bot `/whois` passive lookup fallback via Next.ID
- Caching layer for all provider responses

### Out of Scope

- Gmail passive verification (no protocol maps emails to wallets — stays registration-based)
- Running PrivID's own indexer or backend service
- Embedding or distributing shared API keys
- Mobile app or non-Chrome browser support
- Social graph analysis beyond the existing trust network
- Removing the existing Telegram registration flow (it stays as an optional path)

---

## Assumptions

- Next.ID ProofService remains free and publicly accessible without rate limiting for moderate usage
- Holonym's sybil-resistance API remains free and publicly accessible
- ensdata.net remains free and publicly accessible
- Neynar's free tier provides sufficient queries for initial testing; users bear costs beyond that
- The Graph's ENS subgraph free tier (100K queries/month) is sufficient for moderate personal use
- Twitter/X DOM structure for display names and profiles remains stable enough for badge injection
- Next.ID has meaningful coverage of crypto-native Twitter users (the primary target audience)

---

## Dependencies

- Next.ID ProofService API — primary free identity resolution
- Holonym API — SBT verification (free)
- ensdata.net — ENS enrichment (free)
- Neynar API — Farcaster-based identity resolution (user's key)
- The Graph ENS Subgraph — reverse text record lookups (user's key)
- Gitcoin Passport Scorer API — humanity score enrichment (user's key)
- Chrome Extension APIs — storage, service worker, content scripts
- Twitter/X DOM structure — badge injection targets

---

## Key Entities

| Entity | Attributes |
|--------|------------|
| **IdentityResult** | wallet, verified, sources[], ensName, sbtTypes[], passportScore, farcasterFid |
| **ProviderConfig** | provider_name, requires_key, api_key (user-provided), enabled, last_tested |
| **CachedLookup** | twitter_handle, result (IdentityResult or null), cached_at, ttl |
| **TrustCache** | viewer_id, target_handle, mutual_verified_count, cached_at |

---

## Edge Cases

- **Multiple wallets**: If Next.ID and Neynar return different wallets for the same handle, check all wallets for SBTs. Badge if any wallet is verified.
- **Same wallet from multiple providers**: Deduplicate wallets before SBT checking. The `sources` array on the result should list all providers that confirmed the link, even if they returned the same wallet.
- **Wallet found but no SBTs**: A handle resolves to a wallet via Next.ID or Farcaster, but the wallet holds no Holonym SBTs. No badge is shown. Cache the negative result with 30-minute TTL. This is expected to be the most common outcome.
- **Stale Next.ID proofs**: A user may have revoked their Next.ID proof but the cache hasn't expired. Use reasonable TTLs (5 minutes positive, 30 minutes negative).
- **Rate limiting**: If a provider rate-limits requests, back off gracefully and rely on cached data. Never retry in a tight loop.
- **Handle case sensitivity**: Normalize all Twitter handles to lowercase before lookup.
- **No providers return data**: For the majority of Twitter handles, no identity link will exist. This is expected — show nothing, cache the negative result, move on.
- **User adds API key**: Clear all cached entries so newly-configured providers can resolve handles that were previously negative-cached from free-only lookups.
- **User removes API key**: Clear cached entries. New lookups no longer query that provider.
- **Holonym API outage**: If the HTTP API is unreachable, fall back to direct on-chain SBT verification via the existing blockchain utilities. This is the only verification source — its failure means zero badges, so a fallback is critical.
