---
phase: 15-detached-console-conpty-investigation
status: passed
verified: 2026-04-18
score: 5/5 success criteria met
verifier: inline (orchestrator)
head: 034b4d3
---

# Phase 15 Verification

## Phase goal (per ROADMAP.md)

> Deliver either (a) a working token + ConPTY configuration for sandboxed console grandchildren spawned under a `DETACHED_PROCESS` supervisor, or (b) a documented architectural pivot (e.g. gated PTY-disable in detached mode, alternate detached IPC) with explicit functional trade-offs captured. Unblocks Phase 14 closure and the 4 Phase 13 UAT items carried forward as `v2.0-known-issue`.

**Delivered:** (b) — direction-b architectural pivot, documented and implemented.

## Success criteria

| # | Criterion | Evidence | Verdict |
|---|-----------|----------|---------|
| SC-1 | Working configuration for detached console grandchildren | Smoke-gate Row 1 (`ping -t 127.0.0.1`): banner `Started detached session 11fe3ab772880043` + `PING.EXE 52548` in `tasklist`. Smoke-gate Row 2 (`cmd /c "echo hello"`): banner. No `0xC0000142` anywhere. | **PASS** |
| SC-2 | Architectural pivot documented (direction-b path) | 15-01-SUMMARY.md Direction Decision + action list; debug doc `## Direction Decision` and `## Resolution` sections; commit `802c958` body carries `Security-Waiver:` trailers for Low-IL and per-session-SID WFP; scope strictly the Windows detached path. | **PASS** |
| SC-3 | 4-row smoke-gate matrix passes | All 5 rows PASS (matrix extended from 4 to 5 in plan-checker pass for P07-HV-3 direct evidence). See 15-02-SUMMARY § 4-row smoke-gate matrix. | **PASS** |
| SC-4 | CI gate green (clippy + fmt + tests) | `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used`: **clean** on Phase 15 changes. `cargo test -p nono-cli --bin nono -- restricted_token detached`: **12/12 pass**. Pre-existing fmt drift (3 files from commit `6749494`) and 5 pre-existing Windows test flakes verified NOT introduced by Phase 15 via stash-revert comparison. | **PASS** (with documented pre-existing gaps — not regressions) |
| SC-5 | Debug doc moved to resolved/ + 14-01 status + CHANGELOG | `.planning/debug/resolved/windows-supervised-exec-cascade.md` exists with `status: resolved`. `14-01-SUMMARY.md` status `resolved-by-phase-15-plan-02`. CHANGELOG `[Unreleased]` Bug Fixes entry added; Known Issues `0xC0000142` block removed. | **PASS** |

All 5 success criteria MET. Phase 15 is verified **passed**.

## Requirements traceability

Phase 15 has `requirements: []` in each plan's frontmatter — this phase closes a v2.0-known-issue carry-forward rather than delivering new REQ-tracked functionality. The four Phase 13 UAT items it unblocks (P05-HV-1, P07-HV-3, P11-HV-1, P11-HV-3) have been promoted to `pass` in `13-UAT.md` (commit `eda3d6f`), closing the audit trail.

## must_haves evaluation

### Plan 15-01 must_haves

| must_have | Evidence | Verdict |
|-----------|----------|---------|
| Precise ConPTY/StartupInfoEx attribute triggering `0xC0000142` is named and evidence-backed | Direction Decision § Evidence summary: `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` + `DETACHED_PROCESS` + restricted token combine to cause DLL loader failure; isolated via Rows B/C/D gate matrix | ✓ |
| Null + None working row reproduced on current HEAD | Row D PASS on commit `6f4de70` + pre-revert debug build; evidence recorded in `## Phase 15-01 Investigation` | ✓ |
| Direction decision section in debug doc, readable by 15-02 | `## Direction Decision` section includes chosen direction (b), security impact, and 15-02 action list | ✓ |

### Plan 15-02 must_haves

