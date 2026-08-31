use anstream::eprintln;
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use std::{borrow::Cow, time::Duration};

#[doc(hidden)]
pub use owo_colors::Style;

#[doc(hidden)]
pub fn print(prefix: &str, style: Style, args: std::fmt::Arguments) {
    eprintln!("{:>12} {}", prefix.style(style), args);
}

#[macro_export]
macro_rules! metric {
    ($prefix:expr => $fmt:expr $(, $arg:expr)* $(,)?) => {
        const { assert!($prefix.len() <= 12, "prefix must be 12 characters or less") };
        $crate::print($prefix, $crate::Style::new().bold().cyan(), format_args!($fmt $(, $arg)*))
    };
}

#[macro_export]
macro_rules! info {
    ($prefix:expr => $fmt:expr $(, $arg:expr)* $(,)?) => {
        const { assert!($prefix.len() <= 12, "prefix must be 12 characters or less") };
        $crate::print($prefix, $crate::Style::new().bold().green(), format_args!($fmt $(, $arg)*))
    };
}

#[macro_export]
macro_rules! warn {
    ($prefix:expr => $fmt:expr $(, $arg:expr)* $(,)?) => {
        const { assert!($prefix.len() <= 12, "prefix must be 12 characters or less") };
        $crate::print($prefix, $crate::Style::new().bold().yellow(), format_args!($fmt $(, $arg)*))
    };

    ($fmt:expr $(, $arg:expr)* $(,)?) => {
        $crate::warn!("Warning" => $fmt $(, $arg)*)
    };
}

#[macro_export]
macro_rules! error {
    ($prefix:expr => $fmt:expr $(, $arg:expr)* $(,)?) => {
        const { assert!($prefix.len() <= 12, "prefix must be 12 characters or less") };
        $crate::print($prefix, $crate::Style::new().bold().red(), format_args!($fmt $(, $arg)*))
    };

    ($fmt:expr $(, $arg:expr)* $(,)?) => {
        $crate::error!("Error" => $fmt $(, $arg)*)
    };
}

pub fn spinner<T>(msg: impl Into<Cow<'static, str>>, f: impl FnOnce() -> T) -> T {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(msg);
    pb.enable_steady_tick(Duration::from_millis(100));

    let res = f();

    pb.finish_and_clear();

    res
}
