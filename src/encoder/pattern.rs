use std::fmt::Write;
use std::mem::swap;

use log::kv::VisitSource;
use log::Record;

use crate::{Datetime, Error};
use crate::config::PatternEncoderConfig;
use crate::encoder::Encoder;

const DEFAULT_DATETIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3f%z";

const UNKNOWN_MODULE: &str = "<unknown>";
const UNKNOWN_FILE: &str = "<unknown>";
const UNKNOWN_LINE: u32 = 0;

const ANSI_COLOR_RESET: &str = "\x1b[0m";
const ANSI_COLOR_RED: &str = "\x1b[31m";
const ANSI_COLOR_GREEN: &str = "\x1b[32m";
const ANSI_COLOR_YELLOW: &str = "\x1b[33m";
const ANSI_COLOR_BLUE: &str = "\x1b[34m";
const ANSI_COLOR_MAGENTA: &str = "\x1b[35m";

fn level2color(level: log::Level) -> &'static str {
    match level {
        log::Level::Error => ANSI_COLOR_RED,
        log::Level::Warn => ANSI_COLOR_YELLOW,
        log::Level::Info => ANSI_COLOR_GREEN,
        log::Level::Debug => ANSI_COLOR_BLUE,
        log::Level::Trace => ANSI_COLOR_MAGENTA,
    }
}

pub struct PatternEncoder {
    placeholders: Vec<Placeholder>,
}

enum Placeholder {
    Literal {
        content: String,
    },
    Datetime {
        format: String,
    },
    Level,
    Target,
    Module,
    File,
    Line,
    Message,
    KeyValuePairs {
        pair_separator: String,
        kv_separator: String,
    },
    ColorStart,
    ColorEnd,
}

impl TryFrom<&PatternEncoderConfig> for PatternEncoder {
    type Error = Error;

    fn try_from(config: &PatternEncoderConfig) -> Result<Self, Self::Error> {
        let placeholders =
            parse_placeholders(&config.pattern).map_err(|e| e.concat("invalid pattern"))?;
        Ok(Self { placeholders })
    }
}

fn parse_placeholders(s: &str) -> Result<Vec<Placeholder>, Error> {
    let mut placeholders = vec![];

    enum State {
        CollectLiteral,            // until '{'
        CollectPlaceholder,        // until '(' or '}'
        CollectPlaceholderArg,     // until ')'
        CollectNextPlaceholderArg, // until '(' or '}'
    }

    let mut state = State::CollectLiteral;
    let mut tmp = String::new();
    let mut placeholder_name = String::new();
    let mut placeholder_args = Vec::<String>::new();
    for (i, char) in s.chars().enumerate() {
        match state {
            State::CollectLiteral => {
                if char == '{' {
                    if !tmp.is_empty() {
                        let mut content = String::new();
                        swap(&mut content, &mut tmp);
                        placeholders.push(Placeholder::Literal { content });
                    }
                    state = State::CollectPlaceholder;
                    continue;
                }
                tmp.push(char);
            }
            State::CollectPlaceholder => {
                if char == '}' {
                    let empty: &[&str] = &[];
                    let placeholder = Placeholder::try_from((&tmp, empty)).map_err(|e| {
                        Error::from(format!("placeholder ending at character #{}: {}", i, e))
                    })?;
                    placeholders.push(placeholder);
                    tmp.clear();
                    state = State::CollectLiteral;
                    continue;
                }
                if char == '(' {
                    swap(&mut placeholder_name, &mut tmp);
                    state = State::CollectPlaceholderArg;
                    continue;
                }
                tmp.push(char);
            }
            State::CollectPlaceholderArg => {
                if char == ')' {
                    let mut arg = String::new();
                    swap(&mut arg, &mut tmp);
                    placeholder_args.push(arg);
                    state = State::CollectNextPlaceholderArg;
                    continue;
                }
                tmp.push(char);
            }
            State::CollectNextPlaceholderArg => {
                if char == '(' {
                    state = State::CollectPlaceholderArg;
                    continue;
                }
                if char == '}' {
                    let placeholder =
                        Placeholder::try_from((&placeholder_name, placeholder_args.as_slice()))
                            .map_err(|e| {
                                Error::from(format!(
                                    "placeholder ending at character #{}: {}",
                                    i, e
                                ))
                            })?;
                    placeholders.push(placeholder);
                    placeholder_name.clear();
                    placeholder_args.clear();
                    state = State::CollectLiteral;
                    continue;
                }
                return Err(Error::from(format!(
                    "expecting '(' or '}}' at character #{}",
                    i
                )));
            }
        }
    }
    match state {
        State::CollectLiteral => {
            if !tmp.is_empty() {
                placeholders.push(Placeholder::Literal { content: tmp });
            }
        }
        _ => {
            return Err(Error::from(
                "incomplete placeholder at the end of the pattern",
            ));
        }
    }

    Ok(placeholders)
}

