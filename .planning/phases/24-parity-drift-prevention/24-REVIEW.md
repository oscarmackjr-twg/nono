---
phase: 24-parity-drift-prevention
reviewed: 2026-04-28T04:11:37Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - scripts/check-upstream-drift.sh
  - scripts/check-upstream-drift.ps1
  - tests/integration/test_upstream_drift.sh
  - Makefile
  - .planning/templates/upstream-sync-quick.md
  - docs/cli/development/upstream-drift.mdx
  - .planning/PROJECT.md
findings:
  critical: 0
  high: 0
  medium: 2
  low: 5
  info: 4
  total: 11
status: issues_found
---

# Phase 24: Code Review Report — parity-drift-prevention

**Reviewed:** 2026-04-28T04:11:37Z
**Depth:** standard
**Files Reviewed:** 8 (7 source + 1 modified)
**Status:** issues_found (2 medium, 5 low, 4 info — all non-blocking)

## Summary

Phase 24 ships a maintainer drift-detection toolchain (twin bash + PowerShell scripts, 43-assertion integration test, Makefile target, GSD template, Mintlify runbook, PROJECT.md cross-link). The work is well-scoped, defensively coded, and the threat model is fully exercised by the test suite — every named threat (T-24-01 through T-24-05) is covered by at least one assertion, and the read-only invariant + ref-injection rejection are explicitly tested.

Code quality is high. The twin scripts are deliberately structured to produce byte-identical JSON output (the discovered PS-CRLF footgun is fixed and documented). Security posture is correct: ref-injection regex pre-validation runs BEFORE any git invocation in BOTH twins, the `--` separator provides defense-in-depth, the upstream-remote check is fail-closed, and the script is read-only over `.git`.

**No critical or high-severity issues found.** Two medium findings concern parsing edge cases that are unlikely in practice (brace-style git renames, single-line HTML-comment placeholder smoke-test gap). Low/info findings are documentation polish, redundant pattern entries, and minor robustness opportunities. None block phase sign-off.

### Threat-model coverage matrix

| Threat | Mitigation site | Test assertion |
|---|---|---|
| T-24-01 ref injection | `validate_ref` (sh L71–79) / `Test-RefSyntax` (ps L35–43) — runs BEFORE any git invocation | Test 7 (line 187–193) |
| T-24-02 read-only invariant | No fetch, no `git config` write, no working-tree mod; `--` separator on git log | Test 6 (line 174–182) |
| T-24-03 PS CRLF mojibake | `[Console]::OutputEncoding = UTF8` (ps L29–30) | Tests 1+2 (byte-for-byte fixture diff) |
| T-24-04 JSON metachar in subject | `emit_json_string` 5-substitution escape, backslash FIRST (sh L155–163) | Test 1 (golden-fixture round-trip with real upstream subjects) |
| T-24-05 unfilled `{name}` placeholder | sed substitution + awk-stripped grep (test L289–336) | Test 10 (line 273–336) |

All five threats have explicit assertions.

---

## Medium Issues

### MD-01: Brace-style git renames mis-categorized in both twins

**Files:** `scripts/check-upstream-drift.sh:261–263`, `scripts/check-upstream-drift.ps1:210–212`

**Issue:** `git log --numstat` emits two rename formats:
1. Simple: `12<TAB>34<TAB>old/path.rs => new/path.rs` (full-path arrow)
2. Brace: `12<TAB>34<TAB>src/{old => new}/path.rs` (brace-elided common prefix)

Both twins use the strip-after-`" => "` pattern, which works for form (1) but mangles form (2). For input `src/{old => new}/path.rs`:
- bash `${local_f##* => }` produces `new}/path.rs`
- PS `($f -split ' => ')[-1]` produces `new}/path.rs`

The trailing `}` and the dropped prefix make path-prefix categorization unreliable for brace-renamed files. They would not match `crates/nono-cli/src/profile/` and would fall through to `other`. The bug is twin-symmetric (so fixture diffs still pass), but a real upstream rename of `crates/nono-cli/src/{old_module => new_module}/foo.rs` would be silently miscategorized.

**Why this is medium not low:** path-categorization correctness is the core deliverable; silent miscategorization of any commit is a false-clean. The likelihood is moderate (renames are common during refactoring).

**Fix:** Reconstruct the post-rename path by replacing the `{old => new}` segment with `new`. Verified equivalent on both runtimes:

