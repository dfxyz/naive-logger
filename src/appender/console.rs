use std::io::{stderr, Stderr, stdout, Stdout, Write};

use log::{LevelFilter, Record};

use crate::{Datetime, Error};
use crate::appender::Appender;
use crate::config::ConsoleAppenderConfig;
use crate::encoder::{self, Encoder};

pub struct ConsoleAppender {
    encoder: Box<dyn Encoder + Send>,
    stdout: Stdout,
    stderr: Stderr,
    stderr_level: LevelFilter,
}

impl TryFrom<&ConsoleAppenderConfig> for ConsoleAppender {
    type Error = Error;

    fn try_from(config: &ConsoleAppenderConfig) -> Result<Self, Self::Error> {
        let encoder = encoder::from_config(&config.common.encoder)
            .map_err(|e| e.concat("failed to create encoder"))?;
        Ok(Self {
            encoder,
            stdout: stdout(),
            stderr: stderr(),
            stderr_level: config.stderr_level,
        })
    }
}

impl Appender for ConsoleAppender {
    fn append(&mut self, datetime: &Datetime, record: &Record) {
        let s = self.encoder.encode(datetime, record);
        let destination: &mut dyn Write = if record.level() <= self.stderr_level {
            &mut self.stderr
        } else {
            &mut self.stdout
        };
        writeln!(destination, "{}", s).unwrap();
    }

    fn flush(&mut self) {
        self.stdout.flush().unwrap();
        if self.stderr_level > LevelFilter::Off {
            self.stderr.flush().unwrap();
        }
    }
}
