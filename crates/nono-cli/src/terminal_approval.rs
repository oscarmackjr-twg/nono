//! Terminal-based interactive approval backend for supervisor IPC
//!
//! Prompts the user at the terminal when the sandboxed child requests
//! additional filesystem access. This is the default approval backend
//! for `nono run`.

use nono::supervisor::policy;
use nono::supervisor::{HandleKind, HandleTarget, SocketProtocol, SocketRole};
use nono::{AccessMode, ApprovalBackend, ApprovalDecision, CapabilityRequest, NonoError, Result};
use std::io::{BufRead, IsTerminal, Write};

/// Interactive terminal approval backend.
///
/// Prints capability expansion requests to stderr and reads the user's
/// response from a dedicated terminal input device (not stdin, which belongs
/// to the sandboxed child):
///
/// - On Unix, opens `/dev/tty`.
/// - On Windows, opens `\\.\CONIN$` (the process's attached console input
///   buffer). When no console is attached (for example, a detached or
///   service-hosted supervisor), the backend fail-secure denies with a
///   reason that mentions "console".
///
/// Returns `Denied` automatically if no interactive terminal is available.
pub struct TerminalApproval;

impl ApprovalBackend for TerminalApproval {
    fn request_capability(&self, request: &CapabilityRequest) -> Result<ApprovalDecision> {
        let stderr = std::io::stderr();
        if !stderr.is_terminal() {
            return Ok(ApprovalDecision::Denied {
                reason: "No terminal available for interactive approval".to_string(),
            });
        }

        // Phase 18.1 G-02 fix: route AIPC (non-File) requests through the
        // D-04-locked per-kind templates; File-kind legacy shape
        // (target == None) preserves the Phase 11 multi-line block
        // byte-identical. The helper is side-effect-free so tests can
        // capture the exact string without requiring a TTY.
        let prompt = build_prompt_text(request);
        eprint!("{prompt}");
        if !prompt.ends_with(' ') {
            // AIPC per-kind templates end with `[y/N]` (no trailing space).
            // Legacy File path ends with `[y/N] ` (trailing space, already
            // in the template). For the AIPC case, emit a newline so the
            // approver's typed response appears on a new line.
            eprintln!();
        }
        let _ = std::io::stderr().flush();

        // Read from a dedicated terminal device, not stdin (which belongs
        // to the sandboxed child). On Windows, `\\.\CONIN$` is the
        // equivalent of `/dev/tty`: it opens the process's attached console
        // input buffer regardless of stdin redirection. If no console is
        // attached the open fails, which we translate to a fail-secure
        // denial (T-11-10).
        #[cfg(unix)]
        let tty = std::fs::File::open("/dev/tty").map_err(|e| {
            NonoError::SandboxInit(format!("Failed to open /dev/tty for approval prompt: {e}"))
        })?;

        #[cfg(target_os = "windows")]
        let tty = match std::fs::File::open(r"\\.\CONIN$") {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!(
                    "TerminalApproval: no console available for interactive approval: {e}"
                );
                return Ok(ApprovalDecision::Denied {
                    reason: "No console available for interactive approval".to_string(),
                });
            }
        };
        let mut reader = std::io::BufReader::new(tty);
        let mut input = String::new();
        reader.read_line(&mut input).map_err(|e| {
            NonoError::SandboxInit(format!("Failed to read approval response: {e}"))
        })?;

        let input = input.trim().to_lowercase();
        if input == "y" || input == "yes" {
            eprintln!("[nono] Access granted.");
            Ok(ApprovalDecision::Granted)
        } else {
            eprintln!("[nono] Access denied.");
            Ok(ApprovalDecision::Denied {
                reason: "User denied the request".to_string(),
            })
        }
    }

    fn backend_name(&self) -> &str {
        "terminal"
    }
}

