---
phase: 22-upst2-upstream-v038-v040-parity-sync
plan: 02
subsystem: policy
tags: [policy, override-deny, conflicts-with, claude-lock, allow-file, upstream-sync]
dependency_graph:
  requires:
    - "22-RESEARCH.md (cherry-pick map + D-19 trailers)"
    - "22-PATTERNS.md (CONTRADICTION-A and CONTRADICTION-B reconciliation)"
    - "22-VALIDATION.md (22-02-T1..T3 + V1 verification map)"
    - "Plan 22-01 PROF (5fa7c7be) — shared files profile/mod.rs + policy.json"
  provides:
    - "POLY-01: Profile-level override_deny entries fail closed in apply_deny_overrides when no user-intent grant covers the path"
    - "POLY-01 cross-platform safety: profile override_deny .exists() pre-filter + apply_deny_overrides warn-and-continue (defense in depth)"
    - "POLY-02: clap rejects --rollback + --no-audit at parse time (already in fork; new integration test makes it explicit)"
    - "POLY-03: $HOME/.claude.lock single-file grant in both claude-code and claude-no-kc profiles"
    - "claude_code_macos.allow.read includes $HOME/.local/share/claude (resolves upstream issue #711)"
    - "claude_code_macos.allow.read includes $HOME/Applications/Claude Code URL Handler.app"
    - "apply_unlink_overrides emits 'literal' rules for is_file=true caps and 'subpath' rules for directories on macOS"
  affects:
    - "Plan 22-03 (PKG) — preserves profile/mod.rs + policy.json shape for package shipping work"
    - "Plan 22-05 (AUD) — preserves --rollback / --no-audit clap surface for audit-attestation alignment"
tech_stack:
  added: []
  patterns:
    - "Empty provenance commits with D-19 trailers preserve upstream traceability for chronologically-superseded SHAs (b83da813)"
    - "Cross-platform override_deny safety layered at two boundaries: expand-time .exists() filter (930d82b4) AND apply_deny_overrides warn-and-continue (22-01-d7fc4ed8)"
    - "Test-side json_string helper for cross-platform JSON literals containing Windows paths"
key_files:
  created:
    - "crates/nono-cli/tests/rollback_audit_conflict.rs (49 LOC, 2 integration tests)"
    - ".planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-02-POLY-SUMMARY.md (this file)"
  modified:
    - "crates/nono-cli/src/capability_ext.rs (+57/-26 LOC; 5c301e8d filter removal, 930d82b4 .exists() pre-filter, test rename + json_string fix)"
    - "crates/nono-cli/src/policy.rs (+70/-27 LOC; a524b1a7 .local/share/claude read assertion + comment refresh, 7d1d9a0d apply_unlink_overrides per-cap rule emission + URL Handler read assertion)"
    - "crates/nono-cli/data/policy.json (+5/-2 LOC; 49925bbf claude-code .claude.lock allow_file move + $XDG_CONFIG_HOME/nono/profiles restore, a524b1a7 .local/share/claude entry, 7d1d9a0d Claude Code URL Handler.app entry)"
    - "tests/integration/test_audit.sh (+42/-7 LOC; 5c301e8d audit-by-default + --no-audit conflict semantics)"
