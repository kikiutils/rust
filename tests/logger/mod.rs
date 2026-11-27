use std::{
    env,
    num::NonZeroUsize,
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
    SyncFsOps,
    path,
};
use tracing::{
    Level,
    event,
    span,
};

#[test]
fn init_logger_writes_filtered_events_to_target_files() -> Result<()> {
    let log_dir = unique_temp_dir();
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

    assert!(!(&log_dir / "logger_integration" / "muted.log").exists_sync()?);
    assert_eq!(logger_guard.dropped_count(), 0);

    logger_guard.shutdown()?;
    log_dir.remove_dir_all_sync()?;

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
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or_default(|duration| duration.as_nanos());

    path!(env::temp_dir()) / format!("kikiutils-logger-integration-{nanos}")
}