/// Strip control characters and ANSI escape sequences from untrusted input
/// before displaying on the terminal.
///
/// Handles all standard escape sequence types:
/// - CSI (ESC [): cursor movement, SGR colors, erase commands
/// - OSC (ESC ]): title changes, hyperlinks — terminated by BEL or ST
/// - DCS (ESC P), APC (ESC _), PM (ESC ^), SOS (ESC X): all consume through ST
///
/// All control characters (0x00-0x1F, 0x7F) are replaced with space.
fn sanitize_for_terminal(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if let Some(&next) = chars.peek() {
                if next == '[' {
                    // CSI sequence: consume until final byte 0x40-0x7E
                    chars.next();
                    for seq_c in chars.by_ref() {
                        if ('\x40'..='\x7e').contains(&seq_c) {
                            break;
                        }
                    }
                } else if matches!(next, ']' | 'P' | '_' | '^' | 'X') {
                    // String sequences (OSC, DCS, APC, PM, SOS):
                    // consume until ST (ESC \) or BEL (0x07)
                    chars.next();
                    let mut prev = '\0';
                    for seq_c in chars.by_ref() {
                        if seq_c == '\x07' || (prev == '\x1b' && seq_c == '\\') {
                            break;
                        }
                        prev = seq_c;
                    }
                }
                // Other ESC sequences (e.g. ESC c, ESC 7): drop the ESC
            }
            continue;
        }

        if c.is_control() {
            result.push(' ');
        } else {
            result.push(c);
        }
    }

    result
}

/// Format an access mode for human-readable display.
fn format_access_mode(access: &AccessMode) -> &'static str {
    match access {
        AccessMode::Read => "read-only",
        AccessMode::Write => "write-only",
        AccessMode::ReadWrite => "read+write",
    }
}

/// Render the per-bit Event mask as a human-readable label per D-04.
///
/// Consumed transitively via `format_capability_prompt` from
/// `build_prompt_text`, which `TerminalApproval::request_capability` calls
/// for every AIPC (non-File) approval prompt since Phase 18.1 Plan 18.1-01
/// (G-02 fix).
fn format_event_access(mask: u32) -> &'static str {
    let wait = mask & policy::SYNCHRONIZE != 0;
    let signal = mask & policy::EVENT_MODIFY_STATE != 0;
    match (wait, signal) {
        (true, true) => "wait+signal",
        (true, false) => "wait",
        (false, true) => "signal",
        (false, false) => "(none)",
    }
}

/// Render the per-bit Mutex mask as a human-readable label per D-04.
///
/// Consumed transitively via `format_capability_prompt` from
/// `build_prompt_text`, which `TerminalApproval::request_capability` calls
/// for every AIPC (non-File) approval prompt since Phase 18.1 Plan 18.1-01
/// (G-02 fix).
fn format_mutex_access(mask: u32) -> &'static str {
    let wait = mask & policy::SYNCHRONIZE != 0;
    let release = mask & policy::MUTEX_MODIFY_STATE != 0;
    match (wait, release) {
        (true, true) => "wait+release",
        (true, false) => "wait",
        (false, true) => "release",
        (false, false) => "(none)",
    }
}

/// Render the per-bit Pipe access_mask as a human-readable direction label
/// per D-04 (Phase 18-02). The mask is `GENERIC_READ` and/or `GENERIC_WRITE`;
/// any other bits surface as `(none)` (the dispatcher's pipe helper rejects
/// invalid combinations earlier, so this branch is only reached on valid
/// requests).
fn format_pipe_direction(mask: u32) -> &'static str {
    let read = mask & policy::GENERIC_READ != 0;
    let write = mask & policy::GENERIC_WRITE != 0;
    match (read, write) {
        (true, true) => "read+write",
        (true, false) => "read",
        (false, true) => "write",
        (false, false) => "(none)",
    }
}

/// Render the per-bit Job Object access_mask as a human-readable label per
/// D-04 (Phase 18-03). Recognized bits: `JOB_OBJECT_QUERY`,
/// `JOB_OBJECT_SET_ATTRIBUTES`, `JOB_OBJECT_TERMINATE`,
/// `JOB_OBJECT_ASSIGN_PROCESS`. Unrecognized combinations surface as `(none)`.
/// Returns an owned `String` because the bit combinations multiply.
fn format_job_object_access(mask: u32) -> String {
    let mut parts: Vec<&'static str> = Vec::new();
    if mask & policy::JOB_OBJECT_QUERY != 0 {
        parts.push("query");
    }
    if mask & policy::JOB_OBJECT_SET_ATTRIBUTES != 0 {
        parts.push("set_attributes");
    }
    if mask & policy::JOB_OBJECT_TERMINATE != 0 {
        parts.push("terminate");
    }
    if mask & policy::JOB_OBJECT_ASSIGN_PROCESS != 0 {
        parts.push("assign_process");
    }
    if parts.is_empty() {
        "(none)".to_string()
    } else {
        parts.join("+")
    }
}

