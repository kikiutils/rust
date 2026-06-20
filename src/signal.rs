use std::future::Future;

use anyhow::{
    Error,
    Result,
};
use tokio::signal::ctrl_c;

pub async fn wait_for_shutdown_signal() -> Result<()> {
    let ctrl_c = async { Ok::<(), Error>(ctrl_c().await?) };

    #[cfg(unix)]
    {
        use tokio::signal::unix::{
            SignalKind,
            signal,
        };

        let sigterm = async {
            let mut term_signal = signal(SignalKind::terminate())?;
            term_signal.recv().await;
            Ok::<(), Error>(())
        };

        wait_for_shutdown_signal_from(ctrl_c, sigterm).await
    }

    #[cfg(not(unix))]
    wait_for_shutdown_signal_from(ctrl_c).await
}

#[cfg(unix)]
async fn wait_for_shutdown_signal_from<CtrlC, Sigterm>(ctrl_c: CtrlC, sigterm: Sigterm) -> Result<()>
where
    CtrlC: Future<Output = Result<()>>,
    Sigterm: Future<Output = Result<()>>,
{
    tokio::select! {
        result = ctrl_c => result?,
        result = sigterm => result?,
    };

    Ok(())
}

#[cfg(not(unix))]
async fn wait_for_shutdown_signal_from<CtrlC>(ctrl_c: CtrlC) -> Result<()>
where
    CtrlC: Future<Output = Result<()>>,
{
    ctrl_c.await
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::future::pending;

    use anyhow::anyhow;

    use super::*;

    #[tokio::test]
    async fn returns_when_ctrl_c_future_completes() {
        #[cfg(unix)]
        let result = wait_for_shutdown_signal_from(async { Ok(()) }, pending()).await;

        #[cfg(not(unix))]
        let result = wait_for_shutdown_signal_from(async { Ok(()) }).await;

        assert!(result.is_ok());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn returns_when_sigterm_future_completes() {
        let result = wait_for_shutdown_signal_from(pending(), async { Ok(()) }).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn returns_ctrl_c_error() {
        #[cfg(unix)]
        let result = wait_for_shutdown_signal_from(async { Err(anyhow!("ctrl-c failed")) }, pending()).await;

        #[cfg(not(unix))]
        let result = wait_for_shutdown_signal_from(async { Err(anyhow!("ctrl-c failed")) }).await;

        match result {
            Ok(()) => panic!("ctrl-c errors should propagate"),
            Err(error) => assert_eq!(error.to_string(), "ctrl-c failed"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn returns_sigterm_error() {
        let result = wait_for_shutdown_signal_from(pending(), async { Err(anyhow!("sigterm failed")) }).await;

        match result {
            Ok(()) => panic!("sigterm errors should propagate"),
            Err(error) => assert_eq!(error.to_string(), "sigterm failed"),
        }
    }
}
