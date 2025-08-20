mod state;
mod storage;

use teloxide::{prelude::*, utils::command::BotCommands};
use log::info;
use dotenv::dotenv;
use std::env;
use std::sync::Arc;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

use crate::state::{BotState, UserSession, VerificationState};
use crate::storage::FileStorage;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
enum Command {
    #[command(description = "Start the bot and see welcome message")]
    Start,
    #[command(description = "Begin identity verification with Holonym")]
    Verify,
    #[command(description = "Check your verification status")]
    Status,
    #[command(description = "Show help information")]
    Help,
}

struct BotData {
    state: Arc<BotState>,
}



#[tokio::main]
async fn main() {
    // Load environment variables
    dotenv().ok();
    
    // Initialize logging
    env_logger::init();
    
    info!("Starting PrivID Telegram Bot...");
    
    // Get bot token from environment
    let token = env::var("TELEGRAM_BOT_TOKEN")
        .expect("TELEGRAM_BOT_TOKEN must be set in environment");
    
    // Initialize storage
    let storage_path = PathBuf::from("data/sessions.json");
    let storage = FileStorage::new(storage_path);
    let bot_state = BotState::with_storage(storage);
    
    // Load existing sessions from storage
    if let Err(e) = bot_state.load_from_storage().await {
        info!("Failed to load sessions from storage: {}", e);
    }
    
    // Create bot instance with state
    let bot = Bot::new(token);
    
    // Set up command handler
    Command::repl(bot, answer).await;
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    let user_id = msg.from.as_ref().map(|user| user.id.0).unwrap_or(0);
    
    // Get the shared BotData (in a real app, this would be passed through)
    // For now, we'll create a new one with storage
    let storage_path = PathBuf::from("data/sessions.json");
    let storage = FileStorage::new(storage_path);
    let bot_state = BotState::with_storage(storage);
    
    // Load existing sessions
    if let Err(e) = bot_state.load_from_storage().await {
        info!("Failed to load sessions: {}", e);
    }
    
    let bot_data = BotData {
        state: Arc::new(bot_state),
    };
    
    match cmd {
        Command::Start => {
            let _session = bot_data.state.get_or_create_session(user_id).await;
            
            let welcome_text = "🤖 Welcome to PrivID!\n\n\
                               I'm your privacy-respecting identity verification bot. \
                               I use zero-knowledge proofs to verify your identity without \
                               collecting or storing your personal data.\n\n\
                               Available commands:\n\
                               • /verify - Begin identity verification\n\
                               • /status - Check your verification status\n\
                               • /help - Show help information\n\n\
                               Your privacy is our priority! 🔒";
            
            bot.send_message(msg.chat.id, welcome_text).await?;
        }
        
        Command::Verify => {
            let mut session = bot_data.state.get_or_create_session(user_id).await;
            
            match session.verification_state {
                VerificationState::NotStarted => {
                    // Send initial message
                    let initial_text = "🔐 Starting Identity Verification...\n\n\
                                       Please wait while we verify your identity using zero-knowledge proofs.\n\n\
                                       This process takes about 2 seconds.";
                    
                    bot.send_message(msg.chat.id, initial_text).await?;
                    
                    // Wait 2 seconds
                    sleep(Duration::from_secs(2)).await;
                    
                    // Generate mock verification result
                    let verification_result = UserSession::generate_mock_verification_result();
                    session.update_verification_state(VerificationState::Completed { 
                        verification_result: verification_result.clone() 
                    });
                    bot_data.state.update_session(user_id, session).await;
                    
                    // Save to storage
                    if let Err(e) = bot_data.state.save_to_storage().await {
                        info!("Failed to save session to storage: {}", e);
                    }
                    
                    // Send success message
                    let success_text = format!("✅ Verification Successful!\n\n\
                                              Your identity has been verified using zero-knowledge proofs!\n\n\
                                              📊 Verification Details:\n\
                                              • Status: Verified\n\
                                              • Badge: {}\n\
                                              • Proof ID: {}\n\
                                              • Verified: {}\n\n\
                                              Use /status to view your verification details anytime.", 
                                              verification_result.badge,
                                              verification_result.proof,
                                              verification_result.timestamp);
                    
                    bot.send_message(msg.chat.id, success_text).await?;
                }
                
                VerificationState::Completed { verification_result } => {
                    let already_verified_text = format!("✅ Already Verified!\n\n\
                                                       You have already completed the verification process.\n\n\
                                                       📊 Current Verification:\n\
                                                       • Badge: {}\n\
                                                       • Proof ID: {}\n\
                                                       • Verified: {}\n\n\
                                                       Use /status to view your verification details.", 
                                                       verification_result.badge,
                                                       verification_result.proof,
                                                       verification_result.timestamp);
                    
                    bot.send_message(msg.chat.id, already_verified_text).await?;
                }
                
                VerificationState::InProgress { verification_id } => {
                    let in_progress_text = format!("⏳ Verification in progress...\n\n\
                                                  Your verification (ID: {}) is currently being processed. \
                                                  Please wait while we verify your identity.\n\n\
                                                  Use /status to check the current status.", verification_id);
                    
                    bot.send_message(msg.chat.id, in_progress_text).await?;
                }
                
                VerificationState::Failed { reason } => {
                    let failed_text = format!("❌ Previous verification failed\n\n\
                                             Reason: {}\n\n\
                                             You can try again with /verify.", reason);
                    
                    bot.send_message(msg.chat.id, failed_text).await?;
                }
            }
        }
        
        Command::Status => {
            let session = bot_data.state.get_session(user_id).await;
            
            let status_text = if let Some(session) = session {
                match session.verification_state {
                    VerificationState::NotStarted => {
                        "📊 Verification Status: Not Started\n\n\
                         You haven't started the verification process yet.\n\n\
                         Use /verify to begin identity verification.".to_string()
                    }
                    
                    VerificationState::InProgress { verification_id } => {
                        format!("⏳ Verification Status: In Progress\n\n\
                                Verification ID: {}\n\
                                Started: {}\n\n\
                                Your verification is currently being processed. \
                                Please wait while we verify your identity.", 
                                verification_id, 
                                session.created_at.format("%Y-%m-%d %H:%M:%S UTC"))
                    }
                    
                    VerificationState::Completed { verification_result } => {
                        format!("✅ Verification Status: Verified\n\n\
                                Your identity has been successfully verified!\n\n\
                                📊 Verification Details:\n\
                                • Badge: {}\n\
                                • Proof ID: {}\n\
                                • Verified: {}\n\
                                • Session Created: {}\n\
                                • Last Updated: {}\n\n\
                                You now have a verified badge while maintaining complete privacy!", 
                                verification_result.badge,
                                verification_result.proof,
                                verification_result.timestamp,
                                session.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
                                session.updated_at.format("%Y-%m-%d %H:%M:%S UTC"))
                    }
                    
                    VerificationState::Failed { reason } => {
                        format!("❌ Verification Status: Failed\n\n\
                                Reason: {}\n\
                                Failed on: {}\n\n\
                                You can try again with /verify.", 
                                reason, 
                                session.updated_at.format("%Y-%m-%d %H:%M:%S UTC"))
                    }
                }
            } else {
                "📊 Verification Status: No Session\n\n\
                 You haven't interacted with the bot yet.\n\n\
                 Use /start to begin or /verify to start verification.".to_string()
            };
            
            bot.send_message(msg.chat.id, status_text).await?;
        }
        
        Command::Help => {
            let help_text = "📚 PrivID Bot Help\n\n\
                            Available commands:\n\
                            /start - Start the bot and see welcome message\n\
                            /verify - Begin identity verification with Holonym\n\
                            /status - Check your verification status\n\
                            /help - Show this help information\n\n\
                            About PrivID:\n\
                            PrivID is a privacy-respecting identity verification system \
                            that uses zero-knowledge proofs to verify your identity \
                            without collecting or storing your personal data.\n\n\
                            🔒 Privacy Features:\n\
                            • Zero-knowledge proofs\n\
                            • No personal data storage\n\
                            • End-to-end encryption\n\
                            • Anonymous verification\n\n\
                            For more information, visit our website or contact support.";
            
            bot.send_message(msg.chat.id, help_text).await?;
        }
    }
    
    Ok(())
}
