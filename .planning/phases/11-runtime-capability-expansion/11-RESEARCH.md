# Phase 11: Runtime Capability Expansion - Research

**Researched:** 2026-04-11
**Domain:** Windows supervisor IPC, session token authentication, named-pipe security descriptors
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01**: `session_token: String` field on `CapabilityRequest`. 32-byte random hex via `getrandom::fill`. Delivered via `NONO_SESSION_TOKEN` env var. Constant-time comparison using `subtle` crate before approval backend. Never logged.
- **D-02**: Dedicated capability pipe (separate from control pipe). `SupervisorSocket::bind(rendezvous_path)`. Path delivered via `NONO_SUPERVISOR_PIPE` env var. Child uses `SupervisorSocket::connect()`.
- **D-03**: Background thread (`start_capability_pipe_server()`). Audit entries via `mpsc::channel`. `handle_windows_supervisor_message()` promoted out of `#[cfg(test)]`.
- **D-04**: `TerminalApproval` gains `#[cfg(target_os = "windows")]` branch opening `\\.\CONIN$`.
- **D-05**: Filesystem-only grant for this phase. `ResourceGrant.resource_kind` is already extensible.

### Claude's Discretion
- Token generation: `getrandom::fill` → 32 bytes → hex-encoded → 64-char string.
- Rendezvous path: `std::env::temp_dir()` + session-unique filename (`nono-cap-{session_id}.pipe`).
- Replay protection: `seen_request_ids: HashSet<String>` already in handler — keep as-is.
- Thread shutdown: capability pipe server thread exits naturally on child disconnect (pipe EOF).
- `TerminalApproval` on Windows when no console: deny with "No console available for interactive approval".

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TRUST-01 | A sandboxed child can request additional capabilities via named pipe; supervisor prompts user; requests without valid session token are denied immediately with constant-time comparison | D-01 through D-05; see findings below |
</phase_requirements>

---

## RESEARCH COMPLETE

### Summary

All infrastructure for Phase 11 exists in the codebase — the work is mostly wiring and ungating. The most important finding is the **SDDL gap**: `SupervisorSocket::bind()` in `socket_windows.rs` creates a pipe with `null()` security attributes (no mandatory integrity label), so a Low Integrity sandboxed child cannot connect to it. Plan 11-01 must add `S:(ML;;NW;;;LW)` to the pipe SACL before the capability pipe is usable. The `subtle` crate is already in `nono-proxy` at version 2; it needs to be added to `nono-cli`. `handle_windows_supervisor_message()` is complete but test-gated and lacks token validation — ungating and adding the token check is the bulk of plan work. No breaking serialization changes are needed for `CapabilityRequest` because serde uses field-by-field deserialization.

**Primary recommendation:** Two plans. Plan 11-01: SDDL fix + `session_token` field + token validation + `start_capability_pipe_server()` wiring + env var injection. Plan 11-02: `TerminalApproval` Windows branch + integration test.

---

## Finding 1: `subtle` crate — not in nono or nono-cli, already in nono-proxy

**File:** `crates/nono-proxy/Cargo.toml`, line 29 [VERIFIED: read]
**File:** `crates/nono-cli/Cargo.toml` — no `subtle` entry [VERIFIED: read]
**File:** `crates/nono/Cargo.toml` — no `subtle` entry [VERIFIED: read]

The `subtle = "2"` dependency is present in `crates/nono-proxy/Cargo.toml` only. The full `constant_time_eq` helper is implemented in `crates/nono-proxy/src/token.rs`:

```rust
// crates/nono-proxy/src/token.rs (lines 36-41)
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}
```

**What must change:** Add `subtle = "2"` to `[target.'cfg(target_os = "windows")'.dependencies]` in `crates/nono-cli/Cargo.toml` (token validation lives in Windows-specific supervisor code). The `getrandom` crate is already in `crates/nono/Cargo.toml` at version `0.4` [VERIFIED: line 22].

