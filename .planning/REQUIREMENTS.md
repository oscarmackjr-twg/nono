---
milestone: v2.1
milestone_name: Resource Limits, Extended IPC, Attach-Streaming & Cleanup
status: active
created: 2026-04-18
updated: 2026-04-18
---

# Requirements — v2.1

Scope: Job Object resource limits, extended IPC handle brokering, attach-streaming on detached Windows sessions, and a cleanup workstream for pre-existing drift and WIP accumulated during v2.0.

All requirements are **Windows-first** — Linux and macOS behavior is specified per-requirement where it differs (usually: RESL has native equivalents via cgroups/rlimit/launchd; AIPC is Windows-specific because it extends Phase 11's Windows cap pipe; ATCH is Windows-specific; CLEAN is platform-agnostic except for test flakes which are Windows-only).

---

## RESL — Resource Limits

Context: v2.0 shipped Named Job Objects with basic lifecycle (kill-on-close + die-on-unhandled-exception). v2.1 adds resource-boundary limits so an agent tree cannot hog the host.

### RESL-01: CPU percentage cap

**What:** User can cap the CPU usage of a sandboxed agent tree to a percentage of one logical core (or a share of all cores), enforced by the kernel via `JOB_OBJECT_CPU_RATE_CONTROL_ENABLE`.

**CLI:**
- `nono run --cpu-percent 50 -- <cmd>` — cap child tree at 50% of one logical core.
- Accepted range: 1..=100 (percent of single core) or 100..=100 * NumberOfProcessors (all-core share).

**Enforcement:**
- Windows: `SetInformationJobObject(..., JobObjectCpuRateControlInformation, ...)` with `ControlFlags = JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP`, `CpuRate` field = percent * 100 (10000 = 100%).
- Linux: cgroup v2 `cpu.max` equivalent (out of scope for v2.1; record as a cross-platform follow-up).
- macOS: no native equivalent; CLI accepts the flag with a `not-supported-on-this-platform` warning.

**Security:**
- Fail-closed: if the Job Object can't accept the rate-control info, the run aborts (don't silently ignore).
- No escape hatch: rate control is applied BEFORE `ResumeThread`.

**Acceptance:**
1. `--cpu-percent 25` with a CPU-bound workload (e.g. `powershell -Command "while ($true) {}"`) measurably caps the process-tree's CPU to ~25% (via `Get-Process | Select-Object CPU`).
2. Invalid values (`--cpu-percent 0`, `--cpu-percent 101`) reject with a clear error before launching the child.
3. Unit tests assert `JobObjectCpuRateControlInformation` is set with the expected fields via `QueryInformationJobObject` in a self-test.
4. Integration test: CPU cap is enforced even when the child spawns additional processes (job-wide cap).

**Maps to:** Phase 16.

---

### RESL-02: Memory cap

**What:** User can cap the total memory (committed + working set) of a sandboxed agent tree. Exceeding the cap kills the offending process(es) and logs an `ERROR_NOT_ENOUGH_MEMORY` event.

**CLI:**
- `nono run --memory 512M -- <cmd>` — cap agent tree at 512 MiB total.
- Accepted formats: `512M`, `1G`, `256K`, plain bytes (`268435456`).

**Enforcement:**
- Windows: `JOBOBJECT_EXTENDED_LIMIT_INFORMATION.JobMemoryLimit` (job-wide cap) via `SetInformationJobObject`. `ProcessMemoryLimit` (per-process cap) is a follow-up; start with job-wide because that's what the threat model cares about.
- Set `LimitFlags |= JOB_OBJECT_LIMIT_JOB_MEMORY`.
- Linux: cgroup v2 `memory.max` equivalent (follow-up).
- macOS: no native equivalent; CLI accepts the flag with a platform-support warning.

**Security:**
- Fail-closed: if the Job Object cap can't be set, run aborts.
- Children that exceed the cap are terminated by the kernel — no user-mode escape.

**Acceptance:**
1. `--memory 128M` with a memory-hungry workload (e.g. `powershell -Command "[byte[]]::new(500MB)"`) results in child termination with a kernel-logged OOM-style event.
2. Invalid sizes (`--memory 0`, `--memory -1`, malformed strings) reject with a clear error before launching.
3. Unit tests assert `JobMemoryLimit` reads back the expected value.

**Maps to:** Phase 16.

---

### RESL-03: Wall-clock timeout

