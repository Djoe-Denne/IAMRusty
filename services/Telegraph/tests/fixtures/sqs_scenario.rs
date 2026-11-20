//! Keep the broker shared, but never give two independent scenarios the same inbox.

use std::{
    error::Error,
    sync::{Arc, Once},
};

use anyhow::{ensure, Context};
use rustycog::{
    config::{load_config_fresh, QueueConfig, SqsConfig},
    testing::sqs_testcontainer::TestSqs,
};
use telegraph_configuration::TelegraphConfig;
use telegraph_setup::{
    runtime::{shutdown_owned_tasks, BackgroundTask, OwnedTaskAbortGuard},
    TelegraphApp,
};
use url::Url;
use uuid::Uuid;

use super::super::{TelegraphTestDescriptor, TelegraphTestFixture};

static LOGGING: Once = Once::new();

/// Create a standard queue on the existing disposable `LocalStack` fixture.
///
/// # Errors
/// Fails if the queue cannot be created or the broker does not return a valid URL.
pub async fn create_queue(sqs: &TestSqs) -> anyhow::Result<(String, Url)> {
    let name = format!("telegraph-it-{}", Uuid::new_v4().simple());
    let created = sqs.client.create_queue().queue_name(&name).send().await?;
    let broker_url = Url::parse(created.queue_url().context("missing test queue URL")?)?;
    // LocalStack advertises its own hostname/port. Preserve the returned queue
    // path but use the mapped endpoint of the fixture that created it.
    let mut queue_url = Url::parse(&sqs.endpoint_url)?;
    queue_url.set_path(broker_url.path());
    Ok((name, queue_url))
}

fn normalize_localhost(mut url: Url) -> anyhow::Result<Url> {
    if url.host_str() == Some("localhost") {
        url.set_host(Some("127.0.0.1"))?;
    }
    Ok(url)
}

/// Point both the producer fixture and the app's inbound config at the new
/// queue.
///
/// Keep the explicit outbound routing and leave the queue owned by the
/// disposable broker: an old receive can still complete there after Rust stops.
///
/// # Errors
/// Fails if queue creation fails or the consumer and fixture endpoints disagree.
pub async fn isolate_inbound(
    fixture: &mut TelegraphTestFixture,
    config: &mut SqsConfig,
) -> anyhow::Result<()> {
    let sqs = fixture
        .fixture
        .sqs
        .as_mut()
        .context("missing SQS fixture")?;
    let endpoint = config
        .endpoint_url()
        .context("missing local SQS endpoint")?;
    ensure!(
        config.region == sqs.region
            && normalize_localhost(Url::parse(&endpoint)?)?
                == normalize_localhost(Url::parse(&sqs.endpoint_url)?)?,
        "SQS config does not match the LocalStack fixture"
    );
    let (name, queue_url) = create_queue(sqs).await?;
    ensure!(
        normalize_localhost(Url::parse(&config.queue_url(&name))?)?
            == normalize_localhost(queue_url.clone())?,
        "SQS consumer URL does not match the scenario queue"
    );
    let queue_url = queue_url.to_string();
    sqs.queue_urls.insert(name.clone(), queue_url.clone());
    sqs.queue_url = queue_url;
    config.default_queues = vec![name];
    Ok(())
}

/// Runs the real Telegraph consumer and outbox for tests that publish events
/// and assert on SMTP or Postgres. HTTP tests keep using their existing harness.
pub struct SqsEventScenario {
    app: TelegraphApp,
    tasks: Vec<BackgroundTask>,
    _abort: OwnedTaskAbortGuard,
    pub fixture: TelegraphTestFixture,
}

impl SqsEventScenario {
    /// Start the real consumer and outbox with an isolated inbound queue.
    ///
    /// # Errors
    ///
    /// Returns an error if fixtures, configuration, the app, or its tasks cannot start.
    pub async fn start() -> Result<Self, Box<dyn Error>> {
        let mut fixture = TelegraphTestFixture::new(Arc::new(TelegraphTestDescriptor)).await?;
        let mut config = load_config_fresh::<TelegraphConfig>()?;
        let QueueConfig::Sqs(sqs_config) = &mut config.queue else {
            return Err("SQS event scenario requires SQS configuration".into());
        };
        isolate_inbound(&mut fixture, sqs_config).await?;
        fixture.smtp().clear_emails().await?;
        LOGGING.call_once(|| telegraph_configuration::setup_logging(&config));
        let app = Box::pin(TelegraphApp::new(config)).await?;
        let tasks = app.start_background_tasks();
        let abort = OwnedTaskAbortGuard::new(&tasks);
        if tasks.len() != 2 {
            return Err("SQS event scenario requires a live consumer and outbox dispatcher".into());
        }
        Ok(Self {
            app,
            tasks,
            _abort: abort,
            fixture,
        })
    }

    /// Stop and join the scenario's owned background tasks.
    ///
    /// # Errors
    ///
    /// Returns an error if bounded shutdown fails or a task fails or panics.
    pub async fn stop(mut self) -> anyhow::Result<()> {
        shutdown_owned_tasks(&mut self.tasks, self.app.stop_background_tasks()).await
    }
}