/// Produce the exact prompt string `TerminalApproval::request_capability`
/// writes to the approver's terminal.
///
/// Returning a `String` (no stdio side-effects) makes the prompt-selection
/// logic unit-testable without requiring a TTY — the returned string is
/// what the approver would have seen had stderr been a real terminal.
///
/// Dispatch rule (Phase 18.1 Plan 18.1-01 G-02 fix):
///
/// - `target == Some(_)` → route through [`format_capability_prompt`] with
///   the D-04-locked per-kind template. All 5 new AIPC HandleKinds (Event,
///   Mutex, Pipe, Socket, JobObject) take this path.
/// - `target == None` → Phase 11 legacy File shape. Renders the byte-
///   identical multi-line block (`[nono] The sandboxed process is
///   requesting additional access:` / `Path:` / `Access:` / `Reason:` /
///   blank / `Grant access? [y/N] `) so Phase 11 UAT output is preserved.
///
/// Every untrusted string (`path`, `reason`, and — transitively via
/// `format_capability_prompt` — `name`/`host`) is scrubbed via
/// [`sanitize_for_terminal`] before embedding.
fn build_prompt_text(request: &CapabilityRequest) -> String {
    match request.target.as_ref() {
        Some(target) => {
            // D-01: route the 5 new AIPC HandleKind shapes through the
            // D-04-locked template helper. G-02 fix.
            format_capability_prompt(
                request.kind,
                target,
                request.access_mask,
                request.reason.as_deref(),
            )
        }
        None => {
            // Phase 11 legacy File path — target is None. Preserve the
            // byte-identical pre-fix multi-line block so Phase 11 UAT
            // output does not regress.
            #[allow(deprecated)]
            let request_path = &request.path;
            let mut out = String::new();
            out.push('\n');
            out.push_str("[nono] The sandboxed process is requesting additional access:\n");
            out.push_str(&format!(
                "[nono]   Path:   {}\n",
                sanitize_for_terminal(&request_path.display().to_string())
            ));
            out.push_str(&format!(
                "[nono]   Access: {}\n",
                format_access_mode(&request.access)
            ));
            if let Some(ref reason) = request.reason {
                out.push_str(&format!(
                    "[nono]   Reason: {}\n",
                    sanitize_for_terminal(reason)
                ));
            }
            out.push_str("[nono]\n");
            out.push_str("[nono] Grant access? [y/N] ");
            out
        }
    }
}

