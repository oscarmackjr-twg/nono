---
phase: 12-milestone-bookkeeping-cleanup
plan: 03
subsystem: testing
tags: [ci, clippy, fmt, verification, windows-squash]

requires:
  - phase: 12-milestone-bookkeeping-cleanup
    provides: "Wave 1 planning-trail edits (12-01) and code tech-debt cleanup (12-02)"
provides:
  - "CI gate verification result for Phase 12 Wave 1"
  - "Surfaced pre-existing clippy failure on windows-squash branch (48 disallowed_methods errors across profile/mod.rs, config/mod.rs, sandbox_state.rs) — NOT caused by Phase 12"
affects: [13-uat-archive, follow-up-env-var-guard-migration]

tech-stack:
  added: []
  patterns:
    - "Verification-only plans must still document exact commands and exit codes for the milestone audit trail"

key-files:
  created:
    - .planning/phases/12-milestone-bookkeeping-cleanup/12-03-SUMMARY.md
  modified: []

key-decisions:
  - "STOP on CI failure rather than auto-fix, per plan directive and CLAUDE.md fail-secure principle"
  - "Confirmed failure is pre-existing (introduced by cf5a60a revert dated 2026-04-10, before Phase 11 and 12 started)"
  - "Phase 12's own files (crates/nono/src/sandbox/windows.rs, crates/nono-cli/tests/wfp_port_integration.rs) have zero clippy errors"

patterns-established:
  - "Pre-existing CI debt on a branch must be surfaced as a follow-up plan, not silently absorbed into an unrelated phase"

requirements-completed: []

duration: 6min
completed: 2026-04-11
---

# Phase 12 Plan 03: CI Gate Verification Summary

**`make ci` fallback surfaced 48 pre-existing clippy `disallowed_methods` errors on the `windows-squash` branch; Phase 12's own edits are clippy- and fmt-clean, fmt-check passes, but the CI gate does not exit 0 and requires a follow-up plan to migrate the flagged tests to `EnvVarGuard`.**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-04-11T22:25:00Z
- **Completed:** 2026-04-11T22:31:08Z
- **Tasks:** 1 (verification only)
- **Files modified:** 0 (verification plan)

## Accomplishments

- Ran the `make ci` fallback sequence (make is not available on this Windows bash host)
- Confirmed `cargo fmt --all -- --check` exits 0 (no format drift)
- Confirmed `cargo clippy --all-targets --all-features -- -D warnings -D clippy::unwrap_used` fails with **48 pre-existing `disallowed_methods` errors**
- Verified via `git log` and grep that all 48 failures live in files **not touched by Phase 12** and that the lint regression was introduced by commit `cf5a60a` (revert) dated 2026-04-10, predating both Phase 11 and Phase 12
- Verified Phase 12-02's target files (`crates/nono/src/sandbox/windows.rs`, `crates/nono-cli/tests/wfp_port_integration.rs`) produce zero clippy diagnostics
- Did NOT run `cargo test --workspace` because clippy failed and the plan directive is to STOP on failure without auto-fix

## Commands Run

1. `which make` → not available
2. `cargo clippy --all-targets --all-features -- -D warnings -D clippy::unwrap_used` → **FAIL** (48 errors, `nono-cli` bin "nono" test target failed to compile)
3. `cargo fmt --all -- --check` → **PASS** (exit 0)
4. `cargo test --workspace` → **NOT RUN** (skipped per plan STOP directive)

**Final CI gate status:** FAIL (clippy)

## Task Commits

1. **Task 1: Run `make ci` and confirm it passes** — verification only, no commit for the task itself.

**Plan metadata:** (pending — SUMMARY + STATE/ROADMAP update commit)

## Files Created/Modified

- `.planning/phases/12-milestone-bookkeeping-cleanup/12-03-SUMMARY.md` — this file

## Clippy Failure Details

All 48 errors are `clippy::disallowed_methods` against `std::env::set_var` / `std::env::remove_var` in tests. The project-level lint (added in commit `b489f14 fix(test): add clippy disallowed_methods lint and migrate remaining unguarded env var tests`) mandates using `crate::test_env::EnvVarGuard` instead of raw env mutation to avoid flaky parallel-test failures.

### Error locations (3 files, 48 errors total)

