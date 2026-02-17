<!--
Sync Impact Report
- Version: 1.0.0 (initial)
- Added sections: All (initial creation)
- Removed sections: None
- Templates requiring updates: N/A (no .specify/ template directory in this project)
- Follow-up TODOs: None
-->

# PrivID Project Constitution

**Version**: 1.0.0
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
on-chain attestations (Soul-Bound Tokens) and MUST remain read-only
with respect to user identity data. Verification results MUST be stored
locally (browser storage, device-local SQLite) and MUST never be sent
to a centralized server controlled by PrivID.

**Rationale**: The entire value proposition collapses if users must trust
PrivID with sensitive data. We exist because centralized identity
verification is the problem — we cannot become the problem.

---

## Principle 2: Decentralized Verification Sources

All identity claims MUST be anchored to decentralized, publicly
auditable systems: Human Passport SBTs on Optimism for credential
verification, ENS on Ethereum mainnet for identity resolution and
platform linking. PrivID MUST NOT operate its own identity-issuing
authority or proprietary verification backend. If a verification source
becomes centralized or compromised, it MUST be replaceable without
rebuilding the system.

**Rationale**: Decentralized anchors ensure no single entity (including
PrivID) can forge, revoke, or gatekeep verifications. ENS text records
as the linking mechanism means users control their own identity mapping.

---

## Principle 3: User Sovereignty Over Registration

Users MUST control when and how they register with PrivID. Registration
MUST be opt-in and reversible (deregistration MUST be supported).
Platform linking (Twitter handle, Telegram username, email) MUST be
initiated by the user, not scraped or inferred without consent. The user
MUST be able to verify what data PrivID holds about them and remove it.

**Rationale**: A privacy tool that surveils or traps users contradicts
its own mission. Voluntary participation is a prerequisite for trust.

---

## Principle 4: Graceful Degradation

Every feature MUST degrade gracefully when external dependencies are
unavailable. RPC endpoint down: show cached data or "verification
unavailable" — never crash. ENS resolution fails: skip platform linking,
don't block registration. Twitter DOM changes: badges disappear cleanly
without breaking page functionality. The system MUST never make a
platform worse by its presence.

**Rationale**: PrivID operates as a layer on top of platforms it does
not control (Twitter, Gmail, Telegram, Bluesky). Fragile integrations
that break host platforms destroy user trust instantly.

---

## Principle 5: Operator-Owned, No Central Service

PrivID MUST NOT operate a centralized service that aggregates user data
across instances. The browser extension stores data in the user's own
browser. The Telegram bot stores data in a local SQLite database on the
operator's own machine — that database belongs to whoever runs the bot,
not to PrivID. PrivID provides self-hosted software that works out of
the box; it MUST NOT phone home, sync to a central registry, or require
a PrivID-operated API to function. Each deployment is independent.

**Rationale**: PrivID is a tool, not a platform. The operator owns their
data the same way a WordPress admin owns their database. No central
service means no single point of compromise, subpoena, or control.

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
