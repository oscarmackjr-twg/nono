//! Windows ETW-based learn backend for `nono learn`.
//!
//! # ferrisetw Audit Findings (D-01)
//!
//! This module uses ferrisetw as the ETW consumer library. The following audit was
//! performed before any ETW code was written, as required by plan 10-01 / SC4.
//!
//! - Crate: ferrisetw 1.2.0 (crates.io/crates/ferrisetw)
//! - Released: 2024-06-27
//! - License: MIT OR Apache-2.0 (compatible with nono's Apache-2.0 workspace)
//! - Downloads: ~49,500 (adopted, not abandoned)
//! - Repository: github.com/n4r1b/ferrisetw
//! - Unsafe scope: internal only; public API is safe Rust (docs.rs/ferrisetw/1.2.0)
//! - Thread safety: trace types are Send + Sync + Unpin
//! - Dependency footprint: wraps windows-sys (same 0.59 range the project already uses)
//! - Maintenance: June 2024 release after 2023 release — low churn because underlying
//!   ETW consumer API is stable since Windows Vista
//! - Verdict: SUITABLE for adoption. No blockers.
//! - Known sharp edge: Parser::try_parse returns Result; field name mismatches return
//!   Err silently. Callers must `let Ok(x) = ... else { return; }` and never unwrap.

use crate::cli::LearnArgs;
use crate::learn::LearnResult;
use nono::{NonoError, Result};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tracing::{debug, warn};
use windows_sys::Win32::Storage::FileSystem::QueryDosDeviceW;

/// Error message for non-administrator invocations (D-02).
///
/// Must contain:
/// - "nono learn requires administrator privileges"
/// - "Run from an elevated prompt"
/// - "Run as administrator"
const NON_ADMIN_ERROR: &str = "nono learn requires administrator privileges. \
    Run from an elevated prompt (right-click \u{2192} Run as administrator).";

// ---------------------------------------------------------------------------
// LearnState — shared state passed between future ETW callback threads (10-02)
// ---------------------------------------------------------------------------

/// Shared accumulator for ETW-based path and network discovery.
///
/// `tracked_pids` grows as child processes are spawned.
/// `volume_map` is built once at startup via `build_volume_map()`.
///
/// consumed by plan 10-02 ETW consumer
#[allow(dead_code)] // consumed by plan 10-02 ETW consumer
pub(crate) struct LearnState {
    pub tracked_pids: HashSet<u32>,
    pub result: LearnResult,
    pub volume_map: HashMap<String, String>,
}

impl LearnState {
    #[allow(dead_code)] // consumed by plan 10-02 ETW consumer
    pub fn new(root_pid: u32, volume_map: HashMap<String, String>) -> Self {
        let mut tracked_pids = HashSet::new();
        tracked_pids.insert(root_pid);
        Self {
            tracked_pids,
            result: LearnResult::new(),
            volume_map,
        }
    }
}

// ---------------------------------------------------------------------------
// Volume map — maps NT device prefixes to Win32 drive letters
// ---------------------------------------------------------------------------

/// Build a map from NT device paths (e.g. `\\Device\\HarddiskVolume3`) to
/// Win32 drive prefixes (e.g. `C:\\`).
///
/// Iterates over all 26 drive letters and calls `QueryDosDeviceW` to resolve
/// each one's NT device name. Letters not in use return 0 and are silently
/// skipped.
///
/// consumed by plan 10-02 ETW consumer
#[allow(dead_code)] // consumed by plan 10-02 ETW consumer
pub(crate) fn build_volume_map() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:", char::from(letter));
        // Encode drive name as UTF-16 null-terminated
        let drive_wide: Vec<u16> = drive.encode_utf16().chain(std::iter::once(0)).collect();
        let mut buf = vec![0u16; 260]; // MAX_PATH
                                       // SAFETY: drive_wide is a valid null-terminated UTF-16 string for a drive specifier
                                       // of the form "X:". buf is allocated with 260 u16 slots (MAX_PATH). QueryDosDeviceW
                                       // writes at most `buf.len()` UTF-16 code units into buf and returns the count written
                                       // (including the double-null terminator). A return value of 0 means the drive letter
                                       // is not mapped; we skip it. No aliasing occurs — drive_wide and buf are distinct.
        let written =
            unsafe { QueryDosDeviceW(drive_wide.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) };
        if written == 0 {
            // Drive letter not in use — skip silently
            continue;
        }
        // buf contains one or more null-terminated wide strings; take the first.
        // unwrap_or is used instead of unwrap — returns buf.len() if no null found,
        // which safely yields an empty slice that String::from_utf16_lossy handles.
        let first_end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let device_path = String::from_utf16_lossy(&buf[..first_end]);
        if !device_path.is_empty() {
            debug!("volume map: {} -> {}", device_path, drive);
            map.insert(device_path.to_string(), format!("{}\\", drive));
        }
    }
    map
}

// ---------------------------------------------------------------------------
// NT → Win32 path conversion
// ---------------------------------------------------------------------------

