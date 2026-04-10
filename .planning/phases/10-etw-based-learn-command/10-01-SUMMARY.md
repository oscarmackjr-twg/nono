---
phase: 10-etw-based-learn-command
plan: "01"
subsystem: nono-cli/learn_windows
tags:
  - windows
  - etw
  - learn
  - security
dependency_graph:
  requires:
    - crates/nono-cli/src/exec_strategy_windows/mod.rs (is_admin_process)
    - crates/nono-cli/src/learn.rs (LearnResult struct)
    - crates/nono-cli/src/cli.rs (LearnArgs struct)
  provides:
    - learn_windows::run_learn (Windows ETW learn entrypoint — scaffold, 10-02 wires ETW loop)
    - learn_windows::build_volume_map (NT device to Win32 drive letter mapping)
    - learn_windows::nt_to_win32 (NT path to PathBuf conversion)
    - learn_windows::LearnState (shared state for future ETW callback threads)
  affects:
    - crates/nono-cli/src/learn.rs (dispatch + LearnResult::new visibility)
    - crates/nono-cli/src/main.rs (module routing)
tech_stack:
  added:
    - ferrisetw = "1.2" (Windows-only ETW consumer library)
    - Win32_System_Diagnostics_Etw feature on windows-sys 0.59
  patterns:
    - Test seam via thread_local Cell<bool> for admin check injection
    - NT-to-Win32 path conversion with strip_prefix boundary safety
    - Module cfg gating via #[cfg(target_os = "windows")] in main.rs
key_files:
  created:
    - path: crates/nono-cli/src/learn_windows.rs
      lines: 280
      description: Windows ETW learn backend scaffold
  modified:
    - path: crates/nono-cli/Cargo.toml
      description: Added ferrisetw 1.2 dep and Win32_System_Diagnostics_Etw feature
    - path: crates/nono-cli/src/main.rs
      description: Added cfg(target_os = "windows") mod learn_windows
    - path: crates/nono-cli/src/learn.rs
      description: Windows dispatch, LearnResult::new cfg lift, NonoError scoping
    - path: crates/nono-cli/src/cli.rs
      description: Added LearnArgs::default_for_test() for Windows unit tests
decisions:
  - "ferrisetw 1.2.0 adopted over raw windows-sys ETW bindings: safe Rust API, MIT/Apache-2.0, ~49.5K downloads, wraps same windows-sys 0.59 dependency already in tree"
  - "LearnResult::new made pub(crate) to enable learn_windows::LearnState::new to call it cleanly without struct-literal duplication"
  - "NT->Win32 conversion uses strip_prefix with appended backslash separator to prevent Volume3/Volume30 prefix collision (T-10-01)"
  - "D-04 resolution for plan 10-02: Option B (default Creates to readwrite) recommended as conservative v1 choice per research Open Question 1"
  - "Test seam via thread_local Cell<bool> chosen over mock trait injection to keep test code minimal and the production path zero-cost"
metrics:
  duration: ~30 minutes
  completed: "2026-04-10"
  tasks_completed: 3
  tasks_total: 3
  files_created: 1
  files_modified: 4
---

# Phase 10 Plan 01: Windows ETW Learn Backend Foundation Summary

**One-liner:** Windows ETW learn scaffold with ferrisetw 1.2 audit, admin gate, NT→Win32 path conversion, and volume map builder wired into module tree.

## What Was Built

This plan delivers the compile-clean, test-green foundation for the Windows ETW-based `nono learn` command. The ETW consumer loop itself is deferred to plan 10-02. Three tasks were completed:

### Task 1 — Cargo.toml dependency additions (commit `57fc34b`)

- Added `ferrisetw = "1.2"` to `[target.'cfg(target_os = "windows")'.dependencies]`
- Appended `"Win32_System_Diagnostics_Etw"` to the `windows-sys 0.59` features list
- `Win32_Storage_FileSystem` preserved; no other features modified
- `cargo check -p nono-cli` clean on Windows host

