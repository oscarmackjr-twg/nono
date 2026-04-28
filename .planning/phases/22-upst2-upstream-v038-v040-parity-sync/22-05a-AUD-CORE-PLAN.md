---
phase: 22-upst2-upstream-v038-v040-parity-sync
plan: 05a
type: execute
wave: 2
depends_on: ["22-03", "22-04"]
blocks: ["22-05b"]
files_modified:
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/audit_commands.rs
  - crates/nono-cli/src/audit_integrity.rs
  - crates/nono-cli/src/audit_session.rs
  - crates/nono-cli/src/audit_ledger.rs
  - crates/nono-cli/src/audit_attestation.rs
  - crates/nono-cli/src/exec_identity.rs
  - crates/nono-cli/src/app_runtime.rs
  - crates/nono-cli/src/cli_bootstrap.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/launch_runtime.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/rollback_runtime.rs
  - crates/nono-cli/src/supervised_runtime.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/rollback_commands.rs
  - crates/nono-cli/src/capability_ext.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/trust_cmd.rs
  - crates/nono/src/undo/snapshot.rs
  - crates/nono/src/undo/types.rs
  - crates/nono/src/undo/merkle.rs
  - crates/nono/src/trust/mod.rs
  - crates/nono/src/trust/signing.rs
  - crates/nono-cli/tests/audit_attestation.rs
  - MANUAL_TEST_STEPS.md
  - README.md
  - docs/cli/features/audit.mdx
  - docs/cli/features/execution-modes.mdx
  - docs/cli/features/supervisor.mdx
  - docs/cli/getting_started/quickstart.mdx
  - docs/cli/internals/security-model.mdx
  - docs/cli/usage/flags.mdx
  - Cargo.lock
autonomous: false
requirements: ["AUD-01", "AUD-02", "AUD-03"]

must_haves:
  truths:
    - "`--audit-integrity` and `--audit-sign-key` flags exist on `nono run`; integrity-protected sessions populate `chain_head` and `merkle_root` (AUD-01)"
    - "`nono audit verify <id>` succeeds for an untampered session and rejects tampered ledgers fail-closed (AUD-02)"
    - "`audit-attestation.bundle` is generated via fork's existing `nono::trust::signing::sign_statement_bundle`; key resolution via `keyring://` URI (AUD-02)"
    - "Pre/post Merkle roots are captured at audit-integrity boundaries (AUD-01 cont.)"
    - "Cross-platform `ExecutableIdentity` records SHA-256 of the executed binary (AUD-03 SHA-256 portion)"
    - "Exec identity is unified into the audit-integrity Alpha schema (single envelope; AUD-03 SHA-256 portion cont.)"
    - "Audit ledger path derivation refined; `MANUAL_TEST_STEPS.md` re-introduced (intentional round-trip vs Plan 22-02 5c301e8d deletion per RESEARCH open question #4)"
    - "`tests/audit_attestation.rs` (~188 LOC) ported from upstream `9db06336` and green (D-13)"
    - "Phase 21 `AppliedLabelsGuard` lifecycle preserved end-to-end (snapshot+label Drop ordering unchanged; loaded_profile + AIPC allowlist threading unchanged) — existing 76-test suite remains green; formal `audit_flush_before_drop` regression test ships in Plan 22-05b Task 6"
    - "**Boundary discipline (CONTEXT revision LOCKED): NO `prune`/`cleanup` rename touches in this plan.** `nono prune` subcommand still exists with original semantics; CLEAN-04 invariant test names are unchanged; `auto_prune_is_noop_when_sandboxed` is green BEFORE and AFTER every commit in this plan"
    - "**Boundary discipline: NO `session_commands.rs` / `session_commands_windows.rs` rename surface touched in this plan.** Audit-integrity event emissions wire through rollback_runtime/supervised_runtime/exec_strategy ONLY"
    - "**Boundary discipline: NO Authenticode code added in this plan.** Windows Authenticode + SHA-256 fallback ships in Plan 22-05b atop the unified Alpha schema landed here"
    - "Every cherry-pick commit body contains D-19 trailers; the manual-port `4f9552ec` commit uses D-20 template with explicit replay rationale citing 26 conflict markers / 9 conflicted files / ~563 conflict-span lines (CONTEXT revision empirical baseline)"
    - "`cargo test --workspace --all-features` exits 0 on Windows after each commit (D-18); no NEW failures vs the post-22-04 `5c8df06a` baseline"
  artifacts:
    - path: "crates/nono-cli/src/audit_integrity.rs"
      provides: "Hash-chain + Merkle root tamper-evident ledger (NEW or extended; reuses `nono::undo::merkle::MerkleTree`)"
    - path: "crates/nono-cli/src/audit_session.rs"
      provides: "Audit session directory management for audit-only sessions sharing a common session ID (NEW from upstream `4f9552ec`; flat layout — file at `crates/nono-cli/src/` top level, NOT under `audit/`)"
    - path: "crates/nono-cli/src/audit_ledger.rs"
      provides: "Append-only audit ledger primitives feeding the Alpha schema (NEW from upstream `02ee0bd1`; flat layout)"
    - path: "crates/nono-cli/src/exec_identity.rs"
      provides: "SHA-256-only `ExecutableIdentity` cross-platform exec-identity recorder (upstream 02ee0bd1 + 7b7815f7); platform dispatch hook reserved for 22-05b's Authenticode addition"
    - path: "crates/nono-cli/src/audit_attestation.rs"
      provides: "DSSE/in-toto attestation signing via existing `nono::trust::signing::sign_statement_bundle` (NEW)"
    - path: "crates/nono-cli/src/audit_commands.rs"
      provides: "Subcommand dispatch including `nono audit verify <id>` (chain + Merkle + signature verification per AUD-02 acceptance #2; `--public-key-file` flag) and `nono audit show` updates. NOTE: `audit cleanup` peer subcommand is OUT OF SCOPE for this plan — ships in 22-05b."
    - path: "crates/nono-cli/tests/audit_attestation.rs"
      provides: "Audit attestation sign/verify integration tests (~188 LOC ported from upstream `9db06336` per D-13)"
    - path: "MANUAL_TEST_STEPS.md"
      provides: "Re-introduced per upstream `9db06336` (round-trip vs 22-02 deletion of `5c301e8d` — verified intentional per RESEARCH finding open #4)"
  key_links:
    - from: "ROADMAP § Phase 22 success criterion #5 (audit-integrity portion)"
      to: "`nono audit verify` succeeds + AppliedLabelsGuard lifecycle preserved"
      via: "audit subsystem (flat layout: audit_integrity.rs / audit_session.rs / audit_ledger.rs / audit_attestation.rs / audit_commands.rs / exec_identity.rs at `crates/nono-cli/src/` top level; no rename; no Authenticode)"
      pattern: "audit_integrity|chain_head|merkle_root|audit_attestation|append_event"
    - from: "crates/nono-cli/src/exec_strategy.rs (Direct/Monitor/Supervised)"
      to: "audit_integrity::append_event"
      via: "supervised path emits ledger events while preserving fork's strategy branching + AIPC wiring"
      pattern: "append_event|AuditEvent::"
    - from: "crates/nono-cli/src/rollback_runtime.rs (AppliedLabelsGuard)"
      to: "audit_integrity ledger flush"
      via: "ledger flush completes BEFORE AppliedLabelsGuard Drop (Phase 21 invariant)"
      pattern: "AppliedLabelsGuard|flush_ledger|Drop"
---

<objective>
Land the upstream audit-integrity + attestation + verify cluster (AUD-01, AUD-02, and the SHA-256 portion of AUD-03) into the fork — without the `prune` → `session cleanup` rename and without the fork-only Windows Authenticode addition. Both of those ship in Plan 22-05b on a sequential Wave 3.

Strategy: ONE manual-port replay (`4f9552ec` audit-integrity portions only — boundaries explicit and locked) followed by SIX strict-chronological cherry-picks (`4ec61c29` → `02ee0bd1` → `7b7815f7` → `0b1822a9` → `6ecade2e` → `9db06336`). Cherry-pick of `4f9552ec` is empirically infeasible at HEAD `5c8df06a` per CONTEXT revision: 26 conflict markers across 9 forked files, ~563 lines of conflict span (D-02 thresholds breached 7× on lines/file, 4.5× on file count, 1.4× on total span). Replay is therefore mandatory and uses CONTEXT revision boundary discipline to scope away the rename surface.

`autonomous: false` — the manual-port of `4f9552ec`'s audit-integrity portion has the highest D-20 risk in Phase 22 because three forked files with active v2.0/v2.1/Phase-21 security guarantees (`exec_strategy.rs` Direct/Monitor/Supervised + AIPC, `supervised_runtime.rs` `loaded_profile` + AIPC allowlist, `rollback_runtime.rs` `AppliedLabelsGuard` snapshot+label lifecycle) must be preserved while the new audit-integrity hooks land alongside them.

Purpose: A Windows user running `nono run --audit-integrity --audit-sign-key keyring://nono/audit -- <cmd>` produces a session with populated `chain_head`, `merkle_root`, and `audit-attestation.bundle`; `nono audit verify <id>` succeeds; the `nono prune` subcommand still works exactly as it did before this plan started; CLEAN-04 invariants remain green (because they are not touched). REQ-AUD-01, REQ-AUD-02, and the SHA-256 portion of REQ-AUD-03 are landed here. REQ-AUD-04 (rename) and the Windows Authenticode portion of REQ-AUD-03 are reserved for Plan 22-05b.
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
@.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-04-OAUTH-SUMMARY.md
@.planning/phases/19-cleanup/19-CONTEXT.md
@crates/nono-cli/src/cli.rs
@crates/nono-cli/src/exec_strategy.rs
@crates/nono-cli/src/supervised_runtime.rs
@crates/nono-cli/src/rollback_runtime.rs
@crates/nono/src/undo/merkle.rs

