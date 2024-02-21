use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::Config;
use std::path::PathBuf;

const LOG_PATTERN: &str = "{d(%Y-%m-%d %H:%M:%S)} | {l} | {f}:{L} | {m}{n}";

pub fn initialize_logging() {
    let data_local_dir = if let Ok(s) = std::env::var("BRT_DATA") {
        PathBuf::from(s)
    } else {
        dirs::data_local_dir()
            .expect("Unable to find data directory for brt")
            .join("brt")
    };

    std::fs::create_dir_all(&data_local_dir)
        .unwrap_or_else(|_| panic!("Unable to create {:?}", data_local_dir));

    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(LOG_PATTERN)))
        .build(data_local_dir.join("brt.log"))
        .expect("Failed to build log file appender.");

    let levelfilter = match std::env::var("BRT_LOG_LEVEL")
        .unwrap_or_else(|_| "info".to_string())
        .as_str()
    {
        "off" => LevelFilter::Off,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    };
    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .logger(Logger::builder().build("brt", levelfilter))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))
        .expect("Failed to build logging config.");

    log4rs::init_config(config).expect("Failed to initialize logging.");
}
