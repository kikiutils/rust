use std::fmt::Result as FmtResult;

use anyhow::{
    Context,
    Ok,
    Result,
    ensure,
};
use tracing_subscriber::{
    EnvFilter,
    Layer,
    Registry,
    field::RecordFields,
    fmt::{
        FormatFields,
        format::{
            DefaultFields,
            Writer,
        },
        layer,
    },
    layer::SubscriberExt,
    registry,
    util::SubscriberInitExt,
};

pub mod options;
mod queue;
mod timer;
mod worker;

use self::{
    options::LoggerInitOptions,
    timer::LoggerTimer,
    worker::{
        ConsoleSink,
        FileSink,
        LoggerWorkerGuard,
        spawn_worker,
    },
};

type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;

// Structs

/// [`DefaultFields`] under a distinct type, so the file layer caches its own ANSI-free span fields
/// instead of reusing the ones formatted by the console layer.
#[derive(Debug, Default)]
struct FileFields(DefaultFields);

impl<'writer> FormatFields<'writer> for FileFields {
    fn format_fields<R: RecordFields>(&self, writer: Writer<'writer>, fields: R) -> FmtResult {
        self.0.format_fields(writer, fields)
    }
}

/// Keeps non-blocking logger workers alive and provides explicit flush/shutdown controls.
///
/// Keep this guard for as long as logging should remain active. Dropping the guard flushes and
/// shuts down background workers, but drop cannot report shutdown errors; call [`LoggerGuard::shutdown`]
/// when shutdown failures should be surfaced.
#[derive(Debug)]
pub struct LoggerGuard {
    workers: Vec<LoggerWorkerGuard>,
}

impl Drop for LoggerGuard {
    fn drop(&mut self) {
        for worker in &mut self.workers {
            let _ = worker.shutdown();
        }
    }
}

impl LoggerGuard {
    // Public methods
    /// Returns how many log events were discarded because a queue was full or its worker had stopped.
    pub fn dropped_count(&self) -> u64 {
        self.workers.iter().map(LoggerWorkerGuard::dropped_count).sum()
    }

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

// Functions

/// Initializes the global tracing subscriber with non-blocking logger workers.
///
/// Console output goes to stdout or stderr. File output is split by event target: `a::b::c` is
/// written to `{base_dir}/a/b/c.log`, an empty target to `_default.log`, and targets whose file
/// cannot be opened to `_fallback.log`.
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
    ensure!(
        options.non_blocking.channel_capacity > 0,
        "logger channel capacity must be greater than 0"
    );

    // Parse every filter before spawning workers so invalid directives fail without side effects.
    let console = options
        .console_output
        .map(|console_options| {
            let filter = parse_filter(&console_options.filter).context("invalid console log filter")?;
            Ok((console_options, filter))
        })
        .transpose()?;

    let file = options
        .file_output
        .map(|file_options| {
            let filter = parse_filter(&file_options.filter).context("invalid file log filter")?;
            Ok((file_options, filter))
        })
        .transpose()?;

    let timer = LoggerTimer::new();
    let mut layers = Vec::<BoxedLayer>::new();
    let mut logger_guard = LoggerGuard { workers: Vec::new() };

    if let Some((console_options, filter)) = console {
        let (writer, worker) = spawn_worker(
            "kikiutils-logger-console",
            ConsoleSink::new(console_options.stream),
            options.non_blocking,
            false,
        )?;

        logger_guard.workers.push(worker);
        layers.push(
            layer()
                .with_timer(timer)
                .with_writer(writer)
                .with_ansi(console_options.ansi_enabled)
                .with_target(console_options.include_target)
                .with_filter(filter)
                .boxed(),
        );
    }

    if let Some((file_options, filter)) = file {
        let sink = FileSink::new(file_options.base_dir).context("failed to initialize file log writer")?;
        let (writer, worker) = spawn_worker("kikiutils-logger-file", sink, options.non_blocking, true)?;
        logger_guard.workers.push(worker);
        layers.push(
            layer()
                .fmt_fields(FileFields::default())
                .with_timer(timer)
                .with_writer(writer)
                .with_ansi(false)
                .with_target(true)
                .with_filter(filter)
                .boxed(),
        );
    }

    registry()
        .with(layers)
        .try_init()
        .context("failed to initialize tracing subscriber")?;

    Ok(logger_guard)
}

fn parse_filter(directives: &str) -> Result<EnvFilter> {
    EnvFilter::builder()
        .parse(directives)
        .with_context(|| format!("failed to parse filter directives: {directives}"))
}
