use dashmap::DashMap;
use futures::future::join_all;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::spawn;
use tokio::sync::oneshot::{channel, Sender};
use tokio::task::{AbortHandle, JoinHandle};

// Notify on drop
struct NotifyOnDrop(Option<Sender<()>>);

impl Drop for NotifyOnDrop {
    fn drop(&mut self) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send(());
        }
    }
}

// Task manager
pub struct TaskManager {
    handles: Arc<DashMap<u64, (AbortHandle, JoinHandle<()>)>>,
    next_id: AtomicU64,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            handles: Arc::new(DashMap::new()),
            next_id: AtomicU64::new(0),
        }
    }

    // Private methods
    async fn drain_and_join_existing<F: Fn(&AbortHandle)>(&self, f: F) {
        let mut join_handles = Vec::new();
        let keys = self.handles.iter().map(|kv| *kv.key()).collect::<Vec<_>>();
        for key in keys {
            if let Some((_, (abort, handle))) = self.handles.remove(&key) {
                f(&abort);
                join_handles.push(handle);
            }
        }

        join_all(join_handles).await;
    }

    // Public methods
    pub fn abort_existing(&self) {
        self.handles.iter().for_each(|kv| kv.0.abort());
    }

    pub async fn abort_and_join_existing(&self) {
        self.drain_and_join_existing(|abort| abort.abort()).await;
    }

    pub fn has_tasks(&self) -> bool {
        !self.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }

    pub async fn join_existing(&self) {
        self.drain_and_join_existing(|_| {}).await;
    }

    pub fn spawn<F, T>(&self, future: F) -> JoinHandle<T>
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

        let handles = self.handles.clone();
        let manager_task = spawn(async move {
            let _ = rx.await;
            handles.remove(&task_id);
        });

        self.handles.insert(task_id, (user_task.abort_handle(), manager_task));

        // Return task
        user_task
    }
}
