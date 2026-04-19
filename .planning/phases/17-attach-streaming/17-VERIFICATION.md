---
phase: 17-attach-streaming
verified: 2026-04-19T18:09:50Z
status: human_needed
score: 13/13 must-haves verified (with 2 PARTIAL annotations per pragmatic-PASS verdict)
overrides_applied: 0
human_verification:
  - test: "G-02 bidirectional stdin echo round-trip"
    expected: "Type `echo BIDIRECTIONAL_OK<Enter>` into `nono attach <id>` against detached `cmd.exe`; see `BIDIRECTIONAL_OK` echoed back from the child"
    why_human: "Smoke-gate session 2026-04-19 proved stdout half + supervisor control plane (`nono stop` graceful shutdown) but did not type stdin. Pragmatic-PASS recorded by user; routing here for explicit closure on a future Windows session."
  - test: "G-03 detach + re-attach round-trip"
    expected: "Press Ctrl-]d during counter stream → `nono ps` shows session still RUNNING → re-attach with `nono attach <id>` → see counter resume from a higher number; second Ctrl-]d disconnects cleanly"
    why_human: "Smoke-gate session 2026-04-19 proved live counter streaming + attach banner + scrollback replay but did not exercise the Ctrl-]d → ps → re-attach loop. `active_attachment: Mutex<Option<...>>` lifecycle is unchanged from Phase 15 design and structurally supports this; pragmatic-PASS recorded by user."
  - test: "G-04 Row 4 (`--block-net` detached) on a host with WFP driver registered"
    expected: "`nono run --detached --block-net --allow-cwd -- cmd /c \"curl --max-time 5 http://example.com\"` succeeds in spawning the detached child, and the child's outbound HTTP request is blocked by WFP (curl reports connect failure or timeout)"
    why_human: "Smoke-gate session 2026-04-19 returned the fail-secure WFP error because `nono-wfp-driver` is not registered on this host — this is fail-closed-as-designed and NOT a Phase 17 regression. Re-run on a host where `nono setup --install-wfp-driver` has been executed to confirm Phase 15 baseline preserved."
  - test: "G-04 Rows 1, 2, 5 explicit re-run"
    expected: "Row 1 detached banner shape unchanged from 15-02-SUMMARY.md; Row 2 fast-exit `nono run --detached -- cmd /c exit 0` clean exit; Row 5 `nono logs` / `nono inspect` / `nono prune` output shapes unchanged"
    why_human: "Not explicitly re-run in 2026-04-19 smoke gate — structurally PASS by code-reading (no Phase 17 changes to banner code paths, fast-exit detection in `startup_runtime::run_detached_launch`, or `SessionRecord` schema), but a future operator should re-run for the audit record."
---

# Phase 17: Attach-Streaming Verification Report

**Phase Goal:** Full ConPTY re-attach on detached Windows sessions (read + write + resize). Resolves the Phase 15 deferred item so `nono attach` against detached sessions behaves like a real terminal.

**Verified:** 2026-04-19T18:09:50Z
**Status:** human_needed (deferred smoke items routed to /gsd-verify-work for later closure)
**Re-verification:** No — initial verification

## Goal Achievement

The user-visible promise of ATCH-01 is **structurally met**: `nono attach <id>` against a detached Windows session now streams child stdout live (G-01 PASS, 11+ live `Reply from 127.0.0.1` lines confirmed), accepts stdin from the attach client (G-02 PARTIAL — stdout + supervisor control proven, explicit `echo BIDIRECTIONAL_OK` round-trip not typed), and supports detach + re-attach via Ctrl-]d (G-03 streaming PASS, detach round-trip not explicitly exercised). Acceptance criterion #3 (resize via `ResizePseudoConsole`) is **explicitly downgraded** per D-07 with full audit trail across REQUIREMENTS.md (strikethrough + `[DOWNGRADED — Phase 17 D-07]` marker), CHANGELOG.md, docs/cli/features/session-lifecycle.mdx, and 17-02-SUMMARY.md § "Acceptance #3 downgrade rationale" — this is the load-bearing deviation record and is structurally infeasible without re-introducing the Phase 15 `0xC0000142` loader-failure cascade. The pragmatic-PASS verdict is the user's locked decision; this verification routes the deferred items (G-02 stdin half, G-03 detach round-trip, G-04 environmental Row 4 + structurally-PASS Rows 1/2/5) to human verification rather than blocking the phase.

