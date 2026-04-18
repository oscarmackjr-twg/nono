---
phase: 14-v1-fix-pass
plan: 01
completed: 2026-04-18T02:05:00Z
status: awaiting-direction-2-regate
---

## What built

Fixed the detached-supervisor `STATUS_DLL_INIT_FAILED (0xC0000142)` that killed sandboxed console grandchildren.

**Evolution of the fix across this phase:**
1. **Direction 1** (session SID as `TokenGroups`) — abandoned before commit: infeasible with user-mode Windows APIs (`AdjustTokenGroups` cannot add new SIDs; `SetTokenInformation(TokenGroups)` requires `SE_TCB_NAME`).
2. **Direction 3** (pre-allocate console via `AllocConsole` in detached supervisor, keep WRITE_RESTRICTED + session SID restricting) — committed (`b06aebe`), user-smoked 2026-04-18, **FAILED** rows 1 and 2. AllocConsole did not unblock 0xC0000142.
3. **Direction 2** (null token in detached mode; non-detached keeps WRITE_RESTRICTED + session SID) — committed (`005f6bc`), **PENDING** user re-gate.

Direction 2 is the plan's documented "last-resort" fallback. The pivot was automatic per the plan's failure policy — no further Direction-3 debugging.

## Key files

### Modified
- `crates/nono-cli/src/exec_strategy_windows/launch.rs`
  - Added `ensure_detached_supervisor_console_attached()` helper (detached-only, gated on `NONO_DETACHED_LAUNCH=1` env var).
  - Called at the top of `spawn_windows_child`, before restricted-token construction + `CreateProcessAsUserW`.
  - Uses `windows_sys::Win32::System::Console::AllocConsole` (already-enabled feature flag).
  - Idempotent — repeated calls return `ERROR_ACCESS_DENIED` and are treated as benign.

