//! AC2/3/5/6: actual transport bytes -> public Sentinel handler -> real
//! OpenFGA -> HTTP authorization, nominally and after a real publisher outage.
mod common;
#[path = "fixtures/outbox_runtime.rs"]
mod runtime_fixture;

use aws_sdk_sqs::types::QueueAttributeName;
use std::sync::Arc;
use std::time::Duration;

use common::{TelegraphTestDescriptor, TelegraphTestFixture, TestOpenFga};
use iam_events::{IamDomainEvent, UserEmailVerifiedEvent};
use runtime_fixture::*;
use rustycog::events::{event::BaseEvent, DomainEvent, EventHandler};
use rustycog::outbox::entity::{STATUS_FAILED, STATUS_PUBLISHED};
use rustycog::testing::http::jwt::create_jwt_token;
use rustycog::testing::wiremock::MockServerFixture;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use sentinel_sync::fga_client::Tuple;
use sentinel_sync::{
    fga_client::OpenFgaWriteClient, handler::SyncEventHandler, idempotency::InMemoryEventLedger,
    translator::telegraph::TelegraphTranslator,
};
use serial_test::serial;
use telegraph_events::TelegraphDomainEvent;
use telegraph_infra::repository::entity::{notification_deliveries, notifications};
use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

const OUTBOUND: &str = "test-telegraph-outbound";

fn duplicate_message(tuple: &Tuple) -> String {
    format!(
        "cannot write a tuple which already exists: user: '{}:{}', relation: '{}', object: '{}:{}': invalid write input",
        tuple.user_type, tuple.user_id, tuple.relation, tuple.object_type, tuple.object_id,
    )
}

async fn classifier_client(
    status: u16,
    body: serde_json::Value,
) -> (MockServerFixture, OpenFgaWriteClient) {
    // Existing isolated HTTP fixture, only the outbound Write collaborator.
    // The vertical test below still uses REAL OpenFGA and no preloaded grants.
    let fixture = MockServerFixture::isolated().await;
    Mock::given(method("POST"))
        .and(path("/stores/classifier-test/write"))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .mount(fixture.server().as_ref())
        .await;
    let endpoint = url::Url::parse(&fixture.base_url()).unwrap();
    let config = sentinel_sync::config::OpenFgaConfig {
        scheme: "http".into(),
        host: "127.0.0.1".into(),
        port: endpoint.port().unwrap(),
        store_id: "classifier-test".into(),
        ..Default::default()
    };
    let client = OpenFgaWriteClient::new(config).unwrap();
    (fixture, client)
}

#[tokio::test]
#[serial]
async fn sentinel_classifier_real_fga15_duplicate_is_idempotent_in_write_context() {
    let tuple = Tuple::user("notification", Uuid::new_v4(), "recipient", Uuid::new_v4());
    let (fixture, client) = classifier_client(
        400,
        serde_json::json!({
            "code": "write_failed_due_to_invalid_input", "message": duplicate_message(&tuple),
        }),
    )
    .await;
    client
        .write_idempotent(&[tuple], &[])
        .await
        .expect("real FGA 1.5 duplicate must converge");
    assert_eq!(
        fixture.server().received_requests().await.unwrap().len(),
        2,
        "batch failure is rechecked per tuple"
    );
}

#[tokio::test]
#[serial]
async fn sentinel_classifier_authn_authz_unavailable_errors_remain_observable() {
    for status in [401, 403, 404, 408, 409, 429, 500, 503] {
        let tuple = Tuple::user("notification", Uuid::new_v4(), "recipient", Uuid::new_v4());
        let (fixture, client) = classifier_client(status, serde_json::json!({
            "code": "cannot_write_tuple_which_already_exists", "message": "cannot_write_tuple_which_already_exists",
        })).await;
        let error = client
            .write_idempotent(&[tuple], &[])
            .await
            .expect_err("non-400 status cannot become idempotent success");
        assert!(error
            .to_string()
            .contains(&format!("OpenFGA write returned {status}")));
        assert_eq!(
            fixture.server().received_requests().await.unwrap().len(),
            1,
            "non-idempotent failures must not trigger per-tuple retries"
        );
    }
}

#[tokio::test]
#[serial]
async fn sentinel_classifier_wrong_operation_and_ambiguous_400_are_errors() {
    let tuple = Tuple::user("notification", Uuid::new_v4(), "recipient", Uuid::new_v4());
    for body in [
        serde_json::json!({"code":"validation_error", "message":"cannot_write_tuple_which_already_exists"}),
        serde_json::json!({"code":"write_failed_due_to_invalid_input", "message":"other invalid input; cannot_write_tuple_which_already_exists"}),
        serde_json::json!({"code":"write_failed_due_to_invalid_input", "message":"cannot_delete_unknown_tuple"}),
        serde_json::json!({"code":"write_failed_due_to_invalid_input", "message":duplicate_message(&tuple), "details":["another invalid tuple"]}),
        serde_json::json!({"code":"write_failed_due_to_invalid_input"}),
        serde_json::json!({"code":"write_failed_due_to_invalid_input", "message": duplicate_message(&Tuple::user("notification", Uuid::new_v4(), "recipient", Uuid::new_v4()))}),
    ] {
        let (fixture, client) = classifier_client(400, body).await;
        let error = client
            .write_idempotent(std::slice::from_ref(&tuple), &[])
            .await
            .expect_err("ambiguous or wrong-effect 400 must propagate");
        assert!(error.to_string().contains("OpenFGA write returned 400"));
        assert_eq!(fixture.server().received_requests().await.unwrap().len(), 1);
    }
    let (fixture, client) = classifier_client(400, serde_json::json!({
        "code":"cannot_write_tuple_which_already_exists", "message":"cannot_write_tuple_which_already_exists",
    })).await;
    let error = client
        .write_idempotent(&[], &[tuple])
        .await
        .expect_err("write duplicate cannot establish successful deletion");
    assert!(error.to_string().contains("OpenFGA write returned 400"));
    assert_eq!(fixture.server().received_requests().await.unwrap().len(), 1);
    for raw in [
        "{\"code\":\"write_failed_due_to_invalid_input\",\"message\":\"cannot_write_tuple_which_already_exists\"",
        "{\"code\":\"validation_error\",\"code\":\"write_failed_due_to_invalid_input\",\"message\":\"cannot_write_tuple_which_already_exists\"}",
        "unrelated error: cannot_write_tuple_which_already_exists",
    ] {
        let (fixture, client) = classifier_client(400, serde_json::json!({})).await;
        fixture.reset().await;
        Mock::given(method("POST")).and(path("/stores/classifier-test/write"))
            .respond_with(ResponseTemplate::new(400).set_body_string(raw))
            .mount(fixture.server().as_ref()).await;
        let requested = Tuple::user("notification", Uuid::new_v4(), "recipient", Uuid::new_v4());
        let error = client.write_idempotent(&[requested], &[]).await.expect_err("malformed, duplicate-field and arbitrary marker text must remain errors");
        assert!(error.to_string().contains(raw));
        assert_eq!(fixture.server().received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
#[serial]
async fn sentinel_classifier_historical_markers_keep_their_operation_context() {
    for (marker, deletion) in [
        ("cannot_write_tuple_which_already_exists", false),
        ("cannot_delete_tuple_which_does_not_exist", true),
        ("cannot_delete_unknown_tuple", true),
    ] {
        for code in ["write_failed_due_to_invalid_input", marker] {
            let tuple = Tuple::user("notification", Uuid::new_v4(), "recipient", Uuid::new_v4());
            let (fixture, client) =
                classifier_client(400, serde_json::json!({"code":code, "message":marker})).await;
            let (writes, deletes) = if deletion {
                (vec![], vec![tuple])
            } else {
                (vec![tuple], vec![])
            };
            client
                .write_idempotent(&writes, &deletes)
                .await
                .expect("historical effect is still idempotent in its proper operation");
            assert_eq!(fixture.server().received_requests().await.unwrap().len(), 2);
        }
        let tuple = Tuple::user("notification", Uuid::new_v4(), "recipient", Uuid::new_v4());
        let (fixture, client) = classifier_client(400, serde_json::json!({})).await;
        fixture.reset().await;
        Mock::given(method("POST"))
            .and(path("/stores/classifier-test/write"))
            .respond_with(ResponseTemplate::new(400).set_body_string(marker))
            .mount(fixture.server().as_ref())
            .await;
        let (writes, deletes) = if deletion {
            (vec![], vec![tuple])
        } else {
            (vec![tuple], vec![])
        };
        client
            .write_idempotent(&writes, &deletes)
            .await
            .expect("exact historical plain marker retains its contextual effect");
        assert_eq!(fixture.server().received_requests().await.unwrap().len(), 2);
    }
}

#[tokio::test]
#[serial]
async fn sentinel_classifier_batch_duplicate_never_hides_another_invalid_tuple() {
    let existing = Tuple::user("notification", Uuid::new_v4(), "recipient", Uuid::new_v4());
    let invalid = Tuple::user("notification", Uuid::new_v4(), "recipient", Uuid::new_v4());
    let (fixture, client) = classifier_client(
        400,
        serde_json::json!({
            "code":"write_failed_due_to_invalid_input", "message":duplicate_message(&existing),
        }),
    )
    .await;
    fixture.reset().await;
    Mock::given(method("POST"))
        .and(path("/stores/classifier-test/write"))
        .and(wiremock::matchers::body_json(serde_json::json!({"writes":{"tuple_keys":[{
            "user":format!("{}:{}", invalid.user_type, invalid.user_id), "relation":invalid.relation,
            "object":format!("{}:{}", invalid.object_type, invalid.object_id),
        }]}})))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({"code":"validation_error", "message":"other tuple is invalid"})))
        .mount(fixture.server().as_ref()).await;
    Mock::given(method("POST"))
        .and(path("/stores/classifier-test/write"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "code":"write_failed_due_to_invalid_input", "message":duplicate_message(&existing),
        })))
        .mount(fixture.server().as_ref())
        .await;
    let error = client
        .write_idempotent(&[existing, invalid], &[])
        .await
        .expect_err("per-tuple validation error must remain visible");
    assert!(error.to_string().contains("other tuple is invalid"));
    assert!(fixture.server().received_requests().await.unwrap().len() >= 2);
}

