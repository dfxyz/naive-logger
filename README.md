# naive-logger

`naive-logger` is an asynchronous logger implementation for Rust. It provides following capabilities:

* asynchronous: logging is performed in a separate thread
* selectable destination: stdout, or file, or both
* basic colorization support: when logging to stdout, each line can be optionally colorized based on the log level
* basic file rotation: when logging to file, the log file will be rotated by a simple size-based policy

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
naive-logger = { git = "https://github.com/dfxyz/naive-logger.git" }
```

Then in the application code, configure and initialize the logger:

```rust
fn main() {
    // configure the naive-logger
    let config = naive_logger::Config {
        level: log::LevelFilter::Debug,
        ..Default::default()
    };
    
    // keep the drop guard
    let _logger = naive_logger::init(&config).unwrap();
    
    // use the macros from `log` crate
    log::info!("Hello, world!");
}
```

If feature `serde_support` is enabled, you can use `serde` to deserialize the configuration.
Here is a toml example:

```rust
fn main() {
    let s = r#"
    level = "trace"

    [stdout]
    enable = true
    use_color = true

    [file]
    enable = true
    filename = "program.log"
    rotate_size = "1G"
    max_rotated_num = 4
    "#;
    let config = toml::from_str::<naive_logger::Config>(s).unwrap();
    let _logger = naive_logger::init(&config).unwrap();

    log::info!("Hello, world!");
}
```
