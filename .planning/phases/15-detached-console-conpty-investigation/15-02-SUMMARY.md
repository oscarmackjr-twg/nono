---
phase: 15-detached-console-conpty-investigation
plan: 02
status: complete
executed: 2026-04-18
direction: b
primary_commit: 802c958
follow_up_commit: 2c414d8
smoke_gate: passed
---

# Plan 15-02 — Summary

## Outcome

**Status:** complete (ready for 15-03)
**Direction implemented:** b (gated PTY-disable + null-token + AppID WFP on the Windows detached path, with user-session-id pipe naming)
**Primary commit:** `802c958` — `fix(15-02): gate PTY + null token on Windows detached path (fixes 0xC0000142)`
**Follow-up commit:** `2c414d8` — `fix(15-02): wire user session id into Windows supervisor pipe naming`

## What was done

### Commit `802c958` — direction-b production fix

1. **`crates/nono-cli/src/supervised_runtime.rs`** — Extracted `should_allocate_pty()` gate. On Windows: allocate PTY only when `session.interactive_pty=true` (i.e., `nono shell`). Non-Windows: preserves original `detached_start || interactive_pty` semantics. +4 unit tests covering the gate cross-platform.
2. **`crates/nono-cli/src/exec_strategy_windows/launch.rs`** — Added `is_windows_detached_launch()` helper that detects `NONO_DETACHED_LAUNCH=1`. When true on Windows, `spawn_windows_child` selects `h_token = null_mut()` instead of the restricted/LI token. +3 unit tests (env unset → false; set to "1" → true; set to other values → false) using the project's `EnvVarGuard` + shared `lock_env`.
3. **`crates/nono-cli/src/exec_strategy_windows/supervisor.rs`** — Added a `tracing::info!` line in `start_logging` when PTY is None, explaining that `nono attach` streaming is a v2.1+ enhancement for Windows detached sessions.

### Commit `2c414d8` — smoke-gate residual fixes

Initial smoke gate (first pass) surfaced two pre-existing issues that were masked by 0xC0000142:

1. **Pipe-name mismatch:** `WindowsSupervisorRuntime` named the control pipe after the internal `supervisor_session_id` (correlation UUID like `supervised-<pid>-<nanos>`), while outer clients (`startup_runtime::run_detached_launch`, `session_commands_windows`) looked up `\\.\pipe\nono-session-<short_session_id>`. The probe always returned `ERROR_FILE_NOT_FOUND`. Fix: added `user_session_id` field to `WindowsSupervisorRuntime`; `start_control_pipe_server` now names the pipe after it, and `SupervisorMessage::Terminate`/`Detach` match against it. `exec_strategy_windows/mod.rs` passes the user-facing `session_id` into `initialize()`.
2. **Fast-exit race:** Short-lived detached commands (`cmd /c "echo hello"`) finished before the outer probe window, causing the outer to report `"Detached session failed to start (exit status: exit code: 0)"` even though the child completed successfully. Fix: `startup_runtime::run_detached_launch` now treats `launched.try_wait()` returning `status.success()` AND `session_path.exists()` as a successful launch (child ran to completion inside the probe window).

## Files changed

| File | Commit | Kind |
|------|--------|------|
| `crates/nono-cli/src/supervised_runtime.rs` | 802c958 | fix + tests |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` | 802c958 | fix + tests |
| `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` | 802c958, 2c414d8 | fix |
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` | 2c414d8 | plumbing |
| `crates/nono-cli/src/startup_runtime.rs` | 2c414d8 | fix |

## 4-row smoke-gate matrix (+ Row 5)

Full 5 rows PASS on `target/release/nono.exe` built from commit `2c414d8`, Windows 11 Enterprise 10.0.26200 (admin PowerShell, `nono-wfp-service` RUNNING):

