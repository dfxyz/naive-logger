use std::{
    fs::File,
    io::{Seek, SeekFrom, Write},
    path::PathBuf,
    str::FromStr,
    time::Duration,
};

use chrono::Local as LocalTimezone;
type LocalDateTime = chrono::DateTime<LocalTimezone>;

use log::{Level, LevelFilter, Log};
pub use log::{debug, error, info, log_enabled, trace, warn};
use regex::Regex;

const CHANNEL_BUFFER_SIZE: usize = 128;
const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";
const BACKUP_FILENAME_DATETIME_FORMAT: &str = ".%Y%m%d-%H%M%S-%3f.bak";
const BACKUP_FILENAME_PATTERN: &str = r"\.(\d{4}\d{2}\d{2})-(\d{2}\d{2}\d{2})-(\d{3})\.bak";
const REOPEN_FILE_ON_ERROR_DELAY: Duration = Duration::from_secs(10);

macro_rules! define_env {
    ($name:ident) => {
        const $name: &str = stringify!($name);
    };
}
define_env!(NAIVE_LOG_LEVEL);
define_env!(NAIVE_LOG_CONSOLE);
define_env!(NAIVE_LOG_FILE);
define_env!(NAIVE_LOG_FILE_ROTATE_SIZE);
define_env!(NAIVE_LOG_BACKUP_FILE_NUM);

/// 使用环境变量中的配置初始化「Naive Logger」，并返回一个[`NaiveLoggerDropGuard`]用于控制日志线程的关闭
///
/// 如果初始化失败，将会触发Panic
pub fn init() -> NaiveLoggerDropGuard {
    init_with(Default::default())
}

/// 使用[`NaiveLoggerConfig`]中的配置初始化「Naive Logger」，并返回一个[`NaiveLoggerDropGuard`]用于控制日志线程的关闭
///
/// 如果初始化失败，将会触发Panic
pub fn init_with(mut config: NaiveLoggerConfig) -> NaiveLoggerDropGuard {
    if config.allow_env_vars {
        config.apply_env_vars();
    }

    let (tx, rx) = crossbeam_channel::bounded(CHANNEL_BUFFER_SIZE);

    let frontend = NaiveLoggerFrontend {
        tx: tx.clone(),
        level: config.level,
        target_levels: config.target_levels,
    };
    let backend = NaiveLoggerBackend {
        rx,
        console: config.use_console,
        file: if let Some(path) = config.file_path {
            Some(NaiveLoggerBackendFile::new(
                path,
                config.file_rotate_size,
                config.backup_file_num,
            ))
        } else {
            None
        },
    };
    let drop_guard = NaiveLoggerDropGuard {
        tx,
        handle: Some(std::thread::spawn(move || backend.run())),
    };

    log::set_max_level(frontend.max_level());
    log::set_logger(Box::leak(Box::new(frontend))).expect("failed to set logger");

    drop_guard
}

/// 「Naive Logger」的配置结构体
pub struct NaiveLoggerConfig {
    /// 默认日志等级
    pub level: LevelFilter,

    /// 按目标单独配置的日志等级
    pub target_levels: Vec<(String, LevelFilter)>,

    /// 是否使用控制台输出日志
    pub use_console: bool,

    /// 日志文件的路径
    pub file_path: Option<PathBuf>,

    /// 日志文件的滚动大小（单位：MiB）
    pub file_rotate_size: u64,

    /// 备份文件的最大保留数量
    pub backup_file_num: usize,

    /// 是否允许环境变量覆盖编码设置的配置
    pub allow_env_vars: bool,
}

impl Default for NaiveLoggerConfig {
    fn default() -> Self {
        Self {
            #[cfg(debug_assertions)]
            level: LevelFilter::Debug,
            #[cfg(not(debug_assertions))]
            level: LevelFilter::Info,
            target_levels: Default::default(),
            use_console: true,
            file_path: None,
            file_rotate_size: 0,
            backup_file_num: 0,
            allow_env_vars: true,
        }
    }
}

impl NaiveLoggerConfig {
    fn apply_env_vars(&mut self) {
        self.apply_env_var_level();
        if let Some(var) = parse_env_var_as_bool(NAIVE_LOG_CONSOLE) {
            self.use_console = var;
        }
        if let Some(var) = parse_env_var_as_str(NAIVE_LOG_FILE) {
            self.file_path = Some(var.into());
        }
        if let Some(var) = parse_env_var_as_num(NAIVE_LOG_FILE_ROTATE_SIZE) {
            self.file_rotate_size = var;
        }
        if let Some(var) = parse_env_var_as_num(NAIVE_LOG_BACKUP_FILE_NUM) {
            self.backup_file_num = var as _;
        }
    }

