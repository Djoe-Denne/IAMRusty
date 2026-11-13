//! QA coverage90 C: descriptor selection, fallback and partial-channel failures.
//! Pure factory tests with local port doubles; these are not HTTP/DB integration tests.
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use iam_events::UserSignedUpEvent;
use rustycog::events::DomainEvent;
use telegraph_domain::{
    Communication, CommunicationFactory, CommunicationMode, DomainError, EventContext,
    EventExtractor, EventRecipient, RenderedTemplate, TemplateService,
};
use uuid::Uuid;

struct DescriptorDirectory(PathBuf);

impl DescriptorDirectory {
    fn new(contents: &str) -> Self {
        let path = std::env::temp_dir().join(format!("telegraph-coverage90-{}", Uuid::new_v4()));
        std::fs::create_dir(&path).expect("create owned descriptor directory");
        std::fs::write(path.join("user_signed_up.toml"), contents).expect("write descriptor");
        Self(path)
    }
}

impl Drop for DescriptorDirectory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove owned descriptor directory");
    }
}

struct Variables {
    values: HashMap<String, String>,
    error: Option<DomainError>,
}

#[async_trait]
impl EventExtractor for Variables {
    async fn extract_variables(
        &self,
        _: &dyn DomainEvent,
    ) -> Result<HashMap<String, String>, DomainError> {
        self.error
            .as_ref()
            .map_or_else(|| Ok(self.values.clone()), |error| Err(error.clone()))
    }
}

struct Templates;

#[async_trait]
impl TemplateService for Templates {
    async fn find_template(&self, _: &str, _: &CommunicationMode) -> Result<String, DomainError> {
        unreachable!("factory selects the descriptor's template")
    }

    async fn template_exists(&self, _: &str, _: &CommunicationMode) -> Result<bool, DomainError> {
        unreachable!("not part of factory rendering")
    }

    async fn render_template(
        &self,
        name: &str,
        mode: &CommunicationMode,
        variables: &HashMap<String, String>,
    ) -> Result<RenderedTemplate, DomainError> {
        match name {
            "broken" => Err(DomainError::TemplateRenderError("missing variable".into())),
            "wrong-mode" => Ok(RenderedTemplate::Sms {
                text: "not email or notification".into(),
            }),
            "welcome" => {
                assert_eq!(*mode, CommunicationMode::Email);
                Ok(RenderedTemplate::Email {
                    subject: variables
                        .get("subject")
                        .cloned()
                        .unwrap_or_else(|| "Rendered subject".into()),
                    html_body: Some("<p>Welcome Ada</p>".into()),
                    text_body: "Welcome Ada".into(),
                })
            }
            "toast" => {
                assert_eq!(*mode, CommunicationMode::Notification);
                Ok(RenderedTemplate::Notification {
                    title: "Rendered title".into(),
                    body: "Welcome Ada".into(),
                    data: HashMap::from([("kind".into(), "signup".into())]),
                })
            }
            other => panic!("unexpected descriptor template: {other}"),
        }
    }
}

fn context() -> EventContext {
    EventContext {
        event_id: Uuid::from_u128(1),
        event_type: "user_signed_up".into(),
        recipient: EventRecipient {
            user_id: Some(Uuid::from_u128(42)),
            email: Some("ada@example.test".into()),
        },
        event: Arc::new(UserSignedUpEvent::new(
            Uuid::from_u128(42),
            "ada@example.test".into(),
            "ada".into(),
            true,
            None,
            None,
        )),
        attempt: 1,
        metadata: HashMap::new(),
    }
}

fn factory(
    directory: &DescriptorDirectory,
    values: HashMap<String, String>,
    error: Option<DomainError>,
) -> CommunicationFactory {
    CommunicationFactory::new(
        Arc::new(Templates),
        Arc::new(Variables { values, error }),
        directory.0.clone(),
    )
}

const BOTH: &str = r#"
[email]
subject = "Hello {name} / {name}"
template = "welcome"
[notification]
title = "Signed up: {name}"
template = "toast"
"#;

#[tokio::test]
async fn both_channels_interpolate_descriptor_headers_and_preserve_rendered_bodies() {
    let directory = DescriptorDirectory::new(BOTH);
    let factory = factory(
        &directory,
        HashMap::from([("name".into(), "Ada".into())]),
        None,
    );
    let communications = factory
        .build_communication(&context())
        .await
        .expect("both channels");
    assert_eq!(communications.len(), 2);
    let Communication::Email(email) = &communications[0] else {
        panic!("email expected")
    };
    assert_eq!(email.subject, "Hello Ada / Ada");
    assert_eq!(email.text_body, "Welcome Ada");
    assert_eq!(email.html_body.as_deref(), Some("<p>Welcome Ada</p>"));
    assert_eq!(email.recipient.email.as_deref(), Some("ada@example.test"));
    let Communication::Notification(notification) = &communications[1] else {
        panic!("notification expected")
    };
    assert_eq!(notification.title, "Signed up: Ada");
    assert_eq!(notification.body, "Welcome Ada");
    assert_eq!(
        notification.data.get("kind").map(String::as_str),
        Some("signup")
    );
    assert_eq!(notification.recipient.user_id, Some(Uuid::from_u128(42)));
}

