//! Phase 21: Applied-labels RAII guard.
//!
//! Records the pre-grant mandatory-label state for every path in a
//! `WindowsFilesystemPolicy` and reverts those labels at Drop time. Mirrors
//! the `NetworkEnforcementGuard` pattern in `mod.rs`.
//!
//! D-02 semantics (tightened in revision):
//! - If a path has ANY pre-existing mandatory-label ACE (regardless of RID —
//!   Low, Medium, High, System, or unknown), the guard records `Skip` for that
//!   path: we do NOT apply a new label, we do NOT revert at Drop. This
//!   preserves the contract that nono NEVER mutates a pre-existing label.
//!   The skip emits a `tracing::warn!` so the user can diagnose why a grant
//!   to an already-labeled file produced no observable enforcement change.
//! - If a path has NO label pre-grant (the common case for user files),
//!   we apply the Plan 21-02 mode-derived mask and record `Applied { path }`.
//!   On Drop we clear the ACE (restoring the Medium-IL default).
//! - Concurrent sessions sharing the same path: "last session out restores" —
//!   accepted trade-off versus refcount/lease plumbing.

use std::path::{Path, PathBuf};

use nono::{
    label_mask_for_access_mode, low_integrity_label_and_mask, try_set_mandatory_label, NonoError,
    Result, WindowsFilesystemPolicy,
};

/// Per-path state recorded at snapshot time.
#[derive(Debug)]
enum AppliedLabel {
    /// Path had a pre-existing mandatory-label ACE of some kind; we did not
    /// apply a new label and will not revert at Drop time. D-02
    /// skip-on-any-prior-label arm. Preserves the contract that nono NEVER
    /// mutates a pre-existing mandatory label.
    Skip,
    /// Path had NO pre-existing mandatory-label ACE. We applied a
    /// mode-derived mandatory-label ACE at Low IL via
    /// `nono::try_set_mandatory_label`. On Drop, clear the ACE to restore
    /// the Medium-IL default.
    Applied { path: PathBuf },
}

/// RAII guard that reverts applied mandatory labels when dropped.
///
/// Constructed via [`AppliedLabelsGuard::snapshot_and_apply`]. The guard owns
/// the apply side-effect; Drop runs revert.
#[derive(Debug, Default)]
pub(crate) struct AppliedLabelsGuard {
    entries: Vec<AppliedLabel>,
}

impl AppliedLabelsGuard {
    /// For every rule in `policy.rules`:
    /// 1. Snapshot prior label state via `nono::low_integrity_label_and_mask`.
    /// 2. If `prior.is_some()` (ANY pre-existing mandatory-label ACE),
    ///    record `Skip` and log a warning. Do NOT apply.
    /// 3. Otherwise call `nono::try_set_mandatory_label` with the mode-derived
    ///    mask. Record `Applied { path }`.
    /// 4. If `try_set_mandatory_label` fails at step 3, best-effort-revert any
    ///    `Applied` entries already added to `self.entries`, then return the
    ///    original Err.
    ///
    /// Fail-closed: returns Err(LabelApplyFailed) on any apply failure; no
    /// partial-success state returned.
    pub(crate) fn snapshot_and_apply(policy: &WindowsFilesystemPolicy) -> Result<Self> {
        let mut guard = Self::default();
        for rule in &policy.rules {
            let prior = low_integrity_label_and_mask(&rule.path);
            if let Some((prior_rid, prior_mask)) = prior {
                // D-02 skip-on-any-prior-label: a file with ANY existing
                // mandatory-label ACE is NOT touched. This preserves the
                // contract that nono never mutates a pre-existing label.
                tracing::warn!(
                    path = %rule.path.display(),
                    prior_rid = format!("0x{prior_rid:X}"),
                    prior_mask = format!("0x{prior_mask:X}"),
                    "label guard: path has pre-existing mandatory-label ACE; skipping apply + revert \
                     (grant may have no observable enforcement effect depending on pre-existing label)"
                );
                guard.entries.push(AppliedLabel::Skip);
                continue;
            }

            let mask = label_mask_for_access_mode(rule.access);
            if let Err(err) = try_set_mandatory_label(&rule.path, mask) {
                tracing::warn!(
                    path = %rule.path.display(),
                    mask = format!("0x{mask:X}"),
                    "label guard: apply failed; reverting entries already applied"
                );
                guard.revert_all();
                return Err(err);
            }
            guard.entries.push(AppliedLabel::Applied {
                path: rule.path.clone(),
            });
        }
        Ok(guard)
    }

    /// Best-effort revert of every applied entry. Drop-safe: errors logged,
    /// never panic.
    fn revert_all(&mut self) {
        // Consume entries in LIFO order so the most-recently-applied paths
        // get reverted first. Mirrors the Phase 04 WFP orphan-sweep discipline.
        while let Some(entry) = self.entries.pop() {
            match entry {
                AppliedLabel::Skip => {
                    // No-op: we never applied, so there is nothing to revert.
                }
                AppliedLabel::Applied { path } => {
                    Self::best_effort_revert(&path);
                }
            }
        }
    }

