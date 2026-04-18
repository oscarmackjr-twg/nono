---
phase: 16-resource-limits
reviewed: 2026-04-18T00:00:00Z
depth: standard
files_reviewed: 11
files_reviewed_list:
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/exec_strategy_windows/mod.rs
  - crates/nono-cli/src/exec_strategy_windows/supervisor.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/launch_runtime.rs
  - crates/nono-cli/src/session.rs
  - crates/nono-cli/src/session_commands.rs
  - crates/nono-cli/src/session_commands_windows.rs
  - crates/nono-cli/src/supervised_runtime.rs
findings:
  critical: 0
  warning: 3
  info: 5
  total: 8
status: issues_found
---

# Phase 16: Code Review Report

**Reviewed:** 2026-04-18
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Summary

Phase 16 (Resource Limits) wires Windows Job Object CPU/memory/max-processes
caps plus a supervisor-side `--timeout` timer through the CLI. Coverage of the
security-critical touch points is strong:

- **Job Object flag preservation** — `apply_resource_limits` uses a genuine
  read-modify-write on `JobObjectExtendedLimitInformation`
  (`launch.rs:231-287`), OR-ing in `JOB_OBJECT_LIMIT_JOB_MEMORY` /
  `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` while preserving the
  `KILL_ON_JOB_CLOSE | DIE_ON_UNHANDLED_EXCEPTION` flags set by
  `create_process_containment`. A regression test (`preserves_kill_on_job_close`)
  guards this contract explicitly.
- **Ordering** — resource limits are applied AFTER `AssignProcessToJobObject`
  and BEFORE `ResumeThread` (`launch.rs:1189-1200`), so the child never runs
  without caps. Fail-closed: on any FFI failure the suspended child is
  terminated before the error propagates.
- **FFI / HANDLE hygiene** — `OwnedHandle` / `ProcessContainment` own their
  closes exactly once on drop; no double-close was introduced. The borrowed
  `containment_job` HANDLE on the supervisor runtime is documented and the
  drop-order comment in `mod.rs:632-635` correctly ensures `containment`
  outlives the runtime.
- **Serde backward compat** — `SessionRecord.limits: Option<ResourceLimitsRecord>`
  uses `#[serde(default, skip_serializing_if = "Option::is_none")]`, and two
  regression tests (`session_record_deserializes_without_limits_field`,
  `session_record_deserializes_with_populated_limits`) pin both directions.
- **Overflow safety** — `parse_byte_size` and `parse_duration` both use
  `checked_mul` and reject `0`; `compute_deadline` uses `Instant::checked_add`
  and returns a typed error on `u64::MAX`. No `.unwrap()` / `.expect()` outside
  `#[cfg(test)]` was introduced on the non-test paths I reviewed.
- **No token leaks** — the session-token constant-time check already in place
  is unaffected; the new `ResourceLimitsRecord` doesn't carry any secrets.

Three issues below concern lifetime/liveness of resources when the new timeout
path fires. The most important (WR-01) is a shared HANDLE that the capability
pipe thread may retain after the child is reaped by `TerminateJobObject`; it
is reachable via the new `--timeout` path and was only weakly reachable before
Phase 16. None are structural security regressions — they are robustness
issues under the new timeout-driven exit.

## Warnings

### WR-01: Capability pipe server thread may outlive child process handle

**File:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:338-441`, `843-910`
**Issue:** `start_capability_pipe_server` spawns a detached thread that caches the
child process `HANDLE` inside a `BrokerTargetProcess` built from the raw handle
(lines 389-391). The thread loops on `recv_message` and only breaks on pipe
error or `terminate_requested`. When `run_child_event_loop` exits because
`--timeout` fired (lines 874-893) OR because the child exited naturally
(line 896-908), neither path sets `terminate_requested` nor joins the pipe
thread. When the caller drops `WindowsSupervisorRuntime`, the `OwnedHandle`
wrappers on `WindowsSupervisedChild` close the child HANDLE — but the pipe
thread's cached `target.0` is now a dangling handle. The thread could still
receive a message and pass this handle into `DuplicateHandle`, either failing
with `ERROR_INVALID_HANDLE` or (worse) binding to a recycled handle of another
object.

Phase 16 does not introduce this shared-HANDLE pattern (it existed in Plan
11-02), but the new `--timeout` path in `run_child_event_loop` (lines 874-893)
adds a second exit path that short-circuits the natural `wait_for_exit(100)`
drain without setting `terminate_requested`, making the stale-handle window
easier to hit. The loop calls `TerminateJobObject`, returns `Completed`, and
drops the runtime — the spawned capability pipe thread is never signaled.

**Fix:** In both exit paths (timeout at line 891-892 and natural exit at
line 897-908), set `self.terminate_requested.store(true, Ordering::SeqCst)`
before `self.shutdown()`. Also set it inside `WindowsSupervisorRuntime::drop`
(line 948). This will cause the pipe thread to exit its `loop` on the next
iteration rather than dispatch another message against a potentially dead
child HANDLE. Ideally also `JoinHandle`-track the thread and `.join()` it in
`shutdown()`.

```rust
// supervisor.rs:874-893 (timeout path)
if std::time::Instant::now() >= deadline {
    // ... tracing ...
    self.terminate_requested.store(Ordering::SeqCst, true); // ADD
    if let Err(err) = super::launch::terminate_job_object(...) {
        // ...
    }
    // ... existing shutdown ...
}

