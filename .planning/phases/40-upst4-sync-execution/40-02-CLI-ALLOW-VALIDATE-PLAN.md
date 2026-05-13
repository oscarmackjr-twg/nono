---
phase: 40-upst4-sync-execution
plan: 02
slug: cli-allow-validate
cluster_id: C2
type: execute
wave: 0
depends_on: []
files_modified:
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/sandbox_state.rs
  - crates/nono-cli/src/why_runtime.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/query_ext.rs
upstream_tag_range: v0.52.0..v0.52.1
upstream_commit_count: 2
autonomous: true
requirements: [REQ-UPST4-02]
tags: [upst4, c2, cli, sandbox-state, foundation, wave-0]

must_haves:
  truths:
    - "All 2 cluster-C2 commits cherry-picked onto main in upstream chronological order"
    - "Every Plan 40-02 commit body carries the verbatim D-19 6-line trailer block (lowercase 'a' in Upstream-author)"
    - "f72ea31 — validate --allow paths and persist domain allowlist in sandbox state"
    - "85f0acc — make nono why --host aware of proxy domain filtering"
    - "D-40-E1 invariant: zero edits to *_windows.rs or exec_strategy_windows/ for every commit"
    - "validate_path_within defense-in-depth retained if upstream commit removes it"
    - "All 8 D-40-C2 close-gates pass"
  artifacts:
    - path: "crates/nono-cli/src/sandbox_state.rs"
      provides: "domain allowlist persistence from --allow paths (f72ea31)"
      grep_pattern: "domain_allowlist|allow_domain"
    - path: "crates/nono-cli/src/cli.rs"
      provides: "--allow path validation flag wiring (f72ea31)"
      grep_pattern: "validate.*allow|allow.*validate"
    - path: "crates/nono-cli/src/why_runtime.rs"
      provides: "nono why --host proxy-domain awareness (85f0acc)"
      grep_pattern: "proxy.*domain|domain.*filter"
  key_links:
    - from: "nono run --allow <path>"
      to: "SandboxState domain_allowlist field"
      via: "cli.rs argument parsing → sandbox_state.rs persistence"
      pattern: "allow.*SandboxState|SandboxState.*allow"
    - from: "nono why --host <host>"
      to: "proxy domain filtering query"
      via: "why_runtime.rs → query_ext.rs"
      pattern: "why.*host.*proxy|proxy.*domain.*why"
---

<objective>
Cluster C2 (upstream v0.52.0..v0.52.1, 2 commits): CLI --allow path validation + sandbox state domain-allowlist persistence + nono why --host proxy-domain awareness.

Wave-0 FOUNDATION plan. The SandboxState shape extension from f72ea31 is consumed by Wave-1 plans (40-01 proxy hardening reads SandboxState; 40-04 release ride-alongs build atop post-Wave-0 state). Run in parallel with Plan 40-03 (Cluster 6 scrub module) — surfaces are disjoint (C2 touches nono-cli/; C6 touches nono/src/scrub.rs).

Security: f72ea31 closes an --allow path validation gap that affects all platforms. Retain validate_path_within defense-in-depth if upstream removes it per fork-divergence catalog.

Output: 2 atomic commits with D-19 trailers; PR opened.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/phases/40-upst4-sync-execution/40-CONTEXT.md
@.planning/phases/39-upst4-audit/DIVERGENCE-LEDGER.md
@.planning/templates/upstream-sync-quick.md

<interfaces>
**Cluster C2 cherry-pick chain (2 commits, chronological):**

| Order | SHA | Tag | Subject | Files changed | Upstream Author |
|-------|-----|-----|---------|---------------|-----------------|
| 1 | `f72ea31` | v0.52.1 | fix(cli): validate --allow paths and persist domain allowlist in sandbox state | 5 | unknown (upstream) |
| 2 | `85f0acc` | v0.52.1 | fix(cli): make 'nono why --host' aware of proxy domain filtering | 2 | unknown (upstream) |

