use anyhow::{
    Ok,
    Result,
};
use time::{
    macros::format_description,
    UtcOffset,
};
use tracing_subscriber::{
    fmt::{
        layer,
        time::OffsetTime,
    },
    layer::SubscriberExt,
    registry,
    util::SubscriberInitExt,
    Layer as TraitLayer,
    Registry,
};

pub fn init_tracing_with_layer<L: TraitLayer<Registry> + Send + Sync + 'static>(layer: L) -> Result<()> {
    registry().with(layer).try_init()?;
    Ok(())
}

pub fn init_tracing_with_local_time_format() -> Result<()> {
    init_tracing_with_layer(make_tracing_fmt_layer_with_local_time()?)
}

pub fn make_tracing_fmt_layer_with_local_time() -> Result<impl TraitLayer<Registry>> {
    let local_time_offset = UtcOffset::current_local_offset()?;
    let tracing_time_format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]");
    let tracing_timer = OffsetTime::new(local_time_offset, tracing_time_format);
    Ok(layer().with_timer(tracing_timer))
}
