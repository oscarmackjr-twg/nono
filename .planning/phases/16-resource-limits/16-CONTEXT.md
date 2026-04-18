# Phase 16 Context: Resource Limits (RESL-01..04)

**Gathered:** 2026-04-18
**Status:** Ready for planning
**Source:** Distilled inline from `.planning/REQUIREMENTS.md` RESL-01..04 (user confirmed direct-plan mode; no discuss-phase run).

<domain>
## Phase Boundary

Phase 16 adds **four Job Object resource-limit enforcement knobs** to Windows `nono run`: CPU percentage cap, memory cap, wall-clock timeout, and active-process count cap. All four are exposed as CLI flags that propagate into the existing Named Job Object created by `create_process_containment` (Phase 01 foundation). Non-Windows builds accept the flags with a "not enforced on this platform" warning — the native cgroup/rlimit backends are a follow-up cross-platform milestone explicitly out of scope here.

This phase does NOT:
- Implement cgroup v2 / rlimit on Linux.
- Implement `setrlimit` on macOS.
- Add per-process memory limits (job-wide only).
- Touch the attach-streaming or extended-IPC phases (those are Phases 17 and 18).
- Change the security model of the existing Job Object (WFP, Low-IL, supervisor IPC stay untouched).

</domain>

<decisions>
## Implementation Decisions

### CLI Flag Surface (locked — all four CLI flags are part of the phase's must-haves)

| Flag | Type | Range / Format | REQ-ID |
|------|------|----------------|--------|
| `--cpu-percent <N>` | u16 | `1..=100` | RESL-01 |
| `--memory <SIZE>` | parsed byte-size | `512M`, `1G`, `256K`, or raw bytes `268435456` | RESL-02 |
| `--timeout <DURATION>` | parsed duration | `30s`, `5m`, `1h`, or raw seconds `300` | RESL-03 |
| `--max-processes <N>` | u32 | `1..=65535` | RESL-04 |

- All flags are OPTIONAL (default = no enforcement on that dimension).
- Argument parsing happens via clap, consistent with existing flag handling in `crates/nono-cli/src/cli.rs`.
- Invalid values (`0`, negative, overflow, malformed strings) reject at parse time with a clear error — never reach Job Object setup.

### Windows Enforcement (locked)

- **RESL-01 CPU:** `SetInformationJobObject(..., JobObjectCpuRateControlInformation, ...)`. Set `ControlFlags = JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP`. `CpuRate` field = `percent * 100` (i.e. 10000 = 100%). Hard cap (not soft) so the child cannot exceed the ceiling even when other CPU is idle.
- **RESL-02 Memory:** `SetInformationJobObject(..., JobObjectExtendedLimitInformation, ...)`. Set `LimitFlags |= JOB_OBJECT_LIMIT_JOB_MEMORY` and `JobMemoryLimit = <bytes>`. Job-wide cap (not per-process) because the threat model cares about tree total.
- **RESL-03 Timeout:** Supervisor-side wall-clock timer (spawn a thread or use a tokio timer). On expiry, call `TerminateJobObject(job, exit_code)` with a recognizable exit code (e.g., `STATUS_TIMEOUT = 0x102`). Kernel `JOB_OBJECT_LIMIT_JOB_TIME` is deliberately NOT used as the primary mechanism because it tracks CPU time, not wall-clock time, which doesn't match user intent. Kernel limit MAY be set as a belt-and-suspenders fallback with a sufficiently large value.
- **RESL-04 Process count:** `SetInformationJobObject(..., JobObjectExtendedLimitInformation, ...)`. Set `LimitFlags |= JOB_OBJECT_LIMIT_ACTIVE_PROCESS` and `ActiveProcessLimit = <N>`. Kernel enforces — new `CreateProcess` calls in the job fail with `ERROR_TOO_MANY_PROCESSES` (0x98E) once the limit is hit.

### Enforcement Sequencing (locked)

All four limits are applied to the Job Object **before `ResumeThread`** (before the child is allowed to execute any code). This matches the existing pattern in `crates/nono-cli/src/exec_strategy_windows/launch.rs` where `apply_process_handle_to_containment` + `SetInformationJobObject` calls sequence before `resume_contained_process`. No limit is ever applied after the child has started executing.

