//! Own the Application-returned SDK server; always join before resuming assertions.
use super::d_vendor::must;
use rustycog::http::{HttpShutdownBudget, OwnedHttpServer};
use std::{future::Future, time::Duration};

pub async fn run<F, Fut>(mut server: OwnedHttpServer, check: F)
where
    F: FnOnce(String) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let addrs = server.local_addrs();
    let origin = addrs.first().map(|a| format!("http://{a}"));
    let mut assertions = tokio::spawn(async move {
        assert_eq!(
            addrs.len(),
            1,
            "exactly one Application-owned HTTP listener"
        );
        assert_ne!(
            addrs[0].port(),
            0,
            "actual assigned port must be observable"
        );
        check(origin.expect("owned listener address")).await;
    });
    let result = tokio::time::timeout(Duration::from_secs(20), &mut assertions).await;
    if result.is_err() {
        assertions.abort();
        let _ = assertions.await;
    }
    let now = tokio::time::Instant::now();
    server.request_shutdown(must(
        HttpShutdownBudget::new(now + Duration::from_secs(5), now + Duration::from_secs(6)),
        "SDK five/six-second shutdown budget",
    ));
    let report = must(server.join().await, "Application-owned listener join");
    assert!(
        !report.forced,
        "owned listener must shut down without force"
    );
    result
        .unwrap_or_else(|_| panic!("Application test watchdog fired"))
        .unwrap_or_else(|e| {
            if e.is_panic() {
                std::panic::resume_unwind(e.into_panic());
            }
            panic!("Application assertion task cancelled");
        });
}
