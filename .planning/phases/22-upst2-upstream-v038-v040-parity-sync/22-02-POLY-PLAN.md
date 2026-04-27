---
phase: 22-upst2-upstream-v038-v040-parity-sync
plan: 02
type: execute
wave: 0
depends_on: []
blocks: []
files_modified:
  - crates/nono-cli/src/policy.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/data/policy.json
  - crates/nono-cli/tests/rollback_audit_conflict.rs
autonomous: true
requirements: ["POLY-01", "POLY-02", "POLY-03"]

must_haves:
  truths:
    - "Profile loading rejects orphan `override_deny` entries (no matching grant) at `Profile::resolve` with `NonoError::PolicyError { kind: OrphanOverrideDeny, .. }` (POLY-01)"
    - "Existing fork built-in profiles (claude-code, claude-no-keychain post-22-01-Task-7) all pass POLY-01 audit before the fail-closed check lands"
    - "clap layer rejects `nono run --rollback --no-audit` with `conflicts_with` error at parse time, exit code != 0 (POLY-02)"
    - "Existing rollback integration tests in `crates/nono-cli/tests/rollback*.rs` updated to NOT pair `--rollback` with `--no-audit`"
    - "`.claude.lock` resolves through `allow_file` (not `allow_dir`); pre-existing tests for `.claude.lock` write/read still green (POLY-03)"
    - "Every cherry-pick commit body contains D-19 trailers (Upstream-commit/tag/author + Signed-off-by)"
    - "Coordination with Plan 22-01: atomic small commits on shared files (`profile/mod.rs`, `policy.json`); rebase between commits if 22-01 advances"
  artifacts:
    - path: "crates/nono-cli/src/policy.rs"
      provides: "Orphan override_deny rejection (POLY-01); allow_file move for .claude.lock (POLY-03)"
    - path: "crates/nono-cli/src/cli.rs"
      provides: "clap conflicts_with on --rollback / --no-audit (POLY-02)"
    - path: "crates/nono-cli/src/profile/mod.rs"
      provides: "Profile::resolve POLY-01 enforcement site"
  key_links:
    - from: "ROADMAP § Phase 22 success criterion #2"
      to: "POLY-01..03 enforcement"
      via: "fail-closed error variants + clap conflicts_with"
      pattern: "OrphanOverrideDeny|conflicts_with"
---

<objective>
Land upstream policy-tightening cluster (POLY-01..03) into the fork via chronological cherry-pick of 6 upstream commits. Lands in Wave 0 parallel with Plan 22-01 per D-10 because policy.rs and cli.rs surfaces are disjoint from profile/mod.rs field additions. Reconciles two PATTERN-MAP contradictions (CONTRADICTION-A: fork POLY-01 already stricter; CONTRADICTION-B: fork POLY-02 may already exist) — verify before adding to avoid regressions.

Purpose: Windows users running `nono run --rollback --no-audit -- <cmd>` or loading a profile with an orphan `override_deny` entry see fail-closed errors at CLI parse / profile load time, matching macOS behavior. `.claude.lock` lock-file moves into `allow_file` policy slot.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md
@.planning/REQUIREMENTS.md
@.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md
@.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-RESEARCH.md
@.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-PATTERNS.md
@.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-VALIDATION.md
@crates/nono-cli/src/policy.rs
@crates/nono-cli/src/cli.rs
@crates/nono-cli/src/profile/mod.rs

<interfaces>
**Upstream cherry-pick chain (chronological per D-03):**

| Order | SHA | Upstream subject | REQ |
|-------|-----|------------------|-----|
| 1 | `5c301e8d` | refactor(policy): enforce stricter policy for overrides, rollback (BREAKING CLI) | POLY-01 + POLY-02 |
| 2 | `b83da813` | feat(policy): filter profile override_deny entries without grants | POLY-01 cont. |
| 3 | `930d82b4` | fix(cli): skip non-existent profile deny overrides | POLY-01 cont. |
| 4 | `49925bbf` | fix(policy): move `.claude.lock` to allow_file | POLY-03 |
| 5 | `a524b1a7` | supplementary policy adjustment | POLY-* cont. |
| 6 | `7d1d9a0d` | supplementary policy adjustment | POLY-* cont. |

