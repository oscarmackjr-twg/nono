# Phase 17: Attach-Streaming (ATCH-01) — Pattern Map

**Mapped:** 2026-04-19
**Files analyzed:** 6 modified + 2-3 new test surfaces (≈8 surfaces)
**Analogs found:** 8 / 8 (all in-codebase, all in `*_windows.rs` files)

## File Classification

| Surface | New/Modified | Role | Data Flow | Closest Analog | Match Quality |
|---------|--------------|------|-----------|----------------|---------------|
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` (`spawn_windows_child`, lines 951-1178; new `DetachedStdioPipes` struct) | MODIFY + new struct | child-process launcher (Win32 FFI wrapper) | spawn-time pipe creation + handle inheritance into child | `pty_proxy_windows::open_pty()` (lines 41-87) — same `CreatePipe` + `SECURITY_ATTRIBUTES` shape | exact (role + data-flow) |
| `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` (`start_logging`, lines 463-546) | MODIFY (extend pty_output_read==0 early-return) | IPC bridge thread (child-stdout → log-file + named-pipe attach) | unidirectional, child→supervisor→{log,attach} | existing PTY branch in same function (lines 483-543) | exact (literal mirror — pipe replaces PTY) |
| `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` (`start_data_pipe_server`, lines 767-837) | MODIFY (extend pty_input_write==0 early-return) | IPC bridge thread (named-pipe attach → child-stdin) | unidirectional, attach→supervisor→child | existing PTY branch in same function (lines 785-833) | exact (literal mirror — pipe replaces PTY) |
| `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` (`WindowsSupervisorRuntime` struct + `initialize`, lines 186-328) | MODIFY (add `detached_stdio: Option<DetachedStdioPipes>` field + accessor) | runtime state holder | parent-end handle ownership (Drop = CloseHandle) | `pty: Option<crate::pty_proxy::PtyPair>` field at line 204 + `pty()` accessor at 972-974 | exact (parallel field) |
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` (`execute_supervised`, lines 615-742) | MODIFY (thread parent-end handles into runtime) | orchestration entry point | spawn → handoff → event-loop | existing `pty_pair: Option<PtyPair>` plumbing through `WindowsSupervisorRuntime::initialize` (mod.rs:659-665, runtime.pty() at 707) | exact (parallel plumb) |
| `crates/nono-cli/src/session_commands_windows.rs` (`run_attach`, lines 363-461; `OpenOptions::open` at 391-400) | MODIFY (translate `ERROR_PIPE_BUSY` → `NonoError::AttachBusy`) | client-side error mapper | client→supervisor pipe-open failure-path | `crates/nono/src/supervisor/socket_windows.rs:647-651` — exact `raw_os_error() == Some(ERROR_PIPE_BUSY as i32)` precedent | exact (copy/adapt 5 lines) |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` `#[cfg(test)] mod detached_stdio_tests` (NEW) | CREATE | unit test | env-var-guarded pipe-creation tests | existing `mod detached_token_gate_tests` at lines 1192-1221 (uses `EnvVarGuard` + `lock_env`) | exact (same module, same primitives) |
| `crates/nono-cli/tests/attach_streaming_integration.rs` (NEW, optional) | CREATE | Windows-only `#[ignore]`d integration test | spawn detached → attach → verify byte round-trip | `crates/nono-cli/tests/wfp_port_integration.rs` (lines 1-90) — same `#![cfg(target_os = "windows")]` + `is_elevated()` skip + `#[ignore]` shape | role-match (same harness shape, different verb) |

## Pattern Assignments

### `crates/nono-cli/src/exec_strategy_windows/launch.rs` — new `DetachedStdioPipes` struct

**Role:** child-process launcher / FFI wrapper
**Analog:** `crates/nono-cli/src/pty_proxy_windows.rs::open_pty()` (lines 41-87) and `PtyPair`/`Drop` (lines 11-31)
**Extension point:** new top-level `pub(super) struct DetachedStdioPipes` declared near `OwnedHandle`/`ProcessContainment` in `launch.rs` (after line 31), plus a helper `create()` constructor and `Drop` impl. New code is invoked from inside `spawn_windows_child` (line 951) at the start of the existing `} else {` branch at line 1108 (the non-PTY `STARTUPINFOW` path).

**Imports pattern** (copy verbatim from `pty_proxy_windows.rs` lines 1-8):

```rust
use nono::{NonoError, Result};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::System::Pipes::CreatePipe;
// ADD for Phase 17:
use windows_sys::Win32::Foundation::{HANDLE_FLAG_INHERIT, SetHandleInformation};
use windows_sys::Win32::System::Threading::STARTF_USESTDHANDLES;
```

