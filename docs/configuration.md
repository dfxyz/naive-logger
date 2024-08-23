# Configuration

There are two major concepts in the configuration:

* appender: controls where to write the log messages, and in what format.
* logger: filters the log messages by the given conditions, and routes them to the specified appenders.

Here is a full example of a configuration file:

```yaml
appenders:
  console_appender:
    kind: console
    stderr_level: error
    encoder:
      kind: pattern
      pattern: "{colorStart}{datetime}|{level}|{target}|{message}{kv(|)(=)}{colorEnd}"
  file_appender:
    kind: file
    path: ${PWD}/logs/main.log
    max_file_size: 128M
    max_backup_index: 2
    encoder:
      kind: pattern
      pattern: "{datetime}|{level}|{target}|{message}{kv(|)(=)}"
  error_file_encoder:
    kind: file
    path: ${PWD}/logs/error.log
    encoder:
      kind: pattern
      pattern: "{datetime}|{target}|{message}{kv(|)(=)}"
  json_file_encoder:
    kind: file
    path: ${PWD}/logs/special.log
    encoder:
      kind: json
root:
  level: "debug"
  target: "myapp::"
  target_matcher: "prefix"
  appenders:
    - console_appender
    - file_appender
loggers:
  - target: "myapp::special"
    target_matcher: "exact"
    appenders:
      - jsonfile_appender
  - target: "myapp::"
    target_matcher: "prefix"
    level: "error"
    appenders:
      - console_appender
      - error_file_encoder
  - target_prefix: "myapp::"
    target_matcher: "prefix_inverse"
    level: "warn"
```

There are three sections in the configuration file:

* `appenders`: a map of appender configurations, whose key is the appender name
* `root`: the root logger configuration
* `loggers`: a list of other logger configurations

When a log message is generated, **naive-logger** will first check the `loggers` section to find
if any one of them matches the message. The check is performed in the configuration order.
If none of them matches, try the root logger at last.

After the logger is determined, the log message will be sent to the appenders specified in the configuration,
then the appenders will write the log message to the specified destination.

In the example above, **naive-logger** will behave like this:

* if the log messages are generated for the target `myapp::special`,
  they will be written to the file `${PWD}/logs/special.log` in JSON format
* if the log messages are generated for the target starting with `myapp::`:
    * if the log level is severer than or equals to `error`,
      they will be written to both stderr and file `${PWD}/logs/error.log`
    * otherwise, the root logger will be used:
      * if the log level is severer than or equals to `debug`, they will be written to both stdout and file `${PWD}/logs/main.log`
      * otherwise, they will be ignored
* if the log messages are not generated for the target starting with `myapp::`:
  * if the log level is severer than or equals to `warn`,
    they will be written to both stdout and file `${PWD}/logs/main.log`
  * otherwise, they will be ignored

## Appender

The appender configuration is something like this:

```
<appender_name>:
  kind: <appender_kind>
  encoder: <encoder_config>
  [appender_specific_properties...]
```

The `kind` field specifies the appender type, which can be one of the following:

* `console`: write the log messages to the console (stdout or stderr)
* `file`: write the log messages to a file
  Each kind of appender has its own specific properties

The `encoder` field specifies the encoder configuration for the appender, which will be described later.

### Console Appender

The `console` appender configuration is like this:

```
<appender_name>:
  kind: console
  [common_appender_properties...]
  stderr_level: <stderr_level>
```

The optional `stderr_level` field controls whether the log message will be written to stderr.
If the log level is severer than this value, the log message will be written to stderr.
The log level can be one of the following: [`off`, `error`, `warn`, `info`, `debug`, `trace`].
The default value is `off`, meaning all the log messages will be written to stdout.

### File Appender

The `file` appender configuration is like this:

```
<appender_name>:
  kind: file
  [common_appender_properties...]
  path: <log_file_path>
  max_file_size: <max_file_size>
  max_backup_index: <max_backup_index>
```

The required `path` field specifies the path of the log file. Environment variables are supported if wrapped by `${}`.

The optional `max_file_size` fields specifies the maximum size of the log file.
When the log file reaches this size, it will be rotated.
The value should be a number followed by an optional unit, which can be one of the following: `k/K/m/M/g/G`.
The default value is `0`, meaning the log file will not be rotated.

