---
phase: 24-parity-drift-prevention
verified: 2026-04-27T00:00:00Z
status: passed
score: 16/16 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: none
  previous_score: n/a
  gaps_closed: []
  gaps_remaining: []
  regressions: []
---

# Phase 24: Parity-Drift Prevention Verification Report

**Phase Goal:** A maintainer opening a quick-task for the next upstream release (v0.41.0, v0.42.0, ...) has tooling that inventories the cross-platform commit range and a template that scaffolds a working sync PLAN.md in minutes, not hours.

**Verified:** 2026-04-27
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### ROADMAP Success Criteria (Phase Contract)

| # | Success Criterion | Status | Evidence |
|---|-------------------|--------|----------|
| SC1 | `scripts/check-upstream-drift.sh` against v0.37.1..v0.40.1 reproduces 260424-upr SUMMARY.md commit inventory (same commits, same categorization) | VERIFIED | Live invocation reproduced `total_unique_commits=56`, `by_category={profile:14, policy:11, package:9, proxy:10, audit:5, other:39}` matching SUMMARY narrative. Byte-identical to frozen fixture `tests/integration/fixtures/upstream-drift/v0.37.1__v0.40.1.json`. The 22-commit headline delta vs SUMMARY's "78 non-merge commits" is documented in `tests/integration/fixtures/upstream-drift/README.md` as informational only (D-11 path filter is canonical). |
| SC2 | Upstream-sync quick-task template invoked for hypothetical v0.41.0 yields functional PLAN.md skeleton with diff-range, conflict-file inventory, Windows retrofit checklist pre-populated | VERIFIED | Test 10 (placeholder smoke test) renders `.planning/templates/upstream-sync-quick.md` for v0.41.0 with 22 sed substitutions; zero `{[a-z_]+}` markers remain in user-visible content; rendered frontmatter `slug: 260501-upr-sync-v041`, `range: v0.40.1..v0.41.0` are valid. Template body contains `## Drift inventory`, `## Conflict-file inventory` (6 pre-populated rows), `## Windows-specific retrofit checklist` (7 per-feature checkboxes), `## Fork-divergence catalog` (5 entries). |
| SC3 | `docs/cli/development/upstream-drift.{md,mdx}` exists AND PROJECT.md references upstream-sync template | VERIFIED | `docs/cli/development/upstream-drift.mdx` exists at 157 lines, 7 H2 sections, Mintlify frontmatter present. `.md` form correctly absent (D-16 .mdx convention). `.planning/PROJECT.md` line 166 has `## Upstream Parity Process` H2 with cross-link `[docs/cli/development/upstream-drift.mdx](../docs/cli/development/upstream-drift.mdx)` (line 175) and reference to `.planning\templates\upstream-sync-quick.md` (line 171). Roadmap SC accepts `.md` OR `.mdx` via `{md,mdx}` braces. |

**Score:** 3/3 ROADMAP success criteria verified

### Plan-Level Observable Truths (must_haves.truths)

#### Plan 24-01 (DRIFT-01)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1.1 | `check-upstream-drift.sh --from v0.37.1 --to v0.40.1 --format json` `by_category` matches SUMMARY per-release breakdown | VERIFIED | Live run reproduced exact breakdown; byte-identical fixture diff |
| 1.2 | `check-upstream-drift.ps1 -From v0.40.0 -To v0.40.1 -Format json` byte-identical to bash twin | VERIFIED | Test 2 (twin-parity diff): `ps fixture diff (powershell.exe): v0.40.0__v0.40.1` PASS at all 3 ranges. PS 5.1 `[Console]::Out.Write + LF` discipline prevents CRLF drift |
| 1.3 | No `--from`/`--to` auto-detects last-synced upstream tag (v0.X), NOT fork v1.0/v2.0/v2.1 | VERIFIED | Test 3: auto-detected from-tag matches `^v0\.` pattern; uses `git tag --list 'v0.*' --merged HEAD --sort=-v:refname` |
| 1.4 | No `upstream` remote → exit code 1 + actionable hint (`git remote add upstream …`); never silently auto-add | VERIFIED | Test 4: tmp_repo without upstream remote returns non-zero exit + hint contains "git remote add upstream". Lines 86-93 in script: explicit fail-closed; no `git remote add` invocation anywhere in script |
| 1.5 | `make check-upstream-drift` dispatches via $(OS)==Windows_NT (pwsh fallback to powershell.exe on Windows; bash on Linux/macOS); ARGS passthrough works | VERIFIED (structural) | Makefile lines 79-89: `ifeq ($(OS),Windows_NT) … pwsh -NoProfile -File … else powershell.exe -NoProfile -File … else bash scripts/check-upstream-drift.sh $(ARGS)`. `make` not on this environment's PATH (matches SUMMARY-documented limitation); script-direct equivalent verified working. See "Behavioral Spot-Checks" below. |
| 1.6 | After running script, `git status --porcelain` is empty — read-only invariant | VERIFIED | Test 6 (read-only invariant): PASS. `git status --porcelain` post-test is byte-identical to pre-test. Static audit: scripts contain only `git remote get-url`, `git tag --list`, `git describe`, `git log` — no `git fetch`, `git config`, `git add`, `git commit`, `git push`, `git reset`, `git checkout` |

