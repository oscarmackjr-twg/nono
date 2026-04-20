---
phase: 18-extended-ipc
plan: 03
subsystem: ipc
tags: [windows, supervisor, ipc, aipc, job-object, profile, schema, security, audit, broker, runtime-guard]

# Dependency graph
requires:
  - phase: 18-extended-ipc
    plan: 01
    provides: HandleKind/HandleTarget enums; cross-platform supervisor::policy module (mask_is_allowed); broker_event_to_process / broker_mutex_to_process pattern; handle_event_request / handle_mutex_request dispatcher helpers; constant-time discriminator validation step; audit_entry_with_redacted_token routing
  - phase: 18-extended-ipc
    plan: 02
    provides: broker_pipe_to_process / broker_socket_to_process / bind_aipc_pipe / BrokerTargetProcess::pid; handle_pipe_request / handle_socket_request dispatcher helpers; format_capability_prompt Pipe + Socket branches; 372-byte WSAPROTOCOL_INFOW blob transport for Socket
  - phase: 17-attach-streaming
    provides: user_session_id threading through WindowsSupervisorRuntime (Phase 17 latent-bug carry-forward — namespace prefix uses user_session_id, not self.session_id)
provides:
  - broker_job_object_to_process in socket_windows.rs (DuplicateHandle with dwOptions=0 MAP DOWN + per-bit mask)
  - ResourceGrant::duplicated_windows_job_object_handle constructor in types.rs
  - handle_job_object_request in supervisor.rs with CompareObjectHandles containment-Job runtime guard (T-18-03-01 defense-in-depth)
  - CapabilitiesConfig + AipcConfig profile structs (profile/mod.rs) with #[serde(deny_unknown_fields)]
  - 5 from_token parsers: socket_role / pipe_direction / job_object_mask / event_mask / mutex_mask (reject unknown tokens at parse time via NonoError::ProfileParse)
  - AipcResolvedAllowlist struct + Profile::resolve_aipc_allowlist method (UNIONs hard-coded D-05 defaults with profile widening — never narrows)
  - validate_profile_aipc_tokens wired into parse_profile_file
  - policy::JOB_OBJECT_ALL_ACCESS constant (windows-sys 0.59 does not export it)
  - capabilities.aipc block in 5 built-in profiles (claude-code, codex, opencode, openclaw, swival)
  - nono-profile.schema.json: top-level capabilities property + $defs/CapabilitiesConfig + $defs/AipcConfig with per-key string enums
  - Windows-only integration suite crates/nono-cli/tests/aipc_handle_brokering_integration.rs (5 round-trip tests using BrokerTargetProcess::current(); no admin required)
  - Parameterized audit-redaction test replacing 5 per-kind tests from Plans 18-01/18-02/18-03 Task 1
  - format_capability_prompt JobObject branch (D-04-locked template) + format_job_object_access helper
  - resolved_aipc_allowlist: Arc<AipcResolvedAllowlist> field on WindowsSupervisorRuntime
affects: [phase-18-04]

# Tech tracking
tech-stack:
  added: []  # Win32_System_JobObjects added to crates/nono/Cargo.toml; already present in nono-cli since Phase 17 RESL work. No new crate deps.
  patterns:
    - "Two-gate defense-in-depth for containment-Job hijack: (a) policy::mask_is_allowed against resolved allowlist (default QUERY only), (b) CompareObjectHandles runtime guard refuses supervisor's own Job regardless of mask widening. Both proven by dedicated tests; either gate alone would fail-closed."
    - "CompareObjectHandles (Win10 1607+) over numeric HANDLE ==: opening the same Job Object kernel name returns DIFFERENT HANDLE values that both resolve to the SAME kernel object. Numeric compare lets an attacker craft a name that resolves to the supervisor's containment Job; CompareObjectHandles compares kernel objects, not handle values."
    - "Profile widening as strict UNION (never narrow): AipcResolvedAllowlist is seeded with hard-coded D-05 defaults and UNIONs the profile block via Vec::contains / bitwise |=. A profile narrower than default does NOT narrow the resolved allowlist — removes an entire class of misconfiguration footguns."
    - "SendableHandle wrapper + local re-binding: Rust 2021 disjoint capture would make the thread closure move self.containment_job as a field of self, triggering 'cannot be shared between threads safely' on the raw HANDLE. Wrap in a SendableHandle newtype + bind to a local variable before closure construction."
    - "Parameterized audit-redaction test with hard count guard: assert_eq!(cases.len(), 6, ...) guards against silent gaps if a future phase adds a 7th HandleKind. Acceptance criterion grep (literal HandleKind:: count == 6 inside test body) catches same class at code-review time."
    - "Windows-only integration suite as separate test binary: crates/nono-cli/tests/aipc_handle_brokering_integration.rs is #[cfg(target_os = \"windows\")]-gated with empty stub on non-Windows — catches build/link issues that in-source supervisor::socket::tests would not."