The optional `max_backup_index` field specifies the maximum number of backup files to keep.
When the log file is rotated, the rotated files will be renamed with suffix `.0`, `.1`, `.2`, etc.
The default value is `0`, meaning only one backup file will be kept.

## Encoder

The encoder configuration is used inside the appender configuration. It is something like this:

```
encoder:
  kind: <encoder_kind>
  [encoder_specific_properties...]
```

The `kind` field specifies the encoder type, which can be one of the following:

* `pattern`: format the log message with a customizable pattern
* `json`: format the log message as JSON object

### Pattern Encoder

The `pattern` encoder configuration is like this:

```
encoder:
  kind: pattern
  pattern: <pattern>
```

The optional `pattern` field specifies the pattern to format the log message. It's constructed by the following placeholders:

* `{datetime([format])}`: the datetime when the log message is generated, formatted by a format argument
  which should be valid format string (see `chrono::format::strftime` for details)
  * `[format]`: the format string used by `chrono` (see `chrono::format::strftime` for details);
    optional, default is `%Y-%m-%dT%H:%M:%S%.3f%z`
* `{level}`: the level of the message
* `{target}`: the target of the message
* `{module}`: the module path where the message is generated; if none, `<unknown>` will be used
* `{file}`: the file path where the message is generated; if none, `<unknown>` will be used
* `{line}`: the line number where the message is generated; if none, `0` will be used
* `{message}`: the log message itself
* `{kv(<pairSeparator>)(<keyValueSeparator>)}...`: the key-value pairs in the log message
    * `<pairSeparator>`: the separator inserted before each pair; required
    * `<keyValueSeparator>`: the separator between key and value; required
* `{colorStart}`: the escape sequence to start colorizing the message; the color is determined by the log level:
  * `ERROR`: `\x1b[31m` (red)
  * `WARN`:  `\x1b[33m` (yellow)
  * `INFO`:  `\x1b[32m` (green)
  * `DEBUG`: `\x1b[36m` (cyan)
  * `TRACE`: `\x1b[35m` (magenta)
* `{colorEnd}`: the escape sequence to end colorizing the message

There's rare need to use '{' or '}' in the pattern, or '(' or ')' in the argument of placeholder.
So, for the sake of simplicity, escaping those characters is not implemented:
* literal '{' **is not** allowed in the pattern
* literal ')' **is not** allowed in the argument of placeholder

If `pattern` is not specified, the default pattern will be used:
```
{datetime}|{level}|{target}|{message}{kv(|)(=)}
```

It may output something like this:
```
2024-07-31T12:34:56.789000+08:00|INFO|myapp::test|this is a log message with no kv pair
2024-07-31T12:34:56:789001+08:00|ERROR|myapp::test|something is wrong|context_id=42|source=external
```

### JSON Encoder

The `json` encoder configuration is like this:

```
encoder:
  kind: json
```

It doesn't have any specific properties.

It may output something like this:
```
{"timestamp":1722441599998,"level":"INFO","target":"myapp::test","module":"myapp::test","file":"src/main.rs","line":42,"message":"this is a log message with no kv pair"}
{"timestamp":1722441599999,"level":"ERROR","target":"myapp::test","module":"myapp::test","file":"src/main.rs","line":43,"message":"something is wrong","context_id":42,"source":"external"}
```

## Logger

The logger configuration is like this:

```
level: <level>
target: <target>
target_matcher: <target_matcher>
appenders: [appender_names]
```

The optional `level` field filters the log messages by the log level, which can be one of the following:
[`off`, `error`, `warn`, `info`, `debug`, `trace`].
If the level is lower than the specified value, it will be ignored by the current logger.
The default value is `info`.

The value of optional `target` field should be a string.
Combined with `target_matcher`, it filters the log messages by the target with the following methods:
* if the target exactly equals to the `target`
* if the target starts with the `target`
* if the target doesn't start with the `target`
 
If not specified, the logger won't filter the log messages by the target.

The value of optional `target_matcher` field should be one of the following:
* `exact`: matches the target which exactly equals to the `target`
* `prefix`: matches the target which starts with the `target`
* `prefix_inverse`: matches the target which doesn't start with the `target`

The default value is `prefix`.

The value of `appenders` field should be a list of the appender names.
It's required for the root logger, and optional for the non-root loggers.
If not specified for the non-root loggers, the appenders of the root logger will be used.
