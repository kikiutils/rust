use std::{
    future::Future,
    sync::{
        Arc,
        Mutex,
        MutexGuard,
        PoisonError,
    },
    time::Duration,
};

use tokio::{
    task::JoinHandle,
    time::error::Elapsed,
};
use tokio_util::{
    sync::CancellationToken,
    task::TaskTracker,
};

// Structs

/// Tracks spawned tasks and hands each [`spawn_with_token`](Self::spawn_with_token) task a child of
/// one root token, so [`cancel`](Self::cancel) also reaches tasks spawned afterwards until
/// [`reopen`](Self::reopen).
///
/// Clones share the same tasks and tokens. Waiting from a task spawned by this manager never
/// completes, because the manager waits for that task too.
#[derive(Clone, Debug)]
pub struct TaskManager {
    /// Never cancelled unless it is the parent passed to [`with_parent`](Self::with_parent); every root
    /// is its child.
    base: CancellationToken,
    root: Arc<Mutex<CancellationToken>>,
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

    /// Spawns a task that cancels this manager when the process receives a shutdown signal, see
    /// [`wait_for_shutdown_signal`](crate::signal::wait_for_shutdown_signal).
    ///
    /// The task returns `true` if a signal cancelled the manager, and ends with `false` once the
    /// manager is cancelled otherwise; it does not follow a later [`reopen`](Self::reopen). The signal
    /// handlers are registered when the task first runs, not before this returns; if registering
    /// fails, the task returns the error without cancelling.
    ///
    /// Tokio keeps the handlers registered until the process exits, so after the task ends these
    /// signals no longer terminate the process and are ignored unless something else listens; call
    /// this again after a `reopen`.
    #[cfg(feature = "signal")]
    pub fn cancel_on_shutdown_signal(&self) -> JoinHandle<anyhow::Result<bool>> {
        let root = self.root().clone();
        self.spawn(async move {
            tokio::select! {
                biased;
                () = root.cancelled() => Ok(false),
                result = crate::signal::wait_for_shutdown_signal() => {
                    result?;
                    root.cancel();
                    Ok(true)
                },
            }
        })
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
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.tracker.spawn(future)
    }

    pub fn spawn_with_token<F, Fut>(&self, f: F) -> JoinHandle<Fut::Output>
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
            root: Arc::new(Mutex::new(root)),
            tracker,
        }
    }

    fn root(&self) -> MutexGuard<'_, CancellationToken> {
        // The guarded token stays valid even if a holder panicked.
        self.root.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
