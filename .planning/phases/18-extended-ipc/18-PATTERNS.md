---
phase: 18-extended-ipc
status: pattern-mapped
mapped: 2026-04-19
---

# Phase 18: Extended IPC (AIPC-01) — Pattern Map

**Mapped:** 2026-04-19
**Files analyzed:** 11 (8 source, 1 data, 1 schema, 1 new integration test)
**Analogs found:** 11 / 11 (100% coverage — Phase 11 wiring is the direct prior art for almost every file)

## File Classification

| New / Modified File | Role | Data Flow | Closest Analog | Match Quality |
|--------------------|------|-----------|----------------|---------------|
| `crates/nono/src/supervisor/types.rs` | wire-protocol type | request-response (struct definitions) | self (existing `CapabilityRequest`, `ResourceGrant`, `GrantedResourceKind` blocks at lines 18-111) | exact — extending the same file |
| `crates/nono/src/supervisor/socket_windows.rs` (5 broker fns) | broker (FFI) | supervisor → child grant (`DuplicateHandle` / `WSADuplicateSocketW`) | `broker_file_handle_to_process` lines 269-302 | exact (same shape, swap mask) |
| `crates/nono/src/supervisor/policy.rs` (NEW) | policy / pure data | none (constants + validator) | `crates/nono-cli/src/policy.rs` (group resolver — naming sibling) and `crates/nono-cli/src/exec_strategy_windows/mod.rs:401-402` (existing `JOB_OBJECT_*` constants) | role-match (similar pure-data + validator shape) |
| `crates/nono/src/supervisor/mod.rs` | module root + SDK methods | child → supervisor request | self (existing `pub use socket::{...}` at lines 37-43; `ApprovalBackend` trait at line 86) | exact (extending re-exports + trait surface) |
| `crates/nono/src/error.rs` | error enum | none (read-only confirm) | self lines 39-40 (`UnsupportedPlatform(String)` already exists) | exact — no change needed, just confirm |
| `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` | dispatcher | supervisor server message handler | self — `handle_windows_supervisor_message` lines 1195-1296 (Phase 11 dispatch) | exact (extend in place) |
| `crates/nono-cli/src/terminal_approval.rs` | approval / UX | supervisor → user prompt | self — `request_capability` impl lines 26-88 (Phase 11 prompt template + sanitizer) | exact (extend in place) |
| `crates/nono-cli/src/profile/mod.rs` | profile / config parser | JSON load → struct | self — `PolicyPatchConfig` lines 60-88 (`#[serde(default, deny_unknown_fields)]` nested-struct precedent); `parse_profile_file` line 1224; `validate_custom_credential` lines 259-317 (token-rejection precedent) | exact (mirror existing nested-config pattern) |
| `crates/nono-cli/data/policy.json` | data / config | JSON | self — claude-code entry lines 651-707 (existing built-in profile shape) | exact (extend in place) |
| `crates/nono-cli/data/nono-profile.schema.json` | data / JSON schema | JSON | self — top-level `properties.filesystem` line 32 + `$ref` indirection pattern | exact (add new sibling property + `$defs` entry) |
| `crates/nono-cli/tests/aipc_handle_brokering_integration.rs` (NEW) | integration test | Windows-only round-trip | `crates/nono-cli/tests/wfp_port_integration.rs` (Windows-only `#![cfg(target_os = "windows")]` test scaffolding) | role-match (same Windows-only integration shape; admin-skip pattern not needed for AIPC since current-process brokering works without elevation) |

---

## Pattern Assignments

### `crates/nono/src/supervisor/socket_windows.rs` — five new broker functions (broker, supervisor → child grant)

**Analog:** `broker_file_handle_to_process` (this file, lines 269-302).

**Imports already present** (this file, lines 1-31):
```rust
use crate::error::{NonoError, Result};
use crate::supervisor::types::{ResourceGrant, SupervisorMessage, SupervisorResponse};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows_sys::Win32::Foundation::{
    DuplicateHandle, /* ... */ DUPLICATE_SAME_ACCESS, /* ... */
    GENERIC_READ, GENERIC_WRITE, HANDLE, /* ... */
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;
```
**Imports to add:** `WSADuplicateSocketW`, `WSAPROTOCOL_INFOW`, `SOCKET` from `windows_sys::Win32::Networking::WinSock` — gated behind a new `Win32_Networking_WinSock` feature flag in `crates/nono/Cargo.toml`.

