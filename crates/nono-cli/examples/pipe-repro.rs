//! Standalone repro tool for
//! `.planning/debug/supervisor-pipe-access-denied.md`.
//!
//! This binary reproduces — in isolation — the production failure where a
//! WRITE_RESTRICTED child with a per-session restricting SID cannot
//! `CreateFileW(GENERIC_READ | GENERIC_WRITE)` on the capability pipe the
//! supervisor created. It lets us iterate on SDDL shapes with a
//! ~2-second rebuild+run cycle instead of walking the full supervised
//! runtime on every iteration.
//!
//! ## Modes
//!
//! - **Parent** (default, or `--parent`):
//!   1. Generate a fresh synthetic session SID via the SAME algorithm as
//!      `crate::exec_strategy_windows::restricted_token::generate_session_sid`
//!      — see that file for the invariant (`S-1-5-117-<u32>-<u32>-<u32>-<u32>`).
//!   2. Build an SDDL string from `--sddl-template` (or the default production
//!      template), substituting `{sid}` for the generated SID.
//!   3. Convert the SDDL to a `SECURITY_DESCRIPTOR` via
//!      `ConvertStringSecurityDescriptorToSecurityDescriptorW`.
//!   4. Create a named pipe
//!      `\\.\pipe\nono-pipe-repro-<nonce>` with `PIPE_ACCESS_DUPLEX` + those
//!      security attributes.
//!   5. Create a `WRITE_RESTRICTED` primary token with the session SID as its
//!      single restricting SID (identical to production).
//!   6. Spawn the current executable in child mode (`--child <pipe_name>`) via
//!      `CreateProcessAsUserW` using that token.
//!   7. Wait for the child to exit and print its exit code + any diagnostics.
//!
//! - **Child** (`--child <pipe_name>`):
//!   1. Call `CreateFileW(pipe_name, GENERIC_READ | GENERIC_WRITE, …,
//!      OPEN_EXISTING, …)`.
//!   2. Print success (handle value) or failure (`GetLastError` +
//!      formatted `std::io::Error::last_os_error()`).
//!   3. Exit 0 on success, 1 on `ERROR_ACCESS_DENIED`, 2 on other errors.
//!
//! The repro is Windows-only. On other platforms it is an empty stub so the
//! crate still compiles (the `examples/` directory is auto-discovered by
//! Cargo).

#![allow(clippy::unwrap_used, clippy::expect_used)]

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("pipe-repro: Windows-only; this binary is a stub on other platforms.");
}

#[cfg(target_os = "windows")]
fn main() -> std::process::ExitCode {
    windows_impl::run()
}

