---
phase: 10-etw-based-learn-command
reviewed: 2026-04-10T00:00:00Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/learn.rs
  - crates/nono-cli/src/learn_windows.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/tests/learn_windows_integration.rs
findings:
  critical: 2
  warning: 3
  info: 3
  total: 8
status: issues_found
---

# Phase 10: Code Review Report

**Reviewed:** 2026-04-10
**Depth:** standard
**Files Reviewed:** 6
**Status:** issues_found

## Summary

Phase 10 adds a Windows ETW-based backend for `nono learn`, using the ferrisetw crate to consume Kernel-File, Kernel-Process, and Kernel-Network provider events. The implementation is well-structured: admin gate is first, mutex-poisoning is handled without panicking, and path-prefix boundary attacks are explicitly mitigated. Process-tree tracking, volume-map construction, and NT-to-Win32 path conversion are all correct and well-tested.

Two critical issues were found. The most important is a silent event-collision bug: `EVENT_ID_FILE_CREATE` and `EVENT_ID_TCP_CONNECT` are both defined as `12`. Because both callbacks are registered on the same `UserTrace` session, every Kernel-File Create event (id 12) will also enter the TCP Connect match arm on the network provider callback (or vice versa, depending on which provider delivers the event first). The second critical issue is that `#[allow(dead_code)]` is used broadly on the entire `LearnState` struct and several of its methods; the project's CLAUDE.md explicitly prohibits lazy `dead_code` suppression and requires either removing unused code or writing tests for it. Both items need resolution before merge.

Three warnings cover: an unresolved event-ID collision between `EVENT_ID_FILE_CREATE` and `EVENT_ID_TCP_CONNECT` that the comments acknowledge but do not fix, a `QueryDosDeviceW` buffer that is sized at MAX_PATH (260 code units) when NT device names for volume junctions and long UNC paths can exceed this limit, and a missing deduplication step for outbound network connections (the Vec accumulates duplicate entries for the same remote endpoint).

---

## Critical Issues

### CR-01: EVENT_ID_FILE_CREATE and EVENT_ID_TCP_CONNECT are both 12 — callback collision on shared session

**File:** `crates/nono-cli/src/learn_windows.rs:315-319`

**Issue:** Both constants resolve to the same value:
```rust
const EVENT_ID_FILE_CREATE: u16 = 12;  // line 315
const EVENT_ID_TCP_CONNECT: u16 = 12;  // line 319
```
All three providers (Kernel-File, Kernel-Process, Kernel-Network) are enabled on a single `UserTrace` session (lines 552–557). ETW delivers every event from every provider to every registered callback. The file callback filters on `EVENT_ID_FILE_CREATE == 12`; the network callback matches `EVENT_ID_TCP_CONNECT == 12` in its `match`. When a Kernel-File Create event arrives, it enters both the file callback (correctly) and the network callback's `EVENT_ID_TCP_CONNECT == 12` arm. The network callback will then attempt to parse `daddr`/`DestAddress` fields that do not exist on a file event, fail silently (`try_parse` returns `Err`), and log a spurious "TcpIp/Connect field parse miss" debug line. Conversely, a real TcpIp/Connect event (id 12) from the network provider will also reach the file callback and attempt to parse `FileName` from a network event, also failing silently.

The functional impact is that TCP connect events with id 12 from the Kernel-Network provider are silently dropped by the network callback when there is also a Kernel-File event in flight with the same id — they both arrive but only one source will successfully parse the expected fields. More critically, if the correct TcpIp/Connect id is not 12 (the comment acknowledges this is a "best-known value" that requires empirical verification), any actual connect event will never match. The per-provider callback dispatch means this is not an outright crash, but the silent parse-miss path means network capture may be wholly non-functional.

**Fix:** Separate provider callbacks are isolated by GUID, so the real fix depends on whether event id 12 is actually correct for TcpIp/Connect. Rename the constant to distinguish the two so the collision is immediately visible, and add a provider-GUID guard inside the network callback to verify the event originated from the expected provider before acting on event id:

```rust
// Rename to document they share a value until empirically verified:
const EVENT_ID_FILE_CREATE: u16 = 12;
const EVENT_ID_TCP_CONNECT: u16 = 12;  // NOTE: same as FILE_CREATE — verify empirically

// Inside the network callback, guard on provider GUID:
// ferrisetw EventRecord exposes provider_guid() — check it equals GUID_KERNEL_NETWORK
// before matching on event_id so cross-provider id collisions cannot fire the wrong arm.
```

