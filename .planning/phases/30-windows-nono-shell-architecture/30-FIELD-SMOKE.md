# Phase 30 Wave 1 — Field Smoke Runbook

**Run by:** Plan 30-04 on the Windows test box (Win 11 26200 or any Windows 10 17763+ with WSL/PowerShell 5.1+).
**Outcome drives:** Plan 30-04's cookbook update + PROJECT.md SHELL-01 outcome flip + STATE.md key-decisions block.
**Failure trigger:** If any Acceptance #1-#4 fails, Plan 30-05 (Wave 2 ProcMon) is spawned with a 3-5 working-day timebox per CONTEXT.md D-04.

***

## Pre-test environment hygiene

1. **Close all existing nono sessions on the test box.** Use `nono prune --all` or `taskkill /f /im nono.exe` if any orphaned supervisors are running. The capability-pipe DACL is per-supervisor; stale supervisors hold pipe handles that can confuse the new launch.

2. **Confirm Wave 1 binary is on the working tree.** Plan 30-02's cascade arm must be present. Verify:
   ```
   grep -c "WindowsTokenArm::LowIlPrimary" crates/nono-cli/src/exec_strategy_windows/launch.rs
   ```
   Expected: >= 1.

3. **Build a fresh release binary:**
   ```
   cargo build -p nono-cli --release --target x86_64-pc-windows-msvc
   ```
   Expected: exit 0; binary at `target/x86_64-pc-windows-msvc/release/nono.exe`.

4. **Verify Claude Code is installed:**
   ```
   claude --version
   ```
   Acceptance #2 cannot be exercised without a working `claude` CLI.

5. **(Optional) Clear D-09 leaked Low-IL labels.** The write-deny harness includes this prelude by default. If running smoke tests by hand, run:
   ```powershell
   $leaked = @(
       "$env:USERPROFILE\.cache\claude",
       "$env:USERPROFILE\.cargo",
       "$env:USERPROFILE\.claude",
       "$env:USERPROFILE\.config\git\ignore",
       "$env:USERPROFILE\.gitconfig",
       "$env:USERPROFILE\.local\bin",
       "$env:USERPROFILE\.rustup",
       "$env:USERPROFILE\AppData\Roaming\nono\profiles",
       "$env:USERPROFILE\Nono"
   )
   foreach ($p in $leaked) { icacls $p /setintegritylevel "(NX)Medium" 2>$null }
   ```
   D-09 is OUT OF SCOPE for Phase 30; this is just isolation hygiene to avoid stale labels confusing the smoke result.

***

## Acceptance criteria -> harness commands

| Acceptance | Decision | Harness | Expected | Gate |
|------------|----------|---------|----------|------|
| #1: shell launches without 0xC0000142 | D-01 | Manual: `.\nono.exe shell --profile claude-code --allow-cwd` | Shell prompt appears; no STATUS_DLL_INIT_FAILED; no silent exit | Operator visual |
| #2: claude TUI renders | D-05 | `pwsh -File scripts/test-windows-shell-tui.ps1` | All checklist steps PASS | Script exit 0 |
| #3: write outside grant set is denied | D-06 | `pwsh -File scripts/test-windows-shell-write-deny.ps1` | Inner shell exit 42 (sentinel); harness exit 0 | Script exit 0 with `Acceptance #3 result: PASS` in log |
| #4: read of granted path works | D-06 inverse | Same harness as #3 (`-IncludeReadCheck` default) | Inner shell exit 42 on Get-Content; or 99 if claude.json missing | Script exit 0 with `Acceptance #4 result: PASS` or `SKIPPED (file missing)` |

***

## Smoke-gate evidence table (mirrors Phase 15 shape)

Fill in the rightmost column during Wave 1 field execution.

| Token | PTY | Detached | Outcome (Wave 1 expected) | Outcome (Wave 1 observed) |
|---|---|---|---|---|
| WRITE_RESTRICTED + session-SID | Some (ConPTY) | Yes | 0xC0000142 (legacy regression -- pre-fix) | (regressed if observed) |
| Low-IL primary (Phase 30) | Some (ConPTY) | No | Launches; mandatory-label NO_WRITE_UP enforces write-deny outside grant set; AppID-WFP fallback for network | _(operator fills)_ |
| Null token | Some (ConPTY) | Yes | Phase 15 detached path; launches; no write enforcement | (unchanged) |
| Null token | None | Yes | Phase 15 detached path | (unchanged) |
| WRITE_RESTRICTED + session-SID | None | No | Existing `nono run` non-PTY supervised; unchanged from HEAD | _(verify no regression)_ |

***

## Expected log markers

