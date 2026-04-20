---
phase: 21-windows-single-file-grants
plan: 01
subsystem: planning

tags: [requirements, roadmap, windows, filesystem, sandbox, wsfg, bookkeeping]

# Dependency graph
requires:
  - phase: 21-windows-single-file-grants
    provides: 21-CONTEXT.md D-01..D-07 + I-01..I-03 invariants (authored 2026-04-20, commit 9b4e8a7)
provides:
  - WSFG-01 requirement ID — per-file + write-only-directory Low IL mandatory-label enforcement primitive with mode-derived mask table
  - WSFG-02 requirement ID — label lifecycle, idempotent apply, revert-on-session-end, NonoError::LabelApplyFailed error surface
  - WSFG-03 requirement ID — Phase 18 HUMAN-UAT close-out gate (AIPC cookbook Path B + Path C re-run)
  - ROADMAP.md Phase 21 entry carries **Requirements:** WSFG-01..03 traceability line
  - v2.1 milestone header incremented from 10 → 13 requirements with WSFG-01..03 appended
affects: [phase-21 plans 21-02 through 21-05, phase-18 UAT re-run scheduling, v2.1 milestone close-out bookkeeping]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "UPST/RESL REQ-XX entry shape reused verbatim: Context paragraph + three REQ-XX sub-sections with **What:** / **Enforcement:** / **Security:** / **Acceptance:** numbered list / **Maps to:** trailer"
    - "ROADMAP v2.1 milestone header **Requirements (N):** count-in-parens convention carried forward (10 → 13)"
    - "ROADMAP phase-entry **Requirements:** line slots between **Unblocks:** and **Depends on:** (new convention for Phase 21; prior v2.1 phase entries delegate requirement traceability to the milestone header)"

key-files:
  created: []
  modified:
    - .planning/REQUIREMENTS.md — appended ## WSFG section (63 lines) after UPST-04
    - .planning/ROADMAP.md — 1-line insert in Phase 21 entry + 1-char swap in v2.1 header

key-decisions:
  - "WSFG-01 scope covers BOTH WindowsUnsupportedIssueKind variants (SingleFileGrant + WriteOnlyDirectoryGrant) per CONTEXT.md D-06; didn't split into two separate REQ-XX entries because the enforcement primitive (SetNamedSecurityInfoW + mode-derived mask) is identical."
  - "Mask-encoding table frozen verbatim in WSFG-01 **What:** block so downstream implementation plans (21-02..21-03) cannot drift (NO_EXECUTE_UP always set — rationale in CONTEXT.md <specifics> lines 164-171)."
  - "NonoError::LabelApplyFailed variant shape (path + hresult + hint) chosen over extending UnsupportedPlatform per CONTEXT.md D-04 — planner discretion exercised in favor of a named variant for cleaner match sites."
  - "WSFG-03 (Phase 18 UAT re-run) factored out as a separate requirement ID rather than folding it into WSFG-01/02 acceptance, so the HUMAN-UAT close-out has its own traceable gate and downstream plan 21-05 can carry a single requirements: [WSFG-03] frontmatter without pulling in implementation acceptance."

patterns-established:
  - "Pattern: When a phase adds a new requirement family mid-milestone, append the ## REQ-FAMILY section after the existing terminal section (UPST here) rather than re-ordering. The v2.1 milestone header count-in-parens gets updated in lockstep."
  - "Pattern: Phase-level **Requirements:** line in ROADMAP entries should slot between **Unblocks:** and **Depends on:** — keeps cause/effect/dependency reading order."

requirements-completed: [WSFG-01, WSFG-02, WSFG-03]

# Metrics
duration: 5min
completed: 2026-04-20
---

# Phase 21 Plan 21-01: Requirements Bookkeeping Summary

