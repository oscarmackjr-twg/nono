# Phase 10: ETW-Based Learn Command - Research

**Researched:** 2026-04-10
**Domain:** Windows ETW consumer API, ferrisetw crate, NT-to-Win32 path conversion, process tree tracking
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-01: ETW Library — ferrisetw**
Use the `ferrisetw` crate (n4r1b/ferrisetw) for ETW consumer/provider integration. Higher-level Rust API handles ETW session lifecycle, callback dispatch, and schema decoding. Direct `windows-sys` ETW bindings would require ~200–300 lines of unsafe boilerplate. Prerequisite: audit ferrisetw before committing (activity, soundness, license). Document audit outcome in the plan.

**D-02: Admin Privilege — Upfront Check with Runas Hint**
Call privilege check before touching any ETW API. If not elevated: print actionable error to stderr including "nono learn requires administrator privileges. Run from an elevated prompt (right-click → Run as administrator)." Exit non-zero. Do NOT attempt ETW setup and rely on `ACCESS_DENIED`.

**D-03: Process Tree Scope — Full Tree**
Track direct child PID plus all descendants. Subscribe to `Microsoft-Windows-Kernel-Process` provider events (`CreateProcess`, `ExitProcess`) to maintain a live set of tracked PIDs.

**D-04: FileIo Event → Access Mode Mapping**
Use `DesiredAccess` field from `FileIo/Create` ETW events to classify file access (read/write/readwrite).

**D-05: NT Namespace → Win32 Path Conversion**
Build a volume map at startup by calling `QueryDosDevice` for each drive letter (A–Z). Apply at event-processing time.

### Claude's Discretion

- NT → Win32 path conversion mechanics (QueryDosDevice volume map, built at startup)
- ferrisetw session naming convention (`nono-learn-{pid}` or similar)
- Event buffer sizing and timeout for the ETW consumer loop
- Which `Microsoft-Windows-Kernel-Network` event subtypes to capture (TcpIp/Connect, TcpIp/Accept minimum)
- How to handle ETW events received after the child process exits (drain with a brief timeout)

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LEARN-01 | User can run `nono learn <cmd>` on Windows to capture file and network access patterns via ETW; output format matches Unix learn format so existing profile tooling works unchanged; running without admin privilege produces a clear error rather than silent empty output. | ferrisetw 1.2.0 API verified; `is_admin_process()` reuse confirmed; `LearnResult` struct interface confirmed; NT-to-Win32 path conversion via `QueryDosDevice` (already in `Win32_Storage_FileSystem` feature) confirmed. |
</phase_requirements>

---

## Summary

Phase 10 adds a Windows ETW backend to `nono learn <cmd>`. The implementation subscribes to three kernel ETW providers via the `ferrisetw` crate, filters events to a tracked process tree, converts NT namespace paths to Win32 format, and populates the existing `LearnResult` struct that Unix backends already use.

The research confirms the locked decisions are implementable. Two areas need planner attention:

1. **D-04 field name discrepancy:** The `Microsoft-Windows-Kernel-File` provider's Create event (ID 12) exposes `CreateOptions` and `ShareAccess`, but NOT `DesiredAccess`. The `DesiredAccess` field exists only in the older MOF-based `FileIo_Create` event from the legacy `NT Kernel Logger` session (GUID `{9A62927E-...}`), not in the modern manifest-based `Microsoft-Windows-Kernel-File` provider (GUID `EDD08927-...`). The planner must resolve this: either (a) use `CreateOptions` flags (`FILE_GENERIC_READ`, `FILE_GENERIC_WRITE`, `DELETE`) for the same classification, or (b) subscribe to the legacy MOF provider instead. Option (a) is preferred.

2. **ferrisetw audit result:** Version 1.2.0, released 2024-06-27, MIT OR Apache-2.0 license, ~50K downloads. Active enough for adoption. Audit findings documented below.

**Primary recommendation:** Add `learn_windows.rs` as a new platform module following the `session_commands_windows.rs` routing pattern. Add `ferrisetw = "1.2"` under `[target.'cfg(windows)'.dependencies]`. Add `Win32_System_Diagnostics_Etw` feature to `windows-sys`. Wire into the existing `run_learn` unsupported stub via `#[cfg(target_os = "windows")]`.

