---
slug: 17-detached-child-immediate-exit
status: resolved
trigger: Phase 17 plan 17-01 implementation regression — detached child exits immediately on new pipe-stdio path
created: 2026-04-19
updated: 2026-04-19
phase: 17
related_commits: [1e38381, 2b74d66, 9c82f17, f17ad72, f962606, 03e1e80]
---

# Debug Session: 17-detached-child-immediate-exit

## Symptoms

**Expected:** `nono run --detached --allow-cwd -- ping -t 127.0.0.1` starts a long-running detached child; `nono attach <id>` connects and streams live ping replies.

**Actual:** Child exits immediately. `nono attach <id>` returns `Setup error: Session <id> has already exited.` (User-confirmed for both `ping -t 127.0.0.1` and `cmd.exe`.)

**Error:** `Setup error: Session 076bccf377808f8c has already exited.` (logged at ERROR level by `nono attach`).

**Timeline:** Started immediately after Phase 17 plan 17-01 landed (commits `1e38381`..`ecfeba7` on `windows-squash`, 2026-04-19). Phase 15 5-row matrix worked fine before this.

**Reproduction:**
```powershell
cargo build -p nono-cli --release
.\target\release\nono.exe run --detached --allow-cwd -- ping -t 127.0.0.1
# Note session id, then immediately:
.\target\release\nono.exe attach <session_id>
# EXPECTED: live ping replies stream in
# ACTUAL: "Setup error: Session <id> has already exited."
```

## Suspects (investigated in order — all eliminated)

H1..H8 all related to `STARTUPINFOW` / `CreateProcessW` / pipe wiring on the detached path were eliminated by reading `launch.rs` lines 1118-1399 and confirming the wiring is textbook-correct (matches RESEARCH.md A1/A5/A6, Pitfall 1/3/5):

- `SECURITY_ATTRIBUTES.bInheritHandle = 1` on pipe creation, then parent ends flipped non-inheritable via `SetHandleInformation(HANDLE_FLAG_INHERIT, 0)` — correct (T-17-01 / T-17-08).
- `STARTF_USESTDHANDLES` set; `hStdInput=stdin_read`, `hStdOutput/hStdError=stdout_write` — correct.
- `STARTUPINFOW.cb = size_of::<STARTUPINFOW>()` — correct.
- `bInheritHandles = 1` on the detached-pipe branch, `0` on the PTY branch — correct.
- Plain `STARTUPINFOW` (not EX) — no STARTUPINFOEXW collision with the PTY branch.
- Null-token preserved on the detached path; falls through to `CreateProcessW` (not `CreateProcessAsUserW`) — correct.
- `close_child_ends()` called AFTER `CreateProcess` returns success and BEFORE `ResumeThread` — correct ordering.
- 5/5 `detached_stdio_tests` passed before fix — confirms pipe layout is correct.

The regression had **nothing to do with the pipe wiring**. The grandchild was actually being spawned correctly and writing to the pipe. The smoking gun was found in the diagnostic data, not the spawn code.

## Files of Interest

- `crates/nono-cli/src/exec_strategy_windows/launch.rs:200-208` — **`create_process_containment` job-name format string corrupted** (pre-Phase-17 latent bug exposed by Phase 17).
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:843` — **`start_data_pipe_server` used wrong session ID** (pre-Phase-17 latent bug newly exposed).
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:502` — **`start_logging` used wrong session ID** (same pattern, same fix).

## Resolution

Three minimal surgical fixes, all in `*_windows.rs` files (D-21 invariance preserved). Phase 15 0xC0000142 fix preserved byte-identically (`should_allocate_pty` gate untouched, no ConPTY on detached path).

### Root Cause

Three independent pre-existing bugs that were NEVER exercised before Phase 17 because Phase 15 deferred `nono attach` on detached Windows to v2.1+ (per Phase 15 VERIFICATION.md). Phase 17 is v2.1, and the first time `nono attach` ran end-to-end on a detached Windows session, all three bugs surfaced as symptoms that masqueraded as a Phase 17 regression.

#### Bug 1 — corrupted job-object name (the load-bearing one)

`launch.rs:200-208` (since commit `13f9ca3`, the original Windows native exec commit):

```rust
let name = format!(
    r"Local
ono-session-{}",         // <-- LITERAL NEWLINE between Local and ono
    id
);
```