impl<S1: AsRef<str>, S2: AsRef<str>> TryFrom<(S1, &[S2])> for Placeholder {
    type Error = &'static str;

    fn try_from(tuple: (S1, &[S2])) -> Result<Self, Self::Error> {
        let name = tuple.0.as_ref();
        let args = tuple.1;

        match name {
            x if x == "datetime" => {
                if args.len() > 1 {
                    return Err("expecting at most one argument");
                }
                let format = args
                    .get(0)
                    .map(|x| x.as_ref())
                    .unwrap_or(DEFAULT_DATETIME_FORMAT);
                Ok(Placeholder::Datetime {
                    format: format.to_string(),
                })
            }
            x if x == "level" => {
                if !args.is_empty() {
                    return Err("expecting no argument");
                }
                Ok(Placeholder::Level)
            }
            x if x == "target" => {
                if !args.is_empty() {
                    return Err("expecting no argument");
                }
                Ok(Placeholder::Target)
            }
            x if x == "module" => {
                if !args.is_empty() {
                    return Err("expecting no argument");
                }
                Ok(Placeholder::Module)
            }
            x if x == "file" => {
                if !args.is_empty() {
                    return Err("expecting no argument");
                }
                Ok(Placeholder::File)
            }
            x if x == "line" => {
                if !args.is_empty() {
                    return Err("expecting no argument");
                }
                Ok(Placeholder::Line)
            }
            x if x == "message" => {
                if !args.is_empty() {
                    return Err("expecting no argument");
                }
                Ok(Placeholder::Message)
            }
            x if x == "kv" => {
                if args.len() != 2 {
                    return Err("expecting exactly two arguments");
                }
                let pair_separator = args[0].as_ref();
                let kv_separator = args[1].as_ref();
                Ok(Placeholder::KeyValuePairs {
                    pair_separator: pair_separator.to_string(),
                    kv_separator: kv_separator.to_string(),
                })
            }
            x if x == "colorStart" => {
                if !args.is_empty() {
                    return Err("expecting no argument");
                }
                Ok(Placeholder::ColorStart)
            }
            x if x == "colorEnd" => {
                if !args.is_empty() {
                    return Err("expecting no argument");
                }
                Ok(Placeholder::ColorEnd)
            }
            _ => {
                return Err("unknown placeholder name");
            }
        }
    }
}