---

## ferrisetw Library Audit

**Required by D-01 before ETW code is written.**

| Property | Finding | Source |
|----------|---------|--------|
| Latest version | 1.2.0 | [VERIFIED: crates.io] |
| Release date | 2024-06-27 | [VERIFIED: crates.io] |
| License | MIT OR Apache-2.0 | [VERIFIED: crates.io] — compatible with nono |
| Total downloads | ~49,500 | [VERIFIED: crates.io] |
| Repository | github.com/n4r1b/ferrisetw | [VERIFIED: crates.io] |
| Unsafe scope | Internal only; public API is safe Rust | [CITED: docs.rs/ferrisetw/1.2.0] |
| Thread safety | `Send + Sync + Unpin` on trace types | [CITED: docs.rs/ferrisetw/1.2.0] |
| Maintenance | Last release June 2024; prior release 2023 | [VERIFIED: crates.io] — low churn on stable API |
| Dependencies | `windows-sys` (same version range) | [ASSUMED] — verify in Cargo.lock post-add |

**Audit verdict:** Suitable for adoption. License is compatible. Public API is safe Rust. Infrequent releases reflect API stability, not abandonment. The crate wraps ETW consumer APIs that have not changed since Windows Vista.

**Risk items to note in plan:** ferrisetw's `Parser::try_parse` returns `Result`; field name mismatches silently return `Err`, not panic. Implement graceful skip (log at DEBUG, continue) for unparseable events rather than propagating errors.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ferrisetw | 1.2.0 | ETW consumer session, provider subscription, event field parsing | Locked (D-01); safe Rust over raw ETW Win32 API |
| windows-sys | 0.59 (existing) | `QueryDosDevice`, `Win32_System_Diagnostics_Etw` (new feature) | Already in Cargo.toml; extends existing dep |

### Supporting (all existing in codebase)
| Library | Purpose | When to Use |
|---------|---------|-------------|
| `crate::exec_strategy::is_admin_process()` | Privilege check (D-02) | Reuse — function already in `exec_strategy_windows/mod.rs:353` |
| `nono::NonoError::LearnError` | Error variant for learn failures | Consistent with Unix backends |
| `std::sync::{Arc, Mutex}` | Shared ETW event state across callback thread | ETW callbacks run on a ferrisetw-spawned thread |

### Alternatives Considered (D-01 already locked)
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| ferrisetw 1.2.0 | Direct windows-sys ETW bindings | ~200-300 lines extra unsafe boilerplate; no benefit |

**Installation (Cargo.toml additions):**
```toml
[target.'cfg(target_os = "windows")'.dependencies]
ferrisetw = "1.2"

# Extend existing windows-sys entry to add ETW feature:
windows-sys = { version = "0.59", features = [
  # ... existing features ...
  "Win32_System_Diagnostics_Etw",   # NEW for Phase 10
] }
```

`Win32_Storage_FileSystem` (needed for `QueryDosDeviceW`) is already present in the existing features list. [VERIFIED: crates/nono-cli/Cargo.toml line 90]

---

## Architecture Patterns

### Recommended Module Structure

The Windows learn backend follows the same routing pattern as other Windows-specific modules:

**In `main.rs`:**
```rust
// Existing (no change needed — learn.rs already has the platform stub):
mod learn;
mod learn_runtime;
// Add Windows-specific backend:
#[cfg(target_os = "windows")]
mod learn_windows;
```

**New file:** `crates/nono-cli/src/learn_windows.rs`

**In `learn.rs`**, replace the unsupported stub (lines ~387–392):
```rust
// BEFORE:
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn run_learn(_args: &LearnArgs) -> Result<LearnResult> {
    Err(NonoError::LearnError(
        "nono learn is only available on Linux (strace) and macOS (fs_usage)".to_string(),
    ))
}

// AFTER:
#[cfg(target_os = "windows")]
pub fn run_learn(args: &LearnArgs) -> Result<LearnResult> {
    crate::learn_windows::run_learn(args)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn run_learn(_args: &LearnArgs) -> Result<LearnResult> {
    Err(NonoError::LearnError(
        "nono learn is not supported on this platform".to_string(),
    ))
}
```

