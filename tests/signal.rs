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
    task::manager::TaskManager,
};
use tokio::time::timeout;

// Tests
#[tokio::test]
async fn sigterm_is_reported_as_terminate_and_cancels_the_manager() {
    let manager = TaskManager::new();
    let mut wait = pin!(manager.cancel_on_shutdown_signal());
    // The first poll registers the handlers, so the signal below cannot terminate the process.
    assert!(poll_fn(|cx| Poll::Ready(wait.as_mut().poll(cx).is_pending())).await);

    let pid = process::id().to_string();
    assert!(
        process::Command::new("kill")
            .args(["-TERM", &pid])
            .status()
            .unwrap()
            .success()
    );

    let signal = timeout(Duration::from_secs(5), wait).await.unwrap().unwrap();
    assert_eq!(signal, Some(ShutdownSignal::Terminate));
    assert_eq!(ShutdownSignal::Terminate.to_string(), "SIGTERM");
    assert!(manager.is_cancelled());
}
