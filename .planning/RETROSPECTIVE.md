# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v2.1 — Resource Limits, Extended IPC, Attach-Streaming & Cleanup

**Shipped:** 2026-04-21
**Phases:** 7 (16, 17, 18, 18.1, 19, 20, 21) | **Plans:** 25 | **Sessions:** not instrumented

### What Was Built

- Job Object resource limits — CPU percentage (`JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP`), memory (`JobMemoryLimit`), wall-clock timeout (supervisor-side `Instant` + `TerminateJobObject`), process count (`ActiveProcessLimit`); `nono inspect` surfaces active caps via new `Limits:` block.
- `nono attach <id>` on detached Windows sessions — anonymous-pipe stdio bridged through the supervisor; stdin/stdout streaming live; Ctrl-]d clean detach + re-attach; friendly multi-attach busy error. Resize downgraded to documented limitation per D-07 (anonymous-pipe stdio is structurally exclusive of ConPTY; preserves Phase 15 `0xC0000142` fix).
- Extended IPC handle brokering (AIPC) — Socket / Pipe / Job Object / Event / Mutex handles broker-able over the Phase 11 capability pipe with `DuplicateHandle` MAP-DOWN semantics, server-side access-mask validation, and `capabilities.aipc` profile-widening schema. Containment-Job runtime guard via `CompareObjectHandles` prevents the supervisor-own-Job footgun structurally. Child-side SDK with 5 cross-platform `request_*` methods.
- Phase 18.1 gap closure — 5 HUMAN-UAT gaps (G-02..G-06) closed end-to-end: CONIN$ prompts now route through D-04 per-kind templates; JobObject broker uses `CreateJobObjectW` CREATE-if-not-exists parity; dispatcher broker-failure flip makes `(Approved, grant=None)` structurally unreachable; `Profile::resolve_aipc_allowlist()` wired end-to-end; 5 `wr01_*` regression tests lock the WR-01 reject-stage invariant.
- Cleanup workstream — fmt drift fix, 4 deterministic Windows test bugs (incl. UNC-prefix production bug in `query_path`), 10 WIP items triaged, `is_prunable` + `nono prune --older-than <DURATION>` + `--all-exited` + auto-sweep on `nono ps` + `NONO_CAP_FILE` structural no-op + one-shot cleanup (1343 files) + `docs/session-retention.md`.
- Upstream parity sync to v0.37.1 — `rustls-webpki` 0.103.12 security upgrade (RUSTSEC-2026-0098/0099), `keyring://` URIs, env-var filtering, `--allow-gpu` with NVIDIA/DRM/AMD/WSL2 Linux allowlist, GitLab ID tokens for trust signing with `validate_oidc_issuer` fail-closed validator. D-21 Windows-invariance held across 11 commits.
- Windows single-file filesystem grants — per-file Low-IL mandatory-label ACEs via `SetNamedSecurityInfoW` with mode-derived mask; `AppliedLabelsGuard` RAII lifecycle; ownership-skip pre-check for system-owned paths. Unblocks the `claude-code` profile's `git_config` group on Windows.

### What Worked

- **TDD cycles were crisp for gap-closure work.** Plans 18.1-01 (G-02) and 18.1-02 (G-04) both used RED commit → GREEN commit as the atomic structure. The 6 failing `build_prompt_text_*` tests compiled cleanly as `E0425: cannot find function in this scope` — a clean RED that made the GREEN's minimal scope obvious.
- **Empirical discovery via dedicated harness.** The `crates/nono-cli/examples/pipe-repro.rs` binary let us test 13 SDDL variants in minutes to root-cause the Windows 11 26200 `WRITE_RESTRICTED` + logon-SID co-requirement. Without the harness, we'd still be guessing. Pattern worth repeating: when debugging an undocumented OS behavior, spend ~1h building the minimal repro harness first; it pays back 10x.
- **D-19 cross-phase byte-identical preservation checks caught scope drift.** Every AIPC plan asserted `git diff --stat HEAD~N HEAD -- <out-of-scope-paths>` was empty; caught multiple would-be leaks of drive-by changes into cross-phase files.
- **Phase 18.1 dual-run widening proof was decisive.** Running the same rebuilt binary under `--profile claude-code` vs a widened `aipc-widen.json` profile and getting opposite outcomes (denial after prompt vs successful broker) is the cleanest possible validation that `Profile::resolve_aipc_allowlist()` threading is correct.
- **Phase 20 D-21 Windows-invariance as a structural guard.** Mechanical grep check (`zero *_windows.rs files touched`) across 11 commits prevented Windows-specific regressions during the Unix/macOS parity back-port.
- **Concurrent disjoint phases (19, 20, 21).** Phases 19 (cleanup), 20 (upstream parity), 21 (WSFG) ran without mutual interference because their scope was strictly disjoint. Pattern: when a milestone has multiple independent deliverables, call out the file-scope boundary in CONTEXT.md up front.

