use std::{
    future::Future,
    sync::{
        Arc,
        MutexGuard,
        PoisonError,
    },
    time::Duration,
};

use anyhow::Result;
use tokio::time::error::Elapsed;
use tokio_util::{
    sync::CancellationToken,
    task::TaskTracker,
};

// Enums
#[cfg(feature = "signal")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShutdownOutcome {
    Completed,
    TimedOut,
    Forced(crate::signal::ShutdownSignal),
}

// Structs

/// Tracks spawned tasks and hands each [`spawn_with_token`](Self::spawn_with_token) task a child of
/// one root token, so [`cancel`](Self::cancel) also reaches tasks spawned afterwards until
/// [`reopen`](Self::reopen).
///
/// Clones share the same tasks and tokens. Waiting from a task spawned by this manager never
/// completes, because the manager waits for that task too.
#[derive(Debug, Clone)]
pub struct TaskManager {
    /// Never cancelled unless it is the parent passed to [`with_parent`](Self::with_parent); every root
    /// is its child.
    base: CancellationToken,
    root: Arc<std::sync::Mutex<CancellationToken>>,
    tracker: TaskTracker,
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::from_base(CancellationToken::new())
    }
}

impl TaskManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Cancelling `parent` cancels every task token, including tokens handed out after a
    /// [`reopen`](Self::reopen).
    pub fn with_parent(parent: &CancellationToken) -> Self {
        Self::from_base(parent.clone())
    }

    // Public methods
    pub fn cancel(&self) {
        self.root().cancel();
    }

    /// Waits until the manager is cancelled or the process receives a shutdown signal, and cancels it
    /// on a signal. Returns the signal, or `None` if the manager was cancelled otherwise. See
    /// [`wait_for_shutdown_signal`](crate::signal::wait_for_shutdown_signal) for how the handlers
    /// behave afterwards.
    #[cfg(feature = "signal")]
    pub async fn cancel_on_shutdown_signal(&self) -> std::io::Result<Option<crate::signal::ShutdownSignal>> {
        let root = self.root().clone();
        tokio::select! {
            biased;
            () = root.cancelled() => Ok(None),
            result = crate::signal::wait_for_shutdown_signal() => {
                let signal = result?;
                root.cancel();
                Ok(Some(signal))
            },
        }
    }

    /// Waits like [`cancel_on_shutdown_signal`](Self::cancel_on_shutdown_signal), then cancels the
    /// manager and waits up to `grace` for its tasks. A second shutdown signal during the grace period
    /// returns [`ShutdownOutcome::Forced`] at once; that signal does not terminate the process by
    /// itself, so the caller should exit. Failing to listen for the second signal is ignored. Remaining
    /// tasks keep running and stay tracked.
    ///
    /// On Windows, the system still ends the process a few seconds after
    /// [`CtrlClose`](crate::signal::ShutdownSignal::CtrlClose) or
    /// [`CtrlShutdown`](crate::signal::ShutdownSignal::CtrlShutdown), whatever `grace` is.
    ///
    /// # Errors
    ///
    /// Fails if the first wait cannot register the signal handlers.
    ///
    /// # Panics
    ///
    /// Panics if the runtime has no time driver.
    #[cfg(feature = "signal")]
    pub async fn shutdown_on_signal(&self, grace: Duration) -> std::io::Result<ShutdownOutcome> {
        self.cancel_on_shutdown_signal().await?;
        tokio::select! {
            // A signal that arrived before the deadline wins over a timeout seen in the same poll.
            biased;
            Ok(signal) = crate::signal::wait_for_shutdown_signal() => Ok(ShutdownOutcome::Forced(signal)),
            result = self.shutdown_with_timeout(grace) => Ok(match result {
                Ok(()) => ShutdownOutcome::Completed,
                Err(_) => ShutdownOutcome::TimedOut,
            }),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.root().is_cancelled()
    }

    pub fn is_empty(&self) -> bool {
        self.tracker.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tracker.len()
    }

    /// Hands out fresh tokens again after [`cancel`](Self::cancel); does nothing if the root token is
    /// not cancelled. Calling it while a shutdown is still waiting lets new tasks keep that shutdown
    /// waiting.
    pub fn reopen(&self) {
        let mut root = self.root();
        if root.is_cancelled() {
            *root = self.base.child_token();
        }
    }

    pub async fn shutdown(&self) {
        self.cancel();
        self.wait().await;
    }

    /// On timeout the remaining tasks keep running and stay tracked.
    ///
    /// # Panics
    ///
    /// Panics if the runtime has no time driver, before any task is cancelled.
    pub async fn shutdown_with_timeout(&self, timeout: Duration) -> Result<(), Elapsed> {
        // `timeout` arms its timer here, while `wait` only starts observing the tracker once polled,
        // after the cancel.
        let wait = tokio::time::timeout(timeout, self.wait());
        self.cancel();
        wait.await
    }

    /// The task gets no cancel signal, but [`wait`](Self::wait) and [`shutdown`](Self::shutdown) still
    /// wait for it.
    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.tracker.spawn(future)
    }

    /// Like [`spawn_with_token`](Self::spawn_with_token), but cancels the manager once the task ends
    /// for any reason, including returning early, panicking or being aborted. Tasks that ignore their
    /// token, or were spawned without one, keep running.
    pub fn spawn_critical<F, Fut>(&self, f: F) -> tokio::task::JoinHandle<Fut::Output>
    where
        F: FnOnce(CancellationToken) -> Fut,
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let root = self.root().clone();
        let future = f(root.child_token());
        let cancel_on_exit = root.drop_guard();
        self.spawn(async move {
            let _cancel_on_exit = cancel_on_exit;
            future.await
        })
    }

    pub fn spawn_with_token<F, Fut>(&self, f: F) -> tokio::task::JoinHandle<Fut::Output>
    where
        F: FnOnce(CancellationToken) -> Fut,
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        self.spawn(f(self.token()))
    }

    /// Returns a child of the current root, cancelled by [`cancel`](Self::cancel) or the parent; a later
    /// [`reopen`](Self::reopen) does not revive it.
    pub fn token(&self) -> CancellationToken {
        self.root().child_token()
    }

    /// Includes tasks spawned while waiting, but returns as soon as the manager is empty, even if a task
    /// is spawned right after.
    pub async fn wait(&self) {
        self.tracker.wait().await;
    }

    // Private methods
    fn from_base(base: CancellationToken) -> Self {
        let root = base.child_token();
        // A closed tracker still accepts spawns; staying closed makes `wait` mean "until empty".
        let tracker = TaskTracker::new();
        tracker.close();

        Self {
            base,
            root: Arc::new(std::sync::Mutex::new(root)),
            tracker,
        }
    }

    fn root(&self) -> MutexGuard<'_, CancellationToken> {
        // The guarded token stays valid even if a holder panicked.
        self.root.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