**Token generation pattern to follow** (already working in `crates/nono-proxy/src/token.rs`, lines 21-28):
```rust
let mut bytes = [0u8; 32];
getrandom::fill(&mut bytes).map_err(|e| ...)?;
let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
// hex is 64 chars, zero bytes immediately after
```

---

## Finding 2: `CapabilityRequest` current struct — `session_token` field is missing

**File:** `crates/nono/src/supervisor/types.rs`, lines 18-32 [VERIFIED: read]

Current struct:
```rust
pub struct CapabilityRequest {
    pub request_id: String,
    pub path: PathBuf,
    pub access: AccessMode,
    pub reason: Option<String>,
    pub child_pid: u32,
    pub session_id: String,
}
```

`session_token: String` is absent. Adding it is a breaking serialization change in the sense that old JSON without `session_token` will fail deserialization. However, existing tests that construct `CapabilityRequest` directly (not from JSON fixtures) need to be updated to include the new field.

**Existing test fixtures using `CapabilityRequest` directly:**
- `crates/nono/src/supervisor/mod.rs` lines 134-142 — `make_request()` helper builds it manually.
- `crates/nono/src/supervisor/socket_windows.rs` lines 632-639 — `test_pipe_pair_roundtrip` builds it manually.
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — no direct construction found (handler receives it via deserialization).

Both test helpers must be updated to add `session_token: "test-token".to_string()` (or an empty string for non-security-critical tests).

**No JSON fixture files** were found that persist `CapabilityRequest` to disk — only in-memory test construction. [VERIFIED: grep found no `.json` test fixtures for CapabilityRequest]

**Audit log concern:** `AuditEntry` embeds `request: CapabilityRequest` (line 129). Serialized audit entries will include `session_token`. Plan must redact the token before creating the `AuditEntry` — either by zeroing it or by adding a `#[serde(skip)]` attribute. The CONTEXT.md decision says "Audit log entries must omit the token field entirely." The simplest approach is to clone the request and clear the token field before embedding in `AuditEntry`.

---

## Finding 3: `handle_windows_supervisor_message()` — exact gating and needed changes

**File:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs`, lines 696-804 [VERIFIED: read]

Both `open_windows_supervisor_path` (line 696) and `handle_windows_supervisor_message` (line 723) are gated by `#[cfg(test)]`. Both gates must be removed simultaneously.

The function signature (line 724-731):
```rust
pub(super) fn handle_windows_supervisor_message(
    sock: &mut nono::SupervisorSocket,
    msg: nono::supervisor::SupervisorMessage,
    approval_backend: &dyn ApprovalBackend,
    target_process: nono::BrokerTargetProcess,
    seen_request_ids: &mut HashSet<String>,
    audit_log: &mut Vec<AuditEntry>,
) -> Result<()>
```

**Token validation must be inserted** immediately after `seen_request_ids` replay check (after line 752) and before the `approval_backend.request_capability()` call (line 754). The insertion point:

```rust
// After: seen_request_ids.insert(request.request_id.clone());
// Before: let decision = approval_backend.request_capability(&request)

// Token validation (D-01)
let expected_token: &str = /* passed in as new parameter */;
if !constant_time_eq(request.session_token.as_bytes(), expected_token.as_bytes()) {
    let decision = nono::ApprovalDecision::Denied {
        reason: "Invalid session token".to_string(),
    };
    // Build AuditEntry with token redacted
    let mut redacted = request.clone();
    redacted.session_token = String::new();
    audit_log.push(AuditEntry { request: redacted, decision: decision.clone(), ... });
    return sock.send_response(...);
}
```

**New parameter needed:** The function needs the expected session token. Add `session_token: &str` parameter. The `start_capability_pipe_server()` thread must clone and move the token in.

**`open_windows_supervisor_path`** (lines 697-721) is currently also `#[cfg(test)]` — it gets promoted to production at the same time.

---

## Finding 4: `start_capability_pipe_server()` — pattern and integration point