### What Was Inefficient

- **Phase 21 surfaced the supervisor-pipe regression late.** WSFG-03 (Phase 18 UAT close-out) was planned as Phase 21's final gate, but the first-ever end-to-end `claude-code → supervised → aipc-demo` run only became reachable AFTER Plans 21-02..21-04 landed — at which point the `WRITE_RESTRICTED` pipe ACCESS_DENIED regression surfaced, which was outside Phase 21's scope per its `<critical_rules>`. Result: carry-forward to a dedicated debug session + Phase 18.1 HUMAN-UAT re-run. Lesson: when a phase unblocks a previously-unreachable code path, assume the unblocked path has latent bugs and budget a debug session before promising live-UAT close-out in the same phase.
- **Plan 18-03 `AipcResolvedAllowlist::default()` seed was visible but deferred.** Plan 18-03 explicitly marked `Deferred Issue #1` for Profile threading. Three plans (18-03, 18-04, Phase 18.1's G-02/G-03/G-04) shipped before G-06 closed the gap. HUMAN-UAT discovered the gap end-to-end first. Lesson: deferred issues that block end-to-end validation should either be escalated to same-phase priority or explicitly gated from HUMAN-UAT.
- **CLEAN-02 hypothesis D-07 (parallel env contamination) was wrong.** Three days of debugging under the wrong hypothesis before the empirical data forced the pivot. The actual bugs were 4 distinct deterministic Windows platform issues — including a genuine production bug in `query_path`. Lesson: when flakes have been "in the backlog" for weeks, the "obvious cause" hypothesis (env contamination in test isolation) has already had a confirmation-bias filter applied. Start by writing a single deterministic reproducer per test before hypothesizing the cause.
- **AIPC acceptance shape evolved across Phase 18 → Phase 18.1.** Plan 18-03's Deferred Issue #1, Plan 18-01's JobObject `OpenJobObjectW` vs CREATE semantics, G-04's `(Approved, grant=None)` wire-protocol gap — all three surfaced only during HUMAN-UAT. Lesson: for protocol-heavy phases, stage a live end-to-end smoke test earlier (mid-phase) rather than gating entirely on post-implementation UAT.

### Patterns Established

- **`examples/<probe>.rs` harness for OS-behavior spelunking.** `pipe-repro.rs` established the pattern — a minimal binary under `crates/nono-cli/examples/` that exercises a narrowly-scoped OS API with parameterized inputs for rapid hypothesis testing. Worth preserving + extending.
- **Single-site flow-control tuple reshaping instead of type-level enforcement.** Plan 18.1-02 G-04 rewrote `let grant = if decision.is_granted() { ... }` → `let (decision, grant) = if decision.is_granted() { ... }` at ONE site, making the illegal `(Approved, grant=None)` shape unreachable without cascading into 23 test construction sites. Preferable to wire-protocol compile-time tightening when the tightening would be invasive.
- **Module-level docstring as verdict matrix.** Plan 18.1-04's `//!` docstring at the top of `capability_handler_tests` documents the WR-01 reject-stage matrix + CONTEXT D-14 deferral note. Future readers see the product decision inline with the tests that lock it. Pattern worth repeating for empirically-established invariants.
- **Decimal phases for same-milestone follow-ups.** Phase 18.1 followed Phase 18 for 5-gap HUMAN-UAT remediation. The decimal numbering kept provenance obvious (gaps surfaced during Phase 18 HUMAN-UAT) while keeping Phase 19, 20, 21 numbering stable.
- **Ownership pre-check for subtractive labels.** Phase 21 `try_set_mandatory_label` now skips system-owned paths. Pattern: when an OS security mechanism is subtractive (like Low-IL mandatory labels), test ownership before applying rather than failing on the OS error.

### Key Lessons

1. **Undocumented OS behavior requires systematic reproducers, not documentation searches.** MSDN does not describe the Windows 11 26200 `WRITE_RESTRICTED` + logon-SID co-requirement. The only path to root cause was the 13-SDDL-variant iteration. Build the harness first; read the docs second.
2. **End-to-end unblocking phases have hidden latent-bug budgets.** Phase 21 unblocked the `claude-code → aipc-demo` flow and surfaced TWO latent bugs (supervisor-pipe ACCESS_DENIED + `WSAStartup` gap) that had been present since Phase 11 / Phase 18-04 but unreachable. Budget a debug session after any phase that opens a new end-to-end code path.
3. **Deferred issues that block validation should be flagged as milestone-blockers, not plan-local issues.** Plan 18-03 Deferred Issue #1 blocked G-06 validation. Three plans shipped before it was fixed. Rule of thumb: if a deferred issue prevents end-to-end HUMAN-UAT from passing, it's a milestone-level blocker, not a plan-local tech-debt item.
4. **Empirical dual-run is the decisive validation for configuration wiring.** Running the same binary under two profiles with opposite outcomes is a cleaner wiring proof than any number of unit tests. Keep in the verification toolkit.
5. **D-21 Windows-invariance mechanical guards prevent regression during non-Windows work.** The `zero *_windows.rs files touched` grep across Phase 20's 11 commits caught nothing — because the guard was in place from commit 1. Mechanical guards are cheap insurance.

### Cost Observations

- Not instrumented for this milestone. Future milestones should record model mix + session count via the gsd session-report tooling.

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Phases | Plans | Key Change |
|-----------|--------|-------|------------|
| v1.0 Windows Alpha | 4 | ~12 | Initial Windows-first cut; signed artifacts + WFP packaging |
| v2.0 Windows Gap Closure | 11 | 29 | Closed 7 feature gaps; introduced decimal-phase pattern (Phase 15) for carry-forward closure |
| v2.1 Resource Limits + AIPC + Cleanup | 7 | 25 | Added decimal-phase same-milestone gap closure (Phase 18.1); established `examples/<probe>.rs` harness pattern; mechanical D-21 invariance guards |

### Cumulative Quality

| Milestone | Tests (approx) | Clippy | Fmt | Notes |
|-----------|----------------|--------|-----|-------|
| v1.0 | baseline | clean | clean | — |
| v2.0 | +49 new (Phase 11 + 12 + 13 UAT scaffolding) | clean | clean | — |
| v2.1 | +108 new (18-01..04: 21 + 18.1: 19 + RESL: 8 + ATCH: 17 + CLEAN: 8 + WSFG: 5 + UPST: ~30) | clean | clean (post-19-01) | 5 deterministic Windows test bugs fixed incl. UNC-prefix prod bug |

### Top Lessons (Verified Across Milestones)

1. **Empirical OS-behavior reproducers outperform documentation searches.** (v2.0 Phase 15 direction-b discovery; v2.1 Phase 21 debug session; both required harness-based iteration.)
2. **Deferred issues that block end-to-end validation are milestone-level blockers, not plan-local.** (v2.0 Phase 14 Plan 14-01 escalated to Phase 15; v2.1 Plan 18-03 Deferred Issue #1 escalated to Plan 18.1-03.)
3. **Decimal-phase numbering preserves provenance for carry-forward work.** (v2.0 Phase 15 carried Phase 14-01's architecture work; v2.1 Phase 18.1 carried Phase 18 HUMAN-UAT gaps.)
4. **D-21 Windows-invariance as a mechanical grep guard.** (v2.0 used it per-plan; v2.1 extended it to cross-phase byte-identical preservation checks.)
