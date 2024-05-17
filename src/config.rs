pub use log::LevelFilter;

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Config {
    pub level: LevelFilter,
    pub stdout: StdoutConfig,
    pub file: FileConfig,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            level: LevelFilter::Info,
            stdout: StdoutConfig::default(),
            file: FileConfig::default(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct StdoutConfig {
    pub enable: bool,
    pub use_color: bool,
}
impl Default for StdoutConfig {
    fn default() -> Self {
        Self {
            enable: true,
            use_color: true,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct FileConfig {
    pub enable: bool,
    pub filename: String,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_rotate_size"))]
    pub rotate_size: u64,
    pub max_rotated_num: u32,
}
impl Default for FileConfig {
    fn default() -> Self {
        Self {
            enable: false,
            filename: String::new(),
            rotate_size: 128 << 20,
            max_rotated_num: 1,
        }
    }
}

#[cfg(feature = "serde")]
fn deserialize_rotate_size<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                formatter,
                "a non-negative number or a string with non-negative number and unit suffix (k/K, m/M, g/G)"
            )
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if v < 0 {
                Err(serde::de::Error::custom("negative value is not allowed"))
            } else {
                Ok(v as u64)
            }
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let mut s = v.trim();
            let shift;
            if s.ends_with('k') || s.ends_with('K') {
                s = &s[..s.len() - 1];
                shift = 10;
            } else if s.ends_with('m') || s.ends_with('M') {
                s = &s[..s.len() - 1];
                shift = 20;
            } else if s.ends_with('g') || s.ends_with('G') {
                s = &s[..s.len() - 1];
                shift = 30;
            } else {
                shift = 0;
            }
            let number = s.parse::<u64>().map_err(serde::de::Error::custom)?;
            Ok(number << shift)
        }
    }

    deserializer.deserialize_any(Visitor)
}
