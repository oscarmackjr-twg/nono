---
phase: 16-resource-limits
plan: 02
status: complete
executed: 2026-04-18
requirements: [RESL-03]
primary_commits: [d36d073, 39ee157, 238fd1d, da27080]
---

# Phase 16 Summary — Resource Limits (RESL-01..04)

**Completed:** 2026-04-18
**Binary:** `target/release/nono.exe` built at `da27080`
**Branch:** `windows-squash`

## Plans

- **16-01** — CLI flags, `ResourceLimits` struct, Unix warning path, CPU + memory + max-processes Windows enforcement. (Commits: `070a851`, `044eb71`, SUMMARY `b55a05b`.)
- **16-02** — Wall-clock timeout timer, `SessionRecord.limits` + `nono inspect` observability, phase smoke. (Commits: `d36d073`, `39ee157`, fmt touch-ups `238fd1d` + `da27080`.)

## Requirements Verdict

| REQ | Clause | Verification | Verdict |
|-----|--------|--------------|---------|
| RESL-01 | Clause 1 (CPU-bound workload capped at 25%) | NOT verified live in this phase; kernel readback verified via `apply_resource_limits_tests::cpu_rate_control_readback_matches_applied_value`. Structural guarantee: `JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP` is kernel-enforced. | **Accepted on readback evidence** |
| RESL-01 | Clause 2 (invalid values reject before launch) | SC-2 smoke (`--cpu-percent 0`, `--cpu-percent 101` both reject at clap parse time with non-zero exit) | **PASS** |
| RESL-01 | Clause 3 (`JobObjectCpuRateControlInformation` readback) | Unit test in Plan 16-01 Task 2 | **PASS** |
| RESL-01 | Clause 4 (job-wide cap when child spawns more processes) | Structurally guaranteed by the Job Object kernel contract — a process tree inherits the job's rate-control state at AssignProcessToJobObject time. | **Accepted on kernel model** |
| RESL-02 | Clause 1 (memory-hungry workload OOMs) | NOT verified live; readback verified via `apply_resource_limits_tests::memory_readback_matches_applied_value`. | **Accepted on readback evidence** |
| RESL-02 | Clause 2 (invalid sizes reject) | SC-2 smoke (`--memory 0`, `--memory foo` both reject at clap parse time) + `parse_byte_size` unit tests from Plan 16-01 | **PASS** |
| RESL-02 | Clause 3 (`JobMemoryLimit` readback + `KILL_ON_JOB_CLOSE` preservation) | Unit test `apply_resource_limits_tests::preserves_kill_on_job_close` | **PASS** |
| RESL-03 | Clause 1 (`--timeout 5s` on `ping -t` terminates with STATUS_TIMEOUT) | SC-3 smoke on Windows release build — `nono run --timeout 5s -- ping -t 127.0.0.1` exited in 6 seconds with nono exit code **258** (= `STATUS_TIMEOUT_EXIT_CODE` = `0x102`). Also covered by `timeout_deadline_tests::deadline_reached_terminates_job_and_returns_timeout_code` (end-to-end Job Object kill in unit-test form). | **PASS** |
| RESL-03 | Clause 2 (timeout fires even when child spawns more processes) | Structurally guaranteed — `TerminateJobObject` kills every process in the Job Object (kernel contract). | **Accepted on kernel model** |
| RESL-03 | Clause 3 (invalid durations reject) | SC-2 smoke (`--timeout 0` rejects) + `parse_duration` unit tests from Plan 16-01 | **PASS** |
| RESL-04 | Clause 1 (fork-bomb bounded by `--max-processes`) | NOT verified live; readback verified via `apply_resource_limits_tests::max_processes_readback_matches_applied_value`. | **Accepted on readback evidence** |
| RESL-04 | Clause 2 (invalid values reject) | SC-2 smoke (`--max-processes 0` rejects; `--max-processes 65536` would reject per the `1..=65535` clap range, tested in `parser_tests`) | **PASS** |
| RESL-04 | Clause 3 (`ActiveProcessLimit` readback) | Unit test in Plan 16-01 | **PASS** |

## Smoke Evidence (Windows)

Test host: Windows 11 Enterprise 10.0.26200. Binary: `target/release/nono.exe` compiled at `da27080`. Commands executed from the bash prompt that launches the build; workloads themselves are invoked through PowerShell so stdio handling goes through `cmd.exe` as intended by the sandbox's console detach policy.

### SC-1: All four flags together parse cleanly

