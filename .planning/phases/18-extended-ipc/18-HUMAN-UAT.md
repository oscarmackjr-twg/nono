---
status: complete-with-issues
phase: 18-extended-ipc
source: [18-VERIFICATION.md]
started: 2026-04-19T00:00:00Z
updated: 2026-04-20T00:00:00Z
---

## Current Test

[COMPLETE-WITH-ISSUES — Phase 21 shipped 2026-04-20; all 4 items carry forward as issues pending /gsd-debug on supervisor pipe access-denied regression]

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
result: [issue: supervisor control pipe access denied — pending /gsd-debug investigation]

### 2. WR-01 UX impact assessment — Pipe/Socket pre-approval gate

expected: Per the 18-01 summary invariant, Event/Mutex/JobObject reject BEFORE prompt; per WR-01, Pipe/Socket currently reject AFTER prompt. Confirm whether the UX impact is acceptable for v2.1 or requires a follow-up fix.
why_human: The reviewer flagged this as UX inconsistency rather than a security hole — a product decision is required on whether to accept the deviation or schedule a follow-up.
result: [issue: supervisor control pipe access denied — pending /gsd-debug investigation]

### 3. capabilities.aipc profile widening end-to-end

expected: Widening the profile grants ReadWrite; removing the widening enforces default read-OR-write (not both). The UNION semantic is tested in profile::tests but end-to-end dispatcher consumption of the loaded profile's `resolve_aipc_allowlist` is deferred (18-03 Deferred Issues #1).
why_human: Plan 18-03 seeds `WindowsSupervisorRuntime.resolved_aipc_allowlist` with `default()` pending a future plan that threads Profile through. Need human confirmation that default-only behavior is acceptable for v2.1 and no demo-breaking regression is shipped.
result: [issue: supervisor control pipe access denied — pending /gsd-debug investigation]

### 4. WR-02 CompareObjectHandles empirical test on EDR-instrumented host

expected: `CompareObjectHandles` returns non-zero for same-object and zero for distinct-object on all supported hosts. No EDR-introduced fail-open observed.
why_human: WR-02 is a latent fail-open concern; empirical verification on a hardened host (CrowdStrike / Defender ATP / EDR with Job Object telemetry hooks) is the only way to know whether it's exploitable in practice.
result: [issue: supervisor control pipe access denied — pending /gsd-debug investigation]

## Summary

total: 4
passed: 0
issues: 4
pending: 0
skipped: 0
blocked: 0

## Gaps

### G-01. Supervisor control pipe access-denied regression on live `claude-code` flow

**Discovered:** 2026-04-20 during Phase 21 Plan 21-05 Task 2 HUMAN-UAT re-run, after Phase 21's label primitive + inline ownership-skip fix (`da25619`) landed on `windows-squash`.

**Reproduction:**
```
nono run --profile claude-code -- aipc-demo.exe
```
Produces:
```
Error: SandboxInit("Failed to connect to Windows supervisor pipe
\\\\.\\pipe\\nono-nono-cap-df52a0cb9818081e-pipe-... Access is denied. (os error 5).
Ensure the supervisor created the control channel before launching the child.")
```

**First-observed commits:**
- `da25619` — `fix(21-03): skip mandatory-label apply for paths not owned by current user`
- Phase 21 label-apply landing on `windows-squash` (Wave 2 merge chain: `9ca07c4` + `131c35b` + `d5e6d33`)

**Impact:** All 4 UAT items above cannot be verified end-to-end on Windows until this is root-caused. Library-level and integration-level Phase 21 coverage is fully green (76 `sandbox::windows` tests pass, including the 5 new Plan 21-05 tests); this regression only surfaces in the live `claude-code → supervised → aipc-demo` flow.

**Candidate hypotheses** (to investigate in /gsd-debug session):
1. **AppliedLabelsGuard side-effect on staging dirs:** `.cache\claude` / `.claude` paths get `NO_EXECUTE_UP` (mask 0x4, ReadWrite mode) via the label apply loop, breaking execute access on supervisor handoff binaries/scripts staged there.
2. **Phase 11 `CAPABILITY_PIPE_SDDL` DACL gap:** the SDDL admits mandatory label Low via SACL (`S:(ML;;NW;;;LW)`) but may lack a DACL ACE granting connect/read/write to Low-IL subjects. This is the first time a Low-IL child actually tries to connect to the capability pipe end-to-end on Windows.
3. **Silent supervisor startup failure:** the supervisor may be failing before `CreateNamedPipeW` runs, so the child sees the pipe connect fail with ACCESS_DENIED rather than FILE_NOT_FOUND.

**Next action:** Spawn `/gsd-debug` session to isolate which of the three hypotheses (or another root cause) is responsible. Route fix to a quick-task or Phase 22 once the issue is scoped.

**Not a Phase 21 blocker:** Phase 21's library goal (per-file Read/Write/ReadWrite labeling, write-only directory labeling, fail-closed on error, `WindowsUnsupportedIssueKind` shape preserved per D-06) has shipped on disk and passes its full test suite. This gap blocks the live-CONIN$ UAT verification but not the primitive itself.
