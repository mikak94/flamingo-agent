//! The OS layer. Each module provides:
//!
//! * `process_rss_bytes` - resident set size of this process
//! * `is_elevated` - whether we hold administrative rights
//! * `restrict_to_admins` - lock a file down to admins
//! * `spawn_child` - start the child. It inherits our credentials; there is no elevation step.
//! * `install_shutdown_handler` - route Ctrl+C / SIGTERM to a command channel

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;
