mod blockchain;
mod config;
mod state;
mod storage;
mod verification;

use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use teloxide::utils::command::BotCommands;
use log::info;
use dotenv::dotenv;
use std::sync::Arc;
use std::path::PathBuf;

use crate::blockchain::types::VerificationType;
use crate::config::{Config, VerificationMode};
use crate::state::BotState;
use crate::storage::FileStorage;
use crate::verification::provider::{VerificationError, VerificationProvider};
use crate::verification::mock::MockVerificationProvider;
use crate::blockchain::provider::BlockchainVerificationProvider;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
enum Command {
    #[command(description = "Start the bot and see welcome message")]
    Start,
    #[command(description = "Check wallet verification: /verify 0xABC...")]
    Verify(String),
    #[command(description = "Check your verification status")]
    Status,
    #[command(description = "Show help information")]
    Help,
}

/// Format a wallet address for display: first 6 chars + ... + last 2 chars
fn short_addr(addr: &str) -> String {
    if addr.len() > 8 {
        format!("{}...{}", &addr[..6], &addr[addr.len() - 2..])
    } else {
        addr.to_string()
    }
}

/// Check whether a string looks like a valid Ethereum wallet address.
fn is_wallet_address(s: &str) -> bool {
    s.starts_with("0x") && s.len() == 42 && s[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Build the inline keyboard with verification type buttons.
/// Each button encodes `<callback_data>:<wallet_address>` so the wallet address
/// travels with the callback and no separate state store is needed.
fn verification_keyboard(wallet_address: &str) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    // Row 1: KYC, Phone, Passport
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

    // Row 2: Clean Hands, Biometrics
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

    // Row 3: Check All
    rows.push(vec![InlineKeyboardButton::callback(
        "Check All",
        format!("verify_all:{}", wallet_address),
    )]);

    InlineKeyboardMarkup::new(rows)
}

/// Format the result of a single verification type check.
fn format_single_result(
    wallet_address: &str,
    vtype: VerificationType,
    result: &Result<crate::state::VerificationResult, VerificationError>,
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
                 {} {}: Verified{}\n  Proof: {}",
                vtype.description(),
                addr_short,
                mode_label,
                "\u{2705}",  // checkmark
                vtype.description(),
                expiry_line,
                vr.proof,
            )
        }
        Err(e) => {
            format!(
                "Checking {} verification for {} on {}...\n\n\
                 {} {}: {}",
                vtype.description(),
                addr_short,
                mode_label,
                "\u{274c}",  // X
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

/// Format the results of checking all verification types.
fn format_all_results(
    wallet_address: &str,
    results: &[(VerificationType, Result<crate::state::VerificationResult, VerificationError>)],
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

/// Parse callback data in the format `action:wallet_address`.
/// Returns `(action, wallet_address)`.
fn parse_callback_data(data: &str) -> Option<(&str, &str)> {
    let colon_pos = data.find(':')?;
    let action = &data[..colon_pos];
    let wallet = &data[colon_pos + 1..];
    if wallet.is_empty() {
        return None;
    }
    Some((action, wallet))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: Arc<BotState>,
    provider: Arc<dyn VerificationProvider>,
) -> ResponseResult<()> {
    match cmd {
        Command::Start => {
            let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
            let _session = state.get_or_create_session(user_id).await;

            let welcome = "\
                Welcome to PrivID!\n\n\
                I'm your privacy-respecting identity verification bot. \
                I use zero-knowledge proofs to verify your identity without \
                collecting or storing your personal data.\n\n\
                Available commands:\n\
                /verify <wallet> - Check wallet verification status\n\
                /status - Check your verification status\n\
                /help - Show help information\n\n\
                You can also just paste a wallet address (0x...) and I'll look it up.";
            bot.send_message(msg.chat.id, welcome).await?;
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
                "Select a verification type to check for {}:\n\n\
                 Mode: {}",
                short_addr(&wallet),
                mode_label,
            );

            bot.send_message(msg.chat.id, prompt)
                .reply_markup(verification_keyboard(&wallet))
                .await?;
        }

        Command::Status => {
            let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
            let session = state.get_session(user_id).await;

            let text = if let Some(session) = session {
                match &session.verification_state {
                    crate::state::VerificationState::NotStarted => {
                        "Verification Status: Not Started\n\n\
                         You haven't started the verification process yet.\n\n\
                         Use /verify <wallet> to check a wallet address."
                            .to_string()
                    }
                    crate::state::VerificationState::InProgress { verification_id } => {
                        format!(
                            "Verification Status: In Progress\n\n\
                             Verification ID: {}\n\
                             Started: {}",
                            verification_id,
                            session.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
                        )
                    }
                    crate::state::VerificationState::Completed { verification_result } => {
                        format!(
                            "Verification Status: Verified\n\n\
                             Badge: {}\n\
                             Proof ID: {}\n\
                             Verified: {}\n\
                             Session Created: {}\n\
                             Last Updated: {}",
                            verification_result.badge,
                            verification_result.proof,
                            verification_result.timestamp,
                            session.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
                            session.updated_at.format("%Y-%m-%d %H:%M:%S UTC"),
                        )
                    }
                    crate::state::VerificationState::Failed { reason } => {
                        format!(
                            "Verification Status: Failed\n\n\
                             Reason: {}\n\
                             Failed on: {}\n\n\
                             You can try again with /verify <wallet>.",
                            reason,
                            session.updated_at.format("%Y-%m-%d %H:%M:%S UTC"),
                        )
                    }
                }
            } else {
                "Verification Status: No Session\n\n\
                 You haven't interacted with the bot yet.\n\n\
                 Use /start to begin or /verify <wallet> to check a wallet."
                    .to_string()
            };

            bot.send_message(msg.chat.id, text).await?;
        }

        Command::Help => {
            let help = "\
                PrivID Bot Help\n\n\
                Available commands:\n\
                /start - Start the bot and see welcome message\n\
                /verify <wallet> - Check wallet verification (e.g. /verify 0xABC...)\n\
                /status - Check your verification status\n\
                /help - Show this help information\n\n\
                You can also paste a wallet address directly and I'll look it up.\n\n\
                About PrivID:\n\
                PrivID is a privacy-respecting identity verification system \
                that uses zero-knowledge proofs to verify your identity \
                without collecting or storing your personal data.\n\n\
                Privacy Features:\n\
                - Zero-knowledge proofs\n\
                - No personal data storage\n\
                - End-to-end encryption\n\
                - Anonymous verification";

            bot.send_message(msg.chat.id, help).await?;
        }
    }

    Ok(())
}

/// Handle bare wallet address messages (quality-of-life shortcut).
async fn handle_message(
    bot: Bot,
    msg: Message,
    provider: Arc<dyn VerificationProvider>,
) -> ResponseResult<()> {
    let text = match msg.text() {
        Some(t) => t.trim(),
        None => return Ok(()),
    };

    if is_wallet_address(text) {
        let mode_label = if provider.is_mock() { "Mock" } else { "Optimism" };
        let prompt = format!(
            "Select a verification type to check for {}:\n\n\
             Mode: {}",
            short_addr(text),
            mode_label,
        );

        bot.send_message(msg.chat.id, prompt)
            .reply_markup(verification_keyboard(text))
            .await?;
    }

    Ok(())
}

/// Handle inline keyboard callback queries.
async fn handle_callback(
    bot: Bot,
    q: CallbackQuery,
    _state: Arc<BotState>,
    provider: Arc<dyn VerificationProvider>,
) -> ResponseResult<()> {
    // Extract everything we need before consuming q.id
    let callback_data = q.data.clone();
    let chat_id = match q.message.as_ref() {
        Some(m) => m.chat().id,
        None => {
            bot.answer_callback_query(q.id).await?;
            return Ok(());
        }
    };

    // Acknowledge the callback immediately to stop the spinner
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
        // Check all verification types
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
        // Check a single verification type
        let checking_msg = format!(
            "\u{1f50d} Checking {} verification for {} on {}...",
            vtype.description(),
            short_addr(wallet_address),
            mode_label,
        );
        bot.send_message(chat_id, checking_msg).await?;

        let result = provider
            .check_verification(wallet_address, vtype)
            .await;
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
    // Load environment variables
    dotenv().ok();

    // Initialize logging
    env_logger::init();

    info!("Starting PrivID Telegram Bot...");

    // Load configuration
    let config = Config::from_env();
    info!("Verification mode: {}", config.verification_mode);

    // Initialize storage & shared state
    let storage_path = PathBuf::from("data/sessions.json");
    let storage = FileStorage::new(storage_path);
    let bot_state = BotState::with_storage(storage);

    // Load existing sessions from disk
    if let Err(e) = bot_state.load_from_storage().await {
        info!("Failed to load sessions from storage: {}", e);
    }

    let shared_state: Arc<BotState> = Arc::new(bot_state);

    // Create provider based on config
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

    // Create bot
    let bot = Bot::new(&config.telegram_bot_token);

    // Build the dptree dispatcher
    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(handle_command),
        )
        .branch(Update::filter_message().endpoint(handle_message))
        .branch(Update::filter_callback_query().endpoint(handle_callback));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![shared_state, shared_provider])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
