---
phase: 18-extended-ipc
plan: 01
subsystem: ipc
tags: [windows, supervisor, ipc, aipc, event, mutex, protocol-skeleton, security, constant-time, subtle, repr-u8, serde]

# Dependency graph
requires:
  - phase: 11-runtime-capability-expansion
    provides: SupervisorSocket capability pipe, CapabilityRequest, ApprovalDecision, AuditEntry, ResourceGrant, broker_file_handle_to_process, audit_entry_with_redacted_token, constant-time session-token check, CONIN$ TerminalApproval, sanitize_for_terminal
  - phase: 17-attach-streaming
    provides: WindowsSupervisorRuntime.user_session_id (16-hex user-facing ID); Phase 17 latent-bug carry-forward pattern (kernel-object names use user_session_id, not self.session_id)
provides:
  - HandleKind / HandleTarget / SocketProtocol / SocketRole / PipeDirection wire-protocol enums (#[repr(u8)] discriminators 0..=5 pinned)
  - Extended CapabilityRequest with kind/target/access_mask (Phase 11 path field deprecated in place)
  - ResourceGrant constructors duplicated_windows_event_handle and duplicated_windows_mutex_handle
  - ResourceTransferKind::SocketProtocolInfoBlob (forward declaration; wired in 18-02)
  - crates/nono/src/supervisor/policy.rs (cross-platform per-handle-type access-mask validator + Win32 mask constants + DEFAULT_MASK consolidations)
  - broker_event_to_process and broker_mutex_to_process in socket_windows.rs
  - format_capability_prompt helper in terminal_approval.rs covering File + Event + Mutex (D-04 single-template)
  - Constant-time discriminator validation step (D-03) in handle_windows_supervisor_message
  - Server-side per-kind mask validation step (D-07) in dispatcher BEFORE backend dispatch
  - Live Event + Mutex broker dispatch via canonical Local\nono-aipc-<user_session_id>-<name> namespace
affects: [phase-18-02, phase-18-03, phase-18-04]

# Tech tracking
tech-stack:
  added: []  # No new crate deps; reused existing subtle, serde, windows-sys, std::os::windows::ffi
  patterns:
    - "constant-time discriminator validation via subtle::ConstantTimeEq on a 1-byte #[repr(u8)] enum byte"
    - "tagged-enum HandleTarget with #[serde(tag = \"type\")] for wire-format extensibility"
    - "in-place field deprecation (#[deprecated] on field) to keep wire backward compat without forcing caller rewrites"
    - "per-handle-type ResourceGrant constructors with sentinel AccessMode + meaningful access info in mask field"
    - "per-fn #[allow(clippy::not_unsafe_ptr_arg_deref)] for safe broker functions accepting raw HANDLE"
    - "server-side namespace canonicalization (Local\\nono-aipc-<user_session_id>-<name>) using user-facing 16-hex (Phase 17 carry-forward)"

key-files:
  created:
    - crates/nono/src/supervisor/policy.rs
    - .planning/phases/18-extended-ipc/18-01-SUMMARY.md
  modified:
    - crates/nono/src/supervisor/types.rs
    - crates/nono/src/supervisor/socket_windows.rs
    - crates/nono/src/supervisor/socket.rs
    - crates/nono/src/supervisor/mod.rs
    - crates/nono-cli/src/exec_strategy_windows/supervisor.rs
    - crates/nono-cli/src/terminal_approval.rs
    - crates/nono-cli/src/exec_strategy/supervisor_linux.rs

key-decisions:
  - "HandleKind discriminators 0..=5 are wire-format stability locks. File=0, Socket=1, Pipe=2, JobObject=3, Event=4, Mutex=5. Renumbering breaks every shipped SDK; pinned by handle_kind_discriminator_bytes_stable test."
  - "Phase 11 CapabilityRequest.path field deprecated in place (#[deprecated] attribute) rather than renamed/typed. Type stays PathBuf to avoid 8-12 file rewrites of Phase 11 callers; new code populates target with HandleTarget::FilePath instead. Actual removal deferred to a future phase."
  - "ResourceTransferKind::SocketProtocolInfoBlob and ResourceGrant.protocol_info_blob added now (not in Plan 18-02) so the wire enum is extended exactly once."
  - "Discriminator validation step uses subtle::ConstantTimeEq even though the discriminator carries no secret. Keeps the hot path structurally identical to the Phase 11 token-check; ~6ns/request cost; benefit is reviewer ergonomics."
  - "Server-side mask validation runs BEFORE backend dispatch so out-of-allowlist requests never reach the user's approval prompt (D-07 enforcement gate, Event/Mutex/JobObject paths)."
  - "Event/Mutex brokers pass dwOptions=0 (NOT DUPLICATE_SAME_ACCESS) to DuplicateHandle so the validated mask is the upper bound for the child's handle; supervisor source's full ALL_ACCESS does NOT propagate (T-18-01-11 mitigation)."
  - "Local\\nono-aipc-<user_session_id>-<sanitized_name> namespace prefix uses WindowsSupervisorRuntime.user_session_id (16-hex), NOT self.session_id (supervised-PID-NANOS). Phase 17 latent-bug carry-forward — three pre-existing bugs of exactly this shape were fixed in Phase 17 commit 7db6595."
  - "validate_aipc_object_name rejects path-separator chars (\\, /, :, NUL, control bytes) and enforces 1..=64 byte length. Mitigates T-18-01-03 (cross-session interference structurally impossible)."

patterns-established:
  - "Pattern 1: Constant-time discriminator gating via subtle::ConstantTimeEq on 1-byte #[repr(u8)]. Plans 18-02/18-03 inherit this pattern unchanged."
  - "Pattern 2: Per-handle-type policy module (crates/nono/src/supervisor/policy.rs) — single discoverable source of truth for Win32 mask constants + per-type defaults + the mask_is_allowed validator. Plan 18-03 layers profile widening on top."
  - "Pattern 3: Per-handle-type broker functions in socket_windows.rs mirroring broker_file_handle_to_process byte-for-byte except for explicit mask + per-kind ResourceGrant constructor. Plan 18-02 (Pipe + Socket) and 18-03 (Job Object) follow this template."
  - "Pattern 4: handle_<kind>_request driver helpers at module scope in supervisor.rs combine target-shape validation, mask validation, namespace canonicalization, kernel-object creation, brokering, and source-handle close per D-10."
  - "Pattern 5: format_capability_prompt total over all HandleKind variants — placeholder branches for Socket/Pipe/JobObject in 18-01 are replaced (not removed) by 18-02/18-03 so the helper never panics on an unrecognized kind."

requirements-completed: [AIPC-01]

# Metrics
duration: 60m
completed: 2026-04-19
---

# Phase 18 Plan 01: Extended IPC (AIPC-01) Wire-Protocol Skeleton + Event/Mutex Brokers Summary

**Wire-protocol skeleton (HandleKind / HandleTarget tagged enums + constant-time discriminator gate + cross-platform policy module) plus end-to-end Event and Mutex handle brokering via DuplicateHandle with validated access masks; Plans 18-02/18-03 build Socket/Pipe/JobObject on this foundation.**

## Performance

- **Duration:** ~60 min
- **Started:** 2026-04-19T20:00:00Z (approximately)
- **Completed:** 2026-04-19T21:00:00Z (approximately)
- **Tasks:** 5
- **Files modified:** 7 (1 new file, 6 modified)
- **Test count delta:** +22 unit tests (+6 types, +6 policy, +4 socket-broker, +5 terminal_approval, +7 capability_handler — alongside all pre-existing tests still passing)

## Accomplishments

- HandleKind / HandleTarget / SocketProtocol / SocketRole / PipeDirection wire-protocol enums with #[repr(u8)] discriminator pinning (File=0..Mutex=5).
- Extended CapabilityRequest backward-compatibly: new kind/target/access_mask fields default to HandleKind::File / None / 0 so Phase 11-shaped wire messages decode unchanged.
- New cross-platform policy module (crates/nono/src/supervisor/policy.rs) with the per-handle-type access-mask defaults locked by D-05, the mask_is_allowed subset validator (D-07), and the standard Win32 access-right constants — sourced directly from learn.microsoft.com.
- Two new broker functions (broker_event_to_process, broker_mutex_to_process) mirroring the file broker template byte-for-byte except for the explicit mask (NOT DUPLICATE_SAME_ACCESS) and per-kind ResourceGrant constructor.
- format_capability_prompt helper in terminal_approval.rs covering File + Event + Mutex prompt templates per D-04 (Socket/Pipe/JobObject branches return placeholders for Plans 18-02/18-03).
- handle_windows_supervisor_message dispatcher extended with: (1) constant-time discriminator validation (D-03) using subtle::ConstantTimeEq on the 1-byte discriminator_byte, (2) server-side per-kind mask validation (D-07) BEFORE backend dispatch, (3) match-arm dispatch to per-kind broker helpers (handle_event_request, handle_mutex_request).
- 14/14 capability_handler_tests pass (7 new + 7 Phase 11 carry-forward); 31/31 supervisor:: tests pass.
- Phase 11 invariants preserved byte-identical: bind_low_integrity SDDL, "Invalid session token" string, audit_entry_with_redacted_token primitive, replay-detect HashSet, CONIN$ branch, sanitize_for_terminal.

## Task Commits

Each task was committed atomically with DCO sign-off:

1. **Task 1: Add wire-protocol enums + extend CapabilityRequest** — `51b1c12` (feat)
2. **Task 2: Create policy module with per-handle-type validator** — `8ec09fd` (feat)
3. **Task 3: Add broker_event_to_process and broker_mutex_to_process** — `6e0e987` (feat)
4. **Task 4: Add format_capability_prompt helper for File + Event + Mutex** — `b440775` (feat)
5. **Task 5: Wire AIPC discriminator + Event/Mutex broker dispatch** — `c323372` (feat)

**Style fixup:** `3ea2017` (style — rustfmt drift on the 6 plan files)

## Files Created/Modified

- **NEW** `crates/nono/src/supervisor/policy.rs` (175 lines) — Per-handle-type access-mask allowlists for AIPC-01. Standard Win32 access-right constants + per-type defaults + mask_is_allowed subset validator. 6 unit tests; cross-platform compile.
- `crates/nono/src/supervisor/types.rs` — HandleKind/HandleTarget/SocketProtocol/SocketRole/PipeDirection enums; extended CapabilityRequest (path deprecated in place); GrantedResourceKind extended with Socket/Pipe/JobObject/Event/Mutex; ResourceTransferKind::SocketProtocolInfoBlob added; ResourceGrant.protocol_info_blob added; duplicated_windows_event_handle and duplicated_windows_mutex_handle constructors. 6 new unit tests.
- `crates/nono/src/supervisor/socket_windows.rs` — broker_event_to_process and broker_mutex_to_process functions; 4 new unit tests using BrokerTargetProcess::current(). Phase 11 broker_file_handle_to_process unchanged.
- `crates/nono/src/supervisor/socket.rs` — Test-mod CapabilityRequest construction updated to include the 3 new fields with File defaults (#[allow(deprecated)] on path).
- `crates/nono/src/supervisor/mod.rs` — `pub mod policy;` declaration; re-exports extended with HandleKind/HandleTarget/SocketProtocol/SocketRole/PipeDirection from types and broker_event_to_process/broker_mutex_to_process from socket (Windows-only).
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — Discriminator validation step (subtle::ConstantTimeEq on discriminator_byte); per-kind mask validation gate; match-arm dispatch (File preserved, Event/Mutex live, Socket/Pipe/JobObject placeholder Denied); resolved_mask_for_kind, validate_aipc_object_name, handle_event_request, handle_mutex_request helpers; user_session_id parameter added to handle_windows_supervisor_message + production call site updated. 7 new tests + 7 Phase 11 carry-forward = 14 total.
- `crates/nono-cli/src/terminal_approval.rs` — format_capability_prompt + format_event_access + format_mutex_access helpers (5 new unit tests). request_capability body unchanged (Phase 11 D-04 lock).
- `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` — CapabilityRequest construction updated to include the 3 new fields (Linux supervisor_linux dispatcher).

## Decisions Made

- See key-decisions block in frontmatter. Highlights:
  - Discriminator values pinned 0..=5 as wire-format stability lock.
  - Phase 11 path field deprecated IN PLACE (no rename, no Option wrap) to avoid Phase 11 caller churn.
  - SocketProtocolInfoBlob ResourceTransferKind variant + protocol_info_blob ResourceGrant field added now (not in 18-02) so the enum is extended exactly once.
  - resolved_mask_for_kind returns 0 for Socket/Pipe (Plan 18-02 fills these); the mask-validation gate denies any non-zero requested mask for those kinds until 18-02 wires them. Intentional and consistent with the placeholder Denied dispatch arm.
  - Drive-by: switched a runtime `assert!` on a constant-value MUTEX_DEFAULT_MASK bit-test to `const _: () = assert!(...)` so clippy's `assertions_on_constants` lint passes without losing the symmetry guard on the intentionally-included MUTEX_MODIFY_STATE bit (CONTEXT.md "do not strip" comment).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated 7 pre-existing CapabilityRequest construction sites to include the 3 new fields**
- **Found during:** Task 1 (immediately on `cargo build` after types.rs extension)
- **Issue:** Adding kind/target/access_mask fields to CapabilityRequest broke Rust struct construction at 7 callers (4 tests + 3 production: socket_windows test pair, socket_windows test pair (low-integrity), supervisor mod test, capability_handler_tests make_request, terminal_approval windows test, socket.rs Unix test, exec_strategy/supervisor_linux.rs Unix supervisor production code).
- **Fix:** Each caller updated with `kind: HandleKind::File, target: None, access_mask: 0` and `#[allow(deprecated)]` block on the construction site. Backward-compatibility intent preserved — these are Phase 11 callers that should keep using `path` until a future phase migrates them.
- **Files modified:** crates/nono/src/supervisor/socket_windows.rs, crates/nono/src/supervisor/mod.rs, crates/nono/src/supervisor/socket.rs, crates/nono-cli/src/terminal_approval.rs, crates/nono-cli/src/exec_strategy/supervisor_linux.rs, crates/nono-cli/src/exec_strategy_windows/supervisor.rs
- **Verification:** All pre-existing tests still pass; cargo build clean.
- **Committed in:** 51b1c12 (Task 1 commit).

**2. [Rule 1 - Bug] clippy::assertions_on_constants in policy.rs test**
- **Found during:** Task 3 (cargo clippy gate after broker functions added)
- **Issue:** `assert!(MUTEX_DEFAULT_MASK & MUTEX_MODIFY_STATE == MUTEX_MODIFY_STATE)` in mutex_modify_state_documented_as_reserved test failed clippy's `assertions_on_constants` lint because both operands are compile-time constants.
- **Fix:** Switched to `const _: () = assert!(...)` so the assertion runs at compile time AND satisfies the lint without losing the symmetry guard documented by the CONTEXT.md "do not strip" comment.
- **Files modified:** crates/nono/src/supervisor/policy.rs
- **Verification:** `cargo clippy -p nono --all-targets -- -D warnings -D clippy::unwrap_used` passes; test still runs (the runtime `assert_eq!(MUTEX_MODIFY_STATE, 0x0001)` line is the test body).
- **Committed in:** 6e0e987 (Task 3 commit, alongside the broker functions; bundled because the lint failure surfaced as part of the same clippy run).

**3. [Rule 1 - Bug] clippy::not_unsafe_ptr_arg_deref on broker_event/broker_mutex**
- **Found during:** Task 3 (cargo clippy gate after broker functions added)
- **Issue:** Both broker_event_to_process and broker_mutex_to_process accept `HANDLE` (a raw pointer alias) and aren't marked `unsafe`. Clippy's `not_unsafe_ptr_arg_deref` lint flagged this because the existing broker_file_handle_to_process takes a safe `&File`. The plan signature requires the HANDLE shape (CreateEventW/CreateMutexW return HANDLE, not File).
- **Fix:** Applied `#[allow(clippy::not_unsafe_ptr_arg_deref)]` at function level with a docstring paragraph explaining: the function is `safe` because the caller has already validated `handle` is a live event/mutex kernel-object handle owned by this process (the supervisor opened it via CreateEventW/CreateMutexW); DuplicateHandle itself is the only FFI the function performs and the unsafe contract for that call is documented inside.
- **Files modified:** crates/nono/src/supervisor/socket_windows.rs
- **Verification:** Clippy passes; the in-fn `// SAFETY:` block on the unsafe DuplicateHandle call documents the actual unsafe contract.
- **Committed in:** 6e0e987 (Task 3 commit).

**4. [Rule 1 - Bug] clippy::too_many_arguments on handle_windows_supervisor_message**
- **Found during:** Task 5 (cargo clippy gate after dispatcher extension)
- **Issue:** Adding the `user_session_id: &str` parameter brought the function to 8 args (>7 limit). Packing the dispatcher state into a struct would obscure the per-call ownership semantics of the borrowed `seen_request_ids` and `audit_log` mut refs vs the shared session token + user_session_id strings.
- **Fix:** Applied `#[allow(clippy::too_many_arguments)]` with a rationale comment naming the trade-off explicitly.
- **Files modified:** crates/nono-cli/src/exec_strategy_windows/supervisor.rs
- **Verification:** Clippy passes.
- **Committed in:** c323372 (Task 5 commit).

**5. [Rule 3 - Blocking] #[allow(dead_code)] on format_capability_prompt + format_event_access + format_mutex_access in terminal_approval.rs**
- **Found during:** Task 4 (clippy gate after helpers added)
- **Issue:** Task 4 lands the helpers but the dispatcher (Task 5) is the consumer. Between Task 4 and Task 5 commits, the helpers are dead code as far as production code paths are concerned (only tests consume them). Clippy's `dead_code` lint failed.
- **Fix:** Applied `#[allow(dead_code)]` with comments explicitly naming Task 5 as the consumer and a TODO-style "remove this allow after Task 5 lands" note.
- **Followup needed:** Task 5 actually wires `format!` strings inline in the dispatcher placeholder branches rather than routing through `format_capability_prompt`. The dead_code allows on the helpers are still active after Task 5; they will be removed when Plans 18-02/18-03 wire the per-kind prompts through the dispatcher's CONIN$ open path. The helpers stay tested (5 unit tests cover them) — this is the established Phase 11 pattern (helper + tests land separately from production wiring).
- **Files modified:** crates/nono-cli/src/terminal_approval.rs
- **Verification:** Clippy passes; tests still consume the helpers; comment explicitly names the future removal point.
- **Committed in:** b440775 (Task 4 commit).

---

**Total deviations:** 5 auto-fixed (1 blocking-on-build, 1 blocking-on-helper-wiring, 3 lint compliance)
**Impact on plan:** All deviations were lint/build compliance, not scope changes. No new files added beyond plan; no behavior diverged from CONTEXT.md decisions. The dead_code allows on terminal_approval helpers will be removed in a future plan when the helpers are wired into the live CONIN$ dispatch path.

## Phase 11 Invariant Verification

Verified via `git diff` + grep counts on the post-Task-5 working tree:

| Invariant | Baseline | Post-Plan | Status |
|-----------|----------|-----------|--------|
| `S:(ML;;NW;;;LW)` SDDL count in socket_windows.rs | 2 (const + doc) | 2 | byte-identical |
| `"Invalid session token"` count in supervisor.rs | 2 | 2 | byte-identical |
| `audit_entry_with_redacted_token` count in supervisor.rs | 4 | 7 | +3 (new Denied paths: discriminator + mask + brokering placeholder; all routed through redactor — no bare AuditEntry construction) |
| `audit_log.push(AuditEntry` (bare construction) count | 0 | 0 | byte-identical (every push goes through the redactor) |
| `CONIN$` count in terminal_approval.rs | 4 | 4 | byte-identical |
| `request_capability` body in terminal_approval.rs | unchanged | unchanged | byte-identical (Phase 11 multi-line prompt format preserved; format_capability_prompt is a separate helper) |

`git diff HEAD~6 HEAD -- crates/nono-cli/src/exec_strategy_windows/supervisor.rs | grep '^-.*Invalid session token\|^-.*audit_entry_with_redacted_token'`: 0 lines (no deletions of Phase 11 primitive lines).

## Phase 17 Latent-Bug Carry-Forward Verification

| Check | Required | Actual | Status |
|-------|----------|--------|--------|
| `format!.*nono-aipc.*self\.session_id` count in supervisor.rs | 0 | 0 | PASS — no new bug introduced |
| `format!.*nono-aipc.*user_session_id` count in supervisor.rs | >= 2 | 2 | PASS — Event + Mutex namespace prefixes use the user-facing 16-hex |

The two `format!.*nono-aipc.*user_session_id` sites are inside `handle_event_request` (line ~1257 in supervisor.rs) and `handle_mutex_request` (line ~1311); both construct the canonical `Local\\nono-aipc-{}-{}` namespace prefix using the `user_session_id: &str` parameter passed through the dispatcher signature.

## CI Gate Results

| Gate | Result |
|------|--------|
| `cargo build -p nono --lib` | PASS |
| `cargo build -p nono-cli --bin nono` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | PASS (clean) |
| `cargo fmt --all -- --check` (on plan files) | PASS (rustfmt drift fixed in commit 3ea2017) |
| `cargo test -p nono --lib supervisor::types` | PASS (6/6) |
| `cargo test -p nono --lib supervisor::policy` | PASS (6/6) |
| `cargo test -p nono --lib supervisor::socket` (Windows) | PASS (14/14 — 10 Phase 11 + 4 new event/mutex broker tests) |
| `cargo test -p nono --lib supervisor::` (full) | PASS (31/31) |
| `cargo test -p nono-cli --bin nono terminal_approval` | PASS (20/20 — 15 Phase 11 + 5 new) |
| `cargo test -p nono-cli --bin nono capability_handler_tests` | PASS (14/14 — 7 Phase 11 + 7 new) |

## Issues Encountered

None — the plan's `<interfaces>` block accurately captured every Phase 11 type/function/field shape needed; no codebase exploration was needed beyond what the plan loaded into context. The only friction was the build-cascade from extending CapabilityRequest (Rule 3 deviation #1 above) which was anticipated by the plan but the exact set of 7 callers required hands-on enumeration via `cargo build --workspace --all-targets`.

Pre-existing fmt drifts in `crates/nono-cli/src/exec_strategy_windows/launch.rs` and `crates/nono-cli/src/session_commands_windows.rs` are out of scope for this plan and were intentionally reverted before commit. Those drifts are documented for future cleanup (likely Phase 19 follow-up or Phase 20 maintenance).

## Deferred Issues

**1. format_capability_prompt helpers carry #[allow(dead_code)]**

The 3 helpers (format_capability_prompt, format_event_access, format_mutex_access) in terminal_approval.rs land in Task 4 with #[allow(dead_code)] because Task 5 wires the dispatcher with inline `format!` strings in the placeholder branches rather than routing through the helper. Plans 18-02 (Pipe + Socket) and 18-03 (Job Object) will wire the live CONIN$ prompt path through `format_capability_prompt` (replacing the placeholder branches with the D-04-locked templates) and remove the dead_code allows at that point. The helpers stay tested via 5 unit tests and the placeholder branches in `format_capability_prompt` are intentional placeholders that 18-02/18-03 replace.

**2. Pre-existing `tests/env_vars.rs` (19 failures) and `trust_keystore` flakes (1-3) carried forward unchanged**

Per STATE.md and Phase 19 CLEAN-02 deferred list, these are pre-existing Windows test-host flakes NOT in this plan's scope. Confirmed not regressed by this plan via the 65 targeted-test pass count above.

**3. Workspace fmt drift on `launch.rs` and `session_commands_windows.rs`**

Two pre-existing fmt drifts in files outside this plan's `files_modified` block. NOT fixed (scope boundary). These would need a separate quick-task or Phase 19 follow-up commit (single-file `style:` commit per file is the established pattern).

## Open Paths for Plans 18-02 and 18-03

Plan 18-01 deliberately leaves these tagged-but-unwired surfaces for the next two plans:

- **In `format_capability_prompt`** (terminal_approval.rs): Socket / Pipe / JobObject branches return placeholder `(unsupported in this build)` strings. 18-02 replaces Socket + Pipe with the D-04-locked templates; 18-03 replaces JobObject. The `#[allow(dead_code)]` on the helpers can be removed once the dispatcher's CONIN$ open path routes prompts through this helper (Plan 18-02 deliverable).
- **In dispatcher match-arm** (supervisor.rs): `HandleKind::Socket | HandleKind::Pipe | HandleKind::JobObject` arm returns structured Denied with reason "<kind> brokering not yet implemented in this build". 18-02 splits Socket and Pipe into their own arms; 18-03 wires JobObject. Each new arm calls a new `handle_<kind>_request` helper following the pattern established by `handle_event_request` / `handle_mutex_request`.
- **In `resolved_mask_for_kind`** (supervisor.rs): Socket and Pipe return 0 (denies all non-zero masks at the gate). 18-02 wires the role-based / direction-based resolution. JobObject returns the hard-coded JOB_OBJECT_QUERY default; 18-03 layers profile widening via a new `resolved_aipc_allowlist` field on WindowsSupervisorRuntime.
- **`HandleTarget::SocketEndpoint` validation**: PRIVILEGED_PORT_MAX (1023) constant is in policy.rs and ready for the role-based `port < 1024 → Denied` check that 18-02 will add to `handle_socket_request`.
- **`ResourceTransferKind::SocketProtocolInfoBlob` + `ResourceGrant.protocol_info_blob`**: Already added to the enum/struct so 18-02 doesn't need to extend the wire format twice. 18-02 wires `WSADuplicateSocketW` to populate `protocol_info_blob`.
- **`#[allow(deprecated)]` migration**: When Phase 11's `path` field is finally removed (future phase, not 18-02/18-03), every `#[allow(deprecated)]` block that this plan added at construction sites will become dead. They're a deliberate, audit-trail-friendly marker of where the migration must touch.

## Next Phase Readiness

- Wave 2 (Plans 18-02 Pipe + Socket and 18-03 Job Object + profile schema) can proceed against this foundation.
- All Phase 11 invariants byte-identical; no carryover risk.
- Phase 17 latent-bug pattern (user_session_id NOT self.session_id) explicitly tested and verified at the new Event/Mutex namespace sites.
- 14 capability_handler_tests pass on Windows host; the dispatcher is total over all 6 HandleKind values (no arm panics).
- Cross-platform compile holds: nono builds clean on Linux/macOS (the broker functions and Win32 imports are gated by the file-routing `#[path]` in supervisor/mod.rs).

## Self-Check: PASSED

All 9 files referenced in this summary exist on disk; all 6 commits referenced exist in git log.
- `crates/nono/src/supervisor/policy.rs` — FOUND
- `crates/nono/src/supervisor/types.rs` — FOUND
- `crates/nono/src/supervisor/socket_windows.rs` — FOUND
- `crates/nono/src/supervisor/socket.rs` — FOUND
- `crates/nono/src/supervisor/mod.rs` — FOUND
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — FOUND
- `crates/nono-cli/src/terminal_approval.rs` — FOUND
- `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` — FOUND
- `.planning/phases/18-extended-ipc/18-01-SUMMARY.md` — FOUND
- Commits 51b1c12, 8ec09fd, 6e0e987, b440775, c323372, 3ea2017 — all FOUND in `git log`

---
*Phase: 18-extended-ipc*
*Completed: 2026-04-19*
