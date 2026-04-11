---
phase: 11-runtime-capability-expansion
plan: 01
subsystem: windows-supervisor-ipc
tags: [windows, supervisor, ipc, named-pipe, security, session-token, sddl, low-integrity]
requires:
  - nono::SupervisorSocket::bind/connect (existing)
  - nono::BrokerTargetProcess (existing)
  - nono::CapabilityRequest (existing, extended here)
provides:
  - nono::SupervisorSocket::bind_low_integrity
  - CapabilityRequest.session_token field
  - nono-cli handle_windows_supervisor_message (production)
  - WindowsSupervisorRuntime::start_capability_pipe_server
  - NONO_SESSION_TOKEN / NONO_SUPERVISOR_PIPE env var injection
affects:
  - All Windows supervised runs (add two env vars to child process)
  - Library crate adds Low Integrity-accessible pipe creation path
tech-stack:
  added:
    - subtle 2 (constant-time token comparison, Windows target)
    - getrandom 0.4 (explicit in nono-cli Windows deps; already transitive)
  patterns:
    - SDDL security descriptor via ConvertStringSecurityDescriptorToSecurityDescriptorW
    - Drop guard around PSECURITY_DESCRIPTOR (LocalFree on scope exit)
    - Background pipe server thread with mpsc audit channel drain
    - Constant-time byte comparison via subtle::ConstantTimeEq
    - Audit redaction: clone request, clear session_token, then push
key-files:
  created: []
  modified:
    - crates/nono/src/supervisor/types.rs
    - crates/nono/src/supervisor/mod.rs
    - crates/nono/src/supervisor/socket.rs
    - crates/nono/src/supervisor/socket_windows.rs
    - crates/nono-cli/Cargo.toml
    - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
    - crates/nono-cli/src/exec_strategy_windows/supervisor.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/src/exec_strategy_windows/network.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/src/supervised_runtime.rs
    - crates/nono-cli/src/policy.rs
    - crates/nono-cli/src/trust_keystore.rs
decisions:
  - Token field added with #[serde(default)] — old messages still deserialize
  - SACL is injected via a dedicated bind_low_integrity() method (bool flag rejected to avoid accidental exposure via existing bind() callers)
  - SDDL conversion failure is fail-secure — no null-SD fallback
  - WindowsSupervisorDenyAllApprovalBackend remains defined and is the approval backend used by the capability pipe server thread in plan 11-01 (TerminalApproval wiring deferred to plan 11-02)
  - Capability pipe server only starts when BOTH session_token and cap_pipe_rendezvous_path are Some on SupervisorConfig — satisfies SC #4
  - Audit entries always go through audit_entry_with_redacted_token() — token never crosses the audit log boundary
metrics:
  duration: ~2h
  completed: 2026-04-11
  tasks: 3
  commits: 3
---

# Phase 11 Plan 01: Runtime Capability Expansion IPC Wiring (Windows) Summary

Windows sandboxed children can now request additional filesystem capabilities
at runtime by posting a `RequestCapability` message to the supervisor over a
dedicated Low Integrity-accessible named pipe, authenticated by a per-session
constant-time token, with all decisions funneled through a redacting audit
log. Interactive approval UX (TerminalApproval Windows branch) remains in
plan 11-02.

## What changed

### Library (`crates/nono`)

- `CapabilityRequest` gained a `session_token: String` field with
  `#[serde(default)]`. The field is documented as "never log".
- `SupervisorSocket::bind_low_integrity(path)` added on Windows. Mirrors the
  existing `bind()` path but attaches the SDDL
  `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;LW)` via
  `ConvertStringSecurityDescriptorToSecurityDescriptorW`. The resulting
  `PSECURITY_DESCRIPTOR` is released via a Drop guard (`LocalFree` on scope
  exit). Failure is fail-secure — no null-SD fallback.
- New Windows-only unit test `test_bind_low_integrity_roundtrip` exercises
  bind + connect + message round-trip across threads, using an mpsc ping to
  coordinate server shutdown.
- New JSON round-trip test
  `test_capability_request_json_round_trip_preserves_session_token`.
- Every in-library/test `CapabilityRequest` literal updated to set
  `session_token`.

### CLI (`crates/nono-cli`)

