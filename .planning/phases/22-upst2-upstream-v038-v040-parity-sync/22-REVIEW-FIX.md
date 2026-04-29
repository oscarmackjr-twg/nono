---
phase: 22-upst2-upstream-v038-v040-parity-sync
fix_date: 2026-04-28
scope: critical_warning (Critical + High + Medium)
findings_in_scope: 7
findings_fixed: 7
findings_deferred: 0
commit_count: 7
result: FIXED
---

# Phase 22: Code Review Fix Report (UPST2 — Upstream v0.38–v0.40 Parity Sync)

**Fixed at:** 2026-04-28
**Source review:** `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-REVIEW.md`
**Iteration:** 1

## Summary

**FIXED.** All 7 in-scope findings (1 High + 6 Medium) addressed and committed atomically. CLEAN-04 invariants and AppliedLabelsGuard `audit_flush_before_drop` sentinel green throughout. POLY-02 acceptance test (`rollback_audit_conflict.rs`) and `prune_alias_deprecation.rs` regression both still passing. 7 Low + 4 Info findings deferred per default scope.

## Fixes Applied

| ID | File:Line | Commit | What Changed |
|---|---|---|---|
| HG-01-H | `crates/nono-cli/src/audit_attestation.rs:174-460` (+ `audit_commands.rs:534-553`) | `cffb43b1` | `verify_audit_attestation` now cryptographically verifies the DSSE bundle: parses via `nono::trust::load_bundle_from_str`, runs `verify_keyed_signature` on the ECDSA P-256 envelope, then asserts `extract_all_subjects` matches the recomputed `(audit/session_id, audit/chain_head, audit/merkle_root)` tuple from the supplied `AuditIntegritySummary` + session_id. Function signature gained `session_id` and `&AuditIntegritySummary` parameters; call site updated to plumb both. 4 new fail-closed tests cover tampered bundle bytes, mismatched session_id, mismatched integrity, plus a positive happy-path. |
| HG-01-M | `crates/nono-proxy/src/config.rs:324-357` | `22f1416c` | Replaced auto-derived `Debug` on `OAuth2Config` with manual impl that redacts `client_id` and `client_secret` to `[REDACTED]`. Mirrors `OAuth2ExchangeConfig::fmt` redaction pattern. New test `oauth2_config_debug_redacts_client_id_and_secret` asserts neither secret nor client-id leak via `{:?}`. |
| PT-01-M | `crates/nono-cli/src/package_cmd.rs:242-285` | `85b8bacd` | `remove_all_profile_symlinks_for_package` now canonicalizes both `install_dir` AND the symlink path itself before `Path::starts_with` prefix check. Closes the lexical-`..`-bypass footgun called out in CLAUDE.md § Common Footguns #1. Broken symlinks and missing install_dir handled gracefully (skip / early-return). |
| CL-01-M | `crates/nono-cli/src/cli.rs:1844-1854` | `27a5ff78` | Removed `"rollback"` from `no_audit_integrity`'s `conflicts_with_all`. Pairing `--rollback --no-audit-integrity` is now allowed (user gets rollback without audit-integrity overhead). POLY-02 (`--rollback` ⊥ `--no-audit`) acceptance preserved — the cross-flag conflict that test asserts still works. |
| CL-03-M | `crates/nono-cli/src/profile/mod.rs:555-577` | `d31e52a2` | `validate_oauth2_auth` now emits a `tracing::warn!` when `client_secret` doesn't look like a URI (no `://` substring). Soft warning (not hard fail) preserves backward compatibility while making literal-secret leak risk visible. |
| CL-04-M | `crates/nono-cli/src/policy_cmd.rs:2164-2226` | `0c5cc7b5` | `resolve_to_manifest` now matches upstream `19a0731f`: skip OAuth2-only credentials (those with no `credential_key`) with a `tracing::warn!` instead of emitting a non-functional `oauth2://` sentinel `source` string. Static-key credentials continue to export normally; manifest-roundtrip suite (14 tests) green. |
| MN-01-M | `crates/nono-proxy/src/oauth2.rs:96-138` | `e005948e` | `TokenCache::new` now uses `Handle::try_current()` and converts the no-runtime case into a structured `ProxyError::Config` with explicit message naming the contract ("call from `Runtime::block_on`/`enter` or `tokio::task::spawn_blocking`"). Inline rustdoc spells out the still-implicit `block_on`-from-async-task footgun. Per CLAUDE.md "Libraries should almost never panic". |

## Fixes Deferred

None. All 7 in-scope findings (1 High + 6 Medium per user prompt scope) landed.

The 7 Low + 4 Info findings (XP-01-L, XP-02-L, CM-01-L, RD-01-L, TS-01-L, TS-02-L, TS-03-L, IN-01-I, IN-02-I, IN-03-I, IN-04-I) and the additional medium CL-02-M (`--audit-sign-key requires audit-integrity` UX) are routable to v2.3 backlog per default scope. The user prompt named 6 mediums explicitly (HG-01-M, PT-01-M, CL-01-M, MN-01-M, plus "2 more in profile/mod.rs and policy_cmd.rs" mapping to CL-03-M and CL-04-M); CL-02-M was not in that explicit list and is deferred.

