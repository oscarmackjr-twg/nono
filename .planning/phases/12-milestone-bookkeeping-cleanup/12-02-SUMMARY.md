---
phase: 12-milestone-bookkeeping-cleanup
plan: 02
subsystem: sandbox/windows, tests/wfp
tags: [tech-debt, docs, tests, windows]
dependency_graph:
  requires: []
  provides:
    - "Accurate module doc on crates/nono/src/sandbox/windows.rs"
    - "Ephemeral-port wfp_port_permit_allows_real_tcp_connection test"
  affects:
    - crates/nono/src/sandbox/windows.rs
    - crates/nono-cli/tests/wfp_port_integration.rs
tech_stack:
  added: []
  patterns:
    - "Ephemeral port binding via TcpListener::bind(\"127.0.0.1:0\") + local_addr()"
key_files:
  created: []
  modified:
    - crates/nono/src/sandbox/windows.rs
    - crates/nono-cli/tests/wfp_port_integration.rs
decisions:
  - "Bundle the pre-existing rustfmt-level test module whitespace fix in windows.rs into the Task 1 commit rather than splitting, since it was already uncommitted on the working branch and is purely cosmetic"
metrics:
  tasks_completed: 2
  duration_minutes: 5
  completed: 2026-04-11
---

# Phase 12 Plan 02: Code Tech-Debt Cleanup Summary

**One-liner:** Removed the stale `placeholder` module doc from `sandbox/windows.rs` and refactored the `#[ignore]`d WFP real-TCP test to bind ephemeral loopback ports instead of hardcoded 19876/19877.

## What Shipped

### Task 1 — Windows sandbox module doc

Replaced the 4-line `WIN-101 placeholder` doc block at the top of `crates/nono/src/sandbox/windows.rs` with an accurate description of the module's actual responsibilities: directory read / read-write capability grants, blocked network mode, port-level WFP filtering (connect, bind, localhost), and supervisor activation via `nono-wfp-service` over named-pipe IPC.

**Verification:**
- `grep -c "placeholder" crates/nono/src/sandbox/windows.rs` → `0`
- `grep -c "//! Windows sandbox implementation\." crates/nono/src/sandbox/windows.rs` → `1`
- `grep -c "port-level WFP filtering" crates/nono/src/sandbox/windows.rs` → `1`
- `cargo check -p nono` → clean

**Commit:** `b30a9c6 docs(12-02): replace stale placeholder module doc in sandbox/windows.rs`

### Task 2 — Ephemeral-port refactor of wfp_port_permit_allows_real_tcp_connection

Replaced hardcoded `let allowed_port: u16 = 19876;` / `let blocked_port: u16 = 19877;` bindings with `TcpListener::bind("127.0.0.1:0")` followed by `.local_addr().port()` reads, so the `#[ignore]`d SC5 real-TCP test no longer panics on port collision when the test host is already using 19876 or 19877. All downstream references to `allowed_port` / `blocked_port` (the `add_localhost_port(allowed_port)` call, `policy.localhost_ports.contains(...)` assertions, and both `TcpStream::connect_timeout` calls) continue to work unchanged because the variable names are preserved.

The second test `compile_network_policy_localhost_port_appears_in_policy` was left untouched — it uses hardcoded ports only in capability-set construction (not in bind calls) and is not a tech-debt item.

**Verification:**
- `grep -c "127.0.0.1:0" crates/nono-cli/tests/wfp_port_integration.rs` → `3` (2 bind calls + 1 comment reference; acceptance wanted `>= 2`)
- `grep -c "let allowed_port: u16 = 19876" crates/nono-cli/tests/wfp_port_integration.rs` → `0`
- `grep -c "let blocked_port: u16 = 19877" crates/nono-cli/tests/wfp_port_integration.rs` → `0`
- `grep -c "\.local_addr()" crates/nono-cli/tests/wfp_port_integration.rs` → `2`
- `grep -c "add_localhost_port(allowed_port)" crates/nono-cli/tests/wfp_port_integration.rs` → `1`
- `cargo check -p nono-cli --tests` → clean
- No bare `.unwrap()` introduced; all fallible calls use `.expect("...")` and the file already carries `#![allow(clippy::unwrap_used)]`.

**Commit:** `0ac3193 refactor(12-02): use ephemeral loopback ports in wfp_port test`

## Deviations from Plan

### Auto-fixed Issues

**1. [Bundled pre-existing diff] rustfmt whitespace fix in windows.rs test module**

- **Found during:** Task 1 initial `git diff` inspection
- **Issue:** The working branch already carried an uncommitted 6-line rustfmt fix in `crates/nono/src/sandbox/windows.rs` around line 1541 (breaking a long `assert!` call across multiple lines). Unrelated to the module doc block at lines 1–4.
- **Fix:** Bundled into the Task 1 commit (`b30a9c6`) rather than splitting it into a separate commit, since it's purely cosmetic and was already part of the working-tree state for this file.
- **Files modified:** `crates/nono/src/sandbox/windows.rs`

No other deviations. Rules 1–3 did not trigger.

## Acceptance Criteria

All success criteria from `12-02-PLAN.md` are met:

- [x] Stale `placeholder` doc removed from `windows.rs` and replaced with an accurate current-state description
- [x] WFP integration test uses ephemeral ports, eliminating port-collision panics
- [x] No build regressions (`cargo check -p nono` and `cargo check -p nono-cli --tests` both clean)
- [x] `#![cfg(target_os = "windows")]` gate and SC5 positive/negative assertion structure preserved
- [x] No bare `.unwrap()` introduced; `.expect("...")` used throughout

## Self-Check: PASSED

- FOUND: `crates/nono/src/sandbox/windows.rs` (modified)
- FOUND: `crates/nono-cli/tests/wfp_port_integration.rs` (modified)
- FOUND commit: `b30a9c6` docs(12-02): replace stale placeholder module doc in sandbox/windows.rs
- FOUND commit: `0ac3193` refactor(12-02): use ephemeral loopback ports in wfp_port test
