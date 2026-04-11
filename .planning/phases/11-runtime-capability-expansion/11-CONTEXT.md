# Phase 11: Runtime Capability Expansion - Context

**Gathered:** 2026-04-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Enable a sandboxed child process (running under the Windows supervisor) to request additional filesystem capabilities at runtime by sending a `RequestCapability` message to the supervisor over a dedicated named-pipe IPC channel. The supervisor prompts the user interactively for approval, then either brokers a file handle to the child or denies with a reason.

This phase does NOT add new CLI flags, new output formats, or changes to Unix supervisor paths. The `WindowsSupervisorDenyAllApprovalBackend` is replaced by a wired-up `TerminalApproval` backend for the Windows supervised run path.

</domain>

<decisions>
## Implementation Decisions

### D-01: Session Token — Separate Field, Env Var Delivery

Add a `session_token: String` field to `CapabilityRequest` (in `crates/nono/src/supervisor/types.rs`).

- Supervisor generates a 32-byte random hex secret at startup (per session).
- Token is injected into the child process environment as `NONO_SESSION_TOKEN`.
- Every incoming `CapabilityRequest` must include the token; the supervisor performs a constant-time comparison (`subtle::ConstantTimeEq` or equivalent) **before** the approval backend is consulted.
- Requests with a missing or mismatched token are denied immediately.
- The token is **never written to any log level** — not `debug!`, not `trace!`. Audit log entries must omit the token field entirely.

### D-02: Child Pipe Connectivity — Dedicated Named Pipe via Rendezvous File

Use a dedicated capability request pipe (separate from the existing control pipe `nono-session-{id}`).

- Supervisor calls `SupervisorSocket::bind(rendezvous_path)` where `rendezvous_path` is a temp file path unique to the session.
- The rendezvous path is passed to the child via the `NONO_SUPERVISOR_PIPE` environment variable.
- Child calls `SupervisorSocket::connect(rendezvous_path)` to connect.
- The existing `pair()` anonymous pipe in `initialize_supervisor_control_channel()` is unrelated to capability requests and should remain as-is (or be renamed to clarify scope).
- This follows the existing `bind()`/`connect()` pattern already implemented in `supervisor/socket_windows.rs`.

### D-03: Request Listener — Background Thread with mpsc Audit Channel

Capability requests are served in a background thread, following the exact same pattern as the existing control pipe server and data pipe server.

- A new method (e.g., `start_capability_pipe_server()`) spawns a `std::thread::spawn` background thread.
- The thread calls `SupervisorSocket::bind()`, waits for the child to connect, then loops calling `recv_message()` and dispatching to `handle_windows_supervisor_message()`.
- `handle_windows_supervisor_message()` is promoted from `#[cfg(test)]` to production. The `#[cfg(test)]` gate is removed.
- Audit entries from the handler are sent back to the main supervisor struct via a `std::sync::mpsc::channel`. The event loop (`run_child_event_loop`) drains the receiver each iteration.

### D-04: Windows Terminal Approval — CONIN$ Console Device

`TerminalApproval` gains a platform branch:

- **Unix** (existing): opens `/dev/tty`
- **Windows** (new): opens `\\.\CONIN$` as the equivalent console input device

The sanitize logic (`sanitize_for_terminal`), prompt format (`[nono] Grant access? [y/N]`), and y/N response parsing are shared between platforms. The `is_terminal()` check on stderr remains unchanged.

### D-05: Approval Scope — Filesystem Only, Extensible Response

The approval backend for this phase grants filesystem path access only (open file handle, `DuplicateHandle` into child). This matches SC #1 and the existing `ResourceGrant` / `GrantedResourceKind::File` types.

The approval response structure (`ResourceGrant.resource_kind`) is already extensible — no design changes needed for future resource types.

### Claude's Discretion

