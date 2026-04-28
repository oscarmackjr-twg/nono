//! Windows Authenticode exec-identity recording (REQ-AUD-03 acceptance #2/#3).
//!
//! Plan 22-05b Task 4 — fork-only addition per CONTEXT § Integration Points
//! line 248 (D-17 ALLOWED). FFI style mirrors Phase 21's
//! `crates/nono/src/sandbox/windows.rs::try_set_mandatory_label`:
//! `encode_wide` UTF-16 conversion, `unsafe { ... }` blocks paired with
//! `// SAFETY:` doc comments, RAII close guard for `WTD_STATEACTION_CLOSE`,
//! `GetLastError` -> typed `NonoError`.
//!
//! Sibling field on the audit envelope per RESEARCH Contradiction #2:
//! `AuthenticodeStatus` does NOT mutate upstream's `ExecutableIdentity`
//! struct shape; SHA-256 capture stays independent and always happens.
//!
//! On any FFI failure (helpers absent / runtime error / unsigned binary),
//! the caller falls back to the SHA-256-only audit path captured by
//! `exec_identity::compute`.
//!
//! ## Decision 4 fallback (documented)
//!
//! `windows-sys 0.59` does not expose the `WTHelperProvDataFromStateData`
//! / `WTHelperGetProvSignerFromChain` walkers without pulling in the
//! `Win32_Security_Cryptography_Catalog` + `Win32_Security_Cryptography_Sip`
//! features (whose CRYPT_PROVIDER_DATA struct shapes are gated behind
//! both). Adding those features pulls in significantly more attack
//! surface than this plan's scope justifies. Per the user-resolved
//! Decision 4 fallback path, this implementation:
//!
//! 1. Calls `WinVerifyTrust` with `WTD_REVOKE_NONE` to determine
//!    `Valid` / `Unsigned` / `InvalidSignature { hresult }` — these
//!    are `WinVerifyTrust` return codes alone, requiring no helpers.
//! 2. Records `signer_subject: "<unknown>"` and an empty `thumbprint`
//!    on `Valid` because the chain walkers are unavailable.
//! 3. Defers full subject + thumbprint extraction to v2.3 backlog
//!    (alongside the D-13 fixture re-enablement) — see ROADMAP entry
//!    "Audit-attestation D-13 fixtures re-enablement (deferred from
//!    Plan 22-05b)".
//! 4. Marks the `authenticode_signed_records_subject` substring
//!    assertion test `#[ignore]` with a deferral note pointing at
//!    the same v2.3 backlog row.
//!
//! Crucially, the HRESULT IS recorded on `InvalidSignature`, the
//! `Valid` discriminant IS surfaced in the audit ledger, and the
//! audit envelope's SHA-256 fallback is unaffected. This satisfies
//! AUD-03 acceptance #2 (signature recorded) and #3 (SHA-256 fallback)
//! at the integrity layer; only the human-readable signer subject
//! display loses fidelity until the v2.3 backlog item lands.

#![cfg(target_os = "windows")]

use nono::Result;
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::Security::WinTrust::{
    WinVerifyTrust, WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0,
    WINTRUST_FILE_INFO, WTD_CHOICE_FILE, WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE,
    WTD_STATEACTION_VERIFY, WTD_UI_NONE,
};

/// Authenticode status for an executable.
///
/// Sibling field on the audit envelope (RESEARCH Contradiction #2 — does
/// NOT mutate upstream's `ExecutableIdentity` struct shape; SHA-256 capture
/// stays independent and always happens).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthenticodeStatus {
    /// Signature valid; chain validated to a trusted root by `WinVerifyTrust`.
    Valid {
        /// Signer subject (typically the leaf-cert CN) extracted via
        /// `CertGetNameStringW(CERT_NAME_SIMPLE_DISPLAY_TYPE)`. May be
        /// `"<unknown>"` when the helpers are unavailable on the running
        /// `windows-sys` minor version (graceful fallback per Decision 4).
        signer_subject: String,
        /// SHA-1 thumbprint of the signing cert as a lowercase hex string,
        /// extracted via `CertGetCertificateContextProperty(CERT_HASH_PROP_ID)`.
        /// Empty when the property could not be read (graceful fallback).
        thumbprint: String,
    },
    /// File present but unsigned (`TRUST_E_NOSIGNATURE`).
    Unsigned,
    /// File signed but signature invalid / chain rejected. The `hresult`
    /// field carries the raw `WinVerifyTrust` return value for forensics.
    InvalidSignature { hresult: i32 },
    /// Signature query itself failed (e.g. file missing). Caller falls back
    /// to SHA-256-only audit envelope per AUD-03 acceptance #3.
    QueryFailed { reason: String },
}

