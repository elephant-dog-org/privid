# Feature Specification: PrivID — On-Demand Verification Challenge

**Status**: Draft
**Created**: 2026-06-11
**What it is**: A dApp that lets the wary party invoke a one-tap "prove you're a real human" challenge at the moment of risk — starting on the Telegram bot, sharing the same engine as the browser extension.
**Supersedes for the bot**: Phase 6 "passive `/whois` via Next.ID" in `.speckit/tasks.md` (dropped — see Decision Log)

---

## The Thesis: Friction Is the Whole Game

Nobody has shipped consumer peer-to-peer "prove yourself to me" verification that
stuck. The graveyard is all the same cause of death: **to be verifiable you had to set
something up first, so a stranger with nothing configured hit a dead-end, and the tool
was useless for the 95% case and got abandoned.** Friction killed every predecessor.

So this is a friction bet, and the design discipline is one rule:

> **The friction budget equals the claimant's motivation — and here that budget is
> about one tap.**

The Gitcoin lesson (from the person who led the Passport rollout): people tolerated
Passport friction because they *wanted* something — grant access, matching funds. The
reward sets the budget. Our motivated party is the **claimant** doing cold outreach:
they want your trust/attention. That want is the budget. But our reward is
*real-but-modest* ("this person will actually engage with me"), so the budget is
small. The challenge is therefore an **incentive exchange — "earn my attention with
one tap"** — not a gate.

**Two non-negotiable commitments fall out:**
1. **One-tap test.** Every claimant flow must be clearable by *a stranger with nothing
   set up, in one meaningful action.* The instant it becomes "install X, set up Y,
   come back," they bail. That dead-end is what we are specifically avoiding.
2. **Immediate, visible payoff.** The claimant does the tap → the verdict fires to the
   challenger right then → the connection opens. Opaque friction feels like an
   imposition; *rewarded* friction feels fair. Make the reward legible up front.

Matching friction to motivation also self-selects for signal: the strangers who matter
(genuinely want to connect) will do the tap; the rest are low-stakes (fine) or
scammers (won't, and are filtered).

*(Where this sits: PrivID reads existing personhood/credential data — Holonym, Human
Passport — and is the consumer face of presenting it. The "identity layer" jargon is
deliberately avoided; this is a dApp.)*

---

## Core Concept: The Trust Checkpoint

One mechanism, invoked by **the wary party**, at any moment of consequence. The
counterparty proves *themselves* to you — burden of proof inverted, no scraping, no
passive lookup of strangers.

1. **You** (at risk) DM the bot: `/challenge`.
2. Bot mints a single-use token → deep link `https://t.me/<PrivIDBot>?start=chg_<token>`.
3. **You** paste it to the counterparty: *"Verify you're a real human before I respond / click / agree."*
4. They tap → the **one official bot** reads **their** identity, evaluates it (Enrollment, below), binds the result to your token.
5. Bot DMs **you** the verdict directly — `✅ @handle — tier: KYC'd human, ENS: name.eth` / `❌ not verified`. The verdict travels **bot → you**, never a forwardable artifact the counterparty controls.

### Three trigger moments, one mechanism
| Moment | You do | Outcome |
|--------|--------|---------|
| Cold DM from a stranger | Reply with your challenge link | Verdict on the sender before you engage |
| Someone shares a link to click | Challenge them first | Whether a verified human stands behind it |
| About to agree to a deal / send funds | Challenge the counterparty | A pass = real KYC'd human, not a throwaway |

---

## Enrollment: The One-Tap Ladder (No Dead-Ends)

The claimant **never has to have prepared in advance.** They present *something*
immediately, and the assurance **tier** scales with whatever they're willing to do in
the moment. Every rung is read-only or one tap; no rung is a "go set up X first"
dead-end.

| Rung | Claimant action | Tier returned | Friction |
|------|-----------------|---------------|----------|
| 0 | Just taps the link | **Telegram account** (age/history — weak) | Zero |
| 1 | One-tap OAuth Twitter/X | **Established social graph** (esp. aged acct + trusted-follower overlap) — no wallet, no install | One tap |
| 2 | Already has Passport/Holonym/ENS | **KYC'd human** (read their wallet's SBTs/stamps) | Zero (pure read) |
| 3 | Inert wallet signature to bind a new wallet | **KYC'd human** | One signature (calm path) |

