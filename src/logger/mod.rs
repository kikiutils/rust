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
    bail,
};
use tracing::{
    Event,
    Metadata,
    Subscriber,
    dispatcher,
    level_filters::LevelFilter,
    span,
    subscriber::{
        Interest,
        NoSubscriber,
    },
};
use tracing_log::{
    AsLog,
    LogTracer,
    NormalizeEvent,
};
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
};

pub mod options;
mod queue;
mod sink;
mod timer;
mod worker;

use self::{
    options::{
        LoggerAnsiMode,
        LoggerConsoleStream,
        LoggerInitOptions,
        LoggerQueueOptions,
    },
    sink::{
        ConsoleSink,
        FileSink,
        TARGET_SEPARATOR,
    },
    timer::LoggerTimer,
    worker::{
        LoggerWorkerGuard,
        spawn_worker,
    },
};

type BoxedLayer<S> = Box<dyn Layer<S> + Send + Sync + 'static>;

// Structs

/// [`DefaultFields`] under a distinct type per output. Formatted span fields are cached by formatter type,
/// so a shared type would let one layer reuse fields formatted with another layer's ANSI setting,
/// including a `fmt` layer composed with [`build_logger`].
#[derive(Debug, Default)]
struct ConsoleFields(DefaultFields);

impl<'writer> FormatFields<'writer> for ConsoleFields {
    fn format_fields<R: RecordFields>(&self, writer: Writer<'writer>, fields: R) -> fmt::Result {
        self.0.format_fields(writer, fields)
    }
}

/// See [`ConsoleFields`].
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
    ///
    /// # Errors
    ///
    /// Fails if a worker thread has panicked.
    pub fn flush(&self) -> Result<()> {
        for worker in &self.workers {
            worker.flush()?;
        }

        Ok(())
    }

    /// # Errors
    ///
    /// Fails at the first worker thread that panicked; the remaining workers are still shut down when the
    /// guard drops.
    pub fn shutdown(mut self) -> Result<()> {
        for worker in &mut self.workers {
            worker.shutdown()?;
        }

        Ok(())
    }
}

// Functions

/// Builds the logger layer and spawns its workers without installing a subscriber.
///
/// The layer works in any subscriber built on [`Registry`], since its outputs use per-layer filters.
/// Installing the subscriber is up to the caller:
/// [`SubscriberInitExt::try_init`](tracing_subscriber::util::SubscriberInitExt::try_init) also installs the
/// `log` bridge, so events from the `log` crate keep reaching the logger, while
/// `tracing::subscriber::set_global_default` does not.
///
/// # Errors
///
/// Fails before spawning any worker when the queue capacity is above
/// [`LoggerQueueOptions::MAX_CAPACITY`] or a filter is invalid, and fails when the file output cannot
/// create its base directory or `_fallback.log`.
///
/// # Example
///
/// ```no_run
/// use kikiutils::logger::{
///     build_logger,
///     options::LoggerInitOptions,
/// };
/// use tracing_subscriber::{
///     layer::SubscriberExt,
///     util::SubscriberInitExt,
/// };
///
/// # fn main() -> anyhow::Result<()> {
/// let options = LoggerInitOptions::from_env()?.without_console_output();
/// let (logger_layer, logger_guard) = build_logger(options)?;
/// tracing_subscriber::registry()
///     .with(tracing_subscriber::fmt::layer().compact())
///     .with(logger_layer)
///     .try_init()?;
///
/// tracing::info!("application started");
/// logger_guard.shutdown()?;
/// # Ok(())
/// # }
/// ```
pub fn build_logger<S>(options: LoggerInitOptions) -> Result<(BoxedLayer<S>, LoggerGuard)>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    // Validate everything before spawning workers so invalid options fail without side effects.
    if options.queue.capacity.get() > LoggerQueueOptions::MAX_CAPACITY {
        bail!(
            "logger queue capacity {} exceeds the maximum of {}",
            options.queue.capacity,
            LoggerQueueOptions::MAX_CAPACITY
        );
    }

    let console = options
        .console_output
        .map(|console| parse_filter(&console.filter, "console").map(|filter| (console, filter)))
        .transpose()?;

    let file = options
        .file_output
        .map(|file| parse_filter(&file.filter, "file").map(|filter| (file, filter)))
        .transpose()?;

    let timer = LoggerTimer::new(options.timestamp);
    let mut layers = Vec::<BoxedLayer<S>>::new();
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
                .fmt_fields(ConsoleFields::default())
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

    // An empty `Vec` layer disables every callsite, which would also silence the caller's other layers.
    if layers.is_empty() {
        return Ok((layer::Identity::new().boxed(), logger_guard));
    }

    Ok((layers.boxed(), logger_guard))
}

/// Builds the logger with [`build_logger`] and installs it as the global subscriber, together with the
/// `log` bridge.
///
/// # Errors
///
/// Fails without creating directories or workers when a default subscriber is already set, either
/// globally or scoped on the calling thread. Also fails when a `log` logger is already set; by then the file
/// output may have created its directories, and the workers stop with the dropped guard.
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
    // `tracing::dispatcher::has_been_set` also turns true after any scoped default has ended.
    if dispatcher::get_default(|dispatch| !dispatch.is::<NoSubscriber>()) {
        bail!("failed to initialize tracing subscriber: a default subscriber is already set");
    }

    let (layer, logger_guard) = build_logger::<Registry>(options)?;
    let subscriber = registry().with(layer);
    // Install the `log` bridge first: `try_init` would set the global subscriber before failing on an
    // existing `log` logger, leaving a subscriber whose workers stop with the dropped guard.
    LogTracer::builder()
        .with_max_level(subscriber.max_level_hint().unwrap_or(LevelFilter::TRACE).as_log())
        .init()
        .context("failed to initialize log bridge: a log logger is already set")?;
    tracing::subscriber::set_global_default(subscriber).context("failed to initialize tracing subscriber")?;

    Ok(logger_guard)
}

fn parse_filter(directives: &str, output: &str) -> Result<EnvFilter> {
    EnvFilter::builder()
        .parse(directives)
        .with_context(|| format!("invalid {output} log filter: {directives}"))
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

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{
            AtomicUsize,
            Ordering,
        },
    };

    use tracing::Subscriber;
    use tracing_subscriber::layer::Context;

    use super::*;

    struct CountingLayer(Arc<AtomicUsize>);

    impl<S: Subscriber> Layer<S> for CountingLayer {
        fn on_event(&self, _: &tracing::Event<'_>, _: Context<'_, S>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn logger_without_outputs_leaves_other_layers_working() {
        let options = LoggerInitOptions::default()
            .without_console_output()
            .without_file_output();

        let (logger_layer, logger_guard) = build_logger(options).unwrap();
        let events = Arc::new(AtomicUsize::new(0));
        let subscriber = registry().with(logger_layer).with(CountingLayer(Arc::clone(&events)));

        tracing::subscriber::with_default(subscriber, || tracing::info!("reaches the other layer"));

        assert_eq!(events.load(Ordering::SeqCst), 1);
        logger_guard.shutdown().unwrap();
    }
}
