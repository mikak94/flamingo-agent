//! tiny abstraction to manage logger child process

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::process::Child;
use std::time::{Duration, Instant};

use log::{debug, error, info, warn};

use crate::acl;
use crate::config::Config;
use crate::metrics::Metrics;
use crate::platform;

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);

pub struct ChildLauncher {
    config: Config,
    /// Last tick's child, reaped at the start of the next.
    pending: Option<Child>,
    next_attempt: Instant,
    backoff: Duration,
    consecutive_failures: u32,
    /// Outcome of the last ACL attempt; logged on change only.
    protection_ok: Option<bool>,
}

impl ChildLauncher {
    pub fn new(config: Config) -> Self {
        ChildLauncher {
            config,
            pending: None,
            next_attempt: Instant::now(),
            backoff: INITIAL_BACKOFF,
            consecutive_failures: 0,
            protection_ok: None,
        }
    }

    pub fn tick(&mut self, metrics: Option<&Metrics>) {
        self.reap_previous();
        if Instant::now() < self.next_attempt {
            return;
        }
        let Some(metrics) = metrics else {
            debug!("no metrics this tick, not launching the child");
            return;
        };
        self.launch(metrics);
    }

    fn reap_previous(&mut self) {
        let Some(child) = self.pending.as_mut() else {
            return;
        };
        match child.try_wait() {
            Ok(Some(status)) => {
                // Non-zero is how the child says it could not write the log.
                let stderr = read_stderr(child);
                if !status.success() {
                    warn!("child failed with {status}: {stderr}");
                } else if !stderr.is_empty() {
                    warn!("child said: {stderr}");
                }
            }
            Ok(None) => {
                // A child still alive after a full interval is stuck.
                warn!(
                    "child pid {} is still around after a whole interval, killing it",
                    child.id()
                );
                let _ = child.kill();
                let _ = child.wait();
            }
            Err(e) => error!("can't check on the child ({e}), forgetting about it"),
        }
        self.pending = None;
    }

    fn launch(&mut self, metrics: &Metrics) {
        self.protect_log();

        let args = [
            OsString::from(metrics.to_arg()),
            self.config.child_log.clone().into_os_string(),
        ];
        let arg_refs: Vec<&OsStr> = args.iter().map(OsString::as_os_str).collect();

        match platform::spawn_child(&self.config.child_path, &arg_refs, self.config.console) {
            Ok(child) => {
                debug!("started child pid {}", child.id());
                self.pending = Some(child);
                self.backoff = INITIAL_BACKOFF;
                self.consecutive_failures = 0;
            }
            Err(e) => {
                self.consecutive_failures += 1;
                error!(
                    "couldn't start {} (try {}): {e}, next attempt in {:?}",
                    self.config.child_path.display(),
                    self.consecutive_failures,
                    self.backoff
                );
                self.next_attempt = Instant::now() + self.backoff;
                self.backoff = (self.backoff * 2).min(self.config.max_backoff);
            }
        }
    }

    /// Re-applied before every launch so a recreated log is re-protected.
    /// Failure is logged but does not block the launch.
    fn protect_log(&mut self) {
        match acl::secure_log_file(&self.config.child_log) {
            Ok(()) => {
                if self.protection_ok != Some(true) {
                    info!("locked down {} to admins", self.config.child_log.display());
                    self.protection_ok = Some(true);
                }
            }
            Err(e) => {
                if self.protection_ok != Some(false) {
                    error!(
                        "couldn't lock down {}: {e}, starting the child anyway",
                        self.config.child_log.display()
                    );
                    self.protection_ok = Some(false);
                }
            }
        }
    }

    pub fn shutdown(&mut self) {
        if let Some(child) = self.pending.as_mut() {
            if !matches!(child.try_wait(), Ok(Some(_))) {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        self.pending = None;
    }
}

fn read_stderr(child: &mut Child) -> String {
    let mut text = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_string(&mut text);
    }
    text.trim().to_string()
}

impl Drop for ChildLauncher {
    fn drop(&mut self) {
        self.shutdown();
    }
}
