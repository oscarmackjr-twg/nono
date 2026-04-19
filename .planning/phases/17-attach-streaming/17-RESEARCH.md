# Phase 17: Attach-Streaming (ATCH-01) — Research

**Researched:** 2026-04-19
**Domain:** Win32 anonymous pipes + STARTUPINFOW handle wiring + supervisor-side bridging
**Confidence:** HIGH (codebase grep verified all reference sites; Microsoft Learn docs cross-checked for API contracts)

## Summary

Phase 17 closes the v2.1 attach-streaming gap by replacing the "no PTY → no streaming" early-return on the Windows detached path with anonymous-pipe stdio bridged through the supervisor. The architecture is fully locked in `17-CONTEXT.md` (D-01..D-08, G-01..G-04). Research focused on the *mechanics*: which Win32 calls produce inheritable anonymous pipes, exactly which `STARTUPINFOW` fields wire them into the child, what shape the supervisor's bridge threads take, and how the friendly busy error surfaces from `OpenOptions::open` on `\\.\pipe\nono-data-<id>`.

All change surfaces are confined to `*_windows.rs` files inside `crates/nono-cli/src/exec_strategy_windows/` plus `crates/nono-cli/src/session_commands_windows.rs`. Cross-platform code (D-21 Windows-invariance) is byte-identical post-change. The Phase 15 `0xC0000142` fix is preserved structurally — the `should_allocate_pty()` gate still returns `false` on the detached path, so no `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` is added to `STARTUPINFOEXW`. The two paths (PTY for `nono shell` vs. anonymous-pipe for detached) remain mutually exclusive at `STARTUPINFO` construction time.

