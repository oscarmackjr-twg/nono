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

- [x] **Phase 16: Resource Limits (RESL)** — CPU %, memory cap, wall-clock timeout, process count via `JOB_OBJECT_CPU_RATE_CONTROL_ENABLE`, `JobMemoryLimit`, supervisor-timer + `TerminateJobObject` (kernel JOB_TIME deliberately NOT used since it tracks CPU not wall-clock), `ActiveProcessLimit`. CLI flags: `--cpu-percent`, `--memory`, `--timeout`, `--max-processes`. Cross-platform: Unix accepts flags with a "not enforced on this platform" warning pending cross-platform follow-up milestone. `nono inspect` surfaces active caps via a `Limits:` block. **Completed 2026-04-18.**

  **Depends on:** v2.0 Named Job Object infrastructure (Phase 01 / Phase 06).

  **Plans:** 16-01 (CLI flags + Windows enforcement for CPU/memory/processes) + 16-02 (wall-clock timeout timer + observability). Both complete.

- [x] **Phase 17: Attach-Streaming (ATCH)** — Anonymous-pipe stdio bridges in the Windows detached supervisor: `nono attach <id>` now streams child stdout live, accepts stdin from the attach client, supports clean detach (Ctrl-]d) + re-attach, and a second attach client receives a friendly `Session <id> is already attached` error. Resize on detached sessions is **explicitly downgraded to a documented limitation per D-07** (anonymous-pipe stdio is structurally exclusive of ConPTY; preserves the Phase 15 `0xC0000142` fix). Smoke gate executed 2026-04-19 with pragmatic-PASS verdict (G-01 PASS, G-02 PARTIAL PASS, G-03 PASS, G-04 Row 3 PASS + Row 4 environmental + Rows 1/2/5 structurally PASS); 4 deferred items routed to `17-HUMAN-UAT.md`. Surfaced 3 latent pre-Phase-17 Windows session-id bugs which were fixed in commit `7db6595` (corrupted job-name format string in `create_process_containment` + 2 `self.session_id` → `self.user_session_id` fixes in `start_logging`/`start_data_pipe_server`). Verifier passed 13/13 must-haves with `status: human_needed` (deferred items, not gaps). **Completed 2026-04-19.**

  **Depends on:** Phase 15 attach-pipe naming fix (commit `2c414d8`).

  **Resolved (in CONTEXT.md):** D-01 anonymous-pipes-only on detached path; D-07 resize downgraded to documented limitation; D-21 Windows-invariance — zero changes outside `*_windows.rs` files.

  **Plans:** 2 plans.
  - [x] 17-01-PLAN.md — implementation: DetachedStdioPipes + STARTUPINFOW wiring + start_logging/start_data_pipe_server pipe branches + run_attach friendly busy-error + unit/integration tests (complete 2026-04-19, 9 commits `1e38381`..`ecfeba7`; D-02 + D-21 invariance held)
  - [x] 17-02-PLAN.md — manual smoke gate G-01..G-04 + REQUIREMENTS.md ATCH-01 acceptance #3 downgrade + CHANGELOG [Unreleased] entry + docs/cli/features/session-lifecycle.mdx no-resize note + 13-UAT.md P17-HV-1..4 rows (complete 2026-04-19, commit `ab88cf5`; pragmatic-PASS verdict per user)

- [ ] **Phase 18: Extended IPC (AIPC)** — Broker socket, named-pipe, Job Object, event, and mutex handles over the Phase 11 capability pipe. Each handle type validated server-side against access-mask allowlist.

  **Depends on:** Phase 11 capability pipe protocol, Phase 16 (Job Object handle brokering benefits from RESL work landing first).

  **Plans:** TBD during `/gsd-plan-phase 18` (likely 3 plans: protocol extension + handle-type-specific brokers + security tests).