/// Convert an NT namespace path (as delivered by ETW) into a Win32 `PathBuf`.
///
/// Returns `None` for:
/// - Named pipes (`\\Device\\NamedPipe\\...`)
/// - Mailslots (`\\Device\\Mailslot\\...`)
/// - UNC redirector / MUP (`\\Device\\Mup\\...`, `\\Device\\LanmanRedirector\\...`)
/// - Any NT path whose device prefix is not present in `volume_map`
///
/// # SECURITY note
///
/// The volume separator is appended (`device_prefix + "\\"`) before calling
/// `strip_prefix`, so `\\Device\\HarddiskVolume3` cannot match the longer
/// `\\Device\\HarddiskVolume30`.  This prevents a path-prefix spoofing attack
/// (T-10-01 in the plan threat register).  String-level `starts_with` is safe
/// here because we control both sides of the comparison and always add the `\\`
/// boundary character.
#[allow(dead_code)] // consumed by plan 10-02 ETW consumer
pub(crate) fn nt_to_win32(nt_path: &str, volume_map: &HashMap<String, String>) -> Option<PathBuf> {
    // Skip well-known non-drive NT namespace prefixes that can never map
    // to a drive letter (named pipes, mailslots, UNC redirector, MUP, etc.)
    const NON_DRIVE_PREFIXES: &[&str] = &[
        "\\Device\\NamedPipe",
        "\\Device\\Mailslot",
        "\\Device\\Mup",
        "\\Device\\LanmanRedirector",
    ];
    for p in NON_DRIVE_PREFIXES {
        if nt_path.starts_with(p) {
            return None;
        }
    }

    // Try each volume. We compare on the device prefix followed by '\\'
    // so "\\Device\\HarddiskVolume3" does NOT match "\\Device\\HarddiskVolume30".
    for (device_prefix, drive_prefix) in volume_map {
        let with_sep = format!("{}\\", device_prefix);
        if let Some(rest) = nt_path.strip_prefix(with_sep.as_str()) {
            return Some(PathBuf::from(format!("{}{}", drive_prefix, rest)));
        }
        // Exact match (path is just the device with no trailing content)
        if nt_path == device_prefix.as_str() {
            return Some(PathBuf::from(drive_prefix));
        }
    }

    warn!("nt_to_win32: no volume mapping for path: {}", nt_path);
    None
}

// ---------------------------------------------------------------------------
// run_learn entry point
// ---------------------------------------------------------------------------

/// Run the Windows ETW-based learn mode.
///
/// Requires administrator privileges (D-02, T-10-05). The admin check runs
/// before any ETW API call so that unprivileged invocations produce a clear
/// actionable error immediately (SC3).
///
/// The ETW consumer loop is implemented in plan 10-02.
pub fn run_learn(_args: &LearnArgs) -> Result<LearnResult> {
    // D-02: admin check MUST happen before any ETW API call
    if !is_admin() {
        return Err(NonoError::LearnError(NON_ADMIN_ERROR.to_string()));
    }
    // Plan 10-02 will replace this with the ETW consumer loop
    Err(NonoError::LearnError(
        "nono learn Windows ETW backend not yet implemented (plan 10-02)".to_string(),
    ))
}

/// Thin seam for test injection — production calls through to exec_strategy.
#[cfg(not(test))]
fn is_admin() -> bool {
    crate::exec_strategy::is_admin_process()
}

#[cfg(test)]
fn is_admin() -> bool {
    tests::TEST_IS_ADMIN.with(|c| c.get())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local! {
        pub(super) static TEST_IS_ADMIN: Cell<bool> = const { Cell::new(true) };
    }

    fn sample_map() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("\\Device\\HarddiskVolume3".to_string(), "C:\\".to_string());
        m.insert("\\Device\\HarddiskVolume4".to_string(), "D:\\".to_string());
        m
    }

    #[test]
    fn test_nt_to_win32_happy_path() {
        let map = sample_map();
        let out = nt_to_win32("\\Device\\HarddiskVolume3\\Users\\test\\file.txt", &map);
        assert_eq!(out, Some(PathBuf::from("C:\\Users\\test\\file.txt")));
    }

    #[test]
    fn test_nt_to_win32_volume_prefix_boundary() {
        // Volume3 must NOT match Volume30
        let mut map = HashMap::new();
        map.insert("\\Device\\HarddiskVolume3".to_string(), "C:\\".to_string());
        let out = nt_to_win32("\\Device\\HarddiskVolume30\\foo", &map);
        assert_eq!(out, None);
    }

    #[test]
    fn test_nt_to_win32_named_pipe_returns_none() {
        let map = sample_map();
        assert_eq!(nt_to_win32("\\Device\\NamedPipe\\foo", &map), None);
        assert_eq!(nt_to_win32("\\Device\\Mup\\server\\share\\x", &map), None);
    }

    #[test]
    fn test_nt_to_win32_unknown_device_returns_none() {
        let map = sample_map();
        assert_eq!(nt_to_win32("\\Device\\Cdrom0\\foo", &map), None);
    }

    #[test]
    fn test_non_admin_returns_learn_error() {
        TEST_IS_ADMIN.with(|c| c.set(false));
        let args = LearnArgs::default_for_test();
        let result = run_learn(&args);
        TEST_IS_ADMIN.with(|c| c.set(true)); // restore for other tests
        match result {
            Err(NonoError::LearnError(msg)) => {
                assert!(
                    msg.contains("nono learn requires administrator privileges"),
                    "msg was: {}",
                    msg
                );
                assert!(
                    msg.contains("Run from an elevated prompt"),
                    "msg was: {}",
                    msg
                );
                assert!(msg.contains("Run as administrator"), "msg was: {}", msg);
            }
            other => panic!("expected LearnError, got {:?}", other),
        }
    }

    #[test]
    fn test_build_volume_map_runs_without_panic() {
        // Sanity: on any Windows host, at least drive C: should map.
        // On non-Windows this test is cfg'd out by the module-level target_os guard.
        let map = build_volume_map();
        // Not asserting contents — just that the call returned a HashMap safely.
        let _ = map.len();
    }
}
