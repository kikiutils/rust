use anyhow::Result;
use time::macros::format_description;
use time::UtcOffset;
use tracing_subscriber::fmt::time::OffsetTime;

pub fn init_tracing_with_local_time_format() -> Result<()> {
    let local_time_offset = UtcOffset::current_local_offset()?;
    let tracing_time_format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]");
    let tracing_timer = OffsetTime::new(local_time_offset, tracing_time_format);
    tracing_subscriber::fmt().with_timer(tracing_timer).init();
    return Ok(());
}
