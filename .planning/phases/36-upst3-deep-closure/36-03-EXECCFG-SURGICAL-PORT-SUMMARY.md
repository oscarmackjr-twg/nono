---
phase: "36"
plan: "03"
subsystem: nono-cli/exec_strategy, nono/diagnostic
tags: [upstream-port, d20-manual-replay, d19-cherry-pick, exec-config, diagnostic-parser]
dependency_graph:
  requires: [36-01a, 36-02]
  provides: [D-36-D1-invariant-preserved, D-36-D2-smoke-passed, bbdf7b85-ported]
  affects: [exec_strategy.rs, execution_runtime.rs, cli.rs, pty_proxy.rs, sandbox_log.rs, startup_prompt.rs, profile_save_runtime.rs, diagnostic.rs]
tech_stack:
  added: []
  patterns: [D-20-manual-replay, D-19-cherry-pick, escape-aware-string-parsing]
key_files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/pty_proxy.rs
    - crates/nono-cli/src/sandbox_log.rs
    - crates/nono-cli/src/startup_prompt.rs
    - crates/nono-cli/src/profile_save_runtime.rs
    - crates/nono/src/diagnostic.rs
decisions:
  - "D-36-D1 invariant preserved: ExecConfig 17-field fork shape unchanged throughout all 3 commits"
  - "POST_EXIT_PTY_DRAIN_TIMEOUT declared with #[allow(dead_code)] since drain_master_output is not yet ported to the fork"
  - "compute_executable_identity added as thin delegate to crate::exec_identity::compute to match upstream test-accessible name"
  - "startup_prompt_termination changed to automatic (no interactive Y/N prompt) matching b5f0a3ab upstream intent"
  - "sandbox_log.rs split finish() into finish()/finish_realtime_only()/finish_inner() to match upstream refactor"
metrics:
  duration: "~90 minutes (across two sessions)"
  completed_date: "2026-05-13"
  tasks_completed: 4
  files_modified: 8
---

# Phase 36 Plan 03: EXECCFG Surgical Port Summary

Port upstream commits b5f0a3ab (exec strategy surgical refactor) and bbdf7b85 (escape-aware diagnostic parser) into the fork as exactly 3 sequenced git commits following D-20 manual-replay and D-19 cherry-pick shapes.

## Commits

| # | Hash | Shape | Files |
|---|------|-------|-------|
| 1 | `be0116d0` | D-20 manual-replay | crates/nono/src/diagnostic.rs |
| 2 | `2a720a06` | D-20 manual-replay | exec_strategy.rs, execution_runtime.rs, cli.rs, pty_proxy.rs, sandbox_log.rs, startup_prompt.rs, profile_save_runtime.rs |
| 3 | `98f8cff1` | D-19 cherry-pick | crates/nono/src/diagnostic.rs |

## Acceptance Criteria Results

| Criterion | Result |
|-----------|--------|
| ExecConfig 17-field shape unchanged (D-36-D1) | PASS |
| `should_offer_profile_save` function present | PASS |
| `POST_EXIT_PTY_DRAIN_TIMEOUT` = 100ms | PASS |
| `clear_signal_forwarding_target` has 3 callsites + 1 def | PASS |
| `LearnArgs.trace: bool` field added | PASS |
| `has_visible_output` replaces `has_observed_output` | PASS |
| `print_macos_run_guidance` ref in learn_runtime.rs | PASS |
| bbdf7b85 escape-aware parser body rewrite | PASS |
| 2 new diagnostic tests pass | PASS |
| D-36-D2 smoke: exactly 1 `Upstream-commit:` trailer | PASS |
| cargo clippy -p nono -p nono-cli clean | PASS |
| cargo fmt --all clean | PASS |
| cargo test -p nono (678 tests) | PASS |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `use std::time::Duration` unused on Windows target**
- **Found during:** Task 2 (execution_runtime.rs)
- **Issue:** Bare `use std::time::Duration` is unused on `target_os = "windows"` since all helper functions using it are cfg-gated.
- **Fix:** Added `#[cfg(not(target_os = "windows"))]` gate to the Duration import.
- **Files modified:** crates/nono-cli/src/execution_runtime.rs
- **Commit:** 2a720a06

**2. [Rule 2 - Missing functionality] `POST_EXIT_PTY_DRAIN_TIMEOUT` usage site absent**
- **Found during:** Task 2 (exec_strategy.rs)
- **Issue:** b5f0a3ab changes the constant from 250ms to 100ms, but `drain_master_output()` (the sole callsite) does not exist in the fork. The constant would be flagged as dead code.
- **Fix:** Added `#[allow(dead_code)]` attribute with comment noting the usage site is pending a future port.
- **Files modified:** crates/nono-cli/src/exec_strategy.rs
- **Commit:** 2a720a06

**3. [Rule 2 - Missing functionality] `compute_executable_identity` not in fork as named function**
- **Found during:** Task 2 (execution_runtime.rs)
- **Issue:** Upstream uses `compute_executable_identity(...)` as a named function to allow test-accessible delegation. The fork calls `crate::exec_identity::compute` inline.
- **Fix:** Added thin delegate function `fn compute_executable_identity(resolved_program: &Path) -> crate::Result<...>` wrapping the fork's existing call, matching upstream's test-accessible shape.
- **Files modified:** crates/nono-cli/src/execution_runtime.rs
- **Commit:** 2a720a06

## Known Stubs

None.

## Threat Flags

None. Changes are internal refactors and parser hardening; no new network endpoints, auth paths, or trust boundaries introduced.

## Self-Check: PASSED

- Commits `be0116d0`, `2a720a06`, `98f8cff1` verified in git log.
- `git log --format='%B' main~3..main | grep -c '^Upstream-commit: '` = 1 (D-36-D2 PASS).
- `cargo clippy -p nono -p nono-cli` clean.
- `cargo test -p nono --lib` = 678 passed, 0 failed.
