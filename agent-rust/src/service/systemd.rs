//! Not implemented. Windows is the only supported platform.

use std::ffi::OsString;

use crate::config::Config;
use crate::error::Result;
use crate::service::ServiceHost;

pub fn install(_config: &Config, _passthrough: &[OsString]) -> Result<()> {
    todo!("service registration on unix")
}

pub fn uninstall() -> Result<()> {
    todo!("service removal on unix")
}

pub struct SystemdHost;

impl ServiceHost for SystemdHost {
    fn run(&self, _config: Config) -> Result<()> {
        todo!("running under systemd")
    }
}
