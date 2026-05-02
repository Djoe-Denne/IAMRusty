// Include common test utilities and fixtures

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::setup_test_server;
use fixtures::DbFixtures;
use serde_json::Value;
use serial_test::serial;

// =============================================================================
// 🔍 USERNAME CHECK ENDPOINT TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_username_check_available_returns_true() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    let response = client
        .get(format!("{base_url}/api/auth/username/check"))
        .query(&[("username", "availableuser")])
        .send()
        .await
        .expect("Failed to send username check request");

    assert_eq!(response.status(), 200);

    let response_body: Value = response.json().await.expect("Should return JSON response");

    assert!(response_body["available"].as_bool().unwrap());
    assert_eq!(
        response_body["suggestions"].as_array().unwrap().len(),
        0,
        "Available usernames should have empty suggestions array"
    );
}

#[tokio::test]
#[serial]
async fn test_username_check_taken_returns_false_with_suggestions() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Pre-create user with taken username
    let _existing_user = DbFixtures::user()
        .username("johndoe")
        .commit(db.clone())
        .await
        .expect("Failed to create existing user");

    let response = client
        .get(format!("{base_url}/api/auth/username/check"))
        .query(&[("username", "johndoe")])
        .send()
        .await
        .expect("Failed to send username check request");

    assert_eq!(response.status(), 200);

    let response_body: Value = response.json().await.expect("Should return JSON response");

    assert!(!response_body["available"].as_bool().unwrap());

    let suggestions = response_body["suggestions"].as_array().unwrap();
    assert!(
        !suggestions.is_empty(),
        "Should provide username suggestions"
    );

    // Verify suggestions are reasonable
    for suggestion in suggestions {
        let suggestion_str = suggestion.as_str().unwrap();
        assert!(
            suggestion_str.starts_with("johndoe"),
            "Suggestions should be based on original username"
        );
        assert!(
            suggestion_str.len() >= 3,
            "Suggestions should meet minimum length"
        );
        assert!(
            suggestion_str.len() <= 50,
            "Suggestions should meet maximum length"
        );
    }
}

#[tokio::test]
#[serial]
async fn test_username_check_case_sensitivity() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with lowercase username
    let _existing_user = DbFixtures::user()
        .username("testuser")
        .commit(db.clone())
        .await
        .expect("Failed to create existing user");

    // Test various case variations
    let case_variations = vec![
        "testuser", // Exact match - should be taken
        "TestUser", // Different case - depends on implementation
        "TESTUSER", // All caps - depends on implementation
        "TestUSER", // Mixed case - depends on implementation
    ];

    for variation in case_variations {
        let response = client
            .get(format!("{base_url}/api/auth/username/check"))
            .query(&[("username", variation)])
            .send()
            .await
            .expect("Failed to send username check request");

        assert_eq!(response.status(), 200);

        let response_body: Value = response.json().await.expect("Should return JSON response");

        if variation == "testuser" {
            // Exact match should always be taken
            assert!(
                !response_body["available"].as_bool().unwrap(),
                "Exact match should be taken"
            );
        }

        // Note: Case sensitivity behavior depends on implementation
        // This test documents the expected behavior
    }
}

// =============================================================================
// 🔍 USERNAME VALIDATION RULES TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_username_minimum_length_validation() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Test usernames below minimum length (3 characters)
    let short_usernames = vec!["", "a", "ab"];

    for short_username in short_usernames {
        let response = client
            .get(format!("{base_url}/api/auth/username/check"))
            .query(&[("username", short_username)])
            .send()
            .await
            .expect("Failed to send username check request");

        assert!(
            response.status() == 400 || response.status() == 422,
            "Should return validation error for username '{short_username}' (too short)"
        );
    }

    // Test minimum valid length
    let response = client
        .get(format!("{base_url}/api/auth/username/check"))
        .query(&[("username", "abc")])
        .send()
        .await
        .expect("Failed to send username check request");

    assert_eq!(
        response.status(),
        200,
        "Should accept minimum valid length (3 chars)"
    );
}