`launch.rs` already imports its Win32 symbols via `use super::*;` (line 1) — so the new pipe-related symbols must be added to the `mod.rs` `pub use` glob OR imported locally inside `launch.rs`. Match the existing precedent in `pty_proxy_windows.rs` (direct `use windows_sys::...` at file top) for the new struct.

**`CreatePipe` + SECURITY_ATTRIBUTES + error-cleanup pattern** (copy from `pty_proxy_windows.rs:41-61`):

```rust
pub fn open_pty() -> Result<PtyPair> {
    let mut h_input_read: HANDLE = INVALID_HANDLE_VALUE;
    let mut h_input_write: HANDLE = INVALID_HANDLE_VALUE;
    let mut h_output_read: HANDLE = INVALID_HANDLE_VALUE;
    let mut h_output_write: HANDLE = INVALID_HANDLE_VALUE;

    let sa = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: std::ptr::null_mut(),
        bInheritHandle: 0, // Do NOT inherit  <-- Phase 17 FLIPS to 1, then per-handle SetHandleInformation
    };

    unsafe {
        if CreatePipe(&mut h_input_read, &mut h_input_write, &sa, 0) == 0 {
            return Err(NonoError::Setup("Failed to create input pipe".to_string()));
        }
        if CreatePipe(&mut h_output_read, &mut h_output_write, &sa, 0) == 0 {
            CloseHandle(h_input_read);
            CloseHandle(h_input_write);
            return Err(NonoError::Setup("Failed to create output pipe".to_string()));
        }
        // ... pty_proxy_windows continues with CreatePseudoConsole; Phase 17
        // instead does: SetHandleInformation(parent_end, HANDLE_FLAG_INHERIT, 0)
    }
}
```

**Drop impl pattern** (copy from `pty_proxy_windows.rs:17-31`):

```rust
impl Drop for PtyPair {
    fn drop(&mut self) {
        unsafe {
            if self.hpcon != 0 {
                ClosePseudoConsole(self.hpcon);
            }
            if self.input_write != INVALID_HANDLE_VALUE {
                CloseHandle(self.input_write);
            }
            if self.output_read != INVALID_HANDLE_VALUE {
                CloseHandle(self.output_read);
            }
        }
    }
}
```

Apply the same shape to `DetachedStdioPipes::Drop` for all six handles (3 parent-end + 3 child-end-may-still-be-set-pre-CreateProcess), guarding each with `!= INVALID_HANDLE_VALUE && !is_null()` per the conservative `OwnedHandle` precedent at `launch.rs:9-19`.

---

### `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `spawn_windows_child` STARTUPINFOW + bInheritHandles wiring

**Analog:** the **existing PTY branch** in the same file (lines 1009-1107) plus the **non-PTY branch** at lines 1108-1149.
**Extension point:** at the start of the `} else {` branch at line 1108, conditionally create `DetachedStdioPipes` when `is_windows_detached_launch == true`. Modify the `CreateProcessW`/`CreateProcessAsUserW` call at lines 1118-1146 to (a) set `STARTF_USESTDHANDLES` on `startup_info.dwFlags` and populate the three `hStd*` fields; (b) flip `bInheritHandles` (5th arg, currently `0` at lines 1124 and 1140) to `1` when pipes are wired.

**Existing non-PTY STARTUPINFOW shape** (lines 1108-1148, the block to extend):

```rust
} else {
    let mut startup_info: STARTUPINFOW = unsafe {
        // SAFETY: STARTUPINFOW is a plain Win32 FFI struct; zero-init is valid.
        std::mem::zeroed()
    };
    startup_info.cb = size_of::<STARTUPINFOW>() as u32;

    if !h_token.is_null() {
        unsafe {
            // SAFETY: All pointers are valid for the duration of the call.
            CreateProcessAsUserW(
                h_token,
                application_name.as_ptr(),
                command_line.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,                                                  // <-- bInheritHandles: 0 today, 1 when pipes wired
                CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
                environment_block.as_mut_ptr() as *mut _,
                current_dir_u16.as_ptr(),
                &startup_info,
                &mut process_info,
            )
        }
    } else {
        unsafe {
            // SAFETY: All pointers are valid for the duration of the call.
            CreateProcessW(
                application_name.as_ptr(),
                command_line.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,                                                  // <-- bInheritHandles: 0 today, 1 when pipes wired
                CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
                environment_block.as_mut_ptr() as *mut _,
                current_dir_u16.as_ptr(),
                &startup_info,
                &mut process_info,
            )
        }
    }
};
```

