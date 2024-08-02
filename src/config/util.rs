use std::fmt::Formatter;

use serde::de::{Error, Unexpected, Visitor as VisitorTrait};
use serde::Deserializer;

pub fn deserialize_file_size<'de, D: Deserializer<'de>>(de: D) -> Result<u64, D::Error> {
    struct Visitor;
    impl<'de> VisitorTrait<'de> for Visitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            write!(
                formatter,
                "a positive number followed by an optional unit (k/K/m/M/g/G)"
            )
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: Error,
        {
            if v < 0 {
                return Err(Error::invalid_value(Unexpected::Signed(v), &self));
            }
            Ok(v as _)
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(v)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            if v.ends_with('k') || v.ends_with('K') {
                let n = v[..v.len() - 1]
                    .parse::<u64>()
                    .map_err(Error::custom)?;
                Ok(n * 1024)
            } else if v.ends_with('m') || v.ends_with('M') {
                let n = v[..v.len() - 1]
                    .parse::<u64>()
                    .map_err(Error::custom)?;
                Ok(n * 1024 * 1024)
            } else if v.ends_with('g') || v.ends_with('G') {
                let n = v[..v.len() - 1]
                    .parse::<u64>()
                    .map_err(Error::custom)?;
                Ok(n * 1024 * 1024 * 1024)
            } else {
                v.parse::<u64>().map_err(Error::custom)
            }
        }
    }
    de.deserialize_any(Visitor)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    #[test]
    fn test_deserialize_file_size() {
        #[derive(Deserialize)]
        struct Config {
            #[serde(deserialize_with = "super::deserialize_file_size")]
            size: u64,
        }

        let cases = vec![
            (r#"0"#, 0),
            (r#"1"#, 1),
            (r#""0""#, 0),
            (r#""1""#, 1),
            (r#""1k""#, 1024),
            (r#""1K""#, 1024),
            (r#""1m""#, 1024 * 1024),
            (r#""1M""#, 1024 * 1024),
            (r#""1g""#, 1024 * 1024 * 1024),
            (r#""1G""#, 1024 * 1024 * 1024),
        ];
        for (input, expected) in cases {
            let config = format!(r#"{{"size": {}}}"#, input);
            let config: Config = serde_json::from_str(&config).unwrap();
            assert_eq!(config.size, expected);
        }

        let config = r#"{"size": 3.14}"#;
        let result: Result<Config, _> = serde_json::from_str(config);
        assert!(result.is_err());

        let config = r#"{"size": -1}"#;
        let result: Result<Config, _> = serde_json::from_str(config);
        assert!(result.is_err());

        let config = r#"{"size": "-1"}"#;
        let result: Result<Config, _> = serde_json::from_str(config);
        assert!(result.is_err());

        let config = r#"{"size": "1b"}"#;
        let result: Result<Config, _> = serde_json::from_str(config);
        assert!(result.is_err());
    }
}