- `subtle = "2"` and `getrandom = "0.4"` added under Windows target
  dependencies.
- `handle_windows_supervisor_message` and `open_windows_supervisor_path`
  ungated from `#[cfg(test)]` to production.
- `handle_windows_supervisor_message` now takes `expected_session_token:
  &str` and performs a `subtle::ConstantTimeEq`-backed comparison BEFORE
  the approval backend is consulted. Mismatch returns
  `ApprovalDecision::Denied { reason: "Invalid session token" }` and never
  calls the backend. Invocation count is preserved as unit-test evidence.
- `audit_entry_with_redacted_token()` helper clones the inbound request,
  zeroes the `session_token`, and builds the `AuditEntry`. All three audit
  push sites in the handler go through it (replay path, token-mismatch
  path, approval-backend path).
- `WindowsSupervisorRuntime` gained four fields (`session_token`,
  `cap_pipe_rendezvous_path`, `audit_rx`, `child_process_for_broker`) and
  two new methods: `start_capability_pipe_server()` and
  `set_child_broker_target(HANDLE)`. The capability pipe server thread is
  started only when both a token and a rendezvous path are present on
  `SupervisorConfig` (otherwise the deny-all fallback remains the effective
  approval surface per SC #4).
- The capability pipe server thread: binds
  `SupervisorSocket::bind_low_integrity`, polls for the spawned child's
  process handle, wraps it via
  `BrokerTargetProcess::from_raw_handle`, then loops
  `recv_message → handle_windows_supervisor_message` and pushes audit
  batches over mpsc. Exits on pipe EOF or `terminate_requested`.
- `run_child_event_loop` drains the mpsc receiver on every iteration via
  `drain_capability_audit_entries()`.
- `execute_supervised` calls `runtime.set_child_broker_target(
  child.process_handle_raw())` immediately after `spawn_windows_child()`
  returns, so the pipe thread can start brokering handles.
- `Drop for WindowsSupervisorRuntime` best-effort removes the rendezvous
  file on session teardown.
- `ExecConfig` (Windows) and `SupervisorConfig` carry
  `session_token`/`cap_pipe_rendezvous_path`. Test-only constructors in
  `exec_strategy_windows/network.rs` updated accordingly.
- `execution_runtime.rs` generates per-session credentials on Windows:
  32 random bytes via `getrandom::fill`, hex-encoded to a 64-char owned
  `String`; rendezvous path `{temp}/nono-cap-{session_id}.pipe`. Both are
  pushed into `env_vars` as `NONO_SESSION_TOKEN` and `NONO_SUPERVISOR_PIPE`.
  Owned strings are declared before `env_vars` so the `'a` lifetime holds.
- `supervised_runtime.rs` forwards the new `ExecConfig` fields into
  `SupervisorConfig` as `&str`/`&Path` views.

### Unit tests (Windows-only handler)

Five new tests under `exec_strategy_windows::supervisor::capability_handler_tests`:

1. `handle_rejects_missing_token` — empty token → denied, backend call
   count stays 0, audit redacted.
2. `handle_rejects_wrong_token` — wrong token → denied, backend call count
   stays 0.
3. `handle_consults_backend_for_valid_token` — matching token → backend
   invoked exactly once; denial reason carries the backend's message.
4. `handle_redacts_token_in_audit_entry_json` — serializing the
   `AuditEntry` to JSON and scanning for the secret token string returns
   not-found. The audit entry's own `request.session_token` is empty.
5. `handle_rejects_replay_with_valid_token` — same request_id twice with
   a valid token; second call denies with a replay reason, backend call
   count remains 1.

## Verification

```
cargo check -p nono           # OK
cargo check -p nono-cli       # OK
cargo test  -p nono    --lib supervisor                                # 16 passed (incl. new tests)
cargo test  -p nono-cli --bin nono exec_strategy::supervisor::capability_handler_tests  # 5 passed
cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used         # clean
cargo fmt --all -- --check                                              # clean
```

Grep audit for token leakage in format strings and logging macros:

```
grep -nE "tracing::.*session_token|format!.*session_token|println!.*session_token|eprintln!.*session_token" \
  crates/nono-cli/src/exec_strategy_windows/supervisor.rs \
  crates/nono-cli/src/execution_runtime.rs \
  crates/nono-cli/src/exec_strategy_windows/mod.rs
# (no matches)
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] Pre-existing test compile errors in nono-cli**
- **Found during:** Task 2 verification
- **Issue:** `cargo test -p nono-cli` (needed to run the new capability
  handler tests) failed at compile time with three unrelated errors that
  existed on `main` before this plan:
  - `crates/nono-cli/src/policy.rs:1850` and `:2763` — two
    `#[test]` functions used `std::os::unix::fs::symlink` without a
    `#[cfg(unix)]` gate, so the Windows build path refused to compile
    the test binary.
  - `crates/nono-cli/src/trust_keystore.rs:393` — a Windows-only test
    called a non-existent `backend_description("nono-trust")` helper.
- **Fix:** Gated both policy.rs tests with `#[cfg(unix)]` and rewrote
  the trust_keystore test to call `backend_description_for_ref(&key_ref,
  "nono-trust")` with a valid `TrustKeyRef::Keystore("default")`.
- **Files modified:** `crates/nono-cli/src/policy.rs`,
  `crates/nono-cli/src/trust_keystore.rs`
- **Commit:** c01be2e
- **Why auto-fixed:** These compile errors blocked the plan's own
  `<automated>` verification command. Scope: tests only, no production
  logic.

**2. [Rule 2 — Critical] Stale `#[cfg(test)]` on `SystemTime` import**
- **Found during:** Task 2
- **Issue:** `crates/nono-cli/src/exec_strategy_windows/mod.rs` had
  `#[cfg(test)] use std::time::SystemTime;`. After ungating
  `handle_windows_supervisor_message` to production, `SystemTime` is
  needed at production compile time (audit entries carry a timestamp).
- **Fix:** Removed the `#[cfg(test)]` gate on the import.
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/mod.rs`
- **Commit:** c01be2e

### Planned but adjusted

- The plan described placing `constant_time_eq` at "the top of the
  module". I placed it directly above `handle_windows_supervisor_message`
  to minimize diff surface and keep the helper adjacent to its single
  caller. Behavior identical.
- The plan suggested an "Option A oneshot" or "Option B restructure" for
  delivering the child process handle to the pipe thread. I implemented
  a lighter variant: `Arc<Mutex<Option<SendableHandle>>>` polled by the
  thread with a 50ms sleep until a value arrives (or `terminate_requested`
  is set). This is equivalent to Option A without adding a dedicated
  oneshot crate, and matches the existing `start_control_pipe_server`
  style in the same file.

### Auth gates

None.

## Self-Check

Created files:

```
[ -f .planning/phases/11-runtime-capability-expansion/11-01-SUMMARY.md ]  → FOUND
```

Commits exist on `windows-squash`:

```
git log --oneline -3
032f406 feat(11-01): wire capability pipe server + env var injection
c01be2e feat(11-01): ungate capability handler + constant-time token check
628aa0e feat(11-01): add session_token + low-integrity pipe variant
```

Acceptance criteria grep matches:

- `session_token` in `crates/nono/src/supervisor/types.rs`:  present inside
  `CapabilityRequest`.
- `bind_low_integrity` in `crates/nono/src/supervisor/socket_windows.rs`:
  5 occurrences (method declaration, impl routing, call site, test usage,
  low-integrity helper).
- `S:(ML;;NW;;;LW)` in `crates/nono/src/supervisor/socket_windows.rs`:
  exactly one occurrence (the `CAPABILITY_PIPE_SDDL` constant).
- `^subtle` in `crates/nono-cli/Cargo.toml`: one match under Windows
  target dependencies.
- `start_capability_pipe_server` in `exec_strategy_windows/supervisor.rs`:
  2 occurrences (definition + `initialize()` call site).
- `NONO_SESSION_TOKEN` / `NONO_SUPERVISOR_PIPE` in
  `execution_runtime.rs`: one `env_vars.push` for each, plus doc comments.
- `WindowsSupervisorDenyAllApprovalBackend` still defined in
  `exec_strategy_windows/mod.rs` (SC #4 preserved).
- `audit_rx.try_recv` drain present inside `run_child_event_loop` (via
  `drain_capability_audit_entries`).
- `grep -nE "tracing::.*session_token|format!.*session_token"` across
  changed files: 0 matches.

## Self-Check: PASSED
