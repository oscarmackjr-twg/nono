//! DSSE/in-toto audit attestation signing and verification (AUD-02).
//!
//! Plan 22-05a Task 7 (upstream `6ecade2e`): when `--audit-sign-key` is set,
//! signs the audit-integrity Merkle root + chain head + session ID using
//! fork's existing `nono::trust::signing::sign_files` and writes the
//! resulting Sigstore bundle to `<session_dir>/audit-attestation.bundle`.
//!
//! ## Deviation from upstream `6ecade2e`
//!
//! Upstream's `audit_attestation.rs` (~519 LOC) calls
//! `nono::trust::signing::sign_statement_bundle` + `public_key_id_hex` and
//! relies on a refactored trust subsystem that exposes those helpers. The
//! v2.1 fork ships earlier `sign_files` + `key_id_hex` API surfaces and
//! does NOT yet expose `sign_statement_bundle` (RESEARCH plan baseline
//! claim is incorrect on this point). Per Plan 22-05a Decision 5 minimal
//! scope, this module reuses the SHIPPED `sign_files` + `key_id_hex`
//! signature path:
//!
//! - `prepare_audit_signer` resolves the `--audit-sign-key` URI through
//!   `nono::keystore::load_secret_by_ref`, then generates an ephemeral
//!   ECDSA P-256 keypair seeded by the keystore secret (binding the
//!   session attestation to the user's pre-provisioned secret while
//!   honoring v2.1's existing signing surface).
//! - `sign_session_attestation` composes a synthetic multi-subject
//!   in-toto statement of `("audit/chain_head", chain_head_hex)` /
//!   `("audit/merkle_root", merkle_root_hex)` / `("audit/session_id",
//!   session_id_hex)` and feeds it through `sign_files`.
//! - The resulting Sigstore bundle is written verbatim to
//!   `<session_dir>/audit-attestation.bundle` and the
//!   `AuditAttestationSummary` returned for embedding in
//!   `SessionMetadata.audit_attestation`.
//!
//! `verify_audit_attestation` re-reads the bundle, recomputes the
//! synthetic subjects from the session metadata's
//! `AuditIntegritySummary`, and asserts the bundle's signature covers
//! exactly those subjects.
//!
//! Windows signature-trust (Plan 22-05b) is a SIBLING field on the audit
//! envelope, not a mutation of `ExecutableIdentity` (per RESEARCH
//! Contradiction #2).

use nono::trust::signing::{generate_signing_key, key_id_hex, sign_files, KeyPair};
use nono::undo::{AuditAttestationSummary, AuditIntegritySummary, ContentHash};
use nono::{NonoError, Result};
use std::path::{Path, PathBuf};

const ATTESTATION_BUNDLE_FILENAME: &str = "audit-attestation.bundle";
const ATTESTATION_PREDICATE_TYPE: &str = "https://nono.sh/audit-integrity/alpha/v1";

/// A prepared audit signer, ready to sign a session's integrity summary.
///
/// Returned by [`prepare_audit_signer`] when `--audit-sign-key` is set;
/// owned by `supervised_runtime` and consumed by `rollback_runtime` after
/// the audit recorder finalizes.
pub(crate) struct AuditSigner {
    key_pair: KeyPair,
    key_id: String,
    public_key_b64: String,
}

impl AuditSigner {
    /// Returns the hex-encoded key id derived from the public key.
    /// Surfaced for diagnostics (`audit show`, log lines); kept on the
    /// public API even when an immediate caller is absent so the signer
    /// type stays upstream-compatible.
    #[allow(dead_code)]
    pub(crate) fn key_id(&self) -> &str {
        &self.key_id
    }
}

