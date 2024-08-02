use log::Record;

use crate::{Datetime, Error};
use crate::config::EncoderConfig;
use crate::encoder::json::JsonEncoder;
use crate::encoder::pattern::PatternEncoder;

mod json;
mod pattern;

pub trait Encoder {
    fn encode(&self, datetime: &Datetime, record: &Record) -> String;
}

pub fn from_config(config: &EncoderConfig) -> Result<Box<dyn Encoder + Send>, Error> {
    match config {
        EncoderConfig::Pattern(config) => {
            let encoder = PatternEncoder::try_from(config)?;
            Ok(Box::new(encoder))
        }
        EncoderConfig::Json(config) => {
            let encoder = JsonEncoder::try_from(config)?;
            Ok(Box::new(encoder))
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use log::{Level, RecordBuilder};
    use log::kv::{Source, Value};

    use crate::Datetime;

    pub const TEST_TIMESTAMP: i64 = 1722400496789;
    pub const TEST_LEVEL: Level = Level::Debug;
    pub const TEST_TARGET: &str = "naive_logger::tests";
    pub const TEST_MODULE: &str = module_path!();
    pub const TEST_FILE: &str = file!();
    pub const TEST_LINE: u32 = line!();
    pub const TEST_MESSAGE: &str = "a string with \"quotes\"";
    pub const TEST_KV0: (&str, i32) = ("number", 42);
    pub const TEST_KV1: (&str, &str) = ("string", "hello");
    pub const TEST_KV2: (&str, bool) = ("boolean", true);
    pub const TEST_KV3: (&str, &[i32]) = ("vec", &[0, 1, 2, 3]);

    pub fn test_datetime() -> Datetime {
        DateTime::from_timestamp_millis(TEST_TIMESTAMP)
            .unwrap()
            .into()
    }

    pub fn prepare_test_kvs(kvs: &mut Vec<Box<dyn Source>>) {
        kvs.push(Box::new(TEST_KV0));
        kvs.push(Box::new(TEST_KV1));
        kvs.push(Box::new(TEST_KV2));
        kvs.push(Box::new((TEST_KV3.0, Value::from_serde(&TEST_KV3.1))));
    }

    pub fn prepare_test_log_record(builder: &mut RecordBuilder) {
        builder
            .target(TEST_TARGET)
            .level(TEST_LEVEL)
            .module_path(Some(TEST_MODULE))
            .file(Some(TEST_FILE))
            .line(Some(TEST_LINE))
            .build();
    }
}
