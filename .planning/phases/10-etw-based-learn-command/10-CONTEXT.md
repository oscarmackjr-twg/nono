# Phase 10: ETW-Based Learn Command - Context

**Gathered:** 2026-04-10
**Status:** Ready for planning

<domain>
## Phase Boundary

Add a Windows ETW backend to `nono learn <cmd>` that:
1. Spawns the child command unsandboxed
2. Attaches an ETW consumer session using ferrisetw
3. Captures `Microsoft-Windows-Kernel-File` (file I/O) and `Microsoft-Windows-Kernel-Network` (TCP) events
4. Tracks the full process tree (child + all descendants)
5. Converts NT namespace paths to Win32 format
6. Populates the existing `LearnResult` struct — same output format as Unix backends

This phase does NOT add new CLI flags, new output formats, or changes to the Unix backends.

</domain>

<decisions>
## Implementation Decisions

### D-01: ETW Library — ferrisetw

Use the `ferrisetw` crate (n4r1b/ferrisetw) for ETW consumer/provider integration.

**Rationale:** Higher-level Rust API handles ETW session lifecycle, callback dispatch, and schema decoding. The prior research (260406-bem-RESEARCH.md) already recommended it. Direct `windows-sys` ETW bindings would require ~200–300 lines of unsafe boilerplate we don't need to own.

**Prerequisite:** Audit ferrisetw before committing. Check: (a) last release date and activity, (b) no unsafe soundness issues, (c) license compatible with nono. Document the audit outcome in the plan (per SC4 — "ETW library choice documented with rationale before any ETW code is written").

### D-02: Admin Privilege — Upfront Check with Runas Hint

Call `IsUserAnAdmin()` before touching any ETW API. If not elevated:
- Print a clear, actionable error to stderr
- Error message must include: "nono learn requires administrator privileges. Run from an elevated prompt (right-click → Run as administrator)."
- Exit non-zero immediately

Do NOT attempt ETW setup and rely on `ACCESS_DENIED` to surface the problem. Fail fast.

### D-03: Process Tree Scope — Full Tree (strace -f equivalent)

Track the direct child PID **plus all descendants** spawned during the session.

Implementation: subscribe to `Microsoft-Windows-Kernel-Process` provider events (`CreateProcess`, `ExitProcess`) to maintain a live set of tracked PIDs. When a new process is created with a parent PID in the tracked set, add the child PID. When a process exits, remove it from the set.

This mirrors Linux `strace -f` behavior and is essential for tools like cargo, npm, and make that fork subprocesses.

### D-04: FileIo Event → Access Mode Mapping

Use the `DesiredAccess` field from `FileIo/Create` ETW events to classify each file access:

| DesiredAccess flags | nono access category |
|---------------------|---------------------|
| `GENERIC_READ` only | read |
| `GENERIC_WRITE` or `DELETE` only | write |
| `GENERIC_READ + GENERIC_WRITE`, `GENERIC_ALL`, or other combinations | readwrite |

`FileIo/Read` and `FileIo/Write` operation events alone are too noisy for categorization. The `FileIo/Create` event (triggered by `NtCreateFile`/`NtOpenFile`) captures the caller's intent via `DesiredAccess`.

### D-05: NT Namespace → Win32 Path Conversion

ETW `FileIo/Create` events deliver paths in NT namespace format (`\Device\HarddiskVolume3\path`). SC1 requires Win32 format (`C:\path`).

Conversion: build a volume map at startup by calling `QueryDosDevice` for each drive letter (A–Z), constructing a `\Device\HarddiskVolumeN` → `X:\` lookup table. Apply at event-processing time. This is Claude's discretion — standard approach, no user input needed.

### Claude's Discretion

- NT → Win32 path conversion mechanics (QueryDosDevice volume map, built at startup)
- ferrisetw session naming convention (`nono-learn-{pid}` or similar)
- Event buffer sizing and timeout for the ETW consumer loop
- Which `Microsoft-Windows-Kernel-Network` event subtypes to capture (TcpIp/Connect, TcpIp/Accept minimum)
- How to handle ETW events received after the child process exits (drain with a brief timeout)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Existing learn implementation (Unix backends — must stay unchanged)
- `crates/nono-cli/src/learn.rs` — `LearnResult`, `NetworkConnectionSummary`, `NetworkEndpoint` types; `to_json()`, `to_summary()`, `to_profile()` output methods; unsupported platform stub at line ~386
- `crates/nono-cli/src/learn_runtime.rs` — `run_learn()` entrypoint, admin warning flow, offer-save-profile UX

### CLI wiring
- `crates/nono-cli/src/cli.rs` — `LearnArgs` struct (line ~1533); Learn subcommand definition

### Prior ETW research
- `.planning/quick/260406-bem-research-and-roadmap-windows-gap-closure/260406-bem-RESEARCH.md` — Gap 3 section: ETW provider GUIDs, ferrisetw evaluation, approach comparison table

### Requirements
- `.planning/REQUIREMENTS.md` — LEARN-01 acceptance criteria
- `.planning/ROADMAP.md` — Phase 10 success criteria (SC1–SC4)

### Windows platform patterns (how other Windows modules are structured)
- `crates/nono-cli/src/main.rs` — platform module routing pattern (`#[cfg(target_os = "windows")] #[path = "..."]`)
- `crates/nono-cli/src/session_commands_windows.rs` — reference for a Windows-only module in the same crate

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `LearnResult` struct (`learn.rs`): already defined with `read_paths`, `write_paths`, `readwrite_paths`, `read_files`, `write_files`, `readwrite_files`, `outbound_connections`, `listening_ports` — the Windows backend just needs to populate this
- `NetworkConnectionSummary` / `NetworkEndpoint` (`learn.rs` line ~1128): reuse directly for TCP events
- Unsupported platform stub (`learn.rs` ~386): replace with `#[cfg(target_os = "windows")]` real implementation

### Established Patterns
- Windows-specific modules use `#[cfg(target_os = "windows")] #[path = "..._windows.rs"]` routing in `main.rs`
- `IsUserAnAdmin()` from `windows-sys` — already used in other Windows modules for privilege checks
- `windows-sys = 0.59` already in `Cargo.toml` — ETW features (`Win32_System_Diagnostics_Etw`) need to be added

### Integration Points
- `learn.rs` unsupported stub: replace with `#[cfg(target_os = "windows")]` implementation or add a `learn_windows.rs` module (same routing pattern as other Windows modules)
- `Cargo.toml` (nono-cli): add `ferrisetw` dependency (Windows-only, `[target.'cfg(windows)'.dependencies]`)
- `Cargo.toml` (nono-cli): add `Win32_System_Diagnostics_Etw` feature to `windows-sys`

</code_context>

<specifics>
## Specific Ideas

- ferrisetw audit must happen in plan 10-01 before any ETW code is written — document findings inline in the plan
- Process tree tracking requires subscribing to the `Microsoft-Windows-Kernel-Process` provider in the same ETW session as the file/network providers
- ETW provider GUIDs (from prior research):
  - `Microsoft-Windows-Kernel-File`: `EDD08927-9CC4-4E65-B970-C2560FB5C289`
  - `Microsoft-Windows-Kernel-Network`: `7DD42A49-5329-4832-8DFD-43D979153A88`
  - `Microsoft-Windows-Kernel-Process`: `22FB2CD6-0E7B-422B-A0C7-2FAD1FD0E716`

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 10-etw-based-learn-command*
*Context gathered: 2026-04-10*
