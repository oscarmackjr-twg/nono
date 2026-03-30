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
const EXPECTED_DRIVER_BINARY: &str = "nono-wfp-driver.sys";

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

fn build_protocol_mismatch_response(
    request: &WfpRuntimeActivationRequest,
) -> WfpRuntimeActivationResponse {
    WfpRuntimeActivationResponse {
        protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
        status: "protocol-mismatch".to_string(),
        details: format!(
            "unsupported WFP runtime activation protocol version {}; expected {}",
            request.protocol_version, WFP_RUNTIME_PROTOCOL_VERSION
        ),
    }
}

fn build_invalid_activation_response(
    request: &WfpRuntimeActivationRequest,
) -> WfpRuntimeActivationResponse {
    WfpRuntimeActivationResponse {
        protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
        status: "invalid-request".to_string(),
        details: format!(
            "unsupported WFP runtime activation request kind `{}` for {}",
            request.request_kind, request.runtime_target
        ),
    }
}

fn build_prerequisites_missing_response(details: String) -> WfpRuntimeActivationResponse {
    WfpRuntimeActivationResponse {
        protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
        status: "prerequisites-missing".to_string(),
        details,
    }
}

fn build_enforced_pending_cleanup_response(
    request: &WfpRuntimeActivationRequest,
    details: String,
) -> WfpRuntimeActivationResponse {
    WfpRuntimeActivationResponse {
        protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
        status: "enforced-pending-cleanup".to_string(),
        details: format!(
            "request {} for {} installed target-attached blocked-mode enforcement and requires cleanup after launch: {}",
            request.request_kind, request.runtime_target, details
        ),
    }
}

fn build_cleanup_succeeded_response(
    request: &WfpRuntimeActivationRequest,
    details: String,
) -> WfpRuntimeActivationResponse {
    WfpRuntimeActivationResponse {
        protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
        status: "cleanup-succeeded".to_string(),
        details: format!(
            "request {} for {} removed target-attached blocked-mode enforcement: {}",
            request.request_kind, request.runtime_target, details
        ),
    }
}

fn build_cleanup_failed_response(
    request: &WfpRuntimeActivationRequest,
    details: String,
) -> WfpRuntimeActivationResponse {
    WfpRuntimeActivationResponse {
        protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
        status: "cleanup-failed".to_string(),
        details: format!(
            "request {} for {} could not remove target-attached blocked-mode enforcement: {}",
            request.request_kind, request.runtime_target, details
        ),
    }
}

fn build_filtering_probe_failed_response(
    request: &WfpRuntimeActivationRequest,
    details: String,
) -> WfpRuntimeActivationResponse {
    WfpRuntimeActivationResponse {
        protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
        status: "filtering-probe-failed".to_string(),
        details: format!(
            "request {} for {} could not install the backend-owned filtering probe: {}",
            request.request_kind, request.runtime_target, details
        ),
    }
}

fn current_driver_binary_path() -> Result<std::path::PathBuf, String> {
    let exe = std::env::current_exe()
        .map_err(|err| format!("failed to resolve current service binary path: {}", err))?;
    let parent = exe.parent().ok_or_else(|| {
        format!(
            "failed to resolve parent directory for current service binary {}",
            exe.display()
        )
    })?;
    Ok(parent.join(EXPECTED_DRIVER_BINARY))
}

fn validate_target_request_fields(
    request: &WfpRuntimeActivationRequest,
) -> Result<(std::path::PathBuf, String, String), WfpRuntimeActivationResponse> {
    let target_program = request
        .target_program_path
        .as_ref()
        .map(std::path::PathBuf::from)
        .ok_or_else(|| {
            build_invalid_activation_response(&WfpRuntimeActivationRequest {
                request_kind: request.request_kind.clone(),
                ..request.clone()
            })
        })?;
    let outbound_rule = request.outbound_rule_name.clone().ok_or_else(|| {
        build_invalid_activation_response(&WfpRuntimeActivationRequest {
            request_kind: request.request_kind.clone(),
            ..request.clone()
        })
    })?;
    let inbound_rule = request.inbound_rule_name.clone().ok_or_else(|| {
        build_invalid_activation_response(&WfpRuntimeActivationRequest {
            request_kind: request.request_kind.clone(),
            ..request.clone()
        })
    })?;
    Ok((target_program, outbound_rule, inbound_rule))
}

fn build_target_firewall_add_args(
    outbound_rule: &str,
    inbound_rule: &str,
    target_program: &std::path::Path,
) -> [Vec<String>; 2] {
    [
        vec![
            "advfirewall".to_string(),
            "firewall".to_string(),
            "add".to_string(),
            "rule".to_string(),
            format!("name={outbound_rule}"),
            "dir=out".to_string(),
            "action=block".to_string(),
            format!("program={}", target_program.display()),
            "enable=yes".to_string(),
            "profile=any".to_string(),
        ],
        vec![
            "advfirewall".to_string(),
            "firewall".to_string(),
            "add".to_string(),
            "rule".to_string(),
            format!("name={inbound_rule}"),
            "dir=in".to_string(),
            "action=block".to_string(),
            format!("program={}", target_program.display()),
            "enable=yes".to_string(),
            "profile=any".to_string(),
        ],
    ]
}

