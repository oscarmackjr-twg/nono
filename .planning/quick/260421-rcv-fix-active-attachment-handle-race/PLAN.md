---
task: fix-active-attachment-handle-race
type: bug-fix
severity: high (handle-reuse / data-corruption risk)
source: gemini-code-assist PR review
created: 2026-04-21
---

# Quick Task: Fix race in logging thread WriteFile to active_attachment pipe

## Problem

gemini-code-assist on PR #725 flagged `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` (v2.0-pr lines 460-478; windows-squash lines 681-703):

> There is a race condition between the logging thread and the data pipe server. The logging thread clones the raw handle from active_attachment and uses it in WriteFile after releasing the lock. If the data pipe server closes the handle (by dropping the File wrapper) before WriteFile is called, the logging thread will use a closed handle. This can lead to errors or, in the case of handle reuse by the OS, data corruption of an unrelated resource. Consider holding the lock during the write operation or using DuplicateHandle to provide the logging thread with its own owned handle.

**The race is real:**
- `start_logging` (line ~667): reads child stdout, writes to log file, then reads `active_attachment` under a brief lock to find the attach-pipe HANDLE, drops the lock, calls raw `WriteFile(sendable.0, ...)`.
- `start_data_pipe_server` (line ~968): on attach-client disconnect, acquires the SAME mutex to clear `active_attachment = None`, releases the lock, then drops its `File` wrapper → `CloseHandle(h_pipe)`.

**Interleaving that corrupts:**
1. Logging thread locks, reads `Some(h_pipe)`, unlocks.
2. Pipe-sink thread locks, clears to `None`, unlocks, drops `File`, `CloseHandle(h_pipe)`.
3. Kernel recycles the HANDLE numeric value for an unrelated kernel object (file, pipe, event, whatever) created concurrently by any thread in the process.
4. Logging thread calls `WriteFile(h_pipe, buf, n, ...)` — writes `n` bytes of child stdout into the recycled resource. Silent data corruption.

## Fix

Hold `active_attachment`'s mutex guard across the `WriteFile` call. The pipe-sink thread MUST re-acquire the same mutex before clearing the slot, so while the logging thread holds the guard, the pipe-sink thread cannot drop its File wrapper → cannot close the HANDLE → cannot trigger the recycle.

This is gemini's first suggestion. Rejected gemini's second suggestion (DuplicateHandle) because it requires redesigning the attach lifecycle (separate handle ownership for logging-thread copy + when to close it + who re-dups on reattach) — substantially more invasive for the same correctness gain.

### Contention concern (not actually a concern)

- The pipe-sink thread only holds the lock during `*lock = Some(...)` (attach) and `*lock = None` (disconnect). Microseconds.
- The logging thread now holds the lock across one `WriteFile` on a named pipe with a local attach client. Named pipes in PIPE_WAIT mode CAN block if the pipe buffer is full + no drain, but the attach client is the nono CLI reading continuously — buffer should drain quickly. If it doesn't, `WriteFile` returns `ERROR_BROKEN_PIPE` fast (pipe structurally detects disconnected readers).
- Worst case: logging thread hangs briefly holding the lock while attach client is stuck. Pipe-sink thread's `*lock = None` on disconnect is delayed. But if the client is stuck, the disconnect hasn't happened anyway — there's nothing to delay.

### Deadlock concern (not a concern)

The pipe-sink thread's code path under the lock is straight-line (set the Option). It doesn't acquire any other lock. No reciprocal lock acquisition possible.

## Implementation

File: `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` (one function: `start_logging`, near line 681 on windows-squash / 461 on v2.0-pr).

**Before:**
```rust
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
```

**After:**
```rust
// Hold active_attachment across WriteFile so the pipe-sink thread cannot
// clear the slot + drop its File wrapper (closing the HANDLE, which the OS
// may then recycle for an unrelated resource) between our lookup and the
// write. Pipe-sink only holds this mutex for the brief swap-in / swap-out
// of the Option, so contention is microsecond-scale.
let lock = active_attachment.lock().unwrap_or_else(|p| p.into_inner());
if let Some(sendable) = *lock {
    let mut written = 0;
    // SAFETY: sendable.0 is a valid named-pipe HANDLE for the duration of
    // this WriteFile — we hold active_attachment's mutex above, and the
    // pipe-sink thread must re-acquire the same mutex before clearing the
    // slot + dropping the File wrapper that would close the HANDLE. Raw
    // FFI WriteFile (vs File::write_all) lets us discard ERROR_NO_DATA /
    // ERROR_BROKEN_PIPE without killing the bridge (Pitfall 1).
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
```

(Lock drops at end of the enclosing `while let` iteration body, before the next `source_file.read` call.)

## Verification

- `cargo build --workspace` passes on Windows
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` passes
- `cargo fmt --all -- --check` passes
- Structural check: `grep -n 'attachment_handle = {' crates/nono-cli/src/exec_strategy_windows/supervisor.rs` returns 0 hits (old two-step pattern gone)
- Existing integration test `crates/nono-cli/tests/attach_streaming_integration.rs` still passes (Windows-only, runs on host)

## Non-goals

- Do NOT refactor to `DuplicateHandle`. Simpler fix is sufficient per gemini's suggestion set.
- Do NOT fix the adjacent lifecycle issue I noticed in `start_data_pipe_server` (line 997 uses `File::from_raw_handle` without `ManuallyDrop`, so on client disconnect `file` drops and closes `h_pipe`, breaking subsequent `ConnectNamedPipe` iterations). This is a separate latent bug and outside the scope of this PR comment — flag to user for a follow-up.
- Do NOT touch `active_attachment` elsewhere. Only the WriteFile race in `start_logging` is in scope.

## Propagation

The hazardous code exists on BOTH v2.0-pr and v2.1-pr (Phase 02 — Persistent Sessions introduced the pattern; Phase 17 ATCH-01 added a `detached_stdio` branch to the source-handle side but didn't touch the attachment mirror).

After landing on `windows-squash`:
1. Cherry-pick + amend onto `v2.0-pr` (strip `.planning/`), force-push.
2. Rebase `v2.1-pr` onto updated `v2.0-pr`, force-push.
3. Reply to the PR #725 thread(s) and mark resolved.