### Pattern 1: ferrisetw Consumer Session

**What:** Build a `UserTrace` (for manifest-based providers) or fall back to `KernelTrace` for kernel providers. The `Microsoft-Windows-Kernel-File`, `Microsoft-Windows-Kernel-Network`, and `Microsoft-Windows-Kernel-Process` providers are all manifest-based on Windows 8+; use `UserTrace`.

**Session lifecycle:**
```rust
// Source: [CITED: docs.rs/ferrisetw/1.2.0/ferrisetw/trace/struct.UserTrace.html]
use ferrisetw::prelude::*;

let shared_state: Arc<Mutex<LearnState>> = Arc::new(Mutex::new(LearnState::new()));
let state_clone = shared_state.clone();

let file_provider = Provider::by_guid("EDD08927-9CC4-4E65-B970-C2560FB5C289")
    .add_callback(move |record: &EventRecord, schema_locator: &SchemaLocator| {
        handle_file_event(record, schema_locator, &state_clone);
    })
    .build();

// Session name must be unique per concurrent session; use PID to avoid conflicts
let session_name = format!("nono-learn-{}", std::process::id());

let trace = UserTrace::new()
    .named(session_name)
    .enable(file_provider)
    // .enable(network_provider)  -- add additional providers
    // .enable(process_provider)
    .start()?
    .process()?;  // Blocking until stopped
```

**Stopping:** Call `trace.stop()` or drop the trace handle. Use a thread + channel: spawn the ETW consumer on a background thread, signal it to stop via a `std::sync::atomic::AtomicBool` or a `std::sync::mpsc::channel` after the child process exits.

### Pattern 2: Event Field Parsing

```rust
// Source: [CITED: docs.rs/ferrisetw/1.2.0/ferrisetw/]
fn handle_file_event(
    record: &EventRecord,
    schema_locator: &SchemaLocator,
    state: &Arc<Mutex<LearnState>>,
) {
    let Ok(schema) = schema_locator.event_schema(record) else { return };
    let parser = Parser::create(record, &schema);

    // Filter by tracked PIDs (D-03)
    let pid = record.process_id();
    if !state.lock().unwrap().tracked_pids.contains(&pid) {
        return;
    }

    // FileIo/Create event ID = 12
    if record.event_id() == 12 {
        // CRITICAL NOTE (D-04): Modern Kernel-File provider has CreateOptions,
        // NOT DesiredAccess. See D-04 field name discrepancy section.
        let Ok(path): Result<String, _> = parser.try_parse("FileName") else { return };
        // ... classify access from CreateOptions
    }
}
```

### Pattern 3: NT Namespace to Win32 Path Conversion (D-05)

Build volume map at startup before spawning child process:

```rust
// Source: [VERIFIED: docs.rs/windows-sys/latest — QueryDosDeviceW in Win32_Storage_FileSystem]
fn build_volume_map() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:", char::from(letter));
        // QueryDosDeviceW(lpDeviceName: drive, lpTargetPath: buf, ucchMax: MAX_PATH)
        // Returns device path like \Device\HarddiskVolume3
        // ... unsafe call ...
        map.insert(device_path, format!("{}\\", drive));
    }
    map
}

fn nt_to_win32(nt_path: &str, volume_map: &HashMap<String, String>) -> Option<PathBuf> {
    for (device_prefix, drive_prefix) in volume_map {
        if let Some(rest) = nt_path.strip_prefix(device_prefix.as_str()) {
            return Some(PathBuf::from(format!("{}{}", drive_prefix, rest)));
        }
    }
    None  // UNC paths, named pipes, etc. — skip these
}
```

**CRITICAL:** Use `Path::strip_prefix()` semantics (component-based), not string `starts_with()`. The volume map keys are full NT device paths like `\Device\HarddiskVolume3`, which do not share prefixes with each other in practice, but using the `Path` type prevents the CLAUDE.md footgun ("String comparison for paths is a vulnerability").

