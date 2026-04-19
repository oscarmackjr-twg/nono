---
gsd_state_version: 1.0
milestone: v2.1
milestone_name: Resource Limits, Extended IPC, Attach-Streaming & Cleanup
status: Phase 19 complete (verifier passed 25/25 must-haves); v2.1 remaining: Phase 17 (ATCH-01) and Phase 18 (AIPC-01)
stopped_at: Phase 19 closed 2026-04-19 — verifier commit `6597fbf`; ready to plan Phase 17 or 18
last_updated: "2026-04-19T02:30:00.000Z"
last_activity: 2026-04-19 — Phase 19 (cleanup) complete. All 4 plans shipped, verifier agent passed 25/25 must-haves (commit `6597fbf`). v2.1 now at 15/17 phases complete on disk; milestone remaining = ATCH-01 + AIPC-01.
progress:
  total_phases: 19
  completed_phases: 15
  total_plans: 44
  completed_plans: 44
  percent: 100
---

# Project State: nono — v2.1 (Resource Limits, Extended IPC, Attach-Streaming & Cleanup)

## Project Reference

**Core Value:** Every nono command that works on Linux/macOS should work on Windows with equivalent security guarantees, or be explicitly documented as intentionally unsupported with a clear rationale.

**Current Focus:** Phase 16 + Phase 19 complete. Next: Phase 17 (ATCH-01) or Phase 18 (AIPC-01) — independent; either closes v2.1.

## Current Position

Phase: 19 (cleanup) — COMPLETE 2026-04-19 (4/4 plans; verifier commit `6597fbf` passed 25/25 must-haves)
Plan: — (phase closed; no active plan)
Milestone: v2.1 — 15/17 phases complete. Phases 17 (ATCH-01) and 18 (AIPC-01) remain before milestone ships.

  - v1.0 Windows Alpha — shipped 2026-03-31 (tag `v1.0`).
  - v2.0 Windows Gap Closure — shipped 2026-04-18 (tag `v2.0` pending on merge). Carry-forward closed by Phase 15 the same day.
  - v2.1 — started 2026-04-18, this milestone.

v2.1 phase structure (target — will be finalized by `/gsd-plan-phase`):

  - Phase 16: Resource Limits (RESL-01..04) — Job Object CPU/memory/timeout/process-count caps.
  - Phase 17: Attach-Streaming (ATCH-01) — full ConPTY re-attach on detached Windows sessions.
  - Phase 18: Extended IPC (AIPC-01) — broker additional handle types over Phase 11 cap pipe.
  - Phase 19: Cleanup (CLEAN-01..04) — fmt drift, Windows test flakes, WIP triage, session-file housekeeping.

Phases 1–15 complete on disk (see `.planning/ROADMAP.md` progress table).

Next actions:

  - `/gsd-plan-phase 16` to draft the RESL phase plan.
  - Or `/gsd-discuss-phase 16` first if the Job Object API choices need more scoping.
  - Phase ordering is sticky-to-writer preference: RESL first (most self-contained), ATCH second (finishes Phase 15 story), AIPC third (extends Phase 11 after RESL's Job Object work lands useful abstractions), CLEAN last (catches any fmt/test drift introduced by the feature phases).

Naming note: phase directories `13-v1-human-verification-uat/` and `14-v1-fix-pass/` retain v1-era naming — v2.0/v2.1 is the formal milestone sequence per PROJECT.md/REQUIREMENTS.md.

Last activity: 2026-04-18 -- Phase 19 plan 19-04 (CLEAN-04 session retention + prune CLI extensions + auto-prune on `nono ps` + T-19-04-07 NONO_CAP_FILE structural no-op + one-shot cleanup of 1343 stale session files + user-facing docs) complete; 5 DCO-signed commits (18e9768, a71b2bf, c3defb6, ddf408b, f626e24) + bookkeeping

```
Progress: [████████░░]  80% (8/10 v2.1 requirements validated — RESL-01..04, CLEAN-01..04 shipped; ATCH-01 + AIPC-01 remain)
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

### Key Decisions (v2.1)

- **Phase 19 CLEAN-01 fmt-only commit:** Single `style(19-CLEAN-01):` commit on 3 files (`config/mod.rs`, `restricted_token.rs`, `profile/mod.rs`), no logic changes. `cargo fmt --all -- --check` is now green on the whole workspace (2026-04-18).
- **Phase 19 CLEAN-01 smoke deliberately excludes `make ci`:** CLEAN-02's 5 pending test flakes would keep `make test` red for reasons unrelated to fmt; CLEAN-01 smoke is restricted to `cargo fmt --all -- --check` as specified by the plan (2026-04-18).
- **Phase 19 CLEAN-02 hypothesis D-07 (parallel env-var contamination) was wrong:** diagnosis revealed all 5 tests were deterministic Windows platform bugs of 4 distinct flavors — JSON escape when embedding backslash paths, non-absolute Unix-shaped XDG paths in env guards, UNC `\\?\` prefix mismatch between canonicalized and policy-expanded paths (a genuine production bug in `query_path`), and a Unix-only path literal colliding with a correct production `debug_assert!(path.is_absolute())`. None needed `lock_env() + EnvVarGuard`; each fix was file-local and cfg-gated or helper-routed (2026-04-18).
- **Phase 19 CLEAN-02 option C scope expansion (deviation D-08):** user-approved in-place expansion to land a minimal production fix (UNC-prefix strip in `query_path` + new `#[cfg(windows)]` regression test) alongside the 4 test-only fixes. Production fix is scoped to a single call site with a local helper; no new dependency, no broader normalization refactor. Rationale: fixing the test without fixing the underlying under-reporting of sensitive-path denials for non-existent paths on Windows would have papered over a real bug in `nono why ~/.ssh` (2026-04-18).
- **Phase 19 CLEAN-02 D-06 scope boundary strictly honored:** `tests/env_vars.rs` integration failures (19) and `trust_scan::tests::*` tempdir-race flakes (1–3) exist pre- and post-fix on this Windows host but are NOT in D-06's 5-test scope and were NOT fixed in this plan. Documented in 19-02-SUMMARY § Deferred Issues for potential future cleanup (2026-04-18).
- **Phase 19 CLEAN-03 per-file disposition review:** 10 disk-resident WIP items from D-12 resolved via user-approved disposition table — 6 committed alive (`10-RESEARCH.md`, `10-UAT.md`, quick-260410-nlt PLAN, quick-260412-ajy directory's 7 files, `v1.0-INTEGRATION-REPORT.md`), 2 reverted to HEAD (11-01-PLAN.md, 11-02-PLAN.md working-tree drift), 2 deleted (`host.nono_binary.commit` and `query` debug crumbs + untracked 12-02-PLAN.md reconstruction). Two new root-anchored `.gitignore` patterns prevent recurrence of WFP-service debug crumbs. No production code touched; commits a208761, a4100aa, db4547b, 0391e37, d49fda8, d6bf88f (2026-04-18).
- **Phase 19 CLEAN-04 retention threshold + breaking change + T-19-04-07 mitigation:** Auto-sweep threshold = 100 stale files (compile-time constant, within CONTEXT.md's 50–500 range). `--older-than <DURATION>` with require-suffix parser is a deliberate breaking change from the prior integer-days form — silently interpreting `30` as seconds vs days would be a footgun; scripts now get a migration hint. `--all-exited` is an explicit escape hatch, clap-enforced mutually-exclusive with `--older-than`. T-19-04-07 mitigation is the structural early-return `if env::var_os("NONO_CAP_FILE").is_some() { return; }` as the first statement of `auto_prune_if_needed` — a sandboxed agent calling `nono ps` cannot trigger host-side session-file deletion. Paired with unit test `auto_prune_is_noop_when_sandboxed`. One-shot cleanup on this host: BEFORE=1392, AFTER=49, DELTA=1343. Commits 18e9768, a71b2bf, c3defb6, ddf408b, f626e24 (2026-04-18).

### Roadmap Evolution

- 2026-04-19: Phase 20 (Upstream Parity Sync, UPST) added after Phase 19 completion. Research quick-task `260419-cmp-upstream-036-windows-parity/COMPARISON.md` (commit `7180f23`) established the fork is pinned at crate version `0.30.1` while upstream has shipped 0.31–0.37.1. Phase 20 back-ports missing Unix/macOS functionality (keyring URIs, `--allow-gpu`, GitLab trust tokens, macOS Seatbelt refinements) and, critically, the rustls-webpki RUSTSEC-2026-0098/0099 security upgrade landed upstream in 0.37. Plans TBD during `/gsd-plan-phase 20` — likely grouped by upstream version range + dedicated security-upgrade plan.
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

**Current Milestone:** v2.1 — Resource Limits, Extended IPC, Attach-Streaming & Cleanup
**Last Activity:** 2026-04-18 — Phase 19 plan 19-04 (CLEAN-04 session retention + prune CLI extensions + auto-prune on `nono ps` + T-19-04-07 NONO_CAP_FILE structural no-op + one-shot cleanup of 1343 stale session files + user-facing docs) complete; 5 DCO-signed commits (`18e9768`, `a71b2bf`, `c3defb6`, `ddf408b`, `f626e24`) + bookkeeping on `windows-squash`
**Stopped At:** Phase 19 plans 19-01 + 19-02 + 19-03 + 19-04 all complete on disk; awaiting verifier agent before marking phase itself complete
**Next Steps:** Orchestrator runs the Phase 19 verifier agent to close Phase 19. Then pivot to `/gsd-plan-phase 17` (ATCH-01) or `/gsd-plan-phase 18` (AIPC-01) — both independent feature phases remain for v2.1.

**Status of Phase 19 CLEAN items:**

- CLEAN-01 — COMPLETE (commit `c87b10b`, 2026-04-18). `cargo fmt --all -- --check` exits 0 on whole workspace.
- CLEAN-02 — COMPLETE-WITH-DEVIATION (commits `400f8c9`, `8412fda`, `a449454`, `4db849d`, 2026-04-18). All 5 D-06 tests now pass deterministically; 1 `#[cfg(windows)]` regression test added for the UNC-prefix production fix in `query_path`. Hypothesis D-07 contradicted: real root causes were 4 distinct deterministic Windows platform bugs, not parallel env-var contamination. Deviation D-08 tripped and user-approved (option C). Pre-existing `tests/env_vars.rs` (19) and `trust_scan::tests::*` (1–3) failures are NOT in D-06 scope and remain deferred.
- CLEAN-03 — COMPLETE (commits `a208761`, `a4100aa`, `db4547b`, `0391e37`, `d49fda8`, `d6bf88f`, 2026-04-18). All 10 disk-resident WIP items from D-12 resolved via user-approved disposition table: 6 backfilled alive (`10-RESEARCH.md`, `10-UAT.md`, quick-260410-nlt PLAN, quick-260412-ajy 7-file directory, `v1.0-INTEGRATION-REPORT.md`); 2 reverted to HEAD (11-01-PLAN.md, 11-02-PLAN.md); 2 deleted (`host.nono_binary.commit` + `query` debug crumbs, untracked 12-02-PLAN.md reconstruction); 2 new root-anchored `.gitignore` patterns prevent recurrence. No production code touched; `cargo fmt --all -- --check` still exit 0.
- CLEAN-04 — COMPLETE (commits `18e9768`, `a71b2bf`, `c3defb6`, `ddf408b`, `f626e24`, 2026-04-18). `is_prunable` predicate on `SessionRecord` + 8 unit tests; `nono prune` extended with duration-form `--older-than <DURATION>` (require-suffix parser, breaking change from integer-days) and `--all-exited` escape hatch; `nono ps` auto-sweep with 100-file threshold spawns background prune; T-19-04-07 mitigation via structural early-return on `NONO_CAP_FILE`; one-shot cleanup on this host: BEFORE=1392, AFTER=49, DELTA=1343 stale session files. `docs/session-retention.md` landed. All 3 workspace gates green (fmt / clippy / plan-19-04-test). Pre-existing `tests/env_vars.rs` (19) and `trust_scan::tests::*` (0–2) failures carried forward unchanged from 19-02 deferred list.
