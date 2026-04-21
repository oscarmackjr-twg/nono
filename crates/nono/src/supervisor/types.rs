//! Supervisor IPC types for capability expansion
//!
//! These types define the protocol between a sandboxed child process and its
//! unsandboxed supervisor parent. The child sends [`CapabilityRequest`]s over
//! a supervisor transport, and the supervisor responds with
//! [`ApprovalDecision`]s plus explicit resource-transfer metadata when a
//! request is granted.

use crate::capability::AccessMode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

/// Discriminator for the kind of OS handle being requested via the supervisor
/// capability pipe (Phase 18 AIPC-01).
///
/// Each variant has an explicit `#[repr(u8)]` discriminator pinned for
/// wire-format stability across releases. The supervisor performs a
/// constant-time comparison against the known-good set
/// `[0, 1, 2, 3, 4, 5]` before any backend dispatch (D-03).
///
/// **Wire-format stability:** the discriminator values are part of the public
/// IPC contract between the supervisor and any SDK client. Renumbering
/// breaks every shipped SDK — DO NOT renumber.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HandleKind {
    /// Filesystem file (Phase 11 path).
    File = 0,
    /// TCP/UDP socket (Phase 18-02).
    Socket = 1,
    /// Named pipe (Phase 18-02).
    Pipe = 2,
    /// Windows Job Object (Phase 18-03).
    JobObject = 3,
    /// Event kernel object (Phase 18-01).
    Event = 4,
    /// Mutex kernel object (Phase 18-01).
    Mutex = 5,
}

impl HandleKind {
    /// 1-byte discriminator wire view for constant-time comparison.
    ///
    /// The known-good set is the literal slice `[0, 1, 2, 3, 4, 5]`. The
    /// supervisor uses `subtle::ConstantTimeEq` against this byte to reject
    /// unknown variants before any backend dispatch (Phase 18 D-03).
    #[must_use]
    pub fn discriminator_byte(self) -> u8 {
        self as u8
    }

    /// Default for backward-compatible deserialize when a Phase-11-shaped
    /// `CapabilityRequest` is decoded without a `kind` field.
    #[must_use]
    pub fn file() -> Self {
        HandleKind::File
    }
}

/// Network socket protocol selector for `HandleTarget::SocketEndpoint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SocketProtocol {
    /// TCP (SOCK_STREAM).
    Tcp,
    /// UDP (SOCK_DGRAM).
    Udp,
}

/// Socket role selector for `HandleTarget::SocketEndpoint`. Validated server-
/// side at request time before any `WSADuplicateSocketW` call (D-05).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SocketRole {
    /// Outbound connection (e.g. `connect()`).
    Connect,
    /// Bind a socket to a local address (must be a non-privileged port).
    Bind,
    /// Accept inbound connections (server). Requires profile opt-in.
    Listen,
}

/// Direction of a brokered named pipe handle (read, write, or read+write).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipeDirection {
    /// Read end (child reads).
    Read,
    /// Write end (child writes).
    Write,
    /// Both ends (full-duplex).
    ReadWrite,
}

/// Type-specific descriptor for the resource being requested via
/// `CapabilityRequest`. Tagged-enum representation for serde wire stability
/// (`#[serde(tag = "type")]`).
///
/// Each variant carries ONLY name strings and structured fields; no
/// `HANDLE`-typed field exists in any variant. The wire format itself
/// prevents the confused-deputy class of attack where a child smuggles a
/// supervisor-owned HANDLE value back across the trust boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum HandleTarget {
    /// Filesystem path (Phase 11 shape; new wire view).
    FilePath {
        /// Absolute path on the host filesystem.
        path: PathBuf,
    },
    /// Network endpoint for a socket request.
    SocketEndpoint {
        /// TCP or UDP.
        protocol: SocketProtocol,
        /// Hostname or IP literal (UTF-8). Sanitized server-side.
        host: String,
        /// Port number (privileged ports `<= 1023` are denied unconditionally).
        port: u16,
        /// Role validated against per-profile allowlist server-side.
        role: SocketRole,
    },
    /// Named pipe (server canonicalizes the prefix to
    /// `\\.\pipe\nono-aipc-<user_session_id>-<name>`).
    PipeName {
        /// Caller-supplied leaf name. Server enforces 1..=64 byte length and
        /// rejects path-separator chars (`\`, `/`, `:`, NUL, control bytes).
        name: String,
    },
    /// Job Object kernel object (server canonicalizes the prefix to
    /// `Local\nono-aipc-<user_session_id>-<name>`).
    JobObjectName {
        /// Caller-supplied leaf name (same sanitization as `PipeName`).
        name: String,
    },
    /// Event kernel object (server canonicalizes the prefix to
    /// `Local\nono-aipc-<user_session_id>-<name>`).
    EventName {
        /// Caller-supplied leaf name (same sanitization as `PipeName`).
        name: String,
    },
    /// Mutex kernel object (server canonicalizes the prefix to
    /// `Local\nono-aipc-<user_session_id>-<name>`).
    MutexName {
        /// Caller-supplied leaf name (same sanitization as `PipeName`).
        name: String,
    },
}