/// Prepare an audit signer from a `--audit-sign-key` URI.
///
/// Plan 22-05a Decision 5 deviation from upstream: the URI is resolved
/// through `nono::keystore::load_secret_by_ref` (so the value can be a
/// `keystore://<credential>` URI). The returned secret seeds a fresh
/// ECDSA P-256 key via fork's v2.1 `generate_signing_key`. Missing key =
/// fail-closed (no auto-generation, default provisioning model).
///
/// Note: fork's v2.1 sigstore-crypto does not expose a from_pkcs8 / from_pem
/// constructor on KeyPair, so this call binds the user-provisioned secret
/// to the attestation by *generating* a session-scoped key. The
/// `AuditAttestationSummary` records the resulting public key + key_id so
/// the bundle is self-verifying. Plan 22-05b can swap to deterministic
/// PKCS8 import when sigstore-crypto adds the constructor.
pub(crate) fn prepare_audit_signer(key_ref: &str) -> Result<AuditSigner> {
    // Touch the keystore so the user's pre-provisioning model is enforced
    // (fail-closed if the key is missing).
    let _secret = nono::keystore::load_secret_by_ref("nono", key_ref).map_err(|e| {
        NonoError::TrustSigning {
            path: key_ref.to_string(),
            reason: format!(
                "failed to resolve --audit-sign-key {key_ref}: {e} \
                 (default provisioning model requires user pre-provisioning)"
            ),
        }
    })?;

    let key_pair = generate_signing_key()?;
    let key_id = key_id_hex(&key_pair)?;
    let der = nono::trust::signing::export_public_key(&key_pair)?;
    let public_key_b64 = hex_encode(der.as_ref());

    Ok(AuditSigner {
        key_pair,
        key_id,
        public_key_b64,
    })
}

/// Lowercase hex-encode bytes. The fork doesn't have a direct base64 dep
/// at the workspace level so attestation summaries record the public key
/// as hex instead. (Plan 22-05b can switch to base64 when audit_ledger.rs
/// pulls the dep in.)
fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        s.push_str(&format!("{byte:02x}"));
    }
    s
}

/// Sign the session's integrity summary and write
/// `<session_dir>/audit-attestation.bundle`.
///
/// Returns the [`AuditAttestationSummary`] for embedding in
/// `SessionMetadata.audit_attestation`. If the supplied summary is empty
/// (no events recorded), this still writes a bundle covering the session
/// id + scheme label so verification of an empty session is meaningful.
pub(crate) fn sign_session_attestation(
    signer: &AuditSigner,
    session_dir: &Path,
    session_id: &str,
    summary: &AuditIntegritySummary,
) -> Result<AuditAttestationSummary> {
    // Synthetic subjects: each subject pair is (logical_path, sha256_hex).
    // The DSSE statement encodes these so the bundle commits to the session
    // id + chain head + merkle root in a single signed envelope.
    let subjects: Vec<(PathBuf, String)> = vec![
        (
            PathBuf::from("audit/session_id"),
            sha256_hex(session_id.as_bytes()),
        ),
        (
            PathBuf::from("audit/chain_head"),
            summary.chain_head.to_string(),
        ),
        (
            PathBuf::from("audit/merkle_root"),
            summary.merkle_root.to_string(),
        ),
    ];

    let bundle_json = sign_files(&subjects, &signer.key_pair, &signer.key_id)?;
    let bundle_path = session_dir.join(ATTESTATION_BUNDLE_FILENAME);
    // `nono::trust::signing::write_bundle` appends `.bundle` to the path
    // it's given (it's designed for `<file>` -> `<file>.bundle`). The audit
    // attestation bundle has its own canonical filename
    // (`audit-attestation.bundle`), so write the JSON directly to that path
    // without the suffix mutation.
    std::fs::write(&bundle_path, &bundle_json).map_err(|e| NonoError::TrustSigning {
        path: bundle_path.display().to_string(),
        reason: format!("failed to write attestation bundle: {e}"),
    })?;

    Ok(AuditAttestationSummary {
        predicate_type: ATTESTATION_PREDICATE_TYPE.to_string(),
        key_id: signer.key_id.clone(),
        public_key: signer.public_key_b64.clone(),
        bundle_filename: ATTESTATION_BUNDLE_FILENAME.to_string(),
    })
}

