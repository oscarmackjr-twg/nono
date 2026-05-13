---
phase: 40-upst4-sync-execution
plan: 03
slug: scrub-module
cluster_id: C6
type: execute
wave: 0
depends_on: []
files_modified:
  - crates/nono/src/scrub.rs
  - crates/nono/src/lib.rs
  - crates/nono-cli/src/audit_integrity.rs
  - crates/nono-cli/src/audit_ledger.rs
  - crates/nono-cli/src/command_runtime.rs
upstream_tag_range: v0.52.2..v0.53.0
upstream_commit_count: 2
autonomous: true
requirements: [REQ-UPST4-02]
tags: [upst4, c6, scrub, audit, foundation, wave-0]

must_haves:
  truths:
    - "All 2 cluster-C6 commits cherry-picked onto main in upstream chronological order"
    - "Every Plan 40-03 commit body carries the verbatim D-19 6-line trailer block (lowercase 'a' in Upstream-author)"
    - "6472011 — new crates/nono/src/scrub.rs module + lib.rs re-export + integration into audit event emission sites"
    - "78114e6 — refactor(scrub): optimize and simplify scrubbing logic"
    - "D-40-E1 invariant: zero edits to *_windows.rs or exec_strategy_windows/ for every commit"
    - "D-40-E6 watch: NO Windows-specific scrub rules added; module lands cross-platform unchanged"
    - "D-40-E4: upstream test fixtures from scrub.rs unit tests ported alongside production code"
    - "All 8 D-40-C2 close-gates pass"
  artifacts:
    - path: "crates/nono/src/scrub.rs"
      provides: "scrub module: scrubs command arguments for secrets in audit events"
      grep_pattern: "pub fn scrub|fn scrub_args|SecretScrubber"
    - path: "crates/nono/src/lib.rs"
      provides: "pub use scrub re-export for downstream consumers"
      grep_pattern: "pub.*use.*scrub|mod scrub"
    - path: "crates/nono-cli/src/audit_integrity.rs"
      provides: "scrub integration at audit event emission sites"
      grep_pattern: "scrub::|nono::scrub"
  key_links:
    - from: "command_runtime.rs audit event emission"
      to: "nono::scrub::scrub_args()"
      via: "lib.rs re-export consumed by nono-cli"
      pattern: "scrub.*command|command.*scrub"
---

<objective>
Cluster C6 (upstream v0.53.0, 2 commits): new nono::scrub module — scrubs command arguments for secrets in audit events + subsequent optimization refactor.

Wave-0 FOUNDATION plan. The new crates/nono/src/scrub.rs module is a lib.rs re-export that downstream clusters may consume (wave-hint: foundation). Run in parallel with Plan 40-02 (Cluster 2 CLI changes) — surfaces are disjoint (C6 creates new crates/nono/src/scrub.rs + touches crates/nono/src/lib.rs; C2 touches crates/nono-cli/src/ only).

Critical: scrub module lands cross-platform unchanged per D-40-E6 (NO Windows-specific scrub rules; if Windows credential path scrubbing is wanted, that is a separate future phase). Port upstream unit test fixtures from scrub.rs alongside production code per D-40-E4.

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
**Cluster C6 cherry-pick chain (2 commits, chronological):**

| Order | SHA | Tag | Subject | Files changed | Upstream Author |
|-------|-----|-----|---------|---------------|-----------------|
| 1 | `6472011` | v0.53.0 | feat(core): scrub command arguments for secrets | 14 | unknown (upstream) |
| 2 | `78114e6` | v0.53.0 | refactor(scrub): optimize and simplify scrubbing logic | 1 | unknown (upstream) |

**D-19 trailer shape (verbatim):**

```
Upstream-commit: <8-char-sha>
Upstream-tag: v0.53.0
Upstream-author: <Name> <<email>>
Co-Authored-By: <Name> <<email>>
Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
```

