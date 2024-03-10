use log::LevelFilter;

use naive_logger::{debug, error, info, trace, warn};

fn main() {
    let config = naive_logger::Config {
        level: LevelFilter::Trace,
        ..Default::default()
    };
    let _logger = naive_logger::init(&config).unwrap();

    let mut counter = 0;

    trace!("this is a trace log; counter={}", counter);
    counter += 1;

    debug!("this is a debug log; counter={}", counter);
    counter += 1;

    info!("this is a info log; counter={}", counter);
    counter += 1;

    warn!("this is a warn log; counter={}", counter);
    counter += 1;

    error!("this is a error log; counter={}", counter);
}
