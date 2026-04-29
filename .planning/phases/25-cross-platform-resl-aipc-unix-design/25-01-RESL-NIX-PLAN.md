---
phase: 25-cross-platform-resl-aipc-unix-design
plan: 01
type: execute
wave: 1
depends_on: []
requirements: [RESL-NIX-01, RESL-NIX-02, RESL-NIX-03]
tags: [linux, macos, cgroup-v2, setrlimit, resource-limits, cli]
tdd: false
risk: medium
files_modified:
  - crates/nono-cli/src/exec_strategy.rs                       # Remove the four "not enforced on linux/macos" stderr warning lines from collect_unix_resource_limit_warnings; dispatch into Linux cgroup / macOS setrlimit application paths
  - crates/nono-cli/src/exec_strategy/supervisor_linux.rs      # cgroup v2 delegated-hierarchy lifecycle: detect, mkdir, enable controllers, write limits, place child PID, RAII cleanup, cgroup.kill watchdog
  - crates/nono-cli/src/exec_strategy/supervisor_macos.rs      # NEW: setrlimit application via std::process::Command::pre_exec; supervisor-side Instant deadline + kill(pgrp, SIGKILL) watchdog
  - crates/nono-cli/src/launch_runtime.rs                      # No shape change to ResourceLimits (Phase 16 Plan 01 already defines it); update doc comments noting Linux + macOS now enforce
  - crates/nono-cli/src/cli.rs                                 # Reject --cpu-percent at clap parse time on macOS via a target-gated value_parser wrapper that returns NotSupportedOnPlatform { feature: "cpu_percent_macos" }

autonomous: true

must_haves:
  truths:
    - "`nono run --memory 256m -- bash -c 'tail -c 1G </dev/urandom'` on Linux is OOM-killed by cgroup v2 memory.max — child exit code reflects SIGKILL and `nono inspect <id>` reports `memory_kill: true` (acceptance criterion 1 of REQ-RESL-NIX-01)"
    - "`nono run --cpu-percent 50 -- bash -c 'yes >/dev/null'` on Linux pegs at ~50% of one logical core — verified by `/proc/<pid>/stat` user+system time delta over a 5s window (acceptance criterion 2 of REQ-RESL-NIX-01)"
    - "`nono run --max-processes 10 -- bash -c 'for i in {1..20}; do sleep 60 & done; wait'` on Linux fails after the 10th fork with an error containing `pids.max` (acceptance criterion 3 of REQ-RESL-NIX-01)"
    - "All four `warning: --<flag> is not enforced on linux` / `... on macos` stderr lines (cpu_percent / memory / timeout / max_processes) are removed from `collect_unix_resource_limit_warnings` in `exec_strategy.rs` — verified by `grep -n 'is not enforced on' crates/nono-cli/src/` returning zero matches (acceptance criterion 4 of REQ-RESL-NIX-01)"
    - "Cgroup creation fails fast with `NonoError::UnsupportedPlatform { feature: \"cgroup_v2\" }` on cgroup v1 systems and on systems with no systemd-delegated cgroup (no `/proc/self/cgroup` v2 entry) BEFORE any child is spawned (acceptance criterion 5 of REQ-RESL-NIX-01)"
    - "`nono run --timeout 5s -- sleep 60` on Linux exits at ~5s via `cgroup.kill` write; `nono inspect <id>` reports `timeout_kill: true` (acceptance criterion 1 of REQ-RESL-NIX-02)"
    - "`nono run --timeout 5s -- bash -c 'for i in {1..100}; do sleep 60 & done; wait'` on Linux atomically kills all 100 grandchild processes at the deadline via cgroup.kill (acceptance criterion 2 of REQ-RESL-NIX-02)"
    - "`nono run --memory 256m -- bash -c '<large alloc>'` on macOS aborts via RLIMIT_AS mmap failure; the child PID is never observable without limits because setrlimit runs inside the `pre_exec` hook before `execve` (acceptance criterion 1 of REQ-RESL-NIX-03)"
    - "`nono run --max-processes 10 -- bash -c 'for i in {1..20}; do sleep 60 & done; wait'` on macOS fails after the 10th fork with `EAGAIN` from RLIMIT_NPROC (acceptance criterion 2 of REQ-RESL-NIX-03)"
    - "`nono run --cpu-percent 50 -- ls` on macOS fails at clap parse time with `NonoError::NotSupportedOnPlatform { feature: \"cpu_percent_macos\" }` — exit code is non-zero, no child is spawned, no cgroup or rlimit syscall is issued (acceptance criterion 3 of REQ-RESL-NIX-03)"
    - "Cgroup cleanup happens unconditionally via a `Drop` guard on session exit — verified by `ls /sys/fs/cgroup/<delegated>/nono-*/` on Linux returning empty after both successful and panicking sessions"
    - "`make ci` is green on Linux and macOS lanes: `cargo fmt --all --check` + `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` + `cargo test --workspace --all-features`"
  artifacts:
    - path: "crates/nono-cli/src/exec_strategy/supervisor_linux.rs"
      provides: "CgroupSession struct (RAII): detect cgroup v2 delegation under /proc/self/cgroup, mkdir <delegated>/nono-<session-id>/, enable +memory +cpu +pids in cgroup.subtree_control, write memory.max / cpu.max / pids.max from ResourceLimits, place child PID via cgroup.procs in pre_exec, atomic kill via cgroup.kill, rmdir cleanup in Drop"
      contains: "CgroupSession"
    - path: "crates/nono-cli/src/exec_strategy/supervisor_macos.rs"
      provides: "MacosResourceLimits applier: pre_exec hook calling nix::sys::resource::setrlimit(RLIMIT_AS, RLIMIT_NPROC); supervisor-side timeout watchdog with Instant deadline + kill(pgrp, SIGKILL)"
      contains: "MacosResourceLimits"
    - path: "crates/nono-cli/src/exec_strategy.rs"
      provides: "Removal of the four 'not enforced on <os>' eprintln branches; dispatch into Linux CgroupSession or macOS MacosResourceLimits at the Direct + Supervised entry points"
      contains: "is not enforced on"
    - path: "crates/nono-cli/src/cli.rs"
      provides: "Target-gated value_parser for --cpu-percent that on macOS returns NotSupportedOnPlatform { feature: \"cpu_percent_macos\" } at clap parse time; on Linux/Windows behaves as today (u16 in 1..=100)"
      contains: "cpu_percent_macos"
  key_links:
    - from: "ResourceLimits.memory_bytes / cpu_percent / max_processes / timeout (Phase 16 struct, unchanged)"
      to: "CgroupSession::apply_limits in supervisor_linux.rs"
      via: "exec_strategy::execute_supervised passing &flags.resource_limits into the Linux dispatch path"
      pattern: "CgroupSession::new.*resource_limits"
    - from: "ResourceLimits"
      to: "MacosResourceLimits::install_pre_exec in supervisor_macos.rs"
      via: "exec_strategy::execute_direct + execute_supervised passing &flags.resource_limits into Command::pre_exec at fork time"
      pattern: "MacosResourceLimits::install_pre_exec"
    - from: "Linux supervisor Instant deadline expiry"
      to: "atomic kill of cgroup descendant tree"
      via: "write \"1\\n\" to <cgroup>/cgroup.kill"
      pattern: "cgroup\\.kill"
    - from: "macOS supervisor Instant deadline expiry"
      to: "SIGKILL of child process group"
      via: "nix::sys::signal::kill(Pid::from_raw(-pgrp), Signal::SIGKILL)"
      pattern: "kill.*SIGKILL"
    - from: "cgroup v1 / no-delegation system at startup"
      to: "fail-fast NonoError::UnsupportedPlatform { feature: \"cgroup_v2\" }"
      via: "CgroupSession::detect returning Err before any child spawn"
      pattern: "NonoError::UnsupportedPlatform.*cgroup_v2"
---

<objective>
Light up real Linux + macOS enforcement for the four `nono run` resource-limit flags (`--memory`, `--cpu-percent`, `--max-processes`, `--timeout`) introduced as Windows-only Job Object enforcement in v2.1 Phase 16. Today these flags emit four "not enforced on linux/macos; native backend is a follow-up cross-platform milestone" stderr warnings (`exec_strategy.rs:54-96` per the requirement) and otherwise pass through as silent no-ops. v2.3 closes the Linux POC credibility gap by replacing the warnings with kernel-level enforcement: cgroup v2 delegated hierarchies on Linux (memory.max / cpu.max / pids.max / cgroup.kill) and `setrlimit` + supervisor watchdog on macOS (RLIMIT_AS / RLIMIT_NPROC + Instant deadline + SIGKILL).

