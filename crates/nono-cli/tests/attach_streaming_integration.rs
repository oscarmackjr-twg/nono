//! Integration test: Phase 17 attach-streaming — Windows detached-path
//! anonymous-pipe stdio produces child stdout that lands in the per-session
//! log file (the always-on path) and is visible to a subsequent
//! `nono attach` client.
//!
//! Requires:
//! - Windows OS (hard cfg gate via `#![cfg(target_os = "windows")]`)
//! - `cargo build -p nono-cli` to have produced the `nono` binary
//!   (auto-handled by cargo test via `env!("CARGO_BIN_EXE_nono")`)
//! - No admin privileges, no nono-wfp-service required
//!
//! Run with:
//!
//!   cargo test -p nono-cli --test attach_streaming_integration -- --ignored
//!
//! This is the automated subset of CONTEXT.md G-02 (bidirectional cmd.exe).
//! G-01 (live ping streaming) and G-03 (detach + reconnect) remain manual
//! smoke-gate items in Plan 17-02.

#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]

use std::process::Command;
use std::time::{Duration, Instant};

const NONO_BIN: &str = env!("CARGO_BIN_EXE_nono");
const SENTINEL: &str = "HELLO_FROM_PHASE17_ATCH_INTEGRATION";

#[test]
#[ignore]
fn detached_child_stdout_reaches_session_log_via_anonymous_pipes() {
    // Step 1: Launch a detached session that prints SENTINEL to stdout and
    // exits. `cmd /c "echo ..."` is a simple, deterministic single-line
    // stdout producer — exactly the shape of CONTEXT.md G-01/G-02
    // reproductions.
    let out = Command::new(NONO_BIN)
        .args([
            "run",
            "--detached",
            "--allow-cwd",
            "--",
            "cmd",
            "/c",
            &format!("echo {SENTINEL}"),
        ])
        .output()
        .expect("failed to invoke nono run --detached");

    assert!(
        out.status.success(),
        "nono run --detached failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // Step 2: Parse the session id from the detached banner. Phase 15 +
    // startup_runtime::print_detached_launch_banner write to STDERR:
    //   "Started detached session <id>."
    //   "Attach with: nono attach <id>"
    let stderr = String::from_utf8_lossy(&out.stderr);
    let session_id = parse_session_id_from_banner(&stderr).unwrap_or_else(|| {
        panic!(
            "could not parse session id from banner. stderr was:\n{stderr}\nstdout was:\n{}",
            String::from_utf8_lossy(&out.stdout)
        )
    });

    // Step 3: Poll `nono logs <id>` until SENTINEL appears or timeout.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last_logs = String::new();
    while Instant::now() < deadline {
        let logs_out = Command::new(NONO_BIN)
            .args(["logs", &session_id])
            .output()
            .expect("failed to invoke nono logs");
        last_logs = String::from_utf8_lossy(&logs_out.stdout).to_string();
        if last_logs.contains(SENTINEL) {
            // Cleanup: best-effort prune of the session record.
            let _ = Command::new(NONO_BIN)
                .args(["prune", "--all-exited"])
                .output();
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // Cleanup before panic for diagnostics.
    let _ = Command::new(NONO_BIN)
        .args(["prune", "--all-exited"])
        .output();
    panic!(
        "SENTINEL '{SENTINEL}' did not appear in nono logs within 5s.\nLast logs:\n{last_logs}\nBanner stderr was:\n{stderr}"
    );
}

/// Extract the session id from the detached banner emitted by
/// `startup_runtime::print_detached_launch_banner`. The banner writes two
/// lines to stderr:
///   "Started detached session <id>."
///   "Attach with: nono attach <id>"
/// Either line yields the same id; we scan for the first match.
fn parse_session_id_from_banner(banner: &str) -> Option<String> {
    for line in banner.lines() {
        for anchor in &["Started detached session ", "Attach with: nono attach "] {
            if let Some(idx) = line.find(anchor) {
                let after = &line[idx + anchor.len()..];
                let token: String = after
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                if !token.is_empty() {
                    return Some(token);
                }
            }
        }
    }
    None
}

#[test]
fn parse_session_id_recognizes_started_banner_line() {
    let banner = "Started detached session abc-123_def.\nAttach with: nono attach abc-123_def\n";
    assert_eq!(
        parse_session_id_from_banner(banner),
        Some("abc-123_def".to_string())
    );
}

#[test]
fn parse_session_id_returns_none_when_no_banner() {
    let banner = "some random log output\nthat has no banner line\n";
    assert_eq!(parse_session_id_from_banner(banner), None);
}