// supervisor.rs:896-908 (natural exit path)
if let Some(exit_code) = child.wait_for_exit(100)? {
    self.terminate_requested.store(true, Ordering::SeqCst); // ADD
    self.state = WindowsSupervisorLifecycleState::ShuttingDown;
    self.shutdown();
    // ...
}
```

### WR-02: Dead-code helpers updated with new `limits` parameter but still have no callers

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:951-977`, `1208-1229`, `1231-1252`
**Issue:** `execute_direct_with_low_integrity`, `spawn_supervised_with_low_integrity`,
and `spawn_supervised_with_standard_token` are all `pub(super)` but have no
callers anywhere in the crate (verified by `grep`). Phase 16 added a
`limits: &ResourceLimits` parameter to each of them. The module has
`#![allow(dead_code)]` at the top (`mod.rs:1`) so the compiler won't flag it.

Per CLAUDE.md ("Lazy use of dead code: avoid `#[allow(dead_code)]`. If code is
unused, either remove it or write tests that use it."), this is a process /
hygiene violation. More practically: dead signatures rot — the three helpers
here all take `limits` but none is covered by the `apply_resource_limits_tests`
integration tests that prove the caps actually flow through, so a future
refactor that revives one of them could silently drop `limits` without a test
failure.

**Fix:** Either delete the three functions (they are close in shape to
`spawn_windows_child`, which is the live caller), or remove
`#![allow(dead_code)]` from `mod.rs` and let the compiler name the dead
symbols so they can be removed individually.

### WR-03: `--timeout` deadline race against child natural exit

**File:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:860-909`
**Issue:** The event loop checks `timeout_deadline` (line 874) BEFORE the
`child.wait_for_exit(100)` call (line 896). On a child that exits at
approximately the same `Instant` as the deadline, the code will always observe
the deadline-expired branch first and fire `TerminateJobObject` on an
already-exiting process tree. The supervisor then reports
`STATUS_TIMEOUT_EXIT_CODE` (`0x102`) instead of the child's actual exit code.

This is a minor UX / observability issue rather than a correctness bug — the
comment on lines 870-872 acknowledges ±100ms accuracy — but it's worth calling
out that a child that exits cleanly at the deadline will have its exit code
masked by the timeout. Also note that `TerminateJobObject` on an empty job
still succeeds (it's idempotent), so there is no secondary error.

**Fix:** Call `child.poll_exit_code()` (non-blocking) BEFORE the deadline
check. If the child has already exited, return its actual code without firing
the timeout path:

```rust
loop {
    self.drain_capability_audit_entries();
    if self.terminate_requested.load(Ordering::SeqCst) { ... }

    // NEW: prefer natural exit when both conditions fire on the same tick.
    if let Some(exit_code) = child.poll_exit_code()? {
        // ... natural-exit shutdown ...
        return Ok(exit_code);
    }

    if let Some(deadline) = self.timeout_deadline {
        if std::time::Instant::now() >= deadline { ... }
    }

    if let Some(exit_code) = child.wait_for_exit(100)? { ... }
}
```

## Info

### IN-01: Duplicated `format_bytes_human` / `format_duration_human` across Unix and Windows session-command modules

**File:** `crates/nono-cli/src/session_commands.rs:322-357`, `crates/nono-cli/src/session_commands_windows.rs:448-483`
**Issue:** The two helpers are byte-for-byte identical copies. Both were
introduced in the same Phase 16 commit (`39ee157 feat(16-02): SessionRecord.limits
+ nono inspect Limits block`). Each file also carries its own identical
`inspect_formatting_tests` module with the same ~12 assertions, so the
duplication extends to the tests.

**Fix:** Extract to a single module — e.g., a new `session_commands_format.rs`
or place the helpers in `session.rs` alongside `ResourceLimitsRecord`. The
tests can live beside the shared implementation.

### IN-02: Named pipe / rendezvous path placed in attacker-influenceable TEMP

**File:** `crates/nono-cli/src/execution_runtime.rs:209-213`
**Issue:** `windows_cap_pipe_path = std::env::temp_dir().join(format!("nono-cap-{}.pipe", flags.session.session_id))`.
Per CLAUDE.md ("Validate environment variables before use. Never assume HOME,
TMPDIR, etc. are trustworthy."), `TMP` / `TEMP` / `TMPDIR` can be set by the
parent process. The session_id is 16 hex chars so pre-creation collisions are
effectively impossible, but an attacker who controls TEMP can place a
directory named `nono-cap-<known-id>.pipe` that causes `SupervisorSocket::bind_low_integrity`
to fail — no confidentiality impact since the session token is still required
to send a message, but a denial-of-service on runtime-capability-expansion is
possible.

**Fix (defense-in-depth):** Prefer `dirs::cache_dir()` (falls back to
`%LOCALAPPDATA%\nono\cache` on Windows) or explicitly canonicalize under
`~/.nono/sessions/` where other session artifacts already live (see
`session::sessions_dir()`). Phase 11-02 owned the original decision to use
TEMP; not a Phase-16 regression, but worth logging here since the Windows
token+rendezvous wiring in `execution_runtime.rs` was restructured.

### IN-03: Overflow comment on `CpuRate` computation is imprecise

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:210-211`
**Issue:** The comment reads:

> CpuRate field lives inside an anonymous union representing CpuRate xor MinRate/MaxRate.
> 100% == 10000; percent * 100 is safe for u16 → u32 since 1..=100 * 100 <= 10000.

The math is correct but the type annotation "`u16 → u32`" is misleading —
what actually happens is `u32::from(percent) * 100`, performed as u32
arithmetic. The ceiling `100 * 100 = 10_000` is nowhere near `u32::MAX`. The
clap range `1..=100` (`cli.rs:1450`) makes this unconditionally safe, but the
comment could be clearer.

**Fix:** Reword to:

```
// Clap enforces 1..=100; 100 * 100 = 10_000 fits in u32 trivially.
// Multiplication is performed as u32 after widening from u16.
info.Anonymous.CpuRate = u32::from(percent) * 100;
```

### IN-04: `ResourceLimits` is not covered by a fuzz / property-based test

**File:** `crates/nono-cli/src/launch_runtime.rs:104-135`, `crates/nono-cli/src/cli.rs:15-81`
**Issue:** The parsers (`parse_byte_size`, `parse_duration`) and the limits
struct (`ResourceLimits::from_run_args`, `is_empty`) have good unit-test
coverage, but no property-based test exercises the round-trip `RunArgs →
ResourceLimits → ResourceLimitsRecord → JSON → ResourceLimitsRecord`. The
workspace already depends on `proptest` (per CLAUDE.md Technology Stack) so
the cost is low.

**Fix (optional, follow-up):** Add a `proptest!` that generates arbitrary
`(Option<u16>, Option<u64>, Option<Duration>, Option<u32>)` tuples, builds a
`ResourceLimits`, walks through `ResourceLimitsRecord::from_resource_limits`,
serializes via `serde_json`, deserializes back, and asserts equality. This
would catch any future drift between the in-memory and on-disk representations.

### IN-05: `execute_direct` Windows path still uses a tight `poll_exit_code` loop instead of a blocking wait

**File:** `crates/nono-cli/src/exec_strategy_windows/mod.rs:593-598`
**Issue:** The direct path polls `child.poll_exit_code()` with a 100ms sleep
between ticks:

```rust
loop {
    if let Some(exit_code) = child.poll_exit_code()? { return Ok(exit_code); }
    std::thread::sleep(WINDOWS_SUPERVISOR_POLL_INTERVAL);
}
```

This is the pre-existing pattern (not a Phase 16 introduction) and is not a
bug — `poll_exit_code` calls `WaitForSingleObject(handle, 0)`, so the 100ms
sleep bounds busy-waiting. It DOES mean `nono run --cpu-percent=N -- some-fast-exiting-process`
adds up to 100ms to the observed exit path. Phase 16 didn't change this
behavior on the direct path, but the supervised path likewise uses
`wait_for_exit(100)` (`supervisor.rs:896`) so both are already converged on a
100ms quantum.

**Fix (optional):** Replace the poll/sleep loop with
`WaitForSingleObject(handle, INFINITE)` followed by `GetExitCodeProcess`. The
supervised path cannot because it needs to drain audit entries and honor
`terminate_requested`, but `execute_direct` has no such concurrent concerns.

---

_Reviewed: 2026-04-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
