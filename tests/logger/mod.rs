use std::{
    env,
    num::NonZeroUsize,
    process,
    sync::atomic::{
        AtomicUsize,
        Ordering,
    },
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

use anyhow::{
    Context,
    Result,
};
use kikiutils::logger::{
    build_logger,
    init_logger,
    options::{
        LoggerAnsiMode,
        LoggerConsoleOutputOptions,
        LoggerConsoleStream,
        LoggerFileOutputOptions,
        LoggerInitOptions,
        LoggerQueueFullPolicy,
        LoggerQueueOptions,
    },
};
use pathkit::{
    Path,
    path,
    sync_fs_ops::SyncFsOps,
};
use tracing::{
    Level,
    event,
    span,
};
use tracing_subscriber::{
    Registry,
    layer::SubscriberExt,
};

#[test]
fn init_logger_writes_filtered_events_to_target_files() -> Result<()> {
    let log_dir = unique_temp_dir();
    let invalid = init_logger(
        LoggerInitOptions::default()
            .with_console_output(LoggerConsoleOutputOptions::default().with_filter("info,[invalid"))
            .with_file_output(LoggerFileOutputOptions::new(log_dir.clone())),
    );

    assert!(
        invalid.is_err_and(|error| error.to_string().starts_with("invalid console log filter")),
        "an invalid console filter must fail init"
    );

    assert!(!log_dir.try_exists_sync()?);

    // The console enables only the span, probe and shared targets, so it formats span fields with ANSI
    // and prints just the shared event. The probe target is enabled on the console but not in the file,
    // and the shared target in both, which is the per-layer filter split tokio-rs/tracing#2519 needs.
    let logger_guard = init_logger(
        LoggerInitOptions::default()
            .with_console_output(
                LoggerConsoleOutputOptions::default()
                    .with_ansi(LoggerAnsiMode::Always)
                    .with_filter(
                        "logger_integration::span=info,logger_integration::probe=info,logger_integration::shared=info",
                    )
                    .with_stream(LoggerConsoleStream::Stderr),
            )
            .with_file_output(
                LoggerFileOutputOptions::new(log_dir.clone())
                    .with_filter("info,logger_integration::muted=off,logger_integration::probe=off"),
            )
            .with_queue(
                LoggerQueueOptions::default()
                    .with_capacity(NonZeroUsize::new(128).context("queue capacity must be non-zero")?)
                    .with_full_policy(LoggerQueueFullPolicy::Block),
            ),
    )?;

    event!(target: "logger_integration::bad/target?", Level::INFO, "first message");
    event!(target: "logger_integration::bad/target?", Level::DEBUG, "filtered message");
    event!(target: "logger_integration::bad/target?", Level::WARN, "second message");
    event!(target: "logger_integration::muted", Level::ERROR, "muted message");
    span!(target: "logger_integration::span", Level::INFO, "req", id = 1).in_scope(|| {
        event!(target: "logger_integration::bad/target?", Level::INFO, "span message");
    });

    log::warn!(target: "logger_integration::bridged", "bridged message");
    let _ = tracing::enabled!(target: "logger_integration::probe", Level::INFO);
    event!(target: "logger_integration::shared", Level::INFO, "after enabled check");

    logger_guard.flush()?;

    let content = (&log_dir / "logger_integration" / "bad_target_.log").read_to_string_sync()?;
    let lines = content.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 3, "unexpected log content: {content}");
    assert!(lines.iter().all(|line| has_timestamp_prefix(line)), "{content}");
    assert!(lines[0].ends_with(" INFO logger_integration::bad/target?: first message"));
    assert!(lines[1].ends_with(" WARN logger_integration::bad/target?: second message"));
    assert!(lines[2].ends_with(" INFO req{id=1}: logger_integration::bad/target?: span message"));

    assert!(
        !content.contains('\x1b'),
        "file log must not contain ANSI escapes: {content:?}"
    );

    let bridged = (&log_dir / "logger_integration" / "bridged.log").read_to_string_sync()?;
    assert!(
        bridged.ends_with(" WARN logger_integration::bridged: bridged message\n"),
        "{bridged:?}"
    );

    let shared = (&log_dir / "logger_integration" / "shared.log").read_to_string_sync()?;
    assert!(
        shared.ends_with(" INFO logger_integration::shared: after enabled check\n"),
        "{shared:?}"
    );

    assert!(!(&log_dir / "logger_integration" / "muted.log").try_exists_sync()?);

    let second_dir = unique_temp_dir();
    let second =
        init_logger(LoggerInitOptions::default().with_file_output(LoggerFileOutputOptions::new(second_dir.clone())));

    assert!(second.is_err(), "a second init must fail");
    assert!(!second_dir.try_exists_sync()?);

    event!(target: "logger_integration::shutdown", Level::INFO, "before shutdown");
    assert_eq!(logger_guard.dropped_count(), 0);
    logger_guard.shutdown()?;

    let shutdown = (&log_dir / "logger_integration" / "shutdown.log").read_to_string_sync()?;
    assert!(
        shutdown.ends_with(" INFO logger_integration::shutdown: before shutdown\n"),
        "{shutdown:?}"
    );

    log_dir.remove_dir_all_sync()?;

    Ok(())
}

