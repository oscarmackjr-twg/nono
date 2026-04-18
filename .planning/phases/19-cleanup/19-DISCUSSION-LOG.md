# Phase 19: Cleanup (CLEAN) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-18
**Phase:** 19-cleanup
**Areas discussed:** Scope of Phase 19, CLEAN-02 depth bar, CLEAN-03 disposition policy, CLEAN-04 retention policy

---

## Scope of Phase 19

| Option | Description | Selected |
|--------|-------------|----------|
| Keep all 4 (Recommended) | 1 phase, 4 plans (CLEAN-01..04), parallelizable per ROADMAP default. Single milestone-cleanup boundary. | ✓ |
| Narrow to CLEAN-02 only | Renumber Phase 19 = CLEAN-02 (test flakes), push CLEAN-01/03/04 to a Phase 20 omnibus or defer to v2.2. | |
| Split into two phases | Phase 19 = CLEAN-02 + CLEAN-01, Phase 20 = CLEAN-03 + CLEAN-04. | |

**User's choice:** Keep all 4 (Recommended)
**Notes:** Single phase with 4 parallelizable plans. Locks Phase 19 as the receiver for all four CLEAN-* requirements.

---

## CLEAN-02 depth bar (5 pre-existing Windows test flakes)

| Option | Description | Selected |
|--------|-------------|----------|
| Diagnose + fix all 5 (Recommended) | Root-cause each, push to GREEN, restore `cargo test --workspace` to 0 failures. | ✓ |
| Diagnose all + fix only env-var bugs | Investigate root causes; if any are non-trivial, document and defer. | |
| Diagnose only, defer fixes | Document root cause for each in a triage report, leave tests failing. | |
| Mark all 5 as #[ignore] + file follow-ups | Quickest path to green CI but converts permanent failure to permanent debt. | |

**User's choice:** Diagnose + fix all 5 (Recommended)
**Notes:** Failing tests are a noise floor that masks real regressions. Pay the debt fully. If any test reveals a genuine API bug requiring a redesign, STOP and re-scope rather than expanding silently (captured as D-08).

---

## CLEAN-03 disposition policy (10 disk-resident WIP items)

| Option | Description | Selected |
|--------|-------------|----------|
| Per-file judgment (Recommended) | Inspect each: commit alive, delete dead, archive uncertain. Per-file table in SUMMARY. | ✓ |
| Delete-stale sweep | Treat all uncommitted WIP as stale; `git clean -fd` after backing up modified 11-PLAN.md files. | |
| Commit-everything sweep | `git add -A && git commit`. Locks in stale plans as 'official'. | |

**User's choice:** Per-file judgment (Recommended)
**Notes:** Each WIP item evaluated on its own merits. 11-PLAN.md modifications and 10-UAT.md likely represent real work; debug crumbs (host.nono_binary.commit, query) clearly disposable; INTEGRATION-REPORT may have archive value.

---

## CLEAN-03 stray-file default (`host.nono_binary.commit`, `query`)

| Option | Description | Selected |
|--------|-------------|----------|
| Delete + add to .gitignore (Recommended) | Remove now, prevent recurrence. | ✓ |
| Inspect first, then decide | Read content during execution; defer to per-file judgment. | |

**User's choice:** Delete + add to .gitignore (Recommended)
**Notes:** Both files are clearly debug crumbs. No content inspection needed; .gitignore patterns prevent future recurrence.

---

## CLEAN-04 retention rule

| Option | Description | Selected |
|--------|-------------|----------|
| Age-based: 30 days (Recommended) | Auto-prune sessions with `Status: Exited` AND last-activity > 30 days. Active sessions never pruned. | ✓ |
| Count-based: keep last 100 | Keep most-recent N exited sessions. | |
| Hybrid: 30 days OR last 100 | Whichever keeps MORE wins. | |
| Manual only — no auto-prune | Add `nono prune --older-than 30d`, never auto-run. | |

**User's choice:** Age-based: 30 days (Recommended)
**Notes:** Simple, predictable, matches typical CI/dev cycle. Status guard ensures active sessions are protected regardless of age.

---

## CLEAN-04 auto-prune trigger

| Option | Description | Selected |
|--------|-------------|----------|
| On `nono ps` (Recommended) | Lightweight check at start of `ps`; if >100 stale, prune in background and log count to stderr. | ✓ |
| Dedicated `nono prune` command only | No auto-trigger; operators invoke explicitly. | |
| Background daemon | Out of scope — would need persistent process. Listed only to confirm exclusion. | |

**User's choice:** On `nono ps` (Recommended)
**Notes:** Operators see the housekeeping in their normal workflow without being intrusive. Threshold (100) is a planner-tunable starting point; exact threshold and backgrounding mechanism are Claude's discretion.

---

## Claude's Discretion

- Branch / PR strategy (single PR vs per-plan PR) — planner decides based on git log size
- CLEAN-02 commit grouping (per-test vs per-root-cause-cluster) — planner decides; one commit acceptable if all 5 share one fix
- CLEAN-04 prune trigger UX details — exact log wording, exact threshold (50–500 range), backgrounding mechanism
- CLEAN-04 docs location — `docs/session-retention.md` vs inline in existing CLI reference

## Deferred Ideas

- Background daemon for prune (would need persistent process; not in v2.1)
- Session file format migration (no migration step; prune just deletes old-schema sessions)
- CLEAN-02 deeper refactors that surface API design problems (route to own phase per D-08)
- Cross-platform retention default tuning (30d picked for Windows; per-platform tuning is v2.2 polish)