    fn best_effort_revert(path: &Path) {
        // We only ever applied to paths that had NO prior mandatory-label ACE.
        // Revert = clear the ACE we added, returning the path to the Medium-IL
        // default (Windows' implicit label for user files absent explicit
        // labeling).
        if let Err(err) = clear_mandatory_label(path) {
            tracing::warn!(
                path = %path.display(),
                error = %err,
                "label guard: revert via clear failed; path may remain labeled"
            );
        }
    }
}

impl Drop for AppliedLabelsGuard {
    fn drop(&mut self) {
        self.revert_all();
    }
}

/// Clears the mandatory-label ACE on `path`, returning the path to the
/// Medium-IL default (no label). Uses `SetNamedSecurityInfoW` with a
/// constructed "S:" (empty-SACL) SDDL. Fail-closed: returns
/// `NonoError::LabelApplyFailed` on non-zero.
///
/// **windows-sys 0.59 module paths** (locked per Plan 21-02 SUMMARY §
/// Decisions Made):
/// - `GetSecurityDescriptorSacl` lives in `Win32::Security` (NOT
///   `Win32::Security::Authorization`).
/// - `SetNamedSecurityInfoW` signature is
///   `(PCWSTR, SE_OBJECT_TYPE, OBJECT_SECURITY_INFORMATION, PSID, PSID,
///     *const ACL, *const ACL) -> WIN32_ERROR` — object name is `*const u16`,
///   `psacl` is `*const ACL`.
/// - `SDDL_REVISION_1` is in `Win32::Security::Authorization`.
#[cfg(target_os = "windows")]
fn clear_mandatory_label(path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{GetLastError, LocalFree};
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SetNamedSecurityInfoW,
        SDDL_REVISION_1, SE_FILE_OBJECT,
    };
    use windows_sys::Win32::Security::{
        GetSecurityDescriptorSacl, ACL, LABEL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    };

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // Empty SACL SDDL: "S:" alone means "no SACL entries". Windows interprets
    // this as "clear any existing mandatory-label ACE when passed via
    // SetNamedSecurityInfoW with LABEL_SECURITY_INFORMATION".
    let wide_sddl: Vec<u16> = "S:".encode_utf16().chain(std::iter::once(0)).collect();

    let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    let ok = unsafe {
        // SAFETY: `wide_sddl` is a valid nul-terminated UTF-16 buffer; `sd`
        // is a valid mutable out-pointer. On success the callee allocates an
        // SD which we free below via LocalFree.
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide_sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut sd,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        let hresult = unsafe {
            // SAFETY: GetLastError is a thread-local read with no preconditions.
            GetLastError()
        };
        return Err(NonoError::LabelApplyFailed {
            path: path.to_path_buf(),
            hresult,
            hint: "Failed to construct empty-SACL security descriptor for label clear".to_string(),
        });
    }

    let mut sacl: *mut ACL = std::ptr::null_mut();
    let mut sacl_present: i32 = 0;
    let mut sacl_defaulted: i32 = 0;
    let _ = unsafe {
        // SAFETY: `sd` is a valid SD pointer returned by
        // ConvertStringSecurityDescriptorToSecurityDescriptorW above; the
        // three out-parameters refer to live local storage for the duration
        // of the call.
        GetSecurityDescriptorSacl(sd, &mut sacl_present, &mut sacl, &mut sacl_defaulted)
    };
    // For a "S:" SDDL, sacl_present==1 and sacl==null — that IS the empty-SACL
    // shape Windows uses to clear the mandatory label. We pass whatever
    // GetSecurityDescriptorSacl returned directly.

    let status = unsafe {
        // SAFETY: `wide_path` is a valid nul-terminated UTF-16 buffer;
        // `sacl` points into `sd` which is kept alive until the LocalFree
        // below. SetNamedSecurityInfoW signature per Plan 21-02 SUMMARY:
        // psacl is *const ACL.
        SetNamedSecurityInfoW(
            wide_path.as_ptr(),
            SE_FILE_OBJECT,
            LABEL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
            sacl,
        )
    };

    unsafe {
        // SAFETY: `sd` was allocated by
        // ConvertStringSecurityDescriptorToSecurityDescriptorW and must be
        // freed with LocalFree per Win32 contract.
        LocalFree(sd as _);
    }

    if status == 0 {
        Ok(())
    } else {
        Err(NonoError::LabelApplyFailed {
            path: path.to_path_buf(),
            hresult: status,
            hint: format!("SetNamedSecurityInfoW returned 0x{status:08X} while clearing label"),
        })
    }
}