**Files changed by 6472011 (14 files — large commit; high attention required):**
- NEW: crates/nono/src/scrub.rs (new module with unit tests)
- MODIFIED: crates/nono/src/lib.rs (pub mod scrub re-export)
- MODIFIED: crates/nono-cli/src/audit_integrity.rs
- MODIFIED: crates/nono-cli/src/audit_ledger.rs
- MODIFIED: crates/nono-cli/src/command_runtime.rs
- Additional audit event emission sites (read git show 6472011 for full list)

**D-40-E6 surgical posture — CRITICAL:**
- Do NOT add Windows-specific scrub rules (Windows credential paths, registry-store URIs, etc.)
- The module lands cross-platform unchanged
- If the upstream scrub.rs has a #[cfg(target_os = "windows")] arm → accept it if already upstream, do NOT add new ones

**D-40-E4 — test fixtures:**
- Upstream's scrub.rs unit tests MUST be ported as part of 6472011 cherry-pick
- Do NOT strip test fixtures from the module
- Windows-specific extension tests atop ported fixtures (if any) go behind #[cfg(target_os = "windows")] — but D-40-E6 says no Windows-specific scrub rules, so likely no Windows test additions

**Phase 23 REQ-AUD-05 compatibility:**
- The fork's audit_integrity.rs + audit_ledger.rs consume the scrub module byte-identically per DIVERGENCE-LEDGER rationale
- Post-cherry-pick: `cargo test -p nono -- scrub` must pass (covers the ported unit tests)
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Pre-flight — verify baseline + fetch upstream tags</name>
  <files>(git operations only)</files>
  <read_first>
    - .planning/phases/40-upst4-sync-execution/40-CONTEXT.md § D-40-A2 (Wave 0 parallel), D-40-C4, D-40-E1, D-40-E4, D-40-E6
    - .planning/phases/39-upst4-audit/DIVERGENCE-LEDGER.md § Cluster C6 row
    - .planning/templates/upstream-sync-quick.md § D-19 trailer block
  </read_first>
  <action>
    1. Fetch upstream tags:
       ```bash
       git fetch upstream --tags
       git cat-file -t 6472011   # Expected: commit
       git cat-file -t 78114e6   # Expected: commit
       ```
    2. Verify crates/nono/src/scrub.rs does NOT yet exist (new file from upstream):
       ```bash
       test -f crates/nono/src/scrub.rs && echo "ALREADY EXISTS - investigate" || echo "Not present - expected"
       ```
    3. Record Windows-only sentinel SHA:
       ```bash
       git log -1 --format='%H' -- crates/nono-cli/src/exec_strategy_windows/
       ```
    4. Baseline build:
       ```bash
       cargo build --workspace
       ```
    5. Zero unfilled placeholders:
       ```bash
       grep -oE '\{[a-z_]+\}' .planning/phases/40-upst4-sync-execution/40-03-SCRUB-MODULE-PLAN.md | wc -l   # Expected: 0
       ```
  </action>
  <verify>
    <automated>git fetch upstream --tags &amp;&amp; git cat-file -t 6472011 &amp;&amp; git cat-file -t 78114e6 &amp;&amp; cargo build --workspace</automated>
  </verify>
  <acceptance_criteria>
    - Both C6 SHAs reachable; crates/nono/src/scrub.rs absent (new file); cargo build exits 0; Windows-only sentinel SHA recorded.
  </acceptance_criteria>
  <done>
    Ready for C6 cherry-pick chain.
  </done>
</task>

