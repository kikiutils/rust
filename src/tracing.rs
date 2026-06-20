use anyhow::{
    Ok,
    Result,
};
use time::{
    UtcOffset,
    format_description::BorrowedFormatItem,
    macros::format_description,
};
use tracing_subscriber::{
    Layer as TraitLayer,
    Registry,
    fmt::{
        Layer,
        format::{
            DefaultFields,
            Format,
            Full,
        },
        layer,
        time::OffsetTime,
    },
    layer::SubscriberExt,
    registry,
    util::SubscriberInitExt,
};

// Types
type WithLocalTimeLayer =
    Layer<Registry, DefaultFields, Format<Full, OffsetTime<&'static [BorrowedFormatItem<'static>]>>>;

// Functions
pub fn init_tracing_with_layer<L: TraitLayer<Registry> + Send + Sync + 'static>(layer: L) -> Result<()> {
    registry().with(layer).try_init()?;
    Ok(())
}

pub fn init_tracing_with_local_time_format() -> Result<()> {
    init_tracing_with_layer(make_tracing_fmt_layer_with_local_time()?)
}

pub fn make_tracing_fmt_layer_with_local_time() -> Result<WithLocalTimeLayer> {
    let local_time_offset = UtcOffset::current_local_offset()?;
    let tracing_time_format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]");
    let tracing_timer = OffsetTime::new(local_time_offset, tracing_time_format);
    Ok(layer().with_timer(tracing_timer))
}
