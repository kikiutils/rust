use std::env::var;

use anyhow::{
    Result,
    anyhow,
    bail,
};
use pathkit::Path;
use tracing_subscriber::filter::LevelFilter;

// Constants/Statics
const DEFAULT_ROTATION_MAX_BYTES: u64 = 5 * 1024 * 1024;
const DEFAULT_ROTATION_BACKUP_COUNT: usize = 5;
const ENV_LOG_LEVEL: &str = "RUST_LOG";
const ENV_LOGGER_CONSOLE_ANSI: &str = "LOGGER_CONSOLE_ANSI";
const ENV_LOGGER_CONSOLE_ENABLED: &str = "LOGGER_CONSOLE_ENABLED";
const ENV_LOGGER_CONSOLE_LEVEL: &str = "LOGGER_CONSOLE_LEVEL";
const ENV_LOGGER_CONSOLE_TARGET: &str = "LOGGER_CONSOLE_TARGET";
const ENV_LOGGER_FILE_BASE_DIR: &str = "LOG_DIR";
const ENV_LOGGER_FILE_ENABLED: &str = "LOG_TO_FILE";
const ENV_LOGGER_FILE_LEVEL: &str = "LOGGER_FILE_LEVEL";
const ENV_LOGGER_FILE_ROTATION_BACKUP_COUNT: &str = "LOGGER_FILE_ROTATION_BACKUP_COUNT";
const ENV_LOGGER_FILE_ROTATION_ENABLED: &str = "LOGGER_FILE_ROTATION_ENABLED";
const ENV_LOGGER_FILE_ROTATION_MAX_BYTES: &str = "LOGGER_FILE_ROTATION_MAX_BYTES";

// Enums
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LoggerLogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
    Off,
}

impl LoggerLogLevel {
    pub(super) fn as_level_filter(self) -> LevelFilter {
        match self {
            Self::Trace => LevelFilter::TRACE,
            Self::Debug => LevelFilter::DEBUG,
            Self::Info => LevelFilter::INFO,
            Self::Warn => LevelFilter::WARN,
            Self::Error => LevelFilter::ERROR,
            Self::Off => LevelFilter::OFF,
        }
    }
}

// Structs
#[derive(Clone, Debug)]
pub struct LoggerConsoleOutputOptions {
    pub ansi_enabled: bool,
    pub include_target: bool,
    pub level: LoggerLogLevel,
}

