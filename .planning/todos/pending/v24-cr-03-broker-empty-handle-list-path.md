---
title: "Document or fix broker's empty --inherit-handle list path (CR-03 from Phase 31 review)"
created: 2026-05-09
source: .planning/phases/31-broker-process-architecture-shell-01/31-REVIEW.md (CR-03)
target_milestone: v2.4
priority: low
---

# CR-03: Empty --inherit-handle list path likely returns ERROR_BAD_LENGTH

Plan 31-02 SUMMARY documented an "empty `--inherit-handle` list = most-restrictive (no handles inherited)" code path in `nono-shell-broker`. Per Win32 docs, `UpdateProcThreadAttribute(PROC_THREAD_ATTRIBUTE_HANDLE_LIST, lpValue=ptr, cbSize=0)` returns `ERROR_BAD_LENGTH` — the API requires at least one handle in the list.

**Verifier note (from `31-VERIFICATION.md`):** This path is **structurally unreachable from production** — `crates/nono-cli/src/exec_strategy_windows/launch.rs:1379-1382` always emits both `pty_pair.input_write` and `pty_pair.output_read` as `--inherit-handle` flags. The Plan 31-05 Job Object test asserts JobObject membership BEFORE `ResumeThread` fires, so the broken `UpdateProcThreadAttribute(cbSize=0)` call never executes.

**Suggested fix (one of):**
- (a) Update Plan 31-02 SUMMARY to remove the misleading "empty list = most-restrictive" claim.
- (b) Add a guard in the broker: if `--inherit-handle` count is 0, skip the entire `STARTUPINFOEXW`/`PROC_THREAD_ATTRIBUTE_HANDLE_LIST` setup and use plain `STARTUPINFOW` (default-inherit, which is what an empty list semantically would mean if it worked).
- (c) Document the empty-list case as an unsupported argv shape and reject it in the parser.

**Source:** Phase 31 code review report `31-REVIEW.md` finding CR-03.
**Impact:** Documentation accuracy. Production code unaffected.
**Acceptance gate:** Pick one of (a)/(b)/(c); apply; verify the broker still works end-to-end.
