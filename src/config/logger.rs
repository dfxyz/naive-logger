use log::LevelFilter;
use serde::Deserialize;

const DEFAULT_LEVEL: LevelFilter = LevelFilter::Info;
fn default_level() -> LevelFilter {
    DEFAULT_LEVEL
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoggerConfig {
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub target_matcher: LoggerTargetMatcher,
    #[serde(default = "default_level")]
    pub level: LevelFilter,
    #[serde(default)]
    pub appenders: Vec<String>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum LoggerTargetMatcher {
    #[serde(rename = "prefix")]
    Prefix,
    #[serde(rename = "prefix_inverse")]
    PrefixInverse,
    #[serde(rename = "exact")]
    Exact,
}
impl Default for LoggerTargetMatcher {
    fn default() -> Self {
        Self::Prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize() {
        let s = r#"{"level": "error", "target": "myapp::handlers::", "appenders": ["console"]}"#;
        let config: LoggerConfig = serde_json::from_str(s).unwrap();
        assert_eq!(config.level, LevelFilter::Error);
        assert_eq!(config.target, "myapp::handlers::");
        assert!(matches!(config.target_matcher, LoggerTargetMatcher::Prefix));
        assert_eq!(config.appenders, vec!["console".to_string()]);
    }
}
