use dashmap::DashMap;
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
