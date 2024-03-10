#![allow(unused_imports)]

#[macro_export]
macro_rules! __log {
    // __log!(log_level; "hello {}", "world");
    ($level:expr; $($arg:expr),+) => {
        ::log::log!($level, $($arg),+);
    };

    // __log!(log_level; "hello {}", "world"; arg1="value1", arg2="value2");
    ($level:expr; $($arg:expr),+ ; $($key:tt $(:$capture:tt)? $(= $value:expr)?),+) => {
        ::log::log!($level, $($key $(:$capture)? $(= $value)?),+; $($arg),+);
    }
}

#[macro_export]
macro_rules! trace {
    // trace!("hello {}", "world");
    // trace!("hello {}", "world"; arg1="value1", arg2="value2");
    ($($arg:tt)+) => {
        $crate::__log!(::log::Level::Trace; $($arg)+);
    };
}

#[macro_export]
macro_rules! debug {
    // debug!("hello {}", "world");
    // debug!("hello {}", "world"; arg1="value1", arg2="value2");
    ($($arg:tt)+) => {
        $crate::__log!(::log::Level::Debug; $($arg)+);
    };
}

#[macro_export]
macro_rules! info {
    // info!("hello {}", "world");
    // info!("hello {}", "world"; arg1="value1", arg2="value2");
    ($($arg:tt)+) => {
        $crate::__log!(::log::Level::Info; $($arg)+);
    };
}

#[macro_export]
macro_rules! warn {
    // warn!("hello {}", "world");
    // warn!("hello {}", "world"; arg1="value1", arg2="value2");
    ($($arg:tt)+) => {
        $crate::__log!(::log::Level::Warn; $($arg)+);
    };
}

#[macro_export]
macro_rules! error {
    // error!("hello {}", "world");
    // error!("hello {}", "world"; arg1="value1", arg2="value2");
    ($($arg:tt)+) => {
        $crate::__log!(::log::Level::Error; $($arg)+);
    };
}
