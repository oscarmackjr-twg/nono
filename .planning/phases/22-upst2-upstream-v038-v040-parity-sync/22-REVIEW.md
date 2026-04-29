---
phase: 22-upst2-upstream-v038-v040-parity-sync
review_date: 2026-04-28
depth: standard
files_reviewed_count: 51
findings_total: 18
findings_by_severity:
  critical: 0
  high: 1
  medium: 6
  low: 7
  info: 4
verdict: NEEDS_FIXES
files_reviewed_list:
  - crates/nono-cli/src/audit_integrity.rs
  - crates/nono-cli/src/audit_session.rs
  - crates/nono-cli/src/audit_attestation.rs
  - crates/nono-cli/src/exec_identity.rs
  - crates/nono-cli/src/exec_identity_windows.rs
  - crates/nono-cli/src/package.rs
  - crates/nono-cli/src/package_cmd.rs
  - crates/nono-cli/src/registry_client.rs
  - crates/nono-proxy/src/oauth2.rs
  - crates/nono-cli/tests/exec_identity_windows.rs
  - crates/nono-cli/tests/prune_alias_deprecation.rs
  - crates/nono-cli/tests/audit_attestation.rs
  - crates/nono-cli/tests/rollback_audit_conflict.rs
  - crates/nono-cli/tests/manifest_roundtrip.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/profile/builtin.rs
  - crates/nono-cli/src/policy.rs
  - crates/nono-cli/src/policy_cmd.rs
  - crates/nono-cli/src/network_policy.rs
  - crates/nono-cli/src/sandbox_prepare.rs
  - crates/nono-cli/src/audit_commands.rs
  - crates/nono-cli/src/rollback_runtime.rs
  - crates/nono-cli/src/supervised_runtime.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/rollback_commands.rs
  - crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/mod.rs
  - crates/nono-cli/src/hooks.rs
  - crates/nono-cli/src/app_runtime.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/cli_bootstrap.rs
  - crates/nono-cli/src/launch_runtime.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/profile_runtime.rs
  - crates/nono-cli/src/capability_ext.rs
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/data/policy.json
  - crates/nono-cli/data/nono-profile.schema.json
  - crates/nono-cli/README.md
  - crates/nono-cli/tests/policy_cmd.rs
  - crates/nono-proxy/src/config.rs
  - crates/nono-proxy/src/credential.rs
  - crates/nono-proxy/src/error.rs
  - crates/nono-proxy/src/lib.rs
  - crates/nono-proxy/src/route.rs
  - crates/nono-proxy/src/server.rs
  - crates/nono/src/error.rs
  - crates/nono/src/undo/snapshot.rs
  - crates/nono/src/undo/types.rs
  - bindings/c/src/lib.rs
---

# Phase 22: Code Review Report (UPST2 — Upstream v0.38–v0.40 Parity Sync)

**Reviewed:** 2026-04-28
**Depth:** standard
**Files Reviewed:** 51
**Status:** issues_found
**Verdict:** NEEDS_FIXES (1 high requires remediation; medium/low items routed to backlog)

## Summary

Phase 22 ports ~9k LOC of upstream functionality across six plans (PROF, POLY, PKG, OAUTH, AUD-CORE, AUD-RENAME). The implementation is conservative, well-commented, and mostly clean — every `unsafe` block in `exec_identity_windows.rs` has a `// SAFETY:` comment, every `.unwrap()/.expect()` lives inside `#[cfg(test)]` modules, all canonicalize-then-component-compare patterns use `Path::starts_with` on canonicalized `PathBuf`s (not string ops), and `Zeroizing<String>` covers every secret in `nono-proxy/src/oauth2.rs`. The audit-cluster manual-port preserved the fork's v2.1 invariants (`AppliedLabelsGuard` lifecycle, `loaded_profile` threading) per the empty-structural-diff sentinels in 22-05a SUMMARY.

The single **high-severity** finding is a documentation/implementation mismatch in `audit_attestation.rs::verify_audit_attestation`: the function is documented as the answer to AUD-02 acceptance #2 ("nono audit verify rejects tampered ledgers fail-closed") but performs only a structural shape check (file exists, public_key hex-decodable) — it never cryptographically verifies the DSSE signature against the synthetic subjects. The deferral is documented in the source ("Plan 22-05a Decision 5 minimal scope … deferred to Plan 22-05b"), but Plan 22-05b shipped without re-enabling it (the deferred work was reframed under Decision 4 fallback). A user running `nono audit verify <id>` today sees `Attestation: yes` even on a forged bundle whose signature does not cover the recorded chain head. This finding is high (not critical) because the hash-chain + Merkle-root verification IS implemented and IS fail-closed (verified by `verify_audit_log_rejects_tampered_event_log_fail_closed`), so tampering with the event ledger is still detected; only the signature-binding-to-the-summary leg is missing.

