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
    trace!(target: "naive_logger::young", "this is a trace log; counter={counter}");
    counter += 1;
    
    debug!("this is a debug log; counter={}", counter);
    debug!(target: "naive_logger::young", "this is a debug log; counter={}", counter);
    counter += 1; 
    
    info!("this is a info log; counter={}", counter);
    info!(target: "naive_logger::young", "this is a info log; counter={}", counter);
    counter += 1;
    
    warn!("this is a warn log; counter={}", counter);
    warn!(target: "naive_logger::young", "this is a warn log; counter={}", counter);
    counter += 1;
    
    error!("this is a error log; counter={}", counter);
    error!(target: "naive_logger::young", "this is a error log; counter={}", counter);
}
