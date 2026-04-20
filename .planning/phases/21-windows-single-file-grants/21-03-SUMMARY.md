---
phase: 21-windows-single-file-grants
plan: 03
subsystem: sandbox/windows
tags: [windows, filesystem, sandbox, wsfg, mandatory-label, compile-policy, apply, low-integrity, fail-closed]

# Dependency graph
requires:
  - plan: 21-02
    provides: nono::try_set_mandatory_label, nono::label_mask_for_access_mode, nono::low_integrity_label_and_mask (crate-root, Windows-cfg-gated)
  - plan: 21-01
    provides: WSFG-01 requirement ID
provides:
  - compile_filesystem_policy emits WindowsFilesystemRule entries for single-file grants (Read / Write / ReadWrite) and for write-only directory grants (unsupported vec remains declared but is always empty)
  - apply(&caps) iterates fs_policy.rules and calls try_set_mandatory_label(&rule.path, label_mask_for_access_mode(rule.access)) with ? propagation — fail-closed I-01
  - WINDOWS_SUPPORTED_DETAILS string updated to describe the new supported subset
  - Four new compile-policy unit tests (emits_rule_for_single_file_{read,write,read_write}_grant + emits_rule_for_write_only_directory_grant)
  - Two pre-existing compile_filesystem_policy_classifies_*_as_unsupported tests flipped to *_as_rule (renamed in place, assertions inverted)
  - Two pre-existing apply_rejects_unsupported_{single_file_grant,write_only_directory_grant} tests flipped to apply_accepts_*_and_labels_low_integrity with label-mask readback assertions
  - apply_error_message_remains_explicit_for_unsupported_subset repointed unconditionally to set_ipc_mode(IpcMode::Full) with "IPC mode" message-quality assertion
  - Rule-3 auto-fixes for three additional pre-phase-21 tests that assumed unsupported semantics (validate_launch_paths_rejects_single_file_policy_shapes_as_unsupported, preview_runtime_status_reports_requires_enforcement_for_single_file_policy, preview_runtime_status_reports_requires_enforcement_for_write_only_directory)
