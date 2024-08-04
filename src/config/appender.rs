use std::path::PathBuf;

use log::LevelFilter;
use serde::Deserialize;

use crate::config::EncoderConfig;

const DEFAULT_STDERR_LEVEL: LevelFilter = LevelFilter::Off;
fn default_stderr_level() -> LevelFilter {
    DEFAULT_STDERR_LEVEL
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "kind")]
pub enum AppenderConfig {
    #[serde(rename = "console")]
    Console(ConsoleAppenderConfig),
    #[serde(rename = "file")]
    File(FileAppenderConfig),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppenderCommonProperties {
    pub encoder: EncoderConfig,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsoleAppenderConfig {
    #[serde(flatten)]
    pub common: AppenderCommonProperties,
    #[serde(default = "default_stderr_level")]
    pub stderr_level: LevelFilter,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileAppenderConfig {
    #[serde(flatten)]
    pub common: AppenderCommonProperties,
    #[serde(deserialize_with = "super::util::deserialize_str_with_env_var")]
    pub path: PathBuf,
    #[serde(default, deserialize_with = "super::util::deserialize_file_size")]
    pub max_file_size: u64,
    #[serde(default)]
    pub max_backup_index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize() {
        let s = r#"{"kind": "console", "encoder": {"kind": "pattern"}, "stderr_level": "error"}"#;
        let config: AppenderConfig = serde_json::from_str(s).unwrap();
        assert!(matches!(config, AppenderConfig::Console(_)));

        let s = r#"{"kind": "file", "encoder": {"kind": "json"}, "path": "log.txt", "max_file_size": "1G", "max_backup_index": 2}"#;
        let config: AppenderConfig = serde_json::from_str(s).unwrap();
        assert!(matches!(config, AppenderConfig::File(_)));
    }
}
