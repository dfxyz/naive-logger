use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

use log::{debug, error, info, trace, warn};
use serde::Serialize;

use naive_logger;

const CONFIG: &str = r#"
[appenders.console]
kind = "console"
[appenders.console.encoder]
kind = "pattern"
pattern = "{colorStart}{datetime}|{level}{colorEnd}|{target}|{file}:{line}|{message}{kv(|)(=)}"

[appenders.file]
kind = "file"    
path = "examples/logs/demo.log"
max_file_size = 2128
max_backup_index = 4
[appenders.file.encoder]
kind = "json"

[appenders.foo]
kind = "file"
path = "examples/logs/demo.foo.log"
max_file_size = 1554
max_backup_index = 4
[appenders.foo.encoder]
kind = "pattern"
pattern = "{datetime(%Y-%m-%d %H:%M:%S%.3f)}|{level}|{target}|{file}:{line}|{message}{kv(|)(=)}"

[root]
level = "debug"
appenders = ["console", "file"]

[[loggers]]
target = "demo::foo"
target_matcher = "exact"
level = "trace"
appenders = ["foo"]
"#;

#[derive(Serialize, Clone)]
struct ExampleStruct {
    debug_value: &'static str,
    display_value: &'static str,
}
impl Debug for ExampleStruct {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.debug_value)
    }
}
impl Display for ExampleStruct {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_value)
    }
}

#[derive(Clone)]
struct ExampleError {
    desc: &'static str,
}
impl Debug for ExampleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "debug: {}", self.desc)
    }
}
impl Display for ExampleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "display: {}", self.desc)
    }
}
impl Error for ExampleError {}

fn main() {
    let _ = std::fs::remove_dir_all("examples/logs");
    naive_logger::init_from_toml(CONFIG).unwrap();

    let example1 = ExampleStruct {
        debug_value: "it's output with `Debug` trait",
        display_value: "it's output with `Display` trait",
    };
    let example2 = ExampleError {
        desc: r#"this is an example "error""#,
    };
    let key1: u32 = 42;
    let key2: &str = "test";
    let value = true;

    for i in 0..10 {
        trace!(target: "demo::foo", "loop count = {}", i);
        debug!(target: "demo::foo", param:? = example1, key1, key2, key3=value;
            "example message with params");
        info!(target: "demo::foo", param:% =  example1, key1, key2, key3=value;
            "example message with params");
        warn!(target: "demo::foo", param:serde = example1, key1, key2, key3=value;
            "example message with params");
        error!(target: "demo::foo", param:err = example2, key1, key2, key3=value;
            "example message with params");

        trace!("this won't be logged");
        debug!(key1, key2, key3=value, example1:?; "this is a debug log: {i}");
        info!(key1, key2, key3=value, example1:%; "this is an info log: {i}");
        scope::log(i, example1.clone(), example2.clone());
    }
}

mod scope {
    use log::{error, warn};

    use crate::{ExampleError, ExampleStruct};

    pub fn log(i: u32, example1: ExampleStruct, example2: ExampleError) {
        let key1 = "foobar";
        let key2 = 1024;
        let value = false;
        warn!(key1, key2, key3=value, example1:serde; "this is a warn log: {i}");
        error!(key1, key2, key3=value, example2:err; "this is an error log: {i}");
    }
}