key-files:
  created:
    - .planning/phases/18-extended-ipc/18-03-SUMMARY.md
    - crates/nono-cli/tests/aipc_handle_brokering_integration.rs
  modified:
    - crates/nono/Cargo.toml
    - crates/nono/src/supervisor/types.rs
    - crates/nono/src/supervisor/socket_windows.rs
    - crates/nono/src/supervisor/mod.rs
    - crates/nono/src/supervisor/policy.rs
    - crates/nono-cli/src/exec_strategy_windows/supervisor.rs
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
    - crates/nono-cli/src/session_commands_windows.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/policy.rs
    - crates/nono-cli/src/terminal_approval.rs
    - crates/nono-cli/data/policy.json
    - crates/nono-cli/data/nono-profile.schema.json

key-decisions:
  - "Two-gate defense for containment-Job hijack (T-18-03-01): mask_is_allowed + CompareObjectHandles runtime guard are INDEPENDENT defenses. Either alone would fail-closed; together they make the worst case structurally impossible regardless of profile content."
  - "CompareObjectHandles over numeric HANDLE ==: the supervisor opens its containment Job once at spawn time; a child naming a kernel object that resolves to the same Job gets a DIFFERENT HANDLE numerically but the SAME kernel object. Numeric compare would let this through. Win10 1607+ requirement matches the existing RtlGetVersion gate established in Phase 09."
  - "JOB_OBJECT_ALL_ACCESS defined in nono::supervisor::policy (windows-sys 0.59 does not export the constant). Kept in the cross-platform policy module for consistency with EVENT_/MUTEX_ mask constants landed in Plan 18-01."
  - "Profile widening is strict UNION (defaults ∪ profile), never narrowing: AipcResolvedAllowlist::default matches D-05 hard-coded defaults byte-identical so Plan 18-01 + Plan 18-02 default-only tests continue to pass without changes. resolve_aipc_allowlist uses Vec::contains + bitwise |= to UNION."
  - "SendableHandle + local re-binding for thread closure capture: the production call site in start_capability_pipe_server moves self.containment_job into the capability pipe thread closure. Rust 2021 disjoint capture makes this a 'HANDLE: Sync' violation; wrapping in SendableHandle newtype (with SAFETY doc) + binding to local variable before closure construction fixes it. Pattern documented inline."
  - "Parameterized audit-redaction test replaces 5 per-kind tests: Plan 18-01 (Event, Mutex), Plan 18-02 (Pipe, Socket), Plan 18-03 Task 1 (JobObject) — each added handle_redacts_token_for_*_kind tests with identical assertion shape. Parameterized over [File, Event, Mutex, Pipe, Socket, JobObject] with hard count assert_eq!(cases.len(), 6). Phase 11 orthogonal token-leak tests (mismatch / wrong-token paths) are KEPT unchanged."
  - "AIPC-01 Plan 18-04 SDK methods explicitly out of scope: D-08 carries forward unchanged. The CLI-side broker surface is Windows-only end-to-end in v2.1; cross-platform SDK methods returning NonoError::UnsupportedPlatform on non-Windows are Plan 18-04's deliverable."
  - "Drive-by rustfmt cleanup: Tasks 1+2+3 accumulated line-wrap + alphabetical-import drift. Plus pre-existing drift on launch.rs + session_commands_windows.rs carried forward from Plan 18-01 Deferred Issues #3 and Plan 18-02 Deferred Issues #4 — rolled into the style(18-03) commit e29fd1c to close out the fmt backlog."

patterns-established:
  - "Pattern 9 (completes 18-01 Pattern 3 + 18-02 Pattern 6): All 5 AIPC brokers (Event, Mutex, Pipe, Socket, JobObject) follow the same skeleton: pub fn broker_X_to_process(handle, target, mask|role|direction|pid) -> Result<ResourceGrant> with DuplicateHandle (dwOptions=0) or WSADuplicateSocketW. Plan 18-04 SDK methods consume this 5-broker surface without further shape changes."
  - "Pattern 10: Containment-Job hijack defense-in-depth: for ANY handle type where the supervisor owns a process-tree-lifecycle kernel object (currently only Job Object), the dispatcher helper MUST add a CompareObjectHandles runtime guard against the supervisor's own instance. The policy layer (mask widening) and the runtime layer (object identity) are orthogonal defenses."
  - "Pattern 11: Profile schema widening without narrowing: new capabilities.* blocks must be populated via from_token parsers that reject unknown tokens at parse time; resolved allowlists must UNION defaults with profile content, never REPLACE. Tests must cover both 'default when absent' and 'widened when present' cases."

