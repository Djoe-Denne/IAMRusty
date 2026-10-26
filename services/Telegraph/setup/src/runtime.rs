//! Ownership and bounded shutdown shared by Telegraph and its monolith host.

use std::future::Future;
use std::time::Duration;

use futures::future::select_all;
use tokio::task::{AbortHandle, JoinError, JoinHandle};
use tokio::time::{timeout_at, Instant};

pub type BackgroundTask = JoinHandle<anyhow::Result<()>>;

/// Start the shared standalone and monolith HTTP owner.
///
/// # Errors
/// Returns listener or TLS startup failures from the SDK.
pub async fn start_owned_http(
    router: axum::Router,
    config: rustycog::config::ServerConfig,
) -> anyhow::Result<rustycog::http::OwnedHttpServer> {
    rustycog::http::start_router_owned(router, config).await
}

/// Abort every owned parent if the supervising future itself is cancelled.
/// SQS parents in the pinned SDK own their pollers cancellation-safely.
pub struct OwnedTaskAbortGuard(Vec<AbortHandle>);

impl OwnedTaskAbortGuard {
    #[must_use]
    pub fn new(tasks: &[BackgroundTask]) -> Self {
        Self(tasks.iter().map(JoinHandle::abort_handle).collect())
    }
}

impl Drop for OwnedTaskAbortGuard {
    fn drop(&mut self) {
        for task in &self.0 {
            task.abort();
        }
    }
}

/// Consume results, including tasks which finished before observation.
/// Intentionally suspended transports must not be spawned. Active Telegraph
/// tasks turn an unexpected clean exit into an error at their owning boundary.
///
/// # Errors
/// Returns the first task failure or panic with its context.
pub async fn wait_for_background_failure(tasks: &mut Vec<BackgroundTask>) -> anyhow::Result<()> {
    loop {
        if tasks.is_empty() {
            std::future::pending::<()>().await;
        }
        let (result, index, remaining) = select_all(tasks.iter_mut()).await;
        drop(remaining);
        drop(tasks.swap_remove(index));
        flatten_join_result("Background task", result)?;
    }
}

/// Request stop and consume all owned results within one absolute six-second
/// budget, including time spent in stop. After five seconds abort all remaining
/// async tasks and allow at most one further second for their joins.
///
/// # Errors
/// Reports stop failures, task failures/panics and deadline overruns together.
/// A timeout is never reported as successful cleanup.
pub async fn shutdown_owned_tasks(
    tasks: &mut Vec<BackgroundTask>,
    stop: impl Future<Output = anyhow::Result<()>>,
) -> anyhow::Result<()> {
    let started = Instant::now();
    let grace = started + Duration::from_secs(5);
    let deadline = started + Duration::from_secs(6);
    shutdown_owned_tasks_until(tasks, stop, grace, deadline).await
}

/// Stop HTTP and business tasks concurrently under the same absolute budget.
/// # Errors
/// Preserves both groups' failures and never attests an unobserved HTTP join.
pub async fn shutdown_owned_runtime(
    tasks: &mut Vec<BackgroundTask>,
    http: &mut rustycog::http::OwnedHttpServer,
    stop: impl Future<Output = anyhow::Result<()>>,
) -> anyhow::Result<rustycog::http::HttpShutdownReport> {
    let started = Instant::now();
    let grace = started + Duration::from_secs(5);
    let deadline = started + Duration::from_secs(6);
    http.request_shutdown(rustycog::http::HttpShutdownBudget::new(grace, deadline)?);
    let (background, http) = tokio::join!(
        shutdown_owned_tasks_until(tasks, stop, grace, deadline),
        http.join(),
    );
    match http {
        Ok(report) => {
            background?;
            if report.forced {
                tracing::warn!("HTTP shutdown required forced cancellation");
            }
            Ok(report)
        }
        Err(error) => match background {
            Ok(()) => Err(error),
            Err(background) => Err(anyhow::anyhow!(
                "{background:#}; HTTP cleanup also failed: {error:#}"
            )),
        },
    }
}