```bash
# bash — handle both forms
local_f="$c3"
if [[ "$local_f" == *'{'*' => '*'}'* ]]; then
    # Brace form: src/{old => new}/path → src/new/path
    local prefix="${local_f%%\{*}"
    local rest="${local_f#*\{}"        # old => new}/path
    local newpart="${rest#* => }"      # new}/path
    newpart="${newpart%%\}*}"          # new
    local suffix="${rest#*\}}"         # /path
    local_f="${prefix}${newpart}${suffix}"
elif [[ "$local_f" == *' => '* ]]; then
    local_f="${local_f##* => }"
fi
```

```powershell
# PowerShell equivalent
if ($f -match '\{.* => .*\}') {
    $f = [regex]::Replace($f, '\{[^}]* => ([^}]*)\}', '$1')
} elseif ($f -match ' => ') {
    $f = ($f -split ' => ')[-1]
}
```

Add a regression fixture exercising at least one brace-rename path (or, if no brace renames exist in `v0.37.1..v0.40.1`, document the limitation in `fixtures/README.md` until one appears).

---

### MD-02: Placeholder smoke test misses same-line `<!-- ... --> {placeholder}` cases

**File:** `tests/integration/test_upstream_drift.sh:322–327`

**Issue:** The awk pre-filter strips lines that contain `<!--` until it sees `-->`. For a hypothetical line of the form `foo <!-- guidance --> {real_placeholder}`, the entire line is suppressed by `!in_comment{print}` (because `in_comment=1` was set on the same line before the `print` rule evaluated, AND `-->` resets it AFTER the print decision). Consequence: a `{real_placeholder}` on the SAME line after a closing `-->` would be silently dropped from the smoke check, allowing an unfilled placeholder to slip through.

The current template (`upstream-sync-quick.md`) does not contain any such same-line construct, so this is latent rather than active. It becomes a real risk when a future maintainer adds inline guidance like:

```markdown
- Tags covered: <!-- e.g., v0.41.0 --> {tag_list}
```

**Why this is medium not low:** the smoke test is the structural enforcer of T-24-05; a false negative defeats the threat mitigation.

**Fix:** Strip comment spans within a line BEFORE the line-level filter runs. Example sed pre-pass:

```bash
stripped=$(sed 's/<!--[^>]*-->//g' "$SMOKE_TMPDIR/PLAN.md" \
  | awk '
      /<!--/{in_comment=1}
      !in_comment{print}
      /-->/{in_comment=0}
  ')
```

Or use a single-pass awk that handles both inline and multi-line comments. Add a regression case in the template by inserting one line of the form `... <!-- inline guidance --> trailing-content` and verifying the smoke test still passes.

---

## Low Issues

### LO-01: Auto-detected refs are not re-validated against the injection regex

**Files:** `scripts/check-upstream-drift.sh:96–110`, `scripts/check-upstream-drift.ps1:57–71`

**Issue:** `validate_ref` runs only on the user-supplied `--from`/`--to`. When auto-detection is taken (lines 96 + 106 in bash; lines 58 + 66 in PS), the resolved tag from `git tag --list 'v0.*'` or `git describe --tags --abbrev=0 upstream/main` is used directly without re-running the syntax check.

