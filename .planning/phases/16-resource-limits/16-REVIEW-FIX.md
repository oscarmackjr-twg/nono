---
phase: 16-resource-limits
fixed_at: 2026-04-18T00:00:00Z
review_path: .planning/phases/16-resource-limits/16-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 16: Code Review Fix Report

**Fixed at:** 2026-04-18
**Source review:** `.planning/phases/16-resource-limits/16-REVIEW.md`
**Iteration:** 1
**Scope:** critical + warning (3 findings)

**Summary:**
- Findings in scope: 3 (0 critical, 3 warnings — info findings deferred)
- Fixed: 3
- Skipped: 0
- Pre-existing test failures verified unrelated (4 baseline failures present on
  HEAD~3 before any Phase 16 fix landed; my changes do not introduce any new
  failures and clean clippy + cargo check on the entire `nono-cli` crate).

## Fixed Issues

### WR-01: Capability pipe server thread may outlive child process handle

**Files modified:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs`
**Commit:** `2c0596a`
**Applied fix:** Added `self.terminate_requested.store(true, Ordering::SeqCst)`
to both event-loop exit paths (timeout at lines 881-884 and natural-exit at
lines 901-904), and to `WindowsSupervisorRuntime::drop` (lines 956-961).
This ensures the capability pipe background thread observes termination and
exits its `recv_message` loop before the runtime drops the child `OwnedHandle`,
eliminating the dangling-handle race against `DuplicateHandle`.

### WR-03: --timeout deadline race against child natural exit

**Files modified:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs`
**Commit:** `e842713`
**Applied fix:** Inserted a non-blocking `child.poll_exit_code()?` call at the
top of `run_child_event_loop`'s tick (after `drain_capability_audit_entries`
and the `terminate_requested` check, before the deadline check). When a child
exits cleanly on the same 100 ms tick as deadline expiry, the natural-exit
path now fires first and the real exit code is reported instead of being
masked by `STATUS_TIMEOUT_EXIT_CODE` (`0x102`). The natural-exit branch sets
`terminate_requested` (per WR-01) and shuts down the same way as the
`wait_for_exit(100)` branch below.

### WR-02: Dead-code helpers updated with new `limits` parameter but still have no callers

**Files modified:**
  - `crates/nono-cli/src/exec_strategy_windows/launch.rs` (deletions)
  - `crates/nono-cli/src/exec_strategy_windows/mod.rs` (rationale comment)

**Commit:** `f52b84b`
**Applied fix:** Deleted the three dead `pub(super)` helpers from `launch.rs`
(`execute_direct_with_low_integrity`, `spawn_supervised_with_low_integrity`,
`spawn_supervised_with_standard_token` — 74 lines removed). Verified by
grep that no caller exists anywhere in the workspace.

The blanket `#![allow(dead_code)]` at `mod.rs:1` was retained because three
*pre-existing* dead symbols (`collect_unix_resource_limit_warnings` Windows
shim, `WindowsSupervisorDenyAllApprovalBackend` SC #4 fallback, and
`set_windows_wfp_test_force_ready` debug-only setter) live in this module
and removing those is out of Phase 16 scope. The blanket allow is now
preceded by a 13-line comment that names the three remaining symbols and
flags them for a follow-up cleanup, so the suppression is documented rather
than mute. The comment also explicitly forbids re-introducing broad allows
in `launch.rs`, locking in the WR-02 fix for future contributors.

## Verification

After all three fixes applied:
- `cargo check -p nono-cli` — passes clean.
- `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` — passes
  clean (no new warnings, no `unwrap_used` violations).
- `cargo test -p nono-cli --bin nono supervisor` — 14/14 supervisor tests pass.
- `cargo test -p nono-cli --bin nono apply_resource_limits` — 7/7 Job-Object
  RESL-01/02/04 regression tests pass (including
  `preserves_kill_on_job_close`).
- `cargo test -p nono-cli --bin nono` — 631/635 pass. The 4 failures
  (`capability_ext::tests::test_from_profile_*`,
  `profile::builtin::tests::test_all_profiles_signal_mode_resolves`,
  `query_ext::tests::test_query_path_sensitive_policy_includes_policy_source`)
  also fail on HEAD~3 (the pre-fix baseline) and are unrelated to Phase 16
  resource-limits work — they appear to be pre-existing windows-squash
  branch issues that should be addressed in a separate fix-up commit.

## Skipped Issues

None — all three in-scope warnings were fixed cleanly.

## Info findings (deferred per scope policy)

The following five `IN-*` findings are deferred (`fix_scope=critical_warning`):
- IN-01: Duplicated `format_bytes_human` / `format_duration_human` between
  `session_commands.rs` and `session_commands_windows.rs` (refactor candidate).
- IN-02: Capability pipe path placed in `std::env::temp_dir()` (defense-in-depth;
  attacker-influenceable TEMP can DoS bind, no confidentiality impact).
- IN-03: Imprecise overflow comment on `CpuRate` computation (doc fix).
- IN-04: Missing `proptest` round-trip for `ResourceLimits` (test-coverage gap).
- IN-05: `execute_direct` Windows path uses tight poll/sleep loop instead of
  `WaitForSingleObject(handle, INFINITE)` (perf, not correctness).

These can be addressed in a follow-up `IN-fix` pass or a Phase 17 polish
batch. None are security regressions and none block Phase 16 sign-off.

---

_Fixed: 2026-04-18_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
