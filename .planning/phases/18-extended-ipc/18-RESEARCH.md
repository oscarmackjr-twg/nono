# Phase 18: Extended IPC (AIPC-01) — Research

**Researched:** 2026-04-19
**Domain:** Windows handle brokering (sockets, pipes, Job Objects, events, mutexes) over the existing Phase 11 capability pipe
**Confidence:** HIGH (Win32 mask values + WSADuplicateSocket pattern verified against learn.microsoft.com; Phase 11 / 17 wiring verified by reading current source on disk)

## Summary

CONTEXT.md D-01..D-11 lock the architecture: tagged enum on `CapabilityRequest`, single template prompt, hard-coded defaults + profile widening, SDK-only surface, constant-time discriminator validation, child-owned handle lifetime, `AuditEntry` shape unchanged. This research surfaces the *implementation details* required to plan against those locks.

Key non-obvious findings:

1. **Profile loader is JSON, not TOML.** CONTEXT.md D-06 specifies `[capabilities.aipc]` *TOML* blocks, but the actual loader at `crates/nono-cli/src/profile/mod.rs:1230` uses `serde_json::from_str` and built-in profiles live as JSON inside `crates/nono-cli/data/policy.json`. The planner MUST treat D-06 as "JSON object `capabilities.aipc`" and update the schema at `crates/nono-cli/data/nono-profile.schema.json` accordingly.
2. **`NonoError::PlatformNotSupported` does not exist.** D-09 specifies returning that variant on Unix; the actual library variant is `NonoError::UnsupportedPlatform(String)` (`crates/nono/src/error.rs:39-40`). The plan must either reuse `UnsupportedPlatform(...)` or add a new variant — either is a deliberate decision the planner needs to call out.
3. **Sockets have no per-handle access mask in Winsock.** `WSADuplicateSocket` carries the socket's full state (protocol, role, binding) via `WSAPROTOCOL_INFOW`; the access-mask analog must be enforced at *broker request time* (server validates the requested protocol/role against the per-session allowlist before calling `WSADuplicateSocket`). This is a structurally different validation shape from the four kernel-object types.
4. **`MUTEX_MODIFY_STATE = 0x0001` is documented as "Reserved for future use" in the Microsoft access-rights table.** `ReleaseMutex` works against handles opened with `SYNCHRONIZE` alone in current Windows. Calling out the constant for symmetry with `EVENT_MODIFY_STATE` is fine for documentation, but the profile schema's `mutex = ["release"]` widening should map to *no extra mask bits* — the default `SYNCHRONIZE` already covers `ReleaseMutex` calls. The planner must encode this asymmetry as a comment in the policy module so a future reader doesn't "fix" it.
5. **Job Object brokering is the most dangerous of the five types.** `DuplicateHandle` on the supervisor's *own* containment Job Object (the one used for `--timeout` enforcement and process-tree termination) would let the child reach back into the supervisor's lifecycle. D-05 mitigates this with default `JOB_OBJECT_QUERY` only, but a profile that opts in to `JOB_OBJECT_TERMINATE` opens an actual privilege-escalation footgun. The plan must surface this as a profile-author warning in the schema doc-string.
6. **Phase 17 latent-bug pattern: supervisor correlation ID vs. user-facing session ID.** Three pre-existing Phase 11/15 bugs ate session-id mismatches (`7db6595` fixed `start_logging` and `start_data_pipe_server` to use `user_session_id` instead of `self.session_id`). AIPC-01 namespace prefixes (`\\.\pipe\nono-aipc-<session_id>-<name>`, `Local\nono-aipc-<session_id>-<name>`) MUST use the user-facing `user_session_id` field, not the supervisor correlation `session_id` — otherwise the child's view of the namespace and the supervisor's view diverge, and brokering silently fails on every request.

