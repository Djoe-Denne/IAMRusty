//! Bounded ownership fixture using the production setup/runtime surfaces.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use rustycog::config::load_config_fresh;
use rustycog::outbox::{entity as outbox, OutboxRecorder};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use telegraph_configuration::TelegraphConfig;
use telegraph_domain::{
    entity::communication::{CommunicationRecipient, NotificationCommunication},
    service::NotificationServiceImpl,
};
use telegraph_infra::repository::{
    CombinedNotificationRepositoryImpl, NotificationReadRepositoryImpl,
    NotificationWriteRepositoryImpl,
};
use telegraph_setup::{
    runtime::{shutdown_owned_runtime, BackgroundTask, OwnedTaskAbortGuard},
    TelegraphApp,
};
use uuid::Uuid;

pub struct RunningApp {
    pub app: TelegraphApp,
    pub url: String,
    pub tasks: Vec<BackgroundTask>,
    _abort: OwnedTaskAbortGuard,
    http: rustycog::http::OwnedHttpServer,
}

impl RunningApp {
    pub async fn start(config: TelegraphConfig) -> Self {
        let app = Box::pin(TelegraphApp::new(config))
            .await
            .expect("real setup");
        let tasks = app.start_background_tasks();
        let abort = OwnedTaskAbortGuard::new(&tasks);
        let http = rustycog::http::start_router_owned(
            axum::Router::new().nest(telegraph_http_server::SERVICE_PREFIX, app.router()),
            rustycog::config::ServerConfig {
                host: "127.0.0.1".into(),
                port: 0,
                tls_enabled: false,
                ..Default::default()
            },
        )
        .await
        .expect("owned HTTP startup");
        let url = format!(
            "http://{}{}/",
            http.local_addrs()[0],
            telegraph_http_server::SERVICE_PREFIX
        );
        Self {
            app,
            url: url.trim_end_matches('/').to_string(),
            tasks,
            _abort: abort,
            http,
        }
    }

    /// QA compatibility item 3: HTTP is owned separately from business parents.
    pub async fn assert_http_healthy(&self) {
        let address = self.http.local_addrs()[0];
        assert_eq!(
            self.url,
            format!("http://{address}{}", telegraph_http_server::SERVICE_PREFIX),
            "fixture URL must address its retained HTTP owner"
        );
        let response = reqwest::Client::new()
            .get(format!("{}/health", self.url))
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .expect("owned HTTP health is reachable");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
    }

    pub async fn stop(mut self) {
        let started = tokio::time::Instant::now();
        shutdown_owned_runtime(&mut self.tasks, &mut self.http, async {
            self.app.stop_background_tasks().await
        })
        .await
        .expect("all owned runtime tasks joined");
        assert!(started.elapsed() < Duration::from_secs(6));
        assert!(self.tasks.is_empty());
    }
}

pub fn config() -> TelegraphConfig {
    load_config_fresh::<TelegraphConfig>().expect("fixture-published real endpoints")
}

pub fn service(
    db: Arc<DatabaseConnection>,
) -> NotificationServiceImpl<CombinedNotificationRepositoryImpl> {
    NotificationServiceImpl::new(Arc::new(CombinedNotificationRepositoryImpl::new(
        Arc::new(NotificationReadRepositoryImpl::new(db.clone())),
        Arc::new(NotificationWriteRepositoryImpl::new(
            db,
            OutboxRecorder::new(),
        )),
    )))
}

pub fn notification() -> NotificationCommunication {
    NotificationCommunication {
        id: Some(Uuid::new_v4()),
        recipient: CommunicationRecipient {
            user_id: Some(Uuid::new_v4()),
            email: None,
        },
        title: "suspended durable intention".into(),
        body: "body".into(),
        data: HashMap::new(),
        is_read: None,
        created_at: None,
        updated_at: None,
        read_at: None,
    }
}

pub async fn row(db: &DatabaseConnection, id: Uuid) -> outbox::Model {
    outbox::OutboxEvents::find()
        .filter(outbox::Column::AggregateId.eq(id))
        .one(db)
        .await
        .unwrap()
        .expect("durable intention")
}

pub async fn wait_status(db: &DatabaseConnection, id: Uuid, status: &str) -> outbox::Model {
    tokio::time::timeout(Duration::from_secs(25), async {
        loop {
            let row = row(db, id).await;
            if row.status == status {
                break row;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("outbox reached expected durable state")
}
