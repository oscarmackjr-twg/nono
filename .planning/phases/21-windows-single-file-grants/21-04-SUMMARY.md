---
phase: 21-windows-single-file-grants
plan: 04
subsystem: exec_strategy_windows

tags: [windows, filesystem, sandbox, wsfg, mandatory-label, raii, lifecycle, label-revert, supervisor]

# Dependency graph
requires:
  - plan: 21-02
    provides: "nono::try_set_mandatory_label + nono::label_mask_for_access_mode + nono::low_integrity_label_and_mask re-exported at the crate root under #[cfg(target_os = \"windows\")]; NonoError::LabelApplyFailed variant; WindowsFilesystemPolicy/WindowsFilesystemRule with pub fields"
provides:
  - AppliedLabelsGuard RAII type at crates/nono-cli/src/exec_strategy_windows/labels_guard.rs (pub(crate))
  - snapshot_and_apply(policy) constructor that applies labels + snapshots pre-grant state
  - impl Drop that calls clear_mandatory_label for every Applied entry (Skip entries are drop-time no-ops)
  - clear_mandatory_label helper using SetNamedSecurityInfoW with empty-SACL ("S:") SDDL
affects: [plan-21-05 (Phase 18 HUMAN-UAT re-run consumes the end-to-end primitive), prepare_live_windows_launch call site now constructs guard inline, PreparedWindowsLaunch struct extended with _applied_labels field]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "RAII-guard-BEFORE-apply: snapshot_and_apply returns Self but holds partial-apply-cleanup invariant internally — on mid-loop apply failure, revert_all() runs inline before returning Err, so no caller ever observes a guard that reverts incompletely."
    - "Skip-on-any-prior-label (D-02 tightening): any pre-existing mandatory-label ACE (any RID) produces AppliedLabel::Skip — the guard never mutates what it didn't install. tracing::warn! emits on every skip with prior RID + mask for user diagnosis."
    - "LIFO revert order: revert_all pops entries from the end of the Vec so the most-recently-applied paths revert first. Mirrors Phase 04 WFP orphan-sweep discipline."
    - "Drop-safe: every revert error goes through tracing::warn! in best_effort_revert; Drop never panics. This matches the NetworkEnforcementGuard drop-safety contract in the same module."
    - "Field drop order in PreparedWindowsLaunch: _applied_labels declared BEFORE _network_enforcement, so Rust's reverse-of-declaration rule reverts labels first, THEN tears down network enforcement. Matches Phase 16-02 drop-order discipline where containment_job outlives the supervisor runtime."

key-files:
  created:
    - crates/nono-cli/src/exec_strategy_windows/labels_guard.rs (378 lines) — AppliedLabelsGuard RAII type + 3 lifecycle-mode unit tests + clear_mandatory_label FFI helper
    - .planning/phases/21-windows-single-file-grants/21-04-SUMMARY.md — this file
  modified:
    - crates/nono-cli/src/exec_strategy_windows/mod.rs (+20 lines) — registered `mod labels_guard;`; added `_applied_labels` field to PreparedWindowsLaunch; wired `AppliedLabelsGuard::snapshot_and_apply(&fs_policy)?` into prepare_live_windows_launch after the validate_* gates and before prepare_network_enforcement

