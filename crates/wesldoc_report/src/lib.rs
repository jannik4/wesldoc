use anstream::eprintln;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use std::{sync::LazyLock, time::Duration};

#[doc(hidden)]
pub use owo_colors::Style;

#[doc(hidden)]
pub fn print(prefix: &str, style: Style, args: std::fmt::Arguments) {
    PROGRESS.suspend(|| {
        eprintln!("{:>12} {}", prefix.style(style), args);
    });
}

#[macro_export]
macro_rules! metric {
    ($prefix:expr => $fmt:expr $(, $arg:expr)* $(,)?) => {
        const { assert!($prefix.len() <= 12, "prefix must be 12 characters or less") };
        $crate::print($prefix, $crate::Style::new().bold().purple(), format_args!($fmt $(, $arg)*))
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

static PROGRESS: LazyLock<MultiProgress> = LazyLock::new(MultiProgress::new);

pub fn spinner<T>(msg: &str, f: impl FnOnce() -> T) -> T {
    let pb = ProgressBar::new_spinner();
    PROGRESS.add(pb.clone());
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(msg.style(Style::new().bold().cyan()).to_string());
    pb.enable_steady_tick(Duration::from_millis(100));

    let res = f();

    pb.finish_and_clear();

    res
}

pub struct ProgressBarHandle<'a> {
    pb: &'a ProgressBar,
}

impl ProgressBarHandle<'_> {
    pub fn inc(&self, delta: u64) {
        self.pb.inc(delta);
    }

    pub fn set_position(&self, pos: u64) {
        self.pb.set_position(pos);
    }
}

pub fn progress<T>(msg: &str, len: u64, f: impl FnOnce(ProgressBarHandle) -> T) -> T {
    // TODO: (comp time) check that msg.len() <= 12

    let prefix = format!("{:>12}", msg.style(Style::new().bold().cyan()));

    let pb = ProgressBar::new(len);
    PROGRESS.add(pb.clone());
    pb.set_style(
        ProgressStyle::with_template(&format!("{prefix} {{wide_bar}} {{pos}}/{{len}}")).unwrap(),
    );

    let res = f(ProgressBarHandle { pb: &pb });

    pb.finish_and_clear();

    res
}
