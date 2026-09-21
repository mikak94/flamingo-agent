use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use crate::config::Config;
use crate::error::Result;

#[cfg(windows)]
mod scm;
#[cfg(windows)]
pub use scm::{ScmHost, install, uninstall};

#[cfg(not(windows))]
mod systemd;
#[cfg(not(windows))]
pub use systemd::{SystemdHost, install, uninstall};

mod console;
pub use console::ConsoleHost;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceCommand {
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceReport {
    Running,
    Stopping,
}

pub trait ServiceReporter {
    fn report(&self, report: ServiceReport);
}

pub struct NullReporter;

impl ServiceReporter for NullReporter {
    fn report(&self, _report: ServiceReport) {}
}

pub struct ServiceContext<R> {
    commands: Receiver<ServiceCommand>,
    reporter: R,
}

impl<R: ServiceReporter> ServiceContext<R> {
    pub fn new(commands: Receiver<ServiceCommand>, reporter: R) -> Self {
        ServiceContext { commands, reporter }
    }

    pub fn report(&self, report: ServiceReport) {
        self.reporter.report(report);
    }

    pub fn wait(&self, timeout: Duration) -> Option<ServiceCommand> {
        match self.commands.recv_timeout(timeout) {
            Ok(command) => Some(command),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => Some(ServiceCommand::Stop),
        }
    }
}

pub trait ServiceHost {
    fn run(&self, config: Config) -> Result<()>;
}
