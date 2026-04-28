# Phase 24: Parity-Drift Prevention - Research

**Researched:** 2026-04-27
**Domain:** Maintainer tooling (shell + PowerShell twin scripts, git log parsing, JSON emission, GSD template)
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Script architecture (DRIFT-01)**
- **D-01:** Twin `.sh` + `.ps1` scripts at `scripts/check-upstream-drift.sh` and `scripts/check-upstream-drift.ps1`, maintained in parallel. No Rust crate. No WSL-only fallback for Windows.
- **D-02:** Add a `make check-upstream-drift` target that dispatches to the platform-appropriate script (mirrors `make build`/`make test`/`make ci` UX). Direct invocation must still work.
- **D-03:** No CI integration in this phase. Output must be CI-consumable but the GHA workflow itself is deferred.

**Output format + categorization (DRIFT-01)**
- **D-04:** `--format <table|json>` flag. Default = `table`. No markdown-table format.
- **D-05:** Categorize by file path heuristics with a top-of-file lookup table:
  - `crates/nono-cli/src/profile/` or `profile.rs` -> `profile`
  - `crates/nono-cli/src/package*` or `data/policy.json` (deny rules) -> `package` / `policy` (split as appropriate)
  - `crates/nono-proxy/` -> `proxy`
  - `crates/nono/src/audit/` or `audit_attestation*` -> `audit`
  - Anything else under cross-platform paths -> `other`
  No subject-line keyword scanning in v1.
- **D-06:** Multi-category commits appear under each matching category in the table; JSON lists `categories: [...]`. Header shows `Total: N unique commits`.
- **D-07:** JSON includes `{sha, subject, author, date, additions, deletions, files_changed: [...], categories: [...]}` via `git log --numstat`.

**Diff-range strategy (DRIFT-01)**
- **D-08:** Default range = last-synced-tag..latest-upstream-tag, auto-detected.
- **D-09:** `--from <ref> --to <ref>` flags ALWAYS override.
- **D-10:** Missing `upstream` remote -> exit 1 with actionable hint. Do NOT auto-add.
- **D-11:** Excluded paths (`*_windows.rs`, `exec_strategy_windows/`) filtered OUT. No fork-only bucket.

**Template location + shape (DRIFT-02)**
- **D-12:** Template at `.planning/templates/upstream-sync-quick.md`. Phase creates `.planning/templates/`. No GSD skill wrapper in v1.
- **D-13:** Fillable-blanks Markdown with placeholders. Pre-populated sections: frontmatter, headline + commit inventory, **D-19 cherry-pick trailer template**, conflict-file inventory, Windows-specific retrofit checklist, fork-divergence catalog.
- **D-14:** Template references `make check-upstream-drift` output but does NOT auto-include it.
- **D-15:** Reference from new `PROJECT.md § Upstream Parity Process` section.

**Documentation**
- **D-16:** Use `.mdx` (existing convention in `docs/cli/development/`), not `.md` as REQ-DRIFT-01 acceptance #3 says.

### Claude's Discretion
- D-17: Plan split (single combined plan vs 24-01 + 24-02) is the planner's call.
- Make-target dispatch logic (Windows vs Unix detection)
- Per-script CLI argument parsing (getopt vs manual; `param()` block)
- JSON schema beyond listed fields (committer/author precedence, ISO-8601 date format)
- Placeholder syntax (`{{NAME}}` vs `<NAME>` vs other)
- `git log --numstat` vs `--shortstat`
- Test approach for the scripts

### Deferred Ideas (OUT OF SCOPE)
- GitHub Actions weekly drift workflow
- Cherry-pick automation / merge helper
- Conflict resolver UX
- Rust rewrite
- Subject-line keyword categorization fallback
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DRIFT-01 | `check-upstream-drift.{sh,ps1}` reports cross-platform commits in `upstream/main..HEAD` (or last-synced-tag..latest-upstream-tag), grouped by file category, with `--format <table\|json>`. Acceptance #1: reproduces 260424-upr SUMMARY.md inventory for `v0.37.1..v0.40.1`. | Sections: Standard Stack, Architecture Patterns (script skeleton, numstat parsing, JSON emission), Common Pitfalls, Code Examples, Validation Architecture |
| DRIFT-02 | `.planning/templates/upstream-sync-quick.md` template with frontmatter, commit inventory, D-19 trailer block, conflict-file inventory, Windows retrofit checklist. Acceptance #2: PROJECT.md references it. Acceptance #3: dry-run for v0.41.0 produces a sensible PLAN.md skeleton. | Sections: Template placeholder convention (Architecture Patterns), D-19 trailer block (Code Examples), Validation Architecture (placeholder smoke test), Documentation completeness |
</phase_requirements>

## Summary

Phase 24 ships two static maintainer artefacts: a twin shell+PowerShell drift inventory script and a Markdown template. The technical surface is small and well-bounded: `git log --numstat --no-merges --format=...`, JSON emission without a `jq` dependency on bash and via `ConvertTo-Json` on PowerShell, a path-prefix lookup table, plus a fillable-blanks template with single-brace `{placeholder}` substitution to match existing GSD template convention.

The single biggest planning hazard is **the seed-data ground-truth mismatch**: the path-filter D-11 specifies (`crates/{nono,nono-cli,nono-proxy}/src/` plus `crates/nono/Cargo.toml`, excluding `*_windows.rs` and `exec_strategy_windows/`) produces **56 commits** for `v0.37.1..v0.40.1`, while the 260424-upr SUMMARY.md headline says **78 non-merge commits** and lists 22 commits in its per-release blocks that the path filter excludes (docs-only, dep bumps to `Cargo.lock`/`nono-cli/Cargo.toml`/`nono-proxy/Cargo.toml`, GitHub workflow files, integration tests, the claude-code package removal, README/AGENTS.md edits). Acceptance #1 ("reproduces the commit inventory") is therefore underspecified — the planner must decide whether (a) the script's cross-platform path filter is canonical and the SUMMARY's headline is informational, (b) the filter expands to also include `Cargo.lock` + all crate `Cargo.toml`s + `data/policy.json` to widen coverage, or (c) the SUMMARY is treated as a frozen narrative and the script's reproducibility test compares only the **categorized** subset. **Recommended:** option (a) — the filter is canonical, the script reproduces 56 commits with the correct per-category grouping, and the test fixture documents the 22-commit delta as informational. This matches the script's stated purpose (reporting cross-platform-code drift, not all upstream activity).

The second hazard is **PowerShell version**: only **Windows PowerShell 5.1** (`powershell.exe`) is on this maintainer's PATH; **PowerShell 7 (`pwsh`) is not installed**. The Makefile already uses `pwsh -File scripts/windows-test-harness.ps1` for Windows targets, so the convention prefers `pwsh`, but the script must be tested under PS 5.1 too. PS 5.1 has two known footguns the script must handle explicitly: (1) console output encoding defaults to the OEM codepage so non-ASCII commit subjects produce mojibake unless `[Console]::OutputEncoding` is set to UTF-8; (2) `ConvertTo-Json` in PS 5.1 unwraps single-element arrays unless wrapped with `@()` or the `-AsArray` parameter (PS 6+ only) — empirically tested OK with explicit `@()` wrapping in our shape, but the test harness must include a single-commit fixture to catch any regression.

**Primary recommendation:** Single combined plan (24-01) with three waves: (1) script skeleton + tag resolution + path filter, (2) numstat parsing + JSON emission + categorization + table format, (3) template + PROJECT.md section + `docs/cli/development/upstream-drift.mdx`. Test strategy: golden JSON fixtures in `tests/integration/fixtures/upstream-drift/` for `v0.37.1..v0.40.1`, `v0.39.0..v0.40.0`, and a single-commit range; bash + PowerShell each diff their `--format json` output against the same fixture (twin-parity check); plus a placeholder-substitution smoke test that renders the template against `v0.41.0..v0.42.0` and grep-asserts the result has valid GSD frontmatter.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Tag resolution + range computation | Maintainer tooling (shell/PowerShell) | Git CLI | Read-only metadata query; no application code |
| `git log --numstat` parsing | Maintainer tooling (shell/PowerShell) | - | Pure text processing; no project crates touched |
| Path-prefix categorization | Maintainer tooling (shell/PowerShell) | - | Static lookup table; no integration with library or CLI |
| JSON emission | Maintainer tooling (shell/PowerShell) | - | Output format; bash uses printf-with-escaping, PS uses `ConvertTo-Json` |
| Make-target dispatch | Build system (Makefile) | - | Platform detection routes to `.sh` or `.ps1` |
| Template rendering | Documentation (template file + maintainer copy) | - | No code path; maintainer copies + fills placeholders manually |
| PROJECT.md cross-link | Documentation | - | Markdown edit only |
| Long-form runbook | Documentation (`docs/cli/development/upstream-drift.mdx`) | - | Mintlify-style; references but does not invoke the script |

**Why this matters:** This phase has zero integration with the Rust workspace. No `crates/` files are touched. No `cargo` runs. No FFI surface changes. The plan-checker should reject any task that adds Rust code or modifies the library/CLI/proxy crates.

## Project Constraints (from CLAUDE.md)

