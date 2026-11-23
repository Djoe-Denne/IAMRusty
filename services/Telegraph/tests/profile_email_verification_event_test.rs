//! Exercise only the dedicated event-to-SMTP boundary; do not open the link.

mod common;

use std::time::Duration;

use common::fixtures::sqs_scenario::SqsEventScenario;
use iam_events::ProfileEmailVerificationRequested;
use serial_test::serial;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn profile_verification_intent_sends_to_the_exact_address() {
    let scenario = SqsEventScenario::start()
        .await
        .expect("start isolated Telegraph event fixture");
    let recipient = "profile.proof@example.com";
    let link = format!("https://verify.example/v#{}", Uuid::new_v4().simple());
    let result = tokio::time::timeout(Duration::from_secs(25), async {
        let event = ProfileEmailVerificationRequested::new(
            Uuid::new_v4(),
            recipient.to_owned(),
            link.clone(),
        );
        if scenario.fixture.sqs().send_event(&event).await.is_err() {
            return false;
        }
        loop {
            let Ok(messages) = scenario.fixture.smtp().get_emails().await else {
                return false;
            };
            if !messages.is_empty() {
                return messages.len() == 1
                    && messages[0].subject == "Verify a saved AIForAll email address"
                    && messages[0].to.len() == 1
                    && messages[0].to[0].address == recipient
                    && messages[0].text.contains(&link);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await;
    scenario
        .stop()
        .await
        .expect("join owned Telegraph background tasks");
    assert!(
        matches!(result, Ok(true)),
        "dedicated intent must deliver the configured link to the exact recipient"
    );
}
