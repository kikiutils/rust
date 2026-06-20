use std::{
    env::temp_dir,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

use anyhow::Result;
use kikiutils::logger::{
    init_logger,
    options::{
        LoggerConsoleOutputOptions,
        LoggerFileOutputOptions,
        LoggerFileRotationOptions,
        LoggerInitOptions,
        LoggerLogLevel,
        LoggerNonBlockingOptions,
        LoggerQueueFullPolicy,
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
};

#[test]
fn init_logger_writes_console_and_rotating_target_files() -> Result<()> {
    let log_dir = unique_temp_dir();
    let default_console_options = LoggerConsoleOutputOptions::default();
    let console_options = LoggerConsoleOutputOptions {
        ansi_enabled: false,
        include_target: true,
        level: LoggerLogLevel::Off,
    };

    let file_options = LoggerFileOutputOptions {
        base_dir: log_dir.clone(),
        level: LoggerLogLevel::Trace,
        rotation: Some(LoggerFileRotationOptions {
            backup_file_count: 1,
            max_bytes_per_file: 1,
        }),
    };

    assert!(default_console_options.ansi_enabled);
    assert!(default_console_options.include_target);
    assert_eq!(default_console_options.level, LoggerLogLevel::Info);
    assert!(LoggerInitOptions::disabled().console_output.is_none());

    let logger_guard = init_logger(LoggerInitOptions {
        console_output: Some(console_options),
        file_output: Some(file_options),
        non_blocking: LoggerNonBlockingOptions {
            channel_capacity: 128,
            queue_full_policy: LoggerQueueFullPolicy::Block,
        },
    })?;

    event!(target: "logger_integration::bad/target?", Level::INFO, "first message");
    event!(target: "logger_integration::bad/target?", Level::INFO, "second message");
    event!(target: "logger_integration::bad/target?", Level::INFO, "third message");

    logger_guard.flush()?;

    let target_log = &log_dir / "logger_integration" / "bad_target_.log";
    let rotated_target_log = target_log.with_added_extension("1");

    assert!(target_log.exists_sync()?);
    assert!(rotated_target_log.exists_sync()?);
    assert!(target_log.read_to_string_sync()?.contains("third message"));
    assert!(rotated_target_log.read_to_string_sync()?.contains("second message"));
    assert!(!target_log.with_added_extension("2").exists_sync()?);

    logger_guard.shutdown()?;
    log_dir.remove_dir_all_sync()?;

    Ok(())
}

fn unique_temp_dir() -> Path {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    path!(temp_dir()) / format!("kikiutils-logger-integration-{nanos}")
}