**What:** User can set a maximum run time for a sandboxed agent tree. When the timeout expires, the supervisor terminates the Job Object cleanly (equivalent to `nono terminate`).

**CLI:**
- `nono run --timeout 5m -- <cmd>` — kill after 5 minutes elapsed.
- Accepted formats: `30s`, `5m`, `1h`, plain seconds (`300`).

**Enforcement:**
- Windows: prefer `JOB_OBJECT_LIMIT_JOB_TIME` (kernel-enforced CPU time, not wall-clock) combined with a supervisor-side wall-clock timer that calls `TerminateJobObject` when the deadline hits. Wall-clock is what users actually want ("kill after 5 minutes real time"); document the kernel JOB_TIME limitation (counts CPU only).
- Linux: supervisor-side `setitimer` or tokio sleep + kill.
- macOS: supervisor-side timer + kill.

**Security:**
- Fail-open for timer-missed edge case is acceptable (supervisor crash scenario; Job Object will still kill the tree when the supervisor handle closes). Document this.
- No escape: child cannot extend its own deadline.

**Acceptance:**
1. `--timeout 5s` with `ping -t 127.0.0.1` (non-terminating workload): grandchild is killed after ~5 seconds; `nono inspect <id>` shows `exit_code` as a terminate signal (e.g. `-1` or explicit timeout marker).
2. Timeout expiration fires even if the child spawns additional processes (job-wide kill).
3. Invalid durations reject before launching.

**Maps to:** Phase 16.

---

### RESL-04: Process count cap

**What:** User can limit the total number of active processes in a sandboxed agent tree, preventing fork bombs and runaway spawn behavior.

**CLI:**
- `nono run --max-processes 10 -- <cmd>` — fail new `CreateProcess` calls after 10 active processes.
- Accepted range: 1..=65535.

**Enforcement:**
- Windows: `JOBOBJECT_EXTENDED_LIMIT_INFORMATION.BasicLimitInformation.ActiveProcessLimit` + `LimitFlags |= JOB_OBJECT_LIMIT_ACTIVE_PROCESS`.
- Linux: cgroup v2 `pids.max` equivalent (follow-up).
- macOS: no native equivalent; CLI accepts the flag with a warning.

**Security:**
- Fail-closed: if the limit can't be set, run aborts.
- When the cap is hit, new `CreateProcess` calls in the sandboxed tree fail with `ERROR_TOO_MANY_PROCESSES` — no escape.

**Acceptance:**
1. `--max-processes 3` followed by a fork-bomb-like command (nested `cmd /c` calls) results in additional spawns failing; `nono inspect` shows bounded process count.
2. Invalid values (`--max-processes 0`, negative, non-numeric) reject before launching.
3. Unit test reads back `ActiveProcessLimit` via `QueryInformationJobObject`.

**Maps to:** Phase 16.

---

## AIPC — Advanced IPC

Context: Phase 11 shipped runtime capability expansion over named pipe with `DuplicateHandle` brokering for file handles. AIPC-01 extends that protocol to cover other handle types that a sandboxed agent might need.

### AIPC-01: Extended handle brokering

**What:** The capability pipe protocol accepts requests for additional handle types beyond files, and brokers them into the child with the correct `DuplicateHandle` inheritance and access-mask semantics for each type.

**Handle types in scope:**
- **Socket handles** — TCP/UDP sockets the supervisor opened on behalf of the child (listener on a supervisor-chosen port). Inheritance via `WSADuplicateSocket` + in-child `WSASocket`. Access mask validated against the requested protocol/direction.
- **Named-pipe handles** — both ends of an anonymous pipe, or a specific instance of a named pipe the supervisor created. Inheritance via `DuplicateHandle` with `DUPLICATE_SAME_ACCESS`.
- **Job Object handles** — used by the child to assign subprocesses to a nested Job Object (when supported by Windows version). Rare but necessary for orchestration workloads. Inheritance via `DuplicateHandle`.
- **Event handles** — synchronization primitives the supervisor created (`CreateEventW`). Used for bidirectional lifecycle signaling (e.g., supervisor signals "shutdown").
- **Mutex handles** — cross-process mutex the supervisor owns, child can wait on. `DuplicateHandle`.

