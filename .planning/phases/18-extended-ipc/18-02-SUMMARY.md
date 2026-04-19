---
phase: 18-extended-ipc
plan: 02
subsystem: ipc
tags: [windows, supervisor, ipc, aipc, pipe, socket, wsaduplicatesocket, security, broker]

# Dependency graph
requires:
  - phase: 18-extended-ipc
    plan: 01
    provides: HandleKind/HandleTarget/SocketProtocol/SocketRole/PipeDirection enums; extended CapabilityRequest with kind/target/access_mask; ResourceTransferKind::SocketProtocolInfoBlob + protocol_info_blob field; cross-platform supervisor::policy module (PRIVILEGED_PORT_MAX, GENERIC_READ/WRITE, mask_is_allowed); broker_event_to_process / broker_mutex_to_process pattern; handle_event_request / handle_mutex_request / validate_aipc_object_name dispatcher helpers; constant-time discriminator validation step; format_capability_prompt total-helper structure
  - phase: 17-attach-streaming
    provides: WindowsSupervisorRuntime.user_session_id (16-hex user-facing ID); Phase 17 latent-bug carry-forward pattern (kernel-object names use user_session_id, not self.session_id)
provides:
  - ResourceGrant::duplicated_windows_pipe_handle (encodes PipeDirection in AccessMode for audit readability)
  - ResourceGrant::socket_protocol_info_blob (carries 372-byte WSAPROTOCOL_INFOW serialized blob via existing SocketProtocolInfoBlob variant)
  - broker_pipe_to_process in socket_windows.rs (DuplicateHandle with dwOptions=0 + direction-mapped GENERIC_READ/GENERIC_WRITE — MAP DOWN enforced)
  - broker_socket_to_process in socket_windows.rs (WSADuplicateSocketW + 372-byte blob serialization, target-PID-bound)
  - bind_aipc_pipe in socket_windows.rs (CreateNamedPipeW with byte-identical Phase 11 CAPABILITY_PIPE_SDDL via build_low_integrity_security_attributes; PIPE_UNLIMITED_INSTANCES so concurrent AIPC names don't compete)
  - BrokerTargetProcess::pid accessor + broker_target_pid free function (needed by Socket broker — WSADuplicateSocketW takes a PID, not a HANDLE)
  - handle_pipe_request driver in supervisor.rs (target shape + direction decode + namespace canonicalization with user_session_id + bind/broker/close)
  - handle_socket_request driver in supervisor.rs (target shape + privileged-port unconditional reject + role-based default-allowlist + WSAStartup/WSASocketW + broker/close)
  - format_capability_prompt Pipe + Socket branches (D-04-locked templates replacing Plan 18-01 placeholder stubs)
  - format_pipe_direction helper in terminal_approval.rs
affects: [phase-18-03, phase-18-04]

# Tech tracking
tech-stack:
  added: []  # Win32_Networking_WinSock feature added to existing windows-sys dep on both nono + nono-cli; no new crate deps
  patterns:
    - "MAP DOWN: DuplicateHandle with dwOptions=0 and explicit access mask (NOT DUPLICATE_SAME_ACCESS) so the duplicated handle never inherits supervisor source's full access — same pattern as Plan 18-01 Event/Mutex brokers, applied to Pipe via direction-mapped GENERIC_READ/WRITE"
    - "WSADuplicateSocketW + WSAPROTOCOL_INFOW serialization: supervisor opens source SOCKET, calls WSADuplicateSocketW(socket, target_pid, &mut blob), serializes blob to Vec<u8> via from_raw_parts, transports inline; child reconstructs via WSASocketW(FROM_PROTOCOL_INFO, ...)"
    - "Socket lifecycle ordering: supervisor closes source via closesocket() AFTER broker returns; kernel keeps underlying socket alive until ALL descriptors close"
    - "Server-side namespace canonicalization (\\\\.\\pipe\\nono-aipc-<user_session_id>-<sanitized_name>) using user-facing 16-hex (Phase 17 carry-forward)"
    - "Privileged-port unconditional deny (port <= 1023) at request validation time — cannot be widened by profile in v2.1"
    - "Role-based default allowlist (Connect-only) for sockets — Plan 18-03 layers profile widening; Plan 18-02 hard-codes the default per D-05"
    - "Direction-based default allowlist (read OR write, not both) for pipes — same widening pattern as sockets"

key-files:
  created:
    - .planning/phases/18-extended-ipc/18-02-SUMMARY.md
  modified:
    - crates/nono/Cargo.toml
    - crates/nono/src/supervisor/types.rs
    - crates/nono/src/supervisor/socket_windows.rs
    - crates/nono/src/supervisor/mod.rs
    - crates/nono-cli/Cargo.toml
    - crates/nono-cli/src/exec_strategy_windows/supervisor.rs
    - crates/nono-cli/src/terminal_approval.rs

key-decisions:
  - "MAP DOWN over DUPLICATE_SAME_ACCESS for Pipe broker: dwOptions=0 forces the duplicated handle's mask to be the explicit GENERIC_READ/GENERIC_WRITE argument, not the supervisor source's full PIPE_ACCESS_DUPLEX. T-18-02-01 mitigation; mirrors Plan 18-01 Event/Mutex pattern."
  - "ReadWrite pipe direction explicitly rejected at request validation time (default allowlist is read OR write, not both). Plan 18-03 wires the profile-widening lookup that grants ReadWrite when the profile opts in."
  - "Socket privileged-port deny is UNCONDITIONAL (port <= 1023). Cannot be widened by profile in v2.1 — locked in CONTEXT.md <specifics> line 167. Public allowlist contents acceptable per D-09."
  - "Socket role default allowlist: Connect-only. Bind and Listen require profile widening (Plan 18-03). Plan 18-02 hard-codes the default; the resolver in supervisor::policy is ready for profile widening."
  - "Socket lifecycle ordering: supervisor's source SOCKET closes via closesocket() AFTER broker_socket_to_process returns the serialized WSAPROTOCOL_INFOW blob. The kernel keeps the underlying socket alive until ALL descriptors close, and the duplicated descriptor only materializes when the child calls WSASocketW(FROM_PROTOCOL_INFO, ...). T-18-02-04 mitigation."
  - "bind_aipc_pipe reuses build_low_integrity_security_attributes (Phase 11 helper) directly — the existing Phase 11 helper already encapsulates the SDDL parsing + Drop guard, so no SDDL extraction refactor was needed. CAPABILITY_PIPE_SDDL count stays at 2 (the constant declaration + doc reference) byte-identical to Plan 18-01."
  - "PIPE_UNLIMITED_INSTANCES (not 1) for AIPC pipes so concurrent AIPC capability requests with distinct canonical names don't compete for the single-instance slot the supervisor control pipe uses. The kernel-object name itself is the uniqueness gate (canonicalized server-side from user_session_id + raw_name)."
  - "BrokerTargetProcess::pid + broker_target_pid free function added to socket_windows.rs because WSADuplicateSocketW takes a DWORD process ID, not a HANDLE. GetProcessId returns 0 on failure; broker_target_pid falls back to GetCurrentProcessId() for safety in tests / current-process scenarios."
  - "Drive-by rustfmt: rustfmt re-formatted Task 1+2 test bodies after Task 3 type/import changes triggered re-formatting. Folded into the Task 3 commit (matches Plan 18-01 pattern with the 3ea2017 fixup)."
  - "JobObject branch deliberately preserved as the SOLE remaining placeholder (with explicit 'JobObject brokering not yet implemented in this build' Denied reason) so Plan 18-03 owns it cleanly. Acceptance grep verifies count=1."

patterns-established:
  - "Pattern 6 (extends 18-01 Pattern 3): Per-handle-type broker functions in socket_windows.rs follow the broker_event/broker_mutex template byte-for-byte except for: explicit access mask source (mask param for Event/Mutex/Pipe; pid+role for Socket); per-kind ResourceGrant constructor; and per-kind transport mechanism (DuplicateHandle for Event/Mutex/Pipe; WSADuplicateSocketW for Socket). Plan 18-03 (Job Object) follows this exact template."
  - "Pattern 7: Server-side AIPC pipe creator (bind_aipc_pipe) reuses Phase 11 build_low_integrity_security_attributes to keep the SDDL byte-identical between supervisor capability pipes and AIPC handoff pipes. Cross-cutting refactors are deferred until a third caller appears."
  - "Pattern 8: Socket-class brokers carry both target_pid AND target_process — the PID is needed for WSADuplicateSocketW; the BrokerTargetProcess wrapper is kept in the signature for symmetry with non-socket brokers and to support future ownership tracking."

requirements-completed: [AIPC-01]

# Metrics
duration: 90m
completed: 2026-04-19
---

# Phase 18 Plan 02: Extended IPC (AIPC-01) Pipe + Socket Brokers Summary

**Pipe and Socket handle brokering wired end-to-end via DuplicateHandle (with dwOptions=0 MAP DOWN enforcement) and WSADuplicateSocketW (with target-PID-bound 372-byte WSAPROTOCOL_INFOW blob serialization); 7 new dispatcher tests + 6 new broker tests + 4 new types tests + 3 new prompt tests all green on Windows host.**

## Performance

- **Duration:** ~90 min
- **Started:** 2026-04-19T22:00:00Z (approximately)
- **Completed:** 2026-04-19T23:30:00Z (approximately)
- **Tasks:** 3
- **Files modified:** 7 (1 new SUMMARY, 7 modified)
- **Test count delta:** +20 unit tests (+4 types, +6 socket-broker, +3 terminal_approval, +7 capability_handler — alongside all pre-existing tests still passing)

## Accomplishments

- ResourceGrant::duplicated_windows_pipe_handle constructor (encodes PipeDirection in AccessMode for audit readability) and ResourceGrant::socket_protocol_info_blob constructor (carries the 372-byte WSAPROTOCOL_INFOW serialized blob via the SocketProtocolInfoBlob variant Plan 18-01 forward-declared).
- broker_pipe_to_process in socket_windows.rs: DuplicateHandle with dwOptions=0 and explicit direction-mapped mask (GENERIC_READ / GENERIC_WRITE / both). MAP DOWN enforced + documented in the SAFETY block. Mirrors Plan 18-01's Event/Mutex broker pattern with the per-kind direction-to-mask translation.
- broker_socket_to_process in socket_windows.rs: WSADuplicateSocketW serializes the supervisor source SOCKET into a 372-byte target-PID-bound WSAPROTOCOL_INFOW blob. Per RESEARCH Landmines § Socket: source SOCKET stays open until AFTER the helper returns; closesocket happens via the dispatcher caller (handle_socket_request) AFTER broker_socket_to_process returns the serialized blob.
- bind_aipc_pipe helper in socket_windows.rs: CreateNamedPipeW with the byte-identical Phase 11 CAPABILITY_PIPE_SDDL via the existing build_low_integrity_security_attributes helper. PIPE_UNLIMITED_INSTANCES so concurrent AIPC pipes with distinct canonical names don't compete for the single-instance slot.
- BrokerTargetProcess::pid accessor + broker_target_pid free function: WSADuplicateSocketW takes a DWORD PID (not a HANDLE), so the Socket broker dispatcher needs this. GetProcessId returns 0 on bad handle (no UB); broker_target_pid falls back to GetCurrentProcessId for safety.
- handle_pipe_request driver in supervisor.rs: validates target shape, decodes direction from access_mask, enforces default allowlist (read OR write, not both — ReadWrite explicitly rejected pending Plan 18-03 profile widening), canonicalizes namespace as `\\.\pipe\nono-aipc-<user_session_id>-<sanitized_name>` using user_session_id (Phase 17 latent-bug carry-forward), calls bind_aipc_pipe + broker_pipe_to_process, closes source via CloseHandle per D-10.
- handle_socket_request driver in supervisor.rs: validates target shape, enforces unconditional privileged-port deny (port <= PRIVILEGED_PORT_MAX = 1023), enforces Connect-only default allowlist (Bind/Listen rejected pending Plan 18-03 profile widening), sanitizes host string (rejects empty, >253 bytes, control bytes / NUL), calls WSAStartup + WSASocketW (TCP or UDP per protocol), brokers via broker_socket_to_process with the target PID resolved via broker_target_pid, closes source via closesocket per D-10.
- handle_windows_supervisor_message dispatcher: HandleKind::Socket and HandleKind::Pipe arms wired to the new helpers; HandleKind::JobObject is the SOLE remaining placeholder (returns "JobObject brokering not yet implemented in this build" Denied — Plan 18-03 owns it).
- format_capability_prompt in terminal_approval.rs: Pipe and Socket branches replace the Plan 18-01 placeholder stubs with the D-04-locked templates ("[nono] Grant pipe access? name=<n> direction=<read|write|read+write>" and "[nono] Grant socket access? proto=<tcp|udp> host=<h> port=<p> role=<connect|bind|listen>"). format_pipe_direction helper added (mirrors format_event_access / format_mutex_access). All untrusted strings (host, name, reason) routed through sanitize_for_terminal.
- prompt_falls_back_for_unsupported_kind test updated to use HandleKind::JobObject (Socket no longer in the placeholder fallback after this plan).
- 21/21 capability_handler_tests pass (7 Phase 11 + 7 Plan 18-01 + 7 Plan 18-02). 23/23 terminal_approval tests pass (20 prior + 3 new). 20/20 supervisor::socket tests pass (14 prior + 6 new). 10/10 supervisor::types tests pass (6 prior + 4 new). 6/6 supervisor::policy tests unchanged.
- Phase 11 + Plan 18-01 invariants byte-identical: CAPABILITY_PIPE_SDDL count=2, "Invalid session token" count=2, audit_entry_with_redacted_token routes all push sites, broker_event_to_process / broker_mutex_to_process / handle_event_request / handle_mutex_request / validate_aipc_object_name preserved unchanged.

## Task Commits

Each task was committed atomically with DCO sign-off:

1. **Task 1: Win32_Networking_WinSock + ResourceGrant pipe/socket constructors** — `39c5a82` (feat)
2. **Task 2: broker_pipe_to_process + broker_socket_to_process + bind_aipc_pipe + BrokerTargetProcess::pid** — `834e534` (feat)
3. **Task 3: Pipe + Socket dispatcher arms + format_capability_prompt branches + tests** — `0a1f2ee` (feat) — also includes drive-by rustfmt cleanup on Task 1+2 test bodies that drifted after the Task 3 imports triggered re-formatting

## Files Created/Modified

- `crates/nono/Cargo.toml` — Added `Win32_Networking_WinSock` to the `windows-sys` features list (alphabetical order). All existing features preserved.
- `crates/nono/src/supervisor/types.rs` — `duplicated_windows_pipe_handle` and `socket_protocol_info_blob` constructors added to the existing `impl ResourceGrant` block. 4 new unit tests in the existing `#[cfg(test)] mod tests` block.
- `crates/nono/src/supervisor/socket_windows.rs` — `broker_pipe_to_process`, `broker_socket_to_process`, `bind_aipc_pipe` functions; `BrokerTargetProcess::pid` accessor; `broker_target_pid` free function. Imports extended with WinSock (WSADuplicateSocketW, WSAPROTOCOL_INFOW, SOCKET, INVALID_SOCKET, closesocket) and Pipes (PIPE_UNLIMITED_INSTANCES) and Threading (GetCurrentProcessId, GetProcessId). 6 new unit tests.
- `crates/nono/src/supervisor/mod.rs` — Re-export block extended with `bind_aipc_pipe`, `broker_pipe_to_process`, `broker_socket_to_process`, `broker_target_pid` (Windows-only).
- `crates/nono-cli/Cargo.toml` — Added `Win32_Networking_WinSock` to the `windows-sys` features list (alphabetical order). All existing features preserved.
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — Imports extended with PipeDirection, SocketProtocol, SocketRole, the new broker functions, and WinSock (WSASocketW, WSAStartup, AF_INET, INVALID_SOCKET, IPPROTO_TCP/UDP, SOCK_DGRAM, SOCK_STREAM, WSADATA, WSA_FLAG_OVERLAPPED, closesocket). `handle_pipe_request` and `handle_socket_request` helpers added (~150 lines). Dispatcher match-arm split: `HandleKind::Pipe`, `HandleKind::Socket`, `HandleKind::JobObject` are now 3 separate arms; the Socket/Pipe arms call the new helpers; the JobObject arm keeps the Plan 18-01 placeholder Denied. 7 new dispatcher tests in `capability_handler_tests`.
- `crates/nono-cli/src/terminal_approval.rs` — `format_pipe_direction` helper added (mirrors format_event_access). `format_capability_prompt` Pipe and Socket branches replace the Plan 18-01 placeholder stubs with D-04-locked templates. SocketProtocol + SocketRole imports added. `prompt_falls_back_for_unsupported_kind` test updated to use JobObject (Socket no longer placeholder). 3 new prompt tests.

## Decisions Made

See `key-decisions` block in frontmatter. Highlights:

- **MAP DOWN over DUPLICATE_SAME_ACCESS for Pipe** — same critical safety guarantee as Plan 18-01's Event/Mutex brokers; explicitly documented in the SAFETY block and acceptance-criteria-grep-asserted via `// dwOptions = 0 — MAP DOWN` annotation.
- **Privileged-port deny is UNCONDITIONAL (cannot be widened by profile)** — per CONTEXT.md `<specifics>` line 167. Locked in this plan.
- **ReadWrite pipe direction REJECTED in default allowlist** — explicit "profile widening required" reason. Plan 18-03 wires the lookup; until then ReadWrite is unreachable.
- **Bind/Listen socket roles REJECTED in default allowlist** — same explicit "profile widening required" reason. Same Plan 18-03 deferral.
- **Socket lifecycle ordering** — supervisor closes its source SOCKET via `closesocket()` AFTER `broker_socket_to_process` returns the serialized WSAPROTOCOL_INFOW blob. Documented in handle_socket_request + broker_socket_to_process.
- **No SDDL extraction refactor** — `bind_aipc_pipe` reuses the existing Phase 11 `build_low_integrity_security_attributes` helper directly. CAPABILITY_PIPE_SDDL count stays at 2 (declaration + doc) byte-identical.
- **JobObject is the SOLE remaining placeholder** — explicit Denied reason ("JobObject brokering not yet implemented in this build"); Plan 18-03 wires the broker + the runtime guard against brokering the supervisor's own containment_job.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] clippy::unnecessary_cast on SOCK_STREAM / IPPROTO_TCP in test bodies**
- **Found during:** Task 2 (cargo clippy gate after broker_socket_to_process unit test added)
- **Issue:** `windows-sys` defines `SOCK_STREAM` and `IPPROTO_TCP` as `i32` already, so `as i32` triggered clippy's `unnecessary_cast` lint at `crates/nono/src/supervisor/socket_windows.rs:1518` and `:1519`.
- **Fix:** Dropped the `as i32` casts on both constants in the test body (the surrounding `AF_INET as i32` cast remains because AF_INET is u16 in this binding).
- **Files modified:** crates/nono/src/supervisor/socket_windows.rs
- **Verification:** `cargo clippy -p nono --all-targets -- -D warnings -D clippy::unwrap_used` passes; the test still compiles + passes.
- **Committed in:** Task 2 commit `834e534`.

**2. [Rule 3 - Blocking] Win32_Networking_WinSock added to nono-cli's Cargo.toml too**
- **Found during:** Task 3 planning (the plan stated nono-cli already had the feature per RESEARCH Environment Availability table; verification showed it did NOT).
- **Issue:** Task 3 needs `WSASocketW`, `WSAStartup`, `AF_INET`, `IPPROTO_TCP/UDP`, etc. from `Win32_Networking_WinSock`. nono-cli's Cargo.toml lacked the feature.
- **Fix:** Added `Win32_Networking_WinSock` to nono-cli's `windows-sys` features list (alphabetical order).
- **Files modified:** crates/nono-cli/Cargo.toml
- **Verification:** `cargo build -p nono-cli --bin nono` succeeds.
- **Committed in:** Task 3 commit `0a1f2ee`.

**3. [Rule 1 - Lint] Drive-by rustfmt re-formatted Task 1+2 test bodies after Task 3 imports**
- **Found during:** Task 3 (cargo fmt --all -- --check gate)
- **Issue:** Adding new imports + larger test bodies in Task 3 caused rustfmt to also re-format Task 1+2 test code (line wrapping changes — purely cosmetic). Pre-existing fmt drift in `crates/nono-cli/src/exec_strategy_windows/launch.rs` and `crates/nono-cli/src/session_commands_windows.rs` (carried forward from Plan 18-01 SUMMARY's "Deferred Issues #3") was also flagged.
- **Fix:** Folded the in-scope rustfmt changes (types.rs + socket_windows.rs) into the Task 3 commit (matches Plan 18-01 pattern with the 3ea2017 fixup commit). Reverted out-of-scope drift on `launch.rs` and `session_commands_windows.rs` to keep this plan's diff scope strictly to the `files_modified` allowlist.
- **Files modified:** crates/nono/src/supervisor/types.rs, crates/nono/src/supervisor/socket_windows.rs
- **Verification:** `cargo fmt --all -- --check` exits 0 on the in-scope files.
- **Committed in:** Task 3 commit `0a1f2ee`.

---

**Total deviations:** 3 auto-fixed (1 lint, 1 blocking-on-build, 1 fmt cleanup)
**Impact on plan:** All deviations were lint/build/fmt compliance, not scope changes. No new files added beyond plan; no behavior diverged from CONTEXT.md decisions. Pre-existing fmt drift on launch.rs / session_commands_windows.rs explicitly preserved as out-of-scope (matches Plan 18-01's Deferred Issues #3).

## Phase 11 + Plan 18-01 Invariant Verification

Verified via `git diff` + grep counts on the post-Task-3 working tree:

| Invariant | Baseline | Post-Plan | Status |
|-----------|----------|-----------|--------|
| `S:(ML;;NW;;;LW)` SDDL count in socket_windows.rs | 2 (const + doc) | 2 | byte-identical |
| `"Invalid session token"` count in supervisor.rs | 2 | 2 | byte-identical |
| `audit_entry_with_redacted_token` routes all push sites (no bare `audit_log.push(AuditEntry`) | 0 | 0 | byte-identical |
| `CONIN$` count in terminal_approval.rs | 4 | 4 | byte-identical |
| `request_capability` body in terminal_approval.rs | unchanged | unchanged | byte-identical |
| `fn handle_event_request | fn handle_mutex_request | fn validate_aipc_object_name` count | 3 | 3 | byte-identical |
| `pub fn broker_event_to_process | pub fn broker_mutex_to_process` count | 2 | 2 | byte-identical |
| Plan 18-01 placeholder strings (`Socket / Pipe brokering not yet implemented`) | 1 (combined arm) | 0 | replaced by live brokers |
| `JobObject brokering not yet implemented` placeholder | (combined w/ Socket/Pipe) | 1 | preserved as sole remaining placeholder |

`git diff HEAD~3 HEAD -- crates/nono-cli/src/exec_strategy_windows/supervisor.rs | grep '^-.*Invalid session token\|^-.*audit_entry_with_redacted_token\|^-.*handle_event_request\|^-.*handle_mutex_request\|^-.*validate_aipc_object_name\|^-.*ConstantTimeEq.*expected_session_token'`: 0 lines (no deletions of Phase 11 or Plan 18-01 primitive lines).

## Phase 17 Latent-Bug Carry-Forward Verification

| Check | Required | Actual | Status |
|-------|----------|--------|--------|
| `format!.*nono-aipc.*self\.session_id` count in supervisor.rs | 0 | 0 | PASS — no new bug introduced |
| `format!.*nono-aipc.*user_session_id` count in supervisor.rs | >= 3 (Event + Mutex from 18-01 + Pipe from 18-02) | 3 | PASS — Pipe namespace prefix uses user_session_id at the new site |

The three `format!.*nono-aipc.*user_session_id` sites are inside `handle_event_request` (the Plan 18-01 site), `handle_mutex_request` (the Plan 18-01 site), and `handle_pipe_request` (the new Plan 18-02 site at line ~1269). All three construct the canonical `Local\\nono-aipc-{}-{}` (Event/Mutex) or `\\.\pipe\nono-aipc-{}-{}` (Pipe) namespace prefix using the `user_session_id: &str` parameter passed through the dispatcher signature.

## CI Gate Results

| Gate | Result |
|------|--------|
| `cargo build -p nono --lib` | PASS |
| `cargo build -p nono-cli --bin nono` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | PASS (clean) |
| `cargo fmt --all -- --check` (on plan files) | PASS |
| `cargo test -p nono --lib supervisor::types` | PASS (10/10 — 6 from 18-01 + 4 new) |
| `cargo test -p nono --lib supervisor::policy` | PASS (6/6 — unchanged from 18-01) |
| `cargo test -p nono --lib supervisor::socket -- --test-threads=1` (Windows) | PASS (20/20 — 14 prior + 6 new) |
| `cargo test -p nono-cli --bin nono terminal_approval` | PASS (23/23 — 20 prior + 3 new) |
| `cargo test -p nono-cli --bin nono capability_handler_tests` | PASS (21/21 — 7 Phase 11 + 7 Plan 18-01 + 7 Plan 18-02) |

Pre-existing `tests/env_vars.rs` (19) and `trust_keystore` flakes (1-3) carried forward unchanged from Phase 19 CLEAN-02 deferred list per Plan 18-01 SUMMARY § Deferred Issues #2 — confirmed not in this plan's scope and not regressed.

## Issues Encountered

The plan's `<interfaces>` block accurately captured every Plan 18-01 type/function/field shape needed; no codebase exploration was needed beyond what the plan loaded into context.

The single small friction was the plan's claim that nono-cli already had `Win32_Networking_WinSock` (per RESEARCH Environment Availability table) — verification showed it did NOT. Added in Task 3 commit. Documented as Rule 3 deviation #2 above.

The Socket dispatcher tests (`handle_denies_socket_with_privileged_port` and `handle_denies_socket_bind_role_without_profile_widening`) intentionally land the rejection inside `handle_socket_request` (not before backend dispatch) — this matches the Plan 18-01 broker-failure pattern where the backend is consulted, the broker call returns Err, the audit decision shows Granted with grant=None, and the child receives a Granted response with grant=None. Documented inline in the test bodies.

## Deferred Issues

**1. JobObject branch still placeholder**

Plan 18-03 owns this — it's the SOLE remaining `HandleKind::JobObject` arm with the structured Denied reason "JobObject brokering not yet implemented in this build". Acceptance-criteria-grep-asserted as count=1.

**2. format_capability_prompt helpers retain `#[allow(dead_code)]`**

Plan 18-01 added the helpers with `#[allow(dead_code)]` because the dispatcher (Task 5 / Plan 18-01) wires `format!` strings inline rather than routing through the helper. Plan 18-02 doesn't change that — the helpers stay tested via the new + existing tests but are still not consumed by production code. The dead_code allows will be removed when a future plan (likely Plan 18-04 or beyond) wires the live CONIN$ prompt path through `format_capability_prompt`.

**3. Pre-existing `tests/env_vars.rs` (19 failures) and `trust_keystore` flakes (1-3) carried forward unchanged**

Per STATE.md and Phase 19 CLEAN-02 deferred list. Confirmed not regressed by this plan.

**4. Pre-existing fmt drift on `launch.rs` and `session_commands_windows.rs`**

Carried forward from Plan 18-01 SUMMARY's Deferred Issues #3. Out of this plan's scope; reverted before commit. Would need a separate quick-task or Phase 19 follow-up commit.

**5. Profile widening for ReadWrite pipe direction + Bind/Listen socket roles**

Plan 18-03 deliverable. Plan 18-02 ships with default-only enforcement and explicit "profile widening required" rejection messages so the Plan 18-03 work has a clear extension point.

**6. Hostname resolution + actual connect/bind/listen action for Sockets**

Plan 18-02 hands the child a FRESH socket (no server-side connect performed); the child performs the actual connect against the validated endpoint baked into the audit log. This matches the AIPC-01 acceptance criterion #1 ("supervisor opened a socket on behalf of the child") in REQUIREMENTS.md. Future plans (potentially Plan 18-03 or beyond) may extend handle_socket_request to perform the connect server-side for connect role + bind/listen for the widened roles.

## Open Paths for Plan 18-03

Plan 18-02 deliberately leaves these tagged-but-unwired surfaces for the next plan:

- **In `format_capability_prompt`** (terminal_approval.rs): JobObject branch still returns the placeholder `(unsupported in this build)` string. 18-03 replaces it with the D-04-locked Job Object template.
- **In dispatcher match-arm** (supervisor.rs): `HandleKind::JobObject` arm returns structured Denied with reason "JobObject brokering not yet implemented in this build". 18-03 wires it via a new `handle_jobobject_request` helper following the pattern established by `handle_event_request` / `handle_mutex_request` / `handle_pipe_request`.
- **In `resolved_mask_for_kind`** (supervisor.rs): JobObject returns the hard-coded `JOB_OBJECT_DEFAULT_MASK`; 18-03 layers profile widening via a new `resolved_aipc_allowlist` field on `WindowsSupervisorRuntime` (the same widening mechanism that grants ReadWrite pipe direction + Bind/Listen socket roles).
- **JobObject runtime guard**: 18-03 must add a structural check in `handle_jobobject_request` that rejects any request to broker the supervisor's own `containment_job` (the Job Object the supervisor uses to enforce process tree termination + RESL caps). Brokering it to the child would let the child terminate the supervisor's containment.
- **Profile JSON schema additive widening**: 18-03 extends the profile schema with an `aipc.allowlist` block + the resolver that consumes it.

## Next Phase Readiness

- Plan 18-03 (Job Object + profile schema + audit suite cleanup) can proceed against this foundation.
- All Phase 11 + Plan 18-01 invariants byte-identical; no carryover risk.
- Phase 17 latent-bug pattern (user_session_id NOT self.session_id) explicitly tested and verified at the new Pipe namespace site.
- 21 capability_handler_tests pass on Windows host; the dispatcher is total over all 6 HandleKind values (no arm panics; only JobObject still returns the deliberate placeholder Denied).
- Cross-platform compile holds: nono builds clean on Linux/macOS (the broker functions and Win32 imports are gated by the file-routing `#[path]` in supervisor/mod.rs).
- `Win32_Networking_WinSock` feature now in BOTH nono and nono-cli Cargo.toml — Plan 18-04 SDK methods can use Winsock types without further Cargo.toml changes.

## Self-Check: PASSED

All 8 files referenced in this summary exist on disk; all 3 commits referenced exist in git log.
- `crates/nono/Cargo.toml` — FOUND
- `crates/nono/src/supervisor/types.rs` — FOUND
- `crates/nono/src/supervisor/socket_windows.rs` — FOUND
- `crates/nono/src/supervisor/mod.rs` — FOUND
- `crates/nono-cli/Cargo.toml` — FOUND
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — FOUND
- `crates/nono-cli/src/terminal_approval.rs` — FOUND
- `.planning/phases/18-extended-ipc/18-02-SUMMARY.md` — FOUND (this file)
- Commits 39c5a82, 834e534, 0a1f2ee — all FOUND in `git log`

---
*Phase: 18-extended-ipc*
*Completed: 2026-04-19*
