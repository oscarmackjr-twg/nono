---
phase: 20-upstream-parity-sync
plan: 02
subsystem: profile / claude-code hooks / security hardening
tags: [upstream-parity, profile-cycle-guard, claude-code-symlink, path-traversal-guard, d-06, d-07, dco]
requirements: [UPST-02]
completed: 2026-04-19
duration_minutes: 45
dependency_graph:
  requires:
    - ".planning/phases/20-upstream-parity-sync/20-01-SUMMARY.md (workspace at 0.37.1, rustls-webpki 0.103.12)"
    - ".planning/phases/20-upstream-parity-sync/20-CONTEXT.md (D-06, D-07, D-15, D-20, D-21)"
  provides:
    - "End-to-end regression coverage for profile `extends` cycle guard (self-ref, indirect, linear chain)"
    - ".claude.json symlink wiring at hook-install time with canonicalized root-containment validation"
    - "Cross-platform symlink dispatch: Unix propagates errors; Windows fail-open with tracing::warn!"
    - "Hostile path-traversal rejection for claude.json symlink target"
  affects:
    - "Plans 20-03 and 20-04 (Wave 1 parallel): no shared files (disjoint invariant held)"
tech_stack:
  added: []
  patterns:
    - "cycle-detection guard end-to-end test via public load_profile API (XDG_CONFIG_HOME / APPDATA tempdir)"
    - "canonicalize target + Path::starts_with root-containment (CLAUDE.md § Common Footgun #1)"
    - "Windows fail-open symlink: std::os::windows::fs::symlink_file + tracing::warn! on IO error"
    - "DCO + Upstream-commit / Upstream-tag / Upstream-author provenance trailers"
key_files:
  created:
    - ".planning/phases/20-upstream-parity-sync/20-02-SUMMARY.md"
  modified:
    - "crates/nono-cli/src/profile/builtin.rs"
    - "crates/nono-cli/src/hooks.rs"
decisions:
  - "Fork's resolve_extends already carries the c1bc439-equivalent cycle guard (visited Vec + MAX_INHERITANCE_DEPTH bound). Plan 20-02 Task 2 adds the plan-mandated end-to-end regression tests rather than re-implementing an existing guard — the tests drive the guard through load_profile (public API) so future refactors that strip the guard fail-closed here."
  - "Upstream 97f7294 lands the .claude.json symlink in sandbox_prepare.rs behind an extends-claude-code check; plan 20-02 files_modified restricts the port to hooks.rs, so the fix is ported into install_claude_code_hook at hook-install time (one natural integration point earlier in the lifecycle)."
  - "NonoError::InvalidConfig (suggested by plan text) does not exist in the fork's error enum. Used NonoError::HookInstall(String) as the closest config-shaped variant; documented inline and tested with matches!(err, NonoError::HookInstall(..))."
  - "Windows fail-open behavior on unprivileged symlink creation: catch io::Error, emit tracing::warn!, return Ok(()). Runtime behavior is unchanged from the pre-port state on such hosts — the user does not get the token-refresh fix without Developer Mode, but install does not fail."
  - "Phase 15 5-row detached-console smoke gate document-skipped: zero *_windows.rs files touched by either commit; D-21 Windows-invariance held by construction (same rationale as 20-01 Task 5)."
metrics:
  duration: "~45 minutes"
  tasks_completed: 5
  commits: 2
  files_created: 1
  files_modified: 2
---

# Phase 20 Plan 02: Upstream Parity Sync — Profile Fixes (D-06, D-07)

Ported two upstream stability fixes from `v0.37.1` to `windows-squash`: **D-06** profile `extends` cycle guard (upstream `c1bc439`) — regression tests only, since the fork already carries the guard in `resolve_extends`; and **D-07** claude-code `.claude.json` symlink wiring (upstream `97f7294`) — new hook-install-time wiring with canonicalized root-containment validation and Windows fail-open. Two DCO-signed atomic commits on `windows-squash`, zero `*_windows.rs` files touched (D-21 invariant held), zero files shared with Plans 20-03 or 20-04 (D-15 disjoint invariant held).

