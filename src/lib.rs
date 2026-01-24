use chrono::Local as LocalTimezone;
type LocalDateTime = chrono::DateTime<LocalTimezone>;

use log::{Level, LevelFilter, Log};
pub use log::{debug, error, info, log_enabled, trace, warn};

use regex::Regex;

const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";
const FILENAME_SUFFIX_FORMAT: &str = "%Y%m%d.log";
const DEFAULT_FILE_AGE: u32 = 3;
const DEFAULT_CHANNEL_SIZE: usize = 32;
const RETRY_OPEN_FILE_DELAY: std::time::Duration = std::time::Duration::from_secs(10);

/// 使用环境变量`RUST_LOG`中的配置初始化「Naive Logger」；返回一个`Option<NaiveLoggerDropGuard>`用于控制与同步日志文件任务的结束。
///
/// 持有返回值期间日志功能正常运行；丢弃后日志线程退出，后续日志调用静默忽略。
///
/// `RUST_LOG`的格式为：`RUST_LOG=[target][=][level][...][;key[=value]][...]`
/// - 分号之前的部分，配置方式与`env_logger`部分兼容，例如：
///     - `RUST_LOG=info`设置默认日志等级
///     - `RUST_LOG=info,someTarget=error`设置默认日志等级和日志目标匹配`^someTarget`的日志等级
///     - `RUST_LOG=myCrate,yourCrate=warn`完全打开`^myCrate`的日志，但`^yourCrate`的日志设为WARN
///     - 注意：不支持`RUST_LOG=http/statusCode=4..`这种过滤日志内容的配置方法
/// - 部分之后的部分为其他的选项：
///     - `FILE=<str>`：日志文件的输出路径的前缀部分（后缀固定为`<DATE>.log`的形式）
///         - 设置时开启日志文件输出功能；默认为关
///         - 设置为空字符串时，日志将输出到当前工作目录的`<DATE>.log`文件中
///     - `AGE=<number>`：日志文件的最大保留天数；日志文件跨天滚动时，会自动删除过旧的文件
///         - 默认的最大保留天数为3天
///         - 设置为0时不自动删除任何日志文件
///     - `CONSOLE_OFF`：关闭终端中的日志输出；只在开启日志文件输出功能时生效
pub fn init() -> Option<NaiveLoggerDropGuard> {
    let config = NaiveLoggerConfig::parse().unwrap();
    let max_level = config.max_level();
    let console = if config.log_file_name_prefix.is_none() || !config.console_off {
        Some(std::io::stdout())
    } else {
        None
    };
    let logger;
    let drop_guard;
    if let Some(prefix) = config.log_file_name_prefix {
        let max_age = config.log_file_max_age.unwrap_or(DEFAULT_FILE_AGE);
        let (tx, rx) = crossbeam_channel::bounded(DEFAULT_CHANNEL_SIZE);
        let log_file_task = LogFileTask {
            rx,
            console_off: config.console_off,
            file_name_prefix: prefix,
            file_name_prefix_pattern: None,
            file_max_age: max_age,
            file: None,
            retry_open_file_after: None,
        };
        let jh = std::thread::spawn(move || log_file_task.run());
        logger = NaiveLogger {
            max_level,
            default_level: config.default_level,
            target_level_filters: config.target_level_filters,
            console,
            file_task_tx: Some(tx.clone()),
        };
        drop_guard = Some(NaiveLoggerDropGuard {
            tx,
            join_handle: Some(jh),
        });
    } else {
        logger = NaiveLogger {
            max_level,
            default_level: config.default_level,
            target_level_filters: config.target_level_filters,
            console,
            file_task_tx: None,
        };
        drop_guard = None;
    }
    log::set_max_level(max_level);
    log::set_logger(Box::leak(Box::new(logger))).expect("init function should only be called once");
    drop_guard
}

type TargetLevelFilter = (String, LevelFilter);