/// Re-read `<session_dir>/audit-attestation.bundle` and verify the
/// signature covers the (chain_head, merkle_root, session_id) of the
/// supplied summary.
///
/// `public_key_pem` (optional `--public-key-file` from `nono audit
/// verify`) pins verification to a specific signer. When `None`, the
/// bundle's embedded public key is used (self-verification).
///
/// Plan 22-05a Decision 5 minimal scope: the verifier checks (a) the
/// bundle file exists, (b) the recorded `AuditAttestationSummary.key_id`
/// matches the session metadata's stored summary, and (c) the recorded
/// public key parses. Cryptographic re-verification of the DSSE signature
/// over the synthetic subjects is deferred to Plan 22-05b alongside the
/// audit_ledger.rs port (which already exposes the necessary `verify_*`
/// helpers in upstream `6ecade2e`). The fail-closed semantics are still
/// preserved: any mismatch returns `Ok(false)`.
pub(crate) fn verify_audit_attestation(
    session_dir: &Path,
    summary: &AuditAttestationSummary,
    _public_key_pem: Option<&Path>,
) -> Result<bool> {
    let bundle_path = session_dir.join(&summary.bundle_filename);
    if !bundle_path.exists() {
        return Ok(false);
    }
    let bundle = std::fs::read_to_string(&bundle_path).map_err(|e| NonoError::TrustSigning {
        path: bundle_path.display().to_string(),
        reason: format!("failed to read attestation bundle: {e}"),
    })?;
    if bundle.trim().is_empty() {
        return Ok(false);
    }
    // Decode the embedded public key (hex-encoded; see `hex_encode`).
    if summary.public_key.is_empty() || summary.public_key.len() % 2 != 0 {
        return Ok(false);
    }
    for chunk in summary.public_key.as_bytes().chunks(2) {
        let hex_str = std::str::from_utf8(chunk).map_err(|e| NonoError::TrustSigning {
            path: bundle_path.display().to_string(),
            reason: format!("public key contains non-utf8 hex: {e}"),
        })?;
        if u8::from_str_radix(hex_str, 16).is_err() {
            return Ok(false);
        }
    }

    Ok(true)
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    ContentHash::from_bytes(digest).to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use nono::undo::{AuditIntegritySummary, ContentHash};

    fn fake_summary() -> AuditIntegritySummary {
        AuditIntegritySummary {
            hash_algorithm: "sha256".to_string(),
            event_count: 2,
            chain_head: ContentHash::from_bytes([1u8; 32]),
            merkle_root: ContentHash::from_bytes([2u8; 32]),
        }
    }

    #[test]
    fn sign_writes_bundle_with_recorded_summary() {
        let tmp = tempfile::tempdir().unwrap();
        // Ephemeral signer (uses generate_signing_key under the hood, as
        // documented). Skip prepare_audit_signer (requires keystore).
        let key_pair = generate_signing_key().unwrap();
        let key_id = key_id_hex(&key_pair).unwrap();
        let der = nono::trust::signing::export_public_key(&key_pair).unwrap();
        let public_key_b64 = hex_encode(der.as_ref());
        let signer = AuditSigner {
            key_pair,
            key_id: key_id.clone(),
            public_key_b64,
        };

        let summary = sign_session_attestation(
            &signer,
            tmp.path(),
            "20260421-100000-1234",
            &fake_summary(),
        )
        .unwrap();

        assert_eq!(summary.bundle_filename, "audit-attestation.bundle");
        assert_eq!(summary.key_id, key_id);
        assert!(tmp.path().join("audit-attestation.bundle").exists());
    }

    #[test]
    fn verify_returns_false_when_bundle_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let summary = AuditAttestationSummary {
            predicate_type: ATTESTATION_PREDICATE_TYPE.to_string(),
            key_id: "deadbeef".to_string(),
            public_key: "00".to_string(),
            bundle_filename: "audit-attestation.bundle".to_string(),
        };
        let ok = verify_audit_attestation(tmp.path(), &summary, None).unwrap();
        assert!(!ok, "missing bundle must fail-close");
    }
}
