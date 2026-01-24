# Naive Logger

「Naive Logger」是一个极简的Rust异步日志记录器，它提供下列功能：

- 可通过环境变量或编码进行配置
- 将日志输出到控制台或文件（支持按大小进行滚动文件）

## 配置说明

### 日志等级

使用Debug编译时，默认的日志等级为`debug`；使用Release编译时，默认的日志等级为`info`。

可以通过环境变量`NAIVE_LOG_LEVEL=[target=][level][,...]`设置日志等级，其中等级字段不区分大小写。

示例：

- 修改默认日志等级：`NAIVE_LOG_LEVEL=info`
- 按目标单独设置日志等级：`NAIVE_LOG_LEVEL=myapp=debug,other=off`
- 同时设置默认日志等级与某些目标的日志等级：`NAIVE_LOG_LEVEL=off,myapp=info`

### 是否使用控制台输出日志

默认为使用。可以通过环境变量`NAIVE_LOG_CONSOLE=0`来禁用控制台输出。

### 是否使用文件输出日志

默认为不使用。可以通过环境变量`NAIVE_LOG_FILE=<path>`来配置日志文件路径与启用文件输出。

示例：`NAIVE_LOG_FILE=./naive.log`。

### 是否按大小滚动日志文件

默认为禁用。可以通过环境变量`NAIVE_LOG_FILE_ROTATE_SIZE=<size>`来设置日志文件滚动大小，单位为：MiB。注意：0表示禁用该功能。

示例：`NAIVE_LOG_FILE_ROTATE_SIZE=128`。

### 限制滚动生成的备份文件总数

默认为不限制。可以通过环境变量`NAIVE_LOG_BACKUP_FILE_NUM=<num>`来设置备份文件的最大保留数量。注意：0表示不限制。

示例：`NAIVE_LOG_BACKUP_FILE_NUM=4`。
