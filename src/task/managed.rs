use tokio::task::{
    JoinError,
    JoinHandle,
};
use tokio_util::sync::CancellationToken;

pub struct ManagedTask<T> {
    pub(in crate::task) id: u64,
    pub(in crate::task) handle: JoinHandle<T>,
    pub(in crate::task) token: Option<CancellationToken>,
}

impl<T> ManagedTask<T> {
    pub fn abort(&self) {
        self.handle.abort();
    }

    pub fn cancel(&self) -> bool {
        self.token.as_ref().is_some_and(|t| {
            t.cancel();
            true
        })
    }

    pub fn id(&self) -> &u64 {
        &self.id
    }

    pub fn into_handle(self) -> JoinHandle<T> {
        self.handle
    }

    pub async fn join(self) -> Result<T, JoinError> {
        self.handle.await
    }
}