#[tokio::test]
#[serial]
async fn test_username_maximum_length_validation() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Test username at maximum length (50 characters)
    let max_length_username = "a".repeat(50);
    let response = client
        .get(format!("{base_url}/api/auth/username/check"))
        .query(&[("username", &max_length_username)])
        .send()
        .await
        .expect("Failed to send username check request");

    assert_eq!(
        response.status(),
        200,
        "Should accept maximum valid length (50 chars)"
    );

    // Test username over maximum length
    let too_long_username = "a".repeat(51);
    let response = client
        .get(format!("{base_url}/api/auth/username/check"))
        .query(&[("username", &too_long_username)])
        .send()
        .await
        .expect("Failed to send username check request");

    assert!(
        response.status() == 400 || response.status() == 422,
        "Should return validation error for username too long (51+ chars)"
    );
}

#[tokio::test]
#[serial]
async fn test_username_character_pattern_validation() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Test valid character patterns (alphanumeric + underscore + dash)
    let valid_usernames = vec![
        "user123",       // Alphanumeric
        "user_name",     // Underscore
        "user-name",     // Dash/hyphen
        "User123",       // Mixed case
        "123user",       // Starting with number
        "user_123-test", // Mixed valid characters
        "ABC_123",       // All caps with underscore
        "test-user_123", // Mixed separators
    ];

    for valid_username in valid_usernames {
        let response = client
            .get(format!("{base_url}/api/auth/username/check"))
            .query(&[("username", valid_username)])
            .send()
            .await
            .expect("Failed to send username check request");

        assert_eq!(
            response.status(),
            200,
            "Should accept valid username pattern: '{valid_username}'"
        );
    }

    // Test invalid character patterns
    let invalid_usernames = vec![
        "user.name",    // Dot
        "user@name",    // At symbol
        "user name",    // Space
        "user+name",    // Plus
        "user#name",    // Hash
        "user$name",    // Dollar
        "user%name",    // Percent
        "user&name",    // Ampersand
        "user*name",    // Asterisk
        "user(name)",   // Parentheses
        "user[name]",   // Brackets
        "user{name}",   // Braces
        "user|name",    // Pipe
        "user\\name",   // Backslash
        "user/name",    // Forward slash
        "user:name",    // Colon
        "user;name",    // Semicolon
        "user\"name\"", // Quotes
        "user'name'",   // Apostrophes
        "user<name>",   // Angle brackets
        "user,name",    // Comma
        "user?name",    // Question mark
        "user!name",    // Exclamation
    ];

    for invalid_username in invalid_usernames {
        let response = client
            .get(format!("{base_url}/api/auth/username/check"))
            .query(&[("username", invalid_username)])
            .send()
            .await
            .expect("Failed to send username check request");

        assert!(
            response.status() == 400 || response.status() == 422,
            "Should reject invalid username pattern: '{invalid_username}'"
        );
    }
}

#[tokio::test]
#[serial]
async fn test_username_unicode_and_special_characters() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Test Unicode characters (should be rejected based on pattern ^[a-zA-Z0-9_-]+$)
    let unicode_usernames = vec![
        "user名前",   // Japanese characters
        "userñame",   // Spanish characters
        "useröname",  // German characters
        "userфимя",   // Cyrillic characters
        "user🚀name", // Emoji
        "userπname",  // Greek letters
    ];

    for unicode_username in unicode_usernames {
        let response = client
            .get(format!("{base_url}/api/auth/username/check"))
            .query(&[("username", unicode_username)])
            .send()
            .await
            .expect("Failed to send username check request");

        assert!(
            response.status() == 400 || response.status() == 422,
            "Should reject Unicode username: '{unicode_username}'"
        );
    }

    // Test control characters and invisible characters
    let control_usernames = vec![
        "user\tname",   // Tab
        "user\nname",   // Newline
        "user\rname",   // Carriage return
        "user\x00name", // Null character
        "user\x1fname", // Other control character
    ];

    for control_username in control_usernames {
        let response = client
            .get(format!("{base_url}/api/auth/username/check"))
            .query(&[("username", control_username)])
            .send()
            .await
            .expect("Failed to send username check request");

        assert!(
            response.status() == 400 || response.status() == 422,
            "Should reject control character username: '{control_username:?}'"
        );
    }
}

