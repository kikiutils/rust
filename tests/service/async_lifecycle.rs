#![allow(clippy::unwrap_used)]

use std::{
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
use kikiutils::{
    atomic::enum_cell::AtomicEnumCell,
    impl_async_service_lifecycle,
    service::{
        async_lifecycle::AsyncServiceLifecycle,
        state::ServiceState,
    },
    task::manager::TaskManager,
};
use tokio::{
    sync::Mutex,
    time::sleep,
};

struct TestService {
    lifecycle_lock: Mutex<()>,
    state: AtomicEnumCell<ServiceState>,
    task_manager: TaskManager,
}

impl TestService {
    fn new(state: ServiceState) -> Self {
        Self {
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

    service.task_manager.spawn_with_token(|token| async move {
        token.cancelled().await;
    });

    let error = service
        .execute_start(async { Result::<()>::Err(anyhow!("startup failed")) })
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "startup failed");
    assert_eq!(service.state.get(), ServiceState::Stopped);
    assert!(service.task_manager.is_empty());
}

#[tokio::test]
async fn execute_stop_cancels_tasks_runs_future_and_marks_service_stopped() {
    let service = TestService::new(ServiceState::Running);
    let cleanup_ran = Arc::new(AtomicBool::new(false));
    let cleanup_ran_in_future = Arc::clone(&cleanup_ran);

    service.task_manager.spawn_with_token(|token| async move {
        token.cancelled().await;
    });

    service
        .execute_stop(async move {
            cleanup_ran_in_future.store(true, Ordering::SeqCst);
        })
        .await;

    assert!(cleanup_ran.load(Ordering::SeqCst));
    assert_eq!(service.state.get(), ServiceState::Stopped);
    assert!(service.task_manager.is_empty());
}

#[tokio::test]
async fn execute_stop_is_noop_when_service_is_already_stopped() {
    let service = TestService::new(ServiceState::Stopped);
    let cleanup_ran = Arc::new(AtomicBool::new(false));
    let cleanup_ran_in_future = Arc::clone(&cleanup_ran);

    service
        .execute_stop(async move {
            cleanup_ran_in_future.store(true, Ordering::SeqCst);
        })
        .await;

    sleep(Duration::from_millis(1)).await;

    assert!(!cleanup_ran.load(Ordering::SeqCst));
    assert_eq!(service.state.get(), ServiceState::Stopped);
}