/// A request from the sandboxed child for additional filesystem access.
///
/// Sent over the supervisor Unix socket when the child needs access to a path
/// not covered by its initial sandbox policy.
#[allow(deprecated)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRequest {
    /// Unique identifier for this request (for replay protection and audit)
    pub request_id: String,
    /// The filesystem path being requested.
    ///
    /// **Deprecated since Phase 18 (AIPC-01).** New code MUST populate
    /// `target` with `HandleTarget::FilePath { path }` instead. The field is
    /// kept typed (`PathBuf`, not `Option<PathBuf>`) to avoid an 8-12 file
    /// rewrite of every Phase 11 caller in this release; actual removal is
    /// deferred to a future phase.
    #[deprecated(note = "use HandleTarget::FilePath via the new kind/target fields")]
    pub path: PathBuf,
    /// The access mode requested (read, write, or read+write)
    pub access: AccessMode,
    /// Human-readable reason for the request (provided by the agent)
    pub reason: Option<String>,
    /// PID of the requesting child process
    pub child_pid: u32,
    /// Session identifier for correlating requests within a single run
    pub session_id: String,
    /// 32-byte hex session token (NONO_SESSION_TOKEN).
    ///
    /// Never log this field; always redact before embedding in [`AuditEntry`].
    /// On Windows this is validated in constant time by the supervisor
    /// before any approval backend is consulted (see plan 11-01).
    #[serde(default)]
    pub session_token: String,
    /// Phase 18 AIPC-01: discriminator for the kind of OS handle being
    /// requested. Defaults to `HandleKind::File` for backward compatibility
    /// with Phase 11-shaped requests that lack this field on the wire.
    #[serde(default = "HandleKind::file")]
    pub kind: HandleKind,
    /// Phase 18 AIPC-01: type-specific descriptor for the requested resource.
    /// `None` only on Phase 11-shaped requests (kind defaulted to `File`,
    /// path-based dispatch).
    #[serde(default)]
    pub target: Option<HandleTarget>,
    /// Phase 18 AIPC-01: client-declared access mask. Server-side validated
    /// against the per-handle-type allowlist via
    /// `nono::supervisor::policy::mask_is_allowed` BEFORE any handle is opened
    /// (D-07). UNTRUSTED.
    #[serde(default)]
    pub access_mask: u32,
}

/// The supervisor's response to a [`CapabilityRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalDecision {
    /// Access was granted. Resource-transfer details, if any, are carried by
    /// [`SupervisorResponse::Decision`].
    Granted,
    /// Access was denied with a reason.
    Denied {
        /// Why the request was denied
        reason: String,
    },
    /// The approval request timed out without a decision.
    Timeout,
}

/// The kind of resource the supervisor transferred for a granted request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GrantedResourceKind {
    /// A filesystem-backed file handle or descriptor.
    File,
    /// A TCP or UDP socket (Phase 18-02).
    Socket,
    /// A named-pipe handle (Phase 18-02).
    Pipe,
    /// A Job Object handle (Phase 18-03).
    JobObject,
    /// An event kernel object handle (Phase 18-01).
    Event,
    /// A mutex kernel object handle (Phase 18-01).
    Mutex,
}

/// The transport mechanism used to deliver a granted resource to the child.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceTransferKind {
    /// The resource arrives out-of-band via Unix `SCM_RIGHTS`.
    SidebandFileDescriptor,
    /// The resource is an already-duplicated Windows handle carried inline.
    DuplicatedWindowsHandle,
    /// The resource is a Winsock `WSAPROTOCOL_INFOW` blob (single-use,
    /// target-PID-bound) for socket re-creation in the child via
    /// `WSASocketW`. Used by `HandleKind::Socket` brokerage in Plan 18-02.
    SocketProtocolInfoBlob,
}