**PATTERN-MAP CONTRADICTIONS to reconcile:**

- **CONTRADICTION-A:** Fork already enforces POLY-01 stricter than upstream. Verify before cherry-pick: run `cargo test -p nono-cli profile::builtin::tests::all_builtins_resolve` against the pre-cherry-pick state. If fork's posture is genuinely stricter, the upstream fail-closed check is a no-op for already-clean profiles — but it MAY break upstream-shipped permissive profiles that happen to compile fine on the fork. Plan must NOT regress fork posture.

- **CONTRADICTION-B:** Fork's POLY-02 (`--rollback`/`--no-audit` clap conflict) may already exist. Verify before Task 1:
  ```
  grep -E 'conflicts_with.*(no_audit|rollback)' crates/nono-cli/src/cli.rs
  ```
  If the conflict already exists, the cherry-pick of `5c301e8d` is partially redundant — only the POLY-01 part of `5c301e8d` needs to land; the POLY-02 part is no-op. Document in commit body if so.

**Coordination with Plan 22-01 (Wave 0 parallel per D-10):**
- `crates/nono-cli/src/profile/mod.rs` — touched by both plans (POLY-01 enforcement is at `Profile::resolve`, which 22-01 PROF-* changes are also adjacent to). Use atomic small commits per upstream SHA. Rebase if 22-01 advances during 22-02 work.
- `crates/nono-cli/data/policy.json` — touched by both plans (`claude-no-keychain` registration in 22-01 PROF-04 + `.claude.lock` allow_file move in 22-02 POLY-03). Same rebase discipline.

**D-19 commit body template (cherry-pick path):**
```
<type>(22-02): <one-line subject from upstream>

<2-3 line why-this-matters>

Upstream-commit: <sha>
Upstream-tag: v0.39.0 or v0.40.0 (verify per `git describe --tags <sha>`)
Upstream-author: <git log -1 <sha> --format='%an <%ae>'>
Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
```

