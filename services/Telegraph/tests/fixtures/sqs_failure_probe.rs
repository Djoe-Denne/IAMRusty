//! Failure-only, passive broker observations. Counts are approximate and cannot
//! identify which message was consumed or prove a particular consumer took it.

use std::time::Duration;

use aws_sdk_sqs::{types::QueueAttributeName, Client};
use serde_json::{json, Value};
use uuid::Uuid;

const ATTRIBUTES: [QueueAttributeName; 5] = [
    QueueAttributeName::ApproximateNumberOfMessages,
    QueueAttributeName::ApproximateNumberOfMessagesNotVisible,
    QueueAttributeName::ApproximateNumberOfMessagesDelayed,
    QueueAttributeName::VisibilityTimeout,
    QueueAttributeName::DelaySeconds,
];

pub enum Stage {
    HandlerEntry,
    Notification,
}

/// The one-second diagnostic budget is separate from the already failed phase.
/// No receive, delete, purge, retry loop or assertion is performed here.
pub async fn failure_snapshot(
    client: &Client,
    queue_url: &str,
    stage: Stage,
    event_id: Uuid,
    message_id: Option<&str>,
    phase_elapsed: Duration,
) -> Value {
    let started = tokio::time::Instant::now();
    let broker = capture(client, queue_url, Duration::from_secs(1)).await;
    // Never echo an arbitrary broker-supplied string as an identity.
    let message_uuid = message_id.and_then(|id| Uuid::parse_str(id).ok());
    json!({
        "kind": "telegraph_sqs_failure",
        "stage": match stage {
            Stage::HandlerEntry => "handler_entry",
            Stage::Notification => "notification",
        },
        "event_id": event_id,
        "message_id": message_uuid,
        "message_id_status": match (message_id, message_uuid) {
            (_, Some(_)) => "uuid",
            (Some(_), None) => "unsupported",
            (None, None) => "missing",
        },
        "phase_elapsed_ms": phase_elapsed.as_millis(),
        "probe_elapsed_ms": started.elapsed().as_millis(),
        "broker": broker,
    })
}

