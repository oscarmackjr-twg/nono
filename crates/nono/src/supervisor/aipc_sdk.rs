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
    // TDD RED stub — will be replaced with the CONTEXT.md D-09 text in GREEN.
    "aipc_sdk placeholder message — RED stub"
}

// -------------------------------------------------------------------------
// request_socket
// -------------------------------------------------------------------------

/// Request a brokered Windows `SOCKET` from the supervisor. Returns the raw
/// socket reconstructed from the supervisor-provided `WSAPROTOCOL_INFOW`
/// blob.
#[cfg(target_os = "windows")]
pub fn request_socket(
    _cap_pipe: &mut SupervisorSocket,
    _host: &str,
    _port: u16,
    _protocol: SocketProtocol,
    _role: SocketRole,
    _access_mask: u32,
    _reason: Option<&str>,
) -> Result<RawSocket> {
    // TDD RED stub — real implementation lands in GREEN.
    Err(NonoError::SandboxInit(
        "aipc_sdk::request_socket not implemented (RED stub)".to_string(),
    ))
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
#[cfg(target_os = "windows")]
pub fn request_pipe(
    _cap_pipe: &mut SupervisorSocket,
    _name: &str,
    _direction: PipeDirection,
    _reason: Option<&str>,
) -> Result<RawHandle> {
    Err(NonoError::SandboxInit(
        "aipc_sdk::request_pipe not implemented (RED stub)".to_string(),
    ))
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
#[cfg(target_os = "windows")]
pub fn request_job_object(
    _cap_pipe: &mut SupervisorSocket,
    _name: &str,
    _access_mask: u32,
    _reason: Option<&str>,
) -> Result<RawHandle> {
    Err(NonoError::SandboxInit(
        "aipc_sdk::request_job_object not implemented (RED stub)".to_string(),
    ))
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
#[cfg(target_os = "windows")]
pub fn request_event(
    _cap_pipe: &mut SupervisorSocket,
    _name: &str,
    _access_mask: u32,
    _reason: Option<&str>,
) -> Result<RawHandle> {
    Err(NonoError::SandboxInit(
        "aipc_sdk::request_event not implemented (RED stub)".to_string(),
    ))
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
#[cfg(target_os = "windows")]
pub fn request_mutex(
    _cap_pipe: &mut SupervisorSocket,
    _name: &str,
    _access_mask: u32,
    _reason: Option<&str>,
) -> Result<RawHandle> {
    Err(NonoError::SandboxInit(
        "aipc_sdk::request_mutex not implemented (RED stub)".to_string(),
    ))
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
}
