use std::{
    fs::{
        File,
        OpenOptions,
    },
    io,
    io::{
        BufWriter,
        Write,
    },
};

use anyhow::Result;
use pathkit::{
    Path,
    sync_fs_ops::SyncFsOps,
};
use rustc_hash::FxHashMap;

use super::options::LoggerConsoleStream;

// Constants/Statics
const CONSOLE_BUFFER_CAPACITY: usize = 64 * 1024;
const DEFAULT_LOG_KEY: &str = "_default";
pub(super) const DROPPED_NOTICE_TARGET: &str = "kikiutils::logger";
const FALLBACK_FILE_INDEX: usize = 0;
const FALLBACK_LOG_KEY: &str = "_fallback";
const FILE_BUFFER_CAPACITY: usize = 8 * 1024;
pub(super) const TARGET_SEPARATOR: u8 = 0;

// Traits
pub(super) trait LogSink: Send + 'static {
    fn flush(&mut self);

    fn write_event(&mut self, event: &[u8]);

    fn write_notice(&mut self, line: &[u8]) {
        self.write_event(line);
    }
}

// Structs
pub(super) struct ConsoleSink {
    writer: BufWriter<Box<dyn Write + Send>>,
}

impl LogSink for ConsoleSink {
    fn flush(&mut self) {
        let _ = self.writer.flush();
    }

    fn write_event(&mut self, event: &[u8]) {
        let _ = self.writer.write_all(event);
    }
}

impl ConsoleSink {
    pub(super) fn new(stream: LoggerConsoleStream) -> Self {
        let writer: Box<dyn Write + Send> = match stream {
            LoggerConsoleStream::Stdout => Box::new(io::stdout()),
            LoggerConsoleStream::Stderr => Box::new(io::stderr()),
        };

        Self {
            writer: BufWriter::with_capacity(CONSOLE_BUFFER_CAPACITY, writer),
        }
    }
}

pub(super) struct FileSink {
    base_dir: Path,
    files: Vec<BufWriter<File>>,
    key_indexes: FxHashMap<String, usize>,
    target_indexes: FxHashMap<Box<[u8]>, usize>,
}

impl LogSink for FileSink {
    fn flush(&mut self) {
        for file in &mut self.files {
            let _ = file.flush();
        }
    }

    fn write_event(&mut self, event: &[u8]) {
        let separator = event.iter().position(|&byte| byte == TARGET_SEPARATOR);
        debug_assert!(
            separator.is_some(),
            "file events must be prefixed by TargetPrefixedFormat"
        );

        match separator {
            Some(separator) => self.write_line(&event[..separator], &event[separator + 1..]),
            None => self.write_line(b"", event),
        }
    }

    fn write_notice(&mut self, line: &[u8]) {
        self.write_line(DROPPED_NOTICE_TARGET.as_bytes(), line);
    }
}

impl FileSink {
    pub(super) fn new(base_dir: Path) -> Result<Self> {
        base_dir.create_dir_all_sync()?;
        let fallback = open_file(&(&base_dir / format!("{FALLBACK_LOG_KEY}.log")))?;

        // A target named `_fallback` must share the fallback handle instead of opening the file twice.
        let mut key_indexes = FxHashMap::default();
        key_indexes.insert(FALLBACK_LOG_KEY.to_owned(), FALLBACK_FILE_INDEX);

        Ok(Self {
            base_dir,
            files: vec![fallback],
            key_indexes,
            target_indexes: FxHashMap::default(),
        })
    }

    // Private methods
    fn write_line(&mut self, target: &[u8], line: &[u8]) {
        let index = match self.target_indexes.get(target) {
            Some(&index) => index,
            None => self.resolve_target(target),
        };

        let _ = self.files[index].write_all(line);
    }

    /// Open failures are cached as the fallback file.
    fn resolve_target(&mut self, target: &[u8]) -> usize {
        let key = target_to_key(&String::from_utf8_lossy(target));
        let index = match self.key_indexes.get(&key) {
            Some(&index) => index,
            None => match open_file(&(&self.base_dir / format!("{key}.log"))) {
                Ok(file) => {
                    self.files.push(file);
                    let index = self.files.len() - 1;
                    self.key_indexes.insert(key, index);
                    index
                },
                Err(_) => FALLBACK_FILE_INDEX,
            },
        };

        self.target_indexes.insert(target.into(), index);
        index
    }
}

// Functions
fn open_file(path: &Path) -> Result<BufWriter<File>> {
    path.create_parent_dir_all_sync()?;
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    Ok(BufWriter::with_capacity(FILE_BUFFER_CAPACITY, file))
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
        .to_owned()
}

fn target_to_key(target: &str) -> String {
    let key = target
        .split("::")
        .map(sanitize_target_segment)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("/");

    if key.is_empty() {
        DEFAULT_LOG_KEY.to_owned()
    } else {
        key
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        time::{
            SystemTime,
            UNIX_EPOCH,
        },
    };

    use pathkit::path;

    use super::*;

    #[test]
    fn file_sink_routes_events_by_target() -> Result<()> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let base_dir = path!(env::temp_dir()) / format!("kikiutils-logger-file-sink-{nanos}");
        let mut sink = FileSink::new(base_dir.clone())?;

        sink.write_event(b"x\0first\n");
        // `x.log` is a file, so `x.log/y.log` cannot be created under it.
        sink.write_event(b"x.log::y\0second\n");
        // Maps to the same key as `x`, so it must append through the same handle in order.
        sink.write_event(b"::x::\0third\n");
        sink.write_notice(b"notice\n");
        sink.flush();

        let routed = (&base_dir / "x.log").read_to_string_sync()?;
        let fallback = (&base_dir / format!("{FALLBACK_LOG_KEY}.log")).read_to_string_sync()?;
        let notice = (&base_dir / "kikiutils" / "logger.log").read_to_string_sync()?;
        base_dir.remove_dir_all_sync()?;

        assert_eq!(routed, "first\nthird\n");
        assert_eq!(fallback, "second\n");
        assert_eq!(notice, "notice\n");

        Ok(())
    }

    #[test]
    fn target_to_key_sanitizes_and_preserves_module_hierarchy() {
        assert_eq!(target_to_key("app::endpoint::status"), "app/endpoint/status");
        assert_eq!(target_to_key("../bad::target?"), "_bad/target_");
        // Windows separators and drive prefixes must not escape the segment either.
        assert_eq!(target_to_key("..\\x::C:"), "_x/C_");
        assert_eq!(target_to_key("...::"), "_default");
    }
}
