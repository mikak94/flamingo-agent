//! THE loop: collect, log, launch, wait for a command or the interval.

use log::{error, info};

use crate::child::ChildLauncher;
use crate::config::Config;
use crate::error::Result;
use crate::metrics::Metrics;
use crate::platform;
use crate::service::{ServiceCommand, ServiceContext, ServiceReport, ServiceReporter};

pub fn run<R: ServiceReporter>(config: &Config, context: &ServiceContext<R>) -> Result<()> {
    info!(
        "starting up: pid {}, every {}s, elevated: {}",
        std::process::id(),
        config.interval.as_secs(),
        match platform::is_elevated() {
            Ok(v) => v.to_string(),
            Err(e) => format!("no idea ({e})"),
        }
    );
    info!("child log: {}", config.child_log.display());
    info!("child:     {}", config.child_path.display());
    context.report(ServiceReport::Running);

    let mut launcher = ChildLauncher::new(config.clone());
    let mut failures: u32 = 0;

    loop {
        let metrics = match Metrics::collect() {
            Ok(m) => {
                if failures > 0 {
                    info!("metrics are back after {failures} failed tries");
                    failures = 0;
                }
                info!("metrics {}", m.to_display());
                Some(m)
            }
            Err(e) => {
                failures += 1;
                error!("couldn't collect metrics (attempt {failures}): {e}");
                None
            }
        };

        launcher.tick(metrics.as_ref());

        if let Some(ServiceCommand::Stop) = context.wait(config.interval) {
            break;
        }
    }

    info!("got a stop request, shutting down");
    context.report(ServiceReport::Stopping);
    launcher.shutdown();
    info!("stopped");
    Ok(())
}
