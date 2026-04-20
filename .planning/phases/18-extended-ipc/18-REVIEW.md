---
phase: 18-extended-ipc
reviewed: 2026-04-19T00:00:00Z
depth: standard
files_reviewed: 17
files_reviewed_list:
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/data/nono-profile.schema.json
  - crates/nono-cli/data/policy.json
  - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/exec_strategy_windows/supervisor.rs
  - crates/nono-cli/src/policy.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/session_commands_windows.rs
  - crates/nono-cli/src/terminal_approval.rs
  - crates/nono-cli/tests/aipc_handle_brokering_integration.rs
  - crates/nono/Cargo.toml
  - crates/nono/src/supervisor/aipc_sdk.rs
  - crates/nono/src/supervisor/mod.rs
  - crates/nono/src/supervisor/policy.rs
  - crates/nono/src/supervisor/socket.rs
  - crates/nono/src/supervisor/socket_windows.rs
  - crates/nono/src/supervisor/types.rs
findings:
  critical: 0
  warning: 6
  info: 7
  total: 13
status: issues_found
---

# Phase 18: Code Review Report

**Reviewed:** 2026-04-19
**Depth:** standard
**Files Reviewed:** 17 (plus 4 Phase 18 plan summaries for context)
**Status:** issues_found

## Summary

Phase 18 (AIPC-01) lands a complex, security-critical cross-process handle brokering surface on Windows (5 handle kinds: Event, Mutex, Pipe, Socket, JobObject) plus a cross-platform child-side SDK that returns `NonoError::UnsupportedPlatform` on non-Windows. The implementation is well-structured, thoroughly commented, and hits every major design gate: constant-time discriminator validation, MAP-DOWN access mask enforcement (`dwOptions=0` on `DuplicateHandle`), `CompareObjectHandles` runtime guard against containment-Job hijack, byte-level object-name validation, and a server-side allowlist resolver that UNIONs hard-coded D-05 defaults with profile widening (never narrows).

No critical security holes were identified. Every `unsafe` block carries a `// SAFETY:` comment; no `.unwrap()` / `.expect()` leaks into production paths; D-10 lifecycle ordering is enforced (supervisor closes source after broker returns); audit-token redaction routes through `audit_entry_with_redacted_token` without regressions from Phases 11/17.

The warnings below are second-order concerns: (1) server-side Pipe/Socket mask/role validation runs AFTER the user approval prompt rather than before (unlike Event/Mutex/JobObject, where D-07 is strictly enforced pre-approval), (2) `CompareObjectHandles` returns 0 on error with the same value as "distinct" — a fail-open shape if the API ever errors, (3) Event/Mutex dispatcher uses `CreateEventW`/`CreateMutexW` (which opens an existing object if one exists) rather than `CREATE_NEW`-style semantics, exposing a race-to-create vector from same-session processes that guess the user_session_id, (4) `WSAStartup` return value discarded in the Socket dispatcher path, (5) `NONO_SESSION_TOKEN` and `session_token` string fields are not `zeroize`d after use (pre-existing Phase 11 pattern — not a regression, but a latent concern), (6) an unused `_target_process` parameter on `broker_socket_to_process` is signature bloat.

Info items capture style/readability notes (dead_code allows acknowledged in the summary, name-sanitization completeness, documentation consistency).

## Warnings

### WR-01: Pipe/Socket mask & role validation runs AFTER backend approval, not before