<task type="auto">
  <name>Task 2: Cherry-pick both C6 commits with D-19 trailers</name>
  <files>
    crates/nono/src/scrub.rs
    crates/nono/src/lib.rs
    crates/nono-cli/src/audit_integrity.rs
    crates/nono-cli/src/audit_ledger.rs
    crates/nono-cli/src/command_runtime.rs
  </files>
  <read_first>
    - crates/nono/src/lib.rs (read current pub exports — know where pub mod scrub will be inserted)
    - crates/nono-cli/src/audit_integrity.rs (read current structure — where scrub call sites will appear)
    - crates/nono-cli/src/audit_ledger.rs (read current structure)
    - git show 6472011 (read the FULL upstream diff — 14 files — before cherry-picking)
    - git show 78114e6 (read the refactor diff)
  </read_first>
  <action>
    **Before cherry-picking:** Run `git show 6472011` and read the complete diff. The commit touches 14 files. Understand:
    - The shape of scrub.rs (public API surface)
    - Which audit_integrity.rs / audit_ledger.rs / command_runtime.rs call sites are added
    - Whether upstream test fixtures are present in scrub.rs (they MUST be ported per D-40-E4)
    - Whether any #[cfg(windows)] arms exist (accept if upstream; do not add new per D-40-E6)

    **Commit 1/2: 6472011 (feat(core): scrub command arguments for secrets)**

    ```bash
    git show 6472011   # Read full 14-file diff first
    git cherry-pick 6472011
    # If conflicts in audit_integrity.rs or audit_ledger.rs:
    # - Preserve existing Phase 23 AUD-05 AIPC ledger logic
    # - Add scrub call sites around them, not replacing them
    # - NEVER accept upstream removal of AIPC Windows ledger emission paths
    cargo build --workspace
    # Run scrub unit tests immediately:
    cargo test -p nono -- scrub
    # D-40-E1 check (MUST be zero):
    git diff --stat HEAD~1 HEAD -- crates/ | grep -E '_windows|exec_strategy_windows' | wc -l
    # D-40-E6 verify: no Windows-specific scrub rules added
    grep -r 'cfg.*windows.*scrub\|scrub.*cfg.*windows' crates/nono/src/scrub.rs || echo "OK - no Windows-gated scrub rules"
    # Amend with D-19 trailer:
    git commit --amend --no-edit
    # Append (ONE blank line before trailer block):
    # Upstream-commit: 6472011
    # Upstream-tag: v0.53.0
    # Upstream-author: <Name from git show 6472011> <<email>>
    # Co-Authored-By: <Name from git show 6472011> <<email>>
    # Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    # Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
    ```

    **Commit 2/2: 78114e6 (refactor(scrub): optimize and simplify scrubbing logic)**

    ```bash
    git show 78114e6   # Read refactor diff (1 file — scrub.rs only)
    git cherry-pick 78114e6
    cargo build --workspace
    cargo test -p nono -- scrub   # Verify refactor doesn't break tests
    git diff --stat HEAD~1 HEAD -- crates/ | grep -E '_windows|exec_strategy_windows' | wc -l   # MUST be 0
    git commit --amend --no-edit
    # Append trailer:
    # Upstream-commit: 78114e6
    # Upstream-tag: v0.53.0
    # Upstream-author: <Name from git show 78114e6> <<email>>
    # Co-Authored-By: <Name from git show 78114e6> <<email>>
    # Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    # Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
    ```

    **Post-chain smoke checks:**
    ```bash
    git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-commit: '   # MUST be 2
    git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-author: '   # MUST be 2 (lowercase 'a')
    git log --format='%B' HEAD~2..HEAD | grep -c '^Signed-off-by: '     # MUST be 4
    git log -1 --format='%H' -- crates/nono-cli/src/exec_strategy_windows/
    # MUST equal PRE_PLAN_WINDOWS_SHA from Task 1
    # Verify scrub module is now exported from lib.rs:
    grep -E 'pub.*scrub|pub use.*scrub' crates/nono/src/lib.rs   # Must return at least 1 line
    # Verify unit tests ported (D-40-E4):
    grep -c '#\[test\]' crates/nono/src/scrub.rs   # Must be >= 1
    ```
  </action>
  <verify>
    <automated>git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-commit: ' | grep -E '^2$' &amp;&amp; cargo test -p nono -- scrub &amp;&amp; cargo build --workspace</automated>
  </verify>
  <acceptance_criteria>
    - 2 commits on main; each with verbatim D-19 6-line trailer (lowercase 'a').
    - crates/nono/src/scrub.rs exists and is exported from lib.rs.
    - `cargo test -p nono -- scrub` exits 0 (unit tests from upstream ported per D-40-E4).
    - Per-commit D-40-E1: `git diff --stat HEAD~N HEAD~N+1 -- crates/ | grep -E '_windows|exec_strategy_windows'` empty for N=1,2.
    - Windows-only sentinel SHA unchanged.
    - `grep '#\[test\]' crates/nono/src/scrub.rs | wc -l` >= 1 (test fixtures ported).
    - No Windows-specific scrub rules added (`grep -r 'cfg.*windows' crates/nono/src/scrub.rs` returns empty OR only pre-existing upstream lines).
    - `cargo build --workspace` exits 0.
  </acceptance_criteria>
  <done>
    C6 chain complete; nono::scrub module exported; audit event scrubbing active; test fixtures ported.
  </done>
