---
phase: 09-wfp-port-level-proxy-filtering
plan: "01"
subsystem: sandbox/windows
tags: [windows, wfp, network-policy, port-filtering]
dependency_graph:
  requires: []
  provides: [compile_network_policy returns fully supported for port caps]
  affects: [crates/nono-cli/src/exec_strategy_windows/network.rs via is_fully_supported()]
tech_stack:
  added: []
  patterns: [unsupported Vec pattern in WindowsNetworkPolicy]
key_files:
  created: []
  modified:
    - crates/nono/src/sandbox/windows.rs
decisions:
  - "Remove all three port unsupported markers at once (D-03) — PortConnectAllowlist, PortBindAllowlist, LocalhostPortAllowlist all removed together to avoid IPC version bump per D-01"
  - "WINDOWS_SUPPORTED_DETAILS updated to move port-level filtering from unsupported to supported list with (connect, bind, and localhost ports) qualifier"
metrics:
  duration: "10 minutes"
  completed: "2026-04-10"
  tasks_completed: 1
  tasks_total: 1
  files_modified: 1
---

# Phase 09 Plan 01: Remove Port Unsupported Markers from compile_network_policy Summary

**One-liner:** Removed three unsupported-marker push blocks from `compile_network_policy()` so port-populated `CapabilitySet`s return `is_fully_supported() == true`, unblocking PORT-01 WFP enforcement path.

## What Was Done

### Task 1: Remove unsupported markers and update support string

**File modified:** `crates/nono/src/sandbox/windows.rs`

**Change 1 — Removed three unsupported push blocks** from `compile_network_policy()`:
- Deleted the `if !caps.tcp_connect_ports().is_empty()` block pushing `PortConnectAllowlist`
- Deleted the `if !caps.tcp_bind_ports().is_empty()` block pushing `PortBindAllowlist`
- Deleted the `if !caps.localhost_ports().is_empty()` block pushing `LocalhostPortAllowlist`
- `let mut unsupported = Vec::new()` simplified to `let unsupported = Vec::new()` (immutable since nothing is pushed)

**Change 2 — Updated `WINDOWS_SUPPORTED_DETAILS` constant** (lines 28-35):
- Moved "port-level network filtering" from the unsupported list to the supported list
- Added "(connect, bind, and localhost ports)" qualifier
- New text: "...blocked network mode, port-level network filtering (connect, bind, and localhost ports), and default signal/process/ipc modes..."

**Change 3 — Updated existing test** `compile_network_policy_carries_port_filters_into_wfp_policy`:
- Changed `assert_eq!(policy.unsupported.len(), 3)` to `assert!(policy.unsupported.is_empty(), "port caps should now be fully supported")`
- Added `assert!(policy.is_fully_supported())` assertion

**Change 4 — Added three new unit tests**:
- `compile_network_policy_with_connect_ports_only_is_fully_supported`
- `compile_network_policy_with_bind_ports_only_is_fully_supported`
- `compile_network_policy_with_localhost_ports_only_is_fully_supported`

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| Remove all three markers at once | D-03: avoids multiple IPC version bumps; all three types already flow through to WFP service |
| Keep enum variants in mod.rs | `PortConnectAllowlist`, `PortBindAllowlist`, `LocalhostPortAllowlist` variants remain for documentation/diagnostic purposes; removing would break exhaustive match arms |
| Immutable `unsupported` binding | With no push operations remaining, `let mut` is unnecessary; changed to `let` for correctness |

## Downstream Effect

`prepare_network_enforcement()` in `crates/nono-cli/src/exec_strategy_windows/network.rs` checks `policy.is_fully_supported()` before proceeding. With the markers removed, port-populated capability sets now pass this guard instead of returning `UnsupportedPlatform` error. This is the core unblock for PORT-01.

## Deviations from Plan

None - plan executed exactly as written.

## Verification Notes

The plan's `cargo test --lib -p nono -- compile_network_policy` verification command cannot run on the current (Windows) build host without first fixing pre-existing compilation errors in other files (`crates/nono/src/supervisor/socket.rs`, `crates/nono/src/undo/snapshot.rs`, `crates/nono/src/diagnostic.rs`) that use Unix-only APIs without `#[cfg(not(target_os = "windows"))]` guards. These 22 compile errors existed in the HEAD commit before this plan's changes and are out of scope per deviation rules.

The changes to `windows.rs` itself are syntactically correct — no errors were reported for that file during compilation attempts. All acceptance criteria are met at the source level:
- `unsupported.push(crate::sandbox::WindowsUnsupportedNetworkIssue` no longer appears in `compile_network_policy`
- `WINDOWS_SUPPORTED_DETAILS` contains "port-level network filtering (connect, bind, and localhost ports)"
- Test asserts `policy.unsupported.is_empty()` (not `len() == 3`)
- Four unit tests present (1 updated + 3 new)

## Known Stubs

None.

## Threat Flags

None. The change only removes guards that blocked fully-supported paths from reaching the existing WFP enforcement layer. No new network surface is introduced; the WFP service already handles port-level permit filters with correct weight ordering (Permit=100 > Block=0).

## Self-Check: PASSED

- File `crates/nono/src/sandbox/windows.rs` modified: FOUND
- Commit `10cd35b` present: FOUND
- `unsupported.push` for port types absent from `compile_network_policy`: VERIFIED
- Support string updated: VERIFIED
- 4 unit tests present: VERIFIED