**Primary recommendation:** Single plan covers all of D-01..D-08 + G-01..G-04 with minimum viable structural change. The smoke gate is already structurally separate (G-04 maps to existing Phase 15 5-row matrix executed manually — no new test scaffolding needed). Recommend **2 plans**: (17-01) implementation + unit tests + integration tests, (17-02) closeout (smoke gate, REQUIREMENTS.md acceptance #3 downgrade, CHANGELOG, docs note for "no resize on detached"). See § "Plan-decomposition recommendation" for rationale.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Anonymous pipe creation (3 pairs: stdin, stdout, stderr) | `exec_strategy_windows::launch` (`spawn_windows_child`) | — | Pipes are owned by the supervisor process and bound to child stdio; creation must happen at spawn time, before `CreateProcessW`. |
| Handle inheritance into child | `exec_strategy_windows::launch` (CreateProcessW arg) | — | `bInheritHandles=TRUE` + per-handle `bInheritHandle=TRUE` on `SECURITY_ATTRIBUTES` is the only way `STARTUPINFOW.hStdInput/Output/Error` reach the child. |
| Supervisor-side handle storage | `exec_strategy_windows::supervisor` (`WindowsSupervisorRuntime`) | — | Lifetime must match the child; runtime already holds `pty: Option<PtyPair>` — add a parallel `Option<DetachedStdio>` field. |
| Pipe-to-log + pipe-to-attach-client mirroring | `exec_strategy_windows::supervisor::start_logging` (extended) | — | Existing function already handles the PTY-source case; adding the pipe-source branch keeps the single-thread, single-loop shape and the established `active_attachment` mutex pattern. |
| Attach-client-to-child stdin write | `exec_strategy_windows::supervisor::start_data_pipe_server` (extended) | — | Existing function already handles the PTY-sink case; adding the pipe-sink branch reuses the named-pipe accept loop and `active_attachment` lifecycle. |
| Friendly busy-error translation | `session_commands_windows::run_attach` | `nono::error::NonoError::AttachBusy` (already exists) | Client-side detection of `ERROR_PIPE_BUSY` (231) on `OpenOptions::open` is the natural injection point; `AttachBusy` already carries the right user-visible message. |

## Standard Stack

### Core (already in workspace)

| Crate / Symbol | Version | Purpose | Why Standard |
|---|---|---|---|
| `windows-sys` | 0.59 | `CreatePipe`, `STARTUPINFOW`, `SECURITY_ATTRIBUTES`, `CreateProcessW`, `WriteFile`, `ReadFile`, `ERROR_PIPE_BUSY`, `ERROR_BROKEN_PIPE`, `INVALID_HANDLE_VALUE` | Existing dependency; same crate already used in `pty_proxy_windows.rs` and `supervisor.rs`. [VERIFIED: Cargo.toml + grep] |
| `std::os::windows::io::FromRawHandle` | std | `File::from_raw_handle` for ergonomic Read/Write on a `HANDLE` | Already used at `supervisor.rs:509,673,697,808,810` — well-established pattern. [VERIFIED: grep] |
| `std::mem::ManuallyDrop` | std | Avoid double-close when wrapping a borrowed `HANDLE` in `File` | Already used at `supervisor.rs:508,673,697,809` — established pattern. [VERIFIED: grep] |
| `std::thread::spawn` | std | Bridge threads (one per pipe direction) | Established pattern in `start_logging`, `start_data_pipe_server`, `start_interactive_terminal_io`. [VERIFIED: grep] |

### No new dependencies required.

All Win32 symbols Phase 17 needs are already accessible through `windows_sys::Win32::*`. No new `Cargo.toml` changes. [VERIFIED: grep on `windows_sys`]

## Architecture Patterns

### System Architecture Diagram

```
[ nono attach <id> client ]                    [ detached supervisor (this process) ]
    │                                                        │
    │  std::fs::OpenOptions::open(\\.\pipe\nono-data-<id>)   │   start_data_pipe_server (NEW pipe-sink branch)
    ├────────────────────────────────────────────────────────┤   ├── ConnectNamedPipe (loop)
    │  ◄── ERROR_PIPE_BUSY (231) if 2nd client ──────────────┤   ├── set active_attachment = Some(pipe)
    │       → translate to NonoError::AttachBusy             │   ├── ReadFile(named pipe) → WriteFile(stdin_write)
    │                                                        │   └── on EOF: clear active_attachment
    │                                                        │
    │  scrollback (read $log_path verbatim)                  │   start_logging (NEW pipe-source branch)
    │                                                        │   ├── ReadFile(stdout_read) [+ stderr_read thread]
    │  stdin → pipe.write_all (loop)                         │   ├── ALWAYS append to log file
    │  pipe.read → stdout (loop, separate thread)            │   └── IF active_attachment: WriteFile to client pipe
    │                                                        │   (best effort — drop client on broken pipe, supervisor & log
    │                                                        │    continue uninterrupted)
    │                                                        │
    │                                                        │   spawn_windows_child (NEW pipe-creation branch)
    │                                                        │   ├── CreatePipe ×3 (stdin/stdout/stderr) with bInheritHandle=1
    │                                                        │   ├── SetHandleInformation HANDLE_FLAG_INHERIT on child ends
    │                                                        │   ├── STARTUPINFOW.{cb, dwFlags=STARTF_USESTDHANDLES,
    │                                                        │   │     hStdInput=stdin_read,
    │                                                        │   │     hStdOutput=stdout_write,
    │                                                        │   │     hStdError=stderr_write}
    │                                                        │   ├── CreateProcessW(.., bInheritHandles=TRUE, ..)
    │                                                        │   └── close child-end handles in supervisor; keep parent ends
    │                                                        │
    │                                                        ▼
    │                                              ┌─────────────────────────┐
    │                                              │ sandboxed grandchild    │
    │                                              │ (e.g. ping, cmd.exe)    │
    │                                              │ stdin=pipe(read end)    │
    │                                              │ stdout=pipe(write end)  │
    │                                              │ stderr=pipe(write end)  │
    │                                              └─────────────────────────┘
```

### Recommended File Layout (no new files)

All Phase 17 code lives in existing files. **No new files**, **no new modules** — minimum viable structural change per CONTEXT.md "Claude's Discretion".

```
crates/nono-cli/src/
├── exec_strategy_windows/
│   ├── launch.rs                          # MODIFIED: spawn_windows_child gains pipe-creation branch
│   ├── supervisor.rs                      # MODIFIED: WindowsSupervisorRuntime gains stdio field;
│   │                                      #   start_logging gains pipe-source branch;
│   │                                      #   start_data_pipe_server gains pipe-sink branch
│   └── mod.rs                             # MODIFIED: execute_supervised threads stdio into runtime + child
└── session_commands_windows.rs            # MODIFIED: run_attach translates ERROR_PIPE_BUSY → AttachBusy
```

### Pattern 1: CreatePipe with selective inheritance

Anonymous pipes don't carry inheritance per-pipe — each handle's inheritability is set via `SECURITY_ATTRIBUTES.bInheritHandle` at creation time, with `SetHandleInformation(HANDLE_FLAG_INHERIT, 0)` used to flip it off on the supervisor-end after creation. This is the standard Win32 idiom for "child sees one end, parent sees the other".

```rust
// Source: Microsoft Learn — Anonymous Pipes (Creating a Child Process with Redirected Input and Output)
// https://learn.microsoft.com/en-us/windows/win32/procthread/creating-a-child-process-with-redirected-input-and-output
// [CITED]
let sa = SECURITY_ATTRIBUTES {
    nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
    lpSecurityDescriptor: std::ptr::null_mut(),
    bInheritHandle: 1, // both ends inheritable initially
};

let mut child_stdin_read: HANDLE = INVALID_HANDLE_VALUE;
let mut parent_stdin_write: HANDLE = INVALID_HANDLE_VALUE;
unsafe {
    if CreatePipe(&mut child_stdin_read, &mut parent_stdin_write, &sa, 0) == 0 {
        return Err(NonoError::SandboxInit(format!(
            "CreatePipe(stdin) failed: {}", std::io::Error::last_os_error()
        )));
    }
    // Mark the parent end NON-inheritable so the child does not get a handle to it.
    SetHandleInformation(parent_stdin_write, HANDLE_FLAG_INHERIT, 0);
}
```

### Pattern 2: STARTUPINFOW with STARTF_USESTDHANDLES

The non-PTY branch already builds a plain `STARTUPINFOW` (not `STARTUPINFOEXW`) at `launch.rs:1109-1148`. We extend that branch to set `dwFlags |= STARTF_USESTDHANDLES` and populate the three `hStd*` fields. The PTY branch (which uses `STARTUPINFOEXW` with `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`) stays untouched — pipe + ConPTY is mutually exclusive (see Landmines § "STARTUPINFOEX collision").

```rust
// Source: Microsoft Learn — STARTUPINFOW dwFlags
// https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/ns-processthreadsapi-startupinfow
// [CITED]
startup_info.cb = size_of::<STARTUPINFOW>() as u32;
startup_info.dwFlags = STARTF_USESTDHANDLES;
startup_info.hStdInput  = child_stdin_read;
startup_info.hStdOutput = child_stdout_write;
startup_info.hStdError  = child_stderr_write;
// And `bInheritHandles` MUST be 1 in the CreateProcess call.
```

### Anti-Patterns to Avoid

- **`PIPE_NOWAIT` on either end of the anonymous pipe.** `CreatePipe` produces blocking byte-mode pipes by default and that's exactly what the supervisor's blocking-thread bridge needs. Switching to `PIPE_NOWAIT` would force overlapped I/O for clean EOF semantics. [VERIFIED: docs.microsoft.com/en-us/windows/win32/ipc/anonymous-pipes — "Anonymous pipes are implemented using a named pipe with a unique name … In synchronous (blocking) mode, the read or write operation blocks until the requested number of bytes is read or written"]
- **Setting `bInheritHandle=FALSE` on the SECURITY_ATTRIBUTES then trying to inherit handles in CreateProcessW.** The two are coupled: an inheritable handle requires both per-handle inheritability AND `bInheritHandles=TRUE` on `CreateProcessW`. Forgetting the per-handle bit is the most common cause of "child opens stdin but reads garbage / EOF immediately". [CITED: learn.microsoft.com STARTUPINFOW remarks]
- **Closing the parent-end handle before `WaitForSingleObject` returns.** A premature `CloseHandle` on `parent_stdout_read` makes `ReadFile` return `ERROR_BROKEN_PIPE (109)` on the supervisor side, which we'd misread as "child finished writing". The handles MUST live for the lifetime of the bridge thread (which exits naturally when the child closes its end on exit). [VERIFIED: existing code at `supervisor.rs:512-515` already follows this — match the pattern]
- **Closing the child-end handle BEFORE `CreateProcessW` resumes.** The handle must remain open through `CreateProcessW`; if closed too early the child inherits a handle to a closed object → `INVALID_HANDLE_VALUE` in `STARTUPINFO` and the child fails on first I/O. Order: `CreatePipe` → `CreateProcessW` (suspended) → close child-end handles in supervisor → `ResumeThread`. [CITED: learn.microsoft.com Anonymous Pipes article]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---|---|---|---|
| Pipe back-pressure handling | Custom buffering / overlapped I/O state machine | Default blocking `CreatePipe` + dedicated thread per direction (existing `start_logging` pattern) | The existing PTY worker uses exactly this shape; matching it keeps the code base consistent and avoids overlapped-I/O complexity. [VERIFIED: `supervisor.rs:483-543`] |
| Friendly busy-error UX | New `NonoError` variant | `NonoError::AttachBusy` (already exists at `error.rs:144`) | Already carries the right `Display` impl ("Session already has an active attached client"). Adding context via wrap is fine; new variant unnecessary. [VERIFIED: grep] |
| Single-attach enforcement | New mutex + accept-rejection logic | Existing `active_attachment: Arc<Mutex<Option<SendableHandle>>>` + named-pipe `nMaxInstances=1` | Already enforces single-attach by virtue of `CreateNamedPipeW(..., nMaxInstances=1, ...)` at `supervisor.rs:165`. Second client gets `ERROR_PIPE_BUSY` from the OS. [VERIFIED: `supervisor.rs:130-184`] |
| Stderr separation | Custom merge logic / second log channel | Pass the same `child_stderr_write` handle for `hStdError` AND merge in a single thread, OR set them to separate pipes and spawn two reader threads writing to the same log/relay sink | The non-detached PTY path collapses stderr into stdout (ConPTY behavior); matching that is the simplest path. CONTEXT.md `<specifics>` confirms this is acceptable. [VERIFIED: CONTEXT.md line 127] |
| Log file open race | Custom locking | `OpenOptions::create(true).append(true).open(..)` (already used at `supervisor.rs:496-499`) | Append mode is the established log-write idiom; works regardless of source. [VERIFIED: grep] |

**Key insight:** Phase 17 is structurally a *port* of the existing PTY-source / PTY-sink bridge code into a parallel pipe-source / pipe-sink branch. The same patterns, the same primitives (`SendableHandle`, `Mutex<Option<...>>`, `ManuallyDrop<File::from_raw_handle>`, 4096-byte buffers, blocking thread per direction), the same lifecycle semantics. There is nothing novel to invent.

## Runtime State Inventory

> Phase 17 is a Windows-only feature add — not a rename/refactor/migration. No runtime state in any external system needs migrating. This section is included for completeness per the research checklist; all categories are explicitly **None — verified by file/grep audit**.

| Category | Items Found | Action Required |
|---|---|---|
| Stored data | None — verified by grep across `.planning/`, no SQLite/Mem0/Chroma referenced for `nono attach` | None |
| Live service config | None — `nono-wfp-service` is unaffected; SDDL on the named pipes is unchanged (still `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)`) | None |
| OS-registered state | None — Windows Task Scheduler / pm2 / launchd / Job Object names unchanged | None |
| Secrets / env vars | None — no new env vars; `NONO_DETACHED_LAUNCH` semantic preserved | None |
| Build artifacts / installed packages | None — no new crates, no new generated headers | None |

## Common Pitfalls

### Pitfall 1: Bridge thread exit when child outlives one end of the pipe
**What goes wrong:** When the attach client disconnects (Ctrl-]d sequence triggers `DisconnectNamedPipe`), a `WriteFile` from the supervisor's stdout-bridge to the now-disconnected named pipe returns `ERROR_NO_DATA (232)` or `ERROR_BROKEN_PIPE (109)`. Naive code that uses `?` propagation kills the supervisor's stdout-bridge thread, and the next time the user reattaches there's no live bridge.
**Why it happens:** Disconnect closes the named pipe instance, but the anonymous-pipe stdout-read end is still live (the child is still running and writing).
**How to avoid:** The existing PTY-branch code at `supervisor.rs:528-541` already handles this correctly — `WriteFile` is invoked through unsafe FFI directly with the return code ignored. The pipe-bridge code MUST follow the same pattern: best-effort write, ignore failure, keep reading from the child. The log file write is the load-bearing path; the attach-client mirror is a "if anyone is listening" side effect.
**Warning signs:** Test scenario — reattach after detaching; the second attach shows scrollback but no live output. Means the bridge thread died.