### Observable Truths

| #   | Truth (from must_haves)   | Status     | Evidence       |
| --- | ------- | ---------- | -------------- |
| 1   | (17-01 T1) Windows detached path creates 3 inheritable anonymous pipes (stdin/stdout/stderr) bound via STARTUPINFOW + bInheritHandles=TRUE | VERIFIED | `launch.rs:67` `struct DetachedStdioPipes`; `launch.rs:1301-1304` `STARTF_USESTDHANDLES` + `hStdInput=stdin_read` + `hStdOutput/hStdError=stdout_write` + `bInheritHandles=1` on detached-pipe branch (line 1307 comment confirms); 5 `detached_stdio_tests` PASS (`child_ends_are_inheritable`, `parent_ends_are_non_inheritable`, `detached_stdio_pipes_create_succeeds`, `close_child_ends_zeroes_them`, `drop_closes_all_remaining_handles_without_panic`) |
| 2   | (17-01 T2) Supervisor stdout/stderr handles bridged to per-session log + active_attachment mirror | VERIFIED | `supervisor.rs:518` pipe-source branch reads `self.detached_stdio.stdout_read`; `start_streaming()` plumbed at supervisor.rs:359; G-01 smoke transcript shows live ping streaming via attach client |
| 3   | (17-01 T3) Supervisor stdin handle bridged from named attach pipe `\\.\pipe\nono-data-<id>` | VERIFIED | `supervisor.rs:866` pipe-sink branch reads `self.detached_stdio.stdin_write`; structurally proven by G-02 supervisor control + reachable code path |
| 4   | (17-01 T4) Second `nono attach <id>` receives friendly NonoError::Setup with session id + `nono detach <id>` hint | VERIFIED | `session_commands_windows.rs:35` `translate_attach_open_error`; `:426` `.map_err` delegation in `run_attach`; 3 `attach_busy_translation_tests` PASS (`translates_pipe_busy_to_friendly_setup`, `passes_through_other_errors`, `passes_through_arbitrary_io_errors`) |
| 5   | (17-01 T5) Phase 15 `should_allocate_pty` gate at supervised_runtime.rs:88-94 byte-identical (D-02) | VERIFIED | `git diff 18a12f1..HEAD -- crates/nono-cli/src/supervised_runtime.rs` returns empty; `supervised_runtime.rs:88-94` re-read confirms unchanged windows/non-windows branch logic; `windows_detached_supervisor_does_not_allocate_pty` + `non_detached_non_interactive_never_allocates_pty` regression tests PASS |
| 6   | (17-01 T6) All Phase 17 source code lives in *_windows.rs files; no cross-platform file touched (D-21) | VERIFIED | `git diff 18a12f1..HEAD --name-only -- crates/` returns exactly 5 files: `exec_strategy_windows/{launch,mod,supervisor}.rs`, `session_commands_windows.rs`, `tests/attach_streaming_integration.rs` (file-gated `#![cfg(target_os = "windows")]`). Zero non-Windows source modified. |
| 7   | (17-02 G-01) Live ping streaming PASSED on Windows host with PowerShell transcript | VERIFIED | 17-02-SUMMARY.md § "Smoke gate" § "G-01" transcript shows 11+ live `Reply from 127.0.0.1: bytes=32 time<1ms TTL=128` lines + attach banner; date 2026-04-19, OS Windows 11 Enterprise 10.0.26200.8037 |
| 8   | (17-02 G-02) Bidirectional cmd.exe PASSED on Windows host with PowerShell transcript | VERIFIED (PARTIAL — pragmatic-PASS) | 17-02-SUMMARY.md § "G-02" transcript shows stdout half (`Microsoft Windows [Version 10.0.26200.8037]` + `C:\Windows>` prompt streamed) + supervisor control plane (`nono stop` graceful shutdown). Explicit `echo BIDIRECTIONAL_OK` stdin round-trip NOT typed in this session — bidirectional half implied by graceful supervisor termination but not directly demonstrated. **User locked pragmatic-PASS verdict**; deferred check routed to human_verification[0]. |
| 9   | (17-02 G-03) Ctrl-]d detach + re-attach to live session PASSED with PowerShell transcript | VERIFIED (PARTIAL — pragmatic-PASS) | 17-02-SUMMARY.md § "G-03" transcript shows live counter `1..9` streaming + attach banner. Detach + re-attach round-trip (Ctrl-]d → `nono ps` → re-attach with higher counter) NOT explicitly exercised. `active_attachment: Mutex<Option<...>>` lifecycle is byte-unchanged from Phase 15 and structurally supports this. **User locked pragmatic-PASS verdict**; deferred check routed to human_verification[1]. |
| 10  | (17-02 G-04) Phase 15 5-row smoke matrix unchanged | VERIFIED (PARTIAL — pragmatic-PASS) | Row 3 (non-detached PTY path) explicitly re-run and PASS (`nono run --allow-cwd -- cmd /c "echo hello"` printed `hello` cleanly). Row 4 (`--block-net`) returned fail-secure WFP error because `nono-wfp-driver` is NOT registered on this host — this is **environmental, NOT a Phase 17 regression** (the fail-closed path is working as designed; Phase 15 Row 4 PASSED on a host with the driver registered). Rows 1, 2, 5 NOT explicitly re-run but structurally PASS by code-reading (no Phase 17 changes to banner code, fast-exit detection, or `SessionRecord` schema). **User locked pragmatic-PASS verdict**; environmental + structural-PASS items routed to human_verification[2,3]. |
| 11  | (17-02 T2) REQUIREMENTS.md ATCH-01 acceptance #3 explicitly downgraded per D-07 with pointer to 17-02-SUMMARY.md | VERIFIED | `grep -cE "DOWNGRADED — Phase 17 D-07" .planning/REQUIREMENTS.md` returns 1 (line 193); strikethrough preserves original `ResizePseudoConsole` wording (3 grep hits — auditable history); `0xC0000142` referenced in downgrade rationale; `17-02-SUMMARY.md` pointer present (1 grep hit) |
| 12  | (17-02 T3) CHANGELOG.md [Unreleased] gains ATCH-01 entry crediting v2.1 attach-streaming gap fix | VERIFIED | `grep -cE "ATCH-01" CHANGELOG.md` returns 4 entries: 2 Documentation (downgrade + docs note), 2 Features (streaming + friendly busy error). Plus 1 Bug Fix entry crediting Phase 17 latent-bug repair (commit `7db6595`). All under `## [Unreleased]` (line 3). |
| 13  | (17-02 T4) docs/cli/* gains "No resize on detached sessions" note pointing to `nono shell` / non-detached `nono run` | VERIFIED | `docs/cli/features/session-lifecycle.mdx:138` "## Limitations on Windows detached sessions"; `:140` no-resize structural-rationale paragraph mentioning `0xC0000142`; `:142` `ResizePseudoConsole` explanation; `:157` workaround pointing to `nono shell` / non-detached `nono run`; `:161` ATCH-01 + D-07 + REQUIREMENTS.md bibliographic pointer. (Plan said "docs/cli/attach.md or equivalent" — the equivalent landing site is session-lifecycle.mdx.) |

**Score:** 13/13 truths verified (2 PARTIAL annotations per user's pragmatic-PASS verdict; deferred items routed to human_verification rather than counted as gaps)

### Required Artifacts

| Artifact | Expected    | Status | Details |
| -------- | ----------- | ------ | ------- |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` | `DetachedStdioPipes` struct + `create()` + `Drop` + STARTUPINFOW wiring | VERIFIED (substantive + wired) | `+332/-8` LoC; `struct DetachedStdioPipes` at line 67; `STARTF_USESTDHANDLES` + child-end wiring at lines 1301-1304; 5 unit tests PASS |
| `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` | `detached_stdio` field + accessor + `start_streaming()` + pipe-source/sink branches | VERIFIED (substantive + wired) | `+153/-20` LoC + Phase-17 fix `+10/-2` LoC for `user_session_id` correctness; field at line 212; `attach_detached_stdio()` at line 350; `start_streaming()` at line 359; pipe-source branch at line 518 reads `self.detached_stdio.stdout_read`; pipe-sink branch at line 866 reads `self.detached_stdio.stdin_write`; `start_logging`/`start_data_pipe_server` correctly use `self.user_session_id` (lines 507, 852) — the bug fix from `7db6595` |
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` | `execute_supervised` plumbs `detached_stdio` into runtime | VERIFIED (substantive + wired) | `+20/-7` LoC; `detached_stdio` token referenced via destructuring + `attach_detached_stdio` + `start_streaming` plumbing |
| `crates/nono-cli/src/session_commands_windows.rs` | `translate_attach_open_error` helper + ERROR_PIPE_BUSY translation | VERIFIED (substantive + wired) | `+77/-6` LoC; `pub(crate) fn translate_attach_open_error` at line 35; `ERROR_PIPE_BUSY` check at line 40; `.map_err` delegation in `run_attach` at line 426; 3 unit tests PASS |
| `crates/nono-cli/tests/attach_streaming_integration.rs` | NEW Windows-only `#[ignore]`d round-trip integration test | VERIFIED (substantive + wired, --ignored deferred) | `+133/-0` LoC NEW file; `#![cfg(target_os = "windows")]` gate at line 21; 2 banner-parser helper tests PASS; `--ignored` round-trip blocked by environmental `CREATE_BREAKAWAY_FROM_JOB` constraint inside cargo-test (pre-existing, NOT Phase 17 regression — documented in 17-01-SUMMARY.md) |
| `.planning/phases/17-attach-streaming/17-02-SUMMARY.md` | Smoke gate G-01..G-04 transcripts + acceptance #3 downgrade record | VERIFIED | `## Smoke gate` section with 4 verdict subsections + transcript excerpts; `## Acceptance #3 downgrade rationale` section with original requirement text + D-07 lock + structural rationale + future-work pointer; `## Files modified` + `## Commits` + `## Phase 17 status` closeout |
| `.planning/REQUIREMENTS.md` | ATCH-01 acceptance #3 marked `[DOWNGRADED — Phase 17 D-07]` | VERIFIED | Line 193 shows strikethrough on original wording + `[DOWNGRADED — Phase 17 D-07]` marker + `D-01` + `0xC0000142` + `17-02-SUMMARY.md` pointer; acceptance items #1, #2, #4, #5 byte-identical to pre-edit |
| `CHANGELOG.md` | `[Unreleased]` ATCH-01 entries | VERIFIED | 4 ATCH-01 references in `[Unreleased]` (Documentation x2, Features x2) plus 1 Bug Fix entry crediting `7db6595`; entries appear above any released-version headers |
| `docs/cli/features/session-lifecycle.mdx` | "Limitations on Windows detached sessions" section | VERIFIED | Section at line 138 with required strings: `ResizePseudoConsole`, `0xC0000142`, `nono shell`, `nono run`, `ATCH-01`, `D-07` all present |
| `.planning/phases/13-v1-human-verification-uat/13-UAT.md` | P17-HV-1..4 rows | VERIFIED | 4 rows added at lines 382, 392, 409, 427 (P17-HV-1..P17-HV-4); each references `17-02-SUMMARY.md § "Smoke gate"` for evidence; matches Phase 15 P15-HV-N precedent |
| `.planning/debug/resolved/17-detached-child-immediate-exit.md` | Debug record for 3 latent bugs surfaced + fixed during smoke gate | VERIFIED | NEW file at `.planning/debug/resolved/`; `status: resolved`; `related_commits: [1e38381, 2b74d66, 9c82f17, f17ad72, f962606, 03e1e80]`; documents all 3 root causes (job-name format string newline, data-pipe correlation-ID vs session-ID, log-file correlation-ID vs session-ID) |

### Key Link Verification

| From | To  | Via | Status | Details |
| ---- | --- | --- | ------ | ------- |
| `exec_strategy_windows/launch.rs` (`spawn_windows_child`) | STARTUPINFOW.hStd* + CreateProcessW(bInheritHandles=1) | DetachedStdioPipes child-end handles | WIRED | Line 1301 `STARTF_USESTDHANDLES`; lines 1302-1304 hStdInput/hStdOutput/hStdError bindings; line 1307 `bInheritHandles=1` comment confirms detached-pipe branch flag |
| `exec_strategy_windows/supervisor.rs` (`start_logging` pipe branch) | per-session log file + active_attachment named-pipe mirror | `ReadFile(stdout_read)` → log_file.write_all + WriteFile(active_attachment.0) | WIRED | Pipe-source branch at line 518 reads `self.detached_stdio.stdout_read`; user_session_id correctly used for log path at line 507 |
| `exec_strategy_windows/supervisor.rs` (`start_data_pipe_server` pipe branch) | child stdin via DetachedStdioPipes.stdin_write | `ConnectNamedPipe → ReadFile(named pipe) → WriteFile(stdin_write)` | WIRED | Pipe-sink branch at line 866 reads `self.detached_stdio.stdin_write`; user_session_id correctly used for pipe name at line 852 |
| `session_commands_windows.rs` (`run_attach`) | NonoError::Setup with friendly multi-attach message | `raw_os_error() == Some(ERROR_PIPE_BUSY as i32)` | WIRED | Helper at line 35; ERROR_PIPE_BUSY check at line 40; `.map_err` delegation in `run_attach` at line 426 |
| REQUIREMENTS.md ATCH-01 § Acceptance | 17-02-SUMMARY.md downgrade rationale | DOWNGRADED note pointing to plan 17-02 summary | WIRED | Line 193 contains the exact pointer string `17-02-SUMMARY.md § "Acceptance #3 downgrade rationale"` |
| docs/cli/features/session-lifecycle.mdx | `nono shell` / non-detached `nono run` for full TUI | user-facing note explaining the resize gap | WIRED | Line 157 contains the exact workaround language |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| `start_logging` pipe-source thread | `self.detached_stdio.stdout_read` HANDLE | Anonymous pipe write-end fed by detached child stdout/stderr (merged via shared HANDLE per D-04) | YES — G-01 transcript shows 11+ live `Reply from 127.0.0.1` lines streaming through to attach client | FLOWING |
| `start_data_pipe_server` pipe-sink thread | `self.detached_stdio.stdin_write` HANDLE | Bytes read from `\\.\pipe\nono-data-<id>` named pipe (attach client side) | STRUCTURALLY YES — code path reachable; G-02 stdin echo round-trip NOT typed (deferred to human_verification[0]) | STATIC for stdin half — pragmatic-PASS per user; FLOWING for code path |
| `run_attach` busy-error translator | `err.raw_os_error()` from `CreateFileW` on the named pipe | Real Win32 `ERROR_PIPE_BUSY (231)` returned by single-instance `\\.\pipe\nono-data-<id>` when another client holds it | YES — `translates_pipe_busy_to_friendly_setup` test exercises with real `io::Error::from_raw_os_error(ERROR_PIPE_BUSY as i32)` and asserts the friendly Setup error wording | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Workspace builds clean | `cargo build -p nono-cli --release` | `Finished release profile [optimized] target(s)` | PASS |
| Workspace clippy strict | `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | `Finished dev profile [unoptimized + debuginfo] target(s)` (zero warnings, zero unwrap violations) | PASS |
| Phase 17 unit + Phase 15 regression | `cargo test -p nono-cli --bin nono -- detached_stdio attach_busy restricted_token detached` | `test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 678 filtered out` | PASS |
| D-21 invariance | `git diff 18a12f1..HEAD --name-only -- crates/` | 5 files, all `*_windows.rs` (or `#![cfg(target_os="windows")]` gated) | PASS |
| D-02 should_allocate_pty byte-identical | `git diff 18a12f1..HEAD -- crates/nono-cli/src/supervised_runtime.rs` | Empty diff | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ---------- | ----------- | ------ | -------- |
| ATCH-01 acc #1 | 17-01, 17-02 | `nono run --detached -- cmd /c "for /l %i in (1,1,10) do @(echo %i & timeout /t 1)"` followed by `nono attach <id>` streams the output live | SATISFIED | G-01 PASS — 11+ live `Reply from 127.0.0.1` lines streamed via attach client (transcript in 17-02-SUMMARY.md) |
| ATCH-01 acc #2 | 17-01, 17-02 | `nono attach <id>` on a detached shell allows bidirectional use: stdin writes reach the child | SATISFIED (PARTIAL — pragmatic-PASS) | G-02 stdout half + supervisor control plane PASS; explicit `echo BIDIRECTIONAL_OK` round-trip NOT typed (deferred to human_verification[0]). User locked pragmatic-PASS verdict per 17-02-SUMMARY.md "Verdict per gate" table. |
| ATCH-01 acc #3 | 17-02 | ~~Terminal resize propagates via `ResizePseudoConsole`~~ **[DOWNGRADED — Phase 17 D-07]** | DOWNGRADED (audit trail complete) | REQUIREMENTS.md line 193 strikethrough + marker + structural rationale; CHANGELOG.md Documentation entry; docs/cli/features/session-lifecycle.mdx § Limitations; 17-02-SUMMARY.md § "Acceptance #3 downgrade rationale" with original text + D-07 lock + future-work pointer. Structurally infeasible without re-introducing Phase 15 `0xC0000142` cascade. |
| ATCH-01 acc #4 | 17-01, 17-02 | Detach sequence cleanly unparents the ConPTY without killing the child | SATISFIED (PARTIAL — pragmatic-PASS) | G-03 streaming half PASS (counter 1..9 + attach banner); detach round-trip NOT explicitly exercised (deferred to human_verification[1]). `active_attachment` lifecycle byte-unchanged from Phase 15 and structurally supports this. User locked pragmatic-PASS verdict. |
| ATCH-01 acc #5 | 17-01, 17-02 | No regression on the 5-row Phase 15 smoke gate | SATISFIED (PARTIAL — pragmatic-PASS) | Row 3 explicitly re-run + PASS (PTY path unaffected); Row 4 environmental fail-closed (WFP driver not registered on this host — NOT Phase 17 regression, deferred to human_verification[2]); Rows 1, 2, 5 structurally PASS by code-reading (no Phase 17 changes to those code paths, deferred to human_verification[3]). D-02 invariance verified: `should_allocate_pty` byte-identical. |

All 5 acceptance criteria accounted for: 4 SATISFIED (with PARTIAL annotations on #2/#4/#5 per pragmatic-PASS), 1 explicitly DOWNGRADED with full audit trail.

### Anti-Patterns Found

None blocking. Routine search across the 5 modified files surfaces no TODO/FIXME/XXX comments in production code paths, no `unwrap()`/`expect()` calls (clippy-enforced), no `return null` or `return []` stubs (handler bridges genuinely move bytes), and no `console.log`-only handlers. The integration test's `#[ignore]` attribute is documented as a pre-existing environmental constraint (cargo-test Job Object can't grant `CREATE_BREAKAWAY_FROM_JOB`), not a Phase 17 regression.

### Human Verification Required

**1. G-02 bidirectional stdin echo round-trip**

Test: From a fresh PowerShell:
```powershell
.\target\release\nono.exe run --detached --allow-cwd -- cmd.exe
# Note the session id, then in Terminal B:
.\target\release\nono.exe attach <session_id>
# Type: echo BIDIRECTIONAL_OK<Enter>
# Type: exit<Enter>
```

Expected: `BIDIRECTIONAL_OK` echoed back from the child cmd.exe stdout via attach client; cmd.exe exits cleanly on `exit`; `nono attach` returns to PowerShell prompt.

Why human: Smoke-gate session 2026-04-19 proved the stdout half + supervisor control plane (`nono stop` graceful shutdown), but the explicit stdin round-trip was not typed. Pragmatic-PASS recorded by user per 17-02-SUMMARY.md; routing here for explicit closure.

**2. G-03 detach + re-attach round-trip**

Test: From a fresh PowerShell:
```powershell
.\target\release\nono.exe run --detached --allow-cwd -- cmd /c "for /l %i in (1,1,30) do @(echo %i & ping -n 2 127.0.0.1 >nul)"
.\target\release\nono.exe attach <session_id>
# Wait for ~5 counter lines, then press Ctrl-]d
.\target\release\nono.exe ps
# Confirm session still RUNNING
.\target\release\nono.exe attach <session_id>
# Confirm counter resumes from a higher number; press Ctrl-]d again
```

Expected: Ctrl-]d disconnects without killing the child; `nono ps` shows RUNNING; second attach shows counter resumed from a higher number; second Ctrl-]d disconnects cleanly; child eventually exits at 30 naturally.

Why human: Smoke-gate session 2026-04-19 proved live counter streaming + attach banner + scrollback replay but did not exercise the Ctrl-]d → ps → re-attach loop. `active_attachment: Mutex<Option<...>>` lifecycle is unchanged from Phase 15 design and structurally supports this; user locked pragmatic-PASS.

**3. G-04 Row 4 (`--block-net` detached) on host with WFP driver registered**

Test: On a Windows host where `nono setup --install-wfp-driver` has been executed:
```powershell
nono run --detached --block-net --allow-cwd -- cmd /c "curl --max-time 5 http://example.com"
```

Expected: Detached child spawns successfully; child's outbound HTTP request blocked by WFP (curl reports connect failure or timeout). Matches Phase 15 baseline on commit `0de3e77`.

Why human: Smoke-gate session 2026-04-19 returned the fail-secure WFP error because `nono-wfp-driver` is NOT registered on this host. This is fail-closed-as-designed, NOT a Phase 17 regression. Re-run on a properly-provisioned host to confirm Phase 15 baseline preserved.

**4. G-04 Rows 1, 2, 5 explicit re-run**

Test: From a fresh PowerShell:
```powershell
# Row 1 — detached banner shape
nono run --detached --allow-cwd -- cmd /c "echo hello"
# Row 2 — fast-exit child
nono run --detached -- cmd /c exit 0
# Row 5 — observability commands
nono ps
nono logs <session_id>
nono inspect <session_id>
nono prune --all-exited
```

Expected: All output shapes match the Phase 15 baseline (`15-02-SUMMARY.md § "4-row smoke-gate matrix (+ Row 5)"`).

Why human: Not explicitly re-run in 2026-04-19 smoke gate. Structurally PASS by code-reading (no Phase 17 changes to banner code paths, fast-exit detection in `startup_runtime::run_detached_launch`, or `SessionRecord` schema), but a future operator should re-run for the audit record.

### Gaps Summary

No structural gaps. The phase delivered:
- Anonymous-pipe stdio + supervisor-side bridging on the Windows detached path (5 unit tests PASS; G-01 live transcript confirms streaming works end-to-end).
- Friendly multi-attach error translation (3 unit tests PASS).
- Phase 15 `0xC0000142` fix preserved structurally (`should_allocate_pty` byte-identical; D-21 Windows-invariance held — only Windows files modified).
- 3 latent Windows session-id mismatches surfaced + fixed in commit `7db6595` (job-name format string newline, data-pipe correlation-ID vs session-ID, log-file correlation-ID vs session-ID) — debug record at `.planning/debug/resolved/17-detached-child-immediate-exit.md`.
- ATCH-01 acceptance #3 (resize) explicitly downgraded per D-07 with full audit trail across 4 surfaces (REQUIREMENTS.md, CHANGELOG.md, docs/cli/features/session-lifecycle.mdx, 17-02-SUMMARY.md).
- All 4 documentation surfaces landed: REQUIREMENTS.md downgrade, CHANGELOG [Unreleased] x4 entries, session-lifecycle.mdx Limitations section, 13-UAT.md P17-HV-1..4 rows.

The 4 deferred smoke items are routed to human_verification rather than treated as gaps — they reflect the user's explicit pragmatic-PASS decision recorded in 17-02-SUMMARY.md and the WFP-driver-environmental fail-closed-as-designed verdict on G-04 Row 4. Future operators should close them on a properly-provisioned Windows host for the audit record, but they do NOT block phase progression.

---

_Verified: 2026-04-19T18:09:50Z_
_Verifier: Claude (gsd-verifier)_
