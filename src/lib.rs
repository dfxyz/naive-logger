//! 一个年轻、简单、有时候还有点幼稚的异步Logger的实现

use std::convert::TryFrom;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::num::{NonZeroU32, NonZeroU64};
use std::ops::Deref;
use std::panic::PanicInfo;
use std::str::FromStr;
use std::sync::Arc;
use std::{env, fs, panic, thread};

use backtrace::Backtrace;
use crossbeam_channel::{Receiver, Sender};
use log::{Level, LevelFilter, Log, Metadata, ParseLevelError, Record};
use serde::Deserialize;

const DATETIME_FORMAT: &str = "%FT%T%.6f%:z";

const THREAD_NAME_STDIO: &str = "naive-logger-stdio";
const THREAD_NAME_FILE: &str = "naive-logger-file";

static mut INTERFACE: Option<Interface> = None;
static mut STDIO_IMPL: Option<thread::JoinHandle<()>> = None;
static mut FILE_IMPL: Option<thread::JoinHandle<()>> = None;

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// 日志级别
    pub level: LogLevel,
    /// 是否输出到标准I/O
    pub use_stdio: bool,
    /// 日志文件路径; 如果为空, 则日志不会输出到文件
    pub file_name: String,
    /// 日志文件的最大体积; 超出该限制时会轮换文件内容
    pub max_file_len: NonZeroU64,
    /// 保存轮换内容的日志文件数量
    pub max_rotate_file_num: NonZeroU32,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            level: Default::default(),
            use_stdio: true,
            file_name: "".to_string(),
            max_file_len: unsafe { NonZeroU64::new_unchecked(128 * 1024 * 1024) },
            max_rotate_file_num: unsafe { NonZeroU32::new_unchecked(3) },
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(try_from = "&str")]
pub struct LogLevel(LevelFilter);
impl LogLevel {
    #[inline]
    pub fn set(&mut self, level_filter: LevelFilter) {
        self.0 = level_filter;
    }
}
impl Default for LogLevel {
    #[inline]
    fn default() -> Self {
        Self(LevelFilter::Info)
    }
}
impl Deref for LogLevel {
    type Target = LevelFilter;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl TryFrom<&str> for LogLevel {
    type Error = ParseLevelError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self(LevelFilter::from_str(value)?))
    }
}

pub fn init(config: &Config) {
    let level = *config.level;
    if level == LevelFilter::Off {
        return;
    }
    let file_name = config.file_name.trim();
    if !config.use_stdio && file_name.is_empty() {
        return;
    }

    let mut tx_for_stdio = None;
    let mut stdio_impl = None;
    if config.use_stdio {
        let (tx, rx) = crossbeam_channel::unbounded();
        tx_for_stdio = Some(tx);
        stdio_impl = Some(StdioImpl { rx });
    }

    let mut tx_for_file = None;
    let mut file_impl = None;
    if !file_name.is_empty() {
        let working_directory = env::current_dir().unwrap();
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(working_directory.join(file_name))
            .expect("无法打开日志文件");
        file.seek(SeekFrom::End(0)).expect("无法打开日志文件");
        let metadata = file.metadata().expect("无法打开日志文件");
        let file_len = metadata.len();

        let (tx, rx) = crossbeam_channel::unbounded();
        tx_for_file = Some(tx);
        file_impl = Some(FileImpl {
            rx,
            file,
            file_len,
            file_name: file_name.to_string(),
            max_file_len: config.max_file_len,
            max_rotate_file_num: config.max_rotate_file_num,
        });
    };

    let interface = Interface {
        tx_for_stdio,
        tx_for_file,
        level: level.to_level().unwrap(),
    };
    let interface = unsafe {
        INTERFACE.replace(interface);
        INTERFACE.as_ref().unwrap()
    };

    if let Some(stdio_impl) = stdio_impl {
        let handle = thread::Builder::new()
            .name(THREAD_NAME_STDIO.to_string())
            .spawn(move || stdio_impl.run())
            .expect("创建日志线程 (标准I/O) 失败");
        unsafe { STDIO_IMPL.replace(handle) };
    }

    if let Some(file_impl) = file_impl {
        let handle = thread::Builder::new()
            .name(THREAD_NAME_FILE.to_string())
            .spawn(move || file_impl.run())
            .expect("创建日志线程 (文件) 失败");
        unsafe { FILE_IMPL.replace(handle) };
    }

    log::set_max_level(level);
    log::set_logger(interface).unwrap();

    panic::set_hook(Box::new(panic_handler));
}