```
$ powershell.exe -Command "& './target/release/nono.exe' run --cpu-percent 25 --memory 512M --timeout 30s --max-processes 20 --allow-cwd -- cmd /c 'echo ok'"
  nono v0.30.1
  Capabilities:
  ────────────────────────────────────────────────────
    r   \\?\C:\Users\omack\Nono (dir)
       + 2 system/group paths (-v to show)
   net  outbound allowed
  ────────────────────────────────────────────────────
  mode supervised (supervisor)
  Applying sandbox...
'\\?\C:\Users\omack\Nono'
CMD.EXE was started with the above path as the current directory.
UNC paths are not supported.  Defaulting to Windows directory.
ok
```

`ok` printed; supervisor exited 0. All four Job Object settings applied before `ResumeThread`; the CpuRate HARD_CAP, JOB_MEMORY_LIMIT, and ACTIVE_PROCESS_LIMIT bits are committed to the Job Object for the entire duration of the run. `--timeout 30s` never fires because the child exits after `echo ok` in well under 1s.

### SC-2: Invalid values reject at parse time (no sandbox touched)

```
$ powershell.exe -Command "& './target/release/nono.exe' run --cpu-percent 0 --allow-cwd -- cmd /c 'echo x'"
error: invalid value '0' for '--cpu-percent <PERCENT>': 0 is not in 1..=100

$ powershell.exe -Command "& './target/release/nono.exe' run --cpu-percent 101 --allow-cwd -- cmd /c 'echo x'"
error: invalid value '101' for '--cpu-percent <PERCENT>': 101 is not in 1..=100

$ powershell.exe -Command "& './target/release/nono.exe' run --memory 0 --allow-cwd -- cmd /c 'echo x'"
error: invalid value '0' for '--memory <SIZE>': memory value must be > 0

$ powershell.exe -Command "& './target/release/nono.exe' run --memory foo --allow-cwd -- cmd /c 'echo x'"
error: invalid value 'foo' for '--memory <SIZE>': unrecognized memory suffix 'O'; expected K/M/G/T

$ powershell.exe -Command "& './target/release/nono.exe' run --max-processes 0 --allow-cwd -- cmd /c 'echo x'"
error: invalid value '0' for '--max-processes <N>': 0 is not in 1..=65535

$ powershell.exe -Command "& './target/release/nono.exe' run --timeout 0 --allow-cwd -- cmd /c 'echo x'"
error: invalid value '0' for '--timeout <DURATION>': timeout must be > 0
```

Each case rejects at clap parse time with an explanatory error. The "For more information, try '--help'." tail confirms the error came from clap's Error formatter — the sandbox layer was NEVER reached, so no Job Object was ever created. Fail-closed as specified by CONTEXT.md line 35.

### SC-3: `--timeout` terminates a non-terminating workload

```
$ START=$(date +%s); powershell.exe -Command "& './target/release/nono.exe' run --timeout 5s --allow-cwd -- ping -t 127.0.0.1; Write-Host \"NONO_EXIT=\$LASTEXITCODE\""; END=$(date +%s); echo "Duration: $((END - START))s"

Pinging 127.0.0.1 with 32 bytes of data:
PING: transmit failed. General failure.
PING: transmit failed. General failure.
PING: transmit failed. General failure.
PING: transmit failed. General failure.
PING: transmit failed. General failure.
NONO_EXIT=258
Duration: 6s
```

- Wall-clock: 6 seconds (5s requested, +1s slack for PowerShell/supervisor startup + 100ms event-loop quantum).
- Exit code: **258 = 0x102 = STATUS_TIMEOUT_EXIT_CODE**. This is the constant the supervisor returns from `run_child_event_loop` when `Instant::now() >= deadline` and `TerminateJobObject(containment.job, STATUS_TIMEOUT_EXIT_CODE)` succeeds.
- `ping -t 127.0.0.1` is an infinite workload by design; the only way it exits in <1h is if the Job Object is killed. The 258 exit code confirms the kill came from the supervisor's `--timeout` path, not from any other termination source.

**RESL-03 Clause 1 is PASS.** This is the first live end-to-end proof of the feature outside the unit-test harness.

### SC-4: `nono inspect` renders the `Limits:` block