**Detached-launch gate** (already exists at line 976, reuse):

```rust
let is_windows_detached_launch = is_windows_detached_launch();
```

**Where to close child-end handles** — between `CreateProcessW`/`CreateProcessAsUserW` (line 1146) and `resume_contained_process` at line 1172. The order MUST be: `CreateProcessW` succeeds → child-end handles closed in supervisor → `apply_process_handle_to_containment` → `apply_resource_limits` → `resume_contained_process`. The current `Ok(WindowsSupervisedChild::Native { process, _thread: thread })` at lines 1174-1177 must be extended to thread the parent-end handles back to the caller (`execute_supervised`) — recommended path per RESEARCH.md Open Question #2: stash directly on `WindowsSupervisorRuntime` after `spawn_windows_child` returns; do NOT add a field to `WindowsSupervisedChild::Native`.

---

### `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — `start_logging` pipe-source branch

**Role:** IPC bridge thread (child stdout/stderr → log file + active attach client)
**Analog:** the **existing PTY branch** in the same function (lines 483-543).
**Extension point:** replace the `if pty_output_read == 0 { tracing::info!(...); return Ok(()); }` early-return at lines 472-481 with a guarded dispatch — when `detached_stdio.is_some()`, run a parallel pipe-source thread; when both are `None`, keep the existing `tracing::info!` (now reworded per D-06).

**Imports already in scope** (no new imports needed): `std::io::{Read, Write}`, `std::mem::ManuallyDrop`, `std::os::windows::io::FromRawHandle`, `windows_sys::Win32::Storage::FileSystem::WriteFile`. Verified at supervisor.rs lines 2-4.

**Core PTY-source pattern to mirror** (lines 483-543):

```rust
std::thread::spawn(move || {
    let log_path = match crate::session::session_log_path(&session_id) {
        Ok(path) => path,
        Err(e) => {
            tracing::error!("Failed to resolve log path for session {}: {}", session_id, e);
            return;
        }
    };

    let mut log_file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        Ok(file) => file,
        Err(e) => {
            tracing::error!("Failed to open log file {}: {}", log_path.display(), e);
            return;
        }
    };

    let mut pty_file =
        ManuallyDrop::new(unsafe { std::fs::File::from_raw_handle(pty_output_read as _) });

    let mut buf = [0u8; 4096];
    while let Ok(n) = pty_file.read(&mut buf) {
        if n == 0 {
            break;
        }

        // Write to log file
        let _ = log_file.write_all(&buf[..n]);
        let _ = log_file.flush();

        // On Windows, writing to a named pipe that has no listener will block
        // if we try to write to it directly. We use a shared handle for the active attachment.
        let attachment_handle = {
            let lock = active_attachment.lock().unwrap_or_else(|p| p.into_inner());
            *lock
        };

        if let Some(sendable) = attachment_handle {
            let mut written = 0;
            // SAFETY: sendable.0 is a valid named pipe handle while it's in active_attachment.
            // We use the raw Win32 WriteFile to avoid taking ownership or blocking too long.
            unsafe {
                windows_sys::Win32::Storage::FileSystem::WriteFile(
                    sendable.0,
                    buf.as_ptr(),
                    n as u32,
                    &mut written,
                    std::ptr::null_mut(),
                );
            }
        }
    }
});
```

**Phase 17 pipe-source extension instructions:**
- Substitute `pty_output_read` → `stdout_read` (parent end of the stdin/stdout/stderr trio held on `WindowsSupervisorRuntime.detached_stdio`).
- Optionally spawn a second identical thread for `stderr_read` writing to the same `log_file` + `active_attachment` — OR collapse stderr into stdout by passing the same `child_stdout_write` HANDLE for both `hStdOutput` and `hStdError` at spawn time (RESEARCH.md A5 confirms this is supported). CONTEXT.md `<specifics>` recommends merging for visual consistency with the PTY path.
- Buffer size MUST stay `[u8; 4096]` (RESEARCH.md A8, CONTEXT.md `<specifics>`).
- Keep the `let _ = log_file.write_all(...)` best-effort discard pattern — never `?`-propagate inside the bridge thread (Pitfall 1).
- Keep the raw-FFI `WriteFile` for the attach mirror — never go through `File::write_all` on the named-pipe handle (the existing pattern at lines 530-540 explains why: avoid blocking and ownership conflicts with `active_attachment`).

