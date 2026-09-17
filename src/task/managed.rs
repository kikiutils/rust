use tokio::task::{
    JoinError,
    JoinHandle,
};
use tokio_util::sync::CancellationToken;

use super::TaskId;

#[derive(Debug)]
pub struct ManagedTask<T> {
    cancel_token: Option<CancellationToken>,
    handle: JoinHandle<T>,
    id: TaskId,
}

impl<T> ManagedTask<T> {
    pub(super) fn new(id: TaskId, handle: JoinHandle<T>, cancel_token: Option<CancellationToken>) -> Self {
        Self {
            cancel_token,
            handle,
            id,
        }
    }

    // Public methods
    pub fn abort(&self) {
        self.handle.abort();
    }

    pub fn cancel(&self) -> bool {
        self.cancel_token.as_ref().is_some_and(|cancel_token| {
            cancel_token.cancel();
            true
        })
    }

    pub fn id(&self) -> TaskId {
        self.id
    }

    pub fn into_join_handle(self) -> JoinHandle<T> {
        self.handle
    }

    pub async fn join(self) -> Result<T, JoinError> {
        self.handle.await
    }
}
