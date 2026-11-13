//! QA coverage90 C: event-to-command extraction and validation, no use-case mock.
use std::sync::Arc;

use iam_events::{IamDomainEvent, UserSignedUpEvent};
use rustycog::command::{Command, CommandError};
use telegraph_application::{ProcessEventCommand, SendMessageRecipient};
use uuid::Uuid;

fn signup() -> UserSignedUpEvent {
    UserSignedUpEvent::new(
        Uuid::from_u128(42),
        "ada@example.test".into(),
        "ada".into(),
        true,
        None,
        None,
    )
}

fn command() -> ProcessEventCommand {
    ProcessEventCommand::new(Arc::new(signup()))
}

#[test]
fn direct_and_tagged_signup_extract_the_same_recipient_and_original_identity() {
    let event = signup();
    let direct = ProcessEventCommand::new(Arc::new(event.clone()));
    let nested = ProcessEventCommand::new(Arc::new(IamDomainEvent::UserSignedUp(event)));
    for extracted in [&direct, &nested] {
        assert_eq!(
            extracted.recipient.email.as_deref(),
            Some("ada@example.test")
        );
        assert_eq!(extracted.recipient.user_id, Some(Uuid::from_u128(42)));
        assert_eq!(extracted.event_type, "user_signed_up");
        extracted
            .validate()
            .expect("signup command should validate");
        assert_eq!(extracted.command_type(), "process_event");
    }
    assert_eq!(direct.original_event_id, nested.original_event_id);
    assert_ne!(direct.original_event_id, Uuid::nil());
    assert_ne!(direct.command_id(), nested.command_id());
}

#[test]
fn invalid_event_identity_type_attempt_and_absent_recipient_have_distinct_codes() {
    let mut missing_id = command();
    missing_id.original_event_id = Uuid::nil();
    let mut empty_type = command();
    empty_type.event_type.clear();
    let zero_attempt = command().with_attempt(0);
    let mut no_recipient = command();
    no_recipient.recipient = SendMessageRecipient {
        user_id: None,
        email: None,
    };
    for (invalid, expected_code) in [
        (missing_id, "invalid_event_id"),
        (empty_type, "empty_event_type"),
        (zero_attempt, "invalid_attempt"),
        (no_recipient, "invalid_recipient"),
    ] {
        match invalid
            .validate()
            .expect_err("invalid command must be rejected")
        {
            CommandError::Validation { code, message } => {
                assert_eq!(code, expected_code);
                assert!(!message.is_empty(), "validation needs a diagnostic");
            }
            other => panic!("expected nonretryable validation for {expected_code}: {other:?}"),
        }
    }
}

#[test]
fn either_recipient_identifier_is_sufficient_even_at_maximum_attempt() {
    for recipient in [
        SendMessageRecipient {
            user_id: Some(Uuid::from_u128(42)),
            email: None,
        },
        SendMessageRecipient {
            user_id: None,
            email: Some("ada@example.test".into()),
        },
    ] {
        let mut cmd = command().with_attempt(u32::MAX);
        cmd.recipient = recipient;
        cmd.validate()
            .expect("either recipient identifier suffices");
        assert_eq!(cmd.attempt, u32::MAX, "attempt must not wrap or reset");
    }
}
