use std::{
    io::{
        Result as IoResult,
        Write,
        stdout,
    },
    sync::mpsc::{
        Receiver,
        SyncSender,
        TrySendError,
        sync_channel,
    },
    thread::{
        Builder,
        JoinHandle,
    },
};

use anyhow::{
    Context,
    Result,
    bail,
};
use tracing::Metadata;
use tracing_subscriber::fmt::MakeWriter;

use super::{
    options::{
        LoggerFileOutputOptions,
        LoggerNonBlockingOptions,
        LoggerQueueFullPolicy,
    },
    target::{
        DEFAULT_LOG_KEY,
        TargetFileWriter,
    },
};

// Enums
#[derive(Debug)]
enum LoggerCommand {
    Flush(SyncSender<()>),
    Shutdown(SyncSender<()>),
    Write { bytes: Vec<u8>, key: String },
}

// Structs
#[derive(Debug)]
pub(super) struct LoggerWorkerGuard {
    handle: Option<JoinHandle<()>>,
    sender: SyncSender<LoggerCommand>,
}

impl LoggerWorkerGuard {
    fn new(sender: SyncSender<LoggerCommand>, handle: JoinHandle<()>) -> Self {
        Self {
            handle: Some(handle),
            sender,
        }
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

        handle
            .join()
            .map_err(|_| anyhow::anyhow!("logger worker thread panicked"))
    }
}

#[derive(Clone, Debug)]
pub(super) struct NonBlockingConsoleWriter {
    queue: NonBlockingQueue,
}

impl<'a> MakeWriter<'a> for NonBlockingConsoleWriter {
    type Writer = NonBlockingLogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        NonBlockingLogWriter::new(self.queue.clone(), String::new())
    }
}

impl NonBlockingConsoleWriter {
    pub(super) fn spawn(options: LoggerNonBlockingOptions) -> Result<(Self, LoggerWorkerGuard)> {
        let (sender, receiver) = sync_channel(options.channel_capacity);
        let handle = Builder::new()
            .name("kikiutils-logger-console".to_string())
            .spawn(move || run_console_worker(receiver))
            .context("failed to spawn console logger worker")?;

        Ok((
            Self {
                queue: NonBlockingQueue {
                    full_policy: options.queue_full_policy,
                    sender: sender.clone(),
                },
            },
            LoggerWorkerGuard::new(sender, handle),
        ))
    }
}

#[derive(Clone, Debug)]
pub(super) struct NonBlockingFileWriter {
    queue: NonBlockingQueue,
}

impl<'a> MakeWriter<'a> for NonBlockingFileWriter {
    type Writer = NonBlockingLogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.make_writer_for_key(DEFAULT_LOG_KEY)
    }

    fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
        self.make_writer_for_key(TargetFileWriter::target_to_key(meta.target()))
    }
}

impl NonBlockingFileWriter {
    // Private methods
    fn make_writer_for_key(&self, key: impl Into<String>) -> NonBlockingLogWriter {
        NonBlockingLogWriter::new(self.queue.clone(), key.into())
    }

    // Public methods
    pub(super) fn spawn(
        file_options: LoggerFileOutputOptions,
        worker_options: LoggerNonBlockingOptions,
    ) -> Result<(Self, LoggerWorkerGuard)> {
        let writer = TargetFileWriter::new(&file_options).context("failed to initialize file log writer")?;
        let (sender, receiver) = sync_channel(worker_options.channel_capacity);
        let handle = Builder::new()
            .name("kikiutils-logger-file".to_string())
            .spawn(move || run_file_worker(receiver, writer))
            .context("failed to spawn file logger worker")?;

        Ok((
            Self {
                queue: NonBlockingQueue {
                    full_policy: worker_options.queue_full_policy,
                    sender: sender.clone(),
                },
            },
            LoggerWorkerGuard::new(sender, handle),
        ))
    }
}

