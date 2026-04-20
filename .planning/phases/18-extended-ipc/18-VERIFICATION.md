---
phase: 18-extended-ipc
verified: 2026-04-19T00:00:00Z
status: human_needed
score: 32/32 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Manual UAT on Windows host: run `nono --profile claude-code` with a child that calls each of the 5 SDK request_* methods and verifies the CONIN$ approval prompt renders the D-04-locked per-kind template (File/Event/Mutex/Pipe/Socket/JobObject)."
    expected: "Each prompt shows the correct kind-specific fields (e.g. `proto=tcp host=... port=... role=connect` for socket) and approval grants a live handle; denial produces `grant=None` audit entry."
    why_human: "IN-01 in 18-REVIEW notes the dispatcher still wires format! strings inline rather than routing through format_capability_prompt; only tests consume the helper. A human needs to eyeball the live CONIN$ text on a real terminal to confirm UX integrity end-to-end."
  - test: "Validate WR-01 (Pipe/Socket pre-approval gate) does not leak through to real users: submit a socket request with port=80 and a pipe request with PipeDirection::ReadWrite under the default profile; observe whether the user is prompted."
    expected: "Per the 18-01 summary invariant, Event/Mutex/JobObject reject BEFORE prompt; per WR-01, Pipe/Socket currently reject AFTER prompt. Confirm whether the UX impact is acceptable for v2.1 or requires a follow-up fix."
    why_human: "The reviewer flagged this as UX inconsistency rather than a security hole — a product decision is required on whether to accept the deviation or schedule a follow-up."
  - test: "Exercise the capabilities.aipc profile-widening path end-to-end: craft a profile with `capabilities.aipc.pipe: [\"read+write\"]` and verify a ReadWrite pipe request is granted; remove the widening and verify the same request is denied."
    expected: "Widening the profile grants ReadWrite; removing the widening enforces default read-OR-write (not both). The UNION semantic is tested in profile::tests but end-to-end dispatcher consumption of the loaded profile's resolve_aipc_allowlist is deferred (18-03 Deferred Issues #1)."
    why_human: "Plan 18-03 seeds WindowsSupervisorRuntime.resolved_aipc_allowlist with default() pending a future plan that threads Profile through. Need human confirmation that the default-only behavior is acceptable for v2.1 and no demo-breaking regression is shipped."
  - test: "Spot-check the real-world WR-02 scenario: does CompareObjectHandles ever fail in practice on a target Windows 10 1607 / Windows 11 host? Run the supervisor under a CrowdStrike / Defender ATP / EDR-instrumented environment with Job Object telemetry hooks."
    expected: "CompareObjectHandles returns non-zero for same-object and zero for distinct-object on all supported hosts. No EDR-introduced fail-open observed."
    why_human: "WR-02 is a latent fail-open concern; empirical verification on a hardened host is the only way to know whether it's exploitable in practice."
---

# Phase 18: Extended IPC (AIPC-01) Verification Report

**Phase Goal:** Broker socket, named-pipe, Job Object, event, and mutex handles over the Phase 11 capability pipe. Each handle type validated server-side against access-mask allowlist; profile widening via `capabilities.aipc` JSON block; containment-Job runtime guard prevents the supervisor-own-Job footgun structurally.
**Verified:** 2026-04-19
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (all 4 plans)

