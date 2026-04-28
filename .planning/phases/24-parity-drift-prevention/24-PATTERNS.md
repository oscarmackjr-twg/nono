# Phase 24: Parity-Drift Prevention - Pattern Map

**Mapped:** 2026-04-27
**Files analyzed:** 9 created + 2 modified = 11 total
**Analogs found:** 11 / 11 (all have a strong in-repo analog)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `scripts/check-upstream-drift.sh` | maintainer-script (bash) | batch / read-only-transform | `scripts/test-linux.sh` | role-match (bash twin convention) |
| `scripts/check-upstream-drift.ps1` | maintainer-script (PowerShell) | batch / read-only-transform | `scripts/build-windows-msi.ps1` | role-match (PS twin convention) |
| `.planning/templates/upstream-sync-quick.md` | template (Markdown, fillable-blanks) | document-substitution | `~/.claude/get-shit-done/templates/AI-SPEC.md` (precedent) + `.planning/quick/260424-upr-review-upstream-037-to-040/PLAN.md` (frontmatter shape) | role-match |
| `docs/cli/development/upstream-drift.mdx` | docs (.mdx with Mintlify-style frontmatter) | static document | `docs/cli/development/testing.mdx` | exact (same dir, same convention) |
| `tests/integration/test_upstream_drift.sh` | integration-test runner (bash) | request-response / shell exit-code check | `tests/integration/test_setup.sh` | role-match (small focused test_*.sh) |
| `tests/integration/fixtures/upstream-drift/v0.37.1__v0.40.1.json` | test-fixture (frozen JSON) | static data | none in-repo (greenfield) — shape defined by RESEARCH.md "Output shape" | new file kind |
| `tests/integration/fixtures/upstream-drift/v0.39.0__v0.40.0.json` | test-fixture (frozen JSON) | static data | (same) | new file kind |
| `tests/integration/fixtures/upstream-drift/v0.40.0__v0.40.1.json` | test-fixture (frozen JSON, single-element-array PS edge case) | static data | (same) | new file kind |
| `tests/integration/fixtures/upstream-drift/README.md` | docs (regen procedure + 22-commit informational delta) | static document | none in-repo for this exact role; closest is the SUMMARY.md narrative voice in `.planning/quick/260424-upr-review-upstream-037-to-040/SUMMARY.md` | role-adjacent |
| `Makefile` (MODIFIED) | build-system | platform dispatch | `Makefile` lines 55-65 (`test-windows-*` targets invoke `pwsh -File ...`); D-02 dispatch is new style — no in-repo `ifeq ($(OS),Windows_NT)` precedent yet | role-match (extends existing target style) |
| `PROJECT.md` (MODIFIED) | project document | new section insertion | `PROJECT.md` existing structure (Current Milestone, Validated, Active, Deferred) | exact (in-place edit) |

## Pattern Assignments

### `scripts/check-upstream-drift.sh` (maintainer-script, bash)

**Analog:** `scripts/test-linux.sh`
**Why closest:** Same role (top-level bash maintainer script invoked manually + via Makefile), same shebang convention, same `set -euo pipefail` discipline, same "fail loud, no silent fallback" CLAUDE.md posture.

**Header / shebang pattern** (lines 1-6 of `scripts/test-linux.sh`):
```bash
#!/usr/bin/env bash
# nono Linux Test Script
# Run this on a Linux machine to verify sandbox enforcement

set -euo pipefail
```
Replicate verbatim with the new script's purpose:
```bash
#!/usr/bin/env bash
# scripts/check-upstream-drift.sh
# Reports upstream commits the fork has not absorbed, grouped by file category.
# Read-only - does NOT modify git state.

set -euo pipefail
```

**Why exactly `set -euo pipefail`:** Universal in this repo's bash scripts (test-linux.sh, run_integration_tests.sh, all `tests/integration/test_*.sh` via `source` of test_helpers). Maps directly to the CLAUDE.md "fail secure on any error" rule. Do NOT drop any of the three flags.

**Color / status helpers** (lines 7-48 of `scripts/test-linux.sh`):
```bash
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_header() { echo -e "${BLUE}---...---${NC}"; echo -e "${BLUE}  $1${NC}"; ... }
pass()  { echo -e "  ${GREEN}PASS${NC}: $1"; PASSED=$((PASSED + 1)); }
fail()  { echo -e "  ${RED}FAIL${NC}: $1"; FAILED=$((FAILED + 1)); }
```
Reuse this color palette for the `--format table` grouped output (e.g. blue category headers, plain-white commit rows). Do not invent new color codes.

**Manual-CLI parsing pattern** (already in RESEARCH.md Pattern 1, lines 219-240): manual `while [[ $# -gt 0 ]]; do case "$1" in ... esac; done` loop. Matches the no-`getopt` discipline in this repo (Git for Windows MSYS bash has spotty getopt portability — RESEARCH.md verified).

**Process-substitution-not-pipe** (RESEARCH.md Pitfall 8): when consuming `git log` output into accumulator state, use `done < <(git log ...)`, NOT `git log ... | while read`. Pipe creates a subshell; arrays accumulated inside don't survive.

**Cleanup trap** (lines 19-23 of `scripts/test-linux.sh`):
```bash
cleanup() { rm -rf "$TEST_DIR" 2>/dev/null || true; }
trap cleanup EXIT
```
Apply equivalent if the script writes any temp files (e.g. when staging `git log` output). The drift script is read-only over `.git`, so this is only needed if temp files appear in the executor's design.