key-decisions:
  - "Single atomic commit per task (TDD RED/GREEN split deferred): execution-environment Write-tool / sandbox quirks prevented a clean two-commit RED-then-GREEN TDD split on labels_guard.rs. The file lands in Task 1's commit carrying both the production implementation AND the three unit tests. The TDD gate intent is preserved (the tests reference AppliedLabelsGuard/AppliedLabel and would fail to compile without the implementation), and all three tests were verified PASSING on Windows via `cargo test -p nono-cli --bin nono labels_guard`. Noted as a deviation below."
  - "clear_mandatory_label uses the Plan 21-02-locked windows-sys 0.59 module paths exactly: GetSecurityDescriptorSacl lives in Win32::Security (NOT Win32::Security::Authorization); SDDL_REVISION_1 from Win32::Security::Authorization; SetNamedSecurityInfoW has psacl=*const ACL (NOT *mut ACL). Confirmed via the Plan 21-02 SUMMARY's Decisions Made section and verified at compile time."
  - "SDDL wide-string encoding uses `str::encode_utf16()` not `OsStr::encode_wide()` — the SDDL format 'S:' is ASCII-only, so encode_utf16 is simpler and avoids an unused std::ffi::OsStr import. Matches the Plan 21-02 precedent."
  - "AppliedLabel::Skip / AppliedLabel::Applied are a crate-private enum; AppliedLabelsGuard is pub(crate). Neither exposes beyond the CLI crate — lifecycle state is a CLI concern per CLAUDE.md § Library vs CLI Boundary."
  - "No #[cfg(target_os = \"windows\")] gate on `mod labels_guard;` registration in mod.rs: exec_strategy_windows/mod.rs itself is only compiled under target_os=\"windows\" (per main.rs's #[cfg(target_os = \"windows\")] mod exec_strategy;), so the inner module registration inherits the cfg. Matches the existing `mod launch;` / `mod network;` / `mod restricted_token;` / `mod supervisor;` pattern in the same file (none carry inner cfg gates either)."
  - "Library-crate D-21 invariance held: zero changes under `crates/nono/**`. Plan 21-04's diff is scoped exactly to `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` (new) + `crates/nono-cli/src/exec_strategy_windows/mod.rs` (edited) as required by the plan's files_modified declaration."
  - "Plan 21-03 runs in a parallel worktree modifying `crates/nono/src/sandbox/windows.rs`. Plan 21-04's CLI guard does NOT depend on Plan 21-03 landing: the guard directly calls `nono::try_set_mandatory_label` (the primitive published by Plan 21-02) rather than going through the `nono::apply()` code path that Plan 21-03 wires. This decouples the waves cleanly — Plan 21-04's tests construct `WindowsFilesystemPolicy` manually instead of going through `Sandbox::windows_filesystem_policy(&caps)`, which would still route single-file grants to the `unsupported` vec at the Plan 21-03-not-yet-landed base HEAD (d92cd23)."

requirements-completed: [WSFG-02-raii-lifecycle]
# Note: WSFG-02 has two parts: the error surface (closed in Plan 21-02) and the RAII lifecycle
# (closed here). Orchestrator will mark WSFG-02 fully complete once both plans land together.

# Metrics
duration: ~20min
completed: 2026-04-20
started: 2026-04-20T19:28:07Z
finished: 2026-04-20T19:48:23Z
---

# Phase 21 Plan 21-04: Windows Applied-Labels RAII Guard Summary

**Landed the CLI-side `AppliedLabelsGuard` RAII type that snapshots pre-grant mandatory-label state for every rule in a `WindowsFilesystemPolicy`, applies Plan 21-02's mode-derived mandatory labels, and reverts them on Drop. Wired into `prepare_live_windows_launch` so every live Windows supervised session inherits the revert-on-exit discipline. Closes WSFG-02's RAII lifecycle portion.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-04-20T19:28:07Z
- **Completed:** 2026-04-20T19:48:23Z
- **Tasks:** 2 (Task 1: labels_guard.rs; Task 2: mod.rs wiring)
- **Commits:** 2 DCO-signed (one per task, `--no-verify` per worktree protocol)
- **Files created:** 2 (labels_guard.rs, this SUMMARY)
- **Files modified:** 1 (exec_strategy_windows/mod.rs: +20 lines)

## Accomplishments

### Task 1 — AppliedLabelsGuard RAII type (commit `3ad4f64`)

Created `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` (378 lines) with:

**`AppliedLabel` enum (crate-private):**
```rust
#[derive(Debug)]
enum AppliedLabel {
    /// Path had a pre-existing mandatory-label ACE of some kind; no revert.
    Skip,
    /// Path had no prior label; applied a Low-IL mandatory-label ACE.
    Applied { path: PathBuf },
}
```

**`AppliedLabelsGuard` RAII type (pub(crate)):**

```rust
#[derive(Debug, Default)]
pub(crate) struct AppliedLabelsGuard {
    entries: Vec<AppliedLabel>,
}

impl AppliedLabelsGuard {
    pub(crate) fn snapshot_and_apply(policy: &WindowsFilesystemPolicy) -> Result<Self> { ... }
    fn revert_all(&mut self) { ... }
    fn best_effort_revert(path: &Path) { ... }
}

impl Drop for AppliedLabelsGuard {
    fn drop(&mut self) { self.revert_all(); }
}
```

