//! Child-side SDK for AIPC-01 handle brokering.
//!
//! These functions are the cross-platform consumer surface that sandboxed
//! children call to request a Windows kernel handle from their supervisor.
//! Each function constructs a [`CapabilityRequest`] using the
//! [`HandleKind`]/[`HandleTarget`] protocol skeleton from Plan 18-01, posts
//! it over the existing Phase 11 capability pipe transport, and returns the
//! brokered raw handle on `Granted`.
//!
//! On non-Windows builds (Linux/macOS), each function returns
//! [`NonoError::UnsupportedPlatform`] immediately so cross-platform Rust code
//! compiles against the SDK without `#[cfg]` everywhere. Per CONTEXT.md
//! D-09: Unix has `SCM_RIGHTS` file-descriptor passing as the natural
//! equivalent for sockets/pipes (separate cross-platform requirement, future
//! milestone). Events, mutexes, and Job Objects have no direct Unix analog.
//!
//! # Security
//!
//! - The SDK is a CONVENIENCE wrapper. Server-side enforcement is
//!   load-bearing per CONTEXT.md D-07 — the supervisor re-validates every
//!   `access_mask` against the resolved per-type allowlist. A malicious or
//!   buggy SDK client cannot bypass the supervisor's constant-time
//!   discriminator check (D-03), the per-type mask validator
//!   (`policy::mask_is_allowed`), or the CONIN$ approval prompt by
//!   stamping arbitrary fields on the request.
//! - The SDK reads `NONO_SESSION_TOKEN` from env (Phase 11 D-01) and
//!   stamps it on every request. The token is NEVER logged by the SDK;
//!   the supervisor's audit-redaction helper
//!   (`audit_entry_with_redacted_token`) clears it before any
//!   `AuditEntry` is pushed (Phase 11 D-11; preserved by Plans
//!   18-01..18-03).
//! - All `NonoError` propagation is via `?` per project standards.
//!
//! # Timeout semantics (T-18-04-07)
//!
//! `SupervisorSocket::recv_response` does not impose a read timeout; the
//! caller is responsible for setting one via `set_read_timeout` before
//! invoking the SDK if indefinite blocking is a concern. Long-running
//! children may legitimately wait for a slow human approval via CONIN$.

use crate::error::{NonoError, Result};
use crate::supervisor::socket::SupervisorSocket;
use crate::supervisor::types::{PipeDirection, SocketProtocol, SocketRole};

#[cfg(target_os = "windows")]
use crate::capability::AccessMode;
#[cfg(target_os = "windows")]
use crate::supervisor::types::{
    ApprovalDecision, CapabilityRequest, HandleKind, HandleTarget, ResourceGrant,
    ResourceTransferKind, SupervisorMessage, SupervisorResponse,
};

/// Type alias for a raw Windows `SOCKET` value transported as `u64` over the
/// wire. The wire shape matches Phase 11's `raw_handle: u64` representation;
/// this alias disambiguates caller intent at the `request_socket` method
/// signature without changing the wire format.
pub type RawSocket = u64;

/// Type alias for a raw Windows `HANDLE` value transported as `u64` over the
/// wire. Used by `request_pipe`, `request_job_object`, `request_event`, and
/// `request_mutex`.
pub type RawHandle = u64;

/// The exact error message returned by all 5 `request_*` methods on
/// non-Windows builds, per CONTEXT.md D-09. Single source of truth so
/// downstream consumers can grep for the exact string and tests can assert on
/// it without literal-string duplication.
#[must_use]
pub fn unsupported_platform_message() -> &'static str {
    "AIPC handle brokering is Windows-only on v2.1; Unix has SCM_RIGHTS \
     file-descriptor passing as the natural equivalent for sockets/pipes \
     (separate cross-platform requirement, future milestone). Events, \
     mutexes, and Job Objects have no direct Unix analog."
}

// -------------------------------------------------------------------------
// request_socket
// -------------------------------------------------------------------------

/// Request a brokered Windows `SOCKET` from the supervisor. Returns the raw
/// socket reconstructed from the supervisor-provided `WSAPROTOCOL_INFOW`
/// blob via `WSASocketW(FROM_PROTOCOL_INFO, ...)`.
///
/// # Errors
///
/// Returns `NonoError::UnsupportedPlatform` on non-Windows builds, and
/// `NonoError::SandboxInit` on all other failure modes (transport error,
/// supervisor denial, blob deserialization failure, `WSASocketW` failure).
#[cfg(target_os = "windows")]
pub fn request_socket(
    cap_pipe: &mut SupervisorSocket,
    host: &str,
    port: u16,
    protocol: SocketProtocol,
    role: SocketRole,
    access_mask: u32,
    reason: Option<&str>,
) -> Result<RawSocket> {
    let target = HandleTarget::SocketEndpoint {
        protocol,
        host: host.to_string(),
        port,
        role,
    };
    let grant = send_capability_request(cap_pipe, HandleKind::Socket, target, access_mask, reason)?;
    // Defense-in-depth: the supervisor is supposed to enforce the Socket
    // transport contract (Plan 18-02), but we double-check on the client
    // side so a corrupted response doesn't get silently misinterpreted as
    // a HANDLE. T-18-04-04 mitigation.
    if grant.transfer != ResourceTransferKind::SocketProtocolInfoBlob {
        return Err(NonoError::SandboxInit(format!(
            "expected SocketProtocolInfoBlob transfer for Socket grant, got {:?}",
            grant.transfer
        )));
    }
    let blob = grant.protocol_info_blob.ok_or_else(|| {
        NonoError::SandboxInit("Socket grant missing protocol_info_blob".to_string())
    })?;
    reconstruct_socket_from_blob(&blob)
}

