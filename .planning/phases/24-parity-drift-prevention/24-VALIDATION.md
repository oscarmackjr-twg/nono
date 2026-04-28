---
phase: 24
slug: parity-drift-prevention
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-27
approved: 2026-04-27
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

> Populated 2026-04-27 from 24-01-PLAN.md / 24-02-PLAN.md. Each row maps a task to a concrete automated command. Source: `24-RESEARCH.md` § "Phase Requirements -> Test Map".

**Plan 24-01 task layout:** T1 = twin-script skeleton + JSON emission; T2 = categorization + table format + twin parity; T3 = Makefile + test runner + 3 fixtures + README.
**Plan 24-02 task layout:** T1 = template (D-13 + D-19 trailer + Windows retrofit checklist); T2 = PROJECT.md section + `.mdx` docs + cross-links; T3 = extend `test_upstream_drift.sh` with placeholder smoke + cross-link assertions.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 24-01-01 | 01 | 1 | DRIFT-01 | T-24-01 (V5) | refs validated against `^[A-Za-z0-9._/-]+$`, `--` separator on git invocations, no shell interpolation | unit-style assertion | embedded in `tests/integration/test_upstream_drift.sh` (Test 1: ref-injection rejected with `; touch /tmp/pwn`) | ❌ W0 | ⬜ pending |
| 24-01-01 | 01 | 1 | DRIFT-01 | T-24-02 (V14) | missing `upstream` remote → exit 1 with actionable hint (no auto-add) | exit-code check | embedded (Test 5: temp clone with no upstream remote) | ❌ W0 | ⬜ pending |
| 24-01-01 | 01 | 1 | DRIFT-01 | T-24-03 (V14) | tag auto-detect picks `v0.X` (upstream pattern), not fork `v2.X` | unit-style grep | embedded (Test 4: assert reported "from" tag matches `^v0\.`) | ❌ W0 | ⬜ pending |
| 24-01-01 | 01 | 1 | DRIFT-01 | T-24-03 (V14) | PS 5.1 console UTF-8 pinned + `@()` array wrap on `ConvertTo-Json` | golden-fixture diff | embedded (Test 3: range 3 single-commit subrange v0.40.0..v0.40.1, asserts JSON is `[...]` not `{...}`) | ❌ W0 | ⬜ pending |
| 24-01-01 | 01 | 1 | DRIFT-01 | T-24-04 (V5) | bash hand-rolled JSON escape covers `\\`, `"`, `\n`, `\r`, `\t` | parse validation | `bash scripts/check-upstream-drift.sh --from v0.37.1 --to v0.40.1 --format json \| python3 -m json.tool > /dev/null` exits 0 | ❌ W0 | ⬜ pending |
| 24-01-02 | 01 | 1 | DRIFT-01 | — | `--format table` produces grouped output with category headers | grep assertion | embedded (Test 7: assert output contains `## profile`, `## policy`, `## package`, `## proxy`, `## audit`, `## other`) | ❌ W0 | ⬜ pending |
| 24-01-02 | 01 | 1 | DRIFT-01 | — | twin-script byte-identical JSON output | dual-execution diff | embedded (Test 8: `diff <(bash …) <(powershell.exe -NoProfile -File … )` is empty) | ❌ W0 | ⬜ pending |
| 24-01-03 | 01 | 1 | DRIFT-01 | — | reproduces v0.37.1..v0.40.1 inventory (per-category match — acceptance #1) | golden-fixture diff | embedded (Test 1: `diff fixtures/upstream-drift/v0.37.1__v0.40.1.json <(bash scripts/check-upstream-drift.sh --from v0.37.1 --to v0.40.1 --format json)` is empty) | ❌ W0 | ⬜ pending |
| 24-01-03 | 01 | 1 | DRIFT-01 | — | mid-range audit-cluster fixture matches | golden-fixture diff | embedded (Test 2: same shape, range v0.39.0..v0.40.0) | ❌ W0 | ⬜ pending |
| 24-01-03 | 01 | 1 | DRIFT-01 | T-24-02 (V14) | read-only invariant — `git status --porcelain` empty before/after script run | shell exit-code | embedded (Test 6: snapshot+diff `git status --porcelain` pre/post) | ❌ W0 | ⬜ pending |
| 24-01-03 | 01 | 1 | DRIFT-01 | — | `make check-upstream-drift` Makefile target dispatches to platform-correct script | smoke run | `make check-upstream-drift FROM=v0.40.0 TO=v0.40.1 FORMAT=json` exits 0 with non-empty output | ❌ W0 | ⬜ pending |
| 24-02-01 | 02 | 2 | DRIFT-02 | — | template file exists at canonical path | file-existence | `[[ -f .planning/templates/upstream-sync-quick.md ]]` | ❌ W0 | ⬜ pending |
| 24-02-01 | 02 | 2 | DRIFT-02 | — | template has full D-19 trailer block (6 lines, lowercase `a` in `Upstream-author`, exactly 2 `Signed-off-by`) | grep assertion | `grep -E '^Upstream-commit: \{' .planning/templates/upstream-sync-quick.md` exits 0; `grep -E '^Upstream-author: \{' …` exits 0 (lowercase `a`); `grep -c '^Signed-off-by: ' …` outputs `2` | ❌ W0 | ⬜ pending |
| 24-02-01 | 02 | 2 | DRIFT-02 | — | template includes Windows retrofit checklist + fork-divergence catalog (incl. `validate_path_within`, `ArtifactType::Plugin`) | grep assertion | `grep -F 'validate_path_within' .planning/templates/upstream-sync-quick.md` and `grep -F 'ArtifactType::Plugin' …` and `grep -F 'Windows retrofit' …` all exit 0 | ❌ W0 | ⬜ pending |
| 24-02-02 | 02 | 2 | DRIFT-02 | — | PROJECT.md adds `## Upstream Parity Process` section (~10 lines, links template + .mdx) | grep assertion | `grep -F '## Upstream Parity Process' .planning/PROJECT.md` and `grep -F '.planning/templates/upstream-sync-quick.md' .planning/PROJECT.md` and `grep -F 'docs/cli/development/upstream-drift' .planning/PROJECT.md` all exit 0 | ❌ W0 | ⬜ pending |
| 24-02-02 | 02 | 2 | DRIFT-02 | — | docs file exists at `.mdx` per D-16, NOT `.md` | file-existence | `[[ -f docs/cli/development/upstream-drift.mdx ]] && [[ ! -f docs/cli/development/upstream-drift.md ]]` | ❌ W0 | ⬜ pending |
| 24-02-02 | 02 | 2 | DRIFT-02 | — | `.mdx` cross-links to template + script | grep assertion | `grep -F 'check-upstream-drift' docs/cli/development/upstream-drift.mdx` and `grep -F 'upstream-sync-quick.md' docs/cli/development/upstream-drift.mdx` both exit 0 | ❌ W0 | ⬜ pending |
| 24-02-03 | 02 | 2 | DRIFT-02 | T-24-05 (V14) | placeholder smoke test — substituting `{name}` markers yields valid GSD frontmatter; unfilled markers fail the test | render+grep | embedded extension to `test_upstream_drift.sh` (Test 9: copy template, sed-substitute, assert `head -1` is `^---$` + `slug:` + `date:` present, AND no `\{[a-z_]+\}` remain) | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

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

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (verified 2026-04-27 — all 6 plan tasks reference embedded assertions in `tests/integration/test_upstream_drift.sh`)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (every task has at least one automated check; per-task map above shows ≥1 row per task)
- [x] Wave 0 covers all MISSING references (5 files enumerated above; created by Plan 24-01 Task 3)
- [x] No watch-mode flags (no `--watch`, `cargo watch`, `nodemon`, etc. in any verify command)
- [x] Feedback latency < 15s (estimated runtime ~5–10s for full integration test suite)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-04-27