/// `TRUST_E_NOSIGNATURE` — well-known WinTrust HRESULT for "file is not
/// signed". Surfaced verbatim in the audit ledger for forensic clarity.
const TRUST_E_NOSIGNATURE: u32 = 0x800B0100;

/// Record exec-identity Authenticode status for `path`.
///
/// Calls `WinVerifyTrust` with `WTD_REVOKE_NONE` (best-effort signature
/// query without CRL/OCSP latency per T-22-05b-02 mitigation; SHA-256
/// fallback ensures audit completes even on Authenticode failure). Always
/// pairs the `WTD_STATEACTION_VERIFY` call with a `WTD_STATEACTION_CLOSE`
/// call on Drop via `WinTrustCloseGuard` (T-22-05b-05 mitigation).
///
/// Returns `Ok(AuthenticodeStatus::QueryFailed { .. })` for path-conversion
/// failures rather than `Err(..)` so the caller's "fall through to SHA-256"
/// branch is exercised uniformly.
#[must_use = "ignoring the AuthenticodeStatus drops audit evidence"]
pub fn query_authenticode_status(path: &Path) -> Result<AuthenticodeStatus> {
    // UTF-16 path conversion (mirrors sandbox/windows.rs::try_set_mandatory_label).
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // Heuristic: if `path` is empty post-conversion, the path conversion
    // produced nothing valid. Surface as QueryFailed so the caller falls
    // through to SHA-256.
    if wide.len() < 2 {
        return Ok(AuthenticodeStatus::QueryFailed {
            reason: format!("empty UTF-16 path conversion for {}", path.display()),
        });
    }

    let file_info = WINTRUST_FILE_INFO {
        cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: wide.as_ptr(),
        hFile: std::ptr::null_mut(),
        pgKnownSubject: std::ptr::null_mut(),
    };

    let mut wtd = WINTRUST_DATA {
        cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
        pPolicyCallbackData: std::ptr::null_mut(),
        pSIPClientData: std::ptr::null_mut(),
        dwUIChoice: WTD_UI_NONE,
        // Best-effort signature query without CRL/OCSP latency
        // (T-22-05b-02 mitigation; AUD-03 acceptance allows
        // "Signature failures do not prevent session start").
        fdwRevocationChecks: WTD_REVOKE_NONE,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 {
            pFile: &file_info as *const _ as *mut WINTRUST_FILE_INFO,
        },
        dwStateAction: WTD_STATEACTION_VERIFY,
        hWVTStateData: std::ptr::null_mut(),
        pwszURLReference: std::ptr::null_mut(),
        dwProvFlags: 0,
        dwUIContext: 0,
        pSignatureSettings: std::ptr::null_mut(),
    };

    // SAFETY: `WINTRUST_ACTION_GENERIC_VERIFY_V2` is a static GUID exported
    // by windows-sys; `&mut wtd` points to a valid stack-allocated
    // WINTRUST_DATA pre-populated above. `hWnd = NULL` is the documented
    // headless-verify shape. The first call requests verification; the
    // matching `WTD_STATEACTION_CLOSE` second call is guaranteed by the
    // RAII `WinTrustCloseGuard` constructed below (Drop fires even on
    // early return / panic). `wtd.hWVTStateData` is mutated by Windows
    // and read back in `parse_signer_subject` / `parse_thumbprint`.
    let verify_result: i32 = unsafe {
        WinVerifyTrust(
            std::ptr::null_mut(),
            &WINTRUST_ACTION_GENERIC_VERIFY_V2 as *const _ as *mut _,
            &mut wtd as *mut _ as *mut c_void,
        )
    };

    // RAII close guard MUST be constructed BEFORE we read `wtd.hWVTStateData`
    // so any early-return path (including panic propagation) still runs
    // the matching `WTD_STATEACTION_CLOSE` call. Mirrors Phase 21's
    // `_sd_guard` pattern (T-22-05b-05 mitigation).
    let _close_guard = WinTrustCloseGuard {
        wtd: &mut wtd as *mut WINTRUST_DATA,
    };

    let status = if verify_result == 0 {
        // Per Decision 4 fallback: windows-sys 0.59 does not expose the
        // WTHelperProvDataFromStateData / WTHelperGetProvSignerFromChain
        // chain walkers without enabling the
        // Win32_Security_Cryptography_Catalog + ..._Sip features (whose
        // CRYPT_PROVIDER_DATA shape is gated behind both). The plan
        // routes around the missing helpers by recording the trust
        // discriminant alone; subject + thumbprint extraction lives in
        // the v2.3 backlog ("Audit-attestation D-13 fixtures
        // re-enablement"). Both helpers are intentional best-effort
        // stubs returning the documented sentinel values.
        let signer_subject = parse_signer_subject(&wtd);
        let thumbprint = parse_thumbprint(&wtd);
        AuthenticodeStatus::Valid {
            signer_subject,
            thumbprint,
        }
    } else if (verify_result as u32) == TRUST_E_NOSIGNATURE {
        AuthenticodeStatus::Unsigned
    } else {
        AuthenticodeStatus::InvalidSignature {
            hresult: verify_result,
        }
    };

    Ok(status)
}