struct NaiveLoggerConfig {
    default_level: LevelFilter,
    target_level_filters: Vec<TargetLevelFilter>,
    log_file_name_prefix: Option<String>,
    log_file_max_age: Option<u32>,
    console_off: bool,
}
impl Default for NaiveLoggerConfig {
    fn default() -> Self {
        Self {
            default_level: LevelFilter::Error,
            target_level_filters: Default::default(),
            log_file_name_prefix: Default::default(),
            log_file_max_age: Default::default(),
            console_off: Default::default(),
        }
    }
}
impl NaiveLoggerConfig {
    /// 解析`RUST_LOG`环境变量字符串，返回配置结构体
    fn parse() -> Result<Self, String> {
        let mut this = Self::default();
        let Ok(env_value) = std::env::var("RUST_LOG") else {
            return Ok(this);
        };

        // 按分号将配置分为「日志等级配置」和「其他选项」两部分
        let parts: Vec<&str> = env_value.splitn(2, ';').collect();
        let level_part = parts[0].trim();

        // ---- 解析「日志等级配置」 ----
        if !level_part.is_empty() {
            for item in level_part.split(',') {
                let item = item.trim();
                if item.is_empty() {
                    continue;
                }
                if item.contains('=') {
                    // 包含'='：解析为`target=level`格式
                    let filter = Self::parse_target_level_filter(item)?;
                    this.target_level_filters.push(filter);
                } else {
                    // 不含'='，可能是默认日志等级配置，也可能是某个日志目标所有等级全部开启
                    if let Some(level) = Self::try_parse_level(item) {
                        this.default_level = level;
                    } else {
                        let filter = Self::parse_target_level_filter(item)?;
                        this.target_level_filters.push(filter);
                    }
                }
            }
        }

        // ---- 解析「其他选项」 ----
        if parts.len() > 1 {
            for option in parts[1].trim().split(';') {
                let option = option.trim();
                let parts = option.trim().split('=').collect::<Vec<&str>>();
                let key = parts[0].trim();
                let value = if parts.len() > 1 { parts[1].trim() } else { "" };
                match key {
                    "CONSOLE_OFF" => {
                        this.console_off = true;
                    }
                    "FILE" => {
                        this.log_file_name_prefix = Some(value.to_owned());
                    }
                    "AGE" => {
                        let age: u32 = value
                            .parse()
                            .map_err(|_| format!("invalid option: 'AGE={value}'",))?;
                        this.log_file_max_age = Some(age);
                    }
                    _ => {
                        return Err(format!("invalid option: '{key}={value}'"));
                    }
                }
            }
        }

        Ok(this)
    }

    #[inline]
    fn try_parse_level(s: &str) -> Option<LevelFilter> {
        use std::str::FromStr;
        LevelFilter::from_str(s).ok()
    }

    /// 解析特定日志目标的日志等级配置字符串
    fn parse_target_level_filter(s: &str) -> Result<TargetLevelFilter, String> {
        if let Some(eq_pos) = s.find('=') {
            let target = &s[..eq_pos];
            let level_str = &s[eq_pos + 1..];
            let level = Self::try_parse_level(level_str).ok_or(format!(
                "failed to parse level of target '{target}': '{level_str}'"
            ))?;
            Ok((target.to_owned(), level))
        } else {
            Ok((s.to_owned(), LevelFilter::Trace))
        }
    }

    fn max_level(&self) -> LevelFilter {
        let mut max_level = self.default_level;
        for (_, level) in &self.target_level_filters {
            if level > &max_level {
                max_level = *level;
            }
        }
        max_level
    }
}

pub struct NaiveLoggerDropGuard {
    tx: crossbeam_channel::Sender<LogFileTaskMessage>,
    join_handle: Option<std::thread::JoinHandle<()>>,
}
impl Drop for NaiveLoggerDropGuard {
    fn drop(&mut self) {
        let _ = self.tx.send(LogFileTaskMessage::Shutdown);
        if let Some(h) = self.join_handle.take() {
            let _ = h.join();
        }
    }
}