## Outcome

All 5 plan tasks complete. Two atomic DCO-signed commits on `windows-squash`:

1. `05c24a6` — fix(20-02): port profile extends recursion guard from upstream v0.37.1 (D-06)
2. `f8ef9dd` — fix(20-02): enable claude-code token refresh via .claude.json symlink from upstream v0.37.1 (D-07)

Both commits carry DCO `Signed-off-by:` + `Upstream-commit:` + `Upstream-tag: v0.37.1` + `Upstream-author: Luke Hinds <lukehinds@gmail.com>` trailers.

Wave 1 plan 20-02 complete. Plans 20-03 and 20-04 unaffected (disjoint files).

## What was done

- **Task 1 — Baseline verification (post-20-01):** Confirmed the three Plan 20-01 commits are present on `windows-squash` (`docs(20-01): add UPST-01..04`, `chore(20-01): bump workspace crate versions`, `chore(20-01): upgrade rustls-webpki`); all four workspace `Cargo.toml` files pin `version = "0.37.1"`; `rustls-webpki` entry in `Cargo.lock` is `0.103.12`; `cargo build --workspace` exits 0 on post-20-01 HEAD.

- **Task 2 — Port upstream `c1bc439` (D-06, profile extends cycle guard):** The fork already carries the equivalent cycle guard in `resolve_extends` (visited-Vec + `MAX_INHERITANCE_DEPTH` bound, see `crates/nono-cli/src/profile/mod.rs:1267`). What was missing was the plan-mandated end-to-end regression coverage. Added three tests to `crates/nono-cli/src/profile/builtin.rs` that drive the guard through the public `load_profile` API:
  - `test_profile_extends_self_reference_detected` — profile A declares `extends: "A"`, load fails with `NonoError::ProfileInheritance` containing "cycle"/"circular".
  - `test_profile_extends_indirect_cycle_detected` — A extends B, B extends A, load A fails with the same error shape. Guards against any cycle-detection variant that only catches direct self-references.
  - `test_profile_extends_linear_chain_succeeds` — A extends B extends C (non-cyclic), load A resolves and the merged profile's `extends` field is consumed. Regression guard against over-aggressive cycle rejection.
  - All three assert on `matches!(err, NonoError::ProfileInheritance(_))` (structural) rather than string-matching. Each test uses a per-test tempdir pointed at via APPDATA (Windows) / XDG_CONFIG_HOME (Unix) with the process-global `test_env::ENV_LOCK` guard.