#[cfg(not(target_os = "windows"))]
pub fn request_socket(
    _cap_pipe: &mut SupervisorSocket,
    _host: &str,
    _port: u16,
    _protocol: SocketProtocol,
    _role: SocketRole,
    _access_mask: u32,
    _reason: Option<&str>,
) -> Result<RawSocket> {
    Err(NonoError::UnsupportedPlatform(
        unsupported_platform_message().to_string(),
    ))
}

// -------------------------------------------------------------------------
// request_pipe
// -------------------------------------------------------------------------

/// Request a brokered named-pipe handle from the supervisor.
///
/// The `direction` is encoded server-side into an access mask derived from
/// Plan 18-02's `GENERIC_READ`/`GENERIC_WRITE` mapping. The SDK's
/// `access_mask` value derived from direction is informational here; the
/// supervisor recomputes the mask from `HandleTarget::PipeName` +
/// `PipeDirection` policy gate.
///
/// # Errors
///
/// Returns `NonoError::UnsupportedPlatform` on non-Windows builds, and
/// `NonoError::SandboxInit` on all other failure modes.
#[cfg(target_os = "windows")]
pub fn request_pipe(
    cap_pipe: &mut SupervisorSocket,
    name: &str,
    direction: PipeDirection,
    reason: Option<&str>,
) -> Result<RawHandle> {
    let target = HandleTarget::PipeName {
        name: name.to_string(),
    };
    let access_mask = pipe_mask_for(direction);
    let grant = send_capability_request(cap_pipe, HandleKind::Pipe, target, access_mask, reason)?;
    extract_duplicated_handle(&grant, "Pipe")
}

#[cfg(not(target_os = "windows"))]
pub fn request_pipe(
    _cap_pipe: &mut SupervisorSocket,
    _name: &str,
    _direction: PipeDirection,
    _reason: Option<&str>,
) -> Result<RawHandle> {
    Err(NonoError::UnsupportedPlatform(
        unsupported_platform_message().to_string(),
    ))
}

// -------------------------------------------------------------------------
// request_job_object
// -------------------------------------------------------------------------

/// Request a brokered Job Object handle from the supervisor.
///
/// # Errors
///
/// Returns `NonoError::UnsupportedPlatform` on non-Windows builds, and
/// `NonoError::SandboxInit` on all other failure modes.
#[cfg(target_os = "windows")]
pub fn request_job_object(
    cap_pipe: &mut SupervisorSocket,
    name: &str,
    access_mask: u32,
    reason: Option<&str>,
) -> Result<RawHandle> {
    let target = HandleTarget::JobObjectName {
        name: name.to_string(),
    };
    let grant =
        send_capability_request(cap_pipe, HandleKind::JobObject, target, access_mask, reason)?;
    extract_duplicated_handle(&grant, "JobObject")
}

#[cfg(not(target_os = "windows"))]
pub fn request_job_object(
    _cap_pipe: &mut SupervisorSocket,
    _name: &str,
    _access_mask: u32,
    _reason: Option<&str>,
) -> Result<RawHandle> {
    Err(NonoError::UnsupportedPlatform(
        unsupported_platform_message().to_string(),
    ))
}

// -------------------------------------------------------------------------
// request_event
// -------------------------------------------------------------------------

/// Request a brokered Event kernel-object handle from the supervisor.
///
/// # Errors
///
/// Returns `NonoError::UnsupportedPlatform` on non-Windows builds, and
/// `NonoError::SandboxInit` on all other failure modes.
#[cfg(target_os = "windows")]
pub fn request_event(
    cap_pipe: &mut SupervisorSocket,
    name: &str,
    access_mask: u32,
    reason: Option<&str>,
) -> Result<RawHandle> {
    let target = HandleTarget::EventName {
        name: name.to_string(),
    };
    let grant = send_capability_request(cap_pipe, HandleKind::Event, target, access_mask, reason)?;
    extract_duplicated_handle(&grant, "Event")
}

#[cfg(not(target_os = "windows"))]
pub fn request_event(
    _cap_pipe: &mut SupervisorSocket,
    _name: &str,
    _access_mask: u32,
    _reason: Option<&str>,
) -> Result<RawHandle> {
    Err(NonoError::UnsupportedPlatform(
        unsupported_platform_message().to_string(),
    ))
}

// -------------------------------------------------------------------------
// request_mutex
// -------------------------------------------------------------------------

/// Request a brokered Mutex kernel-object handle from the supervisor.
///
/// # Errors
///
/// Returns `NonoError::UnsupportedPlatform` on non-Windows builds, and
/// `NonoError::SandboxInit` on all other failure modes.
#[cfg(target_os = "windows")]
pub fn request_mutex(
    cap_pipe: &mut SupervisorSocket,
    name: &str,
    access_mask: u32,
    reason: Option<&str>,
) -> Result<RawHandle> {
    let target = HandleTarget::MutexName {
        name: name.to_string(),
    };
    let grant = send_capability_request(cap_pipe, HandleKind::Mutex, target, access_mask, reason)?;
    extract_duplicated_handle(&grant, "Mutex")
}

#[cfg(not(target_os = "windows"))]
pub fn request_mutex(
    _cap_pipe: &mut SupervisorSocket,
    _name: &str,
    _access_mask: u32,
    _reason: Option<&str>,
) -> Result<RawHandle> {
    Err(NonoError::UnsupportedPlatform(
        unsupported_platform_message().to_string(),
    ))
}

// -------------------------------------------------------------------------
// Shared Windows-only helpers (private to module)
// -------------------------------------------------------------------------

