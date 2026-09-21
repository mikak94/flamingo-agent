use std::process::ExitCode;

use flamingo_agent::cli::{self, Command};
use flamingo_agent::error::Result;
use flamingo_agent::service::{self, ConsoleHost, ServiceHost};
use flamingo_agent::{config::Config, logging};

#[cfg(windows)]
use flamingo_agent::service::ScmHost as PlatformHost;
#[cfg(not(windows))]
use flamingo_agent::service::SystemdHost as PlatformHost;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("flamingo-agent: {e}");
            log::error!("giving up: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = cli::parse(std::env::args_os().skip(1))?;
    match cli.command {
        Command::Install => service::install(&cli.config, &cli.passthrough),
        Command::Uninstall => service::uninstall(),
        Command::Service => start(cli.config, PlatformHost),
        Command::Run => start(cli.config, ConsoleHost),
    }
}

fn start(config: Config, host: impl ServiceHost) -> Result<()> {
    logging::init(&config.agent_log)?;
    config.validate()?;
    host.run(config)
}
