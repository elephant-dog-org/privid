<!--
Sync Impact Report
- Version: 1.0.0 → 2.0.0
- Modified principles:
  - Principle 3: "User Sovereignty Over Registration" → "User Sovereignty"
    (expanded to cover BYOK + passive verification, not just registration)
  - Principle 5: "Operator-Owned, No Central Service" → "Toolkit, Not Service"
    (strengthened to require BYOK model, zero operational costs for maintainer)
- Added sections:
  - Principle 7: "BYOK (Bring Your Own Key)"
- Removed sections: None
- Templates requiring updates: N/A (no .specify/ template directory)
- Follow-up TODOs: None
-->

# PrivID Project Constitution

**Version**: 2.0.0
**Ratified**: 2026-02-17
**Last Amended**: 2026-02-17

---

## Preamble

PrivID exists to prove you are who you say you are — without revealing
who you are. This constitution codifies the non-negotiable principles
that govern all design decisions, code contributions, and feature
additions across the PrivID ecosystem (browser extension, Telegram bot,
and any future platform integrations).

---

## Principle 1: Privacy Is the Product

PrivID MUST never store, transmit, or access personal identity documents,
biometric data, or private keys. The system reads existing public
on-chain attestations (Soul-Bound Tokens) and public identity protocol
data (Next.ID proofs, Farcaster verifications, ENS records). PrivID
MUST remain read-only with respect to user identity data. Verification
results MUST be stored locally (browser storage, device-local SQLite)
and MUST never be sent to a centralized server controlled by PrivID.

**Rationale**: The entire value proposition collapses if users must trust
PrivID with sensitive data. We exist because centralized identity
verification is the problem — we cannot become the problem.

---

## Principle 2: Decentralized Verification Sources

All identity claims MUST be anchored to decentralized, publicly
auditable systems: Human Passport SBTs on Optimism for credential
verification, ENS on Ethereum mainnet for identity resolution, and
open identity protocols (Next.ID, Farcaster) for cross-platform
linking. PrivID MUST NOT operate its own identity-issuing authority
or proprietary verification backend. Verification sources MUST be
pluggable — if one becomes centralized or compromised, it MUST be
replaceable without rebuilding the system.

**Rationale**: Decentralized anchors ensure no single entity (including
PrivID) can forge, revoke, or gatekeep verifications. Multiple
verification sources reduce single-point-of-failure risk.

---

## Principle 3: User Sovereignty

Users MUST NOT be required to register with PrivID to be identified
as verified. Passive identification through public on-chain data and
open identity protocols MUST be the primary path. Optional registration
MAY exist as a secondary path for users without public identity links.

Where registration exists, it MUST be opt-in and reversible
(deregistration MUST be supported). The user MUST be able to verify
what data PrivID holds about them and remove it.

**Rationale**: Requiring registration creates friction that undermines
adoption. A verification tool should verify existing public proofs,
not create a new registration silo. Users who have already proven
their identity on-chain should not have to prove it again to PrivID.

---

## Principle 4: Graceful Degradation

Every feature MUST degrade gracefully when external dependencies are
unavailable. API unreachable: show cached data or "verification
unavailable" — never crash. Identity provider down: skip that source,
try the next one. Twitter DOM changes: badges disappear cleanly
without breaking page functionality. The system MUST never make a
platform worse by its presence.

**Rationale**: PrivID operates as a layer on top of platforms it does
not control (Twitter, Gmail, Telegram, Bluesky). Fragile integrations
that break host platforms destroy user trust instantly.

---

## Principle 5: Toolkit, Not Service

PrivID MUST NOT operate a centralized service that users depend on.
The browser extension MUST function entirely client-side — all identity
lookups, verification checks, and badge rendering happen in the user's
browser. The Telegram bot stores data in a local SQLite database on
the operator's own machine.

PrivID MUST NOT require any PrivID-operated API, server, or backend
to function. Each deployment is independent. The maintainer MUST NOT
bear operational costs for other users' verification queries.

**Rationale**: PrivID is a tool, not a platform. No central service
means no single point of compromise, no operational costs for the
maintainer, and no dependency on PrivID's continued operation. If
PrivID the project disappears tomorrow, every installed extension
and running bot should continue to work.

---

## Principle 6: Platform Respect

Content scripts and badge injections MUST be non-destructive to host
platform functionality. Injected elements MUST NOT intercept clicks,
modify existing UI behavior, or interfere with platform interactions.
Badge styling MUST adapt to host platform themes (dark/light mode).
PrivID MUST NOT make API calls to host platforms on the user's behalf
without explicit action.

**Rationale**: Browser extensions that break websites get uninstalled.
Extensions that respect platform boundaries earn lasting adoption.

---

## Principle 7: BYOK (Bring Your Own Key)

When external APIs require authentication or have usage-based costs,
the user MUST provide their own API keys. PrivID MUST NOT embed,
distribute, or share API keys belonging to the project maintainer.

The system MUST provide a useful free baseline using APIs that require
no authentication. Enhanced functionality (broader coverage, more
identity sources) MUST be available by the user plugging in their own
keys for paid services.

API keys MUST be stored only in the user's local browser storage and
MUST only be sent to the respective API endpoints they authenticate
against — never to PrivID or any third party.

**Rationale**: A toolkit that requires the maintainer to pay for API
queries doesn't scale and creates a centralization pressure. BYOK
ensures every user bears their own costs, the maintainer bears none,
and no shared API key creates a single point of rate-limiting or
revocation.

---

## Governance

### Amendment Procedure

1. Propose change with rationale in a pull request modifying this file
2. Changes to MUST statements require explicit review and approval
3. Principle additions require demonstrating a gap not covered by
   existing principles
4. Principle removals require justification that the concern is no
   longer relevant or is covered elsewhere

### Versioning Policy

- **MAJOR**: Principle removed, redefined, or MUST statement weakened
- **MINOR**: New principle added or existing principle materially expanded
- **PATCH**: Wording clarification, typo fix, non-semantic refinement

### Compliance Review

Every spec, plan, and task breakdown produced via the speckit workflow
MUST be checked against these principles. The `/speckit.analyze` command
includes constitution alignment as a validation pass. Any CRITICAL
finding from constitution misalignment MUST be resolved before
implementation begins.