fn panic_handler(info: &PanicInfo) {
    let thread = thread::current();
    let thread_name = thread.name().unwrap_or("<unnamed>");
    if thread_name == THREAD_NAME_FILE || thread_name == THREAD_NAME_STDIO {
        return;
    }
    let datetime = chrono::Local::now().format(DATETIME_FORMAT);
    let msg = match info.payload().downcast_ref::<&'static str>() {
        Some(s) => *s,
        None => match info.payload().downcast_ref::<String>() {
            Some(s) => s.as_str(),
            None => "Box<Any>",
        },
    };
    let location = info.location().unwrap();
    let backtrace = Backtrace::new();

    let mut buf = String::new();
    #[allow(unused)]
    {
        writeln!(
            &mut buf,
            "[{}][PANIC] 线程 '{}' 发生异常: '{}', {}",
            datetime, thread_name, msg, location
        );
        writeln!(&mut buf, "{:?}", backtrace);
    }
    let msg = Arc::new(buf);

    if let Some(interface) = unsafe { INTERFACE.as_ref() } {
        if let Some(tx) = &interface.tx_for_stdio {
            let _ = tx.send(Message::Log(msg.clone()));
        }
        if let Some(tx) = &interface.tx_for_file {
            let _ = tx.send(Message::Log(msg));
        }
    }
}

pub fn shutdown() {
    unsafe {
        if let Some(interface) = INTERFACE.as_mut() {
            interface.tx_for_stdio.take();
            interface.tx_for_file.take();

            if let Some(handle) = STDIO_IMPL.take() {
                let _ = handle.join();
            }
            if let Some(handle) = FILE_IMPL.take() {
                let _ = handle.join();
            }
        }
    }
}

enum Message {
    Log(Arc<String>),
    Flush,
}

struct Interface {
    tx_for_stdio: Option<Sender<Message>>,
    tx_for_file: Option<Sender<Message>>,
    level: Level,
}
impl Log for Interface {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let datetime = chrono::Local::now().format(DATETIME_FORMAT);
        let s = format!(
            "[{}][{}][{}({})] {}\n",
            datetime,
            record.level(),
            record.file().unwrap_or("null"),
            record.line().unwrap_or(0),
            record.args()
        );
        let s = Arc::new(s);
        if let Some(tx) = &self.tx_for_stdio {
            let _ = tx.send(Message::Log(s.clone()));
        }
        if let Some(tx) = &self.tx_for_file {
            let _ = tx.send(Message::Log(s));
        }
    }

    fn flush(&self) {
        if let Some(tx) = &self.tx_for_file {
            let _ = tx.send(Message::Flush);
        }
    }
}

struct StdioImpl {
    rx: Receiver<Message>,
}
impl StdioImpl {
    fn run(self) {
        while let Ok(msg) = self.rx.recv() {
            match msg {
                Message::Log(s) => print!("{}", s),
                Message::Flush => {}
            }
        }
    }
}

struct FileImpl {
    rx: Receiver<Message>,
    file: File,
    file_len: u64,
    file_name: String,
    max_file_len: NonZeroU64,
    max_rotate_file_num: NonZeroU32,
}
impl FileImpl {
    fn run(mut self) {
        while let Ok(msg) = self.rx.recv() {
            match msg {
                Message::Log(s) => self.log(s.as_str()),
                Message::Flush => self.flush(),
            }
        }
        self.flush();
    }

    fn log(&mut self, s: &str) {
        let bytes = s.as_bytes();
        if self.file_len + bytes.len() as u64 >= self.max_file_len.get() {
            self.rotate_file();
        }
        self.file.write_all(bytes).unwrap();
        self.file_len += bytes.len() as u64;
    }

    fn rotate_file(&mut self) {
        let working_directory = env::current_dir().unwrap();
        for i in (0..self.max_rotate_file_num.get()).rev() {
            let source = working_directory.join(self.filename(i));
            if source.exists() {
                let target_path = working_directory.join(self.filename(i + 1));
                let _ = fs::copy(source, target_path);
            }
        }
        self.file.seek(SeekFrom::Start(0)).unwrap();
        self.file.set_len(0).unwrap();
        self.file_len = 0;
    }

    fn flush(&mut self) {
        self.file.flush().unwrap();
    }

    #[inline]
    fn filename(&self, index: u32) -> String {
        if index == 0 {
            self.file_name.clone()
        } else {
            format!("{}.{}", self.file_name, index)
        }
    }
}