#### Plan 24-02 (DRIFT-02)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 2.1 | Maintainer copies template, fills `{from_tag}/{to_tag}/{commit_count}/{date}/{quick_slug}` placeholders → frontmatter-valid Markdown PLAN.md skeleton | VERIFIED | Test 10 PASS: 22 sed substitutions render valid PLAN.md with `slug: 260501-upr-sync-v041`, `range: v0.40.1..v0.41.0` |
| 2.2 | D-19 trailer block VERBATIM in 6 lines: Upstream-commit → Upstream-tag → Upstream-author (lowercase 'a') → Co-Authored-By → 2× Signed-off-by | VERIFIED | Tests 8 (7 PASS): line 222 of template `Upstream-author: {upstream_author_name} <{upstream_author_email}>` (lowercase 'a'); `grep -c '^Signed-off-by: '` = 2; `grep -F 'Upstream-Author'` returns no matches (capital-A regression check) |
| 2.3 | Fork-divergence catalog explicitly names `validate_path_within`, `load_production_trusted_root`, `ArtifactType::Plugin`/deferred enum variants, `hooks.rs` | VERIFIED | Test 9 (5 PASS): all 4 entries + deferred-enum-variants found via `grep -qF`. Template lines 146-198 contain 5 catalog entries with "Action on cherry-pick" sub-blocks |
| 2.4 | Windows-specific retrofit checklist with per-feature gate question (`#[cfg(target_os = "windows")]`) | VERIFIED | Template lines 117-134: 7 checkbox items + final "For each new feature without a Windows code path" item. `grep -qF '#[cfg(target_os = "windows")]'` PASS in Test 9 |
| 2.5 | PROJECT.md `## Upstream Parity Process` H2 with 4-step workflow + cross-link to .mdx | VERIFIED | Test 11 PASS: `.planning/PROJECT.md` line 166 has `## Upstream Parity Process` H2; lines 168-175 contain 4-step numbered workflow (Inventory → Scaffold → Cherry-pick → Verify Windows retrofit); cross-link `[docs/cli/development/upstream-drift.mdx](../docs/cli/development/upstream-drift.mdx)` at line 175. AWK ordering check confirms it precedes `## Evolution` |
| 2.6 | `docs/cli/development/upstream-drift.mdx` with Mintlify frontmatter + sections: running, formats, categorization, template integration, fixture regen | VERIFIED | Test 11 PASS: `title: Upstream Drift Check`, `description:` present; 7 H2 sections (Running the drift check, Output formats, Categorization rules, Using the output with the upstream-sync template, D-19 cherry-pick trailer block, Regenerating the test fixtures, See also) |
| 2.7 | Placeholder smoke test in `test_upstream_drift.sh` substitutes `{name}` placeholders, asserts zero remaining markers, frontmatter validity, D-19 trailer 6-line shape with two Signed-off-by | VERIFIED | Test 10 (6 PASS): HTML-comment-aware awk pre-strip; 22 sed substitutions; zero unfilled markers; frontmatter `---` present; rendered `Upstream-commit: abc12345`; rendered `Signed-off-by` count = 2 |

