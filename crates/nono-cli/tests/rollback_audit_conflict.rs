//! Integration test: clap rejects `--rollback` paired with `--no-audit`.
//!
//! REQ-POLY-02 acceptance: pairing the two flags must fail at parse time
//! with a clear conflicts_with error and a non-zero exit code.

#![allow(clippy::unwrap_used)]

use std::process::Command;

fn nono_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nono"))
}

#[test]
fn rollback_no_audit_conflict_rejected_at_parse() {
    let output = nono_bin()
        .args(["run", "--rollback", "--no-audit", "--", "echo", "x"])
        .output()
        .expect("nono binary should be present");

    assert!(
        !output.status.success(),
        "expected non-zero exit when --rollback and --no-audit are paired"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stderr, stdout);
    assert!(
        combined.contains("cannot be used with")
            || combined.contains("conflicts with")
            || combined.contains("conflict"),
        "expected clap conflict error, got stderr: {stderr}, stdout: {stdout}"
    );
}

#[test]
fn no_audit_rollback_reverse_order_also_rejected() {
    // clap's conflicts_with is symmetric; verify the reverse order is also rejected.
    let output = nono_bin()
        .args(["run", "--no-audit", "--rollback", "--", "echo", "x"])
        .output()
        .expect("nono binary should be present");

    assert!(
        !output.status.success(),
        "expected non-zero exit when --no-audit and --rollback are paired (reverse order)"
    );
}
