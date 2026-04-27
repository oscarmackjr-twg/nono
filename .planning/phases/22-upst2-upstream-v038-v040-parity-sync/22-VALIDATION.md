---
phase: 22
slug: upst2-upstream-v038-v040-parity-sync
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-27
---

# Phase 22 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust workspace) |
| **Config file** | Cargo.toml (workspace + per-crate) |
| **Quick run command** | `cargo test --workspace --lib` |
| **Full suite command** | `make ci` (`cargo build --workspace` + `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` + `cargo fmt --all -- --check` + `cargo test --workspace --all-features`) |
| **Estimated runtime** | ~120s quick / ~600s full on Windows host |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace --lib` (or scoped `cargo test -p <crate> <test_name>` when iterating on a single test surface)
- **After every plan wave:** Run `make ci` — D-18 Windows-regression gate (workspace + clippy + fmt + tests)
- **Before `/gsd-verify-work`:** Full `make ci` green AND D-18 supplemental suites:
  - Phase 15 5-row detached-console smoke (`nono run` → `nono ps` → `nono stop`)
  - `wfp_port_integration` (admin + nono-wfp-service available; documented-skipped otherwise)
  - `learn_windows_integration` (ETW learn smoke)
- **Max feedback latency:** 120s (lib tests) / 600s (full ci)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 22-01-T1 | 01 | 0 | REQ-PROF-01 | — | `unsafe_macos_seatbelt_rules: Vec<String>` deserializes on Windows; runtime application is macOS-only | unit | `cargo test -p nono-cli profile::tests::deserialize_seatbelt_rules` | ❌ W0 | ⬜ pending |
| 22-01-T2 | 01 | 0 | REQ-PROF-02 | — | `packs: Vec<PackRef>` + `command_args: Vec<String>` deserialize via serde defaults; pack resolution short-circuits on Windows when registry unavailable | unit | `cargo test -p nono-cli profile::tests::deserialize_packs_and_command_args` | ❌ W0 | ⬜ pending |
| 22-01-T3 | 01 | 0 | REQ-PROF-03 | T-PROF-03 | `oauth2.token_url=http://...` rejected fail-closed; `client_secret: keyring://...` resolves through `nono::keystore::load_secret` | unit | `cargo test -p nono-cli profile::tests::oauth2_http_token_url_rejected` + `oauth2_keystore_secret_resolves` | ❌ W0 | ⬜ pending |
| 22-01-T4 | 01 | 0 | REQ-PROF-04 | — | `claude-no-keychain` builtin loads via `Profile::load_builtin("claude-no-keychain")` and inherits `claude-code` `override_deny` entries | unit | `cargo test -p nono-cli profile::builtin::tests::claude_no_keychain_loads` | ❌ W0 | ⬜ pending |
| 22-01-V1 | 01 | 0 | REQ-PROF-01..04 | — | Profile struct flow Phase 18.1 wiring intact (`Profile::resolve_aipc_allowlist` end-to-end on Windows) | regression | `cargo test -p nono-cli aipc_allowlist::e2e` | ✅ existing | ⬜ pending |
| 22-02-T1 | 02 | 0 | REQ-POLY-01 | T-POLY-01 | Orphan `override_deny` (no matching grant) fails `Profile::resolve` with `NonoError::PolicyError { kind: OrphanOverrideDeny, .. }` | unit | `cargo test -p nono-cli policy::tests::orphan_override_deny_rejected` | ❌ W0 | ⬜ pending |
| 22-02-T2 | 02 | 0 | REQ-POLY-02 | T-POLY-02 | `--rollback` + `--no-audit` rejected at clap parse with `conflicts_with` error | unit | `cargo test -p nono-cli cli::tests::rollback_no_audit_conflict` | ❌ W0 | ⬜ pending |
| 22-02-T3 | 02 | 0 | REQ-POLY-03 | — | `.claude.lock` resolves through `allow_file` (not `allow_dir`); pre-existing tests for `.claude.lock` write/read still green | unit | `cargo test -p nono-cli policy::tests::claude_lock_in_allow_file` | ❌ W0 | ⬜ pending |
| 22-02-V1 | 02 | 0 | REQ-POLY-01..03 | T-POLY-01,02 | Existing fork built-in profiles all pass POLY-01 audit before POLY-01 fail-closed lands | regression | `cargo test -p nono-cli profile::builtin::tests::all_builtins_resolve` | ✅ existing | ⬜ pending |
| 22-03-T1 | 03 | 1 | REQ-PKG-01 | — | `nono package pull <name>` downloads + installs to `%LOCALAPPDATA%\nono\packages\<name>`; `remove`/`search`/`list` work end-to-end | integration | `cargo test -p nono-cli package_cmd::tests::pull_remove_search_list` | ❌ W0 | ⬜ pending |
| 22-03-T2 | 03 | 1 | REQ-PKG-02 | T-PKG-02 | Long paths use `\\?\` prefix; path-traversal (`..`, symlinks, UNC aliasing) rejected fail-closed | unit | `cargo test -p nono-cli package_cmd::tests::long_path_prefix_applied` + `path_traversal_rejected` | ❌ W0 | ⬜ pending |
| 22-03-T3 | 03 | 1 | REQ-PKG-03 | — | Hook installer registers via fork's `hooks.rs` Windows path; idempotent (re-install is no-op) | integration | `cargo test -p nono-cli package_cmd::tests::hook_install_idempotent` | ❌ W0 | ⬜ pending |
| 22-03-T4 | 03 | 1 | REQ-PKG-04 | T-PKG-04 | Streaming download verifies signed-artifact signature; tampered artifact rejected | integration | `cargo test -p nono-cli package_cmd::tests::signed_artifact_verify` | ❌ W0 | ⬜ pending |
| 22-03-V1 | 03 | 1 | REQ-PKG-* | — | Phase 21 single-file grant semantics survive package install path | regression | `cargo test -p nono --test sandbox_windows wsfg::install_path` | ✅ existing | ⬜ pending |
| 22-04-T1 | 04 | 1 | REQ-OAUTH-01 | T-OAUTH-01 | `OAuth2Config` client-credentials flow exchanges + caches token; expired token refreshes; cache zeroized on drop | unit | `cargo test -p nono-proxy oauth2::tests::client_credentials_token_exchange` + `cache_refresh_on_expiry` + `zeroize_on_drop` | ❌ W0 | ⬜ pending |
| 22-04-T2 | 04 | 1 | REQ-OAUTH-02 | T-OAUTH-02 | Reverse-proxy HTTP upstream gating: loopback (127.0.0.0/8, ::1) allowed; non-loopback rejected fail-closed | unit | `cargo test -p nono-proxy reverse::tests::http_upstream_loopback_only` | ❌ W0 | ⬜ pending |
| 22-04-T3 | 04 | 1 | REQ-OAUTH-03 | — | `--allow-domain` works in strict-proxy-only mode (no host-network bypass) | integration | `cargo test -p nono-cli network_policy::tests::allow_domain_strict_proxy` | ❌ W0 | ⬜ pending |
| 22-04-V1 | 04 | 1 | REQ-OAUTH-* | T-OAUTH-01 | Phase 9 WFP port-level filtering survives proxy-credentials code path | regression | `cargo test -p nono-cli wfp_port_integration::ignored::oauth_proxy` | ✅ existing | ⬜ pending |
| 22-05-T1 | 05 | 2 | REQ-AUD-01 | T-AUD-01 | `--audit-integrity` produces hash-chained ledger with populated `chain_head` per event | unit | `cargo test -p nono-cli audit::tests::hash_chain_continuity` | ❌ W0 | ⬜ pending |
| 22-05-T2 | 05 | 2 | REQ-AUD-02 | T-AUD-02 | DSSE/in-toto attestation signed via `keyring://nono/audit`; `nono audit verify` succeeds; tampered ledger rejected | integration | `cargo test -p nono-cli audit_attestation::sign_and_verify` (ported from upstream `9db06336`) + `audit_attestation::tampered_ledger_rejected` | ❌ W0 | ⬜ pending |
| 22-05-T3 | 05 | 2 | REQ-AUD-03 | T-AUD-03 | Windows exec-identity records Authenticode signature when present; SHA-256 fallback when unsigned | unit (Windows-gated) | `cargo test -p nono-cli --test exec_identity_windows authenticode_signed` + `authenticode_unsigned_sha256_fallback` | ❌ W0 | ⬜ pending |
| 22-05-T4 | 05 | 2 | REQ-AUD-04 | — | `nono session cleanup` (renamed from `prune`) preserves CLEAN-04 invariants; `nono audit cleanup` peer subcommand operates on audit ledgers | unit | `cargo test -p nono-cli session_commands::auto_prune_is_noop_when_sandboxed` (existing) + `session_commands::session_cleanup_renamed` (new) | partial ✅ | ⬜ pending |
| 22-05-V1 | 05 | 2 | REQ-AUD-04 | T-AUD-04 | CLEAN-04 invariants survive rename: 100-file auto-sweep, `--older-than` require-suffix, `--all-exited` escape hatch, sandboxed-no-op | regression | `cargo test -p nono-cli session_commands::tests::{auto_prune_is_noop_when_sandboxed,is_prunable_all_exited_escape_hatch_matches_any_exited,parse_duration_requires_suffix}` + AUTO_PRUNE_STALE_THRESHOLD constant unchanged | ✅ existing | ⬜ pending |
| 22-05-V2 | 05 | 2 | REQ-AUD-* | — | `prune` hidden alias surfaces deprecation note + still works | unit | `cargo test -p nono-cli cli::tests::prune_alias_deprecation_note` | ❌ W0 | ⬜ pending |
| 22-05-V3 | 05 | 2 | REQ-AUD-* | — | Phase 21 `AppliedLabelsGuard` lifecycle survives audit emissions; ledger flush completes before guard Drop | regression | `cargo test -p nono --test sandbox_windows applied_labels_guard::audit_flush_before_drop` | ✅ existing | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Tests below must be added by their respective plan executors (file paths reflect natural homes per D-16):

