use dashmap::DashMap;
use futures::future::join_all;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::spawn;
use tokio::sync::oneshot::channel;
use tokio_util::sync::CancellationToken;

use super::internals::{ManagedTaskEntry, NotifyOnDrop};
use super::managed::ManagedTask;

enum DrainAction {
    Abort,
    Cancel,
    None,
}

pub struct TaskManager {
    entries: Arc<DashMap<u64, ManagedTaskEntry>>,
    next_id: AtomicU64,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            next_id: AtomicU64::new(0),
        }
    }

    // Private methods
    async fn drain_and_join_existing(&self, action: DrainAction) {
        let mut join_handles = Vec::new();
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

                join_handles.push(entry.handle);
            }
        }

        join_all(join_handles).await;
    }

    fn spawn_inner<F, T>(&self, future: F, token: Option<CancellationToken>) -> ManagedTask<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        // Generate task id
        let task_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Spawn tasks and insert them into the map
        let (tx, rx) = channel::<()>();
        let guard = NotifyOnDrop(Some(tx));
        let user_task = spawn(async move {
            let _guard = guard;
            future.await
        });

        let entries = self.entries.clone();
        let manager_task = spawn(async move {
            let _ = rx.await;
            entries.remove(&task_id);
        });

        self.entries.insert(
            task_id,
            ManagedTaskEntry {
                abort: user_task.abort_handle(),
                handle: manager_task,
                token: token.clone(),
            },
        );

        // Return task
        ManagedTask {
            id: task_id,
            handle: user_task,
            token,
        }
    }

    // Public methods
    #[inline]
    pub fn abort(&self, id: &u64) -> bool {
        self.entries.get(id).is_some_and(|kv| {
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
    pub fn cancel(&self, id: &u64) -> bool {
        self.entries.get(id).is_some_and(|kv| {
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

impl Default for TaskManager {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
