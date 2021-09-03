use std::num::{NonZeroU64, NonZeroU32};

fn main() {
    let mut config = naive_logger::Config::default();
    config.level.set(log::LevelFilter::Debug);
    config.file_name = "naive.log".to_string();
    config.max_file_len = unsafe { NonZeroU64::new_unchecked(65536) };
    config.max_rotate_file_num = unsafe { NonZeroU32::new_unchecked(3) };
    naive_logger::init(&config);
    for i in 0..512 {
        log::info!("I will do whatever it takes to serve my country even at the cost of my own life, regardless of fortune or misfortune to myself. ({})", i);
        log::warn!("I will do whatever it takes to serve my country even at the cost of my own life, regardless of fortune or misfortune to myself. ({})", i);
        log::error!("I will do whatever it takes to serve my country even at the cost of my own life, regardless of fortune or misfortune to myself. ({})", i);
    }
    naive_logger::shutdown();
}
