use kv_log_macro::{debug, error, info, trace, warn};

fn main() {
    let raw = r#"
    level = "trace"

    [stdout]
    enable = true
    use_color = true

    [file]
    enable = true
    filename = "naive.log"
    rotate_size = "902"
    max_rotated_num = 4
    "#;
    let conf = toml::from_str::<naive_logger::Config>(raw).unwrap();
    let _logger = naive_logger::init(&conf).unwrap();

    for i in 0..10 {
        trace!("this is a trace log: {i}", { key1: "value1", key2: "value2" });
        debug!("this is a debug log: {i}", { key1: "value1", key2: "value2"});
        info!("this is a info log: {i}", { key1: "value1", key2: "value2" });
        warn!("this is a warn log: {i}", { key1: "value1", key2: "value2" });
        error!("this is a error log: {i}", { key1: "value1", key2: "value2" });
    }
}
