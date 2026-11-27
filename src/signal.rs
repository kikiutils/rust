use std::{
    fmt,
    io,
};

use tokio::{
    select,
    signal::ctrl_c,
};

// Enums
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShutdownSignal {
    /// SIGINT on Unix, Ctrl+C on Windows.
    Interrupt,
    /// SIGTERM, Unix only.
    Terminate,
    /// Windows only.
    CtrlBreak,
    /// The console window was closed, Windows only.
    CtrlClose,
    /// Windows only, and only delivered to services.
    CtrlShutdown,
}

impl fmt::Display for ShutdownSignal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Interrupt if cfg!(windows) => "Ctrl+C",
            Self::Interrupt => "SIGINT",
            Self::Terminate => "SIGTERM",
            Self::CtrlBreak => "Ctrl+Break",
            Self::CtrlClose => "Ctrl+Close",
            Self::CtrlShutdown => "Ctrl+Shutdown",
        })
    }
}

// Functions

/// Registers the handlers when first polled, and fails if one cannot be registered.
///
/// On Unix, Tokio keeps the handlers registered until the process exits, so once this returns these
/// signals no longer terminate the process and are ignored unless something else listens.
pub async fn wait_for_shutdown_signal() -> io::Result<ShutdownSignal> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{
            SignalKind,
            signal,
        };

        let mut sigterm = signal(SignalKind::terminate())?;
        select! {
            result = ctrl_c() => result.map(|()| ShutdownSignal::Interrupt),
            _ = sigterm.recv() => Ok(ShutdownSignal::Terminate),
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
            result = ctrl_c() => result.map(|()| ShutdownSignal::Interrupt),
            _ = ctrl_break.recv() => Ok(ShutdownSignal::CtrlBreak),
            _ = ctrl_close.recv() => Ok(ShutdownSignal::CtrlClose),
            _ = ctrl_shutdown.recv() => Ok(ShutdownSignal::CtrlShutdown),
        }
    }
}
