use dashmap::DashMap;
use futures::FutureExt;
use rand::random;
use std::any::Any;
use std::boxed::Box;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::task::{AbortHandle, JoinHandle};

static TASK_ABORT_HANDLES: LazyLock<DashMap<String, AbortHandle>> = LazyLock::new(DashMap::new);

pub fn abort_all_managed_tasks() {
    TASK_ABORT_HANDLES.iter().for_each(|entry| entry.value().abort());
    TASK_ABORT_HANDLES.clear();
}

pub fn spawn_managed_task<F, T>(future: F) -> JoinHandle<Result<T, Box<dyn Any + Send>>>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let task_abort_handle_id = format!("{:x}{:x}", nanos, random::<u16>());
    let task_abort_handle_id_clone = task_abort_handle_id.clone();
    let task = tokio::spawn(async move {
        let result = AssertUnwindSafe(future).catch_unwind().await;
        TASK_ABORT_HANDLES.remove(&task_abort_handle_id_clone);
        return result;
    });

    TASK_ABORT_HANDLES.insert(task_abort_handle_id, task.abort_handle());
    return task;
}