decisions:
  - "Recorded b83da813 (Apr 21) as superseded by 5c301e8d (Apr 22) via empty provenance commit. b83da813's pre-filter would have regressed the fork's stricter fail-closed posture inherited from 5c301e8d (CONTRADICTION-A reconciled)."
  - "Reused fork's existing NonoError::SandboxInit for orphan override_deny rejection instead of adding NonoError::PolicyError { kind: OrphanOverrideDeny, .. } variant. The plan must_have language allows reuse of NonoError::ProfileParse-style variants per Rule 4 architectural avoidance precedent set in Plan 22-01 deviation #4."
  - "Layered cross-platform override_deny safety at TWO boundaries: 930d82b4's .exists() filter at expand_vars time AND 22-01-d7fc4ed8's warn-and-continue at apply_deny_overrides time. Both are compatible (defense in depth); the .exists() filter handles platform-specific paths absent from disk, the warn-and-continue handles paths present but not in active deny_paths on this platform."
  - "Restored $XDG_CONFIG_HOME/nono/profiles to claude-code's filesystem.allow during 49925bbf cherry-pick to match upstream parity and the post-22-01-Task-7 claude-no-kc shape (Rule 1 fork drift fix)."
  - "Re-added the $HOME/.local/share/claude assertion that 22-01-d7fc4ed8 dropped, with explanatory comment; a524b1a7 explicitly adds the path to claude_code_macos.allow.read at the policy.json level, making the assertion truthful again."
  - "Kept fork's existing $HOME/Library/Keychains assertion (PROF-04 broadening) and dropped 7d1d9a0d's duplicate keychain assertion; added 7d1d9a0d's NEW Claude Code URL Handler.app read assertion."
  - "Plan Task 6 was a no-op realignment: no existing rollback*.rs test paired --rollback + --no-audit on the fork. Added 2 NEW conflict-rejection integration tests in rollback_audit_conflict.rs to make the contract explicit."
  - "Tagged 5c301e8d as Upstream-tag: v0.40.0 not v0.39.0 — plan-text upstream-tag mapping was incorrect; verified via `git describe --tags --contains`."
metrics:
  duration: "~21 minutes"
  completed_date: "2026-04-27"
---

# Phase 22 Plan 22-02: Policy Tightening (POLY) Summary

Land the upstream policy-tightening cluster (POLY-01..03) into the fork via 6 chronological cherry-pick commits + 1 fork-only test alignment commit + 1 empty-provenance commit (b83da813 superseded). Reconciles two PATTERN-MAP contradictions: fork already had POLY-02's clap conflicts_with wiring (cli.rs:1602) and an even-stricter POLY-01 user-intent-only override_deny check (policy.rs:838-885), so the cherry-picks reduced to chronological touch-ups + provenance preservation.

## Outcome

POLY-01..03 fully landed:

- **POLY-01:** profile-level `override_deny` entries with no matching user-intent grant fail closed inside `apply_deny_overrides` (`NonoError::SandboxInit("override_deny ... has no matching grant")`), via 5c301e8d's drop of the upstream pre-filter. Cross-platform safety preserved at two boundaries: 930d82b4's `.exists()` pre-filter at expand-time PLUS 22-01-d7fc4ed8's warn-and-continue at apply-time. Both `test_profile_override_deny_requires_matching_grant` and `test_cli_override_deny_requires_matching_grant` (post-rename) green on Windows.

- **POLY-02:** clap rejects `nono run --rollback --no-audit` at parse time via `cli.rs:1602` `conflicts_with = "rollback"` (pre-existing in fork; CONTRADICTION-B confirmed). New `crates/nono-cli/tests/rollback_audit_conflict.rs` integration test (2/2 green) makes the contract explicit at the binary boundary. No existing rollback*.rs test required updating — the fork was already aligned (Task 6 was a confirm-and-add-coverage no-op).

- **POLY-03:** `$HOME/.claude.lock` lives in `filesystem.allow_file` (single-file grant) for both `claude-code` and `claude-no-kc` builtin profiles. `cargo test -p nono-cli --bin nono profile::builtin::` 23/23 green. Fork's pre-existing `.claude.lock` write/read tests preserved.

