use std::env::var;

use anyhow::{
    Result,
    anyhow,
    bail,
};
use pathkit::Path;

// Constants/Statics
const DEFAULT_CHANNEL_CAPACITY: usize = 8192;
const DEFAULT_FILTER: &str = "info";
const ENV_LOG_FILTER: &str = "RUST_LOG";
const ENV_LOGGER_CHANNEL_CAPACITY: &str = "LOGGER_CHANNEL_CAPACITY";
const ENV_LOGGER_CONSOLE_ANSI: &str = "LOGGER_CONSOLE_ANSI";
const ENV_LOGGER_CONSOLE_ENABLED: &str = "LOGGER_CONSOLE_ENABLED";
const ENV_LOGGER_CONSOLE_LEVEL: &str = "LOGGER_CONSOLE_LEVEL";
const ENV_LOGGER_CONSOLE_STREAM: &str = "LOGGER_CONSOLE_STREAM";
const ENV_LOGGER_CONSOLE_TARGET: &str = "LOGGER_CONSOLE_TARGET";
const ENV_LOGGER_FILE_BASE_DIR: &str = "LOGGER_LOG_DIR";
const ENV_LOGGER_FILE_ENABLED: &str = "LOGGER_LOG_TO_FILE";
const ENV_LOGGER_FILE_LEVEL: &str = "LOGGER_FILE_LEVEL";
const ENV_LOGGER_QUEUE_FULL_POLICY: &str = "LOGGER_QUEUE_FULL_POLICY";

// Enums
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LoggerConsoleStream {
    #[default]
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LoggerQueueFullPolicy {
    Block,
    #[default]
    DropNewest,
}

// Structs
#[derive(Debug, Clone)]
pub struct LoggerConsoleOutputOptions {
    pub ansi_enabled: bool,
    /// [`EnvFilter`](tracing_subscriber::EnvFilter) directives, e.g. `info` or `info,hyper=warn`.
    pub filter: String,
    pub include_target: bool,
    pub stream: LoggerConsoleStream,
}

impl Default for LoggerConsoleOutputOptions {
    fn default() -> Self {
        Self {
            ansi_enabled: true,
            filter: DEFAULT_FILTER.to_owned(),
            include_target: true,
            stream: LoggerConsoleStream::Stdout,
        }
    }
}

#[derive(Debug, Clone)]
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
}

#[derive(Debug, Clone, Copy)]
pub struct LoggerNonBlockingOptions {
    pub channel_capacity: usize,
    pub queue_full_policy: LoggerQueueFullPolicy,
}

impl Default for LoggerNonBlockingOptions {
    fn default() -> Self {
        Self {
            channel_capacity: DEFAULT_CHANNEL_CAPACITY,
            queue_full_policy: LoggerQueueFullPolicy::DropNewest,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoggerInitOptions {
    pub console_output: Option<LoggerConsoleOutputOptions>,
    pub file_output: Option<LoggerFileOutputOptions>,
    pub non_blocking: LoggerNonBlockingOptions,
}

impl Default for LoggerInitOptions {
    fn default() -> Self {
        Self {
            console_output: Some(LoggerConsoleOutputOptions::default()),
            file_output: None,
            non_blocking: LoggerNonBlockingOptions::default(),
        }
    }
}

impl LoggerInitOptions {
    // Public methods
    pub fn disabled() -> Self {
        Self {
            console_output: None,
            file_output: None,
            non_blocking: LoggerNonBlockingOptions::default(),
        }
    }

    /// Builds logger initialization options from environment variables.
    ///
    /// Reads these keys:
    /// - `RUST_LOG`: default filter directives for all outputs
    /// - `LOGGER_CHANNEL_CAPACITY`
    /// - `LOGGER_CONSOLE_ENABLED`
    /// - `LOGGER_CONSOLE_LEVEL`: console filter directives, overrides `RUST_LOG`
    /// - `LOGGER_CONSOLE_STREAM`: `stdout` or `stderr`
    /// - `LOGGER_CONSOLE_TARGET`
    /// - `LOGGER_CONSOLE_ANSI`
    /// - `LOGGER_FILE_LEVEL`: file filter directives, overrides `RUST_LOG`
    /// - `LOGGER_LOG_DIR`
    /// - `LOGGER_LOG_TO_FILE`
    /// - `LOGGER_QUEUE_FULL_POLICY`
    ///
    /// Filter directives are validated by [`init_logger`](super::init_logger).
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|key| var(key).ok())
    }

    // Private methods
    fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let env = EnvReader(lookup);
        let default_filter = env.value(ENV_LOG_FILTER).unwrap_or_else(|| DEFAULT_FILTER.to_owned());
        let console_output = if env.bool(ENV_LOGGER_CONSOLE_ENABLED, true)? {
            Some(LoggerConsoleOutputOptions {
                ansi_enabled: env.bool(ENV_LOGGER_CONSOLE_ANSI, true)?,
                filter: env
                    .value(ENV_LOGGER_CONSOLE_LEVEL)
                    .unwrap_or_else(|| default_filter.clone()),
                include_target: env.bool(ENV_LOGGER_CONSOLE_TARGET, true)?,
                stream: env.console_stream()?,
            })
        } else {
            None
        };

        let file_output = if env.bool(ENV_LOGGER_FILE_ENABLED, false)? {
            let Some(base_dir) = env.value(ENV_LOGGER_FILE_BASE_DIR) else {
                bail!("{ENV_LOGGER_FILE_BASE_DIR} is required when file logging is enabled");
            };

            Some(LoggerFileOutputOptions {
                base_dir: Path::from(base_dir),
                filter: env.value(ENV_LOGGER_FILE_LEVEL).unwrap_or(default_filter),
            })
        } else {
            None
        };

        let channel_capacity = match env.value(ENV_LOGGER_CHANNEL_CAPACITY) {
            Some(value) => value
                .parse()
                .map_err(|error| anyhow!("{ENV_LOGGER_CHANNEL_CAPACITY} must be a non-negative integer: {error}"))?,
            None => DEFAULT_CHANNEL_CAPACITY,
        };

        Ok(Self {
            console_output,
            file_output,
            non_blocking: LoggerNonBlockingOptions {
                channel_capacity,
                queue_full_policy: env.queue_full_policy()?,
            },
        })
    }
}

