# Live smoke test — checklist

Draft for the first live run with real accounts. Bot: **@AskPrivID_bot**.
Mode: `VERIFICATION_MODE=blockchain` (already set in `.env`).

## Before people arrive
```sh
cd apps/telegram-bot
cargo test                                          # 75 unit
cargo test --test live_rpc_test -- --ignored        # 6 live (RPCs reachable?)
RUST_LOG=info cargo run                             # watch the log in this terminal
```
Expect in the log: `Verification mode: blockchain`, `Bot username: @AskPrivID_bot`,
`API server listening on 127.0.0.1:3141`.

## Roles
- **Challenger** (you): any Telegram account.
- **Claimant A** (verified): needs (1) a Human ID SBT at id.human.tech on the
  wallet, (2) an ENS name resolving to that wallet, (3) ENS text record
  `org.telegram` = their Telegram username without `@`.
- **Claimant B** (unverified): any account with nothing set up — this is the
  95% case and must feel fine, not broken.

## Script
| # | Who | Do | Expect |
|---|-----|----|--------|
| 1 | anyone | `/start` in DM | welcome copy mentions Human ID / id.human.tech, not passport.xyz |
| 2 | anyone | `/verify 0x000000000000000000000000000000000000dEaD` → "Check all" | five ❌ **Not found** lines — no "Error:" lines |
| 3 | A | `/register name.eth` in DM | "ENS verified! Checking Human ID SBTs…" then ✅ with SBT list |
| 4 | A | `/status`, `/verifyme` | tier "KYC'd human (Human ID SBT)" |
| 5 | you | `/challenge test deal` in DM | link `https://t.me/AskPrivID_bot?start=chg_…` |
| 6 | A | tap the link | A sees the neutral "sent to the person who asked" line; **you** get ✅ with tier/ENS/SBTs and `For: test deal` |
| 7 | A | tap the same link again | "already been used" — no second verdict |
| 8 | you | `/challenge` → give to B | B taps → B gets neutral line; you get ❌ "Telegram account only" with the *not-proof-of-scam* framing |
| 9 | you | `/challenges` | both tokens listed with status |
| 10 | you | mint a link, wait 31 min (or skip) | "expired" on tap |
| 11 | anyone | `/challenge` in a **group** | refused (DM-only) |
| 12 | anyone | `/whois @A` in a group | ✅ line for A; "not registered" for B |

## Things to watch in the log
- Any `WARN … keeping cached SBTs` = RPC outage handling kicked in.
- Any `Could not deliver verdict` = challenger has the bot blocked.
- Public RPC rate limits: 5 sequential `eth_call`s per check; if
  `publicnode` throttles, set `OPTIMISM_RPC_URL` to an Alchemy/Infura URL.

## Known gaps going in (don't file as bugs)
- No one-tap path for B yet (OAuth rung 1 is the next build).
- Verdicts aren't signed; a screenshot is not proof.
- Human Passport Stamps not read yet — only Human ID SBTs count.
