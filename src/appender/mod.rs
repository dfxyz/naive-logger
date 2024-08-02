use std::sync::{Arc, Mutex};

use log::Record;

use crate::{Datetime, Error};
use crate::appender::console::ConsoleAppender;
use crate::config::AppenderConfig;

mod console;
mod file;

pub trait Appender {
    fn append(&mut self, datetime: &Datetime, record: &Record);
    fn flush(&mut self);
}

pub fn from_config(config: &AppenderConfig) -> Result<Arc<Mutex<dyn Appender + Send>>, Error> {
    match config {
        AppenderConfig::Console(config) => {
            let appender = ConsoleAppender::try_from(config)?;
            Ok(Arc::new(Mutex::new(appender)))
        }
        AppenderConfig::File(config) => {
            let appender = file::FileAppender::try_from(config)?;
            Ok(Arc::new(Mutex::new(appender)))
        }
    }
}