Rules:
- **Read first** — rungs 0 and 2 require *no signature at all* (public reads of
  Holonym SBTs, Human Passport Stamps, ENS). Lowest friction == lowest risk.
- **OAuth before wallet** — rung 1 (social OAuth) needs no wallet and no
  wallet-signature; it's the primary path for a claimant with no web3 setup, and it
  feeds the existing v2 trust network ("trusted by N verified people you know").
- **Signatures are the fallback, never the default**, and only ever inert EIP-191 /
  SIWE *messages* (readable English, cannot move funds).
- **The verdict states a tier, not a binary** — "verified" means different things and
  the challenger sees which.

### Enroll-Once-Present-Many
The only risky action (any signature) happens **once**, at a calm self-chosen moment.
Thereafter the user *presents* (mint a link) and *receives verdicts* (passive) with no
further signing. This structurally relocates the dangerous action out of the high-risk
moment (the 1am cold DM) — see Threat Model.

### Pluggable credential sources
Sources sit behind the existing Rust `VerificationProvider` trait (built for swapping —
the right seam). Adding a source must not touch challenge logic.
- **Holonym SBTs** (have it) — depth: strong ZK KYC/passport/phone.
- **Human Passport Stamps** — breadth: which socials are linked + humanity score.
  Open source; reusable per-platform providers. *(Confirm current API surface with the
  user — they led its rollout — before coding against a possibly-stale endpoint.)*
- Future: EAS attestations, Farcaster, World ID.

---

## Reverse Pipeline: Consume AND Produce (phase 2)