These messages indicate **healthy** Wave 1 behavior. Their absence is a regression signal.

- **`child connected to pipe`** (or equivalent -- `start_capability_pipe_server` log line) -- capability-pipe SDDL admitted the Low-IL child via `OW` ACE. RESEARCH Question 1 / Assumption A1 verified.
- **`label guard: skipping apply + revert`** -- D-09 leaked-label warnings on the 9 known paths. EXPECTED noise; not a failure indicator.
- **`path not owned by current user`** for `C:\Windows` -- Phase 21 design behavior; expected.

These messages indicate **failure** modes:

- **`STATUS_DLL_INIT_FAILED` / `0xC0000142`** -- Wave 1 cascade arm broken; binary regressed; investigate `select_windows_token_arm` truth-table coverage.
- **`Failed to connect to Windows supervisor pipe` / `ERROR_FILE_NOT_FOUND`** -- RESEARCH Pitfall 4 rendezvous-file accessibility. Document and proceed with Wave 2 trigger if persistent.
- **`Access is denied` from `Get-Content` on a granted path** -- Acceptance #4 violated; Low-IL token may be silently mis-applied; check `low_integrity_primary_token_sets_low_il` test result.
- **Silent input drop / broken echo in claude TUI** -- RESEARCH Pitfall 2 / Assumption A4 (Microsoft Q&A integrity-mismatch). Step 5 of TUI checklist FAILS; trigger Wave 2 ProcMon.

***

## Decision matrix (drives Plan 30-04)

| Acceptance #1 | #2 | #3 | #4 | Plan 30-04 path |
|---|---|---|---|---|
| PASS | PASS | PASS | PASS | **Success path:** SHELL-01 -> validated v2.X Phase 30; cookbook security-envelope paragraph; debug session -> resolved/. |
| PASS | PASS | PASS | SKIPPED (file missing) | Same as success path; document the missing file in SUMMARY (test box may not have ~/.claude/claude.json yet). |
| PASS | PASS | FAIL | * | **Wave 2 trigger:** OS-level write-deny did not fire. ProcMon trace MIC enforcement on `~/Desktop`; verify Low-IL token is actually applied. |
| PASS | FAIL | * | * | **Wave 2 trigger:** Pitfall 2 hit. ProcMon trace `\Device\ConDrv` ALPC. |
| FAIL | * | * | * | **Wave 2 trigger:** 0xC0000142 returned (or shell exits silently). ProcMon trace ImageLoad chain in conhost.exe + child to identify failed DllMain. |

If Wave 2 ALSO fails (3-5 working days exhausted): SHELL-01 -> deferred to v3.0; cookbook revert per RESEARCH "Cookbook Rollback Path" Option Rev-B.

***

## Operator log

| Date | Acceptance #1 | #2 | #3 | #4 | Notes |
|------|--------------|----|----|----|-------|
| 2026-05-07 | FAIL | UNTESTED (Checkpoint 1 false-positive PASS) | UNTESTED | UNTESTED | Wave 1 field smoke; Acceptance #1 silent launch failure (same PID + Medium-IL + no nono process per manual `whoami /groups` + `$PID` + `Get-Process nono` after `nono shell`). Acceptance #2 reported PASS at Checkpoint 1 but RETROACTIVELY UNTESTED — `claude` rendered in OUTER shell (RESEARCH Pitfall 2 silent-failure mode). #3/#4 unreachable (couldn't enter sandbox). Two harness bugs surfaced & one fixed inline (`$p:` → `${p}:`); two remain open (`nono shell` does not accept trailing args after `--`; `Out-File '$path' 'content'` invalid syntax). Wave 2 (Plan 30-05) triggered: `wave2-trigger-launch`. Commits 30-01..30-03 shipped; 30-04 ships partial-close. |

***

## References

- Plan 30-02 (token-cascade implementation): `.planning/phases/30-windows-nono-shell-architecture/30-02-PLAN.md`
- Plan 30-03 (this -- harness scripts): `.planning/phases/30-windows-nono-shell-architecture/30-03-PLAN.md`
- Plan 30-04 (smoke execution + outcome flip): `.planning/phases/30-windows-nono-shell-architecture/30-04-PLAN.md`
- Plan 30-05 (Wave 2 ProcMon -- conditional): `.planning/phases/30-windows-nono-shell-architecture/30-05-PLAN.md`
- Phase 15 smoke-gate precedent: `.planning/debug/resolved/windows-supervised-exec-cascade.md` - "Phase 15 Smoke Gate"
- RESEARCH Validation Architecture: `.planning/phases/30-windows-nono-shell-architecture/30-RESEARCH.md` - "Validation Architecture"