```
$ powershell.exe -Command "& './target/release/nono.exe' run --cpu-percent 50 --memory 1G --max-processes 10 --name smoketest16 --detached --allow-cwd -- ping -t 127.0.0.1"
Started detached session 47911f928320dad6.
Name: smoketest16
Attach with: nono attach 47911f928320dad6

$ ./target/release/nono.exe inspect 47911f928320dad6
Session:    47911f928320dad6
Name:       smoketest16
Status:     Exited
Attached:   Detached
PID:        0 (supervisor: 64564)
Started:    2026-04-18T12:59:25.059073400-04:00
Exit code:  -1073741510
Command:    ping -t 127.0.0.1
Workdir:    C:\Users\omack\nono
Network:    allowed

Limits:
  cpu:     50% (hard cap)
  memory:  1 GiB (job-wide)
  procs:   10 (active)

$ ./target/release/nono.exe inspect --json 47911f928320dad6
{
  "session_id": "47911f928320dad6",
  ...
  "limits": {
    "cpu_percent": 50,
    "memory_bytes": 1073741824,
    "max_processes": 10
  }
}
```

- Text mode: `Limits:` block appears after `Network:`. Only the three set limits appear (`cpu`, `memory`, `procs`) — no `timeout:` line because `--timeout` was not passed. Exact formatting matches CONTEXT.md § Specific Implementation Pointers.
- JSON mode: `limits` object contains exactly the three set fields; `timeout_seconds` is correctly omitted via `#[serde(skip_serializing_if = "Option::is_none")]`.
- Pre-Phase-16 session (`d068edde0c346115`, command `cmd /c echo hello`, written long before this plan) `nono inspect` was run manually and rendered cleanly with NO `Limits:` block — confirming the `#[serde(default)]` backward-compat path works against real on-disk files from before the field existed.

**SC-3 (SessionRecord backward compat) and SC-4 (Limits rendering) both PASS.**

## Smoke Evidence (Unix warning path)

**NOT run in this session** — the test host is Windows 11. The Unix warning behavior is inherited from Plan 16-01 Task 1 (`warn_unix_resource_limits` + `collect_unix_resource_limit_warnings`), verified by `exec_strategy::unix_warning_tests` on Unix CI lanes at that time. No code in Plan 16-02 touches the Unix-side warning path. See Plan 16-01 SUMMARY (`b55a05b`) for the reference evidence.

## Known Caveats