This is **not currently exploitable** because:
1. `git tag --list 'v0.*'` constrains the prefix to `v0.`
2. Git tag names cannot contain spaces, semicolons, or shell metacharacters (git's own ref-name validation enforces `^[A-Za-z0-9._/-]+` as a subset)
3. The `--` separator on the `git log` call provides defense-in-depth

But the trust boundary is not symmetric: user input is validated, system-resolved input is trusted. A future change that broadens the tag glob (e.g., `git tag --list '*'`) or accepts user-controlled `--list <pattern>` would silently widen the attack surface.

**Fix:** Apply `validate_ref`/`Test-RefSyntax` to the auto-detected values as well. Defense-in-depth is cheap and matches the project's "fail secure" principle (CLAUDE.md).

```bash
if [[ -z "$FROM_REF" ]]; then
    FROM_REF=$(git tag --list 'v0.*' --merged HEAD --sort=-v:refname | head -n1)
    validate_ref "$FROM_REF"  # add
    ...
```

---

### LO-02: Native-command stderr suppression on `git remote get-url` may swallow CRITICAL git failures

**Files:** `scripts/check-upstream-drift.sh:86`, `scripts/check-upstream-drift.ps1:49`

**Issue:** The "is upstream remote configured?" probe uses `git remote get-url upstream >/dev/null 2>&1` (bash) and `git remote get-url upstream 2>$null` (PS). Both swallow stderr. The intended failure (no remote) is correctly differentiated by `$?`/`$LASTEXITCODE`, but a less-likely failure (e.g., `.git/config` corruption, missing `git` binary, repo not a git dir) produces the same exit code without preserving the underlying message. The user sees the "add upstream remote" hint even when the actual problem is unrelated.

**Fix:** Either (a) check `git rev-parse --is-inside-work-tree` first to distinguish "not a git dir" from "no upstream remote", or (b) capture stderr and include the first line in the diagnostic when it doesn't match the expected "no such remote" shape. Low priority — the current behavior is fine in 99% of cases.

---

### LO-03: PowerShell categorization table has a redundant entry

**File:** `scripts/check-upstream-drift.ps1:99–100`

**Issue:** Lines 99 and 100 declare:

```powershell
'^crates/nono-cli/src/package'        { return 'package' }
'^crates/nono-cli/src/package_cmd\.rs$' { return 'package' }
```

The first regex (no end anchor) is a strict superset of the second (anchored to `package_cmd.rs`). The second never matches anything the first wouldn't already match. Functionally harmless, but invites confusion when the table is updated. The bash twin has the same pattern at line 138 (`crates/nono-cli/src/package*|crates/nono-cli/src/package_cmd.rs|...`).

**Fix:** Remove the redundant `package_cmd.rs` arm in BOTH twins. Alternatively, if explicitness is preferred (defense against accidental table edits), add a comment: `# package_cmd.rs is already covered by the previous arm; kept for explicit documentation`.

---

### LO-04: `.mdx` categorization-rules table abbreviates `audit_attestation*` ambiguously

**File:** `docs/cli/development/upstream-drift.mdx:80`

**Issue:** The table reads:

```
| `crates/nono/src/audit/` or `audit_attestation*` or `crates/nono-cli/src/audit*` | `audit` |
```

The bare `audit_attestation*` could be misread as matching `*/audit_attestation*` anywhere in the tree, when in fact the script only matches `crates/nono/src/audit_attestation*`. A future maintainer reading only the docs (without consulting the script) might add a top-level `audit_attestation/` directory expecting it to categorize as `audit`.

**Fix:** Spell out the full prefix: `` `crates/nono/src/audit_attestation*` ``. Same change applies to the `package*` row (line 78) for symmetry.

---

### LO-05: `set -uo pipefail` in test runner intentionally omits `-e`, but the inline subshell uses `set +e` redundantly

**File:** `tests/integration/test_upstream_drift.sh:13, 22, 123`

**Issue:** The outer script uses `set -uo pipefail` (line 13), then `source test_helpers.sh` flips `-e` on, then line 22 explicitly `set +e`. The Test 4 subshell (line 122) starts its own `set +e` (line 123). Inside a subshell, the outer shell's options are inherited, so this is redundant. Not a bug — just dead defense.

**Fix:** Optional. Remove line 123 with a comment, or keep for explicit-documentation value. Either is acceptable.

---

## Info

### IN-01: `validate_ref` regex `^[A-Za-z0-9._/-]+$` is correct but slightly narrower than git's tag-name spec

**Files:** `scripts/check-upstream-drift.sh:73`, `scripts/check-upstream-drift.ps1:37`

**Observation:** Git allows additional characters in ref names (e.g., `+`, `=`, `:` in some contexts; full spec in `git-check-ref-format(1)`). The regex deliberately uses a strict allowlist that covers all real-world tag naming (`vX.Y.Z`, `vX.Y.Z-rc1`, `release/X.Y`, etc.) and rejects shell-metacharacter-bearing inputs. This is the correct conservative choice for security — but if a future maintainer adds support for `--from upstream/feature+experimental`, the regex would reject it. Document the scope-limit decision in CONTEXT.md or in an inline comment.

No action required; just noting the intentional narrowness.

---

### IN-02: Fixture file `v0.40.0__v0.40.1.json` (smallest) has only 2 commits — single-element-array regression guard is implicit

**File:** `tests/integration/fixtures/upstream-drift/v0.40.0__v0.40.1.json` (referenced by SUMMARY 24-01)

**Observation:** SUMMARY 24-01 calls this fixture "PS 5.1 single-element-array regression guard". With 2 commits, this exercises the multi-element case but not the literal single-element case. If the post-D-11-filter range ever yields 1 commit (or 0), the `@()` wrap behavior is the only thing preventing PS 5.1 unwrap. Consider adding a synthetic 1-commit and 0-commit fixture (or assert exit code + JSON shape on a bound-to-zero-commits range) to lock the invariant explicitly.

No action required for v1; document as a future hardening idea.

---

### IN-03: `2>/dev/null` permitted on `git describe` failure path

**File:** `scripts/check-upstream-drift.sh:106`

**Observation:** Per CLAUDE.md "fail secure: on any error, deny access. Never silently degrade", `2>/dev/null` is generally discouraged. The inline comment on line 104 explicitly justifies this case ("permitted ONLY here because we explicitly catch the failure and substitute a clearer error message below"), and the failure path produces an actionable error and `exit 1`. Compliant with the project's principle. Documented for posterity.

---

### IN-04: Cross-platform `command -v` in Makefile may not handle MSYS2 vs cmd.exe pwsh discovery uniformly

**File:** `Makefile:81`

**Observation:** The `ifeq ($(OS),Windows_NT)` block uses `command -v pwsh` to test for PS 7. This works under MSYS2 bash (which is what `make` typically runs under on Windows). If a maintainer were to run `make` from `cmd.exe` directly (rare, since most Windows make installations come with mingw-w64 / MSYS bash), `command -v` would not exist. The fallback to `powershell.exe` is hardcoded and always available on Windows 10/11. The current Phase 22-03 / Phase 24 maintainer host has bash on PATH (per SUMMARY's "Git for Windows MSYS2 bash"), so this is not actively a problem.

If a CI runner uses cmd.exe directly, document the requirement that `bash` (Git-for-Windows) be on PATH, or rewrite the dispatch in pure cmd syntax. No action for v1.

---

## File-by-file analysis

### `scripts/check-upstream-drift.sh` — bash twin (321 lines)

**Strengths:**
- `set -euo pipefail` enforced.
- T-24-01 mitigation correct: `validate_ref` runs BEFORE any git invocation; `--` separator on `git log` provides defense-in-depth.
- T-24-02 read-only invariant verified by Test 6 (no fetch, no `git config` write).
- T-24-04 JSON escape: backslash-first ordering is correct (mitigation for lines 158–162).
- Process substitution `< <(git log ...)` keeps accumulator state in the parent shell (Pitfall 8 sidestepped).
- Category lookup-table comment locks the audit-before-generic-nono ordering invariant.
- Auto-detect via `git tag --list 'v0.*' --merged HEAD --sort=-v:refname` correctly avoids fork-only tags.

**Findings:** MD-01 (brace-rename), LO-01 (auto-detected ref re-validation), LO-02 (stderr suppression), IN-01 (regex scope), IN-03 (`2>/dev/null` on git describe).

### `scripts/check-upstream-drift.ps1` — PowerShell twin (269 lines)

**Strengths:**
- T-24-03 mitigation correct: `[Console]::OutputEncoding = UTF8` pinned at line 29–30 BEFORE any git invocation. `$OutputEncoding` set as well for outbound pipe encoding.
- `Set-StrictMode -Version Latest` + `$ErrorActionPreference = "Stop"` enforced.
- `[Console]::Out.Write($json + "`n")` replaces `Write-Output` to avoid the PS-CRLF footgun (verified twin-parity).
- `[ordered]@{}` hashtables lock JSON key emission order across PS 5.1 and 7+.
- `@($cats)` and `@($commits)` wrapping prevents single-element-unwrap (PS 5.1 Pitfall 6).
- `[ValidateSet("table","json")]` on `$Format` prevents unknown formats at parse time.

**Findings:** MD-01 (brace-rename — twin-symmetric), LO-01 (auto-detected ref re-validation), LO-02 (stderr suppression), LO-03 (redundant package_cmd.rs entry).

### `tests/integration/test_upstream_drift.sh` — 43-assertion suite (444 lines)

**Strengths:**
- All 5 threats have at least one assertion.
- Trap chaining for tmp_repo + smoke_tmpdir cleanup is correct.
- `set +e` after `source test_helpers.sh` allows full-suite traversal even when an early test fails.
- Local `pass`/`fail` helpers update the shared `TESTS_*` counters so `print_summary` works correctly.
- Test 4 missing-upstream subshell is hermetic (fresh temp repo).
- Test 7 ref-injection rejection verifies BOTH the exit code AND the side-effect file is absent.
- D-19 trailer block enforced by 7 separate per-line assertions (Tests 8.1–8.7) including the lowercase-'a' invariant and the 2-Signed-off-by count.
- Section-ordering invariant (Test 11) uses awk to enforce `Upstream Parity Process` precedes `Evolution` in PROJECT.md.

**Findings:** MD-02 (same-line HTML-comment gap in T-24-05 mitigation), LO-05 (redundant `set +e` in subshell).

### `Makefile` (188 lines, +18 from Phase 24)

**Strengths:**
- `$(OS) == Windows_NT` is the canonical Windows-detection idiom under both MSYS2 bash and cmd.
- pwsh-then-powershell.exe fallback covers PS 5.1 (the maintainer's PS 5.1 ships with Windows 10/11 by default) and PS 7 (forward-compat).
- ARGS passthrough is the standard make idiom.
- `.PHONY` list updated.
- `help` target gained a "Maintainer" section.
- `-NoProfile` flag is present on both pwsh and powershell.exe invocations (avoids per-user profile interference).

**Findings:** IN-04 (cmd.exe-direct edge case).

### `.planning/templates/upstream-sync-quick.md` (257 lines)

**Strengths:**
- Single-brace `{name}` placeholders match the 31-template GSD convention.
- D-19 trailer block at lines 220–225 is byte-exact verbatim (lowercase 'a', 6 lines, 2 Signed-off-by).
- Fork-divergence catalog has 5 entries with explicit "Action on cherry-pick" sub-blocks.
- Windows-specific retrofit checklist enumerates per-feature questions.
- Smoke-check instruction in leading comment (line 14) gives the exact `grep -oE '\{[a-z_]+\}' PLAN.md` invocation.
- Footer cross-link to runbook (line 257) closes the template ↔ docs loop.

**Findings:** None directly in this file. MD-02 in the test runner exposes a latent risk if future template edits add same-line comment + content constructs.

### `docs/cli/development/upstream-drift.mdx` (158 lines)

**Strengths:**
- Mintlify frontmatter (`title:` + `description:` only, no GSD-flavored keys).
- 7 H2 sections cover the full workflow.
- JSON output shape worked example (lines 42–63) is accurate.
- D-19 trailer block worked example with field-rules sub-list (lines 107–124) — does NOT contain the forbidden `Upstream-Author` substring (verified by Test 11's structural grep semantics).
- Fixture-regeneration procedure with all 3 fixture commands.
- `See also` footer cross-references all 4 sibling artifacts.

**Findings:** LO-04 (categorization table prefix abbreviation).

### `.planning/PROJECT.md` (additive +11 lines)

**Strengths:**
- New `## Upstream Parity Process` H2 inserted between `## Key Decisions` and `## Evolution` (verified by Test 11 awk ordering check).
- 4-step numbered workflow with bold-anchored bullets.
- All 3 cross-link targets present: `make check-upstream-drift`, `.planning/templates/upstream-sync-quick.md`, `docs/cli/development/upstream-drift.mdx`.
- Existing sections untouched.

**Findings:** None.

---

## Recommendations (priority order)

1. **(MD-01) Fix brace-rename parsing** in both twins; add a regression fixture or document the limitation. ~30 min effort, low risk.
2. **(MD-02) Harden the placeholder smoke test** to handle inline `<!-- ... --> {name}` constructs. ~15 min effort, low risk.
3. **(LO-01) Re-validate auto-detected refs** against the injection regex. Defense-in-depth alignment with project security principles. ~5 min effort.
4. **(LO-03, LO-04) Documentation polish:** drop redundant `package_cmd.rs` regex arm; spell out full path prefixes in the docs categorization table. ~5 min effort each.

None of these are blocking for phase sign-off. The phase delivers its stated DRIFT-01 and DRIFT-02 acceptance criteria with full threat-model coverage.

---

_Reviewed: 2026-04-28T04:11:37Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
