use std::sync::mpsc::channel;

use log::warn;

use crate::agent;
use crate::config::Config;
use crate::error::Result;
use crate::platform;
use crate::service::{NullReporter, ServiceContext, ServiceHost};

/// Console host impl
pub struct ConsoleHost;

impl ServiceHost for ConsoleHost {
    fn run(&self, config: Config) -> Result<()> {
        let (commands, receiver) = channel();
        if let Err(e) = platform::install_shutdown_handler(commands) {
            warn!("couldn't hook Ctrl+C ({e}), you'll have to kill me");
        }
        agent::run(&config, &ServiceContext::new(receiver, NullReporter))
    }
}
