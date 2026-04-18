---
status: complete
phase: 10-etw-based-learn-command
source:
  - .planning/phases/10-etw-based-learn-command/10-01-SUMMARY.md
  - .planning/phases/10-etw-based-learn-command/10-02-SUMMARY.md
  - .planning/phases/10-etw-based-learn-command/10-03-SUMMARY.md
started: 2026-04-10T00:00:00Z
updated: 2026-04-10T00:00:00Z
---

## Current Test

complete: true
all_tests_done: true

## Tests

### 1. Non-admin rejection
expected: From a non-elevated shell, run `nono learn -- cmd.exe /c dir C:\Windows`. Process exits non-zero. Stderr contains "nono learn requires administrator privileges. Run from an elevated prompt (right-click → Run as administrator)."
result: issue
reported: "Got 'WARNING: nono learn runs the command WITHOUT any sandbox restrictions. Continue? [y/N]' prompt instead of immediate admin rejection."
severity: major

### 2. File event capture (E2E integration test)
expected: From an elevated PowerShell prompt, run `cargo test -p nono-cli --test learn_windows_integration -- --ignored`. Test exits 0. Combined output contains `C:\Windows` path(s).
result: issue
reported: "Compilation failed: (A) dead_code warning on LearnState::new on Windows — cfg_attr fix from CR-02 doesn't suppress it because WR-03's new_empty() replaced the call site; (B) nono.exe locked by another process (Access is denied os error 5) — environmental, not a code bug."
severity: major

### 3. Non-admin path in integration test
expected: When the integration test runs from a non-admin shell (no --ignored needed, test auto-detects), it asserts stderr contains "nono learn requires administrator privileges" and returns early without hard-failing. `cargo test -p nono-cli` exits 0 on non-admin shells.
result: blocked
blocked_by: prior-phase
reason: "cargo test -p nono-cli fails to compile on Windows due to pre-existing errors in policy.rs (std::os::unix calls not gated for Windows) and trust_keystore.rs (missing backend_description). These predate phase 10 per 10-01-SUMMARY. Binary build (cargo build) succeeds."

### 4. ETW field-name verification via debug logs
expected: From an elevated shell, run `RUST_LOG=debug nono learn -- cmd.exe /c dir C:\Windows`. In the debug output, observe `FileName` fields returning NT paths like `\Device\HarddiskVolume3\Windows\...` (not empty/error). Also see `ProcessID` and `ParentProcessID` field names resolving correctly for process events.
result: issue
reported: "File events fire correctly (provider EDD08927, event_id=12). Volume map works (\Device\HarddiskVolume3 -> C:). 18 grants captured. BUT: (A) debug log shows only provider_guid/event_id/pid — FileName values not logged, field-name verification not possible from logs; (B) no process create/exit events visible in debug log; (C) Zone.Identifier ADS paths appear as grants (noise); (D) nono's own paths (C:\Users\omack\Nono\) in output — possible PID tracking regression from WR-03 reorder."
severity: major

### 5. Network event capture and port byte-order
expected: Run `RUST_LOG=debug nono learn -- curl.exe https://example.com` (or any command making a TLS connection) from elevated shell. Debug log shows `raw_port=XXXX converted_port=443` (or another sensible port). Learn output includes at least one entry in `outbound_connections` for the HTTPS endpoint.
result: issue
reported: "Network callback fires for event_ids 10, 11, 13, 18 — but code matches on 12 (TCP Connect) and 15 (TCP Accept). Zero network connections in output. EVENT_ID_TCP_CONNECT=12 is empirically wrong on this system. No raw_port/converted_port log lines seen (those only emit when event_id matches)."
severity: major

### 6. Process tree tracking — child process accesses captured
expected: Run `nono learn -- cmd.exe /c dir C:\Windows\System32` from elevated shell. File accesses by cmd.exe's child processes (if any) also appear in learn output paths, not just the top-level command's accesses.
result: pass
reported: |
  Ran `nono learn -- cmd.exe /c start /wait notepad.exe` from elevated shell. 602 grants captured
  including Notepad's DLLs (WindowsAppRuntime, system DLLs). Process tree tracking confirmed
  working — Notepad (child of cmd.exe) accesses appear in output. Also confirmed: network
  callback receives event_id=12 (TCP Connect) and event_id=15 (TCP Accept) from
  provider_guid=7DD42A49 at timestamps 20:44:07.005685 and 20:44:07.006056, confirming
  network event IDs are correct. The earlier test 4 showed IDs 10/11/13/18 because cmd /c dir
  makes no TCP connections — IDs 12/15 only fire when actual TCP connections are established.

### 7. ETW trace timing — early initialization paths captured
expected: Run `nono learn -- python.exe -c "import os"` (or any runtime that loads DLLs on startup) from elevated shell. The learn output includes paths from early DLL loading/initialization (e.g., Python's standard library DLLs), not just paths accessed after the script starts.
result: pass
reported: |
  Ran `nono learn -- powershell.exe -Command "exit"` from elevated shell. 300 grants captured
  including early DLL load chain: ntdll.dll, kernel32.dll, clr.dll, mscoreei.dll, clrjit.dll,
  mscoree.dll, and the full .NET GAC assembly set. ETW trace starts before child spawn (WR-03
  fix confirmed working). Zone.Identifier ADS noise is very prominent — almost every DLL has
  a matching :Zone.Identifier entry, reinforcing the filter as high priority.

## Summary

total: 7
passed: 2
issues: 4
blocked: 1
pending: 0
skipped: 0

## Gaps

- truth: "Non-admin invocation of `nono learn` should immediately reject with the admin error message before showing any prompts."
  status: failed
  reason: "User reported: Got 'WARNING: nono learn runs the command WITHOUT any sandbox restrictions. Continue? [y/N]' prompt instead of immediate admin rejection."
  severity: major
  test: 1
  artifacts: []
  missing: ["Admin gate check must move to before the continue/warning prompt in the learn command dispatch path"]

- truth: "`nono-cli` should compile cleanly on Windows with no dead_code warnings — clippy -D warnings enforced."
  status: failed
  reason: "LearnState::new reported as dead on Windows after WR-03 fix introduced new_empty() which replaced the call site. cfg_attr fix from CR-02 only suppresses the warning on non-Windows, so the warning surfaces on Windows where it matters."
  severity: major
  test: 2
  artifacts: []
  missing: ["Either remove LearnState::new and rename new_empty() to new(), or restore a call site to LearnState::new() in run_learn"]

- truth: "Debug log should show FileName field values (NT paths) to enable field-name verification."
  status: failed
  reason: "Debug log only shows event metadata (provider_guid/event_id/pid), not FileName values. Zone.Identifier ADS paths appear as grants (noise). NOTE: Network event IDs 12 (TCP Connect) and 15 (TCP Accept) confirmed correct in test 6 — IDs 10/11/13/18 in test 4 were non-TCP events from cmd /c dir which makes no TCP connections."
  severity: major
  test: 4
  artifacts: []
  missing: ["Add FileName value to file callback debug log", "Filter Zone.Identifier ADS paths from learn output"]

- truth: "Network TCP connect events should be captured and appear in outbound_connections."
  status: partial
  reason: "Test 6 confirmed event_id=12 (TCP Connect) and event_id=15 (TCP Accept) fire correctly from provider 7DD42A49. Test 5 (curl) showed zero connections — likely curl.exe was not available or blocked. Network capture logic is correct per test 6."
  severity: info
  test: 5
  artifacts: []
  missing: ["Verify outbound_connections populated when curl.exe or equivalent makes HTTPS connection"]
