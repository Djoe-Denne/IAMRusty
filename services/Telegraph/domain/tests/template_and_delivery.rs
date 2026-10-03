use std::collections::HashMap;

use chrono::Utc;
use telegraph_domain::{
    CommunicationMode, DeliveryStatus, MessageDelivery, MessageTemplate, RenderedTemplate,
    TemplateContent,
};
use uuid::Uuid;

fn email_content() -> TemplateContent {
    TemplateContent::Email {
        subject: "Hello {{name}}".to_string(),
        html_body: Some("<p>{{name}}</p>".to_string()),
        text_body: "Hello {{name}}".to_string(),
    }
}

#[test]
fn template_email_round_trip_and_inactive_render() {
    let mut template = MessageTemplate::new(
        "welcome".to_string(),
        CommunicationMode::Email,
        email_content(),
    )
    .expect("email template should be valid");
    template = template
        .with_description("Welcome mail".to_string())
        .with_default_variable("name".to_string(), "Ada".to_string());

    let rendered = template
        .render(&HashMap::new())
        .expect("active template should render");
    match rendered {
        RenderedTemplate::Email {
            subject,
            html_body,
            text_body,
        } => {
            assert_eq!(subject, "Hello Ada");
            assert_eq!(html_body.as_deref(), Some("<p>Ada</p>"));
            assert_eq!(text_body, "Hello Ada");
        }
        other => panic!("unexpected render: {other:?}"),
    }

    template.deactivate();
    assert!(!template.active);
    assert!(template.render(&HashMap::new()).is_err());
    template.activate();
    assert!(template.active);
}

#[test]
fn template_rejects_mode_mismatch_and_unreplaced_placeholders() {
    let mismatch = MessageTemplate::new(
        "sms".to_string(),
        CommunicationMode::Email,
        TemplateContent::Sms {
            text: "hi".to_string(),
        },
    );
    assert!(mismatch.is_err());

    let template = MessageTemplate::new(
        "welcome".to_string(),
        CommunicationMode::Email,
        email_content(),
    )
    .unwrap();
    assert!(template.render(&HashMap::new()).is_err());
}

#[test]
fn template_notification_and_sms_render_paths() {
    let mut variables = HashMap::new();
    variables.insert("title".to_string(), "Ping".to_string());
    variables.insert("body".to_string(), "Body".to_string());

    let notification = MessageTemplate::new(
        "nudge".to_string(),
        CommunicationMode::Notification,
        TemplateContent::Notification {
            title: "{{title}}".to_string(),
            body: "{{body}}".to_string(),
            default_data: HashMap::from([("kind".to_string(), "toast".to_string())]),
        },
    )
    .unwrap();
    match notification.render(&variables).unwrap() {
        RenderedTemplate::Notification { title, body, data } => {
            assert_eq!(title, "Ping");
            assert_eq!(body, "Body");
            assert_eq!(data.get("kind").map(String::as_str), Some("toast"));
            assert_eq!(data.get("var_title").map(String::as_str), Some("Ping"));
        }
        other => panic!("unexpected render: {other:?}"),
    }

    let now = Utc::now();
    let sms = MessageTemplate {
        id: Uuid::new_v4(),
        name: "sms".to_string(),
        description: None,
        mode: CommunicationMode::Email,
        content: TemplateContent::Sms {
            text: "code {{code}}".to_string(),
        },
        default_variables: HashMap::new(),
        created_at: now,
        updated_at: now,
        active: true,
    };
    let mut sms_vars = HashMap::new();
    sms_vars.insert("code".to_string(), "1234".to_string());
    match sms.render(&sms_vars).unwrap() {
        RenderedTemplate::Sms { text } => assert_eq!(text, "code 1234"),
        other => panic!("unexpected render: {other:?}"),
    }
}

#[test]
fn delivery_status_transitions_and_attempt_marks() {
    let mut delivery = MessageDelivery::new(Uuid::new_v4(), CommunicationMode::Email);
    assert_eq!(delivery.status, DeliveryStatus::Pending);
    assert!(!delivery.can_retry(3));
    assert!(!delivery.is_final());
    assert!(!delivery.is_successful());
    assert!(delivery.delivery_duration().is_none());

    delivery.mark_processing();
    assert_eq!(delivery.status, DeliveryStatus::Processing);

    delivery.mark_sent(Some("provider-1".to_string()));
    assert_eq!(delivery.status, DeliveryStatus::Sent);
    assert_eq!(delivery.attempts, 1);

    delivery.mark_delivered();
    assert!(delivery.is_successful());
    assert!(delivery.is_final());
    assert!(delivery.delivery_duration().is_some());

    delivery.mark_failed("boom".to_string());
    assert_eq!(delivery.status, DeliveryStatus::Failed);
    assert!(delivery.can_retry(5));
    assert!(!delivery.can_retry(1));

    delivery.mark_rejected("rejected".to_string());
    assert!(delivery.is_final());
    delivery.mark_bounced("bounce".to_string());
    assert_eq!(delivery.status, DeliveryStatus::Bounced);
    delivery.mark_read();
    assert!(delivery.is_successful());
    delivery.add_metadata("channel".to_string(), "email".to_string());
    assert_eq!(delivery.metadata.get("channel").unwrap(), "email");

    let mut attempt = telegraph_domain::entity::DeliveryAttempt::new(delivery.id, 1);
    attempt.mark_success(Some("ok".to_string()), Some(12));
    attempt.mark_failed("fail".to_string(), Some(8));
    attempt.mark_rejected("no".to_string(), Some(3));

    assert_eq!(DeliveryStatus::Pending.to_string(), "pending");
    assert_eq!(DeliveryStatus::Processing.to_string(), "processing");
    assert_eq!(DeliveryStatus::Sent.to_string(), "sent");
    assert_eq!(DeliveryStatus::Delivered.to_string(), "delivered");
    assert_eq!(DeliveryStatus::Failed.to_string(), "failed");
    assert_eq!(DeliveryStatus::Rejected.to_string(), "rejected");
    assert_eq!(DeliveryStatus::Bounced.to_string(), "bounced");
    assert_eq!(DeliveryStatus::Read.to_string(), "read");
}