**Core broker pattern to mirror** (lines 269-302) — copy this shape per new broker:
```rust
/// Duplicate an opened file handle into the target process and describe it in
/// the supervisor response contract.
pub fn broker_file_handle_to_process(
    file: &File,
    target_process: BrokerTargetProcess,
    access: crate::capability::AccessMode,
) -> Result<ResourceGrant> {
    let mut duplicated: HANDLE = std::ptr::null_mut();
    let source_handle = file.as_raw_handle() as HANDLE;

    // SAFETY: `source_handle` comes from a live `File`, the current process is
    // the source process, `target_process` was wrapped by the caller from a
    // live process handle, and `duplicated` points to writable storage.
    let ok = unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            source_handle,
            target_process.raw(),
            &mut duplicated,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    };
    if ok == 0 || duplicated.is_null() {
        return Err(NonoError::SandboxInit(format!(
            "Failed to duplicate Windows handle into supervised child: {}",
            std::io::Error::last_os_error()
        )));
    }

    Ok(ResourceGrant::duplicated_windows_file_handle(
        duplicated as usize as u64,
        access,
    ))
}
```

**Per-type deltas:**
| Broker | Mask arg | Source-handle origin | `dwOptions` |
|--------|----------|----------------------|-------------|
| `broker_pipe_to_process` | `match direction { Read => GENERIC_READ, Write => GENERIC_WRITE, ReadWrite => GENERIC_READ \| GENERIC_WRITE }` | server-created via `CreateNamedPipeW` (use existing `bind_low_integrity` SDDL `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;LW)` from line 39) | **`0` — NOT `DUPLICATE_SAME_ACCESS`** (supervisor's source has full PIPE_ACCESS_DUPLEX; must map down) |
| `broker_job_object_to_process` | caller-provided `mask: u32` (pre-validated against allowlist) | supervisor's pre-opened Job Object handle (NOT the containment Job — guard separately) | `0` |
| `broker_event_to_process` | caller-provided `mask: u32` (default `0x0010_0002`) | `CreateEventW(NULL, FALSE, FALSE, name)` where `name = format!("Local\\nono-aipc-{}-{}", user_session_id, raw_name)` | `0` |
| `broker_mutex_to_process` | caller-provided `mask: u32` (default `0x0010_0001`) | `CreateMutexW(NULL, FALSE, name)` same naming | `0` |
| `broker_socket_to_process` | N/A — uses `WSADuplicateSocketW(socket, target_pid, &mut proto_info)` and returns the 372-byte `WSAPROTOCOL_INFOW` blob via the new `ResourceTransferKind::SocketProtocolInfoBlob` variant | supervisor opened via `WSASocketW(...)` then `WSAConnect`/`bind`/`listen` per validated role | N/A — different FFI |

**Existing test to mirror per new broker** (this file, lines 949-971):
```rust
#[test]
fn test_broker_file_handle_to_process_duplicates_handle() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("broker.txt");
    std::fs::write(&path, b"hello").expect("write file");
    let file = File::open(&path).expect("open file");

    let grant =
        broker_file_handle_to_process(&file, BrokerTargetProcess::current(), AccessMode::Read)
            .expect("duplicate handle into current process");
    assert_eq!(
        grant.transfer,
        ResourceTransferKind::DuplicatedWindowsHandle
    );
    let raw_handle = grant.raw_handle.expect("raw handle");
    assert_ne!(raw_handle, 0);

    unsafe {
        // SAFETY: The duplicated handle value came from `DuplicateHandle`
        // above and has not been wrapped or closed yet.
        windows_sys::Win32::Foundation::CloseHandle(raw_handle as usize as HANDLE);
    }
}
```
Per-type tests use `BrokerTargetProcess::current()` so duplication targets the test process itself; close the duplicated handle with `CloseHandle` (or `closesocket` for sockets) at end of test to avoid handle leak.

---

### `crates/nono/src/supervisor/types.rs` — extend `CapabilityRequest`, add new enums (wire-protocol type)

**Analog:** Existing `CapabilityRequest` struct (this file, lines 18-39) plus `ResourceGrant` (lines 73-111).

**Existing struct shape to extend** (lines 18-39):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRequest {
    pub request_id: String,
    pub path: PathBuf,
    pub access: AccessMode,
    pub reason: Option<String>,
    pub child_pid: u32,
    pub session_id: String,
    /// 32-byte hex session token (NONO_SESSION_TOKEN).
    /// Never log this field; always redact before embedding in [`AuditEntry`].
    #[serde(default)]
    pub session_token: String,
}
```
**Add per RESEARCH lines 339-419:** `kind: HandleKind` (with `#[serde(default = "HandleKind::file")]`), `target: Option<HandleTarget>` (with `#[serde(default)]`), `access_mask: u32` (with `#[serde(default)]`). Mark existing `path` `#[deprecated(note = "use HandleTarget::FilePath via the new kind/target fields")]` but keep type as `PathBuf` (per RESEARCH Open Question #1 / Assumption A2 — avoids 8-12 file rewrites of Phase 11 callers).

**Existing constructor pattern to mirror for 5 new ResourceGrant constructors** (this file, lines 87-111):
```rust
impl ResourceGrant {
    #[must_use]
    pub fn sideband_file_descriptor(access: AccessMode) -> Self {
        Self {
            transfer: ResourceTransferKind::SidebandFileDescriptor,
            resource_kind: GrantedResourceKind::File,
            access,
            raw_handle: None,
        }
    }

    #[must_use]
    pub fn duplicated_windows_file_handle(raw_handle: u64, access: AccessMode) -> Self {
        Self {
            transfer: ResourceTransferKind::DuplicatedWindowsHandle,
            resource_kind: GrantedResourceKind::File,
            access,
            raw_handle: Some(raw_handle),
        }
    }
}
```
**Add five sibling constructors:** `duplicated_windows_pipe_handle`, `_job_object_handle`, `_event_handle`, `_mutex_handle`, plus `socket_protocol_info_blob(bytes: Vec<u8>, role: SocketRole)` — the last requires extending `ResourceGrant` with `protocol_info_blob: Option<Vec<u8>>` and `ResourceTransferKind::SocketProtocolInfoBlob`.

**`GrantedResourceKind` extension** — current shape (lines 57-61) is single-variant `File`; add `Socket | Pipe | JobObject | Event | Mutex` keeping `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]`.

---

### `crates/nono/src/supervisor/policy.rs` (NEW FILE) — per-type allowlist constants + validator (policy / pure data)

**Closest analog (naming sibling):** `crates/nono-cli/src/policy.rs` — group resolver, pure-data + validator shape. Different concern but same role (cross-cutting policy primitive).

**Closest analog (constants precedent):** `crates/nono-cli/src/exec_strategy_windows/mod.rs` lines 401-402:
```rust
pub(crate) const JOB_OBJECT_QUERY: u32 = 0x0004;
pub(crate) const JOB_OBJECT_TERMINATE: u32 = 0x0008;
```
The new `policy.rs` consolidates these and adds the rest. Promote visibility from `pub(crate)` to `pub` (cross-platform, library-public, used by the CLI dispatcher).

**Module body pattern** (per RESEARCH lines 261-282 + 422-468) — pure data + tiny validator:
```rust
//! Per-handle-type access-mask allowlists for AIPC-01 (Phase 18).
use crate::supervisor::types::{HandleKind, PipeDirection, SocketRole};

// Standard / generic / per-type Win32 mask constants (verified against
// learn.microsoft.com — see RESEARCH.md § Per-Type Access-Mask Allowlist Constants)
pub const SYNCHRONIZE: u32           = 0x0010_0000;
pub const GENERIC_READ: u32          = 0x8000_0000;
pub const GENERIC_WRITE: u32         = 0x4000_0000;
pub const JOB_OBJECT_QUERY: u32      = 0x0004;
pub const EVENT_MODIFY_STATE: u32    = 0x0002;
pub const MUTEX_MODIFY_STATE: u32    = 0x0001;  // Microsoft: "Reserved for future use"
                                                // Documented for forward-compat symmetry.

// Per-CONTEXT.md D-05 hard-coded defaults
pub const JOB_OBJECT_DEFAULT_MASK: u32 = JOB_OBJECT_QUERY;                  // 0x0004
pub const EVENT_DEFAULT_MASK: u32      = SYNCHRONIZE | EVENT_MODIFY_STATE;  // 0x0010_0002
pub const MUTEX_DEFAULT_MASK: u32      = SYNCHRONIZE | MUTEX_MODIFY_STATE;  // 0x0010_0001
pub const PRIVILEGED_PORT_MAX: u16     = 1023;

/// Validate a requested mask against the resolved per-type allowlist.
/// `resolved` = hard-coded default ∪ profile widening.
pub fn mask_is_allowed(_kind: HandleKind, requested: u32, resolved: u32) -> bool {
    requested & !resolved == 0   // requested must be a subset of resolved
}
```

**Cross-platform (compiles on Linux/macOS so unit tests run in CI).** `_kind` reserved for forward-compat per-kind dispatch; current implementation is a single bitmask subset check.

---

### `crates/nono/src/supervisor/mod.rs` — re-exports + 5 SDK methods (module root + SDK)

**Analog:** Existing re-export block (lines 30-43) and `ApprovalBackend` trait surface (lines 86-100):
```rust
#[cfg(not(target_os = "windows"))]
pub mod socket;
#[cfg(target_os = "windows")]
#[path = "socket_windows.rs"]
pub mod socket;
pub mod types;

#[cfg(target_os = "windows")]
pub use socket::BrokerTargetProcess;
pub use socket::SupervisorSocket;
pub use types::{
    ApprovalDecision, AuditEntry, CapabilityRequest, GrantedResourceKind, ResourceGrant,
    ResourceTransferKind, SupervisorMessage, SupervisorResponse, UrlOpenRequest,
};
```

**Re-exports to add:**
```rust
pub mod policy;  // cross-platform new module
pub use types::{HandleKind, HandleTarget, SocketProtocol, SocketRole, PipeDirection};
#[cfg(target_os = "windows")]
pub use socket::{
    broker_socket_to_process, broker_pipe_to_process, broker_job_object_to_process,
    broker_event_to_process, broker_mutex_to_process,
};
```

**SDK request method pattern (D-08 + D-09)** per RESEARCH lines 633-682 — five new `request_*` functions, each split into Windows / non-Windows arms:
```rust
#[cfg(target_os = "windows")]
pub fn request_event(
    name: &str, access_mask: u32, reason: Option<&str>,
) -> Result<std::os::windows::raw::HANDLE> { /* connect to NONO_SUPERVISOR_PIPE,
    send CapabilityRequest with kind=Event + target=EventName, recv response */ }

#[cfg(not(target_os = "windows"))]
pub fn request_event(
    _name: &str, _access_mask: u32, _reason: Option<&str>,
) -> Result<u64 /* placeholder */> {
    Err(NonoError::UnsupportedPlatform(
        "AIPC handle brokering is Windows-only on v2.1; \
         Unix has SCM_RIGHTS file-descriptor passing as the natural \
         equivalent (separate cross-platform requirement, future \
         milestone)".to_string()
    ))
}
```
**Note:** The variant is `NonoError::UnsupportedPlatform`, NOT `PlatformNotSupported` (CONTEXT.md D-09 typo corrected — see error.rs:39-40).

---

### `crates/nono/src/error.rs` (READ-ONLY confirm)

**Existing variant** (lines 39-40 — already present, no change required):
```rust
#[error("Platform not supported: {0}")]
UnsupportedPlatform(String),
```
Plan must reference this exact variant name. Reusing avoids needless churn.

---

### `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — extend dispatcher (dispatcher, supervisor → child)

**Analog:** Existing `handle_windows_supervisor_message` (this file, lines 1195-1296). Phase 11 dispatch flow:
1. Replay-detect (lines 1207-1222) — preserved unchanged, covers all 6 kinds via shared dispatch.
2. **Constant-time session-token check** (lines 1227-1245) — preserved unchanged.
3. **NEW (D-03): constant-time discriminator validation** — insert here, immediately after token check, before backend dispatch.
4. Approval backend call (lines 1247-1251) — preserved unchanged; backend opaque to kind.
5. **MODIFY: replace single-path `if decision.is_granted() { broker_file_handle_to_process(...) }` (lines 1253-1262) with `match request.kind { ... }`** dispatching to the appropriate per-type broker.
6. Audit emission (lines 1264-1269) — preserved unchanged.

**Existing constant-time helper to reuse** (this file, lines 1160-1166):
```rust
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    use subtle::ConstantTimeEq;
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}
```
**Discriminator validation pattern to add** (per RESEARCH lines 695-711) — uses the same `subtle::ConstantTimeEq` primitive:
```rust
// Constant-time discriminator validation — Phase 18 D-03.
// Even though the discriminator carries no secret, we use the same
// subtle::ConstantTimeEq primitive that validates the session token
// (Phase 11 D-01) so the audit chain is structurally identical for
// both untrusted bytes.
let kind_byte = [request.kind.discriminator_byte()];
let known_kinds: &[u8] = &[0, 1, 2, 3, 4, 5];
let kind_ok = known_kinds.iter().any(|&k| {
    use subtle::ConstantTimeEq;
    bool::from([k].ct_eq(&kind_byte))
});
if !kind_ok {
    let decision = nono::ApprovalDecision::Denied {
        reason: "unknown handle type".to_string(),
    };
    audit_log.push(audit_entry_with_redacted_token(
        &request, &decision, approval_backend.backend_name(), started_at,
    ));
    return sock.send_response(&nono::supervisor::SupervisorResponse::Decision {
        request_id: request.request_id, decision, grant: None,
    });
}
```

**Existing audit redactor to reuse** (this file, lines 1168-1186):
```rust
fn audit_entry_with_redacted_token(
    request: &nono::CapabilityRequest,
    decision: &nono::ApprovalDecision,
    backend_name: &str,
    started_at: Instant,
) -> AuditEntry {
    let mut redacted = request.clone();
    redacted.session_token.clear();
    AuditEntry {
        timestamp: SystemTime::now(),
        request: redacted,
        decision: decision.clone(),
        backend: backend_name.to_string(),
        duration_ms: started_at.elapsed().as_millis() as u64,
    }
}
```
This already covers all 6 kinds (request shape carries `kind` + `target`); audit-entry shape itself doesn't change (D-11). Token-redaction tests must be extended to all 6 kinds.

**Match-arm replacement for the broker call** (replaces lines 1253-1262):
```rust
let grant = if decision.is_granted() {
    Some(match request.kind {
        HandleKind::File => {
            let file = open_windows_supervisor_path(&request.path, &request.access)?;
            nono::supervisor::socket::broker_file_handle_to_process(
                &file, target_process, request.access,
            )?
        }
        HandleKind::Event => {
            // 1. Validate target shape matches kind (server-side; client untrusted)
            // 2. Validate access_mask via policy::mask_is_allowed(...)
            // 3. CreateEventW with name = format!("Local\\nono-aipc-{}-{}",
            //    runtime.user_session_id, raw_name)  ← MUST be user_session_id
            // 4. broker_event_to_process(handle, target_process, mask)
            // 5. Drop OwnedHandle (CloseHandle on supervisor source — D-10)
            todo!("Plan 18-01")
        }
        HandleKind::Mutex => todo!("Plan 18-01"),
        HandleKind::Pipe | HandleKind::Socket => todo!("Plan 18-02"),
        HandleKind::JobObject => todo!("Plan 18-03"),
    })
} else { None };
```

**Existing token-leak test to mirror per new HandleKind** (this file, lines 1450-1479):
```rust
#[test]
fn handle_redacts_token_in_audit_entry_json() {
    let backend = CountingDenyBackend::new();
    let (mut supervisor, mut child) = new_pair();
    let mut seen = HashSet::new();
    let mut audit_log = Vec::new();

    let sensitive_token = "super-secret-token-value-do-not-log";
    let request = make_request(sensitive_token);
    handle_windows_supervisor_message(
        &mut supervisor,
        nono::supervisor::SupervisorMessage::Request(request),
        &backend,
        nono::BrokerTargetProcess::current(),
        &mut seen,
        &mut audit_log,
        sensitive_token,
    )
    .expect("handle");

    assert_eq!(audit_log.len(), 1);
    assert_eq!(audit_log[0].request.session_token, "");
    let json = serde_json::to_string(&audit_log[0]).expect("serialize");
    assert!(
        !json.contains(sensitive_token),
        "audit JSON must not contain the raw session token: {json}"
    );

    let _ = child.recv_response().expect("drain");
}
```
Parameterize `make_request` (lines 1337-1347) over `HandleKind` + `HandleTarget` — produce 5 sibling tests covering Socket/Pipe/JobObject/Event/Mutex per D-11.

---

### `crates/nono-cli/src/terminal_approval.rs` — add `format_capability_prompt` (approval / UX)

**Analog:** Existing `request_capability` impl (this file, lines 26-88). Phase 11 prompt template:
```rust
eprintln!();
eprintln!("[nono] The sandboxed process is requesting additional access:");
eprintln!(
    "[nono]   Path:   {}",
    sanitize_for_terminal(&request.path.display().to_string())
);
eprintln!("[nono]   Access: {}", format_access_mode(&request.access));
if let Some(ref reason) = request.reason {
    eprintln!("[nono]   Reason: {}", sanitize_for_terminal(reason));
}
eprintln!("[nono]");
eprint!("[nono] Grant access? [y/N] ");
```

**Existing sanitizer to reuse for ALL untrusted target string fields** (this file, lines 104-144) — `sanitize_for_terminal` handles ANSI CSI/OSC/DCS/APC/PM/SOS escapes plus all control bytes. Already platform-agnostic; covered by 12+ existing tests.

**New helper signature** (per D-04):
```rust
pub(crate) fn format_capability_prompt(
    kind: HandleKind,
    target: &HandleTarget,
    access_mask: u32,
    reason: Option<&str>,
) -> String {
    // Single template per kind matching D-04:
    //   File:       "[nono] Grant file access? path=<x> access=<...> reason=\"<r>\" [y/N]"
    //   Socket:     "[nono] Grant socket access? proto=<...> host=<h> port=<p> role=<...> reason=\"<r>\" [y/N]"
    //   Pipe:       "[nono] Grant pipe access? name=<n> direction=<...> reason=\"<r>\" [y/N]"
    //   Job Object: "[nono] Grant Job Object access? name=<n> access=<...> reason=\"<r>\" [y/N]"
    //   Event:      "[nono] Grant event access? name=<n> access=<...> reason=\"<r>\" [y/N]"
    //   Mutex:      "[nono] Grant mutex access? name=<n> access=<...> reason=\"<r>\" [y/N]"
    // Every untrusted field (host, name, reason) MUST go through sanitize_for_terminal()
}
```

**CONIN$ branch unchanged** (this file, lines 60-71) — Phase 11 D-04 lock; reuse as-is:
```rust
#[cfg(target_os = "windows")]
let tty = match std::fs::File::open(r"\\.\CONIN$") {
    Ok(f) => f,
    Err(e) => {
        tracing::warn!("TerminalApproval: no console available for interactive approval: {e}");
        return Ok(ApprovalDecision::Denied {
            reason: "No console available for interactive approval".to_string(),
        });
    }
};
```

**Sanitizer test pattern to mirror** (this file, lines 187-203, 281-285) — add `prompt_sanitizes_untrusted_target_strings` test that injects ANSI escapes into `host`, `name`, `reason` and asserts they're stripped from the formatted prompt.

---

### `crates/nono-cli/src/profile/mod.rs` — add `CapabilitiesConfig` + `AipcConfig` (profile / config parser)

**Analog (nested-struct shape):** `PolicyPatchConfig` (this file, lines 60-88):
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyPatchConfig {
    #[serde(default)]
    pub exclude_groups: Vec<String>,
    #[serde(default)]
    pub add_allow_read: Vec<String>,
    // ... etc
}
```
Mirror this exact derive set (`#[derive(Debug, Clone, Default, Serialize, Deserialize)] #[serde(deny_unknown_fields)]`) and `#[serde(default)]` on each sub-field for `CapabilitiesConfig` and the new `AipcConfig`.

**Profile struct extension** (this file, lines 947-992) — add new field next to existing nested configs:
```rust
#[derive(Debug, Clone, Default, Serialize)]
pub struct Profile {
    // ... existing fields ...
    #[serde(default)]
    pub policy: PolicyPatchConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    // ADD HERE — mirror nested-config nesting:
    #[serde(default)]
    pub capabilities: CapabilitiesConfig,
    // ... rest unchanged ...
}
```
And the matching field on `ProfileDeserialize` (lines 994-1028) plus the `From<ProfileDeserialize> for Profile` impl (lines 1030-1049). Three sites must be updated in lockstep (the `deny_unknown_fields` derive on `ProfileDeserialize` will fail loudly if missed).

**Validator pattern to mirror for unknown-token rejection** (this file, lines 259-317 — `validate_custom_credential`):
```rust
fn validate_custom_credential(name: &str, cred: &CustomCredentialDef) -> Result<()> {
    validate_credential_key(name, &cred.credential_key)?;
    // ... structural validation ...
    if !ev.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(NonoError::ProfileParse(format!(
            "env_var '{}' for custom credential '{}' must contain only \
             alphanumeric characters and underscores",
            ev, name
        )));
    }
    // ... mode-specific validation ...
    Ok(())
}
```
Same fail-secure pattern: `from_token(&str) -> Result<u32>` rejects unknown tokens with `NonoError::ProfileParse(...)`. Wire from `parse_profile_file` (this file, line 1224 — uses `serde_json::from_str`, NOT `toml`).

**Loader entry point** (this file, lines 1224-1240):
```rust
fn parse_profile_file(path: &Path) -> Result<Profile> {
    let content = fs::read_to_string(path).map_err(|e| NonoError::ProfileRead {
        path: path.to_path_buf(), source: e,
    })?;
    let profile: Profile =
        serde_json::from_str(&content).map_err(|e| NonoError::ProfileParse(e.to_string()))?;
    validate_profile_custom_credentials(&profile)?;
    validate_env_credential_keys(&profile)?;
    Ok(profile)
}
```
**Add a new validator call** here: `validate_profile_aipc_tokens(&profile)?;` — rejects unknown access tokens at parse time per D-06 / RESEARCH lines 314-317.

**`Profile::resolve_aipc_allowlist(&self) -> AipcResolvedAllowlist` method** — new method that combines hard-coded defaults from `nono::supervisor::policy` with the profile widening tokens. Pre-resolved at supervisor construction time and passed via `Arc<AipcResolvedAllowlist>` into the capability pipe thread closure (RESEARCH Open Question #3 — pre-resolve to keep hot path branch-free).

---

### `crates/nono-cli/data/policy.json` — add `capabilities.aipc` to 5 built-in profiles (data / config)

**Analog:** Existing `claude-code` profile entry (this file, lines 651-707):
```json
"claude-code": {
  "extends": "default",
  "meta": { "name": "claude-code", "version": "1.0.0", ... },
  "security": { "groups": [...], "signal_mode": "isolated", ... },
  "filesystem": { "allow": [...], "allow_file": [...] },
  "network": { "block": false },
  "workdir": { "access": "readwrite" },
  "open_urls": { "allow_origins": ["https://claude.ai"], ... },
  "allow_launch_services": true,
  "hooks": { ... },
  "undo": { ... },
  "interactive": true
}
```

**Insert new top-level key** alongside `network` / `workdir` / `open_urls`:
```jsonc
"capabilities": {
  "aipc": {
    "socket":     ["connect"],
    "pipe":       ["read", "write"],
    "job_object": ["query"],
    "event":      ["wait", "signal"],
    "mutex":      ["wait", "release"]
  }
}
```
Tune per profile per RESEARCH lines 474-525 (claude-code/codex/swival match; opencode adds `read+write` for pipe; openclaw is minimal — empty arrays for `job_object`/`event`/`mutex`). The default profile gets either an empty `aipc` block OR no block at all (hard-coded defaults apply either way).

---

### `crates/nono-cli/data/nono-profile.schema.json` — schema extension (data / JSON schema)

**Analog:** Existing top-level `properties.filesystem` (this file, line 32) and `$defs/FilesystemConfig` indirection:
```json
"filesystem": {
  "$ref": "#/$defs/FilesystemConfig",
  "description": "Filesystem access rules ..."
}
```

**Pattern to add** — sibling property + new `$defs` entry:
```json
"capabilities": {
  "$ref": "#/$defs/CapabilitiesConfig",
  "description": "Per-handle-type AIPC access-mask widening (Phase 18 / AIPC-01). Profile widens hard-coded supervisor defaults; default-deny applies to anything outside the resolved allowlist."
}
```
And add `$defs/CapabilitiesConfig` + `$defs/AipcConfig` with per-key enum-string arrays per RESEARCH lines 327-332:
```jsonc
"socket":     { "type": "array", "items": { "enum": ["connect", "bind", "listen"] } },
"pipe":       { "type": "array", "items": { "enum": ["read", "write", "read+write"] } },
"job_object": { "type": "array", "items": { "enum": ["query", "set_attributes", "terminate"] } },
"event":      { "type": "array", "items": { "enum": ["wait", "signal", "both"] } },
"mutex":      { "type": "array", "items": { "enum": ["wait", "release", "both"] } }
```
Each sub-array must include the "WARNING: terminate access on a Job Object brokered from the supervisor's containment Job allows the child to kill the supervisor process tree" doc-string per RESEARCH Landmines § Job Object.

---

### `crates/nono-cli/tests/aipc_handle_brokering_integration.rs` (NEW) — Windows-only integration (test)

**Analog:** `crates/nono-cli/tests/wfp_port_integration.rs` (lines 1-45 — Windows-only `#![cfg(target_os = "windows")]` test scaffolding):
```rust
#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]

use std::net::{TcpListener, TcpStream};
use std::time::Duration;

fn is_elevated() -> bool { /* ... net session check ... */ }

#[test]
#[ignore] // Requires admin privileges and a running nono-wfp-service
fn wfp_port_permit_allows_real_tcp_connection() {
    if !is_elevated() {
        eprintln!("SKIP: wfp_port_permit test requires administrator privileges");
        return;
    }
    // ... test body ...
}
```

**For AIPC tests:** elevation skip is **NOT** needed — `BrokerTargetProcess::current()` lets duplication target the test process itself, no admin required. Pattern is otherwise identical: `#![cfg(target_os = "windows")]`, per-handle-type round-trip, granted/denied paths, audit-shape assertions. Mirror the existing in-source test `test_broker_file_handle_to_process_duplicates_handle` (`socket_windows.rs:949-971`) at integration scope, end-to-end through `handle_windows_supervisor_message`.

---

## Shared Patterns

### Pattern 1: Constant-time byte comparison via `subtle::ConstantTimeEq`

**Source:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` lines 1160-1166 (Phase 11 D-01)

**Apply to:** Every site in this phase that branches on untrusted bytes — both the existing session-token check and the NEW Phase 18 D-03 discriminator check.

```rust
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    use subtle::ConstantTimeEq;
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}
```
**`subtle = "2"` already in the workspace** — no Cargo.toml change needed for the CLI; if the discriminator check moves library-side, add the dep to `crates/nono/Cargo.toml` Windows target deps.

### Pattern 2: Audit-entry construction with token redaction

**Source:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` lines 1168-1186 (Phase 11 D-03)

**Apply to:** Every dispatch arm in Phase 18 — the helper covers all 6 kinds because `request.kind` and `request.target` are now part of the `CapabilityRequest` struct that `audit_entry_with_redacted_token` already clones.

### Pattern 3: `// SAFETY:` doc on every `unsafe { ... }` block (CLAUDE.md mandate)

**Source:** `crates/nono/src/supervisor/socket_windows.rs` lines 277-279, 244-247, 311-314 (multiple examples).

**Apply to:** All 5 new broker functions. Pattern:
```rust
// SAFETY: <source-handle origin>, <target-process origin>, <output-pointer
//         validity>. <Why this call cannot UB>.
let ok = unsafe { DuplicateHandle(...) };
```

### Pattern 4: Server-side namespace prefix enforcement (CRITICAL — Phase 17 latent-bug carry-forward)

**Source:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` lines 195, 633-690 (the `user_session_id` field and its correct use site in `start_session_pipe_server`).

**Apply to:** EVERY site in `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` that constructs a pipe / `Local\` namespace name in this phase. The supervisor MUST canonicalize:
- Pipe: `format!("\\\\.\\pipe\\nono-aipc-{}-{}", runtime.user_session_id, raw_name)`
- Event/Mutex/Job Object: `format!("Local\\nono-aipc-{}-{}", runtime.user_session_id, raw_name)`

**MUST use `runtime.user_session_id`** (the user-facing 16-hex), **NOT `runtime.session_id`** (the supervisor correlation `supervised-PID-NANOS`). Phase 17 commit `7db6595` fixed three pre-existing bugs of exactly this shape (`start_logging`, `start_data_pipe_server`, `create_process_containment` job-name).

**Planner-must-grep-assert rule:** Plan acceptance criteria MUST include a grep check that returns 0 matches for `format!.*nono-aipc.*self\.session_id` (or the runtime-bound equivalent). Example:
```bash
# Phase-gate assertion in 18-XX-PLAN.md verify section
! grep -nE 'format!\([^)]*nono-aipc[^)]*self\.session_id' \
    crates/nono-cli/src/exec_strategy_windows/supervisor.rs
```

### Pattern 5: Nested-config `#[serde(default, deny_unknown_fields)]` with three-site consistency

**Source:** `crates/nono-cli/src/profile/mod.rs` `PolicyPatchConfig` (lines 60-88), `Profile` (lines 947-992), `ProfileDeserialize` (lines 994-1028), `From<ProfileDeserialize> for Profile` (lines 1030-1049).

**Apply to:** New `CapabilitiesConfig` field. Update all THREE sites in lockstep (`Profile`, `ProfileDeserialize`, `From` impl). The `#[serde(deny_unknown_fields)]` on `ProfileDeserialize` makes silent forgetting impossible at parse time.

### Pattern 6: Token validation rejecting unknown values at parse time (`NonoError::ProfileParse`)

**Source:** `crates/nono-cli/src/profile/mod.rs` lines 259-317 (`validate_custom_credential` family).

**Apply to:** `from_token` parser for AIPC access strings (`"connect"`, `"bind"`, `"query"`, `"terminate"` etc.). Unknown tokens → `NonoError::ProfileParse(...)` — fail-secure on parse error, never silent default.

---

## No Analog Found

**None.** Every new file or code site has a direct or close analog already in the codebase. Phase 11 (capability pipe + `broker_file_handle_to_process` + `TerminalApproval` + audit redaction) is load-bearing prior art for almost every change in Phase 18.

---

## Metadata

**Analog search scope:**
- `crates/nono/src/supervisor/` (types.rs, mod.rs, socket_windows.rs, error.rs)
- `crates/nono-cli/src/exec_strategy_windows/` (supervisor.rs, mod.rs)
- `crates/nono-cli/src/terminal_approval.rs`
- `crates/nono-cli/src/profile/mod.rs`
- `crates/nono-cli/src/policy.rs` (naming-sibling check for new policy.rs)
- `crates/nono-cli/data/policy.json`, `nono-profile.schema.json`
- `crates/nono-cli/tests/wfp_port_integration.rs` (Windows-only integration scaffolding)

**Files scanned:** 11 (full reads on hot files; targeted Grep + offset/limit reads on large files like `supervisor.rs` 1700+ lines, `profile/mod.rs` 4400+ lines, `policy.json` 906 lines)

**Pattern extraction date:** 2026-04-19