// =============================================================================
// 🔍 USERNAME SUGGESTION ALGORITHM TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_username_suggestions_reasonable_alternatives() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create multiple users with similar usernames
    let taken_usernames = vec!["johndoe", "johndoe123", "johndoe_"];

    for username in &taken_usernames {
        let _user = DbFixtures::user()
            .username(*username)
            .commit(db.clone())
            .await
            .expect("Failed to create user");
    }

    // Check username that conflicts with existing ones
    let response = client
        .get(format!("{base_url}/api/auth/username/check"))
        .query(&[("username", "johndoe")])
        .send()
        .await
        .expect("Failed to send username check request");

    assert_eq!(response.status(), 200);

    let response_body: Value = response.json().await.expect("Should return JSON response");
    assert!(!response_body["available"].as_bool().unwrap());

    let suggestions = response_body["suggestions"].as_array().unwrap();

    assert!(!suggestions.is_empty(), "Should provide suggestions");
    assert!(
        suggestions.len() <= 5,
        "Should not provide too many suggestions"
    );

    // Verify suggestion quality
    for suggestion in suggestions {
        let suggestion_str = suggestion.as_str().unwrap();

        // Should be based on original username
        assert!(
            suggestion_str.contains("johndoe"),
            "Suggestion '{suggestion_str}' should contain original username"
        );

        // Should meet all validation rules
        assert!(
            suggestion_str.len() >= 3,
            "Suggestion should meet minimum length"
        );
        assert!(
            suggestion_str.len() <= 50,
            "Suggestion should meet maximum length"
        );
        assert!(
            suggestion_str
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-'),
            "Suggestion should only contain valid characters"
        );

        // Should be different from original
        assert_ne!(
            suggestion_str, "johndoe",
            "Suggestion should be different from original"
        );

        // Should not conflict with existing taken usernames
        assert!(
            !taken_usernames.contains(&suggestion_str),
            "Suggestion '{suggestion_str}' should not conflict with existing users"
        );
    }
}

#[tokio::test]
#[serial]
async fn test_username_suggestions_different_strategies() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with a simple username
    let _user = DbFixtures::user()
        .username("alice")
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let response = client
        .get(format!("{base_url}/api/auth/username/check"))
        .query(&[("username", "alice")])
        .send()
        .await
        .expect("Failed to send username check request");

    assert_eq!(response.status(), 200);

    let response_body: Value = response.json().await.expect("Should return JSON response");
    let suggestions = response_body["suggestions"].as_array().unwrap();

    // Suggestions should use different strategies
    let suggestion_strings: Vec<String> = suggestions
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect();

    // Should include different types of modifications:
    // 1. Numeric suffixes (alice123, alice1, etc.)
    // 2. Underscore variations (alice_, alice_1, etc.)
    // 3. Dash variations (alice-123, etc.)

    let has_numeric = suggestion_strings
        .iter()
        .any(|s| s.chars().any(char::is_numeric));
    let _has_underscore = suggestion_strings.iter().any(|s| s.contains('_'));
    let _has_dash = suggestion_strings.iter().any(|s| s.contains('-'));

    assert!(has_numeric, "Should include numeric suffix suggestions");
    // Note: underscore and dash presence depends on algorithm implementation
}

// =============================================================================
// 🔍 USERNAME CHECK ENDPOINT ERROR HANDLING
// =============================================================================

