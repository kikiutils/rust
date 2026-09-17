use std::{
    future::Future,
    sync::{
        Arc,
        OnceLock,
    },
};

use parking_lot::Mutex;
use tokio::{
    spawn,
    task::AbortHandle,
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
struct CleanupOnDrop {
    completion: CancellationToken,
    entries: Arc<FxDashMap<TaskId, ManagedTaskEntry>>,
    id: Arc<OnceLock<TaskId>>,
}

impl Drop for CleanupOnDrop {
    fn drop(&mut self) {
        self.completion.cancel();
        if let Some(id) = self.id.get() {
            self.entries.remove(id);
        }
    }
}

#[derive(Debug)]
struct ManagedTaskEntry {
    abort: AbortHandle,
    completion: CancellationToken,
    token: Option<CancellationToken>,
}

#[derive(Debug)]
pub struct TaskManager {
    entries: Arc<FxDashMap<TaskId, ManagedTaskEntry>>,
    registration_lock: Mutex<()>,
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
            registration_lock: Mutex::new(()),
        }
    }

    // Private methods
    fn collect_existing_completions(&self, action: &DrainAction) -> Vec<CancellationToken> {
        let _registration_lock = self.registration_lock.lock();
        let mut completions = Vec::with_capacity(self.entries.len());

        for entry in self.entries.iter() {
            match action {
                DrainAction::Abort => entry.abort.abort(),
                DrainAction::Cancel => {
                    if let Some(token) = &entry.token {
                        token.cancel();
                    }
                },
                DrainAction::None => {},
            }

            completions.push(entry.completion.clone());
        }

        completions
    }

    async fn drain_and_join_existing(&self, action: DrainAction) {
        let completions = self.collect_existing_completions(&action);

        for completion in completions {
            completion.cancelled().await;
        }
    }

    fn spawn_inner<F, T>(&self, future: F, token: Option<CancellationToken>) -> ManagedTask<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let _registration_lock = self.registration_lock.lock();

        let entries = self.entries.clone();
        let start = CancellationToken::new();
        let completion = CancellationToken::new();
        let task_id = Arc::new(OnceLock::new());
        let cleanup = CleanupOnDrop {
            completion: completion.clone(),
            entries: entries.clone(),
            id: task_id.clone(),
        };

        let task_start = start.clone();
        let handle = spawn(async move {
            let _cleanup = cleanup;
            task_start.cancelled().await;
            future.await
        });

        let task_id_value = handle.id();
        let _ = task_id.set(task_id_value);

        // The task cannot finish before this entry is registered because the startup gate is closed.
        // The ID is assigned by Tokio and is also available to callers through JoinHandle::id().
        self.entries.insert(
            task_id_value,
            ManagedTaskEntry {
                abort: handle.abort_handle(),
                completion,
                token: token.clone(),
            },
        );

        start.cancel();
        ManagedTask::new(task_id_value, handle, token)
    }

    // Public methods
    #[inline]
    pub fn abort(&self, id: TaskId) -> bool {
        self.entries.get(&id).is_some_and(|kv| {
            kv.abort.abort();
            true
        })
    }

    pub fn abort_existing(&self) {
        let _registration_lock = self.registration_lock.lock();
        self.entries.iter().for_each(|kv| kv.abort.abort());
    }

    pub async fn abort_and_join_existing(&self) {
        self.drain_and_join_existing(DrainAction::Abort).await;
    }

    #[inline]
    pub fn cancel(&self, id: TaskId) -> bool {
        self.entries.get(&id).is_some_and(|kv| {
            kv.token.as_ref().is_some_and(|t| {
                t.cancel();
                true
            })
        })
    }

    pub fn cancel_existing(&self) {
        let _registration_lock = self.registration_lock.lock();
        self.entries.iter().for_each(|kv| {
            if let Some(token) = &kv.token {
                token.cancel();
            }
        });
    }

    pub async fn cancel_and_join_existing(&self) {
        self.drain_and_join_existing(DrainAction::Cancel).await;
    }

    #[inline]
    pub fn has_tasks(&self) -> bool {
        !self.is_empty()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub async fn join(&self, id: TaskId) -> bool {
        let completion = self.entries.get(&id).map(|entry| entry.completion.clone());
        let Some(completion) = completion else {
            return false;
        };

        completion.cancelled().await;
        true
    }

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
        let token = CancellationToken::new();
        self.spawn_inner(f(token.clone()), Some(token))
    }

    #[inline]
    pub fn task_count(&self) -> usize {
        self.len()
    }
}
