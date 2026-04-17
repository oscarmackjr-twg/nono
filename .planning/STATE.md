---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: Human Verification UAT
status: executing
stopped_at: Completed 12-03-PLAN.md verification — CI gate FAILS on pre-existing clippy errors (not Phase 12 regression); follow-up plan required before Phase 13
last_updated: "2026-04-12T22:10:06.056Z"
last_activity: 2026-04-12
progress:
  total_phases: 14
  completed_phases: 11
  total_plans: 35
  completed_plans: 31
  percent: 89
---

# Project State: nono - Windows Gap Closure

## Project Reference

**Core Value:** Every nono command that works on Linux/macOS should work on Windows with equivalent security guarantees, or be explicitly documented as intentionally unsupported with a clear rationale.

**Current Focus:** Phase 13 — v1-human-verification-uat

## Current Position

Phase: 13 (v1-human-verification-uat) — EXECUTING
Plan: 1 of 1
Status: Executing Phase 13
Last activity: 2026-04-17 -- Phase 13 UAT partial pass (2/10) + Phase 14 (v1.0 Fix Pass) added to close 3 blocking gaps

```
Progress: [██████████] 100% (31/31 plans complete on disk)
```

## Accumulated Context

### Key Decisions (carried from v1.0)

- **Supervisor-Broker Pattern:** Research confirms this is the only way to manage elevated tasks like WFP while maintaining user-level CLI (2026-04-04).
- **WFP as Primary Network Backend:** Moving away from temporary firewall rules for true kernel-level enforcement (2026-04-04).
- **Named Job Objects:** Chosen for agent lifecycle management to ensure atomic stop/list capabilities (2026-04-04).
- **SID-Based Filtering:** Prioritized over App-ID to ensure child processes inherit network restrictions (2026-04-04).
- **Double-Launch Strategy:** Used `DETACHED_PROCESS` to decouple the supervisor from the parent terminal (2026-04-04).
- **Restricted Tokens:** Used to apply the session-unique SID to the process tree (2026-04-04).
- **RFC 3161 Timestamping:** Upgraded from legacy /t to /tr + /td sha256 (2026-04-05).
- **WFP Startup Orphan Sweep:** Enumerates NONO_SUBLAYER_GUID filters and removes stale ones at startup (2026-04-05).
- **Machine MSI Owns EventLog Registration:** SYSTEM\CurrentControlSet\Services\EventLog\Application\nono-wfp-service (2026-04-05).
- **MSRV 1.77:** Bumped from 1.74 to align with windows-sys 0.59 (2026-04-05).
- **WaitNamedPipeW Readiness Probe:** run_detached_launch() uses WaitNamedPipeW(50ms) per iteration on Windows (2026-04-05).
- **Single SID Generation Point:** Session SID generated once at ExecConfig construction (2026-04-06).
- **Driver Gate Removed:** activate_policy_mode no longer checks for a kernel driver binary artifact (2026-04-06).

### Key Decisions (v2.0)

- **Phase ordering A→B:** Phase 7 validates the entry-point guard removal pattern before Phase 8 layers ConPTY complexity on top (2026-04-06).
- **Phases 9, 10, 11 are independent:** Can be planned and executed in any order relative to each other and to Phase 7/8 (2026-04-06).
- **Single IPC version bump for Phase 9:** Gaps 4 (proxy) and 5 (port filtering) grouped in one phase to avoid two separate `nono-wfp-service` deployments (2026-04-06).
- **ETW library decision deferred to Phase 10 plan 10-01:** `ferrisetw` vs direct `windows-sys` bindings must be evaluated and documented before any ETW code is written (2026-04-06).
- **Gap 6b deferred to v3.0:** Kernel minifilter driver required; no user-mode workaround acceptable (2026-04-06).
- **Minimum build for ConPTY and ETW:** Windows 10 build 17763 (1809); enforced via `RtlGetVersion` at runtime; no silent fallback (2026-04-06).
- **Anonymous Job Object for wrap:** Pass `None` to `execute_direct` for Direct strategy; empty session_id would produce malformed Job Object name `Local\nono-session-` (2026-04-08).
- **nono wrap available on Windows:** Direct strategy with Job Object + WFP enforcement; documented with "no exec-replace, unlike Unix" qualifier per WRAP-01 (2026-04-08).
- **Phase 09 unreachable!() scoped to Unix:** On Windows, execute_direct returns Ok(i32); unreachable!() moved inside cfg(not(windows)) block; Windows Direct branch captures exit code and calls std::process::exit(exit_code) (2026-04-10).
- **Phase 09 stale test replaced:** apply_rejects_unsupported_proxy_with_ports removed; apply_accepts_port_level_wfp_caps asserts Ok(()) for port-level caps post-Phase-09 semantics (2026-04-10).
- **Phase 12-03 STOP on pre-existing CI failure:** `make ci` fallback surfaced 48 `disallowed_methods` clippy errors in `profile/mod.rs`, `config/mod.rs`, `sandbox_state.rs`. Root-caused to revert `cf5a60a` (2026-04-10), predates Phase 12. Phase 12's own files (`crates/nono/src/sandbox/windows.rs`, `crates/nono-cli/tests/wfp_port_integration.rs`) produce zero clippy diagnostics. Did NOT auto-fix per plan STOP directive (2026-04-11).

### Roadmap Evolution

- 2026-04-17: Phase 14 (v1.0 Fix Pass) added after Phase 13 UAT surfaced three blocking gaps — detached console-child STATUS_DLL_INIT_FAILED (blocks 4 UAT items), setup help-text drift (blocks P07-HV-2), P09-HV-1 runbook flag bug. Phase 14 plans: 3 (one per gap; plan 03 also re-runs the blocked UAT items and finishes Phase 13 Task 3 upstream promotion).

### Research Flags (open)

- **Phase 10 (10-01):** ETW library decision (`ferrisetw` vs `windows-sys` direct) must be resolved before any ETW code is written. Check `ferrisetw` crates.io for current version and open issues at 10-01 start.
- **Phase 10 (10-01):** Verify `Win32_System_Diagnostics_Etw` feature flag in `windows-sys 0.59` compiles cleanly before committing to the implementation approach.
- **Phase 11 (11-01):** Read `crates/nono/src/supervisor/socket_windows.rs` `create_named_pipe` SDDL before planning. If `S:(ML;;NW;;;LW)` is absent, 11-01 must add it; this changes scope.

### Todos

- [ ] Discuss Phase 4 filesystem strategy (VSS vs Merkle Trees)
- [ ] Phase 09 human verification: proxy E2E (HTTPS_PROXY in child env) — requires Windows host + live proxy config
- [ ] Phase 09 human verification: SC5 WFP TCP test (`cargo test -p nono-cli --test wfp_port_integration -- --ignored`) — requires Windows host + admin + nono-wfp-service running

### Blockers

(none)

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 260405-v0e | investigate and fix exec_strategy.rs uncommitted changes | 2026-04-06 | b6e20e4 | [260405-v0e-investigate-and-fix-exec-strategy-rs-unc](./quick/260405-v0e-investigate-and-fix-exec-strategy-rs-unc/) |
| 260405-vjj | Fix PR 555 DCO signoffs, commit PR 583 review feedback fixes, push current changes | 2026-04-06 | 4880c03 | [260405-vjj-fix-pr-555-signoffs-and-merge-conflicts-](./quick/260405-vjj-fix-pr-555-signoffs-and-merge-conflicts-/) |
| 260406-ajy | Assess Windows functional equivalence to macOS and Linux | 2026-04-06 | — | [260406-ajy-assess-windows-functional-equivalence-to](./quick/260406-ajy-assess-windows-functional-equivalence-to/) |
| 260406-bem | Research Windows gaps and create WINDOWS-V2-ROADMAP.md | 2026-04-06 | b67f74a | [260406-bem-research-and-roadmap-windows-gap-closure](./quick/260406-bem-research-and-roadmap-windows-gap-closure/) |
| 260417-kem | Fix EnvVarGuard migration - migrate 48 flagged tests to EnvVarGuard | 2026-04-17 | 6749494 | [260417-kem-fix-envvarguard-migration-migrate-48-fla](./quick/260417-kem-fix-envvarguard-migration-migrate-48-fla/) |
| 260417-wla | Fix Windows CreateProcess ERROR_INVALID_HANDLE from temp-drop use-after-close in spawn_windows_child | 2026-04-17 | eb4730c | [260417-wla-fix-windows-createprocess-handle-uaf](./quick/260417-wla-fix-windows-createprocess-handle-uaf/) |

## Session Continuity

**Current Milestone:** v2.0 Windows Gap Closure
**Last Activity:** 2026-04-17
**Stopped At:** Completed quick task 260417-kem (EnvVarGuard migration) — CI blocker resolved, clippy passes clean
**Next Steps:** Phase 08 (ConPTY Shell) or Phase 10 (ETW-Based Learn Command) — both are plannable now. Phase 09 human-verification items noted in Todos above.
