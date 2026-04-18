---
phase: 19-cleanup
plan: 01
subsystem: testing
tags: [cargo-fmt, rustfmt, code-hygiene, ci-gate]

requires:
  - phase: 16-resource-limits
    provides: "Explicit deferral of fmt drift on 3 files (plan 16-02 SUMMARY § Deferred for follow-up bullet 5)"
provides:
  - "Clean `cargo fmt --all -- --check` exit on the entire workspace"
  - "Restored trust in `make ci`'s fmt-check gate"
  - "Closes CLEAN-01 requirement — drift from commit 6749494 (EnvVarGuard migration) neutralized"
affects: [19-02 CLEAN-02, 19-03 CLEAN-03, 19-04 CLEAN-04, any subsequent phase running make ci]

tech-stack:
  added: []
  patterns:
    - "Zero-logic fmt-only commits are labeled style(NN-CLEAN-NN):"

key-files:
  created:
    - .planning/phases/19-cleanup/19-01-SUMMARY.md
  modified:
    - crates/nono-cli/src/config/mod.rs
    - crates/nono-cli/src/exec_strategy_windows/restricted_token.rs
    - crates/nono-cli/src/profile/mod.rs

key-decisions:
  - "No `make ci` end-to-end smoke — CLEAN-02 still owns 5 pre-existing test flakes, so only `cargo fmt --all -- --check` was used as the smoke (per plan Task 3 step 3)."
  - "`make` binary is not installed on this Windows host; Makefile target `fmt-check` is exactly `cargo fmt --all -- --check`, which was run directly as the functional equivalent."

patterns-established:
  - "Per-file staging (no `git add -A`) for fmt-only commits"
  - "Single style(NN-CLEAN-NN) commit per fmt drift follow-up, scoped to the known-drifted files"

requirements-completed: [CLEAN-01]

duration: 4min
completed: 2026-04-18
---

# Phase 19 Plan 01: CLEAN-01 Summary

**`cargo fmt --all -- --check` is now clean on the entire workspace after re-formatting 3 files drifted by the EnvVarGuard migration (commit 6749494); no logic changes.**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-04-18T20:00:00Z (approx)
- **Completed:** 2026-04-18T20:04:15Z (commit timestamp)
- **Tasks:** 3 (Task 1 verification, Task 2 fix + commit, Task 3 Makefile chain assertion)
- **Files modified:** 3

## Accomplishments

- Confirmed the drift set matches D-03 exactly: `config/mod.rs`, `restricted_token.rs`, `profile/mod.rs` — no collateral drift elsewhere in the workspace.
- Re-formatted the 3 files via `cargo fmt --all`, staged them explicitly by path, and landed a single DCO-signed commit.
- `cargo fmt --all -- --check` now exits 0 on the whole workspace, restoring the `make ci → make check → make fmt-check` gate to a trustworthy signal.

## Task Commits

1. **Task 1: Verify pre-state and confirm 3 drifted files** — read-only verification, no commit (as specified by plan).
2. **Task 2: Apply `cargo fmt --all` and stage only the 3 expected files** — `c87b10b` (style)
3. **Task 3: Verify `make ci` gates fmt and run the plan smoke** — read-only verification, no commit (as specified by plan).

**Plan metadata:** to be attached in the closing docs commit alongside STATE.md / ROADMAP.md updates.

## Files Created/Modified