### Pitfall 2: ERROR_PIPE_BUSY surfaces as opaque `io::Error`
**What goes wrong:** `OpenOptions::open(r"\\.\pipe\nono-data-<id>")` returns `Err(io::Error)` whose `raw_os_error()` is `Some(231)` when the pipe instance is busy. The current code at `session_commands_windows.rs:391-400` displays this as a generic `Setup` error with the io-error formatted into the string — the friendly hint about another attached client gets buried.
**Why it happens:** `OpenOptions::open` doesn't distinguish "pipe doesn't exist" (`ERROR_FILE_NOT_FOUND` = 2) from "pipe busy" (`ERROR_PIPE_BUSY` = 231) — both surface as `io::ErrorKind::NotFound` or `io::ErrorKind::Other` depending on the Win32 error.
**How to avoid:** Inspect `err.raw_os_error()` immediately after `open` returns `Err`; match `Some(231)` → `NonoError::AttachBusy`; everything else → existing setup-error message. The existing `supervisor::socket_windows.rs:648-651` code shows the exact pattern.
**Warning signs:** User sees `Failed to connect to session data pipe: The pipe is busy. (os error 231). Is another client already attached?` instead of the clean `Session <id> is already attached. Use 'nono detach <id>' to release the existing client first.`

### Pitfall 3: Forgetting `bInheritHandles=TRUE` on `CreateProcessW`
**What goes wrong:** Pipes are created inheritable, `STARTF_USESTDHANDLES` is set, `hStdInput/Output/Error` are populated — but the child process gets `INVALID_HANDLE_VALUE` for stdio because `CreateProcessW`'s 5th `BOOL bInheritHandles` parameter is `0` (the existing call at `launch.rs:1086,1135`).
**Why it happens:** The existing call sites all pass `0` because the PTY path uses `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` which doesn't require general handle inheritance. The new pipe-stdio branch needs `1` for inheritance to take effect.
**How to avoid:** When pipes are wired (no PTY, detached path), the `CreateProcessW`/`CreateProcessAsUserW` call MUST use `bInheritHandles=1`. The PTY branch and the no-pipe-no-PTY (non-existent in current code) branches stay at `0`.
**Warning signs:** Child exits immediately with stdio errors; supervisor `ReadFile(stdout_read)` returns `ERROR_BROKEN_PIPE` immediately; log file is empty.

### Pitfall 4: Stderr handle dropped by the runtime before the bridge thread exits
**What goes wrong:** If the supervisor's `WindowsSupervisorRuntime` field holding the parent-end stderr handle is dropped (or the `OwnedHandle` wrapper closes it) while the bridge thread still holds a `ManuallyDrop<File>` reading from it, `ReadFile` returns `ERROR_INVALID_HANDLE (6)`.
**Why it happens:** Lifetime mismatch between owning struct field and the borrowed raw handle in the bridge thread.
**How to avoid:** Mirror the PTY pattern exactly — the `pty: Option<PtyPair>` field on `WindowsSupervisorRuntime` (which owns `hpcon` + `input_write` + `output_read`) is dropped only when the runtime is dropped, which happens AFTER the child exits. Add a parallel `stdio: Option<DetachedStdio>` field with `Drop` that calls `CloseHandle` on the parent ends only after the bridge threads have observed EOF.
**Warning signs:** `tracing::error!` from the bridge thread reading on a stale handle.

### Pitfall 5: Outer probe loop times out because the inner supervisor takes longer
**What goes wrong:** `startup_runtime::run_detached_launch` waits up to 2 seconds for `\\.\pipe\nono-session-<id>` (control pipe). The new code creates 3 anonymous pipes BEFORE `start_control_pipe_server` returns, which adds a few ms. If the order regresses (e.g., pipes after control-pipe creation), the outer probe could fire on a control pipe whose stdio bridge isn't ready yet.
**Why it happens:** `WindowsSupervisorRuntime::initialize` already calls `start_control_pipe_server()` BEFORE `start_logging()` and `start_data_pipe_server()` — preserve that order. The pipe creation in `spawn_windows_child` happens AFTER `initialize` returns (control pipe is up by then), so the outer probe sees the pipe and the banner is printed before the child even starts writing.
**How to avoid:** Don't reorder `start_control_pipe_server` / `start_logging` / `start_data_pipe_server` invocations in `WindowsSupervisorRuntime::initialize` (lines 309-315). If the pipe-source branch is added inside `start_logging`, no reordering is required.
**Warning signs:** Detached banner prints, but `nono attach` immediately after returns "session not found" or hangs on `WaitNamedPipeW`.

