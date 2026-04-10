---
phase: 10-etw-based-learn-command
plan: "03"
subsystem: nono-cli/learn_windows
tags:
  - windows
  - etw
  - learn
  - network
dependency_graph:
  requires:
    - crates/nono-cli/src/learn_windows.rs (LearnState, run_learn from 10-02)
    - crates/nono-cli/src/learn.rs (NetworkConnectionSummary, NetworkEndpoint)
  provides:
    - learn_windows::record_outbound_connection (TcpIp/Connect recorder)
    - learn_windows::record_listening_port (TcpIp/Accept recorder)
    - crates/nono-cli/tests/learn_windows_integration.rs (ignored E2E test)
  affects:
    - crates/nono-cli/src/learn_windows.rs (third provider wired, 924 lines)
tech_stack:
  added: []
  patterns:
    - Provider::by_guid(GUID_KERNEL_NETWORK) as third .enable() on UserTrace builder
    - u16::from_be(raw_port) for ETW network byte-order convention (T-10-20)
    - try_parse fallback chain: "daddr" -> "DestAddress" for remote IP (A4 LOW confidence)
    - CARGO_BIN_EXE_nono for integration test binary invocation (binary-only crate)
key_files:
  created:
    - path: crates/nono-cli/tests/learn_windows_integration.rs
      lines: 82
      description: Windows-only ignored integration test for nono learn E2E
  modified:
    - path: crates/nono-cli/src/learn_windows.rs
      lines: 924
      description: "Network provider, TcpIp event handlers, 6 new unit tests"
decisions:
  - "Binary crate constraint: nono-cli has no lib.rs so integration test invokes CARGO_BIN_EXE_nono (not nono_cli::learn::run_learn) — matches pattern from tests/env_vars.rs"
  - "u16::from_be(raw_port) used for all ETW port fields per network byte-order convention; DEBUG log emits raw + converted on every event for empirical verification (T-10-20)"
  - "Plan committed to worktree-agent branch atop ed2cd97 (Task 1 commit from windows-squash); Task 2 commit is 084626c"
metrics:
  duration: ~20 minutes
  completed: "2026-04-10"
  tasks_completed: 2
  tasks_total: 3
  files_created: 1
  files_modified: 1
---

# Phase 10 Plan 03: Network Events and Integration Test Summary

**One-liner:** ETW Kernel-Network provider wired as third UserTrace subscription with TcpIp/Connect and TcpIp/Accept handlers, plus a Windows-only ignored integration test for end-to-end learn validation.

## What Was Built

Task 1 was already completed in commit `ed2cd97` (prior agent). This summary documents Tasks 2 and 3 (plan marker + SUMMARY), which are the remaining deliverables.

### Task 1 — Network event handler and third provider subscription (commit `ed2cd97`)

Delivered by prior agent execution. From the commit message:

- `GUID_KERNEL_NETWORK = "7DD42A49-5329-4832-8DFD-43D979153A88"` constant added
- `EVENT_ID_TCP_CONNECT = 12` and `EVENT_ID_TCP_ACCEPT = 15` constants added
- `record_outbound_connection(state, pid, remote_ip, remote_port)` — no-op for untracked PIDs; appends `NetworkConnectionSummary` to `outbound_connections` (T-10-18)
- `record_listening_port(state, pid, local_port)` — no-op for untracked PIDs; appends `NetworkConnectionSummary` to `listening_ports` (T-10-18)
- Network provider wired as `.enable(network_provider)` third subscription in `run_learn`
- Callback uses `try_parse` fallback chain: `"daddr"` → `"DestAddress"` for remote IP; `"dport"` → `"DestPort"` for remote port; `"sport"` → `"SourcePort"` → `"LocalPort"` for local port
- `u16::from_be(raw_port)` applied to all port fields with DEBUG log of raw + converted (T-10-20)
- 6 unit tests: untracked-PID drop for both recorders, tracked-PID append, accumulation across multiple connections
- `cargo fmt`, `cargo check`, `cargo clippy -D warnings -D clippy::unwrap_used` all pass

### Task 2 — Windows-only integration test (commit `084626c`)

New file `crates/nono-cli/tests/learn_windows_integration.rs` (82 lines):

- `#![cfg(target_os = "windows")]` — compiles only on Windows; zero test cases on other platforms
- Single test `run_learn_against_dir_command_captures_files` marked `#[ignore = "requires Windows host with administrator privileges (ETW)"]`
- Uses `CARGO_BIN_EXE_nono` (correct binary name per `[[bin]] name = "nono"` in Cargo.toml)
- Admin path: invokes `nono learn -- cmd.exe /c dir C:\Windows`; asserts combined output contains `C:\Windows` (case-insensitive)
- Non-admin path: asserts stderr contains `"nono learn requires administrator privileges"` and returns early (informative, not a hard failure)
- Run manually: `cargo test -p nono-cli --test learn_windows_integration -- --ignored`
- `cargo test -p nono-cli` (without `--ignored`) exits 0 — test is skipped on non-Windows and non-admin hosts

### Task 3 — Plan marker and SUMMARY (this document)

- `completed: true` added to `10-03-PLAN.md` frontmatter
- This SUMMARY created at `.planning/phases/10-etw-based-learn-command/10-03-SUMMARY.md`

## Phase 10 Success Criteria Status