/// Derive a GENERIC_READ/GENERIC_WRITE access mask from a [`PipeDirection`],
/// matching Plan 18-02's server-side mapping.
#[cfg(target_os = "windows")]
fn pipe_mask_for(direction: PipeDirection) -> u32 {
    // Mirrors windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE}
    // without adding a new import at the module top (the constants are
    // `crate::supervisor::policy::GENERIC_READ` / `GENERIC_WRITE`).
    match direction {
        PipeDirection::Read => crate::supervisor::policy::GENERIC_READ,
        PipeDirection::Write => crate::supervisor::policy::GENERIC_WRITE,
        PipeDirection::ReadWrite => {
            crate::supervisor::policy::GENERIC_READ | crate::supervisor::policy::GENERIC_WRITE
        }
    }
}

/// Extract the duplicated raw HANDLE from a `DuplicatedWindowsHandle` grant,
/// validating the transfer contract. Shared by Event / Mutex / Pipe /
/// JobObject paths (all of which use the `DuplicatedWindowsHandle` transport
/// per Plans 18-01..18-03). T-18-04-04 mitigation: catches corrupted
/// responses that mis-tag the transport.
#[cfg(target_os = "windows")]
fn extract_duplicated_handle(grant: &ResourceGrant, kind_name: &str) -> Result<RawHandle> {
    if grant.transfer != ResourceTransferKind::DuplicatedWindowsHandle {
        return Err(NonoError::SandboxInit(format!(
            "expected DuplicatedWindowsHandle transfer for {kind_name} grant, got {:?}",
            grant.transfer
        )));
    }
    grant
        .raw_handle
        .ok_or_else(|| NonoError::SandboxInit(format!("{kind_name} grant missing raw_handle")))
}

/// Generate a 128-bit hex request_id via the workspace `getrandom` dep (no
/// new crate dependency). Rendered as 32 lowercase hex characters.
#[cfg(target_os = "windows")]
fn generate_request_id() -> Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|e| NonoError::SandboxInit(format!("getrandom for request_id failed: {e}")))?;
    // Render as lowercase hex without pulling in the `hex` crate (which is
    // not a `nono` direct dep).
    let mut out = String::with_capacity(32);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    Ok(out)
}

/// Construct a [`CapabilityRequest`], send it, wait for a response,
/// demultiplex the [`ApprovalDecision`] into a typed `Result<ResourceGrant>`.
/// Shared by all 5 Windows-arm SDK methods.
///
/// - Reads `NONO_SESSION_TOKEN` from env (fail-secure if absent).
/// - Reads `NONO_SESSION_ID` from env (audit-correlation only; accept empty).
/// - Stamps `std::process::id()` as `child_pid` for audit correlation.
/// - Validates `response.request_id == request.request_id` (T-18-04-08).
#[cfg(target_os = "windows")]
fn send_capability_request(
    cap_pipe: &mut SupervisorSocket,
    kind: HandleKind,
    target: HandleTarget,
    access_mask: u32,
    reason: Option<&str>,
) -> Result<ResourceGrant> {
    let request_id = generate_request_id()?;

    // Phase 11 D-01 contract: hex-encoded 32 bytes, set by
    // execution_runtime.rs on child spawn. Missing → fail-secure with a
    // clear error. The supervisor would reject the request anyway via its
    // constant-time token check, but failing fast client-side gives a
    // better diagnostic. T-18-04-05 mitigation: error message does NOT
    // include any token value.
    let session_token = std::env::var("NONO_SESSION_TOKEN").map_err(|_| {
        NonoError::SandboxInit(
            "NONO_SESSION_TOKEN env var not set; AIPC SDK requires Phase 11 supervisor plumbing"
                .to_string(),
        )
    })?;

    // session_id is audit-correlation only per Phase 11 D-01; missing env →
    // empty string is acceptable (T-18-04-09).
    let session_id = std::env::var("NONO_SESSION_ID").unwrap_or_default();

    let child_pid = std::process::id();

    // Phase 11 `path` field is deprecated in place per 18-01 D-01. For AIPC
    // requests the target enum carries the typed payload; the `path` field
    // stays at its default (empty PathBuf) since the supervisor dispatches
    // on `kind` + `target`, not `path`.
    #[allow(deprecated)]
    let req = CapabilityRequest {
        request_id: request_id.clone(),
        path: std::path::PathBuf::new(),
        kind,
        target: Some(target),
        access_mask,
        // Phase 11 access field preserved for File path; for AIPC kinds the
        // access information lives in access_mask. Use AccessMode::Read as
        // a benign default — the supervisor ignores this field for non-File
        // kinds per Plan 18-01 dispatch.
        access: AccessMode::Read,
        reason: reason.map(str::to_string),
        child_pid,
        session_id,
        session_token,
    };

    cap_pipe.send_message(&SupervisorMessage::Request(req))?;

    match cap_pipe.recv_response()? {
        SupervisorResponse::Decision {
            request_id: resp_id,
            decision,
            grant,
        } => {
            if resp_id != request_id {
                // T-18-04-08 mitigation: response/request id drift.
                return Err(NonoError::SandboxInit(format!(
                    "supervisor response request_id mismatch: expected {request_id}, got {resp_id}"
                )));
            }
            match decision {
                ApprovalDecision::Granted => grant.ok_or_else(|| {
                    NonoError::SandboxInit(
                        "supervisor granted but returned no ResourceGrant".to_string(),
                    )
                }),
                ApprovalDecision::Denied { reason } => Err(NonoError::SandboxInit(format!(
                    "supervisor denied capability: {reason}"
                ))),
                ApprovalDecision::Timeout => Err(NonoError::SandboxInit(
                    "supervisor approval timed out".to_string(),
                )),
            }
        }
        other => Err(NonoError::SandboxInit(format!(
            "expected Decision response, got {other:?}"
        ))),
    }
}