The intended literal was `Local\nono-session-{id}` (raw string with two characters `\` + `n` between `Local` and `ono`), matching the form used at `supervised_runtime.rs:136` (`format!(r"Local\nono-session-{}", short_session_id)`) which writes the value into the session JSON's `job_object_name` field.

Disk bytes confirmed: position 15 of the format string was `0x0a` (newline), NOT `0x5c 0x6e` (backslash + n). So the actual job object created via `CreateJobObjectW` was named `Local<NEWLINE>ono-session-XYZ` while the session JSON record stored `Local\nono-session-XYZ`.

Downstream consequence: `session::reconcile_session_record()` calls `is_job_object_active(job_name)` on every session load (including by `nono attach`, `nono ps`, etc.). `OpenJobObjectW` is called with the JSON-derived name (backslash + n), but the job is named with a newline character. **`OpenJobObjectW` returns NULL**, `is_job_object_active` returns `false`, and reconcile flips the session record to `status: Exited`, `exit_code: -1` — even though the supervisor and grandchild are alive and well.

This was reproducible on the host: `nono ps` reported `No running sessions` while `tasklist` showed two live `nono.exe` plus two live `ping.exe`, and `~/.nono/sessions/supervised-XXX-YYY.log` was being actively written with ping replies.

**Fix:**
```rust
let name = format!(r"Local\nono-session-{}", id);
```
Single-line raw-string form (single backslash + n on disk), byte-identical to `supervised_runtime.rs:136`.

#### Bug 2 — data pipe named with wrong session ID

`supervisor.rs:843` `start_data_pipe_server`:

```rust
let session_id = self.session_id.clone();   // supervisor correlation ID
...
let pipe_name = format!("\\\\.\\pipe\\nono-data-{}", session_id);
```

But `nono attach` (`session_commands_windows.rs:415`) connects to:
```rust
let data_pipe_name = format!("\\\\.\\pipe\\nono-data-{}", session.session_id);
```
where `session.session_id` is the user-facing 16-hex ID.

The supervisor created the data pipe at `\\.\pipe\nono-data-supervised-PID-NANOS`, but `nono attach` looked for `\\.\pipe\nono-data-{16hex}`. After fixing Bug 1 the symptom became:

> `Failed to connect to session data pipe: The system cannot find the file specified. (os error 2). Is another client already attached?`

The control pipe at `start_control_pipe_server` already used `user_session_id` correctly; this is exactly the pattern Phase 15 / 17 RESEARCH.md flagged. The data pipe call site was the missed parallel.

**Fix:** Use `self.user_session_id.clone()` so the supervisor and the `nono attach` client reference the same pipe name.

#### Bug 3 — log file path used wrong session ID (same root pattern)

`supervisor.rs:502` `start_logging`:

```rust
let session_id = self.session_id.clone();   // supervisor correlation ID
...
let log_path = match crate::session::session_log_path(&session_id) { ... };
```

`nono attach` (`session_commands_windows.rs:405`) reads scrollback from `crate::session::session_log_path(&session.session_id)` (user-facing ID). Same wrong-ID pattern as Bug 2: the supervisor wrote to `~/.nono/sessions/supervised-PID-NANOS.log` while `nono attach` looked at `~/.nono/sessions/{16hex}.log`. Without Bug 3 fixed, the post-fix attach showed live streaming output but **no scrollback** (because the scrollback file was at the wrong path).

**Fix:** Use `self.user_session_id.clone()`.

`start_interactive_terminal_io` (line 715) has the same `let session_id = self.session_id.clone()` pattern and writes to `session_log_path` for `nono shell`. It was left untouched because (a) interactive shell uses the local terminal directly so the secondary log is non-load-bearing, and (b) the scope of this debug session is the detached + attach path. Flagged as a follow-up but not a regression.

### Fix Verified

Re-ran on Windows host (commit-pending):

```
G-01: nono run --detached --allow-cwd -- ping -t 127.0.0.1
      nono ps     -> running, PIDs visible
      nono attach -> live ping replies stream in (with scrollback). PASS
G-02: nono run --detached --allow-cwd -- cmd.exe
      nono ps     -> running, PIDs visible
      nono attach -> Microsoft Windows banner, prompt, attach succeeds. PASS

cargo test -p nono-cli --bin nono detached            13/13 PASS (incl. 5 detached_stdio + 3 token_gate)
cargo clippy -p nono-cli --all-targets -- -D warnings -D clippy::unwrap_used   clean
```

User-driven G-03 / G-04 (broader manual smoke matrix) still pending on Windows host per the original interactive-debug request.

### Constraints Honored

- Phase 15 `0xC0000142` fix preserved byte-identically (`should_allocate_pty` gate at `supervised_runtime.rs:88-94` untouched; no ConPTY on detached path).
- D-21 Windows-invariance — only `*_windows.rs` files modified (`launch.rs` + `supervisor.rs`).
- CLAUDE.md: no `.unwrap()` outside `#[cfg(test)]`, NonoError + `?`, all `// SAFETY:` blocks preserved, no env-mutating tests added.
- Surgical: 1 line changed in `launch.rs`, 2 lines changed in `supervisor.rs` (plus correlated comment blocks). No rewrites.

## Current Focus

```yaml
hypothesis: RESOLVED
test: G-01 + G-02 verified PASS on Windows host (commit-pending)
expecting: -
next_action: ask user to re-run G-01..G-04 manually + commit when satisfied
```

## Evidence

- 2026-04-19 17:00 — User reproduction: `ping -t 127.0.0.1` detached + `nono attach <id>` -> "Session ... has already exited" within seconds. G-01 FAIL.
- 2026-04-19 17:00 — User reproduction: `cmd.exe` detached + `nono attach <id>` -> same "Session ... has already exited". G-02 FAIL.
- Phase 15 5-row matrix (the same `ping -t` and `cmd /c "echo hello"` commands) PASSED on commits `802c958` + `2c414d8` (pre-Phase-17). So the regression *appeared to be* introduced by 17-01.
- 2026-04-19 13:11 — On-host repro confirmed at PID 74160 inner supervisor: `tasklist` shows nono.exe (74160) AND ping.exe (14780, 35704) all alive AND `supervised-74160-NANOS.log` actively growing with ping replies, BUT `nono ps` shows "No running sessions" and the session JSON has been rewritten to `status: "exited", exit_code: -1`. This eliminated H1..H8 (the child IS running) and pointed at `reconcile_session_record`.
- 2026-04-19 13:13 — `od -c` on `launch.rs:202` confirmed disk byte 15 of the format string is `0x0a` (newline), not the intended `0x5c 0x6e` (backslash + n). Bug isolated.
- 2026-04-19 13:14 — git blame: bug present since `13f9ca3` (initial Windows native exec commit). NOT a Phase 17 regression — Phase 15 just never exercised the codepath that exposed it.
- 2026-04-19 13:23 — Bug 1 fix applied + rebuilt. `nono ps` now reports `running` for live detached session. `nono attach` no longer says "already exited" but fails on data pipe lookup.
- 2026-04-19 13:25 — Bug 2 + Bug 3 fixes applied (data pipe + log file both use `user_session_id`). Rebuilt.
- 2026-04-19 13:32 — G-01 verified PASS: `nono attach <id>` shows live ping output with scrollback.
- 2026-04-19 13:34 — G-02 verified PASS: `nono attach <id>` connects to `cmd.exe`, shows banner + prompt.
- 2026-04-19 13:36 — All 13 detached-related unit tests pass; clippy `-D warnings -D clippy::unwrap_used` clean.

## Eliminated

- H1 SECURITY_ATTRIBUTES.bInheritHandle on child-end pipe halves: confirmed correct (=1 in DetachedStdioPipes::create, then parent ends flipped to 0 via SetHandleInformation). 5/5 unit tests pass.
- H2 CreateProcessW(bInheritHandles=TRUE): confirmed correct on detached-pipe branch (=1 via `let inherit_handles = if detached_stdio.is_some() { 1 } else { 0 }`).
- H3 STARTUPINFOW.dwFlags missing STARTF_USESTDHANDLES: confirmed set on detached-pipe branch.
- H4 STARTUPINFOW.cb wrong size: confirmed `size_of::<STARTUPINFOW>()`.
- H5 SetHandleInformation applied to wrong end: confirmed applied to parent ends only (`stdin_write`, `stdout_read`, `stderr_read`).
- H6 STARTUPINFOEXW vs STARTUPINFOW collision: separate `let created = if pty {EX} else {plain W}` branches; no collision.
- H7 Null-token interaction: `h_token = null_mut()` on detached path falls through to `CreateProcessW` (not `CreateProcessAsUserW`). Phase 15 fix preserved.
- H8 Outer probe falsely registering child failure as fast-exit success: irrelevant — the issue was diagnostic-side reconciliation, not actual child failure.