The medium-severity findings cluster around three themes: (1) `OAuth2Config` derives `Debug` automatically and stores `client_secret` as plain `String`, so a user-authored profile with a literal secret leaks it to `tracing::debug!` callsites at the profile-load layer (the proxy-side `OAuth2ExchangeConfig` correctly redacts and zeroizes); (2) several CLI flag conflicts are over-restrictive (`--no-audit-integrity` conflicts with `--rollback` even though the combination is semantically reasonable); (3) `package_cmd.rs::remove_all_profile_symlinks_for_package` does symlink-target comparison against a non-canonicalized `resolved` path, which is a low-impact attack surface (nono is the only writer to `profiles_dir`) but inconsistent with the canonicalize-everywhere posture elsewhere in the file.

The low/info findings are mostly comment-quality and cross-platform UX gaps (silent ignoring of `unsafe_macos_seatbelt_rules` on Windows/Linux with no warning, comment in `execution_runtime.rs` calling SHA-256-hash failure "non-fatal" while the code propagates fatally with `?`).

**Verification of audit prompt focus areas:**

- **FFI safety (`exec_identity_windows.rs`):** PASS — 3 `unsafe` blocks each paired with `// SAFETY:` doc comment; RAII close-guard correctly fires on all early-return paths; no `.unwrap()/.expect()` in production paths (3 hits all in test module); placeholder `<implementation per RESEARCH Pattern 4>` returns 0 hits.
- **Memory safety + zeroization (`oauth2.rs`):** PASS for the proxy-side — `Zeroizing<String>` wraps `client_id`, `client_secret`, request body, response token; custom `Debug` redacts; no `write_to_disk`/`serialize_to_path` (0 hits). FAIL at the profile-load layer — `OAuth2Config` in `nono-proxy/src/config.rs` derives `Debug` and uses plain `String` (Finding HG-01-M).
- **Path security (`package_cmd.rs`):** PASS for the staging-root write path (`validate_path_within` after every artifact-write arm uses canonicalize + `Path::starts_with`). One uncanonicalized comparison in `remove_all_profile_symlinks_for_package` (Finding PT-01-M).
- **Cross-platform safety (`policy.rs`):** PASS — `apply_deny_overrides` warn-and-continue when no deny is in effect on this platform; no panic paths; `apply_unlink_overrides` early-returns on Linux (Windows path stores rules harmlessly but never applies them — Finding XP-01-L).
- **Audit lifecycle (`audit_integrity.rs`, `rollback_runtime.rs`):** PASS — `AuditRecorder.finalize()` is called before `AppliedLabelsGuard::drop()` per the structural test at `labels_guard.rs:512` `audit_flush_before_drop` (80 LOC, T-22-05-05 mitigation).
- **Coding standards:** PASS overall — 0 unwrap/expect in production paths across all reviewed files; all errors propagate via `?`; one missing-rustdoc cluster on `package.rs` pub items (Finding RD-01-L).

---

## Findings

### High

#### HG-01-H: `verify_audit_attestation` does not cryptographically verify the bundle signature

**File:** `crates/nono-cli/src/audit_attestation.rs:190-221`
**Severity rationale:** The function is the implementation of `nono audit verify`'s attestation arm and is invoked from `audit_commands.rs::cmd_verify` (line 537). The doc comment at line 186-189 acknowledges "Cryptographic re-verification of the DSSE signature over the synthetic subjects is deferred to Plan 22-05b" — but Plan 22-05b shipped without landing it (per 22-05b SUMMARY scope). The current implementation returns `Ok(true)` whenever the bundle file exists, has non-empty content, and the recorded `summary.public_key` is hex-decodable. An attacker with write access to a session directory can tamper with the bundle bytes (e.g., re-sign with a different key whose hex-encoded public key they substitute into `SessionMetadata.audit_attestation.public_key`) and `nono audit verify` will report `Attestation: yes`. This contradicts the comment claim that "fail-closed semantics are still preserved: any mismatch returns `Ok(false)`" — there is no signature-vs-summary binding being checked.

The hash-chain + Merkle-root checks via `verify_audit_log` (audit_integrity.rs:269) ARE correctly fail-closed, so modifying the event ledger IS detected. The gap is only on the attestation signature leg. T-22-05a-01 in 22-SECURITY.md is closed correctly (the chain-tamper case IS detected); but the audit accepts T-22-05a-04 ("attestation bundle includes raw env vars") as the only attestation threat — the silent-acceptance-of-forged-bundle case is not in the threat register.

**Issue:** Function returns `Ok(true)` for a forged or substituted attestation bundle. The CLI surface (`Attestation: yes` in `audit_commands.rs:587`) reports a green check that has no cryptographic backing.

**Fix:** One of the following before claiming AUD-02 is fully shipped:

