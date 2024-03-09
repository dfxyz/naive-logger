#![allow(unused_imports)]

#[macro_export]
macro_rules! __log {
    // log!(log_target; log_level; "hello {}", "world");
    ($target:expr; $level:expr; $($arg:expr),+) => {
        ::log::log!(target: $target, $level, $($arg),+);
    };

    // log!(log_target; log_level; "hello {}", "world";
    //      key0,
    //      key1:? = something_debug,
    //      key2:% = something_display,
    //      key3:err = something_error,
    //      key4 = something);
    ($target:expr; $level:expr; $($arg:expr),+ ; $($key:tt $(:$capture:tt)? $(= $value:expr)?),+) => {
        ::log::log!(target: $target, $level, $($key $(:$capture)? $(= $value)?),+; $($arg),+);
    }
}

#[macro_export]
macro_rules! trace {
    // trace!(target: module_path!(); "hello {}", "world");
    (target: $target:expr, $($arg:tt)+) => {
        $crate::__log!($target; ::log::Level::Trace; $($arg)+);
    };

    // trace!("hello {}", "world");
    // trace!("hello {}", "world";
    //      key0,
    //      key1:? = something_debug,
    //      key2:% = something_display,
    //      key3:err = something_error,
    //      key4 = something);
    ($($arg:tt)+) => {
        $crate::__log!(module_path!(); ::log::Level::Trace; $($arg)+);
    };
}

#[macro_export]
macro_rules! debug {
    // debug!(target: module_path!(); "hello {}", "world");
    (target: $target:expr, $($arg:tt)+) => {
        $crate::__log!($target; ::log::Level::Debug; $($arg)+);
    };

    // debug!("hello {}", "world");
    // debug!("hello {}", "world";
    //      key0,
    //      key1:? = something_debug,
    //      key2:% = something_display,
    //      key3:err = something_error,
    //      key4 = something);
    ($($arg:tt)+) => {
        $crate::__log!(module_path!(); ::log::Level::Debug; $($arg)+);
    };
}

#[macro_export]
macro_rules! info {
    // info!(target: module_path!(); "hello {}", "world");
    (target: $target:expr, $($arg:tt)+) => {
        $crate::__log!($target; ::log::Level::Info; $($arg)+);
    };

    // info!("hello {}", "world");
    // info!("hello {}", "world";
    //      key0,
    //      key1:? = something_debug,
    //      key2:% = something_display,
    //      key3:err = something_error,
    //      key4 = something);
    ($($arg:tt)+) => {
        $crate::__log!(module_path!(); ::log::Level::Info; $($arg)+);
    };
}

#[macro_export]
macro_rules! warn {
    // warn!(target: module_path!(); "hello {}", "world");
    (target: $target:expr, $($arg:tt)+) => {
        $crate::__log!($target; ::log::Level::Warn; $($arg)+);
    };

    // warn!("hello {}", "world");
    // warn!("hello {}", "world";
    //      key0,
    //      key1:? = something_debug,
    //      key2:% = something_display,
    //      key3:err = something_error,
    //      key4 = something);
    ($($arg:tt)+) => {
        $crate::__log!(module_path!(); ::log::Level::Warn; $($arg)+);
    };
}

#[macro_export]
macro_rules! error {
    // error!(target: module_path!(); "hello {}", "world");
    (target: $target:expr, $($arg:tt)+) => {
        $crate::__log!($target; ::log::Level::Error; $($arg)+);
    };

    // error!("hello {}", "world");
    // error!("hello {}", "world";
    //      key0,
    //      key1:? = something_debug,
    //      key2:% = something_display,
    //      key3:err = something_error,
    //      key4 = something);
    ($($arg:tt)+) => {
        $crate::__log!(module_path!(); ::log::Level::Error; $($arg)+);
    };
}
