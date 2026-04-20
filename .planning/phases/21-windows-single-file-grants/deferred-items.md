# Phase 21 Deferred Items

Out-of-scope issues discovered during execution that are NOT introduced by Phase 21 work.

## Pre-existing `trust::bundle::tests` TUF signature threshold failures

**Discovered during:** Plan 21-02 Task 2 GREEN-phase verification (`cargo test -p nono --lib`)

**Symptom:** Two tests fail with TUF signature threshold errors:

- `trust::bundle::tests::load_production_trusted_root_succeeds` — panics with `assertion failed: root.is_ok()`
- `trust::bundle::tests::verify_bundle_with_invalid_digest` — panics with `called Result::unwrap() on an Err value: TrustPolicy("failed to load production trusted root: TUF error: TUF repository load failed: Failed to verify trusted root metadata: Signature threshold of 3 not met for role root (0 valid signatures)")`

**Verified baseline:** Reproduced these failures on `853683a^` (the commit before Plan 21-02's RED-phase test commit) by checking out the pre-plan `windows.rs` and running `cargo test -p nono --lib trust::bundle::tests::load_production_trusted_root_succeeds`. Same assertion failure reproduces — confirmed NOT introduced by Plan 21-02.

**Likely root cause:** The embedded TUF trusted root artifact is stale or lacks the required signature quorum. This is unrelated to Windows filesystem grants (WSFG-01/02); it's in the `crates/nono/src/trust/bundle.rs` Sigstore verification code path.

**Disposition:** Deferred — Plan 21-02 is scoped to Windows label enforcement primitives, not trust-bundle TUF verification. A separate quick-task should update the embedded trusted-root artifact or regenerate test fixtures.
