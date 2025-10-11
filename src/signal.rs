use anyhow::{
    Error,
    Result,
};
use tokio::signal::ctrl_c;

pub async fn wait_for_shutdown_signal() -> Result<()> {
    let ctrl_c = async { Ok::<(), Error>(ctrl_c().await?) };

    #[cfg(unix)]
    {
        use tokio::{
            select,
            signal::unix::{
                signal,
                SignalKind,
            },
        };

        let sigterm = async {
            let mut term_signal = signal(SignalKind::terminate())?;
            term_signal.recv().await;
            Ok::<(), Error>(())
        };

        select! {
            _ = ctrl_c => {},
            _ = sigterm => {},
        }
    }

    #[cfg(not(unix))]
    ctrl_c.await?;

    Ok(())
}
