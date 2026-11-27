#![allow(clippy::unwrap_used)]

use std::{
    future::pending,
    sync::Arc,
    time::Duration,
};

use kikiutils::task::manager::TaskManager;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

// Constants/Statics
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);
const PENDING_PROBE: Duration = Duration::from_millis(50);

// Tests
#[tokio::test]
async fn wait_returns_once_every_task_finished() {
    let manager = TaskManager::new();
    let release = CancellationToken::new();
    for _ in 0..8 {
        let release = release.clone();
        manager.spawn(async move { release.cancelled().await });
    }

    assert_eq!(manager.len(), 8);

    let mut wait = Box::pin(manager.wait());
    assert!(timeout(PENDING_PROBE, &mut wait).await.is_err());

    release.cancel();
    wait.await;

    assert!(manager.is_empty());
}

#[tokio::test]
async fn shutdown_cancels_tokens_and_waits_for_tasks_spawned_during_shutdown() {
    let manager = Arc::new(TaskManager::new());
    let child_cancelled = CancellationToken::new();

    let spawning_manager = Arc::clone(&manager);
    let reported_child_cancelled = child_cancelled.clone();
    manager.spawn_with_token(async move |cancel_token| {
        cancel_token.cancelled().await;
        // A task spawned after `cancel` gets an already cancelled token and is still waited for.
        spawning_manager.spawn_with_token(async move |child_token| {
            tokio::task::yield_now().await;
            if child_token.is_cancelled() {
                reported_child_cancelled.cancel();
            }
        });
    });

    timeout(COMPLETION_TIMEOUT, manager.shutdown()).await.unwrap();

    assert!(child_cancelled.is_cancelled());
    assert!(manager.is_empty());
}

#[tokio::test]
async fn reopen_hands_out_fresh_tokens_after_cancel() {
    let manager = TaskManager::new();
    let token_before_cancel = manager.token();
    manager.shutdown().await;
    assert!(token_before_cancel.is_cancelled());
    assert!(manager.is_cancelled());

    let cancelled_before_reopen = manager.spawn_with_token(async |cancel_token| cancel_token.is_cancelled());
    assert!(cancelled_before_reopen.await.unwrap());

    manager.reopen();
    assert!(token_before_cancel.is_cancelled());
    assert!(!manager.token().is_cancelled());
    let cancelled_after_reopen = manager.spawn_with_token(async |cancel_token| cancel_token.is_cancelled());
    assert!(!cancelled_after_reopen.await.unwrap());
}

#[tokio::test]
async fn reopen_keeps_tokens_linked_when_nothing_was_cancelled() {
    let manager = TaskManager::new();
    let task = manager.spawn_with_token(async |cancel_token| cancel_token.cancelled().await);

    manager.reopen();
    manager.cancel();

    timeout(COMPLETION_TIMEOUT, task).await.unwrap().unwrap();
}

#[tokio::test]
async fn cancelling_the_parent_reaches_tokens_handed_out_after_reopen() {
    let parent = CancellationToken::new();
    let manager = TaskManager::with_parent(&parent);
    manager.cancel();
    manager.reopen();

    let task = manager.spawn_with_token(async |cancel_token| cancel_token.cancelled().await);
    parent.cancel();
    timeout(COMPLETION_TIMEOUT, task).await.unwrap().unwrap();

    manager.reopen();
    let cancelled_after_parent = manager.spawn_with_token(async |cancel_token| cancel_token.is_cancelled());
    assert!(cancelled_after_parent.await.unwrap());
}

#[tokio::test]
async fn clones_share_tasks_and_tokens() {
    let manager = TaskManager::new();
    let clone = manager.clone();
    let task = clone.spawn_with_token(async |cancel_token| cancel_token.cancelled().await);

    assert_eq!(manager.len(), 1);
    manager.cancel();
    timeout(COMPLETION_TIMEOUT, task).await.unwrap().unwrap();
    assert!(clone.is_empty());
}

#[cfg(feature = "signal")]
#[tokio::test]
async fn shutdown_signal_listener_does_not_hold_up_shutdown() {
    let manager = TaskManager::new();
    let listener = manager.cancel_on_shutdown_signal();

    timeout(COMPLETION_TIMEOUT, manager.shutdown()).await.unwrap();
    assert!(!listener.await.unwrap().unwrap());
}

#[tokio::test]
async fn shutdown_with_timeout_reports_tasks_that_ignore_cancel() {
    let manager = TaskManager::new();
    manager.spawn_with_token(async |cancel_token| cancel_token.cancelled().await);
    assert!(manager.shutdown_with_timeout(COMPLETION_TIMEOUT).await.is_ok());

    manager.reopen();
    let stuck_task = manager.spawn(pending::<()>());
    assert!(manager.shutdown_with_timeout(PENDING_PROBE).await.is_err());
    assert!(!stuck_task.is_finished());
    assert_eq!(manager.len(), 1);

    stuck_task.abort();
    manager.wait().await;
    assert!(manager.is_empty());
}