async fn capture(client: &Client, queue_url: &str, budget: Duration) -> Value {
    let response = tokio::time::timeout(
        budget,
        client
            .get_queue_attributes()
            .queue_url(queue_url)
            .set_attribute_names(Some(ATTRIBUTES.to_vec()))
            .send(),
    )
    .await;
    match response {
        Ok(Ok(response)) => {
            let counts: Vec<Option<u64>> = ATTRIBUTES
                .iter()
                .map(|attribute| {
                    let raw = response.attributes()?.get(attribute)?;
                    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
                        return None;
                    }
                    raw.parse().ok()
                })
                .collect();
            json!({
                "observation": if counts.iter().all(Option::is_some) { "observed" } else { "partial" },
                "visible": counts[0],
                "inflight": counts[1],
                "delayed": counts[2],
                "visibility_timeout_seconds": counts[3],
                "delay_seconds": counts[4],
            })
        }
        Ok(Err(error)) => json!({
            "observation": "request_failed",
            "http_status": error.raw_response().map(|response| response.status().as_u16()),
        }),
        Err(_) => json!({"observation": "timeout"}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_sqs::config::retry::RetryConfig;
    use wiremock::{matchers::any, Mock, MockServer, ResponseTemplate};

    fn client(server: &MockServer) -> Client {
        Client::from_conf(
            aws_sdk_sqs::Config::builder()
                .behavior_version(aws_config::BehaviorVersion::latest())
                .region(aws_config::Region::new("us-east-1"))
                .credentials_provider(aws_credential_types::Credentials::new(
                    "test",
                    "test",
                    None,
                    None,
                    "passive-probe-unit",
                ))
                .endpoint_url(server.uri())
                .retry_config(RetryConfig::disabled())
                .build(),
        )
    }

    async fn assert_passive_request(server: &MockServer) {
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1, "one read, no mutation or polling loop");
        let request = &requests[0];
        assert_eq!(
            request.headers.get("x-amz-target").unwrap(),
            "AmazonSQS.GetQueueAttributes"
        );
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        let mut names: Vec<_> = body["AttributeNames"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| name.as_str().unwrap())
            .collect();
        names.sort_unstable();
        let mut expected: Vec<_> = ATTRIBUTES.iter().map(QueueAttributeName::as_str).collect();
        expected.sort_unstable();
        assert_eq!(
            names, expected,
            "no All, Policy or other private attributes"
        );
    }

    #[tokio::test]
    async fn probe_reports_only_numeric_attributes_and_safe_identities() {
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "Attributes": {
                    "ApproximateNumberOfMessages": "2",
                    "ApproximateNumberOfMessagesNotVisible": "1",
                    "ApproximateNumberOfMessagesDelayed": "0",
                    "VisibilityTimeout": "30",
                    "DelaySeconds": "0",
                    "Policy": "private-policy-sentinel",
                }
            })))
            .mount(&server)
            .await;
        let event = Uuid::new_v4();
        let snapshot = failure_snapshot(
            &client(&server),
            &server.uri(),
            Stage::Notification,
            event,
            Some("private-message-sentinel"),
            Duration::from_secs(5),
        )
        .await;
        assert_eq!(
            snapshot["broker"],
            json!({
                "observation": "observed", "visible": 2, "inflight": 1, "delayed": 0,
                "visibility_timeout_seconds": 30, "delay_seconds": 0,
            })
        );
        assert_eq!(snapshot["event_id"], json!(event));
        assert_eq!(snapshot["message_id"], Value::Null);
        assert_eq!(snapshot["message_id_status"], "unsupported");
        assert_eq!(snapshot["phase_elapsed_ms"], 5000);
        let text = snapshot.to_string();
        for forbidden in [
            "private-policy-sentinel",
            "private-message-sentinel",
            &server.uri(),
        ] {
            assert!(!text.contains(forbidden));
        }
        assert_passive_request(&server).await;
    }

    #[tokio::test]
    async fn probe_missing_and_invalid_counts_are_unknown_not_zero() {
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "Attributes": {
                    "ApproximateNumberOfMessages": "private-count-sentinel",
                    "ApproximateNumberOfMessagesNotVisible": "-1",
                    "ApproximateNumberOfMessagesDelayed": "18446744073709551616",
                }
            })))
            .mount(&server)
            .await;
        let snapshot = capture(&client(&server), &server.uri(), Duration::from_secs(1)).await;
        assert_eq!(snapshot["observation"], "partial");
        for key in [
            "visible",
            "inflight",
            "delayed",
            "visibility_timeout_seconds",
            "delay_seconds",
        ] {
            assert_eq!(snapshot[key], Value::Null);
        }
        assert!(!snapshot.to_string().contains("private-count-sentinel"));
        assert_passive_request(&server).await;
    }

    #[tokio::test]
    async fn probe_http_failure_is_not_an_empty_queue_or_a_provider_message() {
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(ResponseTemplate::new(403).set_body_json(json!({
                "__type": "AccessDenied", "message": "private-provider-sentinel",
            })))
            .mount(&server)
            .await;
        let snapshot = capture(&client(&server), &server.uri(), Duration::from_secs(1)).await;
        assert_eq!(
            snapshot,
            json!({"observation": "request_failed", "http_status": 403})
        );
        assert_passive_request(&server).await;
    }

    #[tokio::test]
    async fn probe_stalled_request_has_its_own_deadline() {
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({}))
                    .set_delay(Duration::from_secs(5)),
            )
            .mount(&server)
            .await;
        let snapshot = tokio::time::timeout(
            Duration::from_secs(2),
            capture(&client(&server), &server.uri(), Duration::from_millis(100)),
        )
        .await
        .expect("the probe must enforce its own shorter deadline");
        assert_eq!(snapshot, json!({"observation": "timeout"}));
        assert_passive_request(&server).await;
    }
}
