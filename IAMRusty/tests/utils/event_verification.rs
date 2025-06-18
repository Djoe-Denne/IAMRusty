use crate::common::mock_event_publisher::MockEventPublisher;
use domain::entity::events::DomainEvent;
use std::sync::Arc;

/// Event Verification Test Utilities
pub struct EventTestUtils;

impl EventTestUtils {
    /// Assert that a PasswordResetRequested event was published
    pub fn assert_password_reset_requested_event_published(
        mock_publisher: &Arc<MockEventPublisher>,
        expected_email: &str,
    ) {
        assert!(
            mock_publisher.has_password_reset_requested_event(),
            "PasswordResetRequested event should be published"
        );

        let events = mock_publisher.get_password_reset_requested_events();
        assert_eq!(
            events.len(),
            1,
            "Exactly one PasswordResetRequested event should be published"
        );

        match &events[0] {
            DomainEvent::PasswordResetRequested(event) => {
                assert_eq!(
                    event.email, expected_email,
                    "Event should contain the correct email"
                );
            }
            other => panic!("Expected PasswordResetRequested event, got: {:?}", other),
        }
    }

    /// Assert that NO PasswordResetRequested event was published
    pub fn assert_no_password_reset_requested_event_published(
        mock_publisher: &Arc<MockEventPublisher>,
    ) {
        assert!(
            !mock_publisher.has_password_reset_requested_event(),
            "PasswordResetRequested event should NOT be published"
        );
        assert_eq!(
            mock_publisher.get_event_count(),
            0,
            "No events should be published"
        );
    }

    /// Assert that a UserSignedUp event was published
    pub fn assert_user_signed_up_event_published(
        mock_publisher: &Arc<MockEventPublisher>,
        expected_user_id: &str,
        expected_email: &str,
    ) {
        let events = mock_publisher.get_published_events();
        
        let user_signed_up_events: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, DomainEvent::UserSignedUp(_)))
            .collect();

        assert_eq!(
            user_signed_up_events.len(),
            1,
            "Exactly one UserSignedUp event should be published"
        );

        match &user_signed_up_events[0] {
            DomainEvent::UserSignedUp(event) => {
                assert_eq!(
                    event.user_id.to_string(), expected_user_id,
                    "Event should contain the correct user ID"
                );
                assert_eq!(
                    event.email, expected_email,
                    "Event should contain the correct email"
                );
            }
            other => panic!("Expected UserSignedUp event, got: {:?}", other),
        }
    }

    /// Assert that NO UserSignedUp event was published
    pub fn assert_no_user_signed_up_event_published(mock_publisher: &Arc<MockEventPublisher>) {
        let events = mock_publisher.get_published_events();
        
        let user_signed_up_events: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, DomainEvent::UserSignedUp(_)))
            .collect();

        assert_eq!(
            user_signed_up_events.len(),
            0,
            "No UserSignedUp event should be published"
        );
    }

    /// Assert exact number of events published
    pub fn assert_event_count(mock_publisher: &Arc<MockEventPublisher>, expected_count: usize) {
        assert_eq!(
            mock_publisher.get_event_count(),
            expected_count,
            "Should publish exactly {} events",
            expected_count
        );
    }

    /// Clear all events from mock publisher
    pub fn clear_events(mock_publisher: &Arc<MockEventPublisher>) {
        mock_publisher.clear_events();
    }

    /// Get all published events for custom verification
    pub fn get_published_events(mock_publisher: &Arc<MockEventPublisher>) -> Vec<DomainEvent> {
        mock_publisher.get_published_events()
    }

    /// Assert that a specific event type was published
    pub fn assert_event_type_published<F>(
        mock_publisher: &Arc<MockEventPublisher>,
        predicate: F,
        event_description: &str,
    ) where
        F: Fn(&DomainEvent) -> bool,
    {
        let events = mock_publisher.get_published_events();
        let matching_events: Vec<_> = events.iter().filter(|event| predicate(event)).collect();

        assert!(
            !matching_events.is_empty(),
            "{} should be published",
            event_description
        );
    }

    /// Assert that a specific event type was NOT published
    pub fn assert_event_type_not_published<F>(
        mock_publisher: &Arc<MockEventPublisher>,
        predicate: F,
        event_description: &str,
    ) where
        F: Fn(&DomainEvent) -> bool,
    {
        let events = mock_publisher.get_published_events();
        let matching_events: Vec<_> = events.iter().filter(|event| predicate(event)).collect();

        assert!(
            matching_events.is_empty(),
            "{} should NOT be published",
            event_description
        );
    }
} 