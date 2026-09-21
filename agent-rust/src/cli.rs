//! Command-line parsing. I don't want a dependency for this.

use std::ffi::OsString;
use std::path::PathBuf;

use crate::config::{Config, SERVICE_ARG};
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Foreground daemon. The default.
    Run,
    Install,
    Uninstall,
    /// Under the SCM. Set by the registration, not typed by hand.
    Service,
}

pub struct Cli {
    pub command: Command,
    pub config: Config,
    /// The overrides seen, so `--install` can pass them into the registration.
    pub passthrough: Vec<OsString>,
}

pub fn parse<I>(args: I) -> Result<Cli>
where
    I: IntoIterator<Item = OsString>,
{
    let mut config = Config::with_defaults()?;
    let mut command = Command::Run;
    let mut passthrough = Vec::new();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        let text = arg.to_string_lossy().into_owned();
        match text.as_str() {
            "--install" => command = Command::Install,
            "--uninstall" => command = Command::Uninstall,
            SERVICE_ARG => command = Command::Service,
            "--child" => {
                let value = next_value(&mut args, "--child")?;
                config.child_path = PathBuf::from(&value);
                passthrough.push(arg);
                passthrough.push(value);
            }
            "--data-dir" => {
                let value = next_value(&mut args, "--data-dir")?;
                config.set_data_dir(PathBuf::from(&value));
                passthrough.push(arg);
                passthrough.push(value);
            }
            other => {
                return Err(Error::config(&format!(
                    "don't know '{other}'; flags are --install, --uninstall, --child, --data-dir"
                )));
            }
        }
    }

    config.console = command == Command::Run;

    Ok(Cli {
        command,
        config,
        passthrough,
    })
}

fn next_value<I: Iterator<Item = OsString>>(args: &mut I, flag: &str) -> Result<OsString> {
    args.next()
        .ok_or_else(|| Error::config(&format!("{flag} needs a value")))
}