#[tokio::test]
async fn empty_descriptor_headers_use_rendered_headers_instead_of_erasing_them() {
    let directory = DescriptorDirectory::new(
        &BOTH
            .replace("Hello {name} / {name}", "")
            .replace("Signed up: {name}", ""),
    );
    let factory = factory(
        &directory,
        HashMap::from([("subject".into(), "Event subject".into())]),
        None,
    );
    let email = factory
        .build_email_communication(&context())
        .await
        .expect("email fallback");
    assert_eq!(email.subject, "Event subject");
    let notification = factory
        .build_notification_communication(&context())
        .await
        .expect("notification fallback");
    assert_eq!(notification.title, "Rendered title");
}

#[tokio::test]
async fn failure_in_one_channel_does_not_discard_the_other_valid_communication() {
    for (broken_template, expected_email) in [("welcome", false), ("toast", true)] {
        let directory = DescriptorDirectory::new(&BOTH.replace(broken_template, "broken"));
        let factory = factory(
            &directory,
            HashMap::from([("name".into(), "Ada".into())]),
            None,
        );
        let communications = factory
            .build_communication(&context())
            .await
            .expect("remaining valid channel");
        assert_eq!(communications.len(), 1);
        assert_eq!(
            matches!(&communications[0], Communication::Email(_)),
            expected_email
        );
    }
}

#[tokio::test]
async fn direct_builds_propagate_extraction_failure_and_aggregate_refuses_no_valid_channel() {
    let directory = DescriptorDirectory::new(BOTH);
    let error = DomainError::InvalidRecipient("missing signup address".into());
    let factory = factory(&directory, HashMap::new(), Some(error.clone()));
    assert_eq!(
        factory.build_email_communication(&context()).await,
        Err(error.clone())
    );
    assert_eq!(
        factory.build_notification_communication(&context()).await,
        Err(error)
    );
    assert_eq!(
        factory.build_communication(&context()).await,
        Err(DomainError::EventProcessingError(
            "No valid communication configurations found for event type 'user_signed_up'".into()
        ))
    );
}

#[tokio::test]
async fn mismatched_rendered_variant_is_not_accepted_as_a_valid_channel() {
    let directory = DescriptorDirectory::new(
        &BOTH
            .replace("welcome", "wrong-mode")
            .replace("toast", "wrong-mode"),
    );
    let factory = factory(&directory, HashMap::new(), None);
    assert_eq!(
        factory.build_email_communication(&context()).await,
        Err(DomainError::EventProcessingError(
            "Template did not render to email content".into()
        ))
    );
    assert_eq!(
        factory.build_notification_communication(&context()).await,
        Err(DomainError::EventProcessingError(
            "Template did not render to notification content".into()
        ))
    );
}

#[tokio::test]
async fn missing_channel_configuration_is_distinguished_from_missing_descriptor() {
    let directory = DescriptorDirectory::new("");
    let factory = factory(&directory, HashMap::new(), None);
    assert_eq!(
        factory.build_email_communication(&context()).await,
        Err(DomainError::EventProcessingError(
            "No email configuration found for event type 'user_signed_up'".into()
        ))
    );
    assert_eq!(
        factory.build_notification_communication(&context()).await,
        Err(DomainError::EventProcessingError(
            "No notification configuration found for event type 'user_signed_up'".into()
        ))
    );
    let mut missing = context();
    missing.event_type = "missing-event".into();
    let error = factory
        .build_communication(&missing)
        .await
        .expect_err("missing descriptor");
    assert!(
        matches!(error, DomainError::EventProcessingError(ref message) if message.contains("descriptor not found") && message.contains("missing-event")),
        "{error}"
    );
}

#[tokio::test]
async fn malformed_descriptor_reports_parse_error_before_rendering() {
    let directory = DescriptorDirectory::new("[email\nsubject =");
    let factory = factory(&directory, HashMap::new(), None);
    let error = factory
        .build_communication(&context())
        .await
        .expect_err("invalid TOML");
    assert!(
        matches!(error, DomainError::EventProcessingError(ref message) if message.contains("Failed to parse descriptor file") && message.contains("user_signed_up.toml")),
        "{error}"
    );
}

#[tokio::test]
async fn unreadable_descriptor_is_reported_as_read_failure_not_missing_configuration() {
    let directory = DescriptorDirectory::new(BOTH);
    let descriptor = directory.0.join("user_signed_up.toml");
    std::fs::remove_file(&descriptor).expect("replace owned file with directory");
    std::fs::create_dir(&descriptor).expect("unreadable descriptor path");
    let factory = factory(&directory, HashMap::new(), None);
    let error = factory
        .build_communication(&context())
        .await
        .expect_err("directory is not TOML text");
    assert!(
        matches!(error, DomainError::EventProcessingError(ref message) if message.contains("Failed to read descriptor file") && message.contains("user_signed_up.toml")),
        "{error}"
    );
}