Purpose: Close the three RESL-NIX requirements (REQ-RESL-NIX-01 Linux cgroup backends, REQ-RESL-NIX-02 Linux wall-clock timeout, REQ-RESL-NIX-03 macOS setrlimit subset). Acceptance criteria are kernel-enforced and verifiable from outside the sandbox (memory_kill flag in `nono inspect`, atomic kill of grandchild tree, fail-fast on cgroup v1).

Output: Working `nono run --memory 256m --cpu-percent 50 --max-processes 10 --timeout 5s -- <cmd>` on Linux 5.13+ with cgroup v2 delegation; `nono run --memory 256m --max-processes 10 --timeout 5s -- <cmd>` on macOS (with `--cpu-percent` rejected at clap parse time per REQ-RESL-NIX-03 criterion 3); `make ci` green on both Unix lanes; the four "not enforced" warnings deleted from `collect_unix_resource_limit_warnings`.

Locked decisions (referenced from the v2.3 milestone scope-lock; do not re-litigate):
- **Linux Approach (A): unprivileged cgroup v2 with systemd delegation.** Read `/proc/self/cgroup` to find the user's delegated cgroup; mkdir `<delegated>/nono-<session-id>/`; enable controllers via `<delegated>/cgroup.subtree_control`; write limits; move child PID to `<new>/cgroup.procs` via `pre_exec` hook; cleanup via rmdir on exit; atomic kill via `cgroup.kill`. Fail-fast on cgroup v1 / no delegation with `NonoError::UnsupportedPlatform { feature: "cgroup_v2" }`. **Do not plan privileged paths or `systemd-run` shell-out.**
- **macOS:** `setrlimit` via `nix::sys::resource` with `pre_exec` hook on `std::process::Command`. `--cpu-percent` on macOS fails closed at clap-parse time with `NonoError::NotSupportedOnPlatform { feature: "cpu_percent_macos" }`. `--timeout` enforced via supervisor-side `Instant` deadline + `kill(pgrp, SIGKILL)` watchdog (no native wall-clock rlimit; `RLIMIT_CPU` is CPU-time, not wall-clock, and is intentionally not used). Document RLIMIT_AS-vs-RSS gap explicitly in doc comments per REQ-RESL-NIX-03.
- **Style: feature-first** (`tdd: false` in frontmatter). Cgroup integration test runs as a verification gate AFTER implementation, not before. Unit tests for parser shape + ResourceLimits plumbing already exist from Phase 16 — this plan adds platform-gated integration tests.

Out of scope (explicit deferrals — do NOT plan in this file):
- AIPC Unix futures ADR — deferred to Plan 25-02 (separate invocation per the v2.3 phase scope-lock).
- cgroup v1 fallback — fail-fast only; no privileged-path nor `systemd-run` shell-out.
- Windows behavior — unchanged; v2.1 Phase 16 owns Windows enforcement.
- Inspect-side `memory_kill` / `timeout_kill` field plumbing if it does not already exist in the v2.1 Phase 16 inspect surface — if absent, that lands as a follow-up; this plan focuses on enforcement and uses existing exit-code reporting where the inspect field is missing. (Note in the SUMMARY which inspect fields needed wiring vs. were already present.)
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/25-cross-platform-resl-aipc-unix-design/25-CONTEXT.md
@.planning/phases/16-resource-limits/16-01-SUMMARY.md

<!-- Key interfaces the executor needs — extracted from codebase. -->
<!-- Use these directly; do not re-explore the codebase for these shapes. -->

<interfaces>
<!-- launch_runtime.rs: ResourceLimits ALREADY EXISTS from Phase 16 Plan 01. Do NOT change its shape. -->
<!-- File: crates/nono-cli/src/launch_runtime.rs lines 117–155 (verified by grep). -->
```rust
// crates/nono-cli/src/launch_runtime.rs (already present, do not modify shape):
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub(crate) struct ResourceLimits {
    pub(crate) cpu_percent: Option<u16>,
    pub(crate) memory_bytes: Option<u64>,
    pub(crate) timeout: Option<std::time::Duration>,
    pub(crate) max_processes: Option<u32>,
}

impl ResourceLimits {
    pub(crate) fn from_run_args(run_args: &RunArgs) -> Self { /* ... */ }
    pub(crate) fn is_empty(&self) -> bool { /* ... */ }
}

// In ExecutionFlags (line ~174):
pub(crate) resource_limits: ResourceLimits,
```

The Phase 16 plan already populates `flags.resource_limits` from `RunArgs` in `prepare_run_launch_plan`. **This plan reuses that struct unchanged.** All Linux + macOS enforcement work happens DOWNSTREAM of `flags.resource_limits` reaching the dispatch layer.

<!-- exec_strategy.rs: the warnings being removed (lines 44–102 — re-read confirms shape). -->
<!-- The four `warning: --<flag> is not enforced on <os>` eprintln branches inside -->
<!-- `collect_unix_resource_limit_warnings` are deleted by Task 8. -->
```rust
// crates/nono-cli/src/exec_strategy.rs (current shape; will be gutted):
pub(crate) fn collect_unix_resource_limit_warnings(
    limits: &crate::launch_runtime::ResourceLimits,
    silent: bool,
) -> Vec<String> {
    if silent { return Vec::new(); }
    #[cfg(not(target_os = "windows"))]
    {
        // FOUR `out.push(format!("warning: --<flag> is not enforced on {os_name}; ..."))`
        // branches — these are the lines to delete in Task 8. The function itself stays
        // (returns empty Vec) so existing callers don't break, OR is removed entirely
        // along with its call sites — see Task 8 for the choice rationale.
    }
    // ...
}

pub(crate) fn warn_unix_resource_limits(
    limits: &crate::launch_runtime::ResourceLimits,
    silent: bool,
) {
    for line in collect_unix_resource_limit_warnings(limits, silent) {
        eprintln!("{line}");
    }
}
```

<!-- exec_strategy/supervisor_linux.rs: existing Linux supervisor module structure. -->
<!-- This is the seccomp-notify supervisor; cgroup logic is a separate concern but lives in -->
<!-- the same module to keep all Linux supervisor surface in one file. The new CgroupSession -->
<!-- struct lives at the bottom of supervisor_linux.rs in its own `mod cgroup` submodule, -->
<!-- exported as `pub(super) use cgroup::CgroupSession`. -->
```rust
// crates/nono-cli/src/exec_strategy/supervisor_linux.rs (current header):
use super::*;
use crate::trust_intercept::TrustInterceptor;
use nono::AccessMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InitialCapability { /* ... */ }

// Other supervisor primitives (RateLimiter, etc.). The new CgroupSession is a
// peer struct — does NOT replace or modify any existing seccomp-notify code.
```

<!-- nix crate: setrlimit + signal::kill bindings (already in workspace per CLAUDE.md). -->
```rust
// nix::sys::resource (already imported elsewhere in the codebase):
use nix::sys::resource::{setrlimit, Resource};
// Resource::RLIMIT_AS    — address space (used for --memory on macOS)
// Resource::RLIMIT_NPROC — process count (used for --max-processes on macOS)

// nix::sys::signal::kill (already imported in exec_strategy.rs):
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
// kill(Pid::from_raw(-pgrp), Signal::SIGKILL) sends SIGKILL to the entire process group.
```

<!-- /proc/self/cgroup format (cgroup v2 systemd-delegated): -->
```text
$ cat /proc/self/cgroup
0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-foo.scope
```
A SINGLE line beginning with `0::` indicates pure cgroup v2. A line starting with a
non-zero hierarchy ID (e.g., `1:cpu:/`) or multiple lines indicates cgroup v1 or
hybrid mode — fail-fast in that case. The path AFTER `0::` is the absolute cgroup
relative to `/sys/fs/cgroup`. The supervisor's delegated cgroup is exactly that path
joined with `/sys/fs/cgroup`.

<!-- Pre-exec hook on std::process::Command (already used elsewhere for setsid): -->
```rust
use std::os::unix::process::CommandExt;
let mut cmd = std::process::Command::new(program);
unsafe {
    // SAFETY: pre_exec runs in the forked child, post-fork pre-exec.
    // The closure must be async-signal-safe — only call libc / nix syscalls;
    // do NOT allocate or take locks. Move all input data in by-value or by-Copy.
    cmd.pre_exec(move || {
        // setrlimit calls here; cgroup.procs write here on Linux.
        Ok(())
    });
}
```
</interfaces>

<dispatch_layout>
<!-- High-level shape of the new dispatch in exec_strategy.rs after this plan lands: -->
<!-- (sketch, not a literal diff — Task 8 wires it up): -->