/// Metadata describing how a granted resource reaches the sandboxed child.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceGrant {
    /// Which transfer mechanism the child must expect.
    pub transfer: ResourceTransferKind,
    /// What kind of resource was granted.
    pub resource_kind: GrantedResourceKind,
    /// Which access mode the supervisor opened or brokered.
    pub access: AccessMode,
    /// Raw Windows handle value when `transfer` is
    /// [`ResourceTransferKind::DuplicatedWindowsHandle`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_handle: Option<u64>,
    /// Serialized `WSAPROTOCOL_INFOW` blob when `transfer` is
    /// [`ResourceTransferKind::SocketProtocolInfoBlob`]. Wired in Plan 18-02;
    /// reserved here so the wire format does not need a second extension.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_info_blob: Option<Vec<u8>>,
}

impl ResourceGrant {
    /// Build Unix-side metadata for a granted file descriptor transferred via
    /// `SCM_RIGHTS`.
    #[must_use]
    pub fn sideband_file_descriptor(access: AccessMode) -> Self {
        Self {
            transfer: ResourceTransferKind::SidebandFileDescriptor,
            resource_kind: GrantedResourceKind::File,
            access,
            raw_handle: None,
            protocol_info_blob: None,
        }
    }

    /// Build Windows-side metadata for a duplicated file handle transferred
    /// inline over the supervisor transport.
    #[must_use]
    pub fn duplicated_windows_file_handle(raw_handle: u64, access: AccessMode) -> Self {
        Self {
            transfer: ResourceTransferKind::DuplicatedWindowsHandle,
            resource_kind: GrantedResourceKind::File,
            access,
            raw_handle: Some(raw_handle),
            protocol_info_blob: None,
        }
    }

    /// Build Windows-side metadata for a duplicated event-kernel-object
    /// handle (Phase 18-01).
    ///
    /// `access` is set to `AccessMode::ReadWrite` as a sentinel for non-file
    /// kinds where `AccessMode` is semantically inert; the meaningful access
    /// information for events is carried by the supervisor-validated `mask`
    /// (`SYNCHRONIZE | EVENT_MODIFY_STATE` by default).
    #[must_use]
    pub fn duplicated_windows_event_handle(raw_handle: u64, _mask: u32) -> Self {
        Self {
            transfer: ResourceTransferKind::DuplicatedWindowsHandle,
            resource_kind: GrantedResourceKind::Event,
            access: AccessMode::ReadWrite,
            raw_handle: Some(raw_handle),
            protocol_info_blob: None,
        }
    }

    /// Build Windows-side metadata for a duplicated mutex-kernel-object
    /// handle (Phase 18-01).
    ///
    /// `access` is set to `AccessMode::ReadWrite` as a sentinel for non-file
    /// kinds where `AccessMode` is semantically inert; the meaningful access
    /// information for mutexes is carried by the supervisor-validated `mask`
    /// (`SYNCHRONIZE | MUTEX_MODIFY_STATE` by default).
    #[must_use]
    pub fn duplicated_windows_mutex_handle(raw_handle: u64, _mask: u32) -> Self {
        Self {
            transfer: ResourceTransferKind::DuplicatedWindowsHandle,
            resource_kind: GrantedResourceKind::Mutex,
            access: AccessMode::ReadWrite,
            raw_handle: Some(raw_handle),
            protocol_info_blob: None,
        }
    }

    /// Build Windows-side metadata for a duplicated Job Object handle
    /// (Phase 18-03).
    ///
    /// `access` is set to `AccessMode::ReadWrite` as a sentinel for non-file
    /// kinds where `AccessMode` is semantically inert; the meaningful access
    /// information for Job Objects is carried by the supervisor-validated
    /// `mask` (`JOB_OBJECT_QUERY` by default per CONTEXT.md D-05).
    ///
    /// Per CONTEXT.md D-05 footnote runtime guard: callers MUST refuse to
    /// broker the supervisor's own `containment_job` HANDLE regardless of
    /// `mask`. The structural guard lives in `handle_job_object_request`
    /// (CompareObjectHandles); this constructor trusts that the caller has
    /// already passed that gate.
    #[must_use]
    pub fn duplicated_windows_job_object_handle(raw_handle: u64, _mask: u32) -> Self {
        Self {
            transfer: ResourceTransferKind::DuplicatedWindowsHandle,
            resource_kind: GrantedResourceKind::JobObject,
            access: AccessMode::ReadWrite,
            raw_handle: Some(raw_handle),
            protocol_info_blob: None,
        }
    }

