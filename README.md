# Naive Logger

「Naive Logger」是一个简单的Rust日志记录器，基于 [`log`](https://docs.rs/log) 门面库。

**功能：**

- 通过环境变量`RUST_LOG`完成全部配置（配置方式类似[env_logger](https://github.com/rust-cli/env_logger)）
- 支持按目标（target）单独设置日志等级
- 将日志同步输出到控制台和/或异步输出到文件
- 日志文件按日期滚动，并可配置备日志文件的最大保留天数
- 开启`kv`feature时，支持`log`的结构化键值对记录功能

## 快速开始

```rust
fn main() {
    let _guard = naive_logger::init();
    log::info!("Hello, world!");
}
```

调用`init()`完成相关的初始化流程，并返回一个`Option<NaiveLoggerDropGuard>`；开启日志文件输出时，返回值将用于控制与同步日志文件输出任务的关闭与清理。

## 日志格式

```
{datetime}|{level}|{target}|{message}
```

示例：

```
2026-04-25 12:34:56.789| INFO|my_crate::module|server started on port 8080
2026-04-25 12:34:57.001|ERROR|my_crate::db|connection refused: timeout
```

启用`kv`feature时，所有键值对将以`key=value|...|`的形式添加在消息之前：

```
2026-04-25 12:34:56.789| INFO|my_crate|method=GET|status=200|request handled
```

## 配置说明

通过环境变量`RUST_LOG`进行配置，格式为：

```
RUST_LOG=[<日志等级配置>][;<选项>[;...]]
```

### 日志等级配置

日志等级配置部分的格式为`[target][=][level][...]`，基本与`env_logger`的配置方式兼容（不支持使用正则表达式过滤日志内容）。

| 示例 | 说明 |
|------|------|
| `RUST_LOG=info` | 设置默认日志等级为INFO |
| `RUST_LOG=warn,my_crate=debug` | 默认WARN，`my_crate`开头的目标使用DEBUG |
| `RUST_LOG=off,my_crate=info` | 默认关闭所有日志输出，但`my_crate`开头的目标使用INFO |
| `RUST_LOG=my_crate,your_crate=warn` | `my_crate`开头的目标启用所有等级的日志，`your_crate`开头的目标使用WARN |

目标匹配规则为前缀匹配（`starts_with`）；等级字段不区分大小写，可选值：`trace`、`debug`、`info`、`warn`、`error`、`off`。

默认日志等级为ERROR。

### 选项

位于第一个`;`之后的字符串将当作其他选项来解析，格式为`key[=<value>];...`。

#### `FILE=<path>`

配置时开启日志文件输出，`path`为文件路径的前缀部分。

- 设置为空字符串时，日志文件输出到当前工作目录，文件名为 `<YYYYMMDD>.log`
- 未设置时，不开启文件输出

```
RUST_LOG=info;FILE=logs/app
# 生成文件：logs/app.20260425.log
```

```
RUST_LOG=info;FILE
# 生成文件：<YYYYMMDD>.log
```

#### `AGE=<days>`

日志文件的最大保留天数；在特定时刻，会自动检查并删除过于旧的日志文件。

- 默认值：`3`
- 设置为`0`时不限制日志文件的保留天数

```
RUST_LOG=info;FILE=logs/app;AGE=7
```

#### `CONSOLE_OFF`

禁用控制台同步输出，仅在开启文件输出（设置了`FILE`）时生效。

```
RUST_LOG=info;FILE=logs/app;CONSOLE_OFF
```

### 完整示例

```
RUST_LOG=info,my_crate,your_crate=error;FILE=my_app;AGE=7;CONSOLE_OFF
```
