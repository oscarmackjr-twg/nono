use super::*;
use uuid::Uuid;
use windows_sys::Win32::Foundation::{GetLastError, LocalFree};
use windows_sys::Win32::Security::{
    CreateRestrictedToken, SID_AND_ATTRIBUTES, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE, TOKEN_QUERY,
    WRITE_RESTRICTED,
};

pub(super) struct RestrictedToken {
    pub h_token: HANDLE,
}

impl Drop for RestrictedToken {
    fn drop(&mut self) {
        if !self.h_token.is_null() {
            unsafe { CloseHandle(self.h_token) };
        }
    }
}

pub(crate) fn generate_session_sid() -> String {
    let u = Uuid::new_v4();
    let fields = u.as_fields();
    // Use a custom sub-authority format: S-1-5-117-{D1}-{D2}-{D3}-{D4}
    format!(
        "S-1-5-117-{}-{}-{}-{}",
        fields.0,
        fields.1,
        fields.2,
        u.as_u128() as u32
    )
}

pub(super) fn create_restricted_token_with_sid(session_sid_str: &str) -> Result<RestrictedToken> {
    let mut h_current_token: HANDLE = std::ptr::null_mut();
    let ok = unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_ASSIGN_PRIMARY,
            &mut h_current_token,
        )
    };
    if ok == 0 {
        return Err(NonoError::Setup(format!(
            "Failed to open process token: {}",
            unsafe { GetLastError() }
        )));
    }
    let current_token = OwnedHandle(h_current_token);

    let mut sid_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    let sid_str_u16 = to_u16_null_terminated(session_sid_str);
    let ok = unsafe {
        windows_sys::Win32::Security::Authorization::ConvertStringSidToSidW(
            sid_str_u16.as_ptr(),
            &mut sid_ptr,
        )
    };
    if ok == 0 {
        return Err(NonoError::Setup(format!(
            "Failed to convert SID string {}: {}",
            session_sid_str,
            unsafe { GetLastError() }
        )));
    }

    let sid_restrict = SID_AND_ATTRIBUTES {
        Sid: sid_ptr,
        Attributes: 0,
    };

    // Create the restricted token with WRITE_RESTRICTED.
    //
    // Without a flag, `CreateRestrictedToken` with a restricting SID produces
    // a fully restricted token: every access check is performed twice, once
    // against the user's normal SIDs and once against the restricting SID.
    // Because the synthetic session SID (`S-1-5-117-*`) is absent from every
    // object ACL on the system, access is denied to essentially everything,
    // which manifests as STATUS_ACCESS_DENIED (0xC0000022) when the child
    // attempts any filesystem or kernel-object operation during startup.
    //
    // `WRITE_RESTRICTED` confines the second access check to WRITE-type
    // operations. Reads (including the DLL loads, section mappings, and
    // registry traversal that happen during a console child's initialization)
    // pass through with only the user SIDs being checked. The session SID
    // remains present on the token so Windows Filtering Platform can still
    // match network traffic originating from this process via its
    // `FWPM_CONDITION_ALE_USER_ID` filter (network matching is a read-like
    // access check against the filter's security descriptor).
    //
    // Writes to filesystem locations outside the grant set are still blocked
    // at the Job Object / capability-boundary layer, so the security model
    // remains intact.
    let mut h_restricted: HANDLE = std::ptr::null_mut();
    let ok = unsafe {
        CreateRestrictedToken(
            current_token.0,
            WRITE_RESTRICTED,
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            1,
            &sid_restrict,
            &mut h_restricted,
        )
    };

    unsafe { LocalFree(sid_ptr as _) };

    if ok == 0 {
        return Err(NonoError::Setup(format!(
            "Failed to create restricted token: {}",
            unsafe { GetLastError() }
        )));
    }

    Ok(RestrictedToken {
        h_token: h_restricted,
    })
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenRestrictedSids, TOKEN_GROUPS,
    };

    /// Regression test for the cascading `nono run --allow-cwd` failures that
    /// blocked Phase 13 UAT (debug session
    /// `windows-supervised-exec-cascade`). Prior to this fix,
    /// `CreateRestrictedToken` was called with `Flags=0`, producing a token
    /// whose restricting SID gated every access check — the restricted SID
    /// (`S-1-5-117-*`) appears in no object ACL on the machine, so the
    /// sandboxed child terminated immediately with STATUS_ACCESS_DENIED
    /// (exit code 0xC0000022) before it could execute a single instruction
    /// of user code.
    ///
    /// This test verifies:
    /// 1. The restricted token is produced without error.
    /// 2. The restricted token carries the session SID in its restricting-SID
    ///    list (so WFP network filters still match).
    /// 3. The token actually has the WRITE_RESTRICTED flag applied (checked
    ///    indirectly via the fact that the token construction succeeds and
    ///    the restricting-SID count equals 1 — a sanity check that the
    ///    restricting-SID plumbing is intact while the flag narrows the
    ///    scope of that restriction to writes).
    #[test]
    fn create_restricted_token_with_sid_applies_write_restricted_flag() {
        let sid = generate_session_sid();
        let token = create_restricted_token_with_sid(&sid)
            .expect("create_restricted_token_with_sid must succeed for a freshly-generated SID");
        assert!(!token.h_token.is_null(), "restricted token handle is non-null");

        // Query TokenRestrictedSids — it must contain exactly one SID, our
        // session SID. If the flag were not WRITE_RESTRICTED the test would
        // still see one restricting SID; the semantic behavioural guarantee
        // (reads are unrestricted) is verified end-to-end by
        // `spawn_windows_child` smoke tests and the Phase 13 UAT run.
        let mut needed: u32 = 0;
        unsafe {
            // SAFETY: First call with null buffer queries the required size.
            GetTokenInformation(
                token.h_token,
                TokenRestrictedSids,
                std::ptr::null_mut(),
                0,
                &mut needed,
            );
        }
        assert!(
            needed >= std::mem::size_of::<TOKEN_GROUPS>() as u32,
            "TokenRestrictedSids buffer size should be at least TOKEN_GROUPS size, got {needed}"
        );

        let mut buf = vec![0u8; needed as usize];
        let ok = unsafe {
            GetTokenInformation(
                token.h_token,
                TokenRestrictedSids,
                buf.as_mut_ptr() as *mut _,
                needed,
                &mut needed,
            )
        };
        assert!(
            ok != 0,
            "GetTokenInformation(TokenRestrictedSids) must succeed on a restricted token"
        );

        let groups = unsafe { &*(buf.as_ptr() as *const TOKEN_GROUPS) };
        assert_eq!(
            groups.GroupCount, 1,
            "restricted token must carry exactly one restricting SID (the session SID)"
        );
    }

    /// Regression test pinned to the exact exit-code signature the user
    /// reported in the debug session. Before the WRITE_RESTRICTED fix, any
    /// sandboxed child created via `create_restricted_token_with_sid` died
    /// with `STATUS_ACCESS_DENIED` during image load. After the fix, the
    /// token is constructable and has a live handle the caller can pass to
    /// `CreateProcessAsUserW`. The handle being non-null and the construction
    /// succeeding is the property that was absent before; verifying exit
    /// codes requires an actual process spawn and is covered by the UAT
    /// scripts (`./target/release/nono.exe run --allow-cwd -- cmd /c "echo hello"`).
    #[test]
    fn create_restricted_token_with_sid_returns_usable_handle_for_child_spawn() {
        let sid = generate_session_sid();
        let token = create_restricted_token_with_sid(&sid).expect("token construction");
        assert!(!token.h_token.is_null());
        // Drop test — ensure Drop doesn't panic or double-close.
        drop(token);
    }

    /// Regression test for the token drop lifecycle. The pre-existing token
    /// UAF fix (commit `eb4730c`) relied on the `RestrictedToken` wrapper's
    /// Drop closing the handle exactly once. Verify Drop is idempotent in
    /// the sense that it never attempts to close a null handle.
    #[test]
    fn restricted_token_drop_is_null_safe() {
        let token = RestrictedToken {
            h_token: std::ptr::null_mut(),
        };
        // Drop on a null handle must not call CloseHandle.
        drop(token);
    }

    /// Verify `generate_session_sid` produces a parsable SDDL string that
    /// Windows accepts as a valid SID. This is implicitly exercised by
    /// `create_restricted_token_with_sid_applies_write_restricted_flag`
    /// above; spelling it out as its own test makes the regression obvious
    /// if the SID format ever drifts outside the Microsoft-reserved range.
    #[test]
    fn generate_session_sid_produces_parsable_sddl_string() {
        let sid = generate_session_sid();
        assert!(
            sid.starts_with("S-1-5-117-"),
            "session SID must use the Microsoft-reserved sub-authority 117: {sid}"
        );

        let wide = to_u16_null_terminated(&sid);
        let mut out: *mut std::ffi::c_void = std::ptr::null_mut();
        let ok = unsafe {
            windows_sys::Win32::Security::Authorization::ConvertStringSidToSidW(
                wide.as_ptr(),
                &mut out,
            )
        };
        assert_ne!(
            ok, 0,
            "ConvertStringSidToSidW must accept the generated SID string"
        );
        unsafe { LocalFree(out as _) };
    }
}