If empirical verification shows TcpIp/Connect is not id 12, update the constant. Until verified, add a `debug!` log in the file callback that includes the provider GUID so the phase-gate runner can confirm no network events bleed in.

---

### CR-02: Broad `#[allow(dead_code)]` on `LearnState` and its methods violates project coding standards

**File:** `crates/nono-cli/src/learn_windows.rs:58-112`

**Issue:** Eight `#[allow(dead_code)]` attributes are applied to the `LearnState` struct, its `new` constructor, `on_process_create`, `on_process_exit`, `is_tracked`, `build_volume_map`, `nt_to_win32`, and `classify_and_record_file_access`. The `CLAUDE.md` coding standards state: "Avoid `#[allow(dead_code)]`. If code is unused, either remove it or write tests that use it."

These items are not actually dead in the full ETW callback code — all are called from the closures in `run_learn`. However, because the closures capture these functions indirectly and the file is conditionally compiled (`#[cfg(target_os = "windows")]`), the compiler reports them as unused on non-Windows hosts. The appropriate fix is a targeted cfg attribute on the `allow` suppression, not a blanket dead_code allow. The current blanket allows would also mask genuinely unused code that might be added in future.

Additionally, `LearnState::new` and `build_volume_map` both carry `#[allow(dead_code)]` while being called directly from `run_learn` on the same file — this suggests the attributes were added during incremental development and never revisited.

**Fix:** Remove the `#[allow(dead_code)]` attributes from items that are demonstrably called within `run_learn` (they are not dead). For items that are only referenced inside ETW callbacks and thus may appear unused to `rustc` when cross-compiling for non-Windows, use a targeted platform cfg instead:

```rust
// Before the struct definition:
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) struct LearnState { ... }
```

This satisfies the project's prohibition on lazy dead_code suppression while correctly handling cross-platform analysis.

---

## Warnings

### WR-01: `QueryDosDeviceW` buffer sized at MAX_PATH (260) — may truncate extended-length NT device names

**File:** `crates/nono-cli/src/learn_windows.rs:133-141`

**Issue:** The output buffer for `QueryDosDeviceW` is allocated at 260 `u16` slots (the Win32 MAX_PATH). NT device paths for volume mount points, subst drives, and long device names can exceed this. The Windows documentation for `QueryDosDeviceW` states the buffer should be large enough to hold the result; if it is too small the function returns 0 and `GetLastError()` returns `ERROR_INSUFFICIENT_BUFFER`. The current code treats a return of 0 as "drive not in use" and silently skips — meaning a drive whose NT device name exceeds 260 characters would be silently dropped from the volume map. A missing entry causes all ETW file events for that drive to produce "no volume mapping" warnings and those paths go unrecorded.

**Fix:** Increase the buffer to `MAX_PATH * 2` (520) as a conservative baseline, or implement a retry loop that doubles the buffer size when `GetLastError()` returns `ERROR_INSUFFICIENT_BUFFER`:

```rust
let mut buf = vec![0u16; 1024]; // well above practical NT device name length
let written = unsafe { QueryDosDeviceW(drive_wide.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) };
if written == 0 {
    // Check error — distinguish "not mapped" from "buffer too small"
    // For v1 a 1024-slot buffer is sufficient for all known device names.
    continue;
}
```

---

### WR-02: Outbound connections are not deduplicated — high-frequency connections produce unbounded Vec growth

**File:** `crates/nono-cli/src/learn_windows.rs:258-276`

**Issue:** `record_outbound_connection` always appends a new `NetworkConnectionSummary` to `state.result.outbound_connections` with `count: 1`. It never checks whether the same `(addr, port)` pair already exists. A command that makes 1,000 HTTPS requests to the same endpoint will produce a Vec with 1,000 entries, each with `count: 1`, instead of one entry with `count: 1000`. This means:
1. The Vec grows without bound for long-running commands making repetitive connections.
2. `LearnResult::to_json()` and `to_summary()` produce bloated output with duplicate entries.
3. The `count` field on `NetworkConnectionSummary` is documented as a frequency indicator but is always 1, making it meaningless.

