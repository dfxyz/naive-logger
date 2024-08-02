use serde::Deserialize;

const DEFAULT_PATTERN: &str =
    "{datetime}|{level}|{target}|{message}{kv(|)(=)}";
fn default_pattern() -> String {
    DEFAULT_PATTERN.to_string()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "kind")]
pub enum EncoderConfig {
    #[serde(rename = "pattern")]
    Pattern(PatternEncoderConfig),
    #[serde(rename = "json")]
    Json(JsonEncoderConfig),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatternEncoderConfig {
    #[serde(default = "default_pattern")]
    pub pattern: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonEncoderConfig;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    pub fn test_deserialize() {
        let s = r#"{"kind": "pattern", "pattern": "{datetime}|{level}|{message}"}"#;
        let config: EncoderConfig = serde_json::from_str(s).unwrap();
        assert!(matches!(config, EncoderConfig::Pattern(_)));
        
        let s = r#"{"kind": "json"}"#;
        let config: EncoderConfig = serde_json::from_str(s).unwrap();
        assert!(matches!(config, EncoderConfig::Json(_)));
    }
}
