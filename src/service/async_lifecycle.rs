use std::future::Future;

use anyhow::{
    Result,
    bail,
};
use async_trait::async_trait;
use tokio::{
    join,
    sync::Mutex,
};

use super::state::ServiceState;
use crate::{
    atomic::enum_cell::AtomicEnumCell,
    task::manager::TaskManager,
};

pub trait AsyncServiceLifecycle: AsyncServiceLifecycleHooks + Send + Sync {
    fn lifecycle_lock(&self) -> &Mutex<()>;

    fn state(&self) -> &AtomicEnumCell<ServiceState>;

    fn task_manager(&self) -> &TaskManager;

    fn execute_start<Fut: Future<Output = Result<()>> + Send>(
        &self,
        start_future: Fut,
    ) -> impl Future<Output = Result<()>> + Send {
        async {
            let _lifecycle_lock = self.lifecycle_lock().lock().await;

            match self.state().get() {
                ServiceState::CleanupFailed => bail!("cannot start service after cleanup failed; retry cleanup first"),
                ServiceState::Running | ServiceState::Starting | ServiceState::Stopping => return Ok(()),
                ServiceState::Stopped => self.state().store(ServiceState::Starting),
            }

            let cancel_guard = CleanupFailedOnCancel::new(self.state());
            let result = match start_future.await {
                Ok(()) => {
                    self.state().store(ServiceState::Running);
                    Ok(())
                },
                Err(start_error) => {
                    // Preserve the startup error; cleanup failure is represented by the resulting state.
                    let _ = cleanup_resources_and_join_tasks_and_set_state(self).await;
                    Err(start_error)
                },
            };

            cancel_guard.disarm();
            result
        }
    }

    fn execute_stop(&self) -> impl Future<Output = Result<()>> + Send {
        async {
            let _lifecycle_lock = self.lifecycle_lock().lock().await;

            match self.state().get() {
                ServiceState::Stopped | ServiceState::Stopping => return Ok(()),
                ServiceState::Running | ServiceState::Starting | ServiceState::CleanupFailed => {
                    self.state().store(ServiceState::Stopping);
                },
            }

            let cancel_guard = CleanupFailedOnCancel::new(self.state());
            let result = cleanup_resources_and_join_tasks_and_set_state(self).await;
            cancel_guard.disarm();
            result
        }
    }
}

#[async_trait]
pub trait AsyncServiceLifecycleHooks: Send + Sync {
    async fn cleanup_resources(&self) -> Result<()>;
}

// Structs

/// Marks the service as [`ServiceState::CleanupFailed`] if a start or stop future is dropped before it
/// finishes, e.g. by a timeout or `select!`. Otherwise the state would stay `Starting` or `Stopping`,
/// and later calls would silently return `Ok(())` while tasks and resources were left behind. From
/// `CleanupFailed`, the next stop runs cleanup again and start is rejected until it succeeds.
struct CleanupFailedOnCancel<'a> {
    armed: bool,
    state: &'a AtomicEnumCell<ServiceState>,
}

impl Drop for CleanupFailedOnCancel<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.state.store(ServiceState::CleanupFailed);
        }
    }
}

impl<'a> CleanupFailedOnCancel<'a> {
    fn new(state: &'a AtomicEnumCell<ServiceState>) -> Self {
        Self { armed: true, state }
    }

    fn disarm(mut self) {
        self.armed = false;
    }
}

// Functions
async fn cleanup_resources_and_join_tasks_and_set_state<T: AsyncServiceLifecycle + ?Sized>(service: &T) -> Result<()> {
    service.task_manager().cancel_existing();
    let (cleanup_result, ()) = join!(service.cleanup_resources(), service.task_manager().join_existing());
    let cleanup_state = match cleanup_result {
        Ok(()) => ServiceState::Stopped,
        Err(_) => ServiceState::CleanupFailed,
    };

    service.state().store(cleanup_state);
    cleanup_result
}

// Macros
#[macro_export]
macro_rules! impl_async_service_lifecycle {
    ($($t:ty),+ $(,)?) => {
        $(
            impl $crate::service::async_lifecycle::AsyncServiceLifecycle for $t {
                #[inline]
                fn lifecycle_lock(&self) -> &::tokio::sync::Mutex<()> {
                    &self.lifecycle_lock
                }

                #[inline]
                fn state(
                    &self,
                ) -> &$crate::atomic::enum_cell::AtomicEnumCell<$crate::service::state::ServiceState> {
                    &self.state
                }

                #[inline]
                fn task_manager(&self) -> &$crate::task::manager::TaskManager {
                    &self.task_manager
                }
            }
        )*
    };
}
