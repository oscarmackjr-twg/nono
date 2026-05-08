#requires -Version 5.1

<#
.SYNOPSIS
Phase 30 Wave 1 manual TUI-rendering field-smoke runbook for nono shell.

.DESCRIPTION
Sequences the steps for a human operator to verify Claude Code's full TUI
renders inside `nono shell --profile claude-code --allow-cwd`. Closes
acceptance #2 (D-05: TUI rendering required).

This is a MANUAL runbook with PASS/FAIL prompts at each step. It cannot be
fully automated because TUI rendering quality (alternate screen, cursor
positioning, raw-mode input) is subjective and depends on terminal emulator
behavior the harness cannot inspect.

The script:
  1. Prints the checklist.
  2. Asks the operator to run each step manually.
  3. Captures the operator's PASS/FAIL decision per step to a log.
  4. Exits 0 if all PASS / 1 if any FAIL.

Per RESEARCH Pitfall 2 (Microsoft-documented ConPTY+IL-mismatch failure mode),
the operator MUST exercise interactive typing -- silent input drop / broken
echo with a clean launch is one of the documented failure modes.

.PARAMETER NonoBinary
Path to the nono.exe binary to test.

.PARAMETER LogDir
Log directory.

.PARAMETER IncludeCmd
Also test --shell C:\Windows\System32\cmd.exe per RESEARCH Open Question 3.

.EXAMPLE
pwsh -File scripts/test-windows-shell-tui.ps1
#>

param(
    [string]$NonoBinary = "$PSScriptRoot\..\target\x86_64-pc-windows-msvc\release\nono.exe",
    [string]$LogDir = "ci-logs-local",
    [switch]$IncludeCmd
)

$ErrorActionPreference = 'Continue'
$PSNativeCommandUseErrorActionPreference = $false

New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
$logFile = Join-Path $LogDir "test-windows-shell-tui-$(Get-Date -Format 'yyyyMMdd-HHmmss').log"

function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $stamp = Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ'
    $line = "[$stamp] [$Level] $Message"
    Write-Host $line
    Add-Content -Path $logFile -Value $line
}

function Read-PassFail {
    param([string]$Prompt)
    while ($true) {
        $r = Read-Host "$Prompt [PASS/FAIL/SKIP]"
        switch -Regex ($r) {
            '^p|pass$'  { return 'PASS' }
            '^f|fail$'  { return 'FAIL' }
            '^s|skip$'  { return 'SKIP' }
            default { Write-Host "Type PASS, FAIL, or SKIP." }
        }
    }
}

if (-not (Test-Path $NonoBinary)) {
    Write-Log "Nono binary not found at $NonoBinary" "ERROR"
    exit 2
}

Write-Log "==> Phase 30 Wave 1 TUI-rendering manual runbook starting"
Write-Log "==> Log: $logFile"

# ---- Per-shell test sequence -------------------------------------------------
function Invoke-TuiChecklist {
    param([string]$Shell, [string]$Label)

    Write-Log "================================================================"
    Write-Log "==> [$Label] TUI checklist"
    Write-Log "================================================================"

    $shellArg = if ($Shell) { "--shell $Shell" } else { "" }
    $cmd = "$NonoBinary shell --profile claude-code --allow-cwd $shellArg"

    Write-Host @"

================================================================
[$Label] Manual TUI checklist
================================================================
You are about to launch a sandboxed shell. Inside it, run `claude` and
verify the TUI renders cleanly. After each step, type PASS, FAIL, or SKIP.

Per RESEARCH Pitfall 2 (Microsoft Q&A integrity-mismatch failure mode):
silent input drop / broken echo / cmd.exe accepting no commands are
documented failure modes that pass step (1) but fail step (3) or (4).
The full sequence is mandatory.

Steps:
  (1) Run this command in another terminal window:
        $cmd
  (2) Verify the shell prompt renders without `0xC0000142` or silent exit.
  (3) Inside the sandboxed shell, run: claude
  (4) Verify the alternate-screen TUI renders cleanly:
        - Logo + chat input box visible
        - No escape-sequence leakage
        - Cursor positions correctly
        - alt screen active (raw-mode input, not line-mode)
  (5) Type one message; observe the response render correctly (raw-mode input).
  (6) Type /quit to exit claude.
  (7) Type exit to leave the sandboxed shell.

================================================================
"@

    $launch = Read-PassFail -Prompt "(1-2) Shell launches without 0xC0000142 / silent exit"
    Write-Log "[$Label] Step 1-2 launch: $launch"

    $tuiRender = Read-PassFail -Prompt "(3-4) claude TUI renders cleanly (alt screen, cursor, no leakage)"
    Write-Log "[$Label] Step 3-4 TUI render: $tuiRender"

    $rawInput = Read-PassFail -Prompt "(5) Interactive message + response render correctly (raw-mode input)"
    Write-Log "[$Label] Step 5 raw input: $rawInput"

    $exit = Read-PassFail -Prompt "(6-7) /quit + exit clean"
    Write-Log "[$Label] Step 6-7 exit: $exit"

    return @{
        Label = $Label
        Launch = $launch
        TuiRender = $tuiRender
        RawInput = $rawInput
        Exit = $exit
    }
}

# Default: PowerShell 5.1 (per CONTEXT.md acceptance #2 baseline)
$psResult = Invoke-TuiChecklist -Shell "" -Label "PowerShell 5.1 (default)"

# Optional: cmd.exe (RESEARCH Open Question 3)
$cmdResult = $null
if ($IncludeCmd) {
    $cmdResult = Invoke-TuiChecklist `
        -Shell "C:\Windows\System32\cmd.exe" `
        -Label "cmd.exe"
}

# ---- Summary -----------------------------------------------------------------
Write-Log "================================================================"
Write-Log "Phase 30 Wave 1 TUI-rendering manual runbook summary:"
Write-Log "  PowerShell 5.1:"
Write-Log "    Launch:       $($psResult.Launch)"
Write-Log "    TUI render:   $($psResult.TuiRender)"
Write-Log "    Raw input:    $($psResult.RawInput)"
Write-Log "    /quit + exit: $($psResult.Exit)"
if ($cmdResult) {
    Write-Log "  cmd.exe:"
    Write-Log "    Launch:       $($cmdResult.Launch)"
    Write-Log "    TUI render:   $($cmdResult.TuiRender)"
    Write-Log "    Raw input:    $($cmdResult.RawInput)"
    Write-Log "    /quit + exit: $($cmdResult.Exit)"
}
Write-Log "================================================================"

# Overall: PASS if every checked step is PASS or SKIP. FAIL on any FAIL.
$allResults = @($psResult.Launch, $psResult.TuiRender, $psResult.RawInput, $psResult.Exit)
if ($cmdResult) {
    $allResults += @($cmdResult.Launch, $cmdResult.TuiRender, $cmdResult.RawInput, $cmdResult.Exit)
}

if ($allResults -contains 'FAIL') {
    Write-Log "Overall: FAIL -- at least one step failed" "ERROR"
    exit 1
} elseif ($allResults | Where-Object { $_ -eq 'PASS' }) {
    Write-Log "Overall: PASS"
    exit 0
} else {
    Write-Log "Overall: INDETERMINATE -- no PASS results" "WARN"
    exit 2
}
