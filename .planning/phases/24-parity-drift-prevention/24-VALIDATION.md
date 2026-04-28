---
phase: 24
slug: parity-drift-prevention
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-27
---

# Phase 24 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source of truth for sampling depth: `24-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Bash + diff (no external test framework); reuses `tests/integration/test_*.sh` pattern |
| **Config file** | none — runner is `tests/run_integration_tests.sh` (auto-discovers `test_*.sh`) |
| **Quick run command** | `bash tests/integration/test_upstream_drift.sh` |
| **Full suite command** | `bash tests/run_integration_tests.sh` |
| **Estimated runtime** | ~5–10 seconds (a handful of `git log` invocations + diffs) |

---

## Sampling Rate

- **After every task commit:** Run `bash tests/integration/test_upstream_drift.sh`
- **After every plan wave:** Run `bash tests/run_integration_tests.sh`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** ~10 seconds

**Nyquist Dimension 8 rationale:** Tag-resolution (4 edge cases), numstat parsing (4 edge cases — binary/rename/merge/unicode), and categorization (6 categories) cannot be covered by a single fixture. Plan provisions **3 fixtures + 3 unit-style assertions = 6 sampling points** to catch a regression that flips one path-prefix in the lookup table.

---

## Per-Task Verification Map

> Populated by the planner once tasks are drafted. Each row maps a task ID to a concrete automated command. Source: `24-RESEARCH.md` § "Phase Requirements -> Test Map".

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 24-XX-YY | XX | N | DRIFT-01 | T-24-01 (V5) | refs validated, no shell interpolation | golden-fixture diff | `bash tests/integration/test_upstream_drift.sh` | ❌ W0 | ⬜ pending |
| 24-XX-YY | XX | N | DRIFT-01 | — | twin-script byte-identical JSON | dual-execution diff | embedded in `test_upstream_drift.sh` (parity check) | ❌ W0 | ⬜ pending |
| 24-XX-YY | XX | N | DRIFT-01 | — | tag auto-detect picks `v0.X` not fork `v2.X` | unit-style grep | embedded (assert reported "from" matches `^v0\.`) | ❌ W0 | ⬜ pending |
| 24-XX-YY | XX | N | DRIFT-01 | T-24-02 (V14) | missing `upstream` remote → exit 1 + hint | exit-code check | embedded (run in temp clone) | ❌ W0 | ⬜ pending |
| 24-XX-YY | XX | N | DRIFT-01 | — | `--format table` produces grouped output | grep assertion | embedded (assert category headers `## profile`, `## audit`) | ❌ W0 | ⬜ pending |
| 24-XX-YY | XX | N | DRIFT-01 | — | single-commit range emits JSON array (PS 5.1 footgun) | golden-fixture diff | embedded (range 3, single-commit subrange) | ❌ W0 | ⬜ pending |
| 24-XX-YY | XX | N | DRIFT-02 | — | template file exists | file-existence | `[[ -f .planning/templates/upstream-sync-quick.md ]]` | ❌ W0 | ⬜ pending |
| 24-XX-YY | XX | N | DRIFT-02 | — | template has full D-19 trailer block (6 lines) | grep assertion | `grep -E '^Upstream-commit: \{' …` + `grep -c '^Signed-off-by: ' = 2` | ❌ W0 | ⬜ pending |
| 24-XX-YY | XX | N | DRIFT-02 | — | placeholder smoke test (`{name}` substitutes cleanly to valid frontmatter) | render+grep | embedded in `test_upstream_drift.sh` | ❌ W0 | ⬜ pending |
| 24-XX-YY | XX | N | DRIFT-02 | — | PROJECT.md references the template + docs file | grep assertion | `grep -F '.planning/templates/upstream-sync-quick.md' PROJECT.md` and `grep -F 'docs/cli/development/upstream-drift' PROJECT.md` | ❌ W0 | ⬜ pending |
| 24-XX-YY | XX | N | DRIFT-02 | — | docs file exists at `.mdx` (not `.md`) per D-16 | file-existence | `[[ -f docs/cli/development/upstream-drift.mdx ]]` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/integration/test_upstream_drift.sh` — runs fixtures + parity check + smoke test + assertions
- [ ] `tests/integration/fixtures/upstream-drift/v0.37.1__v0.40.1.json` — canonical large-range fixture (acceptance #1)
- [ ] `tests/integration/fixtures/upstream-drift/v0.39.0__v0.40.0.json` — audit-cluster mid-range fixture
- [ ] `tests/integration/fixtures/upstream-drift/v0.40.0__v0.40.1.json` — small-range (3-commit) fixture for PS 5.1 single-element-array test
- [ ] `tests/integration/fixtures/upstream-drift/README.md` — explains regeneration procedure + documents D-11 exclusions (the 22-commit informational delta vs SUMMARY headline)

No framework install needed; bash + diff + grep + sed + git already on box.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Twin-script parity on Windows-only PowerShell 5.1 (no `pwsh`) | DRIFT-01 | CI typically runs on Linux/Mac; PS 5.1 is Windows-only. Maintainer must validate on a Windows box at least once before phase sign-off. | Run `powershell.exe -NoProfile -File scripts/check-upstream-drift.ps1 -From v0.37.1 -To v0.40.1 -Format json > /tmp/win.json` then `diff <(bash scripts/check-upstream-drift.sh --from v0.37.1 --to v0.40.1 --format json) /tmp/win.json` — must be empty. |
| Template-driven sync produces a working PLAN.md (success criterion #2) | DRIFT-02 | Acceptance is "functional PLAN.md skeleton" — only verifiable by a maintainer reading the rendered output. | After Wave 3 completes, copy `.planning/templates/upstream-sync-quick.md` to a scratch dir, fill placeholders for a hypothetical v0.41.0 sync, run the placeholder smoke test, then have a reviewer read it for "would I know what to do next?" |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (5 files: 1 test + 3 fixtures + 1 README)
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
