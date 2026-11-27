use std::{
    future::Future,
    sync::{
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
/// Waiting from a task spawned by this manager never completes, because the manager waits for that
/// task too.
#[derive(Debug)]
pub struct TaskManager {
    /// Never cancelled unless it is the parent passed to [`with_parent`](Self::with_parent); every root
    /// is its child.
    base: CancellationToken,
    root: Mutex<CancellationToken>,
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
        let cancel_token = self.root().child_token();
        self.tracker.spawn(f(cancel_token))
    }

    /// Returns a child of the current root, cancelled by [`cancel`](Self::cancel) or the parent; a later
    /// [`reopen`](Self::reopen) does not revive it.
    pub fn token(&self) -> CancellationToken {
        self.root().child_token()
    }

    /// Waits until no task is running, including tasks spawned while waiting; it returns once the manager
    /// is empty, even if a task is spawned right after.
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
            root: Mutex::new(root),
            tracker,
        }
    }

    fn root(&self) -> MutexGuard<'_, CancellationToken> {
        // The guarded token stays valid even if a holder panicked.
        self.root.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