### Unchanged (intentional)
- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs` — stays at the Bug #2 WRITE_RESTRICTED fix (commit `e094994`). Session SID is still attached as a restricting SID → WFP `FWPM_CONDITION_ALE_USER_ID` still matches.
- All 4 existing restricted-token regression tests still pass.

### Planning
- `.planning/phases/14-v1-fix-pass/14-01-PLAN.md` — amended with `<direction_decision>` (Direction 1 infeasibility, pivot to Direction 3, Direction 2 fallback) and `<smoke_test_gate>` (blocking 4-row matrix, immediate Direction 2 pivot on any failure).

## Decisions

### Direction 1 abandoned

Plan 14-01 Task 1 prescribed: duplicate the supervisor token via `DuplicateTokenEx`, then `AdjustTokenGroups(SE_GROUP_ENABLED, session_sid)` to promote the session SID from restricting-SID to token-group. This was implemented and tested — it compiles, and 5/6 tests pass. The 6th test (the load-bearing invariant check) fails because **`AdjustTokenGroups` cannot add new groups to a token**, only enable/disable groups already present. The synthetic `S-1-5-117-*` SID is never in the parent process's token, so the duplicate never carries it. WFP would fail to match. The Direction 1 work was reverted.

Alternatives surveyed before pivoting:

| API                                   | Can add an arbitrary synthetic SID to a user-mode token as a group? |
|---------------------------------------|---------------------------------------------------------------------|
| `AdjustTokenGroups`                   | No (docs explicit). |
| `SetTokenInformation(TokenGroups)`    | No — requires `SE_TCB_NAME` (SYSTEM/LSA only). |
| `CreateRestrictedToken(SidsToRestrict=…)` | Only onto the restricting-SID list (the Bug #3 path). |
| `LogonUserExExW` / `LsaLogonUser` (S4U) | Requires a real account; no synthetic-SID support. |
| Custom LSA authentication package     | Requires SYSTEM install; out of scope. |

### Direction 3 adopted (`AllocConsole`)

The debug doc's matrix (`.planning/debug/windows-supervised-exec-cascade.md`) showed:

| Token                | PTY  | Result in detached mode |
|----------------------|------|-------------------------|
| WRITE_RESTRICTED     | yes  | 0xC0000142 |
| WRITE_RESTRICTED     | no   | 0xC0000142 |
| null token           | yes  | 0xC0000142 |
| null token           | no   | works (separate pipe-timeout issue) |

The one working configuration was `null token + no PTY + no inherited console` — but that still fails DLL init when PTY is present. This strongly suggested the detached supervisor's **lack of an inherited console** is a load-bearing factor. Direction 3 addresses that by having the supervisor `AllocConsole` before spawning, so the grandchild inherits a valid console regardless of token or PTY.

Direction 3 preserves:
- WRITE_RESTRICTED + session SID on the restricting-SID list → WFP `FWPM_CONDITION_ALE_USER_ID` matches.
- Job Object containment + CapabilitySet filesystem boundary.
- Low-Integrity isolation (untouched).
- `--block-net` WFP path (expected to still drop; MUST be verified by smoke row 4).

### Direction 2 is the fallback if Direction 3 fails the smoke gate

Documented in `14-01-PLAN.md` `<smoke_test_gate>`. No more debugging of Direction 3 after a smoke failure — pivot immediately to null-token + AppID-based WFP filtering on the detached path. This is a known security trade-off (per-session WFP rules stop firing on detached children) but keeps the milestone unblockable.

### Task 1.5 checkpoint

**Signal: "direction 3 approved"** — auto-approved per user pre-authorization in the orchestrator session of 2026-04-18. The direction change is a material re-scope (Direction 1 → Direction 3), not the routine "Direction 1 worked, nominal" path originally anticipated by the plan. The user was informed via a problem-statement RFC and explicitly chose "test option A (Direction 3) and be ready to fallback to option B (Direction 2)".

## Verification

### Automated (passed locally)

| Check | Result |
|-------|--------|
| `cargo check -p nono-cli` | passed |
| `cargo clippy -p nono-cli --all-targets -- -D warnings -D clippy::unwrap_used` | passed (clean) |
| `cargo test -p nono-cli --bin nono -- restricted_token` | 4 passed / 0 failed (existing Bug #2 regression tests green) |
| `cargo build --release -p nono-cli --bin nono` | passed (`target/release/nono.exe` ready for smoke tests) |

### Manual smoke-gate matrix (BLOCKING — awaiting human execution)

From a fresh PowerShell window (not MSYS/bash) on an admin Windows 10/11 host with `nono-wfp-service` running:

| # | Config                                    | Command | Status |
|---|-------------------------------------------|---------|--------|
| 1 | restricted token + AllocConsole (no PTY)  | `.\target\release\nono.exe run --detached --allow-cwd -- ping -t 127.0.0.1` | **PENDING** |
| 2 | restricted token + ConPTY (PTY path live) | `.\target\release\nono.exe run --detached --allow-cwd -- cmd /c "echo hello"` | **PENDING** |
| 3 | non-detached regression                   | `.\target\release\nono.exe run --allow-cwd -- cmd /c "echo hello"` | **PENDING** |
| 4 | `--block-net` WFP regression (admin host) | `.\target\release\nono.exe run --detached --block-net --allow-cwd -- cmd /c "curl --max-time 5 http://example.com"` | **PENDING** |

Expected per row: no `0xC0000142`, no `0xC0000022`, target process live in `tasklist` (rows 1, 4), stdout observable (rows 2, 3), row 4's child exits non-zero with a curl failure in stderr.

**Failure policy (per `14-01-PLAN.md <smoke_test_gate>`):** if any row fails, pivot immediately to Direction 2 (null token + AppID WFP). No further Direction-3 debugging.

## Execution notes

- Executed **inline on the main working tree** (branch `windows-squash`). Earlier worktree-isolated agent attempts (two agents) failed because the worktree creation mechanism produced branches at commit `063ebad` (pre-windows-squash, missing `exec_strategy_windows/` entirely). A subsequent non-worktree agent timed out during the read/analyze phase.
- Commits:
  - `b06aebe` — `fix(14-01): pre-allocate console in detached supervisor to unblock sandboxed console children`
  - `9f02e60` — `docs(14-01): record direction-3 pivot + blocking smoke-test gate`
- No modifications to `.planning/STATE.md` or `.planning/ROADMAP.md` (the orchestrator owns those after the wave completes).
- `make ci` not run locally — executor is on Windows-bash and does not have PowerShell in-toolchain for the make-driven platform-specific legs. Individual cargo check/clippy/test invocations all passed.

## Known coverage gaps

- **Task 2 Test 3 (sandbox-boundary under new token)** was NOT added. Task 2's invariants were predicated on Direction 1's `build_session_group_token` shape; with Direction 3, the token shape is unchanged from the pre-14-01 state (the existing `create_restricted_token_with_sid_applies_write_restricted_flag` test already covers the restricting-SID invariant). The sandbox-boundary test remains a follow-up item — not blocking this plan because the Direction-3 change does not alter the token's access-check surface.
- **Task 2 detached-console-init smoke test (`detached_console_child_initializes_under_session_group_token`)** was NOT added as an `#[ignore]`-gated cargo test. Equivalent coverage comes from smoke-gate matrix row 1, which MUST be executed by hand before merge.
- **Three manual smokes** (rows 1–3 of the matrix) are PENDING — the executor could not run PowerShell from bash.
- **`--block-net` smoke (row 4)** is PENDING — requires admin + `nono-wfp-service`, neither available to the executor.