### Pattern 4: Process Tree Tracking (D-03)

Subscribe to `Microsoft-Windows-Kernel-Process` (GUID `22FB2CD6-0E7B-422B-A0C7-2FAD1FD0E716`). On `CreateProcess` event, if `ParentId` is in the tracked set, add `ProcessId` to the tracked set. On `ExitProcess`, remove `ProcessId`.

```rust
// Shared mutable set of tracked PIDs; updated from process provider callback,
// read from file/network provider callbacks.
struct LearnState {
    tracked_pids: HashSet<u32>,
    result: LearnResult,
    volume_map: HashMap<String, String>,
}
```

All three provider callbacks share this struct via `Arc<Mutex<LearnState>>`.

### Anti-Patterns to Avoid

- **Blocking the main thread on ETW process():** ferrisetw's `process()` is blocking. Always run it on a dedicated background thread and communicate completion via a channel or AtomicBool.
- **String path comparison:** Never use `path_str.starts_with("\\Device\\HarddiskVolume3")` — use `Path::starts_with()` on the normalized path. [CLAUDE.md footgun]
- **Ignoring `try_parse` errors:** These are common (schema version mismatches, wrong event ID branch). Always `let Ok(x) = ... else { return; }` — never `unwrap()`. [CLAUDE.md unwrap policy]
- **Using `expect()` on the Mutex lock:** Tests run in parallel; a poisoned mutex from another test failing would propagate. Use `lock().unwrap_or_else(|e| e.into_inner())` in tests, and consider returning `NonoError` from callbacks if lock fails.
- **Session name collision:** Multiple concurrent `nono learn` invocations will conflict on the same session name. Use `nono-learn-{pid}` to make sessions unique per process.

---

## D-04 Field Name Discrepancy (CRITICAL — Planner Must Resolve)

**Decision D-04** in CONTEXT.md reads: "Use the `DesiredAccess` field from `FileIo/Create` ETW events to classify each file access."

**Research finding:** The `Microsoft-Windows-Kernel-File` provider (GUID `EDD08927-...`, the provider specified in CONTEXT.md) does NOT have a `DesiredAccess` field in its Create event. [VERIFIED via Microsoft Tx repository (Microsoft_Windows_Kernel_File.cs) and MS Learn FileIo_Create documentation]

The Create event (ID 12) fields are:
- `Irp` / `IrpPtr`
- `FileObject`
- `ThreadId` / `IssuingThreadId`
- `CreateOptions` — maps to `NtCreateFile` *CreateOptions* AND *CreateDispositions* parameters
- `CreateAttributes` (V0 only) / `FileAttributes`
- `ShareAccess`
- `FileName` — the NT namespace path

`DesiredAccess` is a field in the **legacy MOF-based** `FileIo_Create` class (used with the old NT Kernel Logger session, not the modern `Microsoft-Windows-Kernel-File` manifest provider).

**Resolution options:**

| Option | Approach | Tradeoff |
|--------|----------|----------|
| **A (recommended)** | Use `CreateOptions` flags for classification: `FILE_WRITE_ACCESS` / `FILE_READ_ACCESS` mask bits in CreateOptions | Same semantic intent; CreateOptions includes the disposition (truncate/create/open) which is more informative anyway |
| B | Use `ShareAccess` field: `FILE_SHARE_WRITE` absent → read-only intent | Less reliable; process can open for exclusive read/write |
| C | Subscribe to legacy MOF provider (NT Kernel Logger session) instead | `DesiredAccess` becomes available; requires `KernelTrace` not `UserTrace`; less flexible for future fields |

**Recommendation for plan 10-01:** Document this discrepancy, confirm Option A with the user if desired, then proceed with `CreateOptions` masking.

**CreateOptions bit classification (for Option A):**
```
FILE_READ_ACCESS     = 0x0001 in DesiredAccess (not in CreateOptions directly)
```

Wait — `CreateOptions` is not `DesiredAccess` either. ETW's `CreateOptions` field in `KFileEvt_Create` maps to the `NtCreateFile` `CreateOptions` parameter (which controls caching, synchronization, etc.), NOT the `DesiredAccess` parameter.

