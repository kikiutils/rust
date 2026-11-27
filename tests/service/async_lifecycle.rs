#![allow(clippy::unwrap_used)]

use std::{
    future::{
        Future,
        pending,
        ready,
    },
    sync::{
        Arc,
        Mutex,
        atomic::{
            AtomicBool,
            AtomicUsize,
            Ordering,
        },
    },
    time::Duration,
};

use anyhow::{
    Result,
    anyhow,
    bail,
};
use kikiutils::service::{
    async_lifecycle::{
        AsyncServiceLifecycle,
        AsyncServiceLifecycleCore,
        AsyncServiceLifecycleExt,
        AsyncServiceTasks,
        StopTimedOut,
    },
    state::ServiceState,
};
use tokio::{
    task::JoinHandle,
    time::timeout,
};
use tokio_util::sync::CancellationToken;

// Constants/Statics
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);
const SHORT: Duration = Duration::from_millis(20);

// Structs
#[derive(Default)]
struct TestService {
    last_token: Mutex<Option<CancellationToken>>,
    lifecycle: AsyncServiceLifecycleCore,
    on_start_calls: AtomicUsize,
    on_stop_blocks: AtomicBool,
    on_stop_calls: AtomicUsize,
    on_stop_saw_task_exited: AtomicBool,
    on_stop_should_fail: AtomicBool,
    spawn_cooperative_task: AtomicBool,
    spawn_stuck_task: AtomicBool,
    start_blocks_until_cancelled: AtomicBool,
    start_gate: CancellationToken,
    start_should_fail: AtomicBool,
    start_waits_for_gate: AtomicBool,
    stopping_spawn_rejected: Arc<AtomicBool>,
    tasks: Mutex<Option<AsyncServiceTasks>>,
    stuck_task: Mutex<Option<JoinHandle<()>>>,
    task_exited: Arc<AtomicBool>,
    token_live_at_start: AtomicBool,
}

impl AsyncServiceLifecycle for TestService {
    fn lifecycle(&self) -> &AsyncServiceLifecycleCore {
        &self.lifecycle
    }

    async fn on_start(self: &Arc<Self>, tasks: &AsyncServiceTasks) -> Result<()> {
        self.on_start_calls.fetch_add(1, Ordering::SeqCst);
        self.token_live_at_start.store(!tasks.is_cancelled(), Ordering::SeqCst);
        *self.last_token.lock().unwrap() = Some(tasks.token());
        *self.tasks.lock().unwrap() = Some(tasks.clone());

        if flag(&self.spawn_cooperative_task) {
            let stopping_spawn_rejected = Arc::clone(&self.stopping_spawn_rejected);
            let task_exited = Arc::clone(&self.task_exited);
            let run_tasks = tasks.clone();
            tasks.spawn_with_token(async move |cancel_token| {
                cancel_token.cancelled().await;
                let spawned = run_tasks.spawn(ready(()));
                stopping_spawn_rejected.store(spawned.is_err(), Ordering::SeqCst);
                // A task that needs another poll to finish shutting down.
                tokio::task::yield_now().await;
                task_exited.store(true, Ordering::SeqCst);
            })?;
        }

        if flag(&self.spawn_stuck_task) {
            *self.stuck_task.lock().unwrap() = Some(tasks.spawn(pending::<()>())?);
        }

        if flag(&self.start_blocks_until_cancelled) {
            tasks.token().cancelled().await;
        }

        if flag(&self.start_waits_for_gate) {
            self.start_gate.cancelled().await;
        }

        if flag(&self.start_should_fail) {
            bail!("startup failed");
        }

        Ok(())
    }

    async fn on_stop(&self) -> Result<()> {
        self.on_stop_calls.fetch_add(1, Ordering::SeqCst);
        self.on_stop_saw_task_exited
            .store(flag(&self.task_exited), Ordering::SeqCst);

        if flag(&self.on_stop_blocks) {
            pending::<()>().await;
        }

        if flag(&self.on_stop_should_fail) {
            Err(anyhow!("cleanup failed"))
        } else {
            Ok(())
        }
    }
}

impl TestService {
    fn abort_stuck_task(&self) {
        if let Some(task) = self.stuck_task.lock().unwrap().take() {
            task.abort();
        }
    }

    fn last_token(&self) -> CancellationToken {
        self.last_token.lock().unwrap().clone().unwrap()
    }

    fn on_stop_calls(&self) -> usize {
        self.on_stop_calls.load(Ordering::SeqCst)
    }

    fn state(&self) -> ServiceState {
        self.lifecycle.state()
    }
}

struct DefaultStopService {
    lifecycle: AsyncServiceLifecycleCore,
}