**Plan 22-01:**
- [ ] `crates/nono-cli/src/profile/mod.rs` (test module `profile::tests`) — PROF-01..03 deserialize stubs
- [ ] `crates/nono-cli/src/profile/builtin.rs` (test module `profile::builtin::tests`) — PROF-04 builtin load stub

**Plan 22-02:**
- [ ] `crates/nono-cli/src/policy.rs` (test module `policy::tests`) — POLY-01 + POLY-03 stubs
- [ ] `crates/nono-cli/src/cli.rs` (test module `cli::tests`) — POLY-02 conflict stub
- [ ] `crates/nono-cli/tests/rollback*.rs` — update existing rollback integration tests so they no longer pair `--rollback` + `--no-audit`

**Plan 22-03:**
- [ ] `crates/nono-cli/src/package_cmd.rs` (test module `package_cmd::tests`) — PKG-01..04 stubs
- [ ] `crates/nono-cli/tests/package_integration.rs` (new) — long-path + path-traversal + signed-artifact integration tests
- [ ] Test fixture port from upstream `8b46573d` + `71d82cd0` + `9ebad89a` + `ec49a7af` — registry mock + signed-artifact fixture (inline tests; no external fixture files per research finding #6)

**Plan 22-04:**
- [ ] `crates/nono-proxy/src/oauth2.rs` (test module `oauth2::tests`) — OAUTH-01 stubs (port 11 inline tests from upstream `9546c879`)
- [ ] `crates/nono-proxy/src/reverse.rs` (test module `reverse::tests`) — OAUTH-02 stubs

**Plan 22-05:**
- [ ] `crates/nono-cli/src/audit/mod.rs` (test module `audit::tests`) — AUD-01 hash-chain stubs
- [ ] `crates/nono-cli/tests/audit_attestation.rs` (new — port from upstream `9db06336`, +188 LOC) — AUD-02 attestation sign/verify
- [ ] `crates/nono-cli/tests/exec_identity_windows.rs` (new, `#[cfg(target_os = "windows")]`) — AUD-03 Authenticode + SHA-256 fallback (fork-only addition per research finding #2)

**Cargo.toml updates (Wave 0):**
- [ ] `crates/nono-cli/Cargo.toml` — add `windows-sys` feature `Win32_Security_WinTrust` (research finding #8)

*If none: covered above per plan.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Phase 15 5-row detached-console smoke | D-18 gate | Requires running `nono run -- <cmd>` against a real interactive console; ConPTY behavior cannot be cleanly asserted in `cargo test` | `nono run --profile claude-code -- claude-code --version` then `nono ps` then `nono stop <session-id>` — verify all 5 rows appear in `ps` output and `stop` succeeds |
| `wfp_port_integration` admin path | D-18 gate | Requires admin elevation + `nono-wfp-service` running; tests are `#[ignore]` by default | `cargo test -p nono-cli --test wfp_port_integration -- --ignored` (run as Administrator with service installed) |
| `learn_windows_integration` ETW path | D-18 gate | Requires admin elevation + ETW provider availability | `cargo test -p nono-cli --test learn_windows_integration -- --ignored` (run as Administrator) |
| Real OAuth2 token endpoint round-trip | REQ-OAUTH-01 acceptance | No public OAuth2 sandbox; production path proves end-to-end behavior | Set `keyring://nono/oauth2-test` to a valid client_secret for a known token endpoint; run `nono run --profile <oauth2-test> -- curl <api>` and verify Bearer header on the request |
| Authenticode signature query against signed binary | REQ-AUD-03 acceptance #2 | Requires a code-signed `.exe` on disk to query | Sign a test binary with a self-signed cert via `signtool.exe`; run `nono run --audit-integrity -- <signed-binary>`; verify ledger event records the signer subject |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 600s (full ci)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
