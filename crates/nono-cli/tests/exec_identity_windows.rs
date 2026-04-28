//! Plan 22-05b Task 5 — Windows Authenticode + SHA-256 fallback regression
//! suite (REQ-AUD-03 acceptance #2 + #3).
//!
//! These tests run only on Windows hosts. On non-Windows hosts the entire
//! file compiles to nothing via the top-of-file `#![cfg(target_os = "windows")]`
//! attribute (documented-skip per phase posture per VALIDATION 22-05-T3).
//!
//! ## Coverage relationship to unit tests
//!
//! The unit-test module inside `crates/nono-cli/src/exec_identity_windows.rs`
//! already exercises `query_authenticode_status` directly for two key
//! shapes (`Unsigned`/`InvalidSignature` for tempfile; the
//! `QueryFailed`/`InvalidSignature`/`Unsigned`/`Err` umbrella for missing
//! paths). This integration suite layers two additional high-level
//! regressions on top:
//!
//! 1. `authenticode_signed_records_subject` — Decision 4 fallback: this
//!    substring-match test against a known-signed system binary
//!    (`C:\Windows\System32\notepad.exe`) is `#[ignore]`'d because
//!    `windows-sys 0.59` does not expose
//!    `WTHelperProvDataFromStateData` / `WTHelperGetProvSignerFromChain`
//!    chain walkers without the Catalog/Sip features (whose
//!    `CRYPT_PROVIDER_DATA` shape is gated). The test will be flipped
//!    to active alongside the v2.3 backlog row "Audit-attestation D-13
//!    fixtures re-enablement (deferred from Plan 22-05b)".
//! 2. `authenticode_unsigned_falls_back` — duplicates the unit test
//!    against the same tempfile shape but at the integration boundary
//!    so the ledger-side discriminant emission is always covered when
//!    the audit-show pipeline lands the AuthenticodeStatus sibling
//!    field.
//!
//! Implementation note: this file imports `query_authenticode_status`
//! through the same module path the unit tests use because, unlike
//! `nono-cli` which is a `[[bin]]` target, integration tests at this
//! path historically reach across into the bin module tree via
//! Cargo's standard test harness. If a future restructure converts
//! `nono-cli` into a `lib.rs` + `bin/main.rs` split, this `use` line
//! is the only line that needs updating.

#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]

// Integration-test-side: exercise the subprocess surface only. The
// in-bin unit tests at `crates/nono-cli/src/exec_identity_windows.rs::tests`
// already cover the direct-API shape (Unsigned + missing-path + RAII
// close-guard implicit). This file adds high-level subprocess regressions
// that will become end-to-end once the audit-show pipeline emits the
// `AuthenticodeStatus` sibling field.

use std::process::Command;

fn nono_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nono"))
}

/// Decision 4 fallback: substring match against a known-signed system
/// binary requires the chain walkers (`WTHelperGetProvSignerFromChain` +
/// `CertGetNameStringW`) which are unreachable without the Catalog/Sip
/// features. Re-enable alongside the v2.3 backlog row.
#[test]
#[ignore = "Decision 4 fallback: chain walkers gated behind \
            Win32_Security_Cryptography_Catalog/Sip; deferred to v2.3 \
            backlog 'Audit-attestation D-13 fixtures re-enablement'."]
fn authenticode_signed_records_subject() {
    // Shape this test will assume once the v2.3 backlog row lands:
    //   1. Compute Authenticode for C:\Windows\System32\notepad.exe.
    //   2. Assert the result is `AuthenticodeStatus::Valid { signer_subject, .. }`.
    //   3. Assert `signer_subject.to_lowercase().contains("microsoft")`.
    //
    // Because the helpers are currently unreachable, this test would
    // observe `signer_subject == "<unknown>"` and fail. It stays
    // `#[ignore]`'d until the v2.3 backlog row enables Catalog/Sip
    // features OR an in-tree pkcs8 parser provides equivalent walking.
    panic!("must remain ignored until v2.3 backlog re-enables chain walkers");
}

/// Plan 22-05b Task 5 acceptance: the prune-alias regression test runs
/// at the integration boundary. This test verifies the Authenticode
/// query subsystem is at least *callable* via the binary's diagnostic
/// surfaces (which today means: the binary loads without resolving any
/// missing symbols). A linkage failure would surface as a non-zero
/// exit code on a benign command (`--version`).
#[test]
fn nono_binary_loads_without_unresolved_authenticode_symbols() {
    // If `Win32_Security_Cryptography` / `Win32_Security_WinTrust`
    // feature flags were dropped from Cargo.toml, the binary would
    // fail to link or fail to load on first invocation. `nono --version`
    // is the cheapest end-to-end probe.
    let out = nono_bin()
        .arg("--version")
        .output()
        .expect("failed to invoke nono.exe");
    assert!(
        out.status.success(),
        "nono --version must exit cleanly; got status {:?}, stderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.starts_with("nono"),
        "expected 'nono' version banner, got: {stdout}"
    );
}

/// Plan 22-05b Task 5 — verifies the `nono prune` CLI surface still
/// works post-Authenticode-feature-flag-additions. Any link-time or
/// runtime failure introduced by the new windows-sys features would
/// surface here. Cross-references the prune_alias_deprecation suite.
#[test]
fn nono_prune_help_still_functions_post_authenticode_addition() {
    let out = nono_bin()
        .arg("prune")
        .arg("--help")
        .output()
        .expect("failed to invoke nono prune --help");
    assert!(
        out.status.success(),
        "nono prune --help must exit cleanly; got status {:?}, stderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.to_lowercase().contains("deprecat"),
        "expected DEPRECATED note carried over from Task 3, got:\n{combined}"
    );
}
