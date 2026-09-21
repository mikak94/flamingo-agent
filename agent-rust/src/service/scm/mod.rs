mod registry;
pub use registry::{install, uninstall};

use std::sync::OnceLock;
use std::sync::mpsc;
use std::time::Duration;

use log::{error, info};
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{
    self, ServiceControlHandlerResult, ServiceStatusHandle,
};
use windows_service::service_dispatcher;

use crate::agent;
use crate::config::{Config, SERVICE_NAME};
use crate::error::{Error, Result};
use crate::service::{ServiceCommand, ServiceContext, ServiceHost, ServiceReport, ServiceReporter};

/// Registration and status reports must agree on this.
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

/// The SCM entry point takes no user data.
static CONFIG: OnceLock<Config> = OnceLock::new();

pub struct ScmHost;

impl ServiceHost for ScmHost {
    fn run(&self, config: Config) -> Result<()> {
        let _ = CONFIG.set(config);
        service_dispatcher::start(SERVICE_NAME, service_entry)
            .map_err(|e| to_error("start the SCM dispatcher", e))
    }
}

/// Called by the SCM on its own thread. The arguments repeat the registered
/// command line, which `main` has already parsed.
extern "system" fn service_entry(_argc: u32, _argv: *mut *mut u16) {
    if let Err(e) = run_service() {
        error!("service died: {e}");
    }
}

fn run_service() -> Result<()> {
    let config = CONFIG
        .get()
        .ok_or_else(|| Error::other("no config set before the SCM called in"))?;

    let (commands, receiver) = mpsc::channel();
    let handle = service_control_handler::register(SERVICE_NAME, move |control| match control {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            let _ = commands.send(ServiceCommand::Stop);
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    })
    .map_err(|e| to_error("register the control handler", e))?;

    let result = agent::run(
        config,
        &ServiceContext::new(receiver, ScmReporter { handle }),
    );

    // Stopped must be reported even on failure, or the SCM waits forever.
    let mut stopped = status(ServiceState::Stopped, ServiceControlAccept::empty());
    if let Err(e) = &result {
        error!("the loop bailed: {e}");
        stopped.exit_code = ServiceExitCode::ServiceSpecific(1);
    }
    if let Err(e) = handle.set_service_status(stopped) {
        error!("couldn't tell the SCM we stopped: {e}");
    }
    result
}

struct ScmReporter {
    handle: ServiceStatusHandle,
}

impl ServiceReporter for ScmReporter {
    fn report(&self, report: ServiceReport) {
        let status = match report {
            ServiceReport::Running => status(
                ServiceState::Running,
                ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
            ),
            ServiceReport::Stopping => {
                let mut s = status(ServiceState::StopPending, ServiceControlAccept::empty());
                s.wait_hint = Duration::from_secs(10);
                s
            }
        };
        match self.handle.set_service_status(status) {
            Ok(()) => info!("told the SCM: {report:?}"),
            Err(e) => error!("couldn't tell the SCM {report:?}: {e}"),
        }
    }
}

fn status(state: ServiceState, accepted: ServiceControlAccept) -> ServiceStatus {
    ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: state,
        controls_accepted: accepted,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    }
}

fn to_error(action: &str, e: windows_service::Error) -> Error {
    Error::Other(format!("couldn't {action}: {e}"))
}
