use std::{
    cell::RefCell,
    fmt::{
        Error as FmtError,
        Result as FmtResult,
    },
    str::from_utf8,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

use time::{
    OffsetDateTime,
    UtcOffset,
    format_description::BorrowedFormatItem,
    macros::format_description,
};
use tracing_subscriber::fmt::{
    format::Writer,
    time::FormatTime,
};

// Constants/Statics
const SECONDS_FORMAT: &[BorrowedFormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

thread_local! {
    static SECONDS_CACHE: RefCell<SecondsCache> = const {
        RefCell::new(SecondsCache {
            prefix: String::new(),
            unix_seconds: None,
        })
    };
}

// Structs

/// Formats `YYYY-MM-DD HH:MM:SS.ffffff` in the local offset captured at initialization.
///
/// The second-resolution prefix is cached per thread and only re-formatted when the second changes.
/// The cache is not keyed by offset because the global subscriber, and therefore the timer, can only
/// be installed once per process.
#[derive(Debug, Clone, Copy)]
pub(super) struct LoggerTimer {
    offset: UtcOffset,
}

impl LoggerTimer {
    pub(super) fn new() -> Self {
        Self {
            offset: UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC),
        }
    }
}

impl FormatTime for LoggerTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> FmtResult {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let unix_seconds = now.as_secs();

        SECONDS_CACHE.with_borrow_mut(|cache| {
            if cache.unix_seconds != Some(unix_seconds) {
                let timestamp = i64::try_from(unix_seconds).map_err(|_| FmtError)?;
                let prefix = OffsetDateTime::from_unix_timestamp(timestamp)
                    .map_err(|_| FmtError)?
                    .to_offset(self.offset)
                    .format(SECONDS_FORMAT)
                    .map_err(|_| FmtError)?;

                cache.prefix.clear();
                cache.prefix.push_str(&prefix);
                cache.prefix.push('.');
                cache.unix_seconds = Some(unix_seconds);
            }

            w.write_str(&cache.prefix)
        })?;

        let mut micros = now.subsec_micros();
        let mut digits = [b'0'; 6];
        for digit in digits.iter_mut().rev() {
            *digit = b'0' + u8::try_from(micros % 10).map_err(|_| FmtError)?;
            micros /= 10;
        }

        w.write_str(from_utf8(&digits).map_err(|_| FmtError)?)
    }
}

struct SecondsCache {
    prefix: String,
    unix_seconds: Option<u64>,
}