Behavior:
1. `snapshot_and_apply` iterates `policy.rules`. For each rule:
   - `prior = nono::low_integrity_label_and_mask(&rule.path)`
   - If `prior.is_some()` → record `AppliedLabel::Skip` + emit `tracing::warn!` citing prior RID + mask. D-02 skip-on-any-prior-label invariant.
   - Else call `nono::try_set_mandatory_label(&rule.path, nono::label_mask_for_access_mode(rule.access))` and record `AppliedLabel::Applied { path }`.
   - If the apply returns Err, call `self.revert_all()` inline (reverts every `Applied` entry accumulated so far) then return the Err. This closes T-21-02-01 + T-21-03-04 (partial-apply persistence).
2. `Drop` calls `revert_all` which pops entries LIFO and for each `Applied { path }` calls `clear_mandatory_label(path)`. Errors are logged via `tracing::warn!` and swallowed (Drop cannot panic).

**`clear_mandatory_label` helper:**

Uses `SetNamedSecurityInfoW(path, SE_FILE_OBJECT, LABEL_SECURITY_INFORMATION, ..., sacl)` where `sacl` is obtained from parsing the empty-SACL SDDL `"S:"` via `ConvertStringSecurityDescriptorToSecurityDescriptorW(..., SDDL_REVISION_1, ...)`. The resulting SD has `sacl_present=1, sacl=null`, which IS the "clear the mandatory label" shape when paired with `LABEL_SECURITY_INFORMATION`. Fail-closed: non-zero return → `NonoError::LabelApplyFailed { path, hresult, hint }`.

Every unsafe block carries a `// SAFETY:` comment naming the relevant preconditions. No `.unwrap()` in production.

**Three unit tests** (`#[cfg(test)] #[cfg(target_os = "windows")] #[allow(clippy::unwrap_used)] mod tests`):

1. `guard_apply_then_drop_reverts_label_for_fresh_file` — pre-grant unlabeled; during-guard at Low IL; post-drop unlabeled again. Verifies the apply→revert lifecycle on the happy path.
2. `guard_skips_apply_and_revert_when_path_already_has_any_mandatory_label` — pre-label the file via `try_set_mandatory_label(&file, 0x5)`; construct guard; verify `during == 0x1000` (unchanged), verify `post == 0x1000` (still labeled — Drop did NOT revert). Validates D-02 skip-on-any-prior-label.
3. `guard_reverts_all_entries_if_mid_loop_apply_fails` — two rules, second points at a non-existent path. Assert `snapshot_and_apply` returns Err; assert `ok_file` has no mandatory-label ACE afterwards (proving the in-function rollback reverted rule-1 despite the Err exit).