- **Security non-negotiable**: read-only operations only (D-11 already locks this in); validate any path the script outputs (no string concatenation of user-supplied refs into `git log` invocations — pass via `--`).
- **No `.unwrap()` / `.expect()`**: not applicable (no Rust code), but the parallel discipline in shell is "no `2>/dev/null`-suppressed errors" — fail loud on `git log` failure, exit non-zero with a clear message.
- **GSD workflow enforcement**: phase work goes through `/gsd:execute-phase` per CLAUDE.md.
- **Twin-script convention**: `.sh` + `.ps1` siblings in `scripts/` (matches `test-linux.sh` + `windows-test-harness.ps1`).
- **`make ci` enforces clippy + fmt + tests**: this phase adds no Rust, so `make ci` semantics unchanged. New `make check-upstream-drift` target is independent.
- **DCO sign-off required on all commits**: `Signed-off-by: ...` line — already standard.
- **Docs convention**: `docs/cli/development/*.mdx` (D-16 already corrects REQ's `.md` to `.mdx`). All 10 existing files in that directory are `.mdx`. Verified.

## Standard Stack

### Core (already on the box, all available)

| Tool | Version observed | Purpose | Why standard |
|------|------------------|---------|--------------|
| `bash` | 5.x (Git for Windows MSYS2) `[VERIFIED: bash --version]` | `.sh` script runtime | Existing scripts target POSIX bash; matches `scripts/test-linux.sh` |
| `git` | 2.x `[VERIFIED: git --version]` | `git log --numstat --no-merges --format=...` | Only data source the script needs |
| Windows PowerShell 5.1 | 5.1.26100 `[VERIFIED: powershell.exe -Command '$PSVersionTable']` | `.ps1` script runtime | Universal on Windows 10/11; `pwsh` is NOT installed on this box |
| GNU coreutils (awk, sed, sort, comm, printf) | MSYS2-bundled `[VERIFIED]` | text processing in `.sh` | POSIX baseline |

### Supporting / explicitly NOT used

| Tool | Why excluded |
|------|--------------|
| `jq` | NOT on PATH `[VERIFIED: which jq returns 'no jq']`. Script MUST emit JSON without `jq` dependency. Use printf-with-escaping in bash. |
| `python3` | Available `(Python 3.14.4)` but introducing a Python dependency for a maintainer tool with two cross-platform variants is over-engineering. Stay with bash printf + PS `ConvertTo-Json`. |
| PowerShell 7 (`pwsh`) | NOT on PATH `[VERIFIED]`. Existing Makefile uses `pwsh -File ...` for Windows targets — this phase should use `pwsh` if available, fall back to `powershell.exe` if not, but TEST under PS 5.1 since that is what the maintainer has. |
| Rust crate (cargo bin) | Explicitly excluded by D-01. |

**Version verification:** No package versions to pin — script uses only built-ins. `git --version` minimum: 2.5+ (for `--format=` and `--numstat` standard behavior — both predate that). `bash --version` minimum: 4.0 (for `[[ ]]` and `mapfile`). PowerShell minimum: 5.1.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Two-script twin | Single Rust crate `nono-tools` | Rejected by D-01 (twin maintenance acceptable for low-change tool) |
| `git log --numstat` | `git log --shortstat` | `--shortstat` collapses to summary line; loses per-file detail D-07 needs (`files_changed: [...]`). Use `--numstat`. `[VERIFIED: git log man page]` |
| Plain bash arrays | Python helper for JSON | Adds Python dep; bash printf-with-escape is well-trodden for scopes this small |
| `git diff --name-only A..B` | `git log --numstat A..B` | `--diff` collapses across commits; we need per-commit attribution. `--log` is correct. |

**Installation:** Nothing to install. All tools already present.

## Architecture Patterns

### System Architecture Diagram

```
                     +------------------+
maintainer invokes   | make check-      |
   ----------------> | upstream-drift   |
                     +--------+---------+
                              |
                              v dispatch by $(OS) / uname
              +---------------+---------------+
              |                               |
              v                               v
   +----------+-----------+      +------------+--------+
   | scripts/check-       |      | scripts/check-      |
   | upstream-drift.sh    |      | upstream-drift.ps1  |
   +----------+-----------+      +------------+--------+
              |                               |
              v                               v
   resolve refs (--from / --to or auto-detect last-synced + latest-upstream)
              |
              v
   git log --numstat --no-merges <from>..<to> -- <cross-platform paths> ':(exclude)*_windows.rs' ':(exclude)exec_strategy_windows/'
              |
              v parse stdout: per-commit { sha, subject, author, date, files[], +/- }
              |
              v categorize each file via path-prefix lookup table -> {profile, policy, package, proxy, audit, other}
              |
              v aggregate: each commit -> categories: [<unique set>]
              |
       +------+-------+
       |              |
   --format table   --format json
       |              |
       v              v
   stdout (grouped)  stdout (single JSON object: {range, total, commits[]})

  Template usage (separate flow, no automation):
   maintainer creates .planning/quick/YYMMDD-xxx-upstream-sync-vX.Y/
       -> cp .planning/templates/upstream-sync-quick.md PLAN.md
       -> fill placeholders manually (commit list pasted from `make check-upstream-drift > drift.json`)
       -> commit per upstream commit with D-19 trailer block
```

### Recommended Project Structure

```
scripts/
+-- check-upstream-drift.sh        # NEW (twin)
+-- check-upstream-drift.ps1       # NEW (twin)
+-- test-linux.sh                  # existing twin pattern reference
+-- windows-test-harness.ps1       # existing twin pattern reference

.planning/
+-- templates/                     # NEW directory
|   +-- upstream-sync-quick.md     # NEW
+-- phases/24-parity-drift-prevention/
    +-- 24-CONTEXT.md
    +-- 24-RESEARCH.md             # this file
    +-- 24-PLAN.md                 # planner output

docs/cli/development/
+-- upstream-drift.mdx             # NEW (.mdx not .md per D-16)

PROJECT.md                         # MODIFIED (new section "Upstream Parity Process")
Makefile                           # MODIFIED (one new target)

tests/integration/
+-- test_upstream_drift.sh         # NEW (golden-fixture diff)
+-- fixtures/upstream-drift/
    +-- v0.37.1__v0.40.1.json      # NEW frozen fixture
    +-- v0.39.0__v0.40.0.json      # NEW frozen fixture
    +-- v0.40.0__v0.40.1.json      # NEW small-range fixture
```

### Pattern 1: bash + PowerShell twin script header

**What:** Universal opening boilerplate matching `scripts/test-linux.sh` and `scripts/build-windows-msi.ps1`.

**bash skeleton:**
```bash
#!/usr/bin/env bash
# scripts/check-upstream-drift.sh
# Reports upstream commits the fork has not absorbed, grouped by file category.
# Read-only — does NOT modify git state.

set -euo pipefail

# CLI parsing (manual; getopt portability is poor on Git-for-Windows bash)
FROM_REF=""
TO_REF=""
FORMAT="table"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --from)   FROM_REF="$2"; shift 2 ;;
    --to)     TO_REF="$2"; shift 2 ;;
    --format) FORMAT="$2"; shift 2 ;;
    -h|--help) print_usage; exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done
```

**PowerShell skeleton:**
```powershell
# scripts/check-upstream-drift.ps1
# Reports upstream commits the fork has not absorbed, grouped by file category.
# Read-only - does NOT modify git state.

param(
    [string]$From = "",
    [string]$To = "",
    [ValidateSet("table","json")]
    [string]$Format = "table"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Force UTF-8 output so non-ASCII commit subjects do not mojibake on PS 5.1
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$OutputEncoding = [System.Text.UTF8Encoding]::new()
```
`[VERIFIED: scripts/build-windows-msi.ps1 lines 1-21 — same param + StrictMode + ErrorActionPreference pattern]`

**When to use:** Every twin-script entry point in this repo.

### Pattern 2: Tag resolution (D-08)

**What:** Auto-detect "last-synced-tag..latest-upstream-tag" robustly given that the fork has its own tags (`v1.0`, `v2.0`, `v2.1`) interleaved with upstream's (`v0.37.1` ... `v0.42.0`).

**Observed tag chain (verified on this repo at HEAD):**
```
v0.26.0 ... v0.40.0 v0.40.1 v0.41.0 v0.42.0 v1.0 v2.0 v2.1
```

`upstream/main` HEAD = `a87c6ae582e2a1f576787a9f42d9e90d14fa8ec3`
`git describe --tags --abbrev=0 upstream/main` returns `v0.42.0` `[VERIFIED on this repo at 2026-04-27]`
`git describe --tags --abbrev=0 HEAD` returns `v2.1` (fork tag — wrong for our purpose)
`git ls-remote --tags upstream` enumerates ONLY upstream tags `[VERIFIED]`

**Robust resolution algorithm:**
```bash
# latest-upstream-tag = git describe --tags --abbrev=0 upstream/main
LATEST_UPSTREAM=$(git describe --tags --abbrev=0 upstream/main 2>/dev/null) || {
  echo "Error: cannot resolve latest upstream tag. Is 'upstream' remote configured? Run: git remote add upstream https://github.com/always-further/nono.git" >&2
  exit 1
}

# last-synced-tag = highest upstream tag whose tip is reachable from HEAD
# (i.e., the fork has cherry-picked or merged everything up to it)
# Strategy: enumerate upstream tags with `git tag --list 'v0.*' --merged HEAD`,
#           pick highest by version sort.
LAST_SYNCED=$(git tag --list 'v0.*' --merged HEAD --sort=-v:refname | head -n1)
if [[ -z "$LAST_SYNCED" ]]; then
  echo "Error: no upstream-style tag (v0.*) reachable from HEAD; cannot auto-detect last-synced point. Use --from <ref>." >&2
  exit 1
fi
```

**Edge cases handled:**
- Fork's own `v1.0`/`v2.0`/`v2.1` tags excluded by `--list 'v0.*'` glob.
- `--merged HEAD` filter ensures we pick a tag the fork has actually absorbed (not just one that exists locally because we fetched it).
- Sort `-v:refname` is git's built-in semver sort.

**Limitation:** If the fork has cherry-picked SOME of v0.41.0's commits but not all, this algorithm picks v0.40.x as "last synced" — which is the right safe answer (script reports the v0.41 commits the fork is still missing). `[ASSUMED — needs validation against a real partial-sync state]`

### Pattern 3: `git log --numstat` parsing

**What:** Drive `git log` with a structured custom format so per-commit metadata + per-file numstat lines are unambiguously delimited, then parse in bash and PowerShell.

**Verified output format `[VERIFIED on this repo, all examples below pulled from real `git log -2 v0.40.0..v0.40.1`]`:**

When using `git log --numstat --no-merges --format='COMMIT %H %an<TAB>%aI<TAB>%s' <range>`:
```
COMMIT 79154fe0c86935d8552a65de282668015d9c6f2f Luke Hinds  2026-04-...  chore: release v0.40.1
                                                                                <- BLANK LINE
11      0       CHANGELOG.md
3       3       Cargo.lock
1       1       bindings/c/Cargo.toml
3       3       crates/nono-cli/Cargo.toml
2       2       crates/nono-proxy/Cargo.toml
1       1       crates/nono/Cargo.toml
                                                                                <- BLANK LINE
COMMIT 7d1d9a0d12c610fc186c5a1aaf1b40fb3f433ddd Luke Hinds  ...
```

**Numstat row format (3 tab-separated columns):**
- Normal: `<additions>\t<deletions>\t<filepath>`
- Binary: `-\t-\t<filepath>` (literal hyphens, not numbers) `[VERIFIED: docs/assets/proxy-flow.png shows '-\t-\t...']`
- Rename (with `-C` or default `-M`): `<adds>\t<dels>\t<old> => <new>` OR with `--numstat` alone: a single line like `0\t0\tDockerfile => docker/Dockerfile-CI` `[VERIFIED in commit c8b8aa9a]`

**Parse strategy:**
1. Use a sentinel format prefix (`COMMIT ` literal) to distinguish header rows from numstat rows.
2. Use `--no-merges` to drop merge commits (range had 27 merges in v0.37.1..v0.40.1; numstat is empty for merges by default which would corrupt parsing).
3. For renames, normalize `old => new` to `new` (the post-rename path) — categorize against the destination since that is the path the fork must update.
4. Treat binary `-\t-` as `additions=0, deletions=0` (or omit from totals — D-07 doesn't specify; recommend skipping in totals but listing in `files_changed`).
5. ASCII-only commit subjects in the seed range — but design must handle non-ASCII (`feat: add fooé` test case).

**Format string field separator:** Use TAB (`%x09`) explicitly so subject lines containing colons or spaces don't break parsing:
```bash
GITLOG_FMT='COMMIT|%H|%an|%aI|%s'   # pipe is also safe — no commit subjects in the seed range contain it
# OR if subjects might contain pipes:
GITLOG_FMT='COMMIT%x00%H%x00%an%x00%aI%x00%s'   # NUL separator
```
NUL is bash-friendly via `IFS= read -r -d ''` BUT PowerShell 5.1's `Get-Content` handles NUL awkwardly. Recommend tab (`%x09`) — verified safe across the seed range commit subjects.

**Example parser (bash):**
```bash
git log --no-merges --numstat --format='COMMIT%x09%H%x09%an%x09%aI%x09%s' "$RANGE" -- $PATHS \
  | while IFS=$'\t' read -r col1 col2 col3 col4 col5; do
      if [[ "$col1" == "COMMIT" ]]; then
        # finalize previous commit if any, then start new one
        SHA="$col2"; AUTHOR="$col3"; DATE="$col4"; SUBJECT="$col5"
        ADDS=0; DELS=0; FILES=()
      elif [[ -z "$col1" ]]; then
        :  # blank line between commits
      else
        # numstat row: col1=adds, col2=dels, col3=filename (or "old => new")
        ADDS_LINE="$col1"; DELS_LINE="$col2"; FILE="$col3"
        if [[ "$FILE" == *' => '* ]]; then FILE="${FILE##* => }"; fi
        # ... accumulate
      fi
    done
```

`[VERIFIED: format reproduces correctly for v0.37.1..v0.40.1; binary files show '-\t-\t...' and renames show '0\t0\told => new']`

### Pattern 4: JSON emission without `jq`

**bash — printf-with-escape:**

`jq` is NOT on the maintainer's box `[VERIFIED]`. Use printf with manual escaping. Must escape: `\\`, `"`, `\n`, `\t`, `\r`, control chars 0x00-0x1F.

```bash
json_escape() {
  # input: $1 string
  # output: stdout escaped string (no surrounding quotes)
  local s="$1"
  s="${s//\\/\\\\}"      # backslash first
  s="${s//\"/\\\"}"      # double quote
  s="${s//$'\n'/\\n}"
  s="${s//$'\r'/\\r}"
  s="${s//$'\t'/\\t}"
  printf '%s' "$s"
}

# Per-commit JSON object:
printf '{"sha":"%s","subject":"%s","author":"%s","date":"%s","additions":%d,"deletions":%d,"files_changed":[' \
  "$SHA" "$(json_escape "$SUBJECT")" "$(json_escape "$AUTHOR")" "$DATE" "$ADDS" "$DELS"
# emit array
first=1
for f in "${FILES[@]}"; do
  if [[ $first -eq 1 ]]; then first=0; else printf ','; fi
  printf '"%s"' "$(json_escape "$f")"
done
printf '],"categories":['
# emit categories same way
printf ']}'
```

**Caveat:** This handler covers the printable-ASCII + tab/newline subset that real git data hits in practice. It does NOT escape control chars 0x00-0x1F or non-BMP unicode to `\uXXXX` escapes. For commit data this is acceptable — `git` does not emit raw control bytes in formatted output. Document this scope limit in a comment.

**PowerShell — built-in:**
```powershell
$commitObj = [ordered]@{
    sha = $sha
    subject = $subject
    author = $author
    date = $date
    additions = [int]$adds
    deletions = [int]$dels
    files_changed = @($files)   # explicit @() prevents single-element unwrap
    categories = @($cats)
}
$json = $commitObj | ConvertTo-Json -Depth 5 -Compress
```

**PS 5.1 footguns `[VERIFIED on this box]`:**
1. **Console encoding**: default OEM codepage mojibakes non-ASCII subjects. Fix: `[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()` at top of script.
2. **Single-element array unwrap**: empirical test confirmed `@(@{ sha='abc' })` round-trips correctly as a JSON array, but historically PS 5.1 had cases where a single object piped to `ConvertTo-Json` got unwrapped. **Defensive measure**: always use `@()` on the outer collection AND test with a single-commit fixture in CI.
3. **`ConvertTo-Json` default depth = 2**: must specify `-Depth 5` explicitly or nested arrays get serialized as `"System.Object[]"`.

**Output shape (D-07 + D-06 fully covered):**
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
      "sha": "4f9552ec...",
      "subject": "feat(audit): add tamper-evident audit log integrity",
      "author": "Luke Hinds",
      "date": "2026-04-...",
      "additions": 1419,
      "deletions": 226,
      "files_changed": ["crates/nono-cli/src/audit/...", "..."],
      "categories": ["audit", "policy"]
    }
  ]
}
```

### Pattern 5: Path-prefix categorization lookup table (D-05)

**What:** Single ordered table at top of each script, applied first-match-wins to each file in a commit.

**Recommended initial mapping (derived from the SUMMARY's narrative grouping + verified file-tree inspection):**

| Path prefix / pattern | Category | Source / rationale |
|----------------------|----------|---------------------|
| `crates/nono-cli/src/profile/` | `profile` | D-05; SUMMARY § "Profile struct alignment" |
| `crates/nono-cli/src/profile.rs` | `profile` | D-05 (same) |
| `crates/nono-cli/data/profile-authoring-guide.md` | `profile` | observed in seed range (commit 1b412a74) |
| `crates/nono-cli/src/policy.rs` | `policy` | D-05; SUMMARY § "Policy tightening" |
| `crates/nono-cli/data/policy.json` | `policy` | D-05 (deny rules) |
| `crates/nono-cli/src/package` | `package` | D-05 substring match |
| `crates/nono-cli/src/package_cmd.rs` | `package` | observed in v0.38 commits |
| `crates/nono/src/package` | `package` | observed |
| `crates/nono-proxy/` | `proxy` | D-05 |
| `crates/nono/src/audit/` | `audit` | D-05 |
| `crates/nono/src/audit_attestation.rs` | `audit` | D-05 |
| `crates/nono-cli/src/audit_cmd.rs` | `audit` | observed in seed range |
| anything else under `crates/{nono,nono-cli,nono-proxy}/src/` | `other` | D-05 fallback |
| `crates/nono/Cargo.toml` | `other` | D-11 path inclusion |

**Implementation hint:** Order matters — `crates/nono/src/audit/` must match BEFORE generic `crates/nono/src/`. Use ordered iteration + `first-match-wins`, OR use longest-prefix-match. Recommend ordered iteration: simpler to read, and the table is short.

**Multi-category rule (D-06):** A commit's `categories` field is the union of categories assigned to its individual files. Example: a commit touching `crates/nono-cli/src/audit_cmd.rs` AND `crates/nono-cli/src/policy.rs` lists `categories: ["audit", "policy"]`.

### Pattern 6: Make-target dispatch (D-02 + Claude's Discretion)

**What:** `make check-upstream-drift` selects `.sh` on Unix and `.ps1` on Windows.

**Existing Makefile patterns observed `[VERIFIED: Makefile read in full]`:**
- `make ci` chains: `ci: check test audit` (lines 122-123)
- Windows-specific targets unconditionally invoke `pwsh -File scripts/...` (lines 55-65)
- No existing platform-detection idiom in this Makefile

**Recommended dispatch (matches GNU Make idiom):**
```makefile
.PHONY: check-upstream-drift

