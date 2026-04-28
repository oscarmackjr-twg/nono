# scripts/check-upstream-drift.ps1
# Reports upstream commits the fork has not absorbed, grouped by file category.
# Read-only - does NOT modify git state.
#
# Usage:
#   pwsh -File scripts/check-upstream-drift.ps1                              # auto-detect
#   pwsh -File scripts/check-upstream-drift.ps1 -From v0.37.1 -To v0.40.1
#   pwsh -File scripts/check-upstream-drift.ps1 -Format json                 # default: table
#
# Path filter (D-11): cross-platform Rust code under crates/{nono,nono-cli,nono-proxy}/src/
# plus crates/nono/Cargo.toml. Excludes *_windows.rs and crates/nono-cli/src/exec_strategy_windows/.
# Dep bumps in Cargo.lock and other crate Cargo.toml files are NOT reported.
# 22-commit informational delta vs 260424-upr SUMMARY headline is documented in
# tests/integration/fixtures/upstream-drift/README.md.

param(
    [string]$From = "",
    [string]$To = "",
    [ValidateSet("table","json")]
    [string]$Format = "table"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# T-24-03 mitigation: PS 5.1 console defaults to OEM codepage; force UTF-8 so
# non-ASCII commit subjects do not mojibake and produce invalid UTF-8 in JSON
# output (which would break fixture diff).
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$OutputEncoding = [System.Text.UTF8Encoding]::new()

# ---------------------------------------------------------------------------
# Ref input validation (T-24-01 - V5 BLOCKING-eligible)
# ---------------------------------------------------------------------------
function Test-RefSyntax {
    param([string]$Ref)
    if ($Ref -ne "" -and $Ref -notmatch '^[A-Za-z0-9._/-]+$') {
        Write-Error "Invalid ref '$Ref' (must match [A-Za-z0-9._/-]+)"
        exit 2
    }
}
Test-RefSyntax $From
Test-RefSyntax $To

# ---------------------------------------------------------------------------
# Tag auto-detection (D-08, D-10; T-24-02 read-only invariant)
# ---------------------------------------------------------------------------
# Verify upstream remote exists. Fail-closed per D-10: never auto-add.
$null = git remote get-url upstream 2>$null
if ($LASTEXITCODE -ne 0) {
    [Console]::Error.WriteLine("Error: 'upstream' remote not configured.")
    [Console]::Error.WriteLine("Add it with:")
    [Console]::Error.WriteLine("  git remote add upstream https://github.com/always-further/nono.git")
    exit 1
}

if ([string]::IsNullOrEmpty($From)) {
    $From = (git tag --list 'v0.*' --merged HEAD --sort=-v:refname | Select-Object -First 1)
    if ([string]::IsNullOrEmpty($From)) {
        Write-Error "No upstream-style tag (v0.*) reachable from HEAD; cannot auto-detect last-synced point. Use -From <ref>."
        exit 1
    }
}

if ([string]::IsNullOrEmpty($To)) {
    $To = (git describe --tags --abbrev=0 upstream/main 2>$null)
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrEmpty($To)) {
        Write-Error "Cannot resolve latest upstream tag. Is upstream/main fetched? Use -To <ref>."
        exit 1
    }
}

# ---------------------------------------------------------------------------
# Path filter (D-11)
# ---------------------------------------------------------------------------
$gitLogPaths = @(
    'crates/nono/src/',
    'crates/nono-cli/src/',
    'crates/nono-proxy/src/',
    'crates/nono/Cargo.toml',
    ':(exclude)*_windows.rs',
    ':(exclude)crates/nono-cli/src/exec_strategy_windows/'
)

# ---------------------------------------------------------------------------
# Drive git log; consume per-commit blocks; build per-commit hashtables.
# Wave 1 emits per-commit objects without `categories`; Wave 2 (Task 2) adds
# `by_category` + per-commit `categories: [...]`.
# ---------------------------------------------------------------------------
$range = "${From}..${To}"
$gitArgs = @(
    'log',
    '--no-merges',
    '--numstat',
    '--format=COMMIT%x09%H%x09%an%x09%aI%x09%s',
    $range,
    '--'
) + $gitLogPaths

$gitOutput = & git @gitArgs
if ($LASTEXITCODE -ne 0) {
    Write-Error "git log failed (exit $LASTEXITCODE) for range $range"
    exit 1
}

$commits = New-Object System.Collections.ArrayList
$current = $null

function Add-CurrentCommit {
    if ($null -ne $script:current) {
        # Wave 1 commit shape: sha, subject, author, date, additions, deletions, files_changed.
        # Wave 2 (Task 2) appends `categories`.
        $commitObj = [ordered]@{
            sha           = $script:current.sha
            subject       = $script:current.subject
            author        = $script:current.author
            date          = $script:current.date
            additions     = [int]$script:current.additions
            deletions     = [int]$script:current.deletions
            files_changed = @($script:current.files)
        }
        [void]$script:commits.Add($commitObj)
        $script:current = $null
    }
}

foreach ($line in $gitOutput) {
    if ($null -eq $line) { continue }
    if ($line -eq "") {
        # blank line between commits
        continue
    }
    $parts = $line -split "`t"
    if ($parts[0] -eq "COMMIT") {
        Add-CurrentCommit
        $script:current = @{
            sha       = $parts[1]
            author    = $parts[2]
            date      = $parts[3]
            subject   = $parts[4]
            additions = 0
            deletions = 0
            files     = New-Object System.Collections.ArrayList
        }
    } elseif ($parts.Count -ge 3) {
        # numstat row: parts[0]=adds, parts[1]=dels, parts[2]=filename (or "old => new")
        $addsRaw = $parts[0]
        $delsRaw = $parts[1]
        $f = $parts[2]
        if ($addsRaw -ne "-") {
            $script:current.additions += [int]$addsRaw
        }
        if ($delsRaw -ne "-") {
            $script:current.deletions += [int]$delsRaw
        }
        if ($f -match ' => ') {
            $f = ($f -split ' => ')[-1]
        }
        [void]$script:current.files.Add($f)
    }
}
Add-CurrentCommit

$total = $commits.Count

# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------

function Emit-Json {
    # Wave 1 outer shape: range, from, to, total_unique_commits, commits.
    # Wave 2 (Task 2) inserts `by_category` between total_unique_commits and commits.
    $result = [ordered]@{
        range                = "${From}..${To}"
        from                 = $From
        to                   = $To
        total_unique_commits = [int]$total
        commits              = @($commits)
    }
    # -Depth 6 (NOT default 2!) so nested arrays don't serialize as
    # "System.Object[]". -Compress matches bash printf no-pretty-print.
    # Use [Console]::Out.Write + explicit LF to match bash's printf output
    # byte-for-byte (PS Write-Output appends CRLF on Windows).
    $json = ($result | ConvertTo-Json -Depth 6 -Compress)
    [Console]::Out.Write($json + "`n")
}

function Emit-Table {
    # Use [Console]::Out.Write with explicit LF to match bash printf
    # byte-for-byte (PS Write-Output appends CRLF on Windows).
    [Console]::Out.Write("Upstream drift: ${From}..${To}`n")
    [Console]::Out.Write(("Total: {0} unique commits`n" -f $total))
    foreach ($c in $commits) {
        $shaShort = ($c.sha).Substring(0, 8)
        [Console]::Out.Write(("  {0}  {1}`n" -f $shaShort, $c.subject))
    }
}

switch ($Format) {
    'json'  { Emit-Json }
    'table' { Emit-Table }
}