async fn shutdown_owned_tasks_until(
    tasks: &mut Vec<BackgroundTask>,
    stop: impl Future<Output = anyhow::Result<()>>,
    grace: Instant,
    deadline: Instant,
) -> anyhow::Result<()> {
    let _abort_on_cancel = OwnedTaskAbortGuard::new(tasks);
    let mut errors = Vec::new();
    match timeout_at(grace, stop).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => errors.push(format!("Background stop failed: {error:#}")),
        Err(_) => errors.push("Background stop exceeded grace deadline".to_string()),
    }
    let mut aborted = false;
    while !tasks.is_empty() {
        let until = if aborted { deadline } else { grace };
        match timeout_at(until, select_all(tasks.iter_mut())).await {
            Ok((result, index, remaining)) => {
                drop(remaining);
                drop(tasks.swap_remove(index));
                if aborted && result.as_ref().is_err_and(JoinError::is_cancelled) {
                    continue;
                }
                if let Err(error) = flatten_join_result("Background cleanup", result) {
                    errors.push(format!("{error:#}"));
                }
            }
            Err(_) if !aborted => {
                for task in tasks.iter() {
                    task.abort();
                }
                aborted = true;
            }
            Err(_) => {
                errors.push(format!(
                    "Global six-second shutdown deadline exceeded; {} task joins unobserved",
                    tasks.len()
                ));
                break;
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(errors.join("; "))
    }
}

/// Preserve both the original runtime error and cleanup diagnostics.
///
/// # Errors
/// Returns either failure, or both contextualized when both operations failed.
pub fn combine_results(
    result: anyhow::Result<()>,
    cleanup: anyhow::Result<()>,
) -> anyhow::Result<()> {
    match (result, cleanup) {
        (Ok(()), result) | (result, Ok(())) => result,
        (Err(error), Err(cleanup)) => Err(anyhow::anyhow!(
            "{error:#}; cleanup also failed: {cleanup:#}"
        )),
    }
}

/// # Errors
/// Contextualizes inner errors and join failures, including panics.
pub fn flatten_join_result(
    task: &str,
    result: Result<anyhow::Result<()>, JoinError>,
) -> anyhow::Result<()> {
    match result {
        Ok(result) => result.map_err(|error| anyhow::anyhow!("{task} failed: {error:#}")),
        Err(error) => Err(anyhow::anyhow!("{task} join failed: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    struct Destroyed(Arc<AtomicUsize>);
    impl Drop for Destroyed {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    async fn c2_health(port: u16) -> Result<String, String> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        tokio::time::timeout(Duration::from_secs(1), async {
            let mut socket = tokio::net::TcpStream::connect(("127.0.0.1", port)).await?;
            socket
                .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .await?;
            let mut response = String::new();
            socket.read_to_string(&mut response).await?;
            Ok::<_, std::io::Error>(response)
        })
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
    }

    // C-2 / QA compatibility item 2: callers agree with the legacy cached port.
    #[tokio::test]
    async fn shared_startup_port_zero_serves_health_on_actual_port() {
        let config = rustycog::config::ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            tls_enabled: false,
            ..Default::default()
        };
        let expected = config.actual_port();
        let router = axum::Router::new().route(
            "/health",
            axum::routing::get(|| async { ([("x-c2-witness", "shared-startup")], "healthy") }),
        );
        let mut http = start_owned_http(router, config)
            .await
            .expect("real shared startup");
        let bound = http.local_addrs()[0];
        let actual_health = c2_health(bound.port()).await;
        let expected_health = c2_health(expected).await;
        let mut tasks = Vec::new();
        let joined = shutdown_owned_runtime(&mut tasks, &mut http, async { Ok(()) }).await;
        eprintln!("HTTP_C2 expected={expected} bound={bound} actual={actual_health:?} expected_health={expected_health:?} joined={joined:?} cleanup=true");
        assert!(
            joined.is_ok(),
            "shared HTTP runtime must join before the port oracle"
        );
        assert!(tasks.is_empty());
        assert!(
            actual_health.unwrap().starts_with("HTTP/1.1 200"),
            "precondition: the real shared startup serves health"
        );
        assert_eq!(
            bound.port(),
            expected,
            "C-2: shared callers must bind actual_port()"
        );
        let response =
            expected_health.expect("C-2: health must be reachable on expected actual_port");
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(response.contains("x-c2-witness: shared-startup\r\n"));
    }

    #[tokio::test]
    async fn observes_already_finished_error_and_panic_without_filtering() {
        for panic in [false, true] {
            let mut tasks = vec![tokio::spawn(async move {
                assert!(!panic, "panic witness");
                anyhow::bail!("finished error witness")
            })];
            while !tasks[0].is_finished() {
                tokio::task::yield_now().await;
            }
            let error = wait_for_background_failure(&mut tasks).await.unwrap_err();
            assert!(format!("{error:#}").contains(if panic {
                "panic witness"
            } else {
                "finished error witness"
            }));
            assert!(tasks.is_empty(), "finished result must be consumed");
        }
    }

    #[tokio::test]
    async fn disabled_empty_tasks_do_not_exit_supervision() {
        let mut tasks = Vec::new();
        assert!(tokio::time::timeout(
            Duration::from_millis(30),
            wait_for_background_failure(&mut tasks)
        )
        .await
        .is_err());
        shutdown_owned_tasks(&mut tasks, async { Ok(()) })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn drains_consumer_dispatcher_and_http_owners_after_cooperative_stop() {
        let destroyed = Arc::new(AtomicUsize::new(0));
        let (stop, _) = tokio::sync::watch::channel(false);
        let mut tasks = Vec::new();
        for _ in 0..3 {
            let owner = Destroyed(destroyed.clone());
            let mut stop = stop.subscribe();
            tasks.push(tokio::spawn(async move {
                let _owner = owner;
                stop.wait_for(|stop| *stop).await?;
                Ok(())
            }));
        }
        let start = Instant::now();
        shutdown_owned_tasks(&mut tasks, async {
            stop.send_replace(true);
            Ok(())
        })
        .await
        .unwrap();
        assert!(start.elapsed() < Duration::from_secs(6));
        assert!(tasks.is_empty());
        assert_eq!(destroyed.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn stop_wait_is_inside_global_budget_and_every_owner_is_destroyed() {
        let destroyed = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();
        for _ in 0..3 {
            let owner = Destroyed(destroyed.clone());
            tasks.push(tokio::spawn(async move {
                let _owner = owner;
                std::future::pending::<()>().await;
                Ok(())
            }));
        }
        let start = Instant::now();
        let error = shutdown_owned_tasks(&mut tasks, std::future::pending())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("stop exceeded grace"));
        assert!(start.elapsed() < Duration::from_secs(6));
        assert!(tasks.is_empty());
        assert_eq!(destroyed.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn cancellation_of_owner_aborts_all_owned_tasks() {
        let destroyed = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();
        for _ in 0..2 {
            let owner = Destroyed(destroyed.clone());
            tasks.push(tokio::spawn(async move {
                let _owner = owner;
                std::future::pending::<()>().await;
                Ok(())
            }));
        }
        let guard = OwnedTaskAbortGuard::new(&tasks);
        drop(guard);
        for task in tasks {
            assert!(task.await.unwrap_err().is_cancelled());
        }
        assert_eq!(destroyed.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn original_and_cleanup_errors_both_remain_observable() {
        let error = combine_results(
            Err(anyhow::anyhow!("original")),
            Err(anyhow::anyhow!("cleanup")),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("original") && error.contains("cleanup"));
    }

    /// G-1/G-5: one absolute deadline covers the real HTTP owner AND stop.
    #[tokio::test]
    async fn owned_http_and_pending_stop_share_budget_and_destroy_request() {
        use tokio::io::AsyncWriteExt;
        let destroyed = Arc::new(AtomicUsize::new(0));
        let entered = Arc::new(tokio::sync::Notify::new());
        let router = axum::Router::new().route(
            "/owned",
            axum::routing::get({
                let (destroyed, entered) = (destroyed.clone(), entered.clone());
                move || {
                    let owner = Destroyed(destroyed.clone());
                    let entered = entered.clone();
                    async move {
                        let _owner = owner;
                        entered.notify_one();
                        std::future::pending::<()>().await;
                        "unreachable"
                    }
                }
            }),
        );
        let mut http = rustycog::http::start_router_owned(
            router,
            rustycog::config::ServerConfig {
                host: "127.0.0.1".into(),
                port: 0,
                tls_enabled: false,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let mut socket = tokio::net::TcpStream::connect(http.local_addrs()[0])
            .await
            .unwrap();
        socket
            .write_all(b"GET /owned HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(1), entered.notified())
            .await
            .unwrap();
        let started = Instant::now();
        let mut tasks = Vec::new();
        let result = shutdown_owned_runtime(&mut tasks, &mut http, std::future::pending()).await;
        let elapsed = started.elapsed();
        let at_return = destroyed.load(Ordering::SeqCst);
        drop(socket);
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("stop exceeded grace"));
        assert!(
            elapsed < Duration::from_secs(6),
            "stop must not receive a second budget: {elapsed:?}"
        );
        assert_eq!(
            at_return, 1,
            "real HTTP handler must be destroyed despite stop timeout"
        );
        assert!(http.join().await.unwrap().forced);
    }
}
