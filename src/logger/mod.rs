use std::io::stdout;

use anyhow::{
    Context,
    Result,
};
use time::{
    UtcOffset,
    format_description::FormatItem,
    macros::format_description,
};
use tracing_subscriber::{
    Layer,
    Registry,
    fmt::{
        layer,
        time::OffsetTime,
    },
    layer::SubscriberExt,
    registry,
    util::SubscriberInitExt,
};

mod file;
pub mod options;
mod target;

use self::{
    options::LoggerInitOptions,
    target::TargetFileWriter,
};

type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;

// Functions
pub fn init_logger(options: LoggerInitOptions) -> Result<()> {
    let timer = make_timer();
    let mut layers = Vec::<BoxedLayer>::new();

    if let Some(console_options) = options.console_output {
        layers.push(
            layer()
                .with_timer(timer.clone())
                .with_writer(stdout)
                .with_ansi(console_options.ansi_enabled)
                .with_target(console_options.include_target)
                .with_filter(console_options.level.as_level_filter())
                .boxed(),
        );
    }

    if let Some(file_options) = options.file_output {
        let target_writer = TargetFileWriter::new(&file_options).context("failed to initialize file log writer")?;
        layers.push(
            layer()
                .with_timer(timer)
                .with_writer(target_writer)
                .with_ansi(false)
                .with_target(true)
                .with_filter(file_options.level.as_level_filter())
                .boxed(),
        );
    }

    registry()
        .with(layers)
        .try_init()
        .context("failed to initialize tracing subscriber")
}

fn make_timer() -> OffsetTime<Vec<FormatItem<'static>>> {
    let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    OffsetTime::new(
        local_offset,
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]").to_vec(),
    )
}