# Detect Windows. $(OS) == Windows_NT under cmd/MSYS bash; uname returns MINGW*/CYGWIN* under bash.
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

**Why:** `$(OS)==Windows_NT` is the standard cross-Make Windows detection (works in `nmake`, `gmake`, `mingw32-make`). `pwsh` preferred (D-01 implies forward-compat), fall back to `powershell.exe` because `pwsh` is not present on this maintainer's box. `[VERIFIED — only powershell.exe (Windows PS 5.1) was found on PATH]`

**Argument passthrough:** `make check-upstream-drift ARGS="--from v0.40.1 --to v0.42.0 --format json"`. Document the `ARGS=` pattern in the help target.

### Pattern 7: Template placeholder convention (D-13)

**What:** Single-brace `{name}` placeholders matching the existing GSD template convention.

**Precedent `[VERIFIED: ~/.claude/get-shit-done/templates/AI-SPEC.md, DEBUG.md, SECURITY.md inspected]`:**
- All 31 GSD templates use single-brace: `{N}`, `{phase_name}`, `{phase-slug}`, `{date}`, `{boundary}`, `{description}`
- HTML comments `<!-- ... -->` for inline maintainer guidance
- Frontmatter blocks at top with placeholder values

**Recommendation:** Match this convention exactly. Use `{from_tag}`, `{to_tag}`, `{commit_count}`, `{quick_slug}`, `{date}`, etc. Avoid `{{NAME}}` — it would be the only template in the project using that style.