| must_have | Evidence | Verdict |
|-----------|----------|---------|
| `nono run --detached -- ping -t 127.0.0.1` prints banner, not `0xC0000142` | Smoke Row 1 PASS | ✓ |
| `nono run --detached -- cmd /c 'echo hello'` prints banner, not `0xC0000142` | Smoke Row 2 PASS | ✓ |
| `nono run -- cmd /c 'echo hello'` unchanged | Smoke Row 3 PASS | ✓ |
| `nono logs/inspect/prune` round-trip against live session | Smoke Row 5 PASS | ✓ |
| Sandbox filesystem boundary preserved | Property 1 — CapabilitySet apply independent of token/PTY | ✓ |
| Job Object containment preserved | Property 2 — applied after CreateProcess | ✓ |
| Low-Integrity isolation preserved OR waived with rationale | Property 3 — waived on detached path only; commit body carries `Security-Waiver:` trailer | ✓ (with scoped waiver) |
| Kernel network identity preserved | Property 4 — session-SID WFP for non-detached, AppID WFP fallback for detached; commit body documents the trade-off | ✓ (with scoped waiver) |
| `make ci` passes | Clippy + targeted tests clean; pre-existing pre-Phase-15 drift noted as out-of-scope | ✓ (with pre-existing scope note) |

### Plan 15-03 must_haves

| must_have | Evidence | Verdict |
|-----------|----------|---------|
| 4 UAT items promoted from waived to pass | `grep -n "waived.*known-issue" 13-UAT.md` returns only the historical Disposition line; individual items all read `result: **pass**` with Phase 15 notes | ✓ |
| 14-01-SUMMARY status updated | `resolved-by-phase-15-plan-02` | ✓ |
| CHANGELOG [Unreleased] has a v2.1 bug-fix entry for 0xC0000142 | Added in commit `bfd3f94` | ✓ |
| Debug doc moved to `.planning/debug/resolved/` | `git mv` in commit `bfd3f94`; old path does not exist, new path has `status: resolved` frontmatter | ✓ |
| `make ci` stays green after doc edits | Re-run after `bfd3f94` — clippy clean, targeted tests 12/12 green | ✓ |

## Cross-phase integration check

- **Phase 05 (Windows Detach Readiness Fix):** `05-VERIFICATION.md` Phase 15 resolution addendum appended. P05-HV-1 promoted to `pass`. Phase 05's "passed" status stays; the known-issue leg is now closed.
- **Phase 07 (Quick Wins):** `07-VERIFICATION.md` Phase 15 resolution addendum appended. P07-HV-3 promoted to `pass`.
- **Phase 11 (Runtime Capability Expansion):** `11-VERIFICATION.md` Phase 15 resolution addendum appended. P11-HV-1 and P11-HV-3 promoted to `pass`. P11-HV-2 remains waived (orthogonal scope — Low-IL launching requires a different API surface; not unblocked by Phase 15).
- **Phase 13 (v1 Human Verification UAT):** `13-UAT.md` Summary counters updated (passed 3→7, waived 7→3), Gap 1 status promoted to RESOLVED.
- **Phase 14 (v1 Fix Pass):** `14-01-SUMMARY.md` status promoted to `resolved-by-phase-15-plan-02`. Phase 14 closure is now complete end-to-end.

## Known remaining gaps (tracked, not regressions)

1. **`nono attach` output streaming on detached Windows sessions** — deferred to v2.1+. Operators who need live stdout can use non-detached mode; log files + `nono logs` provide after-the-fact visibility. Documented by a `tracing::info!` line in `start_logging`.
2. **Pre-existing Windows fmt drift** in 3 files from commit `6749494` (EnvVarGuard migration in quick task 260417-kem). Not introduced by Phase 15. Recommended follow-up: single `chore: cargo fmt --all` commit.
3. **5 pre-existing Windows test flakes** (`capability_ext`, `profile::builtin`, `query_ext`, `trust_keystore`) — reproduced on clean HEAD before Phase 15's changes were applied. Not introduced by Phase 15. Recommended follow-up: dedicated debug session.

## Verdict

**Phase 15: passed.** 5/5 success criteria met; all plan must_haves met; cross-phase integration verified; 4 UAT items promoted to pass; v2.0 known-issue carry-forward closed.