**Protocol:**
- Extend `SupervisorMessage::Request(CapabilityRequest)` with a `handle_type` discriminator.
- Supervisor validates:
  - Session token (existing Phase 11 check).
  - Access mask matches a policy-allowed subset (per-handle-type allowlist, default deny).
  - Child PID matches the expected sandbox owner (existing Phase 11 check).
- Approval UI:
  - File handles: existing CONIN$ prompt.
  - Socket handles: a different prompt that shows `protocol/port` instead of `path`.
  - Others: similar per-type UX with distinct labels.
- Audit: every granted handle is logged with its type, access mask, and grant reason.

**Security:**
- Fail-closed: unsupported or unknown handle types are rejected immediately with a constant-time comparison on the discriminator.
- Token leak audit (like Phase 11's `session_token_redaction` tests) must cover the new request shapes.
- Access-mask validation happens SERVER-SIDE — client-declared masks are untrusted.

**Acceptance:**
1. Protocol round-trip test per handle type: socket, pipe, Job Object, event, mutex. Each exercised via unit + integration test in `crates/nono/src/supervisor/`.
2. Policy denies an access-mask upgrade request (child asks for `FILE_ALL_ACCESS` when policy allows only `GENERIC_READ` — supervisor returns `Denied` with a reason).
3. Token-leak test extended to cover all new request shapes.
4. No platform regression: Unix builds either reject `--request-handle` at parse time or degrade gracefully (Linux has file-descriptor passing over Unix sockets as the natural equivalent; that's a separate cross-platform REQ, explicitly out of v2.1 scope).

**Maps to:** Phase 18.

---

## ATCH — Attach-Streaming

Context: Phase 15 deferred `nono attach` output streaming for detached Windows sessions. On the detached path, `start_logging` returns `Ok(())` with no PTY wiring, so `nono attach <id>` cannot stream child output. v2.1 closes this gap.

### ATCH-01: Full ConPTY re-attach on detached Windows sessions

**What:** `nono attach <session-id>` against a detached Windows session behaves like a real terminal: read child stdout, write to child stdin, and propagate resize events.

**Approach:**
- At detached-launch time, the supervisor still SKIPS initial ConPTY allocation (Phase 15 gate preserves `0xC0000142` fix).
- On the FIRST `nono attach` call after the child has already initialized, the supervisor creates a ConPTY pair, re-parents the child's console, and starts streaming.
- Alternative: use anonymous-pipe stdio at launch time but hand off to a real ConPTY on first attach. Investigate which is more reliable on Windows 10 17763+ vs 22H2+.

**Design unknowns (to resolve in Phase 17):**
- Can `AttachConsole` + `FreeConsole` + re-attach to a new ConPTY work on a DLL-loaded process mid-run without breaking the loader? Prior evidence says no (that's what Phase 15's 0xC0000142 proved).
- If full ConPTY is infeasible post-init, fall back to bidirectional anonymous pipe (read/write, no resize) — still a big improvement over "no output at all".

**Security:**
- Attach-pipe already has SDDL `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)` — preserved.
- Resize events from a remote attach client must be treated as untrusted input (validate dimensions, no injection).

**Acceptance:**
1. `nono run --detached -- cmd /c "for /l %i in (1,1,10) do @(echo %i & timeout /t 1)"` followed by `nono attach <id>` streams the output live.
2. `nono attach <id>` on a detached shell (`nono run --detached -- cmd.exe`) allows bidirectional use: stdin writes reach the child.
3. Terminal resize (Ctrl+Alt+F11, drag-to-resize) propagates to the child via `ResizePseudoConsole`.
4. Detach sequence (default or `--detach-sequence`) cleanly unparents the ConPTY without killing the child.
5. No regression on the 5-row Phase 15 smoke gate.

**Maps to:** Phase 17.

---

## CLEAN — Cleanup

Context: v2.0 accumulated pre-existing fmt drift, pre-existing Windows test flakes, and a scatter of uncommitted WIP files. v2.1 dedicates a phase to paying down this debt so we don't carry it forward again.

### CLEAN-01: Fmt drift fix

**What:** Run `cargo fmt --all`, verify the 3 drifted files from commit `6749494` (EnvVarGuard migration) come back to canon, and commit the result.

**Acceptance:**
1. `cargo fmt --all -- --check` returns clean.
2. Diff is confined to the files with known drift; no unrelated reformatting.
3. `make ci` re-includes `fmt --check` as a hard gate.

**Maps to:** Phase 19.

---

### CLEAN-02: Pre-existing Windows test flakes

**What:** Diagnose and fix the 5 Windows test flakes reproducible on clean HEAD (pre-Phase-15). Root cause is likely env-var test isolation (tests sharing `HOME`, `USERPROFILE`, `XDG_CONFIG_HOME` without using `EnvVarGuard` + `lock_env`).

**Flaky tests:**
- `capability_ext::tests::test_from_profile_allow_file_rejects_directory_when_exact_dir_unsupported`
- `capability_ext::tests::test_from_profile_filesystem_read_accepts_file_paths`
- `profile::builtin::tests::test_all_profiles_signal_mode_resolves`
- `query_ext::tests::test_query_path_sensitive_policy_includes_policy_source`
- `trust_keystore::tests::display_roundtrip_file`

**Acceptance:**
1. Each test is reviewed; env-var mutations migrated to `EnvVarGuard::set_all()` + `lock_env()` pattern (per `crates/nono-cli/src/test_env.rs`).
2. Parallel test execution (`cargo test` default) passes all 5 consistently across 10 iterations.
3. `cargo test --workspace --all-features` completes clean on Windows CI.

**Maps to:** Phase 19.

---

### CLEAN-03: Disk-resident WIP triage

**What:** Walk through every uncommitted or untracked file in `.planning/` (and project root) and decide: commit as-is, refactor and commit, or delete. Leave the working tree clean except for active v2.1 work.

**Files in scope:**
- `.planning/phases/10-etw-based-learn-command/10-RESEARCH.md` — untracked
- `.planning/phases/10-etw-based-learn-command/10-UAT.md` — untracked
- `.planning/phases/11-runtime-capability-expansion/11-01-PLAN.md` — modified
- `.planning/phases/11-runtime-capability-expansion/11-02-PLAN.md` — modified
- `.planning/phases/12-milestone-bookkeeping-cleanup/12-02-PLAN.md` — untracked
- `.planning/quick/260410-nlt-fix-three-uat-gaps-in-phase-10-etw-learn/` — untracked directory
- `.planning/quick/260412-ajy-safe-layer-roadmap-input/` — untracked directory
- `.planning/v1.0-INTEGRATION-REPORT.md` — untracked
- `host.nono_binary.commit` — untracked (project root)
- `query` — untracked (project root)

**Acceptance:**
1. For each file, a disposition is recorded (commit / rewrite / delete).
2. Working tree is clean after triage (modulo files reserved for v2.1 active work).
3. A short triage log lives in the Phase 19 SUMMARY so future sessions understand what was decided.

**Maps to:** Phase 19.

---

### CLEAN-04: Session-file housekeeping

**What:** Prune the 1172 stale session records accumulated in `~/.nono/sessions/` during v2.0 testing. Document a retention policy so session files don't re-accumulate at the same rate.

**Acceptance:**
1. `nono prune` removes stale records (non-dry-run); no running sessions affected.
2. A retention policy is documented in the supervisor runtime code (e.g., sessions older than N days get swept automatically on supervisor start, or on `nono ps`).
3. The policy itself is tested — unit test feeds a fabricated-old session, asserts sweep behavior.

**Maps to:** Phase 19.

---

## Out of Scope (v2.1)

| Item | Reason |
|------|--------|
| cgroup v2 or rlimit on Linux for RESL-01..04 | Separate cross-platform milestone; v2.1 is Windows-focused |
| `setrlimit` on macOS | Same as above; platform-support warnings are enough for v2.1 |
| RESL per-process memory (vs job-wide) | Threat model cares about tree total, not individual processes; follow-up |
| AIPC handle brokering between two sandboxed siblings | Not a v2.1 use case; existing supervisor-mediated flow covers known needs |
| Full PTY attach for Unix detached sessions | Unix detached sessions already work via socket_path; no analogous gap |
| Migrating `windows-supervised-exec-cascade.md` history back into a single debug session | Resolved doc is in `.planning/debug/resolved/`; further consolidation isn't load-bearing |

---

## Cross-platform note

All RESL flags (`--cpu-percent`, `--memory`, `--timeout`, `--max-processes`) accept on Unix but log `warning: resource limit not enforced on this platform for v2.1` when the native backend isn't wired yet. This is so agent developers can write a single CLI invocation that works everywhere, with the security guarantee documented per-platform. A follow-up cross-platform RESL milestone (v2.2 candidate) would add the native Unix backends.

ATCH-01 is Windows-only because Unix detached sessions already support attach streaming via the existing Unix socket path.

AIPC-01 is Windows-only because it extends Phase 11's Windows capability pipe. A cross-platform handle-passing abstraction is out of scope for v2.1.

CLEAN-01..04 are platform-agnostic in spirit; CLEAN-02 specifically targets Windows test flakes.

---

## UPST — Upstream Parity Sync

Context: The fork is pinned at crate version 0.30.1 while upstream `always-further/nono` has shipped 0.31–0.37.1. Phase 20 back-ports selected upstream features and the `rustls-webpki` RUSTSEC-2026-0098/0099 security upgrade while preserving all Windows-specific work from Phases 1–19. Decomposition and scope locked in `.planning/phases/20-upstream-parity-sync/20-CONTEXT.md`.

### UPST-01: Security upgrade + workspace version realignment

**What:** `rustls-webpki` transitive upgrade to `0.103.12` (clears RUSTSEC-2026-0098 and RUSTSEC-2026-0099) and workspace-wide crate version bump from `0.30.1` to `0.37.1` so the fork's published surface matches upstream `v0.37.1`.

**Acceptance:**
1. `cargo tree -i rustls-webpki` shows only versions `>= 0.103.12` in the transitive closure.
2. Every workspace crate (`nono`, `nono-cli`, `nono-proxy`, `nono-ffi`/`bindings-c`) reports `0.37.1` via `cargo pkgid`.
3. `cargo build --workspace` and `cargo test --workspace --all-features` exit 0 on the Windows host.
4. Phase 15 5-row detached-console smoke (`nono run <profile> → nono ps → nono stop`) passes unchanged.

**Maps to:** Phase 20 Plan 20-01.

### UPST-02: Profile & claude-code fixes

**What:** Port upstream profile `extends` infinite-recursion fix (commit `c1bc439`) and claude-code `.claude.json` symlink for token refresh (commit `97f7294`) into the fork's profile/hooks surface.

**Acceptance:**
1. A profile with `extends` referencing itself (direct or cyclic indirect) returns a clear `NonoError` within bounded time instead of stack-overflowing.
2. `.claude.json` symlink path resolution behaves as upstream v0.37.1 when refreshing claude-code tokens.
3. `make ci` passes on the Windows host.

**Maps to:** Phase 20 Plan 20-02.

### UPST-03: Credentials & environment parity

**What:** Port upstream `keyring://service/account` credential URI + `?decode=go-keyring` query-param handling (upstream 0.36), environment-variable filtering (upstream 0.37.0 #688, commit `1b412a7`), and `command_blocking_deprecation.rs` backport (upstream 0.33, ~190 lines).

**Acceptance:**
1. `keyring://service/account` parses into a typed URI variant; hostile inputs (path traversal, junk decode param) are rejected with `NonoError::InvalidConfig` (fail-closed).
2. An env-var filter profile rejects a malformed pattern up-front; well-formed filters allow/deny at the sandboxed process-env boundary as specified upstream.
3. `command_blocking_deprecation.rs` surfaces the deprecation warning for the documented command list and does not break existing `run`/`wrap`/`shell` paths.
4. `make ci` passes on the Windows host.

**Maps to:** Phase 20 Plan 20-03. If the keyring manual port exceeds ~400 lines the plan splits into 20-03a (keyring) and 20-03b (env-filter + deprecation) per CONTEXT § Specifics.

### UPST-04: GPU + trust parity

**What:** Port upstream `--allow-gpu` flag (upstream 0.31–0.33) with NVIDIA procfs + `nvidia-uvm-tools` Linux device allowlist (upstream 0.34), and GitLab ID tokens for trust signing (upstream 0.35) alongside the existing GitHub ID token path.

**Acceptance:**
1. `nono run --allow-gpu …` parses cleanly; on Linux the sandbox grants `/dev/nvidia*` + NVIDIA procfs + `nvidia-uvm-tools`; on macOS it grants Metal/GPU framework paths via Seatbelt; on Windows it accepts the flag with a `not-enforced-on-this-platform` warning (Phase 16 pattern).
2. A `nono trust` signing path that consumes a GitLab ID token completes end-to-end, mirroring the existing GitHub ID token test coverage.
3. `make ci` passes on the Windows host.

**Maps to:** Phase 20 Plan 20-04.
