// Windows native service support (hyper-node run under SCM)
#[cfg(target_os = "windows")]
use crate::{cmd_serve, DEFAULT_PORT, MODE_FILE};

#[cfg(target_os = "windows")]
pub(crate) fn run_windows_service() -> Result<(), String> {
    use windows_service::service_dispatcher;
    // ServiceMain entry must run under the dispatcher (SCM context)
    service_dispatcher::start("hyper-node", ffi_service_main)
        .map_err(|e| format!("dispatcher: {e}"))
}

// define_windows_service! generates the extern "system" ServiceMain wrapper
#[cfg(target_os = "windows")]
windows_service::define_windows_service!(ffi_service_main, service_main);

#[cfg(target_os = "windows")]
fn service_main(_args: Vec<std::ffi::OsString>) {
    if let Err(e) = service_main_inner() {
        eprintln!("hyper-node service error: {e}");
    }
}

#[cfg(target_os = "windows")]
fn service_main_inner() -> Result<(), String> {
    use windows_service::service::ServiceControl;
    use windows_service::service::{
        ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
    };
    use windows_service::service_control_handler::{
        self, ServiceControlHandlerResult, ServiceStatusHandle,
    };

    let event_handler = move |control_event: ServiceControl| -> ServiceControlHandlerResult {
        match control_event {
            // Terminate the process on stop/shutdown so SCM can mark the service stopped
            ServiceControl::Stop | ServiceControl::Shutdown => {
                std::process::exit(0);
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle: ServiceStatusHandle =
        service_control_handler::register("hyper-node", event_handler)
            .map_err(|e| format!("register handler: {e}"))?;
    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: std::time::Duration::default(),
            process_id: None,
        })
        .map_err(|e| format!("set status: {e}"))?;

    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime: {e}"))?;
    let tls = std::fs::read_to_string(MODE_FILE)
        .map(|m| m.trim() != "plain")
        .unwrap_or(true);
    rt.block_on(cmd_serve(DEFAULT_PORT, tls));
    Ok(())
}