    /// Build Windows-side metadata for a duplicated named-pipe handle
    /// transferred inline (Phase 18-02).
    ///
    /// The `direction` is encoded in the access mask passed to
    /// `DuplicateHandle` server-side; here it's stored as
    /// `AccessMode::Read` / `AccessMode::Write` / `AccessMode::ReadWrite`
    /// for human-readability in the audit log.
    #[must_use]
    pub fn duplicated_windows_pipe_handle(raw_handle: u64, direction: PipeDirection) -> Self {
        let access = match direction {
            PipeDirection::Read => AccessMode::Read,
            PipeDirection::Write => AccessMode::Write,
            PipeDirection::ReadWrite => AccessMode::ReadWrite,
        };
        Self {
            transfer: ResourceTransferKind::DuplicatedWindowsHandle,
            resource_kind: GrantedResourceKind::Pipe,
            access,
            raw_handle: Some(raw_handle),
            protocol_info_blob: None,
        }
    }

    /// Build Windows-side metadata for a socket transferred via
    /// `WSADuplicateSocketW` (Phase 18-02).
    ///
    /// The `protocol_info_blob` is the ~372-byte `WSAPROTOCOL_INFOW` struct
    /// serialized as bytes; the child reconstructs the SOCKET via
    /// `WSASocketW(FROM_PROTOCOL_INFO, FROM_PROTOCOL_INFO,
    /// FROM_PROTOCOL_INFO, &proto_info, 0, WSA_FLAG_OVERLAPPED)`. Single-use;
    /// bound to a specific target PID at duplication time (CONTEXT.md
    /// `<specifics>` + RESEARCH Landmines § Socket).
    ///
    /// `access` is set to `AccessMode::ReadWrite` as a sentinel for non-file
    /// kinds where `AccessMode` is semantically inert; the meaningful access
    /// information for sockets is the role recorded in
    /// `CapabilityRequest.target` (the audit log carries it via the original
    /// request).
    #[must_use]
    pub fn socket_protocol_info_blob(bytes: Vec<u8>, role: SocketRole) -> Self {
        // The `role` parameter is currently informational only — kept in the
        // signature so future audit-trail enrichment can carry per-role data
        // without changing the constructor signature. The `let _ = role;`
        // placates clippy without an `#[allow(unused)]` attribute.
        let _ = role;
        Self {
            transfer: ResourceTransferKind::SocketProtocolInfoBlob,
            resource_kind: GrantedResourceKind::Socket,
            access: AccessMode::ReadWrite,
            raw_handle: None,
            protocol_info_blob: Some(bytes),
        }
    }
}

impl ApprovalDecision {
    /// Returns true if access was granted.
    #[must_use]
    pub fn is_granted(&self) -> bool {
        matches!(self, ApprovalDecision::Granted)
    }

    /// Returns true if access was denied.
    #[must_use]
    pub fn is_denied(&self) -> bool {
        matches!(self, ApprovalDecision::Denied { .. })
    }
}

/// A structured audit record for every approval decision.
///
/// Every capability request produces an audit entry regardless of outcome.
/// These entries support fleet-level monitoring and compliance reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// When the decision was made
    pub timestamp: SystemTime,
    /// The original request
    pub request: CapabilityRequest,
    /// The decision that was reached
    pub decision: ApprovalDecision,
    /// Which approval backend handled the request
    pub backend: String,
    /// How long the decision took (milliseconds)
    pub duration_ms: u64,
}

/// A request from the sandboxed child to open a URL in the user's browser.
///
/// Sent over the supervisor Unix socket when the child needs to launch a
/// browser (e.g., for OAuth2 login). The unsandboxed supervisor validates
/// the URL against the profile's allowed origins and opens it outside the
/// sandbox, where the browser can access its own config files freely.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlOpenRequest {
    /// Unique identifier for this request (for replay protection and audit)
    pub request_id: String,
    /// The URL to open in the user's browser
    pub url: String,
    /// PID of the requesting child process
    pub child_pid: u32,
    /// Session identifier for correlating requests within a single run
    pub session_id: String,
}

