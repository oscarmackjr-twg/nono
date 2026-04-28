---
phase: 24-parity-drift-prevention
plan: 02
subsystem: maintainer-tooling
tags: [markdown, mdx, mintlify, gsd-template, upstream-sync, integration-test, d-19-trailer]

# Dependency graph
requires:
  - phase: 24
    provides: "scripts/check-upstream-drift.{sh,ps1} + tests/integration/test_upstream_drift.sh foundation (Plan 24-01)"
provides:
  - ".planning/templates/upstream-sync-quick.md fillable-blanks GSD template (single-brace placeholders, D-19 trailer block verbatim, fork-divergence catalog, Windows retrofit checklist)"
  - "docs/cli/development/upstream-drift.mdx long-form Mintlify runbook (7 H2 sections covering drift check → output formats → categorization → template integration → D-19 trailer → fixture regeneration)"
  - ".planning/PROJECT.md § Upstream Parity Process H2 (4-step workflow: inventory → scaffold → cherry-pick → retrofit)"
  - "tests/integration/test_upstream_drift.sh extended (Tests 8–11) with DRIFT-02 assertions: D-19 trailer block presence, fork-divergence catalog explicit naming, T-24-05 placeholder smoke test, documentation cross-links + section-ordering invariant"
affects: [parity-drift-prevention, upstream-sync-process, project-md, docs-mintlify]

# Tech tracking
tech-stack:
  added: ["Mintlify .mdx documentation pattern formalized for the upstream-sync runbook (first non-Windows-named-flow .mdx in the development docs directory); HTML-comment-stripping awk pre-filter for placeholder smoke testing"]
  patterns: ["single-brace `{name}` placeholder convention (verified against 31 GSD templates) applied to a project-internal template; D-19 cherry-pick trailer block encoded byte-exact verbatim from reference commits 73e1e3b8 / adf81aec / 869349df; HTML-comment-aware placeholder smoke test that mirrors the maintainer's manual replace-all check"]

key-files:
  created:
    - .planning/templates/upstream-sync-quick.md
    - docs/cli/development/upstream-drift.mdx
  modified:
    - .planning/PROJECT.md
    - tests/integration/test_upstream_drift.sh

key-decisions:
  - "Placeholder smoke test strips HTML comment blocks via awk before scanning for unfilled `{name}` markers. The template intentionally uses `{placeholder}`, `{sha}`, `{subject}`, `{adds}`, `{dels}` inside `<!-- guidance -->` comments as illustrative format examples for the maintainer; these are NOT fields and survive sed substitution by design. The test's awk pre-filter mirrors the contract documented in the template's leading `grep -oE '\\{[a-z_]+\\}' PLAN.md` smoke-check instruction (which the maintainer is expected to apply only to user-visible content)."
  - "docs/cli/development/ is in .gitignore (line 13) but the directory contains 18 already-tracked .mdx siblings (testing.mdx, index.mdx, windows-*.mdx). Used `git add -f` for upstream-drift.mdx to track it alongside its peers. The gitignore rule does not retroactively untrack previously-committed files, and the new .mdx is on the same trust footing as its already-tracked siblings."
  - "Removed `(NOT \\`Upstream-Author\\`)` parenthetical from the .mdx body because the plan acceptance criterion `grep -F 'Upstream-Author' docs/cli/development/upstream-drift.mdx outputs nothing` treats any literal `Upstream-Author` substring as a regression. Replaced with `'Upstream-author' uses a lowercase 'a' — capitalizing the 'a' is a regression.` which conveys the same semantic without tripping the structural grep."

patterns-established:
  - "Pattern: HTML-comment-aware placeholder smoke test via awk pre-strip — keeps illustrative `{name}` syntax visible for maintainer guidance while still enforcing zero unfilled fields in user-visible content"
  - "Pattern: Single-brace `{name}` placeholders in project-internal templates (matching 31-template GSD convention) instead of Mustache-style `{{name}}` or JSX-style `<NAME>`"
  - "Pattern: D-19 cherry-pick trailer block encoded byte-exact verbatim in templates (lowercase 'a' in `Upstream-author`, TWO `Signed-off-by` lines for DCO + GitHub attribution) — the structural grep enforcer prevents future drift"
  - "Pattern: Mintlify .mdx for project runbooks living alongside production user docs in `docs/cli/development/` (frontmatter `title:` + `description:` only, NO GSD-flavored frontmatter keys)"
  - "Pattern: Cross-link bidirectional invariant — PROJECT.md § Upstream Parity Process ↔ template ↔ .mdx ↔ scripts; the integration test asserts every direction with structural greps + AWK section-ordering check"

