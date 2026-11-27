use std::{
    io,
    io::Write,
    mem,
    sync::{
        Arc,
        atomic::{
            AtomicU64,
            Ordering,
        },
    },
};

use tracing_subscriber::fmt::MakeWriter;

use super::options::LoggerQueueFullPolicy;

// Enums
#[derive(Debug)]
pub(super) enum LoggerCommand {
    Flush(std::sync::mpsc::SyncSender<()>),
    Shutdown,
    Write(Vec<u8>),
}

// Structs

#[derive(Debug)]
pub(super) struct QueueMakeWriter {
    dropped: Arc<AtomicU64>,
    full_policy: LoggerQueueFullPolicy,
    sender: std::sync::mpsc::SyncSender<LoggerCommand>,
}

impl<'a> MakeWriter<'a> for QueueMakeWriter {
    type Writer = QueueWriter<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        QueueWriter {
            bytes: Vec::new(),
            queue: self,
        }
    }
}

impl QueueMakeWriter {
    pub(super) fn new(
        sender: std::sync::mpsc::SyncSender<LoggerCommand>,
        full_policy: LoggerQueueFullPolicy,
        dropped: Arc<AtomicU64>,
    ) -> Self {
        Self {
            dropped,
            full_policy,
            sender,
        }
    }

    fn send(&self, command: LoggerCommand) {
        let sent = match self.full_policy {
            LoggerQueueFullPolicy::Block => self.sender.send(command).is_ok(),
            LoggerQueueFullPolicy::DropNewest => self.sender.try_send(command).is_ok(),
        };

        if !sent {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Holds one event and enqueues it on drop, so each event is one queue entry even if it arrives in several
/// writes. The fmt layer writes an event with a single `write_all`, so the buffer starts empty and takes one
/// exact-size allocation.
#[derive(Debug)]
pub(super) struct QueueWriter<'a> {
    bytes: Vec<u8>,
    queue: &'a QueueMakeWriter,
}

impl Drop for QueueWriter<'_> {
    fn drop(&mut self) {
        if !self.bytes.is_empty() {
            self.queue.send(LoggerCommand::Write(mem::take(&mut self.bytes)));
        }
    }
}

impl Write for QueueWriter<'_> {
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::{
        Result,
        bail,
    };

    use super::*;

    #[test]
    fn drop_newest_counts_dropped_events_without_blocking() -> Result<()> {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let dropped = Arc::new(AtomicU64::new(0));
        let queue = QueueMakeWriter::new(sender, LoggerQueueFullPolicy::DropNewest, Arc::clone(&dropped));

        queue.make_writer().write_all(b"first\n")?;
        queue.make_writer().write_all(b"second\n")?;
        drop(queue.make_writer());

        assert_eq!(dropped.load(Ordering::Relaxed), 1);
        let LoggerCommand::Write(bytes) = receiver.try_recv()? else {
            bail!("expected a write command");
        };

        assert_eq!(bytes, b"first\n");
        assert!(receiver.try_recv().is_err());

        Ok(())
    }
}