#[derive(Debug)]
pub(super) struct NonBlockingLogWriter {
    key: String,
    queue: NonBlockingQueue,
}

impl Write for NonBlockingLogWriter {
    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let command = LoggerCommand::Write {
            bytes: buf.to_vec(),
            key: self.key.clone(),
        };

        match self.queue.full_policy {
            LoggerQueueFullPolicy::Block => {
                let _ = self.queue.sender.send(command);
            },
            LoggerQueueFullPolicy::DropNewest => match self.queue.sender.try_send(command) {
                Ok(()) | Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {},
            },
        }

        Ok(buf.len())
    }
}

impl NonBlockingLogWriter {
    fn new(queue: NonBlockingQueue, key: String) -> Self {
        Self { key, queue }
    }
}

#[derive(Clone, Debug)]
struct NonBlockingQueue {
    full_policy: LoggerQueueFullPolicy,
    sender: SyncSender<LoggerCommand>,
}

// Functions
pub(super) fn validate_worker_options(options: LoggerNonBlockingOptions) -> Result<()> {
    if options.channel_capacity == 0 {
        bail!("logger channel capacity must be greater than 0");
    }

    Ok(())
}

fn run_console_worker(receiver: Receiver<LoggerCommand>) {
    let mut writer = stdout();

    while let Ok(command) = receiver.recv() {
        match command {
            LoggerCommand::Flush(ack_sender) => {
                let _ = writer.flush();
                let _ = ack_sender.send(());
            },
            LoggerCommand::Shutdown(ack_sender) => {
                let _ = writer.flush();
                let _ = ack_sender.send(());
                break;
            },
            LoggerCommand::Write { bytes, .. } => {
                let _ = writer.write_all(&bytes);
            },
        }
    }
}

fn run_file_worker(receiver: Receiver<LoggerCommand>, writer: TargetFileWriter) {
    while let Ok(command) = receiver.recv() {
        match command {
            LoggerCommand::Flush(ack_sender) => {
                let _ = writer.flush_all();
                let _ = ack_sender.send(());
            },
            LoggerCommand::Shutdown(ack_sender) => {
                let _ = writer.flush_all();
                let _ = ack_sender.send(());
                break;
            },
            LoggerCommand::Write { bytes, key } => {
                let mut file = writer.resolve_writer(&key);
                let _ = file.write_all(&bytes);
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn non_blocking_writer_reports_success_when_queue_accepts_message() -> Result<()> {
        let (sender, receiver) = sync_channel(1);
        let queue = NonBlockingQueue {
            full_policy: LoggerQueueFullPolicy::DropNewest,
            sender,
        };
        let mut writer = NonBlockingLogWriter::new(queue, "target".to_string());

        assert_eq!(writer.write(b"hello")?, 5);

        match receiver.recv()? {
            LoggerCommand::Write { bytes, key } => {
                assert_eq!(bytes, b"hello");
                assert_eq!(key, "target");
            },
            LoggerCommand::Flush(_) | LoggerCommand::Shutdown(_) => panic!("expected write command"),
        }

        Ok(())
    }

    #[test]
    fn drop_newest_policy_does_not_block_or_fail_when_queue_is_full() -> Result<()> {
        let (sender, _receiver) = sync_channel(1);
        sender.send(LoggerCommand::Write {
            bytes: b"existing".to_vec(),
            key: String::new(),
        })?;
        let queue = NonBlockingQueue {
            full_policy: LoggerQueueFullPolicy::DropNewest,
            sender,
        };
        let mut writer = NonBlockingLogWriter::new(queue, String::new());

        assert_eq!(writer.write(b"dropped")?, 7);

        Ok(())
    }

    #[test]
    fn zero_channel_capacity_is_rejected() {
        let result = validate_worker_options(LoggerNonBlockingOptions {
            channel_capacity: 0,
            queue_full_policy: LoggerQueueFullPolicy::DropNewest,
        });

        assert!(result.is_err());
    }
}