**Score:** 6/6 (Plan 24-01) + 7/7 (Plan 24-02) = 13/13 plan-level truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `scripts/check-upstream-drift.sh` | bash twin: tag-resolution + path-filtered git log + categorization + table/json output; contains `set -euo pipefail`, `git log --no-merges --numstat` | VERIFIED | 320 lines; line 18: `set -euo pipefail`; line 266: `git log --no-merges --numstat`; ref-validation regex at line 73 BEFORE git invocation at line 86 |
| `scripts/check-upstream-drift.ps1` | PowerShell twin: PS 5.1+7 compatible; UTF-8 console encoding pinned; `[Console]::OutputEncoding`, `ConvertTo-Json -Depth` | VERIFIED | 268 lines; line 29: `[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()`; line 241: `ConvertTo-Json -Depth 6 -Compress`; LF discipline via `[Console]::Out.Write + "`n"` |
| `tests/integration/test_upstream_drift.sh` | golden-fixture diff × 3 + twin-parity + missing-remote + tag-auto-detect + read-only invariant + placeholder smoke + cross-link assertions | VERIFIED | 443 lines; 43 assertions across 11 test groups; ALL PASS in live run |
| `tests/integration/fixtures/upstream-drift/v0.37.1__v0.40.1.json` | frozen JSON for canonical large-range; reproduces SUMMARY per-category | VERIFIED | 20702 bytes; total_unique_commits=56, by_category matches SUMMARY narrative; byte-identical to live script output |
| `tests/integration/fixtures/upstream-drift/v0.39.0__v0.40.0.json` | audit-cluster mid-range fixture | VERIFIED | 8989 bytes; live run byte-identical (Test 1 PASS) |
| `tests/integration/fixtures/upstream-drift/v0.40.0__v0.40.1.json` | small-range fixture (PS 5.1 single-element-array regression guard) | VERIFIED | 696 bytes; live run byte-identical (Test 1 PASS); PS 5.1 unwrap guard via `@()` wrapping |
| `tests/integration/fixtures/upstream-drift/README.md` | regen procedure + 22-commit informational delta vs SUMMARY headline | VERIFIED | 3597 bytes; 6 sections incl. regeneration commands and informational-delta reconciliation table |
| `Makefile` | `check-upstream-drift` target with $(OS)==Windows_NT dispatch + pwsh-then-powershell.exe fallback + ARGS passthrough | VERIFIED | Lines 9 (.PHONY), 79-89 (ifeq dispatch), 178-180 (help target Maintainer block) |
| `.planning/templates/upstream-sync-quick.md` | fillable-blanks template with single-brace `{name}` placeholders; D-19 trailer; conflict inventory; Windows checklist; fork-divergence catalog | VERIFIED | 257 lines; D-19 trailer at lines 219-226; 5 fork-divergence catalog entries; 7-item Windows retrofit checklist; 6-row conflict-file inventory |
| `docs/cli/development/upstream-drift.mdx` | Mintlify long-form runbook covering running, formats, categorization, template integration, regen | VERIFIED | 157 lines; 7 H2 sections; Mintlify `title:` + `description:` frontmatter |
| `.planning/PROJECT.md` | new `## Upstream Parity Process` H2 between Key Decisions and Evolution; cross-link to .mdx | VERIFIED | Line 166: H2; lines 168-175: 4-step workflow; line 175: cross-link to .mdx; section ordering Key Decisions (144) → Upstream Parity Process (166) → Evolution (177) |

**All 11 artifacts: VERIFIED**

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `scripts/check-upstream-drift.{sh,ps1}` | `git log --no-merges --numstat` with D-11 path filter + Windows-only excludes | subprocess invocation | WIRED | `.sh` line 266 + `.ps1` line 116-125; `':(exclude)*_windows.rs'` and `':(exclude)crates/nono-cli/src/exec_strategy_windows/'` present |
| `tests/integration/test_upstream_drift.sh` | `tests/integration/fixtures/upstream-drift/*.json` | `diff` against golden fixture | WIRED | Test 1 line 59: `diff <(echo "$actual") "$expected"` × 3 ranges |
| `tests/integration/test_upstream_drift.sh` | `scripts/check-upstream-drift.sh` AND `.ps1` | twin-parity dual-execution diff | WIRED | Test 2 lines 78-88: invokes both runners against same fixture; PS auto-detects `pwsh` then `powershell.exe` |
| `Makefile (check-upstream-drift)` | `scripts/check-upstream-drift.{sh,ps1}` | $(OS)==Windows_NT dispatch | WIRED | Makefile line 79: `ifeq ($(OS),Windows_NT)`; pwsh-with-powershell.exe fallback at lines 81-85 |
| `.planning/PROJECT.md § Upstream Parity Process` | `.planning/templates/upstream-sync-quick.md` | step 2 (`copy template`) | WIRED | PROJECT.md line 171: explicit reference |
| `.planning/PROJECT.md § Upstream Parity Process` | `docs/cli/development/upstream-drift.mdx` | runbook cross-link | WIRED | PROJECT.md line 175: markdown link `[docs/cli/development/upstream-drift.mdx](../docs/cli/development/upstream-drift.mdx)` |
| `docs/cli/development/upstream-drift.mdx` | `.planning/templates/upstream-sync-quick.md` | "Using the output with the upstream-sync template" section | WIRED | .mdx lines 88-101: full sub-section instructing template copy + placeholder fill |
| `.planning/templates/upstream-sync-quick.md § Drift inventory` | `make check-upstream-drift` (Plan 24-01) | manual instruction (D-14) | WIRED | Template lines 6-8 + 33-34 + 49: explicit `make check-upstream-drift > drift.json` instruction; does NOT auto-include output (D-14 maintainer-curated) |
| `tests/integration/test_upstream_drift.sh` | `.planning/templates/upstream-sync-quick.md` | placeholder smoke test (sed substitution) | WIRED | Test 10 lines 289-312: 22 sed substitutions render the template; subsequent grep assertions on rendered output |

