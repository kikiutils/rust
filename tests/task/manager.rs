#![allow(clippy::unwrap_used)]

use std::{
    future::pending,
    sync::Arc,
};

use kikiutils::task::manager::TaskManager;
use tokio::{
    spawn,
    sync::oneshot::channel,
    task::yield_now,
};

// Tests
#[tokio::test]
async fn manager_starts_empty_and_tracks_task_counts() {
    let manager = TaskManager::new();

    assert!(manager.is_empty());
    assert!(!manager.has_tasks());
    assert_eq!(manager.len(), 0);
    assert_eq!(manager.task_count(), 0);

    let task = manager.spawn(async {});

    assert!(!manager.is_empty());
    assert!(manager.has_tasks());
    assert_eq!(manager.len(), 1);
    assert_eq!(manager.task_count(), 1);

    task.join().await.unwrap();

    assert!(manager.is_empty());
    assert_eq!(manager.task_count(), 0);
}

#[tokio::test]
async fn managed_task_join_and_into_join_handle_preserve_result() {
    let manager = TaskManager::new();

    let task = manager.spawn(async { 42 });
    assert_eq!(task.join().await.unwrap(), 42);
    assert!(manager.is_empty());

    let task = manager.spawn(async { 84 });
    let join_handle = task.into_join_handle();
    assert_eq!(join_handle.await.unwrap(), 84);
    assert!(manager.is_empty());
}

#[tokio::test]
async fn managed_task_abort_cancels_task() {
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

    let task = manager.spawn_with_token(|token| async move {
        token.cancelled().await;
        42
    });

    assert!(task.cancel());
    assert_eq!(task.join().await.unwrap(), 42);
    assert!(manager.is_empty());
}

#[tokio::test]
async fn manager_cancel_by_id_handles_token_and_non_token_tasks() {
    let manager = TaskManager::new();

    let non_token_task = manager.spawn(pending::<()>());
    let non_token_task_id = non_token_task.id();
    assert!(!manager.cancel(non_token_task_id));

    let token_task = manager.spawn_with_token(|token| async move {
        token.cancelled().await;
    });

    let token_task_id = token_task.id();
    assert!(manager.cancel(token_task_id));

    assert!(manager.abort(non_token_task_id));
    manager.join_existing().await;

    assert!(token_task.join().await.is_ok());
    assert!(non_token_task.join().await.is_err());
    assert!(!manager.cancel(token_task_id));
    assert!(!manager.abort(non_token_task_id));
}

#[tokio::test]
async fn manager_join_by_id_waits_for_completion_and_rejects_missing_ids() {
    let manager = TaskManager::new();
    let (sender, receiver) = channel();
    let task = manager.spawn(async move {
        receiver.await.unwrap();
        42
    });

    let task_id = task.id();

    sender.send(()).unwrap();

    assert!(manager.join(task_id).await);
    assert_eq!(task.join().await.unwrap(), 42);
    assert!(!manager.join(task_id).await);
}

#[tokio::test]
async fn join_existing_waits_for_normal_tasks_and_cleans_entries() {
    let manager = TaskManager::new();

    for _ in 0..16 {
        manager.spawn(async {
            tokio::task::yield_now().await;
        });
    }

    manager.join_existing().await;

    assert!(manager.is_empty());
}

#[tokio::test]
async fn cancel_and_join_existing_is_cooperative() {
    let manager = TaskManager::new();

    for _ in 0..16 {
        manager.spawn_with_token(|token| async move {
            token.cancelled().await;
        });
    }

    manager.cancel_and_join_existing().await;

    assert!(manager.is_empty());
}

#[tokio::test]
async fn abort_existing_and_abort_and_join_existing_cancel_tasks() {
    let manager = TaskManager::new();

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
async fn manager_remains_reusable_after_draining_tasks() {
    let manager = TaskManager::default();

    manager.spawn_with_token(|token| async move {
        token.cancelled().await;
    });

    manager.cancel_and_join_existing().await;

    manager.spawn(async {});
    manager.join_existing().await;

    assert!(manager.is_empty());
}

#[tokio::test]
async fn concurrent_spawn_and_join_does_not_leave_entries_registered() {
    let manager = Arc::new(TaskManager::new());
    let producer_manager = Arc::clone(&manager);
    let producer = spawn(async move {
        for index in 0..128 {
            producer_manager.spawn(async {
                yield_now().await;
            });

            if index % 8 == 0 {
                yield_now().await;
            }
        }
    });

    let joiner_manager = Arc::clone(&manager);
    let joiner = spawn(async move {
        joiner_manager.join_existing().await;
    });

    producer.await.unwrap();
    joiner.await.unwrap();
    manager.join_existing().await;

    assert!(manager.is_empty());
}
