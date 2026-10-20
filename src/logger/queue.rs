use std::{
    io::{
        Result as IoResult,
        Write,
    },
    mem::take,
    sync::{
        Arc,
        atomic::{
            AtomicU64,
            Ordering,
        },
        mpsc::SyncSender,
    },
};

use tracing::Metadata;
use tracing_subscriber::fmt::MakeWriter;

use super::options::LoggerQueueFullPolicy;

// Constants/Statics
const LINE_CAPACITY_HINT: usize = 256;

// Enums
#[derive(Debug)]
pub(super) enum LoggerCommand {
    Flush(SyncSender<()>),
    Shutdown(SyncSender<()>),
    /// `bytes[..target_len]` is the raw event target, `bytes[target_len..]` is the formatted line.
    Write {
        bytes: Vec<u8>,
        target_len: usize,
    },
}

// Structs

/// Hands each formatted event to a background worker with a single allocation on the caller thread.
#[derive(Debug)]
pub(super) struct QueueMakeWriter {
    dropped: Arc<AtomicU64>,
    full_policy: LoggerQueueFullPolicy,
    route_by_target: bool,
    sender: SyncSender<LoggerCommand>,
}

impl<'a> MakeWriter<'a> for QueueMakeWriter {
    type Writer = QueueWriter<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        QueueWriter {
            bytes: Vec::with_capacity(LINE_CAPACITY_HINT),
            queue: self,
            target_len: 0,
        }
    }

    fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
        if !self.route_by_target {
            return self.make_writer();
        }

        let target = meta.target().as_bytes();
        let mut bytes = Vec::with_capacity(target.len() + LINE_CAPACITY_HINT);
        bytes.extend_from_slice(target);
        QueueWriter {
            bytes,
            queue: self,
            target_len: target.len(),
        }
    }
}

impl QueueMakeWriter {
    pub(super) fn new(
        sender: SyncSender<LoggerCommand>,
        full_policy: LoggerQueueFullPolicy,
        route_by_target: bool,
        dropped: Arc<AtomicU64>,
    ) -> Self {
        Self {
            dropped,
            full_policy,
            route_by_target,
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

/// Buffers one event and enqueues it on drop, so a multi-call `write_all` still produces one queue entry.
#[derive(Debug)]
pub(super) struct QueueWriter<'a> {
    bytes: Vec<u8>,
    queue: &'a QueueMakeWriter,
    target_len: usize,
}

impl Drop for QueueWriter<'_> {
    fn drop(&mut self) {
        if self.bytes.len() > self.target_len {
            self.queue.send(LoggerCommand::Write {
                bytes: take(&mut self.bytes),
                target_len: self.target_len,
            });
        }
    }
}

impl Write for QueueWriter<'_> {
    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::sync_channel;

    use anyhow::{
        Result,
        bail,
    };

    use super::*;

    #[test]
    fn drop_newest_counts_dropped_events_without_blocking() -> Result<()> {
        let (sender, receiver) = sync_channel(1);
        let dropped = Arc::new(AtomicU64::new(0));
        let queue = QueueMakeWriter::new(sender, LoggerQueueFullPolicy::DropNewest, false, Arc::clone(&dropped));

        queue.make_writer().write_all(b"first\n")?;
        queue.make_writer().write_all(b"second\n")?;
        drop(queue.make_writer());

        assert_eq!(dropped.load(Ordering::Relaxed), 1);
        let LoggerCommand::Write { bytes, target_len } = receiver.try_recv()? else {
            bail!("expected a write command");
        };

        assert_eq!((bytes.as_slice(), target_len), (b"first\n".as_slice(), 0));
        assert!(receiver.try_recv().is_err());

        Ok(())
    }
}
