#![no_std]

use log::{Level, LevelFilter, Log, Metadata, Record};
use polished_serial_logging::serial_println;

struct SerialLogger;

impl Log for SerialLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let file = record
                .file()
                .and_then(|s| s.rsplit('/').next())
                .unwrap_or("???");
            let line = record.line().unwrap_or(0);
            serial_println!("[{}] ({}:{}) {}", record.level(), file, line, record.args());
        }
    }

    fn flush(&self) {}
}

#[cfg(debug_assertions)]
pub fn init_logger() {
    log::set_logger(&SerialLogger).expect("logger already set");
    log::set_max_level(LevelFilter::Debug);
}

#[cfg(not(debug_assertions))]
pub fn init_logger() {
    log::set_logger(&SerialLogger).expect("logger already set");
    log::set_max_level(LevelFilter::Warn);
}

pub use log::{debug, error, info, trace, warn};