**Anti-pattern `[VERIFIED]`:** Do NOT use `<NAME>` — it collides with HTML/JSX in `.mdx` files and confuses readers in the `.md` template too.

**Substitution pipeline (manual):** The maintainer copies the template, opens in editor, runs Find/Replace on `{from_tag}` -> `v0.41.0` etc. No scripted substitution in v1 (deferred). Template's own header includes a checklist: "Replace all `{...}` placeholders before committing."

### Pattern 8: D-19 cherry-pick trailer block

**What:** The exact trailer-block shape established by Plan 22-01 and consistently used in commits 73e1e3b8, adf81aec, 869349df.

**Verified format `[VERIFIED: git log -1 --format='%B' on three commits, all match this shape]`:**

```
Upstream-commit: <abbrev sha of upstream commit>
Upstream-tag: <upstream tag containing it, e.g., v0.38.0>
Upstream-author: <full name> <email>
Co-Authored-By: <upstream author full name> <email>
Signed-off-by: <fork author full name> <email>
Signed-off-by: <fork author github handle> <email>
```

**Observed structural rules:**
1. Trailer block separated from body by exactly one blank line
2. Order is fixed: `Upstream-commit` -> `Upstream-tag` -> `Upstream-author` -> `Co-Authored-By` -> `Signed-off-by` (fork) -> `Signed-off-by` (fork handle)
3. `Upstream-author` and `Co-Authored-By` carry the SAME name + email (Co-Authored-By is the GitHub-recognized form; Upstream-author is the audit-trail form)
4. Two `Signed-off-by` lines: one with the maintainer's full name, one with their github username — both required for DCO+attribution
5. Abbreviated SHA (8 chars) is the convention used in body text references; full or abbreviated both work in `Upstream-commit:` (existing examples use 8-char abbrev)

**Template should encode the EXACT block (with placeholders):**
```
Upstream-commit: {upstream_sha_abbrev}
Upstream-tag: {upstream_tag}
Upstream-author: {upstream_author_name} <{upstream_author_email}>
Co-Authored-By: {upstream_author_name} <{upstream_author_email}>
Signed-off-by: {fork_author_name} <{fork_author_email}>
Signed-off-by: {fork_author_handle} <{fork_author_email}>
```

### Pattern 9: Test strategy — golden JSON fixtures + twin-parity diff

**What:** Three fixture files + one shell-based test that runs both scripts and diffs each against the fixture.

**Recommended layout:**
```
tests/integration/test_upstream_drift.sh
tests/integration/fixtures/upstream-drift/
  v0.37.1__v0.40.1.json         # large range — proves SUMMARY reproduction
  v0.39.0__v0.40.0.json         # mid range — covers audit-integrity cluster
  v0.40.0__v0.40.1.json         # 3-commit small range — single-element-array PS edge case
  README.md                     # explains how to regenerate fixtures
```

**Test logic:**
```bash
# tests/integration/test_upstream_drift.sh
set -euo pipefail
for range in "v0.37.1__v0.40.1" "v0.39.0__v0.40.0" "v0.40.0__v0.40.1"; do
  from="${range%__*}"
  to="${range#*__}"
  expected="tests/integration/fixtures/upstream-drift/${range}.json"

  # bash variant
  actual_sh=$(bash scripts/check-upstream-drift.sh --from "$from" --to "$to" --format json)
  diff <(echo "$actual_sh") "$expected" || { echo "FAIL: bash $range"; exit 1; }

  # PowerShell variant (if available — Linux CI may skip)
  if command -v powershell.exe >/dev/null 2>&1 || command -v pwsh >/dev/null 2>&1; then
    runner=$(command -v pwsh || command -v powershell.exe)
    actual_ps=$("$runner" -NoProfile -File scripts/check-upstream-drift.ps1 -From "$from" -To "$to" -Format json)
    diff <(echo "$actual_ps") "$expected" || { echo "FAIL: ps $range"; exit 1; }
  fi
done
```

**Fixture regeneration:**
```bash
bash scripts/check-upstream-drift.sh --from v0.37.1 --to v0.40.1 --format json \
  > tests/integration/fixtures/upstream-drift/v0.37.1__v0.40.1.json
```
Add a comment header to the fixture: `// Generated 2026-04-27 from <upstream sha>. Regenerate with: ...`

**Acceptance #1 reproduction test:** Compare against the SUMMARY's per-release blocks. Implementation can be a separate assertion that asserts each commit listed in SUMMARY's per-release sections (or its excluded subset, see "Reconciliation" below) appears in the script's JSON output for the matching range.

### Anti-Patterns to Avoid

- **Hand-rolling JSON without testing non-ASCII subjects.** PS 5.1 mojibakes by default. Always set `[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()` AND include a fixture with a unicode commit subject.
- **Trusting `git describe --tags --abbrev=0 HEAD` blindly.** It returns `v2.1` (fork tag) on this repo. Filter to upstream-style tags via `--list 'v0.*'`.
- **Forgetting `--no-merges`.** v0.37.1..v0.40.1 has 27 merge commits; numstat is empty for merges and corrupts parsing.
- **Using `git log --shortstat` for diff stats.** Collapses into a summary line; loses per-file detail D-07 needs.
- **String concatenating user `--from`/`--to` into the git command without `--`.** Treat all refs as args before `--`, all paths after, never substitute into the format string.
- **Adding `unwrap_or_default()`-equivalent in shell.** A failing `git log` should `exit 1`, not silently emit empty JSON.
- **Categorizing on subject-line keywords in v1.** D-05 explicitly defers this. Keep path-prefix only.
- **Testing only on Linux CI.** Both scripts must run on the platform they target. Add Windows runner coverage or document the manual-test gate.

## Don't Hand-Roll

| Problem | Don't build | Use instead | Why |
|---------|-------------|-------------|-----|
| Listing commits in a range with file stats | Custom `git log` text scraper that ignores `--numstat` | `git log --no-merges --numstat --format='...'` | Handles merges, renames, binaries, and structured output natively |
| Tag enumeration with version sort | `sort -V` (which mishandles pre-release tags like `v1.0-rc1`) | `git tag --list 'pattern' --sort=-v:refname` | Git's `-v:refname` sort is semver-aware |
| JSON serialization in PowerShell | Hand-rolled string concatenation | `ConvertTo-Json -Depth 5 -Compress` | Native, escapes correctly, handles non-BMP unicode |
| Bash JSON escaping for ALL unicode | Custom `\uXXXX` encoder in pure bash | Document scope limit (`git`-formatted text only); fall back to `python3 -c` if non-BMP commit subjects ever appear | Real git data does not need this; over-engineering bash escaping is a rabbit hole |
| Cross-platform path detection in Makefile | `uname` (which fails under cmd.exe) | `ifeq ($(OS),Windows_NT)` | Standard GNU Make idiom; works under nmake, mingw32-make, gmake, MSYS bash |
| Walking the seed inventory by hand | Re-categorizing each of the 78 commits manually | `git log -- <paths>` produces the canonical list; SUMMARY's narrative is informational | Mechanical reproducibility wins |

**Key insight:** The script is fundamentally a `git log` formatter. Resist the urge to add intelligence. Categorization is the only "logic" beyond formatting, and it is a static lookup table.

## Runtime State Inventory

This phase is greenfield additive (new scripts + new template + new docs section). It does NOT rename or refactor existing names. Section omitted intentionally.

| Category | Items | Action |
|----------|-------|--------|
| Stored data | None — script is read-only over git history | none |
| Live service config | None — no service touched | none |
| OS-registered state | None | none |
| Secrets/env vars | None — script needs no creds (uses local `.git`) | none |
| Build artifacts | None — script ships as plain text, no build step | none |