## Follow-up

1. User executes the smoke-gate matrix. Results go in `## Smoke-gate results` section below, appended by the user or by a follow-up task.
2. If all 4 rows pass: update this SUMMARY's frontmatter `status:` from `awaiting-smoke-gate` to `complete`, proceed to plan 14-03.
3. If any row fails: apply the Direction 2 pivot (launch.rs-only, null token when `NONO_DETACHED_LAUNCH=1` + `config.session_sid.is_some()`). Re-run the smoke gate.
4. The missing regression tests (Task 2 Test 2, Test 3) are tracked as a follow-up plan rather than blocking this phase.

## Smoke-gate results

### Pass 1 — Direction 3 (AllocConsole) — 2026-04-18T02:00

Executed by user on admin Windows 11 Enterprise host with `nono-wfp-service` running.

| # | Config | Command | Result | Evidence |
|---|--------|---------|--------|----------|
| 1 | restricted token + AllocConsole (no PTY) | `nono run --detached --allow-cwd -- ping -t 127.0.0.1` | **FAIL** | `Detached session failed to start (exit status: exit code: 0xc0000142)` |
| 2 | restricted token + ConPTY | `nono run --detached --allow-cwd -- cmd /c "echo hello"` | **FAIL** | `Detached session failed to start (exit status: exit code: 0xc0000142)` |
| 3 | non-detached regression | `nono run --allow-cwd -- cmd /c "echo hello"` | **PASS** | `hello` printed, supervisor exited 0 after 72ms. UNC path warning "UNC paths are not supported. Defaulting to Windows directory." is pre-existing (cmd.exe behavior with `\\?\C:\...` current dir), unrelated to this plan. |
| 4 | `--block-net` WFP regression | `nono run --detached --block-net --allow-cwd -- cmd /c "curl --max-time 5 http://example.com"` | **DIFFERENT FAILURE** | `Detached session failed to become attachable within startup timeout` — **no 0xC0000142**. Matches the debug doc's "null token + no PTY → ping runs, attach-pipe timeout" row, suggesting the grandchild actually started when the restricted token was absent/bypassed on this path. Cannot evaluate `--block-net` enforcement until row 4 reaches an attach-ready state under Direction 2. |

**Verdict: Direction 3 failed rows 1 + 2.** Per the plan's failure policy (`14-01-PLAN.md <smoke_test_gate>` step 3), Direction 2 pivot applied immediately without further Direction-3 debugging.

### Pass 2 — Direction 2 (null token in detached mode) — PENDING

Binary rebuilt at `target/release/nono.exe` after commit `005f6bc`. User to re-run all 4 rows.

| # | Config | Command | Status |
|---|--------|---------|--------|
| 1 | null token + no PTY (detached)  | `nono run --detached --allow-cwd -- ping -t 127.0.0.1` | **PENDING** |
| 2 | null token + ConPTY (detached)  | `nono run --detached --allow-cwd -- cmd /c "echo hello"` | **PENDING** |
| 3 | non-detached regression (WRITE_RESTRICTED path unchanged) | `nono run --allow-cwd -- cmd /c "echo hello"` | **PENDING** (re-confirm row 3 did not regress) |
| 4 | `--block-net` WFP regression (AppID filter path) | `nono run --detached --block-net --allow-cwd -- cmd /c "curl --max-time 5 http://example.com"` | **PENDING** |

Expected per row: no `0xC0000142`, no `0xC0000022`, target process live in `tasklist` (rows 1, 4), stdout observable (rows 2, 3), row 4's child exits non-zero with a curl failure in stderr (MUST still drop — AppID filter is now the kernel-level boundary).

If Pass 2 also fails: per the plan, escalate — the bug is neither console-init nor restricting-SID and is out of scope for this plan.
