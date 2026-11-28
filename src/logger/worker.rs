use std::{
    fmt::Write as _,
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
use tracing_subscriber::fmt::{
    format::Writer,
    time::FormatTime,
};

use super::{
    options::LoggerQueueOptions,
    queue::{
        LoggerCommand,
        QueueMakeWriter,
    },
    sink::{
        DROPPED_NOTICE_TARGET,
        LogSink,
    },
    timer::LoggerTimer,
};

// Constants/Statics
/// Commands handled before a forced flush, so a queue that never empties still reaches the sinks.
const MAX_BATCH_COMMANDS: usize = 1024;

// Structs
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
    fn report_and_flush(&mut self, sink: &mut impl LogSink) {
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
                    dropped_notifier.report_and_flush(&mut sink);
                    unflushed = false;
                    let _ = ack_sender.send(());
                },
                LoggerCommand::Shutdown => {
                    dropped_notifier.report_and_flush(&mut sink);
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
            dropped_notifier.report_and_flush(&mut sink);
        }
    }
}

#[cfg(test)]
mod tests {

    use super::{
        super::options::LoggerTimestampOptions,
        *,
    };

    struct RecordingSink(Arc<std::sync::Mutex<Vec<String>>>);

    impl LogSink for RecordingSink {
        fn flush(&mut self) {}

        fn write_event(&mut self, event: &[u8]) {
            if let Ok(mut lines) = self.0.lock() {
                lines.push(String::from_utf8_lossy(event).into_owned());
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
                timer: LoggerTimer::new(LoggerTimestampOptions::default()),
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
}