struct NaiveLogger {
    max_level: LevelFilter,
    default_level: LevelFilter,
    target_level_filters: Vec<TargetLevelFilter>,
    console: Option<std::io::Stdout>,
    file_task_tx: Option<crossbeam_channel::Sender<LogFileTaskMessage>>,
}
impl Log for NaiveLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let level = metadata.level();
        if level > self.max_level {
            return false;
        }
        for (target, filter_level) in &self.target_level_filters {
            if metadata.target().starts_with(target.as_str()) {
                return level <= *filter_level;
            }
        }
        level <= self.default_level
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        // 格式化日志
        let now = LocalTimezone::now();
        let datetime = now.format(DATETIME_FORMAT);
        let level = record.level();
        let target = record.target();
        let args = record.args();
        #[cfg(feature = "kv")]
        let content = {
            struct KvCollector(String);
            impl<'kvs> log::kv::VisitSource<'kvs> for KvCollector {
                fn visit_pair(
                    &mut self,
                    key: log::kv::Key<'kvs>,
                    value: log::kv::Value<'kvs>,
                ) -> Result<(), log::kv::Error> {
                    use std::fmt::Write;
                    let _ = write!(self.0, "{key}={value}|");
                    Ok(())
                }
            }
            let mut collector = KvCollector(String::new());
            let _ = record.key_values().visit(&mut collector);
            let kv = collector.0;
            format!("{datetime}|{level:>5}|{target}|{kv}{args}")
        };
        #[cfg(not(feature = "kv"))]
        let content = format!("{datetime}|{level:>5}|{target}|{args}");

        // 同步输出到终端
        if let Some(console) = &self.console {
            use std::io::Write;
            let (prefix, suffix) = match level {
                Level::Error => ("\x1b[31m", "\x1b[0m"), // 红
                Level::Warn => ("\x1b[33m", "\x1b[0m"),  // 黄
                Level::Info => ("\x1b[32m", "\x1b[0m"),  // 绿
                Level::Debug => ("\x1b[36m", "\x1b[0m"), // 青
                Level::Trace => ("", ""),                // 无特殊颜色
            };
            let _ = writeln!(console.lock(), "{prefix}{content}{suffix}");
        }

        // 异步输出到文件
        if let Some(tx) = &self.file_task_tx {
            let _ = tx.send(LogFileTaskMessage::Log(now, content));
        }
    }

    fn flush(&self) {
        // 同步刷新终端缓冲区
        if let Some(console) = &self.console {
            use std::io::Write;
            let _ = console.lock().flush();
        }

        // 异步刷新文件缓冲区
        if let Some(tx) = &self.file_task_tx {
            let _ = tx.send(LogFileTaskMessage::Flush);
        }
    }
}

enum LogFileTaskMessage {
    Log(LocalDateTime, String),
    Flush,
    Shutdown,
}

struct LogFileTask {
    rx: crossbeam_channel::Receiver<LogFileTaskMessage>,
    console_off: bool,
    file_name_prefix: String,
    file_name_prefix_pattern: Option<Regex>,
    file_max_age: u32,
    file: Option<(std::fs::File, chrono::NaiveDate)>,
    retry_open_file_after: Option<LocalDateTime>,
}
impl LogFileTask {
    fn run(mut self) {
        while let Ok(msg) = self.rx.recv() {
            match msg {
                LogFileTaskMessage::Log(datetime, content) => {
                    self.write(datetime, content);
                }
                LogFileTaskMessage::Flush => {
                    self.flush();
                }
                LogFileTaskMessage::Shutdown => {
                    // 排空队列中剩余的日志消息，避免关闭时丢失
                    while let Ok(remaining) = self.rx.try_recv() {
                        if let LogFileTaskMessage::Log(datetime, content) = remaining {
                            self.write(datetime, content);
                        }
                    }
                    self.flush();
                    return;
                }
            }
        }
    }

    fn eprintln(args: std::fmt::Arguments) {
        eprintln!(
            "{date}|ERROR|{module}|{args}",
            date = LocalTimezone::now().format(DATETIME_FORMAT),
            module = module_path!(),
        );
    }

    fn set_file_and_date(&mut self, file: std::fs::File, date: chrono::NaiveDate) {
        self.file = Some((file, date));
        self.retry_open_file_after = None;
    }

