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

            if let Err(start_error) = start_future.await {
                // Preserve the startup error; cleanup failure is represented by the resulting state.
                let _ = cleanup_resources_and_join_tasks_and_set_state(self).await;
                return Err(start_error);
            }

            self.state().store(ServiceState::Running);
            Ok(())
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

            cleanup_resources_and_join_tasks_and_set_state(self).await
        }
    }
}

#[async_trait]
pub trait AsyncServiceLifecycleHooks: Send + Sync {
    async fn cleanup_resources(&self) -> Result<()>;
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