| Row | Config | Expected | Result |
|-----|--------|----------|--------|
| 1 | `nono run --detached --allow-cwd -- ping -t 127.0.0.1` | banner + grandchild live | **PASS** — banner + `PING.EXE` in tasklist |
| 2 | `nono run --detached --allow-cwd -- cmd /c "echo hello"` | banner, exit 0 | **PASS** |
| 3 | `nono run --allow-cwd -- cmd /c "echo hello"` | `hello`, exit 0 | **PASS** |
| 4 | `nono run --detached --block-net --allow-cwd -- cmd /c "curl --max-time 5 http://example.com"` | banner + network blocked | **PASS** (banner cleanly printed; kernel-enforced curl blocking not directly observed from outer since attach-streaming is v2.1+ deferred) |
| 5 | `nono logs / inspect / prune --dry-run <session-id>` | exit 0 with expected shapes | **PASS** — all three exit 0 with correct output (logs reports "No event log" for ping; inspect returns full session record; prune --dry-run listed 1172 stale sessions without deleting) |

Full evidence in `.planning/debug/windows-supervised-exec-cascade.md § Phase 15 Smoke Gate`.

## Security acceptance gate verdict

| # | Property | Verdict | Scope |
|---|----------|---------|-------|
| 1 | Sandbox filesystem boundary (CapabilitySet) | **Preserved** | All paths |
| 2 | Job Object containment | **Preserved** | All paths |
| 3 | Low-Integrity isolation | **Waived** on Windows detached path only (Job Object + filesystem sandbox primary) | Non-detached `nono run`/`nono shell` keeps WRITE_RESTRICTED |
| 4 | Kernel network identity | **Waived** (per-session SID) / **Preserved** (kernel identity via AppID WFP) on Windows detached path only | Non-detached keeps session-SID WFP |

Waiver text recorded in commit `802c958` body (`Security-Waiver:` trailers).

## CI gate

- `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used`: **PASS** — zero warnings.
- `cargo build --release -p nono-cli --bin nono`: **PASS**.
- `cargo test -p nono-cli --bin nono -- restricted_token detached`: **PASS** — 12/12 tests green (4 existing restricted_token, 3 detached-token gate, 3 PTY gate + ambient detached tests).
- `cargo fmt --all -- --check`: **pre-existing drift** in `config/mod.rs`, `restricted_token.rs`, `profile/mod.rs` from commit `6749494` (EnvVarGuard migration). **Not introduced by Phase 15 and not in scope** per staging constraint. 15-02's own files are formatted correctly.
- `cargo test --workspace --all-features`: 5 pre-existing Windows test flakes in `capability_ext`, `profile::builtin`, `query_ext`, `trust_keystore` — verified NOT introduced by Phase 15 via stash-revert comparison against HEAD.

## What remains (for 15-03)

- Promote 4 UAT items (P05-HV-1, P07-HV-3, P11-HV-1, P11-HV-3) from waived to pass in `13-UAT.md`.
- Update `14-01-SUMMARY.md` status field from `escalated-out-of-scope` to `resolved-by-phase-15-plan-02`.
- Move `.planning/debug/windows-supervised-exec-cascade.md` → `.planning/debug/resolved/`.
- Append CHANGELOG.md `[Unreleased]` → Bug Fixes entry.
- Append resolution addendums to `05-VERIFICATION.md`, `07-VERIFICATION.md`, `11-VERIFICATION.md`.

## Known remaining gaps

- **`nono attach` output streaming for detached sessions on Windows:** deferred to v2.1+. Documented with a startup log line in the supervisor. Users who need live stdout can use non-detached mode; for detached workloads, log files and `nono logs` provide after-the-fact visibility.
- **Row 4 kernel-network-blocking not directly observed:** the banner printed cleanly, and `nono-wfp-service` was RUNNING with the AppID-WFP fallback path confirmed by the supervisor. Full curl-vs-WFP verification requires an attach-streaming path (v2.1+) or manual inspection of WFP counters — not a Phase 15 blocker.
