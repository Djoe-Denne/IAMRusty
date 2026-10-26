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
    runtime::{shutdown_owned_tasks, BackgroundTask, OwnedTaskAbortGuard},
    TelegraphApp,
};
use uuid::Uuid;

pub struct RunningApp {
    pub app: TelegraphApp,
    pub url: String,
    pub tasks: Vec<BackgroundTask>,
    _abort: OwnedTaskAbortGuard,
    http_stop: tokio::sync::oneshot::Sender<()>,
}

impl RunningApp {
    pub async fn start(config: TelegraphConfig) -> Self {
        let app = Box::pin(TelegraphApp::new(config))
            .await
            .expect("real setup");
        let mut tasks = app.start_background_tasks();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!(
            "http://{}{}/",
            listener.local_addr().unwrap(),
            telegraph_http_server::SERVICE_PREFIX
        );
        let router = axum::Router::new().nest(telegraph_http_server::SERVICE_PREFIX, app.router());
        let (http_stop, stopped) = tokio::sync::oneshot::channel();
        tasks.push(tokio::spawn(async move {
            tokio::select! {
                result = axum::serve(listener, router) => result.map_err(Into::into),
                _ = stopped => Ok(()),
            }
        }));
        let abort = OwnedTaskAbortGuard::new(&tasks);
        Self {
            app,
            url: url.trim_end_matches('/').to_string(),
            tasks,
            _abort: abort,
            http_stop,
        }
    }

    pub async fn stop(mut self) {
        let started = tokio::time::Instant::now();
        let stop = self.http_stop;
        shutdown_owned_tasks(&mut self.tasks, async {
            let _ = stop.send(());
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
