use std::{
    fmt::Write as _,
    fs::{
        File,
        OpenOptions,
    },
    io,
    io::{
        BufWriter,
        Write,
    },
    sync::{
        Arc,
        atomic::{
            AtomicU64,
            Ordering,
        },
    },
    thread,
};

use anyhow::{
    Context,
    Result,
    anyhow,
};
use pathkit::{
    Path,
    SyncFsOps,
};
use rustc_hash::FxHashMap;
use tracing_subscriber::fmt::{
    format::Writer,
    time::FormatTime,
};

use super::{
    options::{
        LoggerConsoleStream,
        LoggerQueueOptions,
    },
    queue::{
        LoggerCommand,
        QueueMakeWriter,
    },
    timer::LoggerTimer,
};

// Constants/Statics
const CONSOLE_BUFFER_CAPACITY: usize = 64 * 1024;
const DEFAULT_LOG_KEY: &str = "_default";
const DROPPED_NOTICE_TARGET: &str = "kikiutils::logger";
const FALLBACK_FILE_INDEX: usize = 0;
const FALLBACK_LOG_KEY: &str = "_fallback";
const FILE_BUFFER_CAPACITY: usize = 8 * 1024;
/// Commands handled before a forced flush, so a queue that never empties still reaches the sinks.
const MAX_BATCH_COMMANDS: usize = 1024;
pub(super) const TARGET_SEPARATOR: u8 = 0;

// Traits
pub(super) trait LogSink: Send + 'static {
    fn flush(&mut self);

    fn write_event(&mut self, event: &[u8]);

    fn write_notice(&mut self, line: &[u8]);
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

    fn write_notice(&mut self, line: &[u8]) {
        let _ = self.writer.write_all(line);
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
    files: Vec<SinkFile>,
    key_indexes: FxHashMap<String, usize>,
    target_indexes: FxHashMap<Box<[u8]>, usize>,
}

