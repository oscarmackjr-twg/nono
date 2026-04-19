---
phase: 18-extended-ipc
status: ready-for-planning
gathered: 2026-04-19
---

# Phase 18: Extended IPC (AIPC-01) — Context

**Gathered:** 2026-04-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Extend the Phase 11 capability pipe protocol to broker 5 new Windows handle types from supervisor to sandboxed child:

1. **Socket** — TCP/UDP socket the supervisor opened on the child's behalf (e.g. listener on supervisor-chosen port, or connect-side socket)
2. **Named pipe** — both ends of an anonymous pipe, OR a specific instance of a named pipe the supervisor created
3. **Job Object** — used by the child to assign subprocesses to a nested Job Object (rare orchestration use case)
4. **Event** — bidirectional lifecycle signaling primitives the supervisor created with `CreateEventW`
5. **Mutex** — cross-process mutex the supervisor owns; child can wait on it

Each request goes through the existing capability pipe, gets server-side validated against per-type access-mask allowlist (default-deny), prompts the user for approval through the existing CONIN$ TerminalApproval backend, and on approval transfers the handle into the child via `DuplicateHandle` (or `WSADuplicateSocket` for sockets).

**Out of scope (intentionally):**
- CLI pre-approval flags (`--allow-socket :8080` etc.) — locked SDK-only per D-04. Can land additively in v2.2 if user demand surfaces.
- Linux/macOS handle brokering. AIPC-01 is Windows-only — Unix has file-descriptor passing over Unix sockets as the natural equivalent (separate cross-platform requirement, explicitly out of v2.1 scope per REQUIREMENTS.md line 163).
- Sibling-to-sibling handle brokering (between two sandboxed siblings without supervisor mediation). Listed in REQUIREMENTS.md `Out of Scope (v2.1)`; existing supervisor-mediated flow covers known needs.
- Deprecation/removal of the existing `path: PathBuf` field on Phase 11's `CapabilityRequest`. D-01 deprecates the field IN PLACE for one release; actual removal is a future phase.

</domain>

<decisions>
## Implementation Decisions

### Wire Protocol