requirements-completed: [DRIFT-02]

# Metrics
duration: ~50 min
completed: 2026-04-28
---

# Phase 24 Plan 02: Parity-Drift Prevention (DRIFT-02 Template + Runbook + Cross-Links) Summary

**GSD upstream-sync quick-task template (`.planning/templates/upstream-sync-quick.md`) with byte-exact D-19 trailer block, fork-divergence catalog, Windows retrofit checklist, plus Mintlify long-form runbook (`docs/cli/development/upstream-drift.mdx`), `## Upstream Parity Process` section in PROJECT.md, and 30-assertion integration-test extension (Tests 8–11) covering template structure, fork-divergence catalog naming, T-24-05 placeholder smoke test, and bidirectional documentation cross-links — closes DRIFT-02 with zero stubs and reproducible substitution semantics.**

## Performance

- **Duration:** ~50 min
- **Started:** 2026-04-27 (worktree spawn)
- **Completed:** 2026-04-28
- **Tasks:** 3
- **Files modified:** 4 (2 created + 2 modified)
- **Test assertions:** went from 13 (Plan 24-01) → 43 (added 30 in this plan)

## Accomplishments

- DRIFT-02 deliverable shipped: maintainer copies `.planning/templates/upstream-sync-quick.md` → `.planning/quick/YYMMDD-xxx-upstream-sync-vX.Y/PLAN.md`, fills 22 single-brace placeholders, and ends up with a frontmatter-valid GSD quick-task PLAN.md skeleton ready for cherry-pick execution. The placeholder smoke test (Test 10) renders the template for a hypothetical v0.41.0 sync and asserts every field substitutes cleanly.
- D-19 cherry-pick trailer block encoded byte-exact verbatim in the template (lowercase 'a' in `Upstream-author`, fixed 6-line field order, TWO `Signed-off-by` entries for DCO + GitHub attribution). Validated against the three reference commits (73e1e3b8, adf81aec, 869349df) and locked by the integration test's per-line greps.
- Fork-divergence catalog explicitly names the four highest-risk silent-drop hazards: `validate_path_within` defense-in-depth retention (Phase 22-03 PKG-04), `ArtifactType::Plugin` deferred enum variants, async-runtime wrapping for `load_production_trusted_root`, and `hooks.rs` ownership pattern. Each entry has an "Action on cherry-pick" sub-block telling the maintainer exactly what to do when upstream's commit looks like it removes the divergence.
- `docs/cli/development/upstream-drift.mdx` long-form runbook with 7 H2 sections (Running, Output formats, Categorization rules, Using the output with the upstream-sync template, D-19 cherry-pick trailer block, Regenerating the test fixtures, See also). Mintlify frontmatter (`title:` + `description:` only). Documents the JSON output shape, path-prefix categorization lookup table, and the D-11 excluded-paths filter (matches the template-side `make check-upstream-drift > drift.json` workflow).
- `.planning/PROJECT.md § Upstream Parity Process` H2 section inserted between `## Key Decisions` and `## Evolution`, with a 4-step numbered workflow (inventory → scaffold → cherry-pick → retrofit) and cross-links to the template + .mdx runbook. Section ordering invariant locked by Test 11's awk check. Existing `### Validated` / `### Active (v2.2)` / `### Deferred` lists untouched (each grep -c stays at 1).
- Integration test (`tests/integration/test_upstream_drift.sh`) extended in place from 13 → 43 assertions. Plan 24-01's 7 test groups (golden-fixture, twin-parity, tag auto-detect, missing-remote, table format, read-only invariant, ref-injection) preserved; 4 new groups (Tests 8–11) added before the final `print_summary`. EXIT trap chained so both tmp_repo and smoke_tmpdir cleanup runs.
- Read-only invariant preserved: `git status --porcelain` snapshot is byte-identical pre and post test invocation.
- All 43 assertions pass on this maintainer's machine (Win11 26200, Git for Windows MSYS2 bash, Windows PowerShell 5.1.26100, Python 3.14.4).

## Task Commits

Each task was committed atomically:

1. **Task 1: Upstream-sync quick-task template** — `6b6df9aa` (feat)
2. **Task 2: PROJECT.md § Upstream Parity Process + docs/cli/development/upstream-drift.mdx** — `27a7967c` (feat)
3. **Task 3: test_upstream_drift.sh DRIFT-02 assertions (Tests 8–11)** — `4071023f` (feat)

## Files Created/Modified

- **`.planning/templates/upstream-sync-quick.md`** (NEW, 257 lines): Fillable-blanks Markdown template. Single-brace `{name}` placeholders. Leading HTML-comment usage block + replace-all smoke-check instruction. Sections: frontmatter (slug/created/type/range), headline, Drift inventory (with per-category sub-sections), Conflict-file inventory (pre-populated from Phase 22-03 + Phase 20 sync experience), Windows-specific retrofit checklist, Fork-divergence catalog (5 entries), D-19 cherry-pick trailer block (verbatim), Acceptance, Out-of-scope deferrals, footer cross-link to runbook. UTF-8 no-BOM, LF line endings.
- **`docs/cli/development/upstream-drift.mdx`** (NEW, 167 lines): Mintlify long-form runbook. Frontmatter `title: Upstream Drift Check` + `description: ...`. 7 H2 sections + opening paragraph + See also footer. JSON output shape worked example. Categorization rules table. D-19 trailer block worked example with field-rules sub-list. Fixture regeneration procedure with 3-fixture sample commands.
- **`.planning/PROJECT.md`** (MODIFIED, +11 lines): New `## Upstream Parity Process` H2 section between `## Key Decisions` and `## Evolution`. 4-step numbered workflow with bold-anchored bullets. Cross-link `[\`docs/cli/development/upstream-drift.mdx\`](../docs/cli/development/upstream-drift.mdx)`. Existing sections untouched.
- **`tests/integration/test_upstream_drift.sh`** (MODIFIED, +241 lines): Extended in place. 4 new test groups (Tests 8–11) added before `print_summary`. EXIT trap chained for tmp dir cleanup. 22 sed substitutions in the placeholder smoke test. AWK PROJECT.md section-ordering check. HTML-comment-stripping awk pre-filter for the placeholder smoke test (mirrors the template's documented contract that comment-block placeholders are illustrative).

## Cross-Link Map (Traceability)

```
PROJECT.md § Upstream Parity Process
  ├── make check-upstream-drift              [tooling, Plan 24-01]
  ├── .planning/templates/upstream-sync-quick.md  [template, this plan]
  └── docs/cli/development/upstream-drift.mdx     [runbook, this plan]

docs/cli/development/upstream-drift.mdx
  ├── make check-upstream-drift              [tooling, Plan 24-01]
  ├── .planning/templates/upstream-sync-quick.md  [template, this plan]
  ├── tests/integration/test_upstream_drift.sh  [integration test, Plan 24-01 + 24-02]
  ├── tests/integration/fixtures/upstream-drift/README.md  [regen procedure, Plan 24-01]
  └── .planning/PROJECT.md § Upstream Parity Process  [workflow summary]

.planning/templates/upstream-sync-quick.md
  ├── make check-upstream-drift > drift.json [tooling invocation, Plan 24-01]
  └── docs/cli/development/upstream-drift.mdx  [long-form runbook, this plan]

tests/integration/test_upstream_drift.sh
  ├── .planning/templates/upstream-sync-quick.md  [Test 8: D-19 trailer; Test 9: catalog; Test 10: smoke]
  ├── docs/cli/development/upstream-drift.mdx  [Test 11: cross-links]
  ├── .planning/PROJECT.md  [Test 11: section + cross-links + ordering invariant]
  └── scripts/check-upstream-drift.sh / .ps1  [Tests 1–7, Plan 24-01]
```

## D-19 Trailer Block (As Committed in Template, Verbatim)

The template encodes this 6-line block at the bottom of the cherry-pick trailer section:

```
Upstream-commit: {upstream_sha_abbrev}
Upstream-tag: {upstream_tag}
Upstream-author: {upstream_author_name} <{upstream_author_email}>
Co-Authored-By: {upstream_author_name} <{upstream_author_email}>
Signed-off-by: {fork_author_name} <{fork_author_email}>
Signed-off-by: {fork_author_handle} <{fork_author_email}>
```

Verified verbatim against the three reference commits (73e1e3b8, adf81aec, 869349df) which all use this exact 6-line shape. Lowercase 'a' in `Upstream-author`. Two `Signed-off-by` lines (DCO sign-off + GitHub handle attribution). Field order is fixed.

## Sample-Rendered Template (For v0.41.0 Hypothetical Sync)

When the maintainer fills the placeholders for a v0.41.0 sync (the same values used in Test 10's smoke test), the trailer block renders as:

```
Upstream-commit: abc12345
Upstream-tag: v0.41.0
Upstream-author: Upstream Author <upstream@example.com>
Co-Authored-By: Upstream Author <upstream@example.com>
Signed-off-by: Fork Author <fork@example.com>
Signed-off-by: fork-handle <fork@example.com>
```

And the frontmatter renders as:

```
---
slug: 260501-upr-sync-v041
created: 2026-05-01
type: upstream-sync
range: v0.40.1..v0.41.0
---

# Quick task: Sync upstream v0.40.1 → v0.41.0 into the fork
```

The smoke test (Test 10) asserts both of these post-substitution shapes pass, plus the zero-unfilled-placeholder invariant on user-visible content (HTML comments excluded by design).

## Decisions Made

- **HTML-comment-aware placeholder smoke test (Task 3 deviation Rule 1)**: The first run of Test 10 reported leftover `{placeholder}`, `{sha}`, `{subject}`, `{adds}`, `{dels}` markers — all of them inside `<!-- HTML comment -->` blocks where the template documents placeholder syntax for the maintainer. These are not unfilled fields; they're illustrative guidance. Fix: awk-strip HTML comment blocks before scanning. This mirrors the maintainer's documented manual replace-all check (which is also expected to operate on user-visible content). Single-site fix in the smoke test; template structure unchanged.
- **Removed parenthetical reference to `Upstream-Author` (capitalized) from .mdx (Task 2)**: The plan's acceptance criterion `grep -F 'Upstream-Author' docs/cli/development/upstream-drift.mdx outputs nothing` treats any literal `Upstream-Author` substring as a regression. The original prose `'Upstream-author' is lowercase 'a' (NOT 'Upstream-Author')` conveyed the rule via a forbidden-form example, but tripped the structural grep. Reworded to `'Upstream-author' uses a lowercase 'a' — capitalizing the 'a' is a regression.` Same semantic, no `Upstream-Author` substring.
- **`git add -f` for the .mdx (Task 2 deviation Rule 3)**: `docs/cli/development/` is in `.gitignore` (line 13) but the directory has 18 already-tracked .mdx siblings (testing.mdx, index.mdx, windows-*.mdx). Used `git add -f` to track the new .mdx alongside them. The gitignore rule does not retroactively untrack previously-committed files, so the new .mdx is on the same trust footing as its peers. Documented in the Task 2 commit body for future readers.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Placeholder smoke test reports leftover markers from inside HTML comment blocks**
- **Found during:** Task 3 (first run of `bash tests/integration/test_upstream_drift.sh`)
- **Issue:** Test 10's `grep -oE '\{[a-z_]+\}'` scan found `{placeholder}`, `{sha}`, `{subject}`, `{adds}`, `{dels}` post-substitution. All five markers live inside `<!-- HTML comment -->` blocks in the template — they're illustrative format examples for the maintainer (e.g., the example commit-list format `- {sha} {subject} ({adds}/{dels})` inside a `<!-- For each category, list... -->` block, and `replace all {placeholder} markers` in the leading replace-all instruction). They are not unfilled fields; they are guidance text.
- **Fix:** Add an awk pre-filter that strips HTML comment blocks before scanning. The user-visible content (post-strip) has zero unfilled markers, which is the correct invariant — the template's leading smoke-check instruction `grep -oE '\{[a-z_]+\}' PLAN.md` is itself implicit-scoped to user-visible content (the maintainer wouldn't grep their own guidance comments).
- **Files modified:** tests/integration/test_upstream_drift.sh (Task 3 commit)
- **Verification:** Test 10 reports `all {placeholder} markers substituted cleanly (HTML comments excluded)`; suite is now 43/43 pass.
- **Committed in:** `4071023f` (Task 3 commit)

**2. [Rule 1 - Bug] .mdx prose tripped its own structural grep acceptance criterion**
- **Found during:** Task 2 (verification of `grep -F 'Upstream-Author' docs/cli/development/upstream-drift.mdx outputs nothing`)
- **Issue:** Original prose `'Upstream-author' is lowercase 'a' (NOT 'Upstream-Author').` mentioned the forbidden form as a counter-example. Plan acceptance criterion treats any literal `Upstream-Author` substring as a regression on D-19.
- **Fix:** Reworded to `'Upstream-author' uses a lowercase 'a' — capitalizing the 'a' is a regression.` Same semantic, no `Upstream-Author` substring.
- **Files modified:** docs/cli/development/upstream-drift.mdx
- **Verification:** `grep -F 'Upstream-Author' docs/cli/development/upstream-drift.mdx` outputs nothing.
- **Committed in:** `27a7967c` (Task 2 commit, fix folded into the same commit since the file was new and the issue was discovered before initial commit)

**3. [Rule 3 - Blocking] docs/cli/development/ is in .gitignore — `git add` rejects new file**
- **Found during:** Task 2 (`git add docs/cli/development/upstream-drift.mdx` reported "ignored by one of your .gitignore files")
- **Issue:** `.gitignore` line 13 is `docs/cli/development/`. The plan demands the file at this exact path (D-16). Without `-f`, the file would not be tracked and the integration test's `[[ -f docs/cli/development/upstream-drift.mdx ]]` would fail at the first commit boundary. Pre-existing tracked siblings (18 .mdx files) prove the gitignore rule does not retroactively untrack — only blocks new additions.
- **Fix:** Used `git add -f docs/cli/development/upstream-drift.mdx` to track the new file alongside its already-tracked peers. Same trust footing as the existing siblings.
- **Files modified:** None (this is a tooling decision, documented in the Task 2 commit body)
- **Verification:** `git ls-files docs/cli/development/upstream-drift.mdx` returns the path; `git check-ignore -v` confirms gitignore line 13 was overridden by the explicit `-f`.
- **Committed in:** `27a7967c` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (2× Rule 1 bug, 1× Rule 3 blocking)
**Impact on plan:** All three deviations are local to single sites and do not change task scope. The HTML-comment-aware smoke test is the most consequential — it preserves the maintainer's freedom to document placeholder syntax as illustrative examples in HTML comments while still enforcing the zero-unfilled-fields invariant on user-visible content. The `git add -f` is a one-line invocation matching the existing repo state. The .mdx prose rewording is a micro-edit that doesn't alter user-facing semantic.

## Issues Encountered

- **No `make` invocation in this plan**: Plan 24-02 does not modify the Makefile (Plan 24-01 already added the `check-upstream-drift` target). The `.mdx` documents `make check-upstream-drift ARGS="..."` invocations as user-facing commands, but the integration test does not exercise the make path (Plan 24-01's smoke validates the dispatch logic structurally). Pre-existing limitation: this maintainer's dev box does not have `make` on PATH, so the `make check-upstream-drift` invocation is documented as the canonical way to invoke the tool but verified at the script-direct level (`bash scripts/check-upstream-drift.sh ...`).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- DRIFT-02 deliverable shipped: maintainer absorbing v0.41+ into the fork has a fillable PLAN.md template, a long-form runbook, and a 4-step PROJECT.md workflow. Phase 24 fully closed pending phase-gate verification.
- Cross-link map asserted by 11 integration-test rows + the awk section-ordering check; future drift in any direction (e.g., template renamed without updating PROJECT.md) breaks the test immediately.
- The HTML-comment-aware placeholder smoke test pattern is reusable for any future GSD template that wants to document placeholder syntax inline as guidance.
- D-19 trailer block encoding is now structural — future plans that reference cherry-pick provenance can copy the exact 6-line shape from this template (and the integration test will catch a regression on the lowercase 'a' or the two-Signed-off-by-lines invariant).
- The `git add -f` for `docs/cli/development/upstream-drift.mdx` does not change the repo's gitignore policy; future docs additions in that directory follow the same pattern (force-add to track alongside the 18 existing siblings).

## Self-Check: PASSED

Validated artifacts (filesystem):
- FOUND: .planning/templates/upstream-sync-quick.md
- FOUND: docs/cli/development/upstream-drift.mdx
- FOUND: .planning/PROJECT.md (modified — `## Upstream Parity Process` H2 inserted between Key Decisions and Evolution)
- FOUND: tests/integration/test_upstream_drift.sh (modified — Tests 8–11 appended before print_summary)
- ABSENT: docs/cli/development/upstream-drift.md (D-16 enforced; no stray .md form)

Validated commits (git log):
- FOUND: 6b6df9aa — feat(24-02): add upstream-sync quick-task template (DRIFT-02)
- FOUND: 27a7967c — feat(24-02): add Upstream Parity Process to PROJECT.md + .mdx runbook (DRIFT-02)
- FOUND: 4071023f — feat(24-02): extend test_upstream_drift.sh with DRIFT-02 assertions

Validated test outcomes:
- bash tests/integration/test_upstream_drift.sh: 43 run, 43 pass, 0 fail
- Read-only invariant: git status --porcelain pre/post test run is byte-identical
- Twin-parity diff at 3 ranges (Plan 24-01 inheritance): byte-identical (PowerShell 5.1 verified)
- Placeholder smoke test (T-24-05): 22 sed substitutions, zero remaining `{name}` markers in user-visible content
- D-19 trailer block: lowercase 'a' present, capital A absent, exactly 2 Signed-off-by lines

---
*Phase: 24-parity-drift-prevention*
*Completed: 2026-04-28*