impl Default for LoggerConsoleOutputOptions {
    fn default() -> Self {
        Self {
            ansi_enabled: true,
            include_target: true,
            level: LoggerLogLevel::Info,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LoggerFileOutputOptions {
    pub base_dir: Path,
    pub level: LoggerLogLevel,
    pub rotation: Option<LoggerFileRotationOptions>,
}

impl LoggerFileOutputOptions {
    pub fn new(base_dir: impl Into<Path>) -> Self {
        Self {
            base_dir: base_dir.into(),
            level: LoggerLogLevel::Info,
            rotation: Some(LoggerFileRotationOptions::default()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoggerFileRotationOptions {
    pub backup_file_count: usize,
    pub max_bytes_per_file: u64,
}

impl Default for LoggerFileRotationOptions {
    fn default() -> Self {
        Self {
            backup_file_count: DEFAULT_ROTATION_BACKUP_COUNT,
            max_bytes_per_file: DEFAULT_ROTATION_MAX_BYTES,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LoggerInitOptions {
    pub console_output: Option<LoggerConsoleOutputOptions>,
    pub file_output: Option<LoggerFileOutputOptions>,
}

impl LoggerInitOptions {
    // Public methods
    pub fn disabled() -> Self {
        Self {
            console_output: None,
            file_output: None,
        }
    }

    /// Builds logger initialization options from environment variables.
    ///
    /// Reads these keys:
    /// - `RUST_LOG`
    /// - `LOGGER_CONSOLE_ENABLED`
    /// - `LOGGER_CONSOLE_LEVEL`
    /// - `LOGGER_CONSOLE_TARGET`
    /// - `LOGGER_CONSOLE_ANSI`
    /// - `LOG_TO_FILE`
    /// - `LOG_DIR`
    /// - `LOGGER_FILE_LEVEL`
    /// - `LOGGER_FILE_ROTATION_ENABLED`
    /// - `LOGGER_FILE_ROTATION_MAX_BYTES`
    /// - `LOGGER_FILE_ROTATION_BACKUP_COUNT`
    pub fn from_env() -> Result<Self> {
        let default_level = env_log_level(ENV_LOG_LEVEL, LoggerLogLevel::Info)?;
        let console_output = if env_bool(ENV_LOGGER_CONSOLE_ENABLED, true)? {
            Some(LoggerConsoleOutputOptions {
                ansi_enabled: env_bool(ENV_LOGGER_CONSOLE_ANSI, true)?,
                include_target: env_bool(ENV_LOGGER_CONSOLE_TARGET, true)?,
                level: env_log_level(ENV_LOGGER_CONSOLE_LEVEL, default_level)?,
            })
        } else {
            None
        };

        let file_output = if env_bool(ENV_LOGGER_FILE_ENABLED, false)? {
            Some(LoggerFileOutputOptions {
                base_dir: env_path(ENV_LOGGER_FILE_BASE_DIR)?,
                level: env_log_level(ENV_LOGGER_FILE_LEVEL, default_level)?,
                rotation: env_rotation_options()?,
            })
        } else {
            None
        };

        Ok(Self {
            console_output,
            file_output,
        })
    }
}

// Functions
fn env_bool(key: &str, default: bool) -> Result<bool> {
    let Some(value) = env_value(key) else {
        return Ok(default);
    };

    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" | "on" => Ok(true),
        "0" | "false" | "no" | "n" | "off" => Ok(false),
        _ => bail!("{key} must be a boolean value"),
    }
}

fn env_log_level(key: &str, default: LoggerLogLevel) -> Result<LoggerLogLevel> {
    let Some(value) = env_value(key) else {
        return Ok(default);
    };

    parse_log_level(key, &value)
}

fn env_path(key: &str) -> Result<Path> {
    let Some(value) = env_value(key) else {
        bail!("{key} is required when file logging is enabled");
    };

    Ok(Path::from(value))
}

fn env_rotation_options() -> Result<Option<LoggerFileRotationOptions>> {
    if !env_bool(ENV_LOGGER_FILE_ROTATION_ENABLED, true)? {
        return Ok(None);
    }

    let max_bytes_per_file = env_u64(ENV_LOGGER_FILE_ROTATION_MAX_BYTES, DEFAULT_ROTATION_MAX_BYTES)?;
    if max_bytes_per_file == 0 {
        bail!("{ENV_LOGGER_FILE_ROTATION_MAX_BYTES} must be greater than 0 when file log rotation is enabled");
    }

    Ok(Some(LoggerFileRotationOptions {
        backup_file_count: env_usize(ENV_LOGGER_FILE_ROTATION_BACKUP_COUNT, DEFAULT_ROTATION_BACKUP_COUNT)?,
        max_bytes_per_file,
    }))
}

fn env_u64(key: &str, default: u64) -> Result<u64> {
    let Some(value) = env_value(key) else {
        return Ok(default);
    };

    value
        .parse::<u64>()
        .map_err(|error| anyhow::anyhow!("{key} must be a non-negative integer: {error}"))
}

fn env_usize(key: &str, default: usize) -> Result<usize> {
    let Some(value) = env_value(key) else {
        return Ok(default);
    };

    value
        .parse::<usize>()
        .map_err(|error| anyhow!("{key} must be a non-negative integer: {error}"))
}

fn env_value(key: &str) -> Option<String> {
    var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_log_level(key: &str, value: &str) -> Result<LoggerLogLevel> {
    match value.to_ascii_lowercase().as_str() {
        "trace" => Ok(LoggerLogLevel::Trace),
        "debug" => Ok(LoggerLogLevel::Debug),
        "info" => Ok(LoggerLogLevel::Info),
        "warn" | "warning" => Ok(LoggerLogLevel::Warn),
        "error" => Ok(LoggerLogLevel::Error),
        "off" => Ok(LoggerLogLevel::Off),
        _ => bail!("{key} must be one of trace, debug, info, warn, error, off"),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env::{
            remove_var,
            set_var,
        },
        sync::{
            Mutex,
            OnceLock,
        },
    };

    use pathkit::path;

    use super::*;

    #[test]
    fn log_levels_map_to_level_filters() {
        assert_eq!(LoggerLogLevel::Trace.as_level_filter(), LevelFilter::TRACE);
        assert_eq!(LoggerLogLevel::Debug.as_level_filter(), LevelFilter::DEBUG);
        assert_eq!(LoggerLogLevel::Info.as_level_filter(), LevelFilter::INFO);
        assert_eq!(LoggerLogLevel::Warn.as_level_filter(), LevelFilter::WARN);
        assert_eq!(LoggerLogLevel::Error.as_level_filter(), LevelFilter::ERROR);
        assert_eq!(LoggerLogLevel::Off.as_level_filter(), LevelFilter::OFF);
    }

    #[test]
    fn defaults_match_logger_contract() {
        let console = LoggerConsoleOutputOptions::default();
        assert!(console.ansi_enabled);
        assert!(console.include_target);
        assert_eq!(console.level, LoggerLogLevel::Info);

        let rotation = LoggerFileRotationOptions::default();
        assert_eq!(rotation.backup_file_count, DEFAULT_ROTATION_BACKUP_COUNT);
        assert_eq!(rotation.max_bytes_per_file, DEFAULT_ROTATION_MAX_BYTES);

        let file = LoggerFileOutputOptions::new("logs");
        assert_eq!(file.base_dir, path!("logs"));
        assert_eq!(file.level, LoggerLogLevel::Info);
        assert_eq!(file.rotation, Some(rotation));

        let disabled = LoggerInitOptions::disabled();
        assert!(disabled.console_output.is_none());
        assert!(disabled.file_output.is_none());
    }

    #[test]
    fn from_env_uses_defaults_when_logger_env_is_absent() -> Result<()> {
        with_logger_env(&[], || {
            let options = LoggerInitOptions::from_env()?;
            let console = options
                .console_output
                .ok_or_else(|| anyhow::anyhow!("console output should default to enabled"))?;

            assert!(console.ansi_enabled);
            assert!(console.include_target);
            assert_eq!(console.level, LoggerLogLevel::Info);
            assert!(options.file_output.is_none());

            Ok(())
        })
    }

    #[test]
    fn from_env_builds_file_and_console_options() -> Result<()> {
        with_logger_env(
            &[
                (ENV_LOG_LEVEL, "warn"),
                (ENV_LOGGER_CONSOLE_ENABLED, "true"),
                (ENV_LOGGER_CONSOLE_LEVEL, "debug"),
                (ENV_LOGGER_CONSOLE_TARGET, "false"),
                (ENV_LOGGER_CONSOLE_ANSI, "off"),
                (ENV_LOGGER_FILE_ENABLED, "yes"),
                (ENV_LOGGER_FILE_BASE_DIR, "env-logs"),
                (ENV_LOGGER_FILE_LEVEL, "trace"),
                (ENV_LOGGER_FILE_ROTATION_ENABLED, "on"),
                (ENV_LOGGER_FILE_ROTATION_MAX_BYTES, "123"),
                (ENV_LOGGER_FILE_ROTATION_BACKUP_COUNT, "0"),
            ],
            || {
                let options = LoggerInitOptions::from_env()?;
                let console = options
                    .console_output
                    .ok_or_else(|| anyhow::anyhow!("console output should be enabled"))?;
                let file = options
                    .file_output
                    .ok_or_else(|| anyhow::anyhow!("file output should be enabled"))?;

                assert!(!console.ansi_enabled);
                assert!(!console.include_target);
                assert_eq!(console.level, LoggerLogLevel::Debug);
                assert_eq!(file.base_dir, path!("env-logs"));
                assert_eq!(file.level, LoggerLogLevel::Trace);
                assert_eq!(
                    file.rotation,
                    Some(LoggerFileRotationOptions {
                        backup_file_count: 0,
                        max_bytes_per_file: 123,
                    })
                );

                Ok(())
            },
        )
    }

    #[test]
    fn from_env_supports_disabled_outputs_and_rotation() -> Result<()> {
        with_logger_env(
            &[
                (ENV_LOGGER_CONSOLE_ENABLED, "false"),
                (ENV_LOGGER_FILE_ENABLED, "true"),
                (ENV_LOGGER_FILE_BASE_DIR, "env-logs"),
                (ENV_LOGGER_FILE_ROTATION_ENABLED, "false"),
            ],
            || {
                let options = LoggerInitOptions::from_env()?;
                let file = options
                    .file_output
                    .ok_or_else(|| anyhow::anyhow!("file output should be enabled"))?;

                assert!(options.console_output.is_none());
                assert_eq!(file.rotation, None);

                Ok(())
            },
        )
    }

    #[test]
    fn from_env_rejects_invalid_values() {
        let invalid_bool = with_logger_env(&[(ENV_LOGGER_CONSOLE_ENABLED, "maybe")], LoggerInitOptions::from_env);
        assert!(invalid_bool.is_err());

        let missing_file_dir = with_logger_env(&[(ENV_LOGGER_FILE_ENABLED, "true")], LoggerInitOptions::from_env);
        assert!(missing_file_dir.is_err());

        let zero_rotation = with_logger_env(
            &[
                (ENV_LOGGER_FILE_ENABLED, "true"),
                (ENV_LOGGER_FILE_BASE_DIR, "env-logs"),
                (ENV_LOGGER_FILE_ROTATION_MAX_BYTES, "0"),
            ],
            LoggerInitOptions::from_env,
        );
        assert!(zero_rotation.is_err());
    }

    fn with_logger_env<T>(values: &[(&str, &str)], action: impl FnOnce() -> T) -> T {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let keys = [
            ENV_LOG_LEVEL,
            ENV_LOGGER_CONSOLE_ENABLED,
            ENV_LOGGER_CONSOLE_LEVEL,
            ENV_LOGGER_CONSOLE_TARGET,
            ENV_LOGGER_CONSOLE_ANSI,
            ENV_LOGGER_FILE_ENABLED,
            ENV_LOGGER_FILE_LEVEL,
            ENV_LOGGER_FILE_BASE_DIR,
            ENV_LOGGER_FILE_ROTATION_ENABLED,
            ENV_LOGGER_FILE_ROTATION_MAX_BYTES,
            ENV_LOGGER_FILE_ROTATION_BACKUP_COUNT,
        ];

        let previous_values = keys.map(|key| (key, var(key).ok()));

        for key in keys {
            remove_logger_env_var(key);
        }
        for (key, value) in values {
            set_logger_env_var(key, value);
        }

        let result = action();

        for (key, value) in previous_values {
            match value {
                Some(value) => set_logger_env_var(key, &value),
                None => remove_logger_env_var(key),
            }
        }

        result
    }

    fn remove_logger_env_var(key: &str) {
        // SAFETY: Tests serialize all logger-related environment mutations with ENV_LOCK and restore previous values before releasing it.
        unsafe { remove_var(key) };
    }

    fn set_logger_env_var(key: &str, value: &str) {
        // SAFETY: Tests serialize all logger-related environment mutations with ENV_LOCK and restore previous values before releasing it.
        unsafe { set_var(key, value) };
    }
}
