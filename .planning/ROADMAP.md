# Roadmap: nono Windows Parity & Quality

This roadmap tracks the path to full Windows/Unix parity and ongoing quality-of-life work for `nono`.

## Milestones

- ✅ **v1.0 Windows Alpha** — Phases 1–4 (shipped 2026-03-31; tag `v1.0`)
- ✅ **v2.0 Windows Gap Closure** — Phases 5–14 (shipped 2026-04-18; tag `v2.0` pending merge)
- ✅ **Phase 15 closure** — completed 2026-04-18; closed the v2.0 known-issue carry-forward
- 🚧 **v2.1 Resource Limits, Extended IPC, Attach-Streaming & Cleanup** — Phases 16–19 (scoped 2026-04-18)

## Phases

<details>
<summary>✅ v1.0 Windows Alpha (Phases 1–4) — SHIPPED 2026-03-31</summary>

- [x] Phase 1: Windows Control Foundation (3/3 plans) — completed 2026-04-04
- [x] Phase 2: Persistent Sessions — Detach/Attach (4/4 plans) — completed 2026-04-04
- [x] Phase 3: Network Sandboxing — WFP Integration (4/4 plans) — completed 2026-04-04
- [x] Phase 4: State Integrity & Deployment (3/3 plans) — completed 2026-04-05

</details>

<details>
<summary>✅ v2.0 Windows Gap Closure (Phases 5–14) — SHIPPED 2026-04-18 with carry-forward</summary>

- [x] Phase 5: Windows Detach Readiness Fix (1/1 plan) — completed 2026-04-05
- [x] Phase 6: WFP Enforcement Activation (2/2 plans) — completed 2026-04-06
- [x] Phase 7: Quick Wins (2/2 plans) — completed 2026-04-08
- [x] Phase 8: ConPTY Shell (1/1 plan, UAT-driven) — completed 2026-04-10
- [x] Phase 9: WFP Port-Level + Proxy Filtering (4/4 plans) — completed 2026-04-10
- [x] Phase 10: ETW-Based Learn Command (3/3 plans) — completed 2026-04-10
- [x] Phase 11: Runtime Capability Expansion — stretch (2/2 plans) — completed 2026-04-11
- [x] Phase 12: Milestone Bookkeeping Cleanup (3/3 plans) — completed 2026-04-11
- [x] Phase 13: v2.0 Human Verification UAT (1/1 plan) — resolved 2026-04-18 (3 pass, 7 waived; all terminal)
- [x] Phase 14: v2.0 Fix Pass (2/3 plans, 1 escalated) — complete-with-carry-forward 2026-04-18

Carry-forward → Phase 15: detached-console-grandchild `0xC0000142 STATUS_DLL_INIT_FAILED` bug. Affected UAT items P05-HV-1, P07-HV-3, P11-HV-1, P11-HV-3 waived as `v2.0-known-issue`. See `.planning/milestones/v2.0-ROADMAP.md` for the full v2.0 archive.

</details>

<details>
<summary>✅ Phase 15 closure (2026-04-18)</summary>

- [x] **Phase 15: Detached Console + ConPTY Architecture Investigation** — Delivered direction-b architectural pivot: gated PTY-disable + null-token + AppID WFP on the Windows detached path. 5-row smoke gate pass; 4 Phase 13 UAT items promoted to `pass`; Phase 14 carry-forward closed. Fix commits `802c958` + `2c414d8`; bookkeeping `0de3e77`, `eda3d6f`, `bfd3f94`, `034b4d3`, `83e3db0`. Security waivers scoped strictly to the detached path. Attach-streaming deferred to v2.1 ATCH-01.

</details>

### 🚧 v2.1 Resource Limits, Extended IPC, Attach-Streaming & Cleanup (scoped 2026-04-18)

**Goal:** Deliver Job Object resource limits (CPU / memory / timeout / process-count), extend the Phase 11 capability pipe to broker additional handle types, finish the Phase 15 attach-streaming gap with full ConPTY re-attach, and clean up accumulated v2.0 WIP.

**Requirements (10):** RESL-01..04, AIPC-01, ATCH-01, CLEAN-01..04. See `.planning/REQUIREMENTS.md`.

- [ ] **Phase 16: Resource Limits (RESL)** — CPU %, memory cap, wall-clock timeout, process count via `JOB_OBJECT_CPU_RATE_CONTROL_ENABLE`, `JobMemoryLimit`, supervisor-timer + `JOB_OBJECT_LIMIT_JOB_TIME`, `ActiveProcessLimit`. CLI flags: `--cpu-percent`, `--memory`, `--timeout`, `--max-processes`. Cross-platform: Unix accepts flags with a "not enforced on this platform" warning pending cross-platform follow-up milestone.

  **Depends on:** v2.0 Named Job Object infrastructure (Phase 01 / Phase 06).

  **Plans:** TBD during `/gsd-plan-phase 16` (likely 2 plans: CLI + enforcement; tests).

