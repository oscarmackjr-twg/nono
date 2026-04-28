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
# Categorization lookup table (D-05; ORDER IS LOAD-BEARING)
# ---------------------------------------------------------------------------
# Same prefix order as the bash twin's case-statement order. First-match-wins.
# Audit must match before any generic crates/nono/src/* fallback. No subject-
# line keyword scanning per D-05.
function Get-Category {
    param([string]$Path)
    switch -Regex ($Path) {
        '^crates/nono-cli/src/profile/'                      { return 'profile' }
        '^crates/nono-cli/src/profile\.rs$'                  { return 'profile' }
        '^crates/nono-cli/data/profile-authoring-guide\.md$' { return 'profile' }
        '^crates/nono-cli/src/policy\.rs$'                   { return 'policy' }
        '^crates/nono-cli/data/policy\.json$'                { return 'policy' }
        '^crates/nono-cli/src/package'                       { return 'package' }
        '^crates/nono-cli/src/package_cmd\.rs$'              { return 'package' }
        '^crates/nono/src/package'                           { return 'package' }
        '^crates/nono-proxy/'                                { return 'proxy' }
        '^crates/nono/src/audit/'                            { return 'audit' }
        '^crates/nono/src/audit_attestation'                 { return 'audit' }
        '^crates/nono-cli/src/audit'                         { return 'audit' }
        default                                              { return 'other' }
    }
}

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

# by_category aggregate. Multi-category commits double-count (D-06; the
# total_unique_commits header line disambiguates).
$byCategory = [ordered]@{
    profile = 0
    policy  = 0
    package = 0
    proxy   = 0
    audit   = 0
    other   = 0
}

function Add-CurrentCommit {
    if ($null -ne $script:current) {
        # Compute categories: deduplicated, fixed-order iteration over the 6
        # known categories so JSON output is deterministic across the twin.
        $catSet = @{}
        foreach ($f in $script:current.files) {
            $cat = Get-Category $f
            $catSet[$cat] = $true
        }
        # Same fixed-order emission as bash for byte-parity (audit-first lex).
        $cats = New-Object System.Collections.ArrayList
        foreach ($c in @('audit','other','package','policy','profile','proxy')) {
            if ($catSet.ContainsKey($c)) { [void]$cats.Add($c) }
        }
        # Update by_category aggregate.
        foreach ($c in $cats) {
            $script:byCategory[$c]++
        }
        # Wave 2 commit shape: sha, subject, author, date, additions, deletions,
        # files_changed, categories. @() wrapping prevents PS 5.1 single-element
        # unwrap (Pitfall 6).
        $commitObj = [ordered]@{
            sha           = $script:current.sha
            subject       = $script:current.subject
            author        = $script:current.author
            date          = $script:current.date
            additions     = [int]$script:current.additions
            deletions     = [int]$script:current.deletions
            files_changed = @($script:current.files)
            categories    = @($cats)
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
    # Outer key order locked: range, from, to, total_unique_commits,
    # by_category, commits. by_category key order is the SUMMARY.md narrative
    # order: profile, policy, package, proxy, audit, other (locked by the
    # [ordered]@{} hashtable above).
    $result = [ordered]@{
        range                = "${From}..${To}"
        from                 = $From
        to                   = $To
        total_unique_commits = [int]$total
        by_category          = $byCategory
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
    # Header + per-category grouped output (D-06). The SAME commit appears
    # under EACH matching category. Use [Console]::Out.Write with explicit LF
    # to match bash printf byte-for-byte.
    [Console]::Out.Write("Upstream drift: ${From}..${To}`n")
    [Console]::Out.Write(("Total: {0} unique commits`n" -f $total))
    # Fixed category section order matches SUMMARY.md narrative order.
    foreach ($cat in @('profile','policy','package','proxy','audit','other')) {
        $count = $byCategory[$cat]
        if ($count -eq 0) { continue }
        [Console]::Out.Write(("`n## {0} ({1} commits)`n" -f $cat, $count))
        foreach ($c in $commits) {
            if ($c.categories -contains $cat) {
                $shaShort = ($c.sha).Substring(0, 8)
                [Console]::Out.Write(("  {0}  {1}`n" -f $shaShort, $c.subject))
            }
        }
    }
}

switch ($Format) {
    'json'  { Emit-Json }
    'table' { Emit-Table }
}