1. Add a real signature verification step. Bundle JSON can be parsed via `serde_json` and the embedded signature checked against the `(audit/session_id, audit/chain_head, audit/merkle_root)` synthetic subjects re-computed from `SessionMetadata.audit_integrity` + `session_id`. Reuses `nono::trust::verify_bundle_with_digest` (already imported in `package_cmd.rs:417`) or a sibling.
2. If signature verification cannot land in this phase, downgrade the CLI surface: change `attestation_status: Some(true)` to `Some(VerificationDeferred)` (or a tri-state) so users see "Attestation: present but not cryptographically verified" instead of a green `yes`. Update the rustdoc on `verify_audit_attestation` to call out the deferral as a behavioral gap not just an implementation note.
3. Add a backlog entry to v2.3 with explicit text: "audit attestation signature verification (deferred from Plan 22-05a/22-05b due to sigstore-crypto API gap)" so the gap is tracked.

**Suggested code (option 2 minimal change):**

```rust
// audit_commands.rs:585-587
if let Some(att_ok) = attestation_status {
    eprintln!(
        "  Attestation:         {} (signature verification deferred to v2.3)",
        if att_ok { "structural-only".yellow() } else { "no".red() },
    );
}
```

### Medium

#### HG-01-M: `OAuth2Config` derives Debug and stores `client_secret` as plain `String` — leaks via tracing

**File:** `crates/nono-proxy/src/config.rs:324-336`
**Issue:** `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] pub struct OAuth2Config { ..., pub client_secret: String, ... }`. The proxy-side `OAuth2ExchangeConfig` in `nono-proxy/src/oauth2.rs:49-66` correctly wraps `client_id`/`client_secret` in `Zeroizing<String>` and provides a custom `Debug` impl that prints `[REDACTED]`. But `OAuth2Config` (the type that flows through `Profile.network.custom_credentials.auth`, `RouteConfig.oauth2`, and any clones) has NEITHER. Any `tracing::debug!("config: {config:?}")` callsite on a `Profile` or `RouteConfig` would print the literal `client_secret`. Even though the documented usage pattern is `keyring://`, `env://`, or `file://` URIs (low leak risk because the URI is a reference, not the secret), nothing structurally prevents a user from putting a literal secret in profile JSON, and the type's API does not communicate the expectation. T-22-04-01 (BLOCKING) is satisfied at the proxy boundary but not at the profile-load boundary.

