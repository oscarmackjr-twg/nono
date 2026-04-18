---
phase: 15-detached-console-conpty-investigation
plan: 01
status: complete
executed: 2026-04-18
direction_chosen: b
handoff_to: 15-02
key_commit: 0a0c794
---

# Plan 15-01 — Summary

## Outcome

**Status:** complete (ready for 15-02)
**Direction chosen:** b (gated PTY-disable + null-token + AppID WFP on the Windows detached path)
**Primary commit:** `0a0c794` — `docs(15-01): record investigation matrix + direction decision for Phase 15`

## What was done

1. Applied two `#[cfg(all(target_os = "windows", debug_assertions))]`-gated investigation patches:
   - `supervised_runtime.rs` line ~117 — forced `pty_pair = None` on Windows debug builds.
   - `launch.rs` line ~840 — forced `h_token = null_mut()` when `NONO_DETACHED_LAUNCH=1`.
2. Built debug `target/debug/nono.exe`, ran the `restricted_token` regression suite (4/4 green).
3. Ran the three-row matrix from PowerShell on the user's Windows 11 Enterprise host:

   | Row | Config | Command | Result |
   |-----|--------|---------|--------|
   | B | WRITE_RESTRICTED + no PTY, detached | `nono run --detached --allow-cwd -- ping -t 127.0.0.1` | **FAIL** `0xC0000142` |
   | C | WRITE_RESTRICTED + no PTY gate, non-detached | `nono run --allow-cwd -- cmd /c "echo hello"` | **PASS** `hello`, exit 0 |
   | D | null + no PTY, `NONO_DETACHED_LAUNCH=1` | `NONO_DETACHED_LAUNCH=1 nono run --detached --allow-cwd -- ping -t 127.0.0.1` | **PASS** ping replies streamed live |

4. Reverted both investigation patches to HEAD (no production code shipped from 15-01).
5. Updated `.planning/debug/windows-supervised-exec-cascade.md` with `## Phase 15-01 Investigation` and `## Direction Decision` sections.
6. Committed the debug doc only (staging constraint respected — no pre-existing WIP files swept in).

## Evidence basis for direction-b

- Row B's failure eliminates direction-a: disabling PTY alone with the WRITE_RESTRICTED token still yields `0xC0000142`.
- Row D's success confirms the matrix's only working row (null + None PTY) reproduces on current HEAD `6f4de70`.
- Row C's pass proves the PTY gate does not regress the non-detached supervised path.

## Plan 15-02 action list (copied from Direction Decision section)

1. **`supervised_runtime.rs` ~line 117** — On Windows, allocate PTY only when `session.interactive_pty`. Non-Windows keeps `detached_start || interactive_pty`.
2. **`exec_strategy_windows/launch.rs` ~line 838** — Before token selection, when `cfg(target_os = "windows")` and `NONO_DETACHED_LAUNCH=1`, set `h_token = null_mut()` and skip restricted/LI token creation.
3. **`exec_strategy_windows/supervisor.rs` ~line 405** — Keep `start_logging` Ok(()) for no-PTY; add a log line noting "nono attach will not stream child output on Windows detached sessions (v2.1+ enhancement)".
4. **`pty_proxy_windows.rs`** — No code changes in 15-02. The `open_detached_stdout_pipe` helper from 15-02-PLAN is explicitly deferred (attach-streaming for detached is v2.1+ enhancement).
5. **`restricted_token.rs`** — No changes.
6. **Tests** — Unit tests for: Windows detached produces `pty_pair = None` when `interactive_pty = false`; `spawn_windows_child` selects null token when `NONO_DETACHED_LAUNCH=1` is set alongside `session_sid: Some`. Existing `restricted_token` tests continue to pass.
7. **Commit waiver** — 15-02 Task 1 commit body must include `Security-Waiver:` trailers for the two waived properties (Low-IL isolation on detached; session-SID WFP on detached).

## What 15-02 must NOT do

- Do NOT modify `create_restricted_token_with_sid` — the non-detached path remains correct.
- Do NOT add new `#[allow(dead_code)]` attributes; remove dead items instead.
- Do NOT thread a new `is_detached` field through `ExecConfig` — use `NONO_DETACHED_LAUNCH` env var.
- Do NOT commit any investigation patches (already reverted).

## Security properties verdict

| # | Property | Direction-b verdict |
|---|----------|---------------------|
| 1 | Sandbox filesystem boundary | Preserved |
| 2 | Job Object containment | Preserved |
| 3 | Low-Integrity isolation | Waived on detached path only (Job Object + filesystem sandbox remain primary) |
| 4 | Kernel network identity | Waived (per-session SID) / preserved (kernel identity via AppID WFP); requires `nono-wfp-service` running |

Waivers are strictly scoped to the Windows detached code path (`cfg(target_os = "windows")` + `NONO_DETACHED_LAUNCH=1` or `session.detached_start`). Non-detached and non-Windows retain full WRITE_RESTRICTED + session-SID + ConPTY.

## CI gate (Task 1 regression)

- `cargo test -p nono-cli --bin nono -- restricted_token` — **PASS**, 4/4.
- `cargo build -p nono-cli --bin nono` — **PASS** clean after revert.
- Full `make ci` was NOT re-run from 15-01; 15-02 Task 3 runs the full CI gate on the production fix.

## Known remaining gaps

- `nono attach` output streaming against a detached Windows session — deferred to v2.1+. 15-02 documents this in the supervisor log line but does not implement it.
- Row D was run with `NONO_DETACHED_LAUNCH=1` forced in the outer shell, so the supervisor was not actually detached via the double-launch path. 15-02 Smoke-Gate Row 1 (`nono run --detached`) validates the end-to-end detached path with the production fix applied.
