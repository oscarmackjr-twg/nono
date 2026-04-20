---
status: partial
phase: 18-extended-ipc
source: [18-VERIFICATION.md]
started: 2026-04-19T00:00:00Z
updated: 2026-04-19T00:00:00Z
---

## Current Test

[awaiting human testing on Windows host]

## Tests

### 1. CONIN$ approval prompt renders D-04-locked per-kind templates

expected: Each prompt shows the correct kind-specific fields (e.g. `proto=tcp host=... port=... role=connect` for socket) and approval grants a live handle; denial produces `grant=None` audit entry.
why_human: IN-01 in 18-REVIEW notes the dispatcher still wires `format!` strings inline rather than routing through `format_capability_prompt`; only tests consume the helper. Live CONIN$ text must be eyeballed on a real terminal to confirm UX integrity end-to-end.
result: [pending]

### 2. WR-01 UX impact assessment — Pipe/Socket pre-approval gate

expected: Per the 18-01 summary invariant, Event/Mutex/JobObject reject BEFORE prompt; per WR-01, Pipe/Socket currently reject AFTER prompt. Confirm whether the UX impact is acceptable for v2.1 or requires a follow-up fix.
why_human: The reviewer flagged this as UX inconsistency rather than a security hole — a product decision is required on whether to accept the deviation or schedule a follow-up.
result: [pending]

### 3. capabilities.aipc profile widening end-to-end

expected: Widening the profile grants ReadWrite; removing the widening enforces default read-OR-write (not both). The UNION semantic is tested in profile::tests but end-to-end dispatcher consumption of the loaded profile's `resolve_aipc_allowlist` is deferred (18-03 Deferred Issues #1).
why_human: Plan 18-03 seeds `WindowsSupervisorRuntime.resolved_aipc_allowlist` with `default()` pending a future plan that threads Profile through. Need human confirmation that default-only behavior is acceptable for v2.1 and no demo-breaking regression is shipped.
result: [pending]

### 4. WR-02 CompareObjectHandles empirical test on EDR-instrumented host

expected: `CompareObjectHandles` returns non-zero for same-object and zero for distinct-object on all supported hosts. No EDR-introduced fail-open observed.
why_human: WR-02 is a latent fail-open concern; empirical verification on a hardened host (CrowdStrike / Defender ATP / EDR with Job Object telemetry hooks) is the only way to know whether it's exploitable in practice.
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps
