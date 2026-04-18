---
phase: 15-detached-console-conpty-investigation
status: not-planned
created: 2026-04-18
---

# Phase 15: Detached Console + ConPTY Architecture Investigation

This phase is the follow-up to Phase 14 plan 14-01's escalation. See:

- `.planning/phases/14-v1-fix-pass/14-01-SUMMARY.md` — full post-mortem of the three attempted fix directions (1: TokenGroups infeasible; 3: AllocConsole failed smoke; 2: null-token-with-PTY failed smoke) and the debug-doc matrix that pinpoints `null token + no PTY` as the only working configuration.
- `.planning/phases/14-v1-fix-pass/14-01-PLAN.md` `<direction_decision>` — the Direction 1 infeasibility analysis (why `AdjustTokenGroups` and `SetTokenInformation(TokenGroups)` cannot add synthetic SIDs from user-mode).
- `.planning/debug/windows-supervised-exec-cascade.md` — the original Bug #3 debug session, including the full token × PTY × detached test matrix.

## What needs to happen

1. **Plan this phase**: `/gsd-plan-phase 15`. Expected to produce 1-2 plans covering:
   - Deeper investigation: pin down which ConPTY / `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` interaction with `DETACHED_PROCESS` triggers the `0xC0000142`.
   - Implementation: either (a) a detached-compatible ConPTY setup, or (b) a gated PTY-disable path with pipe-based stdio redirection for `nono attach`.

2. **Execute**: run the plans through the standard GSD flow. The 4-row smoke-gate matrix from `14-01-PLAN.md <smoke_test_gate>` is the acceptance test — re-use it verbatim.

3. **Close**: once the smoke gate passes, resume Phase 14 (plan 14-03) to finish the v1.0 Fix Pass. The 4 Phase 13 UAT items that depended on 14-01 (P05-HV-1, P07-HV-3, P11-HV-1, P11-HV-3) get their second UAT pass under the new Phase 15 fix and can then be promoted to terminal verdicts.

## Related code touchpoints (starting map)

- `crates/nono-cli/src/exec_strategy_windows/launch.rs::spawn_windows_child` — where the token + PTY + `CreateProcessAsUserW` happens.
- `crates/nono-cli/src/exec_strategy_windows/mod.rs::execute_supervised` — passes `runtime.pty()` to `spawn_windows_child`.
- `crates/nono-cli/src/supervised_runtime.rs` / `pty_proxy_windows.rs` — the PTY pair allocation. Gating this on detached mode is the (b)-path lever.
- `crates/nono-cli/src/startup_runtime.rs::run_detached_launch` — where `DETACHED_PROCESS` is set on the supervisor and `NONO_DETACHED_LAUNCH=1` is injected into its env.
- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs` — left at the Bug #2 WRITE_RESTRICTED fix; do NOT need to change unless the solution involves a new token shape.

## Open v1.0 shipping decision

Phase 14 is paused. Before Phase 15 completes, the user should decide whether v1.0 ships with the `0xC0000142` detached-console-grandchild bug as a **documented known issue** (4 Phase 13 UAT items carry-forward), or whether v1.0 waits for Phase 15. This decision is called out in `14-01-SUMMARY.md` but is recorded here for visibility.
