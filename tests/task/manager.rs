use futures::future::join_all;
use rand::{rng, Rng};
use std::time::Duration;
use tokio::time::sleep;

use kikiutils::task::manager::TaskManager;

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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn join_existing_completes() {
    let manager = TaskManager::new();

    spawn_sleeping_tasks(&manager, Duration::from_millis(100));

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
            }
            1 => {
                // Abort task
                let handle = manager.spawn(async { sleep(Duration::from_secs(5)).await });
                handle.abort();
            }
            _ => {
                // Panic task
                manager.spawn(async { panic!("boom") });
            }
        }
    }

    manager.join_existing().await;

    assert!(manager.is_empty());
}

#[tokio::test]
async fn spawn_abort_cancels() {
    let manager = TaskManager::new();

    let handle = manager.spawn(async {
        sleep(Duration::from_secs(5)).await;
        99
    });

    handle.abort();

    let result = handle.await;
    assert!(result.is_err());
    assert!(result.err().unwrap().is_cancelled());
}

#[tokio::test]
async fn spawn_multiple_returns_results() {
    let manager = TaskManager::new();

    let handles: Vec<_> = (0..10).map(|i| manager.spawn(async move { i })).collect();

    let results: Vec<_> = join_all(handles).await.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(results, (0..10).collect::<Vec<_>>());
}

#[tokio::test]
async fn spawn_panic_propagates() {
    let manager = TaskManager::new();

    let handle = manager.spawn(async { panic!() });

    let result = handle.await;
    assert!(result.is_err());
    assert!(result.err().unwrap().is_panic());
}

#[tokio::test]
async fn spawn_returns_result() {
    let manager = TaskManager::new();

    let handle = manager.spawn(async { 42 });

    let result = handle.await.unwrap();
    assert_eq!(result, 42);
}