**Corrected resolution:** The ETW `FileIo/Create` event from the modern kernel provider does not deliver `DesiredAccess` at all. The planner has two clean options:

1. **Classify by CreateOptions disposition bits** (`FILE_SUPERSEDE`, `FILE_CREATE`, `FILE_OPEN`, `FILE_OVERWRITE` — these indicate write intent) combined with `ShareAccess` (exclusive share → write intent).
2. **Default all Create events to `readwrite`** and let the user trim the profile — conservative but always correct.

The plan should pick one and document it. Option 2 is safest for a v1 implementation. Option 1 is more precise but requires empirical validation on Windows.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| ETW session lifecycle | Custom `StartTrace`/`OpenTrace`/`ProcessTrace` unsafe loop | `ferrisetw::UserTrace` | ~300 lines of unsafe boilerplate; error-prone ETW handle management |
| Event schema decoding | Manual TDH (Trace Data Helper) calls | `ferrisetw::SchemaLocator` + `Parser` | TDH API is complex; ferrisetw caches schemas per provider |
| NT→Win32 path conversion | Custom registry lookups | `QueryDosDeviceW` volume map | `QueryDosDeviceW` is the canonical Windows API; registry approach has edge cases |
| Admin privilege check | OpenProcessToken + TOKEN_ELEVATION | `crate::exec_strategy::is_admin_process()` | Already implemented at `exec_strategy_windows/mod.rs:353` — reuse directly |

**Key insight:** The ETW consumer API surface is large and error-prone. ferrisetw exists specifically to eliminate the boilerplate; use it as designed.

---

## Common Pitfalls

### Pitfall 1: ETW Callback Thread vs. Main Thread State
**What goes wrong:** ferrisetw spawns a background thread for event callbacks. Accessing `LearnState` from both the main thread (writing results after child exits) and the callback thread (writing events during tracing) without synchronization causes data races.
**Why it happens:** ETW callbacks arrive on a separate thread managed by ferrisetw.
**How to avoid:** Wrap all shared state in `Arc<Mutex<LearnState>>`. Clone the Arc into each provider callback closure.
**Warning signs:** Rust borrow checker will reject direct capture of `&mut` state in the closure.

### Pitfall 2: ETW Events Arriving After Child Exit
**What goes wrong:** Child process exits, you stop the trace, but the ETW buffer still contains unprocessed events. These get dropped.
**Why it happens:** ETW is asynchronous; kernel events are buffered before delivery to the consumer.
**How to avoid:** After child exits, sleep 200-500ms before stopping the trace (CONTEXT.md "drain with a brief timeout"). The exact value is Claude's discretion.