#[test]
fn build_logger_rejects_oversized_queue_without_side_effects() -> Result<()> {
    let log_dir = unique_temp_dir();
    let capacity = NonZeroUsize::new(LoggerQueueOptions::MAX_CAPACITY + 1).context("capacity must be non-zero")?;
    let result = build_logger::<Registry>(
        LoggerInitOptions::default()
            .with_file_output(LoggerFileOutputOptions::new(log_dir.clone()))
            .with_queue(LoggerQueueOptions::default().with_capacity(capacity)),
    );

    assert!(result.is_err());
    assert!(!log_dir.try_exists_sync()?);

    Ok(())
}

#[test]
fn dropping_guard_flushes_and_later_events_do_not_block() -> Result<()> {
    let log_dir = unique_temp_dir();
    let (layer, logger_guard) = build_logger(
        LoggerInitOptions::default()
            .without_console_output()
            .with_file_output(LoggerFileOutputOptions::new(log_dir.clone()))
            .with_queue(
                LoggerQueueOptions::default()
                    .with_capacity(NonZeroUsize::MIN)
                    .with_full_policy(LoggerQueueFullPolicy::Block),
            ),
    )?;

    let subscriber = Registry::default().with(layer);
    tracing::subscriber::with_default(subscriber, || {
        event!(target: "scoped::drop", Level::INFO, "before drop");
        drop(logger_guard);
        // The worker is gone, so a blocking queue must fail fast instead of waiting forever.
        event!(target: "scoped::drop", Level::INFO, "after drop");
        event!(target: "scoped::drop", Level::INFO, "after drop again");
    });

    let content = (&log_dir / "scoped" / "drop.log").read_to_string_sync()?;
    log_dir.remove_dir_all_sync()?;
    assert!(content.ends_with(" INFO scoped::drop: before drop\n"), "{content:?}");
    assert_eq!(content.lines().count(), 1, "{content:?}");

    Ok(())
}

fn has_timestamp_prefix(line: &str) -> bool {
    let pattern = b"0000-00-00 00:00:00.000000";
    line.len() > pattern.len()
        && line.bytes().zip(pattern).all(|(byte, expected)| {
            if *expected == b'0' {
                byte.is_ascii_digit()
            } else {
                byte == *expected
            }
        })
}

fn unique_temp_dir() -> Path {
    // Tests run in parallel and the clock can be coarse (100 ns on Windows), so the time alone may repeat.
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or_default(|duration| duration.as_nanos());

    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    path!(env::temp_dir()) / format!("kikiutils-logger-integration-{}-{nanos}-{count}", process::id())
}