**File:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs`, lines 316-393 (`start_control_pipe_server`), lines 527-597 (`start_data_pipe_server`) [VERIFIED: read]

**Exact integration point:** `WindowsSupervisorRuntime::initialize()` (line 197-236). After `start_data_pipe_server()` is called (around line 229), `start_capability_pipe_server()` should be called. The runtime struct does not currently have a session_token field — it must be added.

**Pattern to follow** (from `start_control_pipe_server`):
```rust
fn start_capability_pipe_server(&self) -> Result<()> {
    let session_id = self.session_id.clone();
    let session_token = self.session_token.clone();  // new field
    let rendezvous_path = self.cap_pipe_rendezvous_path.clone();  // new field
    let audit_tx = self.audit_tx.clone();  // new field (mpsc sender)
    let terminate_requested = self.terminate_requested.clone();

    std::thread::spawn(move || {
        let mut sock = match nono::SupervisorSocket::bind(&rendezvous_path) {
            Ok(s) => s,
            Err(e) => { tracing::error!(...); return; }
        };
        let mut seen_request_ids = HashSet::new();
        let mut local_audit = Vec::new();
        // ... loop: recv_message → handle_windows_supervisor_message → audit_tx.send(entries)
    });
    Ok(())
}
```

**New fields needed on `WindowsSupervisorRuntime`:**
- `session_token: String` — the 32-byte hex token
- `cap_pipe_rendezvous_path: PathBuf` — where `SupervisorSocket::bind()` publishes its pipe
- `audit_tx: std::sync::mpsc::Sender<Vec<AuditEntry>>` — for draining audit entries in the event loop

**Event loop drain:** `run_child_event_loop` (lines 603-643) currently has no audit drain. The mpsc receiver must be stored on the runtime struct and drained each iteration alongside the terminate/exit checks.

**`BrokerTargetProcess`:** The capability pipe server thread needs the child's process handle to call `DuplicateHandle`. This is only known after `spawn_windows_child()` returns. Two options:
- Option A: Pass the process handle to the thread after spawn via a `oneshot` channel or `Arc<Mutex<Option<HANDLE>>>`.
- Option B: Start the capability pipe server after spawn (not in `initialize()`), using the child's handle directly.

Option B is simpler but changes where `NONO_SUPERVISOR_PIPE` is known (must be computed before spawn, stored, then passed). Option A keeps the thread lifecycle clean. The CONTEXT.md says the thread is called from `initialize()` — so Option A is the decided pattern. The thread can block waiting for the child handle before it starts processing messages.

---

## Finding 5: SDDL gap — CRITICAL: `SupervisorSocket::bind()` creates pipe with null security descriptor

**File:** `crates/nono/src/supervisor/socket_windows.rs`, lines 460-490 [VERIFIED: read]

`create_named_pipe()` passes `std::ptr::null()` as `lpSecurityAttributes`. This means:
- The pipe uses the default DACL (inherited from the process token).
- There is **no mandatory integrity label SACL** (`S:(ML;;NW;;;LW)`).
- A Low Integrity sandboxed child process **cannot write to a Medium or higher integrity pipe** on Windows Vista+ by default (Mandatory Integrity Control).

**STATE.md research flag (confirmed):** "If `S:(ML;;NW;;;LW)` is absent, 11-01 must add it; this changes scope." [VERIFIED: it is absent]

**What must be added:** The `create_named_pipe` function in `socket_windows.rs` (library crate) needs either:
- A new overload/flag that enables SACL injection, OR
- A new `bind_low_integrity()` method on `SupervisorSocket` that uses a security descriptor string.

The SDDL string needed: `"D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;LW)"` — this is the `create_secure_pipe` DACL from `supervisor.rs` plus the Low Integrity SACL.

**`Win32_Security_Authorization` feature** is already in `crates/nono/Cargo.toml`'s windows-sys features (line 49) — `ConvertStringSecurityDescriptorToSecurityDescriptorW` is available. [VERIFIED: read]

**Impact:** This is a scope addition for Plan 11-01. Without it, the capability pipe will silently fail when the sandboxed child (Low Integrity) attempts to connect — the `connect_named_pipe` call will return `ERROR_ACCESS_DENIED` and the child gets no actionable error.

---

## Finding 6: `TerminalApproval` — current Unix implementation and Windows branch needed

**File:** `crates/nono-cli/src/terminal_approval.rs`, lines 1-67 [VERIFIED: read]

The full `TerminalApproval::request_capability()` implementation is Unix-only due to the `/dev/tty` path (line 43). The Windows branch needs:

```rust
#[cfg(target_os = "windows")]
let tty = std::fs::File::open(r"\\.\CONIN$").map_err(|e| {
    NonoError::SandboxInit(format!("Failed to open \\.\CONIN$ for approval prompt: {e}"))
})?;
```

**`is_terminal()` check (line 21):** Works on both platforms — `IsTerminal` is implemented for Windows in std since Rust 1.70 [ASSUMED: based on training knowledge; verifiable via std docs]. Stderr check remains unchanged.

**`sanitize_for_terminal()` function (lines 78-118):** Platform-agnostic, no changes needed.

**Background thread safety of `\\.\CONIN$`:** On Windows, `\\.\CONIN$` opens the console input buffer of the process's attached console, regardless of which thread opens it. This works from background threads as long as the supervisor process has an attached console — which it will during interactive sessions. When there is no console (e.g., fully detached), `File::open(r"\\.\CONIN$")` returns `ERROR_INVALID_HANDLE`, which the error path converts to a denial. [ASSUMED: based on Windows API documentation patterns; should be verified with a quick Windows test]

**Where `TerminalApproval` is wired for Windows:** `crates/nono-cli/src/supervised_runtime.rs` lines 157 and 200. The `approval_backend: &approval_backend` in the Windows `SupervisorConfig` block (line 200) already passes `TerminalApproval` — but `WindowsSupervisorDenyAllApprovalBackend` is what actually handles requests because `handle_windows_supervisor_message()` is test-gated. Once ungated, `TerminalApproval` flows through automatically.

---

## Finding 7: `ExecConfig.env_vars` injection — exact mechanism

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs`, lines 244-355 [VERIFIED: read]
**File:** `crates/nono-cli/src/execution_runtime.rs`, lines 273-283 [VERIFIED: read]