### Pitfall 3: Session Name Conflict (Multiple Concurrent Invocations)
**What goes wrong:** Two `nono learn` sessions with the same name cause the second `UserTrace::start()` to fail with an ETW error.
**Why it happens:** ETW session names are system-wide and unique.
**How to avoid:** Use `nono-learn-{pid}` as the session name. [CONTEXT.md, Claude's Discretion]

### Pitfall 4: NT Namespace Paths That Don't Map to Drive Letters
**What goes wrong:** Some file paths in ETW events are named pipe paths (`\Device\NamedPipe\...`), volume GUIDs, or network paths that have no drive letter mapping.
**Why it happens:** NT namespace is broader than Win32 drive-letter namespace.
**How to avoid:** `nt_to_win32()` returns `Option<PathBuf>`. If `None`, skip the path silently. These are not useful for profile generation.
**Warning signs:** Paths starting with `\Device\NamedPipe`, `\Device\Mup`, `\Device\LanmanRedirector`.

### Pitfall 5: CreateOptions vs DesiredAccess Mismatch (D-04)
**What goes wrong:** Code tries to parse `"DesiredAccess"` from the modern `Microsoft-Windows-Kernel-File` Create event. `parser.try_parse("DesiredAccess")` returns `Err` silently; all file events are skipped.
**Why it happens:** The modern ETW provider does not expose `DesiredAccess`. (See D-04 section above.)
**How to avoid:** Use `"CreateOptions"` field name, or `"FileName"` for the path field.
**Warning signs:** Zero file events captured despite correct PID tracking.

### Pitfall 6: `unwrap()` in ETW Callback Closures
**What goes wrong:** `Mutex::lock().unwrap()` in a callback closure panics if the mutex is poisoned (e.g., due to a panic elsewhere), crashing the ETW consumer thread.
**Why it happens:** CLAUDE.md strictly forbids `unwrap()`. ETW callbacks run on a thread that ferrisetw owns — a panic there may be silently swallowed or crash the process.
**How to avoid:** Use `lock().map_err(|_| ())` and return early on error, logging at ERROR level.

---

## Code Examples

### Admin Check (D-02)
```rust
// Source: [VERIFIED: crates/nono-cli/src/exec_strategy_windows/mod.rs:353]
// Reuse existing function — no new code needed
if !crate::exec_strategy::is_admin_process() {
    return Err(NonoError::LearnError(
        "nono learn requires administrator privileges. \
         Run from an elevated prompt (right-click \u{2192} Run as administrator)."
            .to_string(),
    ));
}
```

### LearnResult Construction
```rust
// Source: [VERIFIED: crates/nono-cli/src/learn.rs:36-73]
// LearnResult::new() is cfg(unix) — Windows backend needs its own constructor
// Use struct literal initialization (all BTreeSet::new() / Vec::new()) until
// plan 10-02 lifts the cfg guard on LearnResult::new()
let mut result = LearnResult {
    read_paths: BTreeSet::new(),
    read_files: BTreeSet::new(),
    write_paths: BTreeSet::new(),
    write_files: BTreeSet::new(),
    readwrite_paths: BTreeSet::new(),
    readwrite_files: BTreeSet::new(),
    system_covered: BTreeSet::new(),
    profile_covered: BTreeSet::new(),
    outbound_connections: Vec::new(),
    listening_ports: Vec::new(),
};
```

**Note:** `LearnResult::new()` is currently `#[cfg(any(target_os = "linux", target_os = "macos"))]`. Plan 10-01 or 10-02 must lift this cfg guard so Windows can use it, or the Windows backend uses a struct literal.

### ferrisetw Provider Subscription (verified API)
```rust
// Source: [CITED: docs.rs/ferrisetw/1.2.0/ferrisetw/]
use ferrisetw::prelude::*;

let provider = Provider::by_guid("EDD08927-9CC4-4E65-B970-C2560FB5C289")
    .add_callback(move |record: &EventRecord, locator: &SchemaLocator| {
        // ... handle event ...
    })
    .build();

let trace = UserTrace::new()
    .named(format!("nono-learn-{}", std::process::id()))
    .enable(provider)
    .start()?   // Returns Result<UserTrace, TraceError>
    .process(); // Blocking
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Legacy NT Kernel Logger (MOF-based, has DesiredAccess) | Modern manifest-based providers (no DesiredAccess, has CreateOptions) | Windows 8+ | Must use CreateOptions-based classification |
| Procmon.exe subprocess invocation | ETW direct via ferrisetw | Phase 10 scope | No external tool dependency |

**Deprecated/outdated:**
- MOF-based `FileIo_Create` (legacy `NT Kernel Logger`): Still works but requires `KernelTrace` session type; modern providers preferred per Microsoft documentation.
- `ferrisetw 0.x` API: Version 1.x introduced breaking changes. Use 1.2.0.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Windows OS | ETW providers | Windows-only build | Windows 10/11 (build 17763+) | No fallback; guarded by `#[cfg(target_os = "windows")]` |
| Administrator privilege | ETW kernel providers | Runtime check | — | Error message + non-zero exit (D-02) |
| ferrisetw 1.2.0 | ETW consumer session | Not yet in Cargo.toml | — | Add in plan 10-01 |
| `Win32_System_Diagnostics_Etw` | windows-sys feature | Not yet enabled | 0.59 | Add feature flag in plan 10-01 |
| `Win32_Storage_FileSystem` | QueryDosDeviceW | Already present | 0.59 | N/A |

**Missing dependencies with no fallback:**
- None that block execution; all missing items are Cargo.toml additions resolved in plan 10-01.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`cargo test`) |
| Config file | none (workspace default) |
| Quick run command | `cargo test -p nono-cli --lib learn_windows 2>/dev/null` |
| Full suite command | `cargo test -p nono-cli` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LEARN-01 (SC1) | NT path `\Device\HarddiskVolumeN\...` → Win32 `C:\...` | unit | `cargo test -p nono-cli --lib learn_windows::tests::test_nt_to_win32` | No — Wave 0 |
| LEARN-01 (SC2) | File I/O and network events captured | integration (Windows host, admin) | `cargo test -p nono-cli --test learn_integration -- --ignored` | No — Wave 0 |
| LEARN-01 (SC3) | Non-admin → clear error, exit non-zero | unit (mock is_admin=false) | `cargo test -p nono-cli --lib learn_windows::tests::test_non_admin_error` | No — Wave 0 |
| LEARN-01 (SC4) | ETW library choice documented | doc / plan artifact | plan 10-01 inline documentation | N/A — plan artifact |

