---
phase: 09-wfp-port-level-proxy-filtering
plan: "04"
subsystem: sandbox-windows / execution-runtime
tags: [gap-closure, test-fix, windows, direct-strategy, exit-code]
dependency_graph:
  requires: [09-01]
  provides: [ci-green-lib-tests, windows-direct-exit-code]
  affects: [crates/nono/src/sandbox/windows.rs, crates/nono-cli/src/execution_runtime.rs]
tech_stack:
  added: []
  patterns: [cfg-platform-split, cleanup-before-exit]
key_files:
  modified:
    - crates/nono/src/sandbox/windows.rs
    - crates/nono-cli/src/execution_runtime.rs
decisions:
  - "unreachable!() moved inside #[cfg(not(target_os = \"windows\"))] — semantically correct only on Unix where execvp never returns"
  - "Windows Direct branch mirrors Supervised branch cleanup pattern: cleanup cap file, drop config+loaded_secrets, std::process::exit(exit_code)"
metrics:
  duration: "~15 minutes"
  completed: "2026-04-10"
  tasks: 2
  files: 2
---

# Phase 09 Plan 04: Gap Closure (Stale Test + Windows Direct Exit-Code Panic) Summary

**One-liner:** Replaced stale `expect_err` test with correct post-Phase-09 `Ok(())` assertion, and fixed Windows Direct strategy to propagate child exit code instead of hitting `unreachable!()`.

## What Was Built

Two targeted fixes closing the two gaps identified in 09-VERIFICATION.md that blocked Phase 09 from passing automated CI.

### Gap 1: Stale unit test in `crates/nono/src/sandbox/windows.rs`

**Before:**
```rust
#[test]
fn apply_rejects_unsupported_proxy_with_ports() {
    let mut caps = CapabilitySet::new();
    caps.add_tcp_bind_port(8080);
    let err = apply(&caps).expect_err("port bind allowlist must be rejected");
    assert!(matches!(err, NonoError::UnsupportedPlatform(_)));
}
```

**After:**
```rust
#[test]
fn apply_accepts_port_level_wfp_caps() {
    // Phase 09 removed the PortConnectAllowlist / PortBindAllowlist /
    // LocalhostPortAllowlist unsupported markers from compile_network_policy().
    // apply() now returns Ok(()) for port-populated capability sets on Windows
    // because the unsupported vec stays empty and the guard at line ~50 passes.
    let mut caps = CapabilitySet::new();
    caps.add_tcp_bind_port(8080);
    caps.add_tcp_connect_port(8443);
    apply(&caps).expect("port-level WFP caps must be accepted on Windows");
}
```

Phase 09 plan 01 removed `PortConnectAllowlist`, `PortBindAllowlist`, and `LocalhostPortAllowlist` from the unsupported markers in `compile_network_policy()`. This means `apply()` now correctly returns `Ok(())` for port-populated capability sets. The old test's `expect_err()` call panicked, failing `cargo test --lib -p nono`. The new test covers both bind and connect port types to exercise both allowlist paths.

### Gap 2: Windows Direct strategy exit-code panic in `crates/nono-cli/src/execution_runtime.rs`

**Before:**
```rust
match strategy {
    exec_strategy::ExecStrategy::Direct => {
        #[cfg(target_os = "windows")]
        {
            exec_strategy::execute_direct(&config, Some(flags.session.session_id.as_str()))?;
        }
        #[cfg(not(target_os = "windows"))]
        {
            exec_strategy::execute_direct(&config)?;
        }
        unreachable!("execute_direct only returns on error");
    }
```

**After:**
```rust
    match strategy {
        exec_strategy::ExecStrategy::Direct => {
            #[cfg(target_os = "windows")]
            {
                let exit_code = exec_strategy::execute_direct(
                    &config,
                    Some(flags.session.session_id.as_str()),
                )?;
                cleanup_capability_state_file(&cap_file_path);
                drop(config);
                drop(loaded_secrets);
                std::process::exit(exit_code);
            }
            #[cfg(not(target_os = "windows"))]
            {
                exec_strategy::execute_direct(&config)?;
                unreachable!("execute_direct only returns on error");
            }
        }
```

On Windows, `execute_direct` returns `Ok(i32)` (the child's exit code from `WaitForSingleObject`). The prior code discarded the return value and fell through to `unreachable!()`, causing a panic on every normal child exit. The fix mirrors the existing Supervised branch pattern: capture exit code, clean up capability state file, drop sensitive values, then `std::process::exit(exit_code)`. The `unreachable!()` is now scoped to `#[cfg(not(target_os = "windows"))]` where it remains semantically valid — Unix `execute_direct` calls `execvp` and truly never returns on success.

## Verification Commands Run

```
cargo test --lib -p nono -- apply_accepts_port_level_wfp_caps apply_rejects_unsupported_write_only_directory_grant apply_rejects_capability_expansion_shape apply_rejects_non_default_ipc_mode
```
Result: **4 passed, 0 failed**

```
cargo check -p nono-cli
```
Result: **Finished (no errors)**

```
cargo clippy --lib -p nono -- -D warnings -D clippy::unwrap_used
cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used
```
Result: **Finished (no warnings)**

## Acceptance Criteria Verification

- `apply_rejects_unsupported_proxy_with_ports` — no matches in windows.rs (confirmed)
- `apply_accepts_port_level_wfp_caps` — exactly one match (confirmed)
- New test body contains `caps.add_tcp_bind_port(8080)` AND `caps.add_tcp_connect_port(8443)` (confirmed)
- New test body contains `apply(&caps).expect(` with no `expect_err` (confirmed)
- Adjacent tests `apply_rejects_unsupported_write_only_directory_grant`, `apply_rejects_capability_expansion_shape`, `apply_rejects_non_default_ipc_mode` untouched (confirmed — all 3 pass)
- `unreachable!("execute_direct only returns on error")` appears exactly once, inside `#[cfg(not(target_os = "windows"))]` (confirmed)
- `let exit_code = exec_strategy::execute_direct` appears exactly once in Direct arm (confirmed)
- `cleanup_capability_state_file(&cap_file_path)` followed by `std::process::exit(exit_code)` in Windows block (confirmed)
- `drop(config)` and `drop(loaded_secrets)` appear before `std::process::exit(exit_code)` (confirmed)
- No `.unwrap()` / `.expect()` in production code (confirmed)

## Requirements Status

**PORT-01** and **PROXY-01** automated CI blockers are now closed:
- `cargo test --lib -p nono` no longer panics on the Windows sandbox test module
- `nono wrap` Direct strategy on Windows now exits with the child's actual exit code instead of panicking

Human-verification items from 09-VERIFICATION.md (WFP permit filter E2E, proxy credential injection) remain as noted — they require a Windows host with WFP service running.

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None — no placeholder data or TODOs introduced.

## Threat Flags

None — changes are test-only (Gap 1) and control-flow cleanup (Gap 2); no new network endpoints, auth paths, file access patterns, or schema changes introduced.

## Self-Check: PASSED

- `crates/nono/src/sandbox/windows.rs` modified — confirmed
- `crates/nono-cli/src/execution_runtime.rs` modified — confirmed
- Task 1 commit: 9679d55 — confirmed
- Task 2 commit: 03255f6 — confirmed
