use anyhow::{
    Context,
    Result,
};
use time::{
    UtcOffset,
    format_description::FormatItem,
    macros::format_description,
};
use tracing_subscriber::{
    Layer,
    Registry,
    fmt::{
        layer,
        time::OffsetTime,
    },
    layer::SubscriberExt,
    registry,
    util::SubscriberInitExt,
};

mod file;
pub mod options;
mod target;
mod worker;

use self::{
    options::LoggerInitOptions,
    worker::{
        LoggerWorkerGuard,
        NonBlockingConsoleWriter,
        NonBlockingFileWriter,
        validate_worker_options,
    },
};

type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;

// Structs

/// Keeps non-blocking logger workers alive and provides explicit flush/shutdown controls.
///
/// Keep this guard for as long as logging should remain active. Dropping the guard flushes and
/// shuts down background workers, but drop cannot report shutdown errors; call [`LoggerGuard::shutdown`]
/// when shutdown failures should be surfaced.
#[derive(Debug)]
pub struct LoggerGuard {
    workers: Vec<LoggerWorkerGuard>,
}

impl LoggerGuard {
    fn new(workers: Vec<LoggerWorkerGuard>) -> Self {
        Self { workers }
    }

    // Public methods
    /// Waits until all log messages queued before this call have been written by the background workers.
    pub fn flush(&self) -> Result<()> {
        for worker in &self.workers {
            worker.flush()?;
        }

        Ok(())
    }

    /// Flushes queued log messages, stops background workers, and reports shutdown errors.
    pub fn shutdown(mut self) -> Result<()> {
        for worker in &mut self.workers {
            worker.shutdown()?;
        }

        Ok(())
    }
}

impl Drop for LoggerGuard {
    fn drop(&mut self) {
        for worker in &mut self.workers {
            let _ = worker.shutdown();
        }
    }
}

// Functions

/// Initializes the global tracing subscriber with non-blocking logger workers.
///
/// The returned [`LoggerGuard`] must be kept alive for logging to continue. If it is dropped,
/// queued messages are flushed and the background workers are shut down.
///
/// # Example
///
/// ```no_run
/// use kikiutils::logger::{
///     init_logger,
///     options::LoggerInitOptions,
/// };
///
/// # fn main() -> anyhow::Result<()> {
/// let logger_guard = init_logger(LoggerInitOptions::default())?;
/// tracing::info!("application started");
/// logger_guard.shutdown()?;
/// # Ok(())
/// # }
/// ```
pub fn init_logger(options: LoggerInitOptions) -> Result<LoggerGuard> {
    validate_worker_options(options.non_blocking)?;

    let timer = make_timer();
    let mut layers = Vec::<BoxedLayer>::new();
    let mut workers = Vec::new();

    if let Some(console_options) = options.console_output {
        let (console_writer, console_guard) = NonBlockingConsoleWriter::spawn(options.non_blocking)?;
        workers.push(console_guard);
        layers.push(
            layer()
                .with_timer(timer.clone())
                .with_writer(console_writer)
                .with_ansi(console_options.ansi_enabled)
                .with_target(console_options.include_target)
                .with_filter(console_options.level.as_level_filter())
                .boxed(),
        );
    }

    if let Some(file_options) = options.file_output {
        let level = file_options.level;
        let (file_writer, file_guard) = NonBlockingFileWriter::spawn(file_options, options.non_blocking)?;
        workers.push(file_guard);
        layers.push(
            layer()
                .with_timer(timer)
                .with_writer(file_writer)
                .with_ansi(false)
                .with_target(true)
                .with_filter(level.as_level_filter())
                .boxed(),
        );
    }

    let logger_guard = LoggerGuard::new(workers);
    if let Err(error) = registry().with(layers).try_init() {
        drop(logger_guard);
        return Err(error).context("failed to initialize tracing subscriber");
    }

    Ok(logger_guard)
}

fn make_timer() -> OffsetTime<Vec<FormatItem<'static>>> {
    let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    OffsetTime::new(
        local_offset,
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]").to_vec(),
    )
}