**Pattern map analogs (PATTERNS.md):**
- POLY-01 enforcement follows existing `Profile::resolve` error-emission pattern + `NonoError::PolicyError` variant style — new `OrphanOverrideDeny` kind slots in identically.
- POLY-02 clap conflict follows existing `conflicts_with` attribute already in fork's CLI (verify CONTRADICTION-B above).
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: CONTRADICTION-A and CONTRADICTION-B preflight verification (BLOCKING — no commits yet)</name>
  <files>(read-only audit — no files modified)</files>
  <read_first>
    - crates/nono-cli/src/policy.rs (existing fork POLY-01 enforcement; surface `override_deny` handling)
    - crates/nono-cli/src/profile/mod.rs (existing `Profile::resolve` error paths)
    - crates/nono-cli/src/cli.rs (existing `--rollback` + `--no-audit` clap definitions; check for existing `conflicts_with`)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-PATTERNS.md (CONTRADICTION-A and CONTRADICTION-B blocks)
  </read_first>
  <action>
    1. Verify CONTRADICTION-B (POLY-02 may already exist):
       ```
       grep -nE 'conflicts_with.*("no_audit"|"rollback")' crates/nono-cli/src/cli.rs
       ```
       Record output. If a conflict between `--rollback` and `--no-audit` already exists, mark `5c301e8d`'s POLY-02 portion as **redundant** in this plan — note in subsequent task commit bodies.

    2. Verify CONTRADICTION-A (fork POLY-01 stricter than upstream):
       ```
       cargo test -p nono-cli profile::builtin::tests::all_builtins_resolve
       cargo test -p nono-cli policy::tests:: 2>&1 | grep -E 'override_deny|orphan'
       ```
       Record which tests already cover orphan-override-deny rejection. If a stricter fork-side check already returns an error variant, document the variant name (`PolicyError::OrphanOverrideDeny`? other?) so subsequent tasks don't introduce a duplicate.

    3. Verify the fork's existing built-in profiles all pass before any new fail-closed check lands:
       ```
       cargo test -p nono-cli profile::builtin:: 2>&1 | tee /tmp/22-02-baseline-builtins.log
       ```
       Save baseline pass/fail counts. If any fork built-in fails the existing POLY-01 stricter check, STOP per CONTEXT STOP trigger #7 — cannot proceed until baseline is clean.

    4. Inspect upstream `5c301e8d` to identify what's NEW vs already-in-fork:
       ```
       git show 5c301e8d --stat
       git show 5c301e8d -- crates/nono-cli/src/policy.rs crates/nono-cli/src/cli.rs
       ```
       Compare upstream's POLY-01 enforcement against fork's existing POLY-01. If they're equivalent: cherry-pick lands as a no-op (acceptable — preserves Upstream-commit: trail). If upstream is genuinely stricter on a dimension fork is permissive: cherry-pick that delta only.

    5. Record findings in a preflight note (not committed; for SUMMARY use):
       - CONTRADICTION-B: redundant / not-redundant
       - CONTRADICTION-A: fork-already-equivalent / fork-stricter / fork-permissive-delta-needed
       - Baseline built-in pass rate: N/N pass
  </action>
  <verify>
    <automated>cargo test -p nono-cli profile::builtin:: &amp;&amp; grep -nE 'conflicts_with.*("no_audit"|"rollback")' crates/nono-cli/src/cli.rs &amp;&amp; cargo test -p nono-cli policy::tests::</automated>
  </verify>
  <acceptance_criteria>
    - All fork built-in profiles pass `cargo test -p nono-cli profile::builtin::` baseline (record exact pass count for SUMMARY).
    - CONTRADICTION-B verified: either redundant (existing `conflicts_with` found) or not-redundant (no existing conflict). Decision recorded in preflight note.
    - CONTRADICTION-A verified: fork's POLY-01 baseline behavior documented in preflight note.
    - No commits made in this task (preflight only).
  </acceptance_criteria>
  <done>
    Both contradictions resolved with documented decisions. Plan 22-02 has a clean baseline and a clear delta-only cherry-pick strategy if applicable.
  </done>
</task>

