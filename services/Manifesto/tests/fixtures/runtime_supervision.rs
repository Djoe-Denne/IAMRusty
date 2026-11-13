//! Supervisor cleanup regressions. Real Tokio tasks; no HTTP/DB/auth simulation.
use super::drain_unselected_task;

#[tokio::test]
async fn selected_failed_server_output_is_not_polled_twice() {
    let mut server = tokio::spawn(async { Err::<(), _>("address already in use") });
    let result = tokio::select! { result = &mut server => result };
    assert_eq!(result.unwrap(), Err("address already in use"));
    drain_unselected_task(server, true).await;
}

#[tokio::test]
async fn selected_successful_background_output_is_not_polled_twice() {
    let mut background = tokio::spawn(async { Ok::<(), &'static str>(()) });
    let result = tokio::select! { result = &mut background => result };
    assert_eq!(result.unwrap(), Ok(()));
    drain_unselected_task(background, true).await;
}

#[tokio::test]
async fn unselected_finished_task_is_still_joined_without_double_poll() {
    let task = tokio::spawn(async {});
    while !task.is_finished() {
        tokio::task::yield_now().await;
    }
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        drain_unselected_task(task, false),
    )
    .await
    .expect("finished task cleanup must terminate");
}

#[tokio::test]
async fn unselected_running_task_is_aborted_and_drained() {
    let (started, ready) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let _ = started.send(());
        std::future::pending::<()>().await;
    });
    ready.await.unwrap();
    let abort_handle = task.abort_handle();
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        drain_unselected_task(task, false),
    )
    .await
    .expect("pending task cleanup must abort and terminate");
    assert!(
        abort_handle.is_finished(),
        "cleanup must wait for task exit"
    );
}
