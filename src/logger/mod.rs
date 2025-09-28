use anyhow::Result;
use time::macros::format_description;
use time::UtcOffset;
use tracing::dispatcher::with_default;
use tracing::Dispatch;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::time::OffsetTime;

pub mod config;

use config::LoggerConfig;

#[derive(Clone)]
pub struct Logger {
    dispatch: Dispatch,
}

impl Logger {
    pub fn new(config: &LoggerConfig) -> Result<Self> {
        let default_timer = {
            let local_time_offset = UtcOffset::current_local_offset()?;
            let tracing_time_format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]");
            OffsetTime::new(local_time_offset, tracing_time_format)
        };

        let subscriber_builder = fmt()
            .with_level(config.display_level)
            .with_target(config.display_target)
            .with_timer(default_timer);

        Ok(Self {
            dispatch: Dispatch::new(subscriber_builder.finish()),
        })
    }

    pub fn error(&self, msg: &str) {
        with_default(&self.dispatch, || tracing::error!("{}", msg));
    }

    pub fn info(&self, msg: &str) {
        with_default(&self.dispatch, || tracing::info!("{}", msg));
    }

    pub fn warn(&self, msg: &str) {
        with_default(&self.dispatch, || tracing::warn!("{}", msg));
    }
}
