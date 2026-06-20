use pathkit::Path;
use tracing_subscriber::filter::LevelFilter;

// Constants/Statics
const DEFAULT_ROTATION_MAX_BYTES: u64 = 5 * 1024 * 1024;
const DEFAULT_ROTATION_BACKUP_COUNT: usize = 5;

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
    pub fn disabled() -> Self {
        Self {
            console_output: None,
            file_output: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use pathkit::path;
    use tracing_subscriber::filter::LevelFilter;

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
}