#[cfg(target_os = "windows")]
mod windows_impl {

use std::env;
use std::os::windows::ffi::OsStrExt;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, LocalFree, ERROR_ACCESS_DENIED, GENERIC_READ, GENERIC_WRITE, HANDLE,
    INVALID_HANDLE_VALUE, WAIT_OBJECT_0,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, ConvertStringSidToSidW,
};
use windows_sys::Win32::Security::{
    CreateRestrictedToken, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, SID_AND_ATTRIBUTES,
    TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE, TOKEN_QUERY, WRITE_RESTRICTED,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::Pipes::{
    CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{
    CreateProcessAsUserW, GetCurrentProcess, GetExitCodeProcess, OpenProcessToken, WaitForSingleObject,
    CREATE_UNICODE_ENVIRONMENT, INFINITE, PROCESS_INFORMATION, STARTUPINFOW,
};

const SDDL_REVISION_1: u32 = 1;
const DEFAULT_SDDL_TEMPLATE: &str =
    "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)(A;;0x0012019F;;;{sid})S:(ML;;NW;;;LW)";
const MAX_PIPE_MESSAGE_SIZE: u32 = 64 * 1024;
const PIPE_CONNECT_TIMEOUT_MS: u32 = 5_000;

pub(super) fn run() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    // Child mode: look for `--child <pipe_name>` anywhere in the args.
    if let Some(pos) = args.iter().position(|a| a == "--child") {
        let pipe_name = args
            .get(pos + 1)
            .expect("--child requires a pipe-name argument");
        return run_child(pipe_name);
    }
    run_parent(&args)
}

// --------------------------------------------------------------------------
// PARENT
// --------------------------------------------------------------------------

fn run_parent(args: &[String]) -> ExitCode {
    // --sddl-template <str>   (default = DEFAULT_SDDL_TEMPLATE)
    let sddl_template = args
        .iter()
        .position(|a| a == "--sddl-template")
        .and_then(|i| args.get(i + 1).cloned())
        .unwrap_or_else(|| DEFAULT_SDDL_TEMPLATE.to_string());

    // Optional --label <str> for pretty-printing which variant is being run.
    let label = args
        .iter()
        .position(|a| a == "--label")
        .and_then(|i| args.get(i + 1).cloned())
        .unwrap_or_else(|| "<unlabeled>".to_string());

    let session_sid = generate_session_sid();
    let sddl = sddl_template.replace("{sid}", &session_sid);
    let nonce = unique_nonce_hex();
    let pipe_name = format!(r"\\.\pipe\nono-pipe-repro-{nonce}");

    println!("===== pipe-repro parent =====");
    println!("label:        {label}");
    println!("session_sid:  {session_sid}");
    println!("sddl:         {sddl}");
    println!("pipe_name:    {pipe_name}");
    println!();

    // Build security attributes from SDDL.
    let sddl_u16 = to_u16_null_terminated(&sddl);
    let mut sd_ptr: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    let ok = unsafe {
        // SAFETY: `sddl_u16` is a valid null-terminated UTF-16 string and
        // `sd_ptr` points to writable stack storage. On success the returned
        // descriptor must be freed via `LocalFree` (done via a guard below).
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl_u16.as_ptr(),
            SDDL_REVISION_1,
            &mut sd_ptr,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 || sd_ptr.is_null() {
        eprintln!(
            "ConvertStringSecurityDescriptorToSecurityDescriptorW failed: {} (err={})",
            std::io::Error::last_os_error(),
            unsafe { GetLastError() }
        );
        return ExitCode::from(10);
    }
    let _sd_guard = SdGuard(sd_ptr);

    let sa = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd_ptr as _,
        bInheritHandle: 0,
    };

    // Create the named pipe.
    let pipe_name_u16 = to_u16_null_terminated(&pipe_name);
    let pipe_handle = unsafe {
        // SAFETY: `pipe_name_u16` is a valid null-terminated UTF-16 string
        // and `sa` carries a live security descriptor for the lifetime of
        // `_sd_guard`.
        CreateNamedPipeW(
            pipe_name_u16.as_ptr(),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            MAX_PIPE_MESSAGE_SIZE,
            MAX_PIPE_MESSAGE_SIZE,
            PIPE_CONNECT_TIMEOUT_MS,
            &sa,
        )
    };
    if pipe_handle == INVALID_HANDLE_VALUE {
        eprintln!(
            "CreateNamedPipeW failed: {} (err={})",
            std::io::Error::last_os_error(),
            unsafe { GetLastError() }
        );
        return ExitCode::from(11);
    }

    // Create the WRITE_RESTRICTED token carrying session_sid as its only
    // restricting SID. Mirrors
    // `nono_cli::exec_strategy_windows::restricted_token::create_restricted_token_with_sid`
    // verbatim.
    let restricted_token = match create_restricted_token_with_sid(&session_sid) {
        Ok(h) => h,
        Err(msg) => {
            eprintln!("create_restricted_token_with_sid failed: {msg}");
            unsafe { CloseHandle(pipe_handle) };
            return ExitCode::from(12);
        }
    };

    // Spawn the child process via CreateProcessAsUserW. Use the current exe
    // with `--child <pipe_name>`.
    let current_exe = env::current_exe().expect("current_exe");
    let child_exit_code = match spawn_child(&current_exe, restricted_token, &pipe_name) {
        Ok(code) => code,
        Err(msg) => {
            eprintln!("spawn_child failed: {msg}");
            unsafe {
                CloseHandle(restricted_token);
                CloseHandle(pipe_handle);
            };
            return ExitCode::from(13);
        }
    };

    // Cleanup.
    unsafe {
        CloseHandle(restricted_token);
        CloseHandle(pipe_handle);
    };

    println!();
    println!("===== child exit code: {child_exit_code} =====");
    println!();
    if child_exit_code == 0 {
        println!("RESULT: CreateFileW succeeded — this SDDL shape ADMITS a WRITE_RESTRICTED child.");
        ExitCode::SUCCESS
    } else if child_exit_code == 1 {
        println!(
            "RESULT: CreateFileW returned ERROR_ACCESS_DENIED — this SDDL shape DOES NOT admit \
             a WRITE_RESTRICTED child with the per-session restricting SID."
        );
        ExitCode::from(1)
    } else {
        println!("RESULT: CreateFileW failed with a non-access-denied error (see child output above).");
        ExitCode::from(2)
    }
}

fn spawn_child(exe: &std::path::Path, h_token: HANDLE, pipe_name: &str) -> Result<u32, String> {
    let application_name = to_u16_null_terminated(&exe.to_string_lossy());
    // CreateProcess takes a MUTABLE command line buffer.
    let cmdline = format!(
        "\"{exe}\" --child {pipe}",
        exe = exe.to_string_lossy(),
        pipe = pipe_name
    );
    let mut cmdline_u16: Vec<u16> = cmdline.encode_utf16().chain(std::iter::once(0)).collect();

    let mut startup: STARTUPINFOW = unsafe { std::mem::zeroed() };
    startup.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    let ok = unsafe {
        // SAFETY: All pointers are valid for the duration of the call.
        // `h_token` is a live restricted token. Environment inherited from parent (null).
        CreateProcessAsUserW(
            h_token,
            application_name.as_ptr(),
            cmdline_u16.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0, // bInheritHandles = FALSE (stdio will still inherit since we don't set hStd*)
            CREATE_UNICODE_ENVIRONMENT,
            std::ptr::null_mut(), // inherit parent environment
            std::ptr::null(),
            &startup,
            &mut pi,
        )
    };
    if ok == 0 {
        return Err(format!(
            "CreateProcessAsUserW failed: {} (err={})",
            std::io::Error::last_os_error(),
            unsafe { GetLastError() }
        ));
    }

    // Wait for the child.
    let wait = unsafe {
        // SAFETY: `pi.hProcess` is a live process handle returned by
        // `CreateProcessAsUserW` above.
        WaitForSingleObject(pi.hProcess, INFINITE)
    };
    if wait != WAIT_OBJECT_0 {
        unsafe {
            CloseHandle(pi.hThread);
            CloseHandle(pi.hProcess);
        };
        return Err(format!("WaitForSingleObject returned {wait}"));
    }

    let mut exit_code: u32 = 0;
    let got = unsafe {
        // SAFETY: `pi.hProcess` is a live process handle; `exit_code` is
        // writable.
        GetExitCodeProcess(pi.hProcess, &mut exit_code)
    };
    unsafe {
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
    };
    if got == 0 {
        return Err(format!(
            "GetExitCodeProcess failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(exit_code)
}

// --------------------------------------------------------------------------
// CHILD
// --------------------------------------------------------------------------

fn run_child(pipe_name: &str) -> ExitCode {
    println!("----- pipe-repro child -----");
    println!("pipe_name: {pipe_name}");
    let pipe_u16 = to_u16_null_terminated(pipe_name);
    let handle = unsafe {
        // SAFETY: `pipe_u16` is a valid null-terminated UTF-16 string. We
        // request duplex access on an existing named pipe.
        CreateFileW(
            pipe_u16.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };
    if handle != INVALID_HANDLE_VALUE {
        println!("CreateFileW: SUCCESS (handle = {handle:?})");
        unsafe { CloseHandle(handle) };
        return ExitCode::SUCCESS;
    }

    let raw = unsafe { GetLastError() };
    let err = std::io::Error::last_os_error();
    println!("CreateFileW: FAILED raw_err={raw} ({err})");
    if raw == ERROR_ACCESS_DENIED {
        ExitCode::from(1)
    } else {
        ExitCode::from(2)
    }
}

// --------------------------------------------------------------------------
// Shared helpers — mirror production exactly
// --------------------------------------------------------------------------

/// Generate a synthetic per-session SID identical in shape to
/// `nono_cli::exec_strategy_windows::restricted_token::generate_session_sid`.
///
/// The production implementation uses `uuid::Uuid::new_v4()`. To avoid
/// pulling `uuid` into this example crate's target-specific deps yet another
/// time, we fabricate a matching shape from high-entropy process-local data
/// (PID, current epoch nanoseconds, wrapping mults). The SID is
/// `S-1-5-117-<u32>-<u32>-<u32>-<u32>` — within SESSION_SID_MAX_LEN and
/// containing only ASCII digits + hyphens, which satisfies
/// `validate_session_sid_for_sddl`.
fn generate_session_sid() -> String {
    let pid: u32 = std::process::id();
    let nanos_u128 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let hi = (nanos_u128 >> 64) as u64;
    let lo = nanos_u128 as u64;
    let a = pid.wrapping_mul(2654435761).wrapping_add(lo as u32);
    let b = (lo >> 32) as u32;
    let c = (lo as u32).wrapping_add(0x9E3779B1);
    let d = hi as u32;
    format!("S-1-5-117-{a}-{b}-{c}-{d}")
}

/// Mirrors
/// `nono_cli::exec_strategy_windows::restricted_token::create_restricted_token_with_sid`
/// verbatim (same `OpenProcessToken` mask, same WRITE_RESTRICTED flag, same
/// `SID_AND_ATTRIBUTES { Attributes: 0 }`, same count=1 restricting SID).
fn create_restricted_token_with_sid(session_sid: &str) -> Result<HANDLE, String> {
    let mut h_current_token: HANDLE = std::ptr::null_mut();
    let ok = unsafe {
        // SAFETY: writes a valid HANDLE or returns 0. We own the handle on
        // success and close it before returning.
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_ASSIGN_PRIMARY,
            &mut h_current_token,
        )
    };
    if ok == 0 {
        return Err(format!(
            "OpenProcessToken failed: {} (err={})",
            std::io::Error::last_os_error(),
            unsafe { GetLastError() }
        ));
    }

    let sid_u16 = to_u16_null_terminated(session_sid);
    let mut sid_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    let ok = unsafe {
        // SAFETY: `sid_u16` is a valid null-terminated UTF-16 string.
        // `sid_ptr` is a writable out-pointer — must be LocalFree'd on success.
        ConvertStringSidToSidW(sid_u16.as_ptr(), &mut sid_ptr)
    };
    if ok == 0 {
        unsafe { CloseHandle(h_current_token) };
        return Err(format!(
            "ConvertStringSidToSidW failed for {session_sid:?}: err={}",
            unsafe { GetLastError() }
        ));
    }

    let sid_restrict = SID_AND_ATTRIBUTES {
        Sid: sid_ptr,
        Attributes: 0,
    };

    let mut h_restricted: HANDLE = std::ptr::null_mut();
    let ok = unsafe {
        // SAFETY: `h_current_token` is a live token; `sid_restrict.Sid` is
        // live until the LocalFree below. `h_restricted` is a writable
        // out-pointer. WRITE_RESTRICTED matches production exactly.
        CreateRestrictedToken(
            h_current_token,
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

    // SAFETY: `sid_ptr` was returned by ConvertStringSidToSidW above; its
    // contents have been copied into the new token by CreateRestrictedToken.
    unsafe { LocalFree(sid_ptr as _) };
    // SAFETY: `h_current_token` is the live token handle opened above.
    unsafe { CloseHandle(h_current_token) };

    if ok == 0 {
        return Err(format!(
            "CreateRestrictedToken failed: err={}",
            unsafe { GetLastError() }
        ));
    }
    if h_restricted.is_null() {
        return Err("CreateRestrictedToken returned NULL handle".to_string());
    }
    Ok(h_restricted)
}

fn to_u16_null_terminated(s: &str) -> Vec<u16> {
    std::ffi::OsString::from(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn unique_nonce_hex() -> String {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{pid:x}-{nanos:x}")
}

struct SdGuard(PSECURITY_DESCRIPTOR);
impl Drop for SdGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: `self.0` was returned by
            // ConvertStringSecurityDescriptorToSecurityDescriptorW and has not
            // been freed by any other path.
            unsafe { LocalFree(self.0 as _) };
        }
    }
}

} // mod windows_impl