/// Render the per-handle-type approval prompt per CONTEXT.md D-04
/// (Phase 18 AIPC-01).
///
/// Every untrusted string field (`host`, `name`, `reason`) is run through
/// [`sanitize_for_terminal`] before embedding in the output. The CONIN$
/// branch + y/N parser are reused unchanged from Phase 11 D-04; this
/// function only produces the prompt string.
///
/// Consumed by `build_prompt_text`, which
/// `TerminalApproval::request_capability` calls for every AIPC (non-File)
/// approval prompt. Phase 18.1 Plan 18.1-01 (G-02) wired the live
/// dispatcher through this helper — before that fix the dispatcher emitted
/// a generic 3-line `Path:/Access:/Reason:` block and the per-kind helpers
/// were unreachable dead code.
pub(crate) fn format_capability_prompt(
    kind: HandleKind,
    target: &HandleTarget,
    access_mask: u32,
    reason: Option<&str>,
) -> String {
    let reason_display = sanitize_for_terminal(reason.unwrap_or(""));
    match (kind, target) {
        (HandleKind::File, HandleTarget::FilePath { path }) => {
            let path_display = sanitize_for_terminal(&path.display().to_string());
            // For File, access semantics come from CapabilityRequest.access
            // (Phase 11 shape) not access_mask. Caller is responsible for
            // passing the right mask string; this branch keeps the historical
            // Phase 11 prompt shape but in the D-04 single-template format.
            format!(
                "[nono] Grant file access? path={path_display} access=0x{access_mask:08x} reason=\"{reason_display}\" [y/N]"
            )
        }
        (HandleKind::Event, HandleTarget::EventName { name }) => {
            let name_display = sanitize_for_terminal(name);
            let access_display = format_event_access(access_mask);
            format!(
                "[nono] Grant event access? name={name_display} access={access_display} reason=\"{reason_display}\" [y/N]"
            )
        }
        (HandleKind::Mutex, HandleTarget::MutexName { name }) => {
            let name_display = sanitize_for_terminal(name);
            let access_display = format_mutex_access(access_mask);
            format!(
                "[nono] Grant mutex access? name={name_display} access={access_display} reason=\"{reason_display}\" [y/N]"
            )
        }
        (HandleKind::Pipe, HandleTarget::PipeName { name }) => {
            let name_display = sanitize_for_terminal(name);
            let direction = format_pipe_direction(access_mask);
            format!(
                "[nono] Grant pipe access? name={name_display} direction={direction} reason=\"{reason_display}\" [y/N]"
            )
        }
        (
            HandleKind::Socket,
            HandleTarget::SocketEndpoint {
                protocol,
                host,
                port,
                role,
            },
        ) => {
            let host_display = sanitize_for_terminal(host);
            let proto = match protocol {
                SocketProtocol::Tcp => "tcp",
                SocketProtocol::Udp => "udp",
            };
            let role_display = match role {
                SocketRole::Connect => "connect",
                SocketRole::Bind => "bind",
                SocketRole::Listen => "listen",
            };
            format!(
                "[nono] Grant socket access? proto={proto} host={host_display} port={port} role={role_display} reason=\"{reason_display}\" [y/N]"
            )
        }
        (HandleKind::JobObject, HandleTarget::JobObjectName { name }) => {
            let name_display = sanitize_for_terminal(name);
            let access_display = format_job_object_access(access_mask);
            format!(
                "[nono] Grant Job Object access? name={name_display} access={access_display} reason=\"{reason_display}\" [y/N]"
            )
        }
        // Mismatched (kind, target) shapes — defense-in-depth; the dispatcher
        // should reject these BEFORE calling this helper, but emitting a
        // clear placeholder is safer than panicking.
        _ => format!(
            "[nono] Grant unknown access? (kind/target mismatch) reason=\"{reason_display}\" [y/N]"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_approval_backend_name() {
        let backend = TerminalApproval;
        assert_eq!(backend.backend_name(), "terminal");
    }

    #[test]
    fn test_format_access_mode() {
        assert_eq!(format_access_mode(&AccessMode::Read), "read-only");
        assert_eq!(format_access_mode(&AccessMode::Write), "write-only");
        assert_eq!(format_access_mode(&AccessMode::ReadWrite), "read+write");
    }

    #[test]
    fn test_sanitize_clean_input() {
        assert_eq!(sanitize_for_terminal("/tmp/harmless"), "/tmp/harmless");
    }

    #[test]
    fn test_sanitize_carriage_return_overwrite() {
        // An attacker could use \r to overwrite the displayed path
        let malicious = "/etc/shadow\r/tmp/harmless";
        let sanitized = sanitize_for_terminal(malicious);
        assert!(!sanitized.contains('\r'));
        assert!(sanitized.contains("/etc/shadow"));
        assert!(sanitized.contains("/tmp/harmless"));
    }

    #[test]
    fn test_sanitize_ansi_escape_csi() {
        // ANSI CSI sequence to change colors / move cursor
        let malicious = "/tmp/\x1b[2K\x1b[1A/etc/shadow";
        let sanitized = sanitize_for_terminal(malicious);
        assert!(!sanitized.contains('\x1b'));
        assert!(sanitized.contains("/tmp/"));
    }

    #[test]
    fn test_sanitize_ansi_escape_osc() {
        // OSC sequence (e.g., change terminal title)
        let malicious = "/tmp/\x1b]0;evil\x07path";
        let sanitized = sanitize_for_terminal(malicious);
        assert!(!sanitized.contains('\x1b'));
        assert!(!sanitized.contains('\x07'));
    }

    #[test]
    fn test_sanitize_null_bytes() {
        let malicious = "/tmp/\0evil";
        let sanitized = sanitize_for_terminal(malicious);
        assert!(!sanitized.contains('\0'));
    }

    #[test]
    fn test_sanitize_all_control_chars_replaced() {
        for byte in 0x00u8..=0x1f {
            let input = format!("/tmp/{}evil", byte as char);
            let sanitized = sanitize_for_terminal(&input);
            assert!(
                !sanitized.chars().any(|c| c == byte as char),
                "Control byte 0x{:02x} should be stripped",
                byte
            );
        }
        // DEL (0x7F) is handled as control too
        let del_input = "/tmp/\x7Fevil";
        let sanitized = sanitize_for_terminal(del_input);
        assert!(!sanitized.contains('\x7F'));
    }

    #[test]
    fn test_sanitize_dcs_sequence() {
        // DCS (ESC P ... ST) -- Device Control String
        let malicious = "/tmp/\x1bPq#0;2;0;0;0#1;2;100;100;0\x1b\\path";
        let sanitized = sanitize_for_terminal(malicious);
        assert!(!sanitized.contains('\x1b'));
        assert!(sanitized.contains("/tmp/"));
        assert!(sanitized.contains("path"));
    }

    #[test]
    fn test_sanitize_apc_sequence() {
        // APC (ESC _) -- Application Program Command
        let malicious = "/tmp/\x1b_evil-command\x1b\\path";
        let sanitized = sanitize_for_terminal(malicious);
        assert!(!sanitized.contains('\x1b'));
        assert!(sanitized.contains("/tmp/"));
        assert!(sanitized.contains("path"));
    }

    #[test]
    fn test_sanitize_pm_sequence() {
        // PM (ESC ^) -- Privacy Message
        let malicious = "/tmp/\x1b^private-data\x1b\\path";
        let sanitized = sanitize_for_terminal(malicious);
        assert!(!sanitized.contains('\x1b'));
        assert!(sanitized.contains("/tmp/"));
        assert!(sanitized.contains("path"));
    }

    #[test]
    fn test_sanitize_sos_sequence() {
        // SOS (ESC X) -- Start of String
        let malicious = "/tmp/\x1bXsome-string\x1b\\path";
        let sanitized = sanitize_for_terminal(malicious);
        assert!(!sanitized.contains('\x1b'));
        assert!(sanitized.contains("/tmp/"));
        assert!(sanitized.contains("path"));
    }

    #[test]
    fn test_sanitize_unterminated_csi() {
        // Unterminated CSI: ESC [ with no final byte -- exhausts iterator cleanly
        let malicious = "/tmp/\x1b[999";
        let sanitized = sanitize_for_terminal(malicious);
        assert!(!sanitized.contains('\x1b'));
        assert!(sanitized.contains("/tmp/"));
    }

    /// Regression guard for Task 1 (plan 11-02): `sanitize_for_terminal`
    /// must remain platform-agnostic and strip ANSI SGR escapes.
    #[test]
    fn sanitize_for_terminal_strips_ansi() {
        let input = "\x1b[31mred\x1b[0m";
        let sanitized = sanitize_for_terminal(input);
        assert_eq!(sanitized, "red");
    }

    /// Windows-only fail-secure check (plan 11-02 Task 1, T-11-10).
    ///
    /// Under `cargo test`, stderr is captured, so `is_terminal()` returns
    /// false and `request_capability` returns `Denied` at the first guard.
    /// That guard's reason mentions "terminal". If a runner ever surfaces
    /// a real console to the test process, the `\\.\CONIN$` open path is
    /// the fallback; either path must produce a denial whose reason
    /// references the absent interactive device.
    #[cfg(target_os = "windows")]
    #[test]
    #[allow(deprecated)]
    fn windows_no_console_denies_gracefully() {
        let backend = TerminalApproval;
        let request = CapabilityRequest {
            request_id: "test-req-1".to_string(),
            path: std::path::PathBuf::from(r"C:\tmp\x"),
            access: AccessMode::Read,
            reason: Some("unit test".to_string()),
            child_pid: std::process::id(),
            session_id: "sess-test".to_string(),
            session_token: String::new(),
            kind: nono::supervisor::types::HandleKind::File,
            target: None,
            access_mask: 0,
        };

        let decision = backend
            .request_capability(&request)
            .expect("request_capability must not error on the deny path");

        match decision {
            ApprovalDecision::Denied { reason } => {
                let lower = reason.to_lowercase();
                assert!(
                    lower.contains("terminal")
                        || lower.contains("console")
                        || lower.contains("tty"),
                    "denial reason should mention the missing interactive device: {reason}"
                );
            }
            other => panic!("expected Denied, got {other:?}"),
        }
    }

    // Phase 18 AIPC-01 Task 4 — `format_capability_prompt` helper tests.

    #[test]
    fn format_capability_prompt_file_kind() {
        let target = HandleTarget::FilePath {
            path: std::path::PathBuf::from("/tmp/x"),
        };
        let prompt = format_capability_prompt(HandleKind::File, &target, 0, Some("agent op"));
        // For File, access is rendered as `0x00000000` here because the
        // helper is the D-04 single template; the legacy `request_capability`
        // body keeps the Phase 11 multi-line format unchanged.
        assert!(
            prompt.contains("Grant file access? path=/tmp/x"),
            "prompt missing File template prefix: {prompt}"
        );
        assert!(prompt.contains("reason=\"agent op\""), "prompt: {prompt}");
        assert!(prompt.ends_with("[y/N]"), "prompt: {prompt}");
    }

    #[test]
    fn format_capability_prompt_event_kind() {
        let target = HandleTarget::EventName {
            name: "shutdown".to_string(),
        };
        let prompt = format_capability_prompt(
            HandleKind::Event,
            &target,
            policy::EVENT_DEFAULT_MASK,
            Some("lifecycle"),
        );
        assert_eq!(
            prompt,
            r#"[nono] Grant event access? name=shutdown access=wait+signal reason="lifecycle" [y/N]"#
        );
    }

    #[test]
    fn format_capability_prompt_mutex_kind() {
        let target = HandleTarget::MutexName {
            name: "logfile".to_string(),
        };
        let prompt =
            format_capability_prompt(HandleKind::Mutex, &target, policy::MUTEX_DEFAULT_MASK, None);
        assert_eq!(
            prompt,
            r#"[nono] Grant mutex access? name=logfile access=wait+release reason="" [y/N]"#
        );
    }

    #[test]
    fn prompt_sanitizes_untrusted_target_strings() {
        let target = HandleTarget::EventName {
            name: "\x1b[31mevil\x1b[0m".to_string(),
        };
        let prompt = format_capability_prompt(
            HandleKind::Event,
            &target,
            policy::EVENT_DEFAULT_MASK,
            Some("\x1b[31malso-evil\x1b[0m"),
        );
        // Sanitizer must strip ANSI bytes from BOTH the name and the reason.
        assert!(!prompt.contains('\x1b'), "ANSI byte leaked: {prompt}");
        assert!(prompt.contains("evil"), "literal name missing: {prompt}");
        assert!(
            prompt.contains("also-evil"),
            "literal reason missing: {prompt}"
        );
    }

    #[test]
    fn prompt_renders_kind_target_mismatch_safely() {
        // After Plan 18-03 wires the JobObject branch, the only remaining
        // catch-all `_` arm in `format_capability_prompt` fires on a
        // (kind, target) shape mismatch — defense-in-depth for a dispatcher
        // bug. The arm must NOT panic and must surface a clear placeholder.
        let target = HandleTarget::JobObjectName {
            name: "wrong-shape".to_string(),
        };
        let prompt = format_capability_prompt(HandleKind::Event, &target, 0, Some("test"));
        assert!(
            prompt.contains("kind/target mismatch"),
            "expected catch-all fallback for kind/target mismatch: {prompt}"
        );
        assert!(prompt.contains("reason=\"test\""), "prompt: {prompt}");
    }

    #[test]
    fn format_capability_prompt_job_object_kind() {
        // Plan 18-03 wires the JobObject branch with the D-04-locked template.
        let target = HandleTarget::JobObjectName {
            name: "test-orch".to_string(),
        };
        let prompt = format_capability_prompt(
            HandleKind::JobObject,
            &target,
            policy::JOB_OBJECT_QUERY,
            Some("orch monitor"),
        );
        assert_eq!(
            prompt,
            r#"[nono] Grant Job Object access? name=test-orch access=query reason="orch monitor" [y/N]"#
        );
    }

    #[test]
    fn format_capability_prompt_job_object_kind_renders_terminate_widening() {
        let target = HandleTarget::JobObjectName {
            name: "agent-tree".to_string(),
        };
        let prompt = format_capability_prompt(
            HandleKind::JobObject,
            &target,
            policy::JOB_OBJECT_QUERY | policy::JOB_OBJECT_TERMINATE,
            Some("kill on hang"),
        );
        // Order is query+terminate per format_job_object_access bit order.
        assert!(
            prompt.contains("access=query+terminate"),
            "expected query+terminate access label: {prompt}"
        );
        assert!(prompt.contains("name=agent-tree"), "prompt: {prompt}");
    }

    // Phase 18 AIPC-01 Plan 18-02 Task 3 — format_capability_prompt tests
    // for the new live Pipe and Socket branches.

    #[test]
    fn format_capability_prompt_pipe_kind() {
        let target = HandleTarget::PipeName {
            name: "test-stream".to_string(),
        };
        let prompt = format_capability_prompt(
            HandleKind::Pipe,
            &target,
            policy::GENERIC_READ,
            Some("agent op"),
        );
        assert_eq!(
            prompt,
            r#"[nono] Grant pipe access? name=test-stream direction=read reason="agent op" [y/N]"#
        );
    }

    #[test]
    fn format_capability_prompt_socket_kind_connect() {
        let target = HandleTarget::SocketEndpoint {
            protocol: SocketProtocol::Tcp,
            host: "example.com".to_string(),
            port: 8080,
            role: SocketRole::Connect,
        };
        let prompt = format_capability_prompt(HandleKind::Socket, &target, 0, Some("agent fetch"));
        assert_eq!(
            prompt,
            r#"[nono] Grant socket access? proto=tcp host=example.com port=8080 role=connect reason="agent fetch" [y/N]"#
        );
    }

    #[test]
    fn prompt_sanitizes_socket_host_string() {
        let target = HandleTarget::SocketEndpoint {
            protocol: SocketProtocol::Udp,
            host: "evil\x1b[31mhost\x1b[0m".to_string(),
            port: 9000,
            role: SocketRole::Connect,
        };
        let prompt = format_capability_prompt(HandleKind::Socket, &target, 0, Some("dns"));
        assert!(!prompt.contains('\x1b'), "ANSI byte leaked: {prompt}");
        assert!(
            prompt.contains("evilhost"),
            "literal host missing: {prompt}"
        );
        assert!(prompt.contains("proto=udp"), "proto missing: {prompt}");
        assert!(prompt.contains("role=connect"), "role missing: {prompt}");
    }

    // Phase 18.1 AIPC-01 Plan 18.1-01 (G-02) — `build_prompt_text` dispatcher
    // tests. These assert the live `TerminalApproval::request_capability`
    // prompt text end-to-end: AIPC kinds route through
    // `format_capability_prompt` (per-kind D-04 templates); File-kind legacy
    // shape (target == None, Phase 11 path) preserves the byte-identical
    // multi-line block. The helper returns a `String` (no TTY side-effects)
    // so the tests can run on any host without stdio capture.

    #[test]
    fn build_prompt_text_event_kind() {
        #[allow(deprecated)]
        let request = CapabilityRequest {
            request_id: "r1".to_string(),
            path: std::path::PathBuf::new(),
            access: AccessMode::Read,
            reason: Some("lifecycle".to_string()),
            child_pid: 0,
            session_id: "s1".to_string(),
            session_token: String::new(),
            kind: HandleKind::Event,
            target: Some(HandleTarget::EventName {
                name: "shutdown".to_string(),
            }),
            access_mask: policy::EVENT_DEFAULT_MASK,
        };
        let prompt = build_prompt_text(&request);
        assert_eq!(
            prompt,
            r#"[nono] Grant event access? name=shutdown access=wait+signal reason="lifecycle" [y/N]"#
        );
    }

    #[test]
    fn build_prompt_text_mutex_kind() {
        #[allow(deprecated)]
        let request = CapabilityRequest {
            request_id: "r1".to_string(),
            path: std::path::PathBuf::new(),
            access: AccessMode::Read,
            reason: Some("logfile sync".to_string()),
            child_pid: 0,
            session_id: "s1".to_string(),
            session_token: String::new(),
            kind: HandleKind::Mutex,
            target: Some(HandleTarget::MutexName {
                name: "logfile".to_string(),
            }),
            access_mask: policy::MUTEX_DEFAULT_MASK,
        };
        let prompt = build_prompt_text(&request);
        assert_eq!(
            prompt,
            r#"[nono] Grant mutex access? name=logfile access=wait+release reason="logfile sync" [y/N]"#
        );
    }

    #[test]
    fn build_prompt_text_pipe_kind() {
        #[allow(deprecated)]
        let request = CapabilityRequest {
            request_id: "r1".to_string(),
            path: std::path::PathBuf::new(),
            access: AccessMode::Read,
            reason: Some("agent op".to_string()),
            child_pid: 0,
            session_id: "s1".to_string(),
            session_token: String::new(),
            kind: HandleKind::Pipe,
            target: Some(HandleTarget::PipeName {
                name: "test-stream".to_string(),
            }),
            access_mask: policy::GENERIC_READ,
        };
        let prompt = build_prompt_text(&request);
        assert_eq!(
            prompt,
            r#"[nono] Grant pipe access? name=test-stream direction=read reason="agent op" [y/N]"#
        );
    }

    #[test]
    fn build_prompt_text_socket_kind() {
        #[allow(deprecated)]
        let request = CapabilityRequest {
            request_id: "r1".to_string(),
            path: std::path::PathBuf::new(),
            access: AccessMode::Read,
            reason: Some("agent fetch".to_string()),
            child_pid: 0,
            session_id: "s1".to_string(),
            session_token: String::new(),
            kind: HandleKind::Socket,
            target: Some(HandleTarget::SocketEndpoint {
                protocol: SocketProtocol::Tcp,
                host: "example.com".to_string(),
                port: 8080,
                role: SocketRole::Connect,
            }),
            access_mask: 0,
        };
        let prompt = build_prompt_text(&request);
        assert_eq!(
            prompt,
            r#"[nono] Grant socket access? proto=tcp host=example.com port=8080 role=connect reason="agent fetch" [y/N]"#
        );
    }

    #[test]
    fn build_prompt_text_job_object_kind() {
        #[allow(deprecated)]
        let request = CapabilityRequest {
            request_id: "r1".to_string(),
            path: std::path::PathBuf::new(),
            access: AccessMode::Read,
            reason: Some("orch monitor".to_string()),
            child_pid: 0,
            session_id: "s1".to_string(),
            session_token: String::new(),
            kind: HandleKind::JobObject,
            target: Some(HandleTarget::JobObjectName {
                name: "test-orch".to_string(),
            }),
            access_mask: policy::JOB_OBJECT_QUERY,
        };
        let prompt = build_prompt_text(&request);
        assert_eq!(
            prompt,
            r#"[nono] Grant Job Object access? name=test-orch access=query reason="orch monitor" [y/N]"#
        );
    }

    #[test]
    fn build_prompt_text_file_kind_preserves_legacy_block() {
        // Phase 11 legacy shape: target=None, kind=File. The pre-fix
        // multi-line block (`[nono] The sandboxed process is requesting...`
        // / Path / Access / Reason / blank / Grant access? [y/N] ) must
        // render byte-identical so Phase 11 UAT output is preserved.
        #[allow(deprecated)]
        let request = CapabilityRequest {
            request_id: "r1".to_string(),
            path: std::path::PathBuf::from("/tmp/secret"),
            access: AccessMode::ReadWrite,
            reason: Some("agent op".to_string()),
            child_pid: 0,
            session_id: "s1".to_string(),
            session_token: String::new(),
            kind: HandleKind::File,
            target: None,
            access_mask: 0,
        };
        let prompt = build_prompt_text(&request);
        assert!(
            prompt.contains("[nono] The sandboxed process is requesting additional access:"),
            "legacy preamble missing: {prompt}"
        );
        assert!(
            prompt.contains("[nono]   Path:   /tmp/secret"),
            "legacy Path line missing: {prompt}"
        );
        assert!(
            prompt.contains("[nono]   Access: read+write"),
            "legacy Access line missing: {prompt}"
        );
        assert!(
            prompt.contains("[nono]   Reason: agent op"),
            "legacy Reason line missing: {prompt}"
        );
        assert!(
            prompt.contains("[nono] Grant access? [y/N] "),
            "legacy trailing prompt missing: {prompt}"
        );
    }
}
