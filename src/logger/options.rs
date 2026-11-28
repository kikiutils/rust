use std::{
    env,
    num::NonZeroUsize,
};

use anyhow::{
    Result,
    anyhow,
    bail,
};
use pathkit::Path;

// Constants/Statics
const DEFAULT_QUEUE_CAPACITY: NonZeroUsize = match NonZeroUsize::new(8192) {
    Some(capacity) => capacity,
    None => panic!("default queue capacity must be non-zero"),
};

const DEFAULT_FILTER: &str = "info";
const ENV_LOG_FILTER: &str = "RUST_LOG";
const ENV_LOGGER_CONSOLE_ANSI: &str = "LOGGER_CONSOLE_ANSI";
const ENV_LOGGER_CONSOLE_ENABLED: &str = "LOGGER_CONSOLE_ENABLED";
const ENV_LOGGER_CONSOLE_FILTER: &str = "LOGGER_CONSOLE_FILTER";
const ENV_LOGGER_CONSOLE_STREAM: &str = "LOGGER_CONSOLE_STREAM";
const ENV_LOGGER_CONSOLE_TARGET: &str = "LOGGER_CONSOLE_TARGET";
const ENV_LOGGER_FILE_DIR: &str = "LOGGER_FILE_DIR";
const ENV_LOGGER_FILE_ENABLED: &str = "LOGGER_FILE_ENABLED";
const ENV_LOGGER_FILE_FILTER: &str = "LOGGER_FILE_FILTER";
const ENV_LOGGER_QUEUE_CAPACITY: &str = "LOGGER_QUEUE_CAPACITY";
const ENV_LOGGER_QUEUE_FULL_POLICY: &str = "LOGGER_QUEUE_FULL_POLICY";
const ENV_LOGGER_TIMESTAMP_OFFSET: &str = "LOGGER_TIMESTAMP_OFFSET";
const ENV_LOGGER_TIMESTAMP_TIME_ZONE: &str = "LOGGER_TIMESTAMP_TIME_ZONE";