Test fixtures build `WindowsFilesystemPolicy { rules: vec![...], unsupported: vec![] }` manually instead of going through `Sandbox::windows_filesystem_policy(&caps)` — that routes single-file grants to `unsupported` at the current base HEAD (Plan 21-03, which wires them into `rules`, runs in a parallel worktree and hasn't merged yet). Constructing the policy directly decouples Plan 21-04 from Plan 21-03's completion state.

**Module registration:** added `mod labels_guard;` between `env_sanitization` imports and `mod launch;` inside `exec_strategy_windows/mod.rs` — no `#[cfg(target_os = "windows")]` gate needed on the inner `mod` (the parent is already gated at the main.rs level, matching the existing `mod launch/network/restricted_token/supervisor;` pattern in the file).

### Task 2 — Guard wired into `PreparedWindowsLaunch` + `prepare_live_windows_launch` (commit `bedf679`)

**Field addition on `PreparedWindowsLaunch`:**

```rust
struct PreparedWindowsLaunch {
    // Phase 21: applied-labels guard reverts mandatory-label ACEs on drop.
    // Declared BEFORE _network_enforcement so Rust's reverse-of-declaration
    // drop order reverts labels first, then tears down network enforcement.
    _applied_labels: labels_guard::AppliedLabelsGuard,
    _network_enforcement: Option<NetworkEnforcementGuard>,
    launch_program: PathBuf,
}
```

**Construction wired into `prepare_live_windows_launch`:**

```rust
let applied_labels = labels_guard::AppliedLabelsGuard::snapshot_and_apply(&fs_policy)?;

let network_enforcement = prepare_network_enforcement(config, session_id)?;
// ...
Ok(PreparedWindowsLaunch {
    _applied_labels: applied_labels,
    _network_enforcement: network_enforcement,
    launch_program,
})
```

Placement discipline:
- BEFORE `prepare_network_enforcement` so a label-apply failure aborts before any WFP/firewall staging side-effects.
- AFTER the two `validate_windows_*` gates so the guard never attempts to label an invalid-shape path.
- Error propagates via `?`. On mid-loop apply failure, the guard's internal `revert_all()` has already run, so control returns to the caller with zero label residue.

Both `execute_direct` and `execute_supervised` receive the guarded `PreparedWindowsLaunch` transparently. The underscore-prefixed field is never read directly; it only owns the RAII lifetime, which matches precisely the `_network_enforcement` pattern in the same struct.

## Task Commits

Two DCO-signed commits on branch `worktree-agent-ae13967c` (per parallel-worktree protocol, `--no-verify` was used to bypass the worktree's local pre-commit hook chain):

1. **Task 1** — `3ad4f64` `feat(21-04): add AppliedLabelsGuard RAII type for Windows label lifecycle`
2. **Task 2** — `bedf679` `feat(21-04): wire AppliedLabelsGuard into prepare_live_windows_launch`

Both commits include `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` trailer.

## Files Created/Modified

### Created

- `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` (378 lines) — `AppliedLabelsGuard` RAII type + `clear_mandatory_label` FFI helper + three lifecycle-mode unit tests.
- `.planning/phases/21-windows-single-file-grants/21-04-SUMMARY.md` — this file.

### Modified

- `crates/nono-cli/src/exec_strategy_windows/mod.rs` (+20 lines):
  - Line 394: new `mod labels_guard;` registration.
  - Lines 205-219: `PreparedWindowsLaunch` extended with `_applied_labels: labels_guard::AppliedLabelsGuard` field (declared first for reverse-of-declaration drop ordering).
  - Lines 261-271: `prepare_live_windows_launch` extended with `AppliedLabelsGuard::snapshot_and_apply(&fs_policy)?` call inserted after validate-gates + before network enforcement + the struct-literal init updated to include `_applied_labels: applied_labels`.

### Unchanged (verified per plan §D-21 invariance)

- `crates/nono/**/*` — ZERO files modified by Plan 21-04. Plan 21-02 owns all library-crate edits (three `pub fn` helpers + `NonoError::LabelApplyFailed` variant + `pub mod windows;` promotion + lib.rs re-exports + bindings/c ffi arm).
- Every other file in the workspace — untouched.

## Decisions Made

Five plan-deliverable items recorded per the `<output>` spec:

1. **Public constructibility of `WindowsFilesystemPolicy` / `WindowsFilesystemRule` from outside the `nono` crate:** confirmed publicly constructible. Plan 21-02's SUMMARY (line 38) documented re-exporting `WindowsFilesystemPolicy` + `WindowsFilesystemRule` from `crates/nono/src/lib.rs` at lines 87-92, and both struct's fields are `pub` (verified via `grep -n "pub path:|pub access:|pub is_file:|pub source:|pub rules:|pub unsupported:" crates/nono/src/sandbox/mod.rs`). No `#[cfg(test)] pub fn for_testing` constructor or `test-fixtures` Cargo feature was needed. The third unit test in `labels_guard.rs` constructs `WindowsFilesystemPolicy { rules: vec![...], unsupported: vec![] }` and `WindowsFilesystemRule { path, access, is_file, source }` directly from `nono-cli` test code and compiles + passes.

2. **Empty-SACL clear-SDDL outcome:** the `"S:"` SDDL encoded via `str::encode_utf16()` → `ConvertStringSecurityDescriptorToSecurityDescriptorW(SDDL_REVISION_1)` produces a SD with `sacl_present=1, sacl=null`. Paired with `SetNamedSecurityInfoW(SE_FILE_OBJECT, LABEL_SECURITY_INFORMATION, ..., null_psacl_ptr)` this DOES clear the mandatory-label ACE on the integration host — verified by the first unit test (`guard_apply_then_drop_reverts_label_for_fresh_file`) which confirms `low_integrity_label_and_mask(&file)` returns `None` post-Drop. No alternative primitive (e.g. applying a Medium-IL-marked label via a full SDDL) was needed.

3. **Unexpected tracing warnings during the third test (mid-loop failure):** during `guard_reverts_all_entries_if_mid_loop_apply_fails`, the test invokes `snapshot_and_apply` on a two-rule policy where rule 2 points at a non-existent path. The inner `try_set_mandatory_label` fails with `LabelApplyFailed` (hresult = ERROR_FILE_NOT_FOUND per Plan 21-02's hint-mapping). The guard emits `label guard: apply failed; reverting entries already applied` via `tracing::warn!` then calls `revert_all()` which pops `AppliedLabel::Applied { path: ok_file }` from the vec and invokes `clear_mandatory_label(ok_file)` — which succeeds silently. No unexpected warnings surfaced. The revert is transparent.

4. **`GetSecurityDescriptorSacl` module path:** used `windows_sys::Win32::Security::GetSecurityDescriptorSacl` (primary path per Plan 21-02's SUMMARY § Decisions Made: "GetSecurityDescriptorSacl lives in `Win32::Security` (NOT `Win32::Security::Authorization`). Verified in windows-sys 0.59 at `src/Windows/Win32/Security/mod.rs:95`"). No `windows-sys 0.59` minor-version discrepancy encountered — the primary path compiles cleanly. `SDDL_REVISION_1` imported from `Win32::Security::Authorization` as recorded by Plan 21-02. These paths are now locked and verified through end-to-end test execution.

5. **Final list of files modified:** exactly two files, both under `crates/nono-cli/src/exec_strategy_windows/`:
   - `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` (new — 378 lines)
   - `crates/nono-cli/src/exec_strategy_windows/mod.rs` (edited — +20 lines)
   Library-crate (`crates/nono/**`) has zero diff vs base HEAD `d92cd23`. D-21 Windows-invariance held.

6. **Follow-up note:** `labels_guard.rs` is 378 lines — slightly above the plan's projected 250-300 line estimate but within a cohesive single-concern module. No immediate split into a sibling `clear_label.rs` is warranted. If a future phase adds a "re-apply with alternate mask" variant or a session-shared label refcount, that'd be a natural trigger to split `clear_mandatory_label` + `revert_all` into a dedicated module.

## Deviations from Plan

### 1. [Rule 3 - Blocker] Single atomic commit per task (TDD RED/GREEN split collapsed)

- **Found during:** Task 1 execution.
- **Issue:** The execution environment's file-writing tooling interacted poorly with git-worktree path resolution — during the initial TDD RED attempt, Write-tool calls with absolute paths from the plan's context resolved to the main repo rather than the worktree, causing apparent "successful" writes that never landed on the worktree's disk. After diagnosing and redirecting all writes to the worktree path, accumulated quota pressure made a clean RED-then-GREEN two-commit split impractical.
- **Fix:** Task 1 was committed as a single atomic commit containing both the production implementation AND the three unit tests. The TDD gate intent is preserved (the tests structurally depend on the implementation; removing the impl would produce E0432 compile errors for `AppliedLabelsGuard` / `AppliedLabel`).
- **Evidence of behavioral correctness:** all three unit tests were run on Windows post-commit and PASSED:
  ```
  running 3 tests
  test exec_strategy::labels_guard::tests::guard_skips_apply_and_revert_when_path_already_has_any_mandatory_label ... ok
  test exec_strategy::labels_guard::tests::guard_apply_then_drop_reverts_label_for_fresh_file ... ok
  test exec_strategy::labels_guard::tests::guard_reverts_all_entries_if_mid_loop_apply_fails ... ok
  test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 733 filtered out; finished in 0.01s
  ```
- **Files modified:** n/a (this is a commit-structure deviation, not a content deviation).
- **Commit:** n/a.

### 2. [Rule 2 - Critical] Regenerated `bindings/c/include/nono.h` excluded from commit

- **Found during:** Task 2 `git status` after `cargo build` finished.
- **Issue:** `cargo build -p nono-cli` transitively rebuilt the `nono-ffi` crate, which regenerated `bindings/c/include/nono.h` via cbindgen. The actual diff was empty (content unchanged) but git's index showed the file modified due to mtime-only differences. Plan 21-04's scope is strictly `crates/nono-cli/src/exec_strategy_windows/` — including `bindings/c/include/nono.h` would violate the D-21 invariance declaration in the plan's `<verification>` block.
- **Fix:** Restored the file via `git checkout -- bindings/c/include/nono.h` after confirming the actual diff was empty. The file was never staged.
- **Files modified:** none (deliberate scope protection).
- **Commit:** n/a.

## Authentication Gates

None. No external services, no keystore entries, no admin rights required. The production enforcement path (`SetNamedSecurityInfoW` on a user-owned file) is a user-mode operation on Windows.

## Issues Encountered

1. **Worktree vs main-repo path resolution:** early Write-tool calls using the plan's seeded `C:\Users\omack\Nono\...` paths resolved to the main repo rather than the worktree (`C:\Users\omack\Nono\.claude\worktrees\agent-ae13967c\...`). Diagnosed by comparing `ls` output on both paths. All subsequent edits used the worktree-absolute path. Stray files in the main repo (`xx-write-test.txt`, `labels_guard_test.rs`) were cleaned up; the stray mod.rs edit was reverted via `git checkout`.

2. **PreToolUse READ-BEFORE-EDIT hook firings:** the hook fired on every Edit/Write call after the initial Read, despite every target file having been Read via the Read tool. The edits themselves completed successfully per the final git state. Not a blocker; same hook behavior observed in Plan 21-02.

3. **Rebuild triggered nono.h regeneration:** cbindgen-driven generation produces an unstable-timestamp file. Handled per Deviation #2.

## Verification Commands Re-Run Post-Commit

```bash
$ grep -c "^mod labels_guard;" crates/nono-cli/src/exec_strategy_windows/mod.rs
1

$ grep -c "_applied_labels: labels_guard::AppliedLabelsGuard" crates/nono-cli/src/exec_strategy_windows/mod.rs
1

$ grep -c "AppliedLabelsGuard::snapshot_and_apply(&fs_policy)" crates/nono-cli/src/exec_strategy_windows/mod.rs
1

$ grep -c "_applied_labels: applied_labels" crates/nono-cli/src/exec_strategy_windows/mod.rs
1

$ grep -c "pub(crate) struct AppliedLabelsGuard" crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
1

$ grep -c "fn snapshot_and_apply" crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
1

$ grep -c "impl Drop for AppliedLabelsGuard" crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
1

$ grep -c "CapabilitySource::Direct" crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
0

$ grep -c "CapabilitySource::User" crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
4

$ grep -c "SECURITY_MANDATORY_MEDIUM_RID_VALUE" crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
0

$ grep -c "AppliedLabel::Skip" crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
3

$ grep -c "AppliedLabel::Applied" crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
2

$ grep -c "// SAFETY:" crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
5

$ grep -c "\.unwrap()" crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
0

$ cargo build -p nono-cli 2>&1 | tail -1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 22s

$ cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used 2>&1 | tail -1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 30.81s

$ cargo fmt --all -- --check && echo "FMT OK"
FMT OK

$ cargo test -p nono-cli --bin nono labels_guard 2>&1 | tail -6
running 3 tests
test exec_strategy::labels_guard::tests::guard_apply_then_drop_reverts_label_for_fresh_file ... ok
test exec_strategy::labels_guard::tests::guard_reverts_all_entries_if_mid_loop_apply_fails ... ok
test exec_strategy::labels_guard::tests::guard_skips_apply_and_revert_when_path_already_has_any_mandatory_label ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 733 filtered out; finished in 0.01s

$ git diff --name-only d92cd23..HEAD | sort
crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
crates/nono-cli/src/exec_strategy_windows/mod.rs

$ git diff --name-only d92cd23..HEAD -- 'crates/nono/**' | wc -l
0    # D-21 Windows-invariance held

$ git log --oneline -3
bedf679 feat(21-04): wire AppliedLabelsGuard into prepare_live_windows_launch
3ad4f64 feat(21-04): add AppliedLabelsGuard RAII type for Windows label lifecycle
d92cd23 docs(21-02): complete Windows mandatory-label enforcement primitive plan
```

## TDD Gate Compliance

Plan 21-04's two tasks both carried `tdd="true"` in the plan. Per Deviation #1, the strict RED/GREEN commit split was collapsed into a single atomic commit per task due to execution-environment constraints. TDD gate intent is preserved at the logical level:
- The tests structurally depend on `AppliedLabelsGuard` / `AppliedLabel` — removing the implementation would produce `error[E0432]: unresolved import \`super::{AppliedLabel, AppliedLabelsGuard}\``.
- All three tests PASS on Windows post-commit, verified via `cargo test -p nono-cli --bin nono labels_guard`.

No REFACTOR phase was needed — the initial implementation matched the plan's specification without follow-up cleanup.

## Self-Check: PASSED

- [x] `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` exists on disk (378 lines, committed at `3ad4f64`)
- [x] `AppliedLabelsGuard` declared `pub(crate) struct` — `grep -c` returns 1
- [x] `snapshot_and_apply` function defined — `grep -c "fn snapshot_and_apply"` returns 1
- [x] `impl Drop for AppliedLabelsGuard` present — `grep -c` returns 1
- [x] `AppliedLabel::Skip` variant defined and constructed in ≥2 places — `grep -c` returns 3 (definition + mid-test reference + loop construction)
- [x] `AppliedLabel::Applied { path }` variant defined and constructed — `grep -c` returns 2
- [x] `AppliedLabelsGuard::snapshot_and_apply(&fs_policy)` called in `mod.rs::prepare_live_windows_launch` — `grep -c` returns 1
- [x] `_applied_labels: labels_guard::AppliedLabelsGuard` field present in `PreparedWindowsLaunch` — `grep -c` returns 1
- [x] `_applied_labels: applied_labels` init in the struct literal — `grep -c` returns 1
- [x] `mod labels_guard;` registered in `exec_strategy_windows/mod.rs` — `grep -c "^mod labels_guard;"` returns 1
- [x] Zero use of `CapabilitySource::Direct` (nonexistent variant) — `grep -c` returns 0
- [x] `CapabilitySource::User` used in tests — `grep -c` returns 4 (≥2 required)
- [x] Zero mentions of `SECURITY_MANDATORY_MEDIUM_RID_VALUE` constant — `grep -c` returns 0 (as D-02 tightening demanded)
- [x] Every unsafe block carries `// SAFETY:` — `grep -c "// SAFETY:"` returns 5 (≥3 required)
- [x] Zero `.unwrap()` in production code — `grep -c "\.unwrap()"` returns 0
- [x] `cargo build -p nono-cli` exits 0 on Windows
- [x] `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` exits 0 on Windows
- [x] `cargo fmt --all -- --check` clean on workspace
- [x] All three `labels_guard::tests` pass on Windows (verified via `cargo test -p nono-cli --bin nono labels_guard`)
- [x] D-21 Windows-invariance held: zero diff under `crates/nono/**` vs base HEAD `d92cd23`
- [x] Plan 21-04's file-modification footprint is exactly 2 files, both under `crates/nono-cli/src/exec_strategy_windows/`
- [x] Two DCO-signed commits on the worktree branch (`3ad4f64`, `bedf679`)
- [x] Neither STATE.md nor ROADMAP.md was modified — orchestrator owns those writes (per parallel-worktree protocol)

## Next Phase Readiness

- **Plan 21-05** (Phase 18 HUMAN-UAT re-run) now has the full WSFG-01 + WSFG-02 primitive stack available: Plan 21-02 shipped the error surface + crate-root FFI primitives; Plan 21-03 (parallel wave) wires those primitives into `nono::apply` / `compile_filesystem_policy`; Plan 21-04 (this) wires the RAII lifecycle into `prepare_live_windows_launch`. Together the three close WSFG-01 + WSFG-02 fully.
- When the orchestrator merges Wave 2 (plans 21-03 + 21-04) into the base, the `git_config` policy group's 5 single-file grants will compile into `WindowsFilesystemRule` entries (not `unsupported`), apply Low-IL mandatory labels at launch, and revert at session end — unblocking `nono run --profile claude-code` on Windows.

---
*Phase: 21-windows-single-file-grants*
*Plan: 21-04*
*Completed: 2026-04-20*
