---
phase: 09-wfp-port-level-proxy-filtering
plan: "02"
subsystem: nono-cli/execution_runtime
tags: [proxy, fail-secure, windows, guard]
dependency_graph:
  requires: []
  provides: [PROXY-01-guard]
  affects: [execution_runtime.rs, proxy_runtime.rs]
tech_stack:
  added: []
  patterns: [fail-secure guard, pre-flight validation]
key_files:
  created: []
  modified:
    - crates/nono-cli/src/execution_runtime.rs
decisions:
  - "Guard placed BEFORE start_proxy_runtime so it fires before any WFP/sandbox activation, capturing user intent directly (D-02)"
  - "Used !proxy.active (not proxy_handle.is_none()) to avoid relying on post-mutation signal"
  - "interactive_shell field added to Windows ExecConfig initializer using flags.interactive_shell (pre-existing missing field)"
metrics:
  duration: "~10 minutes"
  completed: "2026-04-10"
  tasks_completed: 1
  tasks_total: 1
  files_modified: 1
requirements:
  - PROXY-01
---

# Phase 09 Plan 02: ProxyOnly Pre-flight Guard Summary

**One-liner:** Fail-secure guard in execution_runtime.rs rejects ProxyOnly mode when proxy.active is false, firing before start_proxy_runtime and WFP activation.

## What Was Built

Added a pre-flight guard to `crates/nono-cli/src/execution_runtime.rs` that enforces the D-02 requirement: if the user's caps are in `ProxyOnly` mode (set by a profile at capability_ext.rs:535) but `proxy.active` is false (no network profile, no credentials, no upstream proxy), execution is rejected with a clear `SandboxInit` error before any proxy runtime or WFP setup runs.

### Guard Location

Inserted at line 165 (before `let active_proxy = start_proxy_runtime(proxy, &mut caps)?`):

```rust
// Fail-secure guard (D-02): if caps were set to ProxyOnly by a profile or
// credential path but proxy.active is false, fail before start_proxy_runtime
// and before any WFP/sandbox activation.
if matches!(caps.network_mode(), nono::NetworkMode::ProxyOnly { .. }) && !proxy.active {
    return Err(NonoError::SandboxInit(
        "Cannot use proxy-only mode without a network profile or credential configuration."
            .to_string(),
    ));
}
```

### Why This Is Correct

- `proxy.active` is computed in `prepare_proxy_launch_options` (proxy_runtime.rs:52-78) before `start_proxy_runtime` is called — it reflects configured intent, not post-mutation state.
- `start_proxy_runtime` explicitly does NOT mutate caps when `proxy.active == false` (proxy_runtime.rs:186-191), so a pre-call check sees the same network mode a post-call check would — but fires before any runtime setup.
- Profile path sets `ProxyOnly { port: 0, ... }` as placeholder (capability_ext.rs:535); if no credentials/domain/upstream resolve, `proxy.active` evaluates to `false` — this combination is the exact failure mode D-02 closes.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed missing `interactive_shell` field in Windows ExecConfig**

- **Found during:** Task 1 (cargo check)
- **Issue:** `exec_strategy::ExecConfig` on Windows gained an `interactive_shell: bool` field (added during Phase 08 / shell work) but the initializer in `execute_sandboxed` was not updated, causing a compile error that blocked verification.
- **Fix:** Added `interactive_shell: flags.interactive_shell` to the Windows `ExecConfig` initializer at execution_runtime.rs line 282. `ExecutionFlags` already had this field (launch_runtime.rs:99); the fix connects it.
- **Files modified:** crates/nono-cli/src/execution_runtime.rs
- **Commit:** b316c70 (same commit as task)

## Verification Results

- `cargo check -p nono-cli` — passes
- `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` — passes, no warnings
- `cargo fmt --all -- --check` — passes
- Guard string "Cannot use proxy-only mode without a network profile or credential configuration" confirmed in execution_runtime.rs
- `!proxy.active` appears BEFORE `start_proxy_runtime` call

## Known Stubs

None — the guard is fully wired and active.

## Threat Flags

None — no new network endpoints or auth paths introduced. The guard closes threat T-09-03 (Elevation of Privilege) from the plan's threat model.

## Self-Check: PASSED

- File modified: `crates/nono-cli/src/execution_runtime.rs` — FOUND
- Commit b316c70 — FOUND
- Guard string present — FOUND
- Compile clean — VERIFIED
