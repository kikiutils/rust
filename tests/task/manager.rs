#![allow(clippy::unwrap_used)]

use std::{
    future::pending,
    sync::Arc,
    time::Duration,
};

use kikiutils::task::manager::TaskManager;
use tokio::{
    runtime::Builder,
    spawn,
    sync::oneshot::channel,
    task::{
        id,
        yield_now,
    },
    time::timeout,
};
use tokio_util::sync::CancellationToken;

// Constants/Statics
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);
const PENDING_PROBE: Duration = Duration::from_millis(50);

// Tests: task handles
#[tokio::test]
async fn manager_tracks_task_counts_until_tasks_finish() {
    let manager = TaskManager::new();

    assert!(manager.is_empty());
    assert_eq!(manager.len(), 0);

    let task = manager.spawn(async {});

    assert!(!manager.is_empty());
    assert_eq!(manager.len(), 1);

    task.join().await.unwrap();

    assert!(manager.is_empty());
}

#[tokio::test]
async fn managed_task_join_and_into_join_handle_preserve_result() {
    let manager = TaskManager::new();

    assert_eq!(manager.spawn(async { 42 }).join().await.unwrap(), 42);
    assert_eq!(manager.spawn(async { 84 }).into_join_handle().await.unwrap(), 84);
    assert!(manager.is_empty());
}

#[tokio::test]
async fn managed_task_abort_before_first_poll_cleans_up() {
    let manager = TaskManager::new();
    let task = manager.spawn(pending::<()>());

    task.abort();

    assert!(task.join().await.unwrap_err().is_cancelled());
    assert!(manager.is_empty());
}

#[tokio::test]
async fn managed_task_cancel_only_works_with_a_token() {
    let manager = TaskManager::new();

    let task = manager.spawn(async {});
    assert!(!task.cancel());
    task.join().await.unwrap();

    let task = manager.spawn_with_token(async |cancel_token| {
        cancel_token.cancelled().await;
        42
    });

    assert!(task.cancel());
    assert_eq!(task.join().await.unwrap(), 42);
    assert!(manager.is_empty());
}

// Tests: control by ID
#[tokio::test]
async fn manager_cancel_and_abort_by_id_respect_token_and_registration() {
    let manager = TaskManager::new();

    let plain_task = manager.spawn(pending::<()>());
    let plain_task_id = plain_task.id();
    let token_task = manager.spawn_with_token(async |cancel_token| cancel_token.cancelled().await);
    let token_task_id = token_task.id();

    assert!(!manager.cancel(plain_task_id));
    assert!(manager.cancel(token_task_id));
    assert!(manager.abort(plain_task_id));

    assert!(token_task.join().await.is_ok());
    assert!(plain_task.join().await.unwrap_err().is_cancelled());
    assert!(!manager.cancel(token_task_id));
    assert!(!manager.abort(plain_task_id));
}

#[tokio::test]
async fn manager_join_by_id_waits_for_completion_and_rejects_missing_ids() {
    let manager = TaskManager::new();
    let (sender, receiver) = channel();
    let task = manager.spawn(async move { receiver.await.unwrap() });
    let task_id = task.id();

    let mut join = Box::pin(manager.join(task_id));
    assert!(timeout(PENDING_PROBE, &mut join).await.is_err());

    sender.send(42).unwrap();

    assert!(join.await);
    assert_eq!(task.join().await.unwrap(), 42);
    assert!(!manager.join(task_id).await);
}

// Tests: draining existing tasks
#[tokio::test]
async fn join_existing_waits_until_running_tasks_finish() {
    let manager = TaskManager::new();
    let release = CancellationToken::new();
    for _ in 0..16 {
        let release = release.clone();
        manager.spawn(async move { release.cancelled().await });
    }

    let mut join = Box::pin(manager.join_existing());
    assert!(timeout(PENDING_PROBE, &mut join).await.is_err());

    release.cancel();
    join.await;

    assert!(manager.is_empty());
}