affects: [plan-21-04 (AppliedLabelsGuard RAII lifecycle will wrap apply() with revert-on-error semantics; apply() remains stateless), plan-21-05 (Phase 18 HUMAN-UAT re-run consumes the end-to-end integration)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "compile_filesystem_policy single-branch body: every FsCapability (file or directory, any AccessMode) compiles to exactly one WindowsFilesystemRule; the `unsupported` vec is declared but never populated. WindowsUnsupportedIssueKind::{SingleFileGrant, WriteOnlyDirectoryGrant} variants are retained in the enum (D-06 defers retirement); compile_filesystem_policy simply never emits them."
    - "apply() fail-closed label-apply step 1b: `for rule in &fs_policy.rules { try_set_mandatory_label(&rule.path, label_mask_for_access_mode(rule.access))?; }` — sits BETWEEN the fs-shape guard and the network-shape guard. Any Win32 error from SetNamedSecurityInfoW short-circuits with NonoError::LabelApplyFailed via `?`; apply() never returns Ok(()) after a partial label loop."
    - "Scope boundary: apply() is intentionally stateless — no revert of already-applied labels on mid-loop failure. Plan 21-04's AppliedLabelsGuard (CLI layer, exec_strategy_windows/labels_guard.rs) owns the revert lifecycle and always wraps the apply() call site."
    - "TDD gate sequence for Plan 21-03: two RED commits (test(21-03):, one per task) followed by two GREEN commits (feat(21-03):). Task 1 RED = 4 new tests; Task 1 GREEN = compile_filesystem_policy rewire + WINDOWS_SUPPORTED_DETAILS string + flipped classification tests + Rule-3 test fixes. Task 2 RED = flipped acceptance tests + repointed IPC-mode message test. Task 2 GREEN = apply() loop wiring."

key-files:
  created:
    - .planning/phases/21-windows-single-file-grants/21-03-SUMMARY.md — this file
  modified:
    - crates/nono/src/sandbox/windows.rs — compile_filesystem_policy body rewritten (single branch, always emits rules); apply() extended with step 1b label-apply loop; WINDOWS_SUPPORTED_DETAILS string updated; 4 new compile-policy tests added; 2 classification tests flipped in place; 2 acceptance tests flipped in place; 1 IPC-mode message test repointed; 3 Rule-3 auto-fix test renames/assertion flips. Net: +226 / -68 lines.

key-decisions:
  - "Inserted label-apply loop BETWEEN fs-shape guard and network-shape guard. Plan specified this order to ensure labels apply even if network validation would later fail (library-level apply() is stateless; Plan 21-04's CLI guard owns revert). Matches plan's `<action>` Edit 1 literal."
  - "Left the fs-policy-unsupported guard (`if !fs_policy.unsupported.is_empty()`) intact as defense-in-depth despite compile_filesystem_policy never populating `unsupported` post-Plan 21-03. If a future regression repopulates `unsupported`, the guard still fails closed with an explicit UnsupportedPlatform error. Matches plan's explicit `<action>` note."
  - "Rule 3 auto-fixes required for 3 additional pre-phase-21 tests (beyond the 5 the plan enumerated): `validate_launch_paths_rejects_single_file_policy_shapes_as_unsupported`, `preview_runtime_status_reports_requires_enforcement_for_single_file_policy`, `preview_runtime_status_reports_requires_enforcement_for_write_only_directory`. All three assumed the pre-phase-21 unsupported-shape semantics. The fix for each was identical in shape to the plan's explicit flips — rename the test to reflect the new supported semantic and invert the assertion (RequiresEnforcement -> AdvisoryOnly; expect_err -> expect). Plan output #3 explicitly anticipated surfacing a sixth such test — the actual surface was three."
  - "preview_runtime_status semantics: AdvisoryOnly is consistent for any fully-supported policy (no unsupported entries, no exec-dir coverage failure, no non-default IPC / extensions / platform rules). Single-file and write-only-directory grants are now parity with directory read / read-write grants, which already returned AdvisoryOnly. The label application itself happens during Sandbox::apply() (not during preview_runtime_status), so preview does not need a new reason label."
  - "validate_launch_paths_accepts_single_file_policy_shapes: the flipped test asserts Ok when the executable path IS the granted single file. Under the new regime, fs_policy.covers_path(&file, Read) returns true for a matching single-file rule (via windows_paths_equal_case_insensitive), and has_user_intent_directory_rules() returns false (skipping exec-dir check). Plan's pre-phase-21 version asserted expect_err — this is now semantically wrong under Phase 21."
  - "Post-Plan 21-03 sandbox::windows test count: 71 (up from 67 pre-plan). +4 new compile-policy tests. Net tests: 71 passing, 0 failing."
  - "Pre-existing trust::bundle::tests TUF failures (from Plan 21-02's deferred-items.md) remain — NOT introduced by Plan 21-03. cargo test -p nono --lib on Windows: 639 passed, 2 failed (both trust::bundle, out of scope per deferred-items.md)."
  - "D-21 Windows-invariance held: zero diff vs baseline on crates/nono/src/capability.rs, crates/nono/src/sandbox/linux.rs, crates/nono/src/sandbox/macos.rs, crates/nono/src/sandbox/mod.rs, crates/nono/src/error.rs, crates/nono/src/lib.rs. The entire plan diff is concentrated in crates/nono/src/sandbox/windows.rs."
  - "Plan 21-04 boundary respected: zero diff vs baseline on crates/nono-cli/ — no changes to exec_strategy_windows/. The three primitives consumed (try_set_mandatory_label, label_mask_for_access_mode, low_integrity_label_and_mask) are imported from nono:: via the crate-root re-exports Plan 21-02 published. apply()'s in-module calls resolve directly (same module)."

requirements-completed: [WSFG-01-compile-site-integration]
# Note: WSFG-01 has three parts (primitive, compile-site integration, RAII lifecycle). Plan 21-02 closed the primitive;
# Plan 21-03 closes the compile-site integration; Plan 21-04 will close the RAII lifecycle. Plan 21-05 is end-to-end UAT.

# Metrics
duration: ~25min
tasks: 2
commits: 4
completed: 2026-04-20
---

# Phase 21 Plan 21-03: Wire Mandatory-Label Primitive into apply() + compile_filesystem_policy Summary

**Closes D-06's two WindowsUnsupportedIssueKind emission paths (SingleFileGrant, WriteOnlyDirectoryGrant) by converting the two `unsupported.push(..)` sites in `compile_filesystem_policy` to a single `rules.push(..)` branch. Wires the Plan 21-02 `try_set_mandatory_label` primitive into `Sandbox::apply()` as a new step 1b — for every compiled filesystem rule, a mandatory-label ACE is applied fail-closed via `?` propagation. Updates the WINDOWS_SUPPORTED_DETAILS string, flips 2 classification tests + 2 rejection tests + 1 IPC-mode message test, and auto-fixes 3 additional pre-phase-21 tests that assumed the old unsupported-shape semantics. Enum shape preserved (I-02 / D-06 defer). Label revert-on-error is Plan 21-04's scope.**

## Plan 21-03 Output Deliverables

(The plan's output section specifies four items to record. Addressing each:)

### 1. Does `apply()` succeed on the integration host?

**Yes.** Both label-readback acceptance tests pass on x86_64-pc-windows-msvc without admin rights (user-created tempfiles, user-mode label operation):

```
test sandbox::windows::tests::apply_accepts_single_file_grant_and_labels_low_integrity ... ok
test sandbox::windows::tests::apply_accepts_write_only_directory_grant_and_labels_low_integrity ... ok
```

`low_integrity_label_and_mask(&file)` returns `Some((SECURITY_MANDATORY_LOW_RID as u32, NO_WRITE_UP | NO_EXECUTE_UP))` for a single-file Read grant, and `Some((SECURITY_MANDATORY_LOW_RID as u32, NO_READ_UP | NO_EXECUTE_UP))` for a write-only directory grant. The SDDL-constructed SACL in Plan 21-02's `try_set_mandatory_label` is round-trip-correct against `GetNamedSecurityInfoW` / `low_integrity_label_and_mask`.

**No `LabelApplyFailed` surfaced** — the Plan 21-02 primitive is production-correct on the integration host. HRESULT-surface coverage is still pending the Plan 21-05 HUMAN-UAT re-run.

### 2. Did the four new policy-compile tests reveal any edge case CONTEXT.md didn't anticipate?

**No.** All four `compile_filesystem_policy_emits_rule_for_*` tests pass on first attempt. The sort-and-dedup post-processing (lines 840-853 in windows.rs) handles the now-always-empty unsupported vec without issue — it's a no-op when the vec is empty, preserving the return-type invariant.

No cross-file / cross-access-mode interactions surfaced: a single grant produces exactly one rule, and the sort-stability vs the pre-phase-21 order did not affect any other test (the existing `compile_filesystem_policy_keeps_directory_rules` test still passes unchanged).

### 3. Any other existing tests needing updates beyond the explicit flip list?

**Yes — three additional tests.** The plan's Task 2 explicitly enumerated 5 tests to flip/repoint (`apply_rejects_unsupported_single_file_grant`, `apply_rejects_unsupported_write_only_directory_grant`, `compile_filesystem_policy_classifies_single_file_as_unsupported`, `compile_filesystem_policy_classifies_write_only_directory_as_unsupported`, `apply_error_message_remains_explicit_for_unsupported_subset`). Three additional tests also assumed the pre-phase-21 unsupported semantics and broke after Task 1 GREEN:

| Pre-Plan test name | Assumption violated | Post-Plan test name | Post-Plan assertion |
|---|---|---|---|
| `validate_launch_paths_rejects_single_file_policy_shapes_as_unsupported` | Policy is classified as unsupported → `validate_launch_paths` returns Err | `validate_launch_paths_accepts_single_file_policy_shapes` | Policy is fully supported; exe path matches the single-file rule → `validate_launch_paths` returns Ok |
| `preview_runtime_status_reports_requires_enforcement_for_single_file_policy` | `unsupported` is non-empty → `preview_runtime_status` collects a reason → `RequiresEnforcement` | `preview_runtime_status_allows_advisory_only_for_single_file_policy` | No unsupported entries + no directory-rule → no exec-dir check → no reasons → `AdvisoryOnly` (parity with dir read/read-write) |
| `preview_runtime_status_reports_requires_enforcement_for_write_only_directory` | Same as above | `preview_runtime_status_allows_advisory_only_for_write_only_directory` | Write-only dir covers exec_dir → no reasons → `AdvisoryOnly` |

All three were auto-fixed per Rule-3 (blocking-issue). All are in `crates/nono/src/sandbox/windows.rs::tests`. The plan's output section #3 explicitly anticipated a sixth test surfacing; the actual surface was three (captured here for completeness). None were in files Plan 21-03 is constrained to leave untouched — all live inside `sandbox/windows.rs`.

### 4. Exact IPC-mode error message observed

The repointed `apply_error_message_remains_explicit_for_unsupported_subset` input (`CapabilitySet::new().set_ipc_mode(IpcMode::Full)`) hits this branch in `apply()` at windows.rs:97:

```rust
if caps.ipc_mode() != crate::IpcMode::SharedMemoryOnly {
    return Err(NonoError::UnsupportedPlatform(
        "Windows sandbox does not support non-default IPC mode".to_string(),
    ));
}
```

The resulting `err.to_string()` is exactly:

> `Unsupported platform feature: Windows sandbox does not support non-default IPC mode`

(The `NonoError::UnsupportedPlatform(String)` variant's Display format prefixes `"Unsupported platform feature: "`.) The assertion `msg.contains("IPC mode")` matches the exact substring in the message, confirming the message-quality check is orthogonal to both `apply_rejects_capability_expansion_shape` (which uses `enable_extensions()` → "runtime capability expansion" message) and `apply_rejects_non_default_ipc_mode` (which asserts only the error variant, not the message content).

## Tasks Completed

### Task 1 — Rewire `compile_filesystem_policy` + update support-details + flip classification tests

Three edits in `crates/nono/src/sandbox/windows.rs`:

**A. `compile_filesystem_policy` body rewritten.** Replaced the three-branch conditional (is_file → SingleFileGrant push; dir+Write → WriteOnlyDirectoryGrant push; else → rule push) with a single unconditional `rules.push(WindowsFilesystemRule { .. })`. A block comment documents the D-06 decision (enum retained, emission behavior changed). The subsequent sort/dedup blocks are unchanged — `unsupported` is now always empty but post-processing is safe.

**B. `WINDOWS_SUPPORTED_DETAILS` string updated.** Dropped "Single-file grants, write-only directory grants" from the unsupported list; added "directory and single-file grants in read, write, and read-write modes (enforced via per-path mandatory integrity labels)" to the supported list. The existing `support_info_reports_supported_status_for_promoted_subset_contract` test (which asserts the details string contains no "partial") still passes — the new string contains no "partial".

**C. Four new compile-policy unit tests.** Inserted `compile_filesystem_policy_emits_rule_for_{single_file_read_grant, single_file_write_grant, single_file_read_write_grant, write_only_directory_grant}`. Each asserts `policy.unsupported.len() == 0`, `policy.rules.len() == 1`, and the correct `is_file` + `access` shape.

**D. Two pre-existing classification tests flipped.** `compile_filesystem_policy_classifies_single_file_as_unsupported` renamed in place to `compile_filesystem_policy_classifies_single_file_as_rule` with inverted assertions (`policy.rules.len() == 1`, `policy.unsupported.is_empty()`, `policy.rules[0].is_file == true`). Same flip for `compile_filesystem_policy_classifies_write_only_directory_as_unsupported` → `compile_filesystem_policy_classifies_write_only_directory_as_rule` (`is_file == false`, `access == Write`).

**E. Three Rule-3 auto-fix test renames/assertion flips.** Documented in output section #3 above.

**TDD gates:**
- RED commit `2054903` — four new tests fail with `left: 1, right: 0` (unsupported vec still has entries).
- GREEN commit `a59e978` — all seven compile_filesystem_policy tests pass (4 new + 2 flipped + 1 pre-existing `keeps_directory_rules`).

### Task 2 — Wire `try_set_mandatory_label` into `apply()` + flip rejection tests + repoint IPC-mode test

Two edits:

**A. `apply()` extended with step 1b label-apply loop.** Inserted between the fs-shape guard and the network-shape guard:

```rust
for rule in &fs_policy.rules {
    let mask = label_mask_for_access_mode(rule.access);
    try_set_mandatory_label(&rule.path, mask)?;
}
```

Fail-closed via `?` propagation (I-01). Block comment documents the scope boundary (stateless; Plan 21-04 owns revert-on-error).

**B. Flipped rejection tests.** `apply_rejects_unsupported_single_file_grant` → `apply_accepts_single_file_grant_and_labels_low_integrity` with end-to-end label-mask readback assertion. Same for write-only directory. Both tests exercise the full `apply()` + `low_integrity_label_and_mask` roundtrip on a NTFS tempdir.

**C. Repointed `apply_error_message_remains_explicit_for_unsupported_subset`.** Input changed from `FsCapability::new_file(&file, AccessMode::Read)` (now supported) to `CapabilitySet::new().set_ipc_mode(IpcMode::Full)` (still unsupported). Assertion changed from `msg.contains("single-file") || msg.contains("not support")` to `msg.contains("IPC mode") || msg.contains("ipc mode")`. Orthogonal to `apply_rejects_non_default_ipc_mode` which only checks the error variant.

**TDD gates:**
- RED commit `5526068` — the two flipped `apply_accepts_*_and_labels_low_integrity` tests fail because `apply()` does not yet invoke `try_set_mandatory_label` → `low_integrity_label_and_mask` returns None → `.expect(..)` panics. The repointed IPC-mode test already passes (the error message path in `apply()` was unchanged).
- GREEN commit `8c47a6b` — loop wired; both tests pass.

## Task Commits

Each task committed atomically with `--no-verify` (parallel executor) and DCO sign-off on branch `worktree-agent-aaf6168e`:

| # | Commit | Gate | Title |
|---|--------|------|-------|
| 1 | `2054903` | Task 1 RED  | `test(21-03): add failing tests for compile_filesystem_policy single-file/write-only-dir rule emission` |
| 2 | `a59e978` | Task 1 GREEN | `feat(21-03): rewire compile_filesystem_policy to emit rules for single-file and write-only-dir grants` |
| 3 | `5526068` | Task 2 RED  | `test(21-03): flip apply() rejection tests to acceptance tests + repoint IPC-mode message test` |
| 4 | `8c47a6b` | Task 2 GREEN | `feat(21-03): wire try_set_mandatory_label into apply() for every compiled filesystem rule` |

All four commits include `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` (DCO per CLAUDE.md § Commits).

## Renamed Tests Table

| Pre-Plan name | Post-Plan name | Category |
|---|---|---|
| `compile_filesystem_policy_classifies_single_file_as_unsupported` | `compile_filesystem_policy_classifies_single_file_as_rule` | Plan-listed flip |
| `compile_filesystem_policy_classifies_write_only_directory_as_unsupported` | `compile_filesystem_policy_classifies_write_only_directory_as_rule` | Plan-listed flip |
| `apply_rejects_unsupported_single_file_grant` | `apply_accepts_single_file_grant_and_labels_low_integrity` | Plan-listed flip |
| `apply_rejects_unsupported_write_only_directory_grant` | `apply_accepts_write_only_directory_grant_and_labels_low_integrity` | Plan-listed flip |
| `apply_error_message_remains_explicit_for_unsupported_subset` | *(unchanged name, input repointed to `IpcMode::Full`)* | Plan-listed repoint |
| `validate_launch_paths_rejects_single_file_policy_shapes_as_unsupported` | `validate_launch_paths_accepts_single_file_policy_shapes` | Rule-3 auto-fix |
| `preview_runtime_status_reports_requires_enforcement_for_single_file_policy` | `preview_runtime_status_allows_advisory_only_for_single_file_policy` | Rule-3 auto-fix |
| `preview_runtime_status_reports_requires_enforcement_for_write_only_directory` | `preview_runtime_status_allows_advisory_only_for_write_only_directory` | Rule-3 auto-fix |

## Deviations from Plan

1. **Three additional Rule-3 test fixes (validate_launch_paths_rejects_*, two preview_runtime_status_*).** Not in the plan's explicit flip list, but documented in the plan's output section #3 as an anticipated possibility. All three live inside `crates/nono/src/sandbox/windows.rs::tests` and assumed the pre-phase-21 unsupported-shape semantics — identical pattern to the five explicitly-flipped tests. No scope expansion beyond the single file the plan already constrains edits to.

2. **`cargo fmt --all` reformatted four `assert_eq!(..)` single-liners into multi-line shape** in the new compile-policy tests. Content unchanged; formatter reflow per repo defaults.

3. **`bindings/c/include/nono.h` showed as modified** (mtime only, zero diff) during Task 2 verification runs — `git checkout --` restored it cleanly. No content change.

## Issues Encountered

1. **Pre-existing `trust::bundle::tests` TUF failures** — same two tests that Plan 21-02 logged to `deferred-items.md`. Reproduced on `d92cd23^` (pre-plan baseline). Not introduced by Plan 21-03; unchanged from Plan 21-02's disposition.

2. **PreToolUse READ-BEFORE-EDIT hook reminders** (many occurrences) — fired on every Edit call despite every target file having been Read via the Read tool at session start. All edits completed successfully per the tool output; no edits lost. Same hook-tuning follow-up note as Plan 21-01 and 21-02.

3. **cargo test positional-arg limitation** — `cargo test` accepts only ONE positional `TESTNAME` argument. Ran targeted tests with a shared substring prefix when multiple flipped tests needed verification (e.g., `compile_filesystem_policy_emits_rule_for` matches all four new compile-policy tests). Noted; not a blocker.

## User Setup Required

None. All changes compile from the crate-root-published `nono::try_set_mandatory_label`, `nono::label_mask_for_access_mode`, and `nono::low_integrity_label_and_mask` re-exports Plan 21-02 already shipped. No new dependencies, no new env vars, no admin rights, no keystore entries. Downstream CLI (Plan 21-04) picks up the wired `apply()` automatically via the existing `Sandbox::apply()` facade.

## Verification Commands Re-Run Post-Commit

```
$ grep -c "unsupported.push" crates/nono/src/sandbox/windows.rs  # inside compile_filesystem_policy
0   # (3 total in the file; all 3 are in compile_supervisor_support, NOT compile_filesystem_policy)

$ grep -c "supports directory and single-file grants" crates/nono/src/sandbox/windows.rs
1   # new WINDOWS_SUPPORTED_DETAILS substring

$ grep -c "Single-file grants, write-only directory grants" crates/nono/src/sandbox/windows.rs
0   # old phrase gone

$ grep -c "fn compile_filesystem_policy_classifies_single_file_as_unsupported" crates/nono/src/sandbox/windows.rs
0   # old test gone

$ grep -c "fn compile_filesystem_policy_classifies_single_file_as_rule" crates/nono/src/sandbox/windows.rs
1   # flipped test present

$ grep -c "fn compile_filesystem_policy_classifies_write_only_directory_as_unsupported" crates/nono/src/sandbox/windows.rs
0

$ grep -c "fn compile_filesystem_policy_classifies_write_only_directory_as_rule" crates/nono/src/sandbox/windows.rs
1

$ grep -c "fn apply_rejects_unsupported_single_file_grant" crates/nono/src/sandbox/windows.rs
0

$ grep -c "fn apply_accepts_single_file_grant_and_labels_low_integrity" crates/nono/src/sandbox/windows.rs
1

$ grep -c "fn apply_rejects_unsupported_write_only_directory_grant" crates/nono/src/sandbox/windows.rs
0

$ grep -c "fn apply_accepts_write_only_directory_grant_and_labels_low_integrity" crates/nono/src/sandbox/windows.rs
1

$ grep -c "try_set_mandatory_label(&rule.path" crates/nono/src/sandbox/windows.rs
1   # single call site inside apply()

$ grep -c "label_mask_for_access_mode(rule.access)" crates/nono/src/sandbox/windows.rs
1

$ grep -c "try_set_mandatory_label" crates/nono/src/sandbox/windows.rs
6   # definition + call site + doc mentions (>= 3 per plan)

$ grep -c "set_ipc_mode(IpcMode::Full)" crates/nono/src/sandbox/windows.rs
3   # 2 test bodies + 1 doc-comment mention (>= 2 per plan)

$ grep -c "msg.contains(\"single-file\")" crates/nono/src/sandbox/windows.rs
0   # old assertion gone

$ git diff d92cd23e2da0ba1fcbe819fe2ce841c720dc9a62 -- crates/nono/src/sandbox/mod.rs | wc -l
0   # WindowsUnsupportedIssueKind enum shape preserved (I-02, D-06)

$ git diff d92cd23e2da0ba1fcbe819fe2ce841c720dc9a62 -- crates/nono/src/capability.rs crates/nono/src/sandbox/linux.rs crates/nono/src/sandbox/macos.rs crates/nono/src/sandbox/mod.rs crates/nono/src/error.rs crates/nono/src/lib.rs | wc -l
0   # D-21 Windows-invariance held

$ git diff d92cd23e2da0ba1fcbe819fe2ce841c720dc9a62 -- crates/nono-cli/ | wc -l
0   # Plan 21-04 boundary respected

$ cargo test -p nono --lib sandbox::windows --target x86_64-pc-windows-msvc 2>&1 | tail -1
test result: ok. 71 passed; 0 failed; 0 ignored; 0 measured; 570 filtered out; finished in 0.07s

$ cargo test -p nono --lib --target x86_64-pc-windows-msvc 2>&1 | tail -1
test result: FAILED. 639 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
# 2 failures are pre-existing trust::bundle TUF failures, logged to deferred-items.md by Plan 21-02; NOT introduced by Plan 21-03

$ cargo clippy -p nono --target x86_64-pc-windows-msvc --all-targets -- -D warnings -D clippy::unwrap_used 2>&1 | tail -1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.11s

$ cargo fmt --all -- --check
(clean — no output)

$ cargo build --workspace --target x86_64-pc-windows-msvc 2>&1 | tail -1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 28s

$ cargo test -p nono --doc --target x86_64-pc-windows-msvc 2>&1 | tail -1
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.83s

$ git log --oneline d92cd23e2da0ba1fcbe819fe2ce841c720dc9a62..HEAD
8c47a6b feat(21-03): wire try_set_mandatory_label into apply() for every compiled filesystem rule
5526068 test(21-03): flip apply() rejection tests to acceptance tests + repoint IPC-mode message test
a59e978 feat(21-03): rewire compile_filesystem_policy to emit rules for single-file and write-only-dir grants
2054903 test(21-03): add failing tests for compile_filesystem_policy single-file/write-only-dir rule emission
```

## TDD Gate Compliance

Plan 21-03 is not a `type: tdd` plan, but each of its two tasks carried `tdd="true"`. Both gate sequences fully observed:

**Task 1 gate sequence:**
- RED: `2054903` — four new compile-policy tests fail (`left: 1, right: 0` on unsupported-length assertion); the old implementation still pushes to unsupported.
- GREEN: `a59e978` — rewires compile_filesystem_policy body + flips classification tests + updates WINDOWS_SUPPORTED_DETAILS + auto-fixes three additional Rule-3 tests. All compile_filesystem_policy_* and preview_runtime_status_* and validate_launch_paths_* tests pass.

**Task 2 gate sequence:**
- RED: `5526068` — two flipped acceptance tests (`apply_accepts_*_and_labels_low_integrity`) fail because `apply()` does not yet invoke `try_set_mandatory_label`; `low_integrity_label_and_mask(&path)` returns None; `.expect(..)` panics. The repointed IPC-mode test already passes (message path unchanged).
- GREEN: `8c47a6b` — wires the label-apply loop into `apply()`. Both label-readback tests pass.

No REFACTOR phase needed — the initial GREEN implementations matched the plan spec precisely; `cargo fmt` reformatting was a formatter-driven reflow, not semantic refactoring.

## Self-Check: PASSED

- [x] `compile_filesystem_policy` body rewritten to single-branch `rules.push(..)` — verified via `grep -c "unsupported.push" = 0` inside the function body (3 matches elsewhere in compile_supervisor_support)
- [x] `apply()` invokes `try_set_mandatory_label(&rule.path, label_mask_for_access_mode(rule.access))?` for every rule — verified via `grep -c "try_set_mandatory_label(&rule.path" = 1`
- [x] Fail-closed `?` propagation — the loop uses `?` not `.ok()` or `.unwrap_or_default()`
- [x] `WINDOWS_SUPPORTED_DETAILS` updated — new substring "supports directory and single-file grants" present; old "Single-file grants, write-only directory grants" phrase removed
- [x] Four new `compile_filesystem_policy_emits_rule_for_*` tests added — all passing on Windows target
- [x] Two classification tests flipped (`*_classifies_*_as_unsupported` → `*_classifies_*_as_rule`) — old names gone, new names present, assertions inverted
- [x] Two rejection tests flipped (`apply_rejects_unsupported_*` → `apply_accepts_*_and_labels_low_integrity`) — old names gone, new names present, label-mask readback asserts the correct RID + mask
- [x] `apply_error_message_remains_explicit_for_unsupported_subset` repointed to `IpcMode::Full` + "IPC mode" assertion
- [x] Three Rule-3 auto-fix tests renamed (validate_launch_paths + two preview_runtime_status) — all passing
- [x] `WindowsUnsupportedIssueKind` enum shape preserved — zero diff on `crates/nono/src/sandbox/mod.rs` since baseline (I-02, D-06)
- [x] D-21 Windows-invariance held — zero diff on cross-platform files (capability.rs, linux.rs, macos.rs, mod.rs, error.rs, lib.rs)
- [x] Plan 21-04 boundary respected — zero diff on crates/nono-cli/ (no exec_strategy_windows changes)
- [x] No `.unwrap()` in production code — only in tests (permitted); clippy `-D warnings -D clippy::unwrap_used` green
- [x] `cargo test -p nono --lib sandbox::windows --target x86_64-pc-windows-msvc` passes (71 tests)
- [x] `cargo clippy -p nono --target x86_64-pc-windows-msvc --all-targets -- -D warnings -D clippy::unwrap_used` green
- [x] `cargo fmt --all -- --check` green
- [x] `cargo build --workspace --target x86_64-pc-windows-msvc` green
- [x] Four DCO-signed commits on worktree branch — verified via `git log` with `Signed-off-by:` trailers
- [x] SUMMARY.md created at `.planning/phases/21-windows-single-file-grants/21-03-SUMMARY.md`

## Next Plan Readiness

- **Plan 21-04** (AppliedLabelsGuard RAII lifecycle in `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs`) — can now wrap `nono::apply(caps)` calls with a pre-call snapshot (via `nono::low_integrity_label_and_mask`) and a post-exit revert. The `apply()` that Plan 21-03 ships is the bare stateless primitive; Plan 21-04 adds the lifecycle guard around it without modifying library code.

- **Plan 21-05** (Phase 18 HUMAN-UAT re-run) — now end-to-end-runnable. `nono run --profile claude-code` should no longer trip on the five `git_config` single-file grants (five `FsCapability::new_file(..)` caps that previously hit `SingleFileGrant`). The four HUMAN-UAT blocked items can be re-evaluated.

---
*Phase: 21-windows-single-file-grants*
*Plan: 21-03*
*Completed: 2026-04-20*
