// Windows native service support for hyper-relay.
#[cfg(target_os = "windows")]
pub fn run_windows_service() -> Result<(), String> {
    use windows_service::service_dispatcher;
    // The service name must match the SCM registration (install-windows.bat).
    service_dispatcher::start("hyper-relay", ffi_service_main)
        .map_err(|e| format!("dispatcher: {e}"))
}

#[cfg(target_os = "windows")]
windows_service::define_windows_service!(ffi_service_main, service_main);

#[cfg(target_os = "windows")]
fn service_main(_args: Vec<std::ffi::OsString>) {
    if let Err(e) = service_main_inner() {
        eprintln!("hyper-relay service error: {e}");
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
            ServiceControl::Stop | ServiceControl::Shutdown => {
                std::process::exit(0);
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle: ServiceStatusHandle =
        service_control_handler::register("hyper-relay", event_handler)
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
    let addr: std::net::SocketAddr = format!("0.0.0.0:{}", crate::DEFAULT_PORT)
        .parse()
        .expect("invalid addr");
    // Windows service always serves WSS: the cert lives at the standard
    // location (created by `hyper-node key setup` / install script). If the
    // cert is missing the service fails to start — plaintext WS is not
    // supported (TLS is mandatory for the relay).
    let cert = "C:\\ProgramData\\hyper-node\\tls\\relay-cert.pem";
    let key = "C:\\ProgramData\\hyper-node\\tls\\relay-key.pem";
    if !std::path::Path::new(cert).exists() || !std::path::Path::new(key).exists() {
        return Err("relay TLS certificate missing — run `hyper-node key setup` first".into());
    }
    // Install the rustls CryptoProvider before building any TLS config.
    let _ = rustls::crypto::ring::default_provider().install_default();
    rt.block_on(async {
        if let Err(e) = crate::server::run_with_tls(addr, Some(cert), Some(key)).await {
            eprintln!("hyper-relay stopped: {e}");
        }
    });
    Ok(())
}