#[tokio::test]
async fn cancel_and_join_existing_is_cooperative() {
    let manager = TaskManager::new();
    for _ in 0..16 {
        manager.spawn_with_token(async |cancel_token| cancel_token.cancelled().await);
    }

    manager.cancel_and_join_existing().await;

    assert!(manager.is_empty());
}

#[tokio::test]
async fn abort_existing_variants_stop_tasks_and_keep_manager_reusable() {
    let manager = TaskManager::default();

    for _ in 0..8 {
        manager.spawn(pending::<()>());
    }

    manager.abort_existing();
    manager.join_existing().await;
    assert!(manager.is_empty());

    for _ in 0..8 {
        manager.spawn(pending::<()>());
    }

    manager.abort_and_join_existing().await;
    assert!(manager.is_empty());
}

#[tokio::test]
async fn existing_methods_called_from_a_managed_task_skip_the_caller() {
    let manager = Arc::new(TaskManager::new());
    let release = CancellationToken::new();
    let other = manager.spawn({
        let release = release.clone();
        async move { release.cancelled().await }
    });

    let supervisor_manager = Arc::clone(&manager);
    let mut supervisor = manager
        .spawn_with_token(async move |cancel_token| {
            assert!(!supervisor_manager.join(id()).await);
            supervisor_manager.join_existing().await;
            supervisor_manager.cancel_and_join_existing().await;
            supervisor_manager.abort_and_join_existing().await;
            supervisor_manager.cancel_existing();
            supervisor_manager.abort_existing();
            // A self-abort only takes effect once the task yields.
            yield_now().await;
            cancel_token.is_cancelled()
        })
        .into_join_handle();

    // The supervisor must still be waiting for the other task, not for itself.
    assert!(timeout(PENDING_PROBE, &mut supervisor).await.is_err());

    release.cancel();
    // A timeout here means a join method made the supervisor wait for itself.
    let supervisor_cancelled = timeout(COMPLETION_TIMEOUT, supervisor).await.unwrap().unwrap();

    assert!(!supervisor_cancelled);
    other.join().await.unwrap();
    assert!(manager.is_empty());
}

// Tests: cleanup paths
#[tokio::test]
async fn panicking_task_is_cleaned_up() {
    let manager = TaskManager::new();
    let task = manager.spawn(async { panic!("managed task panic") });

    assert!(task.join().await.unwrap_err().is_panic());
    assert!(manager.is_empty());
}

#[test]
fn runtime_shutdown_cleans_up_unfinished_tasks() {
    let manager = TaskManager::new();
    let runtime = Builder::new_current_thread().build().unwrap();
    runtime.block_on(async {
        for _ in 0..8 {
            manager.spawn(pending::<()>());
        }
    });

    assert_eq!(manager.len(), 8);
    drop(runtime);
    assert!(manager.is_empty());
}

#[test]
fn task_dropped_before_registration_does_not_leave_an_entry() {
    let manager = TaskManager::new();
    let runtime = Builder::new_current_thread().build().unwrap();
    let handle = runtime.handle().clone();
    drop(runtime);

    // Tokio drops tasks spawned onto a shut-down runtime inside `spawn`, so the cleanup runs before
    // the manager has registered the entry.
    let _guard = handle.enter();
    manager.spawn(pending::<()>());

    assert!(manager.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_spawns_and_completions_do_not_leave_entries() {
    let manager = Arc::new(TaskManager::new());
    let producers = (0..4)
        .map(|_| {
            let manager = Arc::clone(&manager);
            spawn(async move { (0..2_000).map(|_| manager.spawn(async {})).collect::<Vec<_>>() })
        })
        .collect::<Vec<_>>();

    for producer in producers {
        for task in producer.await.unwrap() {
            task.join().await.unwrap();
        }
    }

    assert!(manager.is_empty());
}
