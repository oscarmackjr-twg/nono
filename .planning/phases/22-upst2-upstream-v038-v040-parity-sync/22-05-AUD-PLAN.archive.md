---
phase: 22-upst2-upstream-v038-v040-parity-sync
plan: 05
type: execute
wave: 2
depends_on: ["22-03", "22-04"]
blocks: []
files_modified:
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/audit/mod.rs
  - crates/nono-cli/src/audit/integrity.rs
  - crates/nono-cli/src/audit/attestation.rs
  - crates/nono-cli/src/audit/exec_identity.rs
  - crates/nono-cli/src/audit/exec_identity_windows.rs
  - crates/nono-cli/src/rollback_runtime.rs
  - crates/nono-cli/src/supervised_runtime.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/session_commands.rs
  - crates/nono-cli/src/session_commands_windows.rs
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/tests/audit_attestation.rs
  - crates/nono-cli/tests/exec_identity_windows.rs
  - MANUAL_TEST_STEPS.md
autonomous: false
requirements: ["AUD-01", "AUD-02", "AUD-03", "AUD-04"]

must_haves:
  truths:
    - "`nono run --audit-integrity --audit-sign-key <ref> -- <cmd>` produces a session with populated `chain_head`, `merkle_root`, and `audit-attestation.bundle` (AUD-01 + AUD-02)"
    - "`nono audit verify <id>` succeeds for an integrity-protected session; tampered ledger rejects fail-closed (AUD-02)"
    - "Windows exec-identity records via `GetModuleFileNameW` for path + `WinVerifyTrust`/`CryptCATAdmin` for Authenticode signature; SHA-256 fallback when unsigned (AUD-03)"
    - "Authenticode is a SEPARATE fork-only commit AFTER cherry-picking 02ee0bd1/7b7815f7 (RESEARCH finding #2 — upstream is SHA-256-only)"
    - "`nono session cleanup` (renamed from `nono prune`) preserves all v2.1 CLEAN-04 invariants: auto_prune_is_noop_when_sandboxed, parse_duration require-suffix, --all-exited escape hatch, AUTO_PRUNE_STALE_THRESHOLD = 100"
    - "`nono audit cleanup` peer subcommand operates on audit ledgers (AUD-04)"
    - "`nono prune` hidden alias still works and surfaces a deprecation note (AUD-04 acceptance #3)"
    - "After EVERY commit touching prune/cleanup code: re-run CLEAN-04 invariant suite (D-04)"
    - "Every cherry-pick commit body contains D-19 trailers; manual-port commits use D-20 template"
    - "`cargo test --workspace --all-features` exits 0 on Windows after each commit (D-18)"
    - "Phase 21 `AppliedLabelsGuard` lifecycle preserved; ledger flush completes before guard Drop"
  artifacts:
    - path: "crates/nono-cli/src/audit/integrity.rs"
      provides: "Hash-chain + Merkle root tamper-evident ledger (NEW or extended)"
    - path: "crates/nono-cli/src/audit/attestation.rs"
      provides: "DSSE/in-toto attestation signing via existing nono::trust::signing (NEW)"
    - path: "crates/nono-cli/src/audit/exec_identity_windows.rs"
      provides: "Windows Authenticode + SHA-256 fallback exec-identity recording (NEW, fork-only ~150 LOC per RESEARCH finding #2)"
    - path: "crates/nono-cli/tests/audit_attestation.rs"
      provides: "Audit attestation sign/verify integration tests (ported from upstream 9db06336, +188 LOC)"
    - path: "crates/nono-cli/tests/exec_identity_windows.rs"
      provides: "Authenticode + SHA-256 fallback Windows-only tests (NEW fork-only)"
  key_links:
    - from: "ROADMAP § Phase 22 success criterion #5"
      to: "nono audit verify success + CLEAN-04 invariants survive"
      via: "audit subsystem + session cleanup rename"
      pattern: "audit_integrity|chain_head|merkle_root|audit_attestation"
---

