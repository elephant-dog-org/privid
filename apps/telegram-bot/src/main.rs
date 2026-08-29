
use teloxide::prelude::*;
use teloxide::types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, ReplyParameters};
use teloxide::utils::command::BotCommands;
use log::{debug, info, warn};
use dotenv::dotenv;
use std::collections::HashSet;
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;

// Everything below comes from the library crate (src/lib.rs). The bin used to
// re-declare each module with `mod ...;`, which compiled the tree twice and
// buried real warnings under bogus "never used" noise.
use privid_telegram_bot::{
    start_api_server, BlockchainVerificationProvider, BotState, Config, Database, EnsResolver,
    FileStorage, MockVerificationProvider, RegistrationEntry, Registry, VerificationError,
    VerificationMode, VerificationProvider, VerificationResult, VerificationType,
};

/// Tracks which (chat_id, user_id) pairs have already been badged this session.
type BadgeTracker = Arc<RwLock<HashSet<(i64, u64)>>>;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
enum Command {
    #[command(description = "Start the bot and see welcome message")]
    Start(String),
    #[command(description = "Mint a verification challenge link to send someone (DM only)")]
    Challenge(String),
    #[command(description = "List the challenges you've sent and their results")]
    Challenges,
    #[command(description = "See what others learn when they verify you")]
    Verifyme,
    #[command(description = "Check wallet verification: /verify 0xABC...")]
    Verify(String),
    #[command(description = "Link your ENS name (DM only): /register name.eth")]
    Register(String),
    #[command(description = "Unlink your ENS name (DM only)")]
    Deregister,
    #[command(description = "Look up a user's verification: /whois @username")]
    Whois(String),
    #[command(description = "Check your verification status")]
    Status,
    #[command(description = "Show help information")]
    Help,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn short_addr(addr: &str) -> String {
    if addr.len() > 10 {
        format!("{}...{}", &addr[..6], &addr[addr.len() - 4..])
    } else {
        addr.to_string()
    }
}

fn is_wallet_address(s: &str) -> bool {
    s.starts_with("0x") && s.len() == 42 && s[2..].chars().all(|c| c.is_ascii_hexdigit())
}

fn is_private_chat(msg: &Message) -> bool {
    msg.chat.is_private()
}

fn verification_keyboard(wallet_address: &str) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    rows.push(vec![
        InlineKeyboardButton::callback(
            "KYC",
            format!("{}:{}", VerificationType::Kyc.callback_data(), wallet_address),
        ),
        InlineKeyboardButton::callback(
            "Phone",
            format!("{}:{}", VerificationType::Phone.callback_data(), wallet_address),
        ),
        InlineKeyboardButton::callback(
            "Passport",
            format!("{}:{}", VerificationType::Passport.callback_data(), wallet_address),
        ),
    ]);

    rows.push(vec![
        InlineKeyboardButton::callback(
            "Clean Hands",
            format!("{}:{}", VerificationType::CleanHands.callback_data(), wallet_address),
        ),
        InlineKeyboardButton::callback(
            "Biometrics",
            format!("{}:{}", VerificationType::Biometrics.callback_data(), wallet_address),
        ),
    ]);

    rows.push(vec![InlineKeyboardButton::callback(
        "Check All",
        format!("verify_all:{}", wallet_address),
    )]);

    InlineKeyboardMarkup::new(rows)
}

fn format_single_result(
    wallet_address: &str,
    vtype: VerificationType,
    result: &Result<VerificationResult, VerificationError>,
    mode_label: &str,
) -> String {
    let addr_short = short_addr(wallet_address);

    match result {
        Ok(vr) => {
            let expiry_line = if let Some(exp) = vr.sbt_expiry {
                let dt = chrono::DateTime::from_timestamp(exp as i64, 0)
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| exp.to_string());
                format!("\n  Expires: {}", dt)
            } else {
                String::new()
            };
            format!(
                "Checking {} verification for {} on {}...\n\n\
                 \u{2705} {}: Verified{}\n  Proof: {}",
                vtype.description(),
                addr_short,
                mode_label,
                vtype.description(),
                expiry_line,
                vr.proof,
            )
        }
        Err(e) => {
            format!(
                "Checking {} verification for {} on {}...\n\n\
                 \u{274c} {}: {}",
                vtype.description(),
                addr_short,
                mode_label,
                vtype.description(),
                match e {
                    VerificationError::NotVerified(_) => "Not found".to_string(),
                    VerificationError::Expired(_) => "Expired".to_string(),
                    VerificationError::Revoked(_) => "Revoked".to_string(),
                    other => format!("Error: {}", other),
                },
            )
        }
    }
}