**File:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:1817-1820, 1852-1906`
**Issue:** The pre-approval mask validation gate is guarded by
```rust
if matches!(request.kind, HandleKind::Event | HandleKind::Mutex | HandleKind::JobObject)
```
— so for `HandleKind::Pipe` and `HandleKind::Socket`, the dispatcher skips the server-side `mask_is_allowed` gate entirely and dispatches straight to the backend approval prompt. The per-kind helpers (`handle_pipe_request`, `handle_socket_request`) then validate direction/role/port AFTER the user has already been prompted.

Per CONTEXT.md D-07 and the 18-01 summary: "Server-side per-kind mask validation runs BEFORE backend dispatch so out-of-allowlist requests never reach the user's approval prompt." That invariant holds for Event/Mutex/JobObject but NOT for Pipe/Socket.

Impact is UX, not a security hole — the helper does reject and the response carries `grant: None`, so no leak. But the user sees a spurious prompt for a request that was going to be denied. A confused or impatient user who clicks "y" on an invalid Pipe direction request still gets a denial, but the approval-UX invariant documented in the summary is violated.

**Fix:** Either (a) extend the pre-approval validation gate to cover Pipe (direction decode from `access_mask`) and Socket (privileged-port + resolved-roles check), or (b) document in the dispatcher that Pipe/Socket validation is intentionally inside the helper and update the summary's D-07 invariant claim. Option (a) is more consistent:
```rust
// Additional pre-approval validation for Pipe and Socket per D-07.
match request.kind {
    HandleKind::Pipe => {
        // Decode direction from mask + check against resolved_allowlist.pipe_directions
    }
    HandleKind::Socket => {
        if let Some(HandleTarget::SocketEndpoint { port, role, .. }) = request.target.as_ref() {
            if *port <= policy::PRIVILEGED_PORT_MAX { /* Denied */ }
            if !resolved_allowlist.socket_roles.contains(role) { /* Denied */ }
        }
    }
    _ => {}
}
```

### WR-02: CompareObjectHandles fail-open shape — error return aliases with "distinct" result

**File:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:1678-1695`
**Issue:** Per Microsoft docs: "If [the handles] do not [refer to the same object], **or if an error occurs**, the return value is zero." The supervisor treats "same object" as `same != 0` (return != 0) and "different object / allow brokering" as `same == 0`. If `CompareObjectHandles` itself fails (e.g. a kernel-access denied quirk on a future Windows build), the API returns 0 — which the code interprets as "different kernel object, safe to broker" — letting the containment Job hijack slip through in that edge.

