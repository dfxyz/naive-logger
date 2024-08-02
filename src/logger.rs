use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use log::{LevelFilter, Record};

use crate::{Datetime, Error};
use crate::appender::Appender;
use crate::config::{LoggerConfig, LoggerTargetMatcher};

pub struct Logger {
    target: String,
    target_matcher: LoggerTargetMatcher,
    level: LevelFilter,
    appenders: Vec<Arc<Mutex<dyn Appender + Send>>>,
}

impl Logger {
    pub fn new(
        config: &LoggerConfig,
        appenders: &HashMap<String, Arc<Mutex<dyn Appender + Send>>>,
        root_logger: Option<&Logger>,
    ) -> Result<Self, Error> {
        let mut logger = Self {
            target: config.target.clone(),
            target_matcher: config.target_matcher,
            level: config.level,
            appenders: vec![],
        };
        if config.appenders.is_empty() {
            let root_logger = root_logger.ok_or_else(|| {
                Error::from("root logger must have at least one appender")
            })?;
            logger.appenders = root_logger.appenders.clone();
        } else {
            for name in &config.appenders {
                let appender = appenders.get(name).ok_or_else(|| {
                    Error::from(format!("no appender '{}'", name))
                })?;
                logger.appenders.push(appender.clone());
            }
        }
        Ok(logger)
    }

    pub fn handle(&self, datetime: &Datetime, record: &Record) -> bool {
        if record.level() > self.level {
            return false;
        }

        match self.target_matcher {
            LoggerTargetMatcher::Prefix => {
                if !record.target().starts_with(&self.target) {
                    return false;
                }
            }
            LoggerTargetMatcher::PrefixInverse => {
                if record.target().starts_with(&self.target) {
                    return false;
                }
            }
            LoggerTargetMatcher::Exact => {
                if record.target() != self.target {
                    return false;
                }
            }
        }

        for appender in &self.appenders {
            let mut guard = appender.lock().unwrap();
            guard.append(datetime, record);
        }
        true
    }
}
