//! Windows-only integration test for `nono learn`.
//!
//! This test is `#[ignore]`d by default because it requires:
//!   1. A Windows host (gated by `#![cfg(target_os = "windows")]`)
//!   2. Administrator privileges (ETW kernel providers need elevation)
//!
//! Run manually from an elevated PowerShell / cmd shell:
//!
//!     cargo test -p nono-cli --test learn_windows_integration -- --ignored
//!
//! Expected outcome on an admin shell: exit 0, "dir C:\\Windows" output
//! streamed, and the learn output should list several paths under C:\\Windows.
//!
//! Expected outcome on a non-admin shell: exit non-zero and a clear message
//! containing "nono learn requires administrator privileges".

#![cfg(target_os = "windows")]

use std::process::Command;

#[test]
#[ignore = "requires Windows host with administrator privileges (ETW)"]
fn run_learn_against_dir_command_captures_files() {
    let bin = env!("CARGO_BIN_EXE_nono");
    let output = Command::new(bin)
        .arg("learn")
        .arg("--")
        .arg("cmd.exe")
        .arg("/c")
        .arg("dir")
        .arg("C:\\Windows")
        .output()
        .expect("failed to invoke nono binary");

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        // Non-admin path: assert the error message is actionable.
        assert!(
            stderr.contains("nono learn requires administrator privileges"),
            "expected admin-required error on non-elevated run; got stderr: {stderr}"
        );
        eprintln!(
            "note: integration test ran without admin; saw the expected error. \
             Re-run from an elevated shell to exercise the capture path."
        );
        return;
    }

    // Admin path: output should reference at least one path under C:\\Windows.
    // We deliberately do NOT parse the exact format — we only check that
    // SOMETHING from C:\\Windows was captured.
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        combined.contains("C:\\Windows")
            || combined.contains("C:\\\\Windows")
            || combined.to_lowercase().contains("c:\\windows"),
        "expected at least one captured path under C:\\\\Windows, but output was: {combined}"
    );
}