### Failure Modes (locked — fail-closed)

- If any of the four `SetInformationJobObject` calls returns zero (failure), the spawn aborts with a `NonoError::SandboxInit` error naming the specific limit and the Win32 last-error code. The suspended child is terminated via `terminate_suspended_process` (existing helper). NO silent degradation.
- If the supervisor-side timeout thread dies (e.g., supervisor panic), the Job Object's existing `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` still guarantees the agent tree dies when the supervisor handle closes. Document this explicitly; it's the safety-net that makes the supervisor-timer approach acceptable.

### Cross-platform Degradation (locked)

On Unix (Linux + macOS), the flags parse and emit a per-flag warning of the form:
```
warning: --cpu-percent is not enforced on linux; the native cgroup backend
is a follow-up (see REQUIREMENTS.md § Cross-platform note). The flag is
accepted for CLI parity with Windows.
```
The flag does NOT silently no-op; the warning is on STDERR and visible unless `--silent` is set. This preserves the "no silent degradation" property for future reviewers who may assume the limit is active.

### Where the Code Lives (locked)

- Flag definitions: `crates/nono-cli/src/cli.rs` (`RunArgs` struct). Shared between `run` and any other commands that want these flags in the future (out-of-scope dimension — don't add to `wrap`/`shell` yet unless the user asks).
- Windows enforcement: `crates/nono-cli/src/exec_strategy_windows/launch.rs` — new function `apply_resource_limits(containment: &ProcessContainment, limits: &ResourceLimits)` called from the supervised spawn path before `ResumeThread`.
- Unix warning path: `crates/nono-cli/src/exec_strategy.rs` (or equivalent unix-side setup) — warn-and-proceed in both the `Direct` and `Supervised` exec strategies.
- Timeout timer: supervisor runtime (`crates/nono-cli/src/supervised_runtime.rs` or `crates/nono-cli/src/exec_strategy_windows/supervisor.rs`) — pick whichever lets the timer share lifetime with the Job Object and the supervisor's event loop.
- Types: `ResourceLimits` struct in `crates/nono-cli/src/launch_runtime.rs` alongside `SessionLaunchOptions` et al. One `Option<T>` field per limit.

### Claude's Discretion

The following are intentionally NOT locked and can be decided by the planner/executor:

- **Byte-size and duration parser crate choice:** Could use an existing `bytesize` crate, a `humantime`-style duration parser, or hand-rolled parsing. Prefer minimal dependencies; if an existing crate is already in `Cargo.lock`, use it.
- **Whether Phase 16 is one plan or two plans:** RESL-01..04 could be one cohesive plan (all four limits land together as a cohesive CLI flag set) OR split into 16-01 (CPU + memory, the "ordinary" limits) and 16-02 (timeout + process count, the "execution-shape" limits). Planner picks based on complexity estimate; one-plan is likely fine because the shape of the Windows API work is identical for all four.
- **Test structure:** Unit tests with `QueryInformationJobObject` read-back, plus integration tests with actual CPU-bound / memory-hungry workloads. Planner decides how to scope the integration-test coverage.
- **Error-message wording:** Follow existing `NonoError::SandboxInit` formatting conventions.

</decisions>

<canonical_refs>
## Canonical References

Downstream agents (planner, executor) MUST read these before planning or implementing:

### Requirements source of truth
- `.planning/REQUIREMENTS.md` RESL-01 through RESL-04 — acceptance criteria, CLI spec, Windows enforcement notes, cross-platform policy.

### Windows Job Object implementation patterns (existing code to mirror)
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `create_process_containment`, `apply_process_handle_to_containment`, `resume_contained_process`, `terminate_suspended_process`. Current Job Object setup path; new limit application slots in between these calls.
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` — `execute_supervised`. Supervisor runtime initialization.
- `crates/nono-cli/src/supervised_runtime.rs` — session lifecycle. Where the supervisor-side timeout timer should live.
- `crates/nono-cli/src/launch_runtime.rs` — `SessionLaunchOptions` + peer structs (`RollbackLaunchOptions`, `TrustLaunchOptions`, `ProxyLaunchOptions`). Template for `ResourceLimits`.

### CLI conventions (existing patterns)
- `crates/nono-cli/src/cli.rs` — `RunArgs` struct. Use the same clap derive style and help-text register.

### Win32 FFI entry points
- `windows-sys` crate (already in `Cargo.toml`):
  - `windows_sys::Win32::System::JobObjects::{JOBOBJECT_CPU_RATE_CONTROL_INFORMATION, JOBOBJECT_BASIC_LIMIT_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectCpuRateControlInformation, JobObjectExtendedLimitInformation, SetInformationJobObject, QueryInformationJobObject, TerminateJobObject}`.
  - Constants: `JOB_OBJECT_CPU_RATE_CONTROL_ENABLE`, `JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP`, `JOB_OBJECT_LIMIT_JOB_MEMORY`, `JOB_OBJECT_LIMIT_ACTIVE_PROCESS`, `JOB_OBJECT_LIMIT_JOB_TIME`.

### Coding standards (project-wide)
- `CLAUDE.md` — unwrap policy, unsafe discipline, path-security principles (not directly relevant to RESL but still applies to all new code).
- Existing `unsafe` pattern in `launch.rs` — every unsafe block carries a `// SAFETY:` comment describing the invariant.

</canonical_refs>

<specifics>
## Specific Implementation Pointers

### Example CLI flow
```bash
# CPU and memory together
nono run --cpu-percent 25 --memory 512M -- python some-agent.py

# With timeout and process cap
nono run --timeout 5m --max-processes 20 -- cargo build --release

# All four
nono run --cpu-percent 50 --memory 1G --timeout 30m --max-processes 50 -- ./agent
```

### Example unit-test shape (RESL-01 readback)
```rust
#[cfg(all(test, target_os = "windows"))]
#[test]
fn cpu_rate_control_readback_matches_applied_value() {
    // 1. create_process_containment → ProcessContainment
    // 2. apply_resource_limits with ResourceLimits { cpu_percent: Some(25), .. }
    // 3. QueryInformationJobObject(JobObjectCpuRateControlInformation)
    // 4. Assert CpuRate == 2500 (== 25 * 100), ControlFlags bits include HARD_CAP | ENABLE
    // 5. Drop ProcessContainment (kills any inherited state cleanly)
}
```

### Example observability in SessionRecord
When a limit is active on a session, include it in the `inspect` output:
```
nono inspect <id>
...
Limits:
  cpu:     25% (hard cap)
  memory:  512 MiB (job-wide)
  timeout: 5 minutes
  procs:   20 (active)
```
This reuses the existing `SessionRecord` serialization path in `crates/nono-cli/src/session.rs`. Add an optional `limits: Option<ResourceLimitsRecord>` field.

</specifics>

<deferred>
## Deferred Ideas

- **Per-process memory cap (`ProcessMemoryLimit`):** Not in v2.1 scope. Follow-up.
- **cgroup v2 backend on Linux:** Explicitly out of scope. A cross-platform RESL milestone is called out in REQUIREMENTS.md.
- **`setrlimit` on macOS:** Same. Warning-only on Unix suffices for v2.1.
- **Dynamic limit adjustment (raise/lower at runtime via IPC):** Phase 11 capability pipe could extend for this; out of scope here.
- **Limits for `nono wrap` and `nono shell`:** Only `nono run` gets the flags in this phase. Adding to other commands is a later cleanup.
- **Global kernel walks / container-level pids.max:** Would require elevated privileges or kernel driver; out of scope.

</deferred>

## Out of Scope for Phase 16

- AIPC-01 extended handle brokering (Phase 18).
- ATCH-01 attach-streaming (Phase 17).
- CLEAN-01..04 (Phase 19).
- Any modification to the existing Job Object containment beyond adding the four new limits.
- Any cross-platform backend for the flags.

---

*Phase: 16-resource-limits*
*Context gathered: 2026-04-18 via direct-plan mode (user skipped discuss-phase; REQUIREMENTS.md was rich enough)*
