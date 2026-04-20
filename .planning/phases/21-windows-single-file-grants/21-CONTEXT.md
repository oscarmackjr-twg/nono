---
phase: 21-windows-single-file-grants
status: ready-for-planning
gathered: 2026-04-20
---

# Phase 21: Windows Single-File Filesystem Grants — Context

**Gathered:** 2026-04-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Extend the Windows filesystem sandbox backend (`crates/nono/src/sandbox/windows.rs`) so that capability grants scoped to a single file are enforceable instead of rejected. Today `compile_filesystem_policy` rejects `FsCapability { is_file: true, .. }` with `WindowsUnsupportedIssueKind::SingleFileGrant`, and rejects directory grants with `AccessMode::Write` with `WindowsUnsupportedIssueKind::WriteOnlyDirectoryGrant`. Both become enforceable in this phase.

Concrete motivator: the `claude-code` profile's `git_config` policy group grants read access to `$HOME/.gitconfig`, `$HOME/.gitignore_global`, `$HOME/.config/git/config`, `$HOME/.config/git/ignore`, `$HOME/.config/git/attributes`. Today any `nono run --profile claude-code` on Windows fails before launch because these five single-file grants all trip `SingleFileGrant`.

**In scope:**
- Per-file enforcement for `AccessMode::Read`, `AccessMode::Write`, and `AccessMode::ReadWrite`.
- Directory-scope `AccessMode::Write` enforcement (closes `WriteOnlyDirectoryGrant`).
- Label lifecycle: apply at launch, revert at session end.
- Platform-split tests (cross-platform unit + Windows integration).
- Phase 18 HUMAN-UAT cookbook re-run as phase close-out (unblocks Path B + Path C, all 4 live-CONIN$ tests).

**Out of scope (deferred):**
- Full removal of the `WindowsUnsupportedIssueKind` enum + its plumbing — defer to a follow-up cleanup phase once v2.1 ships. (Keep the enum as an empty-or-reserved shape during this phase so future unsupported shapes have a home.)
- Windows runtime capability expansion / `--trust` handle brokering of filesystem requests.
- Cross-platform filesystem parity for handle brokering (separate cross-platform requirement, out of v2.1 per REQUIREMENTS.md).
- Kernel minifilter driver (Gap 6b, deferred to v3.0 per STATE.md).

</domain>

<decisions>
## Implementation Decisions

### Enforcement Primitive