**D-19 trailer shape (verbatim — use this exact casing):**

```
Upstream-commit: <8-char-sha>
Upstream-tag: v0.52.1
Upstream-author: <Name> <<email>>
Co-Authored-By: <Name> <<email>>
Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
```

Field `Upstream-author` MUST use lowercase 'a' (not 'Upstream-Author').

**Fork-divergence watch for C2:**
- `validate_path_within(child, parent)` calls in sandbox_state.rs or cli.rs: if upstream's f72ea31 removes any such call, KEEP it and annotate:
  `// Defense-in-depth (fork divergence: see Phase 22-03 PKG-04). Do not remove without security review.`
- validate_path_within uses Path::components() iteration — NOT string starts_with() (CLAUDE.md § Common Footguns #1).
- SandboxState extension: verify the new `domain_allowlist` field (or equivalent) does NOT overwrite the Windows-side sandbox_state deserialization paths from Phase 09 cross-platform work.

**Surface disjointness with Plan 40-03 (running in parallel):**
- Plan 40-02 touches: crates/nono-cli/src/ only
- Plan 40-03 touches: crates/nono/src/scrub.rs (NEW file) + crates/nono/src/lib.rs
- ZERO file overlap — safe to run Wave 0 in parallel.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Pre-flight — verify baseline + fetch upstream tags</name>
  <files>(git operations only)</files>
  <read_first>
    - .planning/phases/40-upst4-sync-execution/40-CONTEXT.md § D-40-A2 (Wave 0 parallel foundations), D-40-C4 (D-19 trailer shape), D-40-E1 (Windows-only invariant)
    - .planning/phases/39-upst4-audit/DIVERGENCE-LEDGER.md § Cluster C2 row
    - .planning/templates/upstream-sync-quick.md § D-19 cherry-pick trailer block + Fork-divergence catalog
  </read_first>
  <action>
    1. Fetch upstream tags to ensure C2 SHAs are reachable:
       ```bash
       git fetch upstream --tags
       git cat-file -t f72ea31   # Expected: commit
       git cat-file -t 85f0acc   # Expected: commit
       ```
    2. Record pre-plan baseline for Windows-only file sentinel:
       ```bash
       git log -1 --format='%H' -- crates/nono-cli/src/exec_strategy_windows/
       # Save this SHA as PRE_PLAN_WINDOWS_SHA for Task 2 verification
       ```
    3. Baseline build:
       ```bash
       cargo build --workspace
       ```
    4. Verify no unfilled placeholders in this PLAN.md:
       ```bash
       grep -oE '\{[a-z_]+\}' .planning/phases/40-upst4-sync-execution/40-02-CLI-ALLOW-VALIDATE-PLAN.md | wc -l
       # Expected: 0
       ```
  </action>
  <verify>
    <automated>git fetch upstream --tags &amp;&amp; git cat-file -t f72ea31 &amp;&amp; git cat-file -t 85f0acc &amp;&amp; cargo build --workspace</automated>
  </verify>
  <acceptance_criteria>
    - Both C2 SHAs reachable via upstream remote; cargo build --workspace exits 0; pre-plan Windows-only SHA recorded; zero unfilled PLAN.md placeholders.
  </acceptance_criteria>
  <done>
    Ready for C2 cherry-pick chain.
  </done>
</task>

<task type="auto">
  <name>Task 2: Cherry-pick both C2 commits with D-19 trailers</name>
  <files>
    crates/nono-cli/src/cli.rs
    crates/nono-cli/src/sandbox_state.rs
    crates/nono-cli/src/why_runtime.rs
    crates/nono-cli/src/profile/mod.rs
    crates/nono-cli/src/query_ext.rs
  </files>
  <read_first>
    - crates/nono-cli/src/sandbox_state.rs (read current structure before cherry-pick — know what fields exist pre-C2)
    - crates/nono-cli/src/cli.rs (read --allow argument parsing section)
    - crates/nono-cli/src/why_runtime.rs (read current --host branch)
    - git show f72ea31 (read the full upstream diff before cherry-picking — know what it changes)
    - git show 85f0acc (read the full upstream diff before cherry-picking)
    - .planning/templates/upstream-sync-quick.md § Fork-divergence catalog (validate_path_within section)
  </read_first>
  <action>
    **Before each cherry-pick:** run `git show <sha>` to read the full upstream diff. Cross-check against fork-divergence catalog. Only then cherry-pick.

    **Commit 1/2: f72ea31 (fix(cli): validate --allow paths and persist domain allowlist in sandbox state)**

    ```bash
    git show f72ea31   # Read full diff first
    git cherry-pick f72ea31
    # Resolve any conflicts:
    # - If upstream removes validate_path_within calls in sandbox_state.rs or cli.rs, KEEP them
    #   and add: // Defense-in-depth (fork divergence: see Phase 22-03 PKG-04). Do not remove without security review.
    # - If new domain_allowlist field in SandboxState conflicts with Windows deserialization paths,
    #   preserve fork's cfg-gated Windows arms and add the new field around them
    cargo build --workspace
    # D-40-E1 check (MUST be zero):
    git diff --stat HEAD~1 HEAD -- crates/ | grep -E '_windows|exec_strategy_windows'
    # Amend with D-19 trailer (read upstream author from git show f72ea31 output):
    git commit --amend --no-edit   # Then open editor and append trailer
    # Trailer to append (ONE blank line before trailer block):
    # Upstream-commit: f72ea31
    # Upstream-tag: v0.52.1
    # Upstream-author: <Name from git show f72ea31> <<email>>
    # Co-Authored-By: <Name from git show f72ea31> <<email>>
    # Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    # Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
    git diff --stat HEAD~1 HEAD -- crates/ | grep -E '_windows|exec_strategy_windows' | wc -l   # MUST be 0
    ```

    **Commit 2/2: 85f0acc (fix(cli): make 'nono why --host' aware of proxy domain filtering)**

    ```bash
    git show 85f0acc   # Read full diff first
    git cherry-pick 85f0acc
    # why_runtime.rs + query_ext.rs: absorb proxy-domain awareness into --host branch
    # No fork-only Windows why-runtime wiring needed per D-40-E6 (surgical retrofit posture)
    cargo build --workspace
    git diff --stat HEAD~1 HEAD -- crates/ | grep -E '_windows|exec_strategy_windows' | wc -l   # MUST be 0
    git commit --amend --no-edit
    # Append trailer:
    # Upstream-commit: 85f0acc
    # Upstream-tag: v0.52.1
    # Upstream-author: <Name from git show 85f0acc> <<email>>
    # Co-Authored-By: <Name from git show 85f0acc> <<email>>
    # Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    # Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
    ```

    **Post-chain smoke checks:**
    ```bash
    git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-commit: '   # MUST be 2
    git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-author: '   # MUST be 2 (lowercase 'a')
    git log --format='%B' HEAD~2..HEAD | grep -c '^Signed-off-by: '     # MUST be 4
    # D-40-E1 final: Windows-only sentinel SHA unchanged
    git log -1 --format='%H' -- crates/nono-cli/src/exec_strategy_windows/
    # MUST equal PRE_PLAN_WINDOWS_SHA from Task 1
    ```
  </action>
  <verify>
    <automated>git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-commit: ' | grep -E '^2$' &amp;&amp; cargo build --workspace</automated>
  </verify>
  <acceptance_criteria>
    - 2 commits on main; each commit body ends with verbatim D-19 6-line trailer (lowercase 'a' in Upstream-author).
    - Per-commit D-40-E1 check: `git diff --stat HEAD~N HEAD~N+1 -- crates/ | grep -E '_windows|exec_strategy_windows'` returns empty for both commits (N=2 and N=1).
    - Windows-only sentinel SHA unchanged from Task 1 baseline.
    - `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-commit: '` returns 2.
    - `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-author: '` returns 2 (lowercase 'a').
    - validate_path_within calls preserved if upstream removed them (grep confirms presence).
    - `cargo build --workspace` exits 0 after each cherry-pick.
    - nono why --host awareness: `grep -r 'proxy.*domain\|domain.*filter' crates/nono-cli/src/why_runtime.rs` returns at least 1 line.
  </acceptance_criteria>
  <done>
    C2 chain complete; SandboxState domain-allowlist extended; nono why --host proxy-aware.
  </done>
</task>

<task type="auto">
  <name>Task 3: D-40-C2 8-check close gate</name>
  <files>(read-only verification)</files>
  <read_first>
    - .planning/phases/40-upst4-sync-execution/40-CONTEXT.md § D-40-C2 (8-check gate verbatim)
  </read_first>
  <action>
    Run all 8 D-40-C2 close-gates in order. Stop and freeze if ANY fails (D-40-C3 STOP-trigger):

    ```bash
    # Gate 1: cargo test
    cargo test --workspace --all-features

    # Gate 2: Windows-host clippy
    cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used

    # Gate 3: Linux cross-target clippy (Phase 25 CR-A lesson — catches cfg-gated Linux drift)
    cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used

    # Gate 4: macOS cross-target clippy (symmetric coverage)
    cargo clippy --workspace --all-targets --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used

    # Gate 5: fmt check
    cargo fmt --all -- --check

    # Gate 6: Phase 15 5-row detached-console smoke gate
    # nono run --detached <cmd> → nono ps → nono attach → detach → nono stop
    # Document result (PASS / documented-skipped with reason)

    # Gate 7: wfp_port_integration test suite
    cargo test --workspace --all-features -p nono-cli -- wfp_port_integration
    # Document: PASS or documented-skipped with admin/service-not-available reason

    # Gate 8: learn_windows_integration test suite
    cargo test --workspace --all-features -p nono-cli -- learn_windows_integration
    # Document: PASS or documented-skipped with reason
    ```

    If Gates 3 or 4 fail with cross-compiler-not-installed error: document as "skip — cross-compiler unavailable on Windows host; CI matrix will catch" per D-40-C2 note in CONTEXT.md.
  </action>
  <verify>
    <automated>cargo test --workspace --all-features &amp;&amp; cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used &amp;&amp; cargo fmt --all -- --check</automated>
  </verify>
  <acceptance_criteria>
    - Gates 1 + 2 + 5: PASS (no skip allowed).
    - Gates 3 + 4: PASS or documented-skipped (cross-compiler unavailable on Windows host).
    - Gates 6 + 7 + 8: PASS or documented-skipped with admin/service-not-available reason.
    - D-19 trailer smoke: `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-commit: '` returns 2.
    - D-40-E1 final: `git diff --stat HEAD~2 HEAD -- crates/ | grep -E '_windows|exec_strategy_windows' | wc -l` returns 0.
  </acceptance_criteria>
  <done>
    All 8 D-40-C2 gates cleared (or documented-skipped per policy); Plan 40-02 close-gate passed.
  </done>
</task>

<task type="auto">
  <name>Task 4: Push + PR</name>
  <files>(git push only)</files>
  <read_first>
    - .planning/phases/40-upst4-sync-execution/40-CONTEXT.md § D-40-C1 (one PR per plan, direct-on-main)
  </read_first>
  <action>
    1. `git push origin main`
    2. Open PR via gh CLI:
       ```bash
       gh pr create \
         --title "Plan 40-02 (C2): CLI --allow path validation + sandbox state domain-allowlist + nono why --host (v0.52.1, 2 commits)" \
         --body "$(cat <<'EOF'
    Wave 0 foundation plan. Absorbs upstream v0.52.1 Cluster C2 (2 commits):

    - f72ea31: validate --allow paths; persist domain allowlist in SandboxState
    - 85f0acc: nono why --host proxy-domain filtering awareness

    D-19 trailers: 2/2. D-40-E1 violations: 0. D-40-C2 gates: all pass.
    Part of Phase 40 UPST4 sync execution (REQ-UPST4-02).
    EOF
       )"
       ```
  </action>
  <verify>
    <automated>git fetch origin &amp;&amp; test "$(git log origin/main..main --oneline 2>/dev/null | wc -l)" = "0"</automated>
  </verify>
  <acceptance_criteria>
    - `git push origin main` exits 0; PR created; origin/main updated.
  </acceptance_criteria>
  <done>
    Plan 40-02 published; Wave 0 C2 PR open.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| User → nono CLI --allow argument | Untrusted path input crossing into SandboxState persistence |
| nono why --host argument | Untrusted hostname crossing into proxy-domain filter query |
| Upstream diff → fork integration | Cherry-pick may silently remove fork-side path-validation defense-in-depth |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation |
|-----------|----------|-----------|----------|-------------|------------|
| T-40-02-01 | Tampering | D-40-E1 Windows-only files invariant | **HIGH** | mitigate (BLOCKING) | Per-commit `git diff --stat HEAD~N HEAD~N+1 -- crates/ | grep -E '_windows|exec_strategy_windows'` must return 0; Windows-only sentinel SHA unchanged from pre-plan baseline |
| T-40-02-02 | Repudiation | D-19 trailer missing on any cherry-pick commit | **HIGH** | mitigate (BLOCKING) | Post-chain: `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-commit: '` must equal 2 |
| T-40-02-03 | Elevation of Privilege | validate_path_within silently dropped by upstream f72ea31 | **HIGH** | mitigate (BLOCKING) | Read `git show f72ea31` BEFORE cherry-pick; if removed, re-add with defense-in-depth annotation per fork-divergence catalog |
| T-40-02-04 | Elevation of Privilege | Path traversal in --allow argument (string starts_with vs Path::starts_with) | **HIGH** | mitigate | CLAUDE.md § Common Footguns #1: use Path::components() iteration, not string ops; verify in cherry-pick diff |
| T-40-02-05 | Information Disclosure | nono why --host leaks proxy credential state to unauthorized caller | medium | accept | nono why is a diagnostic command; access-control is the same as nono run; no additional exposure |
| T-40-02-06 | Tampering | SandboxState domain_allowlist field overwrites Windows-cfg-gated deserialization arms | medium | mitigate | Inspect cherry-pick conflicts in sandbox_state.rs; preserve #[cfg(target_os = "windows")] arms around new field |
</threat_model>

<verification>
- All 8 D-40-C2 close-gates pass (Gates 1+2+5 non-skippable; 3+4+6+7+8 documented if skipped).
- `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-commit: '` returns 2.
- `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-author: '` returns 2 (lowercase 'a' verified).
- `git diff --stat HEAD~2 HEAD -- crates/ | grep -E '_windows|exec_strategy_windows' | wc -l` returns 0.
- validate_path_within preserved: `grep -r 'validate_path_within' crates/nono-cli/src/` returns results if the function was present pre-C2.
- `cargo build --workspace` exits 0.
- origin/main advanced; PR created.
</verification>

<success_criteria>
- 2 atomic commits on main, each with verbatim D-19 6-line trailer (lowercase 'a').
- SandboxState extended with domain-allowlist persistence (--allow paths validated).
- nono why --host proxy-domain aware.
- Wave-0 foundation surface ready for Wave-1 plans (40-01, 40-04) to rebase on top.
- All 8 D-40-C2 gates cleared.
- origin/main advanced; PR open.
</success_criteria>

<output>
After completion, create `.planning/phases/40-upst4-sync-execution/40-02-CLI-ALLOW-VALIDATE-SUMMARY.md`.
</output>