- Token generation: `getrandom::fill` (already used in `socket_windows.rs` for nonce generation) → 32 bytes → hex-encoded → 64-char string.
- Rendezvous path location: session temp directory or `std::env::temp_dir()` + session-unique filename (e.g., `nono-cap-{session_id}.pipe`).
- Replay protection: `handle_windows_supervisor_message` already has `seen_request_ids: HashSet<String>` — keep as-is, no changes needed.
- Thread shutdown: capability pipe server thread exits naturally when the child disconnects (pipe EOF); no explicit shutdown signal needed beyond what Drop already does.
- `TerminalApproval` on Windows when no console is attached: fall back to `ApprovalDecision::Denied` with reason "No console available for interactive approval" (consistent with existing Unix behavior when stderr is not a terminal).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Existing supervisor IPC infrastructure
- `crates/nono/src/supervisor/types.rs` — `CapabilityRequest`, `ApprovalDecision`, `SupervisorMessage`, `SupervisorResponse`, `ResourceGrant`, `AuditEntry` types
- `crates/nono/src/supervisor/socket_windows.rs` — `SupervisorSocket` (bind/connect/pair), `BrokerTargetProcess`, `broker_file_handle_to_process()`
- `crates/nono/src/supervisor/mod.rs` — `ApprovalBackend` trait definition

### Windows supervisor implementation
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — `WindowsSupervisorRuntime`, `handle_windows_supervisor_message()` (currently `#[cfg(test)]`), `initialize_supervisor_control_channel()`, control pipe server, data pipe server
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` — `WindowsSupervisorDenyAllApprovalBackend` (to be replaced), `SupervisorConfig`, `ExecConfig`

### Approval backend
- `crates/nono-cli/src/terminal_approval.rs` — `TerminalApproval`, `sanitize_for_terminal()`, `/dev/tty` open pattern (needs Windows CONIN$ branch)
- `crates/nono-cli/src/supervised_runtime.rs` — where `TerminalApproval` is instantiated for Unix (line ~157); Windows equivalent needs wiring

### Requirements
- `.planning/REQUIREMENTS.md` — TRUST-01 acceptance criteria
- `.planning/ROADMAP.md` — Phase 11 success criteria (SC 1–4)

### Windows platform patterns
- `crates/nono-cli/src/session_commands_windows.rs` — reference for Windows-only module structure
- `crates/nono-cli/src/exec_strategy_windows/network.rs` — reference for WFP Windows-only code

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `SupervisorSocket::bind(path)` / `SupervisorSocket::connect(path)` — production-ready, already handles rendezvous file write/read, nonce-backed pipe names, server PID verification
- `handle_windows_supervisor_message()` — complete handler: replay detection, token check (needs adding), approval dispatch, `DuplicateHandle` brokering, audit log entry. Currently gated by `#[cfg(test)]` — just remove the gate and wire it up.
- `broker_file_handle_to_process()` — already implemented and tested in `socket_windows.rs`
- `sanitize_for_terminal()` — complete ANSI-stripping function in `terminal_approval.rs`; reuse on Windows

### Established Patterns
- Background threads for pipe servers: `start_control_pipe_server()`, `start_data_pipe_server()` in `supervisor.rs` — both use `std::thread::spawn` + `Arc<AtomicBool>` for termination signaling
- `getrandom::fill` for random bytes — already used in `socket_windows.rs`
- `#[cfg(target_os = "windows")]` module routing in `main.rs` for platform-specific code

### Integration Points
- `WindowsSupervisorDenyAllApprovalBackend` in `mod.rs`: remove and replace with `TerminalApproval` (adapted for Windows)
- `start_capability_pipe_server()`: new method on `WindowsSupervisorRuntime`, called from `initialize()` alongside `start_control_pipe_server()` and `start_data_pipe_server()`
- `ExecConfig` in `mod.rs`: may need to carry the rendezvous path so the launcher can inject `NONO_SUPERVISOR_PIPE` into the child's environment
- `handle_windows_supervisor_message()`: remove `#[cfg(test)]` gate; add session token validation before the approval backend call

</code_context>

<specifics>
## Specific Details

- Token env var name: `NONO_SESSION_TOKEN` (32-byte random, hex-encoded, 64-char string)
- Pipe env var name: `NONO_SUPERVISOR_PIPE` (path to the rendezvous file for `SupervisorSocket::connect()`)
- Token field name on `CapabilityRequest`: `session_token: String`
- Constant-time comparison: use `subtle` crate (`ConstantTimeEq`) or equivalent — do NOT use `==` on token strings
- `CONIN$` open path on Windows: `std::fs::File::open(r"\\.\CONIN$")`
- The existing `child_control: Option<SupervisorSocket>` field (anonymous pair) in `WindowsSupervisorRuntime` is the pre-existing infrastructure for in-process test usage; the new capability pipe server is separate

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 11-runtime-capability-expansion*
*Context gathered: 2026-04-11*