fn format_all_results(
    wallet_address: &str,
    results: &[(VerificationType, Result<VerificationResult, VerificationError>)],
    mode_label: &str,
) -> String {
    let addr_short = short_addr(wallet_address);
    let mut lines = vec![format!(
        "Checking all verifications for {} on {}...\n\nVerification Results:",
        addr_short, mode_label
    )];

    let mut valid_count: usize = 0;

    for (vt, result) in results {
        match result {
            Ok(vr) => {
                valid_count += 1;
                let expiry_note = if let Some(exp) = vr.sbt_expiry {
                    let dt = chrono::DateTime::from_timestamp(exp as i64, 0)
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_else(|| exp.to_string());
                    format!(" (expires {})", dt)
                } else {
                    String::new()
                };
                lines.push(format!("\u{2705} {}: Verified{}", vt.description(), expiry_note));
            }
            Err(e) => {
                let reason = match e {
                    VerificationError::NotVerified(_) => "Not found".to_string(),
                    VerificationError::Expired(_) => "Expired".to_string(),
                    VerificationError::Revoked(_) => "Revoked".to_string(),
                    other => format!("Error: {}", other),
                };
                lines.push(format!("\u{274c} {}: {}", vt.description(), reason));
            }
        }
    }

    lines.push(String::new());
    lines.push(format!("{} valid SBTs found.", valid_count));

    lines.join("\n")
}

fn parse_callback_data(data: &str) -> Option<(&str, &str)> {
    let colon_pos = data.find(':')?;
    let action = &data[..colon_pos];
    let wallet = &data[colon_pos + 1..];
    if wallet.is_empty() {
        return None;
    }
    Some((action, wallet))
}

/// How long a minted challenge token stays claimable.
const CHALLENGE_TTL_MINUTES: i64 = 30;
/// Prefix that marks a `/start` deep-link payload as a challenge claim.
const CHALLENGE_PREFIX: &str = "chg_";

/// Mint a fresh single-use challenge token (`chg_<32 hex>`).
fn mint_challenge_token() -> String {
    format!("{}{}", CHALLENGE_PREFIX, hex::encode(rand::random::<[u8; 16]>()))
}

/// Evaluate a claimant against the registry and return (passed, assurance tier).
///
/// The tier names follow the enrollment ladder in
/// `.speckit/spec-telegram-challenge.md`: the verdict states *what kind* of
/// assurance was found, not a bare yes/no, so the challenger can weigh it.
///
/// - rung 0: no registration at all -> only a Telegram account stands behind them
/// - rung 2 (linked, no SBT): ENS is bound, but the wallet holds no valid credential
/// - rung 2 (linked + SBT): a KYC'd human — Holonym SBT on Optimism
///
/// OAuth-social (rung 1) and Human Passport Stamps land in a later pass.
fn evaluate_claimant(entry: Option<&RegistrationEntry>) -> (bool, &'static str) {
    match entry {
        Some(e) if !e.verified_sbt_types.is_empty() => (true, "KYC'd human (Human ID SBT)"),
        Some(_) => (false, "ENS linked, but no valid credential on the wallet"),
        None => (false, "Telegram account only — nothing linked"),
    }
}

/// Re-check a registered wallet's SBTs on-chain and persist the fresh state.
///
/// A verdict says "Checked: <time>", so it must reflect the chain *now*, not
/// whatever was true when the user ran `/register` (SBTs expire; a lapsed KYC
/// would otherwise keep passing challenges indefinitely).
///
/// Fail-safe: if the RPC itself is down (every circuit returns an
/// infrastructure error rather than "not found"), keep the cached state and
/// log it — an outage must not silently turn a verified user into a failure.
/// In mock mode the cached entry is returned untouched.
async fn refresh_entry(
    entry: RegistrationEntry,
    provider: &dyn VerificationProvider,
    registry: &Registry,
    now: chrono::DateTime<chrono::Utc>,
) -> RegistrationEntry {
    if provider.is_mock() {
        return entry;
    }
    let results = provider.check_all_verifications(&entry.wallet_address).await;

    // "Not verified / expired / revoked" are answers; anything else is the RPC
    // failing to answer. Only trust the refresh if at least one circuit answered.
    let answered = results.iter().any(|(_, r)| {
        !matches!(
            r,
            Err(VerificationError::RpcError(_)) | Err(VerificationError::AbiError(_))
        )
    });
    if !answered {
        warn!(
            "On-chain refresh for {} got no answers (RPC down?); keeping cached SBTs",
            entry.wallet_address
        );
        return entry;
    }

    let fresh: Vec<VerificationType> = results
        .iter()
        .filter(|(_, r)| r.is_ok())
        .map(|(vt, _)| *vt)
        .collect();

    let mut updated = entry;
    if updated.verified_sbt_types != fresh {
        info!(
            "SBT state changed for {}: {:?} -> {:?}",
            updated.wallet_address, updated.verified_sbt_types, fresh
        );
    }
    updated.verified_sbt_types = fresh;
    updated.last_verified = now;
    registry.register(updated.clone()).await;
    updated
}

