---
phase: 14-v1-fix-pass
plan: 01
completed: 2026-04-18T02:15:00Z
status: escalated-out-of-scope
---

## Outcome

**Plan 14-01 is escalated out of scope for Phase 14.** Both committed fix directions failed the user-executed smoke gate; per the documented failure policy, no further debugging spiral was attempted in this plan. Code changes have been reverted. Bug #3 (detached console grandchild `STATUS_DLL_INIT_FAILED 0xC0000142`) remains open and requires a follow-up phase with a different architectural approach.

**Evolution of the fix attempts:**
1. **Direction 1** (session SID as `TokenGroups` via `AdjustTokenGroups`) — abandoned before commit. Infeasible with user-mode Windows APIs (`AdjustTokenGroups` cannot add new SIDs to a token; `SetTokenInformation(TokenGroups)` requires `SE_TCB_NAME` privilege that a user process does not hold). See commit `9f02e60` (plan amendment) for the detailed API survey.
2. **Direction 3** (`AllocConsole` in detached supervisor; keep WRITE_RESTRICTED + session SID as restricting SID). Committed `b06aebe`, smoked by user 2026-04-18T02:00 — **FAILED rows 1 and 2** with identical `0xC0000142`. AllocConsole alone did not unblock DLL loader init.
3. **Direction 2** (null token in detached mode; non-detached keeps WRITE_RESTRICTED + session SID). Committed `005f6bc`, smoked by user 2026-04-18T02:10 — **FAILED rows 1 and 2** with identical `0xC0000142`. Null-token-in-detached did not unblock either.

Both committed directions were reverted:
- `bd55893` — Revert Direction 2 (005f6bc).
- `1980df5` — Revert Direction 3 (b06aebe).

Launch.rs and restricted_token.rs are back to their pre-14-01 state (matches `dc4c18b` exactly, i.e. still carries the Bug #2 WRITE_RESTRICTED fix from commit `e094994`). No security trade-offs were left on the branch; the `0xC0000142` bug remains unfixed and surfaces only on the detached console-grandchild path.

## Root cause (as reconstructed from the smoke-gate evidence)

The debug doc (`.planning/debug/windows-supervised-exec-cascade.md`) matrix shows that the ONLY working detached-mode configuration is `null token + no PTY`:

| Token                  | PTY  | Exit (detached, ping.exe)          |
|------------------------|------|-------------------------------------|
| Flags=0 restricting SID | Some | 0xC0000022 (Bug #2, fixed in e094994) |
| WRITE_RESTRICTED       | Some | 0xC0000142 |
| WRITE_RESTRICTED       | None | 0xC0000142 |
| null                   | Some | 0xC0000142 |
| null                   | None | ping runs (separate attach-pipe timeout) |

My Direction 2 implementation nulled the token but left the PTY live. Per the matrix that is still `0xC0000142` — exactly what the Pass-2 smoke showed. Fully matching the matrix's working row would require **both** `null token` AND `no PTY in detached mode`. That second part is not a trivial launch.rs flip — PTY in the current supervised_runtime chain is load-bearing for stdout capture and the supervisor-child IPC handshake. Changing it is a genuine architecture decision, not a tactical fix, and thus out of scope for Phase 14.

## Impact on Phase 14 roadmap

- **Roadmap success criterion #1** (detached console-child initializes) — **NOT MET**. Plan 14-01 did not deliver.
- **Phase 13 UAT items dependent on 14-01** — `P05-HV-1`, `P07-HV-3`, `P11-HV-1`, `P11-HV-3` remain `blocked`. Plan 14-03's 2nd-pass UAT will re-verify whatever it can and leave these 4 items in `blocked` status.
- **Roadmap success criteria #2 (14-02) + #3–5 (14-03)** are not affected by 14-01 and can still be closed in this phase.

## Recommended follow-up

A new phase (Phase 15 candidate, or a GSD note pointing to the debug doc) to:
1. Investigate the PTY + restricted-token + detached-supervisor interaction at the ConPTY / StartupInfoEx attribute level.
2. Evaluate whether detached mode can gate its PTY usage on session type (interactive → PTY; `--detached` → no PTY, stdio-only supervisor).
3. Re-test the matrix's `null + no PTY` working row on the current codebase.
4. Consider alternate detached IPC paths that don't require PTY.

This is genuine architecture work, not a code tweak. The user's "no further debugging spiral" rule was correctly enforced: stop in 14-01, document the state, punt to a proper investigation phase.

## Key files

### Reverted (no net change on branch vs dc4c18b)
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — restored to pre-14-01 state. Direction 3's `AllocConsole` call and `ensure_detached_supervisor_console_attached` helper reverted (commit `1980df5`). Direction 2's null-token-in-detached branch reverted (commit `bd55893`).
- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs` — never modified by 14-01 (stays at the Bug #2 WRITE_RESTRICTED fix from `e094994`). All 4 existing regression tests still pass.

### Planning artifacts kept
- `.planning/phases/14-v1-fix-pass/14-01-PLAN.md` — amended (commit `9f02e60`) with `<direction_decision>` (Direction 1 infeasibility analysis) and `<smoke_test_gate>` (the 4-row matrix + failure policy). These stay on branch as documentation of what was tried and why.
- This SUMMARY file — commits `88ae5ae` (initial), `cc9884e` (Pass-1 results), and the current amendment (escalation).

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

### Manual smoke-gate matrix — both passes failed, plan escalated

User executed the 4-row matrix twice on admin Windows 11 Enterprise host.

**Pass 2 re-gate (Direction 2 — null token in detached) — 2026-04-18T02:10**

| # | Config                                          | Result | Evidence |
|---|-------------------------------------------------|--------|----------|
| 1 | null token + ConPTY (detached)                  | **FAIL** | `Detached session failed to start (exit status: exit code: 0xc0000142)` |
| 2 | null token + ConPTY (detached, cmd /c echo)     | **FAIL** | `Detached session failed to start (exit status: exit code: 0xc0000142)` |
| 3 | non-detached (WRITE_RESTRICTED path unchanged)  | **PASS** | `hello` printed, exit 0 in 52ms. UNC path warning is cmd.exe behavior, pre-existing. |
| 4 | `--block-net` (AppID filter path; but WFP service not installed in user's environment) | **N/A** | Failed earlier at WFP registration check: `nono-wfp-service is not registered. Run nono setup --install-wfp-service first`. Not a DLL init failure and does not help evaluate Direction 2's network-enforcement properties. |

Rows 1 and 2 still hit `0xC0000142` under Direction 2 (null token) — consistent with the debug-doc matrix row `null token + Some PTY → 0xC0000142`. Direction 2 as spec'd in `14-01-PLAN.md` did not disable PTY in detached mode, so this was an incomplete match of the matrix's one-working configuration.

**Escalation decision:** per `14-01-PLAN.md <smoke_test_gate>` step 5, stop and escalate — "the bug is not console-init nor restricting-SID but something more fundamental, which is out of scope for this plan." Both code directions reverted.

**What would be needed for a real fix** (out of scope for plan 14-01):
- Also disable PTY allocation in detached mode (change `supervised_runtime.rs` / PTY wiring so detached mode passes `pty: None` to `spawn_windows_child`).
- Or find an alternate detached IPC mechanism that doesn't require ConPTY.
- Or investigate at the `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` / `StartupInfoEx` level why `null token + PTY` still fails DLL init in a `DETACHED_PROCESS` supervisor.

These are genuine architecture changes, not tactical fixes. Deferred to a follow-up phase.

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
