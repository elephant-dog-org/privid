use privid_telegram_bot::{BotState, UserSession, VerificationState, VerificationResult};

#[tokio::test]
async fn test_bot_state_management() {
    let state = BotState::new();
    
    // Test creating a new session
    let session = state.get_or_create_session(12345).await;
    assert_eq!(session.user_id, 12345);
    assert!(matches!(session.verification_state, VerificationState::NotStarted));
    
    // Test retrieving existing session
    let session2 = state.get_or_create_session(12345).await;
    assert_eq!(session.user_id, session2.user_id);
    
    // Test updating session
    let mut updated_session = session.clone();
    let mock_result = VerificationResult {
        verified: true,
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        proof: "test-proof".to_string(),
        badge: "Test Badge".to_string(),
    };
    updated_session.update_verification_state(VerificationState::Completed { verification_result: mock_result });
    state.update_session(12345, updated_session.clone()).await;
    
    let retrieved_session = state.get_session(12345).await.unwrap();
    assert!(matches!(retrieved_session.verification_state, VerificationState::Completed { .. }));
}

#[tokio::test]
async fn test_mock_verification_result_generation() {
    let result1 = UserSession::generate_mock_verification_result();
    let result2 = UserSession::generate_mock_verification_result();
    
    // Test that results are generated correctly
    assert!(result1.verified);
    assert!(result2.verified);
    assert!(!result1.proof.is_empty());
    assert!(!result2.proof.is_empty());
    assert!(!result1.badge.is_empty());
    assert!(!result2.badge.is_empty());
    assert!(!result1.timestamp.is_empty());
    assert!(!result2.timestamp.is_empty());
    
    // Test that different calls generate different results
    assert_ne!(result1.proof, result2.proof);
}

#[tokio::test]
async fn test_verification_state_transitions() {
    let mut session = UserSession::new(12345);
    
    // Test initial state
    assert!(matches!(session.verification_state, VerificationState::NotStarted));
    
    // Test transition to in progress
    session.update_verification_state(VerificationState::InProgress { 
        verification_id: "test_id".to_string() 
    });
    assert!(matches!(session.verification_state, VerificationState::InProgress { .. }));
    
    // Test transition to completed
    let mock_result = VerificationResult {
        verified: true,
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        proof: "test-proof".to_string(),
        badge: "Test Badge".to_string(),
    };
    session.update_verification_state(VerificationState::Completed { verification_result: mock_result });
    assert!(matches!(session.verification_state, VerificationState::Completed { .. }));
    
    // Test transition to failed
    session.update_verification_state(VerificationState::Failed { 
        reason: "test failure".to_string() 
    });
    assert!(matches!(session.verification_state, VerificationState::Failed { .. }));
} 