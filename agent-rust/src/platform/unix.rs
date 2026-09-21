//! Not implemented. Windows is the only supported platform for now

use std::ffi::OsStr;
use std::path::Path;
use std::process::Child;
use std::sync::mpsc::Sender;

use crate::error::Result;
use crate::service::ServiceCommand;

pub fn process_rss_bytes() -> Result<u64> {
    todo!("process RSS on unix")
}

pub fn is_elevated() -> Result<bool> {
    todo!("elevation check on unix")
}

pub fn restrict_to_admins(_path: &Path) -> Result<()> {
    todo!("file ACL on unix")
}

pub fn spawn_child(_exe: &Path, _args: &[&OsStr], _console: bool) -> Result<Child> {
    todo!("child spawn on unix")
}

pub fn install_shutdown_handler(_commands: Sender<ServiceCommand>) -> Result<()> {
    todo!("SIGINT/SIGTERM handling on unix")
}