<task type="auto">
  <name>Task 2: Cherry-pick `5c301e8d` — stricter override + rollback policy (POLY-01 + POLY-02)</name>
  <files>
    crates/nono-cli/src/policy.rs
    crates/nono-cli/src/cli.rs
    crates/nono-cli/src/profile/mod.rs
  </files>
  <read_first>
    - Task 1 preflight findings (CONTRADICTION-A and CONTRADICTION-B decisions)
    - `git show 5c301e8d` (full diff)
    - crates/nono-cli/src/policy.rs (current state)
    - crates/nono-cli/src/cli.rs (current state, especially around `--rollback` and `--no-audit` arg defs)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 5c301e8d
       ```

    2. **D-02 fallback gate.** policy.rs has +162 upstream / fork v2.1 never_grant + group expansion + WSFG mode encoding (per CONTEXT § Files at HIGH merge-conflict risk). Conflict is likely.
       ```
       git diff --name-only --diff-filter=U
       grep -c '<<<<<<<' crates/nono-cli/src/policy.rs crates/nono-cli/src/cli.rs crates/nono-cli/src/profile/mod.rs 2>/dev/null
       ```
       - If markers ≤ 50 lines AND ≤ 2 files AND semantic clear: resolve in place. Preserve fork's never_grant + group expansion + WSFG mode encoding (D-17 boundary on policy.rs is permissive — policy.rs is cross-platform). Add upstream's POLY-01 + POLY-02 enforcement on top.
       - Else: `git cherry-pick --abort` and apply D-20 manual-port. Replay upstream's logic semantically: (a) `Profile::resolve` returns `NonoError::PolicyError { kind: OrphanOverrideDeny, .. }` when override_deny entry has no matching grant in the resolved capability set; (b) clap layer adds `.conflicts_with("no_audit")` on the `--rollback` arg (or vice versa).

    3. **CONTRADICTION-B redundancy:** If Task 1 found existing `conflicts_with`, the upstream POLY-02 portion of `5c301e8d` may produce a no-op or duplicate. Adjust commit body to note: "POLY-02 portion is no-op — fork already enforced this conflict via Phase X (cite phase from `git log` if known)."

    4. Verify the fork's POLY-01-stricter posture is preserved (CONTRADICTION-A):
       ```
       cargo test -p nono-cli profile::builtin::tests::all_builtins_resolve
       cargo test -p nono-cli policy::tests::
       ```
       If any test that passed in Task 1 baseline now fails: STOP per CONTEXT STOP trigger #7. Investigate before committing.

    5. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       refactor(22-02): enforce stricter policy for overrides and rollback (POLY-01, POLY-02)

       POLY-01: Profile::resolve fails closed with NonoError::PolicyError {
       kind: OrphanOverrideDeny, .. } when an override_deny entry has no
       matching grant in the resolved capability set.
       POLY-02: clap layer rejects --rollback + --no-audit pairing at parse
       time via conflicts_with. <If CONTRADICTION-B redundant: "Existing
       fork conflict from Phase X is preserved; this commit aligns the
       error message with upstream.">

       Upstream-commit: 5c301e8d
       Upstream-tag: v0.39.0
       Upstream-author: <capture from `git log -1 5c301e8d --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
       Preflight: replace `<capture from ...>` with the literal value before commit.
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; cargo test -p nono-cli profile::builtin:: &amp;&amp; grep -E 'OrphanOverrideDeny|orphan_override_deny' crates/nono-cli/src/ -r &amp;&amp; grep -E 'conflicts_with.*("no_audit"|"rollback")' crates/nono-cli/src/cli.rs &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 5c301e8d'</automated>
  </verify>
  <acceptance_criteria>
    - `grep -rE 'OrphanOverrideDeny' crates/nono-cli/src/` returns ≥ 1 hit.
    - `grep -nE 'conflicts_with.*("no_audit"|"rollback")' crates/nono-cli/src/cli.rs` returns ≥ 1 hit.
    - `cargo build --workspace` exits 0.
    - `cargo test -p nono-cli profile::builtin::` matches Task 1 baseline pass count (no regression).
    - `git log -1 --format=%B | grep '^Upstream-commit: 5c301e8d'` returns 1 line.
    - VALIDATION 22-02-T1 (`orphan_override_deny_rejected`) and 22-02-T2 (`rollback_no_audit_conflict`) tests are green or stubbed for Wave 0.
  </acceptance_criteria>
  <done>
    POLY-01 and POLY-02 enforcement landed; fork POLY-01-stricter posture preserved; commit on main with D-19 trailers.
  </done>
</task>