/// IPC message envelope sent from child to supervisor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SupervisorMessage {
    /// A capability expansion request (explicit, from SDK clients)
    Request(CapabilityRequest),
    /// A request to open a URL in the user's browser (e.g., OAuth2 login)
    OpenUrl(UrlOpenRequest),
    /// A request to terminate the supervisor and its child (Windows only)
    Terminate {
        /// Session identifier for verification
        session_id: String,
    },
    /// A request to detach the CLI from the supervisor (Windows only)
    Detach {
        /// Session identifier for verification
        session_id: String,
    },
}

/// IPC message envelope sent from supervisor to child.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SupervisorResponse {
    /// Response to a capability request
    Decision {
        /// The request_id this responds to
        request_id: String,
        /// The approval decision
        decision: ApprovalDecision,
        /// Resource-transfer metadata when the supervisor granted access.
        grant: Option<ResourceGrant>,
    },
    /// Response to a URL open request
    UrlOpened {
        /// The request_id this responds to
        request_id: String,
        /// Whether the URL was opened successfully
        success: bool,
        /// Error message if the open failed
        error: Option<String>,
    },
}

#[cfg(test)]
#[allow(deprecated)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn base_request(token: &str) -> CapabilityRequest {
        CapabilityRequest {
            request_id: "type-test-001".to_string(),
            path: PathBuf::from("/tmp/legacy"),
            access: AccessMode::Read,
            reason: Some("type test".to_string()),
            child_pid: 1234,
            session_id: "sess-type-test".to_string(),
            session_token: token.to_string(),
            kind: HandleKind::File,
            target: None,
            access_mask: 0,
        }
    }

    #[test]
    fn handle_kind_discriminator_bytes_stable() {
        // Wire-format stability lock — DO NOT renumber. These are part of
        // the public IPC contract. Test pinned at File=0..Mutex=5 per
        // CONTEXT.md D-01.
        assert_eq!(HandleKind::File as u8, 0);
        assert_eq!(HandleKind::Socket as u8, 1);
        assert_eq!(HandleKind::Pipe as u8, 2);
        assert_eq!(HandleKind::JobObject as u8, 3);
        assert_eq!(HandleKind::Event as u8, 4);
        assert_eq!(HandleKind::Mutex as u8, 5);
        assert_eq!(HandleKind::File.discriminator_byte(), 0);
        assert_eq!(HandleKind::Mutex.discriminator_byte(), 5);
    }

    #[test]
    fn capability_request_json_round_trip_with_target() {
        let cases: Vec<(HandleKind, HandleTarget)> = vec![
            (
                HandleKind::File,
                HandleTarget::FilePath {
                    path: PathBuf::from("/tmp/x"),
                },
            ),
            (
                HandleKind::Socket,
                HandleTarget::SocketEndpoint {
                    protocol: SocketProtocol::Tcp,
                    host: "example.com".to_string(),
                    port: 8080,
                    role: SocketRole::Connect,
                },
            ),
            (
                HandleKind::Pipe,
                HandleTarget::PipeName {
                    name: "test-pipe".to_string(),
                },
            ),
            (
                HandleKind::JobObject,
                HandleTarget::JobObjectName {
                    name: "test-job".to_string(),
                },
            ),
            (
                HandleKind::Event,
                HandleTarget::EventName {
                    name: "test-event".to_string(),
                },
            ),
            (
                HandleKind::Mutex,
                HandleTarget::MutexName {
                    name: "test-mutex".to_string(),
                },
            ),
        ];
        for (kind, target) in cases {
            let mut req = base_request("tok");
            req.kind = kind;
            req.target = Some(target.clone());
            req.access_mask = 0x0010_0002;
            let json = serde_json::to_string(&req).expect("serialize");
            let decoded: CapabilityRequest = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded.kind, kind, "kind round-trips for {kind:?}");
            assert_eq!(
                decoded.target,
                Some(target),
                "target round-trips for {kind:?}"
            );
            assert_eq!(decoded.access_mask, 0x0010_0002);
            assert_eq!(decoded.session_token, "tok");
        }
    }

    #[test]
    fn phase11_request_deserializes_as_file_kind() {
        // A Phase-11-shaped JSON request without `kind`, `target`, or
        // `access_mask` must deserialize successfully with the new fields
        // defaulted: kind = File, target = None, access_mask = 0.
        let phase11_json = r#"{
            "request_id": "p11-001",
            "path": "/tmp/legacy.txt",
            "access": "Read",
            "reason": "phase 11 client",
            "child_pid": 999,
            "session_id": "sess-p11",
            "session_token": "abcdef0123456789"
        }"#;
        let decoded: CapabilityRequest =
            serde_json::from_str(phase11_json).expect("phase11 deserialize");
        assert_eq!(decoded.kind, HandleKind::File);
        assert!(decoded.target.is_none());
        assert_eq!(decoded.access_mask, 0);
        assert_eq!(decoded.session_token, "abcdef0123456789");
        assert_eq!(decoded.path, PathBuf::from("/tmp/legacy.txt"));
    }

    #[test]
    fn resource_grant_event_constructor_shape() {
        let grant = ResourceGrant::duplicated_windows_event_handle(0xDEAD_BEEF, 0x0010_0002);
        assert_eq!(
            grant.transfer,
            ResourceTransferKind::DuplicatedWindowsHandle
        );
        assert_eq!(grant.resource_kind, GrantedResourceKind::Event);
        assert_eq!(grant.raw_handle, Some(0xDEAD_BEEF));
        assert!(grant.protocol_info_blob.is_none());
    }

    #[test]
    fn resource_grant_mutex_constructor_shape() {
        let grant = ResourceGrant::duplicated_windows_mutex_handle(0xCAFE_F00D, 0x0010_0001);
        assert_eq!(
            grant.transfer,
            ResourceTransferKind::DuplicatedWindowsHandle
        );
        assert_eq!(grant.resource_kind, GrantedResourceKind::Mutex);
        assert_eq!(grant.raw_handle, Some(0xCAFE_F00D));
        assert!(grant.protocol_info_blob.is_none());
    }

    #[test]
    fn socket_protocol_info_blob_variant_round_trips() {
        let kind = ResourceTransferKind::SocketProtocolInfoBlob;
        let json = serde_json::to_string(&kind).expect("serialize");
        let decoded: ResourceTransferKind = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, ResourceTransferKind::SocketProtocolInfoBlob);
        assert!(json.contains("SocketProtocolInfoBlob"));
    }

    // Phase 18 AIPC-01 Plan 18-02 Task 1 — pipe + socket constructor unit tests.

    #[test]
    fn resource_grant_pipe_constructor_shape_read() {
        let grant = ResourceGrant::duplicated_windows_pipe_handle(0xAABB_CCDD, PipeDirection::Read);
        assert_eq!(
            grant.transfer,
            ResourceTransferKind::DuplicatedWindowsHandle
        );
        assert_eq!(grant.resource_kind, GrantedResourceKind::Pipe);
        assert_eq!(grant.access, AccessMode::Read);
        assert_eq!(grant.raw_handle, Some(0xAABB_CCDD));
        assert!(grant.protocol_info_blob.is_none());
    }

    #[test]
    fn resource_grant_pipe_constructor_shape_readwrite() {
        let grant =
            ResourceGrant::duplicated_windows_pipe_handle(0x1234_5678, PipeDirection::ReadWrite);
        assert_eq!(
            grant.transfer,
            ResourceTransferKind::DuplicatedWindowsHandle
        );
        assert_eq!(grant.resource_kind, GrantedResourceKind::Pipe);
        assert_eq!(grant.access, AccessMode::ReadWrite);
        assert_eq!(grant.raw_handle, Some(0x1234_5678));
        assert!(grant.protocol_info_blob.is_none());
    }

    #[test]
    fn resource_grant_socket_protocol_info_blob_shape() {
        let bytes = vec![0xAAu8; 372];
        let grant = ResourceGrant::socket_protocol_info_blob(bytes.clone(), SocketRole::Connect);
        assert_eq!(grant.transfer, ResourceTransferKind::SocketProtocolInfoBlob);
        assert_eq!(grant.resource_kind, GrantedResourceKind::Socket);
        assert!(grant.raw_handle.is_none());
        assert_eq!(grant.protocol_info_blob.as_deref(), Some(bytes.as_slice()));
    }

    #[test]
    fn resource_grant_socket_blob_json_round_trip_preserves_bytes() {
        let bytes = vec![0xAAu8; 372];
        let grant = ResourceGrant::socket_protocol_info_blob(bytes.clone(), SocketRole::Connect);
        let json = serde_json::to_string(&grant).expect("serialize");
        let decoded: ResourceGrant = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(
            decoded.transfer,
            ResourceTransferKind::SocketProtocolInfoBlob
        );
        assert_eq!(decoded.resource_kind, GrantedResourceKind::Socket);
        assert_eq!(
            decoded.protocol_info_blob.as_deref(),
            Some(bytes.as_slice())
        );
        assert!(decoded.raw_handle.is_none());
    }
}
