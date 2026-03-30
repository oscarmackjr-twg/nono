//! Windows WFP backend service placeholder.
//!
//! This binary is the first repo-owned artifact for the future Windows WFP
//! backend. It establishes the expected Windows service contract without
//! claiming working service-host or enforcement behavior yet.

#[path = "../windows_wfp_contract.rs"]
mod windows_wfp_contract;

use std::process::ExitCode;
use windows_wfp_contract::{
    WfpRuntimeActivationRequest, WfpRuntimeActivationResponse, WFP_RUNTIME_PROTOCOL_VERSION,
};

const SERVICE_NAME: &str = "nono-wfp-service";
const SERVICE_MODE_ARG: &str = "--service-mode";
const PROBE_RUNTIME_ACTIVATION_ARG: &str = "--probe-runtime-activation";

fn print_help() {
    println!("nono-wfp-service {}", env!("CARGO_PKG_VERSION"));
    println!("Windows WFP backend service placeholder");
    println!();
    println!("This binary defines the current Windows service contract for the");
    println!("future WFP backend. Service registration and service-host runtime");
    println!("still fail closed until the real backend is implemented.");
    println!();
    println!("Service contract:");
    println!("  service name: {SERVICE_NAME}");
    println!("  startup args: {SERVICE_MODE_ARG}");
    println!();
    println!("Supported options:");
    println!("  --help                 Show this message");
    println!("  --version              Show version information");
    println!("  --print-service-contract");
    println!("                         Print the expected Windows service contract");
    println!(
        "  {PROBE_RUNTIME_ACTIVATION_ARG:<22}Probe the placeholder runtime activation contract"
    );
    println!("  {SERVICE_MODE_ARG:<22}Run the placeholder service entrypoint");
}

fn print_service_contract() {
    println!("service_name={SERVICE_NAME}");
    println!("startup_args={SERVICE_MODE_ARG}");
}

fn run_service_mode() -> ExitCode {
    eprintln!(
        "nono-wfp-service: service runtime is not implemented yet; \
         registration may target '{SERVICE_NAME} {SERVICE_MODE_ARG}', but startup still fails closed"
    );
    ExitCode::from(3)
}

fn build_not_implemented_activation_response(
    request: &WfpRuntimeActivationRequest,
) -> WfpRuntimeActivationResponse {
    WfpRuntimeActivationResponse {
        protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
        status: "not-implemented".to_string(),
        details: format!(
            "runtime activation for {} is not implemented yet; preferred backend: {}, active backend: {}",
            request.runtime_target, request.preferred_backend, request.active_backend
        ),
    }
}

fn probe_runtime_activation() -> ExitCode {
    let stdin = std::io::read_to_string(std::io::stdin());
    let Ok(stdin) = stdin else {
        eprintln!("nono-wfp-service: failed to read runtime activation request from stdin");
        return ExitCode::from(2);
    };
    let request: WfpRuntimeActivationRequest = match serde_json::from_str(&stdin) {
        Ok(request) => request,
        Err(err) => {
            eprintln!(
                "nono-wfp-service: invalid runtime activation request payload: {}",
                err
            );
            return ExitCode::from(2);
        }
    };
    let response = build_not_implemented_activation_response(&request);
    match serde_json::to_string(&response) {
        Ok(json) => println!("{json}"),
        Err(err) => {
            eprintln!(
                "nono-wfp-service: failed to serialize runtime activation response: {}",
                err
            );
            return ExitCode::from(2);
        }
    }
    eprintln!(
        "nono-wfp-service: runtime activation handshake is not implemented yet; \
         service and driver may be registered, but WFP enforcement still fails closed"
    );
    ExitCode::from(4)
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None => {
            eprintln!(
                "nono-wfp-service: missing required mode; use \
                 --print-service-contract, {PROBE_RUNTIME_ACTIVATION_ARG}, or {SERVICE_MODE_ARG}"
            );
            ExitCode::from(2)
        }
        Some("--help") | Some("-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("--version") | Some("-V") => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--print-service-contract") => {
            print_service_contract();
            ExitCode::SUCCESS
        }
        Some(PROBE_RUNTIME_ACTIVATION_ARG) => probe_runtime_activation(),
        Some(SERVICE_MODE_ARG) => run_service_mode(),
        Some(other) => {
            eprintln!("nono-wfp-service: unsupported argument '{other}'");
            eprintln!(
                "Run with --help to inspect the current service contract \
                 or --print-service-contract for machine-readable output."
            );
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_contract_output_is_stable() {
        let text = format!("service_name={SERVICE_NAME}\nstartup_args={SERVICE_MODE_ARG}\n");
        assert!(text.contains("service_name=nono-wfp-service"));
        assert!(text.contains("startup_args=--service-mode"));
    }

    #[test]
    fn service_mode_fails_closed() {
        assert_eq!(run_service_mode(), ExitCode::from(3));
    }

    #[test]
    fn runtime_activation_probe_fails_closed() {
        let request = WfpRuntimeActivationRequest {
            protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
            network_mode: "blocked".to_string(),
            preferred_backend: "windows-filtering-platform".to_string(),
            active_backend: "none".to_string(),
            runtime_target: "blocked Windows network access".to_string(),
        };
        let response = build_not_implemented_activation_response(&request);
        assert_eq!(response.status, "not-implemented");
        assert!(response.details.contains("blocked Windows network access"));
    }
}