    fn apply_env_var_level(&mut self) {
        if let Ok(var) = std::env::var(NAIVE_LOG_LEVEL) {
            let mut target_levels = Vec::new();
            let parts = var.split(",");
            for part in parts {
                match part.split_once("=") {
                    Some((target, level)) => match LevelFilter::from_str(level) {
                        Ok(level) => target_levels.push((target.to_string(), level)),
                        Err(_) => eprintln!(
                            "[Naive Logger] '{part}' in ${NAIVE_LOG_LEVEL} is invalid and ignored!"
                        ),
                    },
                    None => match LevelFilter::from_str(part) {
                        Ok(level) => self.level = level,
                        Err(_) => eprintln!(
                            "[Naive Logger] '{part}' in ${NAIVE_LOG_LEVEL} is invalid and ignored!"
                        ),
                    },
                }
            }
            if !target_levels.is_empty() {
                self.target_levels = target_levels;
            }
        }
    }
}

fn parse_env_var_as_str(var_name: &str) -> Option<String> {
    std::env::var(var_name).ok()
}

fn parse_env_var_as_num(var_name: &str) -> Option<u64> {
    std::env::var(var_name).ok().and_then(|s| match s.parse() {
        Ok(num) => Some(num),
        Err(_) => {
            eprintln!("[Naive Logger] ${var_name} is invalid and ignored!");
            None
        }
    })
}

fn parse_env_var_as_bool(var_name: &str) -> Option<bool> {
    std::env::var(var_name)
        .ok()
        .and_then(|s| match s.to_lowercase().as_str() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => {
                eprintln!("[Naive Logger] ${var_name} is invalid and ignored!");
                None
            }
        })
}

enum Message {
    Log(Level, String),
    Flush,
    Shutdown,
}
type MessageSender = crossbeam_channel::Sender<Message>;
type MessageReceiver = crossbeam_channel::Receiver<Message>;

pub struct NaiveLoggerDropGuard {
    tx: MessageSender,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Drop for NaiveLoggerDropGuard {
    fn drop(&mut self) {
        let _ = self.tx.send(Message::Shutdown);
        if let Err(e) = self
            .handle
            .take()
            .expect("handle should only be taken in 'Drop'")
            .join()
        {
            eprintln!("[Naive Logger] Logger thread panicked: {e:?}");
        }
    }
}

struct NaiveLoggerFrontend {
    level: LevelFilter,
    target_levels: Vec<(String, LevelFilter)>,
    tx: MessageSender,
}

struct NaiveLoggerBackend {
    rx: MessageReceiver,
    console: bool,
    file: Option<NaiveLoggerBackendFile>,
}

struct NaiveLoggerBackendFile {
    path: PathBuf,
    parent: PathBuf,
    rotate_size: u64,
    backup_file_num: usize,
    backup_filename_pattern: Regex,

    file: Option<File>,
    current_size: u64,
    reopen_file_on_error_after: Option<LocalDateTime>,
}

impl NaiveLoggerFrontend {
    fn max_level(&self) -> LevelFilter {
        let mut max_level = self.level;
        for (_, level) in &self.target_levels {
            if *level > max_level {
                max_level = *level;
            }
        }
        max_level
    }
}

impl Log for NaiveLoggerFrontend {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let target = metadata.target();
        let level = metadata.level();
        for (target_prefix, target_level) in &self.target_levels {
            if target.starts_with(target_prefix.as_str()) {
                return level <= *target_level;
            }
        }
        level <= self.level
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let now = LocalTimezone::now();
        let s = format!(
            "{}|{:>5}|{}|{}",
            now.format(DATETIME_FORMAT),
            record.level(),
            record.target(),
            record.args()
        );
        if self.tx.send(Message::Log(record.level(), s)).is_err() {
            eprintln!("[Naive Logger] Logger thread exited!");
        }
    }

    fn flush(&self) {
        if self.tx.send(Message::Flush).is_err() {
            eprintln!("[Naive Logger] Logger thread exited!");
        }
    }
}

impl NaiveLoggerBackend {
    fn run(mut self) {
        while let Ok(msg) = self.rx.recv() {
            match msg {
                Message::Log(level, s) => {
                    if self.console {
                        self.write_console(level, &s);
                    }
                    if let Some(file) = &mut self.file {
                        file.write(&s);
                    }
                }
                Message::Flush => {
                    if let Some(file) = &mut self.file {
                        file.flush();
                    }
                }
                Message::Shutdown => {
                    return;
                }
            }
        }
    }

    fn write_console(&self, level: Level, s: &str) {
        let prefix;
        let suffix;
        match level {
            Level::Error => {
                // red
                prefix = "\x1b[31m";
                suffix = "\x1b[0m";
            }
            Level::Warn => {
                // yellow
                prefix = "\x1b[33m";
                suffix = "\x1b[0m";
            }
            Level::Info => {
                // green
                prefix = "\x1b[32m";
                suffix = "\x1b[0m";
            }
            Level::Debug => {
                // cyan
                prefix = "\x1b[36m";
                suffix = "\x1b[0m";
            }
            Level::Trace => {
                // no special color
                prefix = "";
                suffix = "";
            }
        }
        println!("{prefix}{s}{suffix}");
    }
}

