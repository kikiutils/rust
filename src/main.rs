use anyhow::Result;

mod logger;

use logger::config::LoggerConfig;
use logger::Logger;

fn main() -> Result<()> {
    let logger_config = LoggerConfig::default().with_target(true);
    let logger1 = Logger::new(&logger_config)?;
    let logger2 = Logger::new(&logger_config.clone().with_level(false))?;
    logger1.info("Hello");
    logger1.info("Hello");
    logger2.info("Hello");
    Ok(())
}