## Code Examples

Verified patterns ready to lift into the implementation.

### Example 1: Create three inheritable anonymous pipes in spawn_windows_child

```rust
// In: crates/nono-cli/src/exec_strategy_windows/launch.rs
// New helper, called from spawn_windows_child when (pty.is_none() && is_windows_detached_launch())
//
// Source: Microsoft Learn — Creating a Child Process with Redirected Input and Output
// https://learn.microsoft.com/en-us/windows/win32/procthread/creating-a-child-process-with-redirected-input-and-output
// [CITED]

use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE, HANDLE_FLAG_INHERIT};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::Foundation::{CloseHandle, SetHandleInformation};

pub(super) struct DetachedStdioPipes {
    /// Parent end — supervisor reads child stdout from this.
    pub stdout_read: HANDLE,
    /// Parent end — supervisor reads child stderr from this.
    pub stderr_read: HANDLE,
    /// Parent end — supervisor writes child stdin to this.
    pub stdin_write: HANDLE,
    /// Child end — set in STARTUPINFOW.hStdInput; closed by supervisor after CreateProcess.
    pub stdin_read: HANDLE,
    pub stdout_write: HANDLE,
    pub stderr_write: HANDLE,
}

impl DetachedStdioPipes {
    pub fn create() -> Result<Self> {
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: std::ptr::null_mut(),
            bInheritHandle: 1,
        };

        let (stdin_read, stdin_write) = create_one_pipe(&sa, "stdin")?;
        let (stdout_read, stdout_write) = create_one_pipe(&sa, "stdout").map_err(|e| {
            unsafe { CloseHandle(stdin_read); CloseHandle(stdin_write); }
            e
        })?;
        let (stderr_read, stderr_write) = create_one_pipe(&sa, "stderr").map_err(|e| {
            unsafe {
                CloseHandle(stdin_read); CloseHandle(stdin_write);
                CloseHandle(stdout_read); CloseHandle(stdout_write);
            }
            e
        })?;

        // Mark parent ends NON-inheritable so the child cannot accidentally
        // get a handle to them.
        unsafe {
            SetHandleInformation(stdin_write, HANDLE_FLAG_INHERIT, 0);
            SetHandleInformation(stdout_read, HANDLE_FLAG_INHERIT, 0);
            SetHandleInformation(stderr_read, HANDLE_FLAG_INHERIT, 0);
        }

        Ok(Self {
            stdout_read, stderr_read, stdin_write,
            stdin_read, stdout_write, stderr_write,
        })
    }

    /// Close the child-end handles after CreateProcess inherits them. Must be called
    /// AFTER CreateProcess succeeds (so the child has its own duplicates) and BEFORE
    /// ResumeThread (so child sees the EOF on stdin only when supervisor writes-end closes).
    pub unsafe fn close_child_ends(&mut self) {
        if self.stdin_read != INVALID_HANDLE_VALUE { CloseHandle(self.stdin_read); self.stdin_read = INVALID_HANDLE_VALUE; }
        if self.stdout_write != INVALID_HANDLE_VALUE { CloseHandle(self.stdout_write); self.stdout_write = INVALID_HANDLE_VALUE; }
        if self.stderr_write != INVALID_HANDLE_VALUE { CloseHandle(self.stderr_write); self.stderr_write = INVALID_HANDLE_VALUE; }
    }
}

impl Drop for DetachedStdioPipes {
    fn drop(&mut self) {
        unsafe {
            for h in [self.stdin_read, self.stdout_write, self.stderr_write,
                      self.stdin_write, self.stdout_read, self.stderr_read] {
                if h != INVALID_HANDLE_VALUE && !h.is_null() { CloseHandle(h); }
            }
        }
    }
}

fn create_one_pipe(sa: &SECURITY_ATTRIBUTES, label: &str) -> Result<(HANDLE, HANDLE)> {
    let mut read: HANDLE = INVALID_HANDLE_VALUE;
    let mut write: HANDLE = INVALID_HANDLE_VALUE;
    let ok = unsafe { CreatePipe(&mut read, &mut write, sa as *const _, 0) };
    if ok == 0 {
        return Err(NonoError::SandboxInit(format!(
            "CreatePipe({label}) failed: {}", std::io::Error::last_os_error()
        )));
    }
    Ok((read, write))
}
```

### Example 2: Wire pipes into STARTUPINFOW in spawn_windows_child

```rust
// In: crates/nono-cli/src/exec_strategy_windows/launch.rs
// Replaces the `let mut startup_info: STARTUPINFOW` block at lines 1109-1148 when
// (pty.is_none() && is_windows_detached_launch()) is true.
//
// Source: Microsoft Learn — STARTUPINFOW (dwFlags = STARTF_USESTDHANDLES)
// https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/ns-processthreadsapi-startupinfow
// [CITED]

use windows_sys::Win32::System::Threading::STARTF_USESTDHANDLES;

let mut detached_stdio = if pty.is_none() && is_windows_detached_launch {
    Some(DetachedStdioPipes::create()?)
} else {
    None
};

let mut startup_info: STARTUPINFOW = unsafe { std::mem::zeroed() };
startup_info.cb = size_of::<STARTUPINFOW>() as u32;

if let Some(ref pipes) = detached_stdio {
    startup_info.dwFlags = STARTF_USESTDHANDLES;
    startup_info.hStdInput  = pipes.stdin_read;
    startup_info.hStdOutput = pipes.stdout_write;
    startup_info.hStdError  = pipes.stderr_write;
}

// CRITICAL: bInheritHandles MUST be 1 when STARTF_USESTDHANDLES is set with
// inheritable handles. The PTY branch passes 0 (uses PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE
// instead) and that stays at 0. Detached-stdio branch passes 1.
let inherit_handles: BOOL = if detached_stdio.is_some() { 1 } else { 0 };

let created = unsafe {
    CreateProcessW(
        application_name.as_ptr(),
        command_line.as_mut_ptr(),
        std::ptr::null(),
        std::ptr::null(),
        inherit_handles,
        CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
        environment_block.as_mut_ptr() as *mut _,
        current_dir_u16.as_ptr(),
        &startup_info,
        &mut process_info,
    )
};

if created == 0 {
    return Err(NonoError::SandboxInit(format!(
        "Failed to launch Windows child process (error={})",
        unsafe { GetLastError() }
    )));
}

// IMPORTANT: close the child-end handles BEFORE ResumeThread but AFTER CreateProcess.
// Otherwise the supervisor side will see EOF only when the child closes its end,
// and the supervisor's stdin write thread cannot signal the child to exit (e.g.
// after Ctrl-]d → DisconnectNamedPipe → empty stdin pipe → child sees EOF on stdin).
if let Some(ref mut pipes) = detached_stdio {
    unsafe { pipes.close_child_ends(); }
}

// Return the parent-end handles to the supervisor runtime so it can build the bridges.
Ok(WindowsSupervisedChild::Native {
    process,
    _thread: thread,
    detached_stdio,  // NEW field — see Example 4
})
```