impl NaiveLoggerBackendFile {
    fn new(path: PathBuf, rotate_size: u64, backup_file_num: usize) -> Self {
        let parent = match path.parent() {
            Some(p) => {
                if p.to_str()
                    .expect("invalid character found in log file path")
                    .is_empty()
                {
                    None
                } else {
                    Some(p.to_path_buf())
                }
            }
            None => None,
        }
        .unwrap_or(std::env::current_dir().expect("cannot determine current working directory"));
        std::fs::create_dir_all(&parent).expect("failed to create parent directory for log file");

        let file_name = path
            .file_name()
            .expect("cannot determine file name from log file path")
            .to_str()
            .expect("invalid character found in log file path");
        let file_name = regex::escape(file_name);
        let backup_filename_pattern =
            Regex::new(&format!(r"^{file_name}{BACKUP_FILENAME_PATTERN}$"))
                .expect("invalid backup file suffix pattern");

        let mut this = Self {
            path,
            parent,
            rotate_size: rotate_size << 20, // to MiB
            backup_file_num,
            backup_filename_pattern,
            file: None,
            current_size: 0,
            reopen_file_on_error_after: None,
        };
        this.open().expect("failed to open log file");

        this
    }

    fn open(&mut self) -> std::io::Result<()> {
        let mut file = match File::options()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&self.path)
        {
            Ok(f) => f,
            Err(e) => {
                self.reopen_file_on_error_after =
                    Some(LocalTimezone::now() + REOPEN_FILE_ON_ERROR_DELAY);
                return Err(e);
            }
        };
        self.current_size = match file.seek(SeekFrom::End(0)) {
            Ok(n) => n,
            Err(e) => {
                self.reopen_file_on_error_after =
                    Some(LocalTimezone::now() + REOPEN_FILE_ON_ERROR_DELAY);
                return Err(e);
            }
        };
        self.file.replace(file);
        self.reopen_file_on_error_after = None;
        Ok(())
    }

    fn reopen_on_error(&mut self) -> bool {
        let now = LocalTimezone::now();
        if let Some(datetime) = self.reopen_file_on_error_after {
            if now < datetime {
                return false;
            }
        }
        match self.open() {
            Ok(_) => true,
            Err(e) => {
                eprintln!("[Naive Logger] Failed to reopen log file on unexpected closing: {e}");
                false
            }
        }
    }

    fn write(&mut self, s: &str) {
        if self.file.is_none() && !self.reopen_on_error() {
            return;
        }
        let incr_len = s.len() as u64 + 1;
        if self.rotate_size > 0 && self.current_size + incr_len > self.rotate_size {
            self.rotate();
        }
        if let Some(file) = &mut self.file {
            if let Err(e) = file.write_fmt(format_args!("{s}\n")) {
                eprintln!("[Naive Logger] Failed to write log file: {e}");
                return;
            }
            self.current_size += incr_len;
        }
    }

    fn rotate(&mut self) {
        let now = LocalTimezone::now();
        let bak_filename = format!(
            "{}{}",
            self.path.display(),
            now.format(BACKUP_FILENAME_DATETIME_FORMAT)
        );
        self.file.take();
        if let Err(e) = std::fs::rename(&self.path, &bak_filename) {
            eprintln!("[Naive Logger] Failed to rotate log file: {e}");
            return;
        }
        if self.backup_file_num > 0 {
            self.remove_old_backups();
        }
        if let Err(e) = self.open() {
            eprintln!("[Naive Logger] Failed to reopen log file on rotation: {e}");
        }
    }

    fn remove_old_backups(&mut self) {
        let entries = match self.parent.read_dir() {
            Ok(x) => x,
            Err(e) => {
                eprintln!("[Naive Logger] Failed to read parent directory of log file: {e}");
                return;
            }
        };
        let mut backups: Vec<((u32, u32, u32), PathBuf)> = Vec::new();
        for entry in entries {
            let entry = match entry {
                Ok(x) => x,
                Err(_) => continue,
            };
            let filename = entry.file_name();
            let filename = match filename.to_str() {
                Some(x) => x,
                None => continue,
            };
            let captures = self.backup_filename_pattern.captures(filename);
            if let Some(captures) = captures {
                let date = captures
                    .get(1)
                    .and_then(|s| u32::from_str(s.as_str()).ok())
                    .expect("regex match error on group 1");
                let time = captures
                    .get(2)
                    .and_then(|s| u32::from_str(s.as_str()).ok())
                    .expect("regex match error on group 2");
                let millis = captures
                    .get(3)
                    .and_then(|s| u32::from_str(s.as_str()).ok())
                    .expect("regex match error on group 3");
                backups.push(((date, time, millis), entry.path()));
            }
        }
        if backups.len() > self.backup_file_num {
            backups.sort_by_key(|&(datetime, _)| datetime);
            for (_, path) in &backups[..backups.len() - self.backup_file_num] {
                if let Err(e) = std::fs::remove_file(path) {
                    eprintln!("[Naive Logger] Failed to remove backup file at {path:?}: {e}");
                }
            }
        }
    }

    fn flush(&mut self) {
        if self.file.is_none() && !self.reopen_on_error() {
            return;
        }
        if let Some(file) = &mut self.file {
            if let Err(e) = file.flush() {
                eprintln!("[Naive Logger] Failed to flush log file: {e}");
            }
        }
    }
}