/// Handle a claimant opening a challenge deep link (`/start chg_<token>`).
///
/// Reads the claimant's identity, atomically claims the (single-use) token, DMs the
/// claimant a neutral confirmation, and delivers the verdict to the challenger.
async fn handle_claim(
    bot: &Bot,
    msg: &Message,
    token: &str,
    db: &Database,
    registry: &Registry,
    provider: &dyn VerificationProvider,
    bot_username: &str,
) -> ResponseResult<()> {
    let claimant = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    let challenge = match db.get_challenge(token).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            bot.send_message(msg.chat.id, "This verification link is invalid or has already been used.")
                .await?;
            return Ok(());
        }
        Err(e) => {
            warn!("Failed to read challenge {}: {}", token, e);
            bot.send_message(msg.chat.id, "Something went wrong reading that link. Please ask for a new one.")
                .await?;
            return Ok(());
        }
    };

    let now = chrono::Utc::now();
    if !challenge.is_open(now) {
        let reason = if challenge.status == "claimed" {
            "This verification link has already been used."
        } else {
            "This verification link has expired. Ask the person for a fresh one."
        };
        bot.send_message(msg.chat.id, reason).await?;
        return Ok(());
    }

    // Evaluate the claimant against the registry (read-first), re-checking the
    // chain so the verdict reflects current SBT state.
    let entry = match registry.lookup_by_user_id(claimant.id.0).await {
        Some(e) => Some(refresh_entry(e, provider, registry, now).await),
        None => None,
    };
    let (passed, tier) = evaluate_claimant(entry.as_ref());

    // Atomically claim the token (single-use guard inside SQL).
    let claimed = db
        .claim_challenge(
            token,
            claimant.id.0 as i64,
            claimant.username.clone(),
            tier,
            passed,
            now,
        )
        .await
        .unwrap_or(false);

    if !claimed {
        bot.send_message(msg.chat.id, "This verification link has already been used.")
            .await?;
        return Ok(());
    }

    // Neutral confirmation to the claimant — challenger identity is not revealed.
    bot.send_message(
        msg.chat.id,
        "\u{2705} Thanks — your verification result has been sent to the person who asked. \
         Nothing else about you is shared.",
    )
    .await?;

    // Deliver the verdict to the challenger via DM.
    let claimant_handle = claimant
        .username
        .clone()
        .map(|u| format!("@{}", u))
        .unwrap_or_else(|| claimant.full_name());

    let label_line = challenge
        .label
        .as_ref()
        .map(|l| format!("\nFor: {}", l))
        .unwrap_or_default();

    let verdict = if passed {
        let detail = entry
            .as_ref()
            .map(|e| format!("\nENS: {}\nSBTs: {}", e.ens_name, e.sbt_summary()))
            .unwrap_or_default();
        format!(
            "\u{2705} Challenge passed\n\n{} — {}{}{}\nChecked: {} UTC",
            claimant_handle,
            tier,
            detail,
            label_line,
            now.format("%Y-%m-%d %H:%M"),
        )
    } else {
        format!(
            "\u{274c} Challenge result: not verified\n\n\
             {} opened your challenge but is not verified ({}).{}\n\n\
             Note: \"not verified\" is common and is NOT proof of a scam — most people \
             haven't set this up. A pass is the strong signal, not a fail.\n\n\
             They can set up verification by opening https://t.me/{} and sending /start.",
            claimant_handle, tier, label_line, bot_username,
        )
    };

    // The claimant has already been told "sent". If the challenger can't be
    // reached (blocked the bot, deleted account), log it rather than erroring
    // out of the handler; the result is still visible to them in /challenges.
    if let Err(e) = bot
        .send_message(ChatId(challenge.challenger_user_id), verdict)
        .await
    {
        warn!(
            "Could not deliver verdict for {} to challenger {}: {}",
            token, challenge.challenger_user_id, e
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Command handler
// ---------------------------------------------------------------------------

async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: Arc<BotState>,
    provider: Arc<dyn VerificationProvider>,
    ens_resolver: Arc<EnsResolver>,
    registry: Arc<Registry>,
    badge_tracker: BadgeTracker,
    db: Arc<Database>,
    bot_username: Arc<String>,
) -> ResponseResult<()> {
    match cmd {
        Command::Start(payload) => {
            // Deep-link claim: /start chg_<token>
            let payload = payload.trim();
            if payload.starts_with(CHALLENGE_PREFIX) {
                handle_claim(&bot, &msg, payload, &db, &registry, provider.as_ref(), &bot_username).await?;
                return Ok(());
            }

            let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
            let _session = state.get_or_create_session(user_id).await;

            let welcome = if is_private_chat(&msg) {
                "Welcome to PrivID — proof of personhood for Telegram.\n\n\
                 I link your Human ID to your Telegram identity \
                 using ENS, so people in group chats can see you're a verified human.\n\n\
                 Human ID (by human.tech, formerly Holonym) lets you prove you hold a real government-issued passport, \
                 phone number, or KYC credential using zero-knowledge proofs — no personal data \
                 is revealed. These proofs are stored as Soul-Bound Tokens (SBTs) on Optimism.\n\n\
                 How to register:\n\
                 1. Get your Human ID at id.human.tech (government ID, phone, or ePassport)\n\
                 2. Own an ENS name (e.g. yourname.eth)\n\
                 3. Go to app.ens.domains, edit your ENS text records, \
                 and set org.telegram to your Telegram username (without @)\n\
                 4. DM me: /register yourname.eth\n\n\
                 I'll read the org.telegram record from your ENS to confirm you own it, \
                 resolve your wallet address, and check for Human ID SBTs. \
                 No wallet signature needed — your ENS record is the proof.\n\n\
                 Once registered, I'll automatically badge you as verified in any group chat I'm in.\n\n\
                 Commands:\n\
                 /register <name.eth> — Link your ENS (DM only)\n\
                 /deregister — Remove your registration (DM only)\n\
                 /whois @username — Look up a user in a group\n\
                 /status — Check your registration\n\
                 /verify <wallet> — Check any wallet directly"
            } else {
                "PrivID — proof of personhood for Telegram.\n\n\
                 I automatically identify verified humans in this chat. \
                 Users who hold a Human ID (passport, phone, or KYC verification) \
                 and linked their ENS name will be badged when they message.\n\n\
                 /whois @username — Look up someone's verification\n\
                 /verify <wallet> — Check a wallet address\n\n\
                 To register, DM me directly and type /start."
            };

            bot.send_message(msg.chat.id, welcome).await?;
        }

        Command::Challenge(text) => {
            if !is_private_chat(&msg) {
                bot.send_message(
                    msg.chat.id,
                    "Mint challenges in a DM with me so I can send you the result privately. \
                     Open a private chat and type /challenge.",
                )
                .await?;
                return Ok(());
            }

            let user = match msg.from.as_ref() {
                Some(u) => u,
                None => return Ok(()),
            };

            let label = {
                let t = text.trim().trim_matches('"').trim();
                if t.is_empty() { None } else { Some(t.to_string()) }
            };

            let token = mint_challenge_token();
            let now = chrono::Utc::now();
            let expires = now + chrono::Duration::minutes(CHALLENGE_TTL_MINUTES);

            if let Err(e) = db
                .create_challenge(&token, user.id.0 as i64, label.clone(), now, expires)
                .await
            {
                warn!("Failed to create challenge: {}", e);
                bot.send_message(msg.chat.id, "Could not create a challenge right now. Please try again.")
                    .await?;
                return Ok(());
            }

            let link = format!("https://t.me/{}?start={}", bot_username, token);
            let label_line = label
                .as_ref()
                .map(|l| format!("\nFor: {}", l))
                .unwrap_or_default();

            bot.send_message(
                msg.chat.id,
                format!(
                    "\u{1f6e1} Verification challenge created.{}\n\n\
                     Send this link to the person and ask them to tap it:\n{}\n\n\
                     When they open it, I'll check whether they're a verified human and \
                     send you the result here. The link works once and expires in {} minutes.\n\n\
                     \u{26a0} Only ever send a challenge link that YOU generated. I will never \
                     ask anyone to sign a transaction, approve a token, or send funds.",
                    label_line, link, CHALLENGE_TTL_MINUTES,
                ),
            )
            .await?;
        }

        Command::Challenges => {
            let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
            let now = chrono::Utc::now();

            match db.list_challenges_for(user_id as i64, 10).await {
                Ok(list) if list.is_empty() => {
                    bot.send_message(
                        msg.chat.id,
                        "You haven't sent any challenges yet. Use /challenge to create one.",
                    )
                    .await?;
                }
                Ok(list) => {
                    let mut text = String::from("Your recent challenges:\n");
                    for c in &list {
                        let icon = match c.display_status(now) {
                            "passed" => "\u{2705}",
                            "failed" => "\u{274c}",
                            "expired" => "\u{231b}",
                            _ => "\u{23f3}",
                        };
                        let who = c
                            .claimant_username
                            .as_ref()
                            .map(|u| format!(" — @{}", u))
                            .unwrap_or_default();
                        let label = c
                            .label
                            .as_ref()
                            .map(|l| format!(" ({})", l))
                            .unwrap_or_default();
                        text.push_str(&format!(
                            "\n{} {}{}{} · {}",
                            icon,
                            c.display_status(now),
                            who,
                            label,
                            c.created_at.format("%Y-%m-%d %H:%M"),
                        ));
                    }
                    bot.send_message(msg.chat.id, text).await?;
                }
                Err(e) => {
                    warn!("Failed to list challenges: {}", e);
                    bot.send_message(msg.chat.id, "Could not load your challenges right now.")
                        .await?;
                }
            }
        }

        Command::Verifyme => {
            let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
            // Same live re-check a real challenge performs, so "what others
            // learn" is exactly what they would learn right now.
            let now = chrono::Utc::now();
            let entry = match registry.lookup_by_user_id(user_id).await {
                Some(e) => Some(refresh_entry(e, provider.as_ref(), &registry, now).await),
                None => None,
            };
            let (passed, tier) = evaluate_claimant(entry.as_ref());

            let text = if passed {
                let e = entry.as_ref().unwrap();
                format!(
                    "Here's what someone learns when they challenge you:\n\n\
                     \u{2705} Verified — {}\n\
                     ENS: {}\n\
                     SBTs: {}",
                    tier,
                    e.ens_name,
                    e.sbt_summary(),
                )
            } else {
                format!(
                    "Here's what someone learns when they challenge you:\n\n\
                     \u{274c} {}\n\n\
                     To pass a challenge, link your identity with /register <name.eth> (DM only). \
                     One-tap social verification is coming soon.",
                    tier,
                )
            };
            bot.send_message(msg.chat.id, text).await?;
        }

        Command::Register(text) => {
            let ens_name = text.trim().to_lowercase();

            if !is_private_chat(&msg) {
                bot.send_message(msg.chat.id, "Registration must be done in a DM. Send me a private message with /register <name.eth>")
                    .await?;
                return Ok(());
            }

            let user = match msg.from.as_ref() {
                Some(u) => u,
                None => return Ok(()),
            };

            let username = match &user.username {
                Some(u) => u.clone(),
                None => {
                    bot.send_message(msg.chat.id, "You need a Telegram username to register. Set one in Telegram Settings, then try again.")
                        .await?;
                    return Ok(());
                }
            };

            if ens_name.is_empty() || !ens_name.ends_with(".eth") {
                bot.send_message(msg.chat.id, "Please provide a valid ENS name.\n\nUsage: /register name.eth")
                    .await?;
                return Ok(());
            }

            bot.send_message(msg.chat.id, format!("Looking up {}...", ens_name)).await?;

            if provider.is_mock() {
                // Mock mode: skip ENS, create entry with mock SBTs
                let now = chrono::Utc::now();
                let entry = RegistrationEntry {
                    telegram_user_id: user.id.0,
                    telegram_username: username.clone(),
                    ens_name: ens_name.clone(),
                    wallet_address: "0x0000000000000000000000000000000000000000".to_string(),
                    verified_sbt_types: vec![VerificationType::Kyc, VerificationType::Phone],
                    registered_at: now,
                    last_verified: now,
                };

                registry.register(entry.clone()).await;
                if let Err(e) = registry.save().await {
                    warn!("Failed to save registry: {}", e);
                }

                let mut success_msg = format!(
                    "\u{2705} Registered! (mock mode)\n\n\
                     ENS: {}\n\
                     Username: @{}\n\
                     Mock SBTs: KYC, Phone",
                    ens_name, username
                );

                // In mock mode, skip ENS text record lookup but still allow manual linking later
                success_msg.push_str("\n\nI'll badge you in group chats now.");

                bot.send_message(msg.chat.id, success_msg).await?;
            } else {
                // Blockchain mode: verify ENS org.telegram record
                let tg_record = match ens_resolver.get_text_record(&ens_name, "org.telegram").await {
                    Ok(record) => record,
                    Err(e) => {
                        bot.send_message(
                            msg.chat.id,
                            format!(
                                "\u{274c} Could not read org.telegram text record for {}.\n\n\
                                 Error: {}\n\n\
                                 Make sure you've set an org.telegram text record on your ENS name \
                                 at app.ens.domains with your Telegram username (without @).",
                                ens_name, e
                            ),
                        )
                        .await?;
                        return Ok(());
                    }
                };

                // Compare (case-insensitive, strip @)
                let record_clean = tg_record.trim().trim_start_matches('@').to_lowercase();
                if record_clean != username.to_lowercase() {
                    bot.send_message(
                        msg.chat.id,
                        format!(
                            "\u{274c} ENS org.telegram mismatch.\n\n\
                             ENS record says: {}\n\
                             Your Telegram username: @{}\n\n\
                             Update your ENS text record at app.ens.domains to match your \
                             Telegram username, then try again.",
                            tg_record, username
                        ),
                    )
                    .await?;
                    return Ok(());
                }

                // Resolve wallet address
                let wallet_bytes = match ens_resolver.resolve_address(&ens_name).await {
                    Ok(addr) => addr,
                    Err(e) => {
                        bot.send_message(
                            msg.chat.id,
                            format!("\u{274c} Could not resolve address for {}: {}", ens_name, e),
                        )
                        .await?;
                        return Ok(());
                    }
                };

                let wallet_address = format!("0x{}", hex::encode(wallet_bytes));

                bot.send_message(
                    msg.chat.id,
                    format!("ENS verified! Checking Human ID SBTs for {}...", short_addr(&wallet_address)),
                )
                .await?;

                // Check all SBTs
                let results = provider.check_all_verifications(&wallet_address).await;
                let verified_types: Vec<VerificationType> = results
                    .iter()
                    .filter_map(|(vt, r)| if r.is_ok() { Some(*vt) } else { None })
                    .collect();

                let now = chrono::Utc::now();
                let entry = RegistrationEntry {
                    telegram_user_id: user.id.0,
                    telegram_username: username.clone(),
                    ens_name: ens_name.clone(),
                    wallet_address: wallet_address.clone(),
                    verified_sbt_types: verified_types.clone(),
                    registered_at: now,
                    last_verified: now,
                };

                registry.register(entry.clone()).await;
                if let Err(e) = registry.save().await {
                    warn!("Failed to save registry: {}", e);
                }

                let sbt_list = if verified_types.is_empty() {
                    "None found".to_string()
                } else {
                    verified_types.iter().map(|v| v.short_name()).collect::<Vec<_>>().join(", ")
                };

                // Try to capture Twitter handle from ENS com.twitter text record
                let mut twitter_line = String::new();
                match ens_resolver.get_text_record(&ens_name, "com.twitter").await {
                    Ok(twitter) => {
                        if !twitter.is_empty() {
                            let clean_handle = twitter.strip_prefix('@').unwrap_or(&twitter);
                            match registry.link_platform(entry.telegram_user_id, "twitter", clean_handle, true).await {
                                Ok(()) => {
                                    twitter_line = format!("\nTwitter: @{}", clean_handle);
                                    info!("Linked Twitter @{} for user {}", clean_handle, username);
                                }
                                Err(e) => {
                                    warn!("Failed to link Twitter handle: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // Not an error -- many users won't have com.twitter set
                        debug!("No com.twitter record for {}: {}", ens_name, e);
                    }
                }

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "\u{2705} Registered!\n\n\
                         ENS: {}\n\
                         Wallet: {}\n\
                         SBTs: {}{}\n\n\
                         I'll badge you in group chats now.",
                        ens_name,
                        short_addr(&wallet_address),
                        sbt_list,
                        twitter_line,
                    ),
                )
                .await?;
            }
        }

        Command::Deregister => {
            if !is_private_chat(&msg) {
                bot.send_message(msg.chat.id, "Use /deregister in a DM with me.")
                    .await?;
                return Ok(());
            }

            let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);

            if registry.deregister(user_id).await {
                if let Err(e) = registry.save().await {
                    warn!("Failed to save registry: {}", e);
                }
                // Clear badge tracker entries for this user
                {
                    let mut tracker = badge_tracker.write().await;
                    tracker.retain(|&(_, uid)| uid != user_id);
                }
                bot.send_message(msg.chat.id, "\u{2705} Your registration has been removed.")
                    .await?;
            } else {
                bot.send_message(msg.chat.id, "You don't have a registration to remove.")
                    .await?;
            }
        }

        Command::Whois(text) => {
            let target = text.trim().to_string();

            // Determine who to look up: argument, or replied-to message sender
            let entry = if !target.is_empty() {
                registry.lookup_by_username(&target).await
            } else if let Some(reply) = msg.reply_to_message() {
                if let Some(user) = reply.from.as_ref() {
                    registry.lookup_by_user_id(user.id.0).await
                } else {
                    None
                }
            } else {
                bot.send_message(
                    msg.chat.id,
                    "Usage: /whois @username\nOr reply to someone's message with /whois",
                )
                .await?;
                return Ok(());
            };

            if let Some(entry) = entry {
                let response = format!(
                    "\u{2705} @{} is verified — Human ID\n\
                     ENS: {}\n\
                     SBTs: {}\n\
                     Wallet: {}",
                    entry.telegram_username,
                    entry.ens_name,
                    entry.sbt_summary(),
                    short_addr(&entry.wallet_address),
                );
                bot.send_message(msg.chat.id, response).await?;
            } else {
                let who = if !target.is_empty() {
                    target.clone()
                } else {
                    "That user".to_string()
                };
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "{} is not registered with PrivID.\n\
                         They can DM me with /register <name.eth> to link their identity.",
                        who
                    ),
                )
                .await?;
            }
        }

        Command::Verify(text) => {
            let wallet = text.trim().to_string();
            if wallet.is_empty() || !is_wallet_address(&wallet) {
                bot.send_message(
                    msg.chat.id,
                    "Please provide a valid Ethereum wallet address.\n\n\
                     Usage: /verify 0xABC123...\n\n\
                     The address must start with 0x and be 42 characters long.",
                )
                .await?;
                return Ok(());
            }

            let mode_label = if provider.is_mock() { "Mock" } else { "Optimism" };
            let prompt = format!(
                "Select a verification type to check for {}:\n\nMode: {}",
                short_addr(&wallet),
                mode_label,
            );

            bot.send_message(msg.chat.id, prompt)
                .reply_markup(verification_keyboard(&wallet))
                .await?;
        }

        Command::Status => {
            let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);

            // Check registry first
            if let Some(entry) = registry.lookup_by_user_id(user_id).await {
                let mut text = format!(
                    "Registration Status: Active\n\n\
                     ENS: {}\n\
                     Wallet: {}\n\
                     SBTs: {}\n\
                     Registered: {}\n\
                     Last verified: {}",
                    entry.ens_name,
                    short_addr(&entry.wallet_address),
                    entry.sbt_summary(),
                    entry.registered_at.format("%Y-%m-%d %H:%M UTC"),
                    entry.last_verified.format("%Y-%m-%d %H:%M UTC"),
                );

                // Show linked platforms
                if let Ok(links) = registry.get_platform_links(user_id).await {
                    if !links.is_empty() {
                        text.push_str("\n\n\u{1f4f1} Linked Platforms:");
                        for link in &links {
                            let icon = match link.platform.as_str() {
                                "twitter" => "\u{1f426}",
                                "email" => "\u{1f4e7}",
                                _ => "\u{1f517}",
                            };
                            let verified_tag = if link.verified_via_ens { " (ENS)" } else { "" };
                            text.push_str(&format!(
                                "\n{} {}: @{}{}",
                                icon, link.platform, link.handle, verified_tag
                            ));
                        }
                    }
                }

                bot.send_message(msg.chat.id, text).await?;
            } else {
                bot.send_message(
                    msg.chat.id,
                    "You're not registered.\n\n\
                     Use /register <name.eth> in a DM to link your ENS name.",
                )
                .await?;
            }
        }

        Command::Help => {
            let help = "\
                PrivID — proof of personhood for Telegram\n\n\
                What it does:\n\
                PrivID verifies that a Telegram user holds a Human ID — \
                a zero-knowledge proof of a real-world credential (government passport, phone, or KYC). \
                No personal data is ever revealed or stored — only the fact that you're verified.\n\n\
                How the link works:\n\
                Your ENS name (e.g. yourname.eth) acts as the bridge between your wallet and \
                your Telegram account. By setting an org.telegram text record on your ENS, \
                you prove ownership without needing a wallet signature. The bot reads this record \
                on-chain, resolves your wallet address, and checks for Human ID Soul-Bound Tokens \
                on Optimism.\n\n\
                Setup:\n\
                1. Get your Human ID at id.human.tech (government ID, phone, or ePassport)\n\
                2. Own an ENS name with the same wallet\n\
                3. At app.ens.domains, set text record org.telegram = your Telegram username\n\
                4. DM me /register yourname.eth\n\n\
                Verify a stranger before you trust them:\n\
                /challenge — Create a one-time link to send someone (a cold DM, before \
                a deal, before clicking a link they shared). When they tap it, I check \
                whether they're a verified human and send you the result. The verdict \
                comes from me, not a screenshot they control.\n\n\
                Commands:\n\
                /challenge [note] — Mint a verification link to send someone (DM only)\n\
                /challenges — See challenges you've sent and their results\n\
                /verifyme — See what others learn when they verify you\n\
                /register <name.eth> — Link your ENS identity (DM only)\n\
                /deregister — Remove your registration (DM only)\n\
                /status — Check your registration details\n\
                /whois @username — Look up a user's verification (works in groups)\n\
                /verify <wallet> — Query any wallet for Human ID SBTs\n\n\
                Safety: I will NEVER ask you (or anyone) to sign a transaction, approve \
                a token, or send funds. Only ever act on a challenge link YOU generated — \
                a 'verify here' link arriving in an unsolicited DM is a scam.\n\n\
                In groups, I automatically badge verified users on their first message.";

            bot.send_message(msg.chat.id, help).await?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Message handler (group auto-badge + bare wallet shortcut)
// ---------------------------------------------------------------------------

async fn handle_message(
    bot: Bot,
    msg: Message,
    provider: Arc<dyn VerificationProvider>,
    registry: Arc<Registry>,
    badge_tracker: BadgeTracker,
) -> ResponseResult<()> {
    // Bare wallet address shortcut (works in any chat)
    if let Some(text) = msg.text() {
        let trimmed = text.trim();
        if is_wallet_address(trimmed) {
            let mode_label = if provider.is_mock() { "Mock" } else { "Optimism" };
            let prompt = format!(
                "Select a verification type to check for {}:\n\nMode: {}",
                short_addr(trimmed),
                mode_label,
            );

            bot.send_message(msg.chat.id, prompt)
                .reply_markup(verification_keyboard(trimmed))
                .await?;
            return Ok(());
        }
    }

    // Group auto-badging: only in group/supergroup chats
    if is_private_chat(&msg) {
        return Ok(());
    }

    let user = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    let chat_id = msg.chat.id.0;
    let user_id = user.id.0;

    // Check if already badged in this chat this session
    {
        let tracker = badge_tracker.read().await;
        if tracker.contains(&(chat_id, user_id)) {
            return Ok(());
        }
    }

    // Look up in registry
    if let Some(entry) = registry.lookup_by_user_id(user_id).await {
        if !entry.verified_sbt_types.is_empty() {
            // Badge this user
            let badge_msg = format!(
                "\u{2705} @{} is verified — Human ID\nENS: {} | {}",
                entry.telegram_username,
                entry.ens_name,
                entry.sbt_summary(),
            );

            bot.send_message(msg.chat.id, badge_msg)
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;

            // Mark as badged
            let mut tracker = badge_tracker.write().await;
            tracker.insert((chat_id, user_id));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Callback handler (inline keyboard buttons)
// ---------------------------------------------------------------------------

async fn handle_callback(
    bot: Bot,
    q: CallbackQuery,
    _state: Arc<BotState>,
    provider: Arc<dyn VerificationProvider>,
) -> ResponseResult<()> {
    let callback_data = q.data.clone();
    let chat_id = match q.message.as_ref() {
        Some(m) => m.chat().id,
        None => {
            bot.answer_callback_query(q.id).await?;
            return Ok(());
        }
    };

    bot.answer_callback_query(q.id).await?;

    let data = match callback_data.as_deref() {
        Some(d) => d,
        None => return Ok(()),
    };

    let (action, wallet_address) = match parse_callback_data(data) {
        Some(pair) => pair,
        None => return Ok(()),
    };

    let mode_label = if provider.is_mock() { "Mock" } else { "Optimism" };

    if action == "verify_all" {
        let checking_msg = format!(
            "\u{1f50d} Checking all verifications for {} on {}...",
            short_addr(wallet_address),
            mode_label,
        );
        bot.send_message(chat_id, checking_msg).await?;

        let results = provider.check_all_verifications(wallet_address).await;
        let response = format_all_results(wallet_address, &results, mode_label);
        bot.send_message(chat_id, response).await?;
    } else if let Some(vtype) = VerificationType::from_callback(action) {
        let checking_msg = format!(
            "\u{1f50d} Checking {} verification for {} on {}...",
            vtype.description(),
            short_addr(wallet_address),
            mode_label,
        );
        bot.send_message(chat_id, checking_msg).await?;

        let result = provider.check_verification(wallet_address, vtype).await;
        let response = format_single_result(wallet_address, vtype, &result, mode_label);
        bot.send_message(chat_id, response).await?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();

    info!("Starting PrivID Telegram Bot...");

    let config = Config::from_env();
    info!("Verification mode: {}", config.verification_mode);

    // Session storage
    let storage_path = PathBuf::from("data/sessions.json");
    let storage = FileStorage::new(storage_path);
    let bot_state = BotState::with_storage(storage);
    if let Err(e) = bot_state.load_from_storage().await {
        warn!("Failed to load sessions: {}", e);
    }
    let shared_state: Arc<BotState> = Arc::new(bot_state);

    // SQLite database
    let database = match Database::new("data/privid.db").await {
        Ok(db) => Arc::new(db),
        Err(e) => {
            panic!("Failed to initialize database: {}", e);
        }
    };

    // Migrate from JSON if registry.json exists
    let registry_json_path = "data/registry.json";
    match database.migrate_from_json(registry_json_path).await {
        Ok(0) => {} // No file to migrate, or empty
        Ok(n) => info!("Migrated {} registrations from JSON to SQLite", n),
        Err(e) => warn!("Failed to migrate registry from JSON: {}", e),
    }

    // Identity registry (SQLite-backed)
    let shared_registry: Arc<Registry> = Arc::new(Registry::new(database.clone()));
    info!("Registry: {} registrations loaded", shared_registry.count().await);

    // ENS resolver (Ethereum mainnet)
    let ens_resolver = Arc::new(EnsResolver::new(config.ethereum_rpc_url.clone()));
    info!("ENS resolver: {}", config.ethereum_rpc_url);

    // Verification provider (mock or blockchain)
    let shared_provider: Arc<dyn VerificationProvider> = match config.verification_mode {
        VerificationMode::Mock => {
            info!("Using mock verification provider");
            Arc::new(MockVerificationProvider::new())
        }
        VerificationMode::Blockchain => {
            info!(
                "Using blockchain verification provider (RPC: {}, Hub: {})",
                config.optimism_rpc_url, config.hub_contract_address
            );
            Arc::new(BlockchainVerificationProvider::new(
                config.optimism_rpc_url.clone(),
                config.hub_contract_address.clone(),
            ))
        }
    };

    // Start HTTP API server alongside Telegram bot
    let api_registry = shared_registry.clone();
    let api_port = config.api_port;
    let api_bind = config.api_bind.clone();
    tokio::spawn(async move {
        start_api_server(api_registry, &api_bind, api_port).await;
    });
    info!("API server started on {}:{}", config.api_bind, config.api_port);

    // Badge tracker (resets on restart)
    let badge_tracker: BadgeTracker = Arc::new(RwLock::new(HashSet::new()));

    // Create bot
    let bot = Bot::new(&config.telegram_bot_token);

    // Resolve the bot's username so we can build t.me deep links for challenges.
    let bot_username: Arc<String> = Arc::new(match bot.get_me().await {
        Ok(me) => me.username.clone().unwrap_or_default(),
        Err(e) => {
            warn!(
                "Could not fetch bot username via get_me ({}). Falling back to BOT_USERNAME env.",
                e
            );
            std::env::var("BOT_USERNAME").unwrap_or_default()
        }
    });
    info!("Bot username: @{}", bot_username);

    // Build dptree dispatcher
    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(handle_command),
        )
        .branch(Update::filter_message().endpoint(handle_message))
        .branch(Update::filter_callback_query().endpoint(handle_callback));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![
            shared_state,
            shared_provider,
            ens_resolver,
            shared_registry,
            badge_tracker,
            database,
            bot_username
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