<task type="auto">
  <name>Task 3: Cherry-pick `b83da813` and `930d82b4` — POLY-01 follow-ups</name>
  <files>
    crates/nono-cli/src/policy.rs
    crates/nono-cli/src/profile/mod.rs
    crates/nono-cli/src/cli.rs
  </files>
  <read_first>
    - `git show b83da813 --stat` and full diff
    - `git show 930d82b4 --stat` and full diff
    - Task 2 outcome (POLY-01 enforcement code path)
  </read_first>
  <action>
    1. Cherry-pick `b83da813` (filter override_deny entries without grants):
       ```
       git cherry-pick b83da813
       ```
       D-02 gate.

    2. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-02): filter profile override_deny entries without grants

       POLY-01 follow-up: pre-resolve filter on override_deny entries that
       have no matching grant (vs fail-closed in 5c301e8d). Treats orphans
       as warnings rather than hard failures during profile composition.

       Upstream-commit: b83da813
       Upstream-tag: v0.39.0
       Upstream-author: <capture from `git log -1 b83da813 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    3. Cherry-pick `930d82b4` (skip non-existent profile deny overrides):
       ```
       git cherry-pick 930d82b4
       ```
       D-02 gate.

    4. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       fix(22-02): skip non-existent profile deny overrides

       POLY-01 edge case: deny override referencing a profile that doesn't
       exist no longer panics; resolves to a structured error or skip per
       upstream's semantic.

       Upstream-commit: 930d82b4
       Upstream-tag: v0.39.0
       Upstream-author: <capture from `git log -1 930d82b4 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    5. Verify:
       ```
       cargo test -p nono-cli profile::builtin::
       cargo test -p nono-cli policy::tests::
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; cargo test -p nono-cli policy::tests:: &amp;&amp; git log -2 --format='%b' | grep -E '^Upstream-commit: (b83da813|930d82b4)' | wc -l</automated>
  </verify>
  <acceptance_criteria>
    - `git log -2 --format=%B | grep -E '^Upstream-commit: (b83da813|930d82b4)' | wc -l` returns `2`.
    - `cargo test -p nono-cli policy::tests::` exits 0.
    - Each commit body has 1 Signed-off-by line.
  </acceptance_criteria>
  <done>
    POLY-01 fully landed (3 commits: 5c301e8d POLY-01 portion + b83da813 + 930d82b4).
  </done>
</task>

<task type="auto">
  <name>Task 4: Cherry-pick `49925bbf` — `.claude.lock` allow_file move (POLY-03)</name>
  <files>
    crates/nono-cli/src/policy.rs
    crates/nono-cli/data/policy.json
  </files>
  <read_first>
    - `git show 49925bbf --stat` and full diff
    - crates/nono-cli/src/policy.rs (current `.claude.lock` handling — check if it's allow_dir or allow_file currently)
    - crates/nono-cli/data/policy.json (current `claude-code` profile — `.claude.lock` placement)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 49925bbf
       ```
       D-02 gate. Note: policy.json is co-touched by Plan 22-01 PROF-04 (claude-no-keychain inherits from claude-code). If 22-01 has shipped Task 7 (3c8b6756), the policy.json shape may differ from upstream's pre-49925bbf state. Resolve conflict by:
       - Preserving 22-01's `claude-no-keychain` entry (don't lose it)
       - Applying 49925bbf's `.claude.lock` move from `allow_dir` (or wherever) to `allow_file`
       - Verifying `claude-no-keychain` inherits the correct allow_file lock entry from `claude-code`

    2. Verify `.claude.lock` write/read tests:
       ```
       cargo test -p nono-cli policy::tests::claude_lock_in_allow_file
       cargo test -p nono-cli claude_lock 2>&1 | grep -E '^test '
       ```
       Pre-existing tests must remain green (`.claude.lock` write/read behavior preserved).

    3. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       fix(22-02): move .claude.lock to allow_file (POLY-03)

       Lock file moves from allow_dir scope to allow_file scope so the
       sandboxed agent gets a single-file grant rather than a directory
       grant. Tightens permission scope; pre-existing .claude.lock
       write/read tests preserved.

       Upstream-commit: 49925bbf
       Upstream-tag: v0.39.0
       Upstream-author: <capture from `git log -1 49925bbf --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    4. **Coordinate with Plan 22-01 Task 8 (`713b2e0f`):** Plan 22-01 Task 8's test fixup is consequent to this commit. If 22-01 deferred 713b2e0f per Task 8 step 2(b), now is the chronologically correct moment for that cherry-pick. Coordinate inter-plan: either:
       - (a) 22-02 lands 49925bbf, then 22-01 lands 713b2e0f as a follow-up (preferred per D-03 chronological order — 49925bbf < 713b2e0f).
       - (b) 22-02 includes a SUMMARY note that 22-01 should now land 713b2e0f.
       Document the coordination decision in SUMMARY.
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; cargo test -p nono-cli policy::tests::claude_lock 2&gt;&amp;1 | tail -5 &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 49925bbf'</automated>
  </verify>
  <acceptance_criteria>
    - `cargo test -p nono-cli policy::tests::claude_lock` (or whatever the exact test names are) all pass.
    - `git log -1 --format=%B | grep '^Upstream-commit: 49925bbf'` returns 1 line.
    - SUMMARY documents the 713b2e0f coordination decision with Plan 22-01.
  </acceptance_criteria>
  <done>
    POLY-03 landed. `.claude.lock` resolves through allow_file. Plan 22-01 Task 8 unblocked if previously deferred.
  </done>
</task>

<task type="auto">
  <name>Task 5: Cherry-pick `a524b1a7` and `7d1d9a0d` — supplementary policy adjustments</name>
  <files>(varies — read upstream stat first)</files>
  <read_first>
    - `git show a524b1a7 --stat` and `git show 7d1d9a0d --stat` (confirm supplementary scope; some may be tests, fmt, or docs)
  </read_first>
  <action>
    Cherry-pick each in chronological order per D-03:
    ```
    git cherry-pick a524b1a7
    # D-02 gate per file
    git commit --amend -s -m "$(cat <<'EOF'
    <type>(22-02): <subject from upstream a524b1a7>

    <2-3 line context from upstream commit body>

    Upstream-commit: a524b1a7
    Upstream-tag: <git describe --tags a524b1a7>
    Upstream-author: <capture from `git log -1 a524b1a7 --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"

    git cherry-pick 7d1d9a0d
    git commit --amend -s -m "$(cat <<'EOF'
    <type>(22-02): <subject from upstream 7d1d9a0d>

    Upstream-commit: 7d1d9a0d
    Upstream-tag: <git describe --tags 7d1d9a0d>
    Upstream-author: <capture from `git log -1 7d1d9a0d --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"
    ```

    Per-commit spot check: `cargo test --workspace --lib`. If red, STOP per STOP trigger #2.
  </action>
  <verify>
    <automated>git log -2 --format='%b' | grep -E '^Upstream-commit: (a524b1a7|7d1d9a0d)' | wc -l &amp;&amp; cargo test --workspace --lib</automated>
  </verify>
  <acceptance_criteria>
    - `git log -2 --format=%B | grep -E '^Upstream-commit: (a524b1a7|7d1d9a0d)' | wc -l` returns `2`.
    - `cargo test --workspace --lib` exits 0.
  </acceptance_criteria>
  <done>
    POLY-* supplementary adjustments landed. Full POLY cluster complete.
  </done>
</task>

<task type="auto">
  <name>Task 6: Update existing rollback integration tests so they no longer pair --rollback + --no-audit</name>
  <files>crates/nono-cli/tests/rollback*.rs</files>
  <read_first>
    - All `crates/nono-cli/tests/rollback*.rs` files (`grep -lE '\-\-rollback' crates/nono-cli/tests/`)
    - Task 2 outcome (POLY-02 conflicts_with on --rollback / --no-audit)
  </read_first>
  <action>
    1. Find affected tests:
       ```
       grep -lE 'rollback' crates/nono-cli/tests/ -r
       grep -nE 'rollback.*no.audit|no.audit.*rollback' crates/nono-cli/tests/ -r
       ```

    2. For each test that pairs `--rollback` with `--no-audit`: edit to remove the `--no-audit` flag (the test's intent is presumably rollback behavior, not no-audit; if a test specifically wants both, it's now testing the conflict-rejection path, which should be a NEW test).

    3. Add ONE new test (in `crates/nono-cli/tests/rollback_audit_conflict.rs` — new file) that explicitly verifies the conflict rejection:
       ```rust
       // crates/nono-cli/tests/rollback_audit_conflict.rs
       #[test]
       fn rollback_no_audit_conflict_rejected_at_parse() {
           let output = std::process::Command::new(env!("CARGO_BIN_EXE_nono"))
               .args(["run", "--rollback", "--no-audit", "--", "echo", "x"])
               .output()
               .expect("nono binary should be present");
           assert!(!output.status.success(), "expected non-zero exit");
           let stderr = String::from_utf8_lossy(&output.stderr);
           assert!(stderr.contains("conflicts with") || stderr.contains("conflict"),
                   "expected clap conflict error, got: {}", stderr);
       }
       ```

    4. Commit:
       ```
       git add crates/nono-cli/tests/rollback*.rs
       git commit -s -m "$(cat <<'EOF'
       test(22-02): align rollback integration tests with POLY-02 conflicts_with

       Existing tests pairing --rollback with --no-audit updated to drop
       --no-audit. New test rollback_audit_conflict.rs explicitly verifies
       the clap conflict rejection (REQ-POLY-02 acceptance).

       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
       (Fork-only commit — no `Upstream-commit:` trailer.)

    5. Verify:
       ```
       cargo test -p nono-cli --test rollback_audit_conflict
       cargo test --workspace --lib
       ```
  </action>
  <verify>
    <automated>cargo test -p nono-cli --test rollback_audit_conflict &amp;&amp; cargo test --workspace --lib</automated>
  </verify>
  <acceptance_criteria>
    - New file `crates/nono-cli/tests/rollback_audit_conflict.rs` exists.
    - `cargo test -p nono-cli --test rollback_audit_conflict` exits 0.
    - `cargo test --workspace --lib` exits 0 (no rollback regressions).
    - Commit has Signed-off-by trailer.
  </acceptance_criteria>
  <done>
    Rollback integration tests aligned with POLY-02 conflicts_with. Fail-closed conflict path has explicit test coverage.
  </done>