**Fix:** Use a `HashMap<(IpAddr, u16), usize>` as the accumulator inside `LearnState`, and convert to a `Vec<NetworkConnectionSummary>` only at extraction time in `run_learn`:

```rust
// In LearnState:
pub outbound_counts: HashMap<(IpAddr, u16), usize>,

// In record_outbound_connection:
*state.outbound_counts.entry((remote_ip, remote_port)).or_insert(0) += 1;

// At result extraction:
let outbound_connections = guard.outbound_counts
    .iter()
    .map(|((addr, port), count)| NetworkConnectionSummary {
        endpoint: NetworkEndpoint { addr: *addr, port: *port, hostname: None },
        count: *count,
    })
    .collect();
```

The same fix applies to `record_listening_port` / `listening_ports`.

---

### WR-03: ETW trace is started after child is spawned — early file accesses during process initialization are missed

**File:** `crates/nono-cli/src/learn_windows.rs:371-557`

**Issue:** The child process is spawned at line 372–378, its PID is captured at line 379, and the ETW `UserTrace` is started at lines 552–568. There is a window between child spawn and trace start during which the child process initializes (loads DLLs, opens the executable and its dependencies, reads config files from AppData). These early accesses — often the most important for understanding what a process requires — are silently missed. For short-lived commands like `echo` this is not consequential, but for any real workload (a Python script, a Node.js application, a Rust binary) the loader activity produces significant filesystem access that will not be captured.

The Linux strace backend avoids this by attaching strace before the child starts (`strace -f -e trace=...`). The macOS `fs_usage` backend is started before the child (see `learn.rs` line 451, where `run_fs_usage_and_nettop` starts `fs_usage` first then spawns the child).

**Fix:** Start the ETW trace session before spawning the child:
```
1. Build volume map
2. Set up providers and start UserTrace
3. Spawn child → record child PID
4. Insert child PID into LearnState
5. Wait for child exit, drain, stop
```
The child PID is not available until after spawn, but `LearnState` can be initialized with an empty `tracked_pids` set and the PID inserted immediately after `child.id()` is obtained, before any substantial child activity begins. The critical change is that the trace session must be running before `Command::spawn()` is called.

---

## Info

### IN-01: Duplicate tracing import for macOS and Linux in learn.rs

**File:** `crates/nono-cli/src/learn.rs:31-33`

**Issue:** The `tracing` imports for `debug`, `info`, `warn` are duplicated across two separate `#[cfg]` blocks that resolve to identical items:
```rust
#[cfg(target_os = "linux")]
use tracing::{debug, info, warn};
#[cfg(target_os = "macos")]
use tracing::{debug, info, warn};
```
These can be collapsed into a single import guarded by `#[cfg(any(target_os = "linux", target_os = "macos"))]`.

**Fix:**
```rust
#[cfg(any(target_os = "linux", target_os = "macos"))]
use tracing::{debug, info, warn};
```

---

### IN-02: Event-ID ambiguity documented in comments but not enforced at the type level

**File:** `crates/nono-cli/src/learn_windows.rs:317-323`

**Issue:** The inline comments acknowledge that `EVENT_ID_TCP_CONNECT` and `EVENT_ID_TCP_ACCEPT` are "best-known values" that need empirical verification. This is a known-unknown that could silently produce an empty network section in the learn output, with no visible signal to the user (parse misses are only logged at `DEBUG`). At minimum, if no TCP events are captured during the drain window, a `warn!` should surface to the user at the `INFO` level so they know network capture may not have worked.

**Fix:** After `run_learn` completes, check whether network activity was expected but empty:
```rust
if result.outbound_connections.is_empty() {
    warn!("learn_windows: no outbound TCP connections captured; \
           verify GUID_KERNEL_NETWORK and event IDs are correct for this Windows version");
}
```

---

### IN-03: Integration test uses `.expect()` on process spawn

**File:** `crates/nono-cli/tests/learn_windows_integration.rs:31`

**Issue:** The integration test calls `.expect("failed to invoke nono binary")` on the result of `Command::new(bin).output()`. The CLAUDE.md policy allows `#[allow(clippy::unwrap_used)]` in test modules, so this does not violate the clippy rule. Noted here for completeness because the project style in non-test code avoids panic-on-failure entirely, and if this test file pattern propagates to non-test code it would be a violation.

No fix required — test usage is explicitly permitted by project policy.

---

_Reviewed: 2026-04-10_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