PrivID reads existing linkage to bootstrap, but every enrollment / passed challenge
*produces* new linkage. Write it back so PrivID grows the commons instead of hoarding a
silo:
- **Mint EAS attestations** ("PrivID confirms @handle controls 0xABC, holds Holonym
  KYC, at T") — other tools can consume what we produce.
- **The challenge graph** (who verified to whom, who vouched/transacted) compounds into
  reputation ("N verified humans have vouched-and-transacted with this identity").
- **Consent-gated + privacy-preserving** — write only with consent; hash /
  selective-disclose / ZK so a write is never a public doxx. This is the anti-data-
  broker discipline: you present proof, you don't get profiled.

Flywheel: consume → produce → network compounds → presenting becomes the norm (which is
the real long-run protection — see Threat Model).

---

## Threat Model & Non-Goals (stated plainly — candor is a feature)

**What this does NOT do:**
- It does **not** prevent signature phishing of a *compromised-state* user. A tired /
  stressed / drunk human who decides to sign a malicious payload cannot be saved by
  rule text. No identity tool is "scam-proof"; claiming otherwise is dishonest.
- It does **not** detect malicious payloads / drainer transactions. That's a different
  category — wallet-firewall / transaction-simulation tools (Blockaid, Wallet Guard,
  Pocket Universe, Fire) that warn *at signing time*. PrivID is the identity/trust
  layer (*who*), not the wallet firewall (*what you're signing*). They're
  complementary; owning both builds a worse version of two things.

**What it DOES do — harm reduction, not elimination (seatbelts, not invincibility):**
- **Relocates the risky moment.** Signing happens once, calm, by choice
  (Enroll-Once-Present-Many). At the dangerous moment the user's actions are
  signature-free. The failure surface is *absent* when the human is compromised.
- **Shifts the norm.** As cheap presentation makes verification normal, a scammer's
  inability/refusal to present becomes the red flag — protection by social default, not
  by code catching anyone. This is why low-friction *is* the protection, not a nicety.
- **Removes the drainable surface from the common path.** Reads + OAuth need no wallet
  signature, so "PrivID is asking me to sign a transaction" becomes a tell-tale of a
  *fake* — flipping the user heuristic from "is this the real PrivID?" (hard) to "is it
  asking me to sign a transaction? then no" (easy).

---

## Anti-Imitation Security Model

Can't stop brand-cloning; *can* make every safe action signature-free/inert and every
dangerous-looking request a fake-by-construction:
- **SEC-1** — Common path is signature-free (reads/OAuth). Normal user signs nothing.
- **SEC-2** — Any required signature is provably inert (EIP-191/SIWE readable message;
  never a transaction/approval/permit). Stated everywhere: *PrivID never asks you to
  sign a transaction, approve a token, or sign anything that isn't readable English.*
- **SEC-3** — Verifier-initiated context. The wary party mints the link from the
  official app; the signer acts inside their own trusted PrivID, not a scammer-crafted
  context. A "verify here" link arriving *in* an unsolicited DM is the attack, not the
  product.
- **SEC-4** — Self-authenticating verdicts. Bot/extension signs verdicts with a known
  PrivID key; a real verdict is checkable, a fake bot/screenshot can't forge it. Plus
  Telegram-verified bot username + store-signed extension publisher.
- **SEC-5** — Single-use, 30-min tokens. No replay to manufacture a stale "verified."
- **SEC-6 (OAuth)** — OAuth scopes are read-only (identity/handle only); the permission
  screen is the one trust moment on rung 1, so request the minimum and label it.

---

## Functional Requirements

- **FR-1 `/challenge [label]`** — mint a single-use token bound to the challenger's
  `telegram_user_id`; return the deep link + forwardable copy; optional free-text label
  echoed in the verdict; TTL 30 min (SEC-5).
- **FR-2 claim (`/start chg_<token>`)** — extract token, read claimant identity,
  evaluate via the one-tap ladder (read-first, tiered), bind `(token → claimant,
  tier/pass/fail, timestamp)`, consume token. Claimant sees neutral copy: *"Your result
  was sent to the person who asked. Nothing else is shared."* Challenger identity not
  leaked to claimant.
- **FR-3 verdict to challenger** — proactive DM: pass/fail, **tier**, `@handle`,
  credentials, ENS, UTC timestamp, label. Expired-unclaimed → no DM (absence is
  informative), shown in `/challenges`. A pass for an unverified claimant includes the
  one-tap upgrade path.
- **FR-4 `/verifyme`** — read-back of the user's own tier exactly as a challenger sees
  it (confirm you'll pass / fix setup). Not a shareable proof (screenshots fakeable —
  SEC-4).
- **FR-5 `/challenges`** — challenger's recent tokens + outcomes (pending/passed/
  failed/expired) to match a later verdict to its moment.
- **FR-6 reuse + pluggable sources** — runs through existing `Registry` +
  `VerificationProvider` trait; Holonym today, Human Passport Stamps as a 2nd provider.
  **Next.ID out of scope.**
- **FR-7 reverse-pipeline write** (phase 2, consent-gated, off by default) — optional
  EAS attestation / trust-graph update on enrollment or passed challenge.

---

## User Scenarios
1. **Cold DM, happy path** — stranger pitches; I reply with my link; they OAuth their
   aged X account in one tap → bot DMs me ✅ "Established social graph"; they learn
   nothing about me.
2. **Cold DM, KYC** — claimant already holds Holonym → ✅ "KYC'd human" with zero
   action (pure read).
3. **Cold DM, declines** — claimant won't tap → no verdict / ❌; framed as *caution, not
   proof of scam*; the non-response is itself the signal.
4. **Before a deal** — `/challenge "send 0.5 ETH bridge deal"`; verdict echoes the label.
5. **Phishing-inversion guard** — a "verify here" link arrives *in* an unsolicited DM;
   `/help` states links should originate from *me*, and PrivID never asks for
   keys/transaction-signatures/funds.
6. **No replay** — already-claimed/expired token yields no verdict (SEC-5).
7. **Self pre-check** — `/verifyme` shows my tier + one-tap upgrade if I'd present weak.

---

## Success Criteria (north star first)
- **A stranger with nothing set up can pass a challenge in ONE tap** (OAuth a social),
  and a claimant who already holds a credential passes with **zero** action. *This is
  the primary metric; everything else is secondary.*
- Challenger mints → counterparty claims → bot-delivered verdict, end to end, < 15 s.
- Common path requires **zero wallet signature**; any signature is inert and one-time.
- Verdict states an assurance **tier**, not a binary, and (on a weak/failed result)
  shows the one-tap upgrade.
- Verdict is never a forwardable artifact the claimant controls; tokens single-use +
  expiring.
- `/help` + verdict copy carry the threat-model honesty + anti-imitation rules in plain
  language.
- ≥1 credential source beyond Holonym (Human Passport Stamps) behind the trait.
- Existing commands + `cargo test` stay green.

---

## Scope & Boundaries
**In scope:** `/challenge`, `/start chg_` claim, verdict delivery, `/verifyme`,
`/challenges`; `challenges` SQLite table; OAuth-social claimant path (rung 1); Human
Passport Stamps provider behind the trait; tiered verdicts; `/help` rewrite; flip
deployment to `VERIFICATION_MODE=blockchain` + first live Telegram smoke test (bot has
never run live).

**Out of scope (this phase):** Next.ID / passive stranger lookup; EAS write-back UX
(FR-7 stubbed, consent-gated, off); wallet-drainer / payload detection (different
category — Blockaid/Wallet Guard territory); auto-intercepting others' DMs (impossible
on Telegram — recipient-initiated link is the only correct mechanism); changing existing
registration/extension flows.

