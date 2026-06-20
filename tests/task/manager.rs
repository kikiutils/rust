#![allow(clippy::unwrap_used)]

use std::time::Duration;

use kikiutils::task::manager::TaskManager;
use rand::{
    RngExt,
    rng,
};
use tokio::time::sleep;

// Helpers
fn spawn_sleeping_tasks(manager: &TaskManager, dur: Duration) {
    for _ in 0..50 {
        manager.spawn(async move { sleep(dur).await });
    }
}

// Tests
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn abort_and_join_clears_tasks() {
    let manager = TaskManager::new();

    spawn_sleeping_tasks(&manager, Duration::from_secs(10));

    manager.abort_and_join_existing().await;
    assert!(manager.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn abort_existing_then_join() {
    let manager = TaskManager::new();

    spawn_sleeping_tasks(&manager, Duration::from_secs(10));

    manager.abort_existing();
    manager.join_existing().await;
    assert!(manager.is_empty());
}

#[tokio::test]
async fn abort_by_id_cancels_task() {
    let manager = TaskManager::new();

    let task = manager.spawn(async {
        sleep(Duration::from_secs(5)).await;
        99
    });

    let id = task.id();

    assert!(manager.abort(id));
    assert!(task.join().await.unwrap_err().is_cancelled());
    assert!(!manager.abort(id));
    assert!(manager.is_empty());
}

#[tokio::test]
async fn default_and_new_are_equivalent() {
    let manager1 = TaskManager::new();
    let manager2 = TaskManager::default();

    assert!(manager1.is_empty());
    assert!(manager2.is_empty());
}

#[tokio::test]
async fn len_and_task_count_reflect_state() {
    let manager = TaskManager::new();

    assert_eq!(manager.len(), 0);
    assert_eq!(manager.task_count(), 0);

    manager.spawn(async { sleep(Duration::from_millis(50)).await });

    assert_eq!(manager.len(), 1);
    assert_eq!(manager.task_count(), 1);

    manager.join_existing().await;
    assert_eq!(manager.len(), 0);
    assert_eq!(manager.task_count(), 0);
}

#[tokio::test]
async fn has_tasks_reflects_state() {
    let manager = TaskManager::new();

    assert!(!manager.has_tasks());
    assert!(manager.is_empty());

    manager.spawn(async { sleep(Duration::from_millis(50)).await });
    assert!(manager.has_tasks());

    manager.join_existing().await;
    assert!(!manager.has_tasks());
    assert!(manager.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn immediate_tasks_are_removed_after_joining_handles() {
    let manager = TaskManager::new();

    let tasks = (0..1000).map(|i| manager.spawn(async move { i })).collect::<Vec<_>>();

    let mut results = Vec::new();
    for task in tasks {
        results.push(task.join().await.unwrap());
    }

    assert_eq!(results, (0..1000).collect::<Vec<_>>());
    assert!(manager.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn join_existing_completes() {
    let manager = TaskManager::new();

    spawn_sleeping_tasks(&manager, Duration::from_millis(100));

    manager.join_existing().await;
    assert!(manager.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancel_and_join_existing_clears_cooperative_tasks() {
    let manager = TaskManager::new();

    for _ in 0..50 {
        manager.spawn_with_token(|token| async move {
            token.cancelled().await;
        });
    }

    manager.cancel_and_join_existing().await;
    assert!(manager.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancel_existing_clears_cooperative_tasks_after_join() {
    let manager = TaskManager::new();

    for _ in 0..50 {
        manager.spawn_with_token(|token| async move {
            token.cancelled().await;
        });
    }

    manager.cancel_existing();
    manager.join_existing().await;
    assert!(manager.is_empty());
}

#[tokio::test]
async fn join_with_no_tasks_is_safe() {
    let manager = TaskManager::new();

    manager.join_existing().await;
    manager.abort_and_join_existing().await;
    manager.abort_existing();
    assert!(manager.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mixed_randomized_tasks() {
    let manager = TaskManager::new();
    let mut rng = rng();

    for _ in 0..50 {
        let choice = rng.random_range(0..3);
        match choice {
            0 => {
                // Normal task
                let delay = rng.random_range(10..200);
                manager.spawn(async move { sleep(Duration::from_millis(delay)).await });
            },
            1 => {
                // Abort task
                let task = manager.spawn(async { sleep(Duration::from_secs(5)).await });
                task.abort();
            },
            _ => {
                // Panic task
                manager.spawn(async { panic!("boom") });
            },
        }
    }

    manager.join_existing().await;
    assert!(manager.is_empty());
}

#[tokio::test]
async fn spawn_abort_cancels() {
    let manager = TaskManager::new();

    let task = manager.spawn(async {
        sleep(Duration::from_secs(5)).await;
        99
    });

    task.abort();

    let result = task.join().await;
    assert!(result.is_err());
    assert!(result.err().unwrap().is_cancelled());
}

#[tokio::test]
async fn managed_task_cancel_returns_false_without_token() {
    let manager = TaskManager::new();

    let task = manager.spawn(async { 42 });

    assert!(!task.cancel());
    assert_eq!(task.join().await.unwrap(), 42);
    assert!(manager.is_empty());
}

#[tokio::test]
async fn spawn_with_token_cancel_wakes_task() {
    let manager = TaskManager::new();

    let task = manager.spawn_with_token(|token| async move {
        token.cancelled().await;
        42
    });

    assert!(manager.cancel(task.id()));
    assert_eq!(task.join().await.unwrap(), 42);
    assert!(manager.is_empty());
}

#[tokio::test]
async fn managed_task_cancel_wakes_token_task() {
    let manager = TaskManager::new();

    let task = manager.spawn_with_token(|token| async move {
        token.cancelled().await;
        42
    });

    assert!(task.cancel());
    assert_eq!(task.join().await.unwrap(), 42);
    assert!(manager.is_empty());
}

#[tokio::test]
async fn manager_cancel_returns_false_for_missing_or_non_token_task() {
    let manager = TaskManager::new();

    assert!(!manager.cancel(999));

    let task = manager.spawn(async { sleep(Duration::from_millis(50)).await });
    assert!(!manager.cancel(task.id()));

    task.join().await.unwrap();
    assert!(manager.is_empty());
}

#[tokio::test]
async fn into_handle_returns_original_join_handle() {
    let manager = TaskManager::new();

    let task = manager.spawn(async { 42 });
    let handle = task.into_handle();

    assert_eq!(handle.await.unwrap(), 42);
    assert!(manager.is_empty());
}

#[tokio::test]
async fn spawn_multiple_returns_results() {
    let manager = TaskManager::new();

    let tasks = (0..10).map(|i| manager.spawn(async move { i })).collect::<Vec<_>>();

    let mut results = Vec::new();
    for task in tasks {
        results.push(task.join().await.unwrap());
    }

    assert_eq!(results, (0..10).collect::<Vec<_>>());
}

#[tokio::test]
async fn spawn_panic_propagates() {
    let manager = TaskManager::new();

    let task = manager.spawn(async { panic!() });

    let result = task.join().await;
    assert!(result.is_err());
    assert!(result.err().unwrap().is_panic());
}

#[tokio::test]
async fn spawn_returns_result() {
    let manager = TaskManager::new();

    let task = manager.spawn(async { 42 });

    let result = task.join().await.unwrap();
    assert_eq!(result, 42);
}
