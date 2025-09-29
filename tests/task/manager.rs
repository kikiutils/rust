use futures::future::join_all;
use std::time::Duration;
use tokio::time::sleep;

use kikiutils::task::manager::TaskManager;

#[tokio::test]
async fn multiple_tasks() {
    let manager = TaskManager::new();
    let handles: Vec<_> = (0..10).map(|i| manager.spawn(async move { i })).collect();
    let results: Vec<_> = join_all(handles).await.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(results, (0..10).collect::<Vec<_>>());
}

#[tokio::test]
async fn spawn_and_complete() {
    let manager = TaskManager::new();
    let handle = manager.spawn(async { 42 });
    let result = handle.await.unwrap();
    assert_eq!(result, 42);
}

#[tokio::test]
async fn spawn_and_abort() {
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
async fn spawn_and_panic() {
    let manager = TaskManager::new();
    let handle = manager.spawn(async { panic!() });
    let result = handle.await;
    assert!(result.is_err());
    assert!(result.err().unwrap().is_panic());
}