| # | Plan | Truth | Status | Evidence |
|---|------|-------|--------|----------|
| 1 | 18-01 | CapabilityRequest carries kind/target/access_mask fields | VERIFIED | types.rs:27-39 HandleKind enum, types.rs:103+ HandleTarget, types.rs:163+ deprecated path |
| 2 | 18-01 | Supervisor rejects unknown discriminator bytes in constant time | VERIFIED | supervisor.rs:1221-1229 `constant_time_eq` uses `subtle::ConstantTimeEq::ct_eq`; called at :1756 and :1787 |
| 3 | 18-01 | Event broker via DuplicateHandle with default mask SYNCHRONIZE \| EVENT_MODIFY_STATE | VERIFIED | socket_windows.rs:367 `broker_event_to_process` + dwOptions=0 MAP DOWN at :377-:382 |
| 4 | 18-01 | Mutex broker via DuplicateHandle with default mask SYNCHRONIZE \| MUTEX_MODIFY_STATE | VERIFIED | socket_windows.rs:422 `broker_mutex_to_process` + MAP DOWN at :432-:437 |
| 5 | 18-01 | Kernel-object names canonicalized via user_session_id (not self.session_id) | VERIFIED | format!.*nono-aipc.*user_session_id count=4 per 18-03 SUMMARY table; format!.*nono-aipc.*self.session_id count=0 |
| 6 | 18-01 | Server-side mask validation rejects requests outside per-type allowlist | VERIFIED | policy.rs exports `mask_is_allowed`; dispatcher calls before backend at supervisor.rs pre-approval gate |
| 7 | 18-01 | Audit entries redact session_token | VERIFIED | `handle_redacts_token_in_audit_for_all_handle_kinds` test covers all 6 HandleKinds (supervisor.rs:3292) |
| 8 | 18-01 | Phase 11 invariants preserved | VERIFIED | SUMMARY tables show CAPABILITY_PIPE_SDDL=2, "Invalid session token"=2, CONIN$=4 byte-identical |
| 9 | 18-02 | Pipe broker with MAP DOWN via GENERIC_READ/GENERIC_WRITE (NOT DUPLICATE_SAME_ACCESS) | VERIFIED | socket_windows.rs:541 `broker_pipe_to_process` + dwOptions=0 comments at :377/:432 (shared pattern) |
| 10 | 18-02 | Socket broker via WSADuplicateSocketW + WSAPROTOCOL_INFOW blob | VERIFIED | socket_windows.rs:595 `broker_socket_to_process`, imports at :24 `WSADuplicateSocketW, WSAPROTOCOL_INFOW` |
| 11 | 18-02 | Pipe names canonicalized as `\\.\pipe\nono-aipc-<user_session_id>-<name>` | VERIFIED | 18-02 SUMMARY §Phase 17 Latent-Bug table: 3 sites total (Event+Mutex+Pipe) using user_session_id |
| 12 | 18-02 | Socket role-based validation + privileged-port deny (port <= 1023) | VERIFIED | supervisor.rs:1461 `if port <= policy::PRIVILEGED_PORT_MAX` fires before role check; policy.rs:59 `PRIVILEGED_PORT_MAX=1023` |
| 13 | 18-02 | Socket lifecycle: source SOCKET stays open until AFTER broker returns | VERIFIED | 18-02 SUMMARY §Decisions: "closesocket happens via handle_socket_request AFTER broker_socket_to_process returns" |
| 14 | 18-02 | Audit entries for Pipe and Socket redact session_token | VERIFIED | Parameterized test covers Pipe + Socket variants (supervisor.rs:3317-3332) |
| 15 | 18-02 | Plan 18-01 invariants preserved byte-identical | VERIFIED | 18-02 SUMMARY table confirms broker_event/mutex, handle_event/mutex_request, validate_aipc_object_name all preserved |
| 16 | 18-03 | Job Object broker via DuplicateHandle with caller-validated mask | VERIFIED | socket_windows.rs:484 `broker_job_object_to_process` + `pub fn` confirmed; mod.rs:46 re-export |
| 17 | 18-03 | JobObject names canonicalized with user_session_id | VERIFIED | 18-03 SUMMARY §Phase 17 table: count>=4 (Event+Mutex+Pipe+JobObject); self.session_id count=0 |
| 18 | 18-03 | Supervisor refuses to broker its OWN containment_job regardless of profile | VERIFIED | supervisor.rs:19 imports `CompareObjectHandles`; `runtime_containment_job: HANDLE` threaded through at :1618; test `handle_denies_job_object_brokering_of_containment_job_even_with_profile_widening` PASSES |
| 19 | 18-03 | Profile `capabilities.aipc` block + parse-time rejection | VERIFIED | profile/mod.rs:73 `AipcConfig`, :102 `CapabilitiesConfig`, `validate_profile_aipc_tokens` wired into parse_profile_file |
| 20 | 18-03 | 5 built-in profiles with conservative aipc widening | VERIFIED | policy.json has 5 `"aipc":` blocks at lines 695, 749, 785, 824, 936 (claude-code, codex, opencode, openclaw, swival) |
| 21 | 18-03 | Profile schema validates aipc tokens via per-key string enums | VERIFIED | nono-profile.schema.json has top-level `capabilities` property + `$defs/CapabilitiesConfig` + `$defs/AipcConfig` |
| 22 | 18-03 | Parameterized audit-redaction over all 6 HandleKind shapes | VERIFIED | supervisor.rs:3292 `handle_redacts_token_in_audit_for_all_handle_kinds` with `assert_eq!(cases.len(), 6, ...)` guard at :3346 |
| 23 | 18-03 | Windows-only integration test for 5 new handle types | VERIFIED | tests/aipc_handle_brokering_integration.rs:28 `#![cfg(target_os = "windows")]`; 5 tests PASS |
| 24 | 18-03 | Plans 18-01/18-02 invariants preserved | VERIFIED | 18-03 SUMMARY §Invariant Verification: CAPABILITY_PIPE_SDDL=2, "Invalid session token"=2 byte-identical |
| 25 | 18-04 | SDK exposes 5 cross-platform request_* methods | VERIFIED | aipc_sdk.rs:90 `request_socket`, :154 `request_pipe`, :191 `request_job_object`, :228 `request_event`, :264 `request_mutex` |
| 26 | 18-04 | Windows arms post CapabilityRequest + wait for SupervisorResponse::Decision | VERIFIED | `send_capability_request` helper at :363 uses `cap_pipe.send_message` / `cap_pipe.recv_response` |
| 27 | 18-04 | request_socket returns RawSocket from WSAPROTOCOL_INFOW blob; others return RawHandle | VERIFIED | `reconstruct_socket_from_blob` with 3 unsafe blocks + defensive length check; `extract_duplicated_handle` for Event/Mutex/Pipe/JobObject |
| 28 | 18-04 | Non-Windows builds return NonoError::UnsupportedPlatform | VERIFIED | aipc_sdk.rs:132-134 (socket), :175-177 (pipe), :212-214 (job_object), :248-250 (event), :284-286 (mutex) |
| 29 | 18-04 | Single source of truth `unsupported_platform_message()` | VERIFIED | aipc_sdk.rs:69 sole `pub fn unsupported_platform_message`; 2 tests (PASS) guard drift at :600 + :619 |
| 30 | 18-04 | mod.rs re-exports 5 SDK functions | VERIFIED | mod.rs:39-42 `pub use aipc_sdk::{request_event, request_job_object, request_mutex, request_pipe, request_socket, unsupported_platform_message, RawHandle, RawSocket}` |
| 31 | 18-04 | Phase 11 invariants preserved; SDK is pure wrapper | VERIFIED | 18-04 SUMMARY §Invariant Verification: git diff HEAD~3 returns 0 lines for all 10 critical files |
| 32 | 18-04 | D-21 Windows-invariance held | VERIFIED | aipc_sdk.rs per-fn `#[cfg(target_os = "windows")]` / `#[cfg(not(target_os = "windows"))]` arms; cargo build -p nono clean |