impl LogSink for FileSink {
    fn flush(&mut self) {
        for file in &mut self.files {
            let _ = file.writer.flush();
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
        let fallback = SinkFile::open(&(&base_dir / format!("{FALLBACK_LOG_KEY}.log")))?;

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

        let _ = self.files[index].writer.write_all(line);
    }

    /// Open failures are cached as the fallback file.
    fn resolve_target(&mut self, target: &[u8]) -> usize {
        let key = target_to_key(&String::from_utf8_lossy(target));
        let index = match self.key_indexes.get(&key) {
            Some(&index) => index,
            None => match SinkFile::open(&(&self.base_dir / format!("{key}.log"))) {
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

struct SinkFile {
    writer: BufWriter<File>,
}

impl SinkFile {
    fn open(path: &Path) -> Result<Self> {
        path.create_parent_dir_all_sync()?;
        let file = OpenOptions::new().create(true).append(true).open(path)?;

        Ok(Self {
            writer: BufWriter::with_capacity(FILE_BUFFER_CAPACITY, file),
        })
    }
}

#[derive(Debug)]
pub(super) struct LoggerWorkerGuard {
    dropped: Arc<AtomicU64>,
    handle: Option<thread::JoinHandle<()>>,
    sender: std::sync::mpsc::SyncSender<LoggerCommand>,
}

impl LoggerWorkerGuard {
    pub(super) fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    pub(super) fn flush(&self) -> Result<()> {
        let (ack_sender, ack_receiver) = std::sync::mpsc::sync_channel(0);
        self.sender
            .send(LoggerCommand::Flush(ack_sender))
            .context("failed to send logger flush command")?;

        ack_receiver
            .recv()
            .context("failed to receive logger flush acknowledgement")
    }

    pub(super) fn shutdown(&mut self) -> Result<()> {
        let Some(handle) = self.handle.take() else {
            return Ok(());
        };

        // A failed send means the worker already exited; joining still reports a panic.
        let _ = self.sender.send(LoggerCommand::Shutdown);
        handle.join().map_err(|_| anyhow!("logger worker thread panicked"))
    }
}

struct DroppedNotifier {
    dropped: Arc<AtomicU64>,
    reported: u64,
    timer: LoggerTimer,
}

impl DroppedNotifier {
    fn flush(&mut self, sink: &mut impl LogSink) {
        let dropped = self.dropped.load(Ordering::Relaxed);
        if dropped > self.reported {
            self.write_notice(sink, dropped);
        }

        sink.flush();
    }

    fn write_notice(&mut self, sink: &mut impl LogSink, dropped: u64) {
        let mut line = String::new();
        if self.timer.format_time(&mut Writer::new(&mut line)).is_err() {
            line.clear();
        }

        let count = dropped - self.reported;
        let _ = writeln!(
            line,
            "  WARN {DROPPED_NOTICE_TARGET}: dropped {count} log events because the queue was full"
        );

        sink.write_notice(line.as_bytes());
        self.reported = dropped;
    }
}

// Functions
pub(super) fn spawn_worker(
    name: &str,
    sink: impl LogSink,
    options: LoggerQueueOptions,
    timer: LoggerTimer,
) -> Result<(QueueMakeWriter, LoggerWorkerGuard)> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(options.capacity.get());
    let dropped = Arc::new(AtomicU64::new(0));
    let worker_dropped = Arc::clone(&dropped);
    let handle = thread::Builder::new()
        .name(name.to_owned())
        .spawn(move || {
            run_worker(
                &receiver,
                sink,
                DroppedNotifier {
                    dropped: worker_dropped,
                    reported: 0,
                    timer,
                },
            );
        })
        .with_context(|| format!("failed to spawn {name} worker"))?;

    Ok((
        QueueMakeWriter::new(sender.clone(), options.full_policy, Arc::clone(&dropped)),
        LoggerWorkerGuard {
            dropped,
            handle: Some(handle),
            sender,
        },
    ))
}

/// Drains queued commands before flushing, so bursts are written in batches while an idle queue still
/// flushes each line immediately. Sinks flush their buffers on drop when all senders disconnect.
fn run_worker(
    receiver: &std::sync::mpsc::Receiver<LoggerCommand>,
    mut sink: impl LogSink,
    mut dropped_notifier: DroppedNotifier,
) {
    while let Ok(command) = receiver.recv() {
        let mut next_command = Some(command);
        let mut handled = 0;
        let mut unflushed = false;
        while let Some(command) = next_command.take() {
            match command {
                LoggerCommand::Flush(ack_sender) => {
                    dropped_notifier.flush(&mut sink);
                    unflushed = false;
                    let _ = ack_sender.send(());
                },
                LoggerCommand::Shutdown => {
                    dropped_notifier.flush(&mut sink);
                    return;
                },
                LoggerCommand::Write(bytes) => {
                    sink.write_event(&bytes);
                    unflushed = true;
                },
            }

            handled += 1;
            if handled < MAX_BATCH_COMMANDS {
                next_command = receiver.try_recv().ok();
            }
        }

        if unflushed {
            dropped_notifier.flush(&mut sink);
        }
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

    use super::*;

    struct RecordingSink(Arc<std::sync::Mutex<Vec<String>>>);

    impl LogSink for RecordingSink {
        fn flush(&mut self) {}

        fn write_event(&mut self, event: &[u8]) {
            self.write_notice(event);
        }

        fn write_notice(&mut self, line: &[u8]) {
            if let Ok(mut lines) = self.0.lock() {
                lines.push(String::from_utf8_lossy(line).into_owned());
            }
        }
    }

    #[test]
    fn worker_reports_newly_dropped_events_once() -> Result<()> {
        let lines = Arc::new(std::sync::Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicU64::new(3));
        let (sender, receiver) = std::sync::mpsc::sync_channel(4);
        let (ack_sender, ack_receiver) = std::sync::mpsc::sync_channel(4);
        sender.send(LoggerCommand::Flush(ack_sender.clone()))?;
        sender.send(LoggerCommand::Flush(ack_sender.clone()))?;
        drop(ack_sender);
        sender.send(LoggerCommand::Shutdown)?;

        run_worker(
            &receiver,
            RecordingSink(Arc::clone(&lines)),
            DroppedNotifier {
                dropped,
                reported: 0,
                timer: LoggerTimer::new(),
            },
        );

        assert_eq!(ack_receiver.try_iter().count(), 2);
        let lines = lines.lock().map_err(|_| anyhow!("recording sink lock poisoned"))?;
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(
            lines[0].ends_with("  WARN kikiutils::logger: dropped 3 log events because the queue was full\n"),
            "{lines:?}"
        );

        Ok(())
    }

    #[test]
    fn target_to_key_sanitizes_and_preserves_module_hierarchy() {
        assert_eq!(target_to_key("app::endpoint::status"), "app/endpoint/status");
        assert_eq!(target_to_key("../bad::target?"), "_bad/target_");
        assert_eq!(target_to_key("...::"), "_default");
    }
}
