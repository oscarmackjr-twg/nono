//! Executable identity capture for the audit trail.
//!
//! Computes a cross-platform `ExecutableIdentity` (canonical path + SHA-256
//! hash of the launched binary) before sandbox apply so the audit ledger
//! commits to exactly the bytes the supervisor handed off to the kernel.
//!
//! AUD-03 SHA-256 portion (Plan 22-05a Task 4 — manual port replay of upstream
//! `02ee0bd1` per D-02 fallback gate; the full upstream commit also included
//! `MerkleScheme::DomainSeparatedV3`, `audit_ledger.rs` flock changes, and
//! cross-file refactors that breached D-02 thresholds for cherry-pick).
//!
//! **NOT in this file (per Plan 22-05a boundary discipline LOCKED):** Windows
//! signature-trust verification. That ships in Plan 22-05b as a SIBLING
//! field on the audit envelope (per RESEARCH Contradiction #2 — upstream's
//! `ExecutableIdentity` is SHA-256 only at v0.40.1; the additional Windows
//! signature-trust portion is fork-only and lands later).

use nono::undo::{ContentHash, ExecutableIdentity};
use nono::{NonoError, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Compute the canonical path + SHA-256 hash of the launched executable.
///
/// Canonicalizes the resolved program path then streams its bytes through
/// SHA-256 in 8 KiB chunks. Returns `NonoError::CommandExecution` if either
/// canonicalization, file open, or read fails.
pub(crate) fn compute(resolved_program: &Path) -> Result<ExecutableIdentity> {
    let canonical_path = resolved_program.canonicalize().map_err(|e| {
        NonoError::CommandExecution(std::io::Error::new(
            e.kind(),
            format!(
                "Failed to canonicalize executable {}: {e}",
                resolved_program.display()
            ),
        ))
    })?;
    let mut file = File::open(&canonical_path).map_err(|e| {
        NonoError::CommandExecution(std::io::Error::new(
            e.kind(),
            format!(
                "Failed to open executable {}: {e}",
                canonical_path.display()
            ),
        ))
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = file.read(&mut buffer).map_err(|e| {
            NonoError::CommandExecution(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to read executable {}: {e}",
                    canonical_path.display()
                ),
            ))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(ExecutableIdentity {
        resolved_path: canonical_path,
        sha256: ContentHash::from_bytes(hasher.finalize().into()),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::fs;

    #[test]
    fn compute_hashes_canonical_binary_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("tool");
        fs::write(&binary, b"#!/bin/sh\necho hello\n").unwrap();

        let identity = compute(&binary).unwrap();
        let expected = Sha256::digest(b"#!/bin/sh\necho hello\n");

        assert_eq!(identity.resolved_path, binary.canonicalize().unwrap());
        assert_eq!(identity.sha256.as_bytes(), &<[u8; 32]>::from(expected));
    }

    #[test]
    fn compute_propagates_canonicalize_errors() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");

        let err = compute(&missing).unwrap_err();
        assert!(matches!(err, NonoError::CommandExecution(_)));
    }
}
