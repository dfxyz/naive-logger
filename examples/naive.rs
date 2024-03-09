use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

use naive_logger::{debug, error, info, trace, warn};

fn main() {
    let raw = r#"
    level = "trace"

    [stdout]
    enable = true
    use_color = true

    [file]
    enable = true
    filename = "naive.log"
    rotate_size = "1552"
    max_rotated_num = 4
    "#;
    let conf = toml::from_str::<naive_logger::Config>(raw).unwrap();
    let _logger = naive_logger::init(&conf).unwrap();

    struct ExampleValue {
        debug_value: &'static str,
        display_value: &'static str,
    }
    impl Debug for ExampleValue {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.debug_value)
        }
    }
    impl Display for ExampleValue {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.display_value)
        }
    }

    struct ExampleError {
        value: &'static str,
    }
    impl Debug for ExampleError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "debug: {}", self.value)
        }
    }
    impl Display for ExampleError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "display: {}", self.value)
        }
    }
    impl Error for ExampleError {}

    let example_value = ExampleValue {
        debug_value: "debug",
        display_value: "display",
    };
    let example_error = ExampleError { value: "error" };
    let value_key = "key_value";
    for i in 0..10 {
        trace!("this is a trace log: {i}";
            key = "value",
            value_key,
            debug_key:? = example_value,
            display_key:% = example_value,
            error_key:? = example_error);
        debug!("this is a debug log: {i}";
            key = "value",
            value_key,
            debug_key:? = example_value,
            display_key:% = example_value,
            error_key:? = example_error);
        info!("this is a info log: {i}";
            key = "value",
            value_key,
            debug_key:? = example_value,
            display_key:% = example_value,
            error_key:? = example_error);
        warn!("this is a warn log: {i}";
            key = "value",
            value_key,
            debug_key:? = example_value,
            display_key:% = example_value,
            error_key:? = example_error);
        error!("this is a error log: {i}";
            key = "value",
            value_key,
            debug_key:? = example_value,
            display_key:% = example_value,
            error_key:? = example_error);
    }
}
