use std::{
    env,
    fmt,
    io::{
        self,
        IsTerminal,
    },
};

use anyhow::{
    Context,
    Result,
};
use tracing::{
    Event,
    Metadata,
    Subscriber,
    level_filters::LevelFilter,
    span,
    subscriber::Interest,
};
use tracing_log::NormalizeEvent;
use tracing_subscriber::{
    EnvFilter,
    Layer,
    Registry,
    field::RecordFields,
    fmt::{
        FormatEvent,
        FormatFields,
        format::{
            self,
            DefaultFields,
            Writer,
        },
    },
    layer::{
        self,
        Filter,
        SubscriberExt,
    },
    registry,
    registry::LookupSpan,
    util::SubscriberInitExt,
};

pub mod options;
mod queue;
mod timer;
mod worker;

use self::{
    options::{
        LoggerAnsiMode,
        LoggerConsoleStream,
        LoggerInitOptions,
    },
    timer::LoggerTimer,
    worker::{
        ConsoleSink,
        FileSink,
        LoggerWorkerGuard,
        TARGET_SEPARATOR,
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
    fn format_fields<R: RecordFields>(&self, writer: Writer<'writer>, fields: R) -> fmt::Result {
        self.0.format_fields(writer, fields)
    }
}

/// Reports `Interest::always` callsites as `sometimes`, so every event re-runs the filter.
///
/// Works around tokio-rs/tracing#2519: an `enabled` check that no event follows (`tracing::enabled!`,
/// `log::log_enabled!`) leaves a stale per-layer filter state, and the next always-interest event on that
/// thread is skipped by this layer.
#[derive(Debug)]
struct AskEachTimeFilter(EnvFilter);

impl<S> Filter<S> for AskEachTimeFilter {
    fn enabled(&self, meta: &Metadata<'_>, cx: &layer::Context<'_, S>) -> bool {
        Filter::<S>::enabled(&self.0, meta, cx)
    }

    fn callsite_enabled(&self, meta: &'static Metadata<'static>) -> Interest {
        let interest = Filter::<S>::callsite_enabled(&self.0, meta);
        if interest.is_always() {
            Interest::sometimes()
        } else {
            interest
        }
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        Filter::<S>::max_level_hint(&self.0)
    }

    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: layer::Context<'_, S>) {
        Filter::<S>::on_new_span(&self.0, attrs, id, ctx);
    }

    fn on_record(&self, id: &span::Id, values: &span::Record<'_>, ctx: layer::Context<'_, S>) {
        Filter::<S>::on_record(&self.0, id, values, ctx);
    }

    fn on_enter(&self, id: &span::Id, ctx: layer::Context<'_, S>) {
        Filter::<S>::on_enter(&self.0, id, ctx);
    }

    fn on_exit(&self, id: &span::Id, ctx: layer::Context<'_, S>) {
        Filter::<S>::on_exit(&self.0, id, ctx);
    }

    fn on_close(&self, id: span::Id, ctx: layer::Context<'_, S>) {
        Filter::<S>::on_close(&self.0, id, ctx);
    }
}

/// Prefixes each formatted event with its target and [`TARGET_SEPARATOR`] so the file worker can route
/// it. The normalized target is used because events bridged from the `log` crate all share the `log`
/// callsite target.
#[derive(Debug)]
struct TargetPrefixedFormat<F>(F);

impl<S, N, F> FormatEvent<S, N> for TargetPrefixedFormat<F>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    F: FormatEvent<S, N>,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let normalized_metadata = event.normalized_metadata();
        let metadata = normalized_metadata.as_ref().unwrap_or_else(|| event.metadata());
        writer.write_str(metadata.target())?;
        writer.write_char(char::from(TARGET_SEPARATOR))?;
        self.0.format_event(ctx, writer, event)
    }
}

/// Dropping the guard flushes and shuts down background workers, but drop cannot report shutdown
/// errors; call [`LoggerGuard::shutdown`] when shutdown failures should be surfaced.
#[derive(Debug)]
#[must_use = "dropping the guard shuts down the logger workers"]
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
    /// Returns the total of events discarded by each output because its queue was full or its worker had
    /// stopped; an event dropped by both outputs counts twice.
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

    pub fn shutdown(mut self) -> Result<()> {
        for worker in &mut self.workers {
            worker.shutdown()?;
        }

        Ok(())
    }
}

// Functions

/// Console output goes to stdout or stderr. File output is split by event target: `a::b::c` is
/// written to `{base_dir}/a/b/c.log`, an empty target to `_default.log`, and targets whose file
/// cannot be opened to `_fallback.log`.
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
    // Parse every filter before spawning workers so invalid directives fail without side effects.
    let console = options
        .console_output
        .map(|console_options| {
            let filter = parse_filter(&console_options.filter).context("invalid console log filter")?;
            Ok::<_, anyhow::Error>((console_options, filter))
        })
        .transpose()?;

    let file = options
        .file_output
        .map(|file_options| {
            let filter = parse_filter(&file_options.filter).context("invalid file log filter")?;
            Ok::<_, anyhow::Error>((file_options, filter))
        })
        .transpose()?;

    let timer = LoggerTimer::new();
    let mut layers = Vec::<BoxedLayer>::new();
    let mut logger_guard = LoggerGuard { workers: Vec::new() };

    if let Some((console_options, filter)) = console {
        let (writer, worker) = spawn_worker(
            "kikiutils-logger-console",
            ConsoleSink::new(console_options.stream),
            options.queue,
            timer,
        )?;

        logger_guard.workers.push(worker);
        layers.push(
            tracing_subscriber::fmt::layer()
                .with_timer(timer)
                .with_writer(writer)
                .with_ansi(resolve_ansi(console_options.ansi, console_options.stream))
                .with_target(console_options.include_target)
                .with_filter(AskEachTimeFilter(filter))
                .boxed(),
        );
    }

    if let Some((file_options, filter)) = file {
        let sink = FileSink::new(file_options.base_dir).context("failed to initialize file log writer")?;
        let (writer, worker) = spawn_worker("kikiutils-logger-file", sink, options.queue, timer)?;
        logger_guard.workers.push(worker);
        layers.push(
            tracing_subscriber::fmt::layer()
                .fmt_fields(FileFields::default())
                .event_format(TargetPrefixedFormat(format::Format::default().with_timer(timer)))
                .with_writer(writer)
                .with_ansi(false)
                .with_filter(AskEachTimeFilter(filter))
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

fn resolve_ansi(mode: LoggerAnsiMode, stream: LoggerConsoleStream) -> bool {
    match mode {
        LoggerAnsiMode::Always => true,
        LoggerAnsiMode::Auto => {
            let is_terminal = match stream {
                LoggerConsoleStream::Stdout => io::stdout().is_terminal(),
                LoggerConsoleStream::Stderr => io::stderr().is_terminal(),
            };

            is_terminal && env::var_os("NO_COLOR").is_none_or(|value| value.is_empty())
        },
        LoggerAnsiMode::Never => false,
    }
}
