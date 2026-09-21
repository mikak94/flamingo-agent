//! File logger

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use log::{Level, LevelFilter, Metadata, Record};

use crate::error::Result;

struct FileLogger {
    file: File,
}

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!(
            "{} [{:<5}] {}\n",
            crate::metrics::format_utc(chrono::Utc::now()),
            record.level(),
            record.args()
        );
        let _ = std::io::stderr().write_all(line.as_bytes());
        let _ = (&self.file).write_all(line.as_bytes());
    }

    fn flush(&self) {
        let _ = (&self.file).flush();
    }
}

pub fn init(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    if log::set_boxed_logger(Box::new(FileLogger { file })).is_ok() {
        log::set_max_level(LevelFilter::Debug);
    }
    Ok(())
}