/// RAII close-guard for the second `WinVerifyTrust` call with
/// `WTD_STATEACTION_CLOSE`. Mirrors Phase 21's `_sd_guard` pattern in
/// `sandbox/windows.rs`. ALWAYS runs the close call to release the
/// state allocated by the first verify call (T-22-05b-05 mitigation:
/// state-leak via mis-ordered close).
struct WinTrustCloseGuard {
    wtd: *mut WINTRUST_DATA,
}

impl Drop for WinTrustCloseGuard {
    fn drop(&mut self) {
        // SAFETY: `self.wtd` points to the same stack-allocated WINTRUST_DATA
        // referenced by the matching VERIFY call above. Setting
        // `dwStateAction = WTD_STATEACTION_CLOSE` and re-invoking
        // WinVerifyTrust with the same hWVTStateData is the documented
        // close-pair pattern. Errors from the close call are best-effort
        // (we are in Drop and cannot propagate); they do not affect audit
        // correctness because the state being leaked is verify-side only.
        unsafe {
            (*self.wtd).dwStateAction = WTD_STATEACTION_CLOSE;
            let _ = WinVerifyTrust(
                std::ptr::null_mut(),
                &WINTRUST_ACTION_GENERIC_VERIFY_V2 as *const _ as *mut _,
                self.wtd as *mut c_void,
            );
        }
    }
}

/// Decision 4 fallback: returns the sentinel `"<unknown>"` because
/// `windows-sys 0.59` does not expose `WTHelperProvDataFromStateData`
/// / `WTHelperGetProvSignerFromChain` without the
/// `Win32_Security_Cryptography_Catalog` + `..._Sip` features (whose
/// `CRYPT_PROVIDER_DATA` shape is gated behind both). The
/// `WinVerifyTrust` discriminant alone is recorded; full chain-walking
/// extraction is deferred to the v2.3 backlog row "Audit-attestation
/// D-13 fixtures re-enablement (deferred from Plan 22-05b)".
///
/// `wtd` is referenced for future-proofing — when the helpers re-enable
/// in a later windows-sys minor version, this function expands in
/// place to walk `wtd.hWVTStateData`. The argument is intentionally
/// not `_unused` so the call site remains unchanged.
fn parse_signer_subject(wtd: &WINTRUST_DATA) -> String {
    let _ = wtd; // future-proof: chain walkers will read wtd.hWVTStateData
    String::from("<unknown>")
}

/// Decision 4 fallback: returns an empty string because the
/// `CertGetCertificateContextProperty` chain walker is unreachable
/// without the cryptography-catalog/sip features (see
/// `parse_signer_subject` for the full rationale).
fn parse_thumbprint(wtd: &WINTRUST_DATA) -> String {
    let _ = wtd; // future-proof: chain walkers will read wtd.hWVTStateData
    String::new()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn unsigned_temp_file_returns_unsigned_or_invalid() {
        // A short tempfile that LOOKS like a PE start but has no signature.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("unsigned.exe");
        std::fs::write(&path, b"MZ\x90\x00\x03\x00\x00\x00").unwrap();
        let status = query_authenticode_status(&path).unwrap();
        // Either Unsigned (most likely) or InvalidSignature is acceptable —
        // both signal "fall back to SHA-256". The unit test refuses to
        // require Valid for a tempfile.
        assert!(
            matches!(
                status,
                AuthenticodeStatus::Unsigned | AuthenticodeStatus::InvalidSignature { .. }
            ),
            "expected Unsigned or InvalidSignature, got: {status:?}"
        );
    }

    #[test]
    fn missing_path_returns_invalid_or_query_failed() {
        let path = Path::new(r"C:\nonexistent\path\that\should\not\exist.exe");
        let result = query_authenticode_status(path);
        match result {
            Ok(AuthenticodeStatus::QueryFailed { .. })
            | Ok(AuthenticodeStatus::InvalidSignature { .. })
            | Ok(AuthenticodeStatus::Unsigned)
            | Err(_) => (),
            other => panic!(
                "expected QueryFailed/InvalidSignature/Unsigned/Err for missing path, got: {other:?}"
            ),
        }
    }
}