**Added WSFG-01..03 to REQUIREMENTS.md and linked ROADMAP Phase 21 entry + v2.1 milestone header so downstream plans 21-02..21-05 can carry `requirements:` frontmatter without dangling references.**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-04-20T18:46:04Z
- **Completed:** 2026-04-20T18:50:46Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Three new requirement IDs landed in `.planning/REQUIREMENTS.md` under a new `## WSFG — Windows Single-File Grants` section placed after UPST-04, mirroring the UPST/RESL entry shape (Context paragraph + three REQ-XX sub-sections with `**What:** / **Enforcement:** / **Security:** / **Acceptance:** / **Maps to:**` blocks).
- `.planning/ROADMAP.md` Phase 21 entry now carries a `**Requirements:** WSFG-01, WSFG-02, WSFG-03.` line slotted between the existing `**Unblocks:**` and `**Depends on:**` paragraphs — establishes the phase-entry requirements traceability convention for the rest of the v2.1 milestone.
- `.planning/ROADMAP.md` v2.1 milestone header bumped from `**Requirements (10):**` to `**Requirements (13):**` with `WSFG-01..03` appended to the inline list so the milestone-level count stays in sync with REQUIREMENTS.md.
- Downstream plans 21-02..21-05 can now populate their `requirements:` frontmatter field with valid IDs that resolve to existing acceptance criteria — no more dangling references.

## Task Commits

Each task was committed atomically with DCO sign-off on branch `windows-squash`:

1. **Task 1: Add WSFG section to REQUIREMENTS.md** — `d8dbe2c` (docs)
2. **Task 2: Add Requirements line to ROADMAP.md Phase 21 entry + bump v2.1 header** — `6df4f4b` (docs)

_Note: This plan is pure bookkeeping — no `test/feat/refactor` commit types used. Both commits are `docs(21-01):` scope per the repo's conventional-commit convention._

## Files Created/Modified

- `.planning/REQUIREMENTS.md` — appended 63-line `## WSFG — Windows Single-File Grants` section (Context paragraph + WSFG-01, WSFG-02, WSFG-03 sub-sections) at EOF after UPST-04.
- `.planning/ROADMAP.md` — 1-line insert (phase entry `**Requirements:**` traceability line) + 1-line swap (v2.1 milestone header count 10 → 13 and WSFG-01..03 appended).

## Decisions Made

- **Consolidated two unsupported variants into WSFG-01.** `WindowsUnsupportedIssueKind::SingleFileGrant` and `WriteOnlyDirectoryGrant` share the same enforcement primitive (`SetNamedSecurityInfoW` + mode-derived mask); splitting them would have produced duplicate acceptance criteria. CONTEXT.md D-06 supports this consolidation.
- **Froze the D-01 mask-encoding table verbatim in WSFG-01's `**What:**` block.** Downstream implementation plans (21-02, 21-03) cannot drift from the mask semantics (`Read` → `NO_WRITE_UP | NO_EXECUTE_UP`, `Write` → `NO_READ_UP | NO_EXECUTE_UP`, `ReadWrite` → `NO_EXECUTE_UP`) — rationale in CONTEXT.md `<specifics>` lines 164-171 (execute is never granted through a filesystem capability; always set `NO_EXECUTE_UP`).
- **Chose `NonoError::LabelApplyFailed { path, hresult, hint }` named variant over extending `UnsupportedPlatform`.** CONTEXT.md D-04 left this to planner discretion. A named variant keeps match sites cleaner (the Phase 9 `AttachBusy` precedent informs this choice) and surfaces the three diagnostic fields (path, Win32 code, hint) as a self-documenting type.
- **Gave Phase 18 UAT close-out its own requirement ID (WSFG-03) rather than folding into WSFG-01/02 acceptance.** Plan 21-05 can then carry a single `requirements: [WSFG-03]` frontmatter without implicitly pulling implementation acceptance criteria — cleaner traceability.

## Deviations from Plan

None — plan executed exactly as written.

Both tasks landed with the exact text specified in the plan's `<action>` blocks. All 11 acceptance criteria for Task 1 and all 6 acceptance criteria for Task 2 verified via grep counts before commit.

## Issues Encountered