- [x] **Phase 19: Cleanup (CLEAN)** — `cargo fmt --all` for drifted files from commit `6749494`; diagnose 5 pre-existing Windows test flakes; triage disk-resident WIP (10-*, 11-*, 12-*, quick tasks, INTEGRATION-REPORT); prune 1172 stale session files + document retention policy. (complete 2026-04-19; verifier passed 25/25 must-haves, commit `6597fbf`)

  **Depends on:** Nothing; can run in parallel with the feature phases. Recommended to run last so it catches any drift introduced by the feature phases too.

  **Plans:** 4 plans (all Wave 1, parallel — disjoint files_modified).
  - [x] 19-01-PLAN.md — CLEAN-01 fmt drift fix on 3 files from commit 6749494 (complete 2026-04-18, commit `c87b10b`)
  - [x] 19-02-PLAN.md — CLEAN-02 restore 5 pre-existing Windows test flakes — fixed as 4 distinct deterministic Windows platform bugs (JSON-escape, non-absolute Unix-shaped XDG env guard, UNC-prefix production bug in `query_path`, Unix-only path literal vs absolute-path debug_assert). Hypothesis D-07 (parallel env contamination) contradicted; deviation D-08 tripped and user-approved option C for the production fix (complete-with-deviation 2026-04-18, commits `400f8c9`, `8412fda`, `a449454`, `4db849d`)
  - [x] 19-03-PLAN.md — CLEAN-03 triage 10 disk-resident WIP items (per-file disposition) — 6 backfilled alive, 2 reverted to HEAD, 2 deleted, 2 new `.gitignore` patterns for WFP-service debug crumbs (complete 2026-04-18, commits `a208761`, `a4100aa`, `db4547b`, `0391e37`, `d49fda8`, `d6bf88f`)
  - [x] 19-04-PLAN.md — CLEAN-04 `is_prunable` retention predicate + `nono prune` CLI extensions (duration-form `--older-than`, `--all-exited`) + auto-sweep on `nono ps` (100-file threshold) + T-19-04-07 `NONO_CAP_FILE` structural no-op + one-shot cleanup on this host (1392 → 49, delta 1343) + `docs/session-retention.md` (complete 2026-04-18, commits `18e9768`, `a71b2bf`, `c3defb6`, `ddf408b`, `f626e24`)

- [x] **Phase 20: Upstream Parity Sync (UPST)** — Track and back-port functional changes from upstream `always-further/nono` since the fork branched. Quick-task research (commit `7180f23`, `.planning/quick/260419-cmp-upstream-036-windows-parity/COMPARISON.md`) established the fork is pinned at crate version `0.30.1` while upstream has shipped 0.31–0.37.1 — the fork is missing upstream work in keyring URIs, `--allow-gpu`, GitLab trust tokens, macOS Seatbelt refinements, Cargo version realignment, and (critically) the rustls-webpki RUSTSEC-2026-0098/0099 security upgrade landed upstream in 0.37. This phase re-establishes the Unix/macOS parity baseline without regressing the Windows-specific work in Phases 01–19. **Completed 2026-04-19** (verifier passed 38/38 must-haves).

  **Depends on:** Nothing structural; should land before v2.1 ships so the security upgrade is in the release.

  **Open questions** (for `/gsd-plan-phase 20` or `/gsd-discuss-phase 20`):
  - Rebase onto `upstream/v0.37.1` vs cherry-pick individual changes vs manual port? Rebase is cleanest but risks losing Windows-specific conflict resolution history.
  - Target tag — 0.36.0 (user's stated baseline) or 0.37.1 (latest, includes the security fix)? Recommend 0.37.1.
  - Scope boundary — do we also refresh crate versions in `Cargo.toml` to match upstream 0.37.1, or keep them on fork-internal versioning?

  **Plans:** TBD during `/gsd-plan-phase 20` — likely grouped by upstream version range (0.31–0.33, 0.34–0.35, 0.36–0.37) plus a dedicated plan for the rustls-webpki security upgrade.

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
| 16. Resource Limits (RESL-01..04) | v2.1 | 2/2 | Complete (RESL-01..04 shipped: CPU/memory/processes kernel-enforced, timeout via supervisor timer, `nono inspect` Limits block) | 2026-04-18 |
| 17. Attach-Streaming (ATCH-01) | v2.1 | 2/2 | Complete (anonymous-pipe stdio + supervisor pipe-source/sink bridges + ERROR_PIPE_BUSY friendly translation + 3 latent session-id mismatch fixes via debug `7db6595`; 4 deferred-by-design HUMAN-UAT items per pragmatic-PASS verdict) | 2026-04-19 |
| 18. Extended IPC (AIPC-01) | v2.1 | 0/? | Not Planned (run `/gsd-plan-phase 18` to start) | - |
| 19. Cleanup (CLEAN-01..04) | v2.1 | 4/4 | Complete (19-01 CLEAN-01 fmt drift; 19-02 CLEAN-02 5 test flakes + query_path UNC prod fix complete-with-deviation; 19-03 CLEAN-03 10-item WIP triage; 19-04 CLEAN-04 retention + prune + auto-sweep + T-19-04-07 mitigation + 1343-file one-shot cleanup + docs; verifier passed 25/25 must-haves, commit `6597fbf`) | 2026-04-19 |
| 20. Upstream Parity Sync (UPST) | v2.1 | 4/4 | Complete (Wave 0+1+2 2026-04-19: 20-01 rustls-webpki 0.103.12 + workspace 0.37.1; 20-02 profile extends cycle guard + claude.json symlink; 20-03 keyring:// URI + env-var filter flags + command_blocking_deprecation; 20-04 --allow-gpu + NVIDIA Linux allowlist + GitLab ID tokens. Clippy follow-up `4f08f3f` fixes 2 pre-existing 20-03 unwrap_used violations. Verifier passed 38/38 must-haves, UPST-01..04 all traced; D-21 Windows-invariance held across 11 feat/fix commits — zero `*_windows.rs` touched) | 2026-04-19 |