`ExecConfig.env_vars` is `Vec<(&'a str, &'a str)>` (mod.rs line 96). `build_child_env()` in `launch.rs` appends all `config.env_vars` entries after the base environment (lines 349-351):

```rust
for (key, value) in &config.env_vars {
    env_pairs.push(((*key).to_string(), (*value).to_string()));
}
```

**Injection pattern** (from `execution_runtime.rs` line 273-283):
```rust
let config = exec_strategy::ExecConfig {
    command: &command,
    ...
    env_vars,  // Vec<(&str, &str)> — add NONO_SESSION_TOKEN and NONO_SUPERVISOR_PIPE here
    ...
};
```

**Lifetime constraint:** `env_vars` is `Vec<(&'a str, &'a str)>` where `'a` is tied to the `ExecConfig` lifetime. The token string and rendezvous path string must outlive the `ExecConfig`. They must be stored as owned `String` values before building `env_vars` slices from them.

**Where to generate token and rendezvous path:** In `execution_runtime.rs` before the `ExecConfig` construction (around line 273), alongside existing `session_sid: Some(exec_strategy::generate_session_sid())`. The rendezvous path is `std::env::temp_dir().join(format!("nono-cap-{}.pipe", session_id))`.

---

## Finding 8: `WindowsSupervisorDenyAllApprovalBackend` — SC #4 "remains active when disabled"

**File:** `crates/nono-cli/src/exec_strategy_windows/mod.rs`, lines 111-129 [VERIFIED: read]

`WindowsSupervisorDenyAllApprovalBackend` denies all requests with a message "Windows live runtime capability expansion is not attached...". SC #4 says it "remains active and keeps the system secure when this feature is disabled or unavailable."