None. Two PreToolUse READ-BEFORE-EDIT hook reminders fired during execution (once on REQUIREMENTS.md, twice on ROADMAP.md) but both target files had been fully Read via the Read tool at the start of the session — the edits succeeded as confirmed by the tool output ("The file ... has been updated successfully") and subsequent grep verification. Noted for hook-tuning follow-up but not a blocking issue.

## User Setup Required

None — pure planning artifact modifications, no external service configuration, no environment variables, no code.

## Verification Commands Re-Run Post-Commit

```bash
$ grep -c "^### WSFG-0[123]:" .planning/REQUIREMENTS.md
3

$ grep -c "WSFG-01, WSFG-02, WSFG-03" .planning/ROADMAP.md
1

$ grep -c "Requirements (13):" .planning/ROADMAP.md
1

$ grep -c "Requirements (10):" .planning/ROADMAP.md
0

$ grep -c "^## WSFG — Windows Single-File Grants" .planning/REQUIREMENTS.md
1

$ grep -c "Maps to.*Phase 21 Plans 21-02, 21-03, 21-05" .planning/REQUIREMENTS.md
1

$ grep -c "Maps to.*Phase 21 Plans 21-02, 21-04" .planning/REQUIREMENTS.md
1

$ grep -c "Maps to.*Phase 21 Plan 21-05" .planning/REQUIREMENTS.md
1

$ grep -c "SECURITY_MANDATORY_LOW_RID" .planning/REQUIREMENTS.md
2   # appears in WSFG-01 **What:** block + Acceptance #4

$ grep -c "LabelApplyFailed" .planning/REQUIREMENTS.md
5   # WSFG-02 **What:**, Acceptance #1, #2, #5 + I-02 invariant reference in WSFG-01 **Security:**

$ grep -c "last session out restores" .planning/REQUIREMENTS.md
1

$ grep -c "18-HUMAN-UAT.md" .planning/REQUIREMENTS.md
4   # Context paragraph (implicit via "Phase 18 AIPC UAT cookbook" wording — explicit string matches in WSFG-03 Acceptance #1, #2, #4)
```

All 11 Task-1 acceptance strings present. All 6 Task-2 acceptance strings present. Phase 21 goal paragraph (`WindowsUnsupportedIssueKind::SingleFileGrant`) and Plans-TBD line untouched per non-modification directive.

## Self-Check: PASSED

- [x] `.planning/REQUIREMENTS.md` WSFG section exists (grep count 3 for `### WSFG-0[123]:`)
- [x] `.planning/ROADMAP.md` Phase 21 `**Requirements:** WSFG-01..03` line exists (grep count 1)
- [x] `.planning/ROADMAP.md` v2.1 milestone header bumped to `Requirements (13):` (grep count 1; old `Requirements (10):` count 0)
- [x] Commit `d8dbe2c` (Task 1) present in `git log` with DCO trailer `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`
- [x] Commit `6df4f4b` (Task 2) present in `git log` with DCO trailer `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`
- [x] Phase 21 goal paragraph untouched (`WindowsUnsupportedIssueKind::SingleFileGrant` still grep-count 1)
- [x] Phase 21 **Plans:** line untouched (`TBD during /gsd-plan-phase 21` still grep-count 1)
- [x] Working tree clean after both task commits

## Next Phase Readiness

- Downstream plans 21-02, 21-03, 21-04, 21-05 can now reference WSFG-01, WSFG-02, WSFG-03 in their `requirements:` frontmatter. Acceptance criteria are frozen — implementation plans consume, not redefine.
- The mask-encoding table (WSFG-01 **What:** block) is the authoritative source for plans 21-02..21-03; any implementation drift is a deviation that requires returning to this plan and amending the requirement (not the plan).
- Phase 18 HUMAN-UAT re-run gate (WSFG-03) is scheduled for Plan 21-05 close-out; `18-HUMAN-UAT.md` status transition from `blocked` to `pass/issue` is the Phase 21 completion signal.

---
*Phase: 21-windows-single-file-grants*
*Completed: 2026-04-20*
