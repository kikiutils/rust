use std::{
    collections::HashMap,
    io::{
        Result as IoResult,
        Write,
    },
    sync::{
        Arc,
        Mutex,
        MutexGuard,
        RwLock,
    },
};

use anyhow::Result;
use pathkit::{
    Path,
    SyncFsOps,
};
use tracing::Metadata;
use tracing_subscriber::fmt::MakeWriter;

use super::{
    file::RotatingLogFile,
    options::{
        LoggerFileOutputOptions,
        LoggerFileRotationOptions,
    },
};

// Constants/Statics
pub(super) const DEFAULT_LOG_KEY: &str = "_default";
const FALLBACK_LOG_FILE_NAME: &str = "_fallback.log";

// Enums
#[derive(Debug)]
enum TargetWriterFile<'a> {
    Cached(Arc<Mutex<RotatingLogFile>>),
    Fallback(&'a Mutex<RotatingLogFile>),
}

impl TargetWriterFile<'_> {
    fn lock(&self) -> MutexGuard<'_, RotatingLogFile> {
        let file = match self {
            Self::Cached(file) => file.as_ref(),
            Self::Fallback(file) => file,
        };

        match file.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

// Structs
#[derive(Clone, Debug)]
pub(super) struct TargetFileWriter {
    state: Arc<TargetFileWriterState>,
}

impl<'a> MakeWriter<'a> for TargetFileWriter {
    type Writer = TargetWriter<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        self.resolve_writer(DEFAULT_LOG_KEY)
    }

    fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
        let key = Self::target_to_key(meta.target());
        self.resolve_writer(&key)
    }
}

impl TargetFileWriter {
    pub(super) fn new(options: &LoggerFileOutputOptions) -> Result<Self> {
        options.base_dir.create_dir_all_sync()?;
        let fallback_path = &options.base_dir / FALLBACK_LOG_FILE_NAME;
        let fallback = RotatingLogFile::new(fallback_path, options.rotation)?;

        Ok(Self {
            state: Arc::new(TargetFileWriterState {
                base_dir: options.base_dir.clone(),
                fallback: Mutex::new(fallback),
                files: RwLock::new(HashMap::new()),
                rotation_options: options.rotation,
            }),
        })
    }

    fn get_or_create_file(&self, key: &str) -> Result<Arc<Mutex<RotatingLogFile>>> {
        if let Some(file) = match self.state.files.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
        .get(key)
        {
            return Ok(file.clone());
        }

        let mut files = match self.state.files.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if let Some(file) = files.get(key) {
            return Ok(file.clone());
        }

        let path = &self.state.base_dir / format!("{key}.log");
        let file = Arc::new(Mutex::new(RotatingLogFile::new(path, self.state.rotation_options)?));

        files.insert(key.to_string(), file.clone());
        Ok(file)
    }

    // Public methods
    pub(super) fn flush_all(&self) -> IoResult<()> {
        self.state.lock_fallback().flush()?;

        let files = match self.state.files.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        for file in files.values() {
            lock_rotating_file(file).flush()?;
        }

        Ok(())
    }

    pub(super) fn resolve_writer(&self, key: &str) -> TargetWriter<'_> {
        let file = self
            .get_or_create_file(key)
            .map(TargetWriterFile::Cached)
            .unwrap_or_else(|_| TargetWriterFile::Fallback(&self.state.fallback));

        TargetWriter { file }
    }

    pub(super) fn target_to_key(target: &str) -> String {
        let key = target
            .split("::")
            .map(sanitize_target_segment)
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join("/");

        if key.is_empty() { "_default".to_string() } else { key }
    }
}

#[derive(Debug)]
struct TargetFileWriterState {
    base_dir: Path,
    fallback: Mutex<RotatingLogFile>,
    files: RwLock<HashMap<String, Arc<Mutex<RotatingLogFile>>>>,
    rotation_options: Option<LoggerFileRotationOptions>,
}

impl TargetFileWriterState {
    fn lock_fallback(&self) -> MutexGuard<'_, RotatingLogFile> {
        lock_rotating_file(&self.fallback)
    }
}

#[derive(Debug)]
pub(super) struct TargetWriter<'a> {
    file: TargetWriterFile<'a>,
}

impl Write for TargetWriter<'_> {
    fn flush(&mut self) -> IoResult<()> {
        self.file.lock().flush()
    }

    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.file.lock().write(buf)
    }
}

// Functions
fn lock_rotating_file(file: &Mutex<RotatingLogFile>) -> MutexGuard<'_, RotatingLogFile> {
    match file.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn sanitize_target_segment(value: &str) -> String {
    value
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() || matches!(value, '-' | '_' | '.') {
                value
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        time::{
            SystemTime,
            UNIX_EPOCH,
        },
    };

    use anyhow::Result;
    use pathkit::{
        Path,
        SyncFsOps,
        path,
    };

    use super::{
        super::options::LoggerLogLevel,
        *,
    };

    #[test]
    fn target_to_key_sanitizes_and_preserves_module_hierarchy() {
        assert_eq!(
            TargetFileWriter::target_to_key("endpoint_control_program_runtime::endpoint::status"),
            "endpoint_control_program_runtime/endpoint/status"
        );

        assert_eq!(TargetFileWriter::target_to_key("../bad::target?"), "_bad/target_");
        assert_eq!(TargetFileWriter::target_to_key("...::"), "_default");
    }

    #[test]
    fn target_file_writer_writes_default_and_cached_target_files() -> Result<()> {
        let log_dir = unique_temp_dir("target-writer");
        let writer = TargetFileWriter::new(&LoggerFileOutputOptions {
            base_dir: log_dir.clone(),
            level: LoggerLogLevel::Trace,
            rotation: None,
        })?;

        writer.make_writer().write_all(b"default")?;
        writer.resolve_writer("module/cache").write_all(b"first")?;
        writer.resolve_writer("module/cache").write_all(b"second")?;

        writer.flush_all()?;

        assert_eq!(
            (&log_dir / format!("{DEFAULT_LOG_KEY}.log")).read_to_string_sync()?,
            "default"
        );

        assert_eq!(
            (&log_dir / "module" / "cache.log").read_to_string_sync()?,
            "firstsecond"
        );

        log_dir.remove_dir_all_sync()?;
        Ok(())
    }

    #[test]
    fn fallback_writer_uses_shared_fallback_file() -> Result<()> {
        let temp_dir = unique_temp_dir("fallback");
        let fallback_path = &temp_dir / FALLBACK_LOG_FILE_NAME;
        let fallback = fallback_mutex(RotatingLogFile::new(fallback_path.clone(), None)?);
        let file = TargetWriterFile::Fallback(&fallback);
        let mut writer = TargetWriter { file };

        writer.write_all(b"fallback")?;
        writer.flush()?;

        assert_eq!(fallback_path.read_to_string_sync()?, "fallback");

        temp_dir.remove_dir_all_sync()?;
        Ok(())
    }

    fn fallback_mutex(file: RotatingLogFile) -> Mutex<RotatingLogFile> {
        Mutex::new(file)
    }

    fn unique_temp_dir(label: &str) -> Path {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();

        path!(temp_dir()) / format!("kikiutils-logger-target-{label}-{nanos}")
    }
}
