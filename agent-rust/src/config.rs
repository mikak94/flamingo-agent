use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::{Error, Result};

pub const SERVICE_NAME: &str = "FlamingoAgent";
pub const SERVICE_DISPLAY_NAME: &str = "Flamingo Metrics Agent";
pub const SERVICE_DESCRIPTION: &str = "Gets the UTC time and its own memory use every few seconds and passes them to logger-child, which writes them to an admins-only log.";
/// Appended to the registered command line so the SCM-launched copy knows.
pub const SERVICE_ARG: &str = "--service";
pub const INTERVAL_SECS: u64 = 5;

#[derive(Debug, Clone)]
pub struct Config {
    pub interval: Duration,
    pub child_path: PathBuf,
    pub data_dir: PathBuf,
    pub agent_log: PathBuf,
    /// The file the restrictive ACL is applied to.
    pub child_log: PathBuf,
    /// Foreground run: mirror logs to stderr, let the child inherit stdio.
    pub console: bool,
    pub max_backoff: Duration,
}

impl Config {
    pub fn with_defaults() -> Result<Self> {
        let data_dir = default_data_dir();
        Ok(Config {
            interval: Duration::from_secs(INTERVAL_SECS),
            child_path: default_child_path()?,
            agent_log: data_dir.join("agent.log"),
            child_log: data_dir.join("child.log"),
            data_dir,
            console: true,
            max_backoff: Duration::from_secs(60),
        })
    }

    pub fn set_data_dir(&mut self, dir: PathBuf) {
        self.agent_log = dir.join("agent.log");
        self.child_log = dir.join("child.log");
        self.data_dir = dir;
    }

    pub fn validate(&self) -> Result<()> {
        if self.interval.is_zero() {
            return Err(Error::config("--interval has to be at least 1"));
        }
        if !self.child_path.is_file() {
            return Err(Error::config(&format!(
                "no child binary at {}, point --child at it",
                self.child_path.display()
            )));
        }
        Ok(())
    }
}

fn default_data_dir() -> PathBuf {
    #[cfg(windows)]
    {
        env::var_os("ProgramData")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
            .join("FlamingoAgent")
    }
    #[cfg(unix)]
    {
        todo!("think of log directory on unix")
    }
}

/// Next to our own executable; a service starts in System32, so not the cwd.
fn default_child_path() -> Result<PathBuf> {
    let exe = env::current_exe()?;
    let dir = exe.parent().unwrap_or_else(|| Path::new("."));
    Ok(dir.join(child_file_name()))
}

pub fn child_file_name() -> &'static str {
    if cfg!(windows) {
        "logger-child.exe"
    } else {
        todo!("unix executable name")
    }
}
