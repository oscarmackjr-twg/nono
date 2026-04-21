---
status: complete-with-issues
phase: 18-extended-ipc
source: [18-VERIFICATION.md]
started: 2026-04-19T00:00:00Z
updated: 2026-04-21T00:00:00Z
---

## Current Test

[COMPLETE-WITH-ISSUES — Phase 21 supervisor-pipe regression RESOLVED 2026-04-20 via commits 3c68377 + 938887f + e4c1bfa. 2026-04-20 UAT re-run on live `nono run --profile claude-code -- aipc-demo.exe` successfully brokered 4 of 5 AIPC handle types end-to-end (Event, Mutex, Pipe, Socket). Verdict distribution: 1 issue (D-04 prompt templates not rendered), 1 partial (WR-01 UX impact partially observed), 2 skipped (profile widening e2e not exercised; WR-02 EDR-instrumented host unavailable). Five new gaps (G-02..G-06) documented for /gsd-plan-phase 18.1 --gaps follow-up.]

## Prior Blocker (resolved by Phase 21)

All 4 tests exercise `nono run --profile claude-code -- <aipc-demo.exe>` (per `docs/cli/internals/aipc-uat-cookbook.mdx` Path B/C). The `claude-code` profile pulls in the `git_config` policy group whose members (`.gitconfig`, `.gitignore_global`, `.config/git/config`, `.config/git/ignore`, `.config/git/attributes`) are single-file grants. The Windows filesystem backend **used to** reject these with `WindowsUnsupportedIssueKind::SingleFileGrant` before the sandbox even launched. **Phase 21 (`.planning/phases/21-windows-single-file-grants/`) shipped the structural fix** (per-file Low-IL mandatory labels via `SetNamedSecurityInfoW`) on 2026-04-20, removing this blocker.

Path A (`cargo test -p nono --lib supervisor::aipc_sdk`, `cargo test -p nono-cli --test aipc_handle_brokering_integration`) was NOT blocked — it exercises the SDK surface + broker pipeline via the in-process `SupervisorSocket::pair()` test transport, not a live `nono run`.

## New Issue (carry-forward to /gsd-debug)

When the AIPC UAT cookbook was re-run on 2026-04-20 after Phase 21 landed (with inline ownership-skip fix `da25619` for system-owned paths like `C:\Windows`), a new regression surfaced on the `claude-code → supervised → aipc-demo` path:

```
Error: SandboxInit("Failed to connect to Windows supervisor pipe
\\\\.\\pipe\\nono-nono-cap-df52a0cb9818081e-pipe-... Access is denied. (os error 5).
Ensure the supervisor created the control channel before launching the child.")
```

This is the first time the full `claude-code → supervised → aipc-demo` flow has been exercised end-to-end on Windows — every prior run failed at compile time with `SingleFileGrant`. The supervisor-pipe access-denied is NOT introduced by Phase 21's label primitive; it's a pre-existing latent regression now made observable.

**Reproduction:** `nono run --profile claude-code -- aipc-demo.exe`
**First-observed commits:** `da25619` (Phase 21 ownership-skip fix) + Phase 21 label-apply landing on `windows-squash`
**Candidate hypotheses** (to carry into /gsd-debug session):
1. AppliedLabelsGuard labels `.cache\claude` / `.claude` with `NO_EXECUTE_UP` (mask 0x4 ReadWrite) and breaks supervisor handoff scripts/binaries staged there.
2. Phase 11 `CAPABILITY_PIPE_SDDL` admits mandatory label Low via SACL but may lack a DACL ACE granting connect/read/write to Low-IL subjects — first exposure now that a Low-IL child actually tries.
3. Supervisor startup silently failing before pipe creation.

Root cause will be investigated in a separate `/gsd-debug` session. All 4 HUMAN-UAT items below carry forward with `issue:` verdicts pending that investigation.

## Tests

### 1. CONIN$ approval prompt renders D-04-locked per-kind templates

expected: Each prompt shows the correct kind-specific fields (e.g. `proto=tcp host=... port=... role=connect` for socket) and approval grants a live handle; denial produces `grant=None` audit entry.
why_human: IN-01 in 18-REVIEW notes the dispatcher still wires `format!` strings inline rather than routing through `format_capability_prompt`; only tests consume the helper. Live CONIN$ text must be eyeballed on a real terminal to confirm UX integrity end-to-end.
result: [issue: prompts rendered generic fields ("Path: ...", "Access: read-only", "Reason: demo") instead of D-04 per-kind templates (`proto=tcp host=... port=... role=connect` for socket; `direction=read` for pipe; etc.). Supervisor-pipe path otherwise fully functional: Event/Mutex/Pipe/Socket brokers all prompted and granted live handles end-to-end on 2026-04-20. See G-02 for details and reproduction.]