</task>

<task type="auto">
  <name>Task 7: D-18 Windows-regression gate (BLOCKING — final per-plan close)</name>
  <files>(read-only verification)</files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § D-18
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-VALIDATION.md (per-task verification map for 22-02-T1..T3 + V1)
  </read_first>
  <action>
    1. `cargo test --workspace --all-features` (workspace test).
    2. Phase 15 5-row detached-console smoke gate (Windows host manual).
    3. `cargo test -p nono-cli --test wfp_port_integration -- --ignored` (admin + service available).
    4. `cargo test -p nono-cli --test learn_windows_integration`.
    5. VALIDATION.md gate: 22-02-T1..T3 + V1 green.

    If ANY new regression: STOP. Revert offending commits and re-scope.
  </action>
  <verify>
    <automated>cargo test --workspace --all-features &amp;&amp; cargo test -p nono-cli --test learn_windows_integration &amp;&amp; cargo fmt --all -- --check &amp;&amp; cargo clippy --workspace -- -D warnings -D clippy::unwrap_used</automated>
  </verify>
  <acceptance_criteria>
    - `cargo test --workspace --all-features` exits 0 within deferred-flake window.
    - Phase 15 5-row smoke gate passes (or documented-skip).
    - `wfp_port_integration --ignored` passes or documented-skipped.
    - `learn_windows_integration` exits 0.
    - `cargo fmt --all -- --check` exits 0.
    - `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` exits 0.
    - VALIDATION.md 22-02-T1..T3 + V1 status updated to green.
  </acceptance_criteria>
  <done>
    D-18 Windows-regression safety net cleared for Plan 22-02.
  </done>