**Fix:** Provide a custom `Debug` impl on `OAuth2Config` that mirrors `OAuth2ExchangeConfig::fmt` (print `token_url`, `client_id`, `[REDACTED]` for `client_secret`, `scope`). Optionally wrap `client_secret` in `Zeroizing<String>` (this requires custom Serde impls because `Zeroizing<T>` doesn't auto-derive `Deserialize`; alternatively, a `secrecy::Secret<String>` newtype with serde feature). At minimum, the custom Debug impl is a one-line copy from `oauth2.rs:57-66`.

```rust
// nono-proxy/src/config.rs (after the derive)
impl std::fmt::Debug for OAuth2Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuth2Config")
            .field("token_url", &self.token_url)
            .field("client_id", &"[REDACTED]")
            .field("client_secret", &"[REDACTED]")
            .field("scope", &self.scope)
            .finish()
    }
}
```

(remove `Debug` from the auto-derive list when adding the manual impl)

#### PT-01-M: `remove_all_profile_symlinks_for_package` compares uncanonicalized symlink target to install_dir

**File:** `crates/nono-cli/src/package_cmd.rs:244-268`
**Issue:** Line 254 reads `fs::read_link(&path)` to get the symlink target. If the target is relative, line 259 joins it with `profiles_dir` but does NOT canonicalize. Line 261 then runs `resolved.starts_with(install_dir)` on the un-canonicalized path. A symlink target containing `..` segments could pass this check while the actual filesystem walk would land outside `install_dir`. Concrete example: if `profiles_dir = /home/u/.config/nono/profiles`, `install_dir = /home/u/.config/nono/packages/acme/foo`, and a symlink target is `../packages/acme/foo/../../../../sensitive-dir`, then `resolved = profiles_dir.join(target) = /home/u/.config/nono/profiles/../packages/acme/foo/../../../../sensitive-dir`, and `Path::starts_with` on the lexical form may return `true` because it's prefix-matching components literally including `..`. This is the exact "string-comparison-not-component-comparison" footgun listed in CLAUDE.md § Common Footguns #1 — except here it's `Path::starts_with` on a non-canonicalized PathBuf, which is a subtler variant of the same bug.

Attack feasibility is low because `profiles_dir` is owned by nono and only nono writes symlinks into it via `create_profile_symlinks` (line 727), which always uses an absolute target. But the function is invoked during `nono remove`, and removing the wrong file is a destructive operation. Defense-in-depth here is cheap.

**Fix:** Canonicalize before comparison, mirroring `validate_path_within`:

```rust
let resolved = match fs::canonicalize(&entry.path()) {
    Ok(c) => c,
    Err(_) => continue, // broken symlink — skip
};
let canonical_install = match fs::canonicalize(install_dir) {
    Ok(c) => c,
    Err(_) => return Ok(()), // install_dir already gone — nothing to clean
};
if resolved.starts_with(&canonical_install) {
    let _ = fs::remove_file(&path);
}
```

#### CL-01-M: `--no-audit-integrity` conflicts with `--rollback` overrestrictively

**File:** `crates/nono-cli/src/cli.rs:1845`
**Issue:** `#[arg(long, conflicts_with_all = ["audit_integrity", "rollback"], ...)] pub no_audit_integrity: bool` — pairing `--rollback` with `--no-audit-integrity` is rejected at parse time. But `--rollback` semantically means "snapshot filesystem state for restore"; `--audit-integrity` is the orthogonal Merkleized append-only ledger. A user who wants rollback but does not want the audit-integrity ledger overhead has no way to express it. POLY-02 acceptance only requires `--rollback` to conflict with `--no-audit` (the entire audit trail), not with `--no-audit-integrity` (just the cryptographic ledger). 22-SECURITY.md T-22-02-02 mitigation cites only the `--no-audit` conflict.

This is portrayed as upstream-faithful porting of `5c301e8d`, but it bundles two distinct conflicts. The behavior may match upstream verbatim; if so this is a porting decision not a regression. Worth a discussion-on-merge note.

**Fix:** Remove `"rollback"` from `no_audit_integrity`'s `conflicts_with_all` list. Add a doc-comment note that `--rollback` does not auto-imply `--audit-integrity`. If upstream really requires this conflict, leave a comment explaining why and route to v2.3 backlog as a UX-fork-deviation candidate.

#### CL-02-M: `--audit-sign-key` requires `--audit-integrity` but `--rollback` enables audit without setting `audit_integrity`

**File:** `crates/nono-cli/src/cli.rs:1864-1870`
**Issue:** `audit_sign_key` has `requires = "audit_integrity"`, so `nono run --rollback --audit-sign-key keyring://nono/audit ...` fails clap-required-check even though `--rollback` enables audit (per `create_audit_state` logic in `rollback_runtime.rs:184-194`). The user must spell `--rollback --audit-integrity --audit-sign-key ...`. This works but is not discoverable.

**Fix:** Either (a) drop `requires = "audit_integrity"` from `audit_sign_key` and surface a runtime error in `prepare_audit_signer` if neither rollback-audit nor `--audit-integrity` is active, or (b) update the help text on `audit_sign_key` to spell out the required pairing. (a) is more user-friendly.

#### CL-03-M: `validate_oauth2_auth` accepts plain `client_secret` literals without warning

**File:** `crates/nono-cli/src/profile/mod.rs:737-756`
**Issue:** The function rejects empty `client_secret` but accepts any non-empty string, including a plain literal secret. Per the doc claim "Plain values, `keyring://`, `env://`, `file://`, and `op://` URIs are all accepted at this layer," the intent is to allow plain values — but a profile that ships with a literal secret in JSON is a leak waiting to happen (file shared via git, copy-pasted to a chat, etc.). At least a `tracing::warn!` if the value doesn't look like a `*://` URI would mitigate accidental literal-secret commits.

**Fix:** Add a soft warning (not a hard error) when `client_secret` is not recognized as a known URI scheme:

```rust
if !auth.client_secret.contains("://") {
    tracing::warn!(
        "custom credential '{}' has a literal client_secret value (no URI scheme). \
         Prefer keyring://, env://, file://, or op:// URIs to avoid committing secrets to disk.",
        name
    );
}
```

Or hard-fail in non-test contexts (more aggressive but defensible per CLAUDE.md "Fail Secure").

#### CL-04-M: `OAuth2Config.client_secret` written to manifest as sentinel `oauth2://` loses round-trip fidelity

**File:** `crates/nono-cli/src/policy_cmd.rs:2207-2212`
**Issue:** When a profile uses `auth: OAuth2Config { ... }` (no `credential_key`), `resolve_to_manifest` substitutes `oauth2://` as the `source` field for the manifest credential entry. The comment acknowledges this is a sentinel: "Upstream `19a0731f` used `continue` to skip the entry; fork retains the sentinel-source path for manifest visibility." But the manifest schema does not yet have an OAuth2 representation, so a downstream `nono run --config <manifest>` that reads the manifest back would either (a) fail to parse `oauth2://` as a valid source URI (no `is_*_uri` returns true for it), or (b) silently treat it as a literal secret value. Neither is correct. The current dispatch in `nono-proxy` does not handle `oauth2://` as a known scheme.

This is a UX hole flagged by upstream's choice (skip entry) being safer than the fork's choice (sentinel). The user-visible breakage manifests only when a profile-with-auth is exported via `policy show --format manifest` and re-fed via `--config`, which is the exact round-trip `manifest_roundtrip.rs` claims to verify.

**Fix:** Either:
1. Match upstream's `continue` and emit a separate manifest stanza or warning for OAuth2 credentials. Acceptable if the manifest format is documented as "static-credentials only at this stage."
2. Extend the manifest schema to represent OAuth2 (sibling field on `manifest::Credential`). Larger change.

Recommended: option 1 + a `tracing::warn!("OAuth2 credential '{name}' cannot be represented in the manifest format and is skipped")`. Update the existing comment at policy_cmd.rs:2161-2163 to call this out behaviorally.

#### MN-01-M: `TokenCache::new` calls `Handle::current().block_on(...)` — panics if called from inside a tokio runtime

**File:** `crates/nono-proxy/src/oauth2.rs:108-126`
**Issue:** `TokenCache::new` synchronously blocks on `exchange_token` via `tokio::runtime::Handle::current().block_on(...)`. The doc comment says "Called during `CredentialStore::load()` which is synchronous. We bridge into async via `tokio::runtime::Handle::current().block_on()`." But `Handle::current()` panics if called outside a tokio runtime, AND `block_on` panics if called from within a tokio runtime's worker thread (single-threaded scheduler) or returns a runtime error on multi-threaded scheduler depending on tokio version. If `CredentialStore::load()` is ever invoked from inside an async context (e.g., a future that awaits proxy startup), this will panic in production.

There are no callers yet in the production code path (verified via `grep TokenCache crates/nono-proxy/src/server.rs` — 0 hits; the type is defined but not yet instantiated by the proxy server). So the bug is latent. When Plan 22-04+ wires it up, a future engineer needs to ensure the call site is actually synchronous (not `tokio::spawn`'d).

**Fix:** Either (a) make `TokenCache::new` async and have callers `.await`, or (b) document the panic precondition more aggressively at the function signature level (`# Panics` rustdoc section), or (c) require a runtime `Handle` parameter so the caller's intent is explicit:

```rust
pub fn new(
    config: OAuth2ExchangeConfig,
    tls_connector: TlsConnector,
    runtime_handle: tokio::runtime::Handle,
) -> Result<Self> {
    let (access_token, expires_in) =
        runtime_handle.block_on(exchange_token(&config, &tls_connector))?;
    ...
}
```

This forces the caller to say "I have a runtime handle and I'm calling from outside its async context."

### Low

#### XP-01-L: `apply_unlink_overrides` does not early-return on Windows; rules stored but never applied

**File:** `crates/nono-cli/src/policy.rs:946-989`
**Issue:** The function has `if cfg!(target_os = "linux") { return; }` but no equivalent guard for Windows. On Windows, `caps.add_platform_rule(rule)` validates and stores the Seatbelt S-expression in `platform_rules`, but Windows sandbox apply (in `crates/nono/src/sandbox/windows.rs`) ignores `platform_rules`. The rules are stored as dead data. Wasted CPU + memory; minor confusion in `policy show` output.

**Fix:** Change the early-return guard to non-macOS:

```rust
if !cfg!(target_os = "macos") {
    return; // Unlink overrides are Seatbelt-specific (macOS only)
}
```

#### XP-02-L: `unsafe_macos_seatbelt_rules` silently ignored on Linux/Windows with no warning

**File:** `crates/nono-cli/src/sandbox_prepare.rs:344-364`
**Issue:** `#[cfg(target_os = "macos")]` gates the entire block. On Linux/Windows the rules deserialize cleanly, but no warning is emitted to the user. A user may share a profile with `unsafe_macos_seatbelt_rules: ["(allow iokit-open)"]` expecting it to take effect; on Windows nono silently does nothing. The schema description says "Ignored on Linux and Windows" — but the runtime doesn't echo this. Users may not read the schema.

**Fix:** Add a one-line `tracing::warn!` outside the cfg-gated block when the field is non-empty:

```rust
if let Some(ref profile) = loaded_profile {
    if !profile.unsafe_macos_seatbelt_rules.is_empty() && !cfg!(target_os = "macos") {
        tracing::warn!(
            "Profile declares {} unsafe_macos_seatbelt_rules but this is not macOS — ignoring.",
            profile.unsafe_macos_seatbelt_rules.len()
        );
    }
}
#[cfg(target_os = "macos")]
if let Some(ref profile) = loaded_profile {
    /* existing apply block */
}
```

#### CM-01-L: `execution_runtime.rs` comment claims hash failure is "non-fatal" while the code uses `?`

**File:** `crates/nono-cli/src/execution_runtime.rs:189-201`
**Issue:** The comment "Hash failure is non-fatal for the launch path" contradicts the very next sentence "we propagate that so the user sees a concrete diagnostic rather than running with no audit identity." The code uses `?` — failure IS fatal. Mis-leading comment. Could be confusing during incident triage.

**Fix:** Replace "non-fatal" with "fatal-by-design":

```rust
// AUD-03 SHA-256 portion (upstream 02ee0bd1): capture canonical path +
// SHA-256 of the executable BEFORE sandbox apply so the audit trail
// commits to exactly the bytes the supervisor handed off. Only computed
// for Supervised strategy (Direct/Monitor record nothing).
//
// Hash failure IS fatal (propagated via `?`): we prefer a concrete
// diagnostic over running with no audit identity. Direct/Monitor
// strategies skip the hash entirely and are unaffected.
```

#### RD-01-L: Missing rustdocs on pub items in `package.rs` and `package_cmd.rs`

**Files:**
- `crates/nono-cli/src/package.rs:13-25, 27-94, 96-160, 163-192` (pub structs `PackageRef`, `PackageManifest`, `PackType`, `ArtifactEntry`, `ArtifactType`, `Lockfile`, `LockedPackage`, `PackageProvenance`, `LockedArtifact`, `PackageSearchResult`, `PackageSearchResponse`, `PullResponse`, `PullProvenance`, `PullArtifact` and many of their fields)
- `crates/nono-cli/src/package_cmd.rs:17, 63, 276, 290, 322` (pub fns `run_pull`, `run_remove`, `run_update`, `run_search`, `run_list`)

**Issue:** Per CLAUDE.md § Comments: "Public API documentation is mandatory." Most of these types are bin-crate-internal (`pub` only within the binary, not exposed via `lib.rs`), but the project standard applies uniformly. Several types DO have rustdocs (e.g., `parse_package_ref`); the gap is uneven coverage rather than universal absence.

**Fix:** Add one-line rustdocs to each pub item. Field-level rustdocs are already present on most `LockedArtifact` / `PullProvenance` fields; backfill the missing struct-level docs and the entry-point fn docs.

#### TS-01-L: `audit_attestation.rs` test `verify_returns_false_when_bundle_missing` does not exercise tamper detection

**File:** `crates/nono-cli/src/audit_attestation.rs:268-279`
**Issue:** The single non-trivial test for `verify_audit_attestation` only covers the missing-bundle case (returns false). There is no test for: (a) bundle exists but bytes are tampered, (b) bundle exists with mismatched key_id, (c) bundle exists with mismatched public_key. Combined with HG-01-H (the function doesn't actually verify signatures), this means there is no test that documents the expected fail-closed behavior of attestation verification beyond "file missing." A future contributor extending the function has nothing to break to know they've broken something.

**Fix:** Add at least the "bundle exists but malformed JSON" / "non-empty file but bytes corrupt" cases. Once HG-01-H is fixed, add proper signature-mismatch fixtures.

#### TS-02-L: 2 `#[ignore]`'d tests in `audit_attestation.rs` integration suite lack a v2.3 backlog row reference

**File:** `crates/nono-cli/tests/audit_attestation.rs:111-157, 162-211`
**Issue:** Both ignored tests cite "Plan 22-05a deferred to 22-05b: requires from_pkcs8 KeyPair support + sign_statement_bundle (audit_ledger.rs)" in the `#[ignore = "..."]` reason. But Plan 22-05b shipped without unlocking these fixtures (per 22-05b SUMMARY scope). The `#[ignore]` reason is now stale — pointing at 22-05b which is closed. Future readers will be confused about whether these tests should be active.

**Fix:** Update both `#[ignore]` reasons to point at a v2.3 backlog row name (the 22-05b SUMMARY referred to "Audit-attestation D-13 fixtures re-enablement (deferred from Plan 22-05b)"). Also add a top-of-file comment block linking to the backlog so the blast radius is searchable.

#### TS-03-L: `manifest_roundtrip.rs::manifest_includes_workdir_grant` has a soft assertion (early-return on failure)

**File:** `crates/nono-cli/tests/manifest_roundtrip.rs:218-273`
**Issue:** Lines 248-272 wrap the assertion in `if output.status.success() { ... }`. If `nono policy show --workdir` is broken, the test passes silently. Comment at line 271-272 says "After the fix, this should succeed." But the bug remains untracked and the test is functionally a no-op until the fix lands. This is the inverse of the unit-test contract — a test should fail when the system is broken, not pass-when-the-test-can't-run.

**Fix:** Either (a) hard-fail with `assert!(output.status.success(), "...")` to trigger an immediate test failure if the path regresses, or (b) `#[ignore]` the test with a deferral-reason like the audit_attestation tests, or (c) split the test in two: one that asserts the flag doesn't error, one that asserts the workdir appears (gated on success). Current shape is the worst of all options.

### Info

#### IN-01-I: `audit_session.rs::path_filter.starts_with(p)` reverse-prefix semantics may surprise users

**File:** `crates/nono-cli/src/audit_commands.rs:244`
**Issue:** `s.metadata.tracked_paths.iter().any(|p| p.starts_with(path_filter) || path_filter.starts_with(p))` — this is `Path::starts_with` which does component comparison (good), but the semantics of "either path is a prefix of the other" is unusual. For `path_filter = /home/user/widgets/sub` and `tracked_paths = [/home/user/widgets]`, it matches because the tracked path is a prefix of the filter. For `path_filter = /home` and `tracked_paths = [/home/user/widgets]`, it matches because the filter is a prefix of the tracked path. Both are intentional ("show me sessions that touched anywhere under or above this path") but documented nowhere. Worth a one-line rustdoc.

**Fix:** Add a `///` line on `filter_sessions` explaining the bi-directional `--path` matching semantics.

#### IN-02-I: Dead-code `#[allow(dead_code)]` on `MERKLE_SCHEME_LABEL` is consumed in tests but const description claims runtime use

**File:** `crates/nono-cli/src/audit_integrity.rs:25-26`
**Issue:** `#[allow(dead_code)] // consumed by audit verify in Task 6 pub(crate) const MERKLE_SCHEME_LABEL: &str = "alpha";` — the rustdoc says "Schema label persisted in `AuditIntegritySummary.hash_algorithm` / downstream verification fixtures." But `AuditIntegritySummary.hash_algorithm` is set to `HASH_ALGORITHM = "sha256"` (line 22), not `MERKLE_SCHEME_LABEL`. The `merkle_scheme` field on `AuditVerificationResult` IS set from `MERKLE_SCHEME_LABEL` (line 348). Doc comment is misleading — the constant is a label for the Merkle scheme, not the hash algorithm. Per CLAUDE.md "Lazy use of dead code: Avoid `#[allow(dead_code)]`. If code is unused, either remove it or write tests that use it." The `verify_audit_log_accepts_untampered_session` test at line 413 does use it (`assert_eq!(result.merkle_scheme, "alpha")`) so the `#[allow(dead_code)]` is defensive against future-only-test usage. Could be removed.

**Fix:** Update doc comment to "Schema label persisted in `AuditVerificationResult.merkle_scheme` / downstream verification fixtures." Optionally remove the `#[allow(dead_code)]` if cargo accepts it (test usage should keep the symbol live in the test profile, and production usage is line 348).

#### IN-03-I: `#[allow(dead_code)]` clusters in `audit_session.rs` and `audit_integrity.rs` for "follow-up cherry-pick" unused fns

**Files:** `crates/nono-cli/src/audit_session.rs:19, 69, 101, 159, 201, 212, 227, 233`; `crates/nono-cli/src/audit_integrity.rs:30, 144, 149, 163`
**Issue:** Several pub/pub(crate) items are tagged `#[allow(dead_code)]` with comments like "consumed in audit_commands.rs by follow-up cherry-picks (4ec61c29..9db06336) per Plan 22-05a Decision 5." Per CLAUDE.md these allows should be removed once the follow-up commits land — and according to the SUMMARYs, they DID land (commits `a16704e8`, `ee502107`, `3544d600`, `2ab53fec`, `a8fbb65e`). Many of these symbols are now actually used at the dispatch sites (`run_audit`, `cmd_show`). The `#[allow(dead_code)]` may be stale and Clippy's `unused` lint should cover the still-unused ones cleanly.

**Fix:** Audit each `#[allow(dead_code)]` and remove the ones that are no longer needed. For genuinely-unused items (e.g., `record_capability_decision`, `record_open_url`, `record_network_event` in audit_integrity.rs:144-166 — these are the upstream-API-shape preservation), consolidate into a single module-level `#[allow(dead_code)]` block with a single comment explaining the API-completeness rationale.

#### IN-04-I: README diff covers many doc updates (not reviewed in detail)

**File:** `crates/nono-cli/README.md`
**Issue:** The diff includes a non-trivial README update for the new `nono pull/remove/update/search/list` commands and `nono session cleanup`. Not a code finding per se, but the README is the user's first contact surface. A separate read-through pass focused on doc consistency (every flag name matches the `cli.rs` definition; every example still parses) is worth scheduling as a backlog item — the prompt explicitly excluded this from review scope.

**Fix:** Schedule a doc-review pass for v2.3 milestone open or include as a quick task before the next release tag.

---

## Patterns Observed

1. **Strong defensive coding around path security.** Every artifact-write path in `package_cmd.rs::install_manifest_artifact` runs `validate_path_within(staging_root, &store_path)` after writing, using canonicalize + `Path::starts_with` (the correct pattern per CLAUDE.md). The defense-in-depth comment at line 665-670 is exemplary. Only one place (`remove_all_profile_symlinks_for_package`) deviates (Finding PT-01-M).

2. **Consistent `Zeroizing<String>` discipline at the proxy layer.** 15 hits of `Zeroizing` in `nono-proxy/src/oauth2.rs` cover client_id, client_secret, request body bytes, response token, and even the Slate request-line. Custom `Debug` redacts. Coverage at the profile-load boundary (`OAuth2Config`) is the ONLY gap (Finding HG-01-M).

3. **Consistent FFI-safety discipline in `exec_identity_windows.rs`.** Three `unsafe` blocks each have a `// SAFETY:` comment explaining the precondition. RAII close-guard mirrors Phase 21's `_sd_guard` pattern verbatim. Heuristic UTF-16 length check pre-empts FFI on bad input. No `.unwrap()` in production paths. Decision 4 fallback is documented in 4 separate places (header rustdoc, function rustdoc, the `parse_signer_subject`/`parse_thumbprint` rustdocs, and the test suite). One of the cleanest FFI surfaces in the fork.

4. **Clear `Upstream-commit:` provenance trailers throughout.** Every D-19 commit carries the trailer; manual-port commits use the D-20 template with explicit replay rationale. 22-SECURITY.md verified D-19 trailers via `git log | grep '^Upstream-commit:'` returning expected counts.

5. **Heavy use of `#[allow(dead_code)]` for upstream-API-shape preservation.** Pattern is "ports the type signature now so the upstream call site lands as a one-line cherry-pick later." Defensible but creates a long-tail of reviewer-attention items (Finding IN-03-I). Consolidate or remove on each phase close.

6. **5 files contain comments that describe behavior contradictory to the code** (Finding CM-01-L is the most notable — `executable_identity` "non-fatal" claim contradicts `?`). These accumulate during manual-port replays where comment text is carried verbatim from upstream while the code is adapted. A grep-scan for "non-fatal," "best-effort," and "graceful" in production paths would catch a few more.

7. **Several CLI flag conflicts may be over-restrictive** (Findings CL-01-M, CL-02-M). `--no-audit-integrity` ⊥ `--rollback` and `--audit-sign-key` requiring `--audit-integrity` instead of "audit active" both surfaced. Worth a CLI design review pass.

8. **Test files for new functionality are well-structured but with deferral debt.** 2 `#[ignore]`'d tests in `audit_attestation.rs` integration suite + 1 silent-pass test in `manifest_roundtrip.rs` (Finding TS-03-L). Cumulative effect: 3 tests that don't actually exercise their stated invariants. Cleanup is small but should not be ongoing.

---

## Carry-Over Notes

These are pre-existing issues unchanged by Phase 22. NOT TO FIX in Phase 22; route to backlog if desired.

1. **`Profile` struct now exceeds ~50 fields.** Adding `unsafe_macos_seatbelt_rules`, `packs`, and `command_args` puts the struct deep into "complex deserialize, opaque mutation" territory. Future field additions risk being forgotten in `merge_profiles` (which uses field-by-field manual merging). Backlog candidate: a derive-based merge strategy or a `Profile` → `ProfileFields` flattening.

2. **`policy.rs::ProfileDef` and `profile/mod.rs::ProfileDeserialize` duplicate ~40 fields.** New PROF-01/02/03 fields had to be added to BOTH (and their `From` impls). Same risk as #1 above. Backlog candidate: a single flat `ProfileFields` struct that both consume.

3. **`network_policy.rs` test boilerplate is significant.** Every new field on `CustomCredentialDef` requires updating ~20 test fixture builders manually (visible in this diff: `auth: None,` added 20 times). Backlog candidate: a `CustomCredentialDef::default()` impl or a builder pattern test helper.

4. **`SessionMetadata` now has 12 fields and growing.** Phase 22 adds 4 more (`executable_identity`, `audit_event_count`, `audit_integrity`, `audit_attestation`). Same migration risk as `Profile`. Backlog candidate: split into `SessionCore` + `SessionAudit` + `SessionRollback` sub-structs.

5. **`AuditEventPayload::CapabilityDecision`, `UrlOpen`, `Network` variants are unused** (`#[allow(dead_code)]` at audit_integrity.rs:30). Per the rustdoc they "land in follow-up cherry-picks 4ec61c29..9db06336 per Plan 22-05a Decision 5" — and those commits DID land. The variants ARE referenced from the public methods (`record_capability_decision`, `record_open_url`, `record_network_event`) which are themselves `#[allow(dead_code)]`. The dispatch sites for these methods (the supervisor IPC paths) have not been wired up — that's deferred to Phase 23 AUD-05. Carry-over status: legitimate, tracked in 22-CONTEXT § Out of scope.

6. **`bindings/c/src/lib.rs:117-124` adds `PackageVerification` to `ErrTrustVerification` and `PackageInstall`/`RegistryError` to `ErrConfigParse`.** This is correct per the existing error-mapping pattern, but the C FFI doc header (`bindings/c/include/nono.h` generated by cbindgen) needs to surface the new enum-mapping rationale somewhere; right now the FFI consumer learns only the C error code, not which Rust variants map to it. Carry-over: the FFI surface is documented in `bindings/c/README.md` which was not in scope for Phase 22.

7. **`registry_client.rs::resolve_url` uses string `starts_with` for HTTP/HTTPS scheme detection.** This is correctly flagged in-line as "not a path comparison" with the CLAUDE.md exception cited. It's safe in this context (the registry's `bundle_url` and `download_url` are signed in the bundle so tampering is detected before URL resolution). Carry-over: existing pattern in the fork.

---

_Reviewed: 2026-04-28_
_Reviewer: Claude (gsd-code-reviewer, Opus 4.7 1M context)_
_Depth: standard_
_Verdict: NEEDS_FIXES (1 high-severity item before Phase 22 ships; 6 medium / 7 low / 4 info routable to v2.3 backlog or remediated inline)_
