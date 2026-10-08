use std::future::pending;
use std::sync::Arc;

use futures::future::select_all;
use tokio::task::{JoinError, JoinHandle};

use crate::config::{load_monolith_config, MonolithConfig};
use crate::in_process_binding_grant::InProcessBindingGrantClient;
use crate::in_process_iam_signer::InProcessIamOrganizationSignerClient;
use crate::routes::{compose_routes, MonolithRouters};

/// Compose nested service runtimes and serve the monolith HTTP stack.
///
/// # Errors
///
/// Returns an error if configuration loading, nested application setup, or
/// server bind fails, or if the in-process `IAM` signer or `Manifesto` grant
/// reader is unavailable.
pub async fn run() -> anyhow::Result<()> {
    setup_logging_once();
    let MonolithConfig {
        server,
        iam,
        telegraph,
        hive,
        manifesto,
        lazaret,
    } = load_monolith_config()?;

    // IAM first — InProcess signer requires the application façade (ADR-0306).
    let iam_app = Box::pin(iam_setup::app::build_app_state(iam, None)).await?;
    let iam_signer_facade = iam_app.organization_signer().ok_or_else(|| {
        anyhow::anyhow!(
            "IAM organization signer façade unavailable; refusing HTTP fallback in monolith (ADR-0306 fail-closed)"
        )
    })?;
    let in_process_signer: Arc<dyn hive_domain::port::service::IamOrganizationSignerClient> =
        Arc::new(InProcessIamOrganizationSignerClient::new(iam_signer_facade));

    let telegraph_app = Box::pin(telegraph_setup::AppBuilder::new(telegraph).build()).await?;
    let hive_app = Box::pin(
        hive_setup::AppBuilder::new(hive)
            .with_iam_organization_signer_client(in_process_signer)
            .build(),
    )
    .await?;
    let manifesto_app = Box::pin(manifesto_setup::Application::new(manifesto)).await?;
    let binding_grant_reader = manifesto_app.binding_grant_snapshots().ok_or_else(|| {
        anyhow::anyhow!(
            "Manifesto binding grant snapshot reader unavailable; refusing HTTP fallback in monolith (ADR-0104 fail-closed)"
        )
    })?;
    let in_process_grants: Arc<dyn lazaret_domain::BindingGrantSnapshotPort> =
        Arc::new(InProcessBindingGrantClient::new(binding_grant_reader));
    let lazaret_app = Box::pin(
        lazaret_setup::AppBuilder::new(lazaret)
            .with_outbound(lazaret_setup::LazaretOutboundOverrides {
                binding_grant_snapshots: Some(in_process_grants),
            })
            .build(),
    )
    .await?;

    let mut iam_tasks = iam_app.start_background_tasks();
    let mut background_tasks = Vec::new();
    background_tasks.extend(telegraph_app.start_background_tasks());
    background_tasks.extend(hive_app.start_background_tasks());
    background_tasks.extend(manifesto_app.start_background_tasks());
    background_tasks.extend(lazaret_app.start_background_tasks());

    let readiness = std::sync::Arc::new(readiness::ReadinessProbe::aggregate(
        "monolith",
        vec![
            ("iam", iam_app.readiness()),
            ("telegraph", telegraph_app.readiness()),
            ("hive", hive_app.readiness()),
            ("manifesto", manifesto_app.readiness()),
            ("lazaret", lazaret_app.readiness()),
        ],
    ));

    let router = compose_routes(
        MonolithRouters {
            iam: iam_app.router(),
            telegraph: telegraph_app.router(),
            hive: hive_app.router(),
            manifesto: manifesto_app.router(),
            lazaret: lazaret_app.router(),
        },
        readiness,
    );

    let mut server = tokio::spawn(async move {
        rustycog::http::serve_router(router, server)
            .await
            .map_err(|e| anyhow::anyhow!("Monolith HTTP server failed: {e}"))
    });

    let _abort_on_cancel = OwnedTaskAbortGuard(
        iam_tasks
            .iter()
            .chain(background_tasks.iter())
            .map(JoinHandle::abort_handle)
            .chain(std::iter::once(server.abort_handle()))
            .collect(),
    );
    let (result, server_finished) =
        wait_for_shutdown_or_failure(&mut server, &mut iam_tasks, &mut background_tasks).await;

    let iam_cleanup = iam_app.shutdown_background_tasks(&mut iam_tasks).await;
    telegraph_app.stop_background_tasks().await;
    hive_app.stop_background_tasks().await;
    manifesto_app.stop_background_tasks().await;
    lazaret_app.stop_background_tasks().await;
    let other_cleanup = drain_background_tasks(&mut background_tasks).await;
    if !server_finished {
        server.abort();
        let _ = server.await;
    }
    result.and(iam_cleanup).and(other_cleanup)
}

fn setup_logging_once() {
    let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .try_init();
}

async fn wait_for_shutdown_or_failure(
    server: &mut JoinHandle<anyhow::Result<()>>,
    iam_tasks: &mut Vec<JoinHandle<anyhow::Result<()>>>,
    background_tasks: &mut Vec<JoinHandle<anyhow::Result<()>>>,
) -> (anyhow::Result<()>, bool) {
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutdown signal received; stopping monolith runtime");
            (Ok(()), false)
        }
        result = server => (flatten_join_result("Monolith HTTP server", result), true),
        result = iam_setup::app::wait_for_background_failure(iam_tasks) => (result, false),
        result = wait_for_first_background_task(background_tasks) => (result, false),
    }
}

async fn wait_for_first_background_task(
    background_tasks: &mut Vec<JoinHandle<anyhow::Result<()>>>,
) -> anyhow::Result<()> {
    loop {
        if background_tasks.is_empty() {
            pending::<()>().await;
            unreachable!("pending future never resolves");
        }

        let (result, index, remaining_tasks) = select_all(background_tasks.iter_mut()).await;
        drop(remaining_tasks);
        drop(background_tasks.swap_remove(index));
        if let Ok(Ok(())) = &result {
            tracing::warn!("Monolith background task exited cleanly; HTTP listener continues");
        } else {
            return flatten_join_result("Monolith background task", result);
        }
    }
}

struct OwnedTaskAbortGuard(Vec<tokio::task::AbortHandle>);

impl Drop for OwnedTaskAbortGuard {
    fn drop(&mut self) {
        for task in &self.0 {
            task.abort();
        }
    }
}

async fn drain_background_tasks(
    tasks: &mut Vec<JoinHandle<anyhow::Result<()>>>,
) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut first_error = None;
    while !tasks.is_empty() {
        match tokio::time::timeout_at(deadline, select_all(tasks.iter_mut())).await {
            Ok((result, index, remaining)) => {
                drop(remaining);
                drop(tasks.swap_remove(index));
                let error = flatten_join_result("Monolith background task", result).err();
                if first_error.is_none() {
                    first_error = error;
                }
            }
            Err(_) => {
                for task in tasks.iter() {
                    task.abort();
                }
                for task in tasks.drain(..) {
                    let _ = task.await;
                }
                if first_error.is_none() {
                    first_error = Some(anyhow::anyhow!("Monolith background drain timed out"));
                }
            }
        }
    }
    first_error.map_or(Ok(()), Err)
}

fn flatten_join_result(
    task_name: &str,
    result: Result<anyhow::Result<()>, JoinError>,
) -> anyhow::Result<()> {
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(anyhow::anyhow!("{task_name} failed: {error}")),
        Err(error) => Err(anyhow::anyhow!("{task_name} panicked: {error}")),
    }
}