In practice, both HANDLEs are live process-owned handles, so the API is very unlikely to fail. But the threat model for this guard is T-18-03-01 (the supervisor's own termination Job escaping to the child, letting the child kill the supervisor process tree). "Fail secure on any error" from CLAUDE.md § Core Principles argues for the conservative default.

**Fix:** Check `GetLastError()` after a zero return to distinguish "distinct" from "API error," and on error fail-secure:
```rust
let same = unsafe { CompareObjectHandles(job, runtime_containment_job) };
if same != 0 {
    // Same kernel object — reject as before.
    unsafe { CloseHandle(job); }
    return Err(NonoError::SandboxInit(
        "cannot broker the supervisor's own containment Job Object".to_string(),
    ));
}
// Defense-in-depth: a zero return may mean "different object" OR "API
// error". In the error case we have no way to prove they differ, so treat
// the call as fail-secure per CLAUDE.md § Fail Secure.
let err_code = unsafe { GetLastError() };
if err_code != 0 {
    unsafe { CloseHandle(job); }
    return Err(NonoError::SandboxInit(format!(
        "CompareObjectHandles failed (os error {err_code}); refusing to broker Job Object"
    )));
}
```

### WR-03: CreateEventW / CreateMutexW open existing kernel objects — race-to-create vector

**File:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:1322, 1579`
**Issue:** `handle_event_request` calls `CreateEventW(NULL, 0, 0, wide.as_ptr())` — per MSDN, if a named object with the same name already exists, `CreateEventW` returns a HANDLE to the existing object (not a new one) and `GetLastError` is set to `ERROR_ALREADY_EXISTS`. Same applies to `CreateMutexW`.

A same-logon-session attacker who can guess the canonical name `Local\nono-aipc-<user_session_id>-<name>` could pre-create the Event/Mutex with its own DACL/ACL, then the supervisor opens that existing object and brokers it to the child. The child receives a handle to an attacker-controlled kernel object, potentially exposing it to cross-process signaling by the attacker.

Mitigation in the current design: `user_session_id` is a 16-hex random ID not obviously leaked. But it IS passed as an env var to the child (`NONO_SESSION_ID` per Phase 11 D-01), and same-user processes can enumerate named kernel objects via Object Manager. The attacker need only time the race before CreateEventW.

**Fix:** Either (a) check `GetLastError() == ERROR_ALREADY_EXISTS` after `CreateEventW`/`CreateMutexW` and fail-secure, or (b) add a random component to the canonical name so name prediction becomes infeasible:
```rust
let event: HANDLE = unsafe { CreateEventW(std::ptr::null_mut(), 0, 0, wide.as_ptr()) };
if event.is_null() {
    return Err(NonoError::SandboxInit(/* ... */));
}
// Defense-in-depth: fail secure if the kernel object already existed
// (race-to-create). The supervisor owns this namespace; a pre-existing
// object implies another process got there first.
let err = unsafe { GetLastError() };
if err == windows_sys::Win32::Foundation::ERROR_ALREADY_EXISTS {
    unsafe { CloseHandle(event); }
    return Err(NonoError::SandboxInit(format!(
        "Event \"{canonical}\" already exists (race-to-create refused)"
    )));
}
```
Same pattern for `CreateMutexW` in `handle_mutex_request`.

### WR-04: WSAStartup return value discarded in Socket dispatcher

**File:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:1500`
**Issue:** `let _ = unsafe { WSAStartup(0x0202, &mut wsadata) };` — the return value is discarded. If WSAStartup fails (returns non-zero), the subsequent `WSASocketW` will fail with `WSANOTINITIALISED`, surfacing as a `NonoError::SandboxInit("WSASocketW failed: ...")`. The error message is then slightly misleading (it's actually a startup failure, not a socket-creation failure). Not a security issue — the function is fail-closed by cascade — but it hurts diagnostic quality.

**Fix:**
```rust
let mut wsadata: WSADATA = unsafe { std::mem::zeroed() };
let rc = unsafe { WSAStartup(0x0202, &mut wsadata) };
if rc != 0 {
    return Err(NonoError::SandboxInit(format!(
        "WSAStartup(2.2) failed with error {rc}"
    )));
}
```

### WR-05: session_token not zeroized after use in CapabilityRequest / SDK

**File:** `crates/nono/src/supervisor/types.rs:178-179`, `crates/nono/src/supervisor/aipc_sdk.rs:367-400`
**Issue:** `CapabilityRequest.session_token: String` is a Phase 11 field carrying the 32-byte hex session token. The SDK reads `NONO_SESSION_TOKEN` into a local `String` and stamps it into the request. Both `String`s drop normally without `zeroize`. CLAUDE.md § Coding Standards: "Use the `zeroize` crate for sensitive data (keys/passwords) in memory."

This is a **pre-existing Phase 11 pattern** — not a Phase 18 regression — but Phase 18 multiplies the number of code paths that handle the token (5 new `request_*` SDK methods). If a future memory-dump attack or swap-file exfiltration occurs, the token may linger in heap memory far longer than needed.

**Fix (follow-up, out of Phase 18 scope):** Wrap session_token in `zeroize::Zeroizing<String>`, propagate type changes through the IPC pipeline. The `zeroize` crate is already a workspace dep. Alternatively, change `pub session_token: String` to a dedicated `SessionToken` newtype that implements `Zeroize` on drop.

### WR-06: broker_socket_to_process has unused BrokerTargetProcess parameter

**File:** `crates/nono/src/supervisor/socket_windows.rs:595-600`
**Issue:** `pub fn broker_socket_to_process(socket, _target_process: BrokerTargetProcess, target_pid, role)` — the `_target_process` parameter is unused (WSADuplicateSocketW takes a PID, not a HANDLE). The underscore prefix placates the compiler, but public API surface bloat means every caller must construct a `BrokerTargetProcess` they don't need.

The 18-02 summary notes: "BrokerTargetProcess is kept in the signature for symmetry with non-socket brokers and to support future ownership tracking" — a reasonable rationale, but the public-API implications (future deprecation, caller friction) warrant either a doc-comment explicitly naming the shim or a quiet rename/remove in a minor-version bump.

**Fix:** Add explicit doc-comment calling out the parameter's purpose:
```rust
/// The `_target_process` parameter is currently unused (WSADuplicateSocketW
/// takes a PID rather than a HANDLE). It is retained for API symmetry with
/// the other `broker_*_to_process` functions and to preserve a caller-stable
/// shape when/if future audit enrichment wants the process-handle context.
pub fn broker_socket_to_process(
    socket: SOCKET,
    _target_process: BrokerTargetProcess,
    /* ... */
) -> Result<ResourceGrant> {
```
Or — if no future use is anticipated — remove it and accept the non-uniformity with broker_event/mutex/pipe.

## Info

### IN-01: format_capability_prompt helpers still carry #[allow(dead_code)]

**File:** `crates/nono-cli/src/terminal_approval.rs:165, 183, 200, 217, 254`
**Issue:** `format_event_access`, `format_mutex_access`, `format_pipe_direction`, `format_job_object_access`, and `format_capability_prompt` all carry `#[allow(dead_code)]`. Per Phase 18-04 summary Deferred Issues #3: "The dispatcher wires `format!` strings inline rather than routing through the helper." The helpers ARE consumed by tests, but production code does not route prompts through them.

Tracked as a known deferred issue; CLAUDE.md § "Lazy use of dead code" warns: "Avoid `#[allow(dead_code)]`. If code is unused, either remove it or write tests that use it." Tests do use the helpers, so the allow is technically valid — but the longer-term intent is for Phase 19+ to wire the CONIN$ prompt through `format_capability_prompt`.

**Fix:** Track as a follow-up task; no immediate action required.

### IN-02: Deprecated path field on CapabilityRequest propagates #[allow(deprecated)] across 7 callers

**File:** `crates/nono/src/supervisor/socket.rs:525`, `crates/nono/src/supervisor/socket_windows.rs:1308-1319`, `crates/nono-cli/src/exec_strategy/supervisor_linux.rs:398`, `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:2028, 2047`, `crates/nono-cli/src/terminal_approval.rs:474`, `crates/nono/src/supervisor/mod.rs:156`
**Issue:** Per CONTEXT.md D-01 and 18-01 summary, `CapabilityRequest.path: PathBuf` is `#[deprecated]` but kept in place for one release. Seven callers wrap construction in `#[allow(deprecated)]` to silence the warning. This is deliberate and documented, but the `#[allow(deprecated)]` blocks will become dead when the field is finally removed.

Tracked as 18-01 summary "Open Paths for Plans 18-02 and 18-03" → "migration marker." No action required.

**Fix:** Track as a scheduled removal in a future phase.

### IN-03: Policy.json profiles do not widen capabilities.aipc for 4 bare dev profiles

**File:** `crates/nono-cli/data/policy.json:837-905`
**Issue:** Five built-in profiles gained `capabilities.aipc` blocks (claude-code, codex, opencode, openclaw, swival). The four dev profiles (python-dev, node-dev, go-dev, rust-dev) lack `capabilities.aipc` blocks entirely. That's intentional per 18-03 summary — they inherit the hard-coded D-05 defaults, which match their threat model. But it's worth a sanity check that this isn't an oversight.

**Fix:** Verify intent is documented. If dev profiles should share the same AIPC widening as agent profiles, add blocks. Otherwise, document the deliberate absence.

### IN-04: validate_aipc_object_name accepts `.` and `..` literals

**File:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:1258-1273`
**Issue:** The validator rejects `\`, `/`, `:`, NUL, and control bytes but does NOT reject `.` or `..`. These aren't path-traversal vectors in the kernel-object namespace (where `.` and `..` are just literal names, not parent-directory shortcuts), but including them defends against conceptual confusion in logs / audit trails.

Not a security issue — kernel-object names aren't interpreted by the filesystem layer.

**Fix:** Optional defense-in-depth — reject `.` and `..` as leaf names if a cleaner-looking audit trail is desired.

### IN-05: handle_socket_request validates host length by bytes, not characters

**File:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:1482-1486`
**Issue:** `host.len() > 253` checks byte length. Per DNS RFC 1035, the 253-byte limit IS the correct limit (byte length of encoded FQDN). But a UTF-8 host with multi-byte characters could legitimately be shorter than 253 chars but longer than 253 bytes. This is correct by spec, but non-ASCII host names are uncommon enough that a dev reviewing this line might flag it.

**Fix:** Add a doc-comment or inline comment clarifying the intent is DNS-byte-length, not char count:
```rust
// DNS RFC 1035 limits encoded FQDN to 253 bytes. `.len()` is byte length,
// which is the correct measure here (not `.chars().count()`).
if host.is_empty() || host.len() > 253 {
```

### IN-06: Unused role parameter in socket_protocol_info_blob constructor

**File:** `crates/nono/src/supervisor/types.rs:388-402`
**Issue:** `ResourceGrant::socket_protocol_info_blob(bytes: Vec<u8>, role: SocketRole)` — the `role` parameter is acknowledged as informational only:
```rust
pub fn socket_protocol_info_blob(bytes: Vec<u8>, role: SocketRole) -> Self {
    // The `role` parameter is currently informational only — kept in the
    // signature so future audit-trail enrichment can carry per-role data
    // without changing the constructor signature. The `let _ = role;`
    // placates clippy without an `#[allow(unused)]` attribute.
    let _ = role;
    /* ... */
}
```
Same shape as WR-06 (unused BrokerTargetProcess) but documented inline. Accepted pattern, Info-level mention only.

**Fix:** No action; tracked via doc comment.

### IN-07: Phase 11 MAX_MESSAGE_SIZE cap (64 KiB) unchanged from Phase 11

**File:** `crates/nono/src/supervisor/socket_windows.rs:50`
**Issue:** The 64-KiB message cap from Phase 11 is unchanged. The largest single-message payload in Phase 18 is the WSAPROTOCOL_INFOW blob (~372 bytes) plus CapabilityRequest overhead (~1 KB with a verbose reason). The cap is comfortably above the worst-case AIPC-01 request size.

Noted for completeness — no action required. The cap is a Phase 11 invariant that the 18-01/02/03 summaries verify as byte-identical.

**Fix:** No action.

---

_Reviewed: 2026-04-19_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
