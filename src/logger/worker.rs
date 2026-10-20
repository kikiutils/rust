use std::{
    fs::{
        File,
        OpenOptions,
    },
    io::{
        BufWriter,
        Write,
        stderr,
        stdout,
    },
    sync::{
        Arc,
        atomic::{
            AtomicU64,
            Ordering,
        },
        mpsc::{
            Receiver,
            SyncSender,
            sync_channel,
        },
    },
    thread::{
        Builder,
        JoinHandle,
    },
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

use super::{
    options::{
        LoggerConsoleStream,
        LoggerNonBlockingOptions,
    },
    queue::{
        LoggerCommand,
        QueueMakeWriter,
    },
};

// Constants/Statics
const CONSOLE_BUFFER_CAPACITY: usize = 64 * 1024;
const DEFAULT_LOG_KEY: &str = "_default";
const FALLBACK_FILE_INDEX: usize = 0;
const FALLBACK_LOG_KEY: &str = "_fallback";
const FILE_BUFFER_CAPACITY: usize = 8 * 1024;

// Traits
pub(super) trait LogSink: Send + 'static {
    fn flush(&mut self);

    fn write(&mut self, target: &[u8], line: &[u8]);
}

// Structs
pub(super) struct ConsoleSink {
    writer: BufWriter<Box<dyn Write + Send>>,
}

impl LogSink for ConsoleSink {
    fn flush(&mut self) {
        let _ = self.writer.flush();
    }

    fn write(&mut self, _: &[u8], line: &[u8]) {
        let _ = self.writer.write_all(line);
    }
}

impl ConsoleSink {
    pub(super) fn new(stream: LoggerConsoleStream) -> Self {
        let writer: Box<dyn Write + Send> = match stream {
            LoggerConsoleStream::Stdout => Box::new(stdout()),
            LoggerConsoleStream::Stderr => Box::new(stderr()),
        };

        Self {
            writer: BufWriter::with_capacity(CONSOLE_BUFFER_CAPACITY, writer),
        }
    }
}

/// Routes lines to `{base_dir}/{target key}.log`, falling back to `_fallback.log` when a target file
/// cannot be opened. Owned by a single worker thread, so no locking is needed.
pub(super) struct FileSink {
    base_dir: Path,
    dirty_indexes: Vec<usize>,
    files: Vec<SinkFile>,
    key_indexes: FxHashMap<String, usize>,
    target_indexes: FxHashMap<Box<[u8]>, usize>,
}

impl LogSink for FileSink {
    fn flush(&mut self) {
        for index in self.dirty_indexes.drain(..) {
            let file = &mut self.files[index];
            file.dirty = false;
            let _ = file.writer.flush();
        }
    }

    fn write(&mut self, target: &[u8], line: &[u8]) {
        let index = match self.target_indexes.get(target) {
            Some(&index) => index,
            None => self.resolve_target(target),
        };

        let file = &mut self.files[index];
        let _ = file.writer.write_all(line);
        if !file.dirty {
            file.dirty = true;
            self.dirty_indexes.push(index);
        }
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
            dirty_indexes: Vec::new(),
            files: vec![fallback],
            key_indexes,
            target_indexes: FxHashMap::default(),
        })
    }

    // Private methods
    /// Maps a target seen for the first time to a file index; failures are cached as the fallback file.
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
    dirty: bool,
    writer: BufWriter<File>,
}

impl SinkFile {
    fn open(path: &Path) -> Result<Self> {
        path.create_parent_dir_all_sync()?;
        let file = OpenOptions::new().create(true).append(true).open(path)?;

        Ok(Self {
            dirty: false,
            writer: BufWriter::with_capacity(FILE_BUFFER_CAPACITY, file),
        })
    }
}

#[derive(Debug)]
pub(super) struct LoggerWorkerGuard {
    dropped: Arc<AtomicU64>,
    handle: Option<JoinHandle<()>>,
    sender: SyncSender<LoggerCommand>,
}

impl LoggerWorkerGuard {
    pub(super) fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    pub(super) fn flush(&self) -> Result<()> {
        let (ack_sender, ack_receiver) = sync_channel(0);
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

        let (ack_sender, ack_receiver) = sync_channel(0);
        self.sender
            .send(LoggerCommand::Shutdown(ack_sender))
            .context("failed to send logger shutdown command")?;

        ack_receiver
            .recv()
            .context("failed to receive logger shutdown acknowledgement")?;

        handle.join().map_err(|_| anyhow!("logger worker thread panicked"))
    }
}

// Functions
pub(super) fn spawn_worker(
    name: &str,
    sink: impl LogSink,
    options: LoggerNonBlockingOptions,
    route_by_target: bool,
) -> Result<(QueueMakeWriter, LoggerWorkerGuard)> {
    let (sender, receiver) = sync_channel(options.channel_capacity);
    let handle = Builder::new()
        .name(name.to_owned())
        .spawn(move || run_worker(&receiver, sink))
        .with_context(|| format!("failed to spawn {name} worker"))?;

    let dropped = Arc::new(AtomicU64::new(0));
    Ok((
        QueueMakeWriter::new(
            sender.clone(),
            options.queue_full_policy,
            route_by_target,
            Arc::clone(&dropped),
        ),
        LoggerWorkerGuard {
            dropped,
            handle: Some(handle),
            sender,
        },
    ))
}

/// Drains every queued command before flushing, so bursts are written in batches while an idle
/// queue still flushes each line immediately. Sinks flush their buffers on drop when all senders
/// disconnect.
fn run_worker(receiver: &Receiver<LoggerCommand>, mut sink: impl LogSink) {
    while let Ok(mut command) = receiver.recv() {
        loop {
            match command {
                LoggerCommand::Flush(ack_sender) => {
                    sink.flush();
                    let _ = ack_sender.send(());
                },
                LoggerCommand::Shutdown(ack_sender) => {
                    sink.flush();
                    let _ = ack_sender.send(());
                    return;
                },
                LoggerCommand::Write { bytes, target_len } => {
                    let (target, line) = bytes.split_at(target_len);
                    sink.write(target, line);
                },
            }

            let Ok(next_command) = receiver.try_recv() else {
                break;
            };

            command = next_command;
        }

        sink.flush();
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

    #[test]
    fn target_to_key_sanitizes_and_preserves_module_hierarchy() {
        assert_eq!(target_to_key("app::endpoint::status"), "app/endpoint/status");
        assert_eq!(target_to_key("../bad::target?"), "_bad/target_");
        assert_eq!(target_to_key("...::"), DEFAULT_LOG_KEY);
    }
}