---

## Implementation Sketch (grounded in current code)
| Piece | Where | Note |
|-------|-------|------|
| `Start(String)` payload | `src/main.rs` `Command` enum | widen `Start` to capture `chg_<token>`; empty = normal welcome. |
| `/challenge` handler | `src/main.rs` `handle_command` | mint token (existing `rand`), persist, format deep link from bot username. |
| Claim branch | `Start` handler | payload `chg_` → resolve token → evaluate claimant via provider(s) → record + DM challenger. |
| `challenges` table | `src/db.rs` | `CREATE TABLE IF NOT EXISTS challenges (...)` beside `identities`/`platform_links` in `Database::new`; `create`/`claim`/`list`/`expire` via the existing `conn.call` async pattern. |
| Passport Stamps provider | `src/verification/` | new module impl `VerificationProvider`; `reqwest` GET Stamps API; merge with Holonym. |
| OAuth-social path | new (rung 1) | likely needs a tiny callback endpoint on the existing axum API server (:3141) to receive the OAuth redirect and bind handle→token. **Confirm provider/scopes with user.** |
| Tier formatting | `src/main.rs` helpers | reuse `sbt_summary`, `short_addr`; add tier label. |
| TTL / single-use | `db.rs` claim | enforce `status='pending' AND expires_at > now` in the UPDATE. |
| Live mode | `.env` | set `VERIFICATION_MODE=blockchain` (token already configured); smoke-test live. |

Telegram MVP (challenge mint/claim/verdict, read-first tiers) needs **no new crates**
(`teloxide`, `rusqlite`/`tokio-rusqlite`, `rand`, `chrono`, `reqwest`, `axum` all
present). OAuth + EAS may add deps later.

---

## Decision Log
- **2026-06-11** — Friction is the north star; budget = claimant motivation ≈ one tap;
  the challenge is an incentive exchange, not a gate (Gitcoin Passport lesson, from the
  rollout lead). No-dead-end one-tap ladder; OAuth-social is the primary no-setup path.
- **2026-06-11** — Dropped "identity-layer / L4" framing as buzz; PrivID is a **dApp**
  that reads existing personhood/credential data and presents it.
- **2026-06-11** — Reframed "look up a stranger" → "stranger proves themselves"
  (consent-based; no scraping). From the user's "opt-in verify-yourself-first" idea.
- **2026-06-11** — Dropped Next.ID passive `/whois` (Telegram coverage unconfirmed/thin;
  the challenge model needs no handle→wallet resolution).
- **2026-06-11** — Read-first tiered enrollment + Enroll-Once-Present-Many = friction
  strategy AND core safety property.
- **2026-06-11** — Human Passport Stamps as 2nd credential source (breadth) alongside
  Holonym (depth), behind the `VerificationProvider` trait.
- **2026-06-11** — PrivID also produces linkage back (EAS + trust graph), consent-gated;
  phase 2.
- **2026-06-11** — Honesty principle: state non-goals plainly (no signature-phishing
  prevention for a compromised user; no payload detection). Harm reduction, not
  elimination.

## Open Questions
1. Group challenges (verdict posts in-thread) vs DM-only? *(Default: DM-only, private.)*
2. TTL default 30 min — confirm.
3. Failed claim — notify challenger immediately or only via `/challenges`? *(Proposed:
   immediate.)*
4. OAuth rung 1 — which social provider(s) and exact read-only scopes? (X/Twitter
   first; user to weigh in given Passport-provider history.)
5. EAS write-back — network (Optimism, to match Holonym) and on-chain vs off-chain.