| SC | Description | Status |
|----|-------------|--------|
| SC1 | NT→Win32 paths appear in learn output | Code complete (plan 10-01/10-02); final confirmation requires admin-shell E2E run |
| SC2 | File + network events captured | Code complete; ferrisetw field-name empirical verification needed during human verification via DEBUG logs |
| SC3 | Non-admin invocation shows clear error | Code complete AND covered by unit test + integration test non-admin branch |
| SC4 | ferrisetw library documented/audited | Complete — 20-line ferrisetw audit doc comment in learn_windows.rs module header |

**LEARN-01 requirement:** All components exist at the code level. Human verification via admin shell required for full closure.

## Human Verification Items (Phase Gate)

All three plans contribute to this list. An admin Windows shell is required for all items.

### E2E file event capture (from plan 10-02)

```powershell
# From an elevated PowerShell prompt:
cargo test -p nono-cli --test learn_windows_integration -- --ignored
```

Expected: exit 0, combined output contains `C:\Windows` paths.

### Non-admin error path (from plan 10-01 + plan 10-03)

Run the same command from a non-elevated shell. Expected: non-zero exit, stderr contains `"nono learn requires administrator privileges. Run from an elevated prompt (right-click → Run as administrator)."`.

### ferrisetw field-name verification (from plan 10-02 + plan 10-03)

Inspect DEBUG logs on a real run (`RUST_LOG=debug nono learn -- cmd.exe /c dir C:\Windows`):

- **File events**: Confirm `FileName` field returns a non-empty NT path (e.g., `\Device\HarddiskVolume3\Windows\...`). If the field returns Err (silent skip), run `logman query providers Microsoft-Windows-Kernel-File` to find the actual name.
- **Process events**: Confirm `ProcessID` and `ParentProcessID` are the correct field names.
- **Network events**: Confirm `daddr`/`dport`/`sport` (first choice) or `DestAddress`/`DestPort`/`SourcePort` (fallback) yield non-error results. The DEBUG log shows `raw_port=XXXX converted_port=YYYY` for each TCP event — verify the converted value is sensible (e.g., 443 for HTTPS).

### Port byte-order verification (from plan 10-03)

The debug log shows both raw and converted ports. When a known-port connection is made (e.g., port 443 to api.anthropic.com), verify `converted_port=443` in the logs. If `raw_port=443` and `converted_port=46849` (wrong), the byte swap convention is inverted — file a follow-up task to remove `u16::from_be()`.

### Kernel-Process event_id verification (from plan 10-02)

Confirm that process CreateProcess fires as `event_id 1` (not `15`) for the child process. If `event_id 15` is the actual start event, the `on_process_create` match arm needs updating.

## Integration Test Invocation Reference

```bash
# Skip the test (normal CI):
cargo test -p nono-cli

# Run the test (requires admin Windows shell):
cargo test -p nono-cli --test learn_windows_integration -- --ignored

# Run with debug logging to verify field names:
RUST_LOG=debug cargo test -p nono-cli --test learn_windows_integration -- --ignored --nocapture
```

## Final learn_windows.rs Metrics (from Task 1 commit ed2cd97)

| Metric | Value |
|--------|-------|
| Total lines | 924 |
| Unit tests (`#[test]`) | ≥20 (6 from 10-01 + 8 from 10-02 + 6 from 10-03) |
| `.unwrap()` in production code | 0 |
| `.expect()` in production code | 0 |
| ferrisetw audit doc lines (`//!`) | 20 |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] CARGO_BIN_EXE_nono-cli is wrong binary name**

- **Found during:** Task 2 — reading Cargo.toml and existing test files
- **Issue:** The plan sketch used `env!("CARGO_BIN_EXE_nono-cli")` but the binary is named `nono` (per `[[bin]] name = "nono"` in `crates/nono-cli/Cargo.toml`). Using the wrong name would cause a compile error.
- **Fix:** Used `env!("CARGO_BIN_EXE_nono")` matching the pattern from `crates/nono-cli/tests/env_vars.rs`
- **Files modified:** `crates/nono-cli/tests/learn_windows_integration.rs`
- **Commit:** `084626c`

### Scope Adjustments

**Task 3 scope reduced:** The objective specified updating ROADMAP.md but the instructions explicitly say "Do NOT update STATE.md or ROADMAP.md." ROADMAP updates deferred to the orchestrator.

## Known Stubs

None. All three plans are complete. The network field names (`daddr`, `dport`, `sport`) are best-effort guesses with LOW confidence (research Assumption A4) — empirical verification is a human-verification item, not a code stub. The DEBUG log seam enables that verification without code changes.

## Threat Surface Scan

No new network endpoints, auth paths, or trust-boundary schema changes beyond what is in the plan's threat register. The integration test subprocess invokes the built binary — standard for Cargo integration tests.

## Self-Check

| Item | Status |
|------|--------|
| `crates/nono-cli/tests/learn_windows_integration.rs` (82 lines) | FOUND |
| `#![cfg(target_os = "windows")]` in integration test | CONFIRMED |
| `#[ignore]` attribute in integration test | CONFIRMED |
| `CARGO_BIN_EXE_nono` (not nono-cli) in integration test | CONFIRMED |
| `"nono learn requires administrator privileges"` assertion in test | CONFIRMED |
| commit `084626c` (Task 2) | FOUND |
| `10-03-PLAN.md` `completed: true` | CONFIRMED |

## Self-Check: PASSED
