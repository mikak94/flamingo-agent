use std::ffi::OsString;
use std::time::Duration;

use windows_service::service::{
    Service, ServiceAccess, ServiceAction, ServiceActionType, ServiceErrorControl,
    ServiceFailureActions, ServiceFailureResetPeriod, ServiceInfo, ServiceStartType, ServiceState,
};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

use super::{SERVICE_TYPE, to_error};
use crate::config::{Config, SERVICE_ARG, SERVICE_DESCRIPTION, SERVICE_DISPLAY_NAME, SERVICE_NAME};
use crate::error::Result;

pub fn install(config: &Config, passthrough: &[OsString]) -> Result<()> {
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )
    .map_err(|e| to_error("reach the SCM", e))?;

    let mut launch_arguments = vec![OsString::from(SERVICE_ARG)];
    launch_arguments.extend(passthrough.iter().cloned());

    let info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: SERVICE_TYPE,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: std::env::current_exe()?,
        launch_arguments,
        dependencies: vec![],
        account_name: None, // LocalSystem
        account_password: None,
    };

    let service = manager
        .create_service(
            &info,
            ServiceAccess::CHANGE_CONFIG | ServiceAccess::START | ServiceAccess::QUERY_STATUS,
        )
        .map_err(|e| to_error("create the service", e))?;

    if let Err(e) = service.set_description(SERVICE_DESCRIPTION) {
        eprintln!("couldn't set the description, never mind: {e}");
    }
    if let Err(e) = set_restart_policy(&service) {
        eprintln!("couldn't set the restart policy, never mind: {e}");
    }

    println!("installed '{SERVICE_NAME}' ({SERVICE_DISPLAY_NAME})");
    println!("  binary:   {}", info.executable_path.display());
    println!("  start:    auto, as LocalSystem");
    println!("  interval: {}s", config.interval.as_secs());
    println!("  child:    {}", config.child_path.display());
    println!("  logs:     {}", config.data_dir.display());
    Ok(())
}

fn restart(secs: u64) -> ServiceAction {
    ServiceAction {
        action_type: ServiceActionType::Restart,
        delay: Duration::from_secs(secs),
    }
}

/// Restart after 5s, 30s, then 60s, including on a non-zero exit.
fn set_restart_policy(service: &Service) -> windows_service::Result<()> {
    service.update_failure_actions(ServiceFailureActions {
        reset_period: ServiceFailureResetPeriod::After(Duration::from_secs(86_400)),
        reboot_msg: None,
        command: None,
        actions: Some(vec![restart(5), restart(30), restart(60)]),
    })?;
    service.set_failure_actions_on_non_crash_failures(true)
}

pub fn uninstall() -> Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|e| to_error("reach the SCM", e))?;

    let service = manager
        .open_service(
            SERVICE_NAME,
            ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS,
        )
        .map_err(|e| to_error("find the service", e))?;

    let status = service
        .query_status()
        .map_err(|e| to_error("check the service status", e))?;
    if status.current_state != ServiceState::Stopped {
        println!("stopping '{SERVICE_NAME}'...");
        if let Err(e) = service.stop() {
            eprintln!("stop failed, carrying on: {e}");
        }
        wait_for_stop(&service);
    }

    service
        .delete()
        .map_err(|e| to_error("remove the service", e))?;
    println!("removed '{SERVICE_NAME}'");
    Ok(())
}

fn wait_for_stop(service: &Service) {
    for _ in 0..30 {
        match service.query_status() {
            Ok(s) if s.current_state == ServiceState::Stopped => return,
            Ok(_) => std::thread::sleep(Duration::from_millis(500)),
            Err(_) => return,
        }
    }
    eprintln!("still not stopped after 15s, removing it anyway");
}
