use log::LevelFilter;

use naive_logger::{debug, error, info, trace, warn};

fn main() {
    let config = naive_logger::Config {
        level: LevelFilter::Trace,
        stdout: naive_logger::StdoutConfig {
            enable: false,
            ..Default::default()
        },
        file: naive_logger::FileConfig {
            enable: true,
            filename: "simple.log".to_string(),
            rotate_size: 672,
            max_rotated_num: 4,
        },
    };
    let _logger = naive_logger::init(&config).unwrap();

    for i in 0..10 {
        trace!("this is a trace log: {i}");
        debug!("this is a debug log: {i}");
        info!("this is a info log: {i}");
        warn!("this is a warn log: {i}");
        error!("this is a error log: {i}");
    }
}