**All 9 key links: WIRED**

### Data-Flow Trace (Level 4)

This phase produces tooling/template artifacts (no UI components rendering dynamic data). The data-flow analog here is the pipeline `git log → script → JSON → fixture diff`. Verified end-to-end:

| Artifact | Data Source | Produces Real Data | Status |
|----------|-------------|--------------------|--------|
| `check-upstream-drift.sh` JSON output | `git log --no-merges --numstat ${FROM}..${TO} -- <path-filter>` | YES (56 commits for v0.37.1..v0.40.1, matches SUMMARY) | FLOWING |
| `check-upstream-drift.ps1` JSON output | same git log invocation | YES (byte-identical to bash twin) | FLOWING |
| Fixture files | regenerated from script output | YES (live diff = 0) | FLOWING |
| Rendered template (sed-substituted) | template + 22 placeholder values | YES (zero remaining markers) | FLOWING |

### Behavioral Spot-Checks

| # | Behavior | Command | Result | Status |
|---|----------|---------|--------|--------|
| 1 | Integration test suite passes | `bash tests/integration/test_upstream_drift.sh` | 43 run, 43 passed, 0 failed | PASS |
| 2 | Script reproduces SUMMARY's per-category breakdown | `bash scripts/check-upstream-drift.sh --from v0.37.1 --to v0.40.1 --format json` | total_unique_commits=56, profile=14, policy=11, package=9, proxy=10, audit=5, other=39 (matches SUMMARY narrative) | PASS |
| 3 | Live script output is byte-identical to frozen fixture | `diff <(bash check-upstream-drift.sh --from v0.37.1 --to v0.40.1 --format json) tests/integration/fixtures/upstream-drift/v0.37.1__v0.40.1.json` | empty diff (byte-identical) | PASS |
| 4 | Read-only invariant after live invocation | `git status --porcelain` post all script + test invocations | empty (clean working tree) | PASS |
| 5 | Capital-A `Upstream-Author` regression check | `grep -F 'Upstream-Author' docs/cli/development/upstream-drift.mdx` and template | exit 1 (no match) | PASS |
| 6 | All 6 commits documented in SUMMARYs exist in git | `git log -1 --oneline 1abf04a7 0834aa66 c3e24522 6b6df9aa 27a7967c 4071023f` | all 6 resolve with expected feat() messages | PASS |
| 7 | Makefile dispatch | `make check-upstream-drift ARGS="--from v0.37.1 --to v0.40.1 --format json"` | SKIP — `make` not on PATH in this environment (matches SUMMARY-documented limitation: "make not on Windows MSYS2 PATH"). Structural verification via Makefile grep confirms ifeq($(OS),Windows_NT) dispatch + pwsh→powershell.exe fallback + bash else-branch + ARGS passthrough + .PHONY entry | SKIP (structural verified) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| DRIFT-01 | 24-01-PLAN.md | `check-upstream-drift` reports cross-platform gap; PS+bash twin or cross-platform tool; output groups by category; read-only; reproduces 260424-upr inventory; structured output; documented in `docs/cli/development/upstream-drift.{md,mdx}` | SATISFIED | All 3 acceptance criteria met: (1) 56-commit reproduction LIVE-VERIFIED, (2) JSON + table outputs both implemented and tested, (3) `docs/cli/development/upstream-drift.mdx` exists (the .md/.mdx braced acceptance permits .mdx) |
| DRIFT-02 | 24-02-PLAN.md | Reusable template scaffolding upstream-sync quick task: diff-range, cherry-pick D-19 trailer, conflict-file inventory, Windows retrofit checklist; PROJECT.md references it; dry-run produces sensible PLAN.md skeleton | SATISFIED | All 3 acceptance criteria met: (1) `.planning/templates/upstream-sync-quick.md` exists at agreed location, (2) PROJECT.md § Upstream Parity Process cross-links template at line 171, (3) Test 10 dry-run for hypothetical v0.41.0 produces valid PLAN.md skeleton with zero unfilled placeholders in user-visible content |