**Primary recommendation:** Plan as **3 plans** matching ROADMAP guidance, but split the per-handle-type work along the *risk* axis rather than alphabetic: Plan 18-01 = protocol skeleton + the two synchronization primitives (event, mutex) which are the safest; Plan 18-02 = pipe + socket (medium risk, larger code surface); Plan 18-03 = Job Object (highest risk, includes the supervisor's own Job Object footgun) + extended `session_token_redaction` audit suite covering all 6 `HandleKind` shapes.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Wire protocol types (`HandleKind`, `HandleTarget`) | Library (`crates/nono`) | — | Shared between supervisor and child SDK; lives next to `CapabilityRequest` in `supervisor/types.rs`. Cross-platform (compiles on Linux/macOS, brokering call sites are `#[cfg(windows)]`). |
| Per-handle-type brokers (`broker_socket_*` etc.) | Library Windows file (`crates/nono/src/supervisor/socket_windows.rs`) | — | Phase 17 D-21 invariance pattern — `socket_windows.rs` is a `#[cfg(windows)]` module in a cross-platform crate, established for `broker_file_handle_to_process`. AIPC-01 extends it. |
| Per-type access-mask allowlist + policy resolution | Library (`crates/nono/src/supervisor/policy.rs` — new file) | — | Pure data + small validator function; no Win32 calls; cross-platform compilable so unit tests can run on Linux CI. |
| Server dispatch (`handle_windows_supervisor_message` extension) | CLI (`crates/nono-cli/src/exec_strategy_windows/supervisor.rs`) | — | Already the dispatch site; AIPC-01 inserts the discriminator check + per-type broker call after the existing token check. Stays Windows-only. |
| SDK request methods (`request_socket`, `request_pipe`, …) | Library (`crates/nono/src/supervisor/`) | — | Child-side; cross-platform compile; Windows path does the real work, non-Windows path returns `NonoError::UnsupportedPlatform(...)`. |
| Per-type prompt template | CLI (`crates/nono-cli/src/terminal_approval.rs`) | — | Already owns `request_capability` + sanitizer + CONIN$ branch; AIPC-01 adds a `format_capability_prompt()` helper next to existing code. Stays in CLI for the same reason Phase 11 D-04 kept it there. |
| Profile schema extension | CLI (`crates/nono-cli/src/profile/mod.rs` + `data/nono-profile.schema.json` + `data/policy.json`) | — | Profile loader is CLI-owned; library has no notion of profiles. |

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01 Tagged enum on `CapabilityRequest`.** Add `kind: HandleKind` (enum) + `target: HandleTarget` (enum carrying type-specific fields). Existing `path: PathBuf` becomes `path: Option<PathBuf>` with `#[deprecated]`. Wire backward compat with Phase 11 IS broken (acceptable — supervisor + SDK ship together).
- **D-02 Single envelope variant.** All 5 new handle types route through one `SupervisorMessage::Request(CapabilityRequest)` dispatch site.
- **D-03 Constant-time discriminator validation.** First action after Phase 11 token check; `subtle::ConstantTimeEq` against the known set; unknown variant → immediate `Denied { reason: "unknown handle type" }`.
- **D-04 Single-template, per-type field labels** for `format_capability_prompt(kind, target, access_mask, reason)`. Single `sanitize_for_terminal()` call site. CONIN$ branch unchanged. Unix branch returns Denied per D-09.
- **D-05 Hard-coded supervisor defaults + profile widening.** Per-type defaults baked into `policy::aipc::default_allowlist()`. Socket: TCP/UDP `connect` only (≥ 1024). Pipe: read OR write (not both). Job Object: `JOB_OBJECT_QUERY` only. Event: `SYNCHRONIZE | EVENT_MODIFY_STATE`. Mutex: `SYNCHRONIZE | MUTEX_MODIFY_STATE`.
- **D-06 Profile override schema.** `[capabilities.aipc]` block in profile.toml (NOTE: see "Critical Discrepancy" below — actual format is JSON). Widening only; default-deny remains.
- **D-07 Server-side enforcement is load-bearing.** Client-declared masks are untrusted; supervisor re-validates against resolved allowlist; verbose Denied reason for debuggability.
- **D-08 SDK-only.** Five new `request_*` methods on the child SDK. Zero new CLI flags. Zero changes to `nono run`/`shell`/`wrap` arg shape.
- **D-09 Cross-platform: fail at request time, not parse time.** Non-Windows builds return `NonoError::PlatformNotSupported(...)` — actual variant per error.rs is `NonoError::UnsupportedPlatform(...)`; planner picks one.
- **D-10 Child owns the duplicated handle.** Supervisor closes its source on grant or error; no shared ownership.
- **D-11 Audit log unchanged.** `AuditEntry { timestamp, request, decision, backend, duration_ms }` reused; request now carries `kind` and `target`.

### Claude's Discretion

- Policy module location: `crates/nono/src/supervisor/policy.rs` vs `aipc.rs`. Recommendation: `policy.rs` (mirrors the cross-cutting `policy.rs` in `nono-cli/src/`).
- 5 separate broker functions vs 1 dispatcher + match. Recommendation: 5 separate functions to keep `// SAFETY:` comments tight and per-type unit tests trivial.
- `request_pipe` direction-as-arg vs two methods. Recommendation: one method, `direction: PipeDirection` enum.
- `SocketProtocol` enum vs string. Locked by D-01 to enum.
- Test scaffolding: per-type unit + integration tests, both granted and denied paths. Reuse Phase 11 `session_token_redaction` pattern.

### Deferred Ideas (OUT OF SCOPE)

- CLI pre-approval flags (`--allow-socket :8080` etc.) — v2.2 if demand.
- Sibling-to-sibling handle brokering.
- Linux/macOS handle brokering via SCM_RIGHTS — separate cross-platform requirement.
- Profile narrowing (`[capabilities.aipc.deny]`).
- Bind-to-privileged-port (port < 1024).
- `JOB_OBJECT_TERMINATE` / `JOB_OBJECT_SET_*` from default. Profile opt-in possible, footgun.
- Removal of deprecated `path: PathBuf` field.
- `ApprovalBackend::request_capability` extension to take `HandleKind` directly.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| AIPC-01 | Broker socket / named-pipe / Job Object / event / mutex handles over Phase 11 capability pipe with per-type access-mask allowlists, fail-closed unknown discriminators, server-side mask validation, extended token-leak audit | This document — covers Win32 broker patterns per type, exact mask constants, file change map, broker pseudocode, profile schema, test plan, landmines, and 3-plan decomposition. |

## Project Constraints (from CLAUDE.md)

- **No `.unwrap()` / `.expect()` outside `#[cfg(test)]`.** Enforced by `clippy::unwrap_used`. Every `unsafe { ... }` block needs a `// SAFETY:` doc comment. New broker functions must follow the established pattern in `broker_file_handle_to_process` (CITED: `crates/nono/src/supervisor/socket_windows.rs:269-302`).
- **Path security:** never use string `starts_with` on paths; canonicalize at the enforcement boundary. AIPC-01 doesn't touch filesystem paths but the same principle applies to the namespace prefix on pipe/event/mutex names — prefix MUST be enforced server-side, never client-supplied.
- **Validate environment variables before use.** AIPC-01 reuses `NONO_SESSION_TOKEN` and `NONO_SUPERVISOR_PIPE` (both validated by Phase 11); no new env var.
- **`NonoError` + `?` for propagation.** No `panic!()` in libraries except for unrecoverable bugs.
- **DCO sign-off on every commit.**
- **EnvVarGuard + lock_env for env-mutating tests.** Not relevant to AIPC-01 unless we add tests that mutate `NONO_SUPERVISOR_PIPE`; the existing capability pipe tests in `socket_windows.rs` show the pattern via `mpsc` coordination instead.

## Critical Discrepancy — Profile Format

**CONTEXT.md D-06 says TOML; the actual loader is JSON.**

- VERIFIED: `crates/nono-cli/src/profile/mod.rs:1230` calls `serde_json::from_str(&content)`.
- VERIFIED: built-in profiles live as JSON entries inside `crates/nono-cli/data/policy.json`.
- VERIFIED: schema is `crates/nono-cli/data/nono-profile.schema.json`.

**Implication for planner:** D-06's `[capabilities.aipc]` syntax must be reinterpreted as a JSON object:

```json
{
  "capabilities": {
    "aipc": {
      "socket":     ["connect", "bind", "listen"],
      "pipe":       ["read", "write", "read+write"],
      "job_object": ["query"],
      "event":      ["wait", "signal", "both"],
      "mutex":      ["wait", "release", "both"]
    }
  }
}
```

The `Profile` struct currently has no `capabilities` field — the planner needs to add a `pub capabilities: CapabilitiesConfig` field next to `pub policy: PolicyPatchConfig`, with a sub-struct `CapabilitiesConfig { aipc: Option<AipcConfig> }`. Mirror the existing `#[serde(default, deny_unknown_fields)]` pattern used by every other profile sub-config (CITED: `crates/nono-cli/src/profile/mod.rs:34-88`).

## Critical Discrepancy — Error Variant Name

**CONTEXT.md D-09 references `NonoError::PlatformNotSupported`; actual variant is `NonoError::UnsupportedPlatform(String)`.**

- VERIFIED: `crates/nono/src/error.rs:39-40` defines `#[error("Platform not supported: {0}")] UnsupportedPlatform(String)`.
- VERIFIED: grep shows `PlatformNotSupported` appears only in CONTEXT.md and the discussion log; no Rust code uses it.

**Implication:** Plan should use `NonoError::UnsupportedPlatform("AIPC handle brokering is Windows-only on v2.1; Unix has SCM_RIGHTS file-descriptor passing as the natural equivalent (separate cross-platform requirement, future milestone)".to_string())`. Adding a brand-new `PlatformNotSupported` variant for one call site adds noise; reuse is cleaner and aligns with the existing `Display` format `"Platform not supported: {0}"`.

## Implementation Approach (per handle type)

### 1. Socket — `WSADuplicateSocket` + child-side `WSASocketW`

**Source:** [WSADuplicateSocketW (winsock2.h)](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-wsaduplicatesocketw)

```cpp
int WSAAPI WSADuplicateSocketW(
  [in]  SOCKET              s,
  [in]  DWORD               dwProcessId,         // target child PID
  [out] LPWSAPROTOCOL_INFOW lpProtocolInfo);     // ~372 bytes, opaque
```

**Round-trip pattern:**

1. **Supervisor** (running unsandboxed) opens the socket via `WSASocketW(AF_INET, SOCK_STREAM, IPPROTO_TCP, NULL, 0, WSA_FLAG_OVERLAPPED)`, calls `WSAConnect`/`bind`/`listen` per the validated request role.
2. **Supervisor** allocates a `WSAPROTOCOL_INFOW` struct (≈ 372 bytes), calls `WSADuplicateSocketW(s, child_pid, &proto_info)`. The struct now contains a serialized capability the target process can claim *exactly once*.
3. **Supervisor** sends the `WSAPROTOCOL_INFOW` bytes inline in `SupervisorResponse::Decision { grant: ResourceGrant { transfer: SocketProtocolInfoBlob, ... } }` (new variant of `ResourceTransferKind`).
4. **Supervisor** calls `closesocket(s)` on its source descriptor (D-10: child owns the result). Note: per docs, the underlying socket stays alive until the *last* descriptor is closed.
5. **Child SDK** receives the blob, calls `WSASocketW(FROM_PROTOCOL_INFO, FROM_PROTOCOL_INFO, FROM_PROTOCOL_INFO, &proto_info, 0, WSA_FLAG_OVERLAPPED)` → returns a fresh `SOCKET` valid only in the child's address space.

**Caveats:**
- The blob is **single-use** by the target. Re-sending or replaying fails (Microsoft docs: "The special **WSAPROTOCOL_INFO** structure can only be used once by the target process.").
- The blob is bound to a specific *target* PID baked in at `WSADuplicateSocketW` time. Wrong-PID consumption fails.
- **Lifecycle ordering:** if the supervisor exits before the child consumes the blob, the underlying socket is leaked (no other descriptor exists). Plan: supervisor MUST hold its source descriptor open until it has *sent* the response, then close — the existing `child_process_for_broker` synchronization in Phase 11 already orders the response send before any teardown.
- **Access-mask analog:** there is no per-handle access mask in Winsock. Validation must happen at *broker request time*: `(protocol, role, port)` triple is the validation shape. Privileged ports (< 1024) explicitly excluded per CONTEXT.md `<specifics>`.
- **IFS vs non-IFS providers:** for IFS providers (typical TCP/UDP), `WSAPROTOCOL_INFOW.dwProviderReserved` may carry a kernel handle directly; non-IFS providers go through the SPI `WSPDuplicateSocket`. The plan can ignore this distinction at the application level — `WSADuplicateSocketW` handles both transparently. Only relevant if we ever add raw-socket or layered provider support, which is out of scope.

### 2. Named Pipe — `DuplicateHandle` with direction-mapped access mask

```cpp
BOOL DuplicateHandle(
  HANDLE   hSourceProcessHandle,    // GetCurrentProcess()
  HANDLE   hSourceHandle,           // supervisor's pipe-end handle
  HANDLE   hTargetProcessHandle,    // child process handle (BrokerTargetProcess)
  LPHANDLE lpTargetHandle,
  DWORD    dwDesiredAccess,         // GENERIC_READ | GENERIC_WRITE | (combined)
  BOOL     bInheritHandle,          // FALSE
  DWORD    dwOptions);              // 0 (NOT DUPLICATE_SAME_ACCESS — we MAP DOWN)
```

**Access mask mapping per `direction` request field:**

| Request direction | `dwDesiredAccess` |
|-------------------|-------------------|
| `Read` | `GENERIC_READ` (0x80000000) |
| `Write` | `GENERIC_WRITE` (0x40000000) |
| `ReadWrite` (only if profile widens) | `GENERIC_READ \| GENERIC_WRITE` |

**Key property vs file brokering:** Phase 11's existing `broker_file_handle_to_process` uses `DUPLICATE_SAME_ACCESS` (the duplicated handle inherits the source's full mask). AIPC-01 cannot do this for pipes — the supervisor's source is `PIPE_ACCESS_DUPLEX` (full read+write), so `DUPLICATE_SAME_ACCESS` would over-grant. The pipe broker MUST pass `dwOptions = 0` and an explicit `dwDesiredAccess` derived from the validated direction.

**Anonymous pipe vs named pipe:** Both are duplicable via `DuplicateHandle`. The SDK request distinguishes via the target shape — if `target = HandleTarget::PipeName(name)`, server canonicalizes to `\\.\pipe\nono-aipc-<user_session_id>-<name>` (per CONTEXT.md `<specifics>`) and opens with `CreateFileW(GENERIC_READ | GENERIC_WRITE, ...)` first. For anonymous pipes, the SDK would need a different request shape (out of scope for v2.1 — AIPC-01 only supports named pipes per the CONTEXT.md request types).

**SDDL on AIPC-brokered pipes:** Phase 11's capability pipe uses `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;LW)` (CITED: `crates/nono/src/supervisor/socket_windows.rs:39`). For AIPC-brokered pipes the supervisor creates with `CreateNamedPipeW`, the same SDDL is the safe default — Low Integrity write-up access is required for the sandboxed child to read/write. Reuse `bind_low_integrity` for pipe-end creation when the supervisor needs to materialize a pipe to broker, OR extract the SDDL conversion helper into a shared utility called from both call sites.

### 3. Job Object — `DuplicateHandle` with restricted mask

```cpp
DuplicateHandle(GetCurrentProcess(), source_job, target_process, &dup,
                JOB_OBJECT_QUERY,    // 0x0004
                FALSE, 0);
```

