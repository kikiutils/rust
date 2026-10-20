use std::{
    future::Future,
    sync::Arc,
};

use tokio::{
    spawn,
    task::{
        AbortHandle,
        try_id,
    },
};
use tokio_util::sync::CancellationToken;

use super::{
    TaskId,
    managed::ManagedTask,
};
use crate::types::fx_collections::FxDashMap;

// Enums
enum DrainAction {
    Abort,
    Cancel,
    None,
}

// Structs

/// Lives inside the spawned future, so it runs when the task completes, panics, is aborted or is
/// dropped by runtime shutdown. Tokio sets the task ID while dropping a task's future, so
/// [`try_id`] identifies the task here.
struct CleanupOnDrop {
    completion: CancellationToken,
    entries: Arc<FxDashMap<TaskId, ManagedTaskEntry>>,
}

impl Drop for CleanupOnDrop {
    fn drop(&mut self) {
        // Cancel before removing: `spawn_inner` relies on this order to detect tasks that finished
        // before their entry was registered.
        self.completion.cancel();
        if let Some(id) = try_id() {
            self.entries.remove(&id);
        }
    }
}

#[derive(Debug)]
struct ManagedTaskEntry {
    abort: AbortHandle,
    cancel_token: Option<CancellationToken>,
    completion: CancellationToken,
}

#[derive(Debug)]
pub struct TaskManager {
    entries: Arc<FxDashMap<TaskId, ManagedTaskEntry>>,
}

impl Default for TaskManager {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(FxDashMap::default()),
        }
    }

    // Private methods
    /// Applies `action` to every registered task except the calling one and returns their completion
    /// tokens. Skipping the caller keeps a managed task from aborting or cancelling itself, and from
    /// waiting for itself forever.
    fn apply_to_existing(&self, action: &DrainAction) -> Vec<CancellationToken> {
        let current_id = try_id();
        let mut completions = Vec::with_capacity(self.entries.len());

        for entry in self.entries.iter() {
            if current_id == Some(*entry.key()) {
                continue;
            }

            match action {
                DrainAction::Abort => entry.abort.abort(),
                DrainAction::Cancel => {
                    if let Some(cancel_token) = &entry.cancel_token {
                        cancel_token.cancel();
                    }
                },
                DrainAction::None => {},
            }

            completions.push(entry.completion.clone());
        }

        completions
    }

    async fn drain_and_join_existing(&self, action: DrainAction) {
        let completions = self.apply_to_existing(&action);

        for completion in completions {
            completion.cancelled().await;
        }
    }

    fn spawn_inner<F, T>(&self, future: F, cancel_token: Option<CancellationToken>) -> ManagedTask<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let completion = CancellationToken::new();
        let cleanup = CleanupOnDrop {
            completion: completion.clone(),
            entries: Arc::clone(&self.entries),
        };

        let handle = spawn(async move {
            let _cleanup = cleanup;
            future.await
        });

        let task_id = handle.id();
        self.entries.insert(
            task_id,
            ManagedTaskEntry {
                abort: handle.abort_handle(),
                cancel_token: cancel_token.clone(),
                completion: completion.clone(),
            },
        );

        // The task may have finished before it was registered, in which case its cleanup found no
        // entry to remove. Cleanup cancels `completion` before removing, so either this check sees the
        // cancellation or the cleanup's removal runs after the insert above.
        if completion.is_cancelled() {
            self.entries.remove(&task_id);
        }

        ManagedTask::new(task_id, handle, cancel_token)
    }

    // Public methods
    #[inline]
    pub fn abort(&self, id: TaskId) -> bool {
        self.entries.get(&id).is_some_and(|kv| {
            kv.abort.abort();
            true
        })
    }

    /// Aborts every registered task.
    ///
    /// When called from a task managed by this manager, the calling task is skipped.
    pub fn abort_existing(&self) {
        self.apply_to_existing(&DrainAction::Abort);
    }

    /// Aborts every registered task and waits for them to finish.
    ///
    /// When called from a task managed by this manager, the calling task is skipped.
    pub async fn abort_and_join_existing(&self) {
        self.drain_and_join_existing(DrainAction::Abort).await;
    }

    #[inline]
    pub fn cancel(&self, id: TaskId) -> bool {
        self.entries.get(&id).is_some_and(|kv| {
            kv.cancel_token.as_ref().is_some_and(|t| {
                t.cancel();
                true
            })
        })
    }

    /// Cancels the token of every registered task.
    ///
    /// When called from a task managed by this manager, the calling task is skipped.
    pub fn cancel_existing(&self) {
        self.apply_to_existing(&DrainAction::Cancel);
    }

    /// Cancels the token of every registered task and waits for them to finish.
    ///
    /// When called from a task managed by this manager, the calling task is skipped.
    pub async fn cancel_and_join_existing(&self) {
        self.drain_and_join_existing(DrainAction::Cancel).await;
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Waits for the task with `id` to finish. Returns `false` if it is not registered or is the
    /// calling task itself, which would otherwise wait for itself forever.
    pub async fn join(&self, id: TaskId) -> bool {
        if try_id() == Some(id) {
            return false;
        }

        let completion = self.entries.get(&id).map(|entry| entry.completion.clone());
        let Some(completion) = completion else {
            return false;
        };

        completion.cancelled().await;
        true
    }

    /// Waits for every currently registered task to finish.
    ///
    /// When called from a task managed by this manager, the calling task is skipped.
    pub async fn join_existing(&self) {
        self.drain_and_join_existing(DrainAction::None).await;
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    pub fn spawn<F, T>(&self, future: F) -> ManagedTask<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.spawn_inner(future, None)
    }

    #[inline]
    pub fn spawn_with_token<F, Fut, T>(&self, f: F) -> ManagedTask<T>
    where
        F: FnOnce(CancellationToken) -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let cancel_token = CancellationToken::new();
        self.spawn_inner(f(cancel_token.clone()), Some(cancel_token))
    }
}