```rust
// crates/nono-cli/src/exec_strategy.rs (new dispatch shape):
fn apply_resource_limits_unix(
    flags: &ExecutionFlags,
    cmd: &mut std::process::Command,
    session_id: &str,
) -> Result<UnixResourceLimitGuard> {
    if flags.resource_limits.is_empty() {
        return Ok(UnixResourceLimitGuard::Noop);
    }
    #[cfg(target_os = "linux")]
    {
        let cgroup = supervisor_linux::CgroupSession::new(session_id, &flags.resource_limits)?;
        cgroup.install_pre_exec(cmd);  // writes child PID to cgroup.procs
        Ok(UnixResourceLimitGuard::Linux(cgroup))
    }
    #[cfg(target_os = "macos")]
    {
        let macos = supervisor_macos::MacosResourceLimits::new(&flags.resource_limits)?;
        macos.install_pre_exec(cmd);   // setrlimit calls in pre_exec hook
        Ok(UnixResourceLimitGuard::Macos(macos))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (flags, cmd, session_id);
        Ok(UnixResourceLimitGuard::Noop)
    }
}

enum UnixResourceLimitGuard {
    Noop,
    #[cfg(target_os = "linux")]  Linux(supervisor_linux::CgroupSession),  // RAII rmdir on Drop
    #[cfg(target_os = "macos")]  Macos(supervisor_macos::MacosResourceLimits),
}

// Watchdog (called from supervisor event loop after spawn):
fn spawn_timeout_watchdog(
    deadline: std::time::Instant,
    guard: &UnixResourceLimitGuard,
    child_pgrp: nix::unistd::Pid,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let now = std::time::Instant::now();
        if let Some(remaining) = deadline.checked_duration_since(now) {
            std::thread::sleep(remaining);
        }
        // On Linux: write "1\n" to <cgroup>/cgroup.kill (atomic).
        // On macOS: kill(Pid::from_raw(-child_pgrp.as_raw()), Signal::SIGKILL).
        // ...
    })
}
```

The dispatch is called from BOTH `execute_direct` and the Supervised path. Direct gets the
guard but no watchdog (timeout is enforced supervisor-side; Direct strategy has no
supervisor). For Direct + `--timeout`, fall back to a `wait_timeout`-style polling loop
(or document that `--timeout` requires Supervised on Unix — see <risks> below).
</dispatch_layout>
</context>

<risks>
**Top-3 risks (carry into SUMMARY at end):**

1. **Non-systemd init (e.g., Alpine OpenRC, Nix init=stage1) → no cgroup delegation.** Mitigation: fail-fast with `NonoError::UnsupportedPlatform { feature: "cgroup_v2" }` at `CgroupSession::detect`. This is intentional, not a bug — REQ-RESL-NIX-01 acceptance criterion 5 mandates it. Document in CLI help text + `nono setup` output.
2. **cgroup v2 controller enablement varies by distro.** Some distros require `+memory +cpu +pids` to be written to the parent's `cgroup.subtree_control` before child cgroups can use those controllers. If the user's delegated cgroup is at `<X>` and we mkdir `<X>/nono-<id>/`, we must write `+memory +cpu +pids` to `<X>/cgroup.subtree_control` (NOT `<X>/nono-<id>/cgroup.subtree_control`) before the limits can be set in the child. Mitigation: attempt the write; on `EINVAL`, surface the error with the exact path and `cat /proc/self/cgroup` contents in the error message so the user can diagnose.
3. **Child PID placement race window.** Between `fork()` and write-to-`cgroup.procs`, the child can run uninstrumented for a brief instant. Mitigation: use the `pre_exec` hook (runs in the forked child, post-fork pre-exec) to write `getpid()` to `<cgroup>/cgroup.procs` BEFORE `execve`. The window between fork and the pre_exec hook executing is bounded by the kernel's scheduler latency, not by user-space arithmetic, and the child is a Rust runtime stub at this point — no business logic, no memory pressure. Document the residual race in the SAFETY comment on the pre_exec closure.
</risks>

<verification_gates>
- `cargo test --target x86_64-unknown-linux-gnu --workspace --all-features` clean (Linux gate).
- `cargo test --target x86_64-apple-darwin --workspace --all-features` clean (macOS gate; aarch64-apple-darwin equivalent).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` clean on both Linux and macOS.
- `cargo fmt --all -- --check` clean.
- `grep -nE 'is not enforced on (linux|macos)' crates/nono-cli/src/` returns zero matches (Task 8 removal verification).
- Manual smoke gate Linux: `nono run --memory 256m -- bash -c "tail -c 1G </dev/urandom"` exits with cgroup OOM kill (kernel emits `Memory cgroup out of memory` to dmesg).
- Manual smoke gate macOS: `nono run --cpu-percent 50 -- ls` fails at clap parse with `NotSupportedOnPlatform { feature: "cpu_percent_macos" }`.
- Manual smoke gate Linux+macOS: `nono run --timeout 5s -- sleep 60` exits at ~5s (within ±200ms).
</verification_gates>

<tasks>

<task type="auto">
  <name>Task 1: Linux cgroup v2 detection + fail-fast</name>
  <files>
    crates/nono-cli/src/exec_strategy/supervisor_linux.rs
  </files>
  <read_first>
    - `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` lines 1–60 (current module header + import shape — the new `cgroup` submodule sits at the BOTTOM of this file, not at the top)
    - `crates/nono/src/error.rs` (for `NonoError::UnsupportedPlatform { feature: String }` variant — this is the variant REQ-RESL-NIX-01 criterion 5 mandates; if it does not exist, the executor must add it before this task lands)
    - `/proc/self/cgroup` format note in `<interfaces>` (cgroup v2 = single line starting with `0::`; anything else = fail-fast)
  </read_first>
  <action>
    Add a new submodule `mod cgroup` at the bottom of `supervisor_linux.rs` containing a `CgroupSession::detect()` associated function that:

    1. Reads `/proc/self/cgroup` (use `std::fs::read_to_string`).
    2. Trims the contents and confirms there is EXACTLY ONE line.
    3. Confirms the single line starts with `0::` (the cgroup v2 hierarchy-ID marker). Anything else (e.g., `1:cpu:/...`, `2:memory:/...`, multi-line output) means cgroup v1 or hybrid — return `Err(NonoError::UnsupportedPlatform { feature: "cgroup_v2".into() })`.
    4. Extracts the path after `0::` (e.g., `/user.slice/user-1000.slice/.../app-foo.scope`) — this is the user's delegated cgroup.
    5. Joins it with `/sys/fs/cgroup` to produce the absolute delegated-cgroup path (e.g., `/sys/fs/cgroup/user.slice/.../app-foo.scope`).
    6. Verifies the path exists and is a directory (`std::fs::metadata`); if not, fail-fast with the same `UnsupportedPlatform` variant + a wrapped `io::Error` for diagnostics.
    7. Returns `Ok(PathBuf)` of the delegated path.

    Add a `#[cfg(target_os = "linux")] mod tests` block at the bottom of the new submodule covering:
    - `detect()` on a synthesized `/proc/self/cgroup` containing `0::/user.slice/...` returns `Ok(PathBuf::from("/sys/fs/cgroup/user.slice/..."))` — implemented as a `detect_from_str(s: &str) -> Result<PathBuf>` helper that the public `detect()` calls; the helper is what the unit test exercises (avoids needing a real procfs).
    - `detect_from_str("1:cpu:/foo")` returns `Err(NonoError::UnsupportedPlatform { feature: "cgroup_v2".into() })`.
    - `detect_from_str("0::/foo\n1:cpu:/foo")` returns `Err(...)` (multi-line = hybrid).
    - `detect_from_str("")` returns `Err(...)`.

    `///` doc comments on `CgroupSession::detect` documenting the cgroup v2 invariant + the fail-fast rationale + the exact error variant returned.
  </action>
  <verify>
    <automated>
      cargo fmt --all -- --check
      cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used
      cargo test -p nono-cli --bin nono -- supervisor_linux::cgroup::tests
    </automated>
    grep -n 'UnsupportedPlatform.*cgroup_v2' crates/nono-cli/src/exec_strategy/supervisor_linux.rs returns at least one match (the fail-fast site).
  </verify>
  <done>
    - `CgroupSession::detect` lives in `mod cgroup` at the bottom of `supervisor_linux.rs`.
    - `detect_from_str` helper unit-tested with three invalid inputs (cgroup v1, hybrid, empty) and one valid input.
    - On a non-systemd or cgroup-v1 host, `nono run --memory 256m -- ...` would fail at the supervisor-init phase with `NonoError::UnsupportedPlatform { feature: "cgroup_v2" }` (verified at the call site in Task 2).
    - `///` doc comments + SAFETY commentary on every fs read/syscall path.
  </done>