</task>

<task type="auto">
  <name>Task 3: D-40-C2 8-check close gate</name>
  <files>(read-only verification)</files>
  <read_first>
    - .planning/phases/40-upst4-sync-execution/40-CONTEXT.md § D-40-C2
  </read_first>
  <action>
    Run all 8 D-40-C2 close-gates. Stop and freeze if any non-skippable gate fails:

    ```bash
    # Gate 1
    cargo test --workspace --all-features

    # Gate 2
    cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used

    # Gate 3 (cross-target Linux — catches scrub module lint in cfg-gated Linux code)
    cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used

    # Gate 4 (cross-target macOS)
    cargo clippy --workspace --all-targets --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used

    # Gate 5
    cargo fmt --all -- --check

    # Gate 6: Phase 15 detached-console smoke (nono run --detached → ps → attach → detach → stop)
    # Document: PASS or documented-skipped with reason

    # Gate 7: wfp_port_integration
    cargo test --workspace --all-features -p nono-cli -- wfp_port_integration
    # Document: PASS or skipped with admin/service reason

    # Gate 8: learn_windows_integration
    cargo test --workspace --all-features -p nono-cli -- learn_windows_integration
    # Document: PASS or skipped
    ```
  </action>
  <verify>
    <automated>cargo test --workspace --all-features &amp;&amp; cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used &amp;&amp; cargo fmt --all -- --check</automated>
  </verify>
  <acceptance_criteria>
    - Gates 1+2+5: PASS.
    - Gates 3+4+6+7+8: PASS or documented-skipped per D-40-C2 policy.
    - `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-commit: '` returns 2.
    - `git diff --stat HEAD~2 HEAD -- crates/ | grep -E '_windows|exec_strategy_windows' | wc -l` returns 0.
  </acceptance_criteria>
  <done>
    Plan 40-03 close-gate cleared.
  </done>
</task>

<task type="auto">
  <name>Task 4: Push + PR</name>
  <files>(git push only)</files>
  <read_first>
    - .planning/phases/40-upst4-sync-execution/40-CONTEXT.md § D-40-C1
  </read_first>
  <action>
    ```bash
    git push origin main
    gh pr create \
      --title "Plan 40-03 (C6): nono::scrub module — secret scrubbing for audit events (v0.53.0, 2 commits)" \
      --body "$(cat <<'EOF'
    Wave 0 foundation plan. Absorbs upstream v0.53.0 Cluster C6 (2 commits):

    - 6472011: new crates/nono/src/scrub.rs + lib.rs re-export + audit event integration (14 files)
    - 78114e6: refactor scrub module (optimize + simplify)

    D-19 trailers: 2/2. D-40-E1 violations: 0. D-40-C2 gates: all pass.
    Unit tests ported (D-40-E4). No Windows-specific scrub rules (D-40-E6).
    Part of Phase 40 UPST4 sync execution (REQ-UPST4-02).
    EOF
    )"
    ```
  </action>
  <verify>
    <automated>git fetch origin &amp;&amp; test "$(git log origin/main..main --oneline 2>/dev/null | wc -l)" = "0"</automated>
  </verify>
  <acceptance_criteria>
    - Pushed; PR created; origin/main updated.
  </acceptance_criteria>
  <done>
    Plan 40-03 published; Wave 0 C6 PR open.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Sandboxed agent command arguments → audit event emission | Secrets in argv that could leak into audit log without scrubbing |