struct EnvReader<F>(F);

impl<F: Fn(&str) -> Option<String>> EnvReader<F> {
    fn bool(&self, key: &str, default: bool) -> Result<bool> {
        let Some(value) = self.value(key) else {
            return Ok(default);
        };

        match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "y" | "on" => Ok(true),
            "0" | "false" | "no" | "n" | "off" => Ok(false),
            _ => bail!("{key} must be a boolean value"),
        }
    }

    fn console_stream(&self) -> Result<LoggerConsoleStream> {
        let Some(value) = self.value(ENV_LOGGER_CONSOLE_STREAM) else {
            return Ok(LoggerConsoleStream::default());
        };

        match value.to_ascii_lowercase().as_str() {
            "stdout" => Ok(LoggerConsoleStream::Stdout),
            "stderr" => Ok(LoggerConsoleStream::Stderr),
            _ => bail!("{ENV_LOGGER_CONSOLE_STREAM} must be one of stdout, stderr"),
        }
    }

    fn queue_full_policy(&self) -> Result<LoggerQueueFullPolicy> {
        let Some(value) = self.value(ENV_LOGGER_QUEUE_FULL_POLICY) else {
            return Ok(LoggerQueueFullPolicy::default());
        };

        match value.to_ascii_lowercase().as_str() {
            "block" | "blocking" => Ok(LoggerQueueFullPolicy::Block),
            "drop" | "drop_newest" | "drop-newest" | "lossy" => Ok(LoggerQueueFullPolicy::DropNewest),
            _ => bail!("{ENV_LOGGER_QUEUE_FULL_POLICY} must be one of block, drop_newest"),
        }
    }

    fn value(&self, key: &str) -> Option<String> {
        (self.0)(key)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use pathkit::path;

    use super::*;

    fn from_pairs(pairs: &[(&str, &str)]) -> Result<LoggerInitOptions> {
        LoggerInitOptions::from_lookup(|key| {
            pairs
                .iter()
                .find(|(pair_key, _)| *pair_key == key)
                .map(|(_, value)| (*value).to_owned())
        })
    }

    #[test]
    fn from_env_applies_filter_fallbacks() -> Result<()> {
        let options = from_pairs(&[
            (ENV_LOG_FILTER, "warn,app=debug"),
            (ENV_LOGGER_CONSOLE_STREAM, "stderr"),
            (ENV_LOGGER_FILE_ENABLED, "true"),
            (ENV_LOGGER_FILE_BASE_DIR, "env-logs"),
            (ENV_LOGGER_FILE_LEVEL, "trace"),
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

        assert_eq!(console.filter, DEFAULT_FILTER);
        assert_eq!(console.stream, LoggerConsoleStream::Stdout);
        assert!(options.file_output.is_none());

        Ok(())
    }

    #[test]
    fn from_env_rejects_invalid_values() {
        assert!(from_pairs(&[(ENV_LOGGER_FILE_ENABLED, "true")]).is_err());
        assert!(from_pairs(&[(ENV_LOGGER_CONSOLE_STREAM, "stdin")]).is_err());
        assert!(from_pairs(&[(ENV_LOGGER_CONSOLE_ENABLED, "maybe")]).is_err());
        assert!(from_pairs(&[(ENV_LOGGER_CHANNEL_CAPACITY, "-1")]).is_err());
        assert!(from_pairs(&[(ENV_LOGGER_QUEUE_FULL_POLICY, "wait")]).is_err());
    }
}
