use std::fs::{self, OpenOptions};
use std::path::Path;

use crate::error::Result;
use crate::platform;

pub fn secure_log_file(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    OpenOptions::new().create(true).append(true).open(path)?;
    platform::restrict_to_admins(path)?;
    Ok(())
}
