//! Join only the listeners recorded against this fixture, on pass AND panic.
//! Never stops containers, the SDK singleton, or another fixture's listeners.
use futures::FutureExt;
use std::{future::Future, panic::AssertUnwindSafe};

pub async fn run<T>(fixture: &crate::common::TestFixture, body: impl Future<Output = T>) -> T {
    let outcome = AssertUnwindSafe(body).catch_unwind().await;
    let cleanup = crate::common::cleanup_test_servers(fixture).await;
    if cleanup.is_err() {
        // Keep the original panic, but make a failed join visible without secrets.
        eprintln!("fixture-owned listener cleanup failed; parent lease reconciliation required");
    }
    match outcome {
        Ok(value) => {
            assert!(
                cleanup.is_ok(),
                "fixture-owned listener cleanup must finish before lease release"
            );
            value
        }
        Err(payload) => std::panic::resume_unwind(payload),
    }
}