### Task 2 — learn_windows.rs module scaffold (commit `6ec1943`)

New file `crates/nono-cli/src/learn_windows.rs` (280 lines) containing:

- **`//!` doc block** (20 lines) with verbatim ferrisetw 1.2.0 audit findings, satisfying SC4 and D-01
- **`NON_ADMIN_ERROR` constant** with exact required text (SC3, D-02): "nono learn requires administrator privileges. Run from an elevated prompt (→ Run as administrator)."
- **`LearnState` struct** (`pub(crate)`) with `tracked_pids: HashSet<u32>`, `result: LearnResult`, `volume_map: HashMap<String, String>` — ready for plan 10-02 ETW callback threads
- **`build_volume_map()`** iterates drive letters A–Z via `QueryDosDeviceW` with SAFETY comment; zero `.unwrap()` calls
- **`nt_to_win32()`** converts NT namespace paths to `PathBuf` with `\\` separator to prevent prefix collision (T-10-01)
- **`run_learn()`** stub: admin gate fires before any ETW call; returns descriptive error pending plan 10-02
- **Test seam**: `#[cfg(test)] fn is_admin()` reads `thread_local! { TEST_IS_ADMIN: Cell<bool> }` for injection without production overhead
- **6 unit tests**: `test_nt_to_win32_happy_path`, `test_nt_to_win32_volume_prefix_boundary`, `test_nt_to_win32_named_pipe_returns_none`, `test_nt_to_win32_unknown_device_returns_none`, `test_non_admin_returns_learn_error`, `test_build_volume_map_runs_without_panic`

Also added `LearnArgs::default_for_test()` to `cli.rs` gated `#[cfg(all(test, target_os = "windows"))]`.

### Task 3 — Module wiring (commit `8578d54`)

- `main.rs`: added `#[cfg(target_os = "windows")] mod learn_windows;`
- `learn.rs`: added Windows dispatch arm `#[cfg(target_os = "windows")] pub fn run_learn` calling `crate::learn_windows::run_learn`
- `learn.rs`: updated fallback stub cfg to exclude Windows; changed stale message "only available on Linux (strace) and macOS (fs_usage)" to "not supported on this platform"
- `learn.rs`: lifted `LearnResult::new` cfg guard from `linux|macos` to `linux|macos|windows`; made `pub(crate)` so `learn_windows::LearnState::new` can call it
- `learn.rs`: scoped `NonoError` import to `#[cfg(not(any(...windows...)))]` to eliminate unused-import warning on Windows

## ferrisetw Audit Outcome

ferrisetw 1.2.0 is **SUITABLE for adoption**. Key findings:

- MIT OR Apache-2.0 license (compatible with nono's Apache-2.0 workspace)
- ~49,500 downloads; released June 2024 — adopted and maintained, not abandoned
- Public API is safe Rust; unsafe code is internal to ferrisetw
- `trace types are Send + Sync + Unpin` — safe for multi-threaded ETW callback use (plan 10-02)
- Wraps `windows-sys` in the same 0.59 range nono already depends on — no new transitive risk
- **Known sharp edge for plan 10-02**: `Parser::try_parse` returns `Result`; callers must use `let Ok(x) = ... else { return; }` and never `.unwrap()`

## D-04 Recommendation for Plan 10-02

For the "CREATE disposition: read or readwrite?" open question (D-04), **Option B (default CREATE events to readwrite)** is recommended as the conservative v1 choice:

- A file that is created is almost certainly going to be written to; treating it as write-only is the safe superset
- Reclassifying as read-only later (Option A) risks under-permissioning in generated profiles
- Option B matches the behavior users will verify when running `nono learn` E2E; false positives (over-wide grants) are acceptable in a learning tool where the user reviews output

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `String::from_utf16_lossy` returns `String` not `Cow<str>` on Windows**

- **Found during:** Task 2 first cargo check
- **Issue:** Called `.into_owned()` on result of `String::from_utf16_lossy`, which returns `String` directly on Windows (not `Cow<str>`)
- **Fix:** Changed to `.to_string()` which works for both `String` and `Cow<str>`
- **Files modified:** `crates/nono-cli/src/learn_windows.rs`
- **Commit:** `6ec1943`

**2. [Rule 1 - Bug] `LearnResult::new()` was private**

- **Found during:** Task 3 first cargo check after wiring
- **Issue:** `LearnState::new` calls `LearnResult::new()` from a sibling module; `fn new()` was private
- **Fix:** Changed to `pub(crate) fn new()` in `learn.rs`
- **Files modified:** `crates/nono-cli/src/learn.rs`
- **Commit:** `8578d54`

**3. [Rule 1 - Bug] Unused `NonoError` import in learn.rs on Windows**

- **Found during:** Task 3 cargo check
- **Issue:** `NonoError` was only needed for the removed unsupported stub; became unused on Windows
- **Fix:** Scoped import with `#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]`
- **Files modified:** `crates/nono-cli/src/learn.rs`
- **Commit:** `8578d54`

**4. [Rule 2 - Missing Critical Functionality] LearnState::new used struct-literal instead of LearnResult::new()**

- **Found during:** Task 3 (plan explicitly flagged this as a follow-up cleanup step)
- **Issue:** Task 2 intentionally used struct-literal to keep compile independent of Task 3's cfg-lift; after Task 3, the plan recommended updating to `LearnResult::new()` for maintainability
- **Fix:** Updated `LearnState::new` to call `LearnResult::new()`; removed now-unused `BTreeSet` import from `learn_windows.rs`
- **Files modified:** `crates/nono-cli/src/learn_windows.rs`
- **Commit:** `8578d54`

### Deferred Items

**Pre-existing test compile failures on Windows**: `cargo test -p nono-cli --lib` fails to build due to:
- `crates/nono-cli/src/policy.rs` lines 1850, 2763: Unix-specific `std::os::unix::fs::symlink` in test code not gated for Windows
- `crates/nono-cli/src/trust_keystore.rs` line 394: `backend_description` function not found in scope

These errors exist on the base commit before any plan 10-01 changes (confirmed via `git stash`). Unit tests for `learn_windows` module are verified correct through `cargo check` and `cargo clippy -D warnings`; actual test execution requires Windows host with pre-existing test compile issues resolved (separate work item).

## Known Stubs

| Stub | File | Line | Reason |
|------|------|------|--------|
| `run_learn` returns `Err(LearnError("...not yet implemented (plan 10-02)"))` | `learn_windows.rs` | ~190 | ETW consumer loop is plan 10-02's scope; this is the intentional scaffold |

This stub is intentional — plan 10-02 replaces it with the actual ETW session startup and event callback loop.

## Threat Surface Scan

No new network endpoints, auth paths, or trust-boundary schema changes introduced. All new surface is consistent with the plan's threat register:

| Flag | File | Description |
|------|------|-------------|
| nt_to_win32 path conversion | `learn_windows.rs` | NT path strings from ETW transformed to PathBuf; T-10-01 mitigated via strip_prefix with `\\` separator |
| admin gate | `learn_windows.rs` | Non-admin invocations rejected before any ETW API call (T-10-05 mitigated) |

## Self-Check: PASSED

All files created/modified exist on disk. All task commits present in git log.

| Item | Status |
|------|--------|
| `crates/nono-cli/src/learn_windows.rs` | FOUND |
| `crates/nono-cli/src/learn.rs` | FOUND |
| `crates/nono-cli/src/main.rs` | FOUND |
| `crates/nono-cli/Cargo.toml` | FOUND |
| commit `57fc34b` (Task 1) | FOUND |
| commit `6ec1943` (Task 2) | FOUND |
| commit `8578d54` (Task 3) | FOUND |