impl Encoder for PatternEncoder {
    fn encode(&self, datetime: &Datetime, record: &Record) -> String {
        let mut result = String::new();
        for placeholder in &self.placeholders {
            match placeholder {
                Placeholder::Literal { content } => {
                    write!(result, "{}", content).unwrap();
                }
                Placeholder::Datetime { format } => {
                    write!(result, "{}", datetime.format(format)).unwrap();
                }
                Placeholder::Level => {
                    write!(result, "{}", record.level()).unwrap();
                }
                Placeholder::Target => {
                    write!(result, "{}", record.target()).unwrap();
                }
                Placeholder::Module => {
                    let module = record.module_path().unwrap_or(UNKNOWN_MODULE);
                    write!(result, "{}", module).unwrap();
                }
                Placeholder::File => {
                    let file = record.file().unwrap_or(UNKNOWN_FILE);
                    write!(result, "{}", file).unwrap();
                }
                Placeholder::Line => {
                    let line = record.line().unwrap_or(UNKNOWN_LINE);
                    write!(result, "{}", line).unwrap();
                }
                Placeholder::Message => {
                    write!(result, "{}", record.args()).unwrap();
                }
                Placeholder::KeyValuePairs {
                    kv_separator,
                    pair_separator,
                } => {
                    struct Visitor<'a> {
                        pair_separator: &'a str,
                        kv_separator: &'a str,
                        result: &'a mut String,
                    }
                    impl<'a> VisitSource<'a> for Visitor<'a> {
                        fn visit_pair(
                            &mut self,
                            key: log::kv::Key,
                            value: log::kv::Value,
                        ) -> Result<(), log::kv::Error> {
                            write!(
                                self.result,
                                "{}{}{}{}",
                                self.pair_separator,
                                key,
                                self.kv_separator,
                                serde_json::to_string(&value).unwrap()
                            )
                            .unwrap();
                            Ok(())
                        }
                    }
                    let mut visitor = Visitor {
                        pair_separator,
                        kv_separator,
                        result: &mut result,
                    };
                    record.key_values().visit(&mut visitor).unwrap();
                }
                Placeholder::ColorStart => {
                    write!(result, "{}", level2color(record.level())).unwrap();
                }
                Placeholder::ColorEnd => {
                    write!(result, "{}", ANSI_COLOR_RESET).unwrap();
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use log::RecordBuilder;

    use crate::encoder::Encoder;
    use crate::encoder::pattern::DEFAULT_DATETIME_FORMAT;
    use crate::encoder::tests::*;

    #[test]
    fn test_parse_placeholder() {
        let empty: &[&str] = &[];
        let tuple = ("invalid", empty);
        let result = super::Placeholder::try_from(tuple);
        assert!(result.is_err());

        let tuple = ("datetime", empty);
        let placeholder = super::Placeholder::try_from(tuple).unwrap();
        assert!(
            matches!(placeholder, super::Placeholder::Datetime { format } if format == DEFAULT_DATETIME_FORMAT)
        );
        let datetime_format = "%Y-%m-%d %H:%M:%S%.3f";
        let tuple = ("datetime", &[datetime_format][..]);
        let placeholder = super::Placeholder::try_from(tuple).unwrap();
        assert!(
            matches!(placeholder, super::Placeholder::Datetime { format } if format == datetime_format)
        );
        let tuple = ("datetime", &["", ""][..]);
        let result = super::Placeholder::try_from(tuple);
        assert!(result.is_err());

        let tuple = ("level", empty);
        let placeholder = super::Placeholder::try_from(tuple).unwrap();
        assert!(matches!(placeholder, super::Placeholder::Level));
        let tuple = ("level", &[""][..]);
        let result = super::Placeholder::try_from(tuple);
        assert!(result.is_err());

        let tuple = ("target", empty);
        let placeholder = super::Placeholder::try_from(tuple).unwrap();
        assert!(matches!(placeholder, super::Placeholder::Target));
        let tuple = ("target", &[""][..]);
        let result = super::Placeholder::try_from(tuple);
        assert!(result.is_err());

        let tuple = ("module", empty);
        let placeholder = super::Placeholder::try_from(tuple).unwrap();
        assert!(matches!(placeholder, super::Placeholder::Module));
        let tuple = ("module", &[""][..]);
        let result = super::Placeholder::try_from(tuple);
        assert!(result.is_err());

        let tuple = ("file", empty);
        let placeholder = super::Placeholder::try_from(tuple).unwrap();
        assert!(matches!(placeholder, super::Placeholder::File));
        let tuple = ("file", &[""][..]);
        let result = super::Placeholder::try_from(tuple);
        assert!(result.is_err());

        let tuple = ("line", empty);
        let placeholder = super::Placeholder::try_from(tuple).unwrap();
        assert!(matches!(placeholder, super::Placeholder::Line));
        let tuple = ("line", &[""][..]);
        let result = super::Placeholder::try_from(tuple);
        assert!(result.is_err());

        let tuple = ("message", empty);
        let placeholder = super::Placeholder::try_from(tuple).unwrap();
        assert!(matches!(placeholder, super::Placeholder::Message));
        let tuple = ("message", &[""][..]);
        let result = super::Placeholder::try_from(tuple);
        assert!(result.is_err());

        let tuple = ("kv", &["|", "="][..]);
        let placeholder = super::Placeholder::try_from(tuple).unwrap();
        assert!(
            matches!(placeholder, super::Placeholder::KeyValuePairs { pair_separator, kv_separator } if pair_separator == "|" && kv_separator == "=")
        );
        let tuple = ("kv", empty);
        let result = super::Placeholder::try_from(tuple);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_placeholders() {
        let pattern = "-- {datetime(%Y-%m-%d %H:%M:%S%.3f)}|{colorStart}{level}{colorEnd}|{target}|{module}|{file}:{line}|{message}{kv(|)(=)} --";
        let result = super::parse_placeholders(pattern).unwrap();
        assert!(matches!(&result[0], super::Placeholder::Literal { content } if content == "-- "));
        assert!(
            matches!(&result[1], super::Placeholder::Datetime { format } if format == "%Y-%m-%d %H:%M:%S%.3f")
        );
        assert!(matches!(&result[2], super::Placeholder::Literal { content } if content == "|"));
        assert!(matches!(&result[3], super::Placeholder::ColorStart));
        assert!(matches!(&result[4], super::Placeholder::Level));
        assert!(matches!(&result[5], super::Placeholder::ColorEnd));
        assert!(matches!(&result[6], super::Placeholder::Literal { content } if content == "|"));
        assert!(matches!(&result[7], super::Placeholder::Target));
        assert!(matches!(&result[8], super::Placeholder::Literal { content } if content == "|"));
        assert!(matches!(&result[9], super::Placeholder::Module));
        assert!(matches!(&result[10], super::Placeholder::Literal { content } if content == "|"));
        assert!(matches!(&result[11], super::Placeholder::File));
        assert!(matches!(&result[12], super::Placeholder::Literal { content } if content == ":"));
        assert!(matches!(&result[13], super::Placeholder::Line));
        assert!(matches!(&result[14], super::Placeholder::Literal { content } if content == "|"));
        assert!(matches!(&result[15], super::Placeholder::Message));
        assert!(
            matches!(&result[16], super::Placeholder::KeyValuePairs { pair_separator, kv_separator } if pair_separator == "|" && kv_separator == "=")
        );
        assert!(matches!(&result[17], super::Placeholder::Literal { content } if content == " --"));

        let pattern = "{invalid_placeholder}";
        let result = super::parse_placeholders(pattern);
        assert!(result.is_err());

        let pattern = "{datetime";
        let result = super::parse_placeholders(pattern);
        assert!(result.is_err());

        let pattern = "{datetime(";
        let result = super::parse_placeholders(pattern);
        assert!(result.is_err());

        let pattern = "{datetime(%+)(";
        let result = super::parse_placeholders(pattern);
        assert!(result.is_err());

        let pattern = "{datetime(%+)(}";
        let result = super::parse_placeholders(pattern);
        assert!(result.is_err());

        let pattern = "{datetime(%+)x";
        let result = super::parse_placeholders(pattern);
        assert!(result.is_err());

        let pattern = "{datetime(%+)x}";
        let result = super::parse_placeholders(pattern);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode() {
        let datetime = test_datetime();
        let mut builder = RecordBuilder::new();
        prepare_test_log_record(&mut builder);
        let mut kvs = Vec::new();
        prepare_test_kvs(&mut kvs);
        let encoder = super::PatternEncoder {
            placeholders: vec![
                super::Placeholder::Datetime {
                    format: "%Y-%m-%d %H:%M:%S%.3f".to_string(),
                },
                super::Placeholder::Literal {
                    content: "|".to_string(),
                },
                super::Placeholder::ColorStart,
                super::Placeholder::Level,
                super::Placeholder::ColorEnd,
                super::Placeholder::Literal {
                    content: "|".to_string(),
                },
                super::Placeholder::Target,
                super::Placeholder::Literal {
                    content: "|".to_string(),
                },
                super::Placeholder::Module,
                super::Placeholder::Literal {
                    content: "|".to_string(),
                },
                super::Placeholder::File,
                super::Placeholder::Literal {
                    content: ":".to_string(),
                },
                super::Placeholder::Line,
                super::Placeholder::Literal {
                    content: "|".to_string(),
                },
                super::Placeholder::Message,
                super::Placeholder::KeyValuePairs {
                    pair_separator: "|".to_string(),
                    kv_separator: "=".to_string(),
                },
            ],
        };
        let result = encoder.encode(
            &datetime,
            &builder
                .args(format_args!("{}", TEST_MESSAGE))
                .key_values(&kvs)
                .build(),
        );

        assert_eq!(
            result,
            format!(
                "{}|{}{}{}|{}|{}|{}:{}|{}|{}={}|{}={}|{}={}|{}={}",
                datetime.format("%Y-%m-%d %H:%M:%S%.3f"),
                super::level2color(TEST_LEVEL),
                TEST_LEVEL,
                super::ANSI_COLOR_RESET,
                TEST_TARGET,
                TEST_MODULE,
                TEST_FILE,
                TEST_LINE,
                TEST_MESSAGE,
                TEST_KV0.0,
                serde_json::to_string(&TEST_KV0.1).unwrap(),
                TEST_KV1.0,
                serde_json::to_string(&TEST_KV1.1).unwrap(),
                TEST_KV2.0,
                serde_json::to_string(&TEST_KV2.1).unwrap(),
                TEST_KV3.0,
                serde_json::to_string(&TEST_KV3.1).unwrap(),
            )
        );
    }
}
