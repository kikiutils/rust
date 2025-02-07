use anyhow::{Error, Result};
use tokio::signal;

pub async fn wait_for_shutdown_signal() -> Result<()> {
    let ctrl_c = async { Ok::<(), Error>(signal::ctrl_c().await?) };
    #[cfg(unix)]
    let sigterm = async {
        let mut term_signal = signal::unix::signal(signal::unix::SignalKind::terminate())?;
        term_signal.recv().await;
        return Ok::<(), Error>(());
    };

    #[cfg(unix)]
    tokio::select! {
        _ = ctrl_c => {},
        _ = sigterm => {},
    }

    #[cfg(not(unix))]
    ctrl_c.await?;
    return Ok(());
}
