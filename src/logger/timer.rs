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

use super::options::{
    LoggerTimeZone,
    LoggerTimestampOptions,
};

const OFFSET_FORMAT: &[BorrowedFormatItem<'static>] =
    format_description!("[offset_hour sign:mandatory]:[offset_minute]");

const SECONDS_FORMAT: &[BorrowedFormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

thread_local! {
    static SECONDS_CACHE: RefCell<SecondsCache> = const {
        RefCell::new(SecondsCache {
            key: None,
            prefix: String::new(),
            suffix: String::new(),
        })
    };
}

/// Formats `YYYY-MM-DD HH:MM:SS.ffffff`, optionally followed by the UTC offset.
///
/// The second-resolution prefix and the offset suffix are cached per thread, and the local offset is
/// looked up again whenever they are rebuilt, so daylight saving changes apply within a second without a
/// system lookup per event. A changed system time zone is only picked up after a restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LoggerTimer {
    fallback_offset: UtcOffset,
    include_offset: bool,
    time_zone: LoggerTimeZone,
}

impl LoggerTimer {
    pub(super) fn new(options: LoggerTimestampOptions) -> Self {
        let fallback_offset = match options.time_zone {
            LoggerTimeZone::Local => UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC),
            LoggerTimeZone::Utc => UtcOffset::UTC,
        };

        Self {
            fallback_offset,
            include_offset: options.include_offset,
            time_zone: options.time_zone,
        }
    }

    fn offset_at(self, utc: OffsetDateTime) -> UtcOffset {
        match self.time_zone {
            LoggerTimeZone::Local => UtcOffset::local_offset_at(utc).unwrap_or(self.fallback_offset),
            LoggerTimeZone::Utc => UtcOffset::UTC,
        }
    }
}

impl FormatTime for LoggerTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let unix_seconds = now.as_secs();
        let key = Some((unix_seconds, *self));

        SECONDS_CACHE.with_borrow_mut(|cache| {
            // Keyed by the timer too, since every timer in the process shares this cache.
            if cache.key != key {
                let timestamp = i64::try_from(unix_seconds).map_err(|_| fmt::Error)?;
                let utc = OffsetDateTime::from_unix_timestamp(timestamp).map_err(|_| fmt::Error)?;
                let offset = self.offset_at(utc);
                let prefix = utc.to_offset(offset).format(SECONDS_FORMAT).map_err(|_| fmt::Error)?;

                cache.prefix.clear();
                cache.prefix.push_str(&prefix);
                cache.prefix.push('.');
                cache.suffix.clear();
                if self.include_offset {
                    cache
                        .suffix
                        .push_str(&offset.format(OFFSET_FORMAT).map_err(|_| fmt::Error)?);
                }

                cache.key = key;
            }

            write!(w, "{}{:06}{}", cache.prefix, now.subsec_micros(), cache.suffix)
        })
    }
}

struct SecondsCache {
    key: Option<(u64, LoggerTimer)>,
    prefix: String,
    suffix: String,
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;

    fn format(options: LoggerTimestampOptions) -> Result<String, fmt::Error> {
        let mut output = String::new();
        LoggerTimer::new(options).format_time(&mut Writer::new(&mut output))?;
        Ok(output)
    }

    fn has_shape(value: &str, pattern: &str) -> bool {
        value.len() == pattern.len()
            && value.bytes().zip(pattern.bytes()).all(|(byte, expected)| {
                if expected == b'0' {
                    byte.is_ascii_digit()
                } else {
                    byte == expected
                }
            })
    }

    fn utc_seconds_now() -> Result<String, time::error::Format> {
        OffsetDateTime::now_utc().format(SECONDS_FORMAT)
    }

    #[test]
    fn default_format_has_no_offset() -> Result<(), fmt::Error> {
        let output = format(LoggerTimestampOptions::default())?;
        assert!(has_shape(&output, "0000-00-00 00:00:00.000000"), "{output:?}");
        Ok(())
    }

    #[test]
    fn utc_formats_with_and_without_offset() -> Result<(), Box<dyn std::error::Error>> {
        let utc = LoggerTimestampOptions::default().with_time_zone(LoggerTimeZone::Utc);
        let before = utc_seconds_now()?;
        let plain = format(utc)?;
        let with_offset = format(utc.with_include_offset(true))?;
        let after = utc_seconds_now()?;

        assert!(has_shape(&plain, "0000-00-00 00:00:00.000000"), "{plain:?}");
        assert!(
            has_shape(&with_offset, "0000-00-00 00:00:00.000000+00:00"),
            "{with_offset:?}"
        );

        for output in [&plain, &with_offset] {
            let seconds = &output[..before.len()];
            assert!(
                seconds == before || seconds == after,
                "{output:?} not in {before:?}..{after:?}"
            );
        }

        Ok(())
    }

    /// Runs itself again in a child process, because the C library reads `TZ` only once per process.
    #[cfg(unix)]
    #[test]
    fn local_offset_follows_daylight_saving() {
        use std::{
            env,
            process,
        };

        use time::macros::{
            datetime,
            offset,
        };

        const CHILD_ENV: &str = "KIKIUTILS_LOGGER_DST_CHILD";

        if env::var_os(CHILD_ENV).is_some() {
            let timer = LoggerTimer::new(LoggerTimestampOptions::default());
            assert_eq!(timer.offset_at(datetime!(2026-01-15 12:00 UTC)), offset!(-5));
            assert_eq!(timer.offset_at(datetime!(2026-07-15 12:00 UTC)), offset!(-4));

            // The local timer fills the shared cache first, so the UTC timer must not reuse its entry.
            let before = utc_seconds_now().unwrap();
            let local = format(LoggerTimestampOptions::default()).unwrap();
            let utc = format(LoggerTimestampOptions::default().with_time_zone(LoggerTimeZone::Utc)).unwrap();
            let after = utc_seconds_now().unwrap();
            assert_ne!(local[..before.len()], utc[..before.len()]);
            assert!(utc.starts_with(&before) || utc.starts_with(&after), "{utc:?}");
            return;
        }

        let output = process::Command::new(env::current_exe().unwrap())
            .args(["--exact", "logger::timer::tests::local_offset_follows_daylight_saving"])
            .env(CHILD_ENV, "1")
            // A POSIX rule string, so the test does not depend on installed time zone data.
            .env("TZ", "EST5EDT,M3.2.0,M11.1.0")
            .output()
            .unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Also fails if a rename makes the child run no test at all.
        assert!(output.status.success() && stdout.contains("1 passed"), "{stdout}");
    }
}
