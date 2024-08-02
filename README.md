# naive-logger

**naive-logger** is a simple configurable logger for Rust.

This logger implements the trait required by `log` crate, and provides the following configuration capabilities:

* how to route the log message
* how to encode the log message
* where to write the log message

See [docs/configuration.md](docs/configuration.md) for the details.

## Quick Start

Make a configuration file `program.logger.yaml`:

```yaml
appenders:
  console:
    kind: console
  file:
    kind: file
    path: program.log
root:
  level: info
  appenders:
    - console
    - file
```

Initialize the logger in your program:

```rust
use log::info;

fn main() {
    naive_logger::init("program.logger.yaml").unwrap();
    info!("too young, too simple, sometimes naive.");
    // ...
}
```