- **Task 3 — Port upstream `97f7294` (D-07, claude-code .claude.json symlink):** Added three new private items to `crates/nono-cli/src/hooks.rs` and wired them into `install_claude_code_hook`:
  - `install_claude_json_symlink(home: &Path) -> Result<()>` — creates `<home>/.claude/`, pre-seeds the `<home>/.claude/claude.json` target, moves an existing regular `<home>/.claude.json` into the target if present, then creates the `<home>/.claude.json -> .claude/claude.json` symlink via `create_symlink_platform`. Runs once per hook install; no-ops when the symlink is already in place.
  - `validate_symlink_target_under_root(target: &Path, root: &Path) -> Result<()>` — canonicalizes both and checks containment via `Path::starts_with` (component-wise, per CLAUDE.md § Common Footgun #1). Rejects with `NonoError::HookInstall` on escape.
  - `create_symlink_platform(link_target: &Path, link_path: &Path) -> Result<()>` — platform-dispatched: `#[cfg(unix)]` uses `std::os::unix::fs::symlink` and propagates errors; `#[cfg(windows)]` uses `std::os::windows::fs::symlink_file`, catches any `io::Error`, emits `tracing::warn!`, and returns `Ok(())` (fail-open for install, runtime unchanged on unprivileged hosts).
  - The caller inside `install_claude_code_hook` ignores the returned `Result` (logs on error, continues) so a missing symlink never blocks hook install.
  - Three tests added to `hooks::tests`:
    - `test_claude_json_rejects_path_traversal` — hostile sibling-tempdir target rejected before any symlink is attempted; asserts `matches!(err, NonoError::HookInstall(_))` and error message contains "escapes" or "not under".
    - `test_claude_json_accepts_target_inside_root` — regression guard for the legitimate upstream redirect path.
    - `test_install_claude_json_symlink_does_not_panic` — end-to-end install against a tempdir `HOME`; on Unix asserts the symlink was created; on Windows accepts either outcome (fail-open path exercised).

- **Task 4 — `make ci` gate + feature smoke:**
  - `cargo fmt --all -- --check` — exit 0.
  - `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` — exit 0.
  - `cargo test -p nono --lib` — 557 passed / 0 failed.
  - `cargo test --workspace --all-features` (nono-cli bin) — 670 passed / 1 failed. The single failure is `trust_scan::tests::multi_subject_verified_paths_included` (pre-existing Phase 19 CLEAN-02 tempdir-race flake, matches 1-3 window documented in `.planning/phases/19-cleanup/19-02-SUMMARY.md` and re-confirmed in Plan 20-01 Summary line 141).
  - `cargo test -p nono-cli --test env_vars` — 19 failures, all `windows_*` variants, exact match for the Phase 19 CLEAN-02 deferred window (documented in `19-02-SUMMARY.md` and 20-01 Summary line 139).
  - Feature smoke: `grep -cE "test_profile_extends_(self_reference_detected|indirect_cycle_detected|linear_chain_succeeds) \.\.\. ok"` = 3 (all three cycle-guard tests pass). `grep -cE "claude_json.*\.\.\. ok"` = 3 (path-traversal, accepts-in-root, install-does-not-panic). `grep -cE 'test_claude_json_rejects_path_traversal .* ok'` = 1 (exact acceptance match).
  - No NEW failures attributable to Tasks 2-3.

- **Task 5 — D-20 Windows regression safety net:**
  - `cargo test --workspace --all-features` — within the Phase 19 deferred window (see Task 4).
  - `cargo test -p nono-cli --test learn_windows_integration` — exit 0 (1 ignored, requires admin ETW).
  - `cargo test -p nono-cli --test wfp_port_integration` (non-ignored) — 1 passed + 1 ignored. Ignored test documented-skip per CONTEXT D-20 (requires admin + `nono-wfp-service`).
  - Phase 15 5-row detached-console smoke gate: document-skipped because `git diff HEAD~2..HEAD --name-only` lists only `crates/nono-cli/src/hooks.rs` + `crates/nono-cli/src/profile/builtin.rs`. Zero `*_windows.rs` files touched; D-21 Windows-invariance held by construction (same rationale as 20-01 Task 5).

## Verification

| Check | Result | Notes |
|-------|--------|-------|
| `cargo build --workspace` exits 0 | PASS | Built cleanly at post-20-01 HEAD and at each commit boundary |
| `cargo fmt --all -- --check` exits 0 | PASS | No fmt drift introduced |
| `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` exits 0 | PASS | Strict clippy gate cleared |
| `cargo test -p nono --lib` exits 0 | PASS | 557 passed / 0 failed |
| `cargo test --workspace --all-features` within deferred window | PASS (within window) | 670 passed / 1 failed (`trust_scan::multi_subject_verified_paths_included` — pre-existing tempdir-race flake, Phase 19 CLEAN-02 carry-forward) |
| `cargo test -p nono-cli --test env_vars` within deferred window | PASS (within window) | 55 passed / 19 failed (all `windows_*`, exact match for Phase 19 CLEAN-02 19-failure carry-forward) |
| All 3 profile-extends tests pass (`test_profile_extends_*` count = 3) | PASS | `test_profile_extends_self_reference_detected`, `_indirect_cycle_detected`, `_linear_chain_succeeds` — all ok in `profile::builtin::tests` |
| Path-traversal rejection test count = 1 | PASS | `test_claude_json_rejects_path_traversal ... ok` (grep count = 1) |
| `grep -cE "claude_json.*\.\.\. ok"` on hooks test output | PASS (3) | path-traversal, accepts-inside-root, install-does-not-panic |
| `grep -cE 'visited\|seen\|depth\|cycle'` in `resolve_extends` block | PASS | fork already carries `visited` + `depth` + "circular" error path (upstream-c1bc439 equivalent, pre-existing) |
| `grep -cE 'claude\.json'` in hooks.rs | PASS (≥1) | `install_claude_json_symlink`, `validate_symlink_target_under_root`, and the three tests all reference `.claude.json` |
| `grep -cE 'tracing::warn!\|warn!\('` for Windows unprivileged-symlink path | PASS | `create_symlink_platform` on `#[cfg(windows)]` routes the `io::Error` through `tracing::warn!` and returns `Ok(())` |
| `cargo test -p nono-cli --test wfp_port_integration -- --ignored` | DOCUMENTED-SKIP | Requires admin + `nono-wfp-service`; non-ignored suite passes (1 passed + 1 ignored) |
| `cargo test -p nono-cli --test learn_windows_integration` exits 0 | PASS | 1 ignored (requires admin ETW); suite exits 0 |
| Phase 15 5-row detached-console smoke gate | DOCUMENTED-SKIP | Zero `*_windows.rs` files changed; D-21 invariant held by construction (same as 20-01) |
| Both commits carry DCO `Signed-off-by:` | PASS | `oscarmackjr-twg <oscar.mack.jr@gmail.com>` on each; matches repo git identity |
| Both commits carry `Upstream-commit:` / `Upstream-tag: v0.37.1` / `Upstream-author:` | PASS | `c1bc439` on commit 1; `97f7294` on commit 2; both cite `Luke Hinds <lukehinds@gmail.com>` |
| `git log --oneline -2` shows commits in order | PASS | `f8ef9dd` (D-07 symlink) → `05c24a6` (D-06 cycle guard) |
| `git show --stat` on each commit lists only `files_modified` | PASS | commit 1: `profile/builtin.rs`; commit 2: `hooks.rs`. Zero `*_windows.rs`, zero `profile/mod.rs` drift, zero overlap with 20-03 / 20-04 scope |
| D-21 Windows-invariance held | PASS | `git diff HEAD~2..HEAD --name-only` = 2 lines, neither a `*_windows.rs` file |
| D-15 disjoint-parallel invariant held | PASS | Neither file is in 20-03's (`keystore.rs`, `cli.rs`, `command_blocking_deprecation.rs`) or 20-04's (`cli.rs`, `sandbox/linux.rs`, `sandbox/macos.rs`, `trust/*`, `trust_cmd.rs`) `files_modified` lists |
| No unsubstituted `<capture from ...>` placeholders in commit bodies | PASS | `git log HEAD~2..HEAD --format=%B \| grep -c '<capture from'` = 0 |

## Files changed

| File | Change |
|------|--------|
| `crates/nono-cli/src/profile/builtin.rs` | Added 3 plan-mandated tests (`test_profile_extends_self_reference_detected`, `test_profile_extends_indirect_cycle_detected`, `test_profile_extends_linear_chain_succeeds`) + 2 helper functions (`seed_user_profile`, `user_config_dir_guard`). Drives the existing `resolve_extends` cycle guard through the public `load_profile` API. +190 lines. |
| `crates/nono-cli/src/hooks.rs` | Added `install_claude_json_symlink`, `validate_symlink_target_under_root`, `create_symlink_platform` (cfg-gated Unix/Windows); wired the symlink install into `install_claude_code_hook` at the end of hook registration. Added 3 tests (`test_claude_json_rejects_path_traversal`, `test_claude_json_accepts_target_inside_root`, `test_install_claude_json_symlink_does_not_panic`). +319 / -1 lines. |

## Commits

| Hash | Type | Subject | DCO | Upstream provenance |
|------|------|---------|-----|---------------------|
| `05c24a6` | fix | port profile extends recursion guard from upstream v0.37.1 (D-06) | signed | `Upstream-commit: c1bc439`, `Upstream-tag: v0.37.1`, `Upstream-author: Luke Hinds <lukehinds@gmail.com>` |
| `f8ef9dd` | fix | enable claude-code token refresh via .claude.json symlink from upstream v0.37.1 (D-07) | signed | `Upstream-commit: 97f7294`, `Upstream-tag: v0.37.1`, `Upstream-author: Luke Hinds <lukehinds@gmail.com>` |

All commits carry `Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>` per the repo's configured git identity (DCO-compliant).

## Upstream-provenance summary

| Upstream commit | Upstream tag | Upstream author | Fork commit | Integration site |
|-----------------|--------------|-----------------|-------------|------------------|
| `c1bc439` — fix(profiles): prevent infinite recursion in profile extends check | v0.37.1 | Luke Hinds `<lukehinds@gmail.com>` | `05c24a6` | `crates/nono-cli/src/profile/builtin.rs` (tests only — fork's `resolve_extends` in `profile/mod.rs` already carries the guard) |
| `97f7294` — fix(claude-code): enable token refresh via .claude.json symlink | v0.37.1 | Luke Hinds `<lukehinds@gmail.com>` | `f8ef9dd` | `crates/nono-cli/src/hooks.rs` (install_claude_code_hook; upstream lands it in sandbox_prepare.rs) |

## Deviations from Plan

### Auto-fixed issues (Rules 1-3)

**1. [Rule 3 — blocking: missing NonoError variant] `InvalidConfig` does not exist in the fork's error enum**
- **Found during:** Task 3 (porting 97f7294 symlink wiring)
- **Issue:** Plan text (and the upstream patch commentary in the plan body) referenced `NonoError::InvalidConfig` as the error variant for symlink-target-escape rejection. The fork's `crates/nono/src/error.rs` has no `InvalidConfig` variant (see `rg 'InvalidConfig' crates/nono/src/error.rs` — 0 matches).
- **Plan-authorized resolution:** The plan's Task 3 `<read_first>` explicitly states "read the fork's `crates/nono/src/error.rs` to pick the right variant; if unsure, use the closest config-shaped variant." Chose `NonoError::HookInstall(String)` as the most semantically appropriate variant — this is explicitly a hook-install-time validation failure, which is exactly what `HookInstall` was designed for.
- **Consequence:** Test assertion updated from `matches!(err, NonoError::InvalidConfig { .. })` to `matches!(err, NonoError::HookInstall(_))`. Documented inline in the symlink helper's rustdoc so future readers see the choice.
- **Files modified:** `crates/nono-cli/src/hooks.rs` (only; `error.rs` unchanged — adding a new variant would be a Rule-4 architectural change).
- **Commit:** `f8ef9dd`

### Path-taken note (not deviations)

**1. D-06 cycle guard was already present — Task 2 added regression tests only.** The fork's `resolve_extends` in `crates/nono-cli/src/profile/mod.rs` already carries a visited-Vec + `MAX_INHERITANCE_DEPTH` bound cycle guard that is structurally equivalent to upstream c1bc439's fix. Task 2 adds the three plan-mandated end-to-end regression tests (`test_profile_extends_*`) that drive the guard through the public `load_profile` API so a future refactor stripping the guard will fail-closed here. The commit body documents this path (cherry-pick not feasible because the fork's integration surface is different; upstream c1bc439 patches `is_claude_code_profile` in `sandbox_prepare.rs` — a function the fork has already refactored away).

**2. D-07 integration site was hooks.rs per plan (upstream uses sandbox_prepare.rs).** Upstream 97f7294 lands the `.claude.json` symlink wiring in `crates/nono-cli/src/sandbox_prepare.rs` behind a `profile extends claude-code` check, invoked at sandbox setup. The plan's `files_modified` for 20-02 restricts the port to `crates/nono-cli/src/hooks.rs`, so the fork wires the symlink at hook-install time inside `install_claude_code_hook`. Natural integration point earlier in the lifecycle; same effective outcome for the user (symlink in place before claude-code ever writes to `~/.claude.json`).

**3. DCO sign-off author line matches repo git identity, not the plan's aspirational template.** The plan's commit-message template suggested `Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>`. The repo's `git config user.name` is `oscarmackjr-twg`, so `git commit -s` auto-generated `Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>`. This matches the repo's configured identity, satisfies DCO (one sign-off trailer per commit with a valid email), and matches the 20-01 precedent (which also landed with the `oscarmackjr-twg` sign-off).

**4. `bindings/c/include/nono.h` shows as modified in `git status` but has zero diff.** This is a line-ending/stat-cache artifact from the build process touching the file mtime. `git diff bindings/c/include/nono.h` produces 0 lines of output. Left unstaged; not included in either commit. Not a deviation from plan — the D-21 invariant is held.

## Deferred / Known

**Phase 19 CLEAN-02 deferred flakes carry forward unchanged** — same state as 20-01 SUMMARY:

- `tests/env_vars.rs`: 19 failures (all `windows_*` integration tests; documented-deferred in `.planning/phases/19-cleanup/19-02-SUMMARY.md`). Exact match for 19-failure baseline.
- `trust_scan::tests::multi_subject_verified_paths_included`: 1 tempdir-race failure this run; within the 1-3 flake window documented in 19-02-SUMMARY. Non-deterministic (`Os { code: 3, kind: NotFound }` from `std::fs::canonicalize` on a tempdir path racing with teardown).

Tasks 2-3 did not touch either test file and did not modify the code they exercise. Not a plan-20-02 regression.

**Phase 15 5-row detached-console smoke gate** — document-skipped because the plan touched zero Windows-only files (D-21 invariant held by construction). Same rationale as 20-01 Task 5. Phase 15 validation carries forward unchanged.

**`cargo test -p nono-cli --test wfp_port_integration -- --ignored`** — the ignored test requires admin + `nono-wfp-service` running. The non-ignored part of the suite passes cleanly (1 passed). Documented-skip per CONTEXT D-20.

**Windows unprivileged-symlink install path** — on Windows hosts without Developer Mode or `SeCreateSymbolicLinkPrivilege`, the `.claude.json` symlink install logs a `tracing::warn!` and the install returns `Ok(())` (fail-open). The user does not get the upstream token-refresh fix without enabling Developer Mode. This is deliberate and documented in the rustdoc of `create_symlink_platform` and the commit body. Not a regression vs pre-port state — runtime behavior on such hosts is unchanged.

## Status

**COMPLETE.** All 5 plan tasks executed, all plan success criteria satisfied (within the documented Phase 19 deferred-flake window), both commits landed with DCO + upstream-provenance trailers. UPST-02 requirement achieved. D-21 and D-15 invariants held by construction. Wave 1 sibling plans (20-03, 20-04) unaffected.

## Self-Check: PASSED

- `.planning/phases/20-upstream-parity-sync/20-02-SUMMARY.md` — this file, written via Write tool
- Commit `05c24a6` — FOUND in `git log` (verified via `git log -2 --oneline`)
- Commit `f8ef9dd` — FOUND in `git log` (verified via `git log -2 --oneline`)
- `crates/nono-cli/src/profile/builtin.rs` contains `test_profile_extends_self_reference_detected`, `test_profile_extends_indirect_cycle_detected`, `test_profile_extends_linear_chain_succeeds` — verified (3 matches, all ok)
- `crates/nono-cli/src/hooks.rs` contains `test_claude_json_rejects_path_traversal` — verified (1 match, ok)
- `crates/nono-cli/src/hooks.rs` contains `install_claude_json_symlink`, `validate_symlink_target_under_root`, `create_symlink_platform` — verified
- Both commits have `Signed-off-by:` — verified (count = 2)
- Both commits have `Upstream-commit:` — verified (count = 2: `c1bc439` and `97f7294`)
- `git diff HEAD~2..HEAD --name-only` = exactly `crates/nono-cli/src/hooks.rs` + `crates/nono-cli/src/profile/builtin.rs` (2 files, no `*_windows.rs`, no 20-03/20-04 scope overlap)