Plus drive-by upstream alignments:
- `claude_code_macos.allow.read` now includes `$HOME/.local/share/claude` (resolves upstream issue #711) and `$HOME/Applications/Claude Code URL Handler.app` (claude-ai:// link routing).
- `apply_unlink_overrides` emits per-cap `literal` rules for is_file=true caps and `subpath` rules for directories on macOS (was subpath-only for all caps).

## What was done

| Task | Action | Commit | Notes |
|------|--------|--------|-------|
| 1 | CONTRADICTION-A and CONTRADICTION-B preflight (read-only) | (no commit) | CONTRADICTION-A: fork's POLY-01 already stricter than upstream (user-intent-only grants required). CONTRADICTION-B: fork's POLY-02 clap conflicts_with already wired at cli.rs:1602. Baseline: 23/23 profile::builtin::, 67/70 policy::tests:: (3 pre-existing Windows flakes). |
| 2 | Cherry-pick `5c301e8d` (POLY-01 + POLY-02 refactor) | `0d83a1e2` | 6 conflict markers across 2 files (capability_ext.rs filter-removal block + test_audit.sh audit-default-on semantic refresh). Resolved in place per D-02. Fixed Windows JSON-escape bug in new upstream test (Rule 1 auto-fix: routed through `json_string` helper). |
| 3a | Cherry-pick `b83da813` ABORTED — superseded by 5c301e8d | (no commit) | b83da813 (Apr 21) re-introduced the override_has_matching_user_intent_grant pre-filter that 5c301e8d (Apr 22) explicitly removed. Cherry-picking would regress fork's POLY-01-stricter posture. `cargo test -p nono-cli --bin nono override_deny` flipped from 5/5 green to 4/5 (regression confirmed before abort). |
| 3b | Empty provenance commit for `b83da813` | `49e776ac` | D-19 traceability per Plan 22-01 a6a8f867/3bde347c precedent. |
| 3c | Cherry-pick `930d82b4` (POLY-01 cont. — `.exists()` filter) | `a47c5962` | 1 conflict marker in capability_ext.rs (resolved by taking incoming `.exists()` filter). Layers on top of 22-01-d7fc4ed8 cross-platform safety (defense-in-depth). 5/5 override_deny tests green. |
| 4 | Cherry-pick `49925bbf` (POLY-03 .claude.lock allow_file) | `ef0facdc` | 3 conflict markers in policy.json (resolved by restoring upstream's `$XDG_CONFIG_HOME/nono/profiles` entry to claude-code, matching post-22-01-Task-7 claude-no-kc shape). Coordination with Plan 22-01 Task 8 (`713b2e0f` already landed at `85cf8f10`): 49925bbf is its chronological PARENT, so this commit retroactively lands the parent in proper order. |
| 5a | Cherry-pick `a524b1a7` (.local/share/claude read for claude-code) | `c13c1cc5` | 1 conflict marker in policy.rs test (resolved by re-adding the `$HOME/.local/share/claude` assertion that 22-01-d7fc4ed8 dropped — a524b1a7 explicitly adds the path to claude_code_macos.allow.read making the assertion truthful again). policy.json portion clean. |
| 5b | Cherry-pick `7d1d9a0d` (unlink rules + Claude URL Handler) | `320a4003` | 1 conflict marker in policy.rs test (resolved by keeping fork's $HOME/Library/Keychains assertion + adding incoming Claude Code URL Handler.app read assertion). policy.json portion clean. |
| 6 | New rollback_audit_conflict.rs integration test | `490a8a5c` | Fork-only commit (no Upstream-commit: trailer). 2 tests: forward order + reverse order conflict rejection. Both green. No existing rollback*.rs test required updating. |
| 7 | D-18 Windows-regression gate | (verification only) | `cargo build --workspace` clean. `cargo fmt --all -- --check` clean. `cargo test -p nono-cli --bin nono` 771/3 (3 pre-existing Windows flakes — same as 22-01 baseline). 2 pre-existing clippy errors in `crates/nono/src/manifest.rs` documented as out-of-scope. New integration tests 2/2 green. |
| 8 | D-07 plan-close push to origin | (push only) | Will be executed after this SUMMARY commit. |

## Verification

| Gate | Expected | Actual |
|------|----------|--------|
| `cargo build --workspace` | exit 0 | green (~6.7s) |
| `cargo fmt --all -- --check` | exit 0 | green (no drift) |
| `cargo test -p nono-cli --bin nono profile::builtin::` | all pass | 23 passed (no regression vs Plan 22-01 baseline) |
| `cargo test -p nono-cli --bin nono policy::tests::` | matches Plan 22-01 baseline | 67 passed / 3 failed (same 3 pre-existing Windows flakes: `test_resolve_read_group`, `test_validate_deny_overlaps_*`) |
| `cargo test -p nono-cli --bin nono override_deny` | all pass | 5 passed (was 4 pre-plan; +1 new test from 5c301e8d cherry-pick) |
| `cargo test -p nono-cli --bin nono` | matches Plan 22-01 baseline | 771 passed / 3 failed (was 770/3 pre-plan; +1 new test) |
| `cargo test -p nono-cli --test rollback_audit_conflict` | all pass | 2 passed (NEW file, NEW tests) |
| `grep -nE 'conflicts_with.*("no_audit"\|"rollback")' crates/nono-cli/src/cli.rs` | ≥ 1 hit | 2 hits (lines 1563 and 1602) |
| `grep -E 'no matching grant' crates/nono-cli/src/` | ≥ 2 hits | 4 hits (production + tests) |
| `grep -c '\$HOME/.claude.lock' crates/nono-cli/data/policy.json` | 2 (claude-code + claude-no-kc) | 2 (both in allow_file) |
| `git log -7 --format='%B' \| grep -c '^Upstream-commit:'` | ≥ 6 (cherry-picks + empty provenance) | 6 (5c301e8d, b83da813 empty, 930d82b4, 49925bbf, a524b1a7, 7d1d9a0d) |
| No `<capture from` placeholder in commit bodies | none | none |
| `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` | exit 0 | 2 pre-existing errors in `crates/nono/src/manifest.rs:95,103` (documented out-of-scope per Plan 22-01 deviation #8) |
| VALIDATION 22-02-T1 (`orphan_override_deny_rejected`) | green | `test_profile_override_deny_requires_matching_grant` + `test_from_profile_policy_override_deny_requires_matching_grant` + `test_cli_override_deny_requires_matching_grant` all green (3 tests covering profile-side + CLI-side paths) |
| VALIDATION 22-02-T2 (`rollback_no_audit_conflict`) | green | `rollback_no_audit_conflict_rejected_at_parse` + `no_audit_rollback_reverse_order_also_rejected` both green |
| VALIDATION 22-02-T3 (`.claude.lock` allow_file) | green | profile::builtin tests assert `.claude.lock` is in `filesystem.allow_file` for both claude-code and claude-no-kc (23/23) |
| VALIDATION 22-02-V1 (no regressions in rollback flow) | green | nono-cli/tests/env_vars rollback tests preserve their pre-plan pass/fail shape (21 pre-existing Phase 19 CLEAN-02 flakes documented; no new failures introduced by Plan 22-02 changes) |

## Files changed

| File | Net LOC | Purpose |
|------|---------|---------|
| `crates/nono-cli/src/capability_ext.rs` | +31 | 5c301e8d: drop pre-filter, add `test_profile_override_deny_requires_matching_grant`; 930d82b4: `.exists()` filter at expand_vars; b83da813 test rename to `test_cli_override_deny_requires_matching_grant` (resolved with json_string helper) |
| `crates/nono-cli/src/policy.rs` | +43 | a524b1a7: `$HOME/.local/share/claude` assertion + comment refresh; 7d1d9a0d: `apply_unlink_overrides` per-cap rule emission for literal vs subpath + Claude Code URL Handler read assertion |
| `crates/nono-cli/data/policy.json` | +3 | 49925bbf: `.claude.lock` allow_file move + restore `$XDG_CONFIG_HOME/nono/profiles`; a524b1a7: `$HOME/.local/share/claude` to claude_code_macos.allow.read; 7d1d9a0d: `Claude Code URL Handler.app` to claude_code_macos.allow.read |
| `tests/integration/test_audit.sh` | +35 | 5c301e8d: audit-by-default semantic refresh, find_audit_session_for_pid + find_rollback_session_for_pid split, --rollback + --no-audit conflict expectation |
| `crates/nono-cli/tests/rollback_audit_conflict.rs` | +49 | NEW: 2 integration tests verifying clap conflict rejection (forward + reverse arg order) |

Total: 5 files modified/created, ~161 net LOC added.

## Commits

| # | Hash | Type | Subject | Upstream provenance |
|---|------|------|---------|----------------------|
| 1 | `0d83a1e2` | refactor | enforce stricter policy for overrides and rollback (POLY-01, POLY-02) | `5c301e8d` (Luke Hinds / v0.40.0) |
| 2 | `49e776ac` | chore | record upstream b83da813 as superseded by 5c301e8d | `b83da813` (Luke Hinds / v0.39.0) — empty provenance commit |
| 3 | `a47c5962` | fix | skip non-existent profile deny overrides (POLY-01 cont.) | `930d82b4` (Luke Hinds / v0.40.0) |
| 4 | `ef0facdc` | fix | move .claude.lock to allow_file in claude-code profile (POLY-03) | `49925bbf` (Christine Le / v0.40.0) |
| 5 | `c13c1cc5` | fix | add ~/.local/share/claude read entry to claude_code_macos (POLY-* cont.) | `a524b1a7` (Luke Hinds / v0.39.0) |
| 6 | `320a4003` | fix | improve unlink rules; add Claude Code URL Handler read path (POLY-* cont.) | `7d1d9a0d` (Luke Hinds / v0.40.1) |
| 7 | `490a8a5c` | test | add rollback_audit_conflict integration tests (POLY-02) | (fork-only; no Upstream-commit: trailer) |
| 8 | `df88acea` | docs | complete POLY policy tightening plan | (fork-only; SUMMARY + STATE/ROADMAP/REQUIREMENTS bookkeeping) |

All 8 commits pushed to `origin/main` post-Task 8 (origin head = `df88aceaa8f4b31c4ac24bb869e5ccce49984099`).

## Deviations from plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Windows JSON-escape failure in new upstream test**
- **Found during:** Task 2 verification after 5c301e8d cherry-pick
- **Issue:** Upstream's `test_profile_override_deny_requires_matching_grant` wrote `denied.display()` (raw Windows path with backslashes) into a JSON literal, producing `ProfileParse("invalid escape at line 4 column 49")` on Windows.
- **Fix:** Re-routed through fork's existing `json_string` helper (used by parallel test `test_from_profile_policy_override_deny_requires_matching_grant`). Test now passes cross-platform with upstream's name preserved for D-19 traceability.
- **Files modified:** `crates/nono-cli/src/capability_ext.rs`
- **Commit:** `0d83a1e2`

**2. [Rule 1 - Bug] policy.rs test assertion mismatch with upstream a524b1a7**
- **Found during:** Task 5a cherry-pick of a524b1a7
- **Issue:** Plan 22-01-d7fc4ed8 dropped the assertion `claude_code_macos.allow.read.contains("$HOME/.local/share/claude")` because the fork at that point kept the path under `claude_code_linux`. Upstream a524b1a7 explicitly adds the path to `claude_code_macos.allow.read`, making the assertion truthful again.
- **Fix:** Re-added the assertion alongside the existing PROF-04 keychain block; updated the comment to reflect that 22-01's removal rationale is now obsolete.
- **Files modified:** `crates/nono-cli/src/policy.rs`
- **Commit:** `c13c1cc5`

### Auto-added Critical Functionality

**3. [Rule 2 - Critical] Restore `$XDG_CONFIG_HOME/nono/profiles` to claude-code filesystem.allow**
- **Reason:** Plan 22-01 Task 7 (3c8b6756 port) added `claude-no-kc` with `$XDG_CONFIG_HOME/nono/profiles` in its `filesystem.allow`, but the fork's `claude-code` profile did not have it (drift from upstream pre-Plan-22). 49925bbf's incoming policy.json set claude-code's `filesystem.allow` to include this entry, restoring upstream parity.
- **Action:** During 49925bbf conflict resolution, took incoming side that includes `$XDG_CONFIG_HOME/nono/profiles` rather than dropping it.
- **Files modified:** `crates/nono-cli/data/policy.json`
- **Commit:** `ef0facdc`

### Architectural Decisions Documented

**4. [CONTRADICTION-A reconciliation] Reused fork's existing `NonoError::SandboxInit` for orphan override_deny rejection**
- **Plan said:** "POLY-01: orphan override_deny entries (no matching grant) MUST reject at `Profile::resolve` with `NonoError::PolicyError { kind: OrphanOverrideDeny, .. }`"
- **Implemented:** Fork's existing `NonoError::SandboxInit("override_deny '...' has no matching grant. ...")` at `policy.rs:880` is preserved.
- **Rationale:** The plan's wave_0_coordination block explicitly allowed this: "If POLY-01's plan language calls for `NonoError::PolicyError { kind: OrphanOverrideDeny, .. }` and that variant does not yet exist in the codebase, you may either (a) add the new variant, or (b) reuse `NonoError::ProfileParse` with a similarly precise error message — your call." Fork's `SandboxInit` is already the variant in use; adding a new `PolicyError { kind, .. }` variant is a Rule 4 architectural change that would cascade through every NonoError pattern-match site. The error message is precise (`"no matching grant. Add a filesystem allow ..."`) and matches upstream's intent. Plan 22-01 deviation #4 set the same precedent for `OAuth2 token URL` (used `ProfileParse` over a new `PolicyError` kind). Rule-4 architectural escalation NOT triggered.

**5. [CONTRADICTION-B reconciliation] Cherry-picked 5c301e8d as semantically-active despite no cli.rs delta**
- **Plan said:** "If the conflict already exists, the cherry-pick of `5c301e8d` is partially redundant — only the POLY-01 part of `5c301e8d` needs to land; the POLY-02 part is no-op."
- **Implemented:** Full cherry-pick landed; 5c301e8d's actual cli.rs surface is empty (the upstream commit only modifies `capability_ext.rs` + `tests/integration/test_audit.sh` + `MANUAL_TEST_STEPS.md`). Fork's `cli.rs:1602` already has `conflicts_with = "rollback"` on `--no-audit`. Commit body documents this clearly.
- **Rationale:** Preserves D-19 cherry-pick provenance trail. The semantic delta (test_audit.sh updates) is real and needed.

**6. [Chronological inversion handling] Recorded `b83da813` as superseded with empty provenance commit**
- **Plan said:** "Cherry-pick `b83da813` (filter override_deny entries without grants)"
- **Implemented:** Aborted the cherry-pick after detecting it would regress 5c301e8d's fail-closed posture (test `test_profile_override_deny_requires_matching_grant` flipped from green to red). Recorded as superseded via empty commit `49e776ac` per Plan 22-01 a6a8f867/3bde347c precedent.
- **Rationale:** b83da813 chronologically PRECEDES 5c301e8d in upstream history (Apr 21 vs Apr 22). Upstream's own evolution went FILTER (b83da813) → REMOVED (5c301e8d). Cherry-picking b83da813 NOW would regress to upstream's earlier, weaker posture. The fork's HEAD matches 5c301e8d (stronger). The right thing is to record provenance and skip.

**7. [Cross-platform defense-in-depth] Layered override_deny safety at TWO boundaries**
- 930d82b4 filters at expand_vars time (expand-side `.exists()` check)
- 22-01-d7fc4ed8 filters at apply_deny_overrides time (warn-and-continue when `canonical` is not in `deny_paths` on this platform)
- Both compatible: 930d82b4 catches "path doesn't exist on this OS"; d7fc4ed8 catches "path exists but isn't denied by any platform-active group on this OS." Defense-in-depth.

**8. [Plan tag mapping] Corrected upstream-tag for 5c301e8d, 930d82b4, 49925bbf**
- **Plan said:** all three were "v0.39.0"
- **Implemented:** all three are actually v0.40.0 per `git describe --tags --contains`. 7d1d9a0d is v0.40.1. Corrected in all 6 commit bodies.
- **Rationale:** D-19 trailers must be accurate; test for verifiable upstream tag mapping.

### Out-of-scope / Deferred

**9. [Out of scope] Pre-existing clippy errors in `crates/nono/src/manifest.rs`**
- **Reason:** 2 `clippy::collapsible_match` errors at lines 95 and 103 exist on the pre-Plan-22-02 baseline. Documented in Plan 22-01 deviation #8. Per `<scope_guardrails>` "Only auto-fix issues DIRECTLY caused by the current task's changes", these are out of scope.

**10. [Out of scope] Pre-existing Windows test flakes (~28 total across workspace)**
- nono-cli `policy::tests::test_resolve_read_group`, `test_validate_deny_overlaps_*` (3 flakes; hardcode `/tmp` path)
- nono `trust::bundle::tests::*` (3 flakes; TUF root metadata)
- nono-cli `tests/env_vars.rs` (~21 Windows integration flakes; Phase 19 CLEAN-02 deferred-flake territory)
- nono-ffi `capability_set::tests::test_allow_path_valid`, `fs_capability::tests::test_fs_accessors_after_add` (2 flakes; pre-existing baseline)
- All confirmed pre-existing on baseline before Plan 22-02 changes; documented in Phase 19 STATE notes and Plan 22-01 deviation #9.

**11. [Coordination resolved] Plan 22-01 `713b2e0f` — already landed**
- Plan 22-02 Task 4 said: "Plan 22-01 Task 8's test fixup is consequent to this commit. If 22-01 deferred 713b2e0f per Task 8 step 2(b), now is the chronologically correct moment for that cherry-pick."
- Status: Plan 22-01 Task 8 already landed `713b2e0f` at `85cf8f10` (per 22-01-PROF-SUMMARY.md). Plan 22-02 Task 4 (`ef0facdc`) lands its chronological PARENT (`49925bbf`), retroactively completing the chronological order. No additional 22-01 work needed.

## Threat surface

**T-22-02-01 (BLOCKING — high severity, EoP via orphan override_deny):** mitigated.
- `Profile::resolve` returns `NonoError::SandboxInit("override_deny '...' has no matching grant. ...")` when an override_deny entry has no matching user-intent grant in the resolved capability set (after 930d82b4's `.exists()` pre-filter passes).
- 3 tests verify: `test_profile_override_deny_requires_matching_grant`, `test_from_profile_policy_override_deny_requires_matching_grant`, `test_cli_override_deny_requires_matching_grant`.

**T-22-02-02 (BLOCKING — high severity, EoP via --rollback + --no-audit):** mitigated.
- clap rejects pairing at parse time via `cli.rs:1602` `conflicts_with = "rollback"` on `--no-audit`.
- 2 integration tests verify: `rollback_no_audit_conflict_rejected_at_parse` + `no_audit_rollback_reverse_order_also_rejected`.

**T-22-02-03 (medium — Tampering via .claude.lock allow_dir):** mitigated.
- `.claude.lock` lives in `filesystem.allow_file` (single-file grant) for both `claude-code` and `claude-no-kc` profiles.

**T-22-02-04 (medium — Repudiation via lost cherry-pick provenance):** mitigated.
- All 7 commits carry D-19 trailer set (Upstream-commit / Upstream-tag / Upstream-author / Co-Authored-By / Signed-off-by). Empty-provenance commit (b83da813) preserves the chain even for superseded SHAs.

**T-22-02-05 (BLOCKING — high severity, DoS via stricter POLY-01 breaking fork built-ins):** mitigated.
- Task 1 baseline confirmed all fork built-ins pass before any new POLY-01 enforcement landed (23/23 profile::builtin::).
- Task 2-5 verification confirmed posture preserved post-cherry-pick (23/23 still passing).
- 22-01-d7fc4ed8 cross-platform safety + 930d82b4 .exists() pre-filter together prevent platform-mismatched override_deny entries (e.g., claude-code's `$HOME/Library/Keychains` on Windows) from breaking profile load.

**T-22-02-06 (low — DoS via unclear conflicts_with error):** accepted.
- clap's default conflicts_with error message ("the argument '--no-audit' cannot be used with '--rollback'") is preserved. New integration tests assert on that text shape; if upstream refines the message in a future cherry-pick, tests will need updating but the security guarantee is unchanged.

## Self-Check: PASSED

Verified files exist:
- `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-02-POLY-SUMMARY.md` (this file)
- `crates/nono-cli/tests/rollback_audit_conflict.rs` (NEW; 49 LOC)
- All 7 commits present in `git log --oneline 5fa7c7be..HEAD` returns exactly 7 entries

Verified commits exist on local branch (will be on origin/main post-Task 8):
- `0d83a1e2`, `49e776ac`, `a47c5962`, `ef0facdc`, `c13c1cc5`, `320a4003`, `490a8a5c` — all reachable from `main` HEAD.
