use std::{
    error::Error,
    fmt,
    future::Future,
    mem,
    sync::{
        Arc,
        Mutex,
        MutexGuard,
        PoisonError,
    },
    time::Duration,
};

use anyhow::{
    Result,
    bail,
};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::state::ServiceState;
use crate::task::manager::TaskManager;

// Traits

/// A restartable background service; call it through [`AsyncServiceLifecycleExt`].
///
/// Every start gets a fresh set of tasks, and stopping cancels them, waits for them to exit and then
/// runs [`on_stop`](Self::on_stop). Starts and stops run one at a time, so a stop waits for a start in
/// progress; to interrupt a slow start, cancel the parent token passed to
/// [`AsyncServiceLifecycleCore::with_parent`].
///
/// A stop waits for the service's tasks, so never stop a service from one of its own tasks, and never
/// start or stop it from its own `on_start` or `on_stop`. If the `on_stop` of one service stops
/// another, the tasks of that other service must not wait for the first one to stop.
pub trait AsyncServiceLifecycle: Send + Sync {
    fn lifecycle(&self) -> &AsyncServiceLifecycleCore;

    /// If `tasks` is cancelled before this returns, e.g. by the parent, the start fails and rolls back
    /// even if this returns `Ok(())`; select on `tasks.token().cancelled_owned()` around slow startup
    /// work.
    fn on_start(self: &Arc<Self>, tasks: &AsyncServiceTasks) -> impl Future<Output = Result<()>> + Send;

    /// Releases the service's resources once every task of the run has exited, including after a
    /// failed start. Runs again if a stop is dropped or times out while it runs, so it must tolerate
    /// being repeated.
    fn on_stop(&self) -> impl Future<Output = Result<()>> + Send {
        async { Ok(()) }
    }
}

/// Implemented for every [`AsyncServiceLifecycle`], so the lifecycle state machine cannot be
/// overridden.
pub trait AsyncServiceLifecycleExt: AsyncServiceLifecycle {
    /// Does nothing if the service is already running, but fails if its run was cancelled by the
    /// parent; stop it first.
    fn start(self: &Arc<Self>) -> impl Future<Output = Result<()>> + Send {
        run_start(self)
    }

    fn stop(&self) -> impl Future<Output = Result<()>> + Send {
        run_stop(self)
    }

    /// Bounds the whole stop, including waiting for a start or stop in progress and `on_stop`, and fails
    /// with [`StopTimedOut`] when it runs out. If the stop had begun, the service stays
    /// [`ServiceState::Stopping`] and the next stop retries.
    ///
    /// # Panics
    ///
    /// Panics if the runtime has no time driver.
    fn stop_with_timeout(&self, timeout: Duration) -> impl Future<Output = Result<()>> + Send {
        async move {
            match tokio::time::timeout(timeout, run_stop(self)).await {
                Ok(result) => result,
                Err(_) => Err(StopTimedOut { timeout }.into()),
            }
        }
    }
}

impl<T: AsyncServiceLifecycle + ?Sized> AsyncServiceLifecycleExt for T {}

// Enums
#[derive(Debug)]
enum Phase {
    Running(AsyncServiceTasks),
    Starting(AsyncServiceTasks),
    Stopped,
    Stopping(AsyncServiceTasks),
}

impl Phase {
    fn into_stopping(self) -> Self {
        match self {
            Self::Running(tasks) | Self::Starting(tasks) | Self::Stopping(tasks) => Self::Stopping(tasks),
            Self::Stopped => Self::Stopped,
        }
    }

    fn state(&self) -> ServiceState {
        match self {
            Self::Running(_) => ServiceState::Running,
            Self::Starting(_) => ServiceState::Starting,
            Self::Stopped => ServiceState::Stopped,
            Self::Stopping(_) => ServiceState::Stopping,
        }
    }

    fn tasks(&self) -> Option<&AsyncServiceTasks> {
        match self {
            Self::Running(tasks) | Self::Starting(tasks) | Self::Stopping(tasks) => Some(tasks),
            Self::Stopped => None,
        }
    }
}

// Structs
#[derive(Debug)]
pub struct AsyncServiceLifecycleCore {
    lock: tokio::sync::Mutex<()>,
    parent: CancellationToken,
    phase: Mutex<Phase>,
}

impl Default for AsyncServiceLifecycleCore {
    fn default() -> Self {
        Self::with_parent(&CancellationToken::new())
    }
}

impl AsyncServiceLifecycleCore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Cancelling `parent` cancels the current run, including a start in progress, and rejects later
    /// starts.
    pub fn with_parent(parent: &CancellationToken) -> Self {
        Self {
            lock: tokio::sync::Mutex::new(()),
            parent: parent.clone(),
            phase: Mutex::new(Phase::Stopped),
        }
    }

    // Public methods
    pub fn state(&self) -> ServiceState {
        self.phase().state()
    }

    // Private methods
    fn phase(&self) -> MutexGuard<'_, Phase> {
        // The guarded value stays valid even if a holder panicked.
        self.phase.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn set_phase(&self, phase: Phase) {
        *self.phase() = phase;
    }

    /// Closes the run to new tasks in the same step, so nothing can join it once it is seen stopping.
    fn set_stopping(&self) {
        let mut phase = self.phase();
        *phase = mem::replace(&mut *phase, Phase::Stopped).into_stopping();
        if let Some(tasks) = phase.tasks() {
            tasks.close();
        }
    }
}

