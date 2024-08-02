use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::sync::{Arc, Mutex};

use log::{LevelFilter, Log, Metadata, Record};

use crate::appender::Appender;
use crate::config::{AppenderConfig, Config, LoggerConfig};
use crate::logger::Logger;

mod appender;
mod config;
mod encoder;
mod logger;

type Datetime = chrono::DateTime<chrono::Local>;

#[derive(Debug)]
pub struct Error {
    desc: String,
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.desc)
    }
}
impl std::error::Error for Error {}
impl From<String> for Error {
    fn from(value: String) -> Self {
        Self { desc: value }
    }
}
impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Self {
            desc: value.to_string(),
        }
    }
}
impl Error {
    pub fn concat<X: Display>(self, preceding_msg: X) -> Self {
        Self {
            desc: format!("{}: {}", preceding_msg, self.desc),
        }
    }
}

pub fn init<P: AsRef<Path>>(config_file: P) -> Result<(), Error> {
    let path = config_file.as_ref();
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::from(format!("failed to read config file: {}", e)))?;
    match path.extension() {
        None => Err(Error::from(
            "config file has no extension, cannot determine the deserializer",
        )),
        Some(s) => {
            let ext = s
                .to_str()
                .ok_or_else(|| Error::from("config filename contains invalid UTF-8"))?;
            match ext {
                x if x == "json" => init_from_json(content),
                x if x == "toml" => init_from_toml(content),
                x if x == "yaml" || x == "yml" => init_from_yaml(content),
                _ => Err(Error::from(format!(
                    "unsupported config file extension '{}'",
                    ext
                ))),
            }
        }
    }
}

pub fn init_from_json<S: AsRef<str>>(s: S) -> Result<(), Error> {
    let config = serde_json::from_str(s.as_ref())
        .map_err(|e| Error::from(format!("failed to deserialize config: {}", e)))?;
    init_from_config(config)
}

pub fn init_from_toml<S: AsRef<str>>(s: S) -> Result<(), Error> {
    let config = toml::from_str(s.as_ref())
        .map_err(|e| Error::from(format!("failed to deserialize config: {}", e)))?;
    init_from_config(config)
}

pub fn init_from_yaml<S: AsRef<str>>(s: S) -> Result<(), Error> {
    let config = serde_yaml::from_str(s.as_ref())
        .map_err(|e| Error::from(format!("failed to deserialize config: {}", e)))?;
    init_from_config(config)
}

fn init_from_config(config: Config) -> Result<(), Error> {
    let appenders = construct_appenders(config.appenders)?;
    let root_logger = Logger::new(&config.root, &appenders, None)
        .map_err(|e| e.concat("failed to create root logger"))?;
    let mut loggers = vec![];
    for (i, config) in config.loggers.iter().enumerate() {
        let logger = Logger::new(config, &appenders, Some(&root_logger))
            .map_err(|e| e.concat(format!("failed to create logger #{}'", i)))?;
        loggers.push(logger);
    }
    loggers.push(root_logger);
    let global_level = get_global_level(std::iter::once(&config.root).chain(&config.loggers));

    let log_impl = LogImplementation {
        global_level,
        loggers,
        appenders: appenders.values().cloned().collect(),
    };
    let log_impl = Box::leak(Box::new(log_impl));

    log::set_max_level(global_level);
    log::set_logger(log_impl).map_err(|e| Error::from(format!("failed to set logger: {}", e)))
}

fn construct_appenders(
    config_map: HashMap<String, AppenderConfig>,
) -> Result<HashMap<String, Arc<Mutex<dyn Appender + Send>>>, Error> {
    let mut result = HashMap::new();
    let mut path_set = HashSet::new();
    for (name, config) in config_map {
        if let AppenderConfig::File(config) = &config {
            let path = config.path.to_str().ok_or_else(|| {
                Error::from(format!("appender '{}': path contains invalid UTF-8", name))
            })?;
            if !path_set.insert(path.to_string()) {
                return Err(Error::from(format!(
                    "appenders: path '{}' is used by multiple appenders",
                    path
                )));
            }
        }
        let appender = appender::from_config(&config)
            .map_err(|e| e.concat(format!("failed to create appender '{}'", name)))?;
        result.insert(name, appender);
    }
    Ok(result)
}

fn get_global_level<'a, I: Iterator<Item = &'a LoggerConfig>>(it: I) -> LevelFilter {
    it.map(|config| config.level)
        .max()
        .unwrap_or(LevelFilter::Info)
}

struct LogImplementation {
    global_level: LevelFilter,
    loggers: Vec<Logger>,
    appenders: Vec<Arc<Mutex<dyn Appender + Send>>>,
}

impl Log for LogImplementation {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.global_level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let now = chrono::Local::now();
        for logger in &self.loggers {
            if logger.handle(&now, record) {
                return;
            }
        }
    }

    fn flush(&self) {
        for appender in &self.appenders {
            let mut guard = appender.lock().unwrap();
            guard.flush();
        }
    }
}
