# Roadmap: nono Windows Parity & Quality

This roadmap tracks the path to full Windows/Unix parity and ongoing quality-of-life work for `nono`.

## Milestones

- ✅ **v1.0 Windows Alpha** — Phases 1–4 (shipped 2026-03-31; tag `v1.0`)
- ✅ **v2.0 Windows Gap Closure** — Phases 5–15 (shipped 2026-04-18; tag `v2.0`)
- ✅ **v2.1 Resource Limits, Extended IPC, Attach-Streaming & Cleanup** — Phases 16–21 + 18.1 (shipped 2026-04-21; tag `v2.1`)

## Phases

<details>
<summary>✅ v1.0 Windows Alpha (Phases 1–4) — SHIPPED 2026-03-31</summary>

- [x] Phase 1: Windows Control Foundation (3/3 plans) — completed 2026-04-04
- [x] Phase 2: Persistent Sessions (4/4 plans) — completed 2026-04-04
- [x] Phase 3: Network Sandboxing (4/4 plans) — completed 2026-04-04
- [x] Phase 4: State Integrity & Deployment (3/3 plans) — completed 2026-04-05

See `.planning/milestones/v1.0-*` if archived separately; the `v1.0` git tag points at the formal shipped state.

</details>

<details>
<summary>✅ v2.0 Windows Gap Closure (Phases 5–15) — SHIPPED 2026-04-18</summary>

- [x] Phase 5: Windows Detach Readiness Fix (1/1 plan) — completed 2026-04-05
- [x] Phase 6: WFP Enforcement Activation (2/2 plans) — completed 2026-04-06
- [x] Phase 7: Quick Wins (2/2 plans) — completed 2026-04-08
- [x] Phase 8: ConPTY Shell (1/1 plan, UAT-driven) — completed 2026-04-10
- [x] Phase 9: WFP Port-Level + Proxy Filtering (4/4 plans) — completed 2026-04-10
- [x] Phase 10: ETW-Based Learn Command (3/3 plans) — completed 2026-04-10
- [x] Phase 11: Runtime Capability Expansion — stretch (2/2 plans) — completed 2026-04-11
- [x] Phase 12: Milestone Bookkeeping Cleanup (3/3 plans) — completed 2026-04-11
- [x] Phase 13: v2.0 Human Verification UAT (1/1 plan) — resolved 2026-04-18
- [x] Phase 14: v2.0 Fix Pass (2/3 plans, 1 escalated to Phase 15) — complete-with-carry-forward 2026-04-18
- [x] Phase 15: Detached Console + ConPTY Architecture Investigation (3/3 plans) — completed 2026-04-18

Full details: `.planning/milestones/v2.0-ROADMAP.md`.

</details>

<details>
<summary>✅ v2.1 Resource Limits, Extended IPC, Attach-Streaming & Cleanup (Phases 16–21 + 18.1) — SHIPPED 2026-04-21</summary>

- [x] Phase 16: Resource Limits — RESL-01..04 (2/2 plans) — completed 2026-04-18
- [x] Phase 17: Attach-Streaming — ATCH-01 (2/2 plans) — completed 2026-04-19
- [x] Phase 18: Extended IPC — AIPC-01 (4/4 plans) — completed 2026-04-19
- [x] Phase 18.1: Extended IPC Gap Closure (4/4 plans) — completed 2026-04-21
- [x] Phase 19: Cleanup — CLEAN-01..04 (4/4 plans) — completed 2026-04-19
- [x] Phase 20: Upstream Parity Sync — UPST-01..04 (4/4 plans) — completed 2026-04-19
- [x] Phase 21: Windows Single-File Filesystem Grants — WSFG-01..03 (5/5 plans) — completed-with-issues 2026-04-20 (supervisor-pipe regression surfaced + resolved 2026-04-20; Phase 18.1 closed the 5 AIPC UAT gaps)

Full details: `.planning/milestones/v2.1-ROADMAP.md`.

</details>

### 📋 v2.2 (not yet scoped)

No phases locked. Candidate focus areas surfaced during v2.1 — see `.planning/PROJECT.md § Next Milestone Goals`:

- WR-01 reject-stage unification across all 5 AIPC HandleKinds (currently Event/Mutex/JobObject reject BEFORE prompt; Pipe/Socket reject AFTER).
- AIPC G-04 wire-protocol compile-time tightening (`Approved(ResourceGrant)` inline; removes child SDK demultiplexer branch).
- Cross-platform RESL Unix backends (cgroup v2 `cpu.max` / `memory.max` / `pids.max`; macOS equivalents).
- WR-02 EDR HUMAN-UAT item (deferred to v3.0; needs EDR-instrumented runner).
- Merge `windows-squash` branch to `main`.

Run `/gsd-new-milestone` to scope the next milestone.

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
| 13. v2.0 Human Verification UAT | v2.0 | 1/1 | Resolved | 2026-04-18 |
| 14. v2.0 Fix Pass | v2.0 | 2/3 | Complete-with-carry-forward | 2026-04-18 |
| 15. Detached Console + ConPTY Investigation | v2.0 | 3/3 | Complete | 2026-04-18 |
| 16. Resource Limits (RESL) | v2.1 | 2/2 | Complete | 2026-04-18 |
| 17. Attach-Streaming (ATCH) | v2.1 | 2/2 | Complete | 2026-04-19 |
| 18. Extended IPC (AIPC) | v2.1 | 4/4 | Complete | 2026-04-19 |
| 18.1. Extended IPC Gap Closure | v2.1 | 4/4 | Complete | 2026-04-21 |
| 19. Cleanup (CLEAN) | v2.1 | 4/4 | Complete | 2026-04-19 |
| 20. Upstream Parity Sync (UPST) | v2.1 | 4/4 | Complete | 2026-04-19 |
| 21. Windows Single-File Grants (WSFG) | v2.1 | 5/5 | Complete-with-issues (supervisor-pipe regression resolved in-flight; HUMAN-UAT folded into Phase 18.1) | 2026-04-20 |
