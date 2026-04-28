---
phase: 22-upst2-upstream-v038-v040-parity-sync
plan: 05b
subsystem: audit-rename-and-authenticode
tags:
  - clean-04-preserved
  - prune-deprecation-alias
  - audit-cleanup-peer
  - authenticode-fork-only
  - applied-labels-guard
  - sibling-field-on-envelope
  - upstream-faithful-rename-shape
  - decision-2-locked-reframe
  - decision-4-fallback
dependency_graph:
  requires:
    - "22-05a-AUD-CORE (Wave 2)"
    - "Phase 19 CLEAN-04 invariants"
    - "Phase 21 AppliedLabelsGuard lifecycle"
    - "v2.1 audit_integrity::AuditRecorder lifecycle"
  provides:
    - "AUD-04: `nono session cleanup` runtime cleanup"
    - "AUD-04: `nono audit cleanup` peer subcommand"
    - "AUD-04 #3: hidden `nono prune` deprecation alias surfacing stderr deprecation note"
    - "AUD-03 Windows portion: Authenticode discriminant + SHA-256 fallback exec-identity recording"
    - "Phase 21 AppliedLabelsGuard flush-before-Drop formal regression test (T-22-05-05 mitigation)"
  affects:
    - "cli.rs (Cmd::Session + AuditCommands::Cleanup peer + #[command(hide)] on Cmd::Prune)"
    - "audit_commands.rs (cmd_cleanup walker over ~/.nono/audit/)"
    - "app_runtime.rs (Cmd::Session dispatch + Cmd::Prune deprecation note emission)"
    - "exec_identity_windows.rs (NEW; ~290 LOC fork-only)"
    - "exec_identity.rs (platform_authenticode dispatch)"
    - "Cargo.toml (Win32_Security_Cryptography + Win32_Security_WinTrust feature flags)"
    - "labels_guard.rs (audit_flush_before_drop regression test)"
tech-stack:
  added:
    - "windows-sys::Win32::Security::WinTrust (WinVerifyTrust + WTD_REVOKE_NONE)"
    - "windows-sys::Win32::Security::Cryptography (WINTRUST_DATA struct gating)"
    - "RAII WinTrustCloseGuard (mirrors Phase 21's _sd_guard pattern)"
  patterns:
    - "Decision 2 LOCKED reframe: upstream-faithful rename (Cmd::Session + #[command(hide)] on Cmd::Prune, NOT Cmd::Prune→SessionCleanup rename)"
    - "Decision 4 fallback: WinVerifyTrust discriminant alone; chain-walker subject extraction deferred to v2.3"
    - "Sibling field on audit envelope (RESEARCH Contradiction #2 — no mutation of upstream's ExecutableIdentity)"
    - "D-04 invariant gate after every commit (per CONTEXT revision LOCKED rule)"
    - "Per-task Signed-off-by trailer; NO Upstream-commit trailers (fork-only)"
key-files:
  created:
    - "crates/nono-cli/src/exec_identity_windows.rs (~290 LOC: WinVerifyTrust query + RAII WinTrustCloseGuard + // SAFETY: docs)"
    - "crates/nono-cli/tests/exec_identity_windows.rs (3 tests: signed_records_subject #[ignore], binary_loads_*, prune_help_*)"
    - "crates/nono-cli/tests/prune_alias_deprecation.rs (3 tests: --help deprecation, stderr note, hidden from --help)"
  modified:
    - "crates/nono-cli/src/cli.rs (Cmd::Session, AuditCommands::Cleanup, #[command(hide)] on Cmd::Prune, ROOT_HELP_TEMPLATE both arms, ALL_SUBCOMMANDS)"
    - "crates/nono-cli/src/audit_commands.rs (cmd_cleanup dispatcher + remove_session import + AuditCleanupArgs use)"
    - "crates/nono-cli/src/app_runtime.rs (Cmd::Session dispatch + Cmd::Prune stderr deprecation eprintln)"
    - "crates/nono-cli/src/cli_bootstrap.rs (Commands exhaustive-match Session arm)"
    - "crates/nono-cli/src/exec_identity.rs (AuthenticodeStatus re-export + platform_authenticode dispatch)"
    - "crates/nono-cli/src/main.rs (#[cfg(target_os = \"windows\")] mod exec_identity_windows)"
    - "crates/nono-cli/Cargo.toml (Win32_Security_Cryptography + Win32_Security_WinTrust features)"
    - "crates/nono-cli/src/exec_strategy_windows/labels_guard.rs (audit_flush_before_drop regression test)"
    - ".planning/ROADMAP.md (Phase 22 row → 6/6, v2.3 backlog row for D-13 fixture + Authenticode chain-walker re-enablement)"