**Gotchas to preserve:**
- Shebang exactly `#!/usr/bin/env bash` (not `/bin/bash` — test-linux.sh, run_integration_tests.sh, test_*.sh all use env-bash).
- Pure POSIX-friendly utilities only: `git`, `awk`, `sed`, `printf`, `sort`, `grep`. Do NOT introduce `jq` (RESEARCH.md verified `jq` not on PATH; bash must hand-roll JSON via printf-with-escape).
- ANSI color codes only with `echo -e`. Do not assume color always on; downstream CI consumption of `--format json` must NOT include color.
- The `--format json` branch must emit ONE line of JSON (or pretty-printed deterministic JSON) with NO color escapes.

---

### `scripts/check-upstream-drift.ps1` (maintainer-script, PowerShell)

**Analog:** `scripts/build-windows-msi.ps1`
**Why closest:** Only top-level `.ps1` maintainer script in `scripts/` that uses `param()`, `Set-StrictMode`, `$ErrorActionPreference = "Stop"` — exactly the discipline the new script needs. `windows-test-harness.ps1` is the other candidate but is a test harness, not a one-shot tool, so its shape is heavier.

**param() block pattern** (lines 1-18 of `scripts/build-windows-msi.ps1`):
```powershell
param(
    [Parameter(Mandatory = $true)]
    [string]$VersionTag,

    [Parameter(Mandatory = $true)]
    [string]$BinaryPath,

    [ValidateSet("machine", "user")]
    [string]$Scope = "machine",

    [string]$OutputDir = "dist/windows",
    ...
    [switch]$EmitOnly
)
```
For the drift script (none of the params are mandatory — D-08 auto-detect is the default), apply the same shape minus `Mandatory = $true`:
```powershell
param(
    [string]$From = "",
    [string]$To = "",
    [ValidateSet("table","json")]
    [string]$Format = "table"
)
```
`[ValidateSet(...)]` enforces the `--format <table|json>` D-04 contract at the parameter binding layer (clean validation, friendly error message).

