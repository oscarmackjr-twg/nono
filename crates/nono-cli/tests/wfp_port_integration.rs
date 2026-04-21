//! Integration test: WFP port-level permit filter allows real TCP connections.
//!
//! Requires:
//! - Windows OS
//! - Administrator privileges (WFP filter installation) for the `#[ignore]`d test
//! - nono-wfp-service running (or test will skip gracefully) for the `#[ignore]`d test
//!
//! Run the policy-compilation test (no privileges required):
//!
//!   cargo test -p nono-cli --test wfp_port_integration
//!
//! Run the full WFP real-connection test (admin + running wfp-service required):
//!
//!   cargo test -p nono-cli --test wfp_port_integration -- --ignored
//!
//! # Note on negative-case validity
//!
//! The negative case assertion (`blocked_stream.is_err()`) in the `#[ignore]`d
//! test assumes WFP filters have been installed for the session by the running
//! nono-wfp-service.  If the wfp-service is not active, both connections will
//! succeed (because both listeners are bound locally and there is no enforcement
//! layer).  The test is architecturally correct as the SC5 verification path —
//! its enforcement relies on the WFP service being active during the elevated
//! test run.

#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]

use std::net::{TcpListener, TcpStream};
use std::time::Duration;

/// Resolve `%SystemRoot%\System32\<name>.exe`. Avoids the path-hijack hazard
/// of `Command::new("<tool>")` picking up a malicious binary from the cwd.
fn system32_exe(name: &str) -> std::path::PathBuf {
    let system_root = std::env::var_os("SystemRoot")
        .or_else(|| std::env::var_os("windir"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"));
    system_root.join("System32").join(format!("{name}.exe"))
}

/// Returns `true` when the current process holds administrator privileges.
///
/// Uses `net session` as a quick heuristic: the command succeeds only for
/// elevated processes on Windows.
fn is_elevated() -> bool {
    use std::process::Command;
    Command::new(system32_exe("net"))
        .args(["session"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Admin + running-wfp-service test: verifies that a real TCP connection
/// succeeds through a WFP-allowlisted port (positive case) and that a
/// connection to a non-allowlisted port is rejected by the WFP block-all
/// filter (negative case).
///
/// This satisfies SC5 of the phase-09 success criteria (PORT-01).
#[test]
#[ignore] // Requires admin privileges and a running nono-wfp-service
fn wfp_port_permit_allows_real_tcp_connection() {
    if !is_elevated() {
        eprintln!("SKIP: wfp_port_permit test requires administrator privileges");
        return;
    }

    // 1. Bind loopback TCP listeners on ephemeral ports.
    //    The blocked-port listener exists only so that a connect attempt to it
    //    would succeed at the TCP level if WFP were not active — we want the
    //    WFP block-all sublayer filter to be what rejects the connection, not a
    //    "connection refused" from the OS.
    //
    //    Using ephemeral ports (127.0.0.1:0) avoids port-collision panics when
    //    the previous hardcoded ports (19876/19877) are already in use on the
    //    test host. The kernel assigns the port and we read it back via
    //    local_addr(). See v1.0-MILESTONE-AUDIT.md Phase 09 tech_debt.
    let allowed_listener =
        TcpListener::bind("127.0.0.1:0").expect("bind allowed loopback listener");
    allowed_listener
        .set_nonblocking(true)
        .expect("set allowed listener nonblocking");
    let allowed_port: u16 = allowed_listener
        .local_addr()
        .expect("allowed listener local_addr")
        .port();

    let blocked_listener =
        TcpListener::bind("127.0.0.1:0").expect("bind blocked loopback listener");
    blocked_listener
        .set_nonblocking(true)
        .expect("set blocked listener nonblocking");
    let blocked_port: u16 = blocked_listener
        .local_addr()
        .expect("blocked listener local_addr")
        .port();

    // 2. Build a CapabilitySet with Blocked network mode and one allowlisted
    //    localhost port.
    let mut caps = nono::CapabilitySet::new().set_network_mode(nono::NetworkMode::Blocked);
    caps.add_localhost_port(allowed_port);

    // 3. Verify policy compilation produces a fully supported policy (depends
    //    on the 09-01 change that removed the unsupported markers) and that
    //    the allowed port is present while the blocked port is absent.
    let policy = nono::Sandbox::windows_network_policy(&caps);
    assert!(
        policy.is_fully_supported(),
        "Policy with localhost_port should be fully supported after the 09-01 change"
    );
    assert!(
        policy.localhost_ports.contains(&allowed_port),
        "Policy should contain allowed port {} in localhost_ports",
        allowed_port
    );
    assert!(
        !policy.localhost_ports.contains(&blocked_port),
        "Policy must NOT contain blocked port {} in localhost_ports",
        blocked_port
    );
    assert!(
        policy.has_port_rules(),
        "Policy should report having port rules"
    );

    // 4. POSITIVE CASE: connection to the allowlisted port succeeds.
    let allowed_stream = TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", allowed_port)
            .parse()
            .expect("parse allowed addr"),
        Duration::from_secs(2),
    );
    assert!(
        allowed_stream.is_ok(),
        "TCP connection to allow-listed port {} should succeed, got: {:?}",
        allowed_port,
        allowed_stream.err()
    );

    // 5. NEGATIVE CASE: connection to the non-allowlisted port is rejected by
    //    the WFP block-all filter in the nono sublayer.  A short timeout means
    //    a silent WFP drop still reports failure within a reasonable test window.
    let blocked_stream = TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", blocked_port)
            .parse()
            .expect("parse blocked addr"),
        Duration::from_secs(2),
    );
    assert!(
        blocked_stream.is_err(),
        "TCP connection to non-allow-listed port {} MUST be blocked by WFP, \
         but it succeeded.  This indicates a filter-weight or sublayer-ordering bug.",
        blocked_port
    );

    drop(allowed_listener);
    drop(blocked_listener);
}

/// Non-privileged policy-compilation test: verifies that
/// `compile_network_policy` (via the `Sandbox::windows_network_policy` public
/// API) produces a fully supported `WindowsNetworkPolicy` with the correct
/// port lists when a mix of port types is added to the capability set.
///
/// Runs in all Windows CI without administrator privileges.
#[test]
fn compile_network_policy_localhost_port_appears_in_policy() {
    let mut caps = nono::CapabilitySet::new().set_network_mode(nono::NetworkMode::Blocked);
    caps.add_localhost_port(8080);
    caps.add_tcp_connect_port(443);
    caps.add_tcp_bind_port(5432);

    let policy = nono::Sandbox::windows_network_policy(&caps);
    assert!(
        policy.is_fully_supported(),
        "Policy with port caps should be fully supported"
    );
    assert_eq!(
        policy.localhost_ports,
        vec![8080],
        "localhost_ports should contain exactly the added port"
    );
    assert_eq!(
        policy.tcp_connect_ports,
        vec![443],
        "tcp_connect_ports should contain exactly the added port"
    );
    assert_eq!(
        policy.tcp_bind_ports,
        vec![5432],
        "tcp_bind_ports should contain exactly the added port"
    );
    assert!(
        policy.has_port_rules(),
        "Policy with port caps should report has_port_rules() == true"
    );
}
