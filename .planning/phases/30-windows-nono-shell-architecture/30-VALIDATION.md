---
phase: 30
slug: windows-nono-shell-architecture
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-07
---

# Phase 30 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` + PowerShell harness |
| **Config file** | `Cargo.toml` workspace + `tests/windows-test-harness.ps1` |
| **Quick run command** | `cargo test -p nono-cli --lib token_cascade` |
| **Full suite command** | `make test-cli` (Rust) + `pwsh -File scripts/test-windows-shell-write-deny.ps1` (Wave 1 acceptance) |
| **Estimated runtime** | ~30 seconds (unit), ~60 seconds (live-shell harness) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono-cli` (filtered to changed module)
- **After every plan wave:** Run `make test-cli` + (Wave 1 only) `scripts/test-windows-shell-write-deny.ps1`
- **Before `/gsd-verify-work`:** Full suite must be green; live-shell write-deny acceptance must pass on test box
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

> Filled by planner. Each task in PLAN.md inherits a row here. Live-shell acceptance (#3) is the most critical Wave 1 verification.

| Task ID | Plan | Wave | Requirement / Decision | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|------------------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 30-01-01 | 01 | 1 | D-10 (SHELL-01 bookkeeping) | — | PROJECT.md SHELL-01 row reflects current reality | manual+grep | `grep -E "SHELL-01.*needs-rework" .planning/PROJECT.md` | ✅ | ⬜ pending |
| 30-XX-XX | XX | 1 | D-01 (Low-IL primary token + ConPTY) | — | Token cascade adds 6th arm; spawn succeeds; mandatory-label NO_WRITE_UP fires | unit | `cargo test -p nono-cli token_cascade::low_il_pty_arm` | ❌ W0 | ⬜ pending |
| 30-XX-XX | XX | 1 | D-05 (TUI rendering) | — | `claude` launches inside sandboxed shell with full TUI | manual | `pwsh scripts/test-windows-shell-tui.ps1` | ❌ W0 | ⬜ pending |
| 30-XX-XX | XX | 1 | D-06 (OS-level write-deny) | — | `Out-File outside-grant.txt` returns "Access is denied" | E2E | `pwsh scripts/test-windows-shell-write-deny.ps1` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `scripts/test-windows-shell-write-deny.ps1` — drives `Out-File` from inside the sandboxed shell against a path outside the grant set; asserts "Access is denied" / `UnauthorizedAccessException`. Closes acceptance #3.
- [ ] `scripts/test-windows-shell-tui.ps1` — launches `claude` inside the sandboxed shell, asserts alternate screen buffer + cursor positioning render correctly. Closes acceptance #2.
- [ ] Unit test fixture for `create_low_integrity_primary_token` — exercises the function on its own (currently no tests; first-live-use under Wave 1 per RESEARCH.md).
- [ ] Unit test for the new 6th cascade arm — verifies `pty.is_some()` route selects `create_low_integrity_primary_token` and not `create_restricted_token_with_sid`.

---

## Manual-Only Verifications

| Behavior | Requirement / Decision | Why Manual | Test Instructions |
|----------|------------------------|------------|-------------------|
| End-to-end Claude Code TUI rendering | D-05 (acceptance #2) | Subjective rendering quality; cursor positioning + alternate screen buffer not easily asserted programmatically | Launch `nono.exe shell --profile claude-code --allow-cwd` on Windows 10/11 test box; run `claude`; verify TUI renders cleanly (no escape-sequence leakage, cursor positions correctly, alt screen buffer enters/exits) |
| ConPTY allocation does not trigger 0xC0000142 | D-01 (acceptance #1) | Repro requires real Windows 10/11 box; CI Linux/macOS hosts cannot exercise WIndows-specific token-cascade interactions | Field test on test box. Verify `nono.exe shell --profile claude-code` launches without `STATUS_DLL_INIT_FAILED` and without silent exit. Capture exit code in failure log. |
| AppliedLabelsGuard skipping leaked-label paths is non-fatal | D-09 (out-of-scope but expected to surface during field test) | Pre-existing leaked Low-IL labels on test box are not Wave 1's failure mode; "skipping apply + revert" warnings in logs are EXPECTED noise | Inspect `nono shell` log; "label guard: skipping apply + revert" warnings on previously-leaked paths (e.g. `prior_rid="0x1000"`) are expected and non-fatal |
| ProcMon trace plan executable (Wave 2 contingency) | D-04 (Wave 2 conditional, 3-5 day timebox) | Manual filter setup in ProcMon UI + manual analysis of `\Device\ConDrv` ALPC traces | Steps documented in RESEARCH.md ProcMon Trace Plan section; reproduce filter recipe; record trace; identify "surfaced 6th option" if found |
| Cookbook security-envelope paragraph honesty | D-06 + acceptance #6 | Subjective documentation quality | Read `docs/cli/development/windows-poc-handoff.mdx` post-Wave 1; verify it names: token shape used, what's enforced at OS level, what relies on the Claude Code hook |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (3 new test scripts + 2 unit-test fixtures)
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter once planner backfills the Per-Task Verification Map

**Approval:** pending — planner refines per-task rows after PLAN.md generation