**Orphan check:** REQUIREMENTS.md line 330-331 maps DRIFT-01 + DRIFT-02 to Phase 24. Both claimed by plans (24-01 and 24-02 frontmatter `requirements:` field). No orphans.

### Anti-Patterns Found

Scanned files identified from SUMMARY key-files sections:

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | — | — | — | Scanned all 11 modified/created files for TODO/FIXME/PLACEHOLDER comments, empty handlers, hardcoded empties feeding rendering paths, console.log-only impls. No anti-patterns found in user-visible content. |

Note on intentional `{placeholder}` markers: The template (`.planning/templates/upstream-sync-quick.md`) intentionally contains `{name}` placeholders by design — these are fill-in fields, not stubs. The HTML-comment-stripping awk pre-filter in Test 10 correctly distinguishes intentional placeholders inside `<!-- guidance -->` blocks (illustrative format examples) from unfilled fields in user-visible content. This is a documented and tested invariant (24-02 SUMMARY § "HTML-comment-aware placeholder smoke test").

### Spot-check: SUMMARY claims vs disk

| SUMMARY claim | On disk | Match |
|---------------|---------|-------|
| 24-01 SUMMARY: 8 files modified (7 created + 1 modified) | scripts/check-upstream-drift.sh, .ps1, test_upstream_drift.sh, 3 fixtures, fixtures/README.md, Makefile = 7 created + Makefile modified | YES |
| 24-02 SUMMARY: 4 files (2 created + 2 modified) | upstream-sync-quick.md, upstream-drift.mdx (created); PROJECT.md, test_upstream_drift.sh (modified) | YES |
| 24-02 SUMMARY: "test went 13 → 43 assertions" | Test suite reports `Tests run: 43` | YES |
| 24-01 SUMMARY: by_category profile=14, policy=11, package=9, proxy=10, audit=5, other=39 | Live script output: identical | YES |
| 24-01 SUMMARY: "v0.40.0..v0.40.1 = 2 commits" | Fixture file v0.40.0__v0.40.1.json = 696 bytes (small range; 2 commits) | YES |
| 24-02 SUMMARY: "lowercase 'a' in Upstream-author" | Template line 222 + .mdx line 119 both lowercase | YES |
| 24-02 SUMMARY: "TWO Signed-off-by lines" | grep -c = 2 in template | YES |
| 24-02 SUMMARY: "all 43 assertions pass" | Live run: 43/43 PASS | YES |
| 6 commit hashes claimed | All resolve via git log -1 with expected subject lines | YES |

### Human Verification Required

None. All success criteria, plan-level truths, artifacts, key links, and behavioral spot-checks were programmatically verified or live-reproduced. The single SKIP item (`make` dispatch) is documented as a known environmental limitation (not a deliverable gap) and is structurally verified via Makefile inspection plus script-direct equivalence — same path the plan executor used.

### Gaps Summary

No gaps. Phase 24 fully achieves its goal:

- A maintainer can run `bash scripts/check-upstream-drift.sh --from <ref> --to <ref> --format json` (or `make check-upstream-drift ARGS=...` on a host with `make`) and get a categorized commit inventory in seconds — replacing commit-by-commit narrative review with mechanical path-filter + categorization lookup.
- A maintainer can copy `.planning/templates/upstream-sync-quick.md`, fill 22 single-brace placeholders, and end up with a frontmatter-valid PLAN.md skeleton containing the D-19 cherry-pick trailer block (verbatim 6-line shape), conflict-file inventory (6 pre-populated rows from Phase 22-03 + Phase 20 sync experience), Windows retrofit checklist (7 per-feature checkboxes), and fork-divergence catalog (5 entries naming `validate_path_within`, `load_production_trusted_root`, deferred enum variants, `hooks.rs`, Windows-only file globs).
- The integration test (43 assertions) regression-tests every above invariant, locking byte-for-byte parity between bash + PowerShell twins via golden fixtures and locking the D-19 trailer shape, fork-divergence catalog naming, placeholder smoke, and bidirectional cross-links between PROJECT.md ↔ template ↔ .mdx ↔ scripts.

The next maintainer absorbing v0.41.0+ has tooling that inventories the cross-platform commit range and a template that scaffolds a working sync PLAN.md in minutes, not hours — exactly the phase goal.

---

*Verified: 2026-04-27*
*Verifier: Claude (gsd-verifier)*