</task>

<task type="auto">
  <name>Task 8: D-07 plan-close push to origin</name>
  <files>(no files modified — git push only)</files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § D-07
  </read_first>
  <action>
    ```
    git fetch origin
    git log --oneline origin/main..main   # capture pre-push state
    git push origin main
    git ls-remote origin main             # capture post-push origin/main SHA
    ```
  </action>
  <verify>
    <automated>git fetch origin &amp;&amp; test "$(git log origin/main..main --oneline | wc -l)" = "0"</automated>
  </verify>
  <acceptance_criteria>
    - `git log origin/main..main --oneline | wc -l` returns `0` after push.
    - SUMMARY records the post-push origin/main SHA.
  </acceptance_criteria>
  <done>
    Plan 22-02 commits published to origin.
  </done>
</task>

</tasks>

<non_goals>
**Windows-invariance (D-17):** No Windows-only file (`*_windows.rs` or `target_os="windows"` block) is touched. policy.rs, cli.rs, profile/mod.rs are all cross-platform.

**Plan 22-01 PROF scope:** Profile struct field additions (PROF-01..04) live in 22-01. POLY plan only edits `Profile::resolve` for POLY-01 enforcement; field shape is owned by 22-01.

**`prune` → `session cleanup` rename:** Owned by Plan 22-05 AUD-04, not POLY plan.

