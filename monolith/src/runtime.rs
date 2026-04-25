use std::future::pending;

use futures::future::select_all;
use tokio::task::{JoinError, JoinHandle};

use crate::config::{load_monolith_config, MonolithConfig};
use crate::routes::{compose_routes, MonolithRouters};

pub async fn run() -> anyhow::Result<()> {
    let MonolithConfig {
        server,
        iam,
        telegraph,
        hive,
        manifesto,
    } = load_monolith_config()?;

    setup_logging_once();

    let iam_app = iam_setup::app::build_app_state(iam, None).await?;
    let telegraph_app = telegraph_setup::AppBuilder::new(telegraph)
        .build()
        .await?;
    let hive_app = hive_setup::AppBuilder::new(hive).build().await?;
    let manifesto_app = manifesto_setup::Application::new(manifesto).await?;

    let mut background_tasks = Vec::new();
    background_tasks.extend(telegraph_app.start_background_tasks());
    background_tasks.extend(manifesto_app.start_background_tasks());

    let router = compose_routes(MonolithRouters {
        iam: iam_app.router(),
        telegraph: telegraph_app.router(),
        hive: hive_app.router(),
        manifesto: manifesto_app.router(),
    });

    let server = tokio::spawn(async move {
        rustycog_http::serve_router(router, server)
            .await
            .map_err(|e| anyhow::anyhow!("Monolith HTTP server failed: {}", e))
    });

    let result = wait_for_shutdown_or_failure(server, background_tasks).await;

    telegraph_app.stop_background_tasks().await;
    manifesto_app.stop_background_tasks().await;

    result
}

fn setup_logging_once() {
    let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .try_init();
}

async fn wait_for_shutdown_or_failure(
    mut server: JoinHandle<anyhow::Result<()>>,
    background_tasks: Vec<JoinHandle<anyhow::Result<()>>>,
) -> anyhow::Result<()> {
    let background_wait = wait_for_first_background_task(background_tasks);

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutdown signal received; stopping monolith runtime");
            server.abort();
            Ok(())
        }
        result = &mut server => flatten_join_result("Monolith HTTP server", result),
        result = background_wait => {
            server.abort();
            result
        }
    }
}

async fn wait_for_first_background_task(
    background_tasks: Vec<JoinHandle<anyhow::Result<()>>>,
) -> anyhow::Result<()> {
    if background_tasks.is_empty() {
        pending::<()>().await;
        unreachable!("pending future never resolves");
    }

    let (result, _index, remaining_tasks) = select_all(background_tasks).await;
    for task in remaining_tasks {
        task.abort();
    }

    flatten_join_result("Monolith background task", result)
}

fn flatten_join_result(
    task_name: &str,
    result: Result<anyhow::Result<()>, JoinError>,
) -> anyhow::Result<()> {
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(anyhow::anyhow!("{} failed: {}", task_name, error)),
        Err(error) => Err(anyhow::anyhow!("{} panicked: {}", task_name, error)),
    }
}