| File | Error count | Line range |
|------|-------------|------------|
| `crates/nono-cli/src/profile/mod.rs` | 30 | 1797–2007 |
| `crates/nono-cli/src/config/mod.rs` | 12 | 295–334 |
| `crates/nono-cli/src/sandbox_state.rs` | 6 | 401–417 |

**Representative error:**

```
error: use of a disallowed method `std::env::set_var`
   --> crates\nono-cli\src\sandbox_state.rs:401:9
    |
401 |         std::env::set_var("TMP", dir.path());
    |         ^^^^^^^^^^^^^^^^^
    |
    = note: env var mutation is not thread-safe in tests; use crate::test_env::EnvVarGuard instead
            (see crates/nono-cli/src/test_env.rs)

error: could not compile `nono-cli` (bin "nono" test) due to 48 previous errors
```

### Root cause (not Phase 12)

- Commit `cf5a60a` *"Revert 'chore: merge executor worktree (worktree-agent-a6204f48)'"* dated **2026-04-10 09:12:48 -0400** reverted a batch of EnvVarGuard migrations while leaving the `disallowed_methods` clippy lint in place.
- That revert predates all Phase 11 work and all Phase 12 work on the `windows-squash` branch.
- `git log cf5a60a..HEAD -- .` shows only Phase 11 and Phase 12 commits touching `crates/nono/src/sandbox/windows.rs`, `crates/nono-cli/tests/wfp_port_integration.rs`, capability-pipe files, and planning-trail docs — none of which touch the three files that fail clippy.
- Grep of clippy output against the Phase 12-02 target paths returns **zero matches**:
  - `crates/nono/src/sandbox/windows.rs` — clean
  - `crates/nono-cli/tests/wfp_port_integration.rs` — clean

### Attribution

The CI failure is attributable to commit `cf5a60a` (pre-existing), **not** to:
- `b30a9c6 docs(12-02): replace stale placeholder module doc in sandbox/windows.rs`
- `0ac3193 refactor(12-02): use ephemeral loopback ports in wfp_port test`
- Any 12-01 planning-trail edit (those touch only `.planning/` markdown files)

Phase 12 introduced zero new clippy warnings.

## Decisions Made

- **STOP on clippy failure, do not auto-fix.** The plan explicitly directs: *"If `make ci` fails, capture the exact failing step and error output in the SUMMARY, STOP, do NOT attempt auto-fix — surface the failure for a follow-up plan."*
- **Did not run `cargo test` after clippy failed.** The CI gate short-circuits on the first failing step and the plan's success criterion is the composite gate, so running tests separately would not change the outcome.
- **Kept the SUMMARY honest about the composite exit status.** Even though Phase 12's own edits are clean, the literal acceptance criterion "`make ci` exits with status 0" is NOT met. Phase 12's Plan 03 cannot claim success on that criterion without a follow-up plan that either migrates the flagged tests to `EnvVarGuard` or reverts/adjusts the `disallowed_methods` lint.

## Deviations from Plan

None — plan executed exactly as written, including the explicit STOP-on-failure directive.

## Issues Encountered

**Pre-existing clippy regression on `windows-squash` branch.** 48 `disallowed_methods` errors in `profile/mod.rs`, `config/mod.rs`, and `sandbox_state.rs`. Surfaced, root-caused to commit `cf5a60a`, and documented. **Not fixed in this plan.** Recommended follow-up: a new quick plan to migrate the flagged tests to `EnvVarGuard::set()` / `EnvVarGuard::remove()` per `crates/nono-cli/src/test_env.rs`.

## Next Phase Readiness

- **Phase 12 success criterion 6 (`make ci` passes) is NOT satisfied** on `windows-squash` HEAD. Phase 12 cannot be marked fully complete until the follow-up CI-fix plan lands.
- Phase 12's own tech-debt fixes (12-02) and planning trail edits (12-01) are verified clean.
- Recommended next step: Create a quick plan `/gsd:quick` titled "migrate flagged env-var tests to EnvVarGuard" to restore the CI gate, then re-run this verification before moving to Phase 13 UAT archive.

## Self-Check: PASSED

- Created file exists: `.planning/phases/12-milestone-bookkeeping-cleanup/12-03-SUMMARY.md` — FOUND
- No task commits expected (verification-only plan with STOP on failure)

---
*Phase: 12-milestone-bookkeeping-cleanup*
*Completed: 2026-04-11*