**`tracing::info!` startup line** (lines 476-479) — UPDATE per D-06 to communicate streaming-supported / resize-not-supported on the detached path. Example replacement wording: `"Detached supervisor: child stdout/stderr streamed to log + attach client via anonymous pipes (resize not supported on detached path; use 'nono shell' for full TUI fidelity)"`.

---

### `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — `start_data_pipe_server` pipe-sink branch

**Role:** IPC bridge thread (named-pipe attach client → child stdin)
**Analog:** the **existing PTY branch** in the same function (lines 785-833).
**Extension point:** replace the `if pty_output_read == 0 || pty_input_write == 0 { return Ok(()); }` early-return at lines 781-783 with a guarded dispatch — when `detached_stdio.is_some()`, run a parallel pipe-sink thread; when neither is wired, keep the early-return.

**Core PTY-sink pattern to mirror** (lines 785-834):

```rust
std::thread::spawn(move || {
    let pipe_name = format!("\\\\.\\pipe\\nono-data-{}", session_id);
    let h_pipe = match create_secure_pipe(&pipe_name) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to create supervisor data pipe: {}", e);
            return;
        }
    };

    loop {
        let connected = unsafe { ConnectNamedPipe(h_pipe, std::ptr::null_mut()) };
        if connected != 0
            || unsafe { GetLastError() }
                == windows_sys::Win32::Foundation::ERROR_PIPE_CONNECTED
        {
            {
                let mut lock = active_attachment.lock().unwrap_or_else(|p| p.into_inner());
                *lock = Some(SendableHandle(h_pipe));
            }

            // For input, we read from the pipe and write to PTY input.
            // This thread will block on pipe reading while the client is attached.
            let mut file = unsafe { std::fs::File::from_raw_handle(h_pipe as _) };
            let mut pty_input = ManuallyDrop::new(unsafe {
                std::fs::File::from_raw_handle(pty_input_write as _)
            });

            let mut buf = [0u8; 4096];
            while let Ok(n) = file.read(&mut buf) {
                if n == 0 {
                    break;
                }
                if pty_input.write_all(&buf[..n]).is_err() {
                    break;
                }
            }

            {
                let mut lock = active_attachment.lock().unwrap_or_else(|p| p.into_inner());
                if let Some(sendable) = *lock {
                    if sendable.0 == h_pipe {
                        *lock = None;
                    }
                }
            }
            unsafe { DisconnectNamedPipe(h_pipe) };
        }
    }
});
```

**Phase 17 pipe-sink extension instructions:**
- Substitute `pty_input_write` → `stdin_write` (parent write-end on `detached_stdio.stdin_write`).
- Reuse `create_secure_pipe(&pipe_name)` verbatim — SDDL `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)` (line 136) and `nMaxInstances=1` (line 165) are unchanged. Single-attach enforcement is a structural property of `CreateNamedPipeW`.
- The `active_attachment` mutex semantics are preserved exactly — same set/clear pattern.
- The `ManuallyDrop<File>` pattern around the parent-end stdin handle prevents double-close when the runtime drops (matches PTY pattern at line 809).

---

### `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — `WindowsSupervisorRuntime` field + accessor

**Role:** runtime state holder; owns parent-end stdio handles for the lifetime of the child.
**Analog:** existing `pty: Option<crate::pty_proxy::PtyPair>` field at line 204 + `pty()` accessor at lines 972-974.

**Field declaration pattern** (line 204):

```rust
pty: Option<crate::pty_proxy::PtyPair>,
// Phase 17: add adjacent
detached_stdio: Option<crate::exec_strategy::launch::DetachedStdioPipes>,
```

**Accessor pattern** (lines 972-974):

```rust
pub(super) fn pty(&self) -> Option<&crate::pty_proxy::PtyPair> {
    self.pty.as_ref()
}
// Phase 17: parallel accessor
pub(super) fn detached_stdio(&self) -> Option<&crate::exec_strategy::launch::DetachedStdioPipes> {
    self.detached_stdio.as_ref()
}
```

**Initialization plumbing** (lines 282-307 in `WindowsSupervisorRuntime::initialize`):

```rust
let mut runtime = Self {
    session_id: supervisor_session_id,
    user_session_id,
    requested_features: ...,
    transport_name,
    _parent_control: parent_control,
    child_control: Some(child_control),
    started_at,
    state: WindowsSupervisorLifecycleState::Initializing,
    audit_log: Vec::new(),
    terminate_requested,
    pty,                          // <-- Phase 15 field
    active_attachment,
    interactive_shell: supervisor.interactive_shell,
    session_token: supervisor.session_token.map(str::to_string),
    cap_pipe_rendezvous_path: supervisor.cap_pipe_rendezvous_path.map(|p| p.to_path_buf()),
    audit_rx: None,
    child_process_for_broker: Arc::new(Mutex::new(None)),
    approval_backend: supervisor.approval_backend.clone(),
    timeout_deadline,
    containment_job,
    // Phase 17: detached_stdio: None,  <-- populated post-spawn via attach_detached_stdio()
};

runtime.start_control_pipe_server()?;
if runtime.interactive_shell {
    runtime.start_interactive_terminal_io()?;
} else {
    runtime.start_logging()?;
    runtime.start_data_pipe_server()?;
}
```

