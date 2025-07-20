use privid_telegram_bot::{BotState, UserSession, VerificationState, VerificationResult, FileStorage};
use std::path::PathBuf;
use tempfile::tempdir;

#[tokio::test]
async fn test_file_storage() {
    // Create a temporary directory for testing
    let temp_dir = tempdir().unwrap();
    let storage_path = temp_dir.path().join("test_sessions.json");
    
    // Create storage instance
    let storage = FileStorage::new(storage_path.clone());
    let bot_state = BotState::with_storage(storage);
    
    // Create a test session
    let mut session = UserSession::new(12345);
    let mock_result = VerificationResult {
        verified: true,
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        proof: "test-proof-123".to_string(),
        badge: "Test Badge".to_string(),
    };
    session.update_verification_state(VerificationState::Completed { verification_result: mock_result });
    
    // Add session to state
    bot_state.update_session(12345, session.clone()).await;
    
    // Save to storage
    bot_state.save_to_storage().await.unwrap();
    
    // Verify file was created
    assert!(storage_path.exists());
    
    // Create new bot state and load from storage
    let storage2 = FileStorage::new(storage_path);
    let bot_state2 = BotState::with_storage(storage2);
    bot_state2.load_from_storage().await.unwrap();
    
    // Verify session was loaded correctly
    let loaded_session = bot_state2.get_session(12345).await.unwrap();
    assert_eq!(loaded_session.user_id, 12345);
    assert!(matches!(loaded_session.verification_state, VerificationState::Completed { .. }));
    
    // Clean up
    temp_dir.close().unwrap();
} 