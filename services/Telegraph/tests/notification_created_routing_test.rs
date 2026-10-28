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

// G-T1 (authorization106): check scheduling when FAILED is first observed,
// not after the independent 12s broker sampler has completed.
async fn assert_first_failed_retry_is_future(
    db: &sea_orm::DatabaseConnection,
    id: Uuid,
    deadline: tokio::time::Instant,
) {
    tokio::time::timeout_at(deadline, async {
        loop {
            let failed = rustycog::outbox::entity::OutboxEvents::find()
                .filter(rustycog::outbox::entity::Column::AggregateId.eq(id))
                .one(db)
                .await
                .unwrap_or_else(|_| panic!("first FAILED SQL observation failed: notification={id}"))
                .expect("committed notification must have a durable outbox row");
            if failed.status == STATUS_FAILED {
                let observed_at = chrono::Utc::now();
                assert!(
                    failed.next_attempt_at > observed_at,
                    "first FAILED must schedule a future retry: notification={id}, event={}, attempts={}, next_attempt_at={}, observed_at={observed_at}",
                    failed.event_id, failed.attempts, failed.next_attempt_at,
                );
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("first FAILED must be observed within the existing IAM commit deadline");
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

// G-134-O: recovery only. Keep the actual parents and HTTP owner available for
// an observed cleanup if startup or observation exhausts the fixed deadline.
struct Recovery134 {
    app: telegraph_setup::TelegraphApp,
    tasks: Vec<telegraph_setup::runtime::BackgroundTask>,
    _abort: telegraph_setup::runtime::OwnedTaskAbortGuard,
    http: rustycog::http::OwnedHttpServer,
    url: String,
}

impl Recovery134 {
    async fn start(
        config: telegraph_configuration::TelegraphConfig,
        deadline: tokio::time::Instant,
    ) -> anyhow::Result<Self> {
        // No business parent exists while the fallible factory is awaited.
        let app = tokio::time::timeout_at(
            deadline,
            Box::pin(telegraph_setup::TelegraphApp::new(config)),
        )
        .await
        .map_err(|_| anyhow::anyhow!("134 application factory exceeded D"))?
        .map_err(|_| anyhow::anyhow!("134 application factory failed"))?;
        anyhow::ensure!(
            tokio::time::Instant::now() < deadline,
            "134 factory completed after D"
        );
        let mut tasks = app.start_background_tasks();
        let abort = telegraph_setup::runtime::OwnedTaskAbortGuard::new(&tasks);
        // SDK startup owns partial listeners through RAII; no await follows its
        // coordinator spawn. Parent handles stay HERE, outside this timeout.
        let http = tokio::time::timeout_at(
            deadline,
            rustycog::http::start_router_owned(
                axum::Router::new().nest(telegraph_http_server::SERVICE_PREFIX, app.router()),
                rustycog::config::ServerConfig {
                    host: "127.0.0.1".into(),
                    port: 0,
                    tls_enabled: false,
                    ..Default::default()
                },
            ),
        )
        .await;
        let http = match http {
            Ok(Ok(http)) => http,
            result => {
                let reason = if result.is_err() { "timeout" } else { "error" };
                let cleanup = telegraph_setup::runtime::shutdown_owned_tasks(&mut tasks, async {
                    app.stop_background_tasks().await
                })
                .await;
                return telegraph_setup::runtime::combine_results(
                    Err(anyhow::anyhow!("134 HTTP startup {reason}")),
                    cleanup,
                )
                .map(|()| unreachable!());
            }
        };
        let url = format!(
            "http://{}{}",
            http.local_addrs()[0],
            telegraph_http_server::SERVICE_PREFIX
        );
        let owner = Self {
            app,
            tasks,
            _abort: abort,
            http,
            url,
        };
        if tokio::time::Instant::now() >= deadline || owner.tasks.len() != 2 {
            let cleanup = owner.stop().await;
            return telegraph_setup::runtime::combine_results(
                Err(anyhow::anyhow!(
                    "134 startup late or missing active parents"
                )),
                cleanup,
            )
            .map(|()| unreachable!());
        }
        Ok(owner)
    }

    async fn stop(mut self) -> anyhow::Result<()> {
        let started = tokio::time::Instant::now();
        telegraph_setup::runtime::shutdown_owned_runtime(&mut self.tasks, &mut self.http, async {
            self.app.stop_background_tasks().await
        })
        .await?;
        anyhow::ensure!(
            self.tasks.is_empty() && started.elapsed() < Duration::from_secs(6),
            "134 recovery cleanup did not join within6s"
        );
        Ok(())
    }
}

// G-134-E: a measurement precondition, never extra time added to D.
fn recovery134_clock(
    t0: tokio::time::Instant,
    utc0: chrono::DateTime<chrono::Utc>,
) -> anyhow::Result<()> {
    let elapsed = chrono::Duration::from_std(t0.elapsed())?;
    let difference = chrono::Utc::now().signed_duration_since(utc0 + elapsed);
    anyhow::ensure!(
        difference >= -chrono::Duration::milliseconds(100)
            && difference <= chrono::Duration::milliseconds(100),
        "134 PRECONDITION: UTC/monotonic clocks diverged by more than100ms"
    );
    Ok(())
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
        let mut runtime = Some(RunningApp::start(real.clone()).await);
        let mut recovered = None;
        assert_eq!(
            runtime.as_ref().unwrap().tasks.len(),
            2,
            "both transports remain Live even when the queue is absent"
        );
        runtime.as_ref().unwrap().assert_http_healthy().await;
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
                    if outage {
                        assert_first_failed_retry_is_future(db.as_ref(), created.id, deadline).await;
                    }
                    created
                };
                // Both futures are owned by this select; no detached sampler.
                // Passive observations finish inside the SAME absolute 25s
                // deadline; SQL completion time is logged when it actually occurs.
                let (created, ()) = tokio::join!(sql, inbound_snapshot_schedule(fixture.sqs(), outage, wait_started, deadline));
                created
            } => created,
            result = telegraph_setup::runtime::wait_for_background_failure(&mut runtime.as_mut().unwrap().tasks) => {
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
        let (published, messages) = if outage {
            let failed = wait_status(db.as_ref(), created.id, STATUS_FAILED).await;
            assert!(failed.attempts > 0 && failed.attempts < 20);
            assert!(failed
                .last_error
                .as_deref()
                .is_some_and(|error| !error.is_empty()));
            assert_eq!(failed.event_id, durable.event_id);
            assert_eq!(failed.payload_json, durable.payload_json);
            // Destroy the first instance and its state; reconstruct on the SAME
            // writer only after restoring that one real destination.
            runtime.take().unwrap().stop().await;
            let queue = fixture
                .sqs()
                .client
                .create_queue()
                .queue_name(OUTBOUND)
                .send()
                .await
                .unwrap();
            assert!(queue.queue_url().is_some());
            // G-134-E/D: starts BEFORE eligibility SQL and application factory.
            let t0 = tokio::time::Instant::now();
            let utc0 = chrono::Utc::now();
            let eligible =
                tokio::time::timeout_at(t0 + Duration::from_secs(25), row(db.as_ref(), created.id))
                    .await
                    .expect("134 PRECONDITION: bootstrap eligibility SQL exceeded25s");
            assert!(
                tokio::time::Instant::now() < t0 + Duration::from_secs(25),
                "134 PRECONDITION: bootstrap SQL completed at/after25s"
            );
            recovery134_clock(t0, utc0).expect("134 clock precondition");
            assert_eq!(
                (
                    eligible.id,
                    eligible.event_id,
                    eligible.aggregate_id,
                    eligible.version
                ),
                (durable.id, durable.event_id, created.id, durable.version)
            );
            assert_eq!(eligible.payload_json, durable.payload_json);
            assert!(
                eligible.attempts > 0 && eligible.attempts < 20,
                "134 PRECONDITION: retry budget unavailable"
            );
            let eligibility = match eligible.status.as_str() {
                rustycog::outbox::entity::STATUS_PUBLISHING => {
                    assert!(
                        eligible
                            .locked_by
                            .as_deref()
                            .is_some_and(|owner| !owner.is_empty()),
                        "134 PRECONDITION: publishing owner missing"
                    );
                    Some(
                        eligible
                            .locked_until
                            .expect("134 PRECONDITION: publishing expiry missing"),
                    )
                }
                rustycog::outbox::entity::STATUS_PENDING | STATUS_FAILED => {
                    assert!(
                        eligible.locked_by.is_none() && eligible.locked_until.is_none(),
                        "134 PRECONDITION: retry state has incompatible lock"
                    );
                    Some(eligible.next_attempt_at)
                }
                STATUS_PUBLISHED => {
                    assert!(
                        eligible.locked_by.is_none() && eligible.locked_until.is_none(),
                        "134 PRECONDITION: published state still locked"
                    );
                    None // No invented E: already published still requires real SQS bytes.
                }
                _ => panic!("134 PRECONDITION: unsupported durable status"),
            };
            let residual = eligibility.map_or(Duration::ZERO, |at| {
                at.signed_duration_since(utc0)
                    .to_std()
                    .unwrap_or(Duration::ZERO)
            });
            assert!(
                residual <= Duration::from_secs(30),
                "134 PRECONDITION: residual exceeds30s"
            );
            let deadline = t0 + residual + Duration::from_secs(25);
            let deadline_utc =
                utc0 + chrono::Duration::from_std(residual + Duration::from_secs(25)).unwrap();
            tracing::info!(t0_utc = %utc0, eligibility_utc = ?eligibility,
                residual_us = residual.as_micros(), deadline_utc = %deadline_utc,
                deadline_us = deadline.duration_since(t0).as_micros(), event_id = %durable.event_id,
                "134 fixed recovery budget before reconstruction");
            let rebuilt = Recovery134::start(real.clone(), deadline)
                .await
                .expect("134 owned reconstruction");
            let observation = tokio::time::timeout_at(deadline, async {
                recovery134_clock(t0, utc0)?;
                let published = loop {
                    let state = rustycog::outbox::entity::OutboxEvents::find_by_id(durable.id)
                        .one(db.as_ref()).await.map_err(|_| anyhow::anyhow!("134 durable SQL unavailable"))?
                        .ok_or_else(|| anyhow::anyhow!("134 durable intention disappeared"))?;
                    anyhow::ensure!(state.event_id == durable.event_id && state.aggregate_id == created.id
                        && state.version == durable.version && state.payload_json == durable.payload_json,
                        "134 durable identity/version/payload changed");
                    if state.status == STATUS_PUBLISHED { break state; }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                };
                let messages = fixture.sqs().receive_messages_from_queue(OUTBOUND, 10, 2)
                    .await.map_err(|_| anyhow::anyhow!("134 real SQS receive failed"))?;
                anyhow::ensure!(messages.len() == 1, "134 expected exactly one real SQS message");
                let bytes = messages[0].body().ok_or_else(|| anyhow::anyhow!("134 SQS body missing"))?;
                let envelope: serde_json::Value = serde_json::from_str(bytes)
                    .map_err(|_| anyhow::anyhow!("134 actual SQS envelope invalid"))?;
                anyhow::ensure!(envelope["event_type"] == "notification_created"
                    && envelope["event_id"] == durable.event_id.to_string()
                    && envelope["aggregate_id"] == created.id.to_string()
                    && envelope["version"] == durable.version && envelope["data"] == durable.payload_json,
                    "134 actual SQS identity/version/payload mismatch");
                let received: TelegraphDomainEvent = serde_json::from_value(envelope["data"].clone())
                    .map_err(|_| anyhow::anyhow!("134 actual SQS event invalid"))?;
                let TelegraphDomainEvent::NotificationCreated(payload) = &received;
                anyhow::ensure!(received.event_id() == durable.event_id && payload.user_id == user_id
                    && payload.notification_id == created.id && payload.created_at == created.created_at,
                    "134 actual event recipient/notification/timestamp mismatch");
                let notifications = notifications::Entity::find().filter(notifications::Column::UserId.eq(user_id))
                    .count(db.as_ref()).await.map_err(|_| anyhow::anyhow!("134 notification count unavailable"))?;
                let deliveries = notification_deliveries::Entity::find()
                    .filter(notification_deliveries::Column::NotificationId.eq(created.id)).count(db.as_ref()).await
                    .map_err(|_| anyhow::anyhow!("134 delivery count unavailable"))?;
                let intentions = rustycog::outbox::entity::OutboxEvents::find()
                    .filter(rustycog::outbox::entity::Column::AggregateId.eq(created.id)).count(db.as_ref()).await
                    .map_err(|_| anyhow::anyhow!("134 intention count unavailable"))?;
                anyhow::ensure!((notifications, deliveries, intentions) == (1, 1, 1), "134 business counts changed");
                recovery134_clock(t0, utc0)?;
                // T-1/G-134-D: strict completion AFTER the final covered await.
                let completed_at = tokio::time::Instant::now();
                anyhow::ensure!(completed_at < deadline, "134 observations completed at/after D");
                tracing::info!(t0_utc = %utc0, eligibility_utc = ?eligibility, residual_us = residual.as_micros(),
                    deadline_utc = %deadline_utc, deadline_us = deadline.duration_since(t0).as_micros(),
                    completion_us = completed_at.duration_since(t0).as_micros(), event_id = %durable.event_id,
                    "134 fixed recovery observations completed before D");
                Ok::<_, anyhow::Error>((published, messages))
            }).await;
            match observation {
                Ok(Ok(result)) => {
                    recovered = Some(rebuilt);
                    result
                }
                result => {
                    let cleanup = rebuilt.stop().await;
                    cleanup.expect("134 failed observation cleanup");
                    match result {
                        Err(_) => panic!("134 fixed recovery deadline exceeded"),
                        Ok(Err(error)) => panic!("134 recovery observation failed: {error}"),
                        Ok(Ok(_)) => unreachable!("successful observation matched above"),
                    }
                }
            }
        } else {
            // Nominal deadlines and receive are deliberately unchanged.
            let published = wait_status(db.as_ref(), created.id, STATUS_PUBLISHED).await;
            assert_eq!(published.event_id, durable.event_id);
            assert_eq!(published.payload_json, durable.payload_json);
            let messages = fixture
                .sqs()
                .receive_messages_from_queue(OUTBOUND, 10, 2)
                .await
                .unwrap();
            (published, messages)
        };
        assert_eq!(published.event_id, durable.event_id);
        assert_eq!(published.payload_json, durable.payload_json);
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
        let base_url = recovered.as_ref().map_or_else(
            || runtime.as_ref().unwrap().url.as_str(),
            |owner| owner.url.as_str(),
        );
        let url = format!("{base_url}/api/notifications/{}/read", created.id);
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
        if let Some(recovered) = recovered {
            recovered.stop().await.expect("134 final owned cleanup");
        }
        if let Some(runtime) = runtime {
            runtime.stop().await;
        }
    }
}
