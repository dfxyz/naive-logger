use std::fs::File;
use std::io::{Seek, Write};
use std::path::PathBuf;

use log::Record;

use crate::{Datetime, encoder, Error};
use crate::appender::Appender;
use crate::config::FileAppenderConfig;
use crate::encoder::Encoder;

pub struct FileAppender {
    encoder: Box<dyn Encoder + Send>,
    path: PathBuf,
    filename: String,
    file: File,
    file_len: u64,
    max_file_size: u64,
    max_backup_index: usize,
}

impl TryFrom<&FileAppenderConfig> for FileAppender {
    type Error = Error;

    fn try_from(config: &FileAppenderConfig) -> Result<Self, Self::Error> {
        let encoder = encoder::from_config(&config.common.encoder)
            .map_err(|e| e.concat("failed to create encoder"))?;

        match config.path.parent() {
            None => {}
            Some(dir) => {
                std::fs::create_dir_all(dir)
                    .map_err(|e| Error::from(format!("failed to prepare log directory: {}", e)))?;
            }
        }
        let filename = config
            .path
            .file_name()
            .ok_or_else(|| Error::from("failed to get file name from log path"))?
            .to_str()
            .ok_or_else(|| Error::from("filename contains invalid UTF-8"))?
            .to_string();

        let mut file = File::options()
            .create(true)
            .write(true)
            .open(&config.path)
            .map_err(|e| Error::from(format!("failed to open log file: {}", e)))?;
        let file_len = file
            .seek(std::io::SeekFrom::End(0))
            .map_err(|e| Error::from(format!("failed to seek to the end of log file: {}", e)))?;

        Ok(Self {
            encoder,
            path: config.path.clone(),
            filename,
            file,
            file_len,
            max_file_size: config.max_file_size,
            max_backup_index: config.max_backup_index,
        })
    }
}

impl Appender for FileAppender {
    fn append(&mut self, datetime: &Datetime, record: &Record) {
        let content = self.encoder.encode(datetime, record);
        self.rotate_if_needed(content.len() + 1);
        writeln!(self.file, "{}", content).unwrap();
        self.file_len += content.len() as u64 + 1;
    }

    fn flush(&mut self) {
        self.file.flush().unwrap();
    }
}

impl FileAppender {
    fn backup_file_path(&self, index: usize) -> PathBuf {
        self.path
            .with_file_name(format!("{}.{}", self.filename, index))
    }
    fn rotate_if_needed(&mut self, reserve_len: usize) {
        if self.max_file_size == 0 || self.file_len + reserve_len as u64 <= self.max_file_size {
            return;
        }

        let last_backup_file_path = self.backup_file_path(self.max_backup_index);
        if last_backup_file_path.exists() {
            std::fs::remove_file(&last_backup_file_path).unwrap();
        }

        for i in (0..self.max_backup_index).rev() {
            let src = self.backup_file_path(i);
            let dst = self.backup_file_path(i + 1);
            if src.exists() {
                std::fs::rename(src, dst).unwrap();
            }
        }

        let dst = self.backup_file_path(0);
        std::fs::rename(&self.path, dst).unwrap();

        self.file = File::options()
            .create_new(true)
            .write(true)
            .open(&self.path)
            .unwrap();
        self.file_len = 0;
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{Read, Write};

    use crate::config::{EncoderConfig, JsonEncoderConfig};

    #[test]
    fn test_rotate() {
        {
            for i in 0..=3 {
                let mut f = File::options()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(format!("__test.log.{}", i))
                    .unwrap();
                writeln!(f, "original file index: {}", i).unwrap();
            }
            let mut file = File::options()
                .create(true)
                .write(true)
                .truncate(true)
                .open("__test.log")
                .unwrap();
            writeln!(file, "file be rotated").unwrap();

            let mut appender = super::FileAppender {
                encoder: super::encoder::from_config(&EncoderConfig::Json(JsonEncoderConfig))
                    .unwrap(),
                path: "__test.log".into(),
                filename: "__test.log".to_string(),
                file,
                file_len: 1024,
                max_file_size: 1024,
                max_backup_index: 3,
            };
            appender.rotate_if_needed(1);
        }

        let mut content = String::new();
        File::open("__test.log")
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "");
        content.clear();
        File::open("__test.log.0")
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "file be rotated\n");
        for i in 1..=3 {
            content.clear();
            File::open(format!("__test.log.{}", i))
                .unwrap()
                .read_to_string(&mut content)
                .unwrap();
            assert_eq!(content, format!("original file index: {}\n", i - 1));
        }

        std::fs::remove_file("__test.log").unwrap();
        for i in 0..=3 {
            std::fs::remove_file(format!("__test.log.{}", i)).unwrap();
        }
    }
}