**Strict-mode + error-stop preamble** (lines 20-21 of `scripts/build-windows-msi.ps1`):
```powershell
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
```
Replicate verbatim. Both are required: `Set-StrictMode` catches typos in variable names; `$ErrorActionPreference = "Stop"` makes non-terminating cmdlet errors halt (mirrors bash's `set -e`).

**ADD: UTF-8 console encoding** (NOT in build-windows-msi.ps1 because that script writes a file; the drift script writes to stdout, so PS 5.1 mojibake is a real risk per RESEARCH.md Pitfall 1). Add immediately after `$ErrorActionPreference`:
```powershell
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$OutputEncoding = [System.Text.UTF8Encoding]::new()
```
This is the single most important PS 5.1 fix; without it non-ASCII commit subjects produce invalid UTF-8 in the JSON output and break fixture diffs.

**UTF-8-no-BOM file write pattern** (lines 101-117 of `scripts/build-windows-msi.ps1`):
```powershell
function Write-Utf8NoBomCompat {
    param([Parameter(Mandatory = $true)] [string]$Path,
          [Parameter(Mandatory = $true)] [string]$Value)
    if ($PSVersionTable.PSVersion.Major -ge 6) {
        Set-Content -LiteralPath $Path -Value $Value -Encoding utf8NoBOM
        return
    }
    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Value, $utf8NoBom)
}
```
The drift script writes to stdout, not files, so this exact helper is not needed. BUT: if the executor adds a `--output <path>` later, copy this helper verbatim — same PS 5.1 vs PS 7 forking. Note for now.

**`@()` array wrapping for JSON** (RESEARCH.md Pitfall 6): PS 5.1 unwraps single-element arrays in `ConvertTo-Json`. Always:
```powershell
$result = [ordered]@{
    range = "$from..$to"
    total_unique_commits = $commits.Count
    by_category = $byCategory
    commits = @($commits)            # @() prevents single-element unwrap
}
$result | ConvertTo-Json -Depth 6 -Compress
```
`-Depth 6` (not the default 2!) is required so nested `files_changed: [...]` and `categories: [...]` arrays don't serialize as `"System.Object[]"`. `-Compress` matches the bash `printf` no-pretty-print output so byte-for-byte diff is feasible against the fixture.

**Gotchas to preserve:**
- `param()` block MUST be the first executable line (PowerShell parser rule). Comments above it are fine.
- Script must run under **Windows PowerShell 5.1** (`powershell.exe`), not just `pwsh` 7. RESEARCH.md verified `pwsh` is NOT on this maintainer's PATH. Test against PS 5.1 explicitly.
- LF or CRLF line endings: the existing `build-windows-msi.ps1` uses LF (Git for Windows default; verified by reading the file). Match.
- No BOM on the script file itself.
- Use `[ValidateSet(...)]` instead of hand-rolled `if`-chains for the `--format` arg.

---

### `.planning/templates/upstream-sync-quick.md` (template, fillable-blanks Markdown)

**Analog:** `~/.claude/get-shit-done/templates/AI-SPEC.md` (placeholder convention precedent) + `.planning/quick/260424-upr-review-upstream-037-to-040/PLAN.md` (frontmatter shape for the upstream-sync quick task family).

**Why these two:** The placeholder syntax must match GSD precedent (single-brace `{name}` is universal across 31 GSD templates per RESEARCH.md). The frontmatter shape must match the quick-task family this template scaffolds for (so the maintainer's manually-filled output validates against existing tooling).

**Frontmatter shape** (lines 1-9 of `.planning/quick/260424-upr-review-upstream-037-to-040/PLAN.md`):
```markdown
---
slug: upr-review-upstream-037-to-040
created: 2026-04-24
type: research-only
---

# Quick task: Review upstream v0.37.1 → v0.40.1 for Windows-native impact

**Ask:** ...
**Scope:** ...
**What I did NOT do:** ...
**STATE.md update:** ...
```

Replicate as fillable template:
```markdown
---
slug: {quick_slug}
created: {date}
type: upstream-sync
range: {from_tag}..{to_tag}
---

# Quick task: Sync upstream {from_tag} → {to_tag} into the fork

**Ask:** ...
```

**Placeholder convention** (verified against `~/.claude/get-shit-done/templates/AI-SPEC.md` line 1: `# AI-SPEC — Phase {N}: {phase_name}`):
- Use single-brace `{name}` placeholders. NEVER `{{NAME}}` and NEVER `<NAME>` (RESEARCH.md Pattern 7 explicitly rules these out — `<NAME>` collides with HTML/JSX, `{{NAME}}` is not used by any of the 31 GSD templates).
- Use `<!-- inline guidance comments -->` for maintainer hints, exactly as `AI-SPEC.md` does (line 10: `**System Type:** <!-- RAG | Multi-Agent | ... -->`).

**Headline + commit-inventory shape** (lines 11-26 of `.planning/quick/260424-upr-review-upstream-037-to-040/SUMMARY.md`):
```markdown
# Upstream {from_tag} → {to_tag} review — Windows-native impact

## Headline

**{commit_count} non-merge commits, ~{insertions}k insertions / ~{deletions} deletions.** ...

Five feature groups dominate. In priority order for Windows follow-up:

| # | Feature group | Upstream LOC | Windows impact |
|---|---------------|--------------|----------------|
| 1 | ... | ... | ... |
```
Replicate as a placeholder block in the template — the maintainer pastes drift-check JSON output and curates into this table.

**D-19 cherry-pick trailer block** (verified verbatim from `git log -1 --pretty=format:'%B' 73e1e3b8` tail; identical shape on `adf81aec` and `869349df`):
```
Upstream-commit: 600ba4ec
Upstream-tag: v0.38.0
Upstream-author: Luke Hinds <lukehinds@gmail.com>
Co-Authored-By: Luke Hinds <lukehinds@gmail.com>
Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
```

Encode in the template EXACTLY (same line order, same case — `Upstream-author` lowercase `a`):
```
Upstream-commit: {upstream_sha_abbrev}
Upstream-tag: {upstream_tag}
Upstream-author: {upstream_author_name} <{upstream_author_email}>
Co-Authored-By: {upstream_author_name} <{upstream_author_email}>
Signed-off-by: {fork_author_name} <{fork_author_email}>
Signed-off-by: {fork_author_handle} <{fork_author_email}>
```

**Critical structural rules to preserve** (RESEARCH.md Pattern 8, all verified):
1. Trailer block separated from body by exactly ONE blank line.
2. Order is fixed: `Upstream-commit` → `Upstream-tag` → `Upstream-author` → `Co-Authored-By` → `Signed-off-by` (full name) → `Signed-off-by` (github handle).
3. `Upstream-author` and `Co-Authored-By` carry the SAME name+email.
4. Two `Signed-off-by` lines: full name + github handle, both required for DCO + GitHub attribution.
5. Field name `Upstream-author` has lowercase `a` (NOT `Upstream-Author`).
6. Abbreviated 8-char SHA is the in-use convention.

**Gotchas to preserve:**
- Replace-all checklist at the top of the template (RESEARCH.md Pattern 7 final paragraph): `<!-- Before committing: replace all {placeholder} markers. Smoke check: grep -oE '\\{[a-z_]+\\}' should return 0 matches. -->`
- Section "fork-divergence catalog" must explicitly include `validate_path_within` retention (per CONTEXT.md Specifics §2 and 22-03-PKG-PROGRESS.md). Other entries: async-runtime wrapping for `load_production_trusted_root`, deferred enum variants like `ArtifactType::Plugin`.
- Section "Windows-specific retrofit checklist" must encode the per-feature gate (`#[cfg(target_os = "windows")]`) check.
- Template references `make check-upstream-drift > drift.json` per D-14 but does NOT auto-include the output. Phrase as a manual step the maintainer runs before pasting curated entries.

---

### `docs/cli/development/upstream-drift.mdx` (docs, .mdx)

**Analog:** `docs/cli/development/testing.mdx`
**Why closest:** Both are how-to/runbook style docs in the same directory; same Mintlify frontmatter convention; same code-fence + table style for procedural docs.

**Frontmatter pattern** (lines 1-4 of `docs/cli/development/testing.mdx`):
```mdx
---
title: Testing
description: nono integration test suites, running tests, and CI pipeline
---

nono includes comprehensive integration tests that verify ...
```
Replicate verbatim shape:
```mdx
---
title: Upstream Drift Check
description: How to inventory unabsorbed upstream commits and scaffold an upstream-sync quick task
---

The `make check-upstream-drift` target reports upstream commits the fork hasn't ...
```

**Section convention** (lines 7-37 of `docs/cli/development/testing.mdx`):
- `## Running Tests` (verb-noun H2 sections)
- `### Full Test Suite` followed by a fenced `bash` code block with the command
- `### Individual Test Suites` followed by another fenced bash block
- Brief paragraph between heading and code block

For `upstream-drift.mdx` (D-16) replicate this rhythm:
- `## Running the drift check` → fenced bash block with `make check-upstream-drift`
- `## Output formats` → table comparing `--format table` vs `--format json`
- `## Categorization rules` → fenced block showing the path-prefix lookup table
- `## Using the output with the upstream-sync template` → cross-link to `.planning/templates/upstream-sync-quick.md`
- `## Regenerating the test fixtures` → fenced bash block with the regen procedure (mirrors `tests/integration/fixtures/upstream-drift/README.md`)

**Table pattern** (lines 41-55 of `docs/cli/development/testing.mdx`):
```mdx
| Test Category | What It Verifies |
|---------------|------------------|
| Directory Read | Files can be read in granted directories |
```
Reuse for the categorization rules and output-format comparison tables. Pipe alignment is loose in existing files; do not over-tighten.

**Gotchas to preserve:**
- File extension is `.mdx`, NOT `.md` (D-16; verified all 18 files in `docs/cli/development/` are `.mdx`).
- Frontmatter uses `title:` and `description:` keys (Mintlify schema). Do not include `slug:`, `date:`, or other GSD-flavored keys here.
- `<NAME>` syntax is forbidden inside `.mdx` body — collides with JSX. If the docs file shows the template's placeholders, use single-brace `{name}` (which matches the template's own convention) AND wrap in code fences so MDX doesn't parse them.
- No emojis (CLAUDE.md notes block).

---

### `tests/integration/test_upstream_drift.sh` (integration-test runner)

**Analog:** `tests/integration/test_setup.sh` (closest small focused single-purpose test_*.sh) + RESEARCH.md Pattern 9 for the actual diff-against-fixture logic.

**Why two analogs:** `test_setup.sh` shows the canonical short-script shape and the test-helper sourcing convention used across the integration suite. RESEARCH.md Pattern 9 supplies the diff-against-fixture mechanism that no existing test_*.sh demonstrates exactly.

**Sourcing + header pattern** (lines 1-15 of `tests/integration/test_setup.sh`):
```bash
#!/bin/bash
# Setup Command Tests
# Tests nono setup output and behavior

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/test_helpers.sh"

echo ""
echo -e "${BLUE}=== Setup Tests ===${NC}"

verify_nono_binary

echo ""
```

For `test_upstream_drift.sh`, follow the same shape but DROP `verify_nono_binary` (this test does not invoke the `nono` binary — it tests the drift script):
```bash
#!/bin/bash
# Upstream Drift Check Tests
# Verifies the bash + PowerShell twin scripts produce identical JSON for known ranges.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/test_helpers.sh"

echo ""
echo -e "${BLUE}=== Upstream Drift Tests ===${NC}"

# NOTE: this suite does not require the nono binary; it tests scripts/check-upstream-drift.{sh,ps1}.
```

**Shebang convention:** `tests/integration/test_*.sh` files use `#!/bin/bash` (NOT `#!/usr/bin/env bash` — verified across `test_setup.sh`, `test_audit.sh`, `test_fs_access.sh`). Match the integration-test convention here, even though `scripts/test-linux.sh` uses env-bash. Different convention for different directory.

**Helper functions** (lines 22-40 of `tests/integration/test_setup.sh`):
```bash
expect_success "setup --check-only exits 0" \
    "$NONO_BIN" setup --check-only

expect_output_contains "setup output contains platform info" "Platform:" \
    "$NONO_BIN" setup --check-only
```
These come from `tests/lib/test_helpers.sh` (sourced above). Reuse `expect_success`, `expect_output_contains` for the shell-exit-code and grep-assertion checks (e.g. tag auto-detect picks `^v0\.`, missing remote exits 1, table format contains category headers).

**Diff-against-fixture pattern** (RESEARCH.md Pattern 9, lines 572-591):
```bash
for range in "v0.37.1__v0.40.1" "v0.39.0__v0.40.0" "v0.40.0__v0.40.1"; do
  from="${range%__*}"
  to="${range#*__}"
  expected="$SCRIPT_DIR/fixtures/upstream-drift/${range}.json"

  actual_sh=$(bash "$REPO_ROOT/scripts/check-upstream-drift.sh" --from "$from" --to "$to" --format json)
  diff <(echo "$actual_sh") "$expected" || { echo "FAIL: bash $range"; exit 1; }

  if command -v powershell.exe >/dev/null 2>&1 || command -v pwsh >/dev/null 2>&1; then
    runner=$(command -v pwsh || command -v powershell.exe)
    actual_ps=$("$runner" -NoProfile -File "$REPO_ROOT/scripts/check-upstream-drift.ps1" -From "$from" -To "$to" -Format json)
    diff <(echo "$actual_ps") "$expected" || { echo "FAIL: ps $range"; exit 1; }
  fi
done
```

**Print-summary pattern** (line 39 of `tests/integration/test_setup.sh`):
```bash
print_summary
```
Standard footer from `test_helpers.sh`. Apply at script end so the suite reports pass/fail counts to `run_integration_tests.sh`.

**Auto-discovery integration** (RESEARCH.md Validation Architecture; verified against `tests/run_integration_tests.sh` lines 70-91): `run_integration_tests.sh` uses an explicit `SUITES=(...)` array, NOT auto-discovery by glob. To wire this test into the full suite, the executor must add `"test_upstream_drift.sh:Upstream Drift"` to the `SUITES` array at line ~91 of `tests/run_integration_tests.sh`. **Note:** if the planner decides this test is independent enough to run standalone via `make check-upstream-drift-test` (or similar), the SUITES edit can be skipped — but then it won't be caught by the phase-gate full-suite run. Recommend adding to SUITES.

**Gotchas to preserve:**
- Shebang `#!/bin/bash` (integration-test convention), not env-bash.
- Source `test_helpers.sh` even if not strictly using all helpers (matches the established pattern; future-proof).
- Use `expect_success` / `expect_output_contains` from helpers for shell-exit-code assertions; only fall back to raw bash for the diff-against-fixture flow.
- Process substitution `<(echo "$actual")` vs `<<<` — both work, RESEARCH.md uses `<(echo)` for cross-shell consistency. Match.
- The PS-runner branch must be SKIPPED on Linux CI (where neither `pwsh` nor `powershell.exe` is on PATH); the `command -v` guard handles this. Do NOT make absence-of-PS a test failure.
- Twin-parity check (RESEARCH.md Validation Architecture line 893) — diff `bash output` vs `pwsh output` byte-for-byte. This is the test that catches "maintainer updated only `.sh`" drift.

---

### `tests/integration/fixtures/upstream-drift/v0.37.1__v0.40.1.json` (test-fixture, frozen JSON)

**Analog:** None in-repo (greenfield file kind). **Shape:** RESEARCH.md "Output shape" block, lines 423-450:
```json
{
  "range": "v0.37.1..v0.40.1",
  "from": "v0.37.1",
  "to": "v0.40.1",
  "total_unique_commits": 56,
  "by_category": {
    "profile": 12,
    "policy": 8,
    "package": 11,
    "proxy": 4,
    "audit": 9,
    "other": 19
  },
  "commits": [
    {
      "sha": "...",
      "subject": "...",
      "author": "...",
      "date": "2026-04-...",
      "additions": 1419,
      "deletions": 226,
      "files_changed": [...],
      "categories": ["audit", "policy"]
    }
  ]
}
```

**Generation procedure** (RESEARCH.md Validation Architecture "Reference-fixture strategy", lines 884-888):
1. Run `bash scripts/check-upstream-drift.sh --from v0.37.1 --to v0.40.1 --format json > tests/integration/fixtures/upstream-drift/v0.37.1__v0.40.1.json` once at the canonical commit-graph state.
2. Review fixture against `260424-upr/SUMMARY.md` per-release blocks.
3. Document the 22-commit informational delta in the README.md companion file.
4. Commit verbatim. Both `.sh` and `.ps1` outputs must diff cleanly against it going forward.

**Gotchas to preserve:**
- File MUST be the EXACT byte output of `--format json` (no trailing newline mismatch, no key-order shuffle). The diff in `test_upstream_drift.sh` is byte-for-byte.
- Use `-Compress` in PS / no pretty-print in bash so the fixture is compact (one line). Pretty-print would be reviewer-friendly but introduces whitespace nondeterminism between bash and PS implementations.
- D-08 auto-detect range MUST NOT change this fixture — `--from` / `--to` flags ALWAYS override per D-09.
- Acceptance #1 ground-truth: this fixture's `total_unique_commits` value (RESEARCH.md recommends 56 with the canonical D-11 path filter; ALTERNATE 78 if the planner widens the filter to include `Cargo.lock` etc. — Open Question 1 in RESEARCH.md). **Planner: confirm the count before generating the fixture.**

---

### `tests/integration/fixtures/upstream-drift/v0.39.0__v0.40.0.json` (test-fixture, audit-cluster mid-range)

**Analog:** Same shape as `v0.37.1__v0.40.1.json` above (different range).

**Purpose** (RESEARCH.md Pattern 9, line 567): mid-range fixture covering the "audit-integrity cluster" — exercises the `audit` category lookup-table entries (`crates/nono/src/audit/`, `crates/nono/src/audit_attestation*`, `crates/nono-cli/src/audit*`).

**Gotchas:** Same as the large-range fixture. The categorization output should heavily feature the `audit` category here — if it does not, the executor's lookup table is wrong.

---

### `tests/integration/fixtures/upstream-drift/v0.40.0__v0.40.1.json` (test-fixture, single-element-array PS edge case)

**Analog:** Same shape as `v0.37.1__v0.40.1.json` above (small range).

**Purpose** (RESEARCH.md Pattern 9 line 568, Pitfall 6 line 678): catches the PS 5.1 single-element-array unwrap bug. The `v0.40.0..v0.40.1` range is 3 commits; if the executor needs an even-tighter range to reach a 1-commit case (RESEARCH.md suggests `--from <sha>~1 --to <sha>` for a single-commit subrange), they can construct it but the fixture must capture the array-shape.

**Critical correctness check:** The JSON must contain `"commits": [ { ... } ]` (array even for one element), NEVER `"commits": { ... }`. The PS implementation's `@()` wrapping is what guarantees this; the fixture is what catches a regression.

---

### `tests/integration/fixtures/upstream-drift/README.md` (docs, regen procedure + delta)

**Analog:** No exact in-repo precedent. Closest analog for the role (a "how to regenerate this fixture and what's in it" note) is the brief intro of `.planning/quick/260424-upr-review-upstream-037-to-040/SUMMARY.md` (its Headline + frontmatter give a frozen-snapshot voice).

**Recommended structure** (RESEARCH.md Validation Architecture lines 593-600):
```markdown
# Upstream-drift fixtures

Frozen JSON outputs of `scripts/check-upstream-drift.sh --format json` for known commit ranges.

## Files

- `v0.37.1__v0.40.1.json` — large-range; reproduces the 260424-upr SUMMARY's filtered inventory
- `v0.39.0__v0.40.0.json` — audit-integrity cluster
- `v0.40.0__v0.40.1.json` — small-range PS-5.1 single-element-array edge case

## Regeneration

Generated 2026-04-27 from <upstream sha>. Regenerate with:

\`\`\`bash
bash scripts/check-upstream-drift.sh --from v0.37.1 --to v0.40.1 --format json \\
  > tests/integration/fixtures/upstream-drift/v0.37.1__v0.40.1.json
\`\`\`

## Informational delta vs SUMMARY.md headline

The 260424-upr SUMMARY.md headline reads "78 non-merge commits". The script's
canonical D-11 path filter produces 56 commits for this same range. The
22-commit difference is documented here for future readers:

| Excluded by filter | Why | Listed in SUMMARY for |
|--------------------|-----|----------------------|
| docs-only commits  | not cross-platform code | narrative completeness |
| Cargo.lock dep bumps | not in path filter | dep-bump audit trail |
| GitHub workflow files | not cross-platform code | CI history |
| integration tests | not cross-platform code | test history |
| claude-code package removal | breaking removal not subject to drift | removal record |
```

**Gotchas to preserve:**
- This file is plain `.md` (NOT `.mdx`) — it lives under `tests/`, not `docs/`. Match the rest of the test-fixtures-as-data convention.
- Document the 22-commit delta as informational, not as a bug. Per RESEARCH.md Open Question 1 recommendation: filter is canonical, headline is informational.
- Cross-link to the SUMMARY.md so future maintainers can trace the discrepancy back to its source.

---

### `Makefile` (MODIFIED — add `check-upstream-drift` target)

**Analog:** `Makefile` lines 55-65 (existing `test-windows-*` family) for the `pwsh -File scripts/...` invocation pattern.

**Existing pattern to extend** (lines 55-65):
```makefile
test-windows-harness:
	pwsh -File scripts/windows-test-harness.ps1 -Suite all

test-windows-smoke:
	pwsh -File scripts/windows-test-harness.ps1 -Suite smoke
```

**Insertion point:** Logical group is alongside `test-*` but above `# Check targets (lint + format)`. Approximately after line 68 (`test-doc:` target ends), before line 70 (`# Check targets (lint + format)` comment). Keep `.PHONY` updated on line 9.

**Required dispatch** (RESEARCH.md Pattern 6, lines 489-503; D-02):
```makefile
.PHONY: check-upstream-drift

# Detect Windows. $(OS) == Windows_NT under cmd/MSYS bash.
ifeq ($(OS),Windows_NT)
check-upstream-drift:
	@if command -v pwsh >/dev/null 2>&1; then \
		pwsh -File scripts/check-upstream-drift.ps1 $(ARGS); \
	else \
		powershell.exe -NoProfile -File scripts/check-upstream-drift.ps1 $(ARGS); \
	fi
else
check-upstream-drift:
	@bash scripts/check-upstream-drift.sh $(ARGS)
endif
```

**`.PHONY` update** (Makefile line 9): append `check-upstream-drift` to the existing `.PHONY:` list.

**Help-target update** (Makefile lines 126-162): add an entry under a new "Maintainer" section or under the existing "Other:" block:
```makefile
	@echo ""
	@echo "Maintainer:"
	@echo "  make check-upstream-drift              Inventory unabsorbed upstream commits"
	@echo "  make check-upstream-drift ARGS=\"--from v0.40.1 --to v0.42.0 --format json\"  Override range / format"
```

**Gotchas to preserve:**
- Existing Makefile uses `pwsh -File ...` with NO `-NoProfile`. The new dispatch should ADD `-NoProfile` only on the `powershell.exe` fallback (matches RESEARCH.md Pattern 6); leave the `pwsh` branch matching the existing convention OR add `-NoProfile` to both for consistency. Planner's call; recommend adding to both for hermetic execution.
- `$(OS) == Windows_NT` is the standard cross-Make Windows detection. Verified by RESEARCH.md against existing patterns. There is NO existing platform-detection idiom in this Makefile, so this is the first instance — make it clean.
- `$(ARGS)` is a passthrough variable; document the `ARGS="..."` invocation in `make help`.
- Tab indentation (NOT spaces) — Makefile-strict. Verified across all existing targets.
- Recipe lines must each begin with a tab. The conditional `ifeq` block is at column 0 (no indent), the recipe lines INSIDE are tab-indented.
- Backslash-continuation for the multi-line `if` block: each continuation line must end with `\` and the next line is tab-indented.

---

### `PROJECT.md` (MODIFIED — add `## Upstream Parity Process` section)

**Analog:** `PROJECT.md` itself (existing section structure).

**Existing section style** (verified by full-file read):
- H2 sections (`## Current State`, `## Current Milestone:`, `## What This Is`, `## Core Value`, `## Requirements`, `## Context`, `## Constraints`, `## Key Decisions`, `## Evolution`).
- H3 subsections (`### Validated`, `### Active (v2.2)`, `### Deferred (v2.3+)`, `### Out of Scope`).
- Tables for "Key Decisions" (3-column: Decision | Rationale | Outcome).
- Detail-collapsed sections via `<details><summary>...</summary>...</details>` for "Previously Shipped" and "Deferred candidate areas".
- Footer: `*Last updated: ...*`.

**Insertion point:** After `## Constraints` (line 132) and before `## Context` (line 134) — OR — append between `## Key Decisions` (table on lines 144-165) and `## Evolution` (line 166). The latter is preferable because "Upstream Parity Process" is workflow-procedural like Evolution, not constraint-like.

**Recommended block to insert** (D-15 + RESEARCH.md Open Question 5 recommendation: ~10 lines):
```markdown
## Upstream Parity Process

To prevent the Windows-vs-macOS parity gap from re-opening as upstream ships v0.41+:

1. **Inventory drift** — `make check-upstream-drift` reports unabsorbed upstream commits grouped by file category. JSON output (`make check-upstream-drift ARGS="--format json"`) is suitable for templates and CI.
2. **Scaffold the sync** — copy `.planning/templates/upstream-sync-quick.md` into `.planning/quick/YYMMDD-xxx-upstream-sync-vX.Y/PLAN.md` and fill placeholders.
3. **Cherry-pick per commit** — preserve the `Upstream-commit:` / `Upstream-tag:` / `Upstream-author:` / `Co-Authored-By:` / `Signed-off-by:` trailer block (template encodes the exact form).
4. **Verify Windows retrofit** — for every cross-platform feature absorbed, confirm the Windows path either exists or is added behind `#[cfg(target_os = "windows")]`.

For the long-form runbook (output formats, categorization rules, regeneration procedure for the test fixtures), see [`docs/cli/development/upstream-drift.mdx`](docs/cli/development/upstream-drift.mdx).
```

**Gotchas to preserve:**
- H2 (`##`), not H3 — matches "Evolution", "Constraints", "Context".
- Numbered list (matching the "Evolution" section's numbered steps style) is the right choice for a workflow.
- Bold lead-in `**Inventory drift** —` matches the existing bold-anchored bullet pattern in "Target features" (lines 13-19).
- Cross-link with relative path `docs/cli/development/upstream-drift.mdx` — matches the existing internal-link style (no in-document examples to validate against, but Mintlify/Markdown convention applies).
- Update `*Last updated: ...*` footer (line 184) to mention the new section if the planner wants strict tracking — but this is optional and may be the milestone-end's job, not this phase's.
- DO NOT touch the existing "Validated" / "Active (v2.2)" / "Deferred" lists — those evolve at phase transitions per the Evolution section's rules.

## Shared Patterns

### Twin-script parity discipline
**Source:** `scripts/test-linux.sh` + `scripts/build-windows-msi.ps1` + `scripts/windows-test-harness.ps1` (twin convention)
**Apply to:** Both `check-upstream-drift.sh` and `check-upstream-drift.ps1`
- Same CLI surface (`--from`/`-From`, `--to`/`-To`, `--format`/`-Format` with `table`/`json`).
- Same JSON output shape — byte-for-byte. The integration test diffs both outputs against the same fixture.
- When updating one twin, update the other. The diff-against-fixture in `test_upstream_drift.sh` is the structural enforcer.
- Bash uses `--double-dash-args`; PowerShell uses `-SingleDashArgs`. Both styles are correct for their language.

### Fail-loud-not-silent error handling
**Source:** CLAUDE.md ("Fail Secure: On any error, deny access. Never silently degrade") + `set -euo pipefail` in `scripts/test-linux.sh` + `Set-StrictMode -Version Latest` + `$ErrorActionPreference = "Stop"` in `scripts/build-windows-msi.ps1`
**Apply to:** Both drift scripts and `test_upstream_drift.sh`
- `git log` failure → `exit 1` with clear message, NEVER empty JSON.
- Missing `upstream` remote → `exit 1` with the actionable hint per D-10.
- Tag-resolution failure (no `v0.*` tag merged into HEAD) → `exit 1` with `Use --from <ref>` hint.
- Bash: `set -euo pipefail` at top, no `2>/dev/null` suppression on git commands.
- PowerShell: `Set-StrictMode -Version Latest` + `$ErrorActionPreference = "Stop"` at top.

### UTF-8 output discipline
**Source:** RESEARCH.md Pitfall 1 (verified empirically); not yet present in any repo script
**Apply to:** `check-upstream-drift.ps1` only (bash on Git-for-Windows MSYS handles UTF-8 by default; PS 5.1 does not)
```powershell
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$OutputEncoding = [System.Text.UTF8Encoding]::new()
```
- Place immediately after `$ErrorActionPreference = "Stop"` and before any output.
- Without this, non-ASCII commit subjects produce mojibake AND invalid UTF-8 bytes that break fixture diff.

### Single-brace placeholder convention
**Source:** `~/.claude/get-shit-done/templates/AI-SPEC.md` and 30 sibling templates (RESEARCH.md Pattern 7 verified across all 31)
**Apply to:** `.planning/templates/upstream-sync-quick.md` ONLY (NOT `.mdx` doc, NOT scripts)
- Use `{name}` form. NEVER `{{NAME}}` and NEVER `<NAME>`.
- Inline maintainer hints use `<!-- HTML comments -->`.
- Template's own header includes a "before-commit" checklist: replace all `{...}` markers (smoke-testable via `grep -oE '\\{[a-z_]+\\}' file | wc -l`).

### D-19 cherry-pick trailer block (verified verbatim)
**Source:** Commits `73e1e3b8`, `adf81aec`, `869349df` (verified via `git log -1 --pretty=format:'%B'`)
**Apply to:** `.planning/templates/upstream-sync-quick.md` (the template encodes the exact shape)
- Six lines, fixed order, separated from body by exactly one blank line.
- `Upstream-author` has lowercase `a` (NOT `Upstream-Author`).
- TWO `Signed-off-by` lines (full name + github handle), both required.
- 8-char abbreviated SHA is the in-use convention.
- Template's placeholder-substitution should NEVER produce a single-`Signed-off-by` block; the smoke test verifies `grep -c '^Signed-off-by: ' = 2`.

### Read-only invariant for the drift script
**Source:** D-11 ("read-only inventory tool — no git mutations") + CLAUDE.md security non-negotiables
**Apply to:** `check-upstream-drift.{sh,ps1}` and the integration test
- No `git fetch`, no `git config`, no `git tag` writes, no `git checkout`. Only `git log`, `git tag --list`, `git describe`, `git ls-remote --tags upstream` (read-only on remote refs).
- Integration test asserts `git status` is clean post-invocation (RESEARCH.md Security Domain final paragraph).
- D-10: missing `upstream` remote → exit 1 with hint, NEVER auto-add via `git remote add`.

## No Analog Found

| File | Role | Data Flow | Reason | Mitigation |
|------|------|-----------|--------|------------|
| `tests/integration/fixtures/upstream-drift/*.json` | frozen test fixtures | static JSON | No prior frozen-JSON-fixture test in `tests/integration/`; the existing tests use grep-against-output, not diff-against-file | Use the RESEARCH.md "Output shape" block as the schema; document regeneration in the companion `README.md` |
| `Makefile` cross-platform `ifeq ($(OS),Windows_NT)` dispatch | build-system | platform fork | No existing instance in this Makefile (all current Windows targets unconditionally invoke `pwsh -File ...`) | RESEARCH.md Pattern 6 specifies the canonical GNU Make idiom; first-use in this codebase |

## Metadata

**Analog search scope:**
- `scripts/` (twin-script convention)
- `Makefile` (target style + dispatch)
- `tests/integration/test_*.sh` + `tests/run_integration_tests.sh` + `tests/integration/test_setup.sh` (integration-test runner shape)
- `docs/cli/development/*.mdx` (10 files surveyed; settled on `testing.mdx` as closest)
- `.planning/quick/260424-upr-review-upstream-037-to-040/{PLAN,SUMMARY}.md` (frontmatter + headline shape)
- `~/.claude/get-shit-done/templates/AI-SPEC.md`, `research.md`, others (placeholder convention precedent)
- Recent fork commits `73e1e3b8`, `adf81aec`, `869349df` (D-19 trailer verbatim)
- `PROJECT.md` (existing section structure)

**Files scanned:** ~25 files across 7 directories.

**Pattern extraction date:** 2026-04-27.

**Confidence:** HIGH on every analog. The two "no analog found" rows are well-documented as first-of-kind file shapes with explicit RESEARCH.md schemas to follow.

## PATTERN MAPPING COMPLETE

**Phase:** 24 - parity-drift-prevention
**Files classified:** 11 (9 created + 2 modified)
**Analogs found:** 11 / 11

### Coverage
- Files with exact analog: 2 (`upstream-drift.mdx` ↔ `testing.mdx`; `PROJECT.md` ↔ itself)
- Files with role-match analog: 7 (both scripts, the integration test, the template, the Makefile edit, etc.)
- Files with no in-repo analog: 2 (the JSON fixtures collectively + the cross-platform Makefile dispatch idiom — both have RESEARCH.md schemas)

### Key Patterns Identified
- Twin-script parity is enforced at runtime by the integration test's byte-for-byte JSON diff against shared fixtures. Maintaining one twin without the other will fail the test.
- D-19 cherry-pick trailer block is verbatim across all three reference commits — the template must encode it character-exact, including the lowercase `a` in `Upstream-author` and the two `Signed-off-by` lines.
- PowerShell 5.1 has two specific footguns that bash does not: console encoding mojibake (fixed via `[Console]::OutputEncoding`) and `ConvertTo-Json` single-element-array unwrapping (fixed via `@()` wrapping). Both must be present in the `.ps1` from line one; the small-range fixture catches regressions.
- GSD template placeholder convention is single-brace `{name}` — universal across 31 templates. Do NOT use `{{NAME}}` or `<NAME>`.
- Makefile dispatch via `ifeq ($(OS),Windows_NT)` is new to this repo but is the standard GNU Make idiom; the existing `pwsh -File ...` Windows targets stay unchanged. The new target tries `pwsh` first, falls back to `powershell.exe` (verified `pwsh` is NOT on this maintainer's PATH).
- Integration tests under `tests/integration/test_*.sh` use `#!/bin/bash` (NOT env-bash) and source `tests/lib/test_helpers.sh`. The drift test follows that convention even though `scripts/test-linux.sh` (a different layer) uses env-bash.

### File Created
`C:\Users\OMack\Nono\.planning\phases\24-parity-drift-prevention\24-PATTERNS.md`

### Ready for Planning
Pattern mapping complete. Planner can now reference analog patterns + verbatim excerpts in PLAN.md actions, with explicit guidance on where in `Makefile` and `PROJECT.md` to insert new content.