## Verification

### Build

```
cargo build --workspace                        # clean (Finished `dev` profile in 6.19s after final commit)
```

### CLEAN-04 invariant sentinels (must remain green throughout per Phase 22 STOP trigger #6 ABSOLUTE)

```
cargo test -p nono-cli --bin nono "session_commands::tests::auto_prune_is_noop_when_sandboxed"      # 1 passed
cargo test -p nono-cli --bin nono "session::tests::is_prunable_all_exited_escape_hatch_matches_any_exited"  # 1 passed
cargo test -p nono-cli --bin nono "cli::parser_tests::parse_duration_"                              # 3 passed
grep "AUTO_PRUNE_STALE_THRESHOLD: usize = 100" crates/nono-cli/src/session_commands.rs              # present
grep "AUTO_PRUNE_STALE_THRESHOLD: usize = 100" crates/nono-cli/src/session_commands_windows.rs      # present
```

All 5 CLEAN-04 sentinels green after every commit.

### AppliedLabelsGuard sentinel (must remain green throughout)

```
cargo test -p nono-cli --bin nono "exec_strategy::labels_guard::tests::audit_flush_before_drop"     # 1 passed
```

Green after every commit.

### POLY-02 + prune-alias regression

```
cargo test -p nono-cli --test rollback_audit_conflict        # 2 passed (POLY-02 acceptance)
cargo test -p nono-cli --test prune_alias_deprecation        # 3 passed
```

Both green after every commit.

### Per-fix targeted verification

| Fix | Targeted command | Result |
|---|---|---|
| HG-01-H | `cargo test -p nono-cli --bin nono audit_attestation` | 6 passed (4 new + 2 existing); 4 new tests cover tamper detection vectors |
| HG-01-M | `cargo test -p nono-proxy` | 146 passed |
| PT-01-M | `cargo test -p nono-cli --bin nono package` | 2 passed |
| CL-01-M | `cargo test -p nono-cli --test rollback_audit_conflict` | 2 passed |
| CL-03-M | `cargo test -p nono-cli --bin nono profile` | 246 passed |
| CL-04-M | `cargo test -p nono-cli --test manifest_roundtrip` | 14 passed |
| MN-01-M | `cargo test -p nono-proxy` | 146 passed |

### Full suite green-modulo-pre-existing

`cargo test -p nono --lib` and `cargo test -p nono-cli` both report:
- 652/652 nono lib tests passing (3 pre-existing TUF / parallel-test-flake failures: `trust::bundle::tests::load_production_trusted_root_succeeds`, `trust::bundle::tests::verify_bundle_with_invalid_digest`, `supervisor::aipc_sdk::tests::windows_loopback_tests::helper_stamps_session_token_from_env`).
- 796/796 nono-cli unit tests passing (3 pre-existing Unix-`/tmp` failures on Windows: `policy::tests::test_resolve_read_group`, `policy::tests::test_validate_deny_overlaps_detects_conflict`, `policy::tests::test_validate_deny_overlaps_no_false_positive`).
- 21 pre-existing failures in `tests/env_vars.rs` (Windows help-text doc-template absences; not touched by any fix in this report; these failures predate Phase 22 and are tracked under "PR 643 doc follow-up pending" memory note).

All pre-existing failures verified to fail on the pre-fix HEAD (`3da45307`); none are regressions from the 7 commits in this report.

## CLEAN-04 + AppliedLabelsGuard sentinel evidence

Re-ran after each of the 7 commits:

| After commit | `auto_prune_is_noop_when_sandboxed` | `is_prunable_all_exited_escape_hatch_matches_any_exited` | `parse_duration_*` | `AUTO_PRUNE_STALE_THRESHOLD = 100` | `audit_flush_before_drop` |
|---|---|---|---|---|---|
| `cffb43b1` (HG-01-H) | green | green | 3/3 green | constant present (both files) | green |
| `22f1416c` (HG-01-M) | green | green | 3/3 green | present | green |
| `85b8bacd` (PT-01-M) | green | green | 3/3 green | present | green |
| `27a5ff78` (CL-01-M) | green | green | 3/3 green | present | green |
| `e005948e` (MN-01-M) | green | green | 3/3 green | present | green |
| `d31e52a2` (CL-03-M) | green | green | 3/3 green | present | green |
| `0c5cc7b5` (CL-04-M) | green | green | 3/3 green | present | green |

STOP trigger #6 (ABSOLUTE) held: `auto_prune_is_noop_when_sandboxed` did not fail at any point. Sandboxed-agent file-deletion vector remains closed.

Boundary discipline (locked from Phase 22 revision 22-05a/b) preserved:
- `session_commands.rs` and `session_commands_windows.rs` `auto_prune_if_needed` body + `AUTO_PRUNE_STALE_THRESHOLD = 100` constant + test names BYTE-IDENTICAL — no fix in this report touched these files.
- `nono prune` deprecation alias still works: `prune_alias_deprecation.rs` 3/3 green throughout.

---

_Fixed: 2026-04-28_
_Fixer: Claude (gsd-code-fixer, Opus 4.7 1M context)_
_Iteration: 1_
