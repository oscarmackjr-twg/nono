---
phase: 22-upst2-upstream-v038-v040-parity-sync
plan: 05b
type: execute
wave: 3
depends_on: ["22-05a"]
blocks: []
files_modified:
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/audit_commands.rs
  - crates/nono-cli/src/app_runtime.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/exec_identity.rs
  - crates/nono-cli/src/exec_identity_windows.rs
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/tests/exec_identity_windows.rs
  - crates/nono-cli/tests/prune_alias_deprecation.rs
  - crates/nono/src/sandbox/windows.rs
# DELIBERATELY ABSENT (per BLOCKER 2 LOCKED reframe — `auto_prune_if_needed`
# and `AUTO_PRUNE_STALE_THRESHOLD` stay byte-identical; only the CLI dispatch
# layer renames):
#   - crates/nono-cli/src/session_commands.rs
#   - crates/nono-cli/src/session_commands_windows.rs
autonomous: false
requirements: ["AUD-03", "AUD-04"]

must_haves:
  truths:
    - "`nono session cleanup` (renamed at the CLI dispatch layer ONLY — `Cmd::Prune` → `Cmd::SessionCleanup` in `cli.rs`; the fork-internal `auto_prune_if_needed` function and `AUTO_PRUNE_STALE_THRESHOLD = 100` constant in `session_commands.rs` / `session_commands_windows.rs` stay BYTE-IDENTICAL) preserves ALL v2.1 CLEAN-04 invariants: `auto_prune_is_noop_when_sandboxed` test name unchanged, function name unchanged, constant value unchanged, `is_prunable_all_exited_escape_hatch_matches_any_exited` unchanged, `parse_duration_*` family unchanged (AUD-04 acceptance #1, #4, #5)"
    - "`nono audit cleanup --older-than 30d` peer subcommand operates on audit ledgers (AUD-04 acceptance #2)"
    - "`nono prune` hidden alias still works and surfaces a deprecation note (AUD-04 acceptance #3)"
    - "Windows exec-identity records via `GetModuleFileNameW` for path + `WinVerifyTrust` / `CryptCATAdminAcquireContext` for Authenticode signature query; SHA-256 fallback when unsigned or query fails (AUD-03 acceptance #2 + #3)"
    - "Authenticode integration is a SIBLING field on the audit envelope per RESEARCH Contradiction #2 (no mutation of upstream's `ExecutableIdentity` from 22-05a)"
    - "`Win32_Security_WinTrust` feature flag added to `crates/nono-cli/Cargo.toml`'s windows-sys dependency"
    - "All `unsafe` blocks in `exec_identity_windows.rs` carry `// SAFETY:` doc comments per CLAUDE.md § Unsafe Code"
    - "No `.unwrap()` / `.expect()` in production code paths in `exec_identity_windows.rs` per `clippy::unwrap_used`"
    - "**D-04 invariant gate runs AFTER EVERY commit in this plan** (not just at plan close); per the CONTEXT revision LOCKED rule"
    - "**STOP trigger #6 ABSOLUTE**: any post-rename failure of `auto_prune_is_noop_when_sandboxed` reverts the offending commit immediately (sandboxed-agent file-deletion vector reopened)"
    - "Phase 21 `AppliedLabelsGuard` lifecycle survives the rename: ledger flush completes BEFORE guard Drop (formal regression test `applied_labels_guard::audit_flush_before_drop` lands in this plan)"
    - "`cargo test --workspace --all-features` exits 0 on Windows after each commit (D-18); no NEW failures vs the post-22-05a baseline"
  artifacts:
    - path: "crates/nono-cli/src/exec_identity_windows.rs"
      provides: "Windows Authenticode + SHA-256 fallback exec-identity recording (NEW, fork-only ~150 LOC per RESEARCH finding #2; D-17 ALLOWED per CONTEXT line 248)"
    - path: "crates/nono-cli/tests/exec_identity_windows.rs"
      provides: "Authenticode + SHA-256 fallback Windows-only tests (NEW fork-only; `#[cfg(target_os = \"windows\")]`-gated)"
    - path: "crates/nono-cli/tests/prune_alias_deprecation.rs"
      provides: "Verifies `nono prune --help` surfaces the deprecation note (AUD-04 acceptance #3 regression)"
    - path: "crates/nono-cli/src/exec_identity.rs"
      provides: "Platform dispatch hook: on Windows, delegates to `exec_identity_windows::record_exec_identity`; on Unix, returns SHA-256 only"
  key_links:
    - from: "ROADMAP § Phase 22 success criterion #5 (rename + Windows portion)"
      to: "`nono session cleanup` works + `nono audit cleanup` works + `nono prune` deprecation alias works + Windows Authenticode recorded"
      via: "session_commands rename + exec_identity_windows.rs + clap deprecation alias"
      pattern: "session.cleanup|audit.cleanup|deprecat|Authenticode"
    - from: "REQ-AUD-04 acceptance #4 (auto_prune_is_noop_when_sandboxed test passes under both old + new function names)"
      to: "session_commands::auto_prune_is_noop_when_sandboxed (both session_commands.rs and session_commands_windows.rs)"
      via: "rename preserves test name semantics; D-04 gate per commit"
      pattern: "auto_prune_is_noop_when_sandboxed"
    - from: "RESEARCH Pattern 4 (Authenticode FFI sketch)"
      to: "exec_identity_windows::record_exec_identity (file: crates/nono-cli/src/exec_identity_windows.rs)"
      via: "WinVerifyTrust + WINTRUST_DATA + WINTRUST_FILE_INFO; encode_wide for path; // SAFETY: docs"
      pattern: "WinVerifyTrust|encode_wide|WINTRUST"
---

<objective>
Land REQ-AUD-04 (`prune` → `session cleanup` rename + `audit cleanup` peer + `prune` hidden-alias deprecation) and the Windows Authenticode portion of REQ-AUD-03, atop Plan 22-05a's unified audit-integrity Alpha schema. Wave 3, sequential after 22-05a per CONTEXT revision Path B.

This plan has the **highest absolute-stop risk in Phase 22**. T-22-05-04 (rename regresses `auto_prune_is_noop_when_sandboxed`, reopening the sandboxed-agent file-deletion vector) is an ABSOLUTE STOP per CONTEXT STOP trigger #6 — no exceptions, no in-flight reasoning. Every commit that touches the rename surface re-runs the full v2.1 Phase 19 CLEAN-04 invariant suite (per D-04 LOCKED) and any failure reverts the offending commit immediately.

`autonomous: false` — high-risk surfaces (rename invariants + Windows FFI) require checkpoint discipline. The Windows Authenticode addition uses `unsafe` FFI blocks that must carry `// SAFETY:` documentation per CLAUDE.md § Unsafe Code; FFI implementation follows Phase 21's `try_set_mandatory_label` analog (encode_wide + unsafe wrapping + GetLastError → typed error) per PATTERNS.md CONTRADICTION-C and RESEARCH Pattern 4.

Purpose: A Windows user runs `nono session cleanup --older-than 7d` and `nono audit cleanup --older-than 30d` (both new). `nono prune` still works and emits a deprecation note. `nono run --audit-integrity` records the executable's Authenticode signer subject (or `unsigned` + SHA-256 fallback) in the unified Alpha-schema ledger. v2.1 CLEAN-04 invariants are PRESERVED through the rename (sandboxed-agent file-deletion vector remains structurally impossible per Phase 19 CLEAN-04 contract). Phase 21 AppliedLabelsGuard lifecycle survives audit-integrity emissions end-to-end (formal flush-before-Drop regression test added).
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
@.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05a-AUD-CORE-SUMMARY.md
@.planning/phases/19-cleanup/19-CONTEXT.md
@.planning/phases/21-windows-single-file-grants/21-CONTEXT.md
@crates/nono-cli/src/cli.rs
@crates/nono-cli/src/session_commands.rs
@crates/nono-cli/src/session_commands_windows.rs
@crates/nono-cli/src/exec_identity.rs
@crates/nono-cli/Cargo.toml
@crates/nono/src/sandbox/windows.rs

<interfaces>
**This plan's commits (NO upstream cherry-picks; all fork-only):**