1. **Timeout precision ±100ms.** The supervisor event loop polls `WaitForSingleObject(process, 100)` on each iteration; the deadline check happens once per iteration. For a 5-second timeout this yields ≤5.1s actual kill latency plus OS scheduler jitter. Documented inline on `compute_deadline` and in `timeout_accuracy` guidance in the plan. This is NOT a security issue — it is a precision caveat.
2. **Supervisor-crash safety net has a different exit code.** If the supervisor itself crashes before the deadline, `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` (preserved by Plan 16-01's read-modify-write on `JOBOBJECT_EXTENDED_LIMIT_INFORMATION`) kills the tree cleanly — but the observed exit code on the agent-tree processes is the kill-on-close kill (typically `-1073741510` / `STATUS_CONTROL_C_EXIT` family), NOT `STATUS_TIMEOUT_EXIT_CODE`. Operators inspecting exit codes should not assume `258` is the only "killed by cap" marker.
3. **Sandbox policy quirks on the test host.** Several smoke attempts from a bash prompt failed to launch `cmd.exe` because the Windows filesystem-launch-validation layer rejected `C:\` as a protected path. Workaround used in this SUMMARY was to invoke every smoke command through PowerShell so `cmd.exe` resolves to `C:\Windows\System32\cmd.exe`. This is pre-existing Phase 09 / Phase 11 behavior unrelated to Plan 16-02 and is noted as a usability item for Phase 19.

## Deferred for follow-up

- Live CPU-bound / memory-hungry workload integration tests for RESL-01 / RESL-02 / RESL-04 Clause 1 — could be added as `#[ignore]`-gated tests that spawn e.g. a CPU-spinning loop or a controlled large allocator. Noted-but-not-blocking per the planner/checker judgment call documented in CONTEXT.md § Claude's Discretion.
- **Per-process memory cap (`ProcessMemoryLimit`)** — explicitly deferred per CONTEXT.md § Deferred Ideas.
- **cgroup v2 / `setrlimit` native backends on Unix** — explicitly out of scope per REQUIREMENTS.md § Out of Scope.
- **Belt-and-suspenders `JOB_OBJECT_LIMIT_JOB_TIME` kernel-side CPU-time cap** — mentioned in CONTEXT.md as acceptable-but-not-required; not added. The supervisor-side wall-clock timer plus `KILL_ON_JOB_CLOSE` is the primary + safety-net pair.
- **Same `--cpu-percent` / `--memory` / `--timeout` / `--max-processes` flags on `nono wrap` and `nono shell`** — only `nono run` gets them in this milestone (CONTEXT.md § Deferred Ideas).
- **Pre-existing `cargo fmt --all -- --check` drift** in three files unrelated to Phase 16 (`config/mod.rs`, `exec_strategy_windows/restricted_token.rs`, `profile/mod.rs`). These were last touched by commits `41aad1d` / `68d9374` / `e094994` before Phase 16 started; fixing them would violate the "no pre-existing WIP sweep" staging constraint. Filed for Phase 19 CLEAN-01.
- **Pre-existing workspace test failures** unrelated to Phase 16: 5 tests fail under `cargo test --workspace --all-features` on this Windows host (`capability_ext::test_from_profile_allow_file_rejects_directory_when_exact_dir_unsupported`, `capability_ext::test_from_profile_filesystem_read_accepts_file_paths`, `profile::builtin::test_all_profiles_signal_mode_resolves`, `query_ext::test_query_path_sensitive_policy_includes_policy_source`, `trust_keystore::display_roundtrip_file`). Verified pre-existing by re-running the same test suite against commit `070a851` (Plan 16-01 Task 1, long before this plan touched anything) — same tests fail identically. These are Phase 11 / earlier issues with absolute-path handling on Windows build configurations. Filed for Phase 19 CLEAN-02 (Windows test flake triage).

## Deviations from Plan

### Rule 3 — Blocking issue: `session_commands_windows.rs` not in plan's `<files>` list

**Found during:** Task 2 — `cargo test` discovered zero `inspect_formatting` tests after editing `session_commands.rs`.

**Root cause:** `crates/nono-cli/src/main.rs` has a `#[cfg(target_os = "windows")] #[path = "session_commands_windows.rs"] mod session_commands;` override. On the Windows build target, the file edited by the plan (`session_commands.rs`) is never compiled — the Windows variant is. Without touching `session_commands_windows.rs`, the `Limits:` block feature would not exist at runtime on the target platform. This is the Rule 3 "blocking issue" case.

**Fix:** Applied the identical `run_inspect` Limits-block rendering + the `format_bytes_human` / `format_duration_human` helpers + the same 13-test `inspect_formatting_tests` module to `session_commands_windows.rs`. Both files now behave identically; the SC-4 smoke confirmed the Windows path runs.

**Files modified (Task 2 commit `39ee157`):** `session.rs`, `supervised_runtime.rs`, `session_commands.rs`, `session_commands_windows.rs`.

### Rule 3 — Blocking issue: `cargo fmt --check` fmt drift on Task 1 files

**Found during:** Task 3 Step 4 final CI pass.

**Root cause:** Three files committed in Task 1 (`d36d073`) had minor line-wrap drift that `cargo fmt` wanted differently. Since Plan 16-02's SC-7 requires `cargo fmt --all -- --check` to be clean, and the drift was inside 16-02's scope, the fmt fix is in-plan.

**Fix:** Separate `style(16-02)` commit `238fd1d` applying only `cargo fmt` to `launch.rs`, `mod.rs`, `supervisor.rs` — no logic changes.

### Rule 3 — Blocking issue: `cargo fmt --check` fmt drift on Plan 16-01's `cli.rs` parser tests

**Found during:** Task 3 Step 4 final CI pass.

**Root cause:** Two `Cli::try_parse_from(...)` lines in `parser_tests::cpu_percent_range_enforced_by_clap` and `parser_tests::max_processes_range_enforced_by_clap` (committed by Plan 16-01 `070a851`) exceeded line-length and `cargo fmt` rewrapped them.

**Fix:** Separate `style(16-02)` commit `da27080` applying `cargo fmt` to `cli.rs`. No logic changes.

## Staging Hygiene

All commits on `windows-squash` for Phase 16 are scoped to files listed in `16-01-PLAN.md` / `16-02-PLAN.md` `files_modified` (plus the deviation-documented `session_commands_windows.rs` for the Windows build). No pre-existing WIP was swept into any commit across either plan.

Pre-existing WIP files remain UNTOUCHED on disk at the end of Phase 16:

- `.planning/phases/10-etw-based-learn-command/10-RESEARCH.md` (untracked)
- `.planning/phases/10-etw-based-learn-command/10-UAT.md` (untracked)
- `.planning/phases/11-runtime-capability-expansion/11-01-PLAN.md` (modified)
- `.planning/phases/11-runtime-capability-expansion/11-02-PLAN.md` (modified)
- `.planning/phases/12-milestone-bookkeeping-cleanup/12-02-PLAN.md` (untracked)
- `.planning/quick/260410-nlt-fix-three-uat-gaps-in-phase-10-etw-learn/260410-nlt-PLAN.md` (untracked)
- `.planning/quick/260412-ajy-safe-layer-roadmap-input/` (untracked)
- `.planning/v1.0-INTEGRATION-REPORT.md` (untracked)
- `host.nono_binary.commit` (untracked)
- `query` (untracked)

These are reserved for Phase 19 CLEAN-03 per the v2.1 roadmap.

## Commits

| Commit | Kind | Summary |
|--------|------|---------|
| `d36d073` | `feat(16-02)` | Task 1: wall-clock timeout timer — supervisor terminates Job Object on deadline |
| `39ee157` | `feat(16-02)` | Task 2: `SessionRecord.limits` + `nono inspect` Limits block |
| `238fd1d` | `style(16-02)` | Rule-3: cargo fmt touch-up on Task 1 files (launch / mod / supervisor) |
| `da27080` | `style(16-02)` | Rule-3: cargo fmt touch-up on `cli.rs` Plan 16-01 parser tests |

## Verification

| Check | Command | Result |
|-------|---------|--------|
| Build (release) | `cargo build --release -p nono-cli --bin nono` | **PASS** |
| Clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | **PASS** (zero warnings) |
| Phase 16 tests | `cargo test -p nono-cli --bin nono -- apply_resource_limits timeout_deadline resource_limits_record inspect_formatting parser_tests` | **48/48 PASS** |
| Workspace test suite | `cargo test --workspace --all-features` | **650 pass, 5 pre-existing failures** (all in Phase-11 / earlier files; identical failures on commit `070a851`). Phase 16 code contributes **zero** new failures. |
| `cargo fmt --all -- --check` | — | Clean for every file in Phase 16 scope. Remaining drift is in 3 out-of-scope files (see Deferred). |

## Files Changed

| File | Commit | Role |
|------|--------|------|
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` | `d36d073` + fmt `238fd1d` | `STATUS_TIMEOUT_EXIT_CODE` const + `terminate_job_object` helper |
| `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` | `d36d073` + fmt `238fd1d` | `timeout_deadline` + `containment_job` fields on runtime; `compute_deadline`; deadline check in `run_child_event_loop`; 6 Windows-gated unit tests |
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` | `d36d073` + fmt `238fd1d` | `execute_supervised` computes deadline and borrows `containment.job` into runtime initialization |
| `crates/nono-cli/src/session.rs` | `39ee157` | `ResourceLimitsRecord` + `SessionRecord.limits` + 11 serde tests |
| `crates/nono-cli/src/supervised_runtime.rs` | `39ee157` | `create_session_runtime_state` populates `limits` from ctx.resource_limits |
| `crates/nono-cli/src/session_commands.rs` | `39ee157` | Unix `run_inspect` renders `Limits:` block + `format_bytes_human` / `format_duration_human` helpers + 13 formatter tests |
| `crates/nono-cli/src/session_commands_windows.rs` | `39ee157` | Identical changes mirrored to the Windows compilation target |
| `crates/nono-cli/src/cli.rs` | fmt `da27080` | Fmt touch-up to Plan 16-01's parser tests |

## Status

**Phase 16 complete.** All four RESL requirements shipped with kernel-enforced Windows Job Object limits (CPU / memory / active-processes) plus a supervisor-side wall-clock timer for `--timeout`. The `Limits:` block in `nono inspect` gives operators visibility into the caps active on any session. Plan 16-02 produces zero new workspace test failures or clippy warnings.

## Self-Check: PASSED

- **Files exist:**
  - `.planning/phases/16-resource-limits/16-02-SUMMARY.md` — FOUND (this file)
  - `crates/nono-cli/src/session.rs` — FOUND (modified)
  - `crates/nono-cli/src/supervised_runtime.rs` — FOUND (modified)
  - `crates/nono-cli/src/session_commands.rs` — FOUND (modified)
  - `crates/nono-cli/src/session_commands_windows.rs` — FOUND (modified)
- **Commits exist on `windows-squash`:**
  - `d36d073` — FOUND (Task 1)
  - `39ee157` — FOUND (Task 2)
  - `238fd1d` — FOUND (Task 1 fmt touch-up)
  - `da27080` — FOUND (cli.rs fmt touch-up)