**No new requirement IDs:** POLY-01..03 covered; no others added or removed.
</non_goals>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Profile JSON → Profile::resolve | Untrusted profile data crosses into resolution; orphan override_deny is a privilege-confusion vector. |
| CLI args → clap parser | User-supplied flag combinations cross into command invocation; conflicting safety flags must be rejected at parse time, not silently merged. |
| `.claude.lock` allow_file scope | Reduces permission scope for an existing capability; defensive depth not new attack surface. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation |
|-----------|----------|-----------|----------|-------------|------------|
| T-22-02-01 | Elevation of Privilege | Orphan `override_deny` entry surfaces as silent grant (intent of `override_deny` was to remove a grant, but no grant exists, so user thinks deny applies but actually nothing happens) | **high** | mitigate (BLOCKING) | POLY-01: `Profile::resolve` returns `NonoError::PolicyError { kind: OrphanOverrideDeny }` when override_deny entry has no matching grant. Test: `policy::tests::orphan_override_deny_rejected`. |
| T-22-02-02 | Elevation of Privilege | `--rollback --no-audit` pairing skips audit for a privileged operation (rollback writes to filesystem, no-audit suppresses ledger) | **high** | mitigate (BLOCKING) | POLY-02: clap `conflicts_with` rejects pairing at parse time. Test: `cli::tests::rollback_no_audit_conflict`. |
| T-22-02-03 | Tampering | `.claude.lock` accidentally grants whole-directory access via allow_dir | medium | mitigate | POLY-03: move to allow_file (single-file grant). Tightens permission scope. |
| T-22-02-04 | Repudiation | Cherry-pick provenance lost | medium | mitigate | D-19 trailers enforced. |
| T-22-02-05 | Denial of Service | Stricter POLY-01 rejects fork's existing built-in profiles, breaking `nono run --profile claude-code` | **high** | mitigate (BLOCKING) | Task 1 preflight verifies fork built-ins all pass baseline before adding stricter check; Task 2 verifies posture preserved post-cherry-pick. STOP trigger #7 if any built-in fails. |
| T-22-02-06 | DoS | clap `conflicts_with` rejection error message is unclear, user can't diagnose | low | accept | Upstream's error message preserved by cherry-pick; if unclear, fork can refine in a follow-up commit. |

**BLOCKING threats:** T-22-02-01, T-22-02-02, T-22-02-05 (high severity) — Plan 22-02 cannot close until these are mitigated and verified.
</threat_model>

<verification>
- `cargo build --workspace` exits 0.
- `cargo test --workspace --all-features` exits 0 within deferred-flake tolerance.
- `cargo fmt --all -- --check` exits 0.
- `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` exits 0.
- Phase 15 5-row smoke gate passes.
- VALIDATION.md 22-02-T1..T3 + V1 marked green.
- All cherry-pick commits carry D-19 trailers.
- `git log origin/main..main` shows zero commits ahead post-Task 8.
- No `<capture from` placeholders in any commit body.
- Coordination decisions with Plan 22-01 (713b2e0f, policy.json shared edits) recorded in SUMMARY.
</verification>

<success_criteria>
- 6 atomic upstream cherry-pick commits + 1 fork-only test alignment commit on `main`.
- Profile loading rejects orphan override_deny fail-closed; clap rejects --rollback + --no-audit pairing.
- `.claude.lock` resolves through allow_file.
- Fork's pre-existing POLY-01-stricter posture preserved (CONTRADICTION-A reconciled).
- Existing rollback integration tests aligned; new conflict-rejection test green.
- `make ci` green or matches Phase 19 deferred window.
- `origin/main` advanced to plan-close HEAD.
- Plan SUMMARY records all 8 tasks' outcomes, ~7 commit hashes, and the 713b2e0f coordination decision with Plan 22-01.
</success_criteria>

<output>
Create `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-02-SUMMARY.md` per standard summary template. Required sections: Outcome, What was done (one bullet per task), Verification table, Files changed table, Commits (~7-row table with hashes + upstream provenance), Status, Deferred (CONTRADICTION-A/B preflight findings, Plan 22-01 coordination notes for 713b2e0f).
</output>