- **D-01:** **Tagged enum on `CapabilityRequest`.** Add `kind: HandleKind` (enum: `File | Socket | Pipe | JobObject | Event | Mutex`) + `target: HandleTarget` (enum carrying type-specific fields: `FilePath(PathBuf)`, `SocketEndpoint { protocol: SocketProtocol, host: String, port: u16, role: SocketRole }`, `PipeName(String)`, `JobObjectName(String)`, `EventName(String)`, `MutexName(String)`). The existing `path: PathBuf` field becomes `path: Option<PathBuf>` for one release with `#[deprecated(note = "use HandleTarget::FilePath via the new kind/target fields")]`; actual removal is a future phase. Backward wire compatibility with Phase 11 IS broken — acceptable because the wire is internal to nono (supervisor + child SDK ship together) and the SDK consumer surface (function signatures) stays stable via a thin compatibility shim.
- **D-02:** Single `SupervisorMessage::Request(CapabilityRequest)` envelope variant unchanged. All 5 new handle types route through one server dispatch site (the constant-time discriminator check fires once per request, not 5 times). One audit-entry shape, one policy-validator path.
- **D-03:** **Constant-time discriminator validation.** The first thing the supervisor does on receipt of a `Request(CapabilityRequest)` (after Phase 11's session-token check) is constant-time-compare `request.kind` against the known set — fail-closed on any unknown variant value. `subtle::ConstantTimeEq` (already a workspace dep). Any unknown discriminator returns `Denied { reason: "unknown handle type" }` immediately, BEFORE access-mask validation, BEFORE approval-backend dispatch.

### Per-Handle-Type Approval UX

- **D-04:** **Single template, per-type field labels.** One render function `format_capability_prompt(kind, target, access_mask, reason) -> String` emits a consistent shape:
  - File: `[nono] Grant file access? path=<x> access=<read|write|read+write> reason="<r>" [y/N]`
  - Socket: `[nono] Grant socket access? proto=<tcp|udp> host=<h> port=<p> role=<connect|bind|listen> reason="<r>" [y/N]`
  - Pipe: `[nono] Grant pipe access? name=<n> direction=<read|write|read+write> reason="<r>" [y/N]`
  - Job Object: `[nono] Grant Job Object access? name=<n> access=<query|set_quotas|terminate|...> reason="<r>" [y/N]`
  - Event: `[nono] Grant event access? name=<n> access=<wait|signal|both> reason="<r>" [y/N]`
  - Mutex: `[nono] Grant mutex access? name=<n> access=<wait|release|both> reason="<r>" [y/N]`
  Single sanitizer call site (`sanitize_for_terminal()` from Phase 11) handles ANSI stripping for all targets. Field shape identical across types: `[nono] Grant <kind> access? <type-specific-fields> access=<mask> reason="<r>" [y/N]`. CONIN$ branch from Phase 11 D-04 unchanged; Unix branch (no AIPC support, request returns Denied with reason "AIPC not supported on this platform" before reaching the prompt — see D-09).

### Access-Mask Allowlist Defaults

- **D-05:** **Hard-coded supervisor defaults + profile override.** The supervisor ships with conservative per-type defaults baked in:
  - **Socket:** sockets have NO per-handle access mask in Winsock — `WSADuplicateSocketW` carries the socket's full state via a `WSAPROTOCOL_INFOW` blob (single-use, target-PID-bound). The "access" enforcement is therefore *role-based at request time*: the supervisor validates `(protocol, role, port)` against the allowlist before opening/duplicating the socket. Default allowlist: TCP/UDP `connect` only; `bind`/`listen` require profile opt-in. User-allocated ports only (≥ 1024).
  - **Named pipe:** read OR write (not both); `DUPLICATE_SAME_ACCESS` mapped down to one direction.
  - **Job Object:** `JOB_OBJECT_QUERY` only. `JOB_OBJECT_SET_*` and `JOB_OBJECT_TERMINATE` require profile opt-in (these are highly privileged — TERMINATE on the supervisor's own Job Object would let the child kill the supervisor). **Additional runtime guard:** the supervisor MUST refuse to broker its own `containment_job` handle regardless of profile widening — this is a structural protection layered on top of the access-mask allowlist (the profile-author footgun is a real one; the runtime guard makes the worst case impossible).
  - **Event:** `SYNCHRONIZE | EVENT_MODIFY_STATE` (Wait + Signal). No `EVENT_ALL_ACCESS`.
  - **Mutex:** `SYNCHRONIZE | MUTEX_MODIFY_STATE` (Wait + Release). No `MUTEX_ALL_ACCESS`. Note: `MUTEX_MODIFY_STATE = 0x0001` is documented by Microsoft as "Reserved for future use" — `ReleaseMutex` works against handles opened with `SYNCHRONIZE` alone today. Keep the bit in the default mask for forward-compat symmetry with EVENT_MODIFY_STATE; document the no-op-today reality in the policy module.
  Defaults live in a single `policy::aipc::default_allowlist()` constant function in `crates/nono/src/supervisor/policy.rs` (new file or new module) so they're reviewable in one place.
- **D-06:** **Profile override schema.** Profiles are JSON (loader is `serde_json::from_str`; built-in profiles live in `crates/nono-cli/data/policy.json`). Profiles can widen via a `capabilities.aipc` JSON object on each profile entry (and the JSON-schema at `crates/nono-cli/data/nono-profile.schema.json` extended in lockstep):
  ```json
  "capabilities": {
    "aipc": {
      "socket": ["connect", "bind", "listen"],
      "pipe": ["read", "write", "read+write"],
      "job_object": ["query"],
      "event": ["wait", "signal", "both"],
      "mutex": ["wait", "release", "both"]
    }
  }
  ```
  Built-in profiles (claude-code, codex, opencode, openclaw, swival) get `capabilities.aipc` blocks tuned to their actual needs. Default-deny still applies for any handle type whose access mask is not in either the hard-coded default OR the profile override. Profile override is ADDITIVE — it widens, never narrows the default-deny. (Narrowing the default would require a different mechanism; out of scope for v2.1.)
- **D-07:** **Server-side enforcement is load-bearing.** Client-declared access masks are untrusted (the child SDK stamps a mask onto its request, but the supervisor re-validates server-side against the resolved per-handle-type allowlist). REQUIREMENTS.md line 157 mandate. The supervisor returns `Denied { reason: "access mask <m> not in allowlist for <kind>" }` for any request where the requested mask is not a subset of the resolved allowlist. The reason string is verbose-on-purpose for debuggability; not a security-sensitive surface (the allowlist is publicly known via profile.toml).

### CLI / SDK Surface

- **D-08:** **SDK-only — extends Phase 11 pattern.** Child SDK (in `crates/nono/src/supervisor/`) gains 5 new request methods:
  - `request_socket(host, port, protocol, role, access, reason) -> Result<RawSocket>`
  - `request_pipe(name, direction, reason) -> Result<RawHandle>`
  - `request_job_object(name, access, reason) -> Result<RawHandle>`
  - `request_event(name, access, reason) -> Result<RawHandle>`
  - `request_mutex(name, access, reason) -> Result<RawHandle>`
  Each method constructs a `CapabilityRequest` with the appropriate `kind: HandleKind` + `target: HandleTarget` + `access_mask: u32`, sends it over the existing `\\.\pipe\nono-cap-<session_id>` capability pipe (Phase 11 infrastructure), and returns the raw handle once the supervisor brokers it via `DuplicateHandle` (or `WSADuplicateSocket` for sockets).
  Zero new `--request-*` or `--allow-*` CLI flags. Zero changes to `nono run` / `nono shell` / `nono wrap` argument shape. Matches Phase 11's locked-by-design pattern.
- **D-09:** **Cross-platform behavior — fail at request time, not parse time.** Per REQUIREMENTS.md line 163: "Unix builds either reject `--request-handle` at parse time or degrade gracefully." Since D-08 is SDK-only (no CLI flags), there's no parse-time rejection surface. Instead: the SDK methods are gated `#[cfg(target_os = "windows")]` for the actual brokering path; on non-Windows builds they exist but immediately return `NonoError::UnsupportedPlatform("AIPC handle brokering is Windows-only on v2.1; Unix has SCM_RIGHTS file-descriptor passing as the natural equivalent for sockets/pipes (separate cross-platform requirement, future milestone). Events, mutexes, and Job Objects have no direct Unix analog.")`. (The variant is `UnsupportedPlatform`, not `PlatformNotSupported` — confirmed by reading `crates/nono/src/error.rs:39-40`.) This lets cross-platform Rust code compile against the SDK without `#[cfg]` everywhere; the runtime fail-closed message tells operators why their call failed.

### Handle Lifetime & Audit

- **D-10:** **Child owns the duplicated handle; supervisor closes its source on grant.** Once `DuplicateHandle`/`WSADuplicateSocket` succeeds and the duplicated handle value is sent back to the child in `SupervisorResponse::Decision { grant: ResourceGrant { raw_handle: Some(h) } }`, the supervisor immediately calls `CloseHandle` on its own source handle (it was opened by the supervisor for the sole purpose of brokering). The child is responsible for closing the duplicated handle when done. On error path (duplicate fails after open), supervisor closes its source before returning `Denied`. No reference counting; no shared ownership.
- **D-11:** **Audit log entry per handle type carries kind + target + access_mask + decision + reason.** Phase 11's `AuditEntry { timestamp, request, decision, backend, duration_ms }` shape is reused unchanged — the request now carries `kind` and `target`, so `AuditEntry.request` already discriminates. The session token is redacted as today (Phase 11 D-01 lock); audit entries omit it entirely. Token-leak audit (Phase 11's `session_token_redaction` test) is extended in this phase to cover all 6 `HandleKind` request shapes (not just File).

### Claude's Discretion

- File layout for the new policy module: `crates/nono/src/supervisor/policy.rs` (new file) vs nesting under `crates/nono/src/supervisor/aipc.rs`. Choose what reads cleanest.
- Internal helper structure for the 5 brokers: 5 separate `broker_*_to_process()` functions vs one dispatcher with per-type `match` arms. Choose what minimizes unsafe-block surface and keeps `// SAFETY:` comments short and verifiable.
- Whether `request_pipe` returns the read-end or write-end is a function of the `direction` arg; SDK API can use enums or two methods. Choose for clarity.
- Whether `SocketProtocol` is `enum { Tcp, Udp }` or a string. Enum recommended.
- Test scaffolding: per-handle-type unit tests + integration tests must each cover both the granted path and the denied (mask-violation) path. Reuse Phase 11's `session_token_redaction` test pattern for the extended token-leak audit.
- Audit `duration_ms` granularity unchanged from Phase 11 (millisecond resolution from `Instant::elapsed`).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & roadmap
- `.planning/REQUIREMENTS.md` § AIPC-01 (lines 127-167) — full AIPC-01 spec including the 5 handle types, server-side validation mandate, fail-closed semantics, token-leak audit requirement, and cross-platform note (Unix out of v2.1 scope).
- `.planning/ROADMAP.md` § "Phase 18: Extended IPC (AIPC)" (lines 71-75) — phase scope statement and the "3 plans (likely)" decomposition guidance.
- `.planning/PROJECT.md` § Active (v2.1) → AIPC-01 — milestone-level statement.

### Phase 11 prior art (load-bearing — DO NOT regress)
- `.planning/phases/11-runtime-capability-expansion/11-CONTEXT.md` — Phase 11 D-01..D-05 (session token, named pipe, background thread server, CONIN$ TerminalApproval, ResourceGrant extensibility). AIPC-01 inherits all 5 directly.
- `.planning/phases/11-runtime-capability-expansion/11-01-SUMMARY.md` and `11-02-SUMMARY.md` — what Phase 11 actually shipped on disk.
- `crates/nono/src/supervisor/types.rs` — `CapabilityRequest` (lines 19-39), `ApprovalDecision`, `GrantedResourceKind` (line 57 — single `File` variant; needs 5 new variants), `ResourceTransferKind`, `ResourceGrant`, `AuditEntry`, `SupervisorMessage` (lines 165-180), `SupervisorResponse`.
- `crates/nono/src/supervisor/socket_windows.rs` — `SupervisorSocket` (bind/connect/pair), `BrokerTargetProcess`, `broker_file_handle_to_process()` (lines 269-302; the structural analog for the 5 new brokers).
- `crates/nono/src/supervisor/mod.rs` — `ApprovalBackend` trait.
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` § `handle_windows_supervisor_message` — Phase 11 server-side dispatch site; AIPC-01 extends here.
- `crates/nono-cli/src/terminal_approval.rs` — `TerminalApproval`, `sanitize_for_terminal()`, the prompt template Phase 11 D-04 locked.

### Phase 17 invariance precedent
- `.planning/phases/17-attach-streaming/17-CONTEXT.md` § D-21 — Windows-invariance pattern: server-side brokering code in `crates/nono/src/supervisor/socket_windows.rs` (Windows-only file in cross-platform crate, established pattern); CLI plumbing in `crates/nono-cli/src/exec_strategy_windows/`. AIPC-01 follows the same split.

### Project standards
- `CLAUDE.md` — coding standards (no `.unwrap()` outside `#[cfg(test)]`, `NonoError` + `?`, `// SAFETY:` on every unsafe block, EnvVarGuard + lock_env for env-mutating tests, DCO sign-off on every commit, `subtle` crate for constant-time comparisons).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`SupervisorSocket::bind` / `connect` / `recv_message`** (`socket_windows.rs`) — production-ready capability pipe; AIPC-01 reuses unchanged. Phase 11's `\\.\pipe\nono-cap-<session_id>` rendezvous is the transport.
- **`broker_file_handle_to_process`** (`socket_windows.rs:269-302`) — the prototype for the 5 new brokers. Wraps `DuplicateHandle(GetCurrentProcess(), source, target_process, &mut dup, 0, 0, DUPLICATE_SAME_ACCESS)` with proper `// SAFETY:` doc. Pattern to mirror per-type with the appropriate access-mask passed (instead of `DUPLICATE_SAME_ACCESS`).
- **`BrokerTargetProcess`** — opaque wrapper around the child process handle for `DuplicateHandle`'s target argument. Reused unchanged.
- **`handle_windows_supervisor_message`** (`exec_strategy_windows/supervisor.rs`) — Phase 11 server-side dispatch. Today: replay-detect → token check → backend approval → `broker_file_handle_to_process` → emit audit entry. Phase 18 inserts a discriminator-validation step after the token check (D-03), then dispatches to the per-type broker (D-10).
- **`TerminalApproval` + `sanitize_for_terminal`** (`terminal_approval.rs`) — Phase 11's CONIN$ prompt machinery. Phase 18 adds new prompt templates per handle type (D-04) but reuses the sanitizer + CONIN$ open + y/N parser unchanged.
- **`session_token_redaction` test pattern** (Phase 11) — the test family to extend with new request shapes per D-11.

### Established Patterns

- **Constant-time comparisons via `subtle::ConstantTimeEq`** — Phase 11 D-01 uses this for the session token. AIPC-01 D-03 reuses for the discriminator check. Same crate, same pattern.
- **Background-thread pipe servers with `Arc<AtomicBool>` shutdown signaling** — `start_control_pipe_server`, `start_data_pipe_server`, Phase 11's `start_capability_pipe_server`. AIPC-01 doesn't add a new pipe server (reuses Phase 11's); pattern is documentational.
- **`#[cfg(target_os = "windows")]` module routing** — Phase 18's per-type brokers live in `crates/nono/src/supervisor/socket_windows.rs` (a `#[cfg(windows)]` module per Phase 17 D-21 precedent). The SDK request methods are split: Windows path does the real work, non-Windows path returns `NonoError::PlatformNotSupported` per D-09.
- **Profile TOML schema** — `claude-code.toml`, `codex.toml`, etc. already have nested `[capabilities.*]` blocks; AIPC-01 D-06 adds a new sub-block consistent with the existing convention.
- **Audit-entry emission via `mpsc::channel`** — Phase 11 D-03 pattern. AIPC-01 reuses unchanged; `AuditEntry` shape itself doesn't change (request now carries kind/target).

### Integration Points

- **No new pipe / socket / port** — AIPC-01 is purely additive on top of Phase 11's `\\.\pipe\nono-cap-<session_id>` capability pipe. No new firewall/SDDL surface.
- **Profile loader** — must learn to parse the `[capabilities.aipc]` block. Affects `crates/nono-cli/src/profile/mod.rs` (or wherever profile parsing lives). Built-in profiles get tuned blocks per D-06; pure-default profiles silently inherit hard-coded defaults.
- **No new CLI flags** — D-08 SDK-only locks this. Argument parsers untouched.
- **No new `nono setup` step** — capability pipe is already established by Phase 11 setup.
- **Cross-platform**: SDK methods compile on all platforms (D-09). Brokers are `#[cfg(windows)]`. Unix call site returns `NonoError::PlatformNotSupported` at runtime, not parse time.

</code_context>

<specifics>
## Specific Ideas

- **`HandleKind` discriminator values** should be small integers (`#[repr(u8)]`) to keep the constant-time comparison cheap; not strings. `subtle::ConstantTimeEq` works on byte slices.
- **`HandleTarget` enum** uses Rust enum-with-data idiom; serde-friendly via `#[serde(tag = "type")]` for wire stability.
- **Socket port bounds checking** is a server-side fail-closed step before the access-mask check: `port < 1024 → Denied { reason: "privileged port not allowed" }` unless profile explicitly widens to `bind_privileged: true` (which is NOT in the v2.1 default schema; would need a future profile-schema extension).
- **Pipe name sanitization**: `\\.\pipe\` prefix must be enforced server-side (not client-supplied as part of the name) to prevent the child from requesting brokerage of arbitrary named pipes (e.g. another nono session's control pipe). Server canonicalizes `target = HandleTarget::PipeName(name)` to `\\.\pipe\nono-aipc-<session_id>-<name>` so cross-session interference is structurally impossible.
- **Job Object name namespace** similar to pipe: prefix `Local\nono-aipc-<session_id>-<name>` to scope per-session.
- **Event/Mutex name namespace** similar: `Local\nono-aipc-<session_id>-<name>`.
- **Reason field cap**: 256 bytes (UTF-8). Same as Phase 11's `reason` field. Longer reasons get truncated with a `...[truncated]` suffix in the audit log.
- **Phase 17 latent-bug carry-forward**: AIPC-01 namespace prefixes (`\\.\pipe\nono-aipc-<session_id>-<name>`, `Local\nono-aipc-<session_id>-<name>`) MUST use `WindowsSupervisorRuntime.user_session_id` (the user-facing 16-hex), NOT `self.session_id` (the supervisor correlation `supervised-PID-NANOS`). Three pre-existing bugs of exactly this shape were fixed in Phase 17 commit `7db6595` (`start_logging`, `start_data_pipe_server`, `create_process_containment` job-name). Plan acceptance criteria must include grep-checks asserting `user_session_id` (not `self.session_id`) appears at every new pipe/Local-namespace name construction site.

</specifics>

<deferred>
## Deferred Ideas

- **CLI pre-approval flags** (`--allow-socket :8080`, `--allow-pipe my-pipe`, etc.) — captured at decision time as the "hybrid" rejected option for D-08. Useful for unattended/CI runs where no user is present to answer the CONIN$ prompt. Land additively in v2.2 if user demand surfaces.
- **Sibling-to-sibling handle brokering** between two sandboxed children without supervisor mediation — REQUIREMENTS.md `Out of Scope (v2.1)` line 283. Existing supervisor-mediated flow covers known needs.
- **Linux/macOS handle brokering via SCM_RIGHTS** — listed in REQUIREMENTS.md as a separate cross-platform requirement; explicitly out of v2.1 scope. Future milestone.
- **Profile narrowing** (profile says "deny socket entirely even though hard-coded default allows connect") — D-06 widening-only is the v2.1 lock. Narrowing requires a different mechanism (e.g. `[capabilities.aipc.deny]` block); future phase.
- **Bind-to-privileged-port** (port < 1024) — explicitly denied in v2.1. Profile schema extension to opt in lives in a future phase.
- **`JOB_OBJECT_TERMINATE` and `JOB_OBJECT_SET_*` access masks** — explicitly excluded from default per D-05 because they let the child terminate/reconfigure the supervisor's own Job Object. Profile opt-in possible per D-06 widening.
- **Removal of the deprecated `path: PathBuf` field** on `CapabilityRequest` — D-01 deprecates in place; actual removal is a future phase after one release ships with deprecation warning.
- **`ApprovalBackend::request_capability` extension to take HandleKind directly** instead of full `CapabilityRequest` — Phase 11's trait shape is preserved unchanged for backward compatibility with custom approval backends. Future refactor.

</deferred>

---

*Phase: 18-extended-ipc*
*Context gathered: 2026-04-19*