### Sampling Rate
- **Per task commit:** `cargo test -p nono-cli --lib 2>/dev/null`
- **Per wave merge:** `cargo test -p nono-cli`
- **Phase gate:** Full suite green before `/gsd-verify-work`. SC2 and network capture tests are `#[ignore]` (require Windows host with admin); flag as human-verification items.

### Wave 0 Gaps
- [ ] `crates/nono-cli/src/learn_windows.rs` — new module with `#[cfg(test)]` unit tests covering NT-to-Win32 conversion and admin check
- [ ] ETW integration test file — `crates/nono-cli/tests/learn_integration.rs` (Windows-only, `#[ignore]` for CI)
- [ ] `LearnResult::new()` cfg guard lifted — needed so Windows backend can call the constructor

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | yes | Admin check via `is_admin_process()` before ETW setup |
| V5 Input Validation | yes | Path validation before inserting into `LearnResult`; use `PathBuf` not string ops |
| V6 Cryptography | no | — |

### Known Threat Patterns for ETW Learn Mode

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via NT namespace | Tampering | Convert and validate via `nt_to_win32()`; skip non-drive paths |
| String prefix on paths | Spoofing | Use `Path::starts_with()` not `str::starts_with()` (CLAUDE.md footgun) |
| Uncontrolled process tree scope | Elevation of Privilege | Track only descendants of the spawned child PID; never track PID 0 or PID 4 (System) |
| ETW callback panic propagation | Denial of Service | No `unwrap()` in callbacks; use `let Ok(..) = .. else { return; }` throughout |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `LearnResult::new()` can be made platform-agnostic by lifting the `#[cfg(unix)]` guard; no Unix-specific logic inside | Architecture Patterns — LearnResult Construction | If new() has Unix deps, Windows backend must use struct literal init instead |
| A2 | ferrisetw `UserTrace` works for the three kernel providers (Kernel-File, Kernel-Network, Kernel-Process) without requiring `KernelTrace` | Standard Stack | If any of the three providers require `KernelTrace`, the session setup pattern changes |
| A3 | `Win32_System_Diagnostics_Etw` feature in `windows-sys 0.59` compiles cleanly on the project's MSRV (1.77) | Standard Stack | Compilation failure; must fall back to ferrisetw-only approach with no direct ETW feature flags |
| A4 | ETW `Microsoft-Windows-Kernel-Network` events deliver remote IP and port as parseable fields (`daddr`, `dport` or similar) via ferrisetw Parser | Architecture Patterns | Network capture may require raw byte parsing or a different field name |

**Flagged for plan 10-01 verification before code:** A2 (provider type), A3 (feature flag compile), A4 (network event field names).

---

## Open Questions

1. **D-04 DesiredAccess vs CreateOptions — which classification strategy?**
   - What we know: Modern ETW Kernel-File provider has `CreateOptions`, not `DesiredAccess`. `CreateOptions` encodes disposition (create-new, open-existing, truncate, etc.).
   - What's unclear: Whether disposition bits reliably classify read vs. write intent for profile generation purposes.
   - Recommendation: Plan 10-01 should pick Option A (classify from CreateOptions disposition) or Option B (default all Creates to `readwrite`). Document the choice inline. Both are implementable; Option B is more conservative and simpler for v1.

