# Roadmap: nono Windows Gap Closure

This roadmap tracks the path to full Windows/Unix parity for `nono`. The v2.0 milestone shipped 2026-04-18; Phase 15 is the first follow-on phase for the one carry-forward known issue.

## Milestones

- ✅ **v1.0 Windows Alpha** — Phases 1–4 (shipped 2026-03-31; tag `v1.0`)
- ✅ **v2.0 Windows Gap Closure** — Phases 5–14 (shipped 2026-04-18 with v2.0-known-issue carry-forward; tag `v2.0`)
- ✅ **Candidate v2.1** — Phase 15 (detached console + ConPTY investigation) — completed 2026-04-18; v2.0 carry-forward closed

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

### ✅ Candidate v2.1 (completed 2026-04-18)

- [x] **Phase 15: Detached Console + ConPTY Architecture Investigation** — Delivered direction-b architectural pivot: gated PTY-disable on Windows detached path + null token when `NONO_DETACHED_LAUNCH=1` + AppID-based WFP filtering (replacing per-session SID WFP on the detached path only). 5-row smoke gate passes end-to-end. All 4 Phase 13 UAT items (P05-HV-1, P07-HV-3, P11-HV-1, P11-HV-3) promoted to `pass`. Phase 14 carry-forward closed. Plans: 3/3 complete.

  **Fix commits:** `802c958` (direction-b core fix), `2c414d8` (user-session-id pipe naming + fast-exit race), `0de3e77`/`eda3d6f`/`bfd3f94`/`034b4d3` (bookkeeping, UAT promotions, CHANGELOG, SUMMARY).

  **Security scope:** Waivers (Low-IL isolation + per-session-SID WFP) are restricted to the Windows detached path only. Non-detached `nono run` and `nono shell` retain the full WRITE_RESTRICTED + session-SID + ConPTY configuration. Documented in `802c958` commit body with `Security-Waiver:` trailers.

  **Deferred to v2.1+:** `nono attach` output streaming for detached Windows sessions. Operators who need live stdout on detached Windows can use non-detached mode; `nono logs` provides after-the-fact visibility.

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
| 15. Detached Console + ConPTY Architecture Investigation | v2.1 (candidate) | 3/3 | Complete (direction-b fix; 5-row smoke gate pass; 4 UAT items promoted; carry-forward closed) | 2026-04-18 |
