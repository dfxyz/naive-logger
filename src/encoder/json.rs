use indexmap::IndexMap;
use log::{Level, Record};
use log::kv::{Key, Value, VisitSource};
use serde::Serialize;

use crate::{Datetime, Error};
use crate::config::JsonEncoderConfig;
use crate::encoder::Encoder;

#[derive(Default)]
pub struct JsonEncoder;

impl TryFrom<&JsonEncoderConfig> for JsonEncoder {
    type Error = Error;

    fn try_from(_config: &JsonEncoderConfig) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

impl Encoder for JsonEncoder {
    fn encode(&self, datetime: &Datetime, record: &Record) -> String {
        #[derive(Default)]
        struct Visitor<'a>(IndexMap<Key<'a>, Value<'a>>);
        impl<'a> VisitSource<'a> for Visitor<'a> {
            fn visit_pair(&mut self, key: Key<'a>, value: Value<'a>) -> Result<(), log::kv::Error> {
                self.0.insert(key, value);
                Ok(())
            }
        }
        let mut visitor = Visitor::default();
        record.key_values().visit(&mut visitor).unwrap();

        #[derive(Serialize)]
        struct X<'a> {
            timestamp: i64,
            level: Level,
            target: &'a str,
            module: Option<&'a str>,
            file: Option<&'a str>,
            line: Option<u32>,
            message: &'a std::fmt::Arguments<'a>,
            args: IndexMap<Key<'a>, Value<'a>>,
        }
        let x = X {
            timestamp: datetime.timestamp_millis(),
            level: record.level(),
            target: record.target(),
            module: record.module_path(),
            file: record.file(),
            line: record.line(),
            message: record.args(),
            args: visitor.0,
        };
        serde_json::to_string(&x).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use log::RecordBuilder;

    use crate::encoder::Encoder;
    use crate::encoder::tests::*;

    #[test]
    fn test_encode() {
        let datetime = test_datetime();
        let mut builder = RecordBuilder::new();
        prepare_test_log_record(&mut builder);
        let mut kvs = Vec::new();
        prepare_test_kvs(&mut kvs);
        let encoder = super::JsonEncoder;
        let result = encoder.encode(
            &datetime,
            &builder
                .args(format_args!("{}", TEST_MESSAGE))
                .key_values(&kvs)
                .build(),
        );

        let mut expected = serde_json::Map::new();
        expected.insert("timestamp".to_string(), TEST_TIMESTAMP.into());
        expected.insert("level".to_string(), TEST_LEVEL.to_string().into());
        expected.insert("target".to_string(), TEST_TARGET.into());
        expected.insert("module".to_string(), TEST_MODULE.into());
        expected.insert("file".to_string(), TEST_FILE.into());
        expected.insert("line".to_string(), TEST_LINE.into());
        expected.insert("message".to_string(), TEST_MESSAGE.into());
        let mut expected_kvs = serde_json::Map::new();
        expected_kvs.insert(TEST_KV0.0.to_string(), TEST_KV0.1.into());
        expected_kvs.insert(TEST_KV1.0.to_string(), TEST_KV1.1.into());
        expected_kvs.insert(TEST_KV2.0.to_string(), TEST_KV2.1.into());
        expected_kvs.insert(TEST_KV3.0.to_string(), TEST_KV3.1.into());
        expected.insert("args".to_string(), serde_json::Value::Object(expected_kvs));
        let expected = serde_json::to_string(&expected).unwrap();

        assert_eq!(result, expected);
    }
}