**Job Object access mask values** (Source: [Job Object Security and Access Rights](https://learn.microsoft.com/en-us/windows/win32/procthread/job-object-security-and-access-rights)):

| Constant | Value | Function gated |
|----------|-------|----------------|
| `JOB_OBJECT_ASSIGN_PROCESS` | `0x0001` | `AssignProcessToJobObject` |
| `JOB_OBJECT_SET_ATTRIBUTES` | `0x0002` | `SetInformationJobObject` |
| `JOB_OBJECT_QUERY` | `0x0004` | `QueryInformationJobObject`, `IsProcessInJob` |
| `JOB_OBJECT_TERMINATE` | `0x0008` | `TerminateJobObject` |
| `JOB_OBJECT_SET_SECURITY_ATTRIBUTES` | `0x0010` | (NOT supported on Vista+) |
| `JOB_OBJECT_ALL_ACCESS` | `0x1F001F` | (everything + standard rights) |

**Default-allowed per D-05:** `JOB_OBJECT_QUERY` only. The child can call `QueryInformationJobObject` and `IsProcessInJob` — useful for orchestration introspection (e.g., "am I in a Job Object? what limits apply?") without any control surface.

**Profile-opt-in widening exists for `JOB_OBJECT_TERMINATE` and `JOB_OBJECT_SET_ATTRIBUTES` but is structurally dangerous on the supervisor's own Job Object** — see Landmines § Job Object.

### 4. Event — `DuplicateHandle` with synchronization access

```cpp
DuplicateHandle(GetCurrentProcess(), source_event, target_process, &dup,
                SYNCHRONIZE | EVENT_MODIFY_STATE,    // 0x00100000 | 0x0002 = 0x00100002
                FALSE, 0);
```

**Event access mask values** (Source: [Synchronization Object Security and Access Rights](https://learn.microsoft.com/en-us/windows/win32/sync/synchronization-object-security-and-access-rights)):

| Constant | Value | Function gated |
|----------|-------|----------------|
| `SYNCHRONIZE` | `0x00100000` | `WaitForSingleObject`, `WaitForMultipleObjects` |
| `EVENT_MODIFY_STATE` | `0x0002` | `SetEvent`, `ResetEvent`, `PulseEvent` |
| `EVENT_ALL_ACCESS` | `0x1F0003` | (everything + standard rights) |

**Default-allowed per D-05:** `SYNCHRONIZE | EVENT_MODIFY_STATE = 0x00100002` — wait + signal. Covers the bidirectional lifecycle-signaling use case (supervisor signals "shutdown", child waits; child signals "ready", supervisor waits). Excludes `DELETE`, `WRITE_DAC`, `WRITE_OWNER` from `EVENT_ALL_ACCESS`.

**Supervisor creates the event via `CreateEventW(NULL, FALSE /* manual reset */, FALSE /* initial state */, name)` where `name = "Local\\nono-aipc-<user_session_id>-<request.name>"`.** `Local\` namespace scopes the kernel object to the current logon session, preventing cross-session collisions.

### 5. Mutex — `DuplicateHandle` with synchronization access

```cpp
DuplicateHandle(GetCurrentProcess(), source_mutex, target_process, &dup,
                SYNCHRONIZE | MUTEX_MODIFY_STATE,    // 0x00100000 | 0x0001 = 0x00100001
                FALSE, 0);
```

**Mutex access mask values** (Source: same as event above):

| Constant | Value | Notes |
|----------|-------|-------|
| `SYNCHRONIZE` | `0x00100000` | `WaitForSingleObject` (acquire) |
| `MUTEX_MODIFY_STATE` | `0x0001` | **"Reserved for future use"** per Microsoft. `ReleaseMutex` works against handles opened with `SYNCHRONIZE` alone. |
| `MUTEX_ALL_ACCESS` | `0x1F0001` | (everything + standard rights) |

**Important asymmetry:** Per the Microsoft synchronization docs, `MUTEX_MODIFY_STATE` is *Reserved for future use* — `ReleaseMutex` is gated only by ownership at the kernel level, not by an access right. The default `SYNCHRONIZE | MUTEX_MODIFY_STATE` mask is correct for *symmetry with `EVENT_MODIFY_STATE`* and forward-compat (if Microsoft ever activates the access right), but in current Windows the `MUTEX_MODIFY_STATE` bit is functionally a no-op. Document this in the policy module so a future reviewer doesn't strip the bit thinking it's dead code.

## File-by-File Change Map

For each file in CONTEXT.md `<canonical_refs>` § Implementation surfaces, what changes:

### `crates/nono/src/supervisor/types.rs` (MODIFIED)

- Add `#[repr(u8)] enum HandleKind { File = 0, Socket = 1, Pipe = 2, JobObject = 3, Event = 4, Mutex = 5 }` with `Serialize`/`Deserialize` derives.
- Add `#[serde(tag = "type")] enum HandleTarget { FilePath { path: PathBuf }, SocketEndpoint { protocol: SocketProtocol, host: String, port: u16, role: SocketRole }, PipeName { name: String }, JobObjectName { name: String }, EventName { name: String }, MutexName { name: String } }`.
- Add `enum SocketProtocol { Tcp, Udp }` and `enum SocketRole { Connect, Bind, Listen }` and `enum PipeDirection { Read, Write, ReadWrite }`.
- Mutate `CapabilityRequest`: add `pub kind: HandleKind` (with `#[serde(default = "HandleKind::file")]` for backward compat at deserialize), add `pub target: Option<HandleTarget>` (with `#[serde(default)]`), add `pub access_mask: u32` (with `#[serde(default)]`). Mark `pub path: PathBuf` as `#[deprecated(note = "use HandleTarget::FilePath via the new kind/target fields")]` — but DO NOT change its type in this phase per D-01 (deprecation in place; removal is a future phase). Note: the CONTEXT.md text says "becomes `path: Option<PathBuf>` for one release"; the planner needs to decide whether breaking the type *now* is acceptable. Recommendation: keep `path: PathBuf` typed as today, add a private deprecation attribute, and add the new fields alongside — minimizes blast radius on Phase 11 callers.
- Extend `enum GrantedResourceKind { File, Socket, Pipe, JobObject, Event, Mutex }`.
- Extend `enum ResourceTransferKind { SidebandFileDescriptor, DuplicatedWindowsHandle, SocketProtocolInfoBlob }` (new variant for socket case where the payload is a `WSAPROTOCOL_INFOW` blob, not a raw `HANDLE`).
- Extend `ResourceGrant` with `#[serde(skip_serializing_if = "Option::is_none")] pub protocol_info_blob: Option<Vec<u8>>` (≈ 372 bytes) for the socket transport. New helper constructors: `ResourceGrant::duplicated_windows_pipe_handle`, `_job_object_handle`, `_event_handle`, `_mutex_handle`, `socket_protocol_info_blob`.

### `crates/nono/src/supervisor/socket_windows.rs` (MODIFIED — Windows-only)

- Add `pub fn broker_socket_to_process(socket: SOCKET, target: BrokerTargetProcess, target_pid: u32, role: SocketRole) -> Result<ResourceGrant>` calling `WSADuplicateSocketW`.
- Add `pub fn broker_pipe_to_process(handle: HANDLE, target: BrokerTargetProcess, direction: PipeDirection) -> Result<ResourceGrant>` mapping `direction` to `GENERIC_READ`/`GENERIC_WRITE`/`GENERIC_READ | GENERIC_WRITE` then calling `DuplicateHandle` with `dwOptions = 0`.
- Add `pub fn broker_job_object_to_process(handle: HANDLE, target: BrokerTargetProcess, mask: u32) -> Result<ResourceGrant>` calling `DuplicateHandle` with explicit `mask`.
- Add `pub fn broker_event_to_process(handle: HANDLE, target: BrokerTargetProcess, mask: u32) -> Result<ResourceGrant>` (same pattern).
- Add `pub fn broker_mutex_to_process(handle: HANDLE, target: BrokerTargetProcess, mask: u32) -> Result<ResourceGrant>` (same pattern).
- All five MUST follow the `broker_file_handle_to_process` template (`socket_windows.rs:269-302`): explicit `// SAFETY:` comment, `DuplicateHandle` return code check, `is_null()` check on the duplicated handle, `std::io::Error::last_os_error()` on failure.
- Add a private `make_aipc_pipe_name(user_session_id: &str, raw_name: &str) -> String` and `make_aipc_kernel_object_name(user_session_id: &str, raw_name: &str) -> String` to enforce server-side namespace prefixing.

### `crates/nono/src/supervisor/policy.rs` (NEW — cross-platform)

```rust
//! Per-handle-type access-mask allowlists for AIPC-01 (Phase 18).
use crate::supervisor::types::{HandleKind, PipeDirection, SocketRole};

/// Default-allowed access masks per handle type per CONTEXT.md D-05.
/// All Win32 mask values are documented at learn.microsoft.com.
pub const SOCKET_DEFAULT_ROLES: &[SocketRole] = &[SocketRole::Connect];
pub const PIPE_DEFAULT_DIRECTIONS: &[PipeDirection] =
    &[PipeDirection::Read, PipeDirection::Write];   // not ReadWrite
pub const JOB_OBJECT_DEFAULT_MASK: u32 = 0x0004;    // JOB_OBJECT_QUERY only
pub const EVENT_DEFAULT_MASK: u32      = 0x0010_0002; // SYNCHRONIZE | EVENT_MODIFY_STATE
pub const MUTEX_DEFAULT_MASK: u32      = 0x0010_0001; // SYNCHRONIZE | MUTEX_MODIFY_STATE
pub const PRIVILEGED_PORT_MAX: u16     = 1023;       // ports <= this denied unconditionally

/// Validate a requested mask against the resolved per-type allowlist.
/// `resolved` = hard-coded default ∪ profile widening.
pub fn mask_is_allowed(kind: HandleKind, requested: u32, resolved: u32) -> bool {
    requested & !resolved == 0   // requested must be a subset of resolved
}
```

Pure data + tiny validator. Cross-platform — compiles on Linux/macOS so the `mask_is_allowed` unit tests run in CI. Add unit tests for boundary conditions (empty mask, exact match, single bit over allowlist, `0xFFFFFFFF` always denied).

### `crates/nono/src/supervisor/mod.rs` (MODIFIED)

- Re-export new types: `HandleKind`, `HandleTarget`, `SocketProtocol`, `SocketRole`, `PipeDirection`.
- Re-export new broker functions: `pub use socket::{broker_socket_to_process, broker_pipe_to_process, broker_job_object_to_process, broker_event_to_process, broker_mutex_to_process}`.
- Re-export `pub mod policy` (cross-platform).
- Add new `pub fn request_*` SDK methods (5) that send a `CapabilityRequest` over the existing `\\.\pipe\nono-cap-<session_id>` capability pipe and return the broker result. On non-Windows builds these return `NonoError::UnsupportedPlatform(...)` immediately (per D-09).
- `subtle = "2"` may need to be added to `crates/nono/Cargo.toml` if D-03's discriminator check moves library-side. CONTEXT.md text places the check on the supervisor side (CLI), so library-side `subtle` is not strictly required.

### `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` (MODIFIED)

- Extend `handle_windows_supervisor_message` (currently at line 1195): after the existing constant-time session-token check (lines 1227-1245), insert a *second* constant-time check on the `kind` discriminator byte against the known set `[0u8, 1, 2, 3, 4, 5]`. Unknown values → immediate `Denied { reason: "unknown handle type".to_string() }` and audit-emit. Implementation note: the discriminator is a single byte (per `#[repr(u8)]`); compare via `subtle::ConstantTimeEq::ct_eq` on the 1-byte slice. See "Constant-time on a u8 discriminator" below for the principled justification.
- Replace the current single-path `if decision.is_granted() { broker_file_handle_to_process(...) }` block (lines 1253-1262) with a `match request.kind { HandleKind::File => ..., HandleKind::Socket => ..., ... }`. Each arm:
  1. Validates the `target` field shape matches the kind (server-side; client-supplied target is untrusted).
  2. Validates the `access_mask` against the resolved allowlist via `policy::mask_is_allowed(kind, request.access_mask, resolved_mask)`.
  3. Calls the appropriate `open_*_for_broker(...)` helper to materialize the supervisor-side handle, then the matching `broker_*_to_process(...)` from `socket_windows.rs`.
  4. Closes the supervisor's source handle (D-10) via the existing `OwnedHandle` drop pattern after the `ResourceGrant` is sent.
- The `audit_log.push(audit_entry_with_redacted_token(...))` calls remain unchanged — `AuditEntry` shape doesn't change (D-11); the `request.kind` and `request.target` already discriminate.
- Resolved-mask computation needs the `Profile`'s `capabilities.aipc` block. The runtime currently doesn't carry the profile; the planner must either thread it through `WindowsSupervisorRuntime` (preferred — there's already a `requested_features: HashSet<String>` field for similar cross-cutting state) OR pre-resolve the per-type masks at supervisor construction time and pass a `Arc<AipcResolvedAllowlist>` to the capability pipe thread. Recommendation: pre-resolve at construction time — keeps the hot path branch-free and avoids profile parsing on every request.

### `crates/nono-cli/src/terminal_approval.rs` (MODIFIED)

- Add `pub(crate) fn format_capability_prompt(kind: HandleKind, target: &HandleTarget, access_mask: u32, reason: Option<&str>) -> String` per D-04.
- Per-type prompt strings exactly match the D-04 template. Sanitize EVERY user-supplied string field (`reason`, `host`, `name`) via existing `sanitize_for_terminal()`.
- The existing `request_capability` function needs to dispatch on `request.kind` to format the prompt before reading from CONIN$. Extract the body into a helper that takes the formatted prompt string so the test surface stays simple.
- Format `access_mask: u32` for human display via a per-kind helper:
  - Pipe: `"read"`, `"write"`, `"read+write"` (derived from direction enum, not the raw mask).
  - Job Object: `"query"`, `"set_attributes"`, `"terminate"`, etc., joined by `+`.
  - Event/Mutex: `"wait"`, `"signal"`, `"both"` (or `"wait"`, `"release"`, `"both"` for mutex).
  - Socket: doesn't have a mask — use the role + protocol + port instead.

### `crates/nono-cli/src/profile/mod.rs` (MODIFIED)

- Add `pub struct CapabilitiesConfig { #[serde(default)] pub aipc: Option<AipcConfig> }` with `#[derive(Debug, Clone, Default, Serialize, Deserialize)] #[serde(deny_unknown_fields)]`.
- Add `pub struct AipcConfig { pub socket: Vec<String>, pub pipe: Vec<String>, pub job_object: Vec<String>, pub event: Vec<String>, pub mutex: Vec<String> }` with same derives. Each `Vec<String>` entry is a token like `"connect"` / `"bind"` / `"listen"` / `"query"` / `"terminate"` etc., parsed by a `from_token(&str) -> Result<u32>` helper into the corresponding Win32 mask bit (or, for sockets, into a `SocketRole` value). Reject unknown tokens at parse time with `NonoError::ProfileParse(...)` — same fail-secure pattern as the existing custom-credential validators (CITED: `crates/nono-cli/src/profile/mod.rs:259-317`).
- Wire `capabilities: CapabilitiesConfig` into `Profile` and `ProfileDeserialize` (CITED: `crates/nono-cli/src/profile/mod.rs:947-1058`) following the existing nesting pattern of `policy: PolicyPatchConfig` etc.
- Add a `resolve_aipc_allowlist(&self) -> AipcResolvedAllowlist` method that combines hard-coded defaults from `nono::supervisor::policy` with the profile's widening tokens, producing a single struct the supervisor can pass to the capability pipe thread.

### `crates/nono-cli/data/policy.json` (MODIFIED)

Add a `"capabilities"` object to the 5 named built-in profiles (`claude-code`, `codex`, `opencode`, `openclaw`, `swival`). Sample bodies in § Profile.toml schema below.

### `crates/nono-cli/data/nono-profile.schema.json` (MODIFIED)

Add a `"capabilities"` property at the same level as `"filesystem"`, `"policy"`, `"network"`. Sub-object `"aipc"` with 5 keys (`socket`, `pipe`, `job_object`, `event`, `mutex`), each an array of enum-string values matching the per-type token sets. Per-type enum string sets:
- `socket`: `["connect", "bind", "listen"]`
- `pipe`: `["read", "write", "read+write"]`
- `job_object`: `["query", "set_attributes", "terminate"]`
- `event`: `["wait", "signal", "both"]`
- `mutex`: `["wait", "release", "both"]`

## Concrete Rust Shapes

```rust
// crates/nono/src/supervisor/types.rs

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HandleKind {
    File = 0,
    Socket = 1,
    Pipe = 2,
    JobObject = 3,
    Event = 4,
    Mutex = 5,
}

impl HandleKind {
    /// 1-byte discriminator wire view for constant-time comparison.
    /// The known-good set is the literal slice [0,1,2,3,4,5].
    pub fn discriminator_byte(self) -> u8 { self as u8 }

    /// Default for backward-compatible deserialize when an old (Phase 11)
    /// CapabilityRequest is decoded without a `kind` field.
    pub fn file() -> Self { HandleKind::File }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SocketProtocol { Tcp, Udp }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SocketRole { Connect, Bind, Listen }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipeDirection { Read, Write, ReadWrite }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HandleTarget {
    FilePath {
        path: PathBuf,
    },
    SocketEndpoint {
        protocol: SocketProtocol,
        host: String,         // sanitized server-side before use
        port: u16,            // server validates >= 1024 unconditionally
        role: SocketRole,
    },
    PipeName {
        name: String,         // server prefixes with \\.\pipe\nono-aipc-<session_id>-
    },
    JobObjectName {
        name: String,         // server prefixes with Local\nono-aipc-<session_id>-
    },
    EventName {
        name: String,         // server prefixes with Local\nono-aipc-<session_id>-
    },
    MutexName {
        name: String,         // server prefixes with Local\nono-aipc-<session_id>-
    },
}

// Mutated CapabilityRequest:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRequest {
    pub request_id: String,

    // Phase 11 field — kept for backward compat one release.
    // New code should populate target instead.
    #[deprecated(note = "use HandleTarget::FilePath via the new kind/target fields")]
    pub path: PathBuf,

    pub access: AccessMode,
    pub reason: Option<String>,
    pub child_pid: u32,
    pub session_id: String,
    #[serde(default)]
    pub session_token: String,

    // Phase 18 additions:
    #[serde(default = "HandleKind::file")]
    pub kind: HandleKind,
    #[serde(default)]
    pub target: Option<HandleTarget>,
    #[serde(default)]
    pub access_mask: u32,
}
```

## Per-Type Access-Mask Allowlist Constants

```rust
// crates/nono/src/supervisor/policy.rs
//
// All values verified against learn.microsoft.com:
//   Sync objects: /windows/win32/sync/synchronization-object-security-and-access-rights
//   Job Objects:  /windows/win32/procthread/job-object-security-and-access-rights
//   Generic:      /windows/win32/secauthz/generic-access-rights

// Standard access rights (shared)
pub const SYNCHRONIZE: u32           = 0x0010_0000;
pub const DELETE: u32                = 0x0001_0000;
pub const READ_CONTROL: u32          = 0x0002_0000;
pub const WRITE_DAC: u32             = 0x0004_0000;
pub const WRITE_OWNER: u32           = 0x0008_0000;

// Generic rights (file/pipe direction mapping)
pub const GENERIC_READ: u32          = 0x8000_0000;
pub const GENERIC_WRITE: u32         = 0x4000_0000;
pub const GENERIC_EXECUTE: u32       = 0x2000_0000;
pub const GENERIC_ALL: u32           = 0x1000_0000;

// Job Object specific
pub const JOB_OBJECT_ASSIGN_PROCESS: u32         = 0x0001;
pub const JOB_OBJECT_SET_ATTRIBUTES: u32         = 0x0002;
pub const JOB_OBJECT_QUERY: u32                  = 0x0004;
pub const JOB_OBJECT_TERMINATE: u32              = 0x0008;
pub const JOB_OBJECT_SET_SECURITY_ATTRIBUTES: u32 = 0x0010; // Vista+: not supported
pub const JOB_OBJECT_ALL_ACCESS: u32             = 0x1F_001F;

// Event specific
pub const EVENT_MODIFY_STATE: u32    = 0x0002;
pub const EVENT_ALL_ACCESS: u32      = 0x1F_0003;

// Mutex specific
// MUTEX_MODIFY_STATE is documented as "Reserved for future use" (Microsoft);
// ReleaseMutex works against handles opened with SYNCHRONIZE alone.
// We include the bit for symmetry with EVENT_MODIFY_STATE and forward-compat.
pub const MUTEX_MODIFY_STATE: u32    = 0x0001;
pub const MUTEX_ALL_ACCESS: u32      = 0x1F_0001;

// Per-CONTEXT.md D-05 hard-coded defaults:
pub const JOB_OBJECT_DEFAULT_MASK: u32 = JOB_OBJECT_QUERY;                  // 0x0004
pub const EVENT_DEFAULT_MASK: u32      = SYNCHRONIZE | EVENT_MODIFY_STATE;  // 0x0010_0002
pub const MUTEX_DEFAULT_MASK: u32      = SYNCHRONIZE | MUTEX_MODIFY_STATE;  // 0x0010_0001
```

## Profile Schema (JSON, NOT TOML — per Critical Discrepancy above)

**Tuned per built-in profile.** Each profile widens only what its actual workload needs.

```jsonc
// crates/nono-cli/data/policy.json — claude-code profile
"claude-code": {
  "extends": "default",
  "meta": { "name": "claude-code", "version": "1.0.0", ... },
  // ... existing fields ...
  "capabilities": {
    "aipc": {
      "socket":     ["connect"],                         // outbound only; matches existing `allow_origins` for OAuth
      "pipe":       ["read", "write"],                   // both directions allowed but not bidirectional ReadWrite
      "job_object": ["query"],                           // read-only orchestration introspection
      "event":      ["wait", "signal"],                  // bidirectional lifecycle
      "mutex":      ["wait", "release"]                  // cross-process coordination
    }
  }
}

// codex — same pattern, just connect-only sockets:
"codex": {
  "capabilities": { "aipc": {
    "socket": ["connect"], "pipe": ["read", "write"],
    "job_object": ["query"], "event": ["wait", "signal"],
    "mutex": ["wait", "release"]
  }}
}

// opencode — slightly broader pipes for its node IPC patterns:
"opencode": {
  "capabilities": { "aipc": {
    "socket": ["connect"], "pipe": ["read", "write", "read+write"],
    "job_object": ["query"], "event": ["wait", "signal"],
    "mutex": ["wait", "release"]
  }}
}

// openclaw — minimal; messaging gateway, no Job Object / mutex needs:
"openclaw": {
  "capabilities": { "aipc": {
    "socket": ["connect"], "pipe": ["read", "write"],
    "job_object": [], "event": [], "mutex": []
  }}
}

// swival — same as claude-code:
"swival": {
  "capabilities": { "aipc": {
    "socket": ["connect"], "pipe": ["read", "write"],
    "job_object": ["query"], "event": ["wait", "signal"],
    "mutex": ["wait", "release"]
  }}
}
```

**Default profile** (the one all five extend) gets an empty `capabilities.aipc` block — the hard-coded defaults from `policy::aipc::default_allowlist()` apply. `dev` profiles (`python-dev`, `node-dev`, `go-dev`, `rust-dev`) likely should also get conservative defaults (probably the empty block; no current dev workflow needs AIPC widening).

## Per-Type Broker Pseudocode

Each follows the structural shape of `broker_file_handle_to_process` (CITED: `crates/nono/src/supervisor/socket_windows.rs:269-302`) — explicit `// SAFETY:`, raw FFI call, return-code + null check, error path with `last_os_error()`.

### Socket

```rust
pub fn broker_socket_to_process(
    socket: SOCKET,
    target: BrokerTargetProcess,
    target_pid: u32,
    role: SocketRole,
) -> Result<ResourceGrant> {
    let _ = target;  // not consumed by WSADuplicateSocketW (uses PID, not handle)
    let mut proto_info: WSAPROTOCOL_INFOW = unsafe { std::mem::zeroed() };
    // SAFETY: `socket` is a live SOCKET created by the supervisor with WSASocketW.
    // `target_pid` is the validated child PID. `proto_info` points to writable
    // 372-byte storage. WSADuplicateSocketW serializes the socket capability into
    // proto_info; the result is single-use by the target PID.
    let rc = unsafe { WSADuplicateSocketW(socket, target_pid, &mut proto_info) };
    if rc != 0 {
        return Err(NonoError::SandboxInit(format!(
            "WSADuplicateSocketW failed (target_pid={}): {}",
            target_pid, std::io::Error::last_os_error()
        )));
    }
    // Serialize the WSAPROTOCOL_INFOW into a Vec<u8> the wire can carry.
    let bytes: Vec<u8> = unsafe {
        std::slice::from_raw_parts(
            &proto_info as *const _ as *const u8,
            std::mem::size_of::<WSAPROTOCOL_INFOW>(),
        ).to_vec()
    };
    Ok(ResourceGrant::socket_protocol_info_blob(bytes, role))
}
```

### Pipe

```rust
pub fn broker_pipe_to_process(
    handle: HANDLE,
    target: BrokerTargetProcess,
    direction: PipeDirection,
) -> Result<ResourceGrant> {
    let mask = match direction {
        PipeDirection::Read      => GENERIC_READ,
        PipeDirection::Write     => GENERIC_WRITE,
        PipeDirection::ReadWrite => GENERIC_READ | GENERIC_WRITE,
    };
    let mut dup: HANDLE = std::ptr::null_mut();
    // SAFETY: `handle` is a live pipe-end handle owned by the supervisor for
    // the duration of this call. `target.raw()` is a live process handle.
    // `dwOptions = 0` (NOT DUPLICATE_SAME_ACCESS) is critical — the supervisor's
    // source has full PIPE_ACCESS_DUPLEX, and we MUST map down to the requested
    // direction.
    let ok = unsafe {
        DuplicateHandle(GetCurrentProcess(), handle, target.raw(),
                        &mut dup, mask, 0 /* bInheritHandle */, 0 /* dwOptions */)
    };
    if ok == 0 || dup.is_null() {
        return Err(NonoError::SandboxInit(format!(
            "DuplicateHandle (pipe, mask=0x{mask:08x}) failed: {}",
            std::io::Error::last_os_error())));
    }
    Ok(ResourceGrant::duplicated_windows_pipe_handle(dup as usize as u64, direction))
}
```

### Job Object / Event / Mutex

Identical shape — pseudocode for one suffices, the others swap the `mask` argument and the `ResourceGrant` constructor:

```rust
pub fn broker_job_object_to_process(
    handle: HANDLE,
    target: BrokerTargetProcess,
    mask: u32,    // pre-validated against allowlist by the caller
) -> Result<ResourceGrant> {
    let mut dup: HANDLE = std::ptr::null_mut();
    // SAFETY: `handle` is a live Job Object handle opened by the supervisor.
    // `mask` was validated against the per-session allowlist by the caller
    // before this function was reached. `target.raw()` is a live process handle.
    let ok = unsafe {
        DuplicateHandle(GetCurrentProcess(), handle, target.raw(),
                        &mut dup, mask, 0, 0)
    };
    if ok == 0 || dup.is_null() {
        return Err(NonoError::SandboxInit(format!(
            "DuplicateHandle (Job Object, mask=0x{mask:08x}) failed: {}",
            std::io::Error::last_os_error())));
    }
    Ok(ResourceGrant::duplicated_windows_job_object_handle(
        dup as usize as u64, mask))
}
```

`broker_event_to_process` and `broker_mutex_to_process` are byte-identical except for the constructor name and the `// SAFETY:` text noting the kernel object kind.

## SDK Method Signatures

```rust
// crates/nono/src/supervisor/mod.rs (or a new submodule sdk.rs)

#[cfg(target_os = "windows")]
pub fn request_socket(
    host: &str, port: u16,
    protocol: SocketProtocol, role: SocketRole,
    reason: Option<&str>,
) -> Result<std::os::windows::raw::SOCKET> { ... }

#[cfg(not(target_os = "windows"))]
pub fn request_socket(
    _host: &str, _port: u16,
    _protocol: SocketProtocol, _role: SocketRole,
    _reason: Option<&str>,
) -> Result<u64 /* placeholder */ > {
    Err(NonoError::UnsupportedPlatform(
        "AIPC handle brokering is Windows-only on v2.1; \
         Unix has SCM_RIGHTS file-descriptor passing as the natural \
         equivalent (separate cross-platform requirement, future \
         milestone)".to_string()
    ))
}

#[cfg(target_os = "windows")]
pub fn request_pipe(
    name: &str, direction: PipeDirection, reason: Option<&str>,
) -> Result<std::os::windows::raw::HANDLE> { ... }

#[cfg(target_os = "windows")]
pub fn request_job_object(
    name: &str, access_mask: u32, reason: Option<&str>,
) -> Result<std::os::windows::raw::HANDLE> { ... }

#[cfg(target_os = "windows")]
pub fn request_event(
    name: &str, access_mask: u32, reason: Option<&str>,
) -> Result<std::os::windows::raw::HANDLE> { ... }

#[cfg(target_os = "windows")]
pub fn request_mutex(
    name: &str, access_mask: u32, reason: Option<&str>,
) -> Result<std::os::windows::raw::HANDLE> { ... }
```

Each Windows path:
1. Reads `NONO_SESSION_TOKEN` from env (validated by Phase 11 plumbing — fail-secure if missing).
2. Reads `NONO_SUPERVISOR_PIPE` rendezvous path; calls `SupervisorSocket::connect(path)`.
3. Builds a `CapabilityRequest` with `kind`, `target`, `access_mask`, `session_token`.
4. `send_message(SupervisorMessage::Request(req))` then `recv_response()`.
5. On `Decision { decision: Granted, grant: Some(...) }`, extracts the raw handle / SOCKET; on `Denied`, returns `Err(NonoError::SandboxInit(reason))`.

Non-Windows path: immediately returns `NonoError::UnsupportedPlatform(...)` per D-09. The function still exists so cross-platform Rust code compiles against the SDK without `#[cfg]` everywhere.

## Constant-Time on a u8 Discriminator — Principled Justification

CONTEXT.md D-03 commits to constant-time validation of the discriminator. The discriminator is a `u8` (0..=5) and carries no secret, so the time-leak surface is debatable. The principled answer:

- **Phase 11 D-01 already established constant-time for the session token.** Reusing the same primitive (`subtle::ConstantTimeEq`) for the discriminator is cheap (≈ 6 ns extra per request) and removes any "is this hot path branchy?" question for a future security reviewer.
- **The set of valid discriminators is fixed at 6 values.** Comparing in constant time vs early-exit (`matches! { 0..=5 => ... }`) makes zero practical difference for an attacker — they cannot extract any useful bit from the timing.
- **Forward-compat:** if AIPC-02 ever adds a 7th handle type with security-relevant discrimination (e.g., a "process token" kind), the constant-time path is already wired and won't need to be retrofitted.
- **Defense in depth principle (CLAUDE.md):** "Combine OS-level sandboxing with application-level checks" — constant-time on every untrusted byte the supervisor branches on is consistent with the project's defensive posture.

Document this reasoning in a comment above the discriminator-check call site so future reviewers don't strip it as "paranoia". Implementation:

```rust
// Constant-time discriminator validation — Phase 18 D-03.
// Even though the discriminator carries no secret, we use the same
// subtle::ConstantTimeEq primitive that validates the session token
// (Phase 11 D-01) so the audit chain is structurally identical for
// both untrusted bytes. Cost: ~6ns per request. Benefit: a future
// security review never has to wonder "is this hot path branchy?".
let kind_byte = [request.kind.discriminator_byte()];
let known_kinds: &[u8] = &[0, 1, 2, 3, 4, 5];
let kind_ok = known_kinds.iter().any(|&k| {
    use subtle::ConstantTimeEq;
    bool::from([k].ct_eq(&kind_byte))
});
if !kind_ok {
    return deny(request, "unknown handle type", ...);
}
```

## Test Plan

### Unit tests (cross-platform; live in `crates/nono` or `crates/nono-cli` as appropriate)

| File | Test | Asserts |
|------|------|---------|
| `policy.rs` | `mask_subset_validates_correctly` | `mask_is_allowed(JobObject, 0x0004, 0x0004) = true`; `mask_is_allowed(JobObject, 0x0008, 0x0004) = false` (TERMINATE not in allowlist); `mask_is_allowed(Event, 0x0010_0002, 0x0010_0002) = true`. |
| `policy.rs` | `default_masks_match_d05_lock` | Each `*_DEFAULT_MASK` constant equals its CONTEXT.md D-05 value. |
| `policy.rs` | `privileged_port_unconditionally_denied` | All `port <= 1023` denied even if profile widens to `bind`/`listen`. |
| `policy.rs` | `mutex_modify_state_documented_as_reserved` | Constant comment / doc test asserting the rationale (matches Microsoft "Reserved for future use" note). |
| `types.rs` | `handle_kind_discriminator_bytes_stable` | `HandleKind::Socket as u8 == 1`, `HandleKind::Mutex as u8 == 5` etc. — wire-format stability guard. |
| `types.rs` | `capability_request_json_round_trip_with_target` | Per-`HandleKind` round-trip; `request.target` survives ser→de. |
| `terminal_approval.rs` | `format_capability_prompt_per_kind` | 6 cases (one per `HandleKind`); verifies template matches D-04. |
| `terminal_approval.rs` | `prompt_sanitizes_untrusted_target_strings` | Inject ANSI escape into `host`, `name`, `reason`; output stripped. |
| `profile/mod.rs` | `aipc_capabilities_block_parses` | New profile JSON with `capabilities.aipc` deserializes; unknown token rejected. |
| `profile/mod.rs` | `aipc_default_inherits_when_block_absent` | Profile with no `capabilities.aipc` resolves to hard-coded defaults. |

### Per-handler integration tests (Windows-only; live in `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` `capability_handler_tests` module)

For each of `Socket`, `Pipe`, `JobObject`, `Event`, `Mutex` (5 modules / test families):

| Test | Asserts |
|------|---------|
| `handle_brokers_<kind>_with_default_mask` | Valid token + valid kind + mask-in-allowlist → `Granted`; backend invoked exactly once; audit entry has `request.kind == <kind>` and `decision = Granted`. |
| `handle_denies_<kind>_with_mask_outside_allowlist` | Same setup, but `access_mask` requests a bit outside the per-type default → `Denied { reason: "access mask 0x... not in allowlist for <kind>" }`; backend NOT invoked (mask check is before backend); audit entry recorded. |
| `handle_denies_<kind>_with_invalid_target_shape` | `kind = Socket` but `target = HandleTarget::PipeName(...)` (mismatched) → `Denied { reason: "target shape does not match kind" }`. |
| `handle_redacts_token_in_audit_for_<kind>` | Token set; serialize the AuditEntry to JSON; assert token string absent and `audit_log[0].request.session_token == ""`. (5 new tests, mirroring Phase 11's `handle_redacts_token_in_audit_entry_json` for the File kind.) |

For the actual `DuplicateHandle` round-trip (the broker call needs a real target process), reuse the existing pattern from `socket_windows.rs:test_broker_file_handle_to_process_duplicates_handle` (lines 949-971): use `BrokerTargetProcess::current()` so the duplication happens into the test process itself, then `CloseHandle` the result. This avoids spawning a child for unit-level coverage. End-to-end multi-process tests can be deferred to manual UAT or a separate integration harness.

### Token-leak audit extension (D-11)

Extend Phase 11's `session_token_redaction` test family to cover all 6 `HandleKind` shapes. Pattern is mechanical: parameterize the existing test body over `HandleKind` and `HandleTarget`, run the same "serialize AuditEntry → grep for token" assertion. Six tests total (5 new + the existing File one, possibly refactored into a single parameterized helper).

### Sampling rate (per .planning/config.json conventions)

- **Per task commit:** `cargo test -p nono --lib supervisor::policy` + `cargo test -p nono --lib supervisor::types` (unit tests, < 5s)
- **Per wave merge:** `cargo test -p nono-cli --bin nono capability_handler_tests` + `cargo test -p nono --lib supervisor` (integration, 10-30s on Windows)
- **Phase gate:** `make ci` (full clippy + fmt + tests + Windows-only subset)

## Wave 0 Gaps

- [ ] `crates/nono/src/supervisor/policy.rs` — new file with `mask_is_allowed`, `*_DEFAULT_MASK` constants, unit tests. Required before any per-type broker can validate.
- [ ] `crates/nono/src/supervisor/types.rs` test module — extend with `capability_request_json_round_trip_with_target` covering all 6 `HandleKind` values. Required as the wire-stability regression guard.
- [ ] `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — extract `audit_entry_with_redacted_token` into a parameterized helper that takes `HandleKind` so the 5 new test families can reuse it without duplicating the Phase 11 pattern.

No new test framework or dependencies needed. `subtle = "2"` and `getrandom = "0.4"` already in workspace under Windows target deps (CITED: `crates/nono-cli/Cargo.toml:92-93`). `windows-sys = "0.59"` carries the `Win32_System_JobObjects`, `Win32_System_Pipes`, `Win32_System_Threading` features via the existing `crates/nono-cli/Cargo.toml:90` declaration. The library crate may need `Win32_Networking_WinSock` added to its `windows-sys` feature list for `WSADuplicateSocketW` and `WSAPROTOCOL_INFOW`.

## Landmines

### Socket — `WSADuplicateSocket` lifecycle ordering

The `WSAPROTOCOL_INFOW` blob is bound to a *specific target PID at duplication time* and is *single-use*. If the supervisor exits between calling `WSADuplicateSocketW` and the child consuming the blob via `WSASocketW(FROM_PROTOCOL_INFO, ..., &proto_info, ...)`, the underlying socket is leaked (no other descriptor exists since D-10 requires the supervisor close its source on grant). **Mitigation:** the supervisor must hold its source `SOCKET` open until *after* `send_response()` returns, then `closesocket()`. The existing `child_process_for_broker` `Arc<Mutex<Option<SendableHandle>>>` polling pattern in `start_capability_pipe_server` (CITED: `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:417-426`) provides the right ordering primitive. Add an acceptance criterion: "supervisor source descriptor closed only after `send_response` succeeds".

### Job Object — duplicating the supervisor's OWN containment Job

`DuplicateHandle` on `WindowsSupervisorRuntime.containment_job` (the Job Object that owns the agent process tree, used for `--timeout` enforcement and atomic teardown) gives the child a handle to the same kernel object the supervisor uses to terminate it. With default `JOB_OBJECT_QUERY` only, the child can introspect but not interfere. **However:** a profile that opts in to `JOB_OBJECT_TERMINATE` would let the child call `TerminateJobObject` on the supervisor's own containment job — killing the supervisor along with itself. Plan must:
1. Document the footgun in the schema comment for `capabilities.aipc.job_object` (e.g., "WARNING: `terminate` access on a Job Object brokered from the supervisor's containment Job allows the child to kill the supervisor process tree. Only enable for orchestration workloads that explicitly broker a *separate* Job Object the child should manage.").
2. Add a runtime check: when the supervisor brokers a Job Object handle, refuse to broker the containment Job specifically. Compare against `WindowsSupervisorRuntime.containment_job` HANDLE — if they match, return `Denied { reason: "cannot broker the supervisor's own containment Job Object" }` regardless of profile widening. This is defense-in-depth on top of the profile-author warning.

### Event/Mutex — name collisions in `Local\` namespace

`Local\` scopes kernel objects to the current logon session. If two `nono` sessions on the same logon try to broker the same logical name simultaneously, the second `CreateEventW`/`CreateMutexW` fails with `ERROR_ALREADY_EXISTS`. **Mitigation:** session-scoped naming per CONTEXT.md `<specifics>` — `Local\nono-aipc-<user_session_id>-<name>`. Server canonicalizes server-side; client cannot supply the prefix. Add a unit test asserting two parallel sessions with the same logical `name` produce *different* canonical kernel-object names.

### Pipe — anonymous pipe vs named pipe

`DuplicateHandle` works for both, but they have different APIs and different SDDL stories. AIPC-01 in v2.1 supports named pipes only (the SDK request shape is `PipeName(String)`). If a future request shape `PipeAnonymous { read_end_handle, write_end_handle }` is added, the broker call site must NOT allow the child to specify a raw HANDLE in the request — that would be a confused-deputy attack vector. The plan should add an acceptance criterion: "client cannot supply a raw HANDLE in any `HandleTarget` variant; only opaque names that the server canonicalizes."

### All types — supervisor correlation ID vs user-facing session ID (Phase 17 latent-bug pattern)

Phase 17 surfaced 3 pre-existing bugs where `self.session_id` (supervisor correlation) was used in pipe/object names instead of `self.user_session_id` (user-facing 16-hex). Fix commit `7db6595`. AIPC-01 namespace prefixes (`\\.\pipe\nono-aipc-<id>-<name>`, `Local\nono-aipc-<id>-<name>`) MUST use `user_session_id`. Add an acceptance criterion: "all AIPC-01 namespace prefixes derive from `WindowsSupervisorRuntime.user_session_id`, not `self.session_id`. Verified by grep for `format!.*nono-aipc-{}.*self\.session_id` returning 0 matches."

### Wire format — `#[repr(u8)]` is necessary but not sufficient

`#[repr(u8)]` pins the in-memory representation, but `serde_json` serializes enums by name by default. For wire stability across Phase 11 → Phase 18 rolling upgrades (none expected since CONTEXT.md says the wire is internal, but the principle applies), either:
- Use `#[serde(into = "u8", from = "u8")]` to force integer serialization, OR
- Document explicitly that the JSON wire uses string variant names and pin them via `#[serde(rename = "file")]` etc.

Recommendation: keep JSON variant-name encoding (more debuggable) but add a `#[test] fn handle_kind_discriminator_bytes_stable` regression guard so a future contributor can't accidentally renumber the enum.

### Error variant naming

CONTEXT.md D-09 references `NonoError::PlatformNotSupported`; the actual variant is `NonoError::UnsupportedPlatform`. The plan must explicitly choose: (a) reuse `UnsupportedPlatform` (recommended — no churn), or (b) add a new `PlatformNotSupported` variant. Either is acceptable, but the planner must call this out so an implementer doesn't waste time hunting for a non-existent variant.

### Profile format — JSON not TOML

CONTEXT.md D-06 specifies TOML; actual format is JSON. The plan's "schema integration" task must update `crates/nono-cli/data/policy.json` (5 built-in profile entries), `crates/nono-cli/data/nono-profile.schema.json` (top-level `capabilities` property + sub-schema), and `crates/nono-cli/src/profile/mod.rs` (`Profile` struct + `ProfileDeserialize` mirror). NOT a `profile.toml` file or a `toml` crate dep.

## Plan-Decomposition Recommendation

ROADMAP.md suggests "3 plans (likely): protocol extension + handle-type-specific brokers + security tests". After surveying the actual code surface, **3 plans is correct, but the split should be along the *risk and review-ability* axis, not protocol-vs-broker-vs-tests:**

### Plan 18-01 — Protocol skeleton + low-risk handle types (Event + Mutex)

**Wave 0:** new `crates/nono/src/supervisor/policy.rs` module with mask constants, default-mask constants, `mask_is_allowed` validator, unit tests (cross-platform).

**Wave 1:**
- `crates/nono/src/supervisor/types.rs` — new `HandleKind`, `HandleTarget`, `SocketProtocol`, `SocketRole`, `PipeDirection` enums. Mutate `CapabilityRequest` with new fields (deprecate `path` in place). Extend `GrantedResourceKind`, `ResourceTransferKind`, `ResourceGrant`. JSON round-trip tests.
- `crates/nono/src/supervisor/socket_windows.rs` — `broker_event_to_process` and `broker_mutex_to_process` (the two safest brokers; both are `DuplicateHandle` with a fixed mask and no namespace complexity beyond the prefix helper).
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — discriminator check (D-03), match-arm refactor of `handle_windows_supervisor_message` for File + Event + Mutex (3 of 6 arms; Socket/Pipe/JobObject return "not yet implemented" placeholder until Plans 18-02/18-03).
- `crates/nono-cli/src/terminal_approval.rs` — `format_capability_prompt` helper covering File + Event + Mutex prompt templates.
- 4 new integration tests (event + mutex × granted/denied paths) + 2 token-leak tests.

**Why this split:** Establishes the protocol skeleton, the policy module, and the prompt helper using the *least-risk* handle types so reviewers can focus on the wire-protocol design without simultaneously evaluating Win32 socket-duplication semantics or Job Object footguns. Both event and mutex are pure `DuplicateHandle` calls with no namespace-prefix subtleties beyond the shared helper. Phase 11 invariants verified: `bind_low_integrity`, CONIN$ branch, `\\.\pipe\nono-cap-<session_id>` rendezvous, audit redaction, replay protection — all unchanged.

### Plan 18-02 — Pipe + Socket brokers

**Wave 1:**
- `crates/nono/src/supervisor/socket_windows.rs` — `broker_pipe_to_process` (with the `dwOptions = 0` direction mapping), `broker_socket_to_process` (with `WSADuplicateSocketW` + serialization to bytes).
- `crates/nono/src/supervisor/types.rs` — `ResourceGrant::socket_protocol_info_blob` constructor; the new `ResourceTransferKind::SocketProtocolInfoBlob` variant deserializer in the child SDK.
- `crates/nono-cli/Cargo.toml` (and possibly `crates/nono/Cargo.toml`) — add `Win32_Networking_WinSock` to `windows-sys` features.
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — Socket and Pipe match arms in `handle_windows_supervisor_message`. Includes the privileged-port-< 1024 check for Socket. Includes the supervisor-creates-pipe-with-bind_low_integrity-SDDL pattern for the Pipe case.
- `crates/nono-cli/src/terminal_approval.rs` — Socket + Pipe prompt templates.
- 4 new integration tests (socket + pipe × granted/denied) + 2 token-leak tests.

**Why this split:** Pipe and Socket share the common pattern of supervisor-side handle materialization (open or create the source handle before brokering) but differ in how they map down access (Pipe via direction → mask; Socket via `WSADuplicateSocket` blob serialization). Reviewing them together surfaces the asymmetry naturally. Privileged-port check is pure server-side validation in `policy::aipc::validate_socket_request`.

### Plan 18-03 — Job Object broker + extended audit suite

**Wave 1:**
- `crates/nono/src/supervisor/socket_windows.rs` — `broker_job_object_to_process`.
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — Job Object match arm INCLUDING the runtime guard "refuse to broker the supervisor's own `containment_job` HANDLE" (Landmines § Job Object). Job Object prompt template.
- `crates/nono-cli/src/profile/mod.rs` — `CapabilitiesConfig` + `AipcConfig` structs, profile-loader integration, `Profile::resolve_aipc_allowlist` method, `from_token` parser per-type with unknown-token rejection.
- `crates/nono-cli/data/policy.json` — `"capabilities"` block added to claude-code, codex, opencode, openclaw, swival.
- `crates/nono-cli/data/nono-profile.schema.json` — schema extension.
- 2 new integration tests (Job Object × granted/denied) + 1 token-leak test for Job Object.
- 5-shape parameterized refactor of the `session_token_redaction` audit test, replacing the per-kind individual tests added in Plans 18-01/18-02. (Cleanup task — keeps the regression suite small.)
- New profile-integration test: a custom profile JSON with `capabilities.aipc.job_object: ["query", "terminate"]` widens the allowlist; supervisor refuses to broker the containment Job even with TERMINATE in the resolved mask.

**Why this split:** Job Object is the highest-risk handle type because of the supervisor-own-Job footgun. Combined with the profile schema work (which itself touches the most reviewer-visible files — schema, policy.json, mod.rs) means this plan needs the most careful review. Landing it last lets reviewers compare against the established patterns from Plans 18-01/18-02.

### Why not 1 plan or 2 plans?

- **1 plan** (everything together): too large for atomic review. The 5 broker functions × 2 paths × 2 plumbing layers × profile schema = ~15 files modified. Reviewer fatigue likely.
- **2 plans** (protocol + brokers vs profile + tests): forces the profile schema to wait until all 5 brokers ship, but the 5 brokers can't be properly tested without the schema (the resolved-mask comes from the profile). Circular blocker. The 3-plan split breaks the cycle by including a minimal profile path in 18-01 (default-only allowlist, no profile widening required).

## Open Questions

1. **Path field on `CapabilityRequest`: `PathBuf` vs `Option<PathBuf>` in this phase?** CONTEXT.md D-01 text says the type changes; the practical Phase 11 caller surface (`broker_file_handle_to_process` consumers, the existing tests in `socket_windows.rs:741-800`) expects `PathBuf`. Recommendation: keep `PathBuf` typed, add `#[deprecated]` attribute, defer the type change to a later phase to avoid a 5-file rippling rewrite of Phase 11 tests in this phase. Planner should make the call explicit.
2. **Library-side vs CLI-side discriminator check.** D-03 is implementable in either crate. CLI-side keeps `subtle` out of the library deps; library-side puts the check next to the `CapabilityRequest` type definition (more discoverable). Recommendation: CLI-side, matching where the existing token check lives. Document the choice in a code comment so the next reader knows where to look.
3. **Whether to thread the profile through `WindowsSupervisorRuntime` or pre-resolve the AIPC allowlist.** Recommendation: pre-resolve at supervisor construction time (in `execute_supervised`) and pass an `Arc<AipcResolvedAllowlist>` into the capability pipe thread closure. Keeps the hot path branch-free.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `windows-sys = "0.59"` | All Win32 FFI | ✓ | 0.59 (workspace pin) | — |
| `Win32_Networking_WinSock` feature | `WSADuplicateSocketW`, `WSAPROTOCOL_INFOW` | ✗ (must be added) | — | Add to `windows-sys` feature list in `crates/nono/Cargo.toml` and `crates/nono-cli/Cargo.toml` |
| `Win32_System_JobObjects` | Job Object brokering | ✓ | already in nono-cli | Already imported at `crates/nono-cli/src/exec_strategy_windows/mod.rs:401-402` |
| `Win32_System_Threading` | `DuplicateHandle`, `GetCurrentProcess` | ✓ | already in both crates | — |
| `subtle = "2"` | constant-time discriminator + token | ✓ | nono-cli only (Windows target) | If library-side discriminator chosen, add to `crates/nono/Cargo.toml` Windows target deps |
| `getrandom = "0.4"` | (existing — request_id generation reuses Phase 11 pattern) | ✓ | both crates | — |
| Rust 1.77 MSRV | workspace pin | ✓ | per Cargo.toml | — |

**Missing dependencies with no fallback:** None — all are already in the workspace under Windows target deps or are trivially added via `windows-sys` feature flags.

## Validation Architecture

(Validation enabled per .planning/config.json convention — `nyquist_validation` not set to false.)

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in Rust runner) |
| Config file | none — tests live next to source |
| Quick run command | `cargo test -p nono --lib supervisor::policy --lib supervisor::types` |
| Full suite command | `cargo test -p nono --lib supervisor && cargo test -p nono-cli --bin nono capability_handler_tests` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| AIPC-01 | Per-type round-trip (5 kinds) | integration | `cargo test -p nono-cli --bin nono capability_handler_tests::handle_brokers_<kind>` | ❌ Wave 0 (5 new tests) |
| AIPC-01 | Mask outside allowlist denied | integration | `cargo test -p nono-cli --bin nono capability_handler_tests::handle_denies_<kind>_with_mask_outside_allowlist` | ❌ Wave 0 (5 new tests) |
| AIPC-01 | Token-leak audit per kind | integration | `cargo test -p nono-cli --bin nono capability_handler_tests::handle_redacts_token_in_audit_for_<kind>` | ❌ Wave 0 (5 new tests; refactor pre-existing in 18-03) |
| AIPC-01 | Unknown discriminator denied | integration | `cargo test -p nono-cli --bin nono capability_handler_tests::handle_denies_unknown_discriminator` | ❌ Wave 0 (1 new test) |
| AIPC-01 | Unix returns UnsupportedPlatform | unit | `cargo test -p nono --lib supervisor::sdk::unix_request_returns_unsupported_platform` (cfg(not(windows))) | ❌ Wave 0 (5 new tests, one per request_*) |

### Sampling Rate

- **Per task commit:** `cargo test -p nono --lib supervisor::policy` (≈ 5s, validates mask logic)
- **Per wave merge:** `cargo test -p nono --lib supervisor && cargo test -p nono-cli --bin nono capability_handler_tests` (≈ 30s on Windows host)
- **Phase gate:** `make ci` (full clippy + fmt + tests)

### Wave 0 Gaps

- [ ] `crates/nono/src/supervisor/policy.rs` — new file; covers mask validators
- [ ] Extended `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` test module — 16+ new tests across 3 plans
- [ ] `crates/nono/src/supervisor/types.rs` test module — JSON round-trip per HandleKind (6 tests)
- [ ] `windows-sys` feature flag `Win32_Networking_WinSock` added to `crates/nono/Cargo.toml`

## Security Domain

`security_enforcement` is enabled (default). AIPC-01 is purely Windows-side IPC with explicit access-mask allowlists per resource type — security is the central concern of the phase.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | Phase 11 session token (constant-time, env-injected, never logged) |
| V3 Session Management | yes | Phase 11 session-scoped capability pipe + replay-protection HashSet |
| V4 Access Control | yes | Per-handle-type access-mask allowlist; default-deny; profile widens never narrows |
| V5 Input Validation | yes | Server-side mask validation, server-side namespace prefix enforcement, server-side privileged-port check, untrusted target string sanitization in prompt |
| V6 Cryptography | yes | `subtle::ConstantTimeEq` for both session token (Phase 11) and discriminator (Phase 18 D-03); reuse, don't re-implement |
| V7 Error Handling | yes | Verbose Denied reasons for debuggability (per D-07); failure modes documented per Win32 call |
| V14 Configuration | yes | Hard-coded defaults reviewable in single `policy.rs` constant; profile widening fail-secure on parse error |

### Known Threat Patterns for Win32 Handle Brokering

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Confused-deputy: child requests handle to a privileged kernel object | Elevation of Privilege | Server canonicalizes namespace prefix (`Local\nono-aipc-<session_id>-`); client cannot supply arbitrary names |
| Mask escalation: child claims `JOB_OBJECT_TERMINATE` when policy allows only `JOB_OBJECT_QUERY` | Tampering | Server-side `mask_is_allowed` re-validation against resolved allowlist (D-07) |
| Containment-Job hijack: child obtains TERMINATE on the supervisor's own Job Object | Denial of Service / EoP | Runtime guard: refuse to broker the supervisor's `containment_job` HANDLE specifically (Landmines § Job Object) |
| Cross-session interference: two `nono` sessions broker objects with the same logical name | Spoofing | `Local\nono-aipc-<user_session_id>-<name>` namespace scoping per CONTEXT.md `<specifics>` |
| Replay attack: re-send a previously-granted `request_id` | Tampering | Existing Phase 11 `seen_request_ids: HashSet<String>` covers all kinds via the shared dispatch site |
| Token leakage via audit log | Information Disclosure | `audit_entry_with_redacted_token()` extended to all 6 HandleKind shapes (D-11); regression test per kind |
| ANSI-escape injection in prompt | Spoofing | Single `sanitize_for_terminal()` call site applied to every untrusted string field across all 6 prompts (D-04) |
| Privileged-port bind (port < 1024) | Elevation of Privilege | Server-side hard-coded reject regardless of profile widening; documented in policy module |
| Socket blob replay across processes | Tampering | `WSAPROTOCOL_INFOW` is bound to specific target PID at duplication time; single-use by docs |

## Sources

### Primary (HIGH confidence)
- [Synchronization Object Security and Access Rights — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/sync/synchronization-object-security-and-access-rights) — exact hex values for SYNCHRONIZE, EVENT_*, MUTEX_*, SEMAPHORE_*, TIMER_*, including the "MUTEX_MODIFY_STATE Reserved for future use" note
- [Job Object Security and Access Rights — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/procthread/job-object-security-and-access-rights) — exact hex values for JOB_OBJECT_*, the `nested jobs` access-inheritance note, and the "Vista+ JOB_OBJECT_SET_SECURITY_ATTRIBUTES not supported" note
- [WSADuplicateSocketW (winsock2.h) — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-wsaduplicatesocketw) — function signature, target-PID parameter, single-use guarantee, source/destination flow table
- [Generic Access Rights — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/secauthz/generic-access-rights) — GENERIC_READ/WRITE/EXECUTE/ALL hex values
- [File Security and Access Rights — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/fileio/file-security-and-access-rights) — FILE_GENERIC_* mappings (for context on the existing File path)
- `crates/nono/src/supervisor/socket_windows.rs:269-302` — `broker_file_handle_to_process` template
- `crates/nono/src/supervisor/types.rs:14-203` — current `CapabilityRequest`, `ResourceGrant`, `SupervisorMessage` types
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:1155-1296` — current `handle_windows_supervisor_message` dispatch site
- `crates/nono-cli/src/terminal_approval.rs:1-93` — current TerminalApproval + sanitizer + CONIN$ branch
- `crates/nono-cli/src/profile/mod.rs:947-1058` — current `Profile` and `ProfileDeserialize` shape
- `crates/nono-cli/data/policy.json:651-907` — current built-in profile structure for claude-code, codex, opencode, openclaw, swival
- `crates/nono-cli/src/exec_strategy_windows/mod.rs:401-402` — existing `JOB_OBJECT_QUERY = 0x0004` / `JOB_OBJECT_TERMINATE = 0x0008` constants

### Secondary (MEDIUM confidence)
- [Shared Sockets — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/winsock/shared-sockets-2) — IFS vs non-IFS provider semantics for `WSADuplicateSocketW`
- [DuplicateHandle (handleapi.h) — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-duplicatehandle) — `dwOptions = 0` vs `DUPLICATE_SAME_ACCESS` semantics

### Tertiary (LOW confidence — flagged)
- None. Every claim above is sourced from either Microsoft Learn or directly read source on disk.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The library-vs-CLI split for the discriminator check is CLI-side per Phase 11 precedent | Open Questions #2 | Library-side is also valid; recommendation requires planner sign-off but is reversible. |
| A2 | `path: PathBuf` should stay typed and gain `#[deprecated]` rather than become `Option<PathBuf>` in this phase | Open Questions #1 | If planner picks `Option<PathBuf>`, every Phase 11 caller and test in `socket_windows.rs:741-800` and `mod.rs:test_pipe_pair_roundtrip` etc. needs updating. ≈ 8-12 file rewrites. |
| A3 | The 3-plan split along risk axis (Event/Mutex first, Pipe/Socket second, Job Object last) is the right decomposition | Plan-Decomposition Recommendation | Alternative orderings (e.g., Pipe first because it's most analogous to the existing File broker) are defensible. The split itself (3 plans) is supported by ROADMAP and code-surface analysis. |
| A4 | All built-in profile defaults should opt in to a conservative AIPC widening (connect/read/write/query/wait+signal/wait+release) rather than empty | Profile Schema | If planner prefers strict opt-in, set every built-in to empty `aipc: { socket: [], pipe: [], ... }`. The hard-coded defaults will still apply. |
| A5 | Adding `Win32_Networking_WinSock` to `crates/nono/Cargo.toml` `windows-sys` features is the right place for the WSADuplicateSocketW API | File-by-file change map § socket_windows.rs | Could go in `crates/nono-cli/Cargo.toml` instead if the broker function is only called from the CLI. Recommendation depends on whether the broker function is `pub` from the library. |

## Metadata

**Confidence breakdown:**
- Win32 access mask values: HIGH — verified against Microsoft Learn primary docs
- Phase 11 / 17 wiring: HIGH — verified by reading source on disk
- Profile schema integration: HIGH — verified loader path is `serde_json` not `toml`; CONTEXT.md D-06 needs reinterpretation
- Plan decomposition: MEDIUM — based on code-surface analysis; ROADMAP is non-prescriptive
- `MUTEX_MODIFY_STATE = 0x0001` "Reserved for future use" semantics: HIGH — quoted from Microsoft Learn synchronization-object-security page
- Constant-time on u8 discriminator justification: MEDIUM — principled argument; reviewers may disagree, but D-03 already locks the choice

**Research date:** 2026-04-19
**Valid until:** 2026-05-19 (30 days — Win32 surface is stable; only risk is if `windows-sys` minor versions drop the WinSock features, which has not happened in the 0.5x series)

## RESEARCH COMPLETE
