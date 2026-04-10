---
phase: 10-etw-based-learn-command
fixed_at: 2026-04-10T00:00:00Z
review_path: .planning/phases/10-etw-based-learn-command/10-REVIEW.md
iteration: 1
fix_scope: critical_warning
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 10: Code Review Fix Report

**Fixed at:** 2026-04-10
**Source review:** `.planning/phases/10-etw-based-learn-command/10-REVIEW.md`
**Iteration:** 1

## Summary

All 5 in-scope findings (2 Critical, 3 Warning) were fixed across `crates/nono-cli/src/learn_windows.rs`. Each fix was committed atomically. The changes address: event-ID collision documentation and defence-in-depth guard (CR-01), targeted `dead_code` suppression replacing blanket allows (CR-02), enlarged `QueryDosDeviceW` buffer (WR-01), deduplicating network connection accumulator (WR-02), and reordering ETW trace startup to precede child spawn (WR-03).

## Fixes Applied

### CR-01: EVENT_ID_FILE_CREATE and EVENT_ID_TCP_CONNECT collision

**Status:** Fixed
**Commit:** `d6aa601`
**Files modified:** `crates/nono-cli/src/learn_windows.rs`

Two changes applied:

1. Expanded the `EVENT_ID_TCP_CONNECT` constant comment to explicitly document the shared value with `EVENT_ID_FILE_CREATE` (both == 12), note that ferrisetw routes events per-provider so cross-callback contamination does not occur in practice, and instruct the empirical verifier to update the constant if testing shows the ID is wrong.

2. Added a `debug!` log at the start of the file-event callback that emits `provider_guid = ?record.provider_id()` so that the phase-gate human-verification run can confirm no Kernel-Network events bleed into the file callback.

3. Added a defence-in-depth `debug!` log at the start of the network callback that emits the provider GUID before any event-ID matching, providing an empirical audit trail.

---

### CR-02: Broad `#[allow(dead_code)]` violates coding standards

**Status:** Fixed
**Commit:** `a61aaf8`
**Files modified:** `crates/nono-cli/src/learn_windows.rs`

All 8 blanket `#[allow(dead_code)]` attributes were replaced with `#[cfg_attr(not(target_os = "windows"), allow(dead_code))]`. This satisfies the CLAUDE.md prohibition on lazy dead_code suppression: on Windows (where the module is compiled and all items are reachable from `run_learn`), no dead_code suppression is applied. On non-Windows hosts performing cross-compilation analysis, the suppression is narrowly scoped to only the items that legitimately appear unused from a non-Windows compiler's perspective.

Items affected: `LearnState` struct, `LearnState::new`, `on_process_create`, `on_process_exit`, `is_tracked`, `build_volume_map`, `nt_to_win32`, `classify_and_record_file_access`.

---

### WR-01: `QueryDosDeviceW` buffer sized at MAX_PATH — may truncate extended-length NT device names

**Status:** Fixed
**Commit:** `33c7b53`
**Files modified:** `crates/nono-cli/src/learn_windows.rs`

Buffer in `build_volume_map` increased from `vec![0u16; 260]` (MAX_PATH) to `vec![0u16; 1024]`. The SAFETY comment was updated to reflect the new size. The inline comment now explains that MAX_PATH is insufficient for volume junctions and long UNC paths, and that a return of 0 from `QueryDosDeviceW` was previously silently misinterpreted as "not mapped" when it could indicate `ERROR_INSUFFICIENT_BUFFER`.

---

### WR-02: Outbound connections not deduplicated — unbounded Vec growth

**Status:** Fixed: requires human verification
**Commit:** `7792b00`
**Files modified:** `crates/nono-cli/src/learn_windows.rs`

`LearnState` gained two new fields:

- `outbound_counts: HashMap<(IpAddr, u16), usize>` — deduplicating accumulator for outbound TCP connections
- `listening_counts: HashMap<(IpAddr, u16), usize>` — deduplicating accumulator for listening ports

`record_outbound_connection` and `record_listening_port` were updated to increment the appropriate HashMap entry rather than push to a Vec. `LearnState::new` initializes both fields as empty HashMaps.

At result-extraction time in `run_learn`, both HashMaps are converted to `Vec<NetworkConnectionSummary>` via `into_iter().map(...)`. The `count` field in each summary now correctly reflects the number of times that endpoint was observed.

Unit tests were updated to check `state.outbound_counts` and `state.listening_counts` (HashMap) instead of `state.result.outbound_connections` and `state.result.listening_ports` (Vec). Two new deduplication tests were added: `test_record_outbound_deduplicates_same_endpoint` and `test_record_listening_deduplicates_same_port`.

Note: marked "requires human verification" because the HashMap-to-Vec conversion changes observable ordering of entries in `LearnResult.outbound_connections` and `LearnResult.listening_ports`. If any downstream code (JSON output, profile generation) depends on stable ordering, a sort step should be added after the conversion.

---

### WR-03: ETW trace started after child spawned — early file accesses missed

**Status:** Fixed
**Commit:** `661b443`
**Files modified:** `crates/nono-cli/src/learn_windows.rs`

The startup sequence in `run_learn` was reordered to match the Linux/macOS backends:

**Before:** build volume map → spawn child → seed state with child PID → build providers → start trace → wait

**After:** build volume map → initialize state (empty tracked_pids) → build providers → start trace → spawn child → insert child PID → wait

A `new_empty(volume_map)` constructor was added to `LearnState` to allow initialization before the child PID is known. The child PID is inserted via `guard.tracked_pids.insert(child_pid)` immediately after `Command::spawn()` returns, in a dedicated lock scope before the main `child.wait()` call.

The spawn-failure error path was changed from `.map_err()` (which cannot capture `trace` and `trace_thread` by move) to an explicit `match` that calls `trace.stop()` and `trace_thread.join()` before returning the error, avoiding dangling background threads on spawn failure.

The docblock ETW flow sequence was updated to reflect the new ordering.

## Skipped

None — all 5 in-scope findings were successfully fixed.

---

_Fixed: 2026-04-10_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