</task>

<task type="auto">
  <name>Task 2: Linux cgroup lifecycle — create, enable controllers, RAII cleanup</name>
  <files>
    crates/nono-cli/src/exec_strategy/supervisor_linux.rs
  </files>
  <read_first>
    - Task 1's `CgroupSession::detect` output (the delegated `PathBuf`)
    - cgroup v2 controller-enablement note in `<risks>` (controllers must be enabled in PARENT's `cgroup.subtree_control` before child cgroups can use them)
  </read_first>
  <action>
    Extend `CgroupSession` (still in `mod cgroup` at the bottom of `supervisor_linux.rs`) with:

    1. `pub(super) fn new(session_id: &str, limits: &ResourceLimits) -> Result<Self>` constructor that:
       a. Calls `Self::detect()` to get the delegated path (`<delegated>`).
       b. Constructs the new cgroup path: `<delegated>/nono-<session-id>/`.
       c. Writes `+memory +cpu +pids` to `<delegated>/cgroup.subtree_control` (read-modify-write: read current contents first to avoid clobbering, then OR the new tokens). On `EINVAL`, surface the exact path + `/proc/self/cgroup` contents in `NonoError::SandboxInit` for diagnosability.
       d. `mkdir(<delegated>/nono-<session-id>/)` via `std::fs::create_dir`. On `EEXIST` (rare — duplicate session ID), fail-fast (do NOT silently reuse — leftover state is a security bug).
       e. Stores the new cgroup path in `self.path: PathBuf`.
       f. Stores `self.limits: ResourceLimits` (clone) for use by `apply_limits` in Task 3.
       g. Stores `self.armed: bool = true` so `Drop` knows whether to rmdir.

    2. `impl Drop for CgroupSession` that, if `self.armed`:
       a. Reads `<self.path>/cgroup.procs` and confirms it is empty (no processes still in the cgroup); if non-empty, log a `warn!` with the surviving PIDs (this would indicate a supervisor bug since `cgroup.kill` should have cleared them).
       b. `rmdir(<self.path>)` via `std::fs::remove_dir`. Errors logged via `warn!` but NOT propagated — `Drop` cannot return `Result`.
       c. Sets `self.armed = false` (idempotent).

    3. `pub(super) fn disarm(&mut self)` for the case where the caller explicitly transferred cleanup responsibility elsewhere (rare; mostly for tests).

    Add `#[cfg(all(test, target_os = "linux"))]` integration test gated on `/sys/fs/cgroup/cgroup.controllers` existence + write access:
    - `cgroup_session_lifecycle`: creates a `CgroupSession`, asserts `<path>` exists, drops the session, asserts `<path>` no longer exists. SKIPPED with `eprintln!("skipping: no cgroup v2 delegation")` if `detect()` fails.

    `///` doc comments on each method documenting the read-modify-write semantics on `cgroup.subtree_control`, the EEXIST fail-fast, and the Drop guarantee.
  </action>
  <verify>
    <automated>
      cargo fmt --all -- --check
      cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used
      cargo test -p nono-cli --bin nono -- supervisor_linux::cgroup
    </automated>
    On a Linux dev host with cgroup v2: a manual run of `cargo test cgroup_session_lifecycle` followed by `ls /sys/fs/cgroup/<delegated>/nono-*/` shows no leftover directories.
  </verify>
  <done>
    - `CgroupSession::new` creates the cgroup, enables controllers in the parent, and stores the path.
    - `Drop` removes the cgroup unconditionally (panic-safe).
    - Integration test gated on `target_os = "linux"` + cgroup-v2 availability passes on hosts that have the prerequisites and is skipped (without test failure) on hosts that don't.
    - `cargo clippy ... -D warnings -D clippy::unwrap_used` clean.
  </done>
</task>

<task type="auto">
  <name>Task 3: Linux limit application — memory.max / cpu.max / pids.max</name>
  <files>
    crates/nono-cli/src/exec_strategy/supervisor_linux.rs
  </files>
  <read_first>
    - REQ-RESL-NIX-01 acceptance criteria 1, 2, 3 (the three limits + their kernel files)
  </read_first>
  <action>
    Add a `pub(super) fn apply_limits(&self) -> Result<()>` method on `CgroupSession` that:

    1. If `self.limits.memory_bytes.is_some()`: writes the bytes value (decimal, no suffix) to `<self.path>/memory.max`. Example: `Some(256 * 1024 * 1024)` → write the string `"268435456\n"`.

    2. If `self.limits.cpu_percent.is_some()`: writes `<quota> <period>` to `<self.path>/cpu.max`, where `period = 100000` (100ms) and `quota = percent * period / 100` (e.g., `--cpu-percent 50` → quota=50000, period=100000, so write `"50000 100000\n"`). Use `checked_mul` to avoid overflow even though clap caps percent at 100.

    3. If `self.limits.max_processes.is_some()`: writes the count (decimal) to `<self.path>/pids.max`. Example: `Some(10)` → write the string `"10\n"`.

    4. The `--timeout` field is NOT applied at limit-write time — it is consumed by Task 5's watchdog after spawn.

    All file writes use `std::fs::write` (truncates + writes atomically per write call). On any I/O error: return `NonoError::SandboxInit(format!("Failed to write {} to <self.path>/<file>: {}", value, err))` naming WHICH limit failed and the kernel error.

    Add `#[cfg(all(test, target_os = "linux"))]` integration test:
    - `cgroup_session_apply_limits`: builds `CgroupSession` with all three limits set, calls `apply_limits`, then reads back the three pseudo-files and asserts the contents match. Skipped on non-cgroup-v2 hosts.

    `///` doc comments documenting:
    - The cgroup v2 file format (decimal bytes for memory.max, `<quota> <period>` for cpu.max, decimal count for pids.max).
    - The trailing newline convention.
    - Which limits map to which kernel pseudo-files.
    - That timeout is INTENTIONALLY not handled here (Task 5).
  </action>
  <verify>
    <automated>
      cargo fmt --all -- --check
      cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used
      cargo test -p nono-cli --bin nono -- supervisor_linux::cgroup
    </automated>
  </verify>
  <done>
    - `apply_limits` writes the three pseudo-files with the documented formats.
    - Integration test (gated) asserts readback matches the written values.
    - On any write failure, the spawn aborts with a `NonoError::SandboxInit` message naming the failing limit + kernel errno.
  </done>
</task>

<task type="auto">
  <name>Task 4: Linux child PID placement via pre_exec — closes the race window</name>
  <files>
    crates/nono-cli/src/exec_strategy/supervisor_linux.rs
  </files>
  <read_first>
    - `<risks>` item 3 (child PID placement race) — the entire reason for this task
    - `std::os::unix::process::CommandExt::pre_exec` async-signal-safety contract (must use only async-signal-safe libc functions; no allocation, no locks)
  </read_first>
  <action>
    Add a `pub(super) fn install_pre_exec(&self, cmd: &mut std::process::Command)` method on `CgroupSession` that:

    1. Captures `self.path` as a `PathBuf` (clone) — owned move into the closure.
    2. Builds the `cgroup.procs` path string: `<self.path>/cgroup.procs`.
    3. Calls `cmd.pre_exec(move || -> std::io::Result<()> { ... })` with a closure that:
       a. Calls `libc::getpid()` to get the child's own PID (post-fork pre-exec, `getpid()` is async-signal-safe).
       b. Opens `<cgroup.procs>` for writing via `libc::open` with `O_WRONLY | O_CLOEXEC` (NOT `std::fs::OpenOptions` — `pre_exec` MUST use raw libc to stay async-signal-safe; `OpenOptions::open` allocates).
       c. Formats the PID using `itoa` if the dep is available, or a hand-written loop into a stack buffer (`[u8; 20]`) — `format!()` allocates and is NOT async-signal-safe in `pre_exec`.
       d. Writes the PID + `\n` via `libc::write`.
       e. Closes the fd via `libc::close`.
       f. Returns `Ok(())` on success; `Err(io::Error::last_os_error())` on any libc failure.

    SAFETY block on the `cmd.pre_exec(...)` call documenting:
    - Why the closure is async-signal-safe (libc::open / libc::write / libc::close + stack buffer formatting only — no Rust allocator, no Mutex).
    - The race window: between `fork()` returning in the parent and this pre_exec running in the child, the child is a Rust runtime stub with NO user code yet — kernel scheduler latency only.
    - That the parent has ALREADY called `Self::apply_limits` BEFORE fork (Task 3) so the cgroup is fully configured when the child enters it.

    Add `#[cfg(all(test, target_os = "linux"))]` integration test:
    - `cgroup_session_pre_exec_places_pid`: builds `CgroupSession`, applies limits, builds a `Command` with `install_pre_exec`, spawns `bash -c 'sleep 5'`, asserts the child PID appears in `<cgroup.procs>` within 100ms. Skipped on non-cgroup-v2.
  </action>
  <verify>
    <automated>
      cargo fmt --all -- --check
      cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used
      cargo test -p nono-cli --bin nono -- supervisor_linux::cgroup
    </automated>
  </verify>
  <done>
    - `install_pre_exec` is wired up; the spawned child writes its own PID to cgroup.procs in pre_exec.
    - SAFETY comments document the async-signal-safety constraints.
    - Integration test asserts the PID lands in cgroup.procs.
    - No `format!` / `Vec` / Rust-allocator usage inside the pre_exec closure.
  </done>
</task>

<task type="auto">
  <name>Task 5: Linux timeout watchdog via cgroup.kill</name>
  <files>
    crates/nono-cli/src/exec_strategy/supervisor_linux.rs
    crates/nono-cli/src/exec_strategy.rs
  </files>
  <read_first>
    - REQ-RESL-NIX-02 acceptance criterion 2 (atomic kill of all 100 grandchildren — this is what cgroup.kill provides)
    - `<dispatch_layout>` block above (the `spawn_timeout_watchdog` shape)
  </read_first>
  <action>
    1. Add `pub(super) fn kill_all(&self) -> Result<()>` on `CgroupSession` that writes `"1\n"` to `<self.path>/cgroup.kill`. The kernel atomically delivers SIGKILL to every process in the cgroup tree (descendants included). On I/O error: return `NonoError::SandboxInit` (or a more specific variant if one exists for runtime supervisor errors — let the executor pick the closest match in `nono::error`).

    2. In `exec_strategy.rs`, add a `spawn_linux_timeout_watchdog(deadline: Instant, cgroup_path: PathBuf) -> std::thread::JoinHandle<()>` helper that:
       a. Spawns a thread that sleeps until `deadline` (using `Instant::now()` + `checked_duration_since`).
       b. After the sleep, writes `"1\n"` to `<cgroup_path>/cgroup.kill` (best-effort: log warnings on failure but do not panic — the watchdog might fire AFTER the child has already exited, in which case the cgroup might already be removed by the Drop guard).
       c. Returns the `JoinHandle`.
       d. Sets a "timeout fired" flag in shared state (an `Arc<AtomicBool>`) so the supervisor's wait loop can record `timeout_kill: true` in inspect data when it reaps the child.

    3. Wire the watchdog into the Supervised execution path: after the child is spawned, if `flags.resource_limits.timeout.is_some()`, compute `deadline = Instant::now() + timeout` and spawn the watchdog. On normal child exit (before deadline), join-or-detach the watchdog (the watchdog will fire harmlessly into a closed cgroup).

    Add an integration test (gated on `target_os = "linux"` + cgroup-v2):
    - `cgroup_kill_terminates_grandchildren`: spawns `bash -c "for i in 1 2 3; do sleep 60 & done; wait"`, places it in a cgroup, calls `kill_all`, asserts all 4 PIDs exit within 1s.
  </action>
  <verify>
    <automated>
      cargo fmt --all -- --check
      cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used
      cargo test -p nono-cli --bin nono -- cgroup_kill
    </automated>
    Manual smoke: `nono run --timeout 5s -- bash -c "for i in {1..10}; do sleep 60 & done; wait"` exits at ~5s with all 10 children killed (verified via `ps` showing no surviving sleeps).
  </verify>
  <done>
    - `CgroupSession::kill_all` writes to cgroup.kill atomically.
    - `spawn_linux_timeout_watchdog` is wired into the Supervised path.
    - Integration test confirms grandchild atomic kill.
    - `timeout_kill: true` is reported via inspect (or noted in SUMMARY if the inspect plumbing requires a follow-up task — see <objective> out-of-scope note).
  </done>
</task>

<task type="auto">
  <name>Task 6: macOS setrlimit + clap-time --cpu-percent rejection</name>
  <files>
    crates/nono-cli/src/exec_strategy/supervisor_macos.rs
    crates/nono-cli/src/cli.rs
  </files>
  <read_first>
    - REQ-RESL-NIX-03 acceptance criteria 1, 2, 3 (RLIMIT_AS for memory; RLIMIT_NPROC for max_processes; clap-time rejection for cpu_percent)
    - `nix::sys::resource::{setrlimit, Resource}` API shape from the `<interfaces>` block
    - `crates/nono-cli/src/cli.rs` `cpu_percent` flag definition (Phase 16 added it; this task wraps its `value_parser` with a target-gated rejection)
  </read_first>
  <action>
    **Step 1 — Create `crates/nono-cli/src/exec_strategy/supervisor_macos.rs`:**

    ```rust
    //! macOS resource-limit application via setrlimit + supervisor watchdog.
    //!
    //! Maps:
    //! - --memory <bytes>      -> RLIMIT_AS  (address space; NOT RSS — see doc note)
    //! - --max-processes <N>   -> RLIMIT_NPROC
    //! - --cpu-percent         -> rejected at clap parse time (no per-process
    //!                            CPU-quota equivalent on macOS; see REQ-RESL-NIX-03
    //!                            acceptance criterion 3).
    //! - --timeout <duration>  -> supervisor-side Instant deadline + kill(pgrp, SIGKILL).
    //!
    //! ## RLIMIT_AS vs. RSS
    //!
    //! RLIMIT_AS bounds the process's *virtual address space*, not its resident
    //! set size (RSS). A process can pass --memory 256m and still consume more
    //! than 256MB of physical memory if its mappings are sparse or shared. This
    //! is the documented gap per REQ-RESL-NIX-03; the alternative (RSS-based
    //! enforcement via /proc-equivalent) is not portable on macOS without
    //! polling, and polling has racy bypass windows.

    use crate::launch_runtime::ResourceLimits;
    use nix::sys::resource::{setrlimit, Resource};
    use nono::{NonoError, Result};
    use std::os::unix::process::CommandExt;

    pub(crate) struct MacosResourceLimits {
        memory_bytes: Option<u64>,
        max_processes: Option<u32>,
        // timeout is consumed by the supervisor watchdog, not pre_exec.
    }

    impl MacosResourceLimits {
        pub(crate) fn new(limits: &ResourceLimits) -> Result<Self> {
            // cpu_percent rejection is at clap-parse time; if it slips through here,
            // that's a defense-in-depth fail-fast.
            if limits.cpu_percent.is_some() {
                return Err(NonoError::NotSupportedOnPlatform {
                    feature: "cpu_percent_macos".into(),
                });
            }
            Ok(Self {
                memory_bytes: limits.memory_bytes,
                max_processes: limits.max_processes,
            })
        }

        pub(crate) fn install_pre_exec(&self, cmd: &mut std::process::Command) {
            let memory_bytes = self.memory_bytes;
            let max_processes = self.max_processes;
            unsafe {
                // SAFETY: pre_exec runs in the forked child, post-fork pre-exec.
                // setrlimit is async-signal-safe (POSIX). No allocation or
                // locks taken in the closure body.
                cmd.pre_exec(move || {
                    if let Some(bytes) = memory_bytes {
                        setrlimit(Resource::RLIMIT_AS, bytes, bytes)
                            .map_err(|e| std::io::Error::from_raw_os_error(e as i32))?;
                    }
                    if let Some(n) = max_processes {
                        setrlimit(Resource::RLIMIT_NPROC, n as u64, n as u64)
                            .map_err(|e| std::io::Error::from_raw_os_error(e as i32))?;
                    }
                    Ok(())
                });
            }
        }
    }
    ```

    **Step 2 — Add the timeout watchdog (in the same file or in `exec_strategy.rs`):**

    ```rust
    pub(crate) fn spawn_macos_timeout_watchdog(
        deadline: std::time::Instant,
        child_pgrp: nix::unistd::Pid,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let now = std::time::Instant::now();
            if let Some(remaining) = deadline.checked_duration_since(now) {
                std::thread::sleep(remaining);
            }
            // Negative PID = process group. SIGKILL = atomic, ungraceful.
            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(-child_pgrp.as_raw()),
                nix::sys::signal::Signal::SIGKILL,
            );
        })
    }
    ```

    **Step 3 — Reject `--cpu-percent` at clap parse time on macOS:**

    In `crates/nono-cli/src/cli.rs`, replace the existing `value_parser = clap::value_parser!(u16).range(1..=100)` on `cpu_percent` with a target-gated wrapper:

    ```rust
    /// On macOS, reject --cpu-percent at clap parse time per REQ-RESL-NIX-03 criterion 3.
    /// On Linux/Windows, behaves as today (u16 in 1..=100).
    fn parse_cpu_percent(s: &str) -> std::result::Result<u16, String> {
        #[cfg(target_os = "macos")]
        {
            let _ = s;
            return Err(
                "--cpu-percent is not supported on macOS (no per-process CPU-quota equivalent); \
                 see REQUIREMENTS.md § REQ-RESL-NIX-03 acceptance criterion 3".into()
            );
        }
        #[cfg(not(target_os = "macos"))]
        {
            let n: u16 = s.parse().map_err(|_| format!("invalid cpu_percent '{s}'"))?;
            if !(1..=100).contains(&n) {
                return Err(format!("--cpu-percent must be in 1..=100; got {n}"));
            }
            Ok(n)
        }
    }
    ```

    Wire it into the `RunArgs.cpu_percent` field via `value_parser = parse_cpu_percent`. This produces the clap-time error message; the `NonoError::NotSupportedOnPlatform { feature: "cpu_percent_macos" }` is NOT directly emitted by clap (clap returns `String` errors), but the error string verbatim references REQ-RESL-NIX-03 so the user can grep for it. The `MacosResourceLimits::new` defense-in-depth check above ensures even programmatic callers (tests, FFI) get the typed error variant.

    **Step 4 — Tests:**

    `#[cfg(all(test, target_os = "macos"))]` mod tests covering:
    - `MacosResourceLimits::new` with `cpu_percent = Some(50)` returns `Err(NotSupportedOnPlatform { feature: "cpu_percent_macos" })`.
    - `MacosResourceLimits::new` with all-None returns `Ok` with both fields None.
    - Spawn `bash -c "ulimit -v"` with `memory_bytes = Some(256*1024*1024)` and assert the printed value is the expected RLIMIT_AS in KB.

    Cross-platform test (`#[cfg(test)]`):
    - `parse_cpu_percent("50")` on Linux/Windows returns `Ok(50)`; on macOS returns `Err(...)` containing the substring `"REQ-RESL-NIX-03"`.
  </action>
  <verify>
    <automated>
      cargo fmt --all -- --check
      cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used
      cargo test -p nono-cli --bin nono -- supervisor_macos
      cargo test -p nono-cli --bin nono -- parse_cpu_percent
    </automated>
    Manual smoke (macOS): `cargo run --release -p nono-cli -- run --cpu-percent 50 -- ls` exits non-zero with the REQ-RESL-NIX-03 error string in stderr; no child is spawned.
  </verify>
  <done>
    - `supervisor_macos.rs` exists with `MacosResourceLimits` + watchdog.
    - `parse_cpu_percent` rejects on macOS at clap-parse time.
    - macOS-gated tests pass.
    - SAFETY comments on every unsafe block (pre_exec).
    - The RLIMIT_AS-vs-RSS gap is documented in module doc comment.
  </done>
</task>

<task type="auto">
  <name>Task 7: Wire dispatch + warning removal + module declaration</name>
  <files>
    crates/nono-cli/src/exec_strategy.rs
    crates/nono-cli/src/launch_runtime.rs
  </files>
  <read_first>
    - `<dispatch_layout>` (the `apply_resource_limits_unix` shape)
    - `crates/nono-cli/src/exec_strategy.rs` lines 13–16 (current `mod` declarations — add `supervisor_macos` here gated on `target_os = "macos"`)
    - `collect_unix_resource_limit_warnings` body (the four `out.push` branches that get deleted)
  </read_first>
  <action>
    **Step 1 — Module declarations in `exec_strategy.rs`:**

    Add at the top of `exec_strategy.rs` next to the existing `#[cfg(target_os = "linux")] mod supervisor_linux;`:

    ```rust
    #[cfg(target_os = "macos")]
    mod supervisor_macos;
    ```

    **Step 2 — Add `apply_resource_limits_unix` dispatch helper:**

    Add the dispatch helper as shown in `<dispatch_layout>` above. The `UnixResourceLimitGuard` enum lives in `exec_strategy.rs` (top-level, `pub(crate)`). Variants are gated by target_os.

    **Step 3 — Wire dispatch into Direct + Supervised entry points:**

    In `execute_direct` (search for `pub fn execute_direct` in `exec_strategy.rs`) and the Supervised path (search for `execute_supervised` or the equivalent Linux/macOS spawn site), call `apply_resource_limits_unix(flags, &mut cmd, &session_id)?` BEFORE `cmd.spawn()`. Bind the returned `UnixResourceLimitGuard` to a local variable that lives until child exit (so Drop fires AFTER the child is reaped).

    For `--timeout` enforcement: after `spawn()`, check `flags.resource_limits.timeout`. If `Some(d)`, compute `deadline = Instant::now() + d` and call:
    - `spawn_linux_timeout_watchdog(deadline, cgroup_path)` on Linux (extract cgroup_path from the guard).
    - `spawn_macos_timeout_watchdog(deadline, child_pgrp)` on macOS (extract child_pgrp via `child.id()` + `getpgid`).

    **Step 4 — Delete the four "not enforced" warnings:**

    In `collect_unix_resource_limit_warnings` (lines 44–102 of current `exec_strategy.rs`), delete the four `out.push(format!("warning: --<flag> is not enforced on {os_name}; ..."))` blocks. The function can either:

    (a) **Be removed entirely** along with `warn_unix_resource_limits` and all its call sites — cleanest. Search for `warn_unix_resource_limits` (used in `execute_direct` and `execute_supervised_runtime` per Phase 16-01-PLAN.md). Delete the calls. Delete the helper. Delete the unit tests in `unix_warning_tests`.

    (b) **Keep the helper as an empty-Vec-returning stub** for future use. Adds dead code, fails clippy.

    **Choose (a)** — remove. Keeping (b) would leave dead code that fails clippy's `dead_code` lint and contradicts CLAUDE.md's "lazy use of dead_code" guidance. The replacement is real enforcement, not a different warning.

    **Step 5 — Update `launch_runtime.rs` doc comments:**

    Update the `///` doc comments on `ResourceLimits` to reflect that Linux + macOS now enforce (the Phase 16 doc comments said "On Linux/macOS, each `Some(_)` field emits a warning..."). New doc:

    ```rust
    /// Optional resource caps applied to the sandboxed agent tree.
    ///
    /// - **Windows:** kernel-enforced via Job Object (see Phase 16
    ///   `apply_resource_limits` in `exec_strategy_windows::launch`).
    /// - **Linux:** kernel-enforced via cgroup v2 delegated hierarchy (see
    ///   `supervisor_linux::CgroupSession`). Requires cgroup v2 + systemd
    ///   delegation; fails fast on cgroup v1 hosts.
    /// - **macOS:** kernel-enforced for memory + max_processes via
    ///   `setrlimit(RLIMIT_AS, RLIMIT_NPROC)`. `--cpu-percent` is rejected
    ///   at clap parse time (no per-process CPU-quota equivalent on macOS).
    ///   `--timeout` enforced via supervisor-side `Instant` deadline +
    ///   `kill(pgrp, SIGKILL)` watchdog.
    ```

    **Step 6 — Verify warning removal:**

    Run `grep -nE 'is not enforced on (linux|macos)' crates/nono-cli/src/` and confirm zero matches.
  </action>
  <verify>
    <automated>
      cargo fmt --all -- --check
      cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used
      cargo test --workspace --all-features
    </automated>
    grep -nE 'is not enforced on (linux|macos)' crates/nono-cli/src/ returns ZERO matches.
    grep -n 'warn_unix_resource_limits\|collect_unix_resource_limit_warnings' crates/nono-cli/src/ returns ZERO matches (helpers fully removed per option (a)).
  </verify>
  <done>
    - Dispatch helper `apply_resource_limits_unix` is wired into Direct + Supervised paths.
    - Linux + macOS watchdogs are spawned when `--timeout` is set.
    - The four "not enforced on linux/macos" warning lines are gone.
    - `warn_unix_resource_limits` and `collect_unix_resource_limit_warnings` are removed (option (a)).
    - `ResourceLimits` doc comment reflects the new reality (cross-platform enforcement).
    - `cargo clippy ... -D warnings` clean.
  </done>
</task>

<task type="auto">
  <name>Task 8: End-to-end integration tests + final verification</name>
  <files>
    crates/nono-cli/tests/resl_nix_linux.rs
    crates/nono-cli/tests/resl_nix_macos.rs
  </files>
  <read_first>
    - REQ-RESL-NIX-01 acceptance criteria 1–5 (the must-prove behaviors)
    - REQ-RESL-NIX-02 acceptance criteria 1–2
    - REQ-RESL-NIX-03 acceptance criteria 1–4
  </read_first>
  <action>
    **Linux integration test file (`crates/nono-cli/tests/resl_nix_linux.rs`):**

    `#![cfg(target_os = "linux")]` at the top. Each test gated on `cgroup_v2_available()` helper that reads `/proc/self/cgroup` and skips with `eprintln!("SKIP: ..."); return;` if unavailable.

    1. `linux_memory_limit_oom_kills_child`: `nono run --memory 256m -- bash -c "tail -c 1G </dev/urandom"`; assert exit code reflects SIGKILL (137 typically) and dmesg/journalctl shows cgroup OOM (best-effort grep; OPTIONAL gate). Maps to REQ-RESL-NIX-01 criterion 1.

    2. `linux_cpu_percent_caps_at_50`: `nono run --cpu-percent 50 -- bash -c "yes >/dev/null"`; sleep 5s; read `/proc/<child>/stat` user+system time delta; assert it's between 2.0 and 3.0 seconds (= 40-60% of one core, with margin). Maps to REQ-RESL-NIX-01 criterion 2.

    3. `linux_max_processes_blocks_eleventh_fork`: `nono run --max-processes 10 -- bash -c "for i in {1..20}; do sleep 60 & done; wait"`; assert stderr contains `pids.max` (or fork failure). Maps to REQ-RESL-NIX-01 criterion 3.

    4. `linux_timeout_kills_at_deadline`: `nono run --timeout 5s -- sleep 60`; assert wall time is between 4.5s and 6s. Maps to REQ-RESL-NIX-02 criterion 1.

    5. `linux_timeout_atomic_kill_grandchildren`: `nono run --timeout 5s -- bash -c "for i in {1..100}; do sleep 60 & done; wait"`; capture child PIDs via a sentinel file write; after deadline, assert all 100 PIDs are gone (poll `/proc/<pid>/status` for absence). Maps to REQ-RESL-NIX-02 criterion 2.

    6. `linux_no_warnings_on_resource_flags`: `nono run --memory 256m --cpu-percent 50 --max-processes 10 --timeout 60s -- echo hi`; capture stderr; assert it does NOT contain `is not enforced on linux`. Maps to REQ-RESL-NIX-01 criterion 4.

    **macOS integration test file (`crates/nono-cli/tests/resl_nix_macos.rs`):**

    `#![cfg(target_os = "macos")]` at the top.

    1. `macos_memory_limit_aborts_via_rlimit_as`: `nono run --memory 256m -- bash -c '<small alloc loop that mmaps 1GB>'`; assert exit code is non-zero and stderr contains an mmap/allocation failure indicator. Maps to REQ-RESL-NIX-03 criterion 1.

    2. `macos_max_processes_eagain_on_eleventh_fork`: `nono run --max-processes 10 -- bash -c "for i in {1..20}; do sleep 60 & done; wait"`; assert stderr contains `Resource temporarily unavailable` (EAGAIN) or fork failure. Maps to REQ-RESL-NIX-03 criterion 2.

    3. `macos_cpu_percent_rejected_at_clap_parse`: `nono run --cpu-percent 50 -- ls`; assert exit code is the clap-error code (typically 2) AND stderr contains `REQ-RESL-NIX-03` (or `cpu_percent_macos`). Crucially: assert NO child was spawned (no `ls` output). Maps to REQ-RESL-NIX-03 criterion 3.

    4. `macos_timeout_kills_at_deadline`: `nono run --timeout 5s -- sleep 60`; assert wall time is between 4.5s and 6s. Maps to REQ-RESL-NIX-03 criterion 4.

    Use `assert_cmd` if it's already in dev-dependencies (Phase 16 likely added it); if not, use `std::process::Command::new(env!("CARGO_BIN_EXE_nono"))` directly.

    **Final verification:**

    Run the full CI suite:
    - `cargo fmt --all -- --check`
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used`
    - `cargo test --workspace --all-features`

    All gates green on Linux AND macOS lanes.
  </action>
  <verify>
    <automated>
      cargo fmt --all -- --check
      cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used
      cargo test --target x86_64-unknown-linux-gnu --workspace --all-features -- resl_nix_linux
      cargo test --target x86_64-apple-darwin --workspace --all-features -- resl_nix_macos
      cargo test --workspace --all-features
    </automated>
    grep -nE 'is not enforced on (linux|macos)' crates/nono-cli/src/ returns ZERO matches.
  </verify>
  <done>
    - `resl_nix_linux.rs` covers REQ-RESL-NIX-01 criteria 1–4 + REQ-RESL-NIX-02 criteria 1–2 (6 tests).
    - `resl_nix_macos.rs` covers REQ-RESL-NIX-03 criteria 1–4 (4 tests).
    - All tests pass on their respective platforms (skipped cleanly on unsupported hosts).
    - Final `cargo test --workspace --all-features` green on Linux + macOS.
    - Commit on the appropriate v2.3 branch with DCO sign-off, ONLY the files in `files_modified` + the two new test files staged.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| user → nono CLI argument parser | Untrusted numeric / duration strings reach the parsers (already addressed in Phase 16; this plan adds macOS-specific clap-time rejection for `--cpu-percent`). |
| nono supervisor (parent) → cgroup v2 pseudo-filesystem | Supervisor writes to `<delegated>/cgroup.subtree_control`, `<delegated>/nono-<id>/memory.max`, etc. The kernel trusts the supervisor (running as the user) to write valid format strings. Malformed writes are rejected by the kernel with `EINVAL`, which the supervisor surfaces. |
| supervisor → child via `pre_exec` hook | The pre_exec closure runs in the forked child BEFORE execve. Async-signal-safety contract: only libc syscalls, no Rust allocator. Violation = undefined behavior in the child. |
| child process → its own cgroup | The child writes its own PID to `<cgroup>/cgroup.procs` in pre_exec. The cgroup is owned by the supervisor's UID. The child cannot escape (cgroup v2 propagates to descendants and grandchildren). |
| supervisor watchdog thread → child cgroup tree | The watchdog writes `1\n` to `<cgroup>/cgroup.kill` at deadline. The kernel atomically delivers SIGKILL to all descendants. No race window between deadline detection and kill. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-25-01-01 | Tampering | Malicious child writes to `<cgroup>/cgroup.procs` to move ITSELF out of the cgroup. | mitigate | cgroup v2 by-design forbids non-privileged processes from leaving their cgroup. The child has the cgroup directory's UID-owned permissions but cannot move out of the hierarchy without CAP_SYS_ADMIN. Verified by kernel docs (Documentation/admin-guide/cgroup-v2.rst § Migration Permissions). |
| T-25-01-02 | Denial of Service | Malicious child fork-storms before `cgroup.kill` fires at the timeout deadline, exhausting host PIDs. | mitigate | `pids.max` (REQ-RESL-NIX-01 criterion 3) caps the cgroup's PID count regardless of timeout. Even without `--max-processes`, the supervisor's parent cgroup is itself capped by systemd's user-slice defaults. `cgroup.kill` is atomic — there is no fork window between the kill write and SIGKILL delivery. |
| T-25-01-03 | Elevation of Privilege | Supervisor's cgroup directory (e.g., `<delegated>/nono-<id>/`) inherits the user's UID. A coexisting process at the same UID (NOT in the cgroup) could write to `<cgroup>/memory.max` and lift the limit on the running session. | accept | The threat actor is the user themselves. nono does not protect the user's UID from itself; it protects the user from untrusted code running INSIDE the sandbox. The sandboxed child cannot escape its own cgroup (T-25-01-01). |
| T-25-01-04 | Information Disclosure | The `/proc/self/cgroup` contents leak the user's systemd slice path into NonoError diagnostic messages. | accept | The slice path is already visible to any process at the user's UID via `/proc`. nono error messages already embed similar paths in existing fail-fast surfaces (Phase 16 `GetLastError` precedent). Consistency wins over the low-value information leak. |
| T-25-01-05 | Tampering | The pre_exec closure on macOS uses `setrlimit` directly. A malformed `bytes` value (e.g., u64::MAX) could wrap inside the kernel's `rlim_t` (long) on 32-bit hosts. | mitigate | clap caps `--memory` via `parse_byte_size` (Phase 16); `bytes as rlim_t` cast is checked with `try_into` in `MacosResourceLimits::install_pre_exec`. nono's MSRV (1.77) and `nix` 0.29 both target 64-bit primary; 32-bit Unix is not a supported target. |
| T-25-01-06 | Denial of Service | The supervisor's timeout watchdog thread sleeps until deadline. If the supervisor itself is paused (SIGSTOP via debugger / terminal control), the watchdog stops with it, allowing the child to exceed its timeout. | accept | If the supervisor is debugger-paused, the user is in active interactive control and the timeout SLA does not apply. Production observability (audit ledger emission) is unaffected: the timeout failure mode is recorded once the supervisor resumes. |
| T-25-01-07 | Tampering | A malicious child writes to `<delegated>/nono-<id>/cgroup.procs` to ADD additional PIDs to its own cgroup (bringing in unrelated processes that share its UID). | mitigate | Only PIDs the child has permission to send signals to (same UID + same session) can be added. This grants the child the ability to recruit other user processes into its cgroup but cannot escape — it can only DRAG others into the limit, not escape it. The audit ledger records the cgroup membership at exit time so this becomes detectable post-hoc. The threat is bounded by what the child could already do at its UID anyway. |
| T-25-01-08 | Spoofing | The `parse_cpu_percent` macOS rejection error message references `REQ-RESL-NIX-03` — if the requirement document is renamed, the error string becomes a stale reference. | accept | Acceptance criterion 3 of REQ-RESL-NIX-03 mandates the error mentions `cpu_percent_macos`. The REQUIREMENTS.md cross-reference is supplementary. Renaming requirements is a documented breaking change; tests would fail-fast at the cross-reference. |

## Mitigations Summary

Every mitigation references a specific implementation site:
- T-25-01-01: kernel-enforced (cgroup v2 docs); no nono-side code needed.
- T-25-01-02: `apply_limits` in Task 3 writes `pids.max`; `kill_all` in Task 5 is atomic per kernel.
- T-25-01-05: `try_into` cast in `MacosResourceLimits::install_pre_exec` (Task 6).
- T-25-01-07: audit-ledger emission in Task 5's watchdog flag plumbing (or follow-up if inspect plumbing is deferred).
</threat_model>

<verification>
```bash
# Build (Linux):
cargo build --release -p nono-cli --bin nono --target x86_64-unknown-linux-gnu

# Build (macOS):
cargo build --release -p nono-cli --bin nono --target x86_64-apple-darwin
# (or aarch64-apple-darwin on Apple Silicon)

# Format and lint (must be clean on BOTH platforms):
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used

# Cross-platform unit tests (parsers, ResourceLimits plumbing — Phase 16 surface, regressions):
cargo test --workspace --all-features

# Linux-gated: cgroup v2 lifecycle + integration tests:
cargo test --target x86_64-unknown-linux-gnu -p nono-cli --bin nono -- supervisor_linux::cgroup
cargo test --target x86_64-unknown-linux-gnu -p nono-cli --test resl_nix_linux

# macOS-gated: setrlimit + watchdog + clap-time rejection:
cargo test --target x86_64-apple-darwin -p nono-cli --bin nono -- supervisor_macos
cargo test --target x86_64-apple-darwin -p nono-cli --test resl_nix_macos

# Warning-removal verification (must return ZERO matches):
grep -nE 'is not enforced on (linux|macos)' crates/nono-cli/src/

# Helper-removal verification (must return ZERO matches if option (a) was chosen):
grep -nE 'warn_unix_resource_limits|collect_unix_resource_limit_warnings' crates/nono-cli/src/

# Manual smoke gates:
# Linux:
target/release/nono run --memory 256m -- bash -c "tail -c 1G </dev/urandom"     # OOM kill
target/release/nono run --cpu-percent 50 -- bash -c "yes >/dev/null & sleep 5"   # ~50% peg
target/release/nono run --max-processes 10 -- bash -c "for i in {1..20}; do sleep 60 & done; wait"
target/release/nono run --timeout 5s -- sleep 60                                  # ~5s deadline

# macOS:
target/release/nono run --cpu-percent 50 -- ls                                    # CLAP REJECT
target/release/nono run --memory 256m -- bash -c "<large alloc>"                  # RLIMIT_AS abort
target/release/nono run --max-processes 10 -- bash -c "for i in {1..20}; do sleep 60 & done; wait"
target/release/nono run --timeout 5s -- sleep 60                                  # ~5s deadline
```
</verification>

<success_criteria>
Maps to REQUIREMENTS.md REQ-RESL-NIX-01, REQ-RESL-NIX-02, REQ-RESL-NIX-03 acceptance clauses.

1. **(SC-1) Linux cgroup v2 detection + fail-fast:** On a cgroup-v1 host or no-systemd-delegation host, `nono run --memory 256m -- ...` exits non-zero with `NonoError::UnsupportedPlatform { feature: "cgroup_v2" }` BEFORE any child is spawned. (REQ-RESL-NIX-01 criterion 5.)
2. **(SC-2) Linux memory enforcement:** `nono run --memory 256m -- bash -c "tail -c 1G </dev/urandom"` is OOM-killed by cgroup v2. (REQ-RESL-NIX-01 criterion 1.)
3. **(SC-3) Linux CPU enforcement:** `nono run --cpu-percent 50 -- bash -c "yes >/dev/null"` pegs at ~50% of one logical core. (REQ-RESL-NIX-01 criterion 2.)
4. **(SC-4) Linux PID enforcement:** `nono run --max-processes 10 -- bash -c "for i in {1..20}; do sleep 60 & done; wait"` fails after the 10th fork with `pids.max`. (REQ-RESL-NIX-01 criterion 3.)
5. **(SC-5) Warnings removed:** `grep -nE 'is not enforced on (linux|macos)' crates/nono-cli/src/` returns ZERO matches. (REQ-RESL-NIX-01 criterion 4.)
6. **(SC-6) Linux timeout atomic kill:** `nono run --timeout 5s -- bash -c "for i in {1..100}; do sleep 60 & done; wait"` exits at ~5s with all 100 grandchildren killed. (REQ-RESL-NIX-02 criteria 1+2.)
7. **(SC-7) macOS memory enforcement:** `nono run --memory 256m -- <large alloc>` aborts via RLIMIT_AS. (REQ-RESL-NIX-03 criterion 1.)
8. **(SC-8) macOS PID enforcement:** `nono run --max-processes 10 -- ...` fails after the 10th fork with EAGAIN. (REQ-RESL-NIX-03 criterion 2.)
9. **(SC-9) macOS clap-time rejection:** `nono run --cpu-percent 50 -- ls` fails at clap parse with REQ-RESL-NIX-03 in the error string; no child spawned. (REQ-RESL-NIX-03 criterion 3.)
10. **(SC-10) macOS timeout:** `nono run --timeout 5s -- sleep 60` exits at ~5s. (REQ-RESL-NIX-03 criterion 4.)
11. **(SC-11) `make ci` green** on Linux + macOS lanes: `cargo fmt --check` + `cargo clippy -D warnings -D clippy::unwrap_used` + `cargo test --workspace --all-features` all pass.
12. **(SC-12) Cleanup hygiene:** Cgroup directories created during tests are removed via Drop; `ls /sys/fs/cgroup/<delegated>/nono-*/` is empty after both successful and panicking sessions. No leftover state on macOS (rlimits are per-process and die with the process).
</success_criteria>

<output>
After completion, create `.planning/phases/25-cross-platform-resl-aipc-unix-design/25-01-RESL-NIX-SUMMARY.md` containing:
- The `CgroupSession` struct shape, RAII guarantees, and the `pre_exec`-time PID placement strategy + race-window analysis.
- The `MacosResourceLimits` struct shape, the RLIMIT_AS-vs-RSS gap doc, and the `--cpu-percent` clap-time rejection mechanism.
- The dispatch surface change in `exec_strategy.rs` (new `apply_resource_limits_unix` + `UnixResourceLimitGuard` enum) and the four warning lines that were deleted.
- Watchdog implementation: Linux `cgroup.kill` (atomic) vs. macOS `kill(pgrp, SIGKILL)` (per-PID with negative-PID = process-group); behavioral parity for the user-observable timeout SLA.
- Failure-mode matrix: cgroup v1 host, no systemd delegation, `mkdir EEXIST`, controller-enablement EINVAL, RLIMIT_AS overflow, EAGAIN at fork, clap-time rejection. Each row maps the failure to its visible error message.
- Test coverage table: which acceptance criterion is covered by which test (unit vs. integration; gated by which target_os).
- Top-3 risks (carried from `<risks>`): non-systemd init fail-fast (intentional), cgroup-subtree_control variability across distros, child PID placement race window — with the chosen mitigations.
- Inspect-side plumbing status: `memory_kill: true` and `timeout_kill: true` either land in this plan (if the v2.1 Phase 16 inspect surface accepts them) or are explicitly noted as a follow-up gap with the field name + call site identified.
- Handoff to Plan 25-02 (AIPC Unix futures ADR): no code dependency; this plan is fully orthogonal.
</output>