/// Clones spawn into the same run. Spawning fails with [`TasksClosed`] once the run begins stopping,
/// so no task can start while the stop waits for the others or `on_stop` releases resources.
#[derive(Clone, Debug)]
pub struct AsyncServiceTasks {
    manager: TaskManager,
    open: Arc<Mutex<bool>>,
}

impl AsyncServiceTasks {
    fn new(parent: &CancellationToken) -> Self {
        Self {
            manager: TaskManager::with_parent(parent),
            open: Arc::new(Mutex::new(true)),
        }
    }

    // Public methods
    pub fn is_cancelled(&self) -> bool {
        self.manager.is_cancelled()
    }

    pub fn spawn<F>(&self, future: F) -> Result<JoinHandle<F::Output>, TasksClosed>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        // Spawning under the lock makes every admitted task visible to the stop that closes it.
        let open = self.open();
        if !*open {
            return Err(TasksClosed);
        }

        Ok(self.manager.spawn(future))
    }

    /// `f` runs even if spawning then fails; the future it returns is dropped unpolled.
    pub fn spawn_with_token<F, Fut>(&self, f: F) -> Result<JoinHandle<Fut::Output>, TasksClosed>
    where
        F: FnOnce(CancellationToken) -> Fut,
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        self.spawn(f(self.token()))
    }

    /// Cancelled when the run is stopped or the parent is cancelled.
    pub fn token(&self) -> CancellationToken {
        self.manager.token()
    }

    // Private methods
    fn close(&self) {
        *self.open() = false;
    }

    fn open(&self) -> MutexGuard<'_, bool> {
        self.open.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Leaves the service [`ServiceState::Stopping`] with its run cancelled if the start future is dropped
/// before `on_start` finishes, so the next stop still waits for the tasks and runs `on_stop`.
struct DroppedStartGuard<'a> {
    armed: bool,
    core: &'a AsyncServiceLifecycleCore,
    tasks: &'a AsyncServiceTasks,
}

impl Drop for DroppedStartGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.tasks.manager.cancel();
            self.core.set_stopping();
        }
    }
}

impl DroppedStartGuard<'_> {
    fn disarm(mut self) {
        self.armed = false;
    }
}

/// Returned by [`AsyncServiceLifecycleExt::stop_with_timeout`] inside the [`anyhow::Error`], so
/// callers can tell a timeout apart from a failing `on_stop`.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct StopTimedOut {
    pub timeout: Duration,
}

impl fmt::Display for StopTimedOut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "service did not stop within {:?}; stop it again to retry",
            self.timeout
        )
    }
}

impl Error for StopTimedOut {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct TasksClosed;

impl fmt::Display for TasksClosed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("cannot spawn into a service run that is stopping or stopped")
    }
}

impl Error for TasksClosed {}

// Functions

/// Requires the lifecycle lock and [`ServiceState::Stopping`]. The run is only released once
/// `on_stop` has completed, so a stop that is dropped midway is retried by the next one.
async fn finish_stop<T: AsyncServiceLifecycle + ?Sized>(service: &T) -> Result<()> {
    let core = service.lifecycle();
    let Some(tasks) = core.phase().tasks().cloned() else {
        return Ok(());
    };

    tasks.manager.shutdown().await;
    let result = service.on_stop().await;
    core.set_phase(Phase::Stopped);
    result
}

async fn run_start<T: AsyncServiceLifecycle + ?Sized>(service: &Arc<T>) -> Result<()> {
    let core = service.lifecycle();
    let _lock = core.lock.lock().await;

    let tasks = {
        let mut phase = core.phase();
        match &*phase {
            Phase::Running(tasks) => {
                if tasks.is_cancelled() {
                    bail!("service run was cancelled by its parent; stop it first");
                }

                return Ok(());
            },
            Phase::Starting(_) | Phase::Stopping(_) => {
                bail!("cannot start service before its previous run is stopped; stop it first");
            },
            Phase::Stopped => {},
        }

        let tasks = AsyncServiceTasks::new(&core.parent);
        if tasks.is_cancelled() {
            bail!("cannot start service after its parent token was cancelled");
        }

        *phase = Phase::Starting(tasks.clone());
        tasks
    };

    let guard = DroppedStartGuard {
        armed: true,
        core,
        tasks: &tasks,
    };

    let result = service.on_start(&tasks).await;
    guard.disarm();
    if result.is_ok() && !tasks.is_cancelled() {
        core.set_phase(Phase::Running(tasks));
        return Ok(());
    }

    core.set_stopping();
    // Preserve the startup error; a failing `on_stop` during the rollback is not reported here.
    let _ = finish_stop(service.as_ref()).await;
    match result {
        Ok(()) => bail!("service start was cancelled"),
        Err(start_error) => Err(start_error),
    }
}

async fn run_stop<T: AsyncServiceLifecycle + ?Sized>(service: &T) -> Result<()> {
    let core = service.lifecycle();
    let _lock = core.lock.lock().await;
    core.set_stopping();
    finish_stop(service).await
}