fn build_target_firewall_delete_args(outbound_rule: &str, inbound_rule: &str) -> [Vec<String>; 2] {
    [
        build_firewall_delete_args(inbound_rule),
        build_firewall_delete_args(outbound_rule),
    ]
}

fn build_firewall_delete_args(rule_name: &str) -> Vec<String> {
    vec![
        "advfirewall".to_string(),
        "firewall".to_string(),
        "delete".to_string(),
        "rule".to_string(),
        format!("name={rule_name}"),
    ]
}

fn run_netsh_command(args: &[String]) -> Result<String, String> {
    let output = std::process::Command::new("netsh")
        .args(args)
        .output()
        .map_err(|err| format!("failed to execute netsh {:?}: {}", args, err))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        Err(format!(
            "netsh {:?} failed with status {:?}, stdout: {:?}, stderr: {:?}",
            args,
            output.status.code(),
            stdout,
            stderr
        ))
    }
}

fn activate_target_attached_blocked_mode_with_runner<R>(
    request: &WfpRuntimeActivationRequest,
    run_command: R,
) -> WfpRuntimeActivationResponse
where
    R: Fn(&[String]) -> Result<String, String>,
{
    let (target_program, outbound_rule, inbound_rule) =
        match validate_target_request_fields(request) {
            Ok(fields) => fields,
            Err(response) => return response,
        };
    if !target_program.exists() {
        return build_prerequisites_missing_response(format!(
            "target program for blocked-mode enforcement does not exist: {}",
            target_program.display()
        ));
    }
    let [out_add, in_add] =
        build_target_firewall_add_args(&outbound_rule, &inbound_rule, &target_program);
    match run_command(&out_add) {
        Ok(out_add_result) => match run_command(&in_add) {
            Ok(in_add_result) => build_enforced_pending_cleanup_response(
                request,
                format!(
                    "installed firewall rules {} and {} for target {} (outbound: {:?}, inbound: {:?})",
                    outbound_rule,
                    inbound_rule,
                    target_program.display(),
                    out_add_result,
                    in_add_result
                ),
            ),
            Err(err) => {
                let cleanup_args = build_target_firewall_delete_args(&outbound_rule, &inbound_rule);
                let _ = run_command(&cleanup_args[1]);
                build_filtering_probe_failed_response(
                    request,
                    format!(
                        "installed outbound rule {} but failed to install inbound rule {}: {}",
                        outbound_rule, inbound_rule, err
                    ),
                )
            }
        },
        Err(err) => build_filtering_probe_failed_response(
            request,
            format!(
                "failed to install outbound target-attached rule {} for {}: {}",
                outbound_rule,
                target_program.display(),
                err
            ),
        ),
    }
}

fn deactivate_target_attached_blocked_mode_with_runner<R>(
    request: &WfpRuntimeActivationRequest,
    run_command: R,
) -> WfpRuntimeActivationResponse
where
    R: Fn(&[String]) -> Result<String, String>,
{
    let (_target_program, outbound_rule, inbound_rule) =
        match validate_target_request_fields(request) {
            Ok(fields) => fields,
            Err(response) => return response,
        };
    let [in_delete, out_delete] = build_target_firewall_delete_args(&outbound_rule, &inbound_rule);
    let in_result = run_command(&in_delete);
    let out_result = run_command(&out_delete);
    match (in_result, out_result) {
        (Ok(in_details), Ok(out_details)) => build_cleanup_succeeded_response(
            request,
            format!(
                "removed firewall rules {} and {} (inbound: {:?}, outbound: {:?})",
                inbound_rule, outbound_rule, in_details, out_details
            ),
        ),
        (in_err, out_err) => build_cleanup_failed_response(
            request,
            format!(
                "cleanup results for {} / {} were inbound={:?}, outbound={:?}",
                inbound_rule, outbound_rule, in_err, out_err
            ),
        ),
    }
}