impl AsyncServiceLifecycle for DefaultStopService {
    fn lifecycle(&self) -> &AsyncServiceLifecycleCore {
        &self.lifecycle
    }

    fn on_start(self: &Arc<Self>, _: &AsyncServiceTasks) -> impl Future<Output = Result<()>> + Send {
        ready(Ok(()))
    }
}

// Functions
fn assert_stop_timed_out(error: &anyhow::Error) {
    assert_eq!(error.downcast_ref::<StopTimedOut>().unwrap().timeout, SHORT);
    assert_eq!(
        error.to_string(),
        format!("service did not stop within {SHORT:?}; stop it again to retry")
    );
}

fn flag(value: &AtomicBool) -> bool {
    value.load(Ordering::SeqCst)
}

fn set(value: &AtomicBool, enabled: bool) {
    value.store(enabled, Ordering::SeqCst);
}

// Tests
#[tokio::test]
async fn start_runs_on_start_once_and_marks_the_service_running() {
    let service = Arc::new(TestService::default());

    service.start().await.unwrap();
    service.start().await.unwrap();

    assert_eq!(service.on_start_calls.load(Ordering::SeqCst), 1);
    assert_eq!(service.state(), ServiceState::Running);
}

#[tokio::test]
async fn failed_start_rolls_back_after_tasks_exit_and_keeps_the_start_error() {
    let service = Arc::new(TestService::default());
    set(&service.spawn_cooperative_task, true);
    set(&service.start_should_fail, true);
    set(&service.on_stop_should_fail, true);

    let error = service.start().await.unwrap_err();

    assert_eq!(error.to_string(), "startup failed");
    assert_eq!(service.state(), ServiceState::Stopped);
    assert_eq!(service.on_stop_calls(), 1);
    assert!(flag(&service.on_stop_saw_task_exited));
}

#[tokio::test]
async fn stop_runs_on_stop_after_tasks_exit_and_is_a_noop_once_stopped() {
    let service = Arc::new(TestService::default());
    set(&service.spawn_cooperative_task, true);
    service.start().await.unwrap();

    service.stop().await.unwrap();
    service.stop().await.unwrap();

    assert_eq!(service.on_stop_calls(), 1);
    assert!(flag(&service.on_stop_saw_task_exited));
    assert_eq!(service.state(), ServiceState::Stopped);
}

#[tokio::test]
async fn cancelling_the_parent_interrupts_a_start_in_progress() {
    let parent = CancellationToken::new();
    let service = Arc::new(TestService {
        lifecycle: AsyncServiceLifecycleCore::with_parent(&parent),
        ..TestService::default()
    });

    set(&service.start_blocks_until_cancelled, true);
    let starting_service = Arc::clone(&service);
    let start = tokio::spawn(async move { starting_service.start().await });
    while service.state() != ServiceState::Starting {
        tokio::task::yield_now().await;
    }

    parent.cancel();
    let error = timeout(COMPLETION_TIMEOUT, start).await.unwrap().unwrap().unwrap_err();

    assert_eq!(error.to_string(), "service start was cancelled");
    assert_eq!(service.state(), ServiceState::Stopped);
    assert_eq!(service.on_stop_calls(), 1);
}

#[tokio::test]
async fn cancelled_parent_rejects_starts_while_running_and_after_stopping() {
    let parent = CancellationToken::new();
    let service = Arc::new(DefaultStopService {
        lifecycle: AsyncServiceLifecycleCore::with_parent(&parent),
    });

    service.start().await.unwrap();
    parent.cancel();

    let error = service.start().await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "service run was cancelled by its parent; stop it first"
    );

    service.stop().await.unwrap();
    let error = service.start().await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "cannot start service after its parent token was cancelled"
    );
    assert_eq!(service.lifecycle.state(), ServiceState::Stopped);
}

#[tokio::test]
async fn dropped_start_requires_a_stop_and_the_restart_gets_live_tokens() {
    let service = Arc::new(TestService::default());
    set(&service.start_blocks_until_cancelled, true);

    assert!(timeout(SHORT, service.start()).await.is_err());
    assert_eq!(service.state(), ServiceState::Stopping);
    assert!(service.last_token().is_cancelled());
    assert_eq!(
        service.start().await.unwrap_err().to_string(),
        "cannot start service before its previous run is stopped; stop it first"
    );

    service.stop().await.unwrap();
    assert_eq!(service.on_stop_calls(), 1);

    set(&service.start_blocks_until_cancelled, false);
    service.start().await.unwrap();
    assert_eq!(service.state(), ServiceState::Running);
    assert!(flag(&service.token_live_at_start));
}