**Phase 17 plumbing decision** (per RESEARCH.md Open Question #2): the cleanest path is to add an `initialize` parameter `detached_stdio: Option<DetachedStdioPipes>` parallel to the existing `pty: Option<PtyPair>` parameter (line 261). The detached_stdio is created **inside** `spawn_windows_child` BEFORE `CreateProcessW` and threaded back via the return value, OR (alternative) created in `execute_supervised` via a new helper `should_create_detached_stdio()` and passed into both `spawn_windows_child` AND `WindowsSupervisorRuntime::initialize`. The second approach matches the existing `pty_pair` plumbing style at mod.rs:620 and keeps all "should-allocate" decisions co-located.

---

### `crates/nono-cli/src/exec_strategy_windows/mod.rs` — `execute_supervised` plumbing

**Role:** orchestration entry point — composes containment, runtime, child, event loop.
**Analog:** existing `pty_pair: Option<pty_proxy::PtyPair>` parameter (line 620) and its plumb through `WindowsSupervisorRuntime::initialize(supervisor, pty_pair, ...)` (line 661) and `runtime.pty()` (line 707).

**Existing plumb pattern** (lines 615-665, 702-711):

```rust
pub fn execute_supervised(
    config: &ExecConfig<'_>,
    supervisor: Option<&SupervisorConfig<'_>>,
    _trust_interceptor: Option<crate::trust_intercept::TrustInterceptor>,
    _on_fork: Option<&mut dyn FnMut(u32)>,
    pty_pair: Option<pty_proxy::PtyPair>,                // <-- Phase 15 parameter
    session_id: Option<&str>,
    audit_state: Option<AuditState>,
    rollback_state: Option<RollbackRuntimeState>,
    ...
) -> Result<i32> {
    ...
    let mut runtime = WindowsSupervisorRuntime::initialize(
        supervisor,
        pty_pair,                                        // <-- threaded into runtime
        session_id,
        timeout_deadline,
        containment.job,
    )?;
    ...
    let mut child = spawn_windows_child(
        config,
        launch_program,
        &containment,
        &cmd_args,
        runtime.pty(),                                   // <-- borrowed back from runtime for child spawn
        limits,
        session_id,
    )
    .map_err(|err| runtime.startup_failure(err.to_string()))?;
```

**Phase 17 plumb instructions:**
- Compute `let detached_stdio = if is_windows_detached_launch() && pty_pair.is_none() { Some(DetachedStdioPipes::create()?) } else { None };` BEFORE `WindowsSupervisorRuntime::initialize`.
- Pass `detached_stdio` as a new parameter to `WindowsSupervisorRuntime::initialize` (the runtime takes ownership).
- `spawn_windows_child` borrows the parent-end handles via `runtime.detached_stdio()` (parallel to `runtime.pty()`) — but since `STARTUPINFOW.hStdInput/Output/Error` need the **child-end** handles, the runtime accessor returns `Option<&DetachedStdioPipes>` and `spawn_windows_child` reads `pipes.stdin_read`, `pipes.stdout_write`, `pipes.stderr_write` for `STARTUPINFOW`, then `unsafe { pipes.close_child_ends() }` after `CreateProcessW` returns successfully.

**Watch-out (RESEARCH.md Pitfall 5):** Do NOT reorder `start_control_pipe_server` / `start_logging` / `start_data_pipe_server` in `WindowsSupervisorRuntime::initialize` (lines 309-315). The pipe-source branch is added INSIDE `start_logging`; no reordering is required.

---

### `crates/nono-cli/src/session_commands_windows.rs` — `run_attach` ERROR_PIPE_BUSY translation

**Role:** client-side error mapper.
**Analog:** `crates/nono/src/supervisor/socket_windows.rs:647-651` — production precedent for `raw_os_error() == Some(ERROR_PIPE_BUSY as i32)` detection.
**Extension point:** the `.map_err` block at lines 391-400 of `session_commands_windows.rs`.

**Existing client-side pipe-open pattern** (lines 391-400):

```rust
let pipe_file = std::fs::OpenOptions::new()
    .read(true)
    .write(true)
    .open(&data_pipe_name)
    .map_err(|e| {
        NonoError::Setup(format!(
            "Failed to connect to session data pipe: {}. Is another client already attached?",
            e
        ))
    })?;
```

**Production ERROR_PIPE_BUSY detection precedent** (`socket_windows.rs:647-651`):

```rust
let err = std::io::Error::last_os_error();
if matches!(
    err.raw_os_error(),
    Some(code) if code == ERROR_PIPE_BUSY as i32 || code == ERROR_FILE_NOT_FOUND as i32
) {
    // ... retry / wait pattern
}
```

**Phase 17 patch instructions:**
- Add `use windows_sys::Win32::Foundation::ERROR_PIPE_BUSY;` to the imports at the top of `session_commands_windows.rs` (line 1-9 region).
- Replace the lines 391-400 `.map_err` body with a match on `e.raw_os_error()`:
  - `Some(code) if code == ERROR_PIPE_BUSY as i32` → `NonoError::Setup(format!("Session {} is already attached. Use 'nono detach {}' to release the existing client first.", session.session_id, session.session_id))` (D-08 wording, wraps in `Setup` per RESEARCH.md Code Example 3 recommendation (a) — minimum cross-platform-impact change, no new variant).
  - everything else → existing message (kept).
- DO NOT add a new `NonoError` variant — `NonoError::AttachBusy` already exists at `error.rs:144` but its `Display` (`"Session already has an active attached client"`) does not include the session id; the friendlier wrapped `Setup` form keeps the variant generic per CONTEXT.md D-21 invariance.

**Pre-flight `WaitNamedPipeW`** at line 388 — keep verbatim. It only converts a transient "pipe doesn't exist yet" into a 1-second wait; ERROR_PIPE_BUSY still surfaces from `OpenOptions::open` afterwards.

---

### `crates/nono-cli/src/exec_strategy_windows/launch.rs` — new `#[cfg(test)] mod detached_stdio_tests`

**Role:** unit tests
**Analog:** existing `mod detached_token_gate_tests` in the same file at lines 1192-1221.

**Existing test-module pattern** (lines 1192-1221):

```rust
#[cfg(test)]
mod detached_token_gate_tests {
    use super::is_windows_detached_launch;
    use crate::test_env::{lock_env, EnvVarGuard};

    #[test]
    fn returns_false_when_env_unset() {
        let _lock = lock_env();
        // Ensure the env var is cleared for the duration of the assertion.
        let g = EnvVarGuard::set_all(&[("NONO_DETACHED_LAUNCH", "1")]);
        g.remove("NONO_DETACHED_LAUNCH");
        assert!(!is_windows_detached_launch());
    }

    #[test]
    fn returns_true_when_env_is_one() {
        let _lock = lock_env();
        let _g = EnvVarGuard::set_all(&[("NONO_DETACHED_LAUNCH", "1")]);
        assert!(is_windows_detached_launch());
    }

    #[test]
    fn returns_false_when_env_is_other_value() {
        let _lock = lock_env();
        let _g = EnvVarGuard::set_all(&[("NONO_DETACHED_LAUNCH", "0")]);
        assert!(!is_windows_detached_launch());
        let _g2 = EnvVarGuard::set_all(&[("NONO_DETACHED_LAUNCH", "true")]);
        assert!(!is_windows_detached_launch());
    }
}
```

**Phase 17 test instructions:**
- Place new tests in a sibling module (`mod detached_stdio_tests`) at the bottom of `launch.rs`.
- Tests that don't mutate env vars (e.g., pipe-creation success, parent-end inheritability assertions via `GetHandleInformation`) can skip `lock_env()`.
- Tests that depend on `is_windows_detached_launch()` returning a specific value MUST acquire `lock_env()` and use `EnvVarGuard` per the precedent above (CLAUDE.md "Environment variables in tests" rule).
- All new tests gated on `#[cfg(target_os = "windows")]` — implicit because the module is in `*_windows.rs`.
- No `.unwrap()` in non-`#[allow(clippy::unwrap_used)]` contexts (CLAUDE.md). The existing module above does NOT use `unwrap` — match.

---

### `crates/nono-cli/src/session_commands_windows.rs` — new `#[cfg(test)] mod attach_busy_translation_tests`

**Role:** unit test for `ERROR_PIPE_BUSY` → friendly error mapping.
**Analog:** No existing in-file test module in `session_commands_windows.rs` — borrow shape from `detached_token_gate_tests` above (no env vars needed; just `std::io::Error::from_raw_os_error(231)` synthesis).

**Recommended test shape:**

```rust
#[cfg(test)]
mod attach_busy_translation_tests {
    use super::*;
    use windows_sys::Win32::Foundation::ERROR_PIPE_BUSY;

    /// Verify that the helper used by run_attach maps an io::Error with
    /// raw_os_error() == ERROR_PIPE_BUSY to a friendly Setup error containing
    /// the session id and a hint about `nono detach`.
    #[test]
    fn translates_pipe_busy_to_attach_busy() {
        let err = std::io::Error::from_raw_os_error(ERROR_PIPE_BUSY as i32);
        let translated = translate_attach_open_error(&err, "abc123");
        let msg = format!("{}", translated);
        assert!(msg.contains("abc123"));
        assert!(msg.contains("already attached"));
        assert!(msg.contains("nono detach"));
    }

    #[test]
    fn passes_through_other_errors() {
        let err = std::io::Error::from_raw_os_error(2); // ERROR_FILE_NOT_FOUND
        let translated = translate_attach_open_error(&err, "abc123");
        let msg = format!("{}", translated);
        assert!(msg.contains("Failed to connect"));
    }
}
```

This implies extracting the `.map_err` body into a `fn translate_attach_open_error(e: &std::io::Error, session_id: &str) -> NonoError` helper for testability — recommended pattern.

---

### `crates/nono-cli/tests/attach_streaming_integration.rs` (NEW, optional)

**Role:** Windows-only `#[ignore]`d integration test
**Analog:** `crates/nono-cli/tests/wfp_port_integration.rs` (lines 1-90).

**Existing integration-test header pattern** (`wfp_port_integration.rs:1-31`):

```rust
//! Integration test: WFP port-level permit filter allows real TCP connections.
//!
//! Requires:
//! - Windows OS
//! - Administrator privileges (WFP filter installation) for the `#[ignore]`d test
//! - nono-wfp-service running (or test will skip gracefully) for the `#[ignore]`d test
//!
//! Run the policy-compilation test (no privileges required):
//!
//!   cargo test -p nono-cli --test wfp_port_integration
//!
//! Run the full WFP real-connection test (admin + running wfp-service required):
//!
//!   cargo test -p nono-cli --test wfp_port_integration -- --ignored

#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]

use std::net::{TcpListener, TcpStream};
use std::time::Duration;
```

**`#[ignore]`d test shape** (lines 53-58):

```rust
#[test]
#[ignore] // Requires admin privileges and a running nono-wfp-service
fn wfp_port_permit_allows_real_tcp_connection() {
    if !is_elevated() {
        eprintln!("SKIP: wfp_port_permit test requires administrator privileges");
        return;
    }
    // ...
}
```

**Phase 17 instructions:**
- Match `#![cfg(target_os = "windows")]` + `#![allow(clippy::unwrap_used)]` headers exactly.
- Mark the round-trip test `#[ignore]` since it spawns a real `nono run --detached -- cmd /c "echo X"` and requires the `nono` binary to be built. Local invocation: `cargo test -p nono-cli --test attach_streaming_integration -- --ignored`.
- No admin-privilege check needed (Phase 17 doesn't touch WFP) — but a `nono` binary path discovery helper IS needed; reuse the standard pattern (`env!("CARGO_BIN_EXE_nono")` macro for cargo test contexts).

## Shared Patterns

### Pattern S1 — `SendableHandle` for cross-thread HANDLE passing

**Source:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:30-33`
**Apply to:** All new bridge threads in `start_logging`/`start_data_pipe_server` that move HANDLEs across thread boundaries.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SendableHandle(pub HANDLE);

unsafe impl Send for SendableHandle {}
unsafe impl Sync for SendableHandle {}
```

The Phase 17 pipe-source thread receives the parent-end stdout/stderr HANDLEs via the same wrapper inside the closure capture, OR via raw `usize` cast inside the closure (matching the existing `let pty_output_read = ... as usize` pattern at supervisor.rs:467 — preferred for symmetry).

### Pattern S2 — `ManuallyDrop<File::from_raw_handle>` for borrowed-handle ergonomics

**Source:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:508-509, 673-677, 697-701, 808-811`
**Apply to:** ALL pipe-bridge threads that need `Read`/`Write` ergonomics on a borrowed HANDLE.

```rust
let mut pty_file =
    ManuallyDrop::new(unsafe { std::fs::File::from_raw_handle(pty_output_read as _) });
```

The runtime's `Drop` impl (or the `DetachedStdioPipes::Drop`) is responsible for the actual `CloseHandle`. The bridge thread's `ManuallyDrop` wrapper prevents double-close. Required EVERY time a raw HANDLE is wrapped in `File` for read/write inside a thread that does not own the handle.

### Pattern S3 — `// SAFETY:` doc-comment style on every `unsafe` block

**Source:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:13-15, 25-26, 64-67, 73-74, 84-85` and dozens more
**Apply to:** Every new `unsafe { ... }` block introduced by Phase 17 — `CreatePipe`, `SetHandleInformation`, `CloseHandle`, `WriteFile`, `File::from_raw_handle`.

Example shape:

```rust
unsafe {
    // SAFETY: `parent_stdin_write` was just returned by CreatePipe with
    // bInheritHandle=1 in the SECURITY_ATTRIBUTES; flipping it to 0 here
    // ensures the supervisor-side write end is not inherited by the child.
    SetHandleInformation(parent_stdin_write, HANDLE_FLAG_INHERIT, 0);
}
```

CLAUDE.md mandates the `// SAFETY:` doc; the project enforces this stylistically (no clippy lint, but every existing FFI call in `launch.rs` and `supervisor.rs` has one).

### Pattern S4 — Best-effort discard on bridge-thread I/O failures

**Source:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:518-519, 528-541, 689` (PTY branches)
**Apply to:** ALL new pipe-bridge thread I/O calls.

```rust
let _ = log_file.write_all(&buf[..n]);
let _ = log_file.flush();
// And for the named-pipe attach mirror, raw FFI WriteFile with ignored return code.
```

NEVER use `?` or panic inside the bridge thread (Pitfall 1: writing to a disconnected named pipe returns `ERROR_NO_DATA (232)` or `ERROR_BROKEN_PIPE (109)`; the bridge MUST keep reading from the child).

### Pattern S5 — `active_attachment: Arc<Mutex<Option<SendableHandle>>>` for single-attach broadcast

**Source:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:205, 470, 522-526, 802-803, 824-829`
**Apply to:** Phase 17 reuses this primitive verbatim. The pipe-source bridge consults it on every read; the pipe-sink bridge sets/clears it on connect/disconnect. NO new mutex; NO change to lock semantics; NO change to `nMaxInstances=1` enforcement.

### Pattern S6 — `NonoError` discipline (no new variants for Phase 17)

**Source:** `crates/nono/src/error.rs:140-145` (`SessionNotFound`, `AttachBusy` already exist)
**Apply to:** All Phase 17 error paths — use `NonoError::SandboxInit` (FFI failures), `NonoError::Setup` (user-facing setup errors with friendly wording), `NonoError::CommandExecution` (child-spawn / runtime errors). Per RESEARCH.md "Project Constraints" and Code Example 3 recommendation (a), do NOT add a session-id-bearing variant of `AttachBusy` — wrap in `Setup` at the call site instead.

### Pattern S7 — DCO sign-off on every commit + `windows-squash` branch convention

**Source:** `CLAUDE.md` "Commits" rule + recent commit history (`git log --oneline -5`)
**Apply to:** Every commit produced by Phase 17 plans MUST include `Signed-off-by: <name> <email>`. Recent commit subject style on `windows-squash`: `docs(state): record quick task ...`, `docs(cli): document Phase 20 ...` — Phase 17 commits should follow `feat(attach):` / `feat(supervisor):` / `test(supervisor):` prefixes.

## No Analog Found

| Surface | Reason |
|---------|--------|
| (none) | Every Phase 17 surface has a direct codebase analog. The only "novel" element is the `DetachedStdioPipes` struct, which is structurally a 3-pair variant of `PtyPair` and uses the same `CreatePipe` + `SECURITY_ATTRIBUTES` + `Drop`-with-CloseHandle pattern already proven in `pty_proxy_windows.rs`. |

## Metadata

**Analog search scope:**
- `crates/nono-cli/src/exec_strategy_windows/` (launch.rs, mod.rs, supervisor.rs)
- `crates/nono-cli/src/pty_proxy_windows.rs`
- `crates/nono-cli/src/session_commands_windows.rs`
- `crates/nono-cli/src/supervised_runtime.rs`
- `crates/nono-cli/src/test_env.rs`
- `crates/nono-cli/tests/wfp_port_integration.rs`
- `crates/nono/src/error.rs`
- `crates/nono/src/supervisor/socket_windows.rs`

**Files scanned:** 8
**Pattern extraction date:** 2026-04-19
**Confidence:** HIGH — every analog is in-codebase, every line number verified by `Read` of the source file, every reference cross-checked against RESEARCH.md citations.
