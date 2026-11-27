use std::{
    cell::RefCell,
    fmt,
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

/// Formats `YYYY-MM-DD HH:MM:SS.ffffff` in local time.
///
/// The second-resolution prefix is cached per thread, and the local offset is looked up again
/// whenever that prefix is rebuilt, so daylight saving changes apply within a second without a
/// system lookup per event. A changed system time zone is only picked up after a restart.
#[derive(Debug, Clone, Copy)]
pub(super) struct LoggerTimer {
    /// Used when a lookup fails.
    fallback_offset: UtcOffset,
}

impl LoggerTimer {
    pub(super) fn new() -> Self {
        Self {
            fallback_offset: UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC),
        }
    }
}

impl FormatTime for LoggerTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let unix_seconds = now.as_secs();

        SECONDS_CACHE.with_borrow_mut(|cache| {
            if cache.unix_seconds != Some(unix_seconds) {
                let timestamp = i64::try_from(unix_seconds).map_err(|_| fmt::Error)?;
                let utc = OffsetDateTime::from_unix_timestamp(timestamp).map_err(|_| fmt::Error)?;
                let offset = UtcOffset::local_offset_at(utc).unwrap_or(self.fallback_offset);
                let prefix = utc.to_offset(offset).format(SECONDS_FORMAT).map_err(|_| fmt::Error)?;

                cache.prefix.clear();
                cache.prefix.push_str(&prefix);
                cache.prefix.push('.');
                cache.unix_seconds = Some(unix_seconds);
            }

            w.write_str(&cache.prefix)
        })?;

        write!(w, "{:06}", now.subsec_micros())
    }
}

struct SecondsCache {
    prefix: String,
    unix_seconds: Option<u64>,
}