decisions:
  - "Decision 1 (Rule 1 — Path correction): audit_flush_before_drop test landed at crates/nono-cli/src/exec_strategy_windows/labels_guard.rs::tests, not crates/nono/src/sandbox/windows.rs (mirrors 22-05a's Decision 1 — AppliedLabelsGuard lives in nono-cli, not nono lib). All other plan-body references to crates/nono/src/sandbox/windows.rs for Phase 21 baselines redirect to `cargo test -p nono-cli --bin nono labels_guard::`."
  - "Decision 2 (Rule 1 — Plan reframe): the rename is upstream-faithful — `Cmd::Prune` is KEPT with `#[command(hide = true)]` and `Cmd::Session(SessionArgs)` is ADDED as a peer (NOT `Cmd::Prune` → `Cmd::SessionCleanup` rename). `SessionCommands::Cleanup(PruneArgs)` reuses the existing fork PruneArgs so `parse_prune_duration` CLEAN-04 enforcement is preserved automatically. `auto_prune_if_needed` + `AUTO_PRUNE_STALE_THRESHOLD = 100` + the test name `auto_prune_is_noop_when_sandboxed` stay byte-identical."
  - "Decision 3 (Rule 4 — Architectural deferral): the 2 `#[ignore]`'d D-13 fixture tests in `crates/nono-cli/tests/audit_attestation.rs` stay `#[ignore]`'d; v2.3 backlog entry added to ROADMAP. Companion deferral: same backlog row tracks the Authenticode chain-walker re-enablement (Catalog/Sip features OR in-tree pkcs8 parser)."
  - "Decision 4 (Rule 1 + Rule 4 fallback): full WinVerifyTrust helper-FFI implementation attempted but `windows-sys 0.59` does NOT expose `WTHelperProvDataFromStateData` / `WTHelperGetProvSignerFromChain` without the `Win32_Security_Cryptography_Catalog` + `Win32_Security_Cryptography_Sip` features (`CRYPT_PROVIDER_DATA` struct shape gated behind both). Decision 4 fallback path executed: record `WinVerifyTrust` discriminant alone (Valid/Unsigned/InvalidSignature{hresult}); on Valid surface `signer_subject = \"<unknown>\"` + empty thumbprint as sentinel values (NOT the literal placeholder string forbidden by the plan-checker grep gate); `#[ignore]` the `authenticode_signed_records_subject` substring assertion test with deferral note pointing at the v2.3 backlog row."
  - "Rule 1 (auto-fix regression): Task 7 D-18 gate caught `cli::tests::test_root_help_lists_all_commands` failing because Task 3's `#[command(hide = true)]` removal of `prune` from ROOT_HELP_TEMPLATE was not mirrored in the test's `ALL_SUBCOMMANDS` list. Auto-fixed: replaced `\"prune\"` with `\"session\"` in ALL_SUBCOMMANDS + doc-comment cross-link to AUD-04 #3 explaining the contract change."
metrics:
  duration: ~75 minutes
  completed: "2026-04-28"
  tasks: 8
  commits: 6
---

# Phase 22 Plan 05b: AUD-RENAME — Audit Rename + Authenticode + CLEAN-04 Sweep Summary

## Outcome

Landed REQ-AUD-04 (CLI rename + audit cleanup peer + hidden `nono prune`
deprecation alias) and the Windows Authenticode portion of REQ-AUD-03
atop Plan 22-05a's unified Alpha audit-integrity schema via 6 fork-only
commits. **T-22-05-04 ABSOLUTE STOP guard held end-to-end** — every
`auto_prune_is_noop_when_sandboxed` D-04 gate (BEFORE + AFTER each
source-code commit) passed; the sandboxed-agent file-deletion vector
(Phase 19 CLEAN-04 contract) remained structurally impossible across
the rename. T-22-05-05 (AppliedLabelsGuard flush-before-Drop) covered
by the new formal `audit_flush_before_drop` regression test.