### Example 3: Friendly busy-error translation in run_attach

```rust
// In: crates/nono-cli/src/session_commands_windows.rs
// Replaces the .map_err block at lines 391-400.
//
// Source: docs.rs std::io::Error::raw_os_error + grep on existing pattern
// at crates/nono/src/supervisor/socket_windows.rs:648-651
// [VERIFIED: grep]

use windows_sys::Win32::Foundation::ERROR_PIPE_BUSY;

let pipe_file = std::fs::OpenOptions::new()
    .read(true)
    .write(true)
    .open(&data_pipe_name)
    .map_err(|e| {
        if e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) {
            // Single-attach enforcement: the named pipe was created with
            // nMaxInstances=1 (supervisor.rs:165). A second attach attempt
            // gets ERROR_PIPE_BUSY (231) from the OS.
            NonoError::AttachBusy
        } else {
            NonoError::Setup(format!(
                "Failed to connect to session data pipe {}: {}",
                data_pipe_name, e
            ))
        }
    })?;
```

The `AttachBusy` `Display` impl is `"Session already has an active attached client"` ([VERIFIED: error.rs:143-144]). The exact wording in CONTEXT.md D-08 (`Use 'nono detach <id>' to release the existing client first`) is more user-friendly; consider either:
- (a) Wrapping in a `NonoError::Setup(format!("Session {id} is already attached. Use 'nono detach {id}' to release the existing client first."))`, OR
- (b) Adding a session-id field to `AttachBusy` and updating `Display`.

Recommend (a) for minimum cross-platform-impact change (D-21 Windows-invariance). The friendly message lives at the call site, the variant stays generic.

### Example 4: WindowsSupervisedChild gains an stdio field

```rust
// In: crates/nono-cli/src/exec_strategy_windows/supervisor.rs
// Modify WindowsSupervisedChild::Native at lines 58-64.

#[derive(Debug)]
pub(super) enum WindowsSupervisedChild {
    Native {
        process: OwnedHandle,
        _thread: OwnedHandle,
        /// Parent-end stdio handles when the child was spawned with anonymous-pipe
        /// stdio (Windows detached path only). Lifetime extends until the child
        /// process exits — bridge threads in start_logging/start_data_pipe_server
        /// borrow these via ManuallyDrop<File::from_raw_handle>.
        detached_stdio: Option<crate::exec_strategy::launch::DetachedStdioPipes>,
    },
}
```

The runtime then accesses these via a new accessor (analogous to `pty()`):

```rust
// In WindowsSupervisorRuntime — new field + accessor.
detached_stdio: Option<DetachedStdioHandles>,  // separate from PtyPair so Drop ordering is independent

pub(super) fn detached_stdio(&self) -> Option<&DetachedStdioHandles> {
    self.detached_stdio.as_ref()
}
```

But because `DetachedStdioPipes` lives on the child (not the runtime), the cleanest plumbing is: `spawn_windows_child` returns the parent-end handles inside `WindowsSupervisedChild::Native`; `execute_supervised` extracts them and hands them to `runtime.attach_detached_stdio(...)` BEFORE `runtime.run_child_event_loop(&mut child)` is called. That call moves the handles into the runtime so the bridge threads can borrow them. Plan can choose its own structure here — both work.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|---|---|---|---|
| Detached path: no streaming, log-file-only | Detached path: anonymous-pipe stdio + supervisor-side bridge to log + attach-client | Phase 17 (this) | Closes the v2.1 attach gap; preserves the Phase 15 0xC0000142 fix; no new external deps |

**Deprecated/outdated:**
- The `tracing::info!` line at `supervisor.rs:476-479` ("nono attach output relay is a v2.1+ feature on Windows detached sessions") — replace with new wording per D-06 (streaming supported, resize not). [VERIFIED: code grep]
- REQUIREMENTS.md ATCH-01 acceptance criterion #3 (`Terminal resize ... propagates to the child via ResizePseudoConsole`) — explicitly downgraded per D-07; plan must record the deviation. [VERIFIED: REQUIREMENTS.md line 193, CONTEXT.md D-07]

## Assumptions Log

> Every claim below is tagged with confidence basis. Items marked `[ASSUMED]` need user/discuss-phase confirmation; everything else was verified via grep, codebase reads, or Microsoft Learn citations.

| # | Claim | Section | Risk if Wrong |
|---|---|---|---|
| A1 | The `bInheritHandle=1` on `SECURITY_ATTRIBUTES` AND `bInheritHandles=TRUE` on `CreateProcessW` are BOTH required (cannot use one without the other). [CITED: learn.microsoft.com — no risk] | Pattern 1, Pitfall 3 | None — well-documented Win32 contract. |
| A2 | `OpenOptions::open(r"\\.\pipe\name")` on Windows surfaces `ERROR_PIPE_BUSY` as `raw_os_error() == Some(231)`. [VERIFIED: existing socket_windows.rs:648-651 uses the same pattern in production] | Friendly busy-error translation, Pitfall 2 | Low — pattern already in production code. |
| A3 | Setting `dwFlags |= STARTF_USESTDHANDLES` with non-INVALID `hStdInput/Output/Error` is the correct way to redirect child stdio for a non-console process. [CITED: learn.microsoft.com STARTUPINFOW remarks] | Pattern 2 | Low — canonical Win32 idiom. |
| A4 | The `nMaxInstances=1` argument to `CreateNamedPipeW` at `supervisor.rs:165` enforces single-attach by causing `ConnectNamedPipe` to refuse a 2nd connection while the 1st is open, which the client sees as `ERROR_PIPE_BUSY`. [CITED: learn.microsoft.com CreateNamedPipeW remarks; VERIFIED: existing code] | Don't-hand-roll table | Low — already in production. |
| A5 | Merging stderr into stdout by passing the same `child_stdout_write` HANDLE to BOTH `hStdOutput` and `hStdError` is supported by Win32 (the kernel just routes both child fd 1 and fd 2 writes to the same pipe write end). [CITED: learn.microsoft.com — multiple production examples; common pattern] | Don't-hand-roll table, Specifics from CONTEXT.md | Low — well-established. Alternative is two separate pipes + two reader threads. |
| A6 | The PTY branch's `STARTUPINFOEXW` + `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` is mutually exclusive with `STARTF_USESTDHANDLES`. Setting both is documented to fail with `ERROR_INVALID_PARAMETER`. [ASSUMED — based on Microsoft community guidance, not directly cited; verify in plan] | Landmines § STARTUPINFOEX collision | Medium — but the current architecture never allows both branches simultaneously (PTY path is gated by `should_allocate_pty`, detached path gated by `is_windows_detached_launch`). The two are mutually exclusive at the gate, so the collision is structurally prevented. |
| A7 | `ERROR_PIPE_BUSY` is decimal 231 / `0xE7`. [CITED: learn.microsoft.com System Error Codes 0-499; VERIFIED: re-export in `windows_sys::Win32::Foundation`] | Code Example 3 | None. |
| A8 | The 4096-byte buffer used in existing PTY workers is appropriate for pipe bridging too. [VERIFIED: matches existing `start_logging`/`start_data_pipe_server`/`start_interactive_terminal_io` patterns; CONTEXT.md `<specifics>` confirms this is acceptable] | Specifics from CONTEXT.md | Low — established baseline. |