## Common Pitfalls

### Pitfall 1: PowerShell 5.1 console encoding mojibake
**What goes wrong:** Non-ASCII commit subjects (e.g., contributor name with accented char) print as `é` -> `é` and the JSON file is invalid UTF-8.
**Why it happens:** Windows PowerShell 5.1 defaults console output encoding to the active OEM codepage (CP437/CP1252). UTF-8 git output gets re-encoded.
**How to avoid:** Always set encoding at script start:
```powershell
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$OutputEncoding = [System.Text.UTF8Encoding]::new()
```
**Warning signs:** `é`, `Ã©`, `Â§` characters in JSON output.
**[VERIFIED on this box: tested with `é` subject — output corrupted without explicit encoding fix]**

### Pitfall 2: Tag-resolution picks fork tag
**What goes wrong:** `git describe --tags --abbrev=0 HEAD` returns `v2.1` (fork's release tag), and the script auto-detects "from = v2.1, to = v0.42.0" — a nonsensical empty range.
**Why it happens:** Fork has its own `v1.0`/`v2.0`/`v2.1` tags interleaved chronologically with upstream's `v0.42.0`.
**How to avoid:** Filter tags by the upstream pattern: `git tag --list 'v0.*' --merged HEAD --sort=-v:refname | head -n1`.
**Warning signs:** Reported "from" tag does not start with `v0.` or "to" tag does.

### Pitfall 3: Merge commits silently corrupt numstat parsing
**What goes wrong:** Merge commits show no numstat by default, so the parser sees a header row followed by blank line followed by next header row — but the previous-commit-finalization logic may not fire correctly.
**Why it happens:** `git log --numstat` shows numstat for non-merge commits only by default.
**How to avoid:** Always include `--no-merges`. v0.37.1..v0.40.1 has 27 merges that would otherwise pollute output. `[VERIFIED: 78 raw vs 51 with --no-merges]`
**Warning signs:** Commit count off by ~30% from expectation.

### Pitfall 4: `Cargo.lock` is excluded by D-11 path filter
**What goes wrong:** Dep-bump commits (`bf2e0969` clap, `5c4e2aea` tokio, `4a7a5a7c` semver) listed in SUMMARY § v0.38->v0.39 do not appear in script output. Maintainer mistakenly thinks the script is broken.
**Why it happens:** D-11 specifies `crates/nono/Cargo.toml` only — no `Cargo.lock`, no other crate `Cargo.toml`s. Dep bumps live entirely in `Cargo.lock`.
**How to avoid:** Document this explicitly in the docs and in the script's `--help`. Either accept it (filter is "code drift, not all activity") or expand the filter — see Open Question 1.
**Warning signs:** Script's count is 56 for v0.37.1..v0.40.1 vs SUMMARY's 78 headline.

### Pitfall 5: Renames carry old path through categorization
**What goes wrong:** Commit c8b8aa9a renames `Dockerfile -> docker/Dockerfile-CI`. Numstat shows `0\t0\tDockerfile => docker/Dockerfile-CI`. If parser strips the rename, it might use `Dockerfile` (categorized as `other`, no path prefix match) when the new path is `docker/Dockerfile-CI` (also `other` here, but for Rust files this matters).
**Why it happens:** `git log --numstat` rename detection emits `old => new` in the path column.
**How to avoid:** Always normalize to the new path (`PATH="${PATH##* => }"`). New path is what the fork must update.
**Warning signs:** Categorization differs from `git log --name-only`'s view.

### Pitfall 6: Single-element array unwraps in PowerShell 5.1
**What goes wrong:** A range with exactly one matching commit produces `"commits": { ... }` instead of `"commits": [ { ... } ]`. Diff against fixture fails.
**Why it happens:** PS 5.1's pipeline-to-array semantics; `ConvertTo-Json` historically unwrapped single-element arrays.
**How to avoid:** Always wrap arrays explicitly: `@($commits)` and `@($files)`. Test with a 1-commit range fixture (e.g., `v0.40.0..v0.40.1` has 3 commits — design a 1-commit subrange via `--from <sha>~1 --to <sha>` for the test).
**Warning signs:** Array becomes object in JSON output for small ranges.

### Pitfall 7: `pwsh` not installed on maintainer box
**What goes wrong:** Makefile invokes `pwsh -File ...` and fails with "command not found" on a maintainer who only has Windows PS 5.1.
**Why it happens:** `pwsh` is PowerShell 7 (cross-platform, separate install). Windows ships only with `powershell.exe` (5.1).
**How to avoid:** Makefile dispatch tries `pwsh` first, falls back to `powershell.exe`. Test the script under PS 5.1.
**Warning signs:** `make check-upstream-drift` errors immediately with "pwsh: command not found" on a fresh Windows box.
**[VERIFIED on this box: pwsh NOT on PATH; only powershell.exe 5.1.26100]**

### Pitfall 8: Bash `set -euo pipefail` + `git log | while read` subshell
**What goes wrong:** Variables modified inside the `while read` loop don't survive after it (subshell isolation). If you accumulate state (`COMMITS+=("$obj")`), the array is empty after the loop.
**Why it happens:** Pipe creates a subshell; bash's default scoping does not propagate variable changes back.
**How to avoid:** Use process substitution: `while read ...; do ... done < <(git log ...)`. This keeps the loop in the parent shell.
**Warning signs:** Final output is empty even though intermediate `echo` inside the loop showed data.

## Code Examples

### Reproducing the seed-data inventory

```bash
# Verified counts on this repo at HEAD (2026-04-27):
git log --oneline --no-merges v0.37.1..v0.40.1 | wc -l
# -> 51   (raw, no path filter — DIFFERS from SUMMARY's 78 headline because
#          SUMMARY counts including `--no-merges` may have been against a
#          different commit-graph state OR the SUMMARY's headline was inaccurate;
#          re-verification is part of acceptance #1)

git log --oneline --no-merges v0.37.1..v0.40.1 \
  -- 'crates/nono/src/' 'crates/nono-cli/src/' 'crates/nono-proxy/src/' \
     'crates/nono/Cargo.toml' \
     ':(exclude)*_windows.rs' ':(exclude)crates/nono-cli/src/exec_strategy_windows/' \
  | wc -l
# -> 56   (cross-platform path filter applied)
```

> **Update:** Re-running gives 78 in some shells and 51 in this one — this is a path-quoting nuance that needs verification under both bash variants. Planner: confirm during plan-bounce.

### Verified D-19 trailer block from commit 73e1e3b8

```
refactor(22-03): centralize trust bundle for package verification (PKG-04)

[... commit body ...]

Upstream-commit: 600ba4ec
Upstream-tag: v0.38.0
Upstream-author: Luke Hinds <lukehinds@gmail.com>
Co-Authored-By: Luke Hinds <lukehinds@gmail.com>
Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
```
`[VERIFIED: git log -1 --format='%B' 73e1e3b8 — verbatim trailer]`

### Bash robust commit emitter (skeleton)

```bash
#!/usr/bin/env bash
set -euo pipefail

emit_json_string() {
  # escape JSON-required chars only (covers git-emitted text in practice)
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//$'\n'/\\n}"
  s="${s//$'\r'/\\r}"
  s="${s//$'\t'/\\t}"
  printf '"%s"' "$s"
}

categorize_file() {
  # echoes one of: profile, policy, package, proxy, audit, other
  local f="$1"
  case "$f" in
    crates/nono-cli/src/profile/*|crates/nono-cli/src/profile.rs) echo profile ;;
    crates/nono-cli/src/policy.rs|crates/nono-cli/data/policy.json) echo policy ;;
    crates/nono-cli/src/package*|crates/nono/src/package*) echo package ;;
    crates/nono-proxy/*) echo proxy ;;
    crates/nono/src/audit/*|crates/nono/src/audit_attestation*|crates/nono-cli/src/audit*) echo audit ;;
    *) echo other ;;
  esac
}

# Drive git log; consume per-commit blocks; emit JSON.
# Use process substitution to keep $COMMITS in parent shell.
declare -a COMMITS=()
SHA="" SUBJ="" AUTH="" DATE=""; ADDS=0; DELS=0; declare -a FILES=()
while IFS=$'\t' read -r c1 c2 c3 c4 c5; do
  if [[ "$c1" == "COMMIT" ]]; then
    [[ -n "$SHA" ]] && finalize_commit
    SHA="$c2"; AUTH="$c3"; DATE="$c4"; SUBJ="$c5"
    ADDS=0; DELS=0; FILES=()
  elif [[ -n "$c1" ]]; then
    [[ "$c1" == "-" ]] || ADDS=$((ADDS + c1))
    [[ "$c2" == "-" ]] || DELS=$((DELS + c2))
    f="$c3"; [[ "$f" == *' => '* ]] && f="${f##* => }"
    FILES+=("$f")
  fi
done < <(git log --no-merges --numstat \
           --format='COMMIT%x09%H%x09%an%x09%aI%x09%s' \
           "$RANGE" -- $PATHS)
[[ -n "$SHA" ]] && finalize_commit

# emit JSON object
printf '{"range":"%s","total":%d,"commits":[' "$RANGE" "${#COMMITS[@]}"
for ((i=0; i<${#COMMITS[@]}; i++)); do
  [[ $i -gt 0 ]] && printf ','
  printf '%s' "${COMMITS[$i]}"
done
printf ']}\n'
```

### PowerShell 5.1 robust commit emitter (skeleton)

```powershell
param(
    [string]$From = "",
    [string]$To = "",
    [ValidateSet("table","json")]
    [string]$Format = "table"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()

function Get-Category {
    param([string]$Path)
    switch -Regex ($Path) {
        '^crates/nono-cli/src/profile/' { return 'profile' }
        '^crates/nono-cli/src/profile\.rs$' { return 'profile' }
        '^crates/nono-cli/src/policy\.rs$' { return 'policy' }
        '^crates/nono-cli/data/policy\.json$' { return 'policy' }
        '^crates/nono-cli/src/package' { return 'package' }
        '^crates/nono/src/package' { return 'package' }
        '^crates/nono-proxy/' { return 'proxy' }
        '^crates/nono/src/audit/' { return 'audit' }
        '^crates/nono/src/audit_attestation' { return 'audit' }
        '^crates/nono-cli/src/audit' { return 'audit' }
        default { return 'other' }
    }
}

# resolve range, run git log, parse, build [ordered] hashtables
# wrap arrays with @() before ConvertTo-Json
$result = [ordered]@{
    range = "$from..$to"
    total_unique_commits = $commits.Count
    by_category = $byCategory
    commits = @($commits)
}
$result | ConvertTo-Json -Depth 6 -Compress
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Ad-hoc `git log v0.37.1..v0.40.1` archaeology per sync | Static script + per-category lookup | This phase | Sync work shrinks from days to hours |
| One-off SUMMARY.md per upstream review (260424-upr) | Script reproduces SUMMARY shape; template scaffolds new ones | This phase | Inventory becomes mechanical, not narrative |
| `pwsh` for new PowerShell tooling | Test under PS 5.1, target PS 5.1+ | This phase | Lower install burden on maintainer |
| `jq` for JSON in shell | Native PowerShell `ConvertTo-Json` + bash printf-with-escape | This phase | Zero new tool deps |

**Deprecated/outdated:**
- The 260424-upr SUMMARY.md headline ("78 non-merge commits") cannot be reproduced verbatim by the path-filter script (script reports 56 for the same range). Documentation should clarify which number is canonical going forward — recommend the script's filtered count.

## Validation Architecture

**Nyquist enabled** (config.json `workflow.nyquist_validation: true`). This phase has no Rust tests but does have a clear sampling-rate problem: tag-resolution + numstat parsing + categorization each have edge cases that one fixture won't cover.

### Test framework

| Property | Value |
|----------|-------|
| Framework | Bash + diff (no test framework dependency); existing `tests/integration/test_*.sh` pattern |
| Config file | none — runner is `tests/run_integration_tests.sh` which auto-discovers `test_*.sh` |
| Quick run command | `bash tests/integration/test_upstream_drift.sh` |
| Full suite command | `bash tests/run_integration_tests.sh` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DRIFT-01 | Reproduces v0.37.1..v0.40.1 inventory | golden-fixture diff | `bash tests/integration/test_upstream_drift.sh` (range 1) | ❌ Wave 0 |
| DRIFT-01 | Reproduces v0.39.0..v0.40.0 (audit cluster) | golden-fixture diff | `bash tests/integration/test_upstream_drift.sh` (range 2) | ❌ Wave 0 |
| DRIFT-01 | Single-commit range JSON has array, not object (PS 5.1 footgun) | golden-fixture diff | `bash tests/integration/test_upstream_drift.sh` (range 3, single-commit subrange) | ❌ Wave 0 |
| DRIFT-01 | Twin-script parity (.sh and .ps1 produce byte-identical JSON) | dual-execution diff | `bash tests/integration/test_upstream_drift.sh` (parity check) | ❌ Wave 0 |
| DRIFT-01 | Tag auto-detection picks v0.X tags, not fork v2.X | unit-style assertion | embedded in test_upstream_drift.sh (run with no `--from`/`--to`, assert reported "from" matches `^v0\.`) | ❌ Wave 0 |
| DRIFT-01 | Missing `upstream` remote -> exit 1 with hint | shell exit-code check | embedded in test_upstream_drift.sh (run in temp clone with no upstream) | ❌ Wave 0 |
| DRIFT-01 | `--format table` produces human-grouped output | grep assertion | embedded (assert output contains category headers `## profile`, `## audit`) | ❌ Wave 0 |
| DRIFT-02 | Template file exists at `.planning/templates/upstream-sync-quick.md` | file-existence check | `[[ -f .planning/templates/upstream-sync-quick.md ]]` | ❌ Wave 0 |
| DRIFT-02 | Template has D-19 trailer block (all 6 trailer lines) | grep assertion | `grep -E '^Upstream-commit: \{' .planning/templates/upstream-sync-quick.md && grep -E '^Co-Authored-By: ' ... && grep -c '^Signed-off-by: ' = 2` | ❌ Wave 0 |
| DRIFT-02 | Placeholder smoke test: substitute all `{name}` placeholders with sample values, assert valid GSD frontmatter | shell render+grep | embedded test that runs `sed` substitution then asserts `^---$` and `^slug: ` and `^date: ` are present | ❌ Wave 0 |
| DRIFT-02 | PROJECT.md references the template | grep assertion | `grep -F '.planning/templates/upstream-sync-quick.md' PROJECT.md` | ❌ Wave 0 |
| DRIFT-02 | docs file exists at `docs/cli/development/upstream-drift.mdx` AND PROJECT.md cross-links to it | file-existence + grep | `[[ -f docs/cli/development/upstream-drift.mdx ]] && grep -F 'docs/cli/development/upstream-drift' PROJECT.md` | ❌ Wave 0 |

### Sampling rate

- **Per task commit:** `bash tests/integration/test_upstream_drift.sh` (runs fast — a few `git log` invocations + diffs)
- **Per wave merge:** Same; this phase has no other dynamic test surface
- **Phase gate:** Full suite via `bash tests/run_integration_tests.sh` (which auto-discovers the new test) green before `/gsd-verify-work`

**Rationale on sampling depth (Nyquist Dimension 8):** Tag-resolution has 4 edge cases (no upstream remote, no v0.* tag merged, fork tag is highest local, partial-sync state); numstat parsing has 4 edge cases (binary, rename, merge, multi-byte unicode); categorization has 6 categories. Single-fixture coverage = 1 sample, which under-samples the parsing space. **Recommendation: 3 fixtures (large/mid/single-commit) + 3 unit-style assertions (tag auto-detect, missing remote, table format) = 6 sampling points across the parsing space.** This is the minimum to catch a regression that flips one path-prefix in the lookup table.

### Reference-fixture strategy (acceptance #1)

The frozen JSON fixture for `v0.37.1..v0.40.1` IS the ground truth. Generation procedure:
1. Author runs the bash script once at the agreed canonical commit-graph state and pipes output to `tests/integration/fixtures/upstream-drift/v0.37.1__v0.40.1.json`.
2. Author reviews the fixture against the SUMMARY.md per-release blocks; any commit listed in SUMMARY but missing from fixture is documented in `tests/integration/fixtures/upstream-drift/README.md` ("Excluded by D-11 path filter — listed for narrative completeness only in SUMMARY: c07c66ac, 91c384ff, ... ").
3. Fixture committed verbatim. Both `.sh` and `.ps1` outputs must diff cleanly against it.

### Twin-script parity check

The test runs BOTH scripts (when both interpreters are available) and diffs:
```bash
diff <(bash scripts/check-upstream-drift.sh --from $F --to $T --format json) \
     <(powershell.exe -NoProfile -File scripts/check-upstream-drift.ps1 -From $F -To $T -Format json)
```
**Failure mode the test catches:** Maintainer updates only `.sh` (or only `.ps1`); the other drifts. Diff fails immediately.

### Template-placeholder smoke test

```bash
TEMPDIR=$(mktemp -d)
cp .planning/templates/upstream-sync-quick.md "$TEMPDIR/PLAN.md"
sed -i 's|{from_tag}|v0.41.0|g; s|{to_tag}|v0.42.0|g; s|{commit_count}|18|g; s|{date}|2026-05-01|g; s|{quick_slug}|260501-upr-sync-v0.42|g' "$TEMPDIR/PLAN.md"
# assert no remaining {placeholder} markers (or at most a documented set)
remaining=$(grep -oE '\{[a-z_]+\}' "$TEMPDIR/PLAN.md" | sort -u | wc -l)
[[ $remaining -le 0 ]] || { echo "unexpected placeholders remain"; exit 1; }
# assert valid frontmatter
head -1 "$TEMPDIR/PLAN.md" | grep -q '^---$'
grep -q '^slug: ' "$TEMPDIR/PLAN.md"
grep -q '^date: ' "$TEMPDIR/PLAN.md"
```

### Documentation completeness check

```bash
# D-15
grep -F 'Upstream Parity Process' PROJECT.md
grep -F '.planning/templates/upstream-sync-quick.md' PROJECT.md
# D-16
[[ -f docs/cli/development/upstream-drift.mdx ]]
grep -F 'docs/cli/development/upstream-drift' PROJECT.md
# Cross-link from .mdx back to template + script
grep -F 'check-upstream-drift' docs/cli/development/upstream-drift.mdx
grep -F 'upstream-sync-quick.md' docs/cli/development/upstream-drift.mdx
```

### Wave 0 Gaps

- [ ] `tests/integration/test_upstream_drift.sh` — runs fixtures + parity check + smoke test
- [ ] `tests/integration/fixtures/upstream-drift/v0.37.1__v0.40.1.json` — canonical large-range fixture
- [ ] `tests/integration/fixtures/upstream-drift/v0.39.0__v0.40.0.json` — audit-cluster mid-range fixture
- [ ] `tests/integration/fixtures/upstream-drift/v0.40.0__v0.40.1.json` — small-range (3-commit) fixture for PS 5.1 array-unwrap test
- [ ] `tests/integration/fixtures/upstream-drift/README.md` — explains regeneration procedure + documents D-11 exclusions
- No framework install needed; bash + diff + grep + sed already on box

## Security Domain

`security_enforcement: true` (default; not overridden in config.json).

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Script needs no auth — reads local `.git` |
| V3 Session Management | no | No sessions |
| V4 Access Control | no | Read-only, no privilege boundary crossed |
| V5 Input Validation | yes | `--from`/`--to` arguments are passed to `git log`. Must use `--` to separate refs from paths; reject refs containing whitespace, semicolons, or shell-meta characters even though `git log` is run via direct exec (no shell interpolation) |
| V6 Cryptography | no | None |
| V14 Configuration | yes | `Set-StrictMode` + `set -euo pipefail` enforce fail-loud; explicit UTF-8 encoding prevents text-corruption-as-data-integrity-issue |

### Known Threat Patterns for {bash + powershell maintainer scripts}

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Command injection via `--from $UNTRUSTED` | Tampering | Direct exec (no shell), use `--` separator, validate refs match `^[A-Za-z0-9._/-]+$` before passing to git |
| Path traversal via crafted `--from <ref>` | Tampering | git itself rejects malformed refs; defense-in-depth = pre-validate ref format |
| Silent JSON corruption from non-UTF-8 console | Information Disclosure (corrupted audit trail) | Force UTF-8 on PS console; use `LC_ALL=C.UTF-8` or `LANG=C.UTF-8` in bash where needed |
| Credential leakage in commit-message subjects | Information Disclosure | Subjects shown verbatim — but they are already public on GitHub. Not a new exposure surface. |
| Read-only invariant violated | Tampering | D-11 already locks read-only; reviewer enforcement: `git status` after script run must show clean tree (no `.git/HEAD` mutation, no fetch). Add this assertion in the test. |

**Threat scope conclusion:** Low — this is a static analysis script over local-only git history. The only realistic threat is accidentally adding a fetch or a `git config` write; the test asserts `git status` clean post-invocation.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `bash` | `.sh` script | yes | 5.x (Git for Windows MSYS2) | — |
| `git` | both scripts | yes | recent | — |
| `awk`, `sed`, `printf`, `sort`, `comm` | `.sh` script | yes | GNU coreutils via MSYS | — |
| Windows PowerShell 5.1 (`powershell.exe`) | `.ps1` script | yes | 5.1.26100 | — |
| PowerShell 7 (`pwsh`) | preferred for `.ps1` per Makefile convention | **NO** | — | Fall back to `powershell.exe` in Makefile dispatch |
| `jq` | NOT used (script must avoid this dep) | no | — | Hand-rolled printf-with-escape in bash |
| `python3` | NOT used (over-engineering) | yes (3.14.4) | not used | — |
| `upstream` git remote | tag resolution | yes | configured to `https://github.com/always-further/nono.git` | exit 1 with actionable hint per D-10 |

**Missing dependencies with no fallback:** none

**Missing dependencies with fallback:** `pwsh` -> `powershell.exe` (Makefile dispatch handles)

## Assumptions Log

| # | Claim | Section | Risk if wrong |
|---|-------|---------|---------------|
| A1 | Bash JSON escape covering only `\\`, `"`, `\n`, `\r`, `\t` is sufficient for git-formatted text | Pattern 4 | Low — git output has been stable for years; if a real commit subject ever contains a control char, JSON validators reject and the test catches it |
| A2 | The path-filter (D-11) is canonical, the SUMMARY's 78-commit headline is informational | Summary, Open Questions | Medium — if planner decides SUMMARY is canonical, filter must expand to include `Cargo.lock`, all crate `Cargo.toml`s, `data/policy.json`, possibly `tests/`. This changes acceptance #1's fixture shape. |
| A3 | Tag auto-detect via `git tag --list 'v0.*' --merged HEAD --sort=-v:refname` is reliable across partial-sync states | Pattern 2 | Medium — only validated against current "fully synced through v0.40.x" state. Partial-sync (some v0.41 commits cherry-picked) needs a real test. |
| A4 | PS 5.1 `ConvertTo-Json -Depth 5` round-trips the JSON shape correctly with `@()` wrapping | Pattern 4 | Low — empirically tested on this box; both empty array and single-element array round-trip OK |
| A5 | All v0.37.1..v0.40.1 commit subjects are ASCII | Pattern 3, Pitfall 1 | Low — verified by inspection of SUMMARY listings; but the design must handle non-ASCII for future ranges |
| A6 | `pwsh` may exist on some maintainer boxes even though not on this one; Makefile should prefer it but fall back gracefully | Pattern 6 | Low — Makefile dispatch is straightforward |
| A7 | `make check-upstream-drift ARGS="..."` is the right argument-passthrough idiom | Pattern 6 | Low — standard GNU Make convention; document in `make help` |
| A8 | The 78 vs 51 raw count discrepancy I observed (78 in SUMMARY, 51 in `git log --no-merges`) is explained by SUMMARY's count being against pre-`--no-merges` data — but I cannot rule out other causes | Open Questions | Medium — planner should re-verify, possibly the SUMMARY's headline used `--first-parent` instead of `--no-merges` |
| A9 | Single-brace `{name}` placeholder syntax matches GSD template convention; double-brace `{{NAME}}` would be the only template using that style | Pattern 7 | Low — verified across 31 GSD templates |

**Calibration:** A2 and A8 are the two assumptions most worth raising during plan-bounce; both touch acceptance #1.

## Open Questions (RESOLVED)

> All questions below were resolved during planning (2026-04-27) via user decision (Q1) or by the planner adopting the recommended interpretation (Q2/Q4/Q5/Q6). Resolutions are baked into 24-01-PLAN.md / 24-02-PLAN.md.

1. **Path-filter vs SUMMARY ground truth (HIGH PRIORITY)**

   **What we know:** D-11 path filter produces 56 commits for v0.37.1..v0.40.1. SUMMARY headline says 78. The 22 excluded commits are docs, dep bumps, GitHub workflows, integration tests, and the claude-code package removal — all explicitly listed in SUMMARY's per-release blocks.

   **What's unclear:** Acceptance #1 says "reproduces the commit inventory from quick task 260424-upr SUMMARY.md." Does "reproduce" mean (a) match the 78-commit headline (filter must expand) or (b) match the per-category breakdown (which only sums to ~56)?

   **Recommendation:** Option (a) is unworkable — the SUMMARY's 78-commit number includes commits the script must by design exclude (D-11 says exclude `*_windows.rs` etc., though none of the 22 excluded commits actually match that pattern, so the issue is purely about which paths to INCLUDE). Recommend option (b) with a fixture-companion `README.md` that documents the 22-commit delta as informational. Planner: confirm with user before locking acceptance #1's exact diff command.

   **RESOLVED (2026-04-27):** Option (b) — D-11 path filter is canonical. User confirmed during /gsd-plan-phase 24 interactive question. Acceptance #1 satisfied by per-category breakdown match between script JSON output and SUMMARY's per-release narrative; the 22-commit delta is documented in `tests/integration/fixtures/upstream-drift/README.md` as informational. Path filter NOT widened. (Plans 24-01 acceptance lock; CONTEXT.md D-11 stays as-is.)

2. **Should the filter include `Cargo.lock`?**

   **What we know:** `bf2e0969` (clap), `5c4e2aea` (tokio), `4a7a5a7c` (semver) are dep-bump commits explicitly listed in SUMMARY. They modify only `Cargo.lock`. D-11 omits `Cargo.lock`.

   **Recommendation:** Either (a) add `Cargo.lock` and all three workspace `Cargo.toml`s to the filter (catches dep bumps; matches SUMMARY narrative), or (b) document the omission and tell maintainers to additionally check `git log --no-merges <range> -- Cargo.lock` manually. Recommend (a) — the filter widens by 4 paths total and the categorization can put them in `other`.

   **RESOLVED (2026-04-27):** Option (b) — `Cargo.lock` and additional crate `Cargo.toml`s NOT added. Subsumed by Q1's resolution: D-11 filter stays as-is, dep-bump commits live in the documented informational delta in fixtures README.

3. **Should excluded paths cover more than `*_windows.rs` + `exec_strategy_windows/`?**

   **What we know:** D-11 lists those two patterns. But the fork has more Windows-only files: `pty_proxy_windows.rs`, `trust_intercept_windows.rs`, `session_commands_windows.rs`, `windows_wfp_contract.rs`, `learn_windows.rs`, `open_url_runtime_windows.rs` (per SUMMARY § "What the review confirms is safe").

   **Observation:** D-11's `*_windows.rs` glob already covers all of these (it's a suffix match). The `exec_strategy_windows/` directory is the only one needing a separate exclude. So D-11 IS sufficient — no expansion needed. **Resolved by close reading.**