fn activate_blocked_mode(request: &WfpRuntimeActivationRequest) -> WfpRuntimeActivationResponse {
    if request.protocol_version != WFP_RUNTIME_PROTOCOL_VERSION {
        return build_protocol_mismatch_response(request);
    }
    if request.network_mode != "blocked" {
        return build_invalid_activation_response(request);
    }

    let driver_binary_path = match current_driver_binary_path() {
        Ok(path) => path,
        Err(err) => return build_prerequisites_missing_response(err),
    };
    if !driver_binary_path.exists() {
        return build_prerequisites_missing_response(format!(
            "expected driver artifact is missing beside the service host: {}",
            driver_binary_path.display()
        ));
    }
    activate_target_attached_blocked_mode_with_runner(request, run_netsh_command)
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
    let response = match request.request_kind.as_str() {
        "activate_blocked_mode" => {
            let prereq = activate_blocked_mode(&request);
            match prereq.status.as_str() {
                "prerequisites-missing" | "protocol-mismatch" | "invalid-request" => prereq,
                _ => activate_target_attached_blocked_mode_with_runner(&request, run_netsh_command),
            }
        }
        "deactivate_blocked_mode" => {
            deactivate_target_attached_blocked_mode_with_runner(&request, run_netsh_command)
        }
        _ => build_invalid_activation_response(&request),
    };
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
        "nono-wfp-service: request {} resolved to status {}; \
         service-owned activation still fails closed until a real WFP primitive is available",
        request.request_kind, response.status
    );
    if response.status == "invalid-request" || response.status == "protocol-mismatch" {
        ExitCode::from(2)
    } else if response.status == "prerequisites-missing" {
        ExitCode::from(3)
    } else {
        ExitCode::from(4)
    }
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
            request_kind: "activate_blocked_mode".to_string(),
            network_mode: "blocked".to_string(),
            preferred_backend: "windows-filtering-platform".to_string(),
            active_backend: "none".to_string(),
            runtime_target: "blocked Windows network access".to_string(),
            target_program_path: Some(r"C:\tools\target.exe".to_string()),
            outbound_rule_name: Some("nono-test-out".to_string()),
            inbound_rule_name: Some("nono-test-in".to_string()),
        };
        let response = build_enforced_pending_cleanup_response(&request, "placeholder".to_string());
        assert_eq!(response.status, "enforced-pending-cleanup");
        assert!(response.details.contains("blocked Windows network access"));
    }

    #[test]
    fn invalid_request_kind_returns_invalid_request_response() {
        let request = WfpRuntimeActivationRequest {
            protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
            request_kind: "activate_proxy_mode".to_string(),
            network_mode: "proxy-only".to_string(),
            preferred_backend: "windows-filtering-platform".to_string(),
            active_backend: "none".to_string(),
            runtime_target: "proxy Windows network access".to_string(),
            target_program_path: None,
            outbound_rule_name: None,
            inbound_rule_name: None,
        };
        let response = build_invalid_activation_response(&request);
        assert_eq!(response.status, "invalid-request");
        assert!(response.details.contains("activate_proxy_mode"));
    }

    #[test]
    fn protocol_mismatch_returns_protocol_mismatch_response() {
        let request = WfpRuntimeActivationRequest {
            protocol_version: WFP_RUNTIME_PROTOCOL_VERSION + 1,
            request_kind: "activate_blocked_mode".to_string(),
            network_mode: "blocked".to_string(),
            preferred_backend: "windows-filtering-platform".to_string(),
            active_backend: "none".to_string(),
            runtime_target: "blocked Windows network access".to_string(),
            target_program_path: Some(r"C:\tools\target.exe".to_string()),
            outbound_rule_name: Some("nono-test-out".to_string()),
            inbound_rule_name: Some("nono-test-in".to_string()),
        };
        let response = build_protocol_mismatch_response(&request);
        assert_eq!(response.status, "protocol-mismatch");
        assert!(response.details.contains("expected 1"));
    }

    #[test]
    fn blocked_mode_probe_reports_enforced_pending_cleanup() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("target.exe");
        std::fs::write(&target, b"stub").expect("write target");
        let request = WfpRuntimeActivationRequest {
            protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
            request_kind: "activate_blocked_mode".to_string(),
            network_mode: "blocked".to_string(),
            preferred_backend: "windows-filtering-platform".to_string(),
            active_backend: "none".to_string(),
            runtime_target: "blocked Windows network access".to_string(),
            target_program_path: Some(target.display().to_string()),
            outbound_rule_name: Some("nono-test-out".to_string()),
            inbound_rule_name: Some("nono-test-in".to_string()),
        };
        let response = activate_target_attached_blocked_mode_with_runner(&request, |_args| {
            Ok("ok".to_string())
        });
        assert_eq!(response.status, "enforced-pending-cleanup");
        assert!(response.details.contains("installed firewall rules"));
    }

    #[test]
    fn blocked_mode_probe_reports_filtering_probe_failed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("target.exe");
        std::fs::write(&target, b"stub").expect("write target");
        let request = WfpRuntimeActivationRequest {
            protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
            request_kind: "activate_blocked_mode".to_string(),
            network_mode: "blocked".to_string(),
            preferred_backend: "windows-filtering-platform".to_string(),
            active_backend: "none".to_string(),
            runtime_target: "blocked Windows network access".to_string(),
            target_program_path: Some(target.display().to_string()),
            outbound_rule_name: Some("nono-test-out".to_string()),
            inbound_rule_name: Some("nono-test-in".to_string()),
        };
        let response = activate_target_attached_blocked_mode_with_runner(&request, |_args| {
            Err("access denied".to_string())
        });
        assert_eq!(response.status, "filtering-probe-failed");
        assert!(response.details.contains("access denied"));
    }
}