- **D-01:** **Per-file Low IL mandatory label.** Apply `SYSTEM_MANDATORY_LABEL_ACE` at `SECURITY_MANDATORY_LOW_RID` directly to each granted file via `SetNamedSecurityInfoW(path, SE_FILE_OBJECT, LABEL_SECURITY_INFORMATION, ..)`. This mirrors the existing directory-scope primitive exactly (same API, same integrity model, same Low IL restricted-token child). The mandatory-label ACE `Mask` field encodes the access mode:
  - `AccessMode::Read` → `SYSTEM_MANDATORY_LABEL_NO_WRITE_UP | SYSTEM_MANDATORY_LABEL_NO_EXECUTE_UP` (child can read, cannot write)
  - `AccessMode::Write` → `SYSTEM_MANDATORY_LABEL_NO_READ_UP | SYSTEM_MANDATORY_LABEL_NO_EXECUTE_UP` (child can write, cannot read)
  - `AccessMode::ReadWrite` → `SYSTEM_MANDATORY_LABEL_NO_EXECUTE_UP` only (child can read + write, cannot execute)
  DACL-based enforcement and clone-to-runtime-dir approaches were considered and rejected: DACL does not bypass the integrity-check at the heart of the Low IL model (it reduces to a hybrid of 1 anyway), and clone-to-runtime-dir breaks read-write grants (writes would land in the clone, not the user's real file — racy if other processes also write).

### Label Lifecycle

- **D-02:** **Idempotent apply, revert on session end.** Supervisor records the pre-grant label state for each target file. On `CreateFile`-style label inspection, if the file is already at Low IL (for reasons unrelated to this session), we no-op the apply and skip the revert (no state to restore). If we applied the label, we revert it at session exit / detach-and-stop via the supervisor's existing on-exit cleanup path (sits alongside the WFP orphan sweep from Phase 04). Concurrent sessions sharing the same file get "last session out restores" semantics — acceptable trade-off versus refcount/lease plumbing.

- **D-03:** **Reject write-mode grants to non-existent files at compile time.** `FsCapability::new_file(path, AccessMode::Write | AccessMode::ReadWrite)` must already resolve the path at construction. Missing-file → `PathNotFound` error → `compile_filesystem_policy` never sees a write-mode grant for a file that doesn't exist. If the user needs write-to-new-file semantics, the contract is: grant the parent directory (directory-scope grant covers file creation), or touch the file before running. This preserves least-surprise (nono never creates files on behalf of the user as a side effect of sandboxing).

### Error Surface

- **D-04:** **Named error with path + Win32 cause on label-apply failure.** When `SetNamedSecurityInfoW` returns non-zero (permission denied, ReFS / network share without NTFS-style integrity labels, file held-open by another process, etc.), surface through a Windows-specific error variant carrying (a) the exact path that failed, (b) the Win32 error code, (c) an actionable hint. Matches the diagnostic precedent set by Phase 09 (`ERROR_PIPE_BUSY` → friendly `AttachBusy`) and Phase 17's single-attach friendly message. Fail-closed: launch aborts, no fallback to broader grant. (`crates/nono/src/error.rs` variant shape — new variant `LabelApplyFailed { path, hresult, hint }` or extension of existing `UnsupportedPlatform` — is a planning-phase decision, but the diagnostic quality bar is locked here.)

### Access Modes

- **D-05:** **All three access modes supported at the file level.** Read / Write / ReadWrite. The label mask encodes the mode (see D-01), so there's no mechanism-level reason to support a subset; the type surface (`AccessMode` enum + `FsCapability::is_file`) already carries the information. Full parity with directory-scope grants.

### Unsupported-Variant Scope

- **D-06:** **Close both `WindowsUnsupportedIssueKind` variants in this phase.** `SingleFileGrant` and `WriteOnlyDirectoryGrant` both become enforceable:
  - `SingleFileGrant`: covered by D-01 per-file label with mode mask.
  - `WriteOnlyDirectoryGrant`: the same primitive applied at directory scope with `NO_READ_UP` mask. The directory-label code path already exists (`try_set_low_integrity_label` + `is_low_integrity_compatible_dir`) — only the mask is new.
  Enforcement of "both unsupported variants now resolve cleanly" is the acceptance criterion. Full deletion of the `WindowsUnsupportedIssueKind` enum + unsupported-messages plumbing is **deferred** to a follow-up cleanup phase — keeping the shape during v2.1 means future unsupported shapes have a home, and avoids a large diff that would risk regressing the fail-closed invariant.

### Test Strategy

- **D-07:** **Layered tests: cross-platform unit + Windows integration + Phase 18 UAT cookbook re-run.**
  - **(a) Cross-platform unit tests** in `crates/nono/src/sandbox/windows.rs` under `#[cfg(test)]` (compile on all platforms since pure type / policy compilation): assert `compile_filesystem_policy` emits the expected rule shape (not `unsupported`) for single-file read / write / read-write capabilities and for write-only directory capabilities; assert the correct label mask for each mode.
  - **(b) Windows-only integration tests** in `crates/nono/src/sandbox/windows.rs` gated on `#[cfg(target_os = "windows")]`: create a file in `tempdir()`, invoke the label-apply path, read the label back via `GetNamedSecurityInfoW` (existing code in `low_integrity_label_rid`), assert the mandatory-label ACE type + RID + mask field match expectations. Runs without admin because user-created tempfiles are user-writable. Includes a **silent-degradation regression test**: grant single file X, assert the parent directory's label was NOT mutated (it stays at its pre-grant integrity level). Directly tests the roadmap's "grants do not silently degrade to broader directory grants" invariant.
  - **(c) Phase 18 UAT cookbook re-run** as phase close-out: re-run `docs/cli/internals/aipc-uat-cookbook.mdx` Path B + Path C end-to-end, unblock all 4 live-CONIN$ tests in `.planning/phases/18-extended-ipc/18-HUMAN-UAT.md`. If the UAT still reveals issues, they become follow-up quick-tasks; Phase 21 completes when the 4 blocked items transition from `blocked` to `pass` or `issue`.

### Locked Invariants

- **I-01:** **Fail-closed on any error.** Label-apply failure → launch fails. Never silently degrade to a broader grant (e.g. labeling the parent directory instead). Never silently drop the grant. Never retry with reduced access. Encoded in the D-04 error surface.
- **I-02:** **D-21 Windows-invariance held.** Cross-platform files (`crates/nono/src/capability.rs`, `crates/nono/src/sandbox/mod.rs` type definitions outside `#[cfg(target_os = "windows")]`, `crates/nono-cli/src/capability_ext.rs`, etc.) must stay structurally unchanged unless a type surface genuinely needs a new variant. Check: `git diff` against `main` on all non-Windows-gated files should be empty or additive-only. Precedent: Phase 20 Plans 20-01..20-04 verified this pattern across 11 feat/fix commits.
- **I-03:** **Path component comparison, not string comparison.** All label-apply / revert path operations must use `Path::starts_with` and component comparison, never `path.starts_with(string)`. CLAUDE.md § Common Footguns #1.

### Claude's Discretion

- Exact layout of the new `LabelApplyFailed` error variant vs extension of `UnsupportedPlatform` (D-04) — planner + researcher decide based on which keeps the error enum cleanest without destabilizing existing match sites.
- Exact module placement of the revert-on-exit registration (inside `sandbox/windows.rs` vs `exec_strategy_windows/supervisor.rs`) — planner decides based on where the supervisor lifecycle state already lives.
- Plan count / wave parallelization strategy — planner decides. Candidate shape: Plan 21-01 = enforcement primitive + access modes (D-01, D-05), Plan 21-02 = lifecycle + error surface (D-02, D-03, D-04), Plan 21-03 = `WriteOnlyDirectoryGrant` closure (D-06), Plan 21-04 = tests + UAT re-run (D-07). These four plans have partial ordering: 21-01 must land before the others; 21-02 and 21-03 are independent; 21-04 consumes all three.

### Folded Todos

None — no pending todos in `.planning/STATE.md` matched Phase 21's scope.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Windows Sandbox Backend (the implementation site)

- `crates/nono/src/sandbox/windows.rs` — `compile_filesystem_policy` (lines 560–602) emits `SingleFileGrant` / `WriteOnlyDirectoryGrant` today; `try_set_low_integrity_label` (line 1356) is the existing per-path label primitive; `low_integrity_label_rid` (line 449) is the inverse reader. All extension sites.
- `crates/nono/src/sandbox/mod.rs` — `WindowsUnsupportedIssueKind` enum (line 262), `WindowsFilesystemRule` struct (carries `is_file: bool`), `WindowsFilesystemPolicy` struct (rules + unsupported vector). Type surface consumers.
- `crates/nono-cli/src/exec_strategy_windows/mod.rs:227` — `Sandbox::windows_filesystem_policy` + `validate_windows_launch_paths` call site. Launch-time integration point.
- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs` — `create_restricted_token_with_sid`. Establishes the Low IL child-process identity that the file labels gate access for.
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `choose_windows_runtime_root`, `should_use_low_integrity_windows_launch`. Parent-side policy consumers.

### Motivator — `git_config` Policy Group

- `crates/nono-cli/data/policy.json` §`git_config` (lines 501–512) — the 5 single-file read grants that trip `SingleFileGrant` today. Motivating test case for D-07 (c).
- `crates/nono-cli/data/policy.json` §`claude-code` profile (around line 672) — pulls in `git_config`; end-to-end target for the Phase 18 UAT cookbook re-run.

### Unblocks

- `docs/cli/internals/aipc-uat-cookbook.mdx` — Phase 18 AIPC UAT cookbook, Path B + Path C. Re-run end-to-end as phase close-out (D-07 c).
- `.planning/phases/18-extended-ipc/18-HUMAN-UAT.md` — 4 live-CONIN$ tests currently `blocked_on: phase-21-windows-single-file-grants`. Transition to `pass` or `issue` is the Phase 21 completion gate.

### Cross-Platform Context (for parity comparison)

- `crates/nono/src/sandbox/linux.rs` — Landlock ABI v1+ supports file-scope grants natively (per-file `access_fs_mask` on `path_beneath_rule`). No type-level asymmetry — the CapabilitySet interface already carries `is_file`.
- `crates/nono/src/sandbox/macos.rs` — Seatbelt `(allow file-read* (literal "/path"))` supports file-scope grants natively via `literal` vs `subpath`. Same interface.
  Parity target: Windows reaches the same interface level.

### Architectural Invariants (carry forward)

- `CLAUDE.md` §Security Considerations — "Never grant access to entire directories when specific paths suffice." Phase 21 is the enforcement path for this principle on Windows.
- `CLAUDE.md` §Common Footguns — path comparison via components, not strings (I-03).
- `crates/nono/src/sandbox/mod.rs` doc comments — "Libraries should almost never panic." New error variants must return `Result`, not `panic!`.
- `.planning/STATE.md` Key Decisions (v2.0 + v2.1) — "Single SID Generation Point", "Restricted Tokens", existing Low IL model. Changes in Phase 21 must not regress any of these.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`SetNamedSecurityInfoW` wrapper**: `try_set_low_integrity_label` in `crates/nono/src/sandbox/windows.rs:1356` already encapsulates the FFI call for directory paths. Phase 21 extends this (or adds a sibling function) to take the `Mask` field as a parameter so the same call site can emit read-only / write-only / read-write label shapes for both files and directories.
- **`GetNamedSecurityInfoW` reader**: `low_integrity_label_rid` in `crates/nono/src/sandbox/windows.rs:449` already reads back the mandatory-label ACE. Phase 21 extends this (or adds a sibling) to return the full `(rid, mask)` pair so integration tests (D-07 b) can assert the correct mask was applied.
- **Windows path normalization**: `normalize_windows_path` is already applied throughout `compile_filesystem_policy`; Phase 21 inherits the same canonicalization discipline.
- **On-exit cleanup registration**: the supervisor already owns WFP orphan sweep + Job Object teardown on session exit. Label revert (D-02) registers into the same lifecycle hook set.

### Established Patterns

- **`windows-sys 0.59` features**: `Win32_Security_Authorization` + `Win32_Security` are already feature-gated in `crates/nono/Cargo.toml`. No new feature flags needed; the FFI for `SetNamedSecurityInfoW` with `LABEL_SECURITY_INFORMATION` is already imported (see `crates/nono/src/sandbox/windows.rs:20–28`).
- **Low IL model**: child process runs with a restricted token that includes the Low IL mandatory label SID. Any object NOT labeled Low IL is unreachable. Any object labeled Low IL with `NO_READ_UP` mask blocks reads from Low IL subjects, `NO_WRITE_UP` blocks writes, etc. This is the entire enforcement model — no WFP, no DACL, no minifilter.
- **`#[cfg(target_os = "windows")]` gating**: all Windows-specific logic lives under this cfg; cross-platform files stay byte-identical (D-21 invariant from Phase 20).
- **Test gating**: cross-platform unit tests compile on all platforms; Windows integration tests use `#[cfg(target_os = "windows")]` + `#[cfg(test)]`. Matches `apply_rejects_unsupported_single_file_grant` test at `crates/nono/src/sandbox/windows.rs:1406`.

### Integration Points

- **Compile-time**: `compile_filesystem_policy` (line 561 of `windows.rs`) — the two `unsupported.push(...)` sites at lines 568 and 573 become `rules.push(...)` sites instead.
- **Launch-time**: `validate_windows_launch_paths` — the `policy.unsupported.is_empty()` check at line 609 becomes irrelevant if we've made both variants empty; but the covers_path + covers_execution_dir checks remain.
- **Supervisor lifecycle**: a new "applied labels registry" lives on the supervisor state alongside the existing WFP filter registry. Revert loop iterates on exit (D-02).
- **Error plumbing**: `crates/nono/src/error.rs` `NonoError` enum may gain a `LabelApplyFailed` variant (D-04) — or extend `UnsupportedPlatform` with structured context. Planner decides.

### Constraints Carried from Prior Phases

- **Phase 20 D-21 Windows-invariance**: cross-platform files stay byte-identical. This phase's diff should be concentrated in `sandbox/windows.rs` + `exec_strategy_windows/*` + tests.
- **Phase 18 D-09 UnsupportedPlatform message style**: any new error message string for Windows-specific unsupported shapes should follow the established "{platform} does not support {feature}" pattern.
- **Phase 15 D-02 + D-21 invariance**: the `should_allocate_pty` gate and detached-stdio code paths must not regress. Phase 21's changes are in the FS backend, orthogonal to PTY / stdio paths, so the bar is "prove by test non-regression" not "redesign around it".

</code_context>

<specifics>
## Specific Ideas

- **Mask encoding table** (lock for implementation):
  | `AccessMode` | `SYSTEM_MANDATORY_LABEL_ACE.Mask` bits |
  |---|---|
  | `Read`      | `SYSTEM_MANDATORY_LABEL_NO_WRITE_UP \| SYSTEM_MANDATORY_LABEL_NO_EXECUTE_UP` |
  | `Write`     | `SYSTEM_MANDATORY_LABEL_NO_READ_UP \| SYSTEM_MANDATORY_LABEL_NO_EXECUTE_UP` |
  | `ReadWrite` | `SYSTEM_MANDATORY_LABEL_NO_EXECUTE_UP` |

  Rationale: execute is never granted through a filesystem capability in the nono model — executability is controlled by the profile's command allowlist and the Low IL restricted token's execute rights. Always set `NO_EXECUTE_UP`.

- **Silent-degradation regression test shape** (D-07 b detail):
  ```rust
  #[cfg(target_os = "windows")]
  #[test]
  fn single_file_grant_does_not_label_parent_directory() {
      let dir = tempdir().unwrap();
      let file = dir.path().join("only-this.txt");
      std::fs::write(&file, "x").unwrap();
      let parent_label_before = low_integrity_label_rid(dir.path());
      let mut caps = CapabilitySet::new();
      caps.add_fs(FsCapability::new_file(&file, AccessMode::Read).unwrap());
      apply(&caps).unwrap();
      let parent_label_after = low_integrity_label_rid(dir.path());
      assert_eq!(parent_label_before, parent_label_after,
          "single-file grant must not mutate parent directory's label");
      // and the file itself should now be Low IL:
      assert_eq!(low_integrity_label_rid(&file), Some(SECURITY_MANDATORY_LOW_RID as u32));
  }
  ```

- **Label-apply failure hint shape** (D-04 detail): on `ERROR_ACCESS_DENIED`, hint should read "Ensure the target file is writable by the current user and is on NTFS (not ReFS or a network share)." On unknown HRESULT, include the raw hex value for support triage.

- **Phase 18 UAT blocker closes when**: all 4 items in `.planning/phases/18-extended-ipc/18-HUMAN-UAT.md` transition from `result: [blocked — ...]` to a concrete `pass` or `issue` verdict. That re-run is part of Phase 21's close-out, not a follow-up quick-task.

</specifics>

<deferred>
## Deferred Ideas

- **Full deletion of `WindowsUnsupportedIssueKind` enum + unsupported plumbing** — close both variants in Phase 21 (D-06), but keep the type shape as a reserved home for future unsupported Windows shapes. A dedicated cleanup phase once v2.1 ships can retire the enum if no new unsupported shapes appear.
- **Windows runtime capability expansion for filesystem** (`--trust` for individual files at runtime) — this is the cross-platform gap called out in Phase 11 stretch. Out of v2.1 per REQUIREMENTS.md.
- **Refcount/lease semantics for concurrent sessions sharing a labeled file** — accept "last session out restores" (D-02) for v2.1. Revisit if user-reported label-contention issues emerge.
- **Create-empty-and-label for write-mode grants to non-existent files** (D-03 alternative) — rejected on least-surprise grounds for v2.1. Could be reconsidered if a concrete use-case surfaces that directory-scope grants don't cover.
- **Kernel minifilter driver** — deferred to v3.0 per STATE.md Gap 6b.

</deferred>

---

*Phase: 21-windows-single-file-grants*
*Context gathered: 2026-04-20*