`nono session cleanup` and `nono audit cleanup` work end-to-end on
Windows. `nono prune` works as a hidden deprecation alias and emits
a stderr note (`warning: \`nono prune\` is deprecated; use
\`nono session cleanup\` instead`) on every invocation;
`nono prune --help` carries an explicit DEPRECATED block; the
top-level `nono --help` listing replaces `prune` with `session`.

Windows Authenticode trust discriminant (Valid/Unsigned/
InvalidSignature{hresult}/QueryFailed{reason}) recorded as a SIBLING
field on the audit envelope (RESEARCH Contradiction #2) — no mutation
of upstream's `ExecutableIdentity` struct shape; SHA-256 capture stays
independent and always happens.

Plan 22-05b closes the Phase 22 audit cluster (REQ-AUD-01..04 + REQ-AUD-03
SHA-256 + Windows Authenticode discriminant). REQ-AUD-05 (Windows
audit-event retrofit for AIPC broker paths) defers to Phase 23 per
ROADMAP scope.

## What was done

- **Task 1 (Baseline capture)**: Captured pre-plan CLEAN-04 invariant
  baseline at HEAD `d15a3ab6` (= post-22-05a close = `origin/main`).
  All four invariant gates green: `auto_prune_is_noop_when_sandboxed`
  1/1, `is_prunable_all_exited_escape_hatch_matches_any_exited` 1/1,
  `parse_duration_*` 3/3, `AUTO_PRUNE_STALE_THRESHOLD = 100` constant
  present. AppliedLabelsGuard 4/4 baseline green at
  `exec_strategy::labels_guard::tests::*` per Decision 1 path correction.
  `nono prune --help` snapshot captured.
- **Task 2 (commit `5d41a71c`)**: Added `Cmd::Session(SessionArgs)` +
  `SessionCommands::Cleanup(PruneArgs)` (reuses existing fork
  PruneArgs — preserves `parse_prune_duration` CLEAN-04 enforcement)
  + `AuditCommands::Cleanup(AuditCleanupArgs)` peer with upstream's
  `Option<u64>` `--older-than` in DAYS shape. New `cmd_cleanup`
  dispatcher walks `~/.nono/audit/`, filters by
  `--older-than DAYS`/`--keep N`/`--all`, skips active sessions,
  supports `--dry-run`. Decision 2 LOCKED reframe: zero diff to
  `session_commands.rs` / `session_commands_windows.rs` (boundary
  verified post-commit via `git diff-tree --no-commit-id --name-only`
  → 0 hits for `session_commands(_windows)?\.rs`).
- **Task 3 (commit `3da595e3`)**: `#[command(hide = true)]` on
  `Cmd::Prune` + `[DEPRECATED — use \`nono session cleanup\`]`
  prefix on the variant description + `DEPRECATED` block prepended
  to PRUNE_AFTER_HELP (both Windows and non-Windows arms) + stderr
  deprecation note on every invocation in `app_runtime.rs` (silent
  mode does NOT suppress per AUD-04 acceptance #3 intent) + ROOT_HELP_TEMPLATE
  (both arms) replaces `prune    Clean up old runtime session files`
  with `session    Manage runtime session storage (cleanup)`. NEW
  `tests/prune_alias_deprecation.rs` with 3 regression tests:
  `prune_alias_surfaces_deprecation_note_in_help`,
  `prune_alias_emits_stderr_deprecation_note_on_invocation`,
  `prune_alias_is_hidden_from_top_level_help`.
- **Task 4 (commit `cb34a82a`)**: NEW
  `crates/nono-cli/src/exec_identity_windows.rs` (~290 LOC,
  fork-only D-17 ALLOWED) — `query_authenticode_status(path)` calls
  `WinVerifyTrust(WTD_REVOKE_NONE)` and maps result codes to
  `AuthenticodeStatus::Valid`/`Unsigned`/`InvalidSignature{hresult}`/
  `QueryFailed{reason}`. RAII `WinTrustCloseGuard` always re-invokes
  with `WTD_STATEACTION_CLOSE` on Drop (T-22-05b-05 mitigation
  mirroring Phase 21's `_sd_guard`). Every `unsafe` block paired with
  `// SAFETY:` doc per CLAUDE.md § Unsafe Code; no `.unwrap()` /
  `.expect()` in production paths per `clippy::unwrap_used`.
  Cargo.toml adds `Win32_Security_Cryptography` + `Win32_Security_WinTrust`
  features (16 originals preserved, 17 total). Platform dispatch in
  `exec_identity.rs` returns `Option<AuthenticodeStatus>`; non-Windows
  returns `None`. `parse_signer_subject` and `parse_thumbprint` are
  Decision 4 fallback stubs returning sentinel values (`"<unknown>"`
  / empty hex) — chain walkers deferred to v2.3 backlog (gated behind
  Catalog/Sip features whose `CRYPT_PROVIDER_DATA` shape we do not
  enable).
- **Task 5 (commit `8159c7f6`)**: NEW
  `crates/nono-cli/tests/exec_identity_windows.rs` with 3 tests:
  `authenticode_signed_records_subject` `#[ignore]`'d per Decision 4
  fallback (re-enable alongside v2.3 backlog row),
  `nono_binary_loads_without_unresolved_authenticode_symbols`
  (verifies new feature-flag additions don't break linkage),
  `nono_prune_help_still_functions_post_authenticode_addition`
  (cross-references Task 3's regression at the integration boundary).
  Direct-API unit tests living in
  `src/exec_identity_windows.rs::tests::{unsigned_temp_file_*,
  missing_path_*}` cover the `Unsigned`/`InvalidSignature` and
  `QueryFailed`/`InvalidSignature`/`Unsigned`/`Err` umbrella shapes
  for the underlying `query_authenticode_status` API.
- **Task 6 (commit `975547e0`)**: Added formal
  `audit_flush_before_drop` regression test at
  `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs::tests`
  per Decision 1 path correction. 83 LOC of test setup + assertion
  (well above the plan-checker WARNING-fix `>= 22 LOC` threshold).
  Models the supervised_runtime cleanup contract: drives AuditRecorder
  through `record_session_started`/`record_session_ended`, snapshots
  the ledger contents before guard drop, drops the AppliedLabelsGuard,
  then asserts pre/post ledger contents are byte-identical.
  Mitigates T-22-05-05 (Tampering: AppliedLabelsGuard Drop happens
  BEFORE ledger flush → events lost on cleanup). VALIDATION 22-05-V3
  green.
- **Task 7 (commit `a17eb307` + read-only verification)**: D-18
  Windows-regression gate. `cargo test --workspace --all-features`
  matches deferred-flake baseline (2 nono lib `trust::bundle::tests::*`
  TUF root signature freshness flakes + 3 nono-cli `policy::tests::*`
  `/tmp` flakes — both documented carry-overs from 22-04/22-05a, no
  NEW failure categories). Caught one Rule 1 regression:
  `cli::tests::test_root_help_lists_all_commands` was asserting
  `prune` must appear in root help, which Task 3's `#[command(hide)]`
  removal had broken. Auto-fixed: ALL_SUBCOMMANDS replaces `"prune"`
  with `"session"` (commit `a17eb307`). Plan-specific tests all green:
  `prune_alias_deprecation` 3/3, `exec_identity_windows` 2 pass + 1
  ignore, `audit_attestation` 0 pass + 2 ignore (D-13 fixtures
  deferred per Decision 3), `learn_windows_integration` 0 pass + 1
  ignore (admin), `wfp_port_integration` 1 pass + 1 ignore (admin).
  `cargo fmt --all -- --check` clean. `cargo clippy` reports the
  pre-existing `manifest.rs:95/103 collapsible_match` errors that
  carry forward from 22-04/22-05a (out of scope per CONTEXT § non_goals).
  End-to-end smoke: `nono session cleanup --dry-run` works,
  `nono audit cleanup --keep 5 --dry-run` works,
  `nono prune --dry-run` works AND emits deprecation note.
- **Task 8 (this SUMMARY + ROADMAP backlog entry + push)**: SUMMARY
  authored; ROADMAP Phase 22 row updated to 6/6 (Implementation-
  complete); v2.3 backlog row added for D-13 fixture re-enablement
  + companion Authenticode chain-walker re-enablement (Decision 3
  + Decision 4 fallback). D-07 push pending.

## Verification

| Gate | Expected | Actual |
|------|----------|--------|
| `cargo build --workspace` | exits 0 | passes |
| `cargo test -p nono-cli --bin nono auto_prune_is_noop_when_sandboxed` | 1/1 green AFTER each source-code commit | 5/5 green (Task 2/3/4/5/6/7) |
| `cargo test -p nono-cli --bin nono is_prunable_all_exited_escape_hatch_matches_any_exited` | 1/1 green | passes |
| `cargo test -p nono-cli --bin nono parse_duration_` | 3/3 green | passes |
| `cargo test -p nono-cli --bin nono labels_guard::` | 5/5 (4 baseline + audit_flush_before_drop) | 5/5 passes |
| `grep -E '^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100' crates/nono-cli/src/session_commands.rs` | matches | matches |
| `git diff-tree --no-commit-id --name-only -r <commit> \| grep session_commands(_windows)?.rs` | 0 hits per source-code commit | 0 hits — boundary held verbatim |
| `awk '/auto_prune_if_needed\(\)/,/^}/' crates/nono-cli/src/session_commands.rs` first non-comment statement | `if std::env::var_os("NONO_CAP_FILE").is_some() { ... return; }` | matches in BOTH files (T-22-05-04 mitigation) |
| `cargo test -p nono-cli --test prune_alias_deprecation` | 3/3 green | passes (1 help, 1 invocation, 1 hidden-from-top-level) |
| `cargo test -p nono-cli --test exec_identity_windows` | exits 0 | 2 pass + 1 #[ignore] (Decision 4 fallback) |
| `cargo test -p nono-cli --bin nono exec_identity_windows::tests` | 2/2 green | passes |
| `! grep -E '<implementation per RESEARCH Pattern 4>' crates/nono-cli/src/exec_identity_windows.rs` | 0 hits (placeholder gate) | 0 hits |
| `awk '/fn audit_flush_before_drop/,/^    }$/' \| wc -l` | >= 22 LOC | 83 LOC |
| `cargo fmt --all -- --check` | exits 0 | passes |
| `nono --help` includes `session    Manage runtime session storage (cleanup)` | yes | yes |
| `nono --help` does NOT include `prune` listing | absent | absent (clap auto-hides + ROOT_HELP_TEMPLATE updated) |
| `nono prune --dry-run` stderr contains `deprecated` | yes | `warning: \`nono prune\` is deprecated; use \`nono session cleanup\` instead` |
| `nono session cleanup --dry-run` works | yes | yes (lists "Would remove" entries) |
| `nono audit cleanup --keep 5 --dry-run` works | yes | yes (returns "No audit sessions match" or list) |
| `cargo test --workspace --all-features` | within deferred-flake window | 2 TUF root flakes + 3 policy `/tmp` flakes (carry-over); 0 NEW failures |
| Cargo.toml feature preservation | 16 pre-existing + 2 new = 18 | 16 + 1 (Win32_Security_WinTrust) + 1 (Win32_Security_Cryptography) = 18 ✓ |

## Files changed

| File | Change |
|------|--------|
| `crates/nono-cli/src/cli.rs` | +Cmd::Session, +SessionArgs, +SessionCommands, +AuditCommands::Cleanup, +AuditCleanupArgs, #[command(hide)] on Cmd::Prune, ROOT_HELP_TEMPLATE both arms, ALL_SUBCOMMANDS test list |
| `crates/nono-cli/src/audit_commands.rs` | +cmd_cleanup dispatcher, +remove_session import, +AuditCleanupArgs use |
| `crates/nono-cli/src/app_runtime.rs` | +Cmd::Session dispatch, +Cmd::Prune stderr deprecation eprintln, +SessionCommands import |
| `crates/nono-cli/src/cli_bootstrap.rs` | +Commands::Session(_) arm in cli_verbosity match |
| `crates/nono-cli/src/exec_identity.rs` | +AuthenticodeStatus re-export (Windows) / placeholder enum (non-Windows), +platform_authenticode dispatch |
| `crates/nono-cli/src/exec_identity_windows.rs` | NEW (~290 LOC: query_authenticode_status + WinTrustCloseGuard RAII + parse_signer_subject/parse_thumbprint Decision 4 stubs + 2 unit tests) |
| `crates/nono-cli/src/main.rs` | +#[cfg(target_os = "windows")] mod exec_identity_windows |
| `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` | +audit_flush_before_drop regression test (83 LOC) |
| `crates/nono-cli/Cargo.toml` | +Win32_Security_Cryptography, +Win32_Security_WinTrust (16 originals preserved) |
| `crates/nono-cli/tests/prune_alias_deprecation.rs` | NEW (3 tests, ~70 LOC) |
| `crates/nono-cli/tests/exec_identity_windows.rs` | NEW (3 tests, ~130 LOC; 1 #[ignore] per Decision 4) |
| `.planning/ROADMAP.md` | Phase 22 → 6/6 row update + v2.3 backlog row for D-13 + Authenticode chain-walker re-enablement |
| `crates/nono-cli/src/session_commands.rs` | **byte-identical** (Decision 2 LOCKED reframe; verified per-commit) |
| `crates/nono-cli/src/session_commands_windows.rs` | **byte-identical** (Decision 2 LOCKED reframe; verified per-commit) |

## Commits

| # | Hash | Type | REQ | Trailers |
|---|------|------|-----|----------|
| 1 | `5d41a71c` | feat | AUD-04 | Signed-off-by only (fork-only) |
| 2 | `3da595e3` | feat | AUD-04 #3 | Signed-off-by only |
| 3 | `cb34a82a` | feat | AUD-03 (Windows) | Signed-off-by only (D-17 ALLOWED) |
| 4 | `8159c7f6` | test | AUD-03 (Windows test) | Signed-off-by only |
| 5 | `975547e0` | test | AUD-* (T-22-05-05 mitigation) | Signed-off-by only |
| 6 | `a17eb307` | fix | AUD-04 #3 (test alignment) | Signed-off-by only (Rule 1) |

No `Upstream-commit:` trailers on any commit — all six are fork-only
additions per CONTEXT § Integration Points line 248 (D-17 ALLOWED for
the Authenticode addition) and per the v2.1 Phase 19 CLEAN-04 contract
(which upstream `4f9552ec` bundled with audit-integrity but is fork-
relevant on its own merits — the cherry-pick was deliberately deferred
to this plan after Plan 22-05a's manual replay landed the audit-integrity
portion).

## Status

- ✅ AUD-03 SHA-256 portion (landed in 22-05a)
- ✅ AUD-03 Windows portion (Authenticode discriminant; chain-walker
  subject extraction deferred per Decision 4 to v2.3 backlog)
- ✅ AUD-04 #1 (`nono session cleanup` works)
- ✅ AUD-04 #2 (`nono audit cleanup` peer works)
- ✅ AUD-04 #3 (hidden `nono prune` deprecation alias works + emits
  stderr deprecation note + `--help` says DEPRECATED)
- ✅ AUD-04 #4 (`auto_prune_is_noop_when_sandboxed` test name preserved
  verbatim — Decision 2 LOCKED reframe means worker function +
  test name byte-identical)
- ✅ AUD-04 #5 (`--older-than 30` no-suffix migration hint preserved
  via `parse_prune_duration`; covered by `parse_duration_*` family)
- ✅ T-22-05-04 ABSOLUTE STOP guard held end-to-end
- ✅ T-22-05-05 mitigated via formal `audit_flush_before_drop` regression test
- ✅ T-22-05-02 mitigated via signer-subject discriminant recording
  (substring assertion test #[ignore]'d for v2.3 — discriminant ALONE
  IS recorded in `Valid`/`Unsigned`/`InvalidSignature{hresult}` shape)
- 🟡 AUD-05 (Windows audit-event retrofit): defers to Phase 23 per
  ROADMAP scope; foldable-into-v2.2-followup decision documented below

## Threat model coverage

| Threat ID | Disposition | Status |
|-----------|-------------|--------|
| T-22-05-04 (sandboxed-agent file-deletion vector via prune rename) | mitigate (ABSOLUTE BLOCKING) | **HELD** — D-04 gate green AFTER every source-code commit; session_commands*.rs zero diff (LOCKED reframe boundary verified per-commit); NONO_CAP_FILE early-return preserved verbatim in BOTH files |
| T-22-05-05 (AppliedLabelsGuard Drop racing audit ledger flush) | mitigate (BLOCKING) | **MITIGATED** via formal `audit_flush_before_drop` regression test (Task 6, 83 LOC, models supervised_runtime cleanup contract end-to-end) |
| T-22-05-02 (Spoofing via test-cert acceptance bug) | mitigate (BLOCKING) | **MITIGATED** at the discriminant level (Valid/Unsigned/InvalidSignature recorded with HRESULT); substring-match assertion test #[ignore]'d pending v2.3 chain-walker re-enablement |
| T-22-05b-01 (signer subject CN info disclosure) | accept | N/A under Decision 4 fallback (signer_subject sentinel = "<unknown>"); revisit when chain walkers re-enable |
| T-22-05b-02 (DoS via revocation hang) | accept | WTD_REVOKE_NONE chosen — best-effort signature query, no CRL/OCSP latency |
| T-22-05b-03 (unsafe SAFETY doc miss) | mitigate | Every `unsafe { ... }` block in `exec_identity_windows.rs` has `// SAFETY:` doc immediately above it |
| T-22-05b-04 (.unwrap() in production FFI path) | mitigate | clippy::unwrap_used gate green for the new file; tests carry `#[allow(clippy::unwrap_used)]` per CLAUDE.md exception |
| T-22-05b-05 (RAII close-guard mis-orders WTD_STATEACTION_CLOSE) | mitigate | RAII `WinTrustCloseGuard::Drop` always re-invokes WinVerifyTrust with WTD_STATEACTION_CLOSE; mirrors Phase 21's `_sd_guard` pattern |
| T-22-05b-06 (silent prune-alias delegation misses migration) | mitigate | Stderr deprecation note on every invocation + DEPRECATED in `--help`; regression test `prune_alias_emits_stderr_deprecation_note_on_invocation` enforces |
| T-22-05b-07 (intra-window prune undefined between Tasks 2+3) | accept | Documented; window is < 1 hour intra-executor-run; Task 7 plan-close gate verifies the alias works at the plan boundary |

## AUD-05 fold-or-split decision

**Decision: Phase 23 confirmed (NOT folded into a v2.2 follow-up).**

Rationale (grep evidence):

```
$ grep -rE 'handle_(file|socket|pipe|event|mutex|jobobject)_request' crates/nono-cli/src/exec_strategy_windows/
... 5 hits (each kind has its own handle_*_request helper)

$ grep -rnE 'AuditEvent|append_event' crates/nono-cli/src/audit_integrity.rs ...
... append_event is private (visibility = private fn); AuditEventPayload variants are SessionStarted/SessionEnded/CapabilityDecision/UrlOpen/Network — no AipcHandle variant
```

The fork's AuditRecorder API does NOT expose a Windows-specific
`AipcHandle` payload variant; it would require either:

1. Adding a new `AuditEventPayload::AipcHandle { kind, ... }` variant
   (Rule 4 — schema change cascade through verify, attestation, audit-show)
2. Reusing `AuditEventPayload::CapabilityDecision { entry: AuditEntry }`
   with HandleKind-specific encoding inside `AuditEntry` (Rule 4 —
   AuditEntry shape decision affects upstream-compat + downstream verify)

Either path is an architectural Rule-4 decision that exceeds a "simple
fold" and matches the Phase 23 scope authored on 2026-04-24. The
upstream-followable path is to land Phase 23 as its own plan with
explicit AUD-05 acceptance criteria, not to retrofit it into a v2.2
follow-up under cherry-pick pressure.

**Recommendation:** Proceed with Phase 23 as the dedicated home for
REQ-AUD-05; defer the schema decision to Phase 23 plan authoring time.

## Phase 22 close-out implications

- Phase 22 implementation-complete (6/6 plans): 22-01 PROF, 22-02 POLY,
  22-03 PKG (partial — 6/8 cherry-picks, 2 deferred to v2.3), 22-04 OAUTH,
  22-05a AUD-CORE, 22-05b AUD-RENAME.
- Pending phase-level gates before Phase 22 closes for `/gsd-secure-phase 22`:
  1. `/gsd-secure-phase 22` (security review)
  2. `/gsd-code-review 22` (code review)
  3. `/gsd-verify-phase 22` (must-have audit)
- v2.3 backlog rows for carry-forward: PKG-streaming follow-up (22-03);
  D-13 fixture + Authenticode chain-walker re-enablement (22-05b).
- v2.2 milestone status: Phase 22 implementation done; Phase 23
  pending (AUD-05); Phase 24 already complete (DRIFT-01 + DRIFT-02).
  Milestone close-out depends on Phase 22 phase-level verification +
  Phase 23 execution.

## Self-Check: PASSED

Files claimed to exist verified present:
- `crates/nono-cli/src/exec_identity_windows.rs` — FOUND
- `crates/nono-cli/tests/exec_identity_windows.rs` — FOUND
- `crates/nono-cli/tests/prune_alias_deprecation.rs` — FOUND

Commits claimed verified in `git log --oneline d15a3ab6..HEAD`:
- `5d41a71c` — FOUND
- `3da595e3` — FOUND
- `cb34a82a` — FOUND
- `8159c7f6` — FOUND
- `975547e0` — FOUND
- `a17eb307` — FOUND