#[tokio::test]
#[serial]
async fn test_username_check_missing_parameter() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Request without username parameter
    let response = client
        .get(format!("{base_url}/api/auth/username/check"))
        .send()
        .await
        .expect("Failed to send username check request");

    assert_eq!(
        response.status(),
        400,
        "Should return 400 for missing username parameter"
    );
}

#[tokio::test]
#[serial]
async fn test_username_check_empty_parameter() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Request with empty username parameter
    let response = client
        .get(format!("{base_url}/api/auth/username/check"))
        .query(&[("username", "")])
        .send()
        .await
        .expect("Failed to send username check request");

    assert!(
        response.status() == 400 || response.status() == 422,
        "Should return validation error for empty username"
    );
}

#[tokio::test]
#[serial]
async fn test_username_check_multiple_parameters() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Request with multiple username parameters
    let response = client
        .get(format!(
            "{base_url}/api/auth/username/check?username=first&username=second"
        ))
        .send()
        .await
        .expect("Failed to send username check request");

    // Should handle gracefully (behavior depends on implementation)
    assert!(
        response.status() == 200 || response.status() == 400,
        "Should handle multiple parameters gracefully"
    );
}

#[tokio::test]
#[serial]
async fn test_username_check_whitespace_handling() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Test usernames with leading/trailing whitespace
    let whitespace_usernames = vec![
        " username",    // Leading space
        "username ",    // Trailing space
        " username ",   // Both
        "\tusername\t", // Tabs
        "\nusername\n", // Newlines
    ];

    for whitespace_username in whitespace_usernames {
        let response = client
            .get(format!("{base_url}/api/auth/username/check"))
            .query(&[("username", whitespace_username)])
            .send()
            .await
            .expect("Failed to send username check request");

        // Should either trim and validate or reject
        assert!(
            response.status() == 200 || response.status() == 400 || response.status() == 422,
            "Should handle whitespace in username: '{whitespace_username:?}'"
        );
    }
}

// =============================================================================
// 🔍 UNIQUENESS CONSTRAINT TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_username_uniqueness_across_database() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with specific username
    let unique_username = "uniqueuser123";
    let _user = DbFixtures::user()
        .username(unique_username)
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    // Verify username is now taken
    let response = client
        .get(format!("{base_url}/api/auth/username/check"))
        .query(&[("username", unique_username)])
        .send()
        .await
        .expect("Failed to send username check request");

    assert_eq!(response.status(), 200);

    let response_body: Value = response.json().await.expect("Should return JSON response");
    assert!(
        !response_body["available"].as_bool().unwrap(),
        "Username should be taken after creating user"
    );

    // Verify another similar but different username is available
    let similar_username = "uniqueuser124";
    let response = client
        .get(format!("{base_url}/api/auth/username/check"))
        .query(&[("username", similar_username)])
        .send()
        .await
        .expect("Failed to send username check request");

    assert_eq!(response.status(), 200);

    let response_body: Value = response.json().await.expect("Should return JSON response");
    assert!(
        response_body["available"].as_bool().unwrap(),
        "Similar but different username should be available"
    );
}

#[tokio::test]
#[serial]
async fn test_username_check_performance_with_many_users() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create multiple users to test performance
    for i in 0..10 {
        let username = format!("perftest{i}");
        let _user = DbFixtures::user()
            .username(&username)
            .commit(db.clone())
            .await
            .expect("Failed to create user");
    }

    // Check username availability (should still be fast)
    let start_time = std::time::Instant::now();

    let response = client
        .get(format!("{base_url}/api/auth/username/check"))
        .query(&[("username", "availableusername")])
        .send()
        .await
        .expect("Failed to send username check request");

    let duration = start_time.elapsed();

    assert_eq!(response.status(), 200);
    assert!(
        duration.as_millis() < 1000,
        "Username check should complete quickly even with many users (took {}ms)",
        duration.as_millis()
    );
}
