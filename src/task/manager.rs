use std::{
    future::Future,
    sync::{
        atomic::{
            AtomicU64,
            Ordering,
        },
        Arc,
    },
};

use tokio::{
    spawn,
    task::AbortHandle,
};
use tokio_util::sync::CancellationToken;

use super::managed::ManagedTask;
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
    entries: Arc<FxDashMap<u64, ManagedTaskEntry>>,
    id: u64,
}

impl Drop for CleanupOnDrop {
    fn drop(&mut self) {
        self.completion.cancel();
        self.entries.remove(&self.id);
    }
}

struct ManagedTaskEntry {
    abort: AbortHandle,
    completion: CancellationToken,
    token: Option<CancellationToken>,
}

pub struct TaskManager {
    entries: Arc<FxDashMap<u64, ManagedTaskEntry>>,
    next_id: AtomicU64,
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
            next_id: AtomicU64::new(0),
        }
    }

    // Private methods
    async fn drain_and_join_existing(&self, action: DrainAction) {
        let mut completion_tokens = Vec::new();
        let keys = self.entries.iter().map(|kv| *kv.key()).collect::<Vec<_>>();
        for key in keys {
            if let Some((_, entry)) = self.entries.remove(&key) {
                match action {
                    DrainAction::Abort => entry.abort.abort(),
                    DrainAction::Cancel => {
                        if let Some(token) = &entry.token {
                            token.cancel();
                        }
                    }
                    DrainAction::None => {}
                }

                completion_tokens.push(entry.completion);
            }
        }

        for token in completion_tokens {
            token.cancelled().await;
        }
    }

    fn spawn_inner<F, T>(&self, future: F, token: Option<CancellationToken>) -> ManagedTask<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        // Generate task id
        let task_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Spawn the task behind a startup gate so cleanup cannot run before the entry exists.
        let entries = self.entries.clone();
        let start = CancellationToken::new();
        let completion = CancellationToken::new();
        let cleanup = CleanupOnDrop {
            entries: entries.clone(),
            id: task_id,
            completion: completion.clone(),
        };

        let task_start = start.clone();
        let user_task = spawn(async move {
            let _cleanup = cleanup;
            task_start.cancelled().await;
            future.await
        });

        self.entries.insert(
            task_id,
            ManagedTaskEntry {
                abort: user_task.abort_handle(),
                completion,
                token: token.clone(),
            },
        );

        start.cancel();

        // Return task
        ManagedTask {
            id: task_id,
            handle: user_task,
            token,
        }
    }

    // Public methods
    #[inline]
    pub fn abort(&self, id: u64) -> bool {
        self.entries.get(&id).is_some_and(|kv| {
            kv.abort.abort();
            true
        })
    }

    pub fn abort_existing(&self) {
        self.entries.iter().for_each(|kv| kv.abort.abort());
    }

    pub async fn abort_and_join_existing(&self) {
        self.drain_and_join_existing(DrainAction::Abort).await;
    }

    #[inline]
    pub fn cancel(&self, id: u64) -> bool {
        self.entries.get(&id).is_some_and(|kv| {
            kv.token.as_ref().is_some_and(|t| {
                t.cancel();
                true
            })
        })
    }

    pub fn cancel_existing(&self) {
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