#[tokio::test]
async fn stop_with_timeout_gives_up_on_stuck_tasks_and_the_next_stop_retries() {
    let service = Arc::new(TestService::default());
    set(&service.spawn_stuck_task, true);
    service.start().await.unwrap();

    let error = service.stop_with_timeout(SHORT).await.unwrap_err();

    assert_stop_timed_out(&error);
    assert_eq!(service.state(), ServiceState::Stopping);
    assert_eq!(service.on_stop_calls(), 0);

    service.abort_stuck_task();
    service.stop().await.unwrap();
    assert_eq!(service.state(), ServiceState::Stopped);
    assert_eq!(service.on_stop_calls(), 1);
}

#[tokio::test]
async fn stop_with_timeout_covers_waiting_for_a_start_in_progress() {
    let service = Arc::new(TestService::default());
    set(&service.start_blocks_until_cancelled, true);
    let starting_service = Arc::clone(&service);
    let start = tokio::spawn(async move { starting_service.start().await });
    while service.state() != ServiceState::Starting {
        tokio::task::yield_now().await;
    }

    assert_stop_timed_out(&service.stop_with_timeout(SHORT).await.unwrap_err());
    assert_eq!(service.state(), ServiceState::Starting);

    start.abort();
    let _ = start.await;
    service.stop().await.unwrap();
    assert_eq!(service.state(), ServiceState::Stopped);
}

#[tokio::test]
async fn on_stop_interrupted_by_a_timeout_runs_again_on_the_next_stop() {
    let service = Arc::new(TestService::default());
    service.start().await.unwrap();
    set(&service.on_stop_blocks, true);

    assert_stop_timed_out(&service.stop_with_timeout(SHORT).await.unwrap_err());
    assert_eq!(service.state(), ServiceState::Stopping);

    set(&service.on_stop_blocks, false);
    service.stop().await.unwrap();
    assert_eq!(service.on_stop_calls(), 2);
    assert_eq!(service.state(), ServiceState::Stopped);
}

#[tokio::test]
async fn failed_on_stop_reports_the_error_and_allows_a_restart() {
    let service = Arc::new(TestService::default());
    set(&service.on_stop_should_fail, true);
    service.start().await.unwrap();

    assert_eq!(service.stop().await.unwrap_err().to_string(), "cleanup failed");
    assert_eq!(service.state(), ServiceState::Stopped);

    service.start().await.unwrap();
    assert_eq!(service.state(), ServiceState::Running);
}

#[tokio::test]
async fn stop_with_timeout_reports_a_failing_on_stop() {
    let service = Arc::new(TestService::default());
    set(&service.on_stop_should_fail, true);
    service.start().await.unwrap();

    let error = service.stop_with_timeout(COMPLETION_TIMEOUT).await.unwrap_err();

    assert_eq!(error.to_string(), "cleanup failed");
    assert!(!error.is::<StopTimedOut>());
    assert_eq!(service.state(), ServiceState::Stopped);
}

#[tokio::test]
async fn stop_during_a_start_waits_for_it_and_then_stops_the_run() {
    let service = Arc::new(TestService::default());
    set(&service.start_waits_for_gate, true);
    let starting_service = Arc::clone(&service);
    let start = tokio::spawn(async move { starting_service.start().await });
    while service.state() != ServiceState::Starting {
        tokio::task::yield_now().await;
    }

    let mut stop = std::pin::pin!(service.stop());
    assert!(timeout(SHORT, &mut stop).await.is_err());
    assert_eq!(service.state(), ServiceState::Starting);

    service.start_gate.cancel();
    timeout(COMPLETION_TIMEOUT, start).await.unwrap().unwrap().unwrap();
    timeout(COMPLETION_TIMEOUT, stop).await.unwrap().unwrap();

    assert_eq!(service.state(), ServiceState::Stopped);
    assert_eq!(service.on_stop_calls(), 1);
    assert!(service.last_token().is_cancelled());
}

#[tokio::test]
async fn spawning_is_rejected_once_the_run_begins_stopping() {
    let service = Arc::new(TestService::default());
    set(&service.spawn_cooperative_task, true);
    service.start().await.unwrap();
    let stale_tasks = service.tasks.lock().unwrap().clone().unwrap();

    service.stop().await.unwrap();

    assert!(flag(&service.stopping_spawn_rejected));
    let error = stale_tasks.spawn(ready(())).unwrap_err();
    assert_eq!(
        error.to_string(),
        "cannot spawn into a service run that is stopping or stopped"
    );

    service.start().await.unwrap();
    let live_tasks = service.tasks.lock().unwrap().clone().unwrap();
    assert!(live_tasks.spawn(ready(())).is_ok());
}