2. **Are kernel providers accessible via `UserTrace` or must they use `KernelTrace`?**
   - What we know: ferrisetw docs show both `UserTrace` and `KernelTrace`; kernel providers on Win8+ can be consumed via either.
   - What's unclear: Whether `Microsoft-Windows-Kernel-File` requires `KernelTrace` specifically.
   - Recommendation: Plan 10-01 verifies with a minimal probe before committing to `UserTrace`.

3. **Exact field names for `Microsoft-Windows-Kernel-Network` events**
   - What we know: Provider GUID `7DD42A49-5329-4832-8DFD-43D979153A88`; events include TCP connect/accept.
   - What's unclear: Whether ferrisetw's Parser returns `"daddr"`, `"RemoteAddr"`, `"dport"`, or other field names from this provider.
   - Recommendation: Plan 10-02 (network events) verifies field names empirically via a debug print of all event properties before finalizing the parser calls.

---

## Sources

### Primary (HIGH confidence)
- `crates/nono-cli/src/learn.rs` — `LearnResult`, `NetworkConnectionSummary`, `NetworkEndpoint` structs; unsupported platform stub at line 387; `to_json()`, `to_summary()`, `to_profile()` methods
- `crates/nono-cli/src/learn_runtime.rs` — `run_learn()` entrypoint; warning flow; UX
- `crates/nono-cli/src/exec_strategy_windows/mod.rs:353` — `is_admin_process()` implementation (reuse confirmed)
- `crates/nono-cli/Cargo.toml` — confirmed `windows-sys 0.59` with `Win32_Storage_FileSystem` already present
- `crates/nono-cli/src/main.rs` — platform module routing pattern confirmed
- [CITED: docs.rs/ferrisetw/1.2.0/ferrisetw/] — UserTrace/KernelTrace API, Provider::by_guid, Parser::try_parse
- [CITED: docs.rs/ferrisetw/1.2.0/ferrisetw/trace/struct.UserTrace.html] — UserTrace API
- [VERIFIED: crates.io/crates/ferrisetw] — version 1.2.0, 2024-06-27, MIT OR Apache-2.0, ~49K downloads
- [CITED: learn.microsoft.com/en-us/windows/win32/etw/fileio-create] — FileIo_Create MOF fields: IrpPtr, TTID, FileObject, CreateOptions, FileAttributes, ShareAccess, OpenPath (no DesiredAccess)
- [CITED: github.com/microsoft/Tx/blob/master/Generated/Microsoft_Windows_Kernel_File.cs] — KFileEvt_Create fields: Irp, ThreadId, FileObject, CreateOptions, ShareAccess, FileName (no DesiredAccess confirmed)
- [VERIFIED: docs.rs/windows-sys/latest — QueryDosDeviceW in Win32_Storage_FileSystem]

### Secondary (MEDIUM confidence)
- [github.com/n4r1b/ferrisetw] — repository overview, crate description, examples structure
- [n4r1b.com/doc/ferrisetw/trace/struct.KernelTrace.html] — KernelTrace API surface

### Tertiary (LOW confidence)
- ETW provider GUID `7DD42A49-5329-4832-8DFD-43D979153A88` for Kernel-Network (from CONTEXT.md prior research — not independently verified in this session)
- ferrisetw field names for `Microsoft-Windows-Kernel-Network` events (not verified; flagged as A4)

---

## Metadata

**Confidence breakdown:**
- ferrisetw library audit: HIGH — version, date, license, downloads verified against crates.io
- LearnResult interface: HIGH — read directly from source
- is_admin_process() reuse: HIGH — read directly from source
- ETW file event field names (DesiredAccess discrepancy): HIGH — verified via MS Learn + Tx repo
- ETW network event field names: LOW — not verified empirically; flagged as A4
- QueryDosDevice path conversion: HIGH — API in existing windows-sys feature set

**Research date:** 2026-04-10
**Valid until:** 2026-07-10 (ETW kernel provider APIs are stable Windows platform APIs)