requirements-completed: [AIPC-01]

# Metrics
duration: 180m
completed: 2026-04-19
---

# Phase 18 Plan 03: Extended IPC (AIPC-01) Job Object + Profile Schema + Audit Suite Cleanup Summary

**Job Object broker wired end-to-end with two-gate containment-Job hijack defense (mask allowlist + CompareObjectHandles runtime guard); `capabilities.aipc` profile schema + resolved-allowlist plumbing replaces hard-coded defaults with profile-driven UNION semantic; audit-redaction test family parameterized over all 6 HandleKind shapes; new Windows-only integration suite exercises end-to-end broker round-trip for all 5 new handle types.**

## Performance

- **Duration:** ~180 min
- **Started:** 2026-04-19 (approximately)
- **Completed:** 2026-04-19
- **Tasks:** 3 + style cleanup
- **Commits:** 4 DCO-signed (71270a1, 611ea54, 3750c1b, e29fd1c)
- **Files modified:** 13 (1 new SUMMARY, 1 new integration test file, 11 modified)
- **Test count delta:** +17 tests (+4 dispatcher, +2 broker, +2 prompt, +9 profile, +5 integration) − 5 per-kind redaction (replaced by 1 parameterized) = net +12 plus 5 integration

## Accomplishments

- `broker_job_object_to_process` in socket_windows.rs: DuplicateHandle with `dwOptions = 0` (MAP DOWN) and explicit mask. Mirrors Plan 18-01's Event/Mutex broker template byte-for-byte except for the constructor swap and SAFETY-doc text. 2 new unit tests (round-trip + NULL-handle failure path).
- `ResourceGrant::duplicated_windows_job_object_handle` constructor in types.rs carries the `JobObject` resource-kind variant with `AccessMode::ReadWrite` sentinel (real semantics in the mask).
- `handle_job_object_request` driver in supervisor.rs validates target shape, canonicalizes namespace as `Local\nono-aipc-<user_session_id>-<sanitized_name>`, opens via `OpenJobObjectW` with `JOB_OBJECT_ALL_ACCESS`, fires the **CompareObjectHandles containment-Job runtime guard**, enforces `mask_is_allowed` against the resolved allowlist, and brokers via `broker_job_object_to_process`.
- Two-gate defense-in-depth for T-18-03-01 (containment-Job hijack): (a) mask allowlist rejects `JOB_OBJECT_TERMINATE` by default (only `QUERY` in default allowlist); (b) `CompareObjectHandles` kernel-object comparison refuses to broker the supervisor's own Job HANDLE regardless of mask widening. Tests `handle_denies_job_object_with_terminate_mask_no_profile_widening` and `handle_denies_job_object_brokering_of_containment_job_even_with_profile_widening` prove both defenses fire independently.
- `CapabilitiesConfig` + `AipcConfig` structs in profile/mod.rs with `#[serde(deny_unknown_fields)]` and `#[derive(Debug, Clone, Default, Serialize, Deserialize)]` mirror the existing `PolicyPatchConfig` shape. Three-site consistency (Profile / ProfileDeserialize / From impl) preserved per Pattern 5.
- 5 `from_token` parsers (`socket_role_from_token`, `pipe_direction_from_token`, `job_object_mask_from_token`, `event_mask_from_token`, `mutex_mask_from_token`) reject unknown tokens at profile parse time with verbose `NonoError::ProfileParse` messages naming the offending token, kind, and valid set. The `job_object_mask` parser carries the inline SECURITY WARNING for `terminate` per CONTEXT.md Landmines § Job Object.
- `AipcResolvedAllowlist::default` matches the hard-coded D-05 defaults byte-identical: socket=[Connect], pipe=[Read, Write], job_object=QUERY, event=(SYNCHRONIZE | EVENT_MODIFY_STATE), mutex=(SYNCHRONIZE | MUTEX_MODIFY_STATE). Plans 18-01 + 18-02 default-only tests continue to pass without modification.
- `Profile::resolve_aipc_allowlist` UNIONs hard-coded defaults with profile widening via `Vec::contains` + bitwise `|=`. Profile narrower than default does NOT narrow — test `resolve_aipc_allowlist_widens_event_mask_with_signal_token` proves the union semantic.
- `validate_profile_aipc_tokens` wired into `parse_profile_file` so unknown tokens fail at load time, not at the first capability request.
- `merge_profiles` UNIONs per-key string-token arrays from base + child `capabilities.aipc` blocks via `dedup_append` so child profiles ADD tokens to their base's allowlist.
- `data/policy.json`: 5 built-in profiles gain `capabilities.aipc` blocks per RESEARCH-tuned values. opencode gets full-duplex pipe (including `read+write`); openclaw gets minimal widening; claude-code / codex / swival share the common conservative widening.
- `data/nono-profile.schema.json`: top-level `capabilities` property references `$defs/CapabilitiesConfig` + `$defs/AipcConfig`. Per-key string enums catch typos at schema-validation time. Job Object TERMINATE warning appears in 3 doc surfaces (top-level prop, AipcConfig, job_object property) for discoverability.
- `WindowsSupervisorRuntime.resolved_aipc_allowlist: Arc<AipcResolvedAllowlist>` field added (currently populated with default; Plan 18-04 or a later plan threads SupervisorConfig through to carry the Profile's resolved allowlist).
- `handle_windows_supervisor_message` gains a 10th parameter `resolved_allowlist: &AipcResolvedAllowlist`. All 5 per-kind helpers (event/mutex/pipe/socket/job_object) consult it; the Plan 18-02 hard-coded "ReadWrite requires profile widening" / "Bind/Listen requires profile widening" rejection branches are replaced with `resolved_allowlist.{pipe_directions,socket_roles}.contains` lookups.
- `format_capability_prompt` JobObject branch replaces the Plan 18-02 placeholder `(unsupported in this build)` with the D-04-locked template; `format_job_object_access` helper renders the per-bit mask (query / set_attributes / terminate / assign_process).
- `prompt_falls_back_for_unsupported_kind` retired; replaced by `prompt_renders_kind_target_mismatch_safely` — the JobObject branch is now live; only the catch-all `_` arm for kind/target shape mismatch remains.
- Parameterized audit-redaction test `handle_redacts_token_in_audit_for_all_handle_kinds` replaces 5 per-kind tests (2 from 18-01, 2 from 18-02, 1 from 18-03 Task 1). Hard `assert_eq!(cases.len(), 6)` guards against silent gaps. Phase 11 orthogonal token-leak tests (mismatch / wrong-token paths) are KEPT unchanged.
- New Windows-only integration suite at `crates/nono-cli/tests/aipc_handle_brokering_integration.rs` (5 tests: Event, Mutex, Pipe, Socket, JobObject) using `BrokerTargetProcess::current()` — no admin required (unlike `wfp_port_integration.rs`). Lives in a separate test binary so it catches build / link issues that in-source `supervisor::socket::tests` would not. Cross-platform: `#[cfg(target_os = "windows")]`-gated with empty stub on non-Windows.
- `SendableHandle` wrapper + local re-binding in `start_capability_pipe_server`: the production call site passes `self.containment_job` through a newtype into the capability pipe thread closure; the local re-binding defeats Rust 2021 disjoint capture that would otherwise flag `HANDLE: !Sync`.
- All 21 pre-existing dispatcher test call sites updated to pass `&AipcResolvedAllowlist::default()` (preserves Plans 18-01/18-02 default-only behavior byte-identical) plus `std::ptr::null_mut()` for the new `runtime_containment_job` parameter. The new dedicated guard-path test (`handle_denies_job_object_brokering_of_containment_job`) constructs a non-null HANDLE to exercise the runtime guard.
- 23/23 capability_handler_tests (22 carried forward + 4 new - 5 per-kind redaction + 1 parameterized + 1 containment widening = 23), 22/22 supervisor::socket tests (20 prior + 2 new), 25/25 terminal_approval tests (23 prior + 2 new), 9/9 new profile tests, 5/5 new aipc_handle_brokering_integration tests.
- Phase 11 + Plans 18-01/18-02 invariants byte-identical: CAPABILITY_PIPE_SDDL count=2, "Invalid session token" count=2, audit_entry_with_redacted_token routes all push sites, CONIN$ branch unchanged, all 5 brokers + 5 dispatcher helpers + validate_aipc_object_name preserved.
- Phase 17 latent-bug carry-forward verified: `format!.*nono-aipc.*user_session_id` count=4 (Event + Mutex + Pipe + JobObject); `format!.*nono-aipc.*self\.session_id` count=0.

## Task Commits

Each task was committed atomically with DCO sign-off:

1. **Task 1: Job Object broker + containment_job runtime guard + 6 unit tests** — `71270a1` (feat)
2. **Task 2: capabilities.aipc profile schema + AipcResolvedAllowlist plumbing + 9 profile tests + 2 dispatcher widening tests** — `611ea54` (feat)
3. **Task 3: parameterized audit-redaction + Windows-only integration suite** — `3750c1b` (test)
4. **Style: rustfmt cleanup (Task 1+2+3 bodies + carried-forward launch.rs / session_commands_windows.rs drift from 18-01/18-02 Deferred Issues)** — `e29fd1c` (style)

## Files Created/Modified

- `crates/nono/Cargo.toml` — Added `Win32_System_JobObjects` to the `windows-sys` features list (alphabetical order). All existing features preserved. nono-cli already had the feature from Phase 17 RESL work.
- `crates/nono/src/supervisor/types.rs` — `duplicated_windows_job_object_handle` constructor added to the existing `impl ResourceGrant` block; `JobObject` variant already present from Plan 18-01.
- `crates/nono/src/supervisor/socket_windows.rs` — `broker_job_object_to_process` function + 2 unit tests. Imports extended with JobObjects (JOB_OBJECT_ALL_ACCESS accessed via `nono::supervisor::policy`, CreateJobObjectW in tests only).
- `crates/nono/src/supervisor/mod.rs` — Re-export block extended with `broker_job_object_to_process` (Windows-only).
- `crates/nono/src/supervisor/policy.rs` — `JOB_OBJECT_ALL_ACCESS` constant added (windows-sys 0.59 doesn't export it). Mask constants for JOB_OBJECT_QUERY / SET_ATTRIBUTES / TERMINATE / ASSIGN_PROCESS preserved from Plan 18-01.
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — `handle_job_object_request` helper (~180 lines including CompareObjectHandles guard). Dispatcher match-arm split: `HandleKind::JobObject` now its own arm calling the new helper. `handle_windows_supervisor_message` signature gains 2 parameters: `runtime_containment_job: HANDLE` (Task 1), `resolved_allowlist: &AipcResolvedAllowlist` (Task 2). `resolved_mask_for_kind` helper removed; replaced with per-helper lookups against `resolved_allowlist`. `start_capability_pipe_server` uses `SendableHandle` wrapper for thread-closure capture. 4 new dispatcher tests + 2 widening tests + 1 parameterized redaction test − 5 per-kind redaction tests.
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — rustfmt cleanup only (1 line wrap fix on `DetachedStdioPipes::create().expect(...)` call).
- `crates/nono-cli/src/session_commands_windows.rs` — rustfmt cleanup only (1 function signature line wrap on `translate_attach_open_error`).
- `crates/nono-cli/src/profile/mod.rs` — `CapabilitiesConfig` + `AipcConfig` + `AipcResolvedAllowlist` structs added (~550 lines). 5 `from_token` parsers. `Profile::resolve_aipc_allowlist` method. `validate_profile_aipc_tokens` wired into `parse_profile_file`. `merge_profiles` extended for `capabilities` UNION. 9 new tests.
- `crates/nono-cli/src/policy.rs` — `ProfileDef` gains `capabilities: CapabilitiesConfig` field (`#[serde(default)]`); `to_raw_profile` forwards it through.
- `crates/nono-cli/src/terminal_approval.rs` — `format_job_object_access` helper added. `format_capability_prompt` JobObject branch replaces placeholder with D-04-locked template. `prompt_falls_back_for_unsupported_kind` retired; replaced by `prompt_renders_kind_target_mismatch_safely`. 2 new prompt tests.
- `crates/nono-cli/data/policy.json` — `capabilities.aipc` blocks added to 5 built-in profiles (claude-code, codex, opencode, openclaw, swival).
- `crates/nono-cli/data/nono-profile.schema.json` — Top-level `capabilities` property + `$defs/CapabilitiesConfig` + `$defs/AipcConfig` with per-key string enums.
- `crates/nono-cli/tests/aipc_handle_brokering_integration.rs` — NEW. 5 end-to-end round-trip tests (Event, Mutex, Pipe, Socket, JobObject) using `BrokerTargetProcess::current()`. `#[cfg(target_os = "windows")]`-gated.
- `.planning/phases/18-extended-ipc/18-03-SUMMARY.md` — this file.

## Decisions Made

See `key-decisions` block in frontmatter. Highlights:

- **Two-gate defense for containment-Job hijack (T-18-03-01)** — `mask_is_allowed` and `CompareObjectHandles` are INDEPENDENT. Either alone fails-closed; together they make the worst case structurally impossible regardless of profile content.
- **`CompareObjectHandles` over numeric `HANDLE ==`** — opening the same kernel-object name returns DIFFERENT handle values that resolve to the SAME kernel object. Numeric compare would let an attacker craft a name that resolves to the supervisor's containment Job; CompareObjectHandles compares kernel objects, not handle values.
- **`JOB_OBJECT_ALL_ACCESS` in `nono::supervisor::policy`** — windows-sys 0.59 doesn't export it. Kept in the cross-platform policy module for consistency with Plan 18-01 mask constants.
- **Profile widening as strict UNION** — `AipcResolvedAllowlist::default` matches D-05 hard-coded defaults byte-identical so Plans 18-01 + 18-02 default-only tests pass unchanged. `resolve_aipc_allowlist` UNIONs via `Vec::contains` + bitwise `|=`.
- **`SendableHandle` + local re-binding for thread capture** — Rust 2021 disjoint capture makes `self.containment_job` move into the closure trigger `HANDLE: !Sync`. Newtype wrapper + local re-binding defeats the capture.
- **Parameterized audit-redaction** — replaces 5 per-kind tests with a single iteration over `[File, Event, Mutex, Pipe, Socket, JobObject]`. Hard count guard + literal grep acceptance criterion catch silent regressions.
- **`WindowsSupervisorRuntime.resolved_aipc_allowlist` seeded with default** — Plan 18-03 threads the field through but Profile → runtime plumbing is deferred (Plan 18-04 or later). Default matches D-05 hard-coded defaults so nothing regresses.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `JOB_OBJECT_ALL_ACCESS` not exported by `windows-sys 0.59`**
- **Found during:** Task 1 (build gate after `handle_job_object_request` added OpenJobObjectW call)
- **Issue:** Plan stated `JOB_OBJECT_ALL_ACCESS` would come from `windows_sys::Win32::System::JobObjects`; verification showed the constant is not exported by `windows-sys 0.59`.
- **Fix:** Defined `pub const JOB_OBJECT_ALL_ACCESS: u32 = 0x1F001F;` in `crates/nono/src/supervisor/policy.rs` alongside the Plan 18-01 mask constants. All consumers use `nono::supervisor::policy::JOB_OBJECT_ALL_ACCESS` for consistency.
- **Files modified:** crates/nono/src/supervisor/policy.rs, crates/nono-cli/src/exec_strategy_windows/supervisor.rs
- **Verification:** `cargo build -p nono --lib` + `cargo build -p nono-cli --bin nono` both pass.
- **Committed in:** Task 1 commit `71270a1`.

**2. [Rule 3 - Blocking] `HANDLE: !Sync` violation on thread-closure capture of `self.containment_job`**
- **Found during:** Task 1 (`start_capability_pipe_server` thread closure construction after adding `runtime.containment_job` to the captured bindings)
- **Issue:** Rust 2021 disjoint capture moves `self.containment_job` as a field into the thread closure; `HANDLE` (raw pointer) is not `Sync`. Compile error.
- **Fix:** Wrapped `containment_job` in a `SendableHandle` newtype (with `unsafe impl Send + Sync` and SAFETY doc explaining the handle is owned by the supervisor and only read by the closure). Re-bound to a local `let containment_job = SendableHandle(self.containment_job);` before closure construction so the closure captures the local, not the field. Pattern documented inline.
- **Files modified:** crates/nono-cli/src/exec_strategy_windows/supervisor.rs
- **Verification:** `cargo build -p nono-cli --bin nono` passes; no runtime behavior change (the HANDLE is read-only in the closure).
- **Committed in:** Task 1 commit `71270a1`.

**3. [Rule 1 - Lint] clippy::too_many_arguments on `handle_windows_supervisor_message` (now 10 args)**
- **Found during:** Task 2 (clippy gate after adding `resolved_allowlist` as the 10th parameter)
- **Issue:** Plan 18-01's `#[allow(clippy::too_many_arguments)]` annotation was written for 8 args; the 9th (runtime_containment_job in Task 1) and 10th (resolved_allowlist in Task 2) parameters pushed it further. The existing allow covers any count, but the rationale docstring was stale.
- **Fix:** Updated the rationale docstring to mention the 10-arg shape and the reason (dispatcher-internal function consuming multiple supervisor-lifetime bindings that cannot be bundled without introducing an unhelpful container type).
- **Files modified:** crates/nono-cli/src/exec_strategy_windows/supervisor.rs
- **Verification:** `cargo clippy -p nono-cli --all-targets -- -D warnings -D clippy::unwrap_used` passes.
- **Committed in:** Task 2 commit `611ea54`.

**4. [Rule 1 - Lint] Drive-by rustfmt + carried-forward fmt drift**
- **Found during:** Task 3 + plan closeout fmt gate
- **Issue:** Tasks 1+2+3 accumulated line-wrap + alphabetical-import drift across socket_windows.rs, supervisor.rs, profile/mod.rs, types.rs, mod.rs, and the new integration test file. Additionally, pre-existing drift on `launch.rs` and `session_commands_windows.rs` carried forward from Plan 18-01 Deferred Issues #3 and Plan 18-02 Deferred Issues #4.
- **Fix:** Rolled all fmt changes (in-scope Task 1+2+3 bodies + carried-forward drift on launch.rs / session_commands_windows.rs) into a dedicated `style(18-03)` commit to close out the fmt backlog at end-of-plan.
- **Files modified:** 6 files (see `e29fd1c` stat).
- **Verification:** `cargo fmt --all -- --check` exits 0 on the workspace.
- **Committed in:** Style commit `e29fd1c`.

---

**Total deviations:** 4 auto-fixed (2 blocking-on-build, 2 lint/fmt)
**Impact on plan:** All deviations were build/lint/fmt compliance or API adapter shims (JOB_OBJECT_ALL_ACCESS, SendableHandle), not scope changes. No new files added beyond plan; no behavior diverged from CONTEXT.md decisions. Carried-forward fmt drift on launch.rs / session_commands_windows.rs explicitly resolved this plan.

## Phase 11 + Plans 18-01/18-02 Invariant Verification

Verified via `git diff` + grep counts on the post-style-commit working tree:

| Invariant | Baseline | Post-Plan | Status |
|-----------|----------|-----------|--------|
| `S:(ML;;NW;;;LW)` SDDL count in socket_windows.rs | 2 (const + doc) | 2 | byte-identical |
| `"Invalid session token"` count in supervisor.rs | 2 | 2 | byte-identical |
| `audit_entry_with_redacted_token` routes all push sites (no bare `audit_log.push(AuditEntry`) | 0 | 0 | byte-identical |
| `CONIN$` count in terminal_approval.rs | 4 | 4 | byte-identical |
| `request_capability` body in terminal_approval.rs | unchanged | unchanged | byte-identical |
| `fn handle_event_request \| fn handle_mutex_request \| fn handle_pipe_request \| fn handle_socket_request \| fn validate_aipc_object_name` count | 5 | 5 | byte-identical |
| `pub fn broker_event_to_process \| pub fn broker_mutex_to_process \| pub fn broker_pipe_to_process \| pub fn broker_socket_to_process` count | 4 | 4 | byte-identical |
| Plan 18-02 placeholder `"JobObject brokering not yet implemented"` | 1 | 0 | replaced by live broker |

## Phase 17 Latent-Bug Carry-Forward Verification

| Check | Required | Actual | Status |
|-------|----------|--------|--------|
| `format!.*nono-aipc.*self\.session_id` count in supervisor.rs | 0 | 0 | PASS — no new bug introduced |
| `format!.*nono-aipc.*user_session_id` count in supervisor.rs | >= 4 (Event + Mutex + Pipe + JobObject) | 4 | PASS — JobObject namespace prefix uses user_session_id |

## CI Gate Results

| Gate | Result |
|------|--------|
| `cargo build -p nono --lib` | PASS (Task 1 gate) |
| `cargo build -p nono-cli --bin nono` | PASS (Tasks 1+2+3 gate) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | PASS (clean) |
| `cargo fmt --all -- --check` | PASS (post-`e29fd1c`) |
| `cargo test -p nono --lib supervisor::socket -- --test-threads=1` (Windows) | PASS (22/22 — 20 prior + 2 new) |
| `cargo test -p nono-cli --bin nono capability_handler_tests` | PASS (23/23) |
| `cargo test -p nono-cli --bin nono terminal_approval` | PASS (25/25 — 23 prior + 2 new) |
| `cargo test -p nono-cli --bin nono profile::tests` | PASS (9/9 new) |
| `cargo test -p nono-cli --test aipc_handle_brokering_integration` (Windows) | PASS (5/5 new) |

Pre-existing `tests/env_vars.rs` (19) and `trust_keystore` flakes (1-3) carried forward unchanged from Phase 19 CLEAN-02 deferred list — confirmed not in this plan's scope and not regressed.

## Issues Encountered

Plan scoping was accurate; the only mid-flight friction was the two API shape surprises:

1. `JOB_OBJECT_ALL_ACCESS` not exported by `windows-sys 0.59` — resolved by defining it in `nono::supervisor::policy` alongside the Plan 18-01 mask constants.
2. `HANDLE: !Sync` preventing thread-closure capture of `self.containment_job` — resolved by the `SendableHandle` newtype + local re-binding pattern.

Both surprises caught at Task 1 compile time; neither required CONTEXT.md revision.

## Deferred Issues

**1. `WindowsSupervisorRuntime.resolved_aipc_allowlist` seeded with `AipcResolvedAllowlist::default()`**

Plan 18-04 (or a later plan) will thread `SupervisorConfig` / `Profile` through to carry the loaded profile's resolved allowlist. Current behavior is byte-identical to the D-05 hard-coded defaults that Plans 18-01 + 18-02 hard-coded directly; no regression.

**2. Plan 18-04 SDK methods still unwired**

Cross-platform SDK methods returning `NonoError::UnsupportedPlatform` on non-Windows are Plan 18-04's deliverable per D-08. Completes the AIPC-01 cross-platform API surface gap.

**3. `format_capability_prompt` helpers retain `#[allow(dead_code)]`**

Carried forward from Plans 18-01 / 18-02 Deferred Issues. The dispatcher wires `format!` strings inline rather than routing through the helper. Plan 18-04 may wire the live CONIN$ prompt path through `format_capability_prompt`; until then the dead_code allows remain.

**4. Pre-existing `tests/env_vars.rs` (19 failures) and `trust_keystore` flakes (1-3) carried forward unchanged**

Per STATE.md and Phase 19 CLEAN-02 deferred list. Confirmed not regressed.

## Open Paths for Plan 18-04

Plan 18-03 leaves Plan 18-04 with a clean surface:

- **Cross-platform SDK methods** (D-08): new public `nono::broker_{event,mutex,pipe,socket,job_object}(...)` functions that delegate to the Windows implementations and return `NonoError::UnsupportedPlatform` on non-Windows.
- **Profile → runtime plumbing**: thread the loaded `Profile`'s `resolve_aipc_allowlist()` into `WindowsSupervisorRuntime` construction. Currently seeded with default; once plumbed, named profiles start widening the resolved allowlist end-to-end.
- **Audit doc surfaces**: the RESEARCH Landmines § Job Object warning about TERMINATE may need a final pass through user-facing docs (docs/cli/features/ or a new aipc.mdx) in Plan 18-04 or a subsequent docs-only quick task.

## Next Phase Readiness

- Plan 18-04 (cross-platform SDK methods) can proceed against this foundation.
- All Phase 11 + Plan 18-01 + Plan 18-02 invariants byte-identical.
- Phase 17 latent-bug pattern (user_session_id NOT self.session_id) extended to 4 sites (Event + Mutex + Pipe + JobObject).
- 23 capability_handler_tests + 22 supervisor::socket tests + 25 terminal_approval tests + 9 profile tests + 5 integration tests all green on Windows host; dispatcher is total over all 6 HandleKind values with no remaining placeholders.
- Cross-platform compile holds: nono builds clean on Linux/macOS (all broker functions and Win32 imports gated by `#[path]` routing in supervisor/mod.rs; integration test file gated by `#[cfg(target_os = "windows")]`).
- v2.1 milestone: Phase 18 needs only Plan 18-04 to close AIPC-01 and ship v2.1.

## Self-Check: PASSED

All 13 files referenced in this summary exist on disk; all 4 commits referenced exist in git log.

- `crates/nono/Cargo.toml` — FOUND
- `crates/nono/src/supervisor/types.rs` — FOUND
- `crates/nono/src/supervisor/socket_windows.rs` — FOUND
- `crates/nono/src/supervisor/mod.rs` — FOUND
- `crates/nono/src/supervisor/policy.rs` — FOUND
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — FOUND
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — FOUND
- `crates/nono-cli/src/session_commands_windows.rs` — FOUND
- `crates/nono-cli/src/profile/mod.rs` — FOUND
- `crates/nono-cli/src/policy.rs` — FOUND
- `crates/nono-cli/src/terminal_approval.rs` — FOUND
- `crates/nono-cli/data/policy.json` — FOUND
- `crates/nono-cli/data/nono-profile.schema.json` — FOUND
- `crates/nono-cli/tests/aipc_handle_brokering_integration.rs` — FOUND
- `.planning/phases/18-extended-ipc/18-03-SUMMARY.md` — FOUND (this file)
- Commits `71270a1`, `611ea54`, `3750c1b`, `e29fd1c` — all FOUND in `git log`