// Enums
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoggerAnsiMode {
    Always,
    /// Enabled when the console stream is a terminal and `NO_COLOR` is unset or empty.
    #[default]
    Auto,
    Never,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoggerConsoleStream {
    #[default]
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoggerQueueFullPolicy {
    /// The thread emitting the event waits for queue space, which also stalls an async runtime worker
    /// thread that logs.
    Block,
    /// Dropped events are counted in [`LoggerGuard::dropped_count`](super::LoggerGuard::dropped_count);
    /// while the worker runs, it also reports them with a `WARN kikiutils::logger` line.
    #[default]
    DropNewest,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoggerTimeZone {
    #[default]
    Local,
    Utc,
}

// Structs

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct LoggerConsoleOutputOptions {
    pub ansi: LoggerAnsiMode,
    /// [`EnvFilter`](tracing_subscriber::EnvFilter) directives, e.g. `info` or `info,hyper=warn`.
    pub filter: String,
    pub include_target: bool,
    pub stream: LoggerConsoleStream,
}

impl Default for LoggerConsoleOutputOptions {
    fn default() -> Self {
        Self {
            ansi: LoggerAnsiMode::default(),
            filter: DEFAULT_FILTER.to_owned(),
            include_target: true,
            stream: LoggerConsoleStream::default(),
        }
    }
}

impl LoggerConsoleOutputOptions {
    // Public methods
    #[must_use]
    pub fn with_ansi(mut self, ansi: LoggerAnsiMode) -> Self {
        self.ansi = ansi;
        self
    }

    #[must_use]
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = filter.into();
        self
    }

    #[must_use]
    pub fn with_include_target(mut self, include_target: bool) -> Self {
        self.include_target = include_target;
        self
    }

    #[must_use]
    pub fn with_stream(mut self, stream: LoggerConsoleStream) -> Self {
        self.stream = stream;
        self
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct LoggerFileOutputOptions {
    pub base_dir: Path,
    /// [`EnvFilter`](tracing_subscriber::EnvFilter) directives, e.g. `info` or `info,hyper=warn`.
    pub filter: String,
}

impl LoggerFileOutputOptions {
    pub fn new(base_dir: impl Into<Path>) -> Self {
        Self {
            base_dir: base_dir.into(),
            filter: DEFAULT_FILTER.to_owned(),
        }
    }

    // Public methods
    #[must_use]
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = filter.into();
        self
    }
}

/// Each output gets its own queue.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct LoggerQueueOptions {
    /// The queue preallocates every slot, so building the logger rejects values above
    /// [`MAX_CAPACITY`](Self::MAX_CAPACITY).
    pub capacity: NonZeroUsize,
    pub full_policy: LoggerQueueFullPolicy,
}

impl Default for LoggerQueueOptions {
    fn default() -> Self {
        Self {
            capacity: DEFAULT_QUEUE_CAPACITY,
            full_policy: LoggerQueueFullPolicy::default(),
        }
    }
}

impl LoggerQueueOptions {
    pub const MAX_CAPACITY: usize = 1_048_576;

    // Public methods
    #[must_use]
    pub fn with_capacity(mut self, capacity: NonZeroUsize) -> Self {
        self.capacity = capacity;
        self
    }

    #[must_use]
    pub fn with_full_policy(mut self, full_policy: LoggerQueueFullPolicy) -> Self {
        self.full_policy = full_policy;
        self
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct LoggerInitOptions {
    pub console_output: Option<LoggerConsoleOutputOptions>,
    pub file_output: Option<LoggerFileOutputOptions>,
    pub queue: LoggerQueueOptions,
    pub timestamp: LoggerTimestampOptions,
}

impl Default for LoggerInitOptions {
    fn default() -> Self {
        Self {
            console_output: Some(LoggerConsoleOutputOptions::default()),
            file_output: None,
            queue: LoggerQueueOptions::default(),
            timestamp: LoggerTimestampOptions::default(),
        }
    }
}

impl LoggerInitOptions {
    // Public methods

    #[must_use]
    pub fn with_console_output(mut self, console_output: LoggerConsoleOutputOptions) -> Self {
        self.console_output = Some(console_output);
        self
    }

    #[must_use]
    pub fn with_file_output(mut self, file_output: LoggerFileOutputOptions) -> Self {
        self.file_output = Some(file_output);
        self
    }

    #[must_use]
    pub fn with_queue(mut self, queue: LoggerQueueOptions) -> Self {
        self.queue = queue;
        self
    }

    #[must_use]
    pub fn with_timestamp(mut self, timestamp: LoggerTimestampOptions) -> Self {
        self.timestamp = timestamp;
        self
    }

    #[must_use]
    pub fn without_console_output(mut self) -> Self {
        self.console_output = None;
        self
    }

    #[must_use]
    pub fn without_file_output(mut self) -> Self {
        self.file_output = None;
        self
    }

    /// Reads these keys:
    /// - `RUST_LOG`: default filter directives for all outputs
    /// - `LOGGER_CONSOLE_ANSI`: `auto`, `always`, `never`, or a boolean
    /// - `LOGGER_CONSOLE_ENABLED`
    /// - `LOGGER_CONSOLE_FILTER`: console filter directives, overrides `RUST_LOG`
    /// - `LOGGER_CONSOLE_STREAM`: `stdout` or `stderr`
    /// - `LOGGER_CONSOLE_TARGET`
    /// - `LOGGER_FILE_DIR`: required for file output
    /// - `LOGGER_FILE_ENABLED`: defaults to true when `LOGGER_FILE_DIR` is set
    /// - `LOGGER_FILE_FILTER`: file filter directives, overrides `RUST_LOG`
    /// - `LOGGER_QUEUE_CAPACITY`: a positive integer up to [`LoggerQueueOptions::MAX_CAPACITY`]
    /// - `LOGGER_QUEUE_FULL_POLICY`: `block` or `drop_newest`
    /// - `LOGGER_TIMESTAMP_OFFSET`: a boolean
    /// - `LOGGER_TIMESTAMP_TIME_ZONE`: `local` or `utc`
    ///
    /// Booleans are `1`, `true`, `yes`, `y`, `on` or `0`, `false`, `no`, `n`, `off`. Values are trimmed and
    /// case-insensitive, and blank values count as unset.
    ///
    /// Filter directives are validated when the logger is built.
    pub fn from_env() -> Result<Self> {
        Self::from_env_with_default_filter(DEFAULT_FILTER)
    }

    /// Same as [`from_env`](Self::from_env), but every output uses `default_filter` instead of `info`
    /// when `RUST_LOG` is unset.
    pub fn from_env_with_default_filter(default_filter: &str) -> Result<Self> {
        Self::from_lookup(|key| env::var(key).ok(), default_filter)
    }

    // Private methods
    fn from_lookup(lookup: impl Fn(&str) -> Option<String>, default_filter: &str) -> Result<Self> {
        let env = EnvReader(lookup);
        let default_filter = env.value(ENV_LOG_FILTER).unwrap_or_else(|| default_filter.to_owned());
        let console_output = if env.bool(ENV_LOGGER_CONSOLE_ENABLED, true)? {
            Some(LoggerConsoleOutputOptions {
                ansi: env.parse(
                    ENV_LOGGER_CONSOLE_ANSI,
                    "one of auto, always, never, or a boolean value",
                    LoggerAnsiMode::default(),
                    parse_ansi_mode,
                )?,
                filter: env
                    .value(ENV_LOGGER_CONSOLE_FILTER)
                    .unwrap_or_else(|| default_filter.clone()),
                include_target: env.bool(ENV_LOGGER_CONSOLE_TARGET, true)?,
                stream: env.parse(
                    ENV_LOGGER_CONSOLE_STREAM,
                    "one of stdout, stderr",
                    LoggerConsoleStream::default(),
                    parse_console_stream,
                )?,
            })
        } else {
            None
        };

        let file_dir = env.value(ENV_LOGGER_FILE_DIR);
        let file_output = if env.bool(ENV_LOGGER_FILE_ENABLED, file_dir.is_some())? {
            let Some(base_dir) = file_dir else {
                bail!("{ENV_LOGGER_FILE_DIR} is required when file logging is enabled");
            };

            Some(LoggerFileOutputOptions {
                base_dir: Path::from(base_dir),
                filter: env.value(ENV_LOGGER_FILE_FILTER).unwrap_or(default_filter),
            })
        } else {
            None
        };

        Ok(Self {
            console_output,
            file_output,
            queue: LoggerQueueOptions {
                capacity: env.parse(
                    ENV_LOGGER_QUEUE_CAPACITY,
                    &format!("a positive integer up to {}", LoggerQueueOptions::MAX_CAPACITY),
                    DEFAULT_QUEUE_CAPACITY,
                    parse_queue_capacity,
                )?,
                full_policy: env.parse(
                    ENV_LOGGER_QUEUE_FULL_POLICY,
                    "one of block, drop_newest",
                    LoggerQueueFullPolicy::default(),
                    parse_queue_full_policy,
                )?,
            },
            timestamp: LoggerTimestampOptions {
                include_offset: env.bool(ENV_LOGGER_TIMESTAMP_OFFSET, false)?,
                time_zone: env.parse(
                    ENV_LOGGER_TIMESTAMP_TIME_ZONE,
                    "one of local, utc",
                    LoggerTimeZone::default(),
                    parse_time_zone,
                )?,
            },
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct LoggerTimestampOptions {
    /// Appends the UTC offset, e.g. `+08:00`, right after the microseconds.
    pub include_offset: bool,
    pub time_zone: LoggerTimeZone,
}

impl LoggerTimestampOptions {
    // Public methods
    #[must_use]
    pub fn with_include_offset(mut self, include_offset: bool) -> Self {
        self.include_offset = include_offset;
        self
    }

    #[must_use]
    pub fn with_time_zone(mut self, time_zone: LoggerTimeZone) -> Self {
        self.time_zone = time_zone;
        self
    }
}

struct EnvReader<F>(F);

impl<F: Fn(&str) -> Option<String>> EnvReader<F> {
    fn bool(&self, key: &str, default: bool) -> Result<bool> {
        self.parse(key, "a boolean value", default, parse_bool)
    }

    fn parse<T>(&self, key: &str, expected: &str, default: T, parse: fn(&str) -> Option<T>) -> Result<T> {
        let Some(value) = self.value(key) else {
            return Ok(default);
        };

        parse(&value.to_ascii_lowercase()).ok_or_else(|| anyhow!("{key} must be {expected}"))
    }

    fn value(&self, key: &str) -> Option<String> {
        (self.0)(key)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    }
}

// Functions
fn parse_ansi_mode(value: &str) -> Option<LoggerAnsiMode> {
    match value {
        "always" => Some(LoggerAnsiMode::Always),
        "auto" => Some(LoggerAnsiMode::Auto),
        "never" => Some(LoggerAnsiMode::Never),
        _ => parse_bool(value).map(|enabled| {
            if enabled {
                LoggerAnsiMode::Always
            } else {
                LoggerAnsiMode::Never
            }
        }),
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "1" | "true" | "yes" | "y" | "on" => Some(true),
        "0" | "false" | "no" | "n" | "off" => Some(false),
        _ => None,
    }
}

fn parse_console_stream(value: &str) -> Option<LoggerConsoleStream> {
    match value {
        "stdout" => Some(LoggerConsoleStream::Stdout),
        "stderr" => Some(LoggerConsoleStream::Stderr),
        _ => None,
    }
}

fn parse_queue_capacity(value: &str) -> Option<NonZeroUsize> {
    value
        .parse()
        .ok()
        .filter(|capacity: &NonZeroUsize| capacity.get() <= LoggerQueueOptions::MAX_CAPACITY)
}

fn parse_queue_full_policy(value: &str) -> Option<LoggerQueueFullPolicy> {
    match value {
        "block" => Some(LoggerQueueFullPolicy::Block),
        "drop_newest" => Some(LoggerQueueFullPolicy::DropNewest),
        _ => None,
    }
}

fn parse_time_zone(value: &str) -> Option<LoggerTimeZone> {
    match value {
        "local" => Some(LoggerTimeZone::Local),
        "utc" => Some(LoggerTimeZone::Utc),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use pathkit::path;

    use super::*;

    fn from_pairs(pairs: &[(&str, &str)]) -> Result<LoggerInitOptions> {
        from_pairs_with_default_filter(pairs, DEFAULT_FILTER)
    }

    fn from_pairs_with_default_filter(pairs: &[(&str, &str)], default_filter: &str) -> Result<LoggerInitOptions> {
        LoggerInitOptions::from_lookup(
            |key| {
                pairs
                    .iter()
                    .find(|(pair_key, _)| *pair_key == key)
                    .map(|(_, value)| (*value).to_owned())
            },
            default_filter,
        )
    }

    #[test]
    fn from_env_applies_filter_fallbacks() -> Result<()> {
        let options = from_pairs(&[
            (ENV_LOG_FILTER, "warn,app=debug"),
            (ENV_LOGGER_CONSOLE_STREAM, "stderr"),
            (ENV_LOGGER_FILE_ENABLED, "true"),
            (ENV_LOGGER_FILE_DIR, "env-logs"),
            (ENV_LOGGER_FILE_FILTER, "trace"),
        ])?;

        let console = options
            .console_output
            .ok_or_else(|| anyhow!("console output should be enabled"))?;

        let file = options
            .file_output
            .ok_or_else(|| anyhow!("file output should be enabled"))?;

        assert_eq!(console.filter, "warn,app=debug");
        assert_eq!(console.stream, LoggerConsoleStream::Stderr);
        assert_eq!(file.base_dir, path!("env-logs"));
        assert_eq!(file.filter, "trace");

        let options = from_pairs(&[(ENV_LOG_FILTER, "  ")])?;
        let console = options
            .console_output
            .ok_or_else(|| anyhow!("console output should be enabled"))?;

        assert_eq!(console.filter, "info");
        assert_eq!(console.stream, LoggerConsoleStream::Stdout);
        assert!(options.file_output.is_none());

        let file_pairs = [(ENV_LOGGER_FILE_ENABLED, "true"), (ENV_LOGGER_FILE_DIR, "env-logs")];
        let options = from_pairs_with_default_filter(&file_pairs, "warn,app=info")?;
        let console = options
            .console_output
            .ok_or_else(|| anyhow!("console output should be enabled"))?;

        let file = options
            .file_output
            .ok_or_else(|| anyhow!("file output should be enabled"))?;

        assert_eq!(console.filter, "warn,app=info");
        assert_eq!(file.filter, "warn,app=info");

        let options = from_pairs_with_default_filter(&[(ENV_LOG_FILTER, "debug")], "warn,app=info")?;
        assert_eq!(
            options.console_output.map(|console| console.filter).as_deref(),
            Some("debug")
        );

        Ok(())
    }

    #[test]
    fn from_env_parses_enum_aliases_and_defaults() -> Result<()> {
        for (value, expected) in [
            ("stdout", LoggerConsoleStream::Stdout),
            ("  StDoUt  ", LoggerConsoleStream::Stdout),
            ("stderr", LoggerConsoleStream::Stderr),
            ("  StDeRr  ", LoggerConsoleStream::Stderr),
            ("  ", LoggerConsoleStream::default()),
        ] {
            let options = from_pairs(&[(ENV_LOGGER_CONSOLE_STREAM, value)])?;
            assert_eq!(
                options.console_output.map(|console| console.stream),
                Some(expected),
                "{value:?}"
            );
        }

        for (value, expected) in [
            ("  AuTo  ", LoggerAnsiMode::Auto),
            ("always", LoggerAnsiMode::Always),
            ("never", LoggerAnsiMode::Never),
            ("true", LoggerAnsiMode::Always),
            ("off", LoggerAnsiMode::Never),
            ("  ", LoggerAnsiMode::default()),
        ] {
            let options = from_pairs(&[(ENV_LOGGER_CONSOLE_ANSI, value)])?;
            assert_eq!(
                options.console_output.map(|console| console.ansi),
                Some(expected),
                "{value:?}"
            );
        }

        for (value, expected) in [
            ("block", LoggerQueueFullPolicy::Block),
            ("  DrOp_NeWeSt  ", LoggerQueueFullPolicy::DropNewest),
            ("  ", LoggerQueueFullPolicy::default()),
        ] {
            let options = from_pairs(&[(ENV_LOGGER_QUEUE_FULL_POLICY, value)])?;
            assert_eq!(options.queue.full_policy, expected, "{value:?}");
        }

        for (value, expected) in [
            ("local", LoggerTimeZone::Local),
            ("  UtC  ", LoggerTimeZone::Utc),
            ("  ", LoggerTimeZone::default()),
        ] {
            let options = from_pairs(&[(ENV_LOGGER_TIMESTAMP_TIME_ZONE, value)])?;
            assert_eq!(options.timestamp.time_zone, expected, "{value:?}");
        }

        Ok(())
    }

    #[test]
    fn from_env_parses_toggles_and_queue_capacity() -> Result<()> {
        let options = from_pairs(&[(ENV_LOGGER_CONSOLE_ENABLED, "false")])?;
        assert!(options.console_output.is_none());

        let options = from_pairs(&[(ENV_LOGGER_CONSOLE_TARGET, "false")])?;
        assert_eq!(
            options.console_output.map(|console| console.include_target),
            Some(false)
        );

        let options = from_pairs(&[(ENV_LOGGER_QUEUE_CAPACITY, " 256 ")])?;
        assert_eq!(options.queue.capacity.get(), 256);

        let options = from_pairs(&[(ENV_LOGGER_QUEUE_CAPACITY, "1048576")])?;
        assert_eq!(options.queue.capacity.get(), LoggerQueueOptions::MAX_CAPACITY);

        let options = from_pairs(&[(ENV_LOGGER_TIMESTAMP_OFFSET, "yes")])?;
        assert!(options.timestamp.include_offset);
        assert_eq!(options.timestamp.time_zone, LoggerTimeZone::Local);

        let options = from_pairs(&[])?;
        assert!(!options.timestamp.include_offset);

        Ok(())
    }

    #[test]
    fn from_env_enables_file_output_when_dir_is_set() -> Result<()> {
        let options = from_pairs(&[(ENV_LOGGER_FILE_DIR, "env-logs")])?;
        assert_eq!(options.file_output.map(|file| file.base_dir), Some(path!("env-logs")));

        let options = from_pairs(&[(ENV_LOGGER_FILE_DIR, "env-logs"), (ENV_LOGGER_FILE_ENABLED, "false")])?;
        assert!(options.file_output.is_none());

        let options = from_pairs(&[(ENV_LOGGER_FILE_DIR, "  ")])?;
        assert!(options.file_output.is_none());

        Ok(())
    }

    #[test]
    fn from_env_rejects_invalid_values() {
        assert!(from_pairs(&[(ENV_LOGGER_FILE_ENABLED, "true")]).is_err());
        assert!(from_pairs(&[(ENV_LOGGER_CONSOLE_STREAM, "stdin")]).is_err());
        assert!(from_pairs(&[(ENV_LOGGER_CONSOLE_ENABLED, "maybe")]).is_err());
        assert!(from_pairs(&[(ENV_LOGGER_QUEUE_CAPACITY, "-1")]).is_err());
        assert!(from_pairs(&[(ENV_LOGGER_QUEUE_CAPACITY, "0")]).is_err());
        assert!(from_pairs(&[(ENV_LOGGER_QUEUE_CAPACITY, "1048577")]).is_err());
        assert!(from_pairs(&[(ENV_LOGGER_QUEUE_FULL_POLICY, "wait")]).is_err());
        assert!(from_pairs(&[(ENV_LOGGER_CONSOLE_ANSI, "sometimes")]).is_err());
        assert!(from_pairs(&[(ENV_LOGGER_TIMESTAMP_TIME_ZONE, "gmt")]).is_err());
        assert!(from_pairs(&[(ENV_LOGGER_TIMESTAMP_OFFSET, "maybe")]).is_err());
    }
}