| Upstream diff → fork integration | 14-file commit may silently remove Phase 23 AUD-05 AIPC ledger emission paths |
| Upstream scrub module → fork-only audit_integrity.rs | New scrub call sites may conflict with fork-only Windows AIPC wiring |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation |
|-----------|----------|-----------|----------|-------------|------------|
| T-40-03-01 | Tampering | D-40-E1 Windows-only files invariant | **HIGH** | mitigate (BLOCKING) | Per-commit grep gate; Windows-only sentinel SHA unchanged from pre-plan baseline |
| T-40-03-02 | Repudiation | D-19 trailer missing | **HIGH** | mitigate (BLOCKING) | Post-chain `grep -c '^Upstream-commit: '` must equal 2 |
| T-40-03-03 | Information Disclosure | Incomplete secret scrubbing — scrub.rs misses secret patterns | **HIGH** | mitigate | Port upstream unit tests verbatim (D-40-E4); `cargo test -p nono -- scrub` must pass; do not strip test fixtures |
| T-40-03-04 | Tampering | 6472011 cherry-pick removes Phase 23 AUD-05 AIPC Windows ledger emission logic | **HIGH** | mitigate (BLOCKING) | Read `git show 6472011` BEFORE cherry-pick; verify audit_integrity.rs AIPC emission paths present after cherry-pick |
| T-40-03-05 | Elevation of Privilege | Windows-specific scrub rules inadvertently expose Windows credential path patterns | medium | mitigate | D-40-E6: no Windows-specific scrub rules; post-cherry-pick grep confirms no new cfg-gated Windows code in scrub.rs |
| T-40-03-06 | Information Disclosure | Audit log leaks scrubbing failure (scrub errors silently swallowed) | medium | accept | Upstream scrub design handles this; fork inherits byte-identically; out of Phase 40 scope |
</threat_model>

<verification>
- All 8 D-40-C2 close-gates pass.
- `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-commit: '` returns 2.
- `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-author: '` returns 2 (lowercase 'a').
- `git diff --stat HEAD~2 HEAD -- crates/ | grep -E '_windows|exec_strategy_windows' | wc -l` returns 0.
- `test -f crates/nono/src/scrub.rs` passes.
- `grep -E 'pub.*scrub|pub use.*scrub' crates/nono/src/lib.rs` returns at least 1 line.
- `cargo test -p nono -- scrub` exits 0.
- `grep '#\[test\]' crates/nono/src/scrub.rs | wc -l` >= 1 (test fixtures ported per D-40-E4).
- origin/main advanced; PR created.
</verification>

<success_criteria>
- 2 atomic commits on main, each with verbatim D-19 6-line trailer (lowercase 'a').
- crates/nono/src/scrub.rs created (new module) and exported from lib.rs.
- Scrub module integrated at audit event emission sites.
- Unit tests ported from upstream (D-40-E4 compliance).
- No Windows-specific scrub rules (D-40-E6 compliance).
- Wave-0 foundation surface (nono::scrub re-export) ready for downstream cluster consumption.
- All 8 D-40-C2 gates cleared.
- origin/main advanced; PR open.
</success_criteria>

<output>
After completion, create `.planning/phases/40-upst4-sync-execution/40-03-SCRUB-MODULE-SUMMARY.md`.
</output>