- `crates/nono-cli/src/config/mod.rs` — rustfmt canonicalized 2 `EnvVarGuard::set_all(&[…])` call formations (width wrap).
- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs` — rustfmt canonicalized 1 `use` list + 1 `assert!()` macro formation.
- `crates/nono-cli/src/profile/mod.rs` — rustfmt canonicalized 1 `EnvVarGuard::set_all(&[…])` call formation.
- `.planning/phases/19-cleanup/19-01-SUMMARY.md` — this file.

Diff stats: `3 files changed, 11 insertions(+), 13 deletions(-)`.

## Verification

| Check | Expected | Actual | Status |
|-------|----------|--------|--------|
| `cargo fmt --all -- --check` (post-fix) | exit 0 | exit 0 | PASS |
| Commit subject begins with `style(19-CLEAN-01):` | true | `style(19-CLEAN-01): cargo fmt drift follow-up from commit 6749494` | PASS |
| `Signed-off-by:` line count | 1 | 1 | PASS |
| Files in commit diff | exactly 3 (the target files) | exactly 3 (the target files) | PASS |
| `make fmt-check` exits 0 | true | Functionally equivalent `cargo fmt --all -- --check` exits 0; `make` binary unavailable on this host (documented below). | PASS (equivalent) |
| `grep -E "^check:.*fmt-check" Makefile` | 1 line | `71: check: clippy fmt-check` | PASS |
| `grep -E "^ci:.*check" Makefile` | 1 line | `122: ci: check test audit` | PASS |

### Task 1 pre-state confirmation

`cargo fmt --all -- --check` exited 1 pre-fix, with drift reported in exactly these three files (deduplicated):

```
crates/nono-cli/src/config/mod.rs
crates/nono-cli/src/exec_strategy_windows/restricted_token.rs
crates/nono-cli/src/profile/mod.rs
```

This matches D-03 exactly — no additional drift had accumulated between CONTEXT.md gathering and plan execution.

### Task 2 commit hash

`c87b10b` (full: `c87b10b160430417fff870b74adc41eb0e416176`) on branch `windows-squash`.

### Task 3 smoke result

`cargo fmt --all -- --check` → exit 0. This is the identical command invoked by the Makefile's `fmt-check` target (`cargo fmt --all -- --check`). The `make` binary is not installed on this Windows host, so the Makefile target was verified by reading the target body and invoking it directly; the grep assertions on `check:` and `ci:` lines prove the dependency chain is intact.

### `make ci` gated by CLEAN-02

Per plan Task 3 step 3: **`make ci` as a whole is NOT run here.** `make ci` chains to `make test`, and 5 pre-existing Windows test flakes (enumerated in 19-CONTEXT.md D-06) still fail. Those flakes are owned by plan 19-02 (CLEAN-02). Once 19-02 lands, `make ci` becomes fully green again; CLEAN-01's job was strictly to restore the format-check half of the `check` target.

## Commits

| # | Hash | Subject | Files | Kind |
|---|------|---------|-------|------|
| 1 | `c87b10b` | `style(19-CLEAN-01): cargo fmt drift follow-up from commit 6749494` | 3 | style (fmt-only) |

## Decisions Made

- **No functional changes intermixed** — the commit is pure `cargo fmt` output; the acceptance criterion that the diff touches exactly 3 files serves as the tamper check (T-19-01-01 mitigation).
- **Did not run `make ci` as smoke** — intentional per plan Task 3, since CLEAN-02 flakes would make the whole `ci` target red for reasons unrelated to fmt. Format-check-only smoke is the correct CLEAN-01 gate.
- **Documented `make` unavailability** — the Makefile target body `cargo fmt --all -- --check` was executed directly. Grep assertions on the Makefile prove the target and chain exist; running `make fmt-check` vs the raw cargo command is functionally identical, so this is recorded as an environment caveat rather than a deviation.

## Deviations from Plan

None — plan executed exactly as written. The sole environmental note (no `make` binary on this Windows host) did not change any task outcome; `cargo fmt --all -- --check` is byte-for-byte what `make fmt-check` would invoke.

## Issues Encountered

None.

## Self-Check: PASSED

- `crates/nono-cli/src/config/mod.rs` — FOUND (modified in commit c87b10b)
- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs` — FOUND (modified in commit c87b10b)
- `crates/nono-cli/src/profile/mod.rs` — FOUND (modified in commit c87b10b)
- Commit `c87b10b` — FOUND in `git log`
- `.planning/phases/19-cleanup/19-01-SUMMARY.md` — FOUND (this file)

## Next Phase Readiness

- Plan 19-02 (CLEAN-02) can now proceed against a fmt-clean baseline; any new drift it introduces will show up against a green `fmt-check` rather than being masked.
- `make ci` is still red overall because of CLEAN-02's 5 test flakes; that is the next plan to pick up.
- No blockers for 19-02, 19-03, or 19-04 — CLEAN plans are Wave 1 parallelizable (see 19-CONTEXT.md D-02) and 19-01's changes do not collide with any of their file sets.

---
*Phase: 19-cleanup*
*Completed: 2026-04-18*