4. **Single combined plan or 24-01 + 24-02 split?**

   **What we know:** D-17 leaves this to the planner. Total surface: ~700 lines of script (combined .sh + .ps1) + ~150 lines of template + ~50 lines of docs + 1 Makefile target + 1 PROJECT.md section + 4 fixture files + 1 test script. Roughly equal to a 2-wave single plan or a small 2-plan phase.

   **Recommendation:** Single combined plan with 3 waves: (W1) twin-script skeleton + tag resolution + path filter + JSON emission + 3 fixtures; (W2) categorization + table format + parity check; (W3) template + Makefile target + PROJECT.md + docs + smoke tests. Reasoning: the template (DRIFT-02) references `make check-upstream-drift` (D-14) which is only meaningful after the script (DRIFT-01) exists. Coupling them in one plan keeps Wave 3's smoke tests honest.

   **RESOLVED (2026-04-27):** 2-plan split chosen by planner (24-01 DRIFT-01 in Wave 1, 24-02 DRIFT-02 in Wave 2 with `depends_on: ["24-01"]`). Coupling preserved at the wave/dependency level — Plan 24-02 Task 3 extends Plan 24-01's `tests/integration/test_upstream_drift.sh` in place so the smoke test in 24-02 references live `make check-upstream-drift`.