/// Reconstruct a live `SOCKET` in this process from a serialized
/// `WSAPROTOCOL_INFOW` blob produced by the supervisor via
/// `WSADuplicateSocketW`. The blob is single-use and target-PID-bound per
/// the Microsoft API contract.
///
/// T-18-04-04 mitigation: validates the blob length matches the struct size
/// BEFORE any `unsafe` read; mismatch returns a descriptive
/// `NonoError::SandboxInit` with the exact byte counts.
#[cfg(target_os = "windows")]
fn reconstruct_socket_from_blob(blob: &[u8]) -> Result<RawSocket> {
    use std::mem::size_of;
    use windows_sys::Win32::Networking::WinSock::{
        WSAGetLastError, WSASocketW, AF_UNSPEC, FROM_PROTOCOL_INFO, INVALID_SOCKET,
        WSAPROTOCOL_INFOW, WSA_FLAG_OVERLAPPED,
    };

    let expected = size_of::<WSAPROTOCOL_INFOW>();
    if blob.len() != expected {
        return Err(NonoError::SandboxInit(format!(
            "WSAPROTOCOL_INFOW blob length mismatch: expected {expected} bytes, got {} bytes \
             (likely truncated transport)",
            blob.len()
        )));
    }

    // SAFETY: `blob` is a &[u8] slice of exactly size_of::<WSAPROTOCOL_INFOW>()
    // bytes (validated above). WSAPROTOCOL_INFOW is `#[repr(C)]` per the
    // Microsoft API contract. `read_unaligned` is used because the blob came
    // over the wire as `Vec<u8>` with no alignment guarantee. The blob was
    // produced by WSADuplicateSocketW server-side (Plan 18-02) and
    // serialized as raw bytes; we deserialize back to the same struct shape.
    let proto_info: WSAPROTOCOL_INFOW =
        unsafe { std::ptr::read_unaligned(blob.as_ptr() as *const WSAPROTOCOL_INFOW) };

    // SAFETY: WSASocketW with FROM_PROTOCOL_INFO and a non-null pointer to a
    // valid WSAPROTOCOL_INFOW reconstructs the duplicated socket. The blob
    // is single-use and target-PID-bound (per Plan 18-02 lifecycle); calling
    // on the wrong PID returns INVALID_SOCKET with WSAEINVAL, which is
    // caught and propagated as a typed error below. `AF_UNSPEC` /
    // `FROM_PROTOCOL_INFO` for the type and protocol slots instruct WSASocketW
    // to pull those values from the proto_info struct.
    let sock = unsafe {
        WSASocketW(
            AF_UNSPEC as i32,
            FROM_PROTOCOL_INFO,
            FROM_PROTOCOL_INFO,
            &proto_info as *const _,
            0,
            WSA_FLAG_OVERLAPPED,
        )
    };
    if sock == INVALID_SOCKET {
        // SAFETY: WSAGetLastError is a leaf-safe Win32 call that returns
        // thread-local Winsock error state. Always safe to call immediately
        // after a Winsock failure.
        let err = unsafe { WSAGetLastError() };
        return Err(NonoError::SandboxInit(format!(
            "WSASocketW(FROM_PROTOCOL_INFO) failed: WSAGetLastError = {err}"
        )));
    }
    Ok(sock as RawSocket)
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "windows"))]
    mod sdk_returns_unsupported_platform_on_non_windows {
        use super::*;

        // Helper: build a SupervisorSocket-shaped value just to satisfy the
        // function signature on non-Windows. The function never reads from it
        // because the non-Windows arm returns `UnsupportedPlatform`
        // immediately.
        fn dummy_pipe() -> SupervisorSocket {
            SupervisorSocket::pair().expect("pair").0
        }

        #[test]
        fn request_socket_returns_unsupported_platform() {
            let mut pipe = dummy_pipe();
            let result = request_socket(
                &mut pipe,
                "127.0.0.1",
                8080,
                SocketProtocol::Tcp,
                SocketRole::Connect,
                0,
                Some("test"),
            );
            match result {
                Err(NonoError::UnsupportedPlatform(ref msg))
                    if msg == unsupported_platform_message() => {}
                other => panic!("expected UnsupportedPlatform, got {other:?}"),
            }
        }

        #[test]
        fn request_pipe_returns_unsupported_platform() {
            let mut pipe = dummy_pipe();
            let result = request_pipe(&mut pipe, "test-pipe", PipeDirection::Read, Some("test"));
            match result {
                Err(NonoError::UnsupportedPlatform(ref msg))
                    if msg == unsupported_platform_message() => {}
                other => panic!("expected UnsupportedPlatform, got {other:?}"),
            }
        }

        #[test]
        fn request_job_object_returns_unsupported_platform() {
            let mut pipe = dummy_pipe();
            let result = request_job_object(
                &mut pipe,
                "test-job",
                crate::supervisor::policy::JOB_OBJECT_DEFAULT_MASK,
                Some("test"),
            );
            match result {
                Err(NonoError::UnsupportedPlatform(ref msg))
                    if msg == unsupported_platform_message() => {}
                other => panic!("expected UnsupportedPlatform, got {other:?}"),
            }
        }

        #[test]
        fn request_event_returns_unsupported_platform() {
            let mut pipe = dummy_pipe();
            let result = request_event(
                &mut pipe,
                "test-event",
                crate::supervisor::policy::EVENT_DEFAULT_MASK,
                Some("test"),
            );
            match result {
                Err(NonoError::UnsupportedPlatform(ref msg))
                    if msg == unsupported_platform_message() => {}
                other => panic!("expected UnsupportedPlatform, got {other:?}"),
            }
        }

        #[test]
        fn request_mutex_returns_unsupported_platform() {
            let mut pipe = dummy_pipe();
            let result = request_mutex(
                &mut pipe,
                "test-mutex",
                crate::supervisor::policy::MUTEX_DEFAULT_MASK,
                Some("test"),
            );
            match result {
                Err(NonoError::UnsupportedPlatform(ref msg))
                    if msg == unsupported_platform_message() => {}
                other => panic!("expected UnsupportedPlatform, got {other:?}"),
            }
        }
    }

    #[test]
    fn unsupported_platform_message_is_d09_locked_string() {
        let msg = unsupported_platform_message();
        // Snapshot test — guards against silent message drift breaking
        // cross-platform consumers grepping for the exact string.
        assert!(
            msg.contains("AIPC handle brokering is Windows-only on v2.1"),
            "message missing 'AIPC handle brokering is Windows-only on v2.1': {msg}"
        );
        assert!(
            msg.contains("SCM_RIGHTS"),
            "message missing 'SCM_RIGHTS': {msg}"
        );
        assert!(
            msg.contains("Events, mutexes, and Job Objects"),
            "message missing 'Events, mutexes, and Job Objects': {msg}"
        );
    }

    #[test]
    fn unsupported_platform_message_starts_with_aipc_brokering() {
        // Substring property — preserves intent if punctuation drifts but
        // catches a contributor who shortens the message.
        let msg = unsupported_platform_message();
        assert!(
            msg.starts_with("AIPC handle brokering"),
            "message does not start with 'AIPC handle brokering': {msg}"
        );
        assert!(
            msg.ends_with("no direct Unix analog."),
            "message does not end with 'no direct Unix analog.': {msg}"
        );
    }

    #[cfg(target_os = "windows")]
    // Tests in this module mutate NONO_SESSION_TOKEN / NONO_SESSION_ID via
    // the local `with_test_session_token` save/restore helper. The
    // workspace `clippy.toml` disallows bare `std::env::set_var` /
    // `std::env::remove_var`, recommending `EnvVarGuard` from
    // `crates/nono-cli/src/test_env.rs`. That helper is not reachable from
    // the `nono` crate without introducing a circular dep. Mirrors the
    // `#[allow(clippy::disallowed_methods)]` pattern already used by
    // `crates/nono/src/keystore.rs::tests` (line 1534, same rationale).
    #[allow(clippy::disallowed_methods)]
    mod windows_loopback_tests {
        use super::*;
        use crate::capability::AccessMode;
        use crate::supervisor::types::{
            ApprovalDecision, GrantedResourceKind, ResourceGrant, ResourceTransferKind,
            SupervisorMessage, SupervisorResponse,
        };
        use std::thread;

        /// Save/restore env per CLAUDE.md "Environment variables in tests"
        /// guidance — keep the mutated window as short as possible.
        pub(super) fn with_test_session_token<F: FnOnce()>(token: &str, f: F) {
            let prev_token = std::env::var("NONO_SESSION_TOKEN").ok();
            let prev_session = std::env::var("NONO_SESSION_ID").ok();
            std::env::set_var("NONO_SESSION_TOKEN", token);
            std::env::set_var("NONO_SESSION_ID", "sdk-test-session");
            f();
            match prev_token {
                Some(v) => std::env::set_var("NONO_SESSION_TOKEN", v),
                None => std::env::remove_var("NONO_SESSION_TOKEN"),
            }
            match prev_session {
                Some(v) => std::env::set_var("NONO_SESSION_ID", v),
                None => std::env::remove_var("NONO_SESSION_ID"),
            }
        }

        #[test]
        fn request_event_returns_handle_on_granted() {
            let (mut server, mut client) = SupervisorSocket::pair().expect("pair");
            let supervisor_thread = thread::spawn(move || {
                let msg = server.recv_message().expect("recv");
                let req = match msg {
                    SupervisorMessage::Request(r) => r,
                    other => panic!("wrong variant: {other:?}"),
                };
                assert_eq!(req.kind, crate::supervisor::types::HandleKind::Event);
                server
                    .send_response(&SupervisorResponse::Decision {
                        request_id: req.request_id,
                        decision: ApprovalDecision::Granted,
                        grant: Some(ResourceGrant {
                            transfer: ResourceTransferKind::DuplicatedWindowsHandle,
                            resource_kind: GrantedResourceKind::Event,
                            access: AccessMode::ReadWrite,
                            raw_handle: Some(0xDEAD_BEEF),
                            protocol_info_blob: None,
                        }),
                    })
                    .expect("send");
            });
            with_test_session_token("testtoken12345678", || {
                let result = request_event(
                    &mut client,
                    "test-shutdown",
                    crate::supervisor::policy::EVENT_DEFAULT_MASK,
                    Some("test"),
                );
                let handle = result.expect("granted should return handle");
                assert_eq!(handle, 0xDEAD_BEEF_u64);
            });
            supervisor_thread.join().expect("supervisor thread");
        }

        #[test]
        fn request_event_propagates_denied_reason() {
            let (mut server, mut client) = SupervisorSocket::pair().expect("pair");
            let supervisor_thread = thread::spawn(move || {
                let msg = server.recv_message().expect("recv");
                let req = match msg {
                    SupervisorMessage::Request(r) => r,
                    other => panic!("wrong variant: {other:?}"),
                };
                server
                    .send_response(&SupervisorResponse::Decision {
                        request_id: req.request_id,
                        decision: ApprovalDecision::Denied {
                            reason: "test deny".to_string(),
                        },
                        grant: None,
                    })
                    .expect("send");
            });
            with_test_session_token("testtoken12345678", || {
                let result = request_event(
                    &mut client,
                    "test-name",
                    crate::supervisor::policy::EVENT_DEFAULT_MASK,
                    None,
                );
                match result {
                    Err(NonoError::SandboxInit(ref msg)) if msg.contains("test deny") => {}
                    other => panic!("expected SandboxInit containing 'test deny', got {other:?}"),
                }
            });
            supervisor_thread.join().expect("supervisor thread");
        }

        #[test]
        fn request_pipe_returns_handle_on_granted() {
            let (mut server, mut client) = SupervisorSocket::pair().expect("pair");
            let supervisor_thread = thread::spawn(move || {
                let msg = server.recv_message().expect("recv");
                let req = match msg {
                    SupervisorMessage::Request(r) => r,
                    other => panic!("wrong variant: {other:?}"),
                };
                assert_eq!(req.kind, crate::supervisor::types::HandleKind::Pipe);
                server
                    .send_response(&SupervisorResponse::Decision {
                        request_id: req.request_id,
                        decision: ApprovalDecision::Granted,
                        grant: Some(ResourceGrant {
                            transfer: ResourceTransferKind::DuplicatedWindowsHandle,
                            resource_kind: GrantedResourceKind::Pipe,
                            access: AccessMode::Read,
                            raw_handle: Some(0xCAFE_BABE),
                            protocol_info_blob: None,
                        }),
                    })
                    .expect("send");
            });
            with_test_session_token("testtoken12345678", || {
                let result = request_pipe(
                    &mut client,
                    "test-pipe-name",
                    crate::supervisor::types::PipeDirection::Read,
                    Some("test"),
                );
                let handle = result.expect("granted should return handle");
                assert_eq!(handle, 0xCAFE_BABE_u64);
            });
            supervisor_thread.join().expect("supervisor thread");
        }

        #[test]
        fn helper_stamps_session_token_from_env() {
            let (mut server, mut client) = SupervisorSocket::pair().expect("pair");
            let supervisor_thread = thread::spawn(move || {
                let msg = server.recv_message().expect("recv");
                if let SupervisorMessage::Request(req) = msg {
                    assert_eq!(
                        req.session_token, "testtoken12345678abc",
                        "SDK must stamp NONO_SESSION_TOKEN into CapabilityRequest.session_token"
                    );
                    server
                        .send_response(&SupervisorResponse::Decision {
                            request_id: req.request_id,
                            decision: ApprovalDecision::Denied {
                                reason: "test".to_string(),
                            },
                            grant: None,
                        })
                        .expect("send");
                } else {
                    panic!("wrong variant");
                }
            });
            with_test_session_token("testtoken12345678abc", || {
                // Result is ignored — we only care that the supervisor side
                // saw the right session_token stamped on the request.
                let _ = request_event(
                    &mut client,
                    "test-name",
                    crate::supervisor::policy::EVENT_DEFAULT_MASK,
                    None,
                );
            });
            supervisor_thread.join().expect("supervisor thread");
        }
    }

    /// Windows-only smoke tests that round-trip the SDK against the REAL
    /// Plan 18-01..18-03 broker pipeline via `BrokerTargetProcess::current()`.
    ///
    /// These tests create a real source kernel object, run the actual
    /// `broker_*_to_process` function from `socket_windows.rs`, respond on
    /// the loopback `SupervisorSocket::pair()`, call the SDK method, assert
    /// the returned handle is valid, and close the duplicated handle.
    ///
    /// **Scope lock (see T-18-04-01 disposition):** these tests cover the
    /// SDK ↔ broker wire-format alignment ONLY. The supervisor-side policy
    /// gates (discriminator validation D-03, per-type mask validation D-07,
    /// constant-time token check Phase 11 D-01, name canonicalization,
    /// CONIN$ approval prompt) are covered by
    /// `crates/nono-cli/src/exec_strategy_windows/supervisor.rs`'s
    /// `capability_handler_tests` family (Plans 18-01..18-03) and the
    /// standalone integration suite in
    /// `crates/nono-cli/tests/aipc_handle_brokering_integration.rs`.
    /// Do NOT add "what if the mask is invalid?" tests here — those
    /// belong in the dispatcher test suite.
    ///
    /// The Job Object test creates a fresh Job (NOT the supervisor's
    /// containment Job) so Plan 18-03's CompareObjectHandles runtime
    /// guard is NOT exercised here — that guard is tested by the
    /// containment-Job hijack test in `capability_handler_tests`.
    #[cfg(target_os = "windows")]
    #[allow(clippy::disallowed_methods)]
    mod windows_real_broker_smoke_tests {
        use super::*;
        use crate::supervisor::socket::{
            bind_aipc_pipe, broker_event_to_process, broker_job_object_to_process,
            broker_mutex_to_process, broker_pipe_to_process, broker_socket_to_process,
            BrokerTargetProcess,
        };
        use crate::supervisor::types::{
            ApprovalDecision, PipeDirection, SocketRole, SupervisorMessage, SupervisorResponse,
        };
        use std::thread;
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};

        // Re-use the env-save/restore helper from the sibling module.
        use super::windows_loopback_tests::with_test_session_token;

        /// Transport a raw Windows `HANDLE` (`*mut c_void`) across thread
        /// boundaries by casting to `usize` (which is `Send`) and casting
        /// back inside the spawned thread.
        ///
        /// Cleaner than a `SendableHandle` newtype because rustc's closure-
        /// capture inference treats `source_sendable.0` as a raw pointer
        /// value within the closure body; the pointer-to-usize round-trip
        /// bypasses that inference while preserving the HANDLE's bit pattern
        /// (per the Win32 contract that HANDLE values are kernel-object
        /// indices representable as pointer-sized integers).
        #[inline]
        fn handle_as_usize(h: HANDLE) -> usize {
            h as usize
        }
        #[inline]
        fn usize_as_handle(n: usize) -> HANDLE {
            n as HANDLE
        }

        /// Convert a Rust `&str` to a UTF-16 null-terminated `Vec<u16>` for
        /// Windows W-suffixed APIs.
        fn wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0)).collect()
        }

        #[test]
        fn sdk_request_event_round_trips_through_real_broker() {
            use windows_sys::Win32::System::Threading::CreateEventW;
            let event_name = wide(&format!(
                r"Local\nono-aipc-sdk-smoke-event-{}",
                std::process::id()
            ));
            // SAFETY: CreateEventW with NULL attributes, manual-reset = TRUE,
            // initial-state = FALSE, and a unique name returns either a valid
            // event HANDLE or NULL on failure. We assert non-null below.
            let source: HANDLE =
                unsafe { CreateEventW(std::ptr::null_mut(), 1, 0, event_name.as_ptr()) };
            assert!(
                !source.is_null(),
                "CreateEventW failed: {}",
                std::io::Error::last_os_error()
            );

            let (mut server, mut client) = SupervisorSocket::pair().expect("pair");
            let source_usize = handle_as_usize(source);

            let supervisor_thread = thread::spawn(move || {
                let msg = server.recv_message().expect("recv");
                let req = match msg {
                    SupervisorMessage::Request(r) => r,
                    other => panic!("wrong variant: {other:?}"),
                };
                let source = usize_as_handle(source_usize);
                let target = BrokerTargetProcess::current();
                let grant = broker_event_to_process(
                    source,
                    target,
                    crate::supervisor::policy::EVENT_DEFAULT_MASK,
                )
                .expect("broker");
                server
                    .send_response(&SupervisorResponse::Decision {
                        request_id: req.request_id,
                        decision: ApprovalDecision::Granted,
                        grant: Some(grant),
                    })
                    .expect("send");
                // SAFETY: `source` is a live HANDLE; the broker does NOT
                // close it (CONTEXT.md D-10 — caller closes after
                // send_response returns). CloseHandle on a valid HANDLE is
                // safe.
                let _ = unsafe { CloseHandle(source) };
            });

            with_test_session_token("smokesdktoken1234", || {
                let result = request_event(
                    &mut client,
                    "smoke-test-event",
                    crate::supervisor::policy::EVENT_DEFAULT_MASK,
                    Some("real broker smoke"),
                );
                let dup_handle = result.expect("granted");
                assert_ne!(dup_handle, 0, "duplicated handle should be non-null");
                // SAFETY: `dup_handle` is a duplicated HANDLE owned by this
                // process (BrokerTargetProcess::current() = this process).
                // CloseHandle on a valid HANDLE is safe.
                let _ = unsafe { CloseHandle(dup_handle as HANDLE) };
            });

            supervisor_thread.join().expect("supervisor thread");
        }

        #[test]
        fn sdk_request_mutex_round_trips_through_real_broker() {
            use windows_sys::Win32::System::Threading::CreateMutexW;
            let mutex_name = wide(&format!(
                r"Local\nono-aipc-sdk-smoke-mutex-{}",
                std::process::id()
            ));
            // SAFETY: CreateMutexW with NULL attributes, initial-owner = FALSE,
            // and a unique name returns either a valid HANDLE or NULL.
            let source: HANDLE =
                unsafe { CreateMutexW(std::ptr::null_mut(), 0, mutex_name.as_ptr()) };
            assert!(
                !source.is_null(),
                "CreateMutexW failed: {}",
                std::io::Error::last_os_error()
            );

            let (mut server, mut client) = SupervisorSocket::pair().expect("pair");
            let source_usize = handle_as_usize(source);

            let supervisor_thread = thread::spawn(move || {
                let msg = server.recv_message().expect("recv");
                let req = match msg {
                    SupervisorMessage::Request(r) => r,
                    other => panic!("wrong variant: {other:?}"),
                };
                let source = usize_as_handle(source_usize);
                let target = BrokerTargetProcess::current();
                let grant = broker_mutex_to_process(
                    source,
                    target,
                    crate::supervisor::policy::MUTEX_DEFAULT_MASK,
                )
                .expect("broker");
                server
                    .send_response(&SupervisorResponse::Decision {
                        request_id: req.request_id,
                        decision: ApprovalDecision::Granted,
                        grant: Some(grant),
                    })
                    .expect("send");
                // SAFETY: `source` is a live HANDLE; broker does NOT close it.
                let _ = unsafe { CloseHandle(source) };
            });

            with_test_session_token("smokesdktoken1234", || {
                let result = request_mutex(
                    &mut client,
                    "smoke-test-mutex",
                    crate::supervisor::policy::MUTEX_DEFAULT_MASK,
                    Some("real broker smoke"),
                );
                let dup_handle = result.expect("granted");
                assert_ne!(dup_handle, 0, "duplicated handle should be non-null");
                // SAFETY: `dup_handle` is a live HANDLE owned by this process.
                let _ = unsafe { CloseHandle(dup_handle as HANDLE) };
            });

            supervisor_thread.join().expect("supervisor thread");
        }

        #[test]
        fn sdk_request_pipe_round_trips_through_real_broker() {
            let canonical = format!(r"\\.\pipe\nono-aipc-sdk-smoke-pipe-{}", std::process::id());
            let source = bind_aipc_pipe(&canonical, PipeDirection::Read).expect("bind_aipc_pipe");

            let (mut server, mut client) = SupervisorSocket::pair().expect("pair");
            let source_usize = handle_as_usize(source);

            let supervisor_thread = thread::spawn(move || {
                let msg = server.recv_message().expect("recv");
                let req = match msg {
                    SupervisorMessage::Request(r) => r,
                    other => panic!("wrong variant: {other:?}"),
                };
                let source = usize_as_handle(source_usize);
                let target = BrokerTargetProcess::current();
                let grant =
                    broker_pipe_to_process(source, target, PipeDirection::Read).expect("broker");
                server
                    .send_response(&SupervisorResponse::Decision {
                        request_id: req.request_id,
                        decision: ApprovalDecision::Granted,
                        grant: Some(grant),
                    })
                    .expect("send");
                // SAFETY: `source` is a live HANDLE; broker does NOT close it.
                let _ = unsafe { CloseHandle(source) };
            });

            with_test_session_token("smokesdktoken1234", || {
                let result = request_pipe(
                    &mut client,
                    "smoke-test-pipe",
                    PipeDirection::Read,
                    Some("real broker smoke"),
                );
                let dup_handle = result.expect("granted");
                assert_ne!(dup_handle, 0, "duplicated handle should be non-null");
                // SAFETY: `dup_handle` is a live HANDLE owned by this process.
                let _ = unsafe { CloseHandle(dup_handle as HANDLE) };
            });

            supervisor_thread.join().expect("supervisor thread");
        }

        #[test]
        fn sdk_request_job_object_round_trips_through_real_broker() {
            use windows_sys::Win32::System::JobObjects::CreateJobObjectW;
            let job_name = wide(&format!(
                r"Local\nono-aipc-sdk-smoke-job-{}",
                std::process::id()
            ));
            // SAFETY: CreateJobObjectW with NULL attributes + a unique name
            // returns either a valid HANDLE or NULL on failure. This creates a
            // FRESH Job Object (not the containment Job), so Plan 18-03's
            // CompareObjectHandles runtime guard does NOT fire here.
            let source: HANDLE =
                unsafe { CreateJobObjectW(std::ptr::null_mut(), job_name.as_ptr()) };
            assert!(
                !source.is_null(),
                "CreateJobObjectW failed: {}",
                std::io::Error::last_os_error()
            );

            let (mut server, mut client) = SupervisorSocket::pair().expect("pair");
            let source_usize = handle_as_usize(source);

            let supervisor_thread = thread::spawn(move || {
                let msg = server.recv_message().expect("recv");
                let req = match msg {
                    SupervisorMessage::Request(r) => r,
                    other => panic!("wrong variant: {other:?}"),
                };
                let source = usize_as_handle(source_usize);
                let target = BrokerTargetProcess::current();
                let grant = broker_job_object_to_process(
                    source,
                    target,
                    crate::supervisor::policy::JOB_OBJECT_DEFAULT_MASK,
                )
                .expect("broker");
                server
                    .send_response(&SupervisorResponse::Decision {
                        request_id: req.request_id,
                        decision: ApprovalDecision::Granted,
                        grant: Some(grant),
                    })
                    .expect("send");
                // SAFETY: `source` is a live HANDLE; broker does NOT close it.
                let _ = unsafe { CloseHandle(source) };
            });

            with_test_session_token("smokesdktoken1234", || {
                let result = request_job_object(
                    &mut client,
                    "smoke-test-job",
                    crate::supervisor::policy::JOB_OBJECT_DEFAULT_MASK,
                    Some("real broker smoke"),
                );
                let dup_handle = result.expect("granted");
                assert_ne!(dup_handle, 0, "duplicated handle should be non-null");
                // SAFETY: `dup_handle` is a live HANDLE owned by this process.
                let _ = unsafe { CloseHandle(dup_handle as HANDLE) };
            });

            supervisor_thread.join().expect("supervisor thread");
        }

        #[test]
        fn sdk_request_socket_round_trips_through_real_broker() {
            use windows_sys::Win32::Networking::WinSock::{
                closesocket, WSASocketW, WSAStartup, AF_INET, INVALID_SOCKET, IPPROTO_TCP,
                SOCK_STREAM, WSADATA, WSA_FLAG_OVERLAPPED,
            };

            // SAFETY: WSAStartup with version 2.2 (0x0202) is reference-
            // counted + idempotent per the Winsock API contract.
            let mut wsa: WSADATA = unsafe { std::mem::zeroed() };
            let _ = unsafe { WSAStartup(0x0202, &mut wsa) };

            // SAFETY: WSASocketW with NULL protocol_info creates a fresh
            // source socket. AF_INET / SOCK_STREAM / IPPROTO_TCP are
            // well-defined Winsock constants; WSA_FLAG_OVERLAPPED is the
            // standard flag for brokered sockets.
            let source = unsafe {
                WSASocketW(
                    AF_INET as i32,
                    SOCK_STREAM,
                    IPPROTO_TCP,
                    std::ptr::null(),
                    0,
                    WSA_FLAG_OVERLAPPED,
                )
            };
            assert_ne!(source, INVALID_SOCKET, "WSASocketW must succeed");

            let (mut server, mut client) = SupervisorSocket::pair().expect("pair");

            let supervisor_thread = thread::spawn(move || {
                let msg = server.recv_message().expect("recv");
                let req = match msg {
                    SupervisorMessage::Request(r) => r,
                    other => panic!("wrong variant: {other:?}"),
                };
                let target = BrokerTargetProcess::current();
                let grant = broker_socket_to_process(
                    source,
                    target,
                    std::process::id(),
                    SocketRole::Connect,
                )
                .expect("broker");
                server
                    .send_response(&SupervisorResponse::Decision {
                        request_id: req.request_id,
                        decision: ApprovalDecision::Granted,
                        grant: Some(grant),
                    })
                    .expect("send");
                // SAFETY: closesocket is a leaf-safe Winsock call on a live
                // SOCKET returned by WSASocketW above.
                let _ = unsafe { closesocket(source) };
            });

            with_test_session_token("smokesdktoken1234", || {
                let result = request_socket(
                    &mut client,
                    "127.0.0.1",
                    12345,
                    SocketProtocol::Tcp,
                    SocketRole::Connect,
                    0,
                    Some("real broker smoke"),
                );
                let reconstructed = result.expect("granted should return RawSocket");
                assert_ne!(reconstructed, 0, "reconstructed SOCKET should be non-zero");
                assert_ne!(
                    reconstructed, INVALID_SOCKET as u64 as RawSocket,
                    "reconstructed SOCKET should not be INVALID_SOCKET"
                );
                // SAFETY: `reconstructed` is a live SOCKET owned by this
                // process (BrokerTargetProcess::current() = this process so
                // WSASocketW(FROM_PROTOCOL_INFO) succeeded against this PID).
                let _ = unsafe { closesocket(reconstructed as usize) };
            });

            supervisor_thread.join().expect("supervisor thread");
        }
    }
}
