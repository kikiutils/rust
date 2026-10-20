#![allow(clippy::unwrap_used)]

use std::{
    future::pending,
    sync::{
        Arc,
        atomic::{
            AtomicBool,
            Ordering,
        },
    },
    time::Duration,
};

use anyhow::{
    Result,
    anyhow,
};
use async_trait::async_trait;
use kikiutils::{
    atomic::enum_cell::AtomicEnumCell,
    impl_async_service_lifecycle,
    service::{
        async_lifecycle::{
            AsyncServiceLifecycle,
            AsyncServiceLifecycleHooks,
        },
        state::ServiceState,
    },
    task::manager::TaskManager,
};
use tokio::{
    sync::Mutex,
    time::timeout,
};

// Constants/Statics
const CANCEL_AFTER: Duration = Duration::from_millis(20);

// Structs
struct TestService {
    cleanup_called: AtomicBool,
    cleanup_should_fail: AtomicBool,
    lifecycle_lock: Mutex<()>,
    state: AtomicEnumCell<ServiceState>,
    task_manager: TaskManager,
}

#[async_trait]
impl AsyncServiceLifecycleHooks for TestService {
    async fn cleanup_resources(&self) -> Result<()> {
        self.cleanup_called.store(true, Ordering::SeqCst);
        if self.cleanup_should_fail.load(Ordering::SeqCst) {
            Err(anyhow!("cleanup failed"))
        } else {
            Ok(())
        }
    }
}

impl TestService {
    fn new(state: ServiceState) -> Self {
        Self {
            cleanup_called: AtomicBool::new(false),
            cleanup_should_fail: AtomicBool::new(false),
            lifecycle_lock: Mutex::new(()),
            state: AtomicEnumCell::new(state),
            task_manager: TaskManager::new(),
        }
    }
}

impl_async_service_lifecycle!(TestService);

#[tokio::test]
async fn execute_start_runs_future_and_marks_service_running() {
    let service = TestService::new(ServiceState::Stopped);
    let started = Arc::new(AtomicBool::new(false));
    let started_in_future = Arc::clone(&started);

    service
        .execute_start(async move {
            started_in_future.store(true, Ordering::SeqCst);
            Result::<()>::Ok(())
        })
        .await
        .unwrap();

    assert!(started.load(Ordering::SeqCst));
    assert_eq!(service.state.get(), ServiceState::Running);
}

#[tokio::test]
async fn execute_start_is_noop_when_service_is_already_active() {
    let service = TestService::new(ServiceState::Running);
    let polled = Arc::new(AtomicBool::new(false));
    let polled_in_future = Arc::clone(&polled);

    service
        .execute_start(async move {
            polled_in_future.store(true, Ordering::SeqCst);
            Result::<()>::Ok(())
        })
        .await
        .unwrap();

    assert!(!polled.load(Ordering::SeqCst));
    assert_eq!(service.state.get(), ServiceState::Running);
}

#[tokio::test]
async fn execute_start_failure_restores_stopped_and_clears_tasks() {
    let service = TestService::new(ServiceState::Stopped);

    service.task_manager.spawn_with_token(async |cancel_token| {
        cancel_token.cancelled().await;
    });

    let error = service
        .execute_start(async { Result::<()>::Err(anyhow!("startup failed")) })
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "startup failed");
    assert_eq!(service.state.get(), ServiceState::Stopped);
    assert!(service.task_manager.is_empty());
    assert!(service.cleanup_called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn execute_start_failure_marks_cleanup_failed_when_rollback_fails() {
    let service = TestService::new(ServiceState::Stopped);
    service.cleanup_should_fail.store(true, Ordering::SeqCst);

    let error = service
        .execute_start(async { Result::<()>::Err(anyhow!("startup failed")) })
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "startup failed");
    assert_eq!(service.state.get(), ServiceState::CleanupFailed);
    assert!(service.cleanup_called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn execute_start_is_rejected_after_cleanup_failed() {
    let service = TestService::new(ServiceState::CleanupFailed);

    let error = service.execute_start(async { Result::<()>::Ok(()) }).await.unwrap_err();

    assert_eq!(
        error.to_string(),
        "cannot start service after cleanup failed; retry cleanup first"
    );
}

#[tokio::test]
async fn execute_stop_cancels_tasks_runs_cleanup_and_marks_service_stopped() {
    let service = TestService::new(ServiceState::Running);

    service.task_manager.spawn_with_token(async |cancel_token| {
        cancel_token.cancelled().await;
    });

    service.execute_stop().await.unwrap();

    assert!(service.cleanup_called.load(Ordering::SeqCst));
    assert_eq!(service.state.get(), ServiceState::Stopped);
    assert!(service.task_manager.is_empty());
}

#[tokio::test]
async fn execute_stop_is_noop_when_service_is_already_stopped() {
    let service = TestService::new(ServiceState::Stopped);

    service.execute_stop().await.unwrap();

    assert!(!service.cleanup_called.load(Ordering::SeqCst));
    assert_eq!(service.state.get(), ServiceState::Stopped);
}

#[tokio::test]
async fn execute_stop_marks_cleanup_failed_when_cleanup_fails() {
    let service = TestService::new(ServiceState::Running);
    service.cleanup_should_fail.store(true, Ordering::SeqCst);

    let error = service.execute_stop().await.unwrap_err();

    assert_eq!(error.to_string(), "cleanup failed");
    assert_eq!(service.state.get(), ServiceState::CleanupFailed);
}

#[tokio::test]
async fn execute_stop_retries_cleanup_after_cleanup_failed() {
    let service = TestService::new(ServiceState::CleanupFailed);

    service.execute_stop().await.unwrap();

    assert!(service.cleanup_called.load(Ordering::SeqCst));
    assert_eq!(service.state.get(), ServiceState::Stopped);
}

#[tokio::test]
async fn cancelled_start_requires_cleanup_before_restart() {
    let service = TestService::new(ServiceState::Stopped);

    assert!(
        timeout(CANCEL_AFTER, service.execute_start(pending::<Result<()>>()))
            .await
            .is_err()
    );

    assert_eq!(service.state.get(), ServiceState::CleanupFailed);
    assert!(service.execute_start(async { Result::<()>::Ok(()) }).await.is_err());

    service.execute_stop().await.unwrap();
    assert!(service.cleanup_called.load(Ordering::SeqCst));
    assert_eq!(service.state.get(), ServiceState::Stopped);

    service.execute_start(async { Result::<()>::Ok(()) }).await.unwrap();
    assert_eq!(service.state.get(), ServiceState::Running);
}

#[tokio::test]
async fn cancelled_stop_lets_the_next_stop_retry_cleanup() {
    let service = TestService::new(ServiceState::Running);

    // A task without a cancel token keeps `join_existing` waiting until it is aborted.
    service.task_manager.spawn(pending::<()>());

    assert!(timeout(CANCEL_AFTER, service.execute_stop()).await.is_err());
    assert_eq!(service.state.get(), ServiceState::CleanupFailed);

    service.task_manager.abort_existing();
    service.execute_stop().await.unwrap();

    assert_eq!(service.state.get(), ServiceState::Stopped);
    assert!(service.task_manager.is_empty());
}
