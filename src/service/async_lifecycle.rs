use std::future::Future;

use anyhow::Result;
use tokio::sync::Mutex;

use super::state::ServiceState;
use crate::{
    atomic::enum_cell::AtomicEnumCell,
    task::manager::TaskManager,
};

pub trait AsyncServiceLifecycle: Send + Sync {
    fn lifecycle_lock(&self) -> &Mutex<()>;
    fn state(&self) -> &AtomicEnumCell<ServiceState>;
    fn task_manager(&self) -> &TaskManager;

    fn execute_start<Fut: Future<Output = Result<()>> + Send>(
        &self,
        future: Fut,
    ) -> impl Future<Output = Result<()>> + Send {
        async move {
            let _lock = self.lifecycle_lock().lock().await;

            match self.state().get() {
                ServiceState::Running | ServiceState::Starting | ServiceState::Stopping => return Ok(()),
                ServiceState::Stopped => self.state().store(ServiceState::Starting),
            }

            if let Err(error) = future.await {
                self.state().store(ServiceState::Stopped);
                self.task_manager().cancel_and_join_existing().await;
                return Err(error);
            }

            self.state().store(ServiceState::Running);
            Ok(())
        }
    }

    fn execute_stop<Fut: Future<Output = ()> + Send>(&self, future: Fut) -> impl Future<Output = ()> + Send {
        async move {
            let _lock = self.lifecycle_lock().lock().await;

            match self.state().get() {
                ServiceState::Stopped | ServiceState::Stopping => return,
                ServiceState::Running | ServiceState::Starting => self.state().store(ServiceState::Stopping),
            }

            self.task_manager().cancel_and_join_existing().await;
            future.await;
            self.state().store(ServiceState::Stopped);
        }
    }
}

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
