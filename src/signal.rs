use anyhow::Result;
use tokio::{
    select,
    signal::ctrl_c,
};

/// Waits until the process receives a shutdown signal.
///
/// Listens for SIGINT and SIGTERM on Unix, and for Ctrl+C, Ctrl+Break, console close and system
/// shutdown on Windows. Returns an error if a signal handler cannot be registered.
pub async fn wait_for_shutdown_signal() -> Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{
            SignalKind,
            signal,
        };

        let mut sigterm = signal(SignalKind::terminate())?;
        select! {
            result = ctrl_c() => result?,
            _ = sigterm.recv() => {},
        }
    }

    #[cfg(windows)]
    {
        use tokio::signal::windows::{
            ctrl_break,
            ctrl_close,
            ctrl_shutdown,
        };

        let mut ctrl_break = ctrl_break()?;
        let mut ctrl_close = ctrl_close()?;
        let mut ctrl_shutdown = ctrl_shutdown()?;
        select! {
            result = ctrl_c() => result?,
            _ = ctrl_break.recv() => {},
            _ = ctrl_close.recv() => {},
            _ = ctrl_shutdown.recv() => {},
        }
    }

    Ok(())
}