**Interpretation:** The deny-all backend stays as the fallback in `SupervisorConfig.approval_backend` when the capability pipe feature is not enabled. Since `TerminalApproval` is already wired via `supervised_runtime.rs`, and `WindowsSupervisorDenyAllApprovalBackend` is only used in test scenarios or when the capability pipe thread doesn't start, the SC is satisfied by:
1. Keeping `WindowsSupervisorDenyAllApprovalBackend` defined (do not delete it).
2. The capability pipe server only starts when `NONO_SUPERVISOR_PIPE` is set (i.e., when the feature is enabled).
3. When the feature is not enabled, messages arriving on the control channel hit the deny-all fallback.

No runtime flag needed — the capability pipe server's absence means no capability requests can be processed (the child has no pipe to send to).

---

## Finding 9: Existing tests for `handle_windows_supervisor_message`

**File:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` [VERIFIED: grep]

There are **no test cases** that call `handle_windows_supervisor_message()`. The function is defined but never tested. The `open_windows_supervisor_path` function is also test-gated but never called from a test.

The existing tests for the supervisor type infrastructure are:
- `crates/nono/src/supervisor/mod.rs` — tests `ApprovalBackend` trait implementations (lines 103-195)
- `crates/nono/src/supervisor/socket_windows.rs` — tests pipe round-trips (lines 622-801)

Plan 11-02 should add tests for `handle_windows_supervisor_message` directly: token-mismatch deny, replay-detect deny, grant path (using `BrokerTargetProcess::current()`), and OpenUrl/Terminate/Detach arms.

---

## Implementation Risks

### Risk 1: SDDL addition to `socket_windows.rs` is in the library crate
Adding Low Integrity SACL to `create_named_pipe()` is in `crates/nono` (the library), which has no approval backend concept. The change must be backward-compatible — either add a `create_named_pipe_low_integrity()` variant or add a bool flag. The supervisor.rs `create_secure_pipe` already shows the SDDL pattern.

### Risk 2: `BrokerTargetProcess` requires child process handle before capability pipe can broker
The thread spawned by `start_capability_pipe_server()` cannot call `DuplicateHandle` into the child until the child is spawned. The thread must receive the child handle after `spawn_windows_child()` completes. Use `Arc<Mutex<Option<HANDLE>>>` initialized to `None`, set after spawn, thread waits for non-None before processing capability requests.

### Risk 3: `session_token` in `AuditEntry.request` will be logged if not redacted
`AuditEntry` stores `request: CapabilityRequest` directly. After adding `session_token` to `CapabilityRequest`, any `Debug` print or JSON serialization of `AuditEntry` will expose the token. The handler must clone the request, zero the `session_token` field, then build the `AuditEntry`. Consider adding `#[serde(skip)]` to `session_token` on `CapabilityRequest` — but that breaks the child's ability to send it. Instead, redact in the handler only.

### Risk 4: `ExecConfig.env_vars` lifetime — owned strings must outlive config
`env_vars: Vec<(&'a str, &'a str)>` borrows from owned strings. The token and rendezvous path must be declared as `let token_str = ...` and `let pipe_str = ...` before the `ExecConfig` struct literal to satisfy lifetime `'a`.

### Risk 5: `TerminalApproval` called from background thread on Windows may block indefinitely
The capability pipe server background thread calls `approval_backend.request_capability()` which opens `\\.\CONIN$` and blocks on `read_line`. If the console is closed mid-session, `read_line` may return EOF immediately. The handler should treat EOF on the console input as a denial (consistent with Unix).

### Risk 6: No `subtle` in `nono-cli`'s Windows dependencies yet
Forgetting to add `subtle = "2"` under `[target.'cfg(target_os = "windows")'.dependencies]` will cause a compile error on Windows only, invisible in cross-compilation on Unix.

---

## Recommended Plan Structure

