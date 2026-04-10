---
phase: 09-wfp-port-level-proxy-filtering
reviewed: 2026-04-10T13:20:49Z
depth: standard
files_reviewed: 3
files_reviewed_list:
  - crates/nono/src/sandbox/windows.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/tests/wfp_port_integration.rs
findings:
  critical: 1
  warning: 2
  info: 1
  total: 4
status: issues_found
---

# Phase 09: Code Review Report

**Reviewed:** 2026-04-10T13:20:49Z
**Depth:** standard
**Files Reviewed:** 3
**Status:** issues_found

## Summary

This phase removes three `unsupported` markers from `compile_network_policy()` to make port-level WFP caps fully supported, adds a pre-flight `ProxyOnly`-without-proxy guard in `execution_runtime.rs`, and introduces a Windows-only integration test for WFP port permit enforcement.

The pre-flight guard in `execution_runtime.rs` is correctly placed and uses an appropriate error variant. The `compile_network_policy` change is structurally sound. However, three issues were found:

1. The Windows `execute_direct` path in `execution_runtime.rs` panics via `unreachable!()` after every successful child exit — the exit code is discarded and the process dies with an unexpected panic message rather than with the child's exit code.
2. An existing test (`apply_rejects_unsupported_proxy_with_ports`) directly contradicts the Phase 09 semantics change and will fail after this phase lands.
3. The integration test's ignored test binds hardcoded TCP ports with `expect()`, which will panic on a port collision rather than skip gracefully.

---

## Critical Issues

### CR-01: Windows Direct strategy panics on normal child exit — exit code is lost

**File:** `crates/nono-cli/src/execution_runtime.rs:286-295`

**Issue:** On Windows, `exec_strategy::execute_direct` returns `Ok(i32)` (child exit code) after the child process exits normally. The call at line 289 uses `?`, which short-circuits on `Err` but simply produces the `i32` value on `Ok` — then execution falls through to `unreachable!("execute_direct only returns on error")` which panics. The child's exit code is discarded and the supervisor process terminates with a panic instead of propagating the correct exit status to the caller.

On Unix, `execute_direct` replaces the process via `exec` and therefore truly never returns on success, making `unreachable!()` correct there. On Windows, the function does `CreateProcess + WaitForSingleObject` and must return the exit code.

**Fix:**
```rust
exec_strategy::ExecStrategy::Direct => {
    #[cfg(target_os = "windows")]
    {
        let exit_code =
            exec_strategy::execute_direct(&config, Some(flags.session.session_id.as_str()))?;
        cleanup_capability_state_file(&cap_file_path);
        std::process::exit(exit_code);
    }
    #[cfg(not(target_os = "windows"))]
    {
        exec_strategy::execute_direct(&config)?;
        unreachable!("execute_direct only returns on error");
    }
}
```

---

## Warnings

### WR-01: Stale test contradicts Phase 09 semantics — will fail after this phase

**File:** `crates/nono/src/sandbox/windows.rs:1429-1434`

**Issue:** The test `apply_rejects_unsupported_proxy_with_ports` was written before Phase 09 and asserts that `add_tcp_bind_port` causes `apply()` to return an `UnsupportedPlatform` error. Phase 09 removes the `unsupported` entry for port-level caps in `compile_network_policy`, so `apply()` now returns `Ok(())` for this input. The test will fail.

```rust
#[test]
fn apply_rejects_unsupported_proxy_with_ports() {
    let mut caps = CapabilitySet::new();
    caps.add_tcp_bind_port(8080);
    let err = apply(&caps).expect_err("port bind allowlist must be rejected"); // <- panics: got Ok
    assert!(matches!(err, NonoError::UnsupportedPlatform(_)));
}
```

**Fix:** Replace the test with one that asserts the new expected behavior:
```rust
#[test]
fn apply_accepts_port_level_wfp_caps() {
    let mut caps = CapabilitySet::new().set_network_mode(NetworkMode::Blocked);
    caps.add_tcp_bind_port(8080);
    caps.add_tcp_connect_port(443);
    caps.add_localhost_port(3000);
    assert!(
        apply(&caps).is_ok(),
        "port-level WFP caps must be accepted after Phase 09 promotion"
    );
}
```

### WR-02: Hardcoded ports in integration test bind with `expect()` — port collision panics instead of skipping

**File:** `crates/nono-cli/tests/wfp_port_integration.rs:61-78`

**Issue:** The ignored `wfp_port_permit_allows_real_tcp_connection` test binds two hardcoded loopback ports (19876 and 19877) using `.expect("bind allowed loopback listener")`. If either port is already in use on the test machine (another test run, another service, a CI parallel job), the `expect()` call panics with an unhelpful message rather than printing a skip notice and returning. The test is already gated by `is_elevated()` for one condition; port availability is a second independent precondition that should be handled equally gracefully.

**Fix:** Use `TcpListener::bind` with `?`-or-skip pattern:
```rust
let allowed_listener = match TcpListener::bind(format!("127.0.0.1:{}", allowed_port)) {
    Ok(l) => l,
    Err(e) => {
        eprintln!("SKIP: could not bind port {allowed_port}: {e}");
        return;
    }
};
let blocked_listener = match TcpListener::bind(format!("127.0.0.1:{}", blocked_port)) {
    Ok(l) => l,
    Err(e) => {
        eprintln!("SKIP: could not bind port {blocked_port}: {e}");
        return;
    }
};
```

---

## Info

### IN-01: `WINDOWS_SUPPORTED_DETAILS` string describes port filtering as supported but doc comment at file top still says "placeholder"

**File:** `crates/nono/src/sandbox/windows.rs:1-4`

**Issue:** The module-level doc comment (lines 1-4) still reads `"Windows sandbox implementation placeholder"`. The `WINDOWS_SUPPORTED_DETAILS` string at lines 27-35 now correctly describes port-level WFP filtering as supported. The module-level comment is misleading for anyone reading the file header and should be updated to match the current state.

**Fix:** Update the module doc comment:
```rust
//! Windows sandbox implementation.
//!
//! Provides filesystem policy compilation, network policy compilation with
//! WFP port-level enforcement, integrity label detection, and path utilities
//! for the Windows sandbox backend.
```

---

_Reviewed: 2026-04-10T13:20:49Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
