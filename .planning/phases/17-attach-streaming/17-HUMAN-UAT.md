---
status: partial
phase: 17-attach-streaming
source: [17-VERIFICATION.md]
started: 2026-04-19T18:09:50Z
updated: 2026-04-19T18:09:50Z
---

# Phase 17 — Human Verification (deferred-by-design)

These items were intentionally deferred by the user under the **pragmatic-PASS verdict** recorded in `17-02-SUMMARY.md § Smoke gate`. They are NOT gaps in implementation — the user-visible promise of ATCH-01 was demonstrably met during the 2026-04-19 smoke gate. These items round out the audit record on a properly-provisioned Windows host and can be closed via `/gsd-verify-work` at the user's convenience.

## Current Test

[awaiting human testing]

## Tests

### 1. G-02 bidirectional stdin echo round-trip
expected: Type `echo BIDIRECTIONAL_OK<Enter>` into `nono attach <id>` against detached `cmd.exe`; see `BIDIRECTIONAL_OK` echoed back from the child
why_human: Smoke-gate session 2026-04-19 proved stdout half + supervisor control plane (`nono stop` graceful shutdown) but did not type stdin. Pragmatic-PASS recorded by user; routing here for explicit closure on a future Windows session.
result: [pending]

### 2. G-03 detach + re-attach round-trip
expected: Press Ctrl-]d during counter stream → `nono ps` shows session still RUNNING → re-attach with `nono attach <id>` → see counter resume from a higher number; second Ctrl-]d disconnects cleanly
why_human: Smoke-gate session 2026-04-19 proved live counter streaming + attach banner + scrollback replay but did not exercise the Ctrl-]d → ps → re-attach loop. `active_attachment: Mutex<Option<...>>` lifecycle is unchanged from Phase 15 design and structurally supports this; pragmatic-PASS recorded by user.
result: [pending]

### 3. G-04 Row 4 (`--block-net` detached) on a host with WFP driver registered
expected: `nono run --detached --block-net --allow-cwd -- cmd /c "curl --max-time 5 http://example.com"` succeeds in spawning the detached child, and the child's outbound HTTP request is blocked by WFP (curl reports connect failure or timeout)
why_human: Smoke-gate session 2026-04-19 returned the fail-secure WFP error because `nono-wfp-driver` is not registered on this host — this is fail-closed-as-designed and NOT a Phase 17 regression. Re-run on a host where `nono setup --install-wfp-driver` has been executed to confirm Phase 15 baseline preserved.
result: [pending]

### 4. G-04 Rows 1, 2, 5 explicit re-run
expected: Row 1 detached banner shape unchanged from `15-02-SUMMARY.md`; Row 2 fast-exit `nono run --detached -- cmd /c exit 0` clean exit; Row 5 `nono logs` / `nono inspect` / `nono prune` output shapes unchanged
why_human: Not explicitly re-run in 2026-04-19 smoke gate — structurally PASS by code-reading (no Phase 17 changes to banner code paths, fast-exit detection in `startup_runtime::run_detached_launch`, or `SessionRecord` schema), but a future operator should re-run for the audit record.
result: [pending]

## Summary

```
total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0
```

## Gaps

(none — items are deferred-by-design, not implementation defects)