- [ ] **Phase 17: Attach-Streaming (ATCH)** — Full ConPTY re-attach on detached Windows sessions (read + write + resize). Resolves the Phase 15 deferred item so `nono attach` against detached sessions behaves like a real terminal.

  **Depends on:** Phase 15 attach-pipe naming fix (commit `2c414d8`).

  **Open question:** Can ConPTY attach to an already-running process without breaking the loader? If not, fall back to bidirectional anonymous pipe (no resize). Investigate in plan-phase.

  **Plans:** TBD during `/gsd-plan-phase 17` (likely 2 plans: investigation + implementation + smoke gate).

- [ ] **Phase 18: Extended IPC (AIPC)** — Broker socket, named-pipe, Job Object, event, and mutex handles over the Phase 11 capability pipe. Each handle type validated server-side against access-mask allowlist.

  **Depends on:** Phase 11 capability pipe protocol, Phase 16 (Job Object handle brokering benefits from RESL work landing first).

  **Plans:** TBD during `/gsd-plan-phase 18` (likely 3 plans: protocol extension + handle-type-specific brokers + security tests).

- [ ] **Phase 19: Cleanup (CLEAN)** — `cargo fmt --all` for drifted files from commit `6749494`; diagnose 5 pre-existing Windows test flakes; triage disk-resident WIP (10-*, 11-*, 12-*, quick tasks, INTEGRATION-REPORT); prune 1172 stale session files + document retention policy.

  **Depends on:** Nothing; can run in parallel with the feature phases. Recommended to run last so it catches any drift introduced by the feature phases too.

  **Plans:** TBD during `/gsd-plan-phase 19` (likely 4 plans, one per CLEAN requirement; can parallelize).

## Progress Table

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Windows Control Foundation | v1.0 | 3/3 | Complete | 2026-04-04 |
| 2. Persistent Sessions | v1.0 | 4/4 | Complete | 2026-04-04 |
| 3. Network Sandboxing | v1.0 | 4/4 | Complete | 2026-04-04 |
| 4. State Integrity & Deployment | v1.0 | 3/3 | Complete | 2026-04-05 |
| 5. Windows Detach Readiness Fix | v2.0 | 1/1 | Complete | 2026-04-05 |
| 6. WFP Enforcement Activation | v2.0 | 2/2 | Complete | 2026-04-06 |
| 7. Quick Wins | v2.0 | 2/2 | Complete | 2026-04-08 |
| 8. ConPTY Shell | v2.0 | 1/1 | Complete | 2026-04-10 |
| 9. WFP Port-Level + Proxy Filtering | v2.0 | 4/4 | Complete | 2026-04-10 |
| 10. ETW-Based Learn Command | v2.0 | 3/3 | Complete | 2026-04-10 |
| 11. Runtime Capability Expansion | v2.0 | 2/2 | Complete | 2026-04-11 |
| 12. Milestone Bookkeeping Cleanup | v2.0 | 3/3 | Complete | 2026-04-11 |
| 13. Human Verification UAT | v2.0 | 1/1 | Resolved (2nd-pass 2026-04-18 — 3 pass, 7 waived incl. 4 v2.0-known-issue) | 2026-04-18 |
| 14. Fix Pass | v2.0 | 2/3 | Complete with carry-forward (14-02 done; 14-03 done; 14-01 escalated to Phase 15) | 2026-04-18 |
| 15. Detached Console + ConPTY Architecture Investigation | post-v2.0 closure | 3/3 | Complete (direction-b fix; 5-row smoke gate pass; 4 UAT items promoted; carry-forward closed) | 2026-04-18 |
| 16. Resource Limits (RESL-01..04) | v2.1 | 0/? | Not Planned (run `/gsd-plan-phase 16` to start) | - |
| 17. Attach-Streaming (ATCH-01) | v2.1 | 0/? | Not Planned (run `/gsd-plan-phase 17` to start) | - |
| 18. Extended IPC (AIPC-01) | v2.1 | 0/? | Not Planned (run `/gsd-plan-phase 18` to start) | - |
| 19. Cleanup (CLEAN-01..04) | v2.1 | 0/? | Not Planned (run `/gsd-plan-phase 19` to start) | - |
