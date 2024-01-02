use log::{debug, error, info, trace, warn, LevelFilter};

fn main() {
    let config = naive_logger::Config {
        level: LevelFilter::Trace,
        ..Default::default()
    };
    let _logger = naive_logger::init(&config).unwrap();

    trace!("this is a trace log");
    debug!("this is a debug log");
    info!("this is a info log");
    warn!("this is a warn log");
    error!("this is a error log");
}