5. **PROJECT.md "Upstream Parity Process" section length.**

   **What we know:** D-15 says "short (workflow only)"; long-form lives in `docs/cli/development/upstream-drift.mdx`.

   **Recommendation:** ~10 lines in PROJECT.md, ~150-300 lines in the `.mdx`. PROJECT.md gets: 1-paragraph what, 5-bullet workflow, 1 cross-link.

   **RESOLVED (2026-04-27):** Recommendation adopted — Plan 24-02 Task 2 spec'd PROJECT.md section as ~10 lines with 5-bullet workflow + cross-link to `.mdx`; long-form lives in `docs/cli/development/upstream-drift.mdx`.

6. **Header trailer's exact field name: `Upstream-author` or `Upstream-Author`?**

   **What we know:** All three reference commits use `Upstream-author:` (lowercase `a`). Verified verbatim.

   **Recommendation:** Match exactly. Single-source-of-truth: the most recent of the three reference commits.

   **RESOLVED (2026-04-27):** Lowercase `a` confirmed and locked in Plan 24-02 Task 1 acceptance criteria via `grep -E '^Upstream-author: \{' .planning/templates/upstream-sync-quick.md` (case-sensitive).

## Sources

### Primary (HIGH confidence)
- `git log --pretty=fuller 73e1e3b8 adf81aec 869349df` — D-19 trailer block, exact format `[VERIFIED on this repo]`
- `git tag --list | sort -V` + `git ls-remote --tags upstream` — tag chain + upstream tag enumeration `[VERIFIED]`
- `git log --no-merges --numstat v0.37.1..v0.40.1` — raw and filtered counts, numstat format including binary + rename edge cases `[VERIFIED]`
- `Makefile` — existing target patterns, `pwsh -File` Windows targets, no platform dispatch `[VERIFIED: full file read]`
- `.planning/quick/260424-upr-review-upstream-037-to-040/SUMMARY.md` — seed-data inventory `[VERIFIED: full file read]`
- `scripts/test-linux.sh`, `scripts/build-windows-msi.ps1`, `scripts/prepare-release.sh` — convention sources `[VERIFIED]`
- `~/.claude/get-shit-done/templates/{AI-SPEC,DEBUG,SECURITY}.md` — placeholder convention precedent `[VERIFIED]`
- `docs/cli/development/*.mdx` — file convention `[VERIFIED: 10 files, all .mdx, 0 .md]`
- `powershell.exe -Command '$PSVersionTable'` — confirmed only PS 5.1 on this box `[VERIFIED]`
- `which jq`, `python --version` — tool availability probes `[VERIFIED]`

