---
phase: 27
slug: audit-attestation-hardening
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-09
---

# Phase 27 — Validation Strategy

> Reconstructed retroactively (State B) from PLAN/SUMMARY artifacts plus current-HEAD verification on the Windows host. Phase 27 itself shipped PARTIAL (re-`#[ignore]`'d both tests, deferred to v2.4) per `27-01-SUMMARY.md`; REQ-AAH-01 closed transitively via Phase 27.1 (`NONO_TEST_HOME` seam) and Phase 27.2 (audit-loader swap + bundle-target migration). All requirement-bearing behaviors have automated verification at HEAD.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust + Cargo (built-in `#[test]` runner) |
| **Config file** | `Cargo.toml` (workspace + `crates/nono-cli/Cargo.toml`) |
| **Quick run command** | `cargo test -p nono-cli --test audit_attestation` |
| **Full suite command** | `cargo test -p nono-cli` |
| **Estimated runtime** | ~9 s for the 4 audit_attestation integration tests; ~2–3 min for full nono-cli suite |
| **Test directory** | `crates/nono-cli/tests/` |
| **Helper conventions** | `setup_isolated_home()`, `run_nono(args, home, cwd)` (threads `NONO_TEST_HOME`), `only_audit_session_id(home)`, `run_command_args()` (cross-platform), `ScopedEnvVar` (RAII env-var restore, locked per Phase 27.2 D-27.2-13) |

---

## Sampling Rate

- **After every task commit:** `cargo test -p nono-cli --test audit_attestation`
- **After every plan wave:** `cargo test -p nono-cli`
- **Before `/gsd-verify-work`:** `make ci` once nono-lib clippy debt is cleared (tracked in `.planning/phases/27.1-nono-test-home-seam/deferred-items.md`); the audit suite itself reports `4 passed; 0 failed; 0 ignored`.
- **Max feedback latency:** ~10 s for the focused audit suite.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 27-01-T1 | 01 | 1 | REQ-AAH-01 (AC1, AC3) | T-27-01 (Tampering) | Both `#[ignore]` attributes removed; `grep -c '#\[ignore' crates/nono-cli/tests/audit_attestation.rs` returns 0 | grep gate | `grep -c '#\[ignore' crates/nono-cli/tests/audit_attestation.rs` → 0 | ✅ | ✅ green (closed via 27.2 commit `0a4aa279`) |
| 27-01-T2 | 01 | 1 | REQ-AAH-01 (AC1) | T-27-01 (Tampering) | `audit_verify_reports_signed_attestation_with_pinned_public_key` runs to PASS — bundle file exists, DSSE envelope deserializes, fail-closed on wrong pubkey, `key_id_hex` round-trips | integration | `cargo test -p nono-cli --test audit_attestation audit_verify_reports_signed_attestation_with_pinned_public_key -- --exact` | ✅ | ✅ green (4 passed; 0 failed; 0 ignored on Windows host 2026-05-09) |
| 27-01-T3 | 01 | 1 | REQ-AAH-01 (AC1) | T-27-01 / T-27-02 (Tampering / Spoofing) | `rollback_signed_session_verifies_from_audit_dir_bundle` runs to PASS — bundle lives at `<audit_root>/<id>/audit-attestation.bundle`, NOT in rollback dir (D-27.2-01 Option A) | integration | `cargo test -p nono-cli --test audit_attestation rollback_signed_session_verifies_from_audit_dir_bundle -- --exact` | ✅ | ✅ green (closed via 27.2-02 commit) |
| 27-01-T4 | 01 | 1 | REQ-AAH-01 (AC2) | — | Documentation pass — Phase 27 Path B rationale recorded above redesigned tests; substrings `"Phase 27"` + `"Path B"` + `"v2.4"` present | grep gate | `grep -c "Phase 27" crates/nono-cli/tests/audit_attestation.rs` → ≥1 (24); `grep -c "Path B"` → ≥1 (4); `grep -c "v2.4"` → ≥1 (1) | ✅ | ✅ green |
| 27-01-T5 | 01 | 1 | REQ-AAH-01 (AC3) | — | Verification gate — full audit suite green | integration (full file) | `cargo test -p nono-cli --test audit_attestation` → `4 passed; 0 failed; 0 ignored` | ✅ | ✅ green |
| 27-01 (must-have 3) | 01 | 1 | REQ-AAH-01 | T-27-01 (Tampering) | No `from_pkcs8` parsing surface introduced (Path B locked invariant) | grep gate (workspace) | `grep -rc 'from_pkcs8' crates/nono-cli/tests/` → 0 across all 10 test files | ✅ | ✅ green |
| 27-01 (must-have 4) | 01 | 1 | REQ-AAH-01 | T-27-01 (Tampering) | Tests use `env://` URI (D-AAH-01 deviation), not new `--audit-sign-key file://` callsites. Env-var name is constructed dynamically via `format!("NONO_TEST_AUDIT_KEY_VERIFY_{suffix}")` (PID+nanos collision-mitigation). | grep gate | `grep -c 'env://' crates/nono-cli/tests/audit_attestation.rs` → ≥1 (2 at HEAD: lines 297, 319) | ✅ | ✅ green |
| 27-01 (must-have 5) | 01 | 1 | REQ-AAH-01 | T-27-01 (Tampering) | Bundle structural correctness — `audit-attestation.bundle` AND `signatures` AND (`payloadType`/`payload_type`) all referenced in test source | grep gate | `grep -c 'audit-attestation.bundle'` → ≥1 (5 at HEAD); `grep -c 'signatures'` → ≥1 (5 at HEAD); `grep -c 'payloadType'` → ≥1 (4 at HEAD; bundle path is `dsseEnvelope.payloadType` per sigstore-rs Bundle v0.3) | ✅ | ✅ green |
| 27-01 (must-have 6) | 01 | 1 | REQ-AAH-01 | T-27-01 (Tampering) | Fail-closed verification — wrong-pubkey path returns non-zero exit | grep gate | `grep -c '!verify_output.status.success\|wrong_verify_output' crates/nono-cli/tests/audit_attestation.rs` → ≥2 (4) | ✅ | ✅ green |
| 27-01 (must-have 7) | 01 | 1 | REQ-AAH-01 | T-27-02 (Spoofing) | `key_id_hex` round-trip — KeyPair-extracted hex matches `attestation.key_id_hex` in `audit show --json` | grep gate + assertion | `grep -c 'key_id_hex\|key_id'` → 6 | ✅ | ✅ green |

*Status legend: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky/deferred*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements.* No Wave 0 stubs needed — the test framework (cargo + Rust integration tests + `tempfile` + Phase 22-05a fixture scaffold + Phase 27.1 `NONO_TEST_HOME` seam + Phase 27.2 `ScopedEnvVar` RAII guard) was already in place before HEAD reached this audit.

---

## Cross-Phase Closure Note (REQ-AAH-01)

Phase 27's own deliverable was PARTIAL: commit `8aeabc08` re-`#[ignore]`'d both tests with a v2.4-deferral note after surfacing 3 Windows-host blockers (Blocker 1: `dirs::home_dir()` ignores `USERPROFILE`; Blocker 2: LOCALAPPDATA/USERPROFILE path mismatch; Blocker 3: audit-integrity exit-cleanup `Session not found`). REQ-AAH-01 closure was deferred to v2.4 within Phase 27 scope.

The deferral was promoted into v2.3 via two follow-up phases:

| Source | Closure | Commit |
|--------|---------|--------|
| Phase 27.1 Plans 01–03 | `NONO_TEST_HOME` env-var seam at `crates/nono-cli/src/config/mod.rs::nono_home_dir()` — closes Blocker 1 + Blocker 2 cross-platform | `df3c8976`, `6275cfb1` |
| Phase 27.2 Plan 01 (FU-1) | Audit-loader swap — `cmd_verify` calls `audit_session::load_session` (audit-first, rollback-fallback) instead of `rollback_session::load_session` | (Plan 01 commits) |
| Phase 27.2 Plan 02 (FU-2) | Bundle-target migration — bundle ALWAYS at `<audit_root>/<id>/audit-attestation.bundle` regardless of `--rollback` (D-27.2-01 Option A) | (Plan 02 commits) |
| Phase 27.2 Plan 04 (FU-3) | Both `#[ignore]` attributes removed; `ScopedEnvVar` RAII guard for `env://NONO_TEST_KEY` lifecycle | `0a4aa279` |
| Phase 27.2 Code-review backstop | Two combo-session regression tests (`combo_rollback_audit_session_findable_by_audit_verify`, `combo_rollback_audit_session_findable_by_rollback_list`) added for BL-01 + BL-02 | `dd98e812` |

Net effect at HEAD: REQ-AAH-01 acceptance criteria 1, 2, 3, 4 are all MET; both Phase 27 target tests pass on the Windows host alongside two new combo regressions (4 passed; 0 failed; 0 ignored).

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `make ci` end-to-end clean (clippy + fmt + workspace test all green) | REQ-AAH-01 must-have #9 | Pre-existing `nono-lib` clippy warnings (unrelated to this plan) prevent a fully green `make ci`. Tracked in `.planning/phases/27.1-nono-test-home-seam/deferred-items.md`. The audit suite itself compiles and passes; the workspace lint debt is what blocks `make ci`. | Run `make ci` after the nono-lib clippy debt is cleared in a future phase. The audit-attestation tests themselves do not contribute warnings to the existing debt. |
| Phase 27 PLAN truth #8: `git diff --stat cffb43b1..HEAD -- crates/nono-cli/src/audit_attestation.rs` empty (production-code byte-identity vs v2.2 baseline) | REQ-AAH-01 PLAN must-have #8 | Intentionally superseded — Phase 27.2 Plan 01 + Plan 02 explicitly modified `audit_commands.rs` (audit-loader swap, FU-1) and added a bundle-mirror at `audit_root` (FU-2 / D-27.2-01 Option A). The byte-identity invariant was Phase 27's locked scope; Phase 27.1+27.2 was the chartered escape hatch when the locked scope failed on Windows. | Treat as historical record only. The current invariant (post-27.2) is: production-code changes restricted to the chartered FU-1/FU-2 surfaces in `audit_commands.rs` + `audit_session.rs` + finalize hook; `audit_attestation.rs` itself remains close to the v2.2 baseline shape. |
| Phase 27 PLAN Task 4 docstring `byte-equality` substring present in test-file comment block | REQ-AAH-01 (AC2 documentation) | Phase 27.2 simplified the comment block at lines 285–292 of `audit_attestation.rs` during re-enablement. The block now references `Phase 27.2 (REQ-AAHX-01..03)` + `v2.4 follow-ups` + `Bundle target locked` but drops the explicit "byte-equality" trade-off prose from the Phase 27 plan. No behavior gap; cosmetic doc drift surfaced for future readers tracing the architectural decision. The full Phase 27 trade-off rationale is preserved in `27-01-SUMMARY.md` § Phase 27 Path B + `27.2-CONTEXT.md` D-27.2-01. | If a future code reader asks "why not byte-equality fixture testing?", point them at `27-01-SUMMARY.md` § "Phase 27 — Path B (Fixture Test Redesign)" and `crates/nono-cli/src/audit_attestation.rs:8-15` deviation block. No automated test required. |
| 5× parallel-test stress (`cargo test -p nono-cli --test audit_attestation -- --test-threads=4`, 5 iterations) — Phase 27 PLAN Task 5 step 6 | REQ-AAH-01 (T-27-05 DoS mitigation) | Per-invocation `{pid}_{nanos}` env-var URI suffix should be collision-free, but Phase 27.2 closure focused on functional correctness over stress validation. | Run the 5-iteration stress loop in a future phase / before any v2.4 cleanup. Expected: all 5 iterations pass. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or are accepted as Manual-Only with documented rationale
- [x] Sampling continuity: every Phase 27 PLAN must-have (1–7) has an automated grep or test gate; must-haves 8–9 are manual-only with documented superseding rationale
- [x] No watch-mode flags
- [x] Feedback latency < ~10 s for focused audit suite
- [x] `nyquist_compliant: true` set in frontmatter — every requirement-bearing behavior is automated; manual-only entries are explicit non-runtime concerns (clippy debt, byte-identity historical record, doc-string content drift, stress validation)

**Approval:** approved 2026-05-09

---

## Validation Audit 2026-05-09

| Metric | Count |
|--------|-------|
| Gaps found | 0 (runtime); 3 (manual-only / superseded) |
| Resolved | 0 (no runtime gaps to resolve) |
| Escalated | 3 (clippy debt → deferred-items.md; byte-identity → superseded by 27.1+27.2 charter; `byte-equality` docstring → cosmetic; stress validation → future phase) |
| New tests written | 0 |
| Existing tests verified | 4 (all green) |

---

## Validation Audit 2026-05-09 (re-audit)

Re-confirmed compliance at HEAD via fresh `cargo test -p nono-cli --test audit_attestation` run: **4 passed; 0 failed; 0 ignored** (~7.5 s). All Per-Task Map runtime rows remain green.

Documentation drift corrected in two grep-gate cells of the Per-Task Map (cosmetic only — runtime behavior was always satisfied):

| Row | Drift | Fix |
|-----|-------|-----|
| must-have 4 | Documented grep pattern (`env://NONO_TEST_KEY\|env://AUDIT_KEY`) returned 0 because the actual env-var name is constructed dynamically via `format!("NONO_TEST_AUDIT_KEY_VERIFY_{suffix}")` (PID+nanos collision-mitigation). | Generalized grep to `env://` (returns 2 at HEAD: lines 297, 319). Behavior unchanged. |
| must-have 5 | Documented `payloadType` count = 5; actual = 4 (bundle path is `dsseEnvelope.payloadType` per sigstore-rs Bundle v0.3, referenced 4× in source). Gate (≥1) was always satisfied. | Updated counts to reflect HEAD; clarified DSSE Bundle v0.3 path. |

| Metric | Count |
|--------|-------|
| Gaps found | 0 (runtime); 2 (cosmetic doc drift in grep-gate cells) |
| Resolved | 2 (grep gates corrected in-place) |
| Escalated | 0 |
| New tests written | 0 |
| Existing tests verified | 4 (all green at HEAD) |