<interfaces>
**Upstream cherry-pick chain (STRICT chronological per D-03):**

| Order | SHA | Upstream subject | REQ | This-plan scope |
|-------|-----|------------------|-----|-----------------|
| 1 | `4f9552ec` | feat(audit): add tamper-evident audit log integrity (~1,419+/226− across 21 files; --audit-integrity flag, hash-chain + Merkle root, prune→session cleanup rename, audit cleanup peer) | AUD-01 | **AUDIT-INTEGRITY PORTION ONLY** — `--audit-integrity`/`--audit-sign-key` flags, hash-chain + Merkle root, ledger-event emission. **RENAME PORTION DEFERRED to 22-05b.** |
| 2 | `4ec61c29` | feat(audit): capture pre/post merkle roots | AUD-01 | full cherry-pick |
| 3 | `02ee0bd1` | feat(audit): record executable identity (SHA-256 only) | AUD-03 (SHA-256) | full cherry-pick — **NO Authenticode here** (RESEARCH finding #2) |
| 4 | `7b7815f7` | feat(audit): record exec identity and unify audit integrity | AUD-03 (SHA-256) | full cherry-pick — exec identity unified into Alpha schema |
| 5 | `0b1822a9` | feat(audit): add audit verify command | AUD-02 | full cherry-pick |
| 6 | `6ecade2e` | feat(audit): add audit attestation | AUD-02 | full cherry-pick — reuse fork's existing `nono::trust::signing::sign_statement_bundle` |
| 7 | `9db06336` | feat(audit): refine audit path derivation (re-introduces MANUAL_TEST_STEPS.md, +188 LOC tests/audit_attestation.rs) | AUD-01..03 | full cherry-pick |

**LOCKED OUT of this plan (per CONTEXT revision boundary discipline; ship in 22-05b):**
- `nono prune` → `nono session cleanup` rename (function-name and CLI-binding changes)
- `nono audit cleanup` peer subcommand
- `nono prune` hidden alias / deprecation note
- Windows Authenticode exec-identity recording (~150 LOC fork-only) — atop unified Alpha schema, lands in 22-05b after 7b7815f7 ships here
- `Win32_Security_WinTrust` feature flag in `crates/nono-cli/Cargo.toml`
- `exec_identity_windows.rs` (NEW file)

**RESEARCH-CRITICAL ordering decisions (still binding):**

1. **02ee0bd1 / 7b7815f7 are SHA-256-only.** Upstream's `ExecutableIdentity` struct contains only `resolved_path` + `sha256` (no Authenticode field). RESEARCH Contradiction #2 + finding #2 verified via `git grep WinVerifyTrust v0.40.1` returning 0 results. Authenticode is fork-only and lands in 22-05b on the unified Alpha schema landed here. Do NOT mutate `ExecutableIdentity` to add Authenticode fields in 22-05a.

2. **9db06336 re-introduces MANUAL_TEST_STEPS.md** which `5c301e8d` (Plan 22-02) deleted. RESEARCH open question #4 verified intentional. Cherry-pick will surface this re-add — accept it.

3. **D-13 fixture port lands in `9db06336`.** Upstream's audit_attestation.rs +188 LOC test fixture is the ONE external test fixture file Phase 22 needs to port (RESEARCH finding #6). It ports verbatim with `9db06336`.

**HIGH-CONFLICT FILES per CONTEXT § Files at HIGH merge-conflict risk + RESEARCH baselines + CONTEXT revision empirical measurement:**

| File | Upstream delta in 4f9552ec | Fork drift | Empirical conflict (CONTEXT revision Path B) | This-plan handling |
|------|----------------------------|-----------|----------------------------------------------|--------------------|
| `crates/nono-cli/src/rollback_runtime.rs` | +586 (heaviest) | v2.1 AppliedLabelsGuard snapshot+label lifecycle | 12 markers / 358 lines | manual replay; preserve AppliedLabelsGuard verbatim; add audit-integrity event emissions at decision points |
| `crates/nono-cli/src/supervised_runtime.rs` | +42 | v2.1 SupervisedRuntimeContext.loaded_profile + AIPC allowlist | 2 markers / 68 lines | manual replay; thread audit context alongside `loaded_profile`/AIPC |
| `crates/nono/src/undo/types.rs` | (in 4f9552ec) | ObjectStore additions | 2 markers / 67 lines | manual replay; preserve fork's ObjectStore type extensions |
| `crates/nono-cli/src/cli.rs` | (in 4f9552ec) | massive fork drift | 2 markers / 31 lines | manual replay; **add `--audit-integrity` + `--audit-sign-key` flags ONLY**; do NOT touch `prune`/`Cmd::Prune` here (LOCKED OUT) |
| `crates/nono-cli/src/rollback_commands.rs` | (in 4f9552ec) | fork drift | 3 markers / 15 lines | manual replay |
| `crates/nono/src/undo/snapshot.rs` | +149 (in 4f9552ec) | ObjectStore clone_or_copy + Merkle wiring on Windows | 2 markers / 10 lines | manual replay; preserve clone_or_copy + Merkle wiring; add upstream's audit-integrity Merkle hooks |
| `README.md` | (in 4f9552ec) | minor | 1 marker / 6 lines | accept upstream wording |
| `docs/cli/features/audit.mdx` | (in 4f9552ec) | minor | 1 marker / 4 lines | accept upstream wording |
| `docs/cli/usage/flags.mdx` | (in 4f9552ec) | minor | 1 marker / 4 lines | accept upstream wording |
| `crates/nono-cli/src/exec_strategy.rs` | +144 | v2.0 Direct/Monitor/Supervised + AIPC wiring | auto-merged in CONTEXT-revision dry-run, but flagged semantic-yellow | re-verify post-replay; preserve strategy branching + AIPC wiring; emit ledger events in supervised path only |
| `crates/nono-cli/src/session_commands.rs` | (in 4f9552ec; rename surface) | v2.1 CLEAN-04 invariants | n/a — **LOCKED OUT** | DO NOT modify; rename ships in 22-05b |
| `crates/nono-cli/src/session_commands_windows.rs` | (in 4f9552ec; rename surface) | v2.1 Windows guard logic | n/a — **LOCKED OUT** | DO NOT modify; rename ships in 22-05b |

**CLEAN-04 invariant tests (RESEARCH finding #7) — green BEFORE this plan starts and stay green THROUGHOUT (because we don't touch the rename):**
- `auto_prune_is_noop_when_sandboxed` — `crates/nono-cli/src/session_commands.rs:708` AND `session_commands_windows.rs:801`
- `is_prunable_all_exited_escape_hatch_matches_any_exited` — `crates/nono-cli/src/session.rs:1373`
- `parse_duration_*` family — `crates/nono-cli/src/cli.rs:2454-2472`
- `AUTO_PRUNE_STALE_THRESHOLD: usize = 100` constant — `crates/nono-cli/src/session_commands.rs:32`

**D-04 invariant gate command (run AFTER every commit in this plan as a non-touch sentinel):**
```
cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited
cargo test -p nono-cli cli::parse_duration_
grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs
```
**Expected:** All four pass identically to Task 1 baseline. Because 22-05a does NOT touch the rename surface, any failure here is a structural surprise — STOP per CONTEXT STOP trigger #5 (and STOP trigger #6 ABSOLUTE if `auto_prune_is_noop_when_sandboxed` fails).

**Reusable fork assets (per CONTEXT § Reusable Assets):**
- `nono::keystore::load_secret` (cross-platform) for `--audit-sign-key keyring://nono/audit` (shipped v2.1 Phase 20 UPST-03)
- `nono::trust::signing::sign_statement_bundle` + `public_key_id_hex` for DSSE attestation (shipped v2.1)
- `nono::undo::merkle::MerkleTree` (already used in fork's snapshot system) — reuse for AUD-01 ledger Merkle root
- `current_logon_sid()` + `build_capability_pipe_sddl` (commit `938887f`) — may be needed for write-path SDDL on Windows ledger; verify during Task 2 implementation

**D-19 commit body template (cherry-pick path):** Same as prior plans — `Upstream-commit:` + tag + author + Signed-off-by trailers.

**STOP triggers from CONTEXT § Specifics that fire in this plan:**
- #1: Touch `*_windows.rs` outside out-of-scope → ABORT (no fork-only Windows file added in this plan; any touch is a structural bug)
- #2: `make ci` red, root cause unclear in 30 min → STOP
- #3: Manual port diff exceeds ~400 lines → already exceeded by 4f9552ec audit-integrity portion alone — proceed but flag in commit body
- #4: Phase 15 5-row smoke gate fails → REVERT and re-scope
- #5: ANY CLEAN-04 invariant test fails → REVERT that commit and re-discuss (this plan does NOT touch the rename, so failure = structural surprise)
- #6 (ABSOLUTE STOP): `auto_prune_is_noop_when_sandboxed` failure — sandboxed-agent file-deletion vector reopened → ABSOLUTE STOP

**Boundary deny-list quick reference for Task 2 (manual-port replay):**
- DO NOT modify `crates/nono-cli/src/session_commands.rs`
- DO NOT modify `crates/nono-cli/src/session_commands_windows.rs`
- DO NOT add a `cleanup` subcommand to clap CLI
- DO NOT remove or rename the `prune` subcommand
- DO NOT add `nono audit cleanup` peer subcommand
- DO NOT add Authenticode-related code or fields
- DO NOT add `Win32_Security_WinTrust` to `Cargo.toml`
- DO add `--audit-integrity` flag (clap `bool`) to `nono run`
- DO add `--audit-sign-key` arg (`Option<String>` with `keyring://` URI value parser) to `nono run`
- DO add hash-chain + Merkle root logic in `audit_integrity.rs`
- DO emit ledger events in `rollback_runtime.rs`/`supervised_runtime.rs`/`exec_strategy.rs` while preserving fork's existing wiring
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: D-04 baseline — capture CLEAN-04 invariant + AppliedLabelsGuard pre-state (READ-ONLY)</name>
  <files>(read-only audit — no files modified)</files>
  <read_first>
    - .planning/phases/19-cleanup/19-CONTEXT.md (CLEAN-04 invariant origin)
    - crates/nono-cli/src/session_commands.rs (auto_prune_is_noop_when_sandboxed test, AUTO_PRUNE_STALE_THRESHOLD constant)
    - crates/nono-cli/src/session.rs (is_prunable_all_exited_escape_hatch_matches_any_exited test)
    - crates/nono-cli/src/cli.rs (parse_duration tests at lines 2454-2472)
    - crates/nono-cli/src/session_commands_windows.rs (Windows duplicate of auto_prune_is_noop)
    - crates/nono/src/sandbox/windows.rs::tests applied_labels_guard:: module (Phase 21 lifecycle)
  </read_first>
  <action>
    1. Verify HEAD is at or after the post-22-04 close (`5c8df06a`). Capture: `git rev-parse HEAD > /tmp/22-05a-pre-head.txt`.

    2. Run the full CLEAN-04 invariant suite and capture baseline state (must be all green pre-22-05a):
       ```
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed 2>&1 | tee /tmp/22-05a-baseline-cleanup.log
       cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited 2>&1 | tee -a /tmp/22-05a-baseline-cleanup.log
       cargo test -p nono-cli cli::parse_duration_ 2>&1 | tee -a /tmp/22-05a-baseline-cleanup.log
       ```

    3. Verify the structural invariants:
       ```
       grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs
       grep -nE 'fn auto_prune_is_noop_when_sandboxed' crates/nono-cli/src/session_commands.rs crates/nono-cli/src/session_commands_windows.rs
       grep -E 'pub fn prune|Cmd::Prune|"prune"' crates/nono-cli/src/cli.rs | head -5
       ```
       All three must succeed: constant present, test exists in BOTH files, `nono prune` subcommand still defined in cli.rs.

    4. If ANY CLEAN-04 invariant fails baseline: STOP. Cannot start Plan 22-05a with a broken baseline.

    5. Verify Phase 21 `AppliedLabelsGuard` lifecycle still green:
       ```
       cargo test -p nono --test sandbox_windows applied_labels_guard:: 2>&1 | tee /tmp/22-05a-baseline-aipc.log
       ```

    6. Capture the `nono prune --help` output as a pre-state snapshot (will compare in Task 10):
       ```
       cargo run -p nono-cli -- prune --help > /tmp/22-05a-pre-prune-help.txt 2>&1
       ```

    7. Record baseline pass counts in preflight note (for SUMMARY).
  </action>
  <verify>
    <automated>cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed &amp;&amp; cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited &amp;&amp; cargo test -p nono-cli cli::parse_duration_ &amp;&amp; grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs</automated>
  </verify>
  <acceptance_criteria>
    - All 3 CLEAN-04 invariant test groups exit 0.
    - `AUTO_PRUNE_STALE_THRESHOLD = 100` constant present.
    - `auto_prune_is_noop_when_sandboxed` test exists in BOTH session_commands.rs and session_commands_windows.rs.
    - `nono prune` subcommand grep returns ≥ 1 hit in cli.rs.
    - Phase 21 AppliedLabelsGuard tests green.
    - Baseline counts + pre-state snapshots recorded for SUMMARY.
  </acceptance_criteria>
  <done>
    Baseline CLEAN-04 invariant suite + AppliedLabelsGuard lifecycle + `nono prune` subcommand all green. Plan 22-05a may proceed.
  </done>
</task>

<task type="auto">
  <name>Task 2: Manual-port replay of `4f9552ec` audit-integrity portions ONLY — D-20 (HIGH-RISK; rename surface LOCKED OUT)</name>
  <files>
    crates/nono-cli/src/cli.rs
    crates/nono-cli/src/audit_commands.rs (NEW)
    crates/nono-cli/src/audit_integrity.rs (NEW)
    crates/nono-cli/src/rollback_runtime.rs
    crates/nono-cli/src/supervised_runtime.rs
    crates/nono-cli/src/exec_strategy.rs
    crates/nono-cli/src/rollback_commands.rs
    crates/nono/src/undo/snapshot.rs
    crates/nono/src/undo/types.rs
    README.md
    docs/cli/features/audit.mdx
    docs/cli/usage/flags.mdx
  </files>
  <read_first>
    - `git show 4f9552ec --stat` (read full file list — 21 files / +1419 / -226)
    - `git show 4f9552ec -- crates/nono-cli/src/rollback_runtime.rs` (anticipate +586 conflict; fork has AppliedLabelsGuard)
    - `git show 4f9552ec -- crates/nono-cli/src/supervised_runtime.rs` (fork has loaded_profile + AIPC)
    - `git show 4f9552ec -- crates/nono-cli/src/exec_strategy.rs` (fork has Direct/Monitor/Supervised + AIPC)
    - `git show 4f9552ec -- crates/nono-cli/src/cli.rs` (extract `--audit-integrity` + `--audit-sign-key` flag definitions ONLY — IGNORE all `prune`/`cleanup` portions)
    - `git show 4f9552ec -- crates/nono/src/undo/snapshot.rs crates/nono/src/undo/types.rs` (fork has ObjectStore clone_or_copy + Merkle wiring on Windows)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § Known Risks "exec_strategy.rs + supervised_runtime.rs + rollback_runtime.rs are the audit-cluster minefield"
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md `<revision>` block — empirical conflict measurement table and boundary discipline LOCKED rules
  </read_first>
  <action>
    **This is the highest-risk task in Plan 22-05a. Do NOT cherry-pick — the empirical baseline (CONTEXT revision) confirms D-02 thresholds are breached 7×/4.5×/1.4×. Proceed directly to D-20 manual-port replay.**

    1. Confirm replay path (no cherry-pick attempt) and capture pre-task SHA:
       ```
       git rev-parse HEAD > /tmp/22-05a-task2-pre.txt
       ```

    2. **DENY-LIST (CONTEXT revision boundary discipline LOCKED — these are HARD prohibitions in this plan):**
       - DO NOT modify `crates/nono-cli/src/session_commands.rs`
       - DO NOT modify `crates/nono-cli/src/session_commands_windows.rs`
       - DO NOT modify `crates/nono-cli/src/session_commands_unix.rs` (if present)
       - DO NOT add a `cleanup` subcommand to the clap CLI in this plan
       - DO NOT remove or rename the existing `prune` subcommand
       - DO NOT add `nono audit cleanup` peer subcommand
       - DO NOT add Authenticode-related code, fields, or windows-sys feature flags
       - DO NOT touch `crates/nono-cli/Cargo.toml` for `Win32_Security_WinTrust`
       - DO NOT create `crates/nono-cli/src/exec_identity_windows.rs`

       **POSITIVE SCOPE (this is what to port from `4f9552ec`):**
       - Add `--audit-integrity` flag (clap `bool`) to `nono run`'s arg group
       - Add `--audit-sign-key <key-ref>` arg (`Option<String>` with `keyring://` URI value parser) to `nono run`'s arg group
       - Update `crates/nono-cli/src/audit_commands.rs` (already exists in fork; modified by upstream `4f9552ec`) — adds the `--audit-integrity`/`--audit-sign-key` integration glue under the existing audit-subcommand dispatch (NEW dispatch handlers; existing file extended; NO `audit cleanup` peer here — that ships in 22-05b)
       - Create `crates/nono-cli/src/audit_integrity.rs` (NEW; flat layout — file at `crates/nono-cli/src/` top level, NOT under `audit/` subdirectory) — hash-chain + Merkle root via `nono::undo::merkle::MerkleTree`. Define `pub fn append_event(&mut self, event: AuditEvent) -> Result<ChainHead>`. Define `AuditEvent` enum covering the event categories upstream emits (capability decisions, URL opens, supervisor lifecycle).
       - Thread `audit_integrity` flag and audit context into `rollback_runtime.rs` and `supervised_runtime.rs`. **PRESERVE** fork's `AppliedLabelsGuard` snapshot+label lifecycle and fork's `loaded_profile` + AIPC allowlist threading verbatim. Audit emissions hook AROUND the existing wiring, not into it.
       - Emit ledger events in `exec_strategy.rs` Supervised path. **PRESERVE** Direct/Monitor/Supervised branching + AIPC wiring. Audit emissions are scoped to the Supervised path (per AUD-01 enforcement: `--audit-integrity` requires supervised execution, not Direct).
       - In `rollback_commands.rs` add audit hooks at decision points (preserve fork drift).
       - In `undo/snapshot.rs` and `undo/types.rs` preserve ObjectStore clone_or_copy + Merkle wiring; add upstream's audit-integrity Merkle hooks. New `audit_integrity` fields on session metadata structures land here per upstream.
       - Apply upstream's wording changes to `README.md`, `docs/cli/features/audit.mdx`, `docs/cli/usage/flags.mdx` (4–6 lines per file). **EDIT ONLY THE AUDIT-INTEGRITY DOCUMENTATION; do NOT edit any prune/cleanup documentation in those files.**

    2.5. **WARNING fix — AppliedLabelsGuard structural snapshot capture (BEFORE replay):**
       Capture the structural surface of fork-only invariants BEFORE any code change so a post-replay diff can prove byte-equivalence. The Phase 21 `AppliedLabelsGuard` lifecycle and `supervised_runtime.rs::loaded_profile` threading are the two lifecycle-sensitive surfaces audit-integrity emissions wire around — any silent re-shape is a structural regression even if `cargo test` is green.
       ```
       grep -B2 -A2 'impl Drop for AppliedLabelsGuard' crates/nono/src/sandbox/windows.rs > /tmp/22-05a-aipc-guard-pre.txt
       grep -B2 -A4 'loaded_profile' crates/nono-cli/src/supervised_runtime.rs > /tmp/22-05a-loaded-profile-pre.txt
       cat /tmp/22-05a-aipc-guard-pre.txt /tmp/22-05a-loaded-profile-pre.txt | wc -l
       ```
       (Pre-snapshots compared in step 8.5 below.)

    3. Read upstream's diff per file with the deny-list active:
       ```
       git show 4f9552ec -- crates/nono-cli/src/cli.rs
       git show 4f9552ec -- crates/nono-cli/src/audit_integrity.rs
       git show 4f9552ec -- crates/nono-cli/src/audit_commands.rs
       git show 4f9552ec -- crates/nono-cli/src/rollback_runtime.rs
       git show 4f9552ec -- crates/nono-cli/src/supervised_runtime.rs
       git show 4f9552ec -- crates/nono-cli/src/exec_strategy.rs
       git show 4f9552ec -- crates/nono-cli/src/rollback_commands.rs
       git show 4f9552ec -- crates/nono/src/undo/snapshot.rs
       git show 4f9552ec -- crates/nono/src/undo/types.rs
       git show 4f9552ec -- README.md docs/cli/features/audit.mdx docs/cli/usage/flags.mdx
       ```
       For each file, identify the audit-integrity-related hunks vs the rename hunks. **Replay only the audit-integrity hunks.**

    4. Apply changes file-by-file using Read + Edit/Write. Do NOT use `git cherry-pick` and do NOT use `git apply` on full upstream patches (which would drag in the rename surface).

    5. **Verify the boundary held (CRITICAL — run before commit):**
       ```
       # Boundary check 1: session_commands*.rs untouched
       git diff --stat crates/nono-cli/src/session_commands.rs crates/nono-cli/src/session_commands_windows.rs
       # Expected: empty output (no diff)

       # Boundary check 2: prune subcommand still exists
       grep -nE 'Cmd::Prune|fn prune|"prune"' crates/nono-cli/src/cli.rs | head -5
       # Expected: ≥ 1 hit (whatever was there pre-Task-2 must still be there)

       # Boundary check 3: no cleanup subcommand added
       grep -nE 'Cmd::Cleanup|"cleanup"|Cmd::SessionCleanup' crates/nono-cli/src/cli.rs | head -5
       # Expected: 0 hits (or only pre-existing unrelated occurrences if any — verify against pre-state)

       # Boundary check 4: no Authenticode code or feature flag
       grep -rE 'Authenticode|WinVerifyTrust|Win32_Security_WinTrust' crates/nono-cli/
       # Expected: 0 hits

       # Boundary check 5: --audit-integrity flag exists
       grep -E 'audit[_-]integrity' crates/nono-cli/src/cli.rs | head -5
       # Expected: ≥ 1 hit (the new flag)
       ```

       If ANY boundary check fails, REVERT the working tree (`git checkout -- .`) and re-do the replay with the deny-list strictly applied. STOP per CONTEXT STOP trigger #1 if a `*_windows.rs` touch surfaced.

    6. Build:
       ```
       cargo build --workspace
       ```
       NEW failures = STOP per STOP trigger #2.

    7. Run library + lib-test smoke (no full workspace test yet — that's the per-commit D-04 gate below):
       ```
       cargo test --workspace --lib
       ```

    8. **D-04 invariant gate (BEFORE commit) — non-touch sentinel:**
       ```
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
       cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited
       cargo test -p nono-cli cli::parse_duration_
       grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs
       ```
       Because we did NOT touch the rename surface, all four MUST match Task 1 baseline identically. If ANY fails: REVERT (`git checkout -- .`) and investigate why a non-rename change broke a rename invariant — STOP per CONTEXT STOP trigger #5. ABSOLUTE STOP if `auto_prune_is_noop_when_sandboxed` fails per STOP trigger #6.

    8.5. **WARNING fix — AppliedLabelsGuard structural diff (POST replay; non-touch sentinel):**
       Re-capture the same structural snapshots from step 2.5 and `diff` them. Any non-empty diff means the manual-port silently mutated the lifecycle surface — STOP per CONTEXT STOP trigger #5.
       ```
       grep -B2 -A2 'impl Drop for AppliedLabelsGuard' crates/nono/src/sandbox/windows.rs > /tmp/22-05a-aipc-guard-post.txt
       grep -B2 -A4 'loaded_profile' crates/nono-cli/src/supervised_runtime.rs > /tmp/22-05a-loaded-profile-post.txt
       diff /tmp/22-05a-aipc-guard-pre.txt /tmp/22-05a-aipc-guard-post.txt
       diff /tmp/22-05a-loaded-profile-pre.txt /tmp/22-05a-loaded-profile-post.txt
       ```
       BOTH diffs MUST be empty. Any output = REVERT (`git checkout -- .`) and re-do the replay; the audit-integrity hooks must wrap AROUND `AppliedLabelsGuard` Drop and `loaded_profile` threading, never mutate them in place.

    9. **Phase 21 AppliedLabelsGuard regression spot-check:**
       ```
       cargo test -p nono --test sandbox_windows applied_labels_guard::
       ```
       Must remain identical to Task 1 baseline. If new failures: STOP per STOP trigger #5.

    10. Stage and commit using D-20 manual-port template:
       ```
       git add crates/nono-cli/src/cli.rs \
               crates/nono-cli/src/audit_integrity.rs crates/nono-cli/src/audit_session.rs crates/nono-cli/src/audit_ledger.rs crates/nono-cli/src/audit_attestation.rs crates/nono-cli/src/audit_commands.rs crates/nono-cli/src/exec_identity.rs \
               crates/nono-cli/src/rollback_runtime.rs \
               crates/nono-cli/src/supervised_runtime.rs \
               crates/nono-cli/src/exec_strategy.rs \
               crates/nono-cli/src/rollback_commands.rs \
               crates/nono/src/undo/snapshot.rs \
               crates/nono/src/undo/types.rs \
               README.md docs/cli/features/audit.mdx docs/cli/usage/flags.mdx
       git status   # verify nothing under session_commands*.rs is staged
       git commit -s -m "$(cat <<'EOF'
       feat(22-05a): port audit-integrity portions of 4f9552ec (AUD-01)

       Manual-port replay over heavily-forked rollback_runtime.rs (+586 upstream
       vs fork v2.1 AppliedLabelsGuard), supervised_runtime.rs (fork loaded_profile
       + AIPC allowlist), exec_strategy.rs (fork Direct/Monitor/Supervised + AIPC),
       undo/types.rs, undo/snapshot.rs, and cli.rs. Cherry-pick aborted at 26
       conflict markers across 9 forked files (~563 lines of conflict span);
       empirical D-02 thresholds breached 7x/4.5x/1.4x on lines-per-file/file-count/
       total-span (CONTEXT revision Path B baseline). Replayed semantically.

       SCOPE: audit-integrity portions only. NOT IN THIS COMMIT (CONTEXT revision
       boundary discipline LOCKED): prune->session-cleanup rename;
       session_commands*.rs touches; nono audit cleanup peer; nono prune hidden
       alias; Windows Authenticode. Those ship in Plan 22-05b.

       Adds --audit-integrity / --audit-sign-key flags. Hash-chain + Merkle root
       via nono::undo::merkle::MerkleTree. Preserves fork's AppliedLabelsGuard
       lifecycle, loaded_profile threading, Direct/Monitor/Supervised strategy
       branching, and AIPC allowlist threading verbatim.

       Upstream-commit: 4f9552ec (replayed manually; rename portion deferred to 22-05b)
       Upstream-tag: v0.40.0
       Upstream-author: <capture from `git log -1 4f9552ec --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
       **Replace `<capture from …>` with the actual upstream author before committing.** No placeholders allowed in the final commit body.
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed &amp;&amp; grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs &amp;&amp; grep -E 'audit[_-]integrity' crates/nono-cli/src/cli.rs &amp;&amp; test -z "$(git diff --stat HEAD~1 HEAD -- crates/nono-cli/src/session_commands.rs crates/nono-cli/src/session_commands_windows.rs)" &amp;&amp; ! grep -rE 'Authenticode|WinVerifyTrust|Win32_Security_WinTrust' crates/nono-cli/ &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 4f9552ec' &amp;&amp; grep -B2 -A2 'impl Drop for AppliedLabelsGuard' crates/nono/src/sandbox/windows.rs &gt; /tmp/22-05a-aipc-guard-post.txt &amp;&amp; grep -B2 -A4 'loaded_profile' crates/nono-cli/src/supervised_runtime.rs &gt; /tmp/22-05a-loaded-profile-post.txt &amp;&amp; diff /tmp/22-05a-aipc-guard-pre.txt /tmp/22-05a-aipc-guard-post.txt &amp;&amp; diff /tmp/22-05a-loaded-profile-pre.txt /tmp/22-05a-loaded-profile-post.txt</automated>
  </verify>
  <acceptance_criteria>
    - `--audit-integrity` flag exists in cli.rs.
    - **AppliedLabelsGuard structural snapshot byte-identical pre vs post replay** (diff returns empty for `/tmp/22-05a-aipc-guard-pre.txt` vs `/tmp/22-05a-aipc-guard-post.txt`).
    - **`loaded_profile` references in `supervised_runtime.rs` byte-identical pre vs post replay** (diff returns empty for the loaded_profile snapshots).
    - `--audit-sign-key` arg exists in cli.rs.
    - `audit_integrity.rs` (NEW) and `audit_commands.rs` (no separate mod.rs in flat layout) (NEW) exist; integrity.rs uses `nono::undo::merkle::MerkleTree`.
    - **Boundary held — `crates/nono-cli/src/session_commands.rs` and `crates/nono-cli/src/session_commands_windows.rs` are byte-identical to pre-Task-2 state** (no diff in this commit).
    - **Boundary held — `nono prune` subcommand still defined** in cli.rs (grep `Cmd::Prune|fn prune|"prune"` returns ≥ 1 hit).
    - **Boundary held — no `cleanup` subcommand added** in this commit.
    - **Boundary held — no Authenticode / WinVerifyTrust / Win32_Security_WinTrust references** anywhere in the diff.
    - **CLEAN-04 invariants ALL GREEN** and IDENTICAL to Task 1 baseline (non-touch sentinel — must not change because rename surface is locked out).
    - `AUTO_PRUNE_STALE_THRESHOLD = 100` constant unchanged.
    - Phase 21 AppliedLabelsGuard tests still green.
    - `git log -1 --format=%B | grep '^Upstream-commit: 4f9552ec'` returns 1 line (with `(replayed manually; rename portion deferred to 22-05b)` annotation).
    - No `<capture from` placeholder remains in commit body.
    - `cargo build --workspace` exits 0.
  </acceptance_criteria>
  <done>
    AUD-01 audit-integrity hash-chain + Merkle root + flags landed. Fork's v2.1 AppliedLabelsGuard / loaded_profile / Direct-Monitor-Supervised + AIPC threading preserved. Rename surface and Authenticode strictly out of scope.
  </done>
</task>

<task type="auto">
  <name>Task 3: Cherry-pick `4ec61c29` — pre/post merkle root capture (AUD-01)</name>
  <files>
    crates/nono-cli/src/audit_integrity.rs
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
       D-02 gate. If conflicts > thresholds: abort and replay manually with same boundary discipline (do NOT touch session_commands*.rs).

    2. Build + lib smoke:
       ```
       cargo build --workspace
       cargo test --workspace --lib
       ```

    3. **D-04 non-touch sentinel** (rollback touches the audit hooks; sentinel must remain green):
       ```
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
       cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited
       cargo test -p nono-cli cli::parse_duration_
       ```

    4. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-05a): capture pre/post merkle roots (AUD-01)

       Records the ledger merkle root BEFORE and AFTER each audit-integrity
       boundary so verification can detect missing events between captures.

       Upstream-commit: 4ec61c29
       Upstream-tag: v0.40.0
       Upstream-author: <capture from `git log -1 4ec61c29 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
       Replace `<capture from …>` with actual upstream author. No placeholders.
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 4ec61c29'</automated>
  </verify>
  <acceptance_criteria>
    - `cargo build --workspace` exits 0.
    - CLEAN-04 invariants still green (non-touch sentinel).
    - `git log -1 --format=%B | grep '^Upstream-commit: 4ec61c29'` returns 1 line.
    - No `<capture from` placeholder.
  </acceptance_criteria>
  <done>
    Pre/post merkle root capture landed.
  </done>
</task>

<task type="auto">
  <name>Task 4: Cherry-pick `02ee0bd1` — record executable identity (SHA-256 only — AUD-03 partial)</name>
  <files>
    crates/nono-cli/src/audit_integrity.rs
    crates/nono-cli/src/exec_identity.rs (NEW — SHA-256 only; cross-platform)
  </files>
  <read_first>
    - `git show 02ee0bd1 --stat` and full diff
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-RESEARCH.md "Authenticode is fork-only" finding (#2) — `git grep WinVerifyTrust v0.40.1` returns 0 results
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 02ee0bd1
       ```
       D-02 gate. Likely lands cleanly (new file or extension in audit module).

    2. **CRITICAL — RESEARCH Contradiction #2 enforcement:** This commit ships SHA-256-only `ExecutableIdentity` (`resolved_path` + `sha256`). Do NOT add Authenticode here, do NOT mutate the struct shape, do NOT add `windows-sys` features. Authenticode lands in Plan 22-05b atop this and 7b7815f7.

    3. Verify cleanly applied:
       ```
       grep -nE 'pub struct ExecutableIdentity' crates/nono-cli/src/exec_identity.rs
       grep -rE 'WinVerifyTrust|Authenticode' crates/nono-cli/src/exec_identity.rs crates/nono-cli/src/audit_integrity.rs crates/nono-cli/src/audit_attestation.rs crates/nono-cli/src/audit_commands.rs crates/nono-cli/src/audit_session.rs crates/nono-cli/src/audit_ledger.rs
       # Expected: ExecutableIdentity grep returns ≥ 1; Authenticode grep returns 0 hits
       ```

    4. Build + lib smoke + D-04 non-touch sentinel:
       ```
       cargo build --workspace
       cargo test --workspace --lib
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
       ```

    5. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-05a): record executable identity (AUD-03 SHA-256 portion)

       Records SHA-256 of the executed binary in the audit ledger. Windows
       Authenticode signature recording lands as a fork-only follow-up
       (exec_identity_windows.rs at crates/nono-cli/src/ top level) in Plan 22-05b atop this and 7b7815f7
       (the unified Alpha schema). Per RESEARCH Contradiction #2: upstream's
       ExecutableIdentity is SHA-256 only (git grep WinVerifyTrust v0.40.1 = 0).

       Upstream-commit: 02ee0bd1
       Upstream-tag: v0.40.0
       Upstream-author: <capture from `git log -1 02ee0bd1 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
       Replace `<capture from …>`. No placeholders.
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; grep -nE 'pub struct ExecutableIdentity' crates/nono-cli/src/exec_identity.rs &amp;&amp; ! grep -rE 'WinVerifyTrust|Authenticode' crates/nono-cli/src/exec_identity.rs crates/nono-cli/src/audit_integrity.rs crates/nono-cli/src/audit_attestation.rs crates/nono-cli/src/audit_commands.rs crates/nono-cli/src/audit_session.rs crates/nono-cli/src/audit_ledger.rs &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 02ee0bd1'</automated>
  </verify>
  <acceptance_criteria>
    - `ExecutableIdentity` struct exists in fork audit module.
    - SHA-256 hash field present; **NO Authenticode field, NO WinVerifyTrust call** (grep verifies).
    - `cargo build --workspace` exits 0.
    - CLEAN-04 non-touch sentinel green.
    - `git log -1 --format=%B | grep '^Upstream-commit: 02ee0bd1'` returns 1 line.
    - No `<capture from` placeholder.
  </acceptance_criteria>
  <done>
    SHA-256 executable identity portion landed. Authenticode follow-up reserved for Plan 22-05b.
  </done>
</task>

<task type="auto">
  <name>Task 5: Cherry-pick `7b7815f7` — record exec identity and unify audit integrity (AUD-03 SHA-256 portion cont.)</name>
  <files>
    crates/nono-cli/src/audit_integrity.rs
    crates/nono-cli/src/exec_identity.rs
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

    2. Verify the unified Alpha schema (used by 22-05b for Authenticode integration):
       ```
       grep -rnE 'integrity.*Alpha|enum AuditEvent|struct ExecutableIdentity' crates/nono-cli/src/audit_integrity.rs crates/nono-cli/src/exec_identity.rs crates/nono-cli/src/audit_ledger.rs | head -10
       # Expected: visible Alpha-schema-shaped enum / variant for downstream Authenticode field addition
       ```

    3. **CRITICAL — Same RESEARCH Contradiction #2 guard:** still SHA-256 only. Do NOT add Authenticode here. Do NOT mutate `ExecutableIdentity` to make room for Authenticode now — that mutation lands in 22-05b as a SIBLING field on the audit envelope per RESEARCH guidance.

    4. Build + lib smoke + D-04 non-touch sentinel.

    5. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-05a): record exec identity and unify audit integrity (AUD-03 SHA-256)

       Unifies the audit-integrity event shape so exec identity + chain head
       + merkle root coexist in a single Alpha schema. This is the schema
       Plan 22-05b's fork-only Authenticode addition will extend with a
       sibling Option<AuthenticodeStatus> field per RESEARCH Contradiction #2
       (no mutation of upstream's ExecutableIdentity).

       Upstream-commit: 7b7815f7
       Upstream-tag: v0.40.0
       Upstream-author: <capture from `git log -1 7b7815f7 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; ! grep -rE 'WinVerifyTrust|Authenticode' crates/nono-cli/src/exec_identity.rs crates/nono-cli/src/audit_integrity.rs crates/nono-cli/src/audit_attestation.rs crates/nono-cli/src/audit_commands.rs crates/nono-cli/src/audit_session.rs crates/nono-cli/src/audit_ledger.rs &amp;&amp; cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 7b7815f7'</automated>
  </verify>
  <acceptance_criteria>
    - `cargo build --workspace` exits 0.
    - Unified Alpha schema visible in audit module (suitable for 22-05b Authenticode sibling field).
    - **No Authenticode references yet** (grep returns 0 hits).
    - `git log -1 --format=%B | grep '^Upstream-commit: 7b7815f7'` returns 1 line.
    - CLEAN-04 invariants still green (non-touch sentinel).
    - No `<capture from` placeholder.
  </acceptance_criteria>
  <done>
    Exec identity unified into audit integrity Alpha schema. Schema is now ready for Plan 22-05b's Authenticode sibling-field addition.
  </done>
</task>

<task type="auto">
  <name>Task 6: Cherry-pick `0b1822a9` — `nono audit verify` command (AUD-02)</name>
  <files>
    crates/nono-cli/src/cli.rs
    crates/nono-cli/src/audit_commands.rs (verify subcommand block) (NEW)
    crates/nono-cli/src/audit_commands.rs
  </files>
  <read_first>
    - `git show 0b1822a9 --stat` and full diff
    - .planning/REQUIREMENTS.md § AUD-02 acceptance #2 (`nono audit verify <session-id> --public-key-file <path>` succeeds for untampered, fails for tampered)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 0b1822a9
       ```
       D-02 gate.

    2. **Boundary check** — this commit adds `nono audit verify` which is a NEW subcommand under `audit`. It should NOT add `audit cleanup` (that's part of the rename in 22-05b). Verify:
       ```
       grep -nE '"verify"|AuditCmd::Verify' crates/nono-cli/src/cli.rs | head -5
       # Expected: ≥ 1 hit (the new verify subcommand)
       grep -nE '"cleanup"|AuditCmd::Cleanup' crates/nono-cli/src/cli.rs | head -5
       # Expected: 0 hits (audit cleanup ships in 22-05b)
       ```
       If the cleanup grep returns hits, this commit accidentally pulled in rename material — investigate and revert.

    3. Build + smoke + D-04 sentinel:
       ```
       cargo build --workspace
       cargo run -p nono-cli -- audit verify --help 2>&1 | head -10
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
       ```
       `audit verify --help` should list the subcommand without error.

    4. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-05a): add audit verify command (AUD-02)

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
    <automated>cargo build --workspace &amp;&amp; cargo run -p nono-cli -- audit verify --help 2&gt;&amp;1 | head &amp;&amp; cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 0b1822a9'</automated>
  </verify>
  <acceptance_criteria>
    - `cargo run -p nono-cli -- audit verify --help` lists the subcommand without error.
    - **Boundary held: no `audit cleanup` subcommand added** in this commit.
    - `cargo build --workspace` exits 0.
    - CLEAN-04 invariants green.
    - `git log -1 --format=%B | grep '^Upstream-commit: 0b1822a9'` returns 1 line.
    - No `<capture from` placeholder.
  </acceptance_criteria>
  <done>
    `nono audit verify` command landed. AUD-02 verification path covered.
  </done>
</task>

<task type="auto">
  <name>Task 7: Cherry-pick `6ecade2e` — audit attestation (AUD-02)</name>
  <files>
    crates/nono-cli/src/audit_attestation.rs (NEW)
    crates/nono-cli/src/audit_commands.rs
  </files>
  <read_first>
    - `git show 6ecade2e --stat` and full diff
    - .planning/REQUIREMENTS.md § AUD-02 (DSSE/in-toto attestation)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § Reusable Assets ("`nono::trust::signing::sign_statement_bundle` + `public_key_id_hex` already exists in fork from v2.1")
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § Claude's Discretion ("Audit signing key provisioning model on Windows" — default to "user pre-provisioning + fail-closed if missing")
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 6ecade2e
       ```
       D-02 gate.

    2. Verify integration with fork's existing trust signing (per CONTEXT § Reusable Assets):
       ```
       grep -rnE 'sign_statement_bundle|public_key_id_hex' crates/nono-cli/src/audit_attestation.rs crates/nono-cli/src/audit_commands.rs
       # Expected: ≥ 1 hit (reuses fork's existing module, no duplicate signing impl)
       ```
       If 0 hits and upstream introduced a separate signing helper: WARN in commit body and reconcile to fork's existing module to avoid double-implementation.

    3. **Audit signing key provisioning model (Claude's Discretion per CONTEXT line 105).** Default to "user pre-provisioning + fail-closed if missing". If `--audit-sign-key keyring://nono/audit` references a missing key, fail with clear error. Do NOT auto-generate. Verify the code path:
       ```
       grep -rE 'load_secret|keyring://' crates/nono-cli/src/audit_attestation.rs
       # Expected: ≥ 1 hit (resolves through nono::keystore::load_secret)
       ```

    4. Build + smoke + D-04 sentinel.

    5. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-05a): add audit attestation (AUD-02)

       DSSE/in-toto attestation signing via existing nono::trust::signing
       (shipped v2.1). Key resolution via keyring:// URI through fork's
       existing nono::keystore::load_secret. Default provisioning model:
       user pre-provisions key; missing key fails fail-closed (per
       CONTEXT Claude's Discretion bullet — "user pre-provisioning + fail-closed
       if missing").

       Upstream-commit: 6ecade2e
       Upstream-tag: v0.40.0
       Upstream-author: <capture from `git log -1 6ecade2e --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; grep -rnE 'sign_statement_bundle' crates/nono-cli/src/audit_attestation.rs &amp;&amp; cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 6ecade2e'</automated>
  </verify>
  <acceptance_criteria>
    - `crates/nono-cli/src/audit_attestation.rs` (or equivalent) exists.
    - Reuses `nono::trust::signing::sign_statement_bundle` (no duplicate signing implementation).
    - Key resolution flows through `nono::keystore::load_secret` for `keyring://` URIs.
    - `cargo build --workspace` exits 0.
    - CLEAN-04 invariants green.
    - `git log -1 --format=%B | grep '^Upstream-commit: 6ecade2e'` returns 1 line.
    - No `<capture from` placeholder.
  </acceptance_criteria>
  <done>
    AUD-02 attestation landed. Fork's v2.1 trust-signing + keystore reused.
  </done>
</task>

<task type="auto">
  <name>Task 8: Cherry-pick `9db06336` — refine audit path derivation + port `audit_attestation.rs` test fixture (188 LOC) + re-introduce MANUAL_TEST_STEPS.md</name>
  <files>
    crates/nono-cli/src/audit_commands.rs
    crates/nono-cli/src/profile/mod.rs (+1 LOC per upstream)
    crates/nono-cli/src/rollback_runtime.rs (+76 per upstream)
    crates/nono-cli/src/supervised_runtime.rs (+2 per upstream)
    crates/nono-cli/src/capability_ext.rs (+27 per upstream)
    crates/nono-cli/tests/audit_attestation.rs (NEW — ported from upstream, +188 LOC)
    MANUAL_TEST_STEPS.md (re-introduced per RESEARCH open question #4)
  </files>
  <read_first>
    - `git show 9db06336 --stat` (verify +188 LOC tests/audit_attestation.rs)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-RESEARCH.md (open question #4: 9db06336 re-introduces MANUAL_TEST_STEPS.md after Plan 22-02's 5c301e8d deleted it; verified intentional)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 9db06336
       ```
       D-02 gate. Heavily-forked files reappear here (`rollback_runtime.rs`, `supervised_runtime.rs`, `capability_ext.rs`) — the Task 2 manual-port already established the right shape, so this should land cleanly or with small conflicts.

    1.5. **D-02 fallback gate (WARNING fix).** If `git cherry-pick 9db06336` produces conflicts that breach D-02 thresholds (>50 lines/file OR >2 forked files), abort the cherry-pick and replay manually with the **same boundary discipline as Task 2**: do NOT touch `crates/nono-cli/src/session_commands.rs` or `crates/nono-cli/src/session_commands_windows.rs`. The rename surface is LOCKED OUT for the entire 22-05a plan; a manual-replay fallback here must preserve that. Use the D-20 manual-port commit-body template (per CONTEXT § Specifics) and annotate the SHA `9db06336 (replayed manually; rename portion deferred to 22-05b)`.

    2. **MANUAL_TEST_STEPS.md re-introduction.** Plan 22-02's `5c301e8d` deleted this file; `9db06336` re-adds it. Accept the round-trip per RESEARCH open #4.
       ```
       test -f MANUAL_TEST_STEPS.md
       wc -l MANUAL_TEST_STEPS.md
       ```

    3. **D-13 fixture port (the ONE Phase-22 external-test-fixture port per RESEARCH finding #6).** Verify:
       ```
       test -f crates/nono-cli/tests/audit_attestation.rs
       wc -l crates/nono-cli/tests/audit_attestation.rs   # expect ~188
       ```

    4. Run the ported test fixture:
       ```
       cargo test -p nono-cli --test audit_attestation
       ```
       MUST exit 0. If red: investigate whether the prior tasks' Alpha-schema changes broke the fixture's expectations; the fixture port and the production code are in the same upstream commit so they should agree.

    5. **D-04 conservative gate** — `9db06336` "refines audit path derivation" which could implicitly touch session paths even though it does NOT touch session_commands*.rs:
       ```
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
       cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited
       cargo test -p nono-cli cli::parse_duration_
       grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs
       ```

    6. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-05a): refine audit path derivation + port attestation test fixture

       Refines audit ledger path derivation (consistent across platforms);
       re-introduces MANUAL_TEST_STEPS.md (intentional round-trip vs Plan
       22-02 5c301e8d deletion per RESEARCH open #4); ports +188 LOC
       audit_attestation.rs integration test fixture (D-13 satisfied — the
       one Phase-22 external-test-fixture port per RESEARCH finding #6).

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
    - `MANUAL_TEST_STEPS.md` re-introduced (round-trip with 22-02 deletion).
    - `cargo build --workspace` exits 0.
    - **CLEAN-04 invariants ALL GREEN** (D-04 conservative gate — sentinel must hold).
    - `git log -1 --format=%B | grep '^Upstream-commit: 9db06336'` returns 1 line.
    - No `<capture from` placeholder.
  </acceptance_criteria>
  <done>
    Full upstream AUD cluster landed in Plan 22-05a. Audit attestation test fixture covering AUD-02 acceptance is green. SHA-256 portion of AUD-03 complete (Authenticode reserved for 22-05b).
  </done>
</task>

<task type="auto">
  <name>Task 9: Phase 21 AppliedLabelsGuard regression spot-check (BLOCKING)</name>
  <files>(read-only verification — no files modified)</files>
  <read_first>
    - .planning/phases/21-windows-single-file-grants/21-CONTEXT.md (AppliedLabelsGuard origin + lifecycle invariants)
    - crates/nono/src/sandbox/windows.rs (applied_labels_guard test module — Phase 21 76-test suite)
  </read_first>
  <action>
    1. Re-run Phase 21 AppliedLabelsGuard lifecycle suite:
       ```
       cargo test -p nono --test sandbox_windows applied_labels_guard:: 2>&1 | tee /tmp/22-05a-final-aipc.log
       ```
       Compare to Task 1 baseline (`/tmp/22-05a-baseline-aipc.log`). Pass count MUST be identical.

    2. Spot-check the integration with audit-integrity emissions: any new test exercising the supervised path with audit-integrity enabled should not race with guard Drop. The formal end-to-end ledger-flush-before-Drop test ships in Plan 22-05b Task 6 alongside the rename's invariant sweep — note this in SUMMARY.

    3. If new regression: STOP per CONTEXT STOP trigger #5; the audit-integrity hook is racing with `AppliedLabelsGuard` Drop or has corrupted snapshot+label lifecycle. Bisect across Tasks 2–8 commits to find the regressing commit, revert it, and re-discuss.
  </action>
  <verify>
    <automated>cargo test -p nono --test sandbox_windows applied_labels_guard::</automated>
  </verify>
  <acceptance_criteria>
    - `applied_labels_guard::` test pass count identical to Task 1 baseline.
    - SUMMARY records pass count + delta vs baseline (expected: 0 delta).
  </acceptance_criteria>
  <done>
    Phase 21 AppliedLabelsGuard lifecycle survives Plan 22-05a.
  </done>
</task>

<task type="auto">
  <name>Task 10: D-18 Windows-regression gate (BLOCKING — final per-plan close gate)</name>
  <files>(read-only verification)</files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § D-18
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-VALIDATION.md (per-task verification map for 22-05-T1..T2 + part of T3 + V1 + V3)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-04-OAUTH-SUMMARY.md (deferred-flake baseline — 5 pre-existing categories carry over from `5c8df06a`)
  </read_first>
  <action>
    1. Capture pre-gate HEAD: `git rev-parse HEAD > /tmp/22-05a-final-head.txt`

    2. Run the full D-18 safety net (vs Plan 22-04 baseline `5c8df06a`):
       ```
       cargo test --workspace --all-features 2>&1 | tee /tmp/22-05a-final-ci.log
       ```
       Compare failure categories to 22-04's deferred-flake baseline (TUF root signature freshness 2 tests, `convert_filesystem_grants` `/tmp` 1 test, `policy::tests::test_resolve_*` `/tmp` 3 tests, `windows_*_help_reports_documented_limitation` 4 tests, `windows_run_allows_*` UNC path 8 tests). NEW failures = STOP per STOP trigger #2.

    3. Phase 15 5-row detached-console smoke gate (Windows host):
       ```
       # Manual: nono run --profile claude-code -- claude-code --version; nono ps; nono stop <id>
       # Verify all 5 rows appear in `ps` output and `stop` succeeds
       ```
       If non-Windows host or no profile available: documented-skip with rationale.

    4. WFP integration suite (if admin + service available):
       ```
       cargo test -p nono-cli --test wfp_port_integration -- --ignored
       ```
       Documented-skip if not.

    5. Learn-Windows ETW smoke:
       ```
       cargo test -p nono-cli --test learn_windows_integration
       ```

    6. Audit-attestation fixture (ported from 9db06336 in Task 8):
       ```
       cargo test -p nono-cli --test audit_attestation
       ```

    7. End-to-end audit-integrity smoke (Windows host):
       ```
       # Pre-provision keyring://nono/audit with a signing key
       cargo run -p nono-cli -- run --audit-integrity --audit-sign-key keyring://nono/audit -- echo hi
       # Capture <session-id> from output
       cargo run -p nono-cli -- audit verify <session-id>
       ```
       Verify session has populated `chain_head`, `merkle_root`, and `audit-attestation.bundle`; verify command exits 0.

    8. Format + clippy:
       ```
       cargo fmt --all -- --check
       cargo clippy --workspace -- -D warnings -D clippy::unwrap_used
       ```
       Pre-existing carry-over: 2 errors in `crates/nono/src/manifest.rs:95/103` (`collapsible_match`) per 22-04 SUMMARY — accept; surface in this plan's SUMMARY as carry-over.

    9. **Final boundary re-check (CONTEXT revision LOCKED):**
       ```
       # session_commands*.rs unchanged in this plan's commit range (vs Task 1 pre-state)
       PRE_HEAD=$(cat /tmp/22-05a-pre-head.txt)
       git diff --stat $PRE_HEAD HEAD -- crates/nono-cli/src/session_commands.rs crates/nono-cli/src/session_commands_windows.rs
       # Expected: empty (no diff)

       # nono prune subcommand still exists
       grep -E 'Cmd::Prune|fn prune|"prune"' crates/nono-cli/src/cli.rs | head -1
       # Expected: ≥ 1 hit

       # No Authenticode references anywhere
       grep -rE 'Authenticode|WinVerifyTrust|Win32_Security_WinTrust' crates/nono-cli/
       # Expected: 0 hits
       ```
       If ANY check fails: scope creep escaped the boundary discipline. STOP and revert the offending commit.

    10. VALIDATION.md gate: 22-05-T1..T2 + 22-05-T3 (SHA-256 portion) + V1 + V3 marked green; V2 (`prune_alias_deprecation_note`) deferred to 22-05b; T4 (rename) deferred to 22-05b; T3 Authenticode portion deferred to 22-05b.
  </action>
  <verify>
    <automated>cargo test --workspace --all-features &amp;&amp; cargo test -p nono-cli --test learn_windows_integration &amp;&amp; cargo test -p nono-cli --test audit_attestation &amp;&amp; cargo fmt --all -- --check &amp;&amp; cargo clippy --workspace -- -D warnings -D clippy::unwrap_used</automated>
  </verify>
  <acceptance_criteria>
    - `cargo test --workspace --all-features` exits 0 within deferred-flake window (no NEW categories vs `5c8df06a` baseline).
    - Phase 15 5-row smoke gate passes (or documented-skip with rationale).
    - `wfp_port_integration --ignored` passes or documented-skipped.
    - `learn_windows_integration` exits 0.
    - `audit_attestation` test passes.
    - End-to-end audit-integrity smoke produces session with chain_head + merkle_root + attestation bundle; `nono audit verify` succeeds.
    - `cargo fmt --all -- --check` exits 0.
    - `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` exits 0 (carry-over manifest.rs documented).
    - **Final boundary re-check passes** — session_commands*.rs unchanged; `nono prune` subcommand still defined; 0 Authenticode/WinVerifyTrust references.
    - VALIDATION.md 22-05-T1..T2 + T3 SHA-256 + V1 + V3 marked green; T3 Authenticode portion + T4 + V2 explicitly carried over to 22-05b.
  </acceptance_criteria>
  <done>
    D-18 Windows-regression safety net cleared for Plan 22-05a. Plan 22-05b is unblocked and may begin Wave 3.
  </done>
</task>

<task type="auto">
  <name>Task 11: Plan SUMMARY + D-07 plan-close push to origin</name>
  <files>
    .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05a-AUD-CORE-SUMMARY.md (NEW)
  </files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-04-OAUTH-SUMMARY.md (sibling plan SUMMARY format template)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § D-07 (per-plan push pattern)
    - $HOME/.claude/get-shit-done/templates/summary.md (standard summary template)
  </read_first>
  <action>
    1. Author `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05a-AUD-CORE-SUMMARY.md` matching 22-04-OAUTH-SUMMARY.md frontmatter shape (`phase`, `plan: 05a`, `subsystem: audit-core`, `tags`, `dependency_graph` with requires/provides/affects, `tech_stack` added/patterns, `key_files` created/modified, `decisions`, `metrics`).

       Required body sections: Outcome, What was done (one bullet per Task 1–11), Verification table (gate / expected / actual), Files changed table, Commits (~7-row table — 1 manual-port D-20 + 6 cherry-picks; hashes + upstream provenance + manual-replay annotation on 4f9552ec), Status, Deferred-to-22-05b (rename, Authenticode, prune deprecation note, audit cleanup peer subcommand — explicit forward references), Threat model coverage (T-22-05-01/02/03/06/07/08/09/10/11 mitigated; T-22-05-04 NOT IN SCOPE — that's 22-05b's; T-22-05-05 partial — formal flush-before-Drop test deferred to 22-05b).

    2. Verify SUMMARY references all 11 tasks and explicitly documents that boundary discipline held end-to-end (no rename-surface touches; no Authenticode adds; `nono prune` subcommand intact; CLEAN-04 invariants identical to baseline).

    3. Stage SUMMARY:
       ```
       git add .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05a-AUD-CORE-SUMMARY.md
       git commit -s -m "$(cat <<'EOF'
       docs(22-05a): close plan with audit-integrity + attestation + verify cluster

       7 commits land AUD-01, AUD-02, and SHA-256 portion of AUD-03 (1 manual-
       port replay D-20 + 6 cherry-picks). Boundary discipline held: no rename-
       surface touches; no Authenticode; CLEAN-04 invariants identical to
       pre-plan baseline. Plan 22-05b ships rename + Authenticode + CLEAN-04
       full sweep on Wave 3.

       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    4. D-07 plan-close push:
       ```
       git fetch origin
       git log --oneline origin/main..main
       git push origin main
       git ls-remote origin main
       ```

    5. Record post-push origin/main SHA in SUMMARY metadata.
  </action>
  <verify>
    <automated>test -f .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05a-AUD-CORE-SUMMARY.md &amp;&amp; git fetch origin &amp;&amp; test "$(git log origin/main..main --oneline | wc -l)" = "0"</automated>
  </verify>
  <acceptance_criteria>
    - `22-05a-AUD-CORE-SUMMARY.md` exists with the standard frontmatter + body sections.
    - SUMMARY explicitly documents boundary discipline held (no session_commands*.rs diff; no Authenticode; CLEAN-04 baseline match).
    - SUMMARY documents what is forward-deferred to 22-05b (rename, Authenticode, prune deprecation note, audit cleanup peer).
    - `git log origin/main..main --oneline | wc -l` returns `0` after push.
    - SUMMARY records post-push origin/main SHA for traceability.
  </acceptance_criteria>
  <done>
    Plan 22-05a closed and published to origin. Plan 22-05b unblocked.
  </done>
</task>

</tasks>

<non_goals>
**Boundary discipline LOCKED OUT (CONTEXT revision; ships in Plan 22-05b):**
- `nono prune` → `nono session cleanup` rename (function-name + CLI-binding changes)
- `nono audit cleanup` peer subcommand
- `nono prune` hidden alias / deprecation note
- Windows Authenticode exec-identity recording (`exec_identity_windows.rs` ~150 LOC fork-only)
- `Win32_Security_WinTrust` feature flag in `crates/nono-cli/Cargo.toml`
- Full CLEAN-04 invariant regression sweep with rename-touch (the rename happens in 22-05b)
- `applied_labels_guard::audit_flush_before_drop` formal regression test (lands alongside the rename invariant sweep in 22-05b)

**D-17 still ABSOLUTE in this plan:** Any cherry-pick or manual port that touches a `*_windows.rs` file is a BUG — ABORT and investigate. There are NO planned Windows-only file additions in 22-05a; the planned fork-only Windows file (`exec_identity_windows.rs`) lands in 22-05b.

**AUD-04 (rename) is NOT in this plan's requirements.** Plan 22-05a covers AUD-01, AUD-02, and the SHA-256 portion of AUD-03.

**AUD-05 fold-or-split decision-point** — defer to Plan 22-05b's close (Phase 23 ROADMAP retains AUD-05 as default home; revisit if 22-05b's full schema makes a fold trivial).

**Plan 22-03 PKG / Plan 22-04 OAUTH scopes:** Already landed in Wave 1; this plan does not retouch package_cmd.rs or oauth2.rs.

**Pre-existing Phase 19 deferred flakes:** `tests/env_vars.rs` (≤19 failures) and `trust_scan::tests::*` (1–3 failures) are documented-deferred. Plan 22-05a must NOT attempt fixes but also MUST NOT let them mask new regressions.
</non_goals>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Audit ledger file → `nono audit verify` | Ledger file on disk; tampered ledger = Tampering threat. Hash-chain + Merkle root + DSSE signature gate verification. |
| Sign-key (`keyring://nono/audit`) → process memory | OS keystore → in-memory key. Key compromise = Spoofing threat for attestation. |
| Cross-platform exec-identity SHA-256 capture → ledger | Hash of binary on disk. Spoofed binary swap before capture = Spoofing for AUD-03 evidence (mitigated only by Authenticode in 22-05b on Windows). |
| AppliedLabelsGuard Drop → ledger flush | Phase 21 invariant: guard cleanup must NOT race ahead of ledger flush. |
| Manual-port replay of `4f9552ec` → fork's v2.0/v2.1 security guarantees | Replay must preserve Direct/Monitor/Supervised + AIPC + AppliedLabelsGuard + loaded_profile + WSFG-safe ObjectStore. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation |
|-----------|----------|-----------|----------|-------------|------------|
| T-22-05a-01 | Tampering | Audit ledger file modified out-of-band | **high** | mitigate (BLOCKING) | AUD-01: hash-chain + Merkle root via Tasks 2 + 3 (4f9552ec audit-integrity portion + 4ec61c29). AUD-02: DSSE attestation signs the chain head (Task 7). `nono audit verify` rejects fail-closed (Task 6). Test: `tests/audit_attestation.rs` (Task 8). |
| T-22-05a-02 | Spoofing | Audit signing key compromise | **high** | accept | Key lives in OS keystore (`keyring://nono/audit`); compromise = OS-level breach beyond nono's threat model. Default provisioning: pre-provisioning + fail-closed if missing (Task 7). |
| T-22-05a-03 | Tampering | AppliedLabelsGuard Drop happens BEFORE ledger flush; events lost on cleanup | **high** | mitigate (BLOCKING) | Phase 21 invariant: ledger flush in supervised_runtime cleanup must complete BEFORE AppliedLabelsGuard Drop. Task 2 manual-port preserves the existing Drop ordering; Task 9 spot-checks. Formal regression test (`applied_labels_guard::audit_flush_before_drop`) DEFERRED to Plan 22-05b's CLEAN-04 sweep where the lifecycle gets full coverage. |
| T-22-05a-04 | Information Disclosure | Audit-attestation bundle includes raw env vars / paths that leak secrets | medium | mitigate | Upstream's bundle shape minimizes raw payloads; reuse fork's `sanitize_for_terminal` helper if needed (verify in Task 7 commit body). |
| T-22-05a-05 | DoS | Hash-chain recomputation on `nono audit verify` is O(N) over ledger size | low | accept | Linear in session size; bounded by realistic session length. No O(N²) or unbounded retry. |
| T-22-05a-06 | Repudiation | Cherry-pick provenance lost across heavily-forked manual port | medium | mitigate | D-19 + D-20 trailers enforced. The Task 2 manual-port commit explicitly cites "(replayed manually)" with empirical conflict-marker counts (26/9/~563) from CONTEXT revision. |
| T-22-05a-07 | Tampering | `--audit-sign-key keyring://...` references a missing key; nono silently writes unsigned ledger | medium | mitigate | Default provisioning model = fail-closed if missing (Task 7 explicit acceptance criterion). User pre-provisions key; missing = clear error. |
| T-22-05a-08 | Elevation of Privilege | Drift in `exec_strategy.rs` / `supervised_runtime.rs` / `rollback_runtime.rs` manual-port loses fork's v2.0/v2.1/Phase-21 security guarantees | **high** | mitigate (BLOCKING) | D-20 manual-port template (Task 2) forces explicit replay reasoning; per-commit D-04 non-touch sentinel + Phase 21 AppliedLabelsGuard regression check (Task 9) catches drift. STOP per STOP trigger #1 if a `*_windows.rs` touch surfaces. |
| T-22-05a-09 | Tampering | Manual-port replay of `4f9552ec` accidentally drags in rename surface (Cmd::Cleanup, session_commands*.rs touch) — sandboxed-agent file-deletion vector reopens prematurely | **high** | mitigate (BLOCKING) | Task 2 deny-list explicit (cli/session_commands prohibitions). Task 2 step 5 boundary checks (5 grep gates). Task 10 final boundary re-check. STOP trigger #5 / #6 (ABSOLUTE) if CLEAN-04 invariants regress without the rename-surface ever being touched. |
| T-22-05a-10 | Spoofing | Cross-platform SHA-256 exec-identity is not enough to detect Authenticode-signed binary swap on Windows | medium | accept (in this plan; mitigated in 22-05b) | This plan ships SHA-256 only. Authenticode signature query (T-22-05-02 in archive plan) ships in 22-05b. Acknowledged scope reduction; not exposed to user — `nono run --audit-integrity` still records SHA-256 immediately. |

**BLOCKING threats:** T-22-05a-01, T-22-05a-03, T-22-05a-08, T-22-05a-09 — Plan 22-05a cannot close until all four are mitigated and verified.
</threat_model>

<verification>
- `cargo build --workspace` exits 0.
- `cargo test --workspace --all-features` exits 0 within Phase 19 deferred-flake tolerance (no NEW categories vs `5c8df06a` baseline).
- `cargo fmt --all -- --check` exits 0.
- `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` exits 0 (pre-existing manifest.rs carry-over documented).
- Phase 15 5-row smoke gate passes (or documented-skip).
- `audit_attestation` (ported from 9db06336, +188 LOC) test green.
- VALIDATION.md 22-05-T1, T2, T3 (SHA-256 portion), V1, V3 marked green.
- ALL CLEAN-04 invariants match Task 1 baseline (zero delta — non-touch sentinel held throughout).
- `AUTO_PRUNE_STALE_THRESHOLD = 100` constant unchanged.
- `auto_prune_is_noop_when_sandboxed` test green in BOTH session_commands.rs and session_commands_windows.rs.
- Phase 21 AppliedLabelsGuard lifecycle still green (Task 9).
- End-to-end smoke: `nono run --audit-integrity --audit-sign-key keyring://nono/audit -- echo hi` produces session with populated chain_head + merkle_root + attestation bundle; `nono audit verify <id>` succeeds.
- All 6 upstream cherry-pick commits carry D-19 trailers; the Task-2 manual-port commit carries D-20 template with empirical conflict-baseline citations.
- No `<capture from` placeholders in any commit body.
- `git log origin/main..main` shows zero commits ahead post-Task 11.
- **Boundary discipline held end-to-end:** session_commands*.rs unchanged across the entire plan's commit range; `nono prune` subcommand still works as it did before this plan started; 0 Authenticode/WinVerifyTrust/Win32_Security_WinTrust references in `crates/nono-cli/`.
</verification>

<success_criteria>
- 7 commits on `main`: 1 D-20 manual-port (`4f9552ec` audit-integrity portion) + 6 D-19 cherry-picks (`4ec61c29` → `9db06336`). All DCO-signed.
- Audit-integrity hash-chain + Merkle root populated; DSSE attestation bundle generated; `nono audit verify <id>` succeeds; tampered ledger rejects.
- Cross-platform SHA-256 `ExecutableIdentity` recorded in unified Alpha audit-integrity schema.
- `make ci` green or matches Phase 19 deferred window (no NEW categories vs `5c8df06a`).
- `origin/main` advanced to plan-close HEAD; Plan 22-05b unblocked.
- Plan SUMMARY records all 11 tasks' outcomes, ~7 commit hashes (1 manual-port + 6 cherry-picks), explicit boundary-discipline confirmation, AUD-04/Authenticode/prune-deprecation deferral to 22-05b, and AUD-05 fold-or-split decision deferred to 22-05b's close.
- **Boundary discipline confirmed in SUMMARY:** session_commands*.rs untouched; `nono prune` subcommand still functional with original semantics; 0 Authenticode references; CLEAN-04 invariants identical to pre-plan baseline.
</success_criteria>

<output>
Create `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05a-AUD-CORE-SUMMARY.md` per standard summary template (matching 22-04-OAUTH-SUMMARY.md frontmatter shape). Required sections: Outcome, What was done (one bullet per Task 1–11), Verification table, Files changed table, Commits (~7-row table with hashes + upstream provenance + manual-replay annotation on 4f9552ec), Status, Deferred-to-22-05b (explicit forward references for rename / Authenticode / prune deprecation / audit cleanup peer / formal flush-before-Drop regression test), Threat model coverage (T-22-05a-01..10), Boundary discipline self-check.
</output>