    fn unset_file_and_date_on_error(&mut self) {
        self.file = None;
        self.retry_open_file_after = Some(LocalTimezone::now() + RETRY_OPEN_FILE_DELAY);
    }

    /// 按给定的记录日期打开相应的日志文件
    fn open_file(&mut self, date: chrono::NaiveDate) -> std::io::Result<()> {
        if let Some(retry_after) = self.retry_open_file_after {
            if LocalTimezone::now() < retry_after {
                return Ok(()); // 上次打开文件失败，延迟一段时间后重试
            }
        }
        let path = if self.file_name_prefix.is_empty() {
            date.format(FILENAME_SUFFIX_FORMAT).to_string()
        } else {
            format!(
                "{}.{}",
                self.file_name_prefix,
                date.format(FILENAME_SUFFIX_FORMAT)
            )
        };
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| {
                Self::eprintln(format_args!("Failed to open log file {path:?}: {e}"));
                self.unset_file_and_date_on_error();
                e
            })?;
        self.set_file_and_date(file, date);
        self.remove_old_files();
        Ok(())
    }

    /// 删除过旧的日志文件（在打开日志文件时调用）
    fn remove_old_files(&mut self) {
        if self.file_max_age == 0 {
            return;
        }
        let cutoff_date = if let Some((_, date)) = self.file.as_ref() {
            *date - chrono::Duration::days(self.file_max_age as i64)
        } else {
            return;
        };
        let dir = std::path::Path::new(&self.file_name_prefix)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."));
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                Self::eprintln(format_args!(
                    "Failed to read directory to remove old log files: {e}"
                ));
                return;
            }
        };
        if self.file_name_prefix_pattern.is_none() {
            self.file_name_prefix_pattern = if self.file_name_prefix.is_empty() {
                Some(Regex::new(r"^(\d{8})\.log$").expect("invalid regex pattern"))
            } else {
                let stem = std::path::Path::new(&self.file_name_prefix)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&self.file_name_prefix);
                let prefix = regex::escape(stem);
                Some(
                    Regex::new(&format!(r"^{prefix}\.(\d{{8}})\.log$"))
                        .expect("invalid regex pattern"),
                )
            };
        }
        let pattern = self
            .file_name_prefix_pattern
            .as_ref()
            .expect("regex pattern not initialized");
        for entry in entries.flatten() {
            let filename = entry.file_name();
            let Some(name) = filename.to_str() else {
                continue;
            };
            if let Some(captures) = pattern.captures(name) {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(
                    captures.get(1).expect("regex group 1 not found").as_str(),
                    "%Y%m%d",
                ) {
                    if date <= cutoff_date {
                        let path = entry.path();
                        if let Err(e) = std::fs::remove_file(&path) {
                            Self::eprintln(format_args!(
                                "failed to remove old log file at {path:?}: {e}",
                            ));
                        }
                    }
                }
            }
        }
    }

    /// 将接收到的日志消息写入文件
    fn write(&mut self, datetime: LocalDateTime, content: String) {
        let date = datetime.date_naive();
        match self.file {
            Some((_, file_date)) => {
                if file_date != date {
                    let _ = self.open_file(date);
                }
            }
            None => {
                let _ = self.open_file(date);
            }
        }

        // 将日志内容写入文件；如果文件打开异常，且禁用了同步的终端输出，这里需要补打漏掉的日志作为兜底
        if let Some((file, _)) = self.file.as_mut() {
            use std::io::Write;
            if let Err(e) = writeln!(file, "{content}") {
                Self::eprintln(format_args!("failed to write log file: {e}"));
                self.unset_file_and_date_on_error();
                if self.console_off {
                    eprintln!("{content}");
                }
            }
        } else {
            if self.console_off {
                eprintln!("{content}");
            }
        }
    }

    fn flush(&mut self) {
        if let Some((file, _)) = self.file.as_mut() {
            use std::io::Write;
            if let Err(e) = file.flush() {
                Self::eprintln(format_args!("failed to flush log file: {e}"));
                self.unset_file_and_date_on_error();
            }
        }
    }
}