### 2. WR-01 UX impact assessment — Pipe/Socket pre-approval gate

expected: Per the 18-01 summary invariant, Event/Mutex/JobObject reject BEFORE prompt; per WR-01, Pipe/Socket currently reject AFTER prompt. Confirm whether the UX impact is acceptable for v2.1 or requires a follow-up fix.
why_human: The reviewer flagged this as UX inconsistency rather than a security hole — a product decision is required on whether to accept the deviation or schedule a follow-up.
result: [partial: live approval prompts worked end-to-end on 2026-04-20 — all 5 broker requests prompted and user approvals granted (Event, Mutex, Pipe, Socket duplicated handles received; JobObject failed with a separate broker bug tracked as G-03). However the reject-BEFORE-prompt vs reject-AFTER-prompt semantic invariant was NOT empirically verified: no explicit deny-at-different-stages scenarios were run during the UAT (would require scripted deny flows or malformed capability request inputs). See G-05.]

### 3. capabilities.aipc profile widening end-to-end

expected: Widening the profile grants ReadWrite; removing the widening enforces default read-OR-write (not both). The UNION semantic is tested in profile::tests but end-to-end dispatcher consumption of the loaded profile's `resolve_aipc_allowlist` is deferred (18-03 Deferred Issues #1).
why_human: Plan 18-03 seeds `WindowsSupervisorRuntime.resolved_aipc_allowlist` with `default()` pending a future plan that threads Profile through. Need human confirmation that default-only behavior is acceptable for v2.1 and no demo-breaking regression is shipped.
result: [skipped: profile widening test not executed during the 2026-04-20 UAT cycle (scope was supervisor-pipe regression verification + end-to-end broker smoke; profile widening + revert was out of that scope). Plan 18-03's resolution-allowlist wiring remains unit-tested but not live-tested. See G-06.]

### 4. WR-02 CompareObjectHandles empirical test on EDR-instrumented host

expected: `CompareObjectHandles` returns non-zero for same-object and zero for distinct-object on all supported hosts. No EDR-introduced fail-open observed.
why_human: WR-02 is a latent fail-open concern; empirical verification on a hardened host (CrowdStrike / Defender ATP / EDR with Job Object telemetry hooks) is the only way to know whether it's exploitable in practice.
result: [skipped: no EDR-instrumented host available in the current test environment. Production correctness assumes the MSDN `CompareObjectHandles` contract; empirical verification on a CrowdStrike / Defender ATP / EDR-with-Job-Object-telemetry host is carried forward to v3.0 when an EDR-instrumented test runner is available.]

## Summary

total: 4
passed: 0
issues: 1
partial: 1
pending: 0
skipped: 2
blocked: 0

## Gaps

### G-01. Supervisor control pipe access-denied regression on live `claude-code` flow — RESOLVED 2026-04-20

**Status:** RESOLVED. Root cause and fix documented in `.planning/debug/resolved/supervisor-pipe-access-denied.md`.

**Resolution summary:** Windows 11 26200's WRITE_RESTRICTED second-pass DACL access check requires TWO ACEs, not one: an ACE for a SID in `TokenRestrictedSids` (the per-session restricting SID) AND an ACE for a group SID with `SE_GROUP_MANDATORY` in `TokenGroups` (empirically, `OW` Owner Rights does NOT satisfy this — the logon SID does). Microsoft's `CreateRestrictedToken` documentation does not describe this co-requirement; it was discovered by systematically testing 13 SDDL variants in the new `crates/nono-cli/examples/pipe-repro.rs` harness.

**Three-commit fix chain on `windows-squash`:**
- `3c68377` — `fix(supervisor): grant per-session restricting SID FILE_GENERIC_RW on capability pipe DACL` (necessary but insufficient on Windows 11 26200)
- `938887f` — `fix(supervisor): append logon-SID ACE to capability pipe DACL for WRITE_RESTRICTED access` (the actual fix: appends `(A;;0x0012019F;;;<logon_sid>)` where the logon SID is retrieved at runtime via the new `current_logon_sid()` helper which queries `TokenGroups` for the entry with `SE_GROUP_LOGON_ID`)
- `e4c1bfa` — `fix(aipc-01): initialize Winsock before socket handle reconstruction` (companion fix: Phase 18 Plan 18-04's `reconstruct_socket_from_blob` was missing `WSAStartup` before `WSASocketW(FROM_PROTOCOL_INFO)`; unit tests passed because they called WSAStartup themselves — surfaced by the same end-to-end UAT re-run after the pipe DACL fix unblocked the brokers)

**Verification:** Live `nono run --profile claude-code -- .\target\release\aipc-demo.exe` re-run on 2026-04-20 successfully brokered 4 of 5 AIPC handle types end-to-end (Event, Mutex, Pipe, Socket received duplicated handles in the child process). The 5th broker type (JobObject) failed with a SEPARATE bug (G-03 below), unrelated to supervisor-pipe access-denied.

### G-02. CONIN$ approval prompts render generic fields instead of D-04 per-kind templates

**Discovered:** 2026-04-20 during the post-supervisor-pipe-fix UAT re-run, once Event/Mutex/Pipe/Socket brokers began producing live prompts end-to-end.

**Observed behavior:** Each prompt rendered the SAME generic three-field block regardless of handle kind:

```
Path: <something>
Access: read-only
Reason: demo
```

**Expected behavior (per Plan 18-02 D-04 locked templates):**

- Pipe: `Grant pipe access? name=<n> direction=<read|write|read+write>`
- Socket: `Grant socket access? proto=<tcp|udp> host=<h> port=<p> role=<connect|bind|listen>`
- Event/Mutex: `Grant <event|mutex> access? name=<n> access=<duplicate|modify_state>`

**Impact:** UX regression. The D-04 templates exist in `format_capability_prompt` and are unit-tested; the dispatcher is not routing through them. Matches `18-REVIEW` IN-01's observation that `format!` strings are wired inline at the dispatch site. Security is NOT weakened — approvals still gate the handle duplication — but the approver sees less context than D-04 mandated.

**Next action:** A Phase 18.1 plan should audit the dispatcher's per-kind prompt construction and route ALL live prompts through `format_capability_prompt` (replacing inline `format!` calls). Add end-to-end tests that capture the prompt text via a CONIN$ test fixture or stdin-feeding test harness.

### G-03. JobObject broker fails with `OpenJobObjectW` returning ERROR_FILE_NOT_FOUND

**Discovered:** 2026-04-20 UAT re-run. The JobObject broker path was the 5th (and only failing) AIPC handle type during the end-to-end smoke.

**Observed error:**

```
OpenJobObjectW("Local\nono-aipc-ffcc5b85bf926368-aipc-demo-job") failed: os error 2
```

`os error 2` = `ERROR_FILE_NOT_FOUND`. The broker attempted to OPEN a named JobObject at `Local\nono-aipc-<user_session_id>-aipc-demo-job` that does not exist in the kernel namespace.

**Likely root causes (to investigate):**

1. **Broker should CREATE-if-not-exists:** The Event/Mutex/Pipe/Socket brokers all CREATE their resource if it doesn't exist (semantic: "supervisor creates the named kernel object on demand, brokers a handle into the child"). The JobObject broker appears to only OPEN, which breaks parity.
2. **Demo is misdesigned:** `aipc-demo.exe` may be expected to pre-create the JobObject before requesting brokering, and the current demo binary is skipping that step.
3. **Naming mismatch:** The supervisor computes `Local\nono-aipc-<user_session_id>-<name>` and the demo passes a `name` of `aipc-demo-job`; if the demo creates the JobObject under a different name (e.g. unprefixed `aipc-demo-job` in the default namespace), the supervisor's OpenJobObjectW under `Local\nono-aipc-...-aipc-demo-job` finds nothing.

**Next action:** In Phase 18.1, audit `handle_job_object_request` in the supervisor (commit b440775 / plan 18-03) and compare its CREATE-vs-OPEN posture against the Event/Mutex/Pipe/Socket handlers. Update the broker semantic or update the demo, whichever is the design-intended contract. Add an integration test that exercises the JobObject path end-to-end through the broker.

### G-04. Protocol violation: supervisor returns `granted=true` with `grant=None` on empty ResourceGrant

**Discovered:** 2026-04-20 UAT re-run, as the downstream surfacing of G-03.

**Observed behavior:** After `OpenJobObjectW` failed in the JobObject broker (G-03), the supervisor returned `granted=true` (approval success) but with `grant=None` (empty `ResourceGrant` payload). The child SDK's `send_capability_request` demultiplexer surfaced this as:

```
Error: SandboxInit("supervisor granted but returned no ResourceGrant")
```

**Why this is a protocol violation:** The wire protocol's approval semantic is binary: an "approved" response MUST carry a valid `ResourceGrant` (the duplicated handle or the reconstructed blob); a "denied" response MUST carry a deny reason. Returning `approved=true; grant=None` is neither — the supervisor told the child "you were approved" while failing to actually produce the resource. The child has no recourse except to fail.

**Where the bug likely lives:** In `handle_job_object_request` (the broker handler added in Plan 18-03). The handler probably returns `ApprovalDecision::Approved` at the outer level after the user confirms the prompt, but its internal `broker_job_object_to_process` result (which carries the `OpenJobObjectW` error) is not propagated back to flip the decision to `Denied { reason: ... }`. The Event/Mutex/Pipe/Socket handlers likely have the same latent shape but never hit it because their CREATE-if-not-exists semantics don't fail.

**Not caught by existing tests:** The Phase 18 broker integration test suite exercises the happy path (supervisor creates + brokers successfully). There is no test that exercises "user approves but kernel-object operation fails" — which is what G-03 triggered.

**Next action:** In Phase 18.1:
1. Audit ALL 5 broker handlers (Event, Mutex, Pipe, Socket, JobObject) for this propagation bug. If the kernel object operation (CreateEvent, OpenJobObject, WSADuplicateSocket, etc.) returns an error AFTER the user has approved, the handler MUST flip `ApprovalDecision::Approved` → `ApprovalDecision::Denied { reason: "broker failed: <error>" }` rather than returning an empty grant.
2. Add integration tests that inject a failure at the broker step (e.g. via a test seam that pre-deletes or pre-locks the named object) and assert the response is `Denied { reason: ... }`, not `Approved { grant: None }`.
3. Consider tightening the wire-protocol types to make `Approved` structurally carry a non-optional `ResourceGrant` (so this bug is a compile error rather than a runtime violation).

### G-05. WR-01 reject-BEFORE-prompt vs reject-AFTER-prompt invariant not empirically verified

**Discovered:** 2026-04-20 UAT re-run.

**Expected:** Per the Plan 18-01 summary invariant, Event/Mutex/JobObject rejections happen BEFORE the CONIN$ prompt is shown (mask-validation gate); per WR-01, Pipe/Socket rejections currently happen AFTER the prompt (the rejection is evaluated post-approval).

**What was exercised:** All 5 broker requests during the 2026-04-20 UAT run prompted normally and were approved by the user. No explicit deny-stage scenarios (malformed mask, out-of-allowlist Pipe/Socket target, mask-widening beyond D-04 default) were attempted. The UX-inconsistency concern WR-01 flagged remains un-verified empirically.

**Next action:** In Phase 18.1, construct scripted deny scenarios:
1. Event/Mutex with a mask outside DEFAULT_MASK → expect rejection WITHOUT CONIN$ prompt (Phase 18-01 invariant).
2. JobObject with a mask outside DEFAULT_MASK → expect rejection WITHOUT CONIN$ prompt.
3. Pipe with `direction=read+write` → expect rejection AFTER CONIN$ prompt (WR-01 current behavior, pending future fix).
4. Socket with privileged port (`port <= 1023`) → expect rejection AFTER CONIN$ prompt (same WR-01 stage issue).

Document whether the WR-01 UX inconsistency is acceptable for v2.1 or warrants a follow-up fix.

### G-06. capabilities.aipc profile widening end-to-end not live-tested

**Discovered:** 2026-04-20 UAT re-run scope decision.

**Expected:** Plan 18-03 Deferred Issues #1 documents that `WindowsSupervisorRuntime.resolved_aipc_allowlist` is seeded with `default()` pending a future plan that threads `Profile` through the dispatcher initialization. The UNION semantic of profile widening + default allowlist is unit-tested in `profile::tests` but no end-to-end test exercises it through a real `nono run --profile <widened>` invocation.

**What was exercised:** The 2026-04-20 UAT focused on supervisor-pipe regression closure + end-to-end broker smoke with the stock `claude-code` profile. Profile widening was out of the re-run's scope.

**Next action:** In Phase 18.1 (or a dedicated plan), construct a test profile that widens the AIPC allowlist (e.g. permits `ReadWrite` on a Pipe name the default allowlist rejects, or permits a Socket port in the unprivileged range the default denies) and verify:
1. `nono run --profile <widened>` allows the widened capability request to reach the prompt AND be approved.
2. `nono run --profile <default>` against the same capability request rejects at the resolution-allowlist gate BEFORE the prompt.
3. Removing the widening from the profile and re-running returns to default behavior (no stale caching).