#[cfg(test)]
#[cfg(target_os = "windows")]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use nono::{AccessMode, CapabilitySource, WindowsFilesystemRule};
    use tempfile::tempdir;

    fn single_file_read_rule(path: std::path::PathBuf) -> WindowsFilesystemPolicy {
        WindowsFilesystemPolicy {
            rules: vec![WindowsFilesystemRule {
                path,
                access: AccessMode::Read,
                is_file: true,
                source: CapabilitySource::User,
            }],
            unsupported: vec![],
        }
    }

    #[test]
    fn guard_apply_then_drop_reverts_label_for_fresh_file() {
        let dir = tempdir().expect("tempdir");
        let file = dir.path().join("note.txt");
        std::fs::write(&file, "x").expect("write file");
        let policy = single_file_read_rule(file.clone());

        // Pre-condition: file has no mandatory-label ACE.
        let pre = low_integrity_label_and_mask(&file);
        assert!(
            pre.is_none(),
            "test precondition: file must not already carry a mandatory label; got {pre:?}"
        );

        {
            let _guard = AppliedLabelsGuard::snapshot_and_apply(&policy).expect("apply");
            // Inside the guard's scope, file MUST be Low IL.
            let during = low_integrity_label_and_mask(&file).expect("label must be present");
            assert_eq!(
                during.0, 0x1000,
                "file must be at Low IL during guard lifetime"
            );
        } // guard drops here → revert

        // Post-condition: file's label was cleared (Medium-IL default restored).
        let post = low_integrity_label_and_mask(&file);
        assert!(
            post.is_none(),
            "file must not carry a mandatory-label ACE after guard drop; got {post:?}"
        );
    }

    #[test]
    fn guard_skips_apply_and_revert_when_path_already_has_any_mandatory_label() {
        // Pre-label a file Low IL via the raw primitive (simulating a prior
        // session's residue OR a third-party tool that hardened the file).
        // Then construct a guard over the same path and verify Drop does NOT
        // clear the label.
        let dir = tempdir().expect("tempdir");
        let file = dir.path().join("note.txt");
        std::fs::write(&file, "x").expect("write file");
        // Use nono's raw primitive to mandatorily-label the file externally.
        try_set_mandatory_label(&file, 0x5) // NO_WRITE_UP | NO_EXECUTE_UP
            .expect("pre-label apply");
        let pre = low_integrity_label_and_mask(&file).expect("pre-label present");
        assert_eq!(pre.0, 0x1000);

        let policy = single_file_read_rule(file.clone());
        {
            let _guard = AppliedLabelsGuard::snapshot_and_apply(&policy).expect("apply");
            // D-02 skip-on-any-prior-label: we did not touch the label.
            let during = low_integrity_label_and_mask(&file).expect("still labeled");
            assert_eq!(during.0, 0x1000);
            // Sanity: the Skip variant is exercised.
            let _skip_variant_reference = AppliedLabel::Skip;
        } // guard drops

        // D-02 "skip revert on any pre-existing label" — label persists.
        let post = low_integrity_label_and_mask(&file).expect("label must persist");
        assert_eq!(
            post.0, 0x1000,
            "guard must not revert a label it did not apply"
        );
    }

    #[test]
    fn guard_reverts_all_entries_if_mid_loop_apply_fails() {
        // Two files; first applies fine, second fails (we simulate by pointing
        // rule 2 at a path that does not exist — apply will fail with
        // LabelApplyFailed).
        let dir = tempdir().expect("tempdir");
        let ok_file = dir.path().join("ok.txt");
        std::fs::write(&ok_file, "x").expect("write file");
        let bad_path = dir.path().join("does-not-exist.txt");

        // Manually construct the policy with a non-existent path to bypass
        // FsCapability::new_file's path-existence check.
        //
        // CapabilitySource::User is the correct variant for grants originating
        // from CLI flags (e.g. --allow-file) — matches the four actual variants
        // in crates/nono/src/capability.rs. There is NO `Direct` variant.
        let policy = WindowsFilesystemPolicy {
            rules: vec![
                WindowsFilesystemRule {
                    path: ok_file.clone(),
                    access: AccessMode::Read,
                    is_file: true,
                    source: CapabilitySource::User,
                },
                WindowsFilesystemRule {
                    path: bad_path.clone(),
                    access: AccessMode::Read,
                    is_file: true,
                    source: CapabilitySource::User,
                },
            ],
            unsupported: vec![],
        };

        let result = AppliedLabelsGuard::snapshot_and_apply(&policy);
        assert!(result.is_err(), "apply must fail on non-existent path");

        // Post-condition: ok_file was labeled (briefly) and then reverted by
        // the in-function rollback — it must NOT carry a mandatory-label ACE
        // anymore.
        let post = low_integrity_label_and_mask(&ok_file);
        assert!(
            post.is_none(),
            "ok_file must not carry a mandatory-label ACE after failed apply; got {post:?}"
        );
    }
}