<objective>
Land upstream audit-integrity + attestation cluster (AUD-01..04) into the fork: 7 chronologically-ordered upstream commits totaling ~1.4k LOC across heavily-forked files (`rollback_runtime.rs` +586 upstream / fork has v2.1 AppliedLabelsGuard; `supervised_runtime.rs` +42 upstream / fork has v2.1 loaded_profile; `exec_strategy.rs` +144 upstream / fork has v2.0 Direct/Monitor/Supervised + AIPC). PLUS one fork-only commit adding Windows Authenticode exec-identity recording (RESEARCH finding #2: upstream's `02ee0bd1`/`7b7815f7` ship SHA-256-only `ExecutableIdentity`; Authenticode is a fork-only addition AFTER those cherry-picks).

`autonomous: false` — Plan 22-05 has the highest D-02 manual-port probability of any plan in Phase 22. The audit-cluster minefield (`exec_strategy.rs`, `supervised_runtime.rs`, `rollback_runtime.rs`) requires read-upstream-and-replay per CONTEXT § Known Risks. Each `prune`/`cleanup` rename commit must run the full CLEAN-04 invariant suite per D-04 BEFORE commit.

Purpose: A Windows user runs `nono run --audit-integrity --audit-sign-key keyring://nono/audit -- <cmd>` and the resulting session has a populated `chain_head`, `merkle_root`, and `audit-attestation.bundle`; `nono audit verify <id>` succeeds; v2.1 CLEAN-04 invariants survive the `prune` → `session cleanup` rename (REQ-AUD-01..04).
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
@.planning/phases/19-cleanup/19-CONTEXT.md
@crates/nono-cli/src/cli.rs
@crates/nono-cli/src/session_commands.rs
@crates/nono-cli/src/session_commands_windows.rs
@crates/nono-cli/src/exec_strategy.rs
@crates/nono-cli/src/supervised_runtime.rs
@crates/nono-cli/src/rollback_runtime.rs
@crates/nono/src/undo/merkle.rs

<interfaces>
**Upstream cherry-pick chain (STRICT chronological per D-03):**

| Order | SHA | Upstream subject | REQ |
|-------|-----|------------------|-----|
| 1 | `4f9552ec` | feat(audit): add tamper-evident audit log integrity (~1,419+/226− across 21 files; --audit-integrity flag, hash-chain + Merkle root, prune→session cleanup rename, audit cleanup peer) | AUD-01 + AUD-04 |
| 2 | `4ec61c29` | feat(audit): capture pre/post merkle roots | AUD-01 |
| 3 | `02ee0bd1` | feat(audit): record executable identity (SHA-256 only upstream) | AUD-03 (SHA-256 portion) |
| 4 | `7b7815f7` | feat(audit): record exec identity and unify audit integrity | AUD-03 (SHA-256 portion) |
| 5 | `0b1822a9` | feat(audit): add audit verify command | AUD-02 |
| 6 | `6ecade2e` | feat(audit): add audit attestation | AUD-02 |
| 7 | `9db06336` | feat(audit): refine audit path derivation (re-introduces MANUAL_TEST_STEPS.md, +188 LOC tests/audit_attestation.rs) | AUD-01..04 |

**Fork-only addition (RESEARCH finding #2):**
- `feat(22-05): add Windows Authenticode exec-identity recording (fork-only)` — ~150 LOC NEW `crates/nono-cli/src/audit/exec_identity_windows.rs`. Lands AFTER 9db06336 so it builds on the unified Alpha scheme. D-17 ALLOWED here per CONTEXT line 248 — fork's planned Windows-internal addition.

**RESEARCH-CRITICAL ordering decisions:**

1. **02ee0bd1 / 7b7815f7 are SHA-256-only.** Upstream's `ExecutableIdentity` struct contains only SHA-256 hash. Authenticode (~150 LOC: `WinVerifyTrust`, `CryptCATAdminAcquireContext`, signer subject parsing) is fork-only. Land it as a SEPARATE commit (Task 5b) AFTER both upstream cherry-picks. CONTEXT § Integration Points line 248 already says "D-17 ALLOWED here (this is the fork's planned addition, not an upstream commit touching `*_windows.rs`)".

2. **9db06336 re-introduces MANUAL_TEST_STEPS.md** which `5c301e8d` (Plan 22-02) deleted. RESEARCH open question #4 verified intentional. Cherry-pick will surface this re-add — accept it.

3. **windows-sys feature flag.** RESEARCH finding #8: fork already has `windows-sys 0.59`; only the `Win32_Security_WinTrust` feature flag must be added to `crates/nono-cli/Cargo.toml` for the Authenticode integration (Task 5b).

**HIGH-CONFLICT FILES per CONTEXT § Files at HIGH merge-conflict risk + RESEARCH baselines:**

| File | Upstream delta | Fork drift | D-02 prediction |
|------|---------------|-----------|-----------------|
| `crates/nono-cli/src/rollback_runtime.rs` | +586 | v2.1 AppliedLabelsGuard snapshot+label lifecycle | D-20 manual-port LIKELY |
| `crates/nono-cli/src/supervised_runtime.rs` | +42 | v2.1 SupervisedRuntimeContext.loaded_profile + AIPC allowlist | D-20 manual-port POSSIBLE |
| `crates/nono-cli/src/exec_strategy.rs` | +144 | v2.0 Direct/Monitor/Supervised branching + AIPC wiring | D-20 manual-port LIKELY |
| `crates/nono/src/undo/snapshot.rs` | +149 (in 4f9552ec) | ObjectStore clone_or_copy + Merkle wiring on Windows | D-20 manual-port POSSIBLE |
| `crates/nono/src/undo/types.rs` | (in 4f9552ec) | (same — fork ObjectStore additions) | review per-commit |

**CLEAN-04 invariant tests located (RESEARCH finding #7):**
- `auto_prune_is_noop_when_sandboxed` — `crates/nono-cli/src/session_commands.rs:708` AND `session_commands_windows.rs:801`
- `is_prunable_all_exited_escape_hatch_matches_any_exited` — `crates/nono-cli/src/session.rs:1373`
- `parse_duration_*` family — `crates/nono-cli/src/cli.rs:2454-2472`
- `AUTO_PRUNE_STALE_THRESHOLD: usize = 100` constant — `crates/nono-cli/src/session_commands.rs:32`
- NO test named `older_than_requires_suffix` exists — CLEAN-04 covers via `parse_duration` + clap value_parser

**D-04 invariant gate command (run after every prune/cleanup-touching commit):**
```
cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited
cargo test -p nono-cli cli::parse_duration_
grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs
```
If any fails: STOP per CONTEXT STOP trigger #5 ("CLEAN-04 invariant test fails after a Plan 22-05 commit → revert that commit and re-discuss"). STOP trigger #6: `auto_prune_is_noop_when_sandboxed` test failure post-rename = ABSOLUTE STOP (sandboxed-agent file-deletion vector reopened).

**Reusable fork assets (per CONTEXT § Reusable Assets):**
- `nono::keystore::load_secret` (cross-platform) for `--audit-sign-key keyring://nono/audit`
- `nono::trust::signing::sign_statement_bundle` + `public_key_id_hex` for DSSE attestation (already shipped v2.1)
- `nono::undo::merkle::MerkleTree` (already used in fork's snapshot system) — reuse for AUD-01 ledger Merkle root
- `current_logon_sid()` + `build_capability_pipe_sddl` (commit `938887f`, 2026-04-20) — may be needed for write-path SDDL on Windows ledger, verify during planning
- `windows-sys 0.59` already in fork — only `Win32_Security_WinTrust` feature flag missing

**D-19 commit body template (cherry-pick path):** Same as prior plans.
**D-20 commit body template (manual-port fallback):** Same as prior plans, with explicit replay rationale.

**STOP triggers from CONTEXT § Specifics that fire in this plan:**
- #1: Touch `*_windows.rs` outside audit/exec_identity_windows.rs → ABORT
- #2: `make ci` red, root cause unclear in 30 min → STOP
- #3: Manual port diff exceeds ~400 lines → consider splitting plan (per CONTEXT, "Plan 22-05 is the most likely candidate")
- #4: Phase 15 5-row smoke gate fails → REVERT and re-scope
- #5: CLEAN-04 invariant test fails → REVERT that commit and re-discuss
- #6 (ABSOLUTE STOP): `auto_prune_is_noop_when_sandboxed` post-rename failure
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: D-04 baseline — capture CLEAN-04 invariant pre-state</name>
  <files>(read-only audit — no files modified)</files>
  <read_first>
    - .planning/phases/19-cleanup/19-CONTEXT.md (CLEAN-04 invariant origin)
    - crates/nono-cli/src/session_commands.rs (auto_prune_is_noop_when_sandboxed test, AUTO_PRUNE_STALE_THRESHOLD constant)
    - crates/nono-cli/src/session.rs (is_prunable_all_exited_escape_hatch_matches_any_exited test)
    - crates/nono-cli/src/cli.rs (parse_duration tests at lines 2454-2472)
    - crates/nono-cli/src/session_commands_windows.rs (Windows duplicate of auto_prune_is_noop)
  </read_first>
  <action>
    1. Run the full CLEAN-04 invariant suite and capture baseline state (must be all green pre-22-05):
       ```
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed 2>&1 | tee /tmp/22-05-baseline-cleanup.log
       cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited 2>&1 | tee -a /tmp/22-05-baseline-cleanup.log
       cargo test -p nono-cli cli::parse_duration_ 2>&1 | tee -a /tmp/22-05-baseline-cleanup.log
       ```

    2. Verify the structural invariants:
       ```
       grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs
       grep -nE 'auto_prune_is_noop_when_sandboxed' crates/nono-cli/src/session_commands.rs crates/nono-cli/src/session_commands_windows.rs
       ```
       Both files must contain the test (line 708 / line 801 per RESEARCH).

    3. If ANY CLEAN-04 invariant fails baseline: STOP. Cannot start Plan 22-05 with a broken baseline (rename plan would mask the failure).

    4. Verify Phase 21 `AppliedLabelsGuard` lifecycle still green:
       ```
       cargo test -p nono --test sandbox_windows applied_labels_guard:: 2>&1 | tee /tmp/22-05-baseline-aipc.log
       ```

    5. Record baseline pass counts in preflight note (for SUMMARY).
  </action>
  <verify>
    <automated>cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed &amp;&amp; cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited &amp;&amp; cargo test -p nono-cli cli::parse_duration_ &amp;&amp; grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs</automated>
  </verify>
  <acceptance_criteria>
    - All 3 CLEAN-04 invariant test groups exit 0.
    - `AUTO_PRUNE_STALE_THRESHOLD = 100` constant present.
    - `auto_prune_is_noop_when_sandboxed` test exists in BOTH session_commands.rs and session_commands_windows.rs.
    - Phase 21 AppliedLabelsGuard tests green.
    - Baseline counts recorded in preflight note.
  </acceptance_criteria>
  <done>
    Baseline CLEAN-04 invariant suite green; Plan 22-05 may proceed.
  </done>
</task>

<task type="auto">
  <name>Task 2: Cherry-pick `4f9552ec` — tamper-evident audit log integrity (AUD-01 + AUD-04 bulk) — D-02 manual-port LIKELY</name>
  <files>
    crates/nono-cli/src/cli.rs
    crates/nono-cli/src/audit/mod.rs (NEW or extended)
    crates/nono-cli/src/audit/integrity.rs (NEW)
    crates/nono-cli/src/rollback_runtime.rs
    crates/nono-cli/src/supervised_runtime.rs
    crates/nono-cli/src/exec_strategy.rs
    crates/nono-cli/src/session_commands.rs
    crates/nono-cli/src/session_commands_windows.rs
    crates/nono/src/undo/snapshot.rs
    crates/nono/src/undo/types.rs
  </files>
  <read_first>
    - `git show 4f9552ec --stat` (read full file list — 21 files / +1419 / -226)
    - `git show 4f9552ec -- crates/nono-cli/src/rollback_runtime.rs` (anticipate +586 conflict; fork has AppliedLabelsGuard)
    - `git show 4f9552ec -- crates/nono-cli/src/supervised_runtime.rs` (fork has loaded_profile + AIPC)
    - `git show 4f9552ec -- crates/nono-cli/src/exec_strategy.rs` (fork has Direct/Monitor/Supervised + AIPC)
    - `git show 4f9552ec -- crates/nono-cli/src/session_commands.rs` (prune→session cleanup rename)
    - `git show 4f9552ec -- crates/nono/src/undo/snapshot.rs` crates/nono/src/undo/types.rs (fork has ObjectStore clone_or_copy + Merkle wiring on Windows)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § Known Risks "exec_strategy.rs + supervised_runtime.rs + rollback_runtime.rs are the audit-cluster minefield"
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 4f9552ec
       ```

    2. **D-02 manual-port gate (HIGH PROBABILITY).** Almost certainly conflicts on rollback_runtime.rs / supervised_runtime.rs / exec_strategy.rs.
       ```
       git diff --name-only --diff-filter=U
       for f in $(git diff --name-only --diff-filter=U); do
         echo "=== $f ==="
         grep -c '<<<<<<<' "$f"
       done
       ```

    3. **If conflicts exceed D-02 thresholds (>50 lines per file OR >2 forked files OR semantic ambiguity — almost certainly the case here):**
       - `git cherry-pick --abort`
       - Apply D-20 manual-port template. Read upstream's diff per file:
         ```
         git show 4f9552ec -- <file>
         ```
       - Replay upstream's logic on top of fork's current state:
         - `cli.rs`: add `--audit-integrity` flag (clap `bool` arg) + `--audit-sign-key` arg (`Option<String>` with `keyring://` URI value parser)
         - `audit/mod.rs` + `audit/integrity.rs` (NEW): hash-chain + Merkle root logic. Reuse `nono::undo::merkle::MerkleTree`. `pub fn append_event(&mut self, event: AuditEvent) -> Result<ChainHead>`
         - `rollback_runtime.rs`: thread `audit_integrity` flag through; emit ledger events at decision points. PRESERVE fork's `AppliedLabelsGuard` snapshot+label lifecycle.
         - `supervised_runtime.rs`: thread audit context. PRESERVE fork's `loaded_profile` + AIPC allowlist threading.
         - `exec_strategy.rs`: emit ledger events in Direct/Monitor/Supervised paths. PRESERVE fork's strategy branching.
         - `session_commands.rs`: rename `prune` function family to `cleanup`; preserve all CLEAN-04 invariants (`auto_prune_is_noop_when_sandboxed`, `AUTO_PRUNE_STALE_THRESHOLD`, etc.).
         - `session_commands_windows.rs`: rename `prune` to `cleanup`; preserve Windows-specific guard logic.
         - `undo/snapshot.rs` + `undo/types.rs`: preserve fork's ObjectStore clone_or_copy + Merkle wiring; add upstream's audit-integrity Merkle hooks.

    4. **D-04 invariant gate (BEFORE commit).** Run the full CLEAN-04 invariant suite per Task 1 commands. If ANY fails: REVERT all changes (`git reset --hard <pre-task-2-SHA>`) and re-discuss.

       ABSOLUTE STOP if `auto_prune_is_noop_when_sandboxed` fails — sandboxed-agent file-deletion vector reopened.

    5. **D-04 spot check on the rename:** Confirm ALL `auto_prune_*` test names also exist in renamed form:
       ```
       grep -nE 'fn (auto_prune|auto_cleanup|session_cleanup_is_noop)_' crates/nono-cli/src/session_commands.rs crates/nono-cli/src/session_commands_windows.rs
       ```
       If upstream renamed test functions, the renamed forms must exist. If upstream removed tests, fork-only follow-up restores them.

    6. Commit (D-19 if cherry-pick succeeded; D-20 with manual-replay annotation if abort+replay used):
       ```
       git commit -s -m "$(cat <<'EOF'
       feat(22-05): add tamper-evident audit log integrity (AUD-01, AUD-04)

       Manual-port replay over heavily-forked rollback_runtime.rs (+586 upstream
       vs fork AppliedLabelsGuard), supervised_runtime.rs (fork loaded_profile),
       and exec_strategy.rs (fork Direct/Monitor/Supervised + AIPC). Cherry-pick
       aborted at <count> conflict markers across <file-list>; replayed
       semantically.

       Adds --audit-integrity / --audit-sign-key flags. Hash-chain + Merkle root
       via nono::undo::merkle::MerkleTree. Renames `nono prune` → `nono session
       cleanup` (CLEAN-04 invariants preserved per D-04 gate). Adds `nono audit
       cleanup` peer.

       Upstream-commit: 4f9552ec (replayed manually)
       Upstream-tag: v0.40.0
       Upstream-author: <capture from `git log -1 4f9552ec --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    7. Verify build:
       ```
       cargo build --workspace
       cargo test --workspace --lib
       ```
       NEW failures = STOP per STOP trigger #2.
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed &amp;&amp; grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs &amp;&amp; grep -E '\-\-audit-integrity|audit_integrity' crates/nono-cli/src/cli.rs &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 4f9552ec'</automated>
  </verify>
  <acceptance_criteria>
    - `--audit-integrity` flag exists in cli.rs.
    - `nono session cleanup` subcommand exists.
    - `nono audit cleanup` subcommand exists (peer).
    - `nono prune` hidden alias still works (verify with `cargo run -p nono-cli -- prune --help` or `nono prune --help`).
    - **CLEAN-04 invariants ALL GREEN:** `auto_prune_is_noop_when_sandboxed`, `is_prunable_all_exited_escape_hatch_matches_any_exited`, `parse_duration_*` family.
    - `AUTO_PRUNE_STALE_THRESHOLD = 100` constant unchanged.
    - Phase 21 AppliedLabelsGuard tests still green.
    - `git log -1 --format=%B | grep '^Upstream-commit: 4f9552ec'` returns 1 line.
    - `cargo build --workspace` exits 0.
  </acceptance_criteria>
  <done>
    AUD-01 (hash-chain + Merkle root) + AUD-04 (rename) bulk landed. CLEAN-04 invariants survive. Fork's v2.1 AppliedLabelsGuard / loaded_profile / Direct-Monitor-Supervised + AIPC threading preserved.
  </done>
</task>

<task type="auto">
  <name>Task 3: Cherry-pick `4ec61c29` — pre/post merkle root capture (AUD-01)</name>
  <files>
    crates/nono-cli/src/audit/integrity.rs
    crates/nono-cli/src/rollback_runtime.rs
  </files>
  <read_first>
    - `git show 4ec61c29 --stat` and full diff
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 4ec61c29
       ```
       D-02 gate.

    2. D-04 gate after commit (rollback touches prune surface).

    3. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-05): capture pre/post merkle roots (AUD-01)

       Records the ledger merkle root BEFORE and AFTER each audit-integrity
       boundary so verification can detect missing events between captures.

       Upstream-commit: 4ec61c29
       Upstream-tag: v0.40.0
       Upstream-author: <capture from `git log -1 4ec61c29 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 4ec61c29'</automated>
  </verify>
  <acceptance_criteria>
    - `cargo build --workspace` exits 0.
    - CLEAN-04 invariants still green.
    - `git log -1 --format=%B | grep '^Upstream-commit: 4ec61c29'` returns 1 line.
  </acceptance_criteria>
  <done>
    Pre/post merkle root capture landed.
  </done>
</task>

<task type="auto">
  <name>Task 4: Cherry-pick `02ee0bd1` — record executable identity (SHA-256 only — AUD-03 partial)</name>
  <files>
    crates/nono-cli/src/audit/integrity.rs
    crates/nono-cli/src/audit/exec_identity.rs (NEW — SHA-256 only)
  </files>
  <read_first>
    - `git show 02ee0bd1 --stat` and full diff
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-RESEARCH.md "Authenticode is fork-only" finding (#2)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 02ee0bd1
       ```
       D-02 gate. Likely lands cleanly (new file or extension in audit module).

    2. **CRITICAL:** This commit ships SHA-256-only `ExecutableIdentity`. Do NOT add Authenticode here — that's Task 5b (separate fork-only commit per RESEARCH finding #2).

    3. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-05): record executable identity (AUD-03 SHA-256 portion)

       Records SHA-256 of the executed binary in the audit ledger. Windows
       Authenticode signature recording lands as a fork-only follow-up
       (audit/exec_identity_windows.rs) AFTER the upstream cluster is in.

       Upstream-commit: 02ee0bd1
       Upstream-tag: v0.40.0
       Upstream-author: <capture from `git log -1 02ee0bd1 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; grep -rE 'pub struct ExecutableIdentity' crates/nono-cli/src/audit/ &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 02ee0bd1'</automated>
  </verify>
  <acceptance_criteria>
    - `ExecutableIdentity` struct exists in fork audit module.
    - SHA-256 hash field present; NO Authenticode field yet (that lands in Task 5b).
    - `cargo build --workspace` exits 0.
    - `git log -1 --format=%B | grep '^Upstream-commit: 02ee0bd1'` returns 1 line.
  </acceptance_criteria>
  <done>
    SHA-256 executable identity portion landed. Authenticode follow-up reserved for Task 5b.
  </done>
</task>

<task type="auto">
  <name>Task 5: Cherry-pick `7b7815f7` — record exec identity and unify audit integrity (AUD-03 SHA-256 portion cont.)</name>
  <files>
    crates/nono-cli/src/audit/integrity.rs
    crates/nono-cli/src/audit/exec_identity.rs
  </files>
  <read_first>
    - `git show 7b7815f7 --stat` and full diff
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 7b7815f7
       ```
       D-02 gate.

    2. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-05): record exec identity and unify audit integrity (AUD-03 SHA-256)

       Unifies the audit-integrity event shape so exec identity + chain head
       + merkle root coexist in a single Alpha schema. Authenticode integration
       lands next as a fork-only follow-up.

       Upstream-commit: 7b7815f7
       Upstream-tag: v0.40.0
       Upstream-author: <capture from `git log -1 7b7815f7 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 7b7815f7'</automated>
  </verify>
  <acceptance_criteria>
    - `cargo build --workspace` exits 0.
    - `git log -1 --format=%B | grep '^Upstream-commit: 7b7815f7'` returns 1 line.
    - CLEAN-04 invariants still green.
  </acceptance_criteria>
  <done>
    Exec identity unified into audit integrity Alpha schema. Ready for fork-only Authenticode.
  </done>
</task>

<task type="auto">
  <name>Task 5b: Add Windows Authenticode exec-identity recording (FORK-ONLY — D-17 ALLOWED)</name>
  <files>
    crates/nono-cli/src/audit/exec_identity_windows.rs (NEW — ~150 LOC)
    crates/nono-cli/src/audit/exec_identity.rs (extend with platform dispatch)
    crates/nono-cli/Cargo.toml (add windows-sys feature flag)
    crates/nono-cli/tests/exec_identity_windows.rs (NEW)
  </files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-RESEARCH.md (Authenticode FFI sketch + windows-sys feature flag finding)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-PATTERNS.md (CONTRADICTION-C: FFI style analog only — `try_set_mandatory_label` in sandbox/windows.rs for encode_wide + unsafe + SAFETY + GetLastError pattern)
    - crates/nono/src/sandbox/windows.rs (FFI style analog: encode_wide, unsafe wrapping, SAFETY docs)
    - crates/nono-cli/Cargo.toml (windows-sys feature list)
    - CLAUDE.md § Unsafe Code ("Restrict to FFI; must be wrapped in safe APIs with `// SAFETY:` docs")
  </read_first>
  <action>
    1. Add the `Win32_Security_WinTrust` feature flag to `crates/nono-cli/Cargo.toml`:
       ```
       # Find the windows-sys line and add the feature
       grep -n 'windows-sys' crates/nono-cli/Cargo.toml
       # Edit: add "Win32_Security_WinTrust" to the features array
       ```

    2. Create `crates/nono-cli/src/audit/exec_identity_windows.rs`:
       ```rust
       //! Windows Authenticode exec-identity recording (REQ-AUD-03 acceptance #2/#3).
       //! Fork-only addition per CONTEXT § Integration Points line 248 (D-17 ALLOWED).
       //! Follows FFI style established in crates/nono/src/sandbox/windows.rs:
       //! encode_wide, unsafe wrapping with SAFETY docs, GetLastError → typed error.

       #![cfg(target_os = "windows")]

       use crate::error::NonoError;
       use std::path::Path;
       use std::os::windows::ffi::OsStrExt;
       use windows_sys::Win32::Foundation::*;
       use windows_sys::Win32::Security::WinTrust::*;
       // ... full impl per RESEARCH cheat-sheet:
       //   - GetModuleFileNameW for path
       //   - WinVerifyTrust with WINTRUST_DATA + WINTRUST_FILE_INFO for signature query
       //   - On signed: parse WTD_STATEACTION_VERIFY result, extract signer subject CN
       //   - On unsigned: SHA-256 fallback (call into existing audit::exec_identity::sha256_of_file)
       //   - // SAFETY: docs on every unsafe block per CLAUDE.md

       pub struct WindowsExecIdentity {
           pub path: std::path::PathBuf,
           pub authenticode: Option<AuthenticodeRecord>,
           pub sha256: [u8; 32],
       }

       pub struct AuthenticodeRecord {
           pub signer_subject: String,
           pub thumbprint: String,
           pub state: AuthenticodeState,
       }

       pub enum AuthenticodeState {
           ValidSigned,
           InvalidSignature,
           Unsigned,
           QueryFailed,
       }

       pub fn record_exec_identity(path: &Path) -> Result<WindowsExecIdentity, NonoError> {
           // Per RESEARCH cheat-sheet implementation
           // ...
       }
       ```

    3. Wire platform dispatch in `audit/exec_identity.rs`:
       ```rust
       #[cfg(target_os = "windows")]
       pub fn platform_exec_identity(path: &Path) -> Result<ExecutableIdentity, NonoError> {
           let win = exec_identity_windows::record_exec_identity(path)?;
           // Convert WindowsExecIdentity → ExecutableIdentity (extending the unified Alpha schema
           // from 7b7815f7 with optional authenticode field)
       }
       ```

    4. Add Windows-only tests in `crates/nono-cli/tests/exec_identity_windows.rs`:
       ```rust
       #![cfg(target_os = "windows")]

       #[test]
       fn authenticode_signed_records_subject() {
           // Use a known-signed system binary (e.g., notepad.exe) for the integration test
           let id = nono_cli::audit::exec_identity_windows::record_exec_identity(
               std::path::Path::new(r"C:\Windows\System32\notepad.exe"),
           ).expect("should query exec identity");
           assert!(id.authenticode.is_some(), "system binary should be Authenticode-signed");
           let auth = id.authenticode.unwrap();
           assert!(matches!(auth.state, AuthenticodeState::ValidSigned));
           assert!(auth.signer_subject.contains("Microsoft"), "expected Microsoft signer subject");
       }

       #[test]
       fn authenticode_unsigned_falls_back_to_sha256() {
           // Create a temp .exe via tempfile that's NOT signed
           let dir = tempfile::tempdir().unwrap();
           let path = dir.path().join("unsigned.exe");
           std::fs::write(&path, b"MZ\x90\x00\x03\x00\x00\x00").unwrap(); // minimal PE-ish
           let id = nono_cli::audit::exec_identity_windows::record_exec_identity(&path)
               .expect("unsigned should still record SHA-256");
           assert!(id.authenticode.is_some(), "even unsigned should record Authenticode state");
           let auth = id.authenticode.unwrap();
           assert!(matches!(auth.state, AuthenticodeState::Unsigned));
           assert_ne!(id.sha256, [0u8; 32], "SHA-256 must be computed for unsigned binary");
       }
       ```

    5. Commit (FORK-ONLY — no Upstream-commit trailer; D-17 ALLOWED:
       ```
       git add crates/nono-cli/src/audit/exec_identity_windows.rs \
               crates/nono-cli/src/audit/exec_identity.rs \
               crates/nono-cli/Cargo.toml \
               crates/nono-cli/tests/exec_identity_windows.rs
       git commit -s -m "$(cat <<'EOF'
       feat(22-05): add Windows Authenticode exec-identity recording (REQ-AUD-03)

       Fork-only addition per CONTEXT § Integration Points line 248 — D-17
       ALLOWED for the audit/exec_identity_windows.rs surface (planned fork
       Windows-internal addition, not an upstream port).

       Implements WinVerifyTrust + CryptCATAdminAcquireContext signature query
       atop the unified Alpha schema landed by 02ee0bd1/7b7815f7. SHA-256
       fallback when unsigned. Adds Win32_Security_WinTrust feature flag to
       windows-sys 0.59 (already in fork). FFI style mirrors
       crates/nono/src/sandbox/windows.rs (encode_wide, unsafe + SAFETY,
       GetLastError → typed error).

       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; test -f crates/nono-cli/src/audit/exec_identity_windows.rs &amp;&amp; grep -E 'Win32_Security_WinTrust' crates/nono-cli/Cargo.toml</automated>
  </verify>
  <acceptance_criteria>
    - `crates/nono-cli/src/audit/exec_identity_windows.rs` exists.
    - `crates/nono-cli/Cargo.toml` includes `Win32_Security_WinTrust` feature.
    - `cargo build --workspace` exits 0 (host-native — Plan 22-05 runs on Windows host per phase posture; no cross-compile target needed).
    - **Windows-host-gated test (manual gate):** On a Windows host, `cargo test -p nono-cli --test exec_identity_windows` must exit 0. On non-Windows hosts the test compiles to nothing (`#[cfg(target_os = "windows")]`) and is documented-skipped — record the skip in SUMMARY with rationale.
    - All `unsafe` blocks have `// SAFETY:` docs per CLAUDE.md.
    - No `.unwrap()` / `.expect()` in production code (clippy::unwrap_used).
    - Commit has Signed-off-by trailer; NO `Upstream-commit:` trailer (fork-only).
  </acceptance_criteria>
  <done>
    Windows Authenticode + SHA-256 fallback exec-identity recording landed. REQ-AUD-03 fully covered.
  </done>
</task>

<task type="auto">
  <name>Task 6: Cherry-pick `0b1822a9` — `nono audit verify` command (AUD-02)</name>
  <files>
    crates/nono-cli/src/cli.rs
    crates/nono-cli/src/audit/mod.rs (or audit/verify.rs NEW)
  </files>
  <read_first>
    - `git show 0b1822a9 --stat` and full diff
    - REQUIREMENTS.md § AUD-02 (acceptance: nono audit verify <id> succeeds; tampered ledger rejects)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 0b1822a9
       ```
       D-02 gate.

    2. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-05): add audit verify command (AUD-02)

       nono audit verify <id> reads the integrity-protected ledger, recomputes
       the hash chain + merkle root, and validates the chain. Tampered ledger
       rejects fail-closed.

       Upstream-commit: 0b1822a9
       Upstream-tag: v0.40.0
       Upstream-author: <capture from `git log -1 0b1822a9 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; cargo run -p nono-cli -- audit verify --help 2&gt;&amp;1 | head &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 0b1822a9'</automated>
  </verify>
  <acceptance_criteria>
    - `cargo run -p nono-cli -- audit verify --help` lists the subcommand without error.
    - `cargo build --workspace` exits 0.
    - `git log -1 --format=%B | grep '^Upstream-commit: 0b1822a9'` returns 1 line.
  </acceptance_criteria>
  <done>
    `nono audit verify` command landed.
  </done>
</task>

<task type="auto">
  <name>Task 7: Cherry-pick `6ecade2e` — audit attestation (AUD-02)</name>
  <files>
    crates/nono-cli/src/audit/attestation.rs (NEW)
    crates/nono-cli/src/audit/mod.rs
  </files>
  <read_first>
    - `git show 6ecade2e --stat` and full diff
    - REQUIREMENTS.md § AUD-02 (DSSE/in-toto attestation)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § Reusable Assets ("`nono::trust::signing::sign_statement_bundle` + `public_key_id_hex` already exists in fork from v2.1")
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 6ecade2e
       ```
       D-02 gate.

    2. Verify integration with fork's existing trust signing:
       ```
       grep -rE 'sign_statement_bundle|public_key_id_hex' crates/nono-cli/src/audit/
       ```
       Reuse fork's existing `nono::trust::signing::sign_statement_bundle` per CONTEXT.

    3. **Audit signing key provisioning model (Claude's Discretion per CONTEXT line 105).** Default to "user pre-provisioning + fail-closed if missing" — if `--audit-sign-key keyring://nono/audit` references a missing key, fail with clear error. Do NOT auto-generate (requires explicit setup). Document in commit body.

    4. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-05): add audit attestation (AUD-02)

       DSSE/in-toto attestation signing via existing nono::trust::signing
       (shipped v2.1). Key resolution via keyring:// URI with fork's
       existing nono::keystore::load_secret. Default provisioning model:
       user pre-provisions key; missing key fails fail-closed (per
       CONTEXT Claude's Discretion bullet).

       Upstream-commit: 6ecade2e
       Upstream-tag: v0.40.0
       Upstream-author: <capture from `git log -1 6ecade2e --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; grep -rE 'sign_statement_bundle' crates/nono-cli/src/audit/ &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 6ecade2e'</automated>
  </verify>
  <acceptance_criteria>
    - `crates/nono-cli/src/audit/attestation.rs` (or equivalent) exists.
    - Reuses `nono::trust::signing::sign_statement_bundle` (no duplicate signing implementation).
    - `cargo build --workspace` exits 0.
    - `git log -1 --format=%B | grep '^Upstream-commit: 6ecade2e'` returns 1 line.
  </acceptance_criteria>
  <done>
    AUD-02 attestation landed.
  </done>
</task>

<task type="auto">
  <name>Task 8: Cherry-pick `9db06336` — refine audit path derivation + port audit_attestation.rs test fixture (188 LOC)</name>
  <files>
    crates/nono-cli/src/audit/mod.rs
    crates/nono-cli/tests/audit_attestation.rs (NEW — ported from upstream, +188 LOC)
    MANUAL_TEST_STEPS.md (re-introduced per RESEARCH open question #4)
  </files>
  <read_first>
    - `git show 9db06336 --stat` and full diff (verify +188 LOC tests/audit_attestation.rs)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-RESEARCH.md (open question #4: 9db06336 re-introduces MANUAL_TEST_STEPS.md after Plan 22-02's 5c301e8d deleted it; verified intentional)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 9db06336
       ```
       D-02 gate.

    2. **MANUAL_TEST_STEPS.md re-introduction.** Plan 22-02's `5c301e8d` deleted this file; `9db06336` re-adds it. Accept the round-trip per RESEARCH finding.

    3. **audit_attestation.rs test fixture port (D-13).** This is the ONE external test fixture file the entire Phase 22 needs to port (per RESEARCH finding #6). Verify it landed:
       ```
       test -f crates/nono-cli/tests/audit_attestation.rs
       wc -l crates/nono-cli/tests/audit_attestation.rs   # expect ~188
       ```

    4. Run the ported test fixture:
       ```
       cargo test -p nono-cli --test audit_attestation
       ```

    5. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-05): refine audit path derivation + port attestation test fixture

       Refines audit ledger path derivation (consistent across platforms);
       re-introduces MANUAL_TEST_STEPS.md (intentional round-trip vs Plan
       22-02 5c301e8d deletion); ports +188 LOC audit_attestation.rs
       integration test fixture (D-13 satisfied).

       Upstream-commit: 9db06336
       Upstream-tag: v0.40.1
       Upstream-author: <capture from `git log -1 9db06336 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; test -f crates/nono-cli/tests/audit_attestation.rs &amp;&amp; cargo test -p nono-cli --test audit_attestation &amp;&amp; cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 9db06336'</automated>
  </verify>
  <acceptance_criteria>
    - `crates/nono-cli/tests/audit_attestation.rs` exists, ~188 LOC.
    - `cargo test -p nono-cli --test audit_attestation` exits 0.
    - `MANUAL_TEST_STEPS.md` re-introduced.
    - `cargo build --workspace` exits 0.
    - `git log -1 --format=%B | grep '^Upstream-commit: 9db06336'` returns 1 line.
    - **D-04 conservative gate** (per WARNING #4 from plan-checker — `9db06336` "refines audit path derivation" which could implicitly touch session paths): `cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed` exits 0 (CLEAN-04 still preserved post-9db06336).
  </acceptance_criteria>
  <done>
    Full upstream AUD cluster landed. Audit attestation test fixture covering AUD-02 acceptance is green.
  </done>
</task>

<task type="auto">
  <name>Task 9: Verify `nono prune` hidden alias deprecation note (AUD-04 acceptance #3)</name>
  <files>crates/nono-cli/src/cli.rs (verify only — already touched in Task 2 manual port)</files>
  <read_first>
    - crates/nono-cli/src/cli.rs (post-Task-2 state — find `prune` subcommand definition)
    - REQUIREMENTS.md § AUD-04 acceptance #3 ("nono prune (hidden alias) still works and surfaces a deprecation note")
  </read_first>
  <action>
    1. Verify hidden alias works:
       ```
       cargo run -p nono-cli -- prune --help 2>&1 | head
       cargo run -p nono-cli -- prune --all-exited 2>&1 | head -10
       ```
       Expected: prune surfaces a deprecation note (e.g., "warning: `nono prune` is deprecated; use `nono session cleanup` instead") and still functions.

    2. If deprecation note is missing, add it as a fork-only follow-up commit:
       ```rust
       // In cli.rs, near the prune Cmd variant:
       #[command(hide = true, after_help = "DEPRECATED: use `nono session cleanup` instead.")]
       Prune { ... },
       ```
       Or in the dispatcher in main.rs, emit `eprintln!("warning: ...")` before delegating to session cleanup logic.

    3. Add or verify a test:
       ```rust
       // crates/nono-cli/tests/prune_alias_deprecation.rs (or extend existing)
       #[test]
       fn prune_alias_surfaces_deprecation_note() {
           let output = std::process::Command::new(env!("CARGO_BIN_EXE_nono"))
               .args(["prune", "--help"])
               .output()
               .expect("nono binary should be present");
           let combined = format!("{}\n{}",
               String::from_utf8_lossy(&output.stdout),
               String::from_utf8_lossy(&output.stderr));
           assert!(combined.to_lowercase().contains("deprecat"),
                   "expected deprecation note in `nono prune --help`, got: {}", combined);
       }
       ```

    4. Commit (fork-only follow-up if needed):
       ```
       git commit -s -m "$(cat <<'EOF'
       feat(22-05): surface deprecation note on `nono prune` hidden alias (REQ-AUD-04 #3)

       Plan 22-05 Task 2 renamed prune → session cleanup. Hidden alias still
       works per REQ-AUD-04 acceptance #3; this commit ensures the
       deprecation note appears in --help output and on invocation.

       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
  </action>
  <verify>
    <automated>cargo run -p nono-cli -- prune --help 2&gt;&amp;1 | grep -i 'deprecat'</automated>
  </verify>
  <acceptance_criteria>
    - `cargo run -p nono-cli -- prune --help` output contains the word "deprecated" (case-insensitive).
    - `cargo run -p nono-cli -- prune --all-exited` (or equivalent) still functions.
    - VALIDATION 22-05-V2 (`prune_alias_deprecation_note`) test green.
  </acceptance_criteria>
  <done>
    Hidden alias deprecation surface verified. REQ-AUD-04 acceptance #3 satisfied.
  </done>
</task>

<task type="auto">
  <name>Task 10: Full CLEAN-04 + AppliedLabelsGuard regression sweep (BLOCKING)</name>
  <files>(read-only verification — no files modified)</files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-VALIDATION.md (22-05-V1, 22-05-V3 regression rows)
    - .planning/phases/19-cleanup/19-CONTEXT.md (CLEAN-04 invariant origin)
  </read_first>
  <action>
    1. Re-run the FULL CLEAN-04 invariant suite (vs Task 1 baseline):
       ```
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
       cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited
       cargo test -p nono-cli cli::parse_duration_
       grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs
       ```
       ALL must match Task 1 baseline. If `auto_prune_is_noop_when_sandboxed` fails: ABSOLUTE STOP per CONTEXT STOP trigger #6.

    2. Re-run Phase 21 AppliedLabelsGuard lifecycle:
       ```
       cargo test -p nono --test sandbox_windows applied_labels_guard::
       ```

    3. New audit-flush regression test (VALIDATION 22-05-V3): verify ledger flush completes BEFORE AppliedLabelsGuard Drop. If the test doesn't exist yet, add it:
       ```rust
       // crates/nono/tests/sandbox_windows.rs::applied_labels_guard module
       #[test]
       #[cfg(target_os = "windows")]
       fn audit_flush_before_drop() {
           // Construct a SupervisedRuntimeContext with audit_integrity enabled
           // Drop the AppliedLabelsGuard
           // Assert ledger has the expected events flushed (i.e., flush happened pre-Drop)
       }
       ```

    4. If new regression: STOP per CONTEXT STOP trigger #5.
  </action>
  <verify>
    <automated>cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed &amp;&amp; cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited &amp;&amp; cargo test -p nono-cli cli::parse_duration_ &amp;&amp; grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs</automated>
  </verify>
  <acceptance_criteria>
    - All CLEAN-04 invariant tests green (match Task 1 baseline).
    - `AUTO_PRUNE_STALE_THRESHOLD = 100` constant unchanged.
    - Phase 21 AppliedLabelsGuard lifecycle tests still green.
    - VALIDATION 22-05-V1 + V3 marked green.
  </acceptance_criteria>
  <done>
    Full regression sweep clean. v2.1 invariants survive Plan 22-05.
  </done>
</task>

<task type="auto">
  <name>Task 11: D-18 Windows-regression gate (BLOCKING — final per-plan close)</name>
  <files>(read-only verification)</files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § D-18
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-VALIDATION.md (per-task verification map for 22-05-T1..T4 + V1..V3)
  </read_first>
  <action>
    1. `cargo test --workspace --all-features`
    2. Phase 15 5-row detached-console smoke gate (Windows host)
    3. `cargo test -p nono-cli --test wfp_port_integration -- --ignored` (admin + service)
    4. `cargo test -p nono-cli --test learn_windows_integration`
    5. `cargo test -p nono-cli --test audit_attestation` (ported fixture from Task 8)
    6. `cargo test -p nono-cli --test exec_identity_windows` (Authenticode + SHA-256 fallback from Task 5b)
    7. End-to-end smoke: `nono run --audit-integrity --audit-sign-key keyring://nono/audit -- echo hi` then `nono audit verify <id>`. Verify session has populated chain_head, merkle_root, and audit-attestation.bundle.
    8. VALIDATION.md gate: 22-05-T1..T4 + V1..V3 green.

    If new regression: STOP per CONTEXT STOP trigger #4.
  </action>
  <verify>
    <automated>cargo test --workspace --all-features &amp;&amp; cargo test -p nono-cli --test learn_windows_integration &amp;&amp; cargo test -p nono-cli --test audit_attestation &amp;&amp; cargo fmt --all -- --check &amp;&amp; cargo clippy --workspace -- -D warnings -D clippy::unwrap_used</automated>
  </verify>
  <acceptance_criteria>
    - `cargo test --workspace --all-features` exits 0 within deferred-flake window.
    - Phase 15 5-row smoke gate passes.
    - `wfp_port_integration --ignored` passes or documented-skipped.
    - `learn_windows_integration` exits 0.
    - `audit_attestation` and `exec_identity_windows` (on Windows) tests pass.
    - End-to-end audit-integrity smoke produces session with chain_head + merkle_root + attestation bundle; `nono audit verify` succeeds.
    - `cargo fmt --all -- --check` exits 0.
    - `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` exits 0.
    - VALIDATION.md 22-05-T1..T4 + V1..V3 status updated to green.
  </acceptance_criteria>
  <done>
    D-18 Windows-regression safety net cleared for Plan 22-05. Phase 22 ready to close.
  </done>
</task>

<task type="auto">
  <name>Task 12: D-07 plan-close push to origin + AUD-05 fold-or-split decision</name>
  <files>(no files modified — git push + decision)</files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § D-07, § Claude's Discretion ("AUD-05 fold-or-split decision-point")
    - .planning/ROADMAP.md § Phase 23 (default: AUD-05 lives in Phase 23)
  </read_first>
  <action>
    1. Push:
       ```
       git fetch origin
       git log --oneline origin/main..main
       git push origin main
       git ls-remote origin main
       ```

    2. **AUD-05 fold-or-split decision (CONTEXT Claude's Discretion).** Now that Plan 22-05 is in flight or just landed, examine whether upstream's ledger event shape covers AIPC HandleKinds cleanly with NO Windows-specific surface needed. Run:
       ```
       grep -rE 'handle_(file|socket|pipe|event|mutex|jobobject)_request' crates/nono-cli/src/exec_strategy_windows/
       grep -rE 'AuditEvent|append_event' crates/nono-cli/src/audit/
       ```
       - If upstream's `AuditEvent` enum + `audit/integrity.rs::append_event` cleanly accepts AIPC HandleKind events with no Windows-specific shape required: **Recommend fold into 22-05** (would have been a simple `let kind = HandleKind::File; audit::append_event(...)` per `handle_*_request`). Surface as a SUMMARY recommendation; defer the actual fold to Phase 23 ROADMAP review.
       - If upstream's `AuditEvent` shape REQUIRES a HandleKind-specific extension that's Windows-only: **Confirm Phase 23 stays as-is** (separate plan).

    3. Document the decision in SUMMARY:
       - Decision: fold-recommended / Phase-23-confirmed
       - Rationale: 2-3 lines on what evidence drove the call
       - Action: defer to Phase 23 planning per CONTEXT D-11
  </action>
  <verify>
    <automated>git fetch origin &amp;&amp; test "$(git log origin/main..main --oneline | wc -l)" = "0"</automated>
  </verify>
  <acceptance_criteria>
    - `git log origin/main..main --oneline | wc -l` returns `0` after push.
    - SUMMARY records the post-push origin/main SHA.
    - SUMMARY documents AUD-05 fold-or-split decision with rationale.
  </acceptance_criteria>
  <done>
    Plan 22-05 commits published to origin. Phase 22 cherry-pick work complete. AUD-05 disposition documented for Phase 23 planning.
  </done>
</task>

</tasks>

<non_goals>
**D-17 ALLOWED for audit/exec_identity_windows.rs ONLY (per CONTEXT line 248):** This is the ONE planned fork-only Windows file in Phase 22. ANY OTHER Windows-only file touched (`*_windows.rs` outside audit/exec_identity_windows.rs, or any cherry-pick that surfaces such a touch) is a BUG — abort and investigate per D-17.

**Plan 22-03 PKG scope:** Already landed in Wave 1.

**Plan 22-04 OAUTH scope:** Already landed in Wave 1.

**AUD-05 (Phase 23):** Windows AIPC broker audit-event emissions live in Phase 23. Plan 22-05 Task 12 surfaces a fold-or-split recommendation but does NOT execute the AIPC retrofit.

**DRIFT-01..02 (Phase 24):** Drift-check tooling + GSD upstream-sync template are out of Phase 22 scope per CONTEXT D-11.

**Pre-existing Phase 19 deferred flakes:** `tests/env_vars.rs` (≤19 failures) and `trust_scan::tests::*` (1–3 failures) are documented-deferred. Plan 22-05 must NOT attempt fixes but also MUST NOT let them mask new regressions.

**`prune` alias deprecation timeline (CONTEXT Claude's Discretion):** Whether `nono prune` survives one release (v2.3) or longer is a v2.3-milestone scoping decision, NOT Phase 22.
</non_goals>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Audit ledger file → `nono audit verify` | Ledger file on disk; tampered ledger = Tampering threat. Hash-chain + Merkle root + DSSE signature gate verification. |
| Sign-key (keyring://nono/audit) → process memory | OS keystore → in-memory key. Key compromise = Spoofing threat for attestation. |
| Executable on disk → exec-identity capture | `GetModuleFileNameW` + `WinVerifyTrust` query. Spoofed signature = Spoofing threat for AUD-03 evidence. |
| Sandboxed agent → file system (via prune/cleanup) | CRITICAL: sandboxed agent must NOT delete files via auto-prune. CLEAN-04 invariant. |
| AppliedLabelsGuard Drop → ledger flush | Phase 21 invariant: guard cleanup must NOT race ahead of ledger flush. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation |
|-----------|----------|-----------|----------|-------------|------------|
| T-22-05-01 | Tampering | Audit ledger file modified out-of-band | **high** | mitigate (BLOCKING) | AUD-01: hash-chain + Merkle root via 4f9552ec/4ec61c29; `nono audit verify` rejects fail-closed. AUD-02: DSSE attestation signs the chain head. Tests: `audit::tests::tampered_ledger_rejected`, `tests/audit_attestation.rs::sign_and_verify`. |
| T-22-05-02 | Spoofing | Authenticode signature parsed from a binary signed by an attacker-controlled cert (test cert acceptance bug) | **high** | mitigate (BLOCKING) | Task 5b records `signer_subject` + `thumbprint` + `state` (ValidSigned/InvalidSignature/Unsigned/QueryFailed). Verification at audit-show time displays the subject; downstream can policy-reject untrusted signers. Test: `exec_identity_windows::authenticode_signed_records_subject`. |
| T-22-05-03 | Spoofing | Audit signing key compromise | **high** | accept | Key lives in OS keystore (`keyring://nono/audit`); compromise = OS-level breach beyond nono's threat model. Mitigation: provisioning-via-pre-existing-key (CONTEXT Claude's Discretion) avoids auto-gen on weak entropy. |
| T-22-05-04 | Elevation of Privilege | `prune` → `cleanup` rename regresses `auto_prune_is_noop_when_sandboxed` invariant; sandboxed agent deletes files | **CRITICAL — ABSOLUTE STOP** | mitigate (ABSOLUTE BLOCKING) | D-04 gate after every prune-touching commit; CONTEXT STOP trigger #6 (ABSOLUTE STOP). Tests must remain green; failure = revert that commit + ABSOLUTE STOP. |
| T-22-05-05 | Tampering | AppliedLabelsGuard Drop happens BEFORE ledger flush; events lost on cleanup | **high** | mitigate (BLOCKING) | Phase 21 invariant: ledger flush in supervised_runtime cleanup must complete BEFORE AppliedLabelsGuard Drop. Test: `applied_labels_guard::audit_flush_before_drop` (Task 10 step 3). |
| T-22-05-06 | Information Disclosure | Audit-attestation bundle includes raw env vars / paths that leak secrets | medium | mitigate | Upstream's bundle shape minimizes raw payloads; reuse fork's `sanitize_for_terminal` helper if needed. Verify in commit body. |
| T-22-05-07 | DoS | Hash-chain recomputation on `nono audit verify` is O(N) over ledger size; large sessions take minutes | low | accept | Linear in session size; bounded by realistic session length. No O(N²) or unbounded retry. |
| T-22-05-08 | Repudiation | Cherry-pick provenance lost across heavily-forked manual ports | medium | mitigate | D-19 + D-20 trailers enforced. Manual-port commits explicitly cite "(replayed manually)" with conflict-marker counts. |
| T-22-05-09 | Tampering | `--audit-sign-key keyring://...` references a missing key; nono silently writes unsigned ledger | medium | mitigate | Default provisioning model = fail-closed if missing (Task 7). User pre-provisions key; missing = clear error. |
| T-22-05-10 | DoS | `WinVerifyTrust` blocks on network revocation check (CRL/OCSP) | low | accept | Default behavior; Authenticode query is best-effort per AUD-03 acceptance. SHA-256 fallback ensures audit completes even on Authenticode failure. |
| T-22-05-11 | Elevation of Privilege | Drift in `exec_strategy.rs` / `supervised_runtime.rs` / `rollback_runtime.rs` manual-port loses fork's v2.0/v2.1/Phase 21 security guarantees | **high** | mitigate (BLOCKING) | D-20 manual-port template forces explicit replay reasoning; per-commit D-04 gate + Phase 21 AppliedLabelsGuard regression check (Task 10) catches drift. STOP per STOP trigger #1 if a `*_windows.rs` touch surfaces. |

**BLOCKING threats:** T-22-05-01, T-22-05-02, T-22-05-04 (ABSOLUTE), T-22-05-05, T-22-05-11 — Plan 22-05 cannot close until all five are mitigated and verified.
</threat_model>

<verification>
- `cargo build --workspace` exits 0.
- `cargo test --workspace --all-features` exits 0 within Phase 19 deferred-flake tolerance.
- `cargo fmt --all -- --check` exits 0.
- `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` exits 0.
- Phase 15 5-row smoke gate passes.
- `audit_attestation` (ported from 9db06336, +188 LOC) tests green.
- `exec_identity_windows` (fork-only Authenticode) tests green on Windows.
- VALIDATION.md 22-05-T1..T4 + V1..V3 marked green.
- ALL CLEAN-04 invariants match Task 1 baseline (no regressions).
- `AUTO_PRUNE_STALE_THRESHOLD = 100` constant unchanged.
- `auto_prune_is_noop_when_sandboxed` test green in BOTH session_commands.rs and session_commands_windows.rs.
- Phase 21 AppliedLabelsGuard lifecycle still green.
- End-to-end smoke: `nono run --audit-integrity --audit-sign-key keyring://nono/audit -- echo hi` produces session with populated chain_head + merkle_root + attestation bundle; `nono audit verify <id>` succeeds.
- `nono prune --help` surfaces deprecation note (REQ-AUD-04 #3).
- All 7 upstream cherry-pick commits carry D-19 trailers (or D-20 for manual-replay).
- Fork-only Authenticode commit (Task 5b) and prune-deprecation commit (Task 9) carry Signed-off-by trailer; NO Upstream-commit trailer.
- `git log origin/main..main` shows zero commits ahead post-Task 12.
- No `<capture from` placeholders in any commit body.
- AUD-05 fold-or-split decision documented in SUMMARY.
</verification>

<success_criteria>
- 7 upstream cherry-pick commits + 1 fork-only Authenticode commit + 1 fork-only prune-deprecation commit (if needed) on `main`. Total ~9 commits, all DCO-signed.
- Audit-integrity hash-chain + Merkle root populated; DSSE attestation bundle generated.
- `nono audit verify <id>` succeeds; tampered ledger rejects.
- Windows Authenticode exec-identity recorded; SHA-256 fallback when unsigned.
- `nono prune` → `nono session cleanup` rename complete; v2.1 CLEAN-04 invariants ALL preserved.
- `nono audit cleanup` peer subcommand functional.
- `nono prune` hidden alias still works with deprecation note.
- Phase 21 AppliedLabelsGuard lifecycle preserved.
- `make ci` green or matches Phase 19 deferred window.
- `origin/main` advanced to plan-close HEAD; Phase 22 done.
- Plan SUMMARY records all 12 tasks' outcomes, ~9 commit hashes, manual-port replay rationale per heavily-forked file, and AUD-05 fold-or-split decision for Phase 23.
</success_criteria>

<output>
Create `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05-SUMMARY.md` per standard summary template. Required sections: Outcome, What was done (one bullet per task), Verification table, Files changed table, Commits (~9-row table with hashes + upstream provenance + manual-replay annotations), Status, Deferred (AUD-05 disposition decision; any deferred CLEAN-04 follow-ups; manual-port reasoning for heavily-forked files).
</output>
