use std::collections::HashMap;

use serde::Deserialize;

pub use appender::*;
pub use encoder::*;
pub use logger::*;

mod appender;
mod encoder;
mod logger;
mod util;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub appenders: HashMap<String, AppenderConfig>,
    pub root: LoggerConfig,
    pub loggers: Vec<LoggerConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_config() {
        let config = r#"
        {
            "appenders": {
                "console": {
                    "kind": "console",
                    "encoder": {
                        "kind": "pattern",
                        "pattern": "{datetime}|{level}|{target}|{message}"
                    },
                    "stderr_level": "error"
                },
                "file": {
                    "kind": "file",
                    "path": "logs/log.log",
                    "encoder": {
                        "kind": "json"
                    },
                    "max_file_size": "1G",
                    "max_backup_index": 2
                }
            },
            "root": {
                "level": "info",
                "appenders": ["console"]
            },
            "loggers": [
                {
                    "target": "myapp::profiler",
                    "appenders": ["file"]
                },
                {
                    "target": "myapp::",
                    "target_matcher": "prefix_inverse",
                    "level": "warn"
                }
            ]
        }
        "#;
        let _config: Config = serde_json::from_str(config).unwrap();
    }
}
