//! A separate test binary, because a signal reaches every listener in the process.
#![cfg(all(unix, feature = "signal", feature = "task"))]
#![allow(clippy::unwrap_used)]
use std::{
    future::poll_fn,
    pin::pin,
    process,
    task::Poll,
    time::Duration,
};

use kikiutils::{
    signal::ShutdownSignal,
    task::manager::{
        ShutdownOutcome,
        TaskManager,
    },
};
use tokio::time::timeout;

// Constants/Statics
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);

// Functions
fn send_sigterm() {
    let pid = process::id().to_string();
    assert!(
        process::Command::new("kill")
            .args(["-TERM", &pid])
            .status()
            .unwrap()
            .success()
    );
}

// Tests

/// One test, because tests run in parallel and the signal would also reach the other one.
#[tokio::test]
async fn shutdown_signal_waits_end_on_cancel_or_sigterm() {
    let manager = TaskManager::new();
    let mut wait = pin!(manager.cancel_on_shutdown_signal());
    // The first poll registers the handlers, so the signal below cannot terminate the process.
    assert!(poll_fn(|cx| Poll::Ready(wait.as_mut().poll(cx).is_pending())).await);
    manager.cancel();
    assert_eq!(timeout(Duration::from_secs(5), wait).await.unwrap().unwrap(), None);

    manager.reopen();
    let mut wait = pin!(manager.cancel_on_shutdown_signal());
    assert!(poll_fn(|cx| Poll::Ready(wait.as_mut().poll(cx).is_pending())).await);

    send_sigterm();
    let signal = timeout(COMPLETION_TIMEOUT, wait).await.unwrap().unwrap();
    assert_eq!(signal, Some(ShutdownSignal::Terminate));
    assert!(manager.is_cancelled());

    // Tasks that honour the token finish within the grace period.
    let manager = TaskManager::new();
    manager.spawn_with_token(async |token| token.cancelled().await);
    let mut shutdown = pin!(manager.shutdown_on_signal(COMPLETION_TIMEOUT));
    assert!(poll_fn(|cx| Poll::Ready(shutdown.as_mut().poll(cx).is_pending())).await);
    manager.cancel();
    let outcome = timeout(COMPLETION_TIMEOUT, shutdown).await.unwrap().unwrap();
    assert_eq!(outcome, ShutdownOutcome::Completed);
    assert!(manager.is_empty());

    // A task that ignores the token outlives the grace period and stays tracked.
    let manager = TaskManager::new();
    let stuck = manager.spawn(std::future::pending::<()>());
    manager.cancel();
    let outcome = timeout(
        COMPLETION_TIMEOUT,
        manager.shutdown_on_signal(Duration::from_millis(20)),
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(outcome, ShutdownOutcome::TimedOut);
    assert_eq!(manager.len(), 1);
    stuck.abort();

    // A second signal during the grace period ends the wait at once.
    let manager = TaskManager::new();
    let stuck = manager.spawn(std::future::pending::<()>());
    let mut shutdown = pin!(manager.shutdown_on_signal(Duration::from_secs(60)));
    assert!(poll_fn(|cx| Poll::Ready(shutdown.as_mut().poll(cx).is_pending())).await);
    send_sigterm();
    // The poll that sees the first signal also registers the listener for the second one.
    timeout(COMPLETION_TIMEOUT, async {
        while !manager.is_cancelled() {
            assert!(poll_fn(|cx| Poll::Ready(shutdown.as_mut().poll(cx).is_pending())).await);
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    send_sigterm();
    let outcome = timeout(COMPLETION_TIMEOUT, shutdown).await.unwrap().unwrap();
    assert_eq!(outcome, ShutdownOutcome::Forced(ShutdownSignal::Terminate));
    stuck.abort();
}