### Plan 11-01: Core IPC wiring, token auth, SDDL fix
**Wave 1 (foundation):**
- Add `session_token: String` to `CapabilityRequest` in `types.rs`. Update both in-code test fixtures (`mod.rs` `make_request()` and `socket_windows.rs` `test_pipe_pair_roundtrip`).
- Add `subtle = "2"` under Windows target dependencies in `crates/nono-cli/Cargo.toml`.
- Add `S:(ML;;NW;;;LW)` SACL to `create_named_pipe()` in `socket_windows.rs` — either via a new `create_named_pipe_with_low_integrity_sacl()` variant or a bool param. The `Win32_Security_Authorization` feature is already present.

**Wave 2 (token validation in handler):**
- Remove `#[cfg(test)]` from `open_windows_supervisor_path` and `handle_windows_supervisor_message` in `supervisor.rs`.
- Add `session_token: &str` parameter to `handle_windows_supervisor_message`.
- Insert constant-time token check before `approval_backend.request_capability()`. Redact token in `AuditEntry`.

**Wave 3 (capability pipe server thread):**
- Add `session_token: String`, `cap_pipe_rendezvous_path: PathBuf`, `audit_tx: mpsc::Sender<Vec<AuditEntry>>`, and `child_process_for_broker: Arc<Mutex<Option<HANDLE>>>` fields to `WindowsSupervisorRuntime`.
- Implement `start_capability_pipe_server()` following `start_control_pipe_server()` pattern.
- Wire into `initialize()` after `start_data_pipe_server()`.
- Add mpsc receiver drain to `run_child_event_loop`.

**Wave 4 (env var injection):**
- In `execution_runtime.rs`, generate token and rendezvous path before `ExecConfig` construction.
- Add `("NONO_SESSION_TOKEN", token_str.as_str())` and `("NONO_SUPERVISOR_PIPE", pipe_str.as_str())` to `env_vars`.
- Store rendezvous path in `WindowsSupervisorRuntime` for cleanup on drop.
- Set `child_process_for_broker` after `spawn_windows_child()` returns.

### Plan 11-02: `TerminalApproval` Windows branch + tests
**Wave 1:**
- Add `#[cfg(target_os = "windows")]` branch to `TerminalApproval::request_capability()` opening `\\.\CONIN$` instead of `/dev/tty`.

**Wave 2 (tests):**
- Add unit tests for `handle_windows_supervisor_message`: token-valid grant, token-mismatch deny, replay deny, OpenUrl arm, Terminate/Detach arms.
- Add unit test for `TerminalApproval` Windows deny path (no console available — `stderr.is_terminal()` returns false).
- Add unit test that `session_token` never appears in `AuditEntry` serialization.

---

## Sources

### Primary (HIGH confidence)
- `crates/nono/src/supervisor/types.rs` — exact `CapabilityRequest` struct fields, `AuditEntry` shape
- `crates/nono/src/supervisor/socket_windows.rs` — `SupervisorSocket::bind/connect`, `create_named_pipe` SDDL (null = no SACL)
- `crates/nono/src/supervisor/mod.rs` — `ApprovalBackend` trait, test fixtures
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — `handle_windows_supervisor_message`, `start_control_pipe_server`, `WindowsSupervisorRuntime`
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` — `ExecConfig`, `SupervisorConfig`, `WindowsSupervisorDenyAllApprovalBackend`
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `build_child_env`, env var injection mechanism
- `crates/nono-cli/src/terminal_approval.rs` — `TerminalApproval`, `sanitize_for_terminal`
- `crates/nono-cli/src/supervised_runtime.rs` — where `TerminalApproval` and `SupervisorConfig` are wired for Windows
- `crates/nono-proxy/src/token.rs` — `subtle`-based constant-time comparison pattern, token generation pattern
- `crates/nono-proxy/Cargo.toml` — confirms `subtle = "2"` dependency
- `crates/nono/Cargo.toml` — confirms `getrandom = "0.4"`, `Win32_Security_Authorization` already featured
- `crates/nono-cli/Cargo.toml` — confirms `subtle` is absent, must be added

### Secondary (ASSUMED)
- `IsTerminal` trait on stderr works on Windows [ASSUMED: Rust std feature since 1.70]
- `\\.\CONIN$` accessible from background threads when console is attached [ASSUMED: Windows API behavior]
