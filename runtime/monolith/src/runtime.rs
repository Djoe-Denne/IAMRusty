use std::sync::Arc;

use telegraph_setup::runtime::{
    combine_results, flatten_join_result, shutdown_owned_tasks, wait_for_background_failure,
    OwnedTaskAbortGuard,
};
use tokio::task::JoinHandle;

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

    let (http_stop, http_stopped) = tokio::sync::oneshot::channel();
    let mut server = tokio::spawn(async move {
        tokio::select! {
            result = rustycog::http::serve_router(router, server) => result.map_err(|e| anyhow::anyhow!("Monolith HTTP server failed: {e}")),
            _ = http_stopped => Ok(()),
        }
    });

    let _abort_iam = OwnedTaskAbortGuard::new(&iam_tasks);
    let _abort_background = OwnedTaskAbortGuard::new(&background_tasks);
    let _abort_http = OwnedTaskAbortGuard::new(std::slice::from_ref(&server));
    let (result, server_finished) =
        wait_for_shutdown_or_failure(&mut server, &mut iam_tasks, &mut background_tasks).await;

    background_tasks.append(&mut iam_tasks);
    if !server_finished {
        background_tasks.push(server);
    }
    // The shared absolute deadline begins BEFORE any stop is polled. Stops run
    // concurrently, so another nested service cannot postpone Telegraph's stop.
    let cleanup = shutdown_owned_tasks(&mut background_tasks, async {
        let _ = http_stop.send(());
        let (iam, telegraph, (), (), ()) = tokio::join!(
            iam_app.stop_background_tasks(),
            telegraph_app.stop_background_tasks(),
            hive_app.stop_background_tasks(),
            manifesto_app.stop_background_tasks(),
            lazaret_app.stop_background_tasks(),
        );
        combine_results(iam, telegraph)
    })
    .await;
    combine_results(result, cleanup)
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
        result = wait_for_background_failure(background_tasks) => (result, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    struct Owner(Arc<AtomicUsize>);
    impl Drop for Owner {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn monolith_observes_finished_telegraph_error_and_panic_then_drains_without_other_services(
    ) {
        for panic in [false, true] {
            let destroyed = Arc::new(AtomicUsize::new(0));
            let owner = Owner(destroyed.clone());
            let mut server = tokio::spawn(async move {
                let _owner = owner;
                std::future::pending::<()>().await;
                Ok(())
            });
            let mut background = vec![tokio::spawn(async move {
                assert!(!panic, "Telegraph panic witness");
                anyhow::bail!("Telegraph error witness")
            })];
            while !background[0].is_finished() {
                tokio::task::yield_now().await;
            }
            // IAM's existing contract rejects an empty worker set; a pending
            // owned handle models the host without constructing IAM itself.
            let mut iam = vec![tokio::spawn(std::future::pending())];
            let _abort_iam = OwnedTaskAbortGuard::new(&iam);
            let _abort_background = OwnedTaskAbortGuard::new(&background);
            let _abort_http = OwnedTaskAbortGuard::new(std::slice::from_ref(&server));
            let (result, server_finished) =
                wait_for_shutdown_or_failure(&mut server, &mut iam, &mut background).await;
            let error = result.unwrap_err().to_string();
            assert!(error.contains(if panic {
                "Telegraph panic witness"
            } else {
                "Telegraph error witness"
            }));
            assert!(!server_finished);
            assert!(background.is_empty());
            background.append(&mut iam);
            background.push(server);
            let started = tokio::time::Instant::now();
            shutdown_owned_tasks(&mut background, async { Ok(()) })
                .await
                .unwrap();
            assert!(started.elapsed() < Duration::from_secs(6));
            assert_eq!(destroyed.load(Ordering::SeqCst), 1);
            assert!(background.is_empty());
        }
    }

    #[tokio::test]
    async fn monolith_disabled_telegraph_keeps_http_alive() {
        let mut server = tokio::spawn(std::future::pending());
        let mut iam = vec![tokio::spawn(std::future::pending())];
        let _abort_iam = OwnedTaskAbortGuard::new(&iam);
        let _abort_http = OwnedTaskAbortGuard::new(std::slice::from_ref(&server));
        let mut background = Vec::new();
        assert!(tokio::time::timeout(
            Duration::from_millis(30),
            wait_for_shutdown_or_failure(&mut server, &mut iam, &mut background)
        )
        .await
        .is_err());
        server.abort();
        assert!(server.await.unwrap_err().is_cancelled());
        for task in iam {
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
        }
    }
}
