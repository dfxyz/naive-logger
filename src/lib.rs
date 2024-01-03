use std::io::{Seek, Write};

pub use config::Config;
pub use config::FileConfig;
pub use config::StdoutConfig;

mod config;

const CHANNEL_CAPACITY: usize = 1024;
const DATETIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S.%6f%z";

/// Initialize the logger with the given config.
/// On success, a logging thread will be spawned and a drop guard will be returned.
pub fn init(conf: &Config) -> Result<DropGuard, String> {
    check_config(conf)?;

    if !conf.stdout.enable && !conf.file.enable {
        return Ok(DropGuard { inner: None });
    }

    let (tx, rx) = crossbeam_channel::bounded::<Message>(CHANNEL_CAPACITY);
    let consumer = {
        let stdout = if conf.stdout.enable {
            Some(StandardOutput {
                inner: std::io::stdout(),
                use_color: conf.stdout.use_color,
            })
        } else {
            None
        };
        let file = if conf.file.enable {
            let f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&conf.file.filename)
                .map_err(|e| format!("failed to open the log file: {}", e))?;
            Some(FileOutput {
                inner: Some(f),
                filename: conf.file.filename.clone(),
                rotate_size: conf.file.rotate_size,
                max_rotated_num: conf.file.max_rotated_num,
            })
        } else {
            None
        };
        Consumer { rx, stdout, file }
    };
    let jh = std::thread::spawn(move || {
        consumer.run();
    });

    let producer = Producer {
        tx: tx.clone(),
        level: conf.level,
    };
    log::set_max_level(conf.level);
    log::set_logger(Box::leak(Box::new(producer))).unwrap();

    Ok(DropGuard {
        inner: Some(DropGuardInner { tx, jh }),
    })
}

fn check_config(conf: &Config) -> Result<(), String> {
    if conf.file.enable {
        if conf.file.filename.trim().is_empty() {
            return Err("file output is enabled but 'file.filename' is empty".to_string());
        }
        if conf.file.rotate_size == 0 {
            return Err("file output is enabled but 'file.rotate_size' is 0".to_string());
        }
        if conf.file.max_rotated_num == 0 {
            return Err("file output is enabled but 'file.max_rotated_num' is 0".to_string());
        }
    }
    Ok(())
}

/// Carries a request from others to the logging thread.
enum Message {
    Payload {
        datetime: chrono::DateTime<chrono::Local>,
        level: log::Level,
        file: String,
        line: u32,
        desc: String,
    },
    Flush,
    Close,
}

/// If dropped, send a close message to the logging thread and join it.
pub struct DropGuard {
    inner: Option<DropGuardInner>,
}
struct DropGuardInner {
    tx: crossbeam_channel::Sender<Message>,
    jh: std::thread::JoinHandle<()>,
}
impl Drop for DropGuard {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            inner
                .tx
                .send(Message::Close)
                .expect("channel closed unexpectedly");
            inner.jh.join().expect("logging thread panicked");
        }
    }
}

/// Collect the logging request from application codes and send them to the logging thread with a [`Message`] sender.
struct Producer {
    tx: crossbeam_channel::Sender<Message>,
    level: log::LevelFilter,
}
impl log::Log for Producer {
    #[inline]
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.level >= metadata.level()
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let datetime = chrono::Local::now();
            let level = record.level();
            let file = record.file().unwrap_or("<unknown>").to_string();
            let line = record.line().unwrap_or(0);
            let desc = record.args().to_string();
            self.tx
                .send(Message::Payload {
                    datetime,
                    level,
                    file,
                    line,
                    desc,
                })
                .expect("channel closed unexpectedly");
        }
    }

    fn flush(&self) {
        self.tx
            .send(Message::Flush)
            .expect("channel closed unexpectedly");
    }
}

/// Runs in the logging thread, consumes the messages sent by [`Producer`].
struct Consumer {
    rx: crossbeam_channel::Receiver<Message>,
    stdout: Option<StandardOutput>,
    file: Option<FileOutput>,
}
impl Consumer {
    fn run(mut self) {
        while let Ok(msg) = self.rx.recv() {
            match msg {
                Message::Payload {
                    datetime,
                    level,
                    file,
                    line,
                    desc,
                } => {
                    let datetime = datetime.format(DATETIME_FORMAT);
                    let s = format!("{datetime}|{level}|{file}:{line}|{desc}");
                    if let Some(stdout) = self.stdout.as_mut() {
                        stdout.log(level, s.as_str()).unwrap();
                    }
                    if let Some(file) = self.file.as_mut() {
                        file.log(s.as_str()).unwrap();
                    }
                }
                Message::Flush => {
                    if let Some(stdout) = self.stdout.as_mut() {
                        stdout.flush().unwrap();
                    }
                    if let Some(file) = self.file.as_mut() {
                        file.flush().unwrap();
                    }
                }
                Message::Close => {
                    return;
                }
            }
        }
    }
}
struct StandardOutput {
    inner: std::io::Stdout,
    use_color: bool,
}
impl StandardOutput {
    fn log(&mut self, level: log::Level, s: &str) -> std::io::Result<()> {
        if self.use_color {
            let prefix = match level {
                log::Level::Error => "\x1b[31m", // red
                log::Level::Warn => "\x1b[33m",  // yellow
                log::Level::Info => "\x1b[32m",  // green
                log::Level::Debug => "\x1b[36m", // cyan
                log::Level::Trace => "\x1b[34m", // blue
            };
            writeln!(&mut self.inner, "{prefix}{s}\x1b[0m")
        } else {
            writeln!(&mut self.inner, "{}", s)
        }
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
struct FileOutput {
    inner: Option<std::fs::File>,
    filename: String,
    rotate_size: u64,
    max_rotated_num: u32,
}
impl FileOutput {
    #[inline]
    fn inner(&mut self) -> std::io::Result<&mut std::fs::File> {
        self.inner.as_mut().ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            "log file closed unexpectedly",
        ))
    }

    fn log(&mut self, s: &str) -> std::io::Result<()> {
        let current_size = self.inner()?.seek(std::io::SeekFrom::End(0))?;
        if current_size + s.len() as u64 + 1 > self.rotate_size {
            self.rotate()?;
        }
        writeln!(self.inner()?, "{s}")
    }

    fn rotate(&mut self) -> std::io::Result<()> {
        self.inner.take();

        let filename = self.filename.as_str();
        for i in (1..=self.max_rotated_num).rev() {
            let target = format!("{filename}.{i}");
            let rename_result = if i == 1 {
                std::fs::rename(filename, target)
            } else {
                let j = i - 1;
                let source = format!("{filename}.{j}");
                std::fs::rename(source, target)
            };
            match rename_result {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
                _ => {}
            }
        }

        self.inner.replace(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(filename)
                .map_err(|e| {
                    std::io::Error::new(e.kind(), format!("failed to reopen the log file: {}", e))
                })?,
        );
        Ok(())
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner()?.flush()
    }
}