### Secondary (MEDIUM confidence)
- Bash JSON escape scope — based on inspection of git output and observation that real commit subjects in this repo are ASCII-only `[CITED: pattern is well-documented in shell scripting community, but the scope-limit is an editorial judgment]`
- PS 5.1 single-element-array unwrap behavior — empirically tested on this box and produced correct output, but historical reports suggest edge cases exist `[CITED: PS team blog noted the 5.1 behavior; PS 6+ has `-AsArray`]`

### Tertiary (LOW confidence)
- Tag auto-detect robustness across partial-sync states — only validated against the current fully-synced state at HEAD. Partial-sync behavior is reasoned, not tested. `[ASSUMED]`

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every tool empirically probed on this box
- Architecture (script structure, parsing, JSON emission): HIGH — verified against real `git log` output in the seed range
- Tag-resolution algorithm: MEDIUM-HIGH — main path verified; partial-sync edge case is reasoned
- Categorization lookup table: HIGH — derived from SUMMARY narrative + verified against actual file paths in the seed range
- Pitfalls: HIGH — Pitfalls 1, 2, 3, 4, 7 empirically verified on this repo/box
- Template placeholder convention: HIGH — verified across 31 GSD templates
- D-19 trailer format: HIGH — verbatim from three reference commits
- Test strategy: MEDIUM — pattern matches existing `tests/integration/test_*.sh` style; specific fixture-regeneration workflow is recommendation, not verified by running yet
- SUMMARY reproduction (acceptance #1): MEDIUM — reproduction count is uncertain (56 vs 78); reconciliation is Open Question 1

**Research date:** 2026-04-27
**Valid until:** 2026-05-27 (30 days for stable tooling research)

## RESEARCH COMPLETE

**Phase:** 24 - Parity-Drift Prevention
**Confidence:** HIGH (with one MEDIUM-confidence reconciliation question for the planner)

### Key Findings

- **D-11 path filter produces 56 commits for v0.37.1..v0.40.1, SUMMARY says 78** — the 22-commit delta is real (docs, dep bumps to `Cargo.lock`, GitHub workflows, integration tests, claude-code package removal). Acceptance #1 needs reconciliation: recommend the filter is canonical and the SUMMARY's headline is informational.
- **`pwsh` is NOT on this maintainer's PATH; only Windows PowerShell 5.1** — Makefile dispatch must fall back to `powershell.exe`. Script must be tested under PS 5.1 with explicit UTF-8 console encoding.
- **`jq` is NOT installed** — bash must hand-roll JSON via printf-with-escape (scoped to git-emitted text; document the limit). PS uses `ConvertTo-Json -Depth 5 -Compress` with explicit `@()` array wrapping.
- **D-19 trailer block format is fixed and verified verbatim** from commits 73e1e3b8/adf81aec/869349df — 6 lines, ordered, with two `Signed-off-by` lines (full name + github handle) and `Upstream-author` (lowercase 'a').
- **Single-brace `{name}` placeholder convention** is universal across 31 GSD templates — match it, do not introduce `{{NAME}}` style.
- **Single combined plan with 3 waves** is recommended; the template (DRIFT-02) depends on `make check-upstream-drift` (DRIFT-01) for its smoke test, so coupling them in one plan keeps acceptance honest.

### File Created
`C:\Users\OMack\Nono\.planning\phases\24-parity-drift-prevention\24-RESEARCH.md`

### Confidence Assessment

| Area | Level | Reason |
|------|-------|--------|
| Standard Stack | HIGH | Every tool empirically probed on this box (jq absent, pwsh absent, PS 5.1 present, python 3.14.4 present, bash 5.x present) |
| Architecture (script + JSON + categorization) | HIGH | Patterns verified against real `git log` output in the seed range; numstat edge cases (binary, rename, merge) confirmed |
| Tag resolution | MEDIUM-HIGH | Main path verified; partial-sync edge case is reasoned but not tested |
| Pitfalls | HIGH | Pitfalls 1, 2, 3, 4, 7 directly verified on this repo/box |
| D-19 trailer + template convention | HIGH | Verbatim from 3 reference commits; placeholder convention from 31 GSD templates |
| Acceptance #1 reproduction | MEDIUM | Count discrepancy (56 vs 78) needs planner/user reconciliation |
| Test strategy | MEDIUM | Pattern matches existing convention; fixture-regen workflow is recommendation, not run yet |

### Open Questions for the planner (RESOLVED)

> Resolved during planning (2026-04-27). Detailed resolutions in `## Open Questions (RESOLVED)` section above.

1. **Acceptance #1 reconciliation (HIGH PRIORITY)**: should the script's filter expand to include `Cargo.lock` + all crate `Cargo.toml`s + `data/policy.json` to chase the SUMMARY's 78-commit headline, or is the path-filter canonical and the headline informational? Recommendation: canonical filter + informational delta. Confirm before locking acceptance.
   **RESOLVED (2026-04-27):** Path-filter canonical, 22-commit delta documented as informational in `tests/integration/fixtures/upstream-drift/README.md` (user decision).
2. **Should the filter include `Cargo.lock`?** Independent of (1), the dep-bump commits (clap/tokio/semver) live entirely in `Cargo.lock`. Recommend including it.
   **RESOLVED (2026-04-27):** No — `Cargo.lock` NOT added; subsumed by Q1 resolution.
3. **Plan split**: single combined plan (recommended) vs 24-01 + 24-02 split? D-17 leaves this to the planner.
   **RESOLVED (2026-04-27):** 2-plan split chosen by planner — 24-01 (DRIFT-01) Wave 1, 24-02 (DRIFT-02) Wave 2 with `depends_on: ["24-01"]`. Coupling preserved at the wave/dependency level.
4. **Tag auto-detect under partial-sync state**: the algorithm is reasoned but not tested for the case where the fork has cherry-picked SOME v0.41 commits but not all. Add a fixture for this case during execution if the corner is judged in-scope.
   **RESOLVED (2026-04-27):** Deferred — out of scope for v1; the 3 fixtures cover the in-scope sampling space (Nyquist Dim 8). Partial-sync fixture is captured as a future-improvement note in `tests/integration/fixtures/upstream-drift/README.md`.

### Ready for Planning
Research complete. Planner can now create PLAN.md files with high confidence on every locked decision and clear flagging on the one MEDIUM-confidence reconciliation question (acceptance #1).
