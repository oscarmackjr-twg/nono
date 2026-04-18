---
phase: 16-resource-limits
plan: 01
status: complete
executed: 2026-04-18
requirements: [RESL-01, RESL-02, RESL-04]
primary_commits: [070a851, 044eb71]
---

# Plan 16-01 — Summary

## Outcome

**Status:** complete (ready for 16-02)
**Requirements delivered:** RESL-01 (--cpu-percent), RESL-02 (--memory), RESL-04 (--max-processes). RESL-03 (--timeout) flag parsed and threaded through but NOT yet enforced — that's Plan 16-02 Task 1.

## What was done

### Commit `070a851` — CLI flags + ResourceLimits plumbing (Task 1)

* `crates/nono-cli/src/cli.rs`: `parse_byte_size` + `parse_duration` hand-rolled parsers (no bytesize/humantime dep added — neither was in Cargo.lock). Four new `RunArgs` fields: `cpu_percent: Option<u16>` (1..=100), `memory: Option<u64>` (parsed byte-size), `timeout: Option<Duration>` (parsed duration), `max_processes: Option<u32>` (1..=65535). All under `RESOURCE LIMITS` help heading. 10 parser tests covering suffixes, raw values, overflow, clap-level range enforcement.
* `crates/nono-cli/src/launch_runtime.rs`: `ResourceLimits` struct with `Option<T>` per dimension + `from_run_args` + `is_empty`. Wired into `ExecutionFlags` + populated from `run_args` in `prepare_run_launch_plan` (capture `run_args.*` fields BEFORE partial move).
* `crates/nono-cli/src/exec_strategy.rs` (Unix build): `collect_unix_resource_limit_warnings` (pure, returns `Vec<String>`) + `warn_unix_resource_limits` (eprintln! wrapper). 5 `unix_warning_tests` covering silent, per-field emission, Windows no-op.
* `crates/nono-cli/src/exec_strategy_windows/mod.rs`: Windows no-op stubs for the same two helpers so cross-platform callers don't need `#[cfg]` gating.
* `crates/nono-cli/src/supervised_runtime.rs`: `resource_limits: &ResourceLimits` field on `SupervisedRuntimeContext`; warn call at start of `execute_supervised_runtime`.
* `crates/nono-cli/src/execution_runtime.rs`: warn call before strategy dispatch; pass `&flags.resource_limits` into `SupervisedRuntimeContext` AND into Windows `execute_direct`.

### Commit `044eb71` — Windows kernel enforcement (Task 2)

* `crates/nono-cli/src/exec_strategy_windows/launch.rs`: new `apply_resource_limits(&ProcessContainment, &ResourceLimits) -> Result<()>`. Applied AFTER `apply_process_handle_to_containment` and BEFORE `resume_contained_process`. Uses `JobObjectCpuRateControlInformation` (ENABLE | HARD_CAP + CpuRate = percent * 100) and read-modify-write on `JobObjectExtendedLimitInformation` for `JOB_OBJECT_LIMIT_JOB_MEMORY + JobMemoryLimit` and `JOB_OBJECT_LIMIT_ACTIVE_PROCESS + ActiveProcessLimit`. Fail-closed: any FFI failure returns `NonoError::SandboxInit` + terminates the suspended child.
* `spawn_windows_child` signature: added `limits: &ResourceLimits` parameter. The 4 call sites updated (3 wrappers in launch.rs + `execute_supervised`/`execute_direct` in mod.rs). All threaded cleanly.
* 7 `#[cfg(all(test, target_os = "windows"))]` tests in `apply_resource_limits_tests` using `QueryInformationJobObject` readback. Includes the critical `preserves_kill_on_job_close` regression guard (named explicitly per plan-checker Finding #1).

## Verification

| Check | Command | Result |
|-------|---------|--------|
| Build (debug) | `cargo build -p nono-cli --bin nono` | **PASS** |
| Clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | **PASS** (zero warnings) |
| Parser tests | `cargo test -p nono-cli --bin nono -- parser_tests` | **10/10 PASS** |
| Resource-limits tests (Windows) | `cargo test -p nono-cli --bin nono -- apply_resource_limits_tests` | **7/7 PASS** |
| Unix warnings (gated `#[cfg(not(target_os = "windows"))]`) | runs on Unix CI | — (skipped on Windows) |

## Security invariants honored

| # | Invariant | Honored at |
|---|-----------|------------|
| 1 | Limits applied BEFORE `ResumeThread` | launch.rs — `apply_resource_limits` slotted between `apply_process_handle_to_containment` and `resume_contained_process` |
| 2 | Fail-closed on `SetInformationJobObject` / `QueryInformationJobObject` error | launch.rs — both call sites abort with `NonoError::SandboxInit(...)` naming the failing limit + `GetLastError()`, then `terminate_suspended_process` |
| 3 | Unix emits per-flag warning (not silent no-op) | exec_strategy.rs — `warn_unix_resource_limits` + tests covering per-field emission |
| 4 | Memory is JOB_MEMORY (job-wide) NOT ProcessMemoryLimit | launch.rs — `info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_JOB_MEMORY; info.JobMemoryLimit = mem` |
| 5 (Plan 16-02) | Timeout is supervisor-side, NOT kernel JOB_TIME | Not applied in 16-01 by design — deferred to 16-02 Task 1 |
| 6 | Tests gated `#[cfg(all(test, target_os = "windows"))]` with `QueryInformationJobObject` readback | launch.rs `apply_resource_limits_tests` |
| 7 | Staging constraint: pre-existing WIP NOT touched | Commits `070a851` + `044eb71` stage only the 6 `files_modified` files |

## Files changed

| File | Commit | Kind |
|------|--------|------|
| `crates/nono-cli/src/cli.rs` | 070a851 | CLI flags + parsers + parser_tests |
| `crates/nono-cli/src/launch_runtime.rs` | 070a851 | ResourceLimits struct + ExecutionFlags wiring |
| `crates/nono-cli/src/exec_strategy.rs` | 070a851 | Unix warn helpers + unix_warning_tests |
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` | 070a851, 044eb71 | Unix warn stubs + JobObjects imports + execute_direct/supervised param |
| `crates/nono-cli/src/execution_runtime.rs` | 070a851, 044eb71 | Warn call + pass resource_limits into ctx and into execute_direct |
| `crates/nono-cli/src/supervised_runtime.rs` | 070a851, 044eb71 | resource_limits ctx field + wire into Windows execute_supervised |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` | 044eb71 | apply_resource_limits + spawn_windows_child signature + 3 call-site helpers + 7 tests |

## Staging constraint

All 9 pre-existing WIP files (2 modified 11-*.md, 7 untracked) remain untouched on disk. Phase 16 commits only staged files explicitly listed in each plan's `files_modified` frontmatter. CLEAN-03 (Phase 19) will triage them.

## Known remaining work

- RESL-03 (--timeout) enforcement: Plan 16-02 Task 1.
- `SessionRecord.limits` field + `nono inspect` Limits block: Plan 16-02 Task 2.
- End-to-end smoke tests (live CPU-bound / memory-hungry workloads): Plan 16-02 Task 3.

## Status

Plan 16-01 complete. Downstream Plan 16-02 can proceed without blocking.