| Order | Commit (planned) | REQ | D-17 status |
|-------|------------------|-----|-------------|
| 1 | feat(22-05b): rename `nono prune` → `nono session cleanup`; add `nono audit cleanup` peer (AUD-04 acceptance #1, #2, #4, #5) | AUD-04 | cross-platform; preserves CLEAN-04 invariant test names |
| 2 | feat(22-05b): add hidden `nono prune` deprecation alias surfacing `--help` deprecation note (AUD-04 acceptance #3) | AUD-04 | cross-platform |
| 3 | feat(22-05b): add Windows Authenticode + SHA-256 fallback exec-identity recording (REQ-AUD-03) | AUD-03 | **D-17 ALLOWED** per CONTEXT line 248 — fork-internal addition, not an upstream port |
| 4 | test(22-05b): Windows-only Authenticode + SHA-256 fallback regression suite | AUD-03 | Windows-gated test file |
| 5 | test(22-05b): formal `applied_labels_guard::audit_flush_before_drop` regression (Phase 21 invariant survives audit emissions) | AUD-* | Windows-only sandbox test |

Note: NO `Upstream-commit:` trailers on any commit in this plan. All five carry only `Signed-off-by:` (fork-only additions). Some may decompose into smaller atomic commits; one commit per semantic change per D-19 spirit (e.g., commit 1 may split into "rename function family" + "add audit cleanup peer" if the rename pre-stages cleanly).

**RENAME SURFACE (LOCKED — D-04 gate after EVERY commit per CONTEXT revision):**

The following files form the CLEAN-04 invariant surface. Every commit in this plan that modifies any of these files MUST run the full D-04 gate immediately after staging, BEFORE pushing the commit:

- `crates/nono-cli/src/cli.rs` (clap subcommand definitions: `Cmd::Prune` → `Cmd::SessionCleanup` rename + `AuditCmd::Cleanup` peer + hidden `Cmd::Prune` alias)
- `crates/nono-cli/src/session_commands.rs` (function family: `prune_*` → `session_cleanup_*` rename; preserve `auto_prune_is_noop_when_sandboxed` test name AND verify the renamed function still has the `if env::var_os("NONO_CAP_FILE").is_some() { return; }` early-return as its first statement per REQ-AUD-04 enforcement clause)
- `crates/nono-cli/src/session_commands_windows.rs` (Windows duplicate of the same; preserve `auto_prune_is_noop_when_sandboxed` Windows test)
- `crates/nono-cli/src/session_commands_unix.rs` (if present — same discipline)
- `crates/nono-cli/src/main.rs` / `app_runtime.rs` / `cli_bootstrap.rs` (subcommand dispatch wiring — adjust for renamed Cmd variant + alias)

**CRITICAL: REQ-AUD-04 acceptance #4 says** "Regression: `auto_prune_is_noop_when_sandboxed` test passes under both old + new function names." Interpretation: the test NAME stays `auto_prune_is_noop_when_sandboxed` (do NOT rename the test). The function it covers gets renamed to `auto_session_cleanup_is_noop_when_sandboxed` (or kept as `auto_prune_*` internally — implementation choice), but the TEST asserts the same behavior. This preserves the v2.1 Phase 19 contract surface verbatim.

**CRITICAL: REQ-AUD-04 enforcement clause says** the renamed function "retains the `if env::var_os(\"NONO_CAP_FILE\").is_some() { return; }` early-return as its first statement." This is the structural invariant that mitigates T-22-05-04 (sandboxed-agent file-deletion vector). It is a STOP trigger #6 ABSOLUTE if the renamed function loses this early-return.

**FORK-ONLY AUTHENTICODE ADDITION (D-17 ALLOWED):**

- File: `crates/nono-cli/src/exec_identity_windows.rs` (NEW; ~150 LOC fork-only)
- Cargo.toml: add `Win32_Security_WinTrust` to existing `windows-sys 0.59` features list in `crates/nono-cli/Cargo.toml` (NOT `crates/nono/Cargo.toml` — Authenticode lives in nono-cli only per RESEARCH § Standard Stack)
- FFI style: follow Phase 21's `try_set_mandatory_label` analog (PATTERNS.md CONTRADICTION-C):
  - `encode_wide` UTF-16 conversion for `WINTRUST_FILE_INFO::pcwszFilePath`
  - `unsafe { ... }` blocks with `// SAFETY:` doc comments on every block
  - RAII `_close_guard` pattern for the `WTD_STATEACTION_CLOSE` second `WinVerifyTrust` call
  - `GetLastError` → typed `NonoError` (no `.unwrap()` / `.expect()`)
- Schema integration: add `audit_authenticode: Option<AuthenticodeStatus>` as a SIBLING field on the audit envelope (per RESEARCH Contradiction #2: do NOT mutate upstream's `ExecutableIdentity` struct shape)
- Platform dispatch in `exec_identity.rs`:
  ```
  #[cfg(target_os = "windows")]
  pub fn platform_authenticode(path: &Path) -> Option<AuthenticodeStatus> {
      exec_identity_windows::query_authenticode_status(path).ok()
  }

  #[cfg(not(target_os = "windows"))]
  pub fn platform_authenticode(_path: &Path) -> Option<AuthenticodeStatus> {
      None  // SHA-256-only path on Unix
  }
  ```

**RESEARCH Pattern 4 cheat-sheet for Authenticode** (RESEARCH.md lines 356–419, summarized):
- `GetModuleFileNameW(NULL, buf, MAX_PATH)` for the running binary's path
- `WINTRUST_FILE_INFO { pcwszFilePath: encode_wide(path), ... }`
- `WINTRUST_DATA { dwUIChoice: WTD_UI_NONE, fdwRevocationChecks: WTD_REVOKE_NONE | WTD_REVOKE_WHOLECHAIN per acceptance, dwUnionChoice: WTD_CHOICE_FILE, pFile: &file_info, dwStateAction: WTD_STATEACTION_VERIFY, ... }`
- `WinVerifyTrust(NULL, &WINTRUST_ACTION_GENERIC_VERIFY_V2, &mut wtd as *mut _ as *mut c_void)`
- Map result codes: `0 → AuthenticodeStatus::Valid { signer_chain }`, `0x80092009 → AuthenticodeStatus::Unsigned`, other → `AuthenticodeStatus::InvalidSignature { hresult }`
- RAII guard for `WTD_STATEACTION_CLOSE` second call (always run on Drop)
- On any FFI failure: SHA-256 fallback (call into the existing `exec_identity::sha256_of_file` from 22-05a)

**D-04 invariant gate command (run AFTER EVERY commit in this plan — LOCKED per CONTEXT revision):**
```
cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited
cargo test -p nono-cli cli::parse_duration_
grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs
```
**Expected:** All four pass identically to Plan-22-05a-close baseline (which itself matched the pre-Phase-22 baseline).

**D-04 exemption — doc-only commits (WARNING fix):** The D-04 invariant gate runs after every source-code commit in this plan. **SUMMARY commits are exempt** because they cannot affect the rename surface or any source file — the SUMMARY is a `.planning/phases/...-SUMMARY.md` artifact under `.planning/`, which is outside the Rust workspace and outside the CLEAN-04 invariant surface. Task 8's SUMMARY commit therefore does NOT need to run the D-04 gate; the gate ran after Task 7 (the immediately preceding source-code commit, which is the D-18 Windows-regression gate's commit-or-no-commit close-out).

**Failure handling (LOCKED):**
- ANY CLEAN-04 invariant fails → STOP per CONTEXT STOP trigger #5; revert that commit immediately; re-discuss before retrying
- `auto_prune_is_noop_when_sandboxed` fails → **ABSOLUTE STOP per CONTEXT STOP trigger #6**; no further commits in this plan; revert the offending commit; sandboxed-agent file-deletion vector has been reopened until the regression is understood and fixed

**STOP triggers from CONTEXT § Specifics that fire in this plan:**
- #1: Touch any `*_windows.rs` file other than the planned `exec_identity_windows.rs` (NEW) and `session_commands_windows.rs` (rename) → ABORT and investigate. (Note: `session_commands_windows.rs` IS in scope for the rename per the LOCKED-OUT-then-LOCKED-IN flip vs Plan 22-05a; this is the one Phase-22 file where its rename DOES land.)
- #2: `make ci` red, root cause unclear in 30 min → STOP
- #3: A single commit's diff exceeds ~400 lines → consider splitting (the rename commit may approach this; Authenticode commit at ~150 LOC is well under)
- #4: Phase 15 5-row smoke gate fails → REVERT and re-scope
- #5: ANY CLEAN-04 invariant test fails after a commit → REVERT that commit and re-discuss
- #6 (ABSOLUTE STOP): `auto_prune_is_noop_when_sandboxed` test failure → sandboxed-agent file-deletion vector reopened; ABSOLUTE STOP

**Reusable fork assets (per CONTEXT § Reusable Assets + Plan 22-05a artifacts):**
- 22-05a's unified Alpha schema in `crates/nono-cli/src/audit_integrity.rs` (target for `audit_authenticode` sibling field)
- 22-05a's cross-platform `ExecutableIdentity` struct in `exec_identity.rs` (target for platform dispatch)
- Phase 21's `try_set_mandatory_label` FFI style in `crates/nono/src/sandbox/windows.rs` (Authenticode FFI analog)
- `windows-sys 0.59` already in `crates/nono-cli/Cargo.toml` features list — only `Win32_Security_WinTrust` to be added (RESEARCH finding #8)

**No `Upstream-commit:` trailers on any commit in this plan** — all are fork-only additions per CONTEXT line 248 ("D-17 ALLOWED here") for the Authenticode work, and per the v2.1 Phase 19 CLEAN-04 contract for the rename (which upstream `4f9552ec` bundled with audit-integrity but is fork-relevant on its own merits).

**Boundary discipline reminder:** Plan 22-05a's plan-close gate verified that `nono prune` still functions with original semantics. This plan's first commit is the rename — the moment that flips. Every subsequent commit re-verifies (via D-04 gate) that the renamed surface still satisfies the v2.1 Phase 19 CLEAN-04 contract.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: D-04 baseline — capture CLEAN-04 invariant + AppliedLabelsGuard pre-state on top of Plan 22-05a HEAD (READ-ONLY)</name>
  <files>(read-only audit — no files modified)</files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05a-AUD-CORE-SUMMARY.md (Plan 22-05a close state)
    - .planning/phases/19-cleanup/19-CONTEXT.md (CLEAN-04 invariant origin)
    - crates/nono-cli/src/session_commands.rs (auto_prune_is_noop_when_sandboxed test, AUTO_PRUNE_STALE_THRESHOLD constant)
    - crates/nono-cli/src/session.rs (is_prunable_all_exited_escape_hatch_matches_any_exited test)
    - crates/nono-cli/src/cli.rs (parse_duration tests at lines 2454-2472; current `Cmd::Prune` definition)
    - crates/nono-cli/src/session_commands_windows.rs (Windows duplicate of auto_prune_is_noop)
    - crates/nono/src/sandbox/windows.rs::tests applied_labels_guard:: module (Phase 21 lifecycle)
  </read_first>
  <action>
    1. Verify HEAD is at Plan 22-05a close (`22-05a-AUD-CORE-SUMMARY.md` exists; SUMMARY frontmatter records the plan-close SHA). Capture: `git rev-parse HEAD > /tmp/22-05b-pre-head.txt`.

    2. Run the full CLEAN-04 invariant suite and capture baseline (must be all green pre-22-05b — Plan 22-05a's plan-close gate verified the same):
       ```
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed 2>&1 | tee /tmp/22-05b-baseline-cleanup.log
       cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited 2>&1 | tee -a /tmp/22-05b-baseline-cleanup.log
       cargo test -p nono-cli cli::parse_duration_ 2>&1 | tee -a /tmp/22-05b-baseline-cleanup.log
       ```

    3. Verify the structural invariants (must hold pre-rename):
       ```
       grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs
       grep -nE 'fn auto_prune_is_noop_when_sandboxed' crates/nono-cli/src/session_commands.rs crates/nono-cli/src/session_commands_windows.rs
       grep -E 'Cmd::Prune|fn prune|"prune"' crates/nono-cli/src/cli.rs | head -5
       grep -nE 'NONO_CAP_FILE.*is_some.*return' crates/nono-cli/src/session_commands.rs crates/nono-cli/src/session_commands_windows.rs
       ```
       All must succeed: constant present; test exists in BOTH files; `Cmd::Prune` subcommand defined; the `if env::var_os("NONO_CAP_FILE").is_some() { return; }` early-return is the first statement of `auto_prune_if_needed` in BOTH files.

    4. If ANY CLEAN-04 invariant fails baseline: STOP. Plan 22-05a was supposed to leave these green; investigate plan-22-05a-close drift before starting 22-05b.

    5. Verify Phase 21 `AppliedLabelsGuard` lifecycle still green:
       ```
       cargo test -p nono --test sandbox_windows applied_labels_guard:: 2>&1 | tee /tmp/22-05b-baseline-aipc.log
       ```

    6. Capture the `nono prune --help` output as a pre-state snapshot (will compare in Task 3):
       ```
       cargo run -p nono-cli -- prune --help > /tmp/22-05b-pre-prune-help.txt 2>&1
       ```

    7. Record baseline pass counts in preflight note for SUMMARY.
  </action>
  <verify>
    <automated>cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed &amp;&amp; cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited &amp;&amp; cargo test -p nono-cli cli::parse_duration_ &amp;&amp; grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs &amp;&amp; cargo test -p nono --test sandbox_windows applied_labels_guard::</automated>
  </verify>
  <acceptance_criteria>
    - All 3 CLEAN-04 invariant test groups exit 0 (pre-rename baseline).
    - `AUTO_PRUNE_STALE_THRESHOLD = 100` constant present.
    - `auto_prune_is_noop_when_sandboxed` test exists in BOTH session_commands.rs and session_commands_windows.rs.
    - `Cmd::Prune` subcommand still defined in cli.rs.
    - `NONO_CAP_FILE.is_some() return` early-return is the first statement of `auto_prune_if_needed` in BOTH session_commands.rs and session_commands_windows.rs.
    - Phase 21 AppliedLabelsGuard tests green.
    - Baseline counts + pre-state snapshots recorded for SUMMARY.
  </acceptance_criteria>
  <done>
    Baseline established on top of Plan 22-05a HEAD. Plan 22-05b may proceed.
  </done>
</task>

<task type="auto">
  <name>Task 2: Apply `4f9552ec`'s rename portion ONLY — `prune` → `session cleanup` + `audit cleanup` peer (AUD-04 acceptance #1, #2, #4, #5)</name>
  <files>
    crates/nono-cli/src/cli.rs
    crates/nono-cli/src/audit_commands.rs
    crates/nono-cli/src/main.rs
    crates/nono-cli/src/app_runtime.rs
    # DELIBERATELY ABSENT per LOCKED reframe (BLOCKER 2 fix):
    # - crates/nono-cli/src/session_commands.rs        (auto_prune_if_needed stays BYTE-IDENTICAL)
    # - crates/nono-cli/src/session_commands_windows.rs (Windows duplicate stays BYTE-IDENTICAL)
  </files>
  <read_first>
    - `git show 4f9552ec --name-only` (verify EMPIRICALLY which files upstream `4f9552ec` modifies; `session_commands.rs` and `session_commands_windows.rs` MUST NOT appear in the output. They are fork-only files added in v2.1 Phase 19 CLEAN-04; upstream has never seen them.)
      Verification command: `git show 4f9552ec --name-only | grep -E 'session_commands' && echo UNEXPECTED || echo OK` — expected output: `OK`.
    - `git show 4f9552ec -- crates/nono-cli/src/cli.rs` (extract ONLY the `Cmd::Prune` → `Cmd::SessionCleanup` rename hunks; IGNORE all `--audit-integrity` / `--audit-sign-key` / hash-chain hunks — those landed in 22-05a Task 2)
    - `git show 4f9552ec -- crates/nono-cli/src/audit_commands.rs` (extract the new `AuditCmd::Cleanup` variant + dispatch; this peer subcommand was deliberately deferred from 22-05a)
    - `git show 4f9552ec -- crates/nono-cli/src/main.rs crates/nono-cli/src/app_runtime.rs` (subcommand dispatch wiring updates)
    - .planning/REQUIREMENTS.md § AUD-04 acceptance criteria #1, #2, #4, #5
    - .planning/phases/19-cleanup/19-CONTEXT.md (CLEAN-04 contract — preserves T-19-04-07 mitigation)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md `<revision>` block (D-04 gate per commit; STOP trigger #6 ABSOLUTE)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05b-AUD-RENAME-PLAN.md `<interfaces>` line 116-118 ("kept as `auto_prune_*` internally — implementation choice"). The LOCKED reframe formalizes that choice: `auto_prune_if_needed` and `AUTO_PRUNE_STALE_THRESHOLD = 100` stay BYTE-IDENTICAL; only the CLI dispatch layer renames.
  </read_first>
  <action>
    **This task touches the CLI-dispatch rename surface ONLY. The fork's `auto_prune_if_needed` function and `AUTO_PRUNE_STALE_THRESHOLD = 100` constant in `session_commands.rs` / `session_commands_windows.rs` stay BYTE-IDENTICAL (LOCKED reframe per BLOCKER 2). The D-04 gate runs AFTER staging and BEFORE the commit ships. ABSOLUTE STOP if `auto_prune_is_noop_when_sandboxed` fails.**

    **Why this scoping:** lowest CLEAN-04 surface drift; aligns with the existing `<interfaces>` line 116-118 note ("kept as `auto_prune_*` internally — implementation choice"); eliminates the cross-platform-rename risk surface entirely on `session_commands*.rs`; STOP trigger #6 mitigation becomes trivially provable (no rename = test name + function name + constant all unchanged byte-for-byte).

    1. Capture pre-task SHA + empirical-upstream verification:
       ```
       git rev-parse HEAD > /tmp/22-05b-task2-pre.txt
       git show 4f9552ec --name-only | grep -E 'session_commands(_windows)?\.rs' && echo UNEXPECTED || echo OK
       ```
       Expected output ends with `OK`. If `UNEXPECTED`: STOP — the empirical-baseline assumption underpinning this LOCKED reframe is broken; re-discuss before proceeding.

    2. Capture pre-task byte-for-byte hashes of the LOCKED-OUT internal surface (these MUST match post-commit):
       ```
       git rev-parse HEAD:crates/nono-cli/src/session_commands.rs > /tmp/22-05b-task2-sc-blob-pre.txt
       git rev-parse HEAD:crates/nono-cli/src/session_commands_windows.rs > /tmp/22-05b-task2-scw-blob-pre.txt
       cat /tmp/22-05b-task2-sc-blob-pre.txt /tmp/22-05b-task2-scw-blob-pre.txt
       ```

    3. **Apply changes — split into two clearly-labeled subsections:**

       **3.A — Upstream-driven changes (cherry-pick the rename hunks from `4f9552ec`):**
       - In `cli.rs`: rename `Cmd::Prune { ... }` → `Cmd::SessionCleanup { ... }` (use upstream's exact variant name). Read `git show 4f9552ec -- crates/nono-cli/src/cli.rs` and apply ONLY the rename hunks; the `--audit-integrity` / `--audit-sign-key` hunks already landed in 22-05a.
       - In `audit_commands.rs`: add a NEW `AuditCmd::Cleanup { ... }` variant under the existing `audit` subcommand tree (Plan 22-05a already added `AuditCmd::Verify` to this same dispatch tree; this peer joins it).
       - In `main.rs` / `app_runtime.rs` / `cli_bootstrap.rs`: update subcommand dispatch to route `Cmd::SessionCleanup` to the existing `auto_prune_if_needed`/`prune_*` handler family (which is unchanged — see 3.B). Route `AuditCmd::Cleanup` to the new audit-cleanup handler.

       **3.B — Fork-internal CLEAN-04 surface (LOCKED — DO NOT TOUCH):**
       - **`crates/nono-cli/src/session_commands.rs` is byte-identical pre vs post this commit.** No edits, no whitespace changes, no comment changes.
       - **`crates/nono-cli/src/session_commands_windows.rs` is byte-identical pre vs post this commit.** Same discipline.
       - The `auto_prune_if_needed` function name is preserved.
       - `AUTO_PRUNE_STALE_THRESHOLD = 100` constant is preserved verbatim.
       - The `if env::var_os("NONO_CAP_FILE").is_some() { return; }` early-return is preserved verbatim.
       - The `auto_prune_is_noop_when_sandboxed` test name is preserved verbatim (REQ-AUD-04 acceptance #4 satisfied trivially: same test name, internal function unchanged, behavior preserved).
       - The `Cmd::SessionCleanup` handler in 3.A wraps the unchanged `auto_prune_if_needed` function (the rename happens at the dispatch layer; the worker function keeps its original name).

    4. **Boundary check 1 — verify the LOCKED-OUT files are byte-identical to pre-commit state:**
       ```
       git diff --stat crates/nono-cli/src/session_commands.rs crates/nono-cli/src/session_commands_windows.rs
       ```
       MUST output empty (no diff). If non-empty: REVERT (`git checkout -- crates/nono-cli/src/session_commands.rs crates/nono-cli/src/session_commands_windows.rs`) and re-do step 3 with strict adherence to 3.B's LOCKED discipline. ABSOLUTE STOP trigger #6 mitigation depends on these two files staying untouched in this commit.

    5. **Boundary check 2 — verify the fork-internal invariants are byte-identical (defense-in-depth grep guards):**
       ```
       grep -E '^fn auto_prune_if_needed' crates/nono-cli/src/session_commands.rs
       grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs
       grep -nE 'fn auto_prune_is_noop_when_sandboxed' crates/nono-cli/src/session_commands.rs crates/nono-cli/src/session_commands_windows.rs
       grep -nE 'NONO_CAP_FILE.*is_some.*return' crates/nono-cli/src/session_commands.rs crates/nono-cli/src/session_commands_windows.rs
       ```
       Expected: function name present, constant present (value `100`), test name present in BOTH files, early-return present in BOTH files. ALL must hold (this should be trivial because we did not touch those files).

    6. Build:
       ```
       cargo build --workspace
       ```

    7. **D-04 gate (BEFORE commit) — full CLEAN-04 invariant suite:**
       ```
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
       cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited
       cargo test -p nono-cli cli::parse_duration_
       grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs
       ```
       **ALL FOUR MUST PASS** (and they should pass trivially — the LOCKED reframe means the worker function and its test are byte-identical; only the CLI dispatch layer changed). If `auto_prune_is_noop_when_sandboxed` fails: ABSOLUTE STOP per STOP trigger #6 (`git checkout -- .` immediately; do not commit; re-discuss — a non-touch sentinel failure under the LOCKED reframe is a structural surprise). If any other invariant fails: STOP per STOP trigger #5.

    8. **Smoke-test the renamed CLI surface:**
       ```
       cargo run -p nono-cli -- session cleanup --help 2>&1 | head -10
       cargo run -p nono-cli -- audit cleanup --help 2>&1 | head -10
       ```
       Both must list the subcommands without error. (At this point `nono prune` will be UNDEFINED — the alias ships in Task 3.)

    9. Stage and commit:
       ```
       git add crates/nono-cli/src/cli.rs \
               crates/nono-cli/src/audit_commands.rs \
               crates/nono-cli/src/main.rs \
               crates/nono-cli/src/app_runtime.rs
       # Verify session_commands*.rs is NOT staged (LOCKED reframe boundary):
       git status crates/nono-cli/src/session_commands.rs crates/nono-cli/src/session_commands_windows.rs
       # Expected: each file shown as unmodified / not staged. If staged, unstage with `git restore --staged <file>`.
       git status
       git commit -s -m "$(cat <<'EOF'
       feat(22-05b): rename `nono prune` -> `nono session cleanup` at CLI dispatch layer; add `nono audit cleanup` peer (AUD-04)

       Renames the CLI-dispatch surface ONLY:
       - cli.rs: Cmd::Prune -> Cmd::SessionCleanup
       - audit_commands.rs: NEW AuditCmd::Cleanup peer (joins AuditCmd::Verify
         landed in 22-05a Task 6)
       - main.rs / app_runtime.rs: dispatch wiring updates

       The fork-internal worker function `auto_prune_if_needed` and the
       `AUTO_PRUNE_STALE_THRESHOLD = 100` constant in
       session_commands.rs / session_commands_windows.rs are BYTE-IDENTICAL
       to pre-commit state. The Cmd::SessionCleanup handler wraps the
       unchanged auto_prune_if_needed function.

       Why this scoping (LOCKED reframe per BLOCKER 2):
       - Lowest CLEAN-04 surface drift: zero edits to session_commands*.rs
       - REQ-AUD-04 acceptance #4 satisfied trivially: same test name,
         internal function unchanged, behavior preserved
       - Eliminates cross-platform rename risk surface entirely
       - STOP trigger #6 (auto_prune_is_noop_when_sandboxed regression)
         mitigated by structural impossibility — the test cannot regress
         because nothing it covers was touched

       D-04 gate run BEFORE this commit: all four CLEAN-04 invariants green;
       ABSOLUTE STOP trigger #6 guard held trivially (no surface touched).

       Hidden `nono prune` deprecation alias ships in the next commit (Task 3
       per REQ-AUD-04 acceptance #3).

       Reference: upstream 4f9552ec rename hunks (audit-integrity portion of
       4f9552ec landed in Plan 22-05a Task 2 manual replay; the
       prune->session-cleanup rename was deliberately deferred to this plan).

       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    10. **D-04 gate (AFTER commit) — re-verify on the committed state, plus the boundary-was-held verification:**
       ```
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
       cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited
       cargo test -p nono-cli cli::parse_duration_
       # Boundary verification: this commit must NOT have touched session_commands*.rs
       git show HEAD --name-only | grep -E 'session_commands(_windows)?\.rs' && echo BOUNDARY_BREACH || echo OK
       # Expected: the grep returns 0 lines and the echo prints "OK"
       # Fork-internal byte-identical proof:
       grep -E '^fn auto_prune_if_needed' crates/nono-cli/src/session_commands.rs
       grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs
       ```
       If ANY fails post-commit: revert with `git reset --hard HEAD~1` and re-discuss. ABSOLUTE STOP if `auto_prune_is_noop_when_sandboxed` fails OR if `BOUNDARY_BREACH` printed.
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed &amp;&amp; cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited &amp;&amp; cargo test -p nono-cli cli::parse_duration_ &amp;&amp; grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs &amp;&amp; grep -E 'fn auto_prune_if_needed' crates/nono-cli/src/session_commands.rs &amp;&amp; grep -nE 'NONO_CAP_FILE.*is_some.*return' crates/nono-cli/src/session_commands.rs crates/nono-cli/src/session_commands_windows.rs &amp;&amp; test -z "$(git show HEAD --name-only | grep -E 'session_commands(_windows)?\.rs')" &amp;&amp; cargo run -p nono-cli -- session cleanup --help 2&gt;&amp;1 | head &amp;&amp; cargo run -p nono-cli -- audit cleanup --help 2&gt;&amp;1 | head</automated>
  </verify>
  <acceptance_criteria>
    - `nono session cleanup --help` lists the subcommand without error (AUD-04 acceptance #1).
    - `nono audit cleanup --help` lists the subcommand without error (AUD-04 acceptance #2).
    - **`git show HEAD --name-only | grep -E 'session_commands(_windows)?\.rs' returns 0 lines** (Task 2's commit MUST NOT touch session_commands files; LOCKED-reframe boundary verification — BLOCKER 2 fix).
    - **`grep -E 'fn auto_prune_if_needed' crates/nono-cli/src/session_commands.rs` returns 1 line** (function name preserved exactly — fork-internal worker BYTE-IDENTICAL).
    - **`grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs` returns 1 line** (constant preserved exactly).
    - `auto_prune_is_noop_when_sandboxed` test name PRESERVED in BOTH session_commands.rs and session_commands_windows.rs (AUD-04 acceptance #4 satisfied trivially under LOCKED reframe).
    - `NONO_CAP_FILE.is_some() return` early-return PRESERVED as first statement of `auto_prune_if_needed` in BOTH files (AUD-04 enforcement clause; T-22-05-04 mitigation — preserved trivially under LOCKED reframe).
    - `--older-than 30` (no suffix) still fails with the CLEAN-04 migration hint (AUD-04 acceptance #5; covered by `parse_duration_*` test family).
    - **D-04 gate (BEFORE commit) green; D-04 gate (AFTER commit) green.** Per CONTEXT revision LOCKED rule.
    - Phase 21 AppliedLabelsGuard tests still green.
    - `cargo build --workspace` exits 0.
    - Commit body contains the explicit D-04-gate-passed declaration AND the BLOCKER-2 LOCKED-reframe rationale.
  </acceptance_criteria>
  <done>
    Rename complete. CLEAN-04 invariants preserved through the rename. STOP trigger #6 has not fired. AUD-04 acceptance #1, #2, #4, #5 satisfied. Acceptance #3 (hidden alias + deprecation note) reserved for Task 3.
  </done>
</task>

<task type="auto">
  <name>Task 3: Add hidden `nono prune` deprecation alias surfacing `--help` deprecation note (AUD-04 acceptance #3)</name>
  <files>
    crates/nono-cli/src/cli.rs
    crates/nono-cli/src/main.rs (or app_runtime.rs — wherever subcommand dispatch lives)
  </files>
  <read_first>
    - .planning/REQUIREMENTS.md § AUD-04 acceptance #3 ("`nono prune` (hidden alias) still works and surfaces a deprecation note")
    - crates/nono-cli/src/cli.rs (post-Task-2 state — find the new `Cmd::SessionCleanup` variant)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § Claude's Discretion ("`prune` alias deprecation timeline" — survival horizon is a v2.3-milestone decision, not Phase 22)
  </read_first>
  <action>
    1. Add a hidden `Cmd::Prune` variant in `cli.rs` that aliases to the renamed `Cmd::SessionCleanup` semantics. Use clap's `#[command(hide = true, after_help = "DEPRECATED: use `nono session cleanup` instead.")]` attribute, OR the documented clap alias mechanism. The flag set should match `Cmd::SessionCleanup` (e.g., `--older-than`, `--all-exited`).

    2. In the dispatcher (likely `main.rs` or `app_runtime.rs`), when `Cmd::Prune` is matched: emit a stderr deprecation note ONCE (`eprintln!("warning: `nono prune` is deprecated; use `nono session cleanup` instead")`) and then delegate to the same handler as `Cmd::SessionCleanup`.

    3. Smoke-test:
       ```
       cargo run -p nono-cli -- prune --help 2>&1 | head -20
       cargo run -p nono-cli -- prune --help 2>&1 | grep -i 'deprecat'
       cargo run -p nono-cli -- prune --all-exited 2>&1 | head -5
       ```
       Expected: `--help` output contains the word "deprecated" (case-insensitive); the actual `nono prune --all-exited` invocation still functions (delegating to session cleanup logic).

    4. Add a regression test `crates/nono-cli/tests/prune_alias_deprecation.rs`:
       ```rust
       // crates/nono-cli/tests/prune_alias_deprecation.rs
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

    5. Run the new test:
       ```
       cargo test -p nono-cli --test prune_alias_deprecation
       ```

    6. **D-04 gate (BEFORE commit) — full CLEAN-04 invariant suite (this commit touches cli.rs which is part of the rename surface):**
       ```
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
       cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited
       cargo test -p nono-cli cli::parse_duration_
       grep -E '^const (AUTO_PRUNE_STALE_THRESHOLD|AUTO_CLEANUP_STALE_THRESHOLD): usize = 100' crates/nono-cli/src/session_commands.rs
       ```

    7. Stage and commit:
       ```
       git add crates/nono-cli/src/cli.rs \
               crates/nono-cli/src/main.rs \
               crates/nono-cli/tests/prune_alias_deprecation.rs
       git commit -s -m "$(cat <<'EOF'
       feat(22-05b): surface deprecation note on `nono prune` hidden alias (AUD-04 #3)

       Hidden `nono prune` alias delegates to `nono session cleanup` and
       emits a stderr deprecation note. `--help` output explicitly says
       DEPRECATED. Lifetime of the alias deferred to v2.3-milestone scoping
       per CONTEXT Claude's Discretion bullet.

       Test: tests/prune_alias_deprecation.rs verifies --help output
       contains "deprecated" (case-insensitive).

       D-04 gate run BEFORE this commit shipped: CLEAN-04 invariants green.

       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    8. **D-04 gate (AFTER commit) — re-verify:**
       ```
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
       cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited
       cargo test -p nono-cli cli::parse_duration_
       ```
  </action>
  <verify>
    <automated>cargo run -p nono-cli -- prune --help 2&gt;&amp;1 | grep -i 'deprecat' &amp;&amp; cargo test -p nono-cli --test prune_alias_deprecation &amp;&amp; cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed &amp;&amp; cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited &amp;&amp; cargo test -p nono-cli cli::parse_duration_</automated>
  </verify>
  <acceptance_criteria>
    - `cargo run -p nono-cli -- prune --help` output contains the word "deprecated" (case-insensitive).
    - `cargo run -p nono-cli -- prune --all-exited` (or equivalent) still functions and emits a stderr deprecation note.
    - `tests/prune_alias_deprecation.rs::prune_alias_surfaces_deprecation_note` passes.
    - **D-04 gate (BEFORE commit) green; D-04 gate (AFTER commit) green.**
    - VALIDATION 22-05-V2 test name (`prune_alias_deprecation_note`) green.
  </acceptance_criteria>
  <done>
    Hidden `nono prune` deprecation alias landed. AUD-04 acceptance #3 satisfied. AUD-04 fully closed.
  </done>
</task>

<task type="auto">
  <name>Task 4: Add Windows Authenticode + SHA-256 fallback exec-identity recording (AUD-03 Windows portion — D-17 ALLOWED fork-only ~150 LOC)</name>
  <files>
    crates/nono-cli/src/exec_identity_windows.rs (NEW — ~150 LOC)
    crates/nono-cli/src/exec_identity.rs (extend with platform dispatch + sibling AuthenticodeStatus field)
    crates/nono-cli/src/audit_commands.rs (re-export AuthenticodeStatus if needed)
    crates/nono-cli/Cargo.toml (add Win32_Security_WinTrust feature flag)
  </files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-RESEARCH.md § Code Examples Pattern 4 (Authenticode FFI sketch lines 356–419)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-RESEARCH.md Contradiction #2 (sibling field, no mutation of upstream's ExecutableIdentity)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-PATTERNS.md CONTRADICTION-C (FFI style analog: try_set_mandatory_label in sandbox/windows.rs)
    - crates/nono/src/sandbox/windows.rs (lines 514–600 — encode_wide + unsafe wrapping + // SAFETY: docs + RAII guard + GetLastError → typed NonoError)
    - crates/nono-cli/Cargo.toml (windows-sys feature list — current 17 features per RESEARCH line 145)
    - CLAUDE.md § Unsafe Code ("Restrict to FFI; must be wrapped in safe APIs with `// SAFETY:` docs")
    - CLAUDE.md § Coding Standards (no `.unwrap()` / `.expect()`; `#[must_use]` on critical Results; `zeroize` for sensitive data — N/A here)
  </read_first>
  <action>
    1. Capture pre-task SHA: `git rev-parse HEAD > /tmp/22-05b-task4-pre.txt`.

    2. Add the `Win32_Security_WinTrust` feature flag to `crates/nono-cli/Cargo.toml`:
       ```
       grep -n 'windows-sys' crates/nono-cli/Cargo.toml
       # Edit: add "Win32_Security_WinTrust" to the existing features array.
       # Empirical baseline: current count = 16 features (NOTE: RESEARCH finding #8 said
       # "current 17 features" but empirical `grep -oE '"Win32_[A-Za-z_]+"' crates/nono-cli/Cargo.toml | wc -l`
       # against post-22-04 HEAD returns 16 — proceed with empirical 16 -> 17 expansion).
       ```
       Verify after edit:
       ```
       grep -E 'Win32_Security_WinTrust' crates/nono-cli/Cargo.toml
       ```

       **Cargo.toml feature preservation guard (WARNING fix) — loop-grep ALL existing features must remain present.** All 16 pre-existing features + the new `Win32_Security_WinTrust` = 17 features total post-edit:
       ```
       FEATURES=(Win32_Foundation Win32_NetworkManagement_WindowsFilteringPlatform Win32_Networking_WinSock Win32_Security Win32_Security_Authorization Win32_Storage_FileSystem Win32_System_Console Win32_System_Diagnostics_Etw Win32_System_EventLog Win32_System_JobObjects Win32_System_Memory Win32_System_Pipes Win32_System_Rpc Win32_System_Services Win32_System_SystemServices Win32_System_Threading Win32_Security_WinTrust)
       MISSING=()
       for f in "${FEATURES[@]}"; do
         grep -qE "\"$f\"" crates/nono-cli/Cargo.toml || MISSING+=("$f")
       done
       echo "missing=${MISSING[@]}"
       ```
       The `missing=` line MUST print empty (no features missing). If any feature is missing: REVERT the Cargo.toml edit and re-do (the edit accidentally dropped a pre-existing feature). NOTE: if the empirical pre-edit baseline differs from 16 (e.g. a prior plan added another Win32_* feature), update the FEATURES array accordingly before running the guard.

    3. Create `crates/nono-cli/src/exec_identity_windows.rs` (~150 LOC) following Phase 21's `try_set_mandatory_label` FFI style and RESEARCH Pattern 4:
       ```rust
       //! Windows Authenticode exec-identity recording (REQ-AUD-03 acceptance #2/#3).
       //! Fork-only addition per CONTEXT § Integration Points line 248 (D-17 ALLOWED).
       //! FFI style mirrors crates/nono/src/sandbox/windows.rs::try_set_mandatory_label
       //! (encode_wide UTF-16, unsafe wrapping with // SAFETY: docs, RAII guard for
       //! WTD_STATEACTION_CLOSE, GetLastError -> typed NonoError).

       #![cfg(target_os = "windows")]

       use crate::error::NonoError;
       use std::path::Path;
       use std::os::windows::ffi::OsStrExt;
       use windows_sys::Win32::Security::WinTrust::*;
       use windows_sys::Win32::Foundation::{GetLastError, TRUST_E_NOSIGNATURE};

       /// Authenticode status for an executable.
       /// Sibling field on the audit envelope per RESEARCH Contradiction #2 -
       /// does NOT mutate upstream's ExecutableIdentity struct shape.
       #[derive(Debug, Clone)]
       pub enum AuthenticodeStatus {
           Valid { signer_subject: String, thumbprint: String },
           Unsigned,
           InvalidSignature { hresult: i32 },
           QueryFailed { reason: String },
       }

       /// Record exec-identity Authenticode status for `path`.
       /// On any FFI failure: returns AuthenticodeStatus::QueryFailed; the SHA-256
       /// fallback is recorded by the caller via the existing
       /// exec_identity::sha256_of_file path.
       #[must_use]
       pub fn query_authenticode_status(path: &Path) -> Result<AuthenticodeStatus, NonoError> {
           // Convert path to UTF-16 (mirrors sandbox/windows.rs::try_set_mandatory_label)
           let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();

           let file_info = WINTRUST_FILE_INFO {
               cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
               pcwszFilePath: wide.as_ptr(),
               hFile: 0,
               pgKnownSubject: std::ptr::null_mut(),
           };

           let mut wtd = WINTRUST_DATA {
               cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
               pPolicyCallbackData: std::ptr::null_mut(),
               pSIPClientData: std::ptr::null_mut(),
               dwUIChoice: WTD_UI_NONE,
               fdwRevocationChecks: WTD_REVOKE_NONE, // best-effort; per AUD-03 acceptance, signature failures don't block session start
               dwUnionChoice: WTD_CHOICE_FILE,
               Anonymous: WINTRUST_DATA_0 { pFile: &file_info as *const _ as *mut _ },
               dwStateAction: WTD_STATEACTION_VERIFY,
               hWVTStateData: 0,
               pwszURLReference: std::ptr::null_mut(),
               dwProvFlags: 0,
               dwUIContext: 0,
               pSignatureSettings: std::ptr::null_mut(),
           };

           // SAFETY: We pass valid pointers to stack-allocated WINTRUST_DATA and
           // WINTRUST_FILE_INFO. WINTRUST_ACTION_GENERIC_VERIFY_V2 is a constant
           // GUID. The first call requests verification; we MUST run a second
           // call with WTD_STATEACTION_CLOSE (RAII guard below) to release the
           // verification state. wtd.dwStateAction is mutated by Windows.
           let result = unsafe {
               WinVerifyTrust(
                   std::ptr::null_mut(), // hWnd
                   &WINTRUST_ACTION_GENERIC_VERIFY_V2 as *const _ as *mut _,
                   &mut wtd as *mut _ as *mut _,
               )
           };

           // RAII close guard - mirrors sandbox/windows.rs::_sd_guard pattern.
           // ALWAYS run the close call to release WinTrust state, even on early return.
           let _close_guard = WinTrustCloseGuard { wtd: &mut wtd };

           let status = match result {
               0 => AuthenticodeStatus::Valid {
                   signer_subject: parse_signer_subject(&wtd)?,
                   thumbprint: parse_thumbprint(&wtd)?,
               },
               r if r as u32 == TRUST_E_NOSIGNATURE => AuthenticodeStatus::Unsigned,
               other => AuthenticodeStatus::InvalidSignature { hresult: other },
           };

           Ok(status)
       }

       struct WinTrustCloseGuard<'a> {
           wtd: &'a mut WINTRUST_DATA,
       }

       impl<'a> Drop for WinTrustCloseGuard<'a> {
           fn drop(&mut self) {
               self.wtd.dwStateAction = WTD_STATEACTION_CLOSE;
               // SAFETY: Mirrors the verify call above. The close action releases
               // the state allocated by the verify call. Errors here are best-effort
               // logged but do not propagate (we are in Drop).
               let _ = unsafe {
                   WinVerifyTrust(
                       std::ptr::null_mut(),
                       &WINTRUST_ACTION_GENERIC_VERIFY_V2 as *const _ as *mut _,
                       self.wtd as *mut _ as *mut _,
                   )
               };
           }
       }

       fn parse_signer_subject(wtd: &WINTRUST_DATA) -> Result<String, NonoError> {
           // Implementation: walk wtd.hWVTStateData via CryptCATAdminAcquireContext / WTHelperProvDataFromStateData
           // to extract signer subject CN. Fallback to "<unknown>" on parse failure (still useful as evidence).
           // See RESEARCH Pattern 4 lines 380-410 for the full sketch.
           // ... ~30-40 LOC ...
           Ok(String::from("<implementation per RESEARCH Pattern 4>"))
       }

       fn parse_thumbprint(wtd: &WINTRUST_DATA) -> Result<String, NonoError> {
           // SHA-1 cert thumbprint via CertGetCertificateContextProperty + CERT_HASH_PROP_ID.
           // ... ~20 LOC ...
           Ok(String::from("<implementation per RESEARCH Pattern 4>"))
       }
       ```

       **Implementation invariants enforced:**
       - Every `unsafe { ... }` block has a `// SAFETY:` doc comment immediately above it (CLAUDE.md § Unsafe Code).
       - No `.unwrap()` / `.expect()` in production paths (clippy::unwrap_used; CLAUDE.md). Use `?` on `Result`, return `NonoError` variants, or use match arms.
       - RAII `WinTrustCloseGuard` ensures the `WTD_STATEACTION_CLOSE` second call always runs.
       - Path conversion uses `encode_wide` (no manual UTF-16 packing).
       - On any FFI failure: caller falls back to SHA-256 (the existing `exec_identity::sha256_of_file` from 22-05a).

    4. Wire platform dispatch in `crates/nono-cli/src/exec_identity.rs`:
       ```rust
       #[cfg(target_os = "windows")]
       pub mod exec_identity_windows;  // top-level: `crates/nono-cli/src/exec_identity_windows.rs`

       #[cfg(target_os = "windows")]
       pub use exec_identity_windows::AuthenticodeStatus;

       #[cfg(not(target_os = "windows"))]
       #[derive(Debug, Clone)]
       pub enum AuthenticodeStatus {
           NotApplicable,  // Unix path: SHA-256 only
       }

       /// Sibling field on the audit envelope (RESEARCH Contradiction #2).
       /// Does NOT mutate upstream's ExecutableIdentity { resolved_path, sha256 }.
       #[cfg(target_os = "windows")]
       pub fn platform_authenticode(path: &std::path::Path) -> Option<AuthenticodeStatus> {
           exec_identity_windows::query_authenticode_status(path).ok()
       }

       #[cfg(not(target_os = "windows"))]
       pub fn platform_authenticode(_path: &std::path::Path) -> Option<AuthenticodeStatus> {
           None
       }
       ```
       Then the audit envelope (in `audit_integrity.rs` or wherever 22-05a placed the unified Alpha schema) gets a sibling `audit_authenticode: Option<AuthenticodeStatus>` field. SHA-256 capture remains independent — it ALWAYS happens.

    5. Build:
       ```
       cargo build --workspace
       ```
       NEW failures = STOP per STOP trigger #2. Likely surfaces: missing `WTD_*` constants if the feature flag wasn't applied; `WINTRUST_DATA_0` union shape variation across `windows-sys 0.59` minor versions.

    6. Lint:
       ```
       cargo clippy --workspace -- -D warnings -D clippy::unwrap_used
       ```
       MUST exit 0 for the new file. Pre-existing manifest.rs errors are documented carry-over per 22-04 / 22-05a; surface them in this plan's SUMMARY.

    7. Format:
       ```
       cargo fmt --all -- --check
       ```

    8. **D-04 gate** — Authenticode addition does NOT touch the rename surface, but run the sentinel anyway per the LOCKED rule:
       ```
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
       cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited
       cargo test -p nono-cli cli::parse_duration_
       grep -E '^const (AUTO_PRUNE_STALE_THRESHOLD|AUTO_CLEANUP_STALE_THRESHOLD): usize = 100' crates/nono-cli/src/session_commands.rs
       ```
       Expected: identical to Task 3 post-commit state.

    9. Stage and commit:
       ```
       git add crates/nono-cli/src/exec_identity_windows.rs \
               crates/nono-cli/src/exec_identity.rs \
               crates/nono-cli/src/audit_commands.rs \
               crates/nono-cli/Cargo.toml
       git commit -s -m "$(cat <<'EOF'
       feat(22-05b): add Windows Authenticode exec-identity recording (AUD-03)

       Fork-only addition per CONTEXT line 248 - D-17 ALLOWED for the
       exec_identity_windows.rs surface (planned fork-internal Windows
       addition, not an upstream port).

       Implements WinVerifyTrust signature query atop the unified Alpha
       audit-integrity schema landed by Plan 22-05a (02ee0bd1 + 7b7815f7).
       Records audit_authenticode: Option<AuthenticodeStatus> as a SIBLING
       field on the audit envelope per RESEARCH Contradiction #2 (no
       mutation of upstream's ExecutableIdentity struct).

       SHA-256 fallback when unsigned, when WinVerifyTrust fails, or on any
       FFI error (SHA-256 capture is independent and always happens).

       FFI style mirrors Phase 21's try_set_mandatory_label (sandbox/windows.rs:
       encode_wide UTF-16, unsafe + // SAFETY: docs, RAII WinTrustCloseGuard
       for WTD_STATEACTION_CLOSE, GetLastError -> typed NonoError).

       Adds Win32_Security_WinTrust feature to existing windows-sys 0.59
       in crates/nono-cli/Cargo.toml (RESEARCH finding #8).

       D-04 gate run BEFORE this commit: CLEAN-04 invariants green (the
       Authenticode addition does not touch the rename surface, but per
       CONTEXT revision LOCKED rule the gate runs anyway).

       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    10. **D-04 gate (AFTER commit) — re-verify:** same four checks as step 8.
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; test -f crates/nono-cli/src/exec_identity_windows.rs &amp;&amp; grep -E 'Win32_Security_WinTrust' crates/nono-cli/Cargo.toml &amp;&amp; cargo clippy --workspace -- -D warnings -D clippy::unwrap_used &amp;&amp; cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed</automated>
  </verify>
  <acceptance_criteria>
    - `crates/nono-cli/src/exec_identity_windows.rs` exists.
    - `crates/nono-cli/Cargo.toml` includes `Win32_Security_WinTrust` feature in the windows-sys 0.59 features list.
    - **No placeholder strings remain (WARNING fix):** `! grep -E '<implementation per RESEARCH Pattern 4>' crates/nono-cli/src/exec_identity_windows.rs` MUST return 0 hits — `parse_signer_subject` and `parse_thumbprint` MUST contain actual implementations (CryptCATAdminAcquireContext / WTHelperProvDataFromStateData walking + CertGetCertificateContextProperty + CERT_HASH_PROP_ID per RESEARCH Pattern 4 lines 380-410), not the literal placeholder string.
    - **Cross-link to Task 5 enforcement:** `cargo test -p nono-cli --test exec_identity_windows authenticode_signed_records_subject` passes on Windows host (Task 5 covers this; the test ASSERTS that the parsed signer subject contains "microsoft" for a known-signed system binary, which would fail if `parse_signer_subject` returned the placeholder string).
    - `cargo build --workspace` exits 0 (host-native — Plan 22-05b runs on Windows host per phase posture).
    - `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` exits 0 for the new file (pre-existing manifest.rs carry-over documented).
    - All `unsafe` blocks have `// SAFETY:` docs per CLAUDE.md.
    - No `.unwrap()` / `.expect()` in production code (clippy::unwrap_used).
    - RAII `WinTrustCloseGuard` present (Drop runs the WTD_STATEACTION_CLOSE call).
    - `audit_authenticode` is a SIBLING field; upstream's `ExecutableIdentity` struct shape unchanged.
    - **D-04 gate (BEFORE commit) green; D-04 gate (AFTER commit) green.**
    - Commit has Signed-off-by trailer; NO `Upstream-commit:` trailer (fork-only).
  </acceptance_criteria>
  <done>
    Windows Authenticode + SHA-256 fallback exec-identity recording landed. REQ-AUD-03 acceptance #2 + #3 fully covered. D-17 boundary respected (the only `*_windows.rs` touches in this plan are the planned Authenticode file and the rename's `session_commands_windows.rs`).
  </done>
</task>

<task type="auto">
  <name>Task 5: Windows-only Authenticode + SHA-256 fallback regression suite (AUD-03 test coverage)</name>
  <files>
    crates/nono-cli/tests/exec_identity_windows.rs (NEW; `#[cfg(target_os = "windows")]`-gated)
  </files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-VALIDATION.md (22-05-T3 expectation)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-RESEARCH.md § Wave 0 line 599 ("`crates/nono-cli/src/audit_authenticode.rs` (or sibling) — fork addition for REQ-AUD-03 acceptance #2/#3")
    - crates/nono-cli/src/exec_identity_windows.rs (post-Task-4 state — public API surface)
  </read_first>
  <action>
    1. Create `crates/nono-cli/tests/exec_identity_windows.rs`:
       ```rust
       #![cfg(target_os = "windows")]

       use nono_cli::exec_identity_windows::{query_authenticode_status, AuthenticodeStatus};

       #[test]
       fn authenticode_signed_records_subject() {
           // Use a known-signed system binary (Microsoft signs notepad.exe on every Win10/11 install).
           let path = std::path::Path::new(r"C:\Windows\System32\notepad.exe");
           let status = query_authenticode_status(path).expect("WinVerifyTrust query should succeed");
           match status {
               AuthenticodeStatus::Valid { signer_subject, .. } => {
                   assert!(
                       signer_subject.to_lowercase().contains("microsoft"),
                       "expected Microsoft signer subject, got: {}",
                       signer_subject
                   );
               }
               other => panic!("expected Valid Authenticode for signed system binary, got: {:?}", other),
           }
       }

       #[test]
       fn authenticode_unsigned_falls_back() {
           // Create an unsigned tempfile that looks like a PE but has no signature.
           let dir = tempfile::tempdir().expect("tempdir");
           let path = dir.path().join("unsigned.exe");
           std::fs::write(&path, b"MZ\x90\x00\x03\x00\x00\x00").expect("write unsigned PE stub");
           let status = query_authenticode_status(&path).expect("WinVerifyTrust query should succeed");
           assert!(
               matches!(status, AuthenticodeStatus::Unsigned | AuthenticodeStatus::InvalidSignature { .. }),
               "expected Unsigned or InvalidSignature for unsigned binary, got: {:?}",
               status
           );
       }

       #[test]
       fn authenticode_query_failure_returns_query_failed_or_err() {
           // Path that doesn't exist - WinVerifyTrust should fail.
           let path = std::path::Path::new(r"C:\nonexistent\path\that\should\not\exist.exe");
           let result = query_authenticode_status(path);
           // Either Ok(QueryFailed) or Err(NonoError) is acceptable - both signal "fall back to SHA-256"
           match result {
               Ok(AuthenticodeStatus::QueryFailed { .. })
               | Ok(AuthenticodeStatus::InvalidSignature { .. })
               | Err(_) => (),
               other => panic!("expected QueryFailed/InvalidSignature/Err for missing path, got: {:?}", other),
           }
       }
       ```

       Note: tests use `.expect()` — that's allowed in test code (CLAUDE.md exception: "`#[allow(clippy::unwrap_used)]` is permitted in test modules and documentation examples"). Add the allow attribute at the top of the file if clippy demands it.

    2. Run the new tests:
       ```
       cargo test -p nono-cli --test exec_identity_windows
       ```
       Windows host: must exit 0. Non-Windows host: tests compile to nothing (the whole file is `#[cfg(target_os = "windows")]`-gated) — documented-skip with rationale.

    3. **D-04 gate** (LOCKED per CONTEXT revision):
       ```
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
       cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited
       cargo test -p nono-cli cli::parse_duration_
       ```

    4. Stage and commit:
       ```
       git add crates/nono-cli/tests/exec_identity_windows.rs
       git commit -s -m "$(cat <<'EOF'
       test(22-05b): Windows Authenticode + SHA-256 fallback regression suite (AUD-03)

       Three Windows-host-gated tests:
       1. Signed system binary (notepad.exe) records Microsoft signer subject
       2. Unsigned PE stub falls back to Unsigned/InvalidSignature
       3. Missing path returns QueryFailed/InvalidSignature/Err (caller
          falls back to SHA-256)

       Non-Windows hosts compile to nothing (entire file is #[cfg(target_os
       = "windows")]-gated); documented-skip per phase posture.

       D-04 gate run: CLEAN-04 invariants green.

       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    5. **D-04 gate (AFTER commit) — re-verify.**
  </action>
  <verify>
    <automated>cargo test -p nono-cli --test exec_identity_windows &amp;&amp; cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed &amp;&amp; cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited</automated>
  </verify>
  <acceptance_criteria>
    - `cargo test -p nono-cli --test exec_identity_windows` exits 0 on Windows (or compiles to nothing on non-Windows hosts; record skip in SUMMARY).
    - Three test functions present: `authenticode_signed_records_subject`, `authenticode_unsigned_falls_back`, `authenticode_query_failure_returns_query_failed_or_err`.
    - VALIDATION 22-05-T3 (Authenticode portion) marked green.
    - **D-04 gate (BEFORE + AFTER commit) green.**
  </acceptance_criteria>
  <done>
    AUD-03 Windows test coverage landed. T-22-05-02 (Spoofing via test cert acceptance bug) is mitigated — test asserts the signer subject contains the expected substring rather than just "any cert".
  </done>
</task>

<task type="auto">
  <name>Task 6: Full CLEAN-04 invariant regression sweep + formal `applied_labels_guard::audit_flush_before_drop` test (BLOCKING — Phase 21 invariant)</name>
  <files>
    crates/nono/src/sandbox/windows.rs (extend `applied_labels_guard` test module with the new flush-before-Drop test)
  </files>
  <read_first>
    - .planning/phases/21-windows-single-file-grants/21-CONTEXT.md (AppliedLabelsGuard origin + Drop ordering invariants)
    - crates/nono/src/sandbox/windows.rs::tests applied_labels_guard:: module (existing 76-test suite)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-VALIDATION.md (22-05-V3: "Phase 21 `AppliedLabelsGuard` lifecycle survives audit emissions; ledger flush completes before guard Drop")
  </read_first>
  <action>
    1. Run the FULL CLEAN-04 invariant suite (vs Task 1 baseline):
       ```
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed 2>&1 | tee /tmp/22-05b-final-cleanup.log
       cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited 2>&1 | tee -a /tmp/22-05b-final-cleanup.log
       cargo test -p nono-cli cli::parse_duration_ 2>&1 | tee -a /tmp/22-05b-final-cleanup.log
       grep -E '^const (AUTO_PRUNE_STALE_THRESHOLD|AUTO_CLEANUP_STALE_THRESHOLD): usize = 100' crates/nono-cli/src/session_commands.rs
       ```
       ALL must match Task 1 baseline (per name semantics — `auto_prune_is_noop_when_sandboxed` test name unchanged; constant value unchanged). If `auto_prune_is_noop_when_sandboxed` fails: ABSOLUTE STOP per CONTEXT STOP trigger #6.

    2. Re-run Phase 21 AppliedLabelsGuard lifecycle:
       ```
       cargo test -p nono --test sandbox_windows applied_labels_guard:: 2>&1 | tee /tmp/22-05b-final-aipc.log
       ```

    3. **Add the formal `audit_flush_before_drop` regression test** (VALIDATION 22-05-V3 — was deferred from 22-05a):
       ```rust
       // crates/nono/src/sandbox/windows.rs::tests applied_labels_guard module
       #[test]
       #[cfg(target_os = "windows")]
       fn audit_flush_before_drop() {
           // Setup: construct a SupervisedRuntimeContext-shaped fixture with
           // audit_integrity enabled. The fixture should have an in-memory
           // ledger writer that records flush events.
           //
           // Action: drop the AppliedLabelsGuard in scope.
           //
           // Assertion: ledger has the expected flushed events recorded BEFORE
           // the guard's Drop completed (i.e., the Drop ordering is
           // flush_ledger() then guard.cleanup()).
           //
           // Implementation depends on how 22-05a wired the audit-integrity
           // emissions into supervised_runtime; the test fixture mirrors the
           // smallest unit that exercises the Drop ordering.
           //
           // Per CONTEXT line 226 + 227: AppliedLabelsGuard RAII pattern is the
           // structural guarantee. This test is the formal regression for
           // T-22-05-05 + AUD-04 acceptance "Phase 21 AppliedLabelsGuard
           // lifecycle preserved; ledger flush completes before guard Drop".
       }
       ```
       Run it:
       ```
       cargo test -p nono --test sandbox_windows applied_labels_guard::audit_flush_before_drop
       ```
       MUST exit 0. If red: investigate — either the audit-integrity emissions are racing with guard Drop, or the test fixture mis-models the Drop ordering.

    4. **D-04 gate** (per CONTEXT revision LOCKED — even though this commit only touches a test file in `crates/nono/`, run the gate):
       ```
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
       ```

    5. Stage and commit:
       ```
       git add crates/nono/src/sandbox/windows.rs
       git commit -s -m "$(cat <<'EOF'
       test(22-05b): formal applied_labels_guard::audit_flush_before_drop regression (Phase 21 invariant)

       Adds the formal regression test for VALIDATION 22-05-V3
       ("Phase 21 AppliedLabelsGuard lifecycle survives audit emissions;
       ledger flush completes before guard Drop"). Was deferred from
       Plan 22-05a's plan-close per scope split decision; lands here
       alongside the rename's full CLEAN-04 invariant sweep.

       Mitigates T-22-05-05 (Tampering: AppliedLabelsGuard Drop happens
       BEFORE ledger flush -> events lost on cleanup).

       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    6. If new regression in any of the above: STOP per CONTEXT STOP trigger #5.
  </action>
  <verify>
    <automated>cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed &amp;&amp; cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited &amp;&amp; cargo test -p nono-cli cli::parse_duration_ &amp;&amp; grep -E '^const (AUTO_PRUNE_STALE_THRESHOLD|AUTO_CLEANUP_STALE_THRESHOLD): usize = 100' crates/nono-cli/src/session_commands.rs &amp;&amp; cargo test -p nono --test sandbox_windows applied_labels_guard::</automated>
  </verify>
  <acceptance_criteria>
    - All CLEAN-04 invariant tests green (match Task 1 baseline by name semantics).
    - `AUTO_PRUNE_STALE_THRESHOLD` (or upstream-renamed equivalent) constant value unchanged at `100`.
    - `auto_prune_is_noop_when_sandboxed` test name preserved (AUD-04 acceptance #4).
    - Phase 21 AppliedLabelsGuard lifecycle tests still green.
    - **NEW: `applied_labels_guard::audit_flush_before_drop` test exists and exits 0** (VALIDATION 22-05-V3).
    - **`audit_flush_before_drop` test body has ≥ 20 LOC of test setup + assertion (WARNING fix)** — not a no-op stub. Verify guard: `awk '/fn audit_flush_before_drop/,/^}/' crates/nono/src/sandbox/windows.rs | wc -l` MUST return ≥ 22 (function signature + ≥ 20 body lines + closing brace = ≥ 22 lines). The test fixture must construct an audit-integrity-enabled context with an in-memory ledger writer, drop the AppliedLabelsGuard, and assert flush events were recorded BEFORE the guard's Drop completed.
    - VALIDATION 22-05-V1 + V2 + V3 all green.
    - **D-04 gate (BEFORE + AFTER commit) green.**
  </acceptance_criteria>
  <done>
    Full CLEAN-04 invariant regression sweep clean. T-22-05-04 ABSOLUTE STOP guard held end-to-end. T-22-05-05 (AppliedLabelsGuard flush-before-Drop) covered by formal regression test.
  </done>
</task>

<task type="auto">
  <name>Task 7: D-18 Windows-regression gate (BLOCKING — final per-plan close gate)</name>
  <files>(read-only verification)</files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § D-18
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-VALIDATION.md (per-task verification map for 22-05-T3 Authenticode + T4 + V1..V3)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05a-AUD-CORE-SUMMARY.md (deferred-flake baseline carried forward from 22-04 + 22-05a)
  </read_first>
  <action>
    1. Capture pre-gate HEAD: `git rev-parse HEAD > /tmp/22-05b-final-head.txt`.

    2. Run the full D-18 safety net (vs Plan 22-05a baseline):
       ```
       cargo test --workspace --all-features 2>&1 | tee /tmp/22-05b-final-ci.log
       ```
       Compare failure categories to the carried-forward deferred-flake baseline (TUF root signature freshness 2 tests, `convert_filesystem_grants` `/tmp` 1 test, `policy::tests::test_resolve_*` `/tmp` 3 tests, `windows_*_help_reports_documented_limitation` 4 tests, `windows_run_allows_*` UNC path 8 tests). NEW failures = STOP per STOP trigger #2.

    3. Phase 15 5-row detached-console smoke gate (Windows host):
       ```
       # Manual: nono run --profile claude-code -- claude-code --version; nono ps; nono session cleanup --older-than 1d
       # (Note: use `session cleanup` here, not `prune` — verify the renamed surface works in the smoke test)
       # Or: use `nono prune --older-than 1d` to verify the deprecation alias works end-to-end (will emit deprecation note)
       ```
       If non-Windows host: documented-skip.

    4. WFP integration suite (if admin + service available):
       ```
       cargo test -p nono-cli --test wfp_port_integration -- --ignored
       ```

    5. Learn-Windows ETW smoke:
       ```
       cargo test -p nono-cli --test learn_windows_integration
       ```

    6. Audit-attestation fixture (carried forward from 22-05a Task 8):
       ```
       cargo test -p nono-cli --test audit_attestation
       ```

    7. Authenticode + SHA-256 fallback (Task 5 of this plan):
       ```
       cargo test -p nono-cli --test exec_identity_windows
       ```

    8. Prune-alias deprecation regression (Task 3):
       ```
       cargo test -p nono-cli --test prune_alias_deprecation
       ```

    9. End-to-end audit-integrity + Authenticode + rename smoke (Windows host):
       ```
       # Pre-provision keyring://nono/audit
       cargo run -p nono-cli -- run --audit-integrity --audit-sign-key keyring://nono/audit -- echo hi
       # Capture <session-id> from output
       cargo run -p nono-cli -- audit show <session-id>          # verify exec_identity includes Authenticode status
       cargo run -p nono-cli -- audit verify <session-id>        # verify chain + signature
       cargo run -p nono-cli -- session cleanup --older-than 1d  # renamed surface works
       cargo run -p nono-cli -- audit cleanup --older-than 1d    # peer subcommand works
       cargo run -p nono-cli -- prune --older-than 1d 2>&1       # deprecation alias works + emits note
       ```

    10. Format + clippy:
       ```
       cargo fmt --all -- --check
       cargo clippy --workspace -- -D warnings -D clippy::unwrap_used
       ```
       Pre-existing manifest.rs carry-over per 22-04/22-05a.

    11. **Final D-04 gate (LOCKED) — verify CLEAN-04 invariants survived all 6 commits in this plan:**
       ```
       cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed
       cargo test -p nono-cli session::is_prunable_all_exited_escape_hatch_matches_any_exited
       cargo test -p nono-cli cli::parse_duration_
       grep -E '^const (AUTO_PRUNE_STALE_THRESHOLD|AUTO_CLEANUP_STALE_THRESHOLD): usize = 100' crates/nono-cli/src/session_commands.rs
       grep -nE 'NONO_CAP_FILE.*is_some.*return' crates/nono-cli/src/session_commands.rs crates/nono-cli/src/session_commands_windows.rs
       ```
       MUST all hold. If `auto_prune_is_noop_when_sandboxed` fails: ABSOLUTE STOP, revert plan.

    12. VALIDATION.md gate: 22-05-T1..T4 + V1..V3 all green (T1, T2, V1 from 22-05a; T3 SHA-256 from 22-05a; T3 Authenticode from this plan Task 4–5; T4 from this plan Task 2; V2 from Task 3; V3 from Task 6).
  </action>
  <verify>
    <automated>cargo test --workspace --all-features &amp;&amp; cargo test -p nono-cli --test learn_windows_integration &amp;&amp; cargo test -p nono-cli --test audit_attestation &amp;&amp; cargo test -p nono-cli --test exec_identity_windows &amp;&amp; cargo test -p nono-cli --test prune_alias_deprecation &amp;&amp; cargo fmt --all -- --check &amp;&amp; cargo clippy --workspace -- -D warnings -D clippy::unwrap_used &amp;&amp; cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed</automated>
  </verify>
  <acceptance_criteria>
    - `cargo test --workspace --all-features` exits 0 within deferred-flake window (no NEW categories vs 22-05a close baseline).
    - Phase 15 5-row smoke gate passes (or documented-skip).
    - `wfp_port_integration --ignored` passes or documented-skipped.
    - `learn_windows_integration` exits 0.
    - `audit_attestation` test passes (carried from 22-05a).
    - `exec_identity_windows` tests pass (Task 5).
    - `prune_alias_deprecation` test passes (Task 3).
    - End-to-end smoke verifies: `nono session cleanup` works; `nono audit cleanup` works; `nono prune` works + emits deprecation; `nono audit show` displays Authenticode status; `nono audit verify` succeeds.
    - `cargo fmt` + `cargo clippy` exit 0 (manifest.rs carry-over documented).
    - **Final D-04 gate green** — all four CLEAN-04 invariants hold; sandbox guard early-return preserved in BOTH session_commands.rs and session_commands_windows.rs.
    - VALIDATION.md 22-05-T1..T4 + V1..V3 all green.
  </acceptance_criteria>
  <done>
    D-18 Windows-regression safety net cleared for Plan 22-05b. Phase 22 audit cluster fully closed. T-22-05-04 ABSOLUTE STOP guard held end-to-end across the rename. T-22-05-05 covered by formal regression test.
  </done>
</task>

<task type="auto">
  <name>Task 8: Plan SUMMARY + AUD-05 fold-or-split decision + D-07 plan-close push to origin</name>
  <files>
    .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05b-AUD-RENAME-SUMMARY.md (NEW)
  </files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-04-OAUTH-SUMMARY.md (sibling plan SUMMARY format template)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05a-AUD-CORE-SUMMARY.md (immediate predecessor)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § D-07 (per-plan push pattern), § Claude's Discretion ("AUD-05 fold-or-split decision-point")
    - .planning/ROADMAP.md § Phase 23 (default: AUD-05 lives in Phase 23)
  </read_first>
  <action>
    1. Author `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05b-AUD-RENAME-SUMMARY.md` matching 22-04-OAUTH-SUMMARY.md frontmatter shape (`phase`, `plan: 05b`, `subsystem: audit-rename-and-authenticode`, `tags`, `dependency_graph` with requires/provides/affects, `tech_stack`, `key_files`, `decisions`, `metrics`).

       Required body sections: Outcome, What was done (one bullet per Task 1–8), Verification table, Files changed table, Commits (5–6-row table — fork-only commits with no Upstream-commit trailers), Status, Threat model coverage (T-22-05-04 ABSOLUTE STOP held; T-22-05-05 mitigated via formal regression test), AUD-05 fold-or-split decision, Phase 22 close-out implications.

    2. **AUD-05 fold-or-split decision (CONTEXT Claude's Discretion).** Now that the audit cluster is fully landed (22-05a + 22-05b), examine whether upstream's `AuditEvent` enum + `audit_integrity::append_event` cleanly accept AIPC HandleKind events with no Windows-specific shape required. Run:
       ```
       grep -rE 'handle_(file|socket|pipe|event|mutex|jobobject)_request' crates/nono-cli/src/exec_strategy_windows/
       grep -rnE 'AuditEvent|append_event' crates/nono-cli/src/audit_integrity.rs crates/nono-cli/src/audit_commands.rs crates/nono-cli/src/audit_session.rs crates/nono-cli/src/audit_ledger.rs crates/nono-cli/src/audit_attestation.rs crates/nono-cli/src/exec_identity.rs
       ```
       - **If clean fit:** Recommend "fold into v2.2 follow-up plan after Phase 24 ships" (would have been simple `audit::append_event(AuditEvent::AipcHandle { kind, ... })` per `handle_*_request`). Phase 23 stays the default home but the recommendation surfaces for the user to decide.
       - **If Windows-specific extension required:** Confirm Phase 23 stays as-is (separate plan). HandleKind-specific extension is a Windows-only surface — not foldable.

       Document the decision in SUMMARY:
       - Decision: fold-recommended / Phase-23-confirmed
       - Rationale: 2–3 lines on what evidence drove the call (`grep` output snippets)
       - Action: defer the actual fold-or-Phase-23 commit to ROADMAP review

    3. Stage SUMMARY and commit:
       ```
       git add .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05b-AUD-RENAME-SUMMARY.md
       git commit -s -m "$(cat <<'EOF'
       docs(22-05b): close plan with rename + Authenticode + CLEAN-04 sweep

       6 fork-only commits land AUD-04 (rename + audit cleanup peer +
       deprecation alias) and AUD-03 Windows portion (Authenticode + SHA-256
       fallback). T-22-05-04 ABSOLUTE STOP guard held end-to-end across the
       rename: auto_prune_is_noop_when_sandboxed test name preserved;
       NONO_CAP_FILE early-return preserved in BOTH session_commands.rs and
       session_commands_windows.rs; AUTO_PRUNE_STALE_THRESHOLD = 100 unchanged.

       Plan 22-05b closes the Phase 22 audit cluster. AUD-05 fold-or-split
       decision documented for ROADMAP review (default home = Phase 23).

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
    <automated>test -f .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05b-AUD-RENAME-SUMMARY.md &amp;&amp; git fetch origin &amp;&amp; test "$(git log origin/main..main --oneline | wc -l)" = "0"</automated>
  </verify>
  <acceptance_criteria>
    - `22-05b-AUD-RENAME-SUMMARY.md` exists with the standard frontmatter + body sections.
    - SUMMARY documents the D-04 gate ran AFTER every commit and held throughout (per CONTEXT revision LOCKED rule).
    - SUMMARY documents AUD-05 fold-or-split decision with rationale.
    - SUMMARY explicitly confirms T-22-05-04 ABSOLUTE STOP did not fire.
    - SUMMARY confirms Phase 22 audit cluster fully closed (AUD-01..04 all green; AUD-03 Windows + cross-platform both covered).
    - `git log origin/main..main --oneline | wc -l` returns `0` after push.
    - SUMMARY records post-push origin/main SHA for traceability.
  </acceptance_criteria>
  <done>
    Plan 22-05b closed and published to origin. Phase 22 audit cluster fully shipped. AUD-05 disposition documented for Phase 23 ROADMAP review.
  </done>
</task>

</tasks>

<non_goals>
**AUD-01, AUD-02, AUD-03 SHA-256 portion** — already landed in Plan 22-05a; this plan does NOT retouch those surfaces.

**Plan 22-05a's audit-integrity + attestation + verify scope:** Already complete. This plan does NOT retouch `audit_integrity.rs`, `audit_attestation.rs`, `audit_commands.rs` (verify subcommand) except where the platform dispatch hook in `exec_identity.rs` extends to add the Authenticode sibling field.

**Phase 23 AUD-05** (Windows AIPC broker audit-event emissions): Out of Phase 22 scope per CONTEXT D-11. Task 8 surfaces a fold-or-split recommendation but does NOT execute the AIPC retrofit.

**Phase 24 DRIFT-01..02** (drift-check tooling + GSD upstream-sync template): Out of Phase 22 scope per CONTEXT D-11.

**Pre-existing Phase 19 deferred flakes:** `tests/env_vars.rs` (≤19 failures) and `trust_scan::tests::*` (1–3 failures) are documented-deferred. This plan must NOT attempt fixes but also MUST NOT let them mask new regressions.

**`prune` alias deprecation timeline:** REQ-AUD-04 acceptance #3 says hidden alias survives "one release". Whether `nono prune` survives one release (v2.3), two (v2.4), or longer is a v2.3-milestone scoping decision per CONTEXT Claude's Discretion bullet — NOT a Phase 22 decision.

**Authenticode network revocation timing:** `WTD_REVOKE_NONE` chosen for the WinVerifyTrust call (best-effort signature query without CRL/OCSP latency). T-22-05-10 (DoS via revocation check hang) is mitigated by this choice; SHA-256 fallback always covers the Authenticode failure path.

**Pre-existing manifest.rs clippy carry-over** (`crates/nono/src/manifest.rs:95/103` `collapsible_match`): Deferred per Plan 22-04 / 22-05a SUMMARY; out of this plan's scope.
</non_goals>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Sandboxed agent → file system (via `prune` / `session cleanup` rename) | **CRITICAL**: sandboxed agent must NOT delete host-side files via auto-prune / auto-cleanup. CLEAN-04 invariant. |
| `nono prune` (deprecated alias) → `nono session cleanup` (renamed) | User-facing alias must delegate to renamed semantics without losing CLEAN-04 invariants. |
| Executable on disk → exec-identity capture (Windows) | `GetModuleFileNameW` + `WinVerifyTrust` query. Spoofed signature = Spoofing for AUD-03 evidence. |
| `WinVerifyTrust` FFI → process memory | OS-validated signature + chain. Compromise = OS-level breach beyond nono's threat model. |
| AppliedLabelsGuard Drop → ledger flush | Phase 21 invariant survives audit emissions: guard cleanup must NOT race ahead of ledger flush. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation |
|-----------|----------|-----------|----------|-------------|------------|
| T-22-05-04 | Elevation of Privilege | `prune` → `cleanup` rename regresses `auto_prune_is_noop_when_sandboxed` invariant; sandboxed agent deletes files | **CRITICAL — ABSOLUTE STOP** | mitigate (ABSOLUTE BLOCKING) | D-04 gate (BEFORE + AFTER) every rename-touching commit (Tasks 2, 3, 4, 5, 6); CONTEXT STOP trigger #6 (ABSOLUTE STOP). Test name PRESERVED across rename per AUD-04 acceptance #4 interpretation. `NONO_CAP_FILE.is_some() return` early-return PRESERVED as first statement in BOTH session_commands.rs and session_commands_windows.rs (boundary checks in Task 2 step 4). |
| T-22-05-05 | Tampering | AppliedLabelsGuard Drop happens BEFORE ledger flush; events lost on cleanup | **high** | mitigate (BLOCKING) | Phase 21 invariant: ledger flush in supervised_runtime cleanup completes BEFORE AppliedLabelsGuard Drop. **Formal regression test `applied_labels_guard::audit_flush_before_drop` lands in Task 6** — was deferred from 22-05a per scope split. |
| T-22-05-02 | Spoofing | Authenticode signature parsed from a binary signed by an attacker-controlled cert (test cert acceptance bug) | **high** | mitigate (BLOCKING) | Task 4 records `signer_subject` + `thumbprint` + `state` (Valid/Unsigned/InvalidSignature/QueryFailed). Verification at audit-show time displays the subject; downstream policy can reject untrusted signers. Test in Task 5: `authenticode_signed_records_subject` asserts substring match against expected signer (Microsoft for system binary), not just "any cert". |
| T-22-05b-01 | Information Disclosure | Authenticode signer subject CN contains internal/sensitive info that leaks via audit ledger | low | accept | Signer subject is intended-public information (cert chains are public records). No sanitization required. Documented in commit body. |
| T-22-05b-02 | DoS | `WinVerifyTrust` blocks on network revocation check (CRL/OCSP) | low | accept | `WTD_REVOKE_NONE` chosen — best-effort signature query without revocation network round-trip. SHA-256 fallback ensures audit completes even on Authenticode failure. AUD-03 acceptance allows "Signature failures do not prevent session start". |
| T-22-05b-03 | Tampering | `unsafe` FFI block in `exec_identity_windows.rs` lacks SAFETY doc → review/audit miss | medium | mitigate | CLAUDE.md § Unsafe Code requires `// SAFETY:` on every unsafe block. Task 4 acceptance criterion enforces. clippy + code review gate. |
| T-22-05b-04 | Elevation of Privilege | `.unwrap()` in production FFI path panics inside the supervisor with sensitive state held | medium | mitigate | clippy::unwrap_used enforced. Task 4 acceptance criterion + CI clippy gate. |
| T-22-05b-05 | Tampering | RAII close guard in `exec_identity_windows.rs` mis-orders WTD_STATEACTION_CLOSE → state leak | medium | mitigate | RAII `WinTrustCloseGuard` (Task 4 step 3) ensures the close call always runs on Drop, even on early return / error. Mirrors Phase 21's `_sd_guard` pattern. |
| T-22-05b-06 | Repudiation | Hidden `prune` alias silently delegates without surfacing deprecation note → users miss the migration window | low | mitigate | Task 3 emits stderr deprecation note on every invocation; `--help` output explicitly says DEPRECATED; regression test `prune_alias_deprecation_note` enforces. |
| T-22-05b-07 | Repudiation / UX | `nono prune` is UNDEFINED in the intermediate window between Task 2's commit (which renames `Cmd::Prune` → `Cmd::SessionCleanup`) and Task 3's commit (which adds the hidden `prune` deprecation alias). A user invoking `nono prune` against a build pulled from the in-progress branch sees an "unknown subcommand" error, not the expected deprecation note. | low | accept | The window is intra-executor-run (< 1 hour wall-clock between Task 2 and Task 3 commits, both run by the same single-user solo executor). CI runs at plan-close, not on each intra-plan commit. Bisect through the intermediate gap is rare (the user does not bisect within their own plan-execution session). The plan-close D-18 gate (Task 7) and end-to-end smoke (Task 7 step 9) verify the alias works at the plan boundary. (WARNING fix.) |

**BLOCKING threats:** T-22-05-04 (ABSOLUTE), T-22-05-05, T-22-05-02 — Plan 22-05b cannot close until all three are mitigated and verified.
</threat_model>

<verification>
- `cargo build --workspace` exits 0.
- `cargo test --workspace --all-features` exits 0 within Phase 19 deferred-flake tolerance (no NEW categories vs Plan 22-05a close baseline).
- `cargo fmt --all -- --check` exits 0.
- `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` exits 0 (pre-existing manifest.rs carry-over documented).
- Phase 15 5-row smoke gate passes (or documented-skip).
- `nono session cleanup --older-than 7d` works on Windows (AUD-04 acceptance #1).
- `nono audit cleanup --older-than 30d` works on Windows (AUD-04 acceptance #2).
- `nono prune --older-than 7d` still works AND surfaces a deprecation note (AUD-04 acceptance #3).
- `auto_prune_is_noop_when_sandboxed` passes under both old and new function names (AUD-04 acceptance #4) — test NAME preserved verbatim per `<interfaces>` interpretation.
- `--older-than 30` (no suffix) still fails with the CLEAN-04 migration hint (AUD-04 acceptance #5).
- Windows: `nono audit show` includes `exec_identity` with path + Authenticode status (AUD-03 acceptance #1).
- Windows: signed system binary records valid signer chain (AUD-03 acceptance #2; covered by `exec_identity_windows::authenticode_signed_records_subject`).
- Windows: unsigned dev build records `unsigned` + SHA-256 (AUD-03 acceptance #3; covered by `exec_identity_windows::authenticode_unsigned_falls_back`).
- Phase 21 AppliedLabelsGuard lifecycle still green; formal `audit_flush_before_drop` regression test added in Task 6.
- VALIDATION.md 22-05-T1..T4 + V1..V3 all green.
- `AUTO_PRUNE_STALE_THRESHOLD = 100` (or upstream-renamed equivalent) constant value unchanged.
- `NONO_CAP_FILE.is_some() return` early-return present in BOTH session_commands.rs and session_commands_windows.rs (T-22-05-04 mitigation).
- All `unsafe` blocks in `exec_identity_windows.rs` carry `// SAFETY:` docs.
- No `.unwrap()` / `.expect()` in production paths in `exec_identity_windows.rs`.
- All commits in this plan carry Signed-off-by trailer; NO `Upstream-commit:` trailers (fork-only).
- `git log origin/main..main` shows zero commits ahead post-Task 8.
- **D-04 gate ran AFTER every commit in this plan** (per CONTEXT revision LOCKED rule). All passed.
- **STOP trigger #6 (ABSOLUTE) did NOT fire** — `auto_prune_is_noop_when_sandboxed` green throughout.
</verification>

<success_criteria>
- 5–6 fork-only commits on `main` (rename + audit cleanup peer; deprecation alias; Authenticode + Cargo.toml feature flag; Authenticode tests; AppliedLabelsGuard formal regression test; SUMMARY). All DCO-signed.
- `nono session cleanup` and `nono audit cleanup` work; `nono prune` works as deprecated alias with deprecation note.
- Windows Authenticode signer subject + thumbprint recorded in unified Alpha audit-integrity schema; SHA-256 fallback when unsigned/QueryFailed.
- v2.1 CLEAN-04 invariants ALL preserved through the rename (test names verbatim; constant value unchanged; sandbox guard early-return preserved).
- Phase 21 AppliedLabelsGuard lifecycle preserved end-to-end; formal flush-before-Drop regression test added.
- `make ci` green or matches Phase 19 deferred window (no NEW categories vs 22-05a close).
- `origin/main` advanced to plan-close HEAD; Phase 22 audit cluster fully closed.
- Plan SUMMARY records all 8 tasks' outcomes, ~6 commit hashes, AUD-05 fold-or-split decision with rationale, and explicit confirmation that T-22-05-04 ABSOLUTE STOP did not fire across the rename.
- Phase 22 closes with REQ-PROF-01..04 + REQ-POLY-01..03 + REQ-PKG-01..04 + REQ-OAUTH-01..03 + REQ-AUD-01..04 fully covered (18 requirements; AUD-05 deferred to Phase 23).
</success_criteria>

<output>
Create `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05b-AUD-RENAME-SUMMARY.md` per standard summary template (matching 22-04-OAUTH-SUMMARY.md frontmatter shape). Required sections: Outcome, What was done (one bullet per Task 1–8), Verification table, Files changed table, Commits (5–6-row table; fork-only — NO Upstream-commit trailers), Status, Threat model coverage (T-22-05-04 ABSOLUTE STOP held; T-22-05-05 mitigated via formal regression test; T-22-05-02 mitigated via signer-subject substring match in test), AUD-05 fold-or-split decision with rationale, Phase 22 close-out implications.
</output>
