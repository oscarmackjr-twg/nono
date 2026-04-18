---
slug: windows-supervised-exec-cascade
status: partially-resolved
trigger: Windows supervised execution cascade — after fixing token UAF in spawn_windows_child (eb4730c / quick 260417-wla), two more blockers block every `nono run` on Windows
created: 2026-04-17
updated: 2026-04-17
branch: windows-squash
head: f46e2c9
milestone: v1.0
milestone_blocker: true
related_phase: 13-v1-human-verification-uat
related_quick: 260417-wla
---

# Debug Session: windows-supervised-exec-cascade

## Current Focus

hypothesis (confirmed for Bug #2): `create_restricted_token_with_sid` in `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs:71-84` calls `CreateRestrictedToken` with `Flags=0` and one restricting SID (the fresh session SID `S-1-5-117-*`). That makes every access check double-gated against the restricting SID. The session SID is absent from every object ACL on the system, so any sandboxed child dies with STATUS_ACCESS_DENIED (0xC0000022) during image load. Fix: pass `WRITE_RESTRICTED` as the flag, which narrows the restricting-SID access check to write-type operations only — reads (DLL loads, section maps, registry traversal) pass through with only the user SIDs checked.

hypothesis (open for Bug #3): after the WRITE_RESTRICTED fix, the detached path still fails for console applications with STATUS_DLL_INIT_FAILED (0xC0000142). This is a **separate** failure from the original 0xC0000022 Bug #3 and emerges only once the primary restricted-token gate is unlocked. Root cause is not yet pinned; strongest suspect is that the WRITE_RESTRICTED token, combined with the detached supervisor (DETACHED_PROCESS, no inherited console, null stdio), prevents the console grandchild from connecting to the auto-allocated console or the ConPTY host during DLL initialization. GUI apps (notepad.exe) work in detached mode under the same conditions; only console apps fail.

test: run `./target/release/nono.exe run --allow-cwd -- cmd /c "echo hello"` from PowerShell — expect `hello\nEXIT=0`. Run the cargo test `exec_strategy::restricted_token::tests::create_restricted_token_with_sid_applies_write_restricted_flag` — expect pass.
expecting: Bug #2 completely fixed, Bug #3 reduced from STATUS_ACCESS_DENIED to STATUS_DLL_INIT_FAILED and documented as follow-up.
next_action: Bug #2 resolved. Bug #3 requires a separate quick-task investigation into restricted-token + detached-supervisor + console-child initialization semantics. Candidate directions: (1) add the session SID as a token *group* (not a restricting SID) so WFP still matches but no restriction applies; (2) use `CreateProcessAsUser` with the unrestricted supervisor token and rely on Job Object containment + WFP AppID-based filtering instead of SID-based filtering for the detached code path; (3) investigate whether the detached supervisor needs to attach itself to a console before spawning a console grandchild.
reasoning_checkpoint: (empty)
tdd_checkpoint: (empty)

## Symptoms

### Expected behavior
`nono run --allow-cwd -- <any-cmd>` starts a sandboxed child on Windows, executes the command, exits with the child's exit code. `nono run --detached --allow-cwd -- <long-running-cmd>` returns a session ID immediately and runs the command in the background under a supervisor.

### Actual behavior
- Non-detached path: fails with `Windows filesystem policy does not cover the absolute path argument required for launch: C:\` before the child is launched (pre-flight validation error).
- Detached path: the detached supervisor subprocess exits with `0xc0000022` (STATUS_ACCESS_DENIED), returning "Detached session failed to start (exit status: exit code: 0xc0000022)" with the capability set echoed from the startup-log summary.

Both happen 100% of the time on the user's Windows 11 Enterprise 10.0.26200 host.

### Error messages
Bug #2:
```
Sandbox initialization failed: Windows supervised execution failed during shutting-down ...:
Sandbox initialization failed: Windows filesystem policy does not cover the absolute path argument required for launch: C:\
```
Emitted from `crates/nono/src/sandbox/windows.rs:680-684` via `validate_absolute_path_args` → `validate_candidate_path` chain.

Bug #3:
```
Sandbox initialization failed: Detached session failed to start (exit status: exit code: 0xc0000022): r   \\?\C:\Users\omack\Nono (dir)
```
The `r \\?\C:\Users\omack\Nono (dir)` fragment is the capability-set line echoed by `read_startup_log_summary` (crates/nono-cli/src/startup_runtime.rs:107-120), not a separate error.

### Timeline
- Never worked end-to-end on Windows with a filesystem capability set. Previously masked by the handle UAF in `spawn_windows_child` (fixed as quick task 260417-wla / commit `eb4730c` today).
- Discovered 2026-04-17 during Phase 13 v1.0 human-verification UAT.

### Reproduction
Fresh PowerShell window (not Claude Code bash), from project root on branch `windows-squash` at HEAD `f46e2c9`, with `target/release/nono.exe` rebuilt post-fix (mtime 17:10):

```
# Bug #2
./target/release/nono.exe run --allow-cwd -- cmd /c "echo hello"

# Bug #3
./target/release/nono.exe run --detached --allow-cwd -- ping -t 127.0.0.1
```

## Prior-art notes

### Fixed in the same investigation
- Bug #1: Token handle use-after-close in `spawn_windows_child` — resolved in commit `eb4730c`. Before the fix, neither Bug #2 nor Bug #3 was reachable because every supervised launch died earlier with `ERROR_INVALID_HANDLE (6)` from a closed token handle.

### Open questions the debugger should answer
1. Bug #2: which arg from `cmd /c "echo hello"` reaches `validate_candidate_path` as an absolute-path candidate, and what does it canonicalize to? `/c` is the likely culprit — `Path::new("/c").is_absolute()` is false on Windows per Rust docs, but `extract_arg_path_candidate` trims quotes and applies its own check; confirm.
2. Bug #3: is the Low-Integrity token being applied to the detached supervisor itself, or only to the sandboxed grandchild? Which PID exits with 0xc0000022 — the immediate detached process, or a later stage?
3. Is `should_use_low_integrity_windows_launch` semantically correct? Should any filesystem-grant capability really force Low IL, or is it over-reaching?
4. Are there unit tests that would have caught either bug? If not, flag as a test-coverage gap.
5. Single root-cause fix or two separate fixes?

## Constraints (non-negotiable)
- Security-critical codebase. Any fix must preserve Low-IL isolation for the sandboxed child — removing Low IL to make the tests pass is NOT acceptable.
- `-D warnings -D clippy::unwrap_used` stays green.
- DCO sign-off: `Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>` on every commit.
- Do NOT stage these pre-existing uncommitted files (they belong to other work):
  - `.planning/phases/11-runtime-capability-expansion/11-01-PLAN.md`
  - `.planning/phases/11-runtime-capability-expansion/11-02-PLAN.md`
  - `.planning/v1.0-MILESTONE-AUDIT.md`
  - `.planning/phases/10-etw-based-learn-command/10-RESEARCH.md`
  - `.planning/phases/10-etw-based-learn-command/10-UAT.md`
  - `.planning/phases/12-milestone-bookkeeping-cleanup/12-02-PLAN.md`
  - `.planning/quick/260410-nlt-fix-three-uat-gaps-in-phase-10-etw-learn/`
  - `.planning/quick/260412-ajy-safe-layer-roadmap-input/`
  - `.planning/v1.0-INTEGRATION-REPORT.md`

## Files to inspect
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` (post-fix `spawn_windows_child`, `create_low_integrity_primary_token`, `should_use_low_integrity_windows_launch`, token-integrity adjustment at ~760-793)
- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs`
- `crates/nono-cli/src/startup_runtime.rs` (detached-launch orchestration)
- `crates/nono/src/sandbox/windows.rs` (`validate_absolute_path_args`, `validate_candidate_path`, `extract_arg_path_candidate`, `validate_launch_paths`)
- `crates/nono-cli/src/execution_runtime.rs` (detached-supervisor spawn, env plumbing)

## User availability
oscarmackjr-twg is active, on Windows 11 Enterprise 10.0.26200, can rebuild, run commands in a fresh PowerShell, and paste output back. Phase 13 UAT (`.planning/phases/13-v1-human-verification-uat/13-UAT.md`) is paused awaiting resolution.

## Evidence

- 2026-04-17T21:31 — Instrumented `validate_absolute_path_args` with eprintln! traces. Re-ran from PowerShell: args arrived as `["/c", "echo hello"]`, both returned `None` from `extract_arg_path_candidate` (neither is absolute on Windows — empirically confirmed `Path::new("/c").is_absolute()` == false via standalone test). Validation passed. Result: Bug #2's `C:\` rejection as described in the session header is a bash/MSYS artifact where the shell rewrites `/c` → `C:\` before nono sees it. The real failure under PowerShell is that the restricted token causes the child to exit with `-1073741790` == `0xC0000022` (STATUS_ACCESS_DENIED).
- 2026-04-17T21:32 — Reproduced `0xC0000022` exit in PowerShell and bash (with `MSYS_NO_PATHCONV=1`) on `nono run --allow-cwd -- cmd /c "echo hello"`. The supervisor's own `execute_supervised_runtime` returns `exit_code: -1073741790` and then `std::process::exit(exit_code)` propagates it. That exit code is the **sandboxed child's** exit code, not the supervisor's.
- 2026-04-17T21:35 — Patched `spawn_windows_child` to force `h_token = null` (i.e., use the caller's token, not the restricted token). Re-ran `cmd /c "echo hello"` → printed `hello`, exit 0. Confirmed restricted token is the root cause of Bug #2 (and of the original 0xC0000022 Bug #3).
- 2026-04-17T21:45 — Patched `create_restricted_token_with_sid` to pass `WRITE_RESTRICTED` as the flag argument. `cmd /c "echo hello"` → `hello`, exit 0. Bug #2 fully resolved. Token still carries the session SID as a restricting SID (verified via `GetTokenInformation(TokenRestrictedSids)` in the new regression tests), so WFP network filtering via `FWPM_CONDITION_ALE_USER_ID` remains functional.
- 2026-04-17T22:30 — Detached path with WRITE_RESTRICTED fix applied: exit code changed from `0xC0000022` (STATUS_ACCESS_DENIED) to `0xC0000142` (STATUS_DLL_INIT_FAILED). Confirmed for `ping.exe`, `cmd /c "ver"`, `cmd /c "timeout /t 15"`.
- 2026-04-17T22:15 — Detached path with `C:/Windows/System32/notepad.exe` (GUI app) under the WRITE_RESTRICTED token: succeeds, exit 0 (notepad spawns and is visible via tasklist).
- 2026-04-17T22:22 — Detached path with `h_token = null` AND pty disabled: ping runs to completion; failure becomes a different (separate) "Detached session failed to become attachable within startup timeout" — the named pipe readiness probe times out, but the grandchild itself executes correctly. Confirms that `0xC0000142` in the WRITE_RESTRICTED detached path is bound to the restricted-token + console-child combination (GUI apps and unrestricted-token console apps both work).
- Test matrix (detached mode, `ping.exe`):

  | Token | PTY | Exit |
  |---|---|---|
  | Flags=0 restricting SID | Some | 0xC0000022 (STATUS_ACCESS_DENIED) |
  | WRITE_RESTRICTED | Some | 0xC0000142 (STATUS_DLL_INIT_FAILED) |
  | WRITE_RESTRICTED | None | 0xC0000142 (STATUS_DLL_INIT_FAILED) |
  | null | Some | 0xC0000142 (STATUS_DLL_INIT_FAILED) |
  | null | None | ping runs, attach-pipe timeout (separate issue) |

  Control (non-detached mode, `cmd /c "echo hello"`):

  | Token | Exit |
  |---|---|
  | Flags=0 restricting SID | 0xC0000022 |
  | WRITE_RESTRICTED | hello → exit 0 |

## Eliminated hypotheses

- "Bug #2's `C:\` rejection is caused by `/c` canonicalizing to `C:\` inside `extract_arg_path_candidate`." — **Wrong.** `Path::new("/c").is_absolute()` is `false` on Windows; the validation function correctly returns `None` for `/c`. The `C:\` string only appears when bash/MSYS pre-translates `/c` → `C:\` in argv before argument parsing. From PowerShell, validation passes, and the failure is instead a STATUS_ACCESS_DENIED exit from the restricted token.
- "`should_use_low_integrity_windows_launch` over-applies Low IL to the supervisor." — **Wrong for these bugs.** In the supervised code path, `config.session_sid.is_some()` is always true, so the `else if should_use_low_integrity_windows_launch(config.caps)` branch is never taken. The Low IL token code is unused in Bug #2 / Bug #3 reproductions; the `spawn_windows_child` chooses the `create_restricted_token_with_sid` branch unconditionally.
- "Bug #3 is caused by ConPTY failing in a DETACHED_PROCESS supervisor." — **Partially right, partially wrong.** Confirmed `CreatePseudoConsole` returns success (hr=0x0, non-null hpcon) even from a DETACHED_PROCESS supervisor. However, ConPTY is **not** the sole trigger of Bug #3's 0xC0000142: dropping the PTY (`runtime.pty() → None`) leaves WRITE_RESTRICTED + detached still failing identically. ConPTY is a contributing factor only.
- "Single root-cause fix." — **Wrong.** Bug #2 and the original 0xC0000022 Bug #3 share a root cause (restricted-token over-restriction), but the 0xC0000142 post-fix Bug #3 is a **separate** failure that surfaces only after Bug #2 is addressed.

## Resolution
status: partially-resolved
root_cause:
  - Bug #2 (non-detached) and original Bug #3 (detached, 0xC0000022): `CreateRestrictedToken` invoked with `Flags=0` combined with a restricting SID absent from every object ACL. Every access check by the sandboxed child hits the restricting-SID gate and fails with STATUS_ACCESS_DENIED before user code runs.
  - Bug #3 residual (detached, 0xC0000142, post-fix): separate issue, not pinpointed. Occurs only for console applications under WRITE_RESTRICTED token + detached (no-console) supervisor. GUI apps (notepad.exe) work. Null-token + no-PTY also works. The failure mode is deterministic but narrower than originally reported.
fix:
  - `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs`: pass `WRITE_RESTRICTED` (8u32) as the `Flags` argument to `CreateRestrictedToken`. This confines the second (restricting-SID) access check to WRITE-type operations only. Reads pass through with just the user SIDs checked, so the child initializes correctly. The session SID remains present on the token as a restricting SID so Windows Filtering Platform's `FWPM_CONDITION_ALE_USER_ID` conditions continue to match child traffic.
verification:
  - `cargo build -p nono-cli --release` — PASS.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` — PASS, zero warnings.
  - `cargo test -p nono-cli --release --bin nono -- restricted_token` — PASS, 4/4 tests green (`create_restricted_token_with_sid_applies_write_restricted_flag`, `create_restricted_token_with_sid_returns_usable_handle_for_child_spawn`, `restricted_token_drop_is_null_safe`, `generate_session_sid_produces_parsable_sddl_string`).
  - Smoke test `./target/release/nono.exe run --allow-cwd -- cmd /c "echo hello"` — PASS (prints `hello`, exits 0).
  - Smoke test `./target/release/nono.exe run --detached --allow-cwd -- ping -t 127.0.0.1` — STILL FAILS with 0xC0000142; filed as residual Bug #3 for a follow-up quick task.
files_changed:
  - `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs` — added `WRITE_RESTRICTED` to the `CreateRestrictedToken` flags argument; added 4 new regression tests.
follow_up:
  - **Bug #3 residual (0xC0000142 STATUS_DLL_INIT_FAILED in detached path):** needs a separate debug pass. Candidate investigations:
    1. Does adding the session SID to the token as a *group SID* (via `AddGroupSids` / a rebuilt token) instead of as a restricting SID let WFP still match while removing all access-check restriction? That would eliminate the need for `CreateRestrictedToken` entirely on this path.
    2. Does switching the detached code path to AppID-based WFP filtering (`get_app_id_blob(target_program)`, the existing fallback in `install_wfp_policy_filters` at `nono-wfp-service.rs:1364-1367`) work? That would let the detached child run with the unrestricted supervisor token.
    3. Is the issue specific to how a console child auto-allocates its console when the parent has DETACHED_PROCESS + restricted token? Try pre-attaching the supervisor to a `conhost.exe` via `AllocConsole` before spawning the grandchild, or pre-allocating the grandchild's console.
  - Phase 13 UAT item **P05-HV-1** (`nono run --detach -- ping -t 127.0.0.1`) remains blocked by the residual Bug #3. Other Phase 13 items that rely on detached mode (**P07-HV-3** session commands round-trip, **P11-HV-1**/**P11-HV-3** supervised capability flows) are likely affected.
  - Phase 13 UAT items that exercise the **non-detached** supervised path (P07-HV-1 wrap exit code, P09-HV-1 proxy env injection, P09-HV-2 WFP port integration) should now be unblocked by the Bug #2 fix — they did not require detached mode.

## Phase 15-01 Investigation

Date: 2026-04-18
HEAD: 6f4de70 (branch `windows-squash`)
Binary: `target/debug/nono.exe` (debug build with gates described below)

### Method

Two temporary `#[cfg(all(target_os = "windows", debug_assertions))]`-gated patches were applied to the current codebase to isolate which component of the failing configuration drives the 0xC0000142:

1. `crates/nono-cli/src/supervised_runtime.rs` line 117 — force `pty_pair = None` unconditionally on Windows debug builds (overrides the `session.detached_start || session.interactive_pty` allocation path).
2. `crates/nono-cli/src/exec_strategy_windows/launch.rs` line 840 — force `h_token = null` (caller's token) when `NONO_DETACHED_LAUNCH=1` is set in the outer shell.

Both patches were reverted after the matrix was captured. No production code was shipped from this plan.

### Refined matrix

Ran from PowerShell on Windows 11 Enterprise 10.0.26200 with fresh `target/debug/nono.exe`:

| Row | Token shape | PTY | Outer detached | Command | Result | Notes |
|-----|-------------|-----|----------------|---------|--------|-------|
| B | WRITE_RESTRICTED + session SID | None (gate 1) | Yes (`--detached`) | `nono run --detached --allow-cwd -- ping -t 127.0.0.1` | **FAIL** `0xC0000142` (STATUS_DLL_INIT_FAILED) | Disabling PTY alone does NOT unblock detached console grandchild. |
| C | WRITE_RESTRICTED + session SID | None (gate 1, not exercised — non-detached) | No | `nono run --allow-cwd -- cmd /c "echo hello"` | **PASS** — `hello`, exit 0 | Non-detached regression clean. The no-PTY gate does not affect non-interactive path. |
| D | null (gate 2) | None (gate 1) | `NONO_DETACHED_LAUNCH=1` set manually | `NONO_DETACHED_LAUNCH=1 nono run --detached --allow-cwd -- ping -t 127.0.0.1` | **PASS** — ping replies streamed live; grandchild initialized; no DLL-init failure | Matches the existing matrix's sole working row (null + None). |

Note on Row D: setting `NONO_DETACHED_LAUNCH=1` in the outer shell does NOT detach the supervisor — it only triggers the investigation-gate null-token path inside `spawn_windows_child`. The grandchild still ran with null token + no PTY, which is the configuration under test.

### Conclusion

- **Row B's failure** proves direction-a (WRITE_RESTRICTED + no PTY) is NOT viable. PTY alone is not the sole blocker of 0xC0000142; the WRITE_RESTRICTED token also contributes.
- **Row D's success** proves the combined `null + no PTY` row of the existing matrix is reproducible on current HEAD (commit `6f4de70`).
- **Row C** is the required non-detached regression control — the no-PTY gate does not break `nono run` without `--detached`.

Direction-b (gated PTY-disable + null token on detached path, with AppID-based WFP filtering as the kernel boundary) is the only viable path for Plan 15-02.

### Follow-up checks (not required for decision, recorded for completeness)

- Row D does not prove that the supervisor's outer double-launch path with `DETACHED_PROCESS` also works with the new token/PTY shape — Plan 15-02's smoke-gate Row 1 (`nono run --detached -- ping`) validates that end-to-end.
- Row D attach-pipe behavior was not exercised (the supervisor was not detached). Plan 15-02 must add a pipe-based stdout capture path or document that `nono attach` on detached-windows is a stub in v2.1.

## Phase 15 Smoke Gate

Date: 2026-04-18
Binary: `target/release/nono.exe` built from commit `2c414d8`
Host: Windows 11 Enterprise 10.0.26200 (Admin PowerShell)
`nono-wfp-service`: STATE=RUNNING

| Row | Config | Expected | Result | Notes |
|-----|--------|----------|--------|-------|
| 1 | `nono run --detached --allow-cwd -- ping -t 127.0.0.1` | banner + grandchild live | **PASS** | Banner: `Started detached session 11fe3ab772880043`. `tasklist | findstr /I "ping"` showed `PING.EXE 52548`. |
| 2 | `nono run --detached --allow-cwd -- cmd /c "echo hello"` | banner, exit 0 | **PASS** | Banner: `Started detached session 1971e4eee9318230`. Fast-exit race handled by `startup_runtime` guard: inner exits success + session file exists → treat as launched. |
| 3 | `nono run --allow-cwd -- cmd /c "echo hello"` | `hello`, exit 0 | **PASS** | `hello` printed; supervisor exit 0. Non-detached path unchanged (full WRITE_RESTRICTED + ConPTY). |
| 4 | `nono run --detached --block-net --allow-cwd -- cmd /c "curl --max-time 5 http://example.com"` | banner + network blocked | **PASS** (partial verification) | Banner: `Started detached session 0b770bb299f1c2d9`. Detached supervisor launched cleanly under `--block-net`; no `0xC0000142`. Kernel-enforced curl blocking was not directly observed from the outer invocation (child output is not streamed through the supervisor on the detached Windows path — v2.1+ attach-streaming). |
| 5 | `nono logs / inspect / prune --dry-run <session-id>` | exit 0 with expected shapes | **PASS** | `logs 11fe3ab772880043` → exit 0, reports "No event log recorded" (correct for ping which has no audit entries). `inspect 11fe3ab772880043` → exit 0, full session record (session_id, name, status, attached, pids, started, exit_code, command, workdir, network). `prune --dry-run` → exit 0, lists 1172 stale sessions without deleting. |

All 5 rows PASS. Phase 15 acceptance gate is met.

## Resolution

status: resolved

### Final root cause

1. **Primary:** `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` combined with `DETACHED_PROCESS` causes `STATUS_DLL_INIT_FAILED (0xC0000142)` in console-application grandchildren. The WRITE_RESTRICTED + session-SID token amplifies the failure — both must be addressed for the detached path.
2. **Secondary (exposed by the primary fix):** `WindowsSupervisorRuntime` named its control pipe after the internal `supervisor_session_id` (correlation ID `supervised-<pid>-<nanos>`), but the outer `startup_runtime::run_detached_launch` readiness probe and `session_commands_windows` clients looked up pipes by the user-facing `short_session_id`. The mismatch produced "Detached session failed to become attachable within startup timeout" even when the grandchild ran correctly. Pre-existing; documented in the matrix as `attach-pipe timeout (separate issue)`. Masked by 0xC0000142 until the primary fix landed.

### Fix

Direction-b, applied on `windows-squash`:

1. **`crates/nono-cli/src/supervised_runtime.rs`** (commit `802c958`) — `should_allocate_pty()` gates PTY allocation. On Windows, allocate only when `session.interactive_pty` is true (`nono shell`). Non-Windows keeps `detached_start || interactive_pty`.
2. **`crates/nono-cli/src/exec_strategy_windows/launch.rs`** (commit `802c958`) — when `NONO_DETACHED_LAUNCH=1` is set (the inner detached supervisor's env var), `spawn_windows_child` skips `create_restricted_token_with_sid` / `create_low_integrity_primary_token` and uses `std::ptr::null_mut()` so `CreateProcessW` runs the grandchild with the caller's token. Kernel network identity falls back to AppID-based WFP filtering.
3. **`crates/nono-cli/src/exec_strategy_windows/supervisor.rs`** (commit `802c958` + `2c414d8`) — added a diagnostic log line when `start_logging` finds no PTY; added `user_session_id` field to `WindowsSupervisorRuntime` so pipes are named after the short session ID (matches what clients and the outer readiness probe look up).
4. **`crates/nono-cli/src/exec_strategy_windows/mod.rs`** (commit `2c414d8`) — passes the user-facing `session_id` into `WindowsSupervisorRuntime::initialize`.
5. **`crates/nono-cli/src/startup_runtime.rs`** (commit `2c414d8`) — when the inner supervisor exits with `status.success()` AND the session record exists, treat the launch as successful (fast-exit race resolution for short-lived detached commands).

### Security waivers (scoped to Windows detached path only)

- **Low-Integrity isolation**: waived. Null token inherits caller IL. Job Object + filesystem sandbox (CapabilitySet) remain primary isolation.
- **Per-session SID WFP**: waived. Detached children share one AppID WFP filter. Still kernel-enforced; requires `nono-wfp-service` running for network enforcement.
- Non-detached `nono run` and `nono shell` retain the full WRITE_RESTRICTED + session-SID + ConPTY configuration — unchanged.

### Verification

- CI gate: `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` — clean.
- Tests: 12/12 targeted tests (PTY gate, detached-token gate, existing restricted_token regression, ambient detached tests). Pre-existing Windows test drift in `capability_ext`, `profile/builtin`, `query_ext`, `trust_keystore` is NOT introduced by Phase 15 (verified by stash-revert comparison against HEAD).
- Smoke gate: all 5 rows PASS (see § Phase 15 Smoke Gate above).

### Deferred to v2.1+

- `nono attach` output streaming for detached sessions on Windows. `start_logging` returns `Ok(())` when PTY is None; a pipe-based stdout relay is a future enhancement. Operators get a clear log line explaining the gap.

## Direction Decision

Date: 2026-04-18
Chosen by: oscarmackjr-twg (confirmed at Plan 15-01 Task 1.5 checkpoint)

### Chosen direction: **b**

Gate PTY off on Windows when detached, null the token when `NONO_DETACHED_LAUNCH=1`, switch the detached path to AppID-based WFP filtering. The WRITE_RESTRICTED + session-SID + ConPTY shape that works in non-detached mode is abandoned on the detached path.

### Evidence summary

- Row B (WRITE_RESTRICTED + no PTY, detached) **failed** with `0xC0000142`. Disabling PTY alone does not unblock the console grandchild's DLL loader — the token also matters. Direction-a is eliminated.
- Row C (WRITE_RESTRICTED + no PTY gate, non-detached) **passed**. The PTY gate does not regress the non-detached supervised path.
- Row D (null token + no PTY, `NONO_DETACHED_LAUNCH=1`) **passed** — ping replies streamed live. The matrix's only working row is reproducible on current HEAD `6f4de70`.

### Security impact

Four non-negotiable properties from 15-02-PLAN.md `<security_acceptance_gate>` evaluated under direction-b:

| # | Property | Verdict |
|---|----------|---------|
| 1 | Sandbox filesystem boundary (CapabilitySet) | **Preserved.** Capability apply is independent of token/PTY. |
| 2 | Job Object containment | **Preserved.** Job Object is applied after `CreateProcess`, independent of the token. Grandchild still terminates when supervisor dies. |
| 3 | Low-Integrity isolation | **Waived on detached path only.** Null token inherits the supervisor's IL. Job Object + filesystem sandbox remain primary isolation; LI was additive, not the primary security boundary. A waiver clause must appear in the 15-02 commit body. |
| 4 | Kernel network identity | **Waived (per-session SID) / preserved (kernel identity).** Session-SID WFP (`FWPM_CONDITION_ALE_USER_ID`) is replaced by AppID-based WFP filtering (`get_app_id_blob`, existing fallback at `nono-wfp-service.rs:1364-1367`). Still kernel-enforced; still requires `nono-wfp-service` to be running. Trade-off: two detached sessions of the same binary share one AppID filter — per-session SID differentiation is lost on this path. |

Non-detached (`nono run --allow-cwd`, `nono shell`) retains the full WRITE_RESTRICTED + session-SID + ConPTY configuration. The waivers are strictly scoped to the Windows-detached code path (gated by `cfg(target_os = "windows")` and either `session.detached_start` or `NONO_DETACHED_LAUNCH=1`).

### Plan 15-02 action list

Plan 15-02 must make exactly these changes (and no more):

1. **`crates/nono-cli/src/supervised_runtime.rs` (line ~117):** Replace the unconditional PTY allocation with a platform-conditional block. On Windows, allocate a PTY only when `session.interactive_pty` is true (i.e., for `nono shell`). `session.detached_start` alone must NOT trigger PTY allocation on Windows. Non-Windows platforms keep the existing `detached_start || interactive_pty` semantics.

2. **`crates/nono-cli/src/exec_strategy_windows/launch.rs` (line ~838 `spawn_windows_child`):** Before the existing `config.session_sid`-based token selection, add a check for `NONO_DETACHED_LAUNCH=1`. When present on Windows, skip `create_restricted_token_with_sid`, skip `create_low_integrity_primary_token`, and set `h_token = null_mut()` so `CreateProcessW` uses the caller's token. Detection must be `#[cfg(target_os = "windows")]`-guarded.

3. **`crates/nono-cli/src/exec_strategy_windows/supervisor.rs` (line ~405 `start_logging`):** Currently returns `Ok(())` when `pty_output_read == 0`. Keep this behavior — pipe-based stdout capture for `nono attach` against a detached session is a v2.1+ enhancement. Add a log line noting "detached session on Windows: PTY output relay skipped; nono attach will not stream child output" so operators understand the scope.

4. **`crates/nono-cli/src/pty_proxy_windows.rs`:** No changes in 15-02. The `open_detached_stdout_pipe` helper described in 15-02-PLAN's direction-b Action item 3 is deferred — `nono attach` streaming for detached sessions is a v2.1+ feature, not part of Phase 15's closure scope. If any `#[allow(dead_code)]` attributes in this file become reachable via the supervised_runtime.rs gate, they may be removed.

5. **`crates/nono-cli/src/exec_strategy_windows/restricted_token.rs`:** No changes. The WRITE_RESTRICTED + session-SID path remains correct for non-detached runs; direction-b only skips this path on the detached gate, it does not modify the path itself.

6. **Tests (both files changed):**
    - `supervised_runtime.rs`: unit test asserting on Windows, `pty_pair` is `None` when `detached_start=true, interactive_pty=false`, and `Some` when `interactive_pty=true`.
    - `launch.rs`: unit test (or integration test) asserting `h_token` is `null_mut` when `NONO_DETACHED_LAUNCH=1` is set and `session_sid` is `Some`. Use `EnvVarGuard` pattern to save/restore env.
    - Existing `restricted_token` regression tests (4 tests) must continue to pass.

7. **Commit body waiver clause (required for 15-02 Task 1 commit):** Include a `Security-Waiver:` trailer block documenting the two waived properties from the table above. This is the audit trail for the non-negotiable-with-waiver gate.

### What 15-02 must NOT do

- Do NOT modify `create_restricted_token_with_sid` or its tests — the WRITE_RESTRICTED fix from 2026-04-17 (commit `eb4730c` → follow-up) stays intact for the non-detached path.
- Do NOT add new `#[allow(dead_code)]` attributes. If the PTY gate leaves any `pty_proxy_windows` items unused on the detached path, they may be removed instead.
- Do NOT thread a new `is_detached` field through `ExecConfig` — use the existing `NONO_DETACHED_LAUNCH` env var (set by `startup_runtime.rs::run_detached_launch`). This avoids churn in all callers.
- Do NOT commit the investigation patches used for this plan. They have already been reverted at the end of Task 1.