## Open Questions

1. **Should the Plan use 2 plans (impl + closeout) or 1 plan + manual smoke gate as a checklist?**
   - What we know: ROADMAP.md line 67 suggests "likely 2 plans: investigation + implementation + smoke gate". CONTEXT.md confirms investigation is COMPLETE (decisions are locked). Phase 15 used 3 plans because it had separate investigation and execution phases.
   - What's unclear: Whether the closeout (CHANGELOG, REQUIREMENTS.md acceptance #3 downgrade, doc update for "no resize on detached") deserves its own plan or can be a single closing task in the impl plan.
   - Recommendation: **2 plans.** 17-01 = implementation + unit tests + integration tests. 17-02 = manual smoke gate execution (G-01..G-04) + REQUIREMENTS.md downgrade + CHANGELOG + docs. This mirrors Phase 15's 02 / 03 split.

2. **Do we need a separate `DetachedStdioPipes` struct on `WindowsSupervisedChild::Native`, or should we collapse into `WindowsSupervisorRuntime` directly?**
   - What we know: `PtyPair` lives on the runtime, not on the child; the child only needs the process+thread handles for `WaitForSingleObject`/`TerminateProcess`. The PTY's child-end handles are closed by `CreatePseudoConsole` before child spawn.
   - What's unclear: For anonymous pipes, the child-end handles MUST live until `CreateProcessW` returns (so the child can inherit them) and then are closed by the supervisor. The parent-end handles live for the lifetime of the child. Where they're stored is a code-clarity choice.
   - Recommendation: Stash parent-end handles directly on `WindowsSupervisorRuntime` (analogous to `pty: Option<PtyPair>`). The child-end handles are local variables in `spawn_windows_child` — they don't need to outlive the function call. `WindowsSupervisedChild::Native` does NOT need a new field.

3. **Should the smoke gate (G-01..G-04) be added to `13-UAT.md` as new HV rows, or kept as a Phase-17 closeout checklist only?**
   - What we know: Phase 15 added rows to `13-UAT.md` and `15-02-SUMMARY.md`. Phase 17 closes the v2.1+ deferred item from Phase 15.
   - What's unclear: Whether v2.1 has a separate UAT doc or extends `13-UAT.md`.
   - Recommendation: Defer to plan-phase. Leaning toward "extend 13-UAT.md" since the same machine + same tester runs both gates and Phase 15 set the precedent.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|---|---|---|---|---|
| Windows 10/11 (build 17763+) | All Win32 APIs used (CreatePipe, STARTUPINFOW, CreateProcessW with handle inheritance) | ✓ (target environment is Windows 11 Enterprise 10.0.26200 per recent commits) | 10.0.26200 | None — Phase 17 is Windows-only by design |
| `windows-sys` 0.59 | `ERROR_PIPE_BUSY`, `STARTF_USESTDHANDLES`, `SetHandleInformation`, `HANDLE_FLAG_INHERIT` constants | ✓ | 0.59 (Cargo.toml workspace deps) | None |
| Cargo (Rust 1.77+) | Build | ✓ (project minimum from CLAUDE.md) | 1.77 | None |
| `nono-wfp-service` (RUNNING) | Phase 15 G-04 row 4 (kernel-network-blocking) | ✓ on smoke-gate host (assumed; matches Phase 15 baseline) | n/a | None — same baseline as Phase 15 |
| `ping`, `cmd.exe` | Smoke gate G-01, G-02, G-03 reproduction commands | ✓ (built-in Windows tools) | n/a | None |

**Missing dependencies with no fallback:** None — Phase 17 is purely a code-extension within the existing Windows build/test environment.

**Missing dependencies with fallback:** None.

## Validation Architecture

> `.planning/config.json` does not appear in the repository (verified via `ls`). Per the documented default, treat `nyquist_validation` as enabled.

### Test Framework