fn handler(fga: &TestOpenFga) -> SyncEventHandler {
    SyncEventHandler::new(
        vec![Arc::new(TelegraphTranslator::new())],
        Arc::new(InMemoryEventLedger::new()),
        OpenFgaWriteClient::new(fga.client_config()).expect("real OpenFGA write client"),
    )
}

async fn stored_notification(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
    deadline: tokio::time::Instant,
) -> notifications::Model {
    let mut completed_queries = 0;
    tokio::time::timeout_at(deadline, async {
        loop {
            let rows = notifications::Entity::find()
                .filter(notifications::Column::UserId.eq(user_id))
                .all(db)
                .await
                .unwrap();
            completed_queries += 1;
            if let Some(row) = rows.into_iter().next() {
                break row;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap_or_else(|error| panic!("IAM event committed exactly one notification: {error:?}; user_id={user_id}; successful SQL polls={completed_queries}"))
}

async fn inbound_queue_snapshot(
    sqs: &rustycog::testing::sqs_testcontainer::TestSqs,
    stage: &str,
    started: tokio::time::Instant,
    deadline: tokio::time::Instant,
) -> std::collections::HashMap<QueueAttributeName, String> {
    let response = tokio::time::timeout_at(
        deadline.min(tokio::time::Instant::now() + Duration::from_secs(1)),
        sqs.client
            .get_queue_attributes()
            .queue_url(&sqs.queue_url)
            .attribute_names(QueueAttributeName::VisibilityTimeout)
            .attribute_names(QueueAttributeName::DelaySeconds)
            .attribute_names(QueueAttributeName::ApproximateNumberOfMessages)
            .attribute_names(QueueAttributeName::ApproximateNumberOfMessagesNotVisible)
            .attribute_names(QueueAttributeName::ApproximateNumberOfMessagesDelayed)
            .send(),
    )
    .await
    .unwrap_or_else(|_| panic!("bounded passive queue snapshot timed out: {stage}"))
    .unwrap_or_else(|error| panic!("passive queue snapshot failed ({stage}): {error}"));
    let attributes = response
        .attributes()
        .expect("requested queue attributes")
        .clone();
    for required in [
        QueueAttributeName::VisibilityTimeout,
        QueueAttributeName::DelaySeconds,
        QueueAttributeName::ApproximateNumberOfMessages,
        QueueAttributeName::ApproximateNumberOfMessagesNotVisible,
        QueueAttributeName::ApproximateNumberOfMessagesDelayed,
    ] {
        assert!(
            attributes.contains_key(&required),
            "required passive broker attribute missing: {required}"
        );
    }
    {
        use std::io::Write;
        let mut output = std::io::stdout().lock();
        writeln!(output, "TELEGRAPH_INBOUND_SNAPSHOT {}", serde_json::json!({
            "stage": stage, "elapsed_ms": started.elapsed().as_millis(),
            "VisibilityTimeout": attributes.get(&QueueAttributeName::VisibilityTimeout),
            "DelaySeconds": attributes.get(&QueueAttributeName::DelaySeconds),
            "ApproximateNumberOfMessages": attributes.get(&QueueAttributeName::ApproximateNumberOfMessages),
            "ApproximateNumberOfMessagesNotVisible": attributes.get(&QueueAttributeName::ApproximateNumberOfMessagesNotVisible),
            "ApproximateNumberOfMessagesDelayed": attributes.get(&QueueAttributeName::ApproximateNumberOfMessagesDelayed),
        })).expect("persist bounded passive broker observation");
        output.flush().expect("flush passive broker observation");
    }
    attributes
}

async fn inbound_snapshot_schedule(
    sqs: &rustycog::testing::sqs_testcontainer::TestSqs,
    enabled: bool,
    started: tokio::time::Instant,
    deadline: tokio::time::Instant,
) {
    if !enabled {
        return;
    }
    for offset in [0, 1, 3, 6, 12] {
        tokio::time::sleep_until(started + Duration::from_secs(offset)).await;
        inbound_queue_snapshot(sqs, &format!("second_iam_{offset}s"), started, deadline).await;
    }
}

async fn assert_business_counts(db: &sea_orm::DatabaseConnection, user_id: Uuid, id: Uuid) {
    assert_eq!(
        notifications::Entity::find()
            .filter(notifications::Column::UserId.eq(user_id))
            .count(db)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        notification_deliveries::Entity::find()
            .filter(notification_deliveries::Column::NotificationId.eq(id))
            .count(db)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        rustycog::outbox::entity::OutboxEvents::find()
            .filter(rustycog::outbox::entity::Column::AggregateId.eq(id))
            .count(db)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
#[serial]
async fn user_email_verified_routes_same_event_through_sentinel_and_real_authorization_after_publisher_recovery(
) {
    let fixture = TelegraphTestFixture::new(Arc::new(TelegraphTestDescriptor))
        .await
        .unwrap();
    let db = fixture.db();
    let mut real = config();
    // Reuse the existing logger, once in this binary, with no external exporter.
    // INFO exposes consumer/SQL failures without dumping credentials/JWTs.
    real.logging.level = "info".into();
    real.logging.filter = Some("info".into());
    real.logging.scaleway_loki = None;
    telegraph_configuration::setup_logging(&real);
    assert_eq!(real.openfga.cache_ttl_seconds, Some(0));
    let rustycog::config::QueueConfig::Sqs(sqs_config) = &real.queue else {
        panic!("real SQS config");
    };
    assert_eq!(
        sqs_config.get_queue_names("notification_created"),
        vec![OUTBOUND]
    );
    assert!(!sqs_config
        .default_queues
        .iter()
        .any(|name| name == OUTBOUND));
    let outbound_url = fixture.sqs().queue_url_for(OUTBOUND).unwrap().to_string();

    // Third, explicitly authorized experiment: change ONLY visibility timing,
    // before the first app, and preserve the original stop/restart chronology.
    let experiment_started = tokio::time::Instant::now();
    let initial = inbound_queue_snapshot(
        fixture.sqs(),
        "initial_before_first_app",
        experiment_started,
        experiment_started + Duration::from_secs(2),
    )
    .await;
    let initial_visibility: u64 = initial
        .get(&QueueAttributeName::VisibilityTimeout)
        .expect("initial visibility observed")
        .parse()
        .expect("numeric initial visibility");
    assert_ne!(
        initial_visibility, 5,
        "non-distinct visibility intervention: STOP/inconclusive"
    );
    tokio::time::timeout(
        Duration::from_secs(2),
        fixture
            .sqs()
            .client
            .set_queue_attributes()
            .queue_url(&fixture.sqs().queue_url)
            .attributes(QueueAttributeName::VisibilityTimeout, "5")
            .send(),
    )
    .await
    .expect("bounded test visibility configuration")
    .unwrap_or_else(|error| panic!("test visibility configuration failed: {error}"));
    let configured = inbound_queue_snapshot(
        fixture.sqs(),
        "configured_before_first_app",
        experiment_started,
        tokio::time::Instant::now() + Duration::from_secs(2),
    )
    .await;
    assert_eq!(
        configured
            .get(&QueueAttributeName::VisibilityTimeout)
            .map(String::as_str),
        Some("5")
    );

    for outage in [false, true] {
        if outage {
            // Only the outbound destination is unavailable; never stop the
            // LocalStack instance which also carries IAM ingress.
            fixture
                .sqs()
                .client
                .delete_queue()
                .queue_url(&outbound_url)
                .send()
                .await
                .unwrap();
        }
        let mut runtime = RunningApp::start(real.clone()).await;
        assert_eq!(
            runtime.tasks.len(),
            2,
            "both transports remain Live even when the queue is absent"
        );
        runtime.assert_http_healthy().await;
        let user_id = Uuid::new_v4();
        let other = Uuid::new_v4();
        let event = IamDomainEvent::UserEmailVerified(UserEmailVerifiedEvent {
            base: BaseEvent::new("user_email_verified".into(), user_id),
            user_id,
            email: format!("outbox-{user_id}@example.test"),
        });
        let message_id = fixture.sqs().send_event(&event).await.unwrap();
        tracing::info!(publisher_outage = outage, iam_event_id = %event.event_id(), %message_id, %user_id, queue_url = %fixture.sqs().queue_url, "Waiting for inbound IAM commit");
        let wait_started = tokio::time::Instant::now();
        let deadline = wait_started + Duration::from_secs(25);
        let created = tokio::select! {
            created = async {
                let sql = async {
                    let created = stored_notification(db.as_ref(), user_id, deadline).await;
                    tracing::info!(publisher_outage = outage, %user_id, notification_id = %created.id, elapsed_ms = wait_started.elapsed().as_millis(), "Expected IAM notification committed");
                    created
                };
                // Both futures are owned by this select; no detached sampler.
                // Passive observations finish inside the SAME absolute 25s
                // deadline; SQL completion time is logged when it actually occurs.
                let (created, ()) = tokio::join!(sql, inbound_snapshot_schedule(fixture.sqs(), outage, wait_started, deadline));
                created
            } => created,
            result = telegraph_setup::runtime::wait_for_background_failure(&mut runtime.tasks) => {
                panic!("Owned runtime exited before IAM commit (publisher_outage={outage}): {result:?}");
            }
        };
        let durable = row(db.as_ref(), created.id).await;
        assert_business_counts(db.as_ref(), user_id, created.id).await;
        let object = format!("notification:{}", created.id);
        assert!(
            fixture
                .openfga()
                .read_tuples(None, None, Some(&object))
                .await
                .unwrap()
                .is_empty(),
            "no grant preloaded for new UUID"
        );
        if outage {
            let failed = wait_status(db.as_ref(), created.id, STATUS_FAILED).await;
            assert!(failed.attempts > 0 && failed.attempts < 20);
            assert!(failed
                .last_error
                .as_deref()
                .is_some_and(|error| !error.is_empty()));
            assert!(failed.next_attempt_at > chrono::Utc::now());
            assert_eq!(failed.event_id, durable.event_id);
            assert_eq!(failed.payload_json, durable.payload_json);
            // Destroy the first instance and its state; reconstruct on the SAME
            // writer only after restoring that one real destination.
            runtime.stop().await;
            let queue = fixture
                .sqs()
                .client
                .create_queue()
                .queue_name(OUTBOUND)
                .send()
                .await
                .unwrap();
            assert!(queue.queue_url().is_some());
            runtime = RunningApp::start(real.clone()).await;
        }
        let published = wait_status(db.as_ref(), created.id, STATUS_PUBLISHED).await;
        assert_eq!(published.event_id, durable.event_id);
        assert_eq!(published.payload_json, durable.payload_json);
        let messages = fixture
            .sqs()
            .receive_messages_from_queue(OUTBOUND, 10, 2)
            .await
            .unwrap();
        assert_eq!(messages.len(), 1);
        let bytes = messages[0].body().expect("real SQS body");
        let envelope: serde_json::Value = serde_json::from_str(bytes).unwrap();
        assert_eq!(envelope["event_type"], "notification_created");
        assert_eq!(envelope["event_id"], durable.event_id.to_string());
        assert_eq!(envelope["aggregate_id"], created.id.to_string());
        assert_eq!(envelope["version"], durable.version);
        assert_eq!(envelope["data"], durable.payload_json);
        let received: TelegraphDomainEvent =
            serde_json::from_value(envelope["data"].clone()).unwrap();
        assert_eq!(received.event_id(), durable.event_id);
        let TelegraphDomainEvent::NotificationCreated(payload) = &received;
        assert_eq!(payload.user_id, user_id);
        assert_eq!(payload.notification_id, created.id);
        assert_eq!(payload.created_at, created.created_at);
        let sync = handler(fixture.openfga());
        sync.handle_event(Box::new(received)).await.unwrap();
        // Acknowledge the REAL received message only after the public handler
        // succeeded. Replays below reuse the exact same bytes and identity.
        fixture
            .sqs()
            .client
            .delete_message()
            .queue_url(&outbound_url)
            .receipt_handle(messages[0].receipt_handle().unwrap())
            .send()
            .await
            .unwrap();
        for sync in [&sync, &handler(fixture.openfga())] {
            let replay: serde_json::Value = serde_json::from_str(bytes).unwrap();
            let event: TelegraphDomainEvent =
                serde_json::from_value(replay["data"].clone()).unwrap();
            assert_eq!(event.event_id(), durable.event_id);
            sync.handle_event(Box::new(event)).await.unwrap();
        }
        let tuples = fixture
            .openfga()
            .read_tuples(None, None, Some(&object))
            .await
            .unwrap();
        assert_eq!(
            tuples.len(),
            1,
            "same and fresh ledgers converge on one real tuple"
        );
        assert_eq!(tuples[0].user, format!("user:{user_id}"));
        assert_eq!(tuples[0].relation, "recipient");
        assert_eq!(tuples[0].object, object);
        let client = reqwest::Client::new();
        let url = format!("{}/api/notifications/{}/read", runtime.url, created.id);
        let before = notifications::Entity::find_by_id(created.id)
            .one(db.as_ref())
            .await
            .unwrap()
            .unwrap();
        let forbidden = client
            .put(&url)
            .bearer_auth(create_jwt_token(other))
            .send()
            .await
            .unwrap();
        assert_eq!(forbidden.status(), reqwest::StatusCode::FORBIDDEN);
        let after = notifications::Entity::find_by_id(created.id)
            .one(db.as_ref())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            (after.is_read, after.read_at, after.updated_at),
            (before.is_read, before.read_at, before.updated_at),
            "denial cannot mutate the notification"
        );
        let allowed = client
            .put(&url)
            .bearer_auth(create_jwt_token(user_id))
            .send()
            .await
            .unwrap();
        assert_eq!(allowed.status(), reqwest::StatusCode::OK);
        let read = notifications::Entity::find_by_id(created.id)
            .one(db.as_ref())
            .await
            .unwrap()
            .unwrap();
        assert!(read.is_read && read.read_at.is_some());
        assert_business_counts(db.as_ref(), user_id, created.id).await;
        let inbound = fixture.sqs().receive_messages(10, 0).await.unwrap();
        tracing::info!(publisher_outage = outage, message_ids = ?inbound.iter().filter_map(|message| message.message_id()).collect::<Vec<_>>(), "Existing fallback/inbound routing check (no additional Receive)");
        assert!(
            inbound.iter().all(|message| {
                let envelope: serde_json::Value =
                    serde_json::from_str(message.body().unwrap()).unwrap();
                envelope["event_type"] != "notification_created"
            }),
            "AuthZ event must not land on fallback/inbound"
        );
        // A bounded, non-secret evidence line survives libtest's successful-test
        // capture. No JWT, transport credential, or full payload is logged.
        {
            use std::io::Write;
            let mut output = std::io::stdout().lock();
            writeln!(
                output,
                "TELEGRAPH_OUTBOX_PROOF {}",
                serde_json::json!({
                    "publisher_outage": outage,
                    "event_id": durable.event_id,
                    "notification_id": created.id,
                    "recipient_id": user_id,
                    "other_user_id": other,
                    "published_status": published.status,
                    "tuple_count": tuples.len(),
                    "tuple_user": tuples[0].user,
                    "other_http_status": forbidden.status().as_u16(),
                    "recipient_http_status": allowed.status().as_u16(),
                    "is_read": read.is_read,
                })
            )
            .expect("persist bounded correlation witness");
            output.flush().expect("flush correlation witness");
        }
        runtime.stop().await;
    }
}