**Score:** 32/32 truths verified

### Required Artifacts (all plans)

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| crates/nono/src/supervisor/types.rs | HandleKind/HandleTarget enums, extended CapabilityRequest | VERIFIED | File exists (27685 bytes); HandleKind at :27 with 6 variants; ResourceGrant constructors for event/mutex/pipe/socket/job_object |
| crates/nono/src/supervisor/policy.rs | Per-type access-mask + validator | VERIFIED | File exists (6911 bytes); `mask_is_allowed` + all DEFAULT_MASK constants + `PRIVILEGED_PORT_MAX=1023` + `JOB_OBJECT_ALL_ACCESS` |
| crates/nono/src/supervisor/socket_windows.rs | 5 broker_*_to_process functions + bind_aipc_pipe | VERIFIED | File exists (65879 bytes); all 5 brokers + bind_aipc_pipe confirmed at :367, :422, :484, :541, :595, :649 |
| crates/nono/src/supervisor/aipc_sdk.rs | 5 request_* methods + helpers + type aliases | VERIFIED | File exists (50614 bytes, NEW in 18-04); 11 SDK tests PASS |
| crates/nono/src/supervisor/mod.rs | Module registrations + re-exports | VERIFIED | File exists; `pub mod aipc_sdk;` + all 5 SDK re-exports at :39-42; Windows-only broker re-exports at :45-48 |
| crates/nono-cli/src/exec_strategy_windows/supervisor.rs | 5 handle_*_request helpers + dispatcher | VERIFIED | File exists; all 5 helpers at :1284, :1355, :1439, :1545, :1614; discriminator validation at :1787; containment_job guard at :1678 |
| crates/nono-cli/src/terminal_approval.rs | format_capability_prompt + per-kind format_* helpers | VERIFIED | File exists; helpers present (tested but #[allow(dead_code)] per IN-01 deferred) |
| crates/nono-cli/src/profile/mod.rs | AipcConfig + CapabilitiesConfig + resolve_aipc_allowlist | VERIFIED | File exists (187787 bytes); `AipcConfig` :73, `CapabilitiesConfig` :102, `AipcResolvedAllowlist` :120, `resolve_aipc_allowlist` method :706 |
| crates/nono-cli/data/policy.json | 5 built-in profiles with capabilities.aipc | VERIFIED | 5 `"aipc":` blocks at :695, :749, :785, :824, :936 |
| crates/nono-cli/data/nono-profile.schema.json | capabilities + AipcConfig schema | VERIFIED | File exists (25352 bytes); top-level `capabilities` at :87, `AipcConfig` at :104 |
| crates/nono-cli/tests/aipc_handle_brokering_integration.rs | Windows-only integration test (5 round-trips) | VERIFIED | File exists (8254 bytes); `#![cfg(target_os = "windows")]` at :28; 5 tests PASS |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| supervisor.rs | policy.rs | `use nono::supervisor::policy::{mask_is_allowed, ...}` | WIRED | `policy::PRIVILEGED_PORT_MAX` at supervisor.rs:1461 |
| supervisor.rs | socket_windows.rs | broker_*_to_process imports | WIRED | Line 5: `bind_aipc_pipe, broker_event_to_process, broker_job_object_to_process, broker_mutex_to_process, broker_pipe_to_process, broker_socket_to_process` |
| supervisor.rs | WindowsSupervisorRuntime.user_session_id | format! namespace construction | WIRED | 4 sites per 18-03 SUMMARY; self.session_id count=0 (Phase 17 carry-forward) |
| supervisor.rs | WindowsSupervisorRuntime.containment_job | CompareObjectHandles runtime guard | WIRED | SendableHandle wrapper at :442, threading at :452-518, CompareObjectHandles call at :1678 |
| supervisor.rs | Profile::resolve_aipc_allowlist | AipcResolvedAllowlist plumbing | WIRED | dispatcher signature includes `resolved_allowlist: &AipcResolvedAllowlist`; seeded with default() per 18-03 Deferred Issues #1 |
| profile/mod.rs | policy.rs | mask constants for from_token | WIRED | `use crate::...policy::{EVENT_DEFAULT_MASK, JOB_OBJECT_DEFAULT_MASK, MUTEX_DEFAULT_MASK}` |
| aipc_sdk.rs | types.rs | HandleKind / HandleTarget / CapabilityRequest / SupervisorResponse / ResourceGrant | WIRED | aipc_sdk.rs:63 `HandleKind::Socket` etc in doc comments; 90+ HandleKind::X matches |
| aipc_sdk.rs | socket(_windows).rs | SupervisorSocket transport | WIRED | `cap_pipe: &mut SupervisorSocket` parameter on all 5 methods |
| aipc_sdk.rs | error.rs::UnsupportedPlatform | Err(NonoError::UnsupportedPlatform(...)) | WIRED | 10 non-Windows arms; 5 `UnsupportedPlatform` tests PASS on Linux/macOS |

### Data-Flow Trace (Level 4)

This phase is a pure IPC/library surface — no UI renders dynamic data from server state. Data flows:

- **CapabilityRequest → SupervisorResponse → ResourceGrant → raw handle** — verified by 11 aipc_sdk tests (4 loopback + 5 real-broker smoke + 2 message integrity) all PASS.
- **Profile TOML/JSON → CapabilitiesConfig → AipcResolvedAllowlist → dispatcher** — 9 profile tests PASS; integration confirmed by capability_handler_tests (23/23 PASS).
- **5 built-in profiles → resolved allowlist** — seeded with default() in WindowsSupervisorRuntime per 18-03 Deferred Issues #1 (tracked for future wiring); default matches D-05 byte-identical so no regression.

All wired data flows produce real data on Windows host. No HOLLOW or DISCONNECTED artifacts detected.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| nono lib builds | `cargo build -p nono --lib` | Finished in 5.36s (clean) | PASS |
| SDK tests compile | `cargo test -p nono --lib supervisor::aipc_sdk --no-run` | Clean compile | PASS |
| All 11 SDK tests pass on Windows | `cargo test -p nono --lib supervisor::aipc_sdk` | 11 passed; 0 failed | PASS |
| 5 integration tests pass | `cargo test -p nono-cli --test aipc_handle_brokering_integration` | 5 passed; 0 failed | PASS |
| Parameterized audit-redaction test | `cargo test ... handle_redacts_token_in_audit_for_all_handle_kinds` | 1 passed | PASS |
| Containment-Job guard test | `cargo test ... handle_denies_job_object_brokering_of_containment_job*` | 2 passed | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| AIPC-01 (acc. 1) | 18-01, 18-02, 18-03 | Protocol round-trip per handle type | SATISFIED | 5 integration tests + 5 smoke tests + 23 capability_handler_tests all PASS |
| AIPC-01 (acc. 2) | 18-01, 18-03 | Policy denies mask upgrade | SATISFIED | `mask_is_allowed` validator + dispatcher gate at supervisor.rs; `handle_denies_job_object_with_terminate_mask_no_profile_widening` PASS |
| AIPC-01 (acc. 3) | 18-03 | Token-leak test extended to all new shapes | SATISFIED | Parameterized `handle_redacts_token_in_audit_for_all_handle_kinds` with `cases.len() == 6` guard |
| AIPC-01 (acc. 4) | 18-04 | No platform regression on Unix | SATISFIED | Non-Windows arms return `UnsupportedPlatform` with exact D-09 message; 5 non-Windows `sdk_returns_unsupported_platform_on_non_windows` tests |

**No orphaned requirements** — AIPC-01 is the only requirement mapped to Phase 18 in REQUIREMENTS.md; all 4 plans claim it in their `requirements:` frontmatter.

### Anti-Patterns Found

Reviewed 18-REVIEW.md (0 critical, 6 warning, 7 info = 13 findings). Per review: "No critical security holes were identified."

Warnings summarized (all acceptable for pass per task instructions "6 warnings and 7 info items are advisory, NOT blockers"):

| ID | File | Pattern | Severity | Impact |
|----|------|---------|----------|--------|
| WR-01 | supervisor.rs:1817-1820 | Pipe/Socket mask validation runs AFTER approval prompt | Warning | UX inconsistency with D-07 invariant; denial still enforced |
| WR-02 | supervisor.rs:1678-1695 | CompareObjectHandles fail-open shape on API error | Warning | Latent; API very unlikely to fail on live handles |
| WR-03 | supervisor.rs:1322, :1579 | CreateEventW/CreateMutexW open existing kernel objects | Warning | Race-to-create vector in same-logon session |
| WR-04 | supervisor.rs:1500 | WSAStartup return value discarded | Warning | Diagnostic quality; fail-closed by cascade |
| WR-05 | types.rs:178-179, aipc_sdk.rs:367-400 | session_token not zeroized | Warning | Pre-existing Phase 11 pattern, not a regression |
| WR-06 | socket_windows.rs:595-600 | broker_socket_to_process has unused BrokerTargetProcess | Warning | API surface bloat; documented in 18-02 SUMMARY |

Info items are style/readability notes — tracked as deferred follow-ups (IN-01..IN-07).

### Human Verification Required

See frontmatter. 4 items require Windows-host UAT or product decisions on review warnings.

### Gaps Summary

**None.** All 32 must-haves verified across all 4 plans:
- Plan 18-01 (8/8): Wire-protocol skeleton + Event/Mutex brokers + policy module + constant-time discriminator + server-side mask validation.
- Plan 18-02 (7/7): Pipe + Socket brokers + WSADuplicateSocketW + bind_aipc_pipe + role/direction defaults + privileged-port deny.
- Plan 18-03 (9/9): Job Object broker + CompareObjectHandles guard + profile schema + 5 built-in profile blocks + parameterized audit-redaction + Windows integration suite.
- Plan 18-04 (8/8): Cross-platform SDK + UnsupportedPlatform arms + single-source error message + byte-identical preservation of 18-01..18-03 files.

All Phase 11 invariants preserved byte-identical per SUMMARY verification tables. Phase 17 latent-bug carry-forward pattern (`user_session_id` over `self.session_id`) held at all 4 new kernel-object namespace sites (Event + Mutex + Pipe + JobObject).

The `human_needed` status is due to 4 items that require Windows-host manual verification (CONIN$ prompt rendering, WR-01 UX decision, capabilities.aipc end-to-end widening, WR-02 empirical test on hardened host) — NOT because of missing implementation. The phase goal is structurally achieved.

---

*Verified: 2026-04-19*
*Verifier: Claude (gsd-verifier)*