| Property | Value |
|---|---|
| Framework | Built-in `cargo test` (Rust 1.77 std test runner) |
| Config file | `Cargo.toml` workspace defs; no separate test-config file |
| Quick run command | `cargo test -p nono-cli --bin nono -- exec_strategy_windows::detached_stdio` |
| Full suite command | `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used && cargo test --workspace --all-features` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|---|---|---|---|---|
| ATCH-01 (acc #1, G-01) | Live ping streaming through attach client | manual | n/a — `nono run --detached -- ping -t 127.0.0.1` then `nono attach <id>` | n/a (manual smoke gate) |
| ATCH-01 (acc #2, G-02) | Bidirectional cmd.exe through attach client | manual | n/a — `nono run --detached -- cmd.exe` then `nono attach <id>` | n/a (manual smoke gate) |
| ATCH-01 (acc #4, G-03) | Detach sequence releases pipe; re-attach works | integration | `cargo test -p nono-cli --bin nono -- session_commands_windows::detach_then_reattach -- --ignored` | ❌ Wave 0 (new) |
| ATCH-01 (acc #5, G-04) | Phase 15 5-row matrix unchanged | manual + existing tests | `cargo test -p nono-cli --bin nono -- restricted_token detached` (existing tests) + manual smoke | n/a (manual + existing) |
| D-08 friendly busy error | Second attach client gets `NonoError::AttachBusy` | unit | `cargo test -p nono-cli --bin nono -- session_commands_windows::translates_pipe_busy_to_attach_busy` | ❌ Wave 0 (new) |
| D-04 pipe-source bridge writes to log file | Log file accumulates child stdout | integration | `cargo test -p nono-cli --bin nono -- exec_strategy_windows::detached_stdio_writes_to_log` | ❌ Wave 0 (new) |
| D-05 pipe-sink bridge writes attach-client stdin to child | `cmd /c "echo > %1" file.txt` round-trip | integration | `cargo test -p nono-cli --bin nono -- exec_strategy_windows::detached_stdin_round_trip` | ❌ Wave 0 (new) |
| Pipe creation succeeds with inheritable flags | Three pipes created, flags correct | unit | `cargo test -p nono-cli --bin nono -- exec_strategy_windows::detached_stdio_pipes_have_correct_inheritance` | ❌ Wave 0 (new) |
| `is_windows_detached_launch` gate unchanged | Existing tests still pass | unit (regression) | `cargo test -p nono-cli --bin nono -- exec_strategy_windows::detached_token_gate_tests` | ✅ existing |

### Sampling Rate

- **Per task commit:** `cargo test -p nono-cli --bin nono -- exec_strategy_windows session_commands_windows`
- **Per wave merge:** `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used && cargo test -p nono-cli --bin nono`
- **Phase gate:** Full suite green (matching Phase 15 baseline; pre-existing fmt drift and 5 Windows test flakes documented as out-of-scope) + manual G-01..G-04 smoke executed and recorded in 17-02 SUMMARY.

### Wave 0 Gaps

- [ ] `crates/nono-cli/src/exec_strategy_windows/launch.rs` — new `#[cfg(test)] mod detached_stdio_tests` for pipe-creation unit tests
- [ ] `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — new `#[cfg(test)] mod detached_stdio_bridge_tests` for bridge-thread shape tests (mock pipes via `CreatePipe` directly + verify ReadFile/WriteFile loop)
- [ ] `crates/nono-cli/src/session_commands_windows.rs` — new `#[cfg(test)] mod attach_busy_translation_tests` for `ERROR_PIPE_BUSY` → `NonoError::AttachBusy` translation
- [ ] `crates/nono-cli/tests/` integration test (Windows-only, `#[cfg(target_os = "windows")]`): spawn `nono run --detached -- cmd /c "echo X"`, attach, verify `X` reaches stdout via the attach pipe. Marked `#[ignore]` for CI by default, runnable locally with `--ignored`.
- Framework install: none required — built-in `cargo test`.

## Security Domain

> `security_enforcement` setting absent from any visible config; treat as enabled per default.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---|---|---|
| V2 Authentication | no | Phase 17 inherits the Phase 15 attach-pipe SDDL; no new auth surface. |
| V3 Session Management | yes | Existing per-session named-pipe naming (`\\.\pipe\nono-data-<id>`) and `nMaxInstances=1` enforce single-attach. Preserved. |
| V4 Access Control | yes | SDDL `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)` on the named pipe (existing, unchanged). The anonymous pipes between supervisor and child are NOT exposed via name — only via inherited handle. |
| V5 Input Validation | yes | Stdin bytes from the attach client flow through to the child unchanged. The child runs sandboxed under the Phase 15 token+job+WFP regime; supervisor does not interpret the stream. No injection risk at the supervisor (no shell evaluation). |
| V6 Cryptography | no | No new crypto. Existing `subtle::ConstantTimeEq` for session token comparison is unchanged. |
| V7 Errors & Logging | yes | Child stdout/stderr is written verbatim to the per-session log file; no token/secret material is generated by Phase 17 itself. The log file already exists from Phase 15; no new exposure. |

### Known Threat Patterns for Anonymous-Pipe Stdio

| Pattern | STRIDE | Standard Mitigation |
|---|---|---|
| Handle leak via `bInheritHandles=TRUE` exposing supervisor-private handles to child | Information Disclosure | `SetHandleInformation(parent_end, HANDLE_FLAG_INHERIT, 0)` on supervisor-side handles immediately after `CreatePipe`. Pattern documented in code Example 1. [CITED: learn.microsoft.com] |
| Second attach client races for the data pipe | Tampering | Existing `nMaxInstances=1` on `CreateNamedPipeW` (supervisor.rs:165) — kernel rejects the 2nd `ConnectNamedPipe` until the 1st disconnects. |
| Attach client sends crafted bytes to bypass sandbox | Elevation of Privilege | The child runs under the Phase 15 token+job+WFP regime; stdin bytes can only do what the child's binary allows. Supervisor does not interpret stdin. The sandbox boundary is unchanged. |
| Pipe-busy error message leaks session enumeration | Information Disclosure | The friendly error mentions the user-supplied session id (which the user already knows). No additional information is disclosed. |
| Forgotten `CloseHandle` on parent-end handles after child exits | Resource Exhaustion (DoS local) | `Drop` impl on `DetachedStdioPipes` ensures every handle is closed exactly once, even on supervisor panic. Pattern matches `OwnedHandle` and `PtyPair`. |

## Plan-Decomposition Recommendation

**Recommendation: 2 plans.**

### Plan 17-01: Implementation + Tests (~7 tasks)

| Task | What | Where | Tests |
|---|---|---|---|
| T1 | Add `DetachedStdioPipes` struct + helpers | `launch.rs` (new struct + 1 helper fn) | Unit: pipe creation succeeds, parent-end inheritability is OFF, child-ends are ON |
| T2 | Wire stdio into `spawn_windows_child` STARTUPINFOW + flip CreateProcessW `bInheritHandles` | `launch.rs` (modify existing function) | Unit (regression): non-detached path unchanged; existing 12 tests still pass |
| T3 | Thread parent-end handles from `WindowsSupervisedChild` → `WindowsSupervisorRuntime` | `launch.rs` + `supervisor.rs` + `mod.rs::execute_supervised` | Compile-only (lifetime correctness validated by borrow-checker) |
| T4 | Extend `start_logging`: pipe-source branch (read stdout, write to log + mirror to active_attachment) | `supervisor.rs` (modify existing function) | Integration (Windows-only `#[ignore]`): detached `cmd /c "echo X"` produces `X` in log file |
| T5 | Extend `start_data_pipe_server`: pipe-sink branch (read named pipe, write child stdin) | `supervisor.rs` (modify existing function) | Integration (Windows-only `#[ignore]`): bidirectional round-trip via attach client |
| T6 | Update `tracing::info!` line at supervisor.rs:476-479 for new state per D-06 | `supervisor.rs` (1-line change) | None (cosmetic) |
| T7 | Translate `ERROR_PIPE_BUSY` → `NonoError::AttachBusy` (with friendly Setup-wrap message per D-08) in `run_attach` | `session_commands_windows.rs` (modify existing function) | Unit: simulate ERROR_PIPE_BUSY, verify wrapping |

CI gate per task: `cargo test -p nono-cli --bin nono -- exec_strategy_windows session_commands_windows`. Phase gate: full clippy + test workspace.

### Plan 17-02: Smoke Gate + Closeout (~4 tasks)

| Task | What | Where |
|---|---|---|
| T1 | Execute G-01..G-04 manual smoke gate; record evidence | `17-02-SUMMARY.md` § "Smoke gate" with PowerShell transcripts |
| T2 | Downgrade REQUIREMENTS.md ATCH-01 acceptance #3 (resize) per D-07 | `.planning/REQUIREMENTS.md` line 193 with note pointing to 17-02 SUMMARY |
| T3 | CHANGELOG `[Unreleased]` entry for ATCH-01 closure | `CHANGELOG.md` |
| T4 | Add `docs/cli/attach.md` note (or equivalent — TBD by inspection) for "no resize on detached sessions; use `nono shell` or non-detached `nono run` for full TUI fidelity" per D-06 | `docs/cli/attach.md` (or equivalent — recommend reusing existing CLI doc if no dedicated attach.md exists) |

This split keeps the implementation atomic (single PR/commit chain), defers the manual smoke gate to its own plan (matching Phase 15's 02→03 split), and isolates the documentation/closeout from code review of the implementation.

**Why not 1 plan:** Mixing manual smoke gate evidence into the implementation plan obscures whether the code passed CI vs. passed user-facing behavior. Phase 15's split worked well; replicate it.

**Why not 3 plans:** No investigation phase needed — CONTEXT.md locked all decisions. The "investigation + implementation + smoke gate" framing in ROADMAP.md predates the discuss-phase step that produced CONTEXT.md.

## Sources

### Primary (HIGH confidence)
- [VERIFIED] `crates/nono-cli/src/exec_strategy_windows/launch.rs` (1382 lines) — `spawn_windows_child` + `is_windows_detached_launch` + STARTUPINFO[EX]W shape
- [VERIFIED] `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` (1100+ lines) — `start_logging` (463-546), `start_data_pipe_server` (767-837), `WindowsSupervisorRuntime`, `SendableHandle`, `active_attachment`
- [VERIFIED] `crates/nono-cli/src/exec_strategy_windows/mod.rs` (700+ lines) — `execute_supervised` orchestration, runtime initialization order
- [VERIFIED] `crates/nono-cli/src/pty_proxy_windows.rs` (133 lines) — `CreatePipe` precedent, `PtyPair` Drop ordering, `SECURITY_ATTRIBUTES` shape
- [VERIFIED] `crates/nono-cli/src/session_commands_windows.rs` (363-460) — `run_attach` client-side scrollback + Ctrl-]d + current `OpenOptions::open` failure path
- [VERIFIED] `crates/nono-cli/src/supervised_runtime.rs` (88-94) — `should_allocate_pty` Phase 15 gate (read-only, preserved)
- [VERIFIED] `crates/nono-cli/src/startup_runtime.rs` (1-139) — outer probe loop and detached-launch banner
- [VERIFIED] `crates/nono/src/error.rs` (140-145) — `NonoError::AttachBusy` variant
- [VERIFIED] `crates/nono/src/supervisor/socket_windows.rs` (640-665) — established pattern for `ERROR_PIPE_BUSY` detection via `raw_os_error()`
- [VERIFIED] `crates/nono-cli/src/test_env.rs` (1-67) — `EnvVarGuard` + `lock_env` shape for unit tests
- [VERIFIED] `.planning/phases/17-attach-streaming/17-CONTEXT.md` — locked decisions D-01..D-08 + smoke gate G-01..G-04
- [VERIFIED] `.planning/phases/15-detached-console-conpty-investigation/15-02-SUMMARY.md` and `15-03-SUMMARY.md` — Phase 15 prior art, 5-row smoke matrix, security waivers
- [VERIFIED] `.planning/REQUIREMENTS.md` (170-197) — ATCH-01 spec including the resize criterion that gets downgraded
- [VERIFIED] `.planning/ROADMAP.md` (61-67) — Phase 17 scope statement and plan-count guidance
- [VERIFIED] `.planning/debug/resolved/windows-supervised-exec-cascade.md` — full 0xC0000142 investigation; Phase 17 must not invalidate any conclusion
- [VERIFIED] `CLAUDE.md` — workspace standards (no-unwrap, NonoError, EnvVarGuard, DCO)

### Secondary (HIGH confidence — official docs)
- [CITED] Microsoft Learn — Anonymous Pipes: https://learn.microsoft.com/en-us/windows/win32/ipc/anonymous-pipes
- [CITED] Microsoft Learn — Creating a Child Process with Redirected Input and Output: https://learn.microsoft.com/en-us/windows/win32/procthread/creating-a-child-process-with-redirected-input-and-output
- [CITED] Microsoft Learn — STARTUPINFOW (dwFlags = STARTF_USESTDHANDLES): https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/ns-processthreadsapi-startupinfow
- [CITED] Microsoft Learn — CreateProcessW (bInheritHandles parameter): https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw
- [CITED] Microsoft Learn — System Error Codes (ERROR_PIPE_BUSY = 231): https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499-
- [CITED] Microsoft Learn — CreateNamedPipeW (nMaxInstances): https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createnamedpipew
- [CITED] Microsoft Learn — SetHandleInformation (HANDLE_FLAG_INHERIT): https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-sethandleinformation

### Tertiary (LOW confidence — needs validation)
- None. All claims either verified in codebase or cited from learn.microsoft.com.

## Project Constraints (from CLAUDE.md)

The planner MUST verify each Phase 17 task complies with these directives:

- **No `.unwrap()` / `.expect()` in non-test code.** Enforced by `clippy::unwrap_used` in CI. New helpers must propagate errors via `?` and `NonoError`.
- **`NonoError` is the only error type.** Use existing variants (`NonoError::SandboxInit`, `NonoError::Setup`, `NonoError::CommandExecution`, `NonoError::AttachBusy`) — do NOT add new variants for Phase 17 unless absolutely necessary (recommended: zero new variants).
- **Unsafe code wrapping FFI must have `// SAFETY:` docs** explaining handle lifetime and inheritance assumptions. Match the existing `// SAFETY:` style in `launch.rs` and `supervisor.rs`.
- **Path security:** N/A for Phase 17 (no new path-handling logic).
- **`zeroize` for sensitive data:** N/A for Phase 17 (no new secrets; child stdout is not secret material).
- **Tests that mutate env vars must use `EnvVarGuard` + `lock_env`.** The existing `is_windows_detached_launch` tests at `launch.rs:1192-1221` are the precedent — match exactly.
- **DCO sign-off (`Signed-off-by: Name <email>`) on every commit.** Recall: the project's `windows-squash` branch enforces this.
- **`#[cfg(target_os = "windows")]` discipline:** All new code lives in `*_windows.rs` files (already cfg-gated by parent module). No `#[cfg]` annotations in cross-platform files.
- **Defense in depth:** The named-pipe SDDL stays unchanged; the anonymous-pipe handle non-inheritability flips for parent ends; both are belt-and-suspenders.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all Win32 symbols already imported in workspace; learn.microsoft.com verified
- Architecture: HIGH — pattern is a structural mirror of existing PTY bridge code; no novel design
- Pitfalls: HIGH — every pitfall is either documented in learn.microsoft.com or backed by a grep on existing production code
- Plan decomposition: MEDIUM — 2 plans recommended based on Phase 15 precedent; 1 plan defensible if user prefers atomicity
- Manual-smoke-gate scope: HIGH — G-01..G-04 from CONTEXT.md are the load-bearing acceptance criteria; mirrors Phase 15

**Research date:** 2026-04-19
**Valid until:** 2026-05-19 (30 days for stable Win32 APIs that have been frozen since Vista). The risk window is the `windows-sys` crate version — if it bumps to 0.60+ between now and execution, the imports may need adjustment.

## RESEARCH COMPLETE
