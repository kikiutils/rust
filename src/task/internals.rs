use tokio::sync::oneshot::Sender;
use tokio::task::{AbortHandle, JoinHandle};
use tokio_util::sync::CancellationToken;

// NotifyOnDrop
pub struct NotifyOnDrop(pub Option<Sender<()>>);

impl Drop for NotifyOnDrop {
    fn drop(&mut self) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send(());
        }
    }
}

// ManagedTaskEntry
pub struct ManagedTaskEntry {
    pub abort: AbortHandle,
    pub handle: JoinHandle<()>,
    pub token: Option<CancellationToken>,
}
