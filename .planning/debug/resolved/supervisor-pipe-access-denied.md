---
slug: supervisor-pipe-access-denied
status: resolved
trigger: Phase 21 Plan 21-05 Task 2 HUMAN-UAT re-run
created: "2026-04-20T22:04:29Z"
updated: "2026-04-21T12:00:00Z"
branch: windows-squash
head_commit: e4c1bfa
related_phase: 21-windows-single-file-grants
related_uat: .planning/phases/18-extended-ipc/18-HUMAN-UAT.md (G-01)
---

# Debug Session: Windows supervisor control pipe ERROR_ACCESS_DENIED on live claude-code flow

## Symptoms

**Expected behavior:**
`C:\Users\omack\Nono\target\release\nono.exe run --profile claude-code -- .\target\release\aipc-demo.exe` launches the child under the supervised runtime. The supervisor process creates a control pipe (`\\.\pipe\nono-nono-cap-<hex>-pipe-<hex>-<hex>`), the detached child connects to it, handshake succeeds, child continues into `aipc-demo.exe` execution.

**Actual behavior:**
After the user answers `y` to the `--allow-cwd` prompt, output shows:
```
  mode supervised (supervisor)
  Applying sandbox...
2026-04-20T21:47:48Z  WARN label guard: path not owned by current user; skipping mandatory label apply (system paths are Medium-IL by default and already readable by Low-IL subjects) path=C:\Windows access=Read
Error: SandboxInit("Failed to connect to Windows supervisor pipe \\\\.\\pipe\\nono-nono-cap-df52a0cb9818081e-pipe-6768c6a4b46d912c-be404f17aa746c70551f857540c28ca9: Access is denied. (os error 5). Ensure the supervisor created the control channel before launching the child.")
```
Child aborts immediately; no further output. aipc-demo.exe never executes.

**Error messages:**
- Primary: `SandboxInit("Failed to connect to Windows supervisor pipe \\.\pipe\nono-nono-cap-<hex>-pipe-<hex>-<hex>: Access is denied. (os error 5). Ensure the supervisor created the control channel before launching the child.")`
- Windows system error: `os error 5` = `ERROR_ACCESS_DENIED`
- Preceding warn (expected, NOT the cause): the new Phase-21 `da25619` ownership-skip warn for `C:\Windows`

**Timeline — never worked:**
This is the first end-to-end run of `--profile claude-code → supervised → aipc-demo.exe` on Windows. Every prior attempt failed at compile time with `WindowsUnsupportedIssueKind::SingleFileGrant` on the `git_config` policy group's 5 single-file read grants. Phase 21 closed those rejection paths (commits `a59e978` / `8c47a6b`) and the inline ownership-skip fix `da25619` closed the downstream `C:\Windows` label-apply regression. Both fixes are on disk; the binary at `target/release/nono.exe` was rebuilt at 2026-04-20T17:05Z and includes them (confirmed — the ownership-skip warn is visible in the output).

**Reproduction (deterministic):**
```
cd C:\Users\omack\scratch\aipc-demo
cargo build --release --bin aipc-demo          # from the aipc-demo project
C:\Users\omack\Nono\target\release\nono.exe run --profile claude-code -- .\target\release\aipc-demo.exe
# answer 'y' to the allow-cwd prompt
# child aborts with ERROR_ACCESS_DENIED on the supervisor pipe connection
```

## Environment

- Host: Windows 11 Enterprise 10.0.26200, user `omack` (unprivileged)
- Branch: `windows-squash` at `a0676dc`
- Binary: `C:\Users\omack\Nono\target\release\nono.exe` (built 2026-04-20T17:05Z)
- Profile: `claude-code` (pulls `git_config`, `system_read_windows`, standard claude-code groups)
- CWD at invocation: `C:\Users\omack\scratch\aipc-demo`
- Demo binary: `.\target\release\aipc-demo.exe` (external project)

## Candidate Hypotheses (from HUMAN-UAT G-01)

1. **AppliedLabelsGuard side-effect on `.cache\claude` / `.claude` staging dirs** — Plan 21-04's guard labels the 3 `r+w` user dirs (`.claude`, `.cache\claude`, `scratch\aipc-demo`) with `NO_EXECUTE_UP` (mask 0x4, ReadWrite mode). If the supervisor stages the detached child binary under `.cache\claude` and tries to execute from there, the Low-IL-labeled directory may block execution or IPC paths.
2. **Phase 11 `CAPABILITY_PIPE_SDDL` DACL gap for Low-IL subjects** — The capability pipe's SDDL includes a mandatory-label SACL `S:(ML;;NW;;;LW)` admitting Low-IL, but may lack a DACL ACE granting `FILE_WRITE_DATA | FILE_READ_DATA` to the Low-IL well-known SID (`S-1-16-4096`). Never hit before because no Low-IL child ever connected.
3. **Silent supervisor startup failure** — Supervisor is launched detached (Phase 15 anonymous-pipe stdio). If supervisor startup itself errors out (e.g., cannot write to a labeled `.cache\claude`), the pipe is never created. Child then races a non-existent pipe and gets ERROR_ACCESS_DENIED (Windows returns ACCESS_DENIED on non-existent named pipes with restricted ACLs in some cases — actually usually ERROR_FILE_NOT_FOUND, which makes hypothesis #3 less likely than #1 or #2).

Ranking by plausibility (pre-investigation): **#1 > #2 > #3**.

## Current Focus

- cycle: 3 (SDDL correct at bind — deeper access-check subtlety)
- hypothesis: H2-restricted-sid fix landed (`3c68377`) but verification FAILED (see Resolution.verification). Three candidates for why the dynamic ACE is not reaching the pipe DACL:
  - **C1** — `session_sid` arrives as `None` at `bind_low_integrity_with_session_sid` despite source chain looking right.
  - **C2** — `bind_low_integrity_with_session_sid` is reached with `Some(sid)` but fails before `CreateNamedPipeW` (e.g. `validate_session_sid_for_sddl` rejects, or SDDL has a syntax error). Weakened because a missing-pipe path would yield `ERROR_FILE_NOT_FOUND`, and we see `ERROR_ACCESS_DENIED`.
  - **C3** — two different pipe bindings (e.g. a legacy `bind_low_integrity` path also binds at the same kernel name). Less likely given single-process single-call architecture.
- test: Add targeted diagnostic tracing to expose which candidate is live. Logs will surface `session_sid.is_some()` at every hop in the chain AND the full SDDL string actually built. Commit separately from any future fix.
- expecting: After rebuild + re-run with `RUST_LOG=info`, the log will show one of:
  - No supervisor-side log at all → supervisor thread never spawned OR never reached `bind_low_integrity_with_session_sid` (narrows C1 to a gating check at the supervisor layer).
  - `bind_low_integrity_with_session_sid` entered with `session_sid=none` → C1 confirmed, chain is broken at some layer between `execution_runtime.rs:315` and the thread body.
  - `bind_low_integrity_with_session_sid` entered with `session_sid=some(prefix=S-1-5-117-...)` AND SDDL log shows the `(A;;0x0012019F;;;<sid>)` ACE → the DACL is correct at bind time. That would refute all three candidates and force a new hypothesis (perhaps DACL is correct but Windows is applying a different check, or the child's effective restricting SID differs from the one plumbed here).
  - `build_capability_pipe_sddl` log missing an ACE or showing validation error before SDDL log → C2 confirmed.
- next_action: Add four `tracing::info!` call sites (enumerated below), rebuild release binary, surface CHECKPOINT to orchestrator for user to re-run the repro.
- instrumentation_sites:
  1. `crates/nono/src/supervisor/socket_windows.rs::bind_low_integrity_with_session_sid` — entry log: `session_sid` presence + first-16-char prefix only.
  2. `crates/nono/src/supervisor/socket_windows.rs::build_capability_pipe_sddl` — entry log: full SDDL string built (DACL structure is not secret).
  3. `crates/nono-cli/src/exec_strategy_windows/supervisor.rs::start_capability_pipe_server` — entry log: `self.session_sid.is_some()`.
  4. `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` spawned thread (inside the `move ||` at supervisor.rs:461) — pre-bind log: rendezvous_path + session_sid presence.
- security_constraint: never log full SID — first 16 chars only (`S-1-5-117-...`). SDDL ok to log in full.
- fail_closed_constraint: instrumentation is `tracing::info!` only — no new `?`, no panics, no control-flow changes.

## Evidence

- timestamp: 2026-04-20T21:47:48Z — `label guard: path not owned by current user; skipping mandatory label apply` fired for `C:\Windows access=Read`. Confirms ownership-skip (`da25619`) is active and NOT the cause.
- timestamp: 2026-04-20T21:47:48Z — Pipe name in error: `\\.\pipe\nono-nono-cap-df52a0cb9818081e-pipe-6768c6a4b46d912c-be404f17aa746c70551f857540c28ca9`. Pattern: `nono-nono-cap-<session_id>-pipe-<request_id>-<nonce>`. Consistent with Phase 11 capability-pipe naming.
- timestamp: 2026-04-20T21:47:48Z — Sandbox bring-up ran for ~30s (21:47:18 "Applying sandbox..." to 21:47:48 error). Long enough for supervisor to spawn and pipe to be created — argues against hypothesis #3.
- timestamp: 2026-04-20T21:47:18Z — `Applying sandbox...` message appears. This is printed AFTER `compile_filesystem_policy` but BEFORE `AppliedLabelsGuard::snapshot_and_apply`. So the guard ran after this print but before the child tries to connect.
- 2026-04-20T22:20:00Z — **Pipe-name origin confirmed.** `execution_runtime.rs:211` sets `windows_cap_pipe_path = std::env::temp_dir().join(format!("nono-cap-{}.pipe", flags.session.session_id))`. `socket_windows.rs:pipe_name_from_rendezvous_path` sanitizes the basename (`nono-cap-<id>.pipe` → `nono-cap-<id>-pipe`) and formats `\\.\pipe\nono-{safe_leaf}-{short_hash}-{nonce_hex}` = `\\.\pipe\nono-nono-cap-<id>-pipe-<hash>-<nonce>` ← matches the observed pipe name.
- 2026-04-20T22:22:00Z — **Error site localized.** The message `"Failed to connect to Windows supervisor pipe <name>: Access is denied. (os error 5). Ensure the supervisor created the control channel before launching the child."` is produced ONLY at `socket_windows.rs:1039-1043` inside `connect_named_pipe`, **after** `CreateFileW` returns with an error that is neither `ERROR_PIPE_BUSY` nor `ERROR_FILE_NOT_FOUND`. The rendezvous-file parse step has its own distinct error messages. So: rendezvous file parsed OK; pipe name resolved OK; the OS returned `ERROR_ACCESS_DENIED` specifically on `CreateFileW`.
- 2026-04-20T22:25:00Z — **H1 eliminated.** `AppliedLabelsGuard` only labels three user dirs (`.claude`, `.cache\claude`, `scratch\aipc-demo`) with `NO_EXECUTE_UP` (mask 0x4). None of the pipe handling touches those paths. The rendezvous file is under `std::env::temp_dir()` (default `C:\Users\omack\AppData\Local\Temp`, NOT under any labeled dir). The named pipe lives in the kernel pipe namespace, not filesystem. AppliedLabelsGuard cannot cause `ERROR_ACCESS_DENIED` on pipe CreateFile.
- 2026-04-20T22:28:00Z — **H2 refined to H2-restricted-sid and confirmed.** `CAPABILITY_PIPE_SDDL = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;LW)"`. The SACL labels the pipe Low-IL (so MIC's no-write-up for Low-IL subjects is a no-op — object IL == subject IL). The DACL grants GA to SY, BA, OW. The child's token owner is the unprivileged user, which matches OW, so a NORMAL access check would grant GA. BUT: `restricted_token.rs:94-107` creates the child token with `CreateRestrictedToken(..., WRITE_RESTRICTED, ..., 1, &sid_restrict, ...)` where `sid_restrict` is the synthetic per-session SID `S-1-5-117-<guid>` (`restricted_token.rs:22-32`). Per the file's own docstring (lines 72-93), `WRITE_RESTRICTED` causes every WRITE access check to be performed twice — once normally, once against the restricting-SID set. `CreateFileW(pipe, GENERIC_READ|GENERIC_WRITE, ...)` is a write access. The pipe DACL has NO ACE matching `S-1-5-117-<guid>` — so the restricted pass fails and the OS returns `ERROR_ACCESS_DENIED`.
- 2026-04-20T22:30:00Z — **Why never seen before.** Phase 11 integration test `test_bind_low_integrity_roundtrip` (`socket_windows.rs:1270-1332`) runs BOTH sides inside the same unrestricted test process — the client never carries a WRITE_RESTRICTED + session-SID token. Phase 13 UAT exercised `cmd /c "echo hello"` children that never touched capability expansion. Phase 21 UAT with `aipc-demo.exe` is the first flow where a WRITE_RESTRICTED+session-SID child actually performs `CreateFileW(pipe, GENERIC_READ|GENERIC_WRITE)`.
- 2026-04-20T22:32:00Z — **Config confirmation.** `execution_runtime.rs:315` ALWAYS populates `config.session_sid = Some(exec_strategy::generate_session_sid())` for Windows. `launch.rs:1144-1149` uses that to call `create_restricted_token_with_sid(sid)` when NOT on the detached path. The repro command has no `--detached` flag, so `is_windows_detached_launch()` returns false and the WRITE_RESTRICTED branch is taken.

- 2026-04-21T00:30:00Z — **Cycle 3: standalone repro tool landed as commit `b83a961`.** Built `crates/nono-cli/examples/pipe-repro.rs` — a single binary that in parent mode creates the pipe + restricted token + spawns itself as `--child` via `CreateProcessAsUserW`, and in child mode calls `CreateFileW(GENERIC_READ | GENERIC_WRITE)`. Mirrors production's `create_restricted_token_with_sid` verbatim. Accepts `--sddl-template` with `{sid}` placeholder for rapid SDDL iteration.
- 2026-04-21T00:32:00Z — **V1 (baseline = current production SDDL) reproduces the bug.** Running `target/release/examples/pipe-repro.exe --sddl-template "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)(A;;0x0012019F;;;{sid})S:(ML;;NW;;;LW)"` produces `CreateFileW: FAILED raw_err=5 (Access is denied. (os error 5))`. Confirms the repro tool faithfully reproduces the runtime failure in isolation, validating all subsequent iterations.
- 2026-04-21T00:32:00Z — **V2 (GA mnemonic for session SID ACE): FAIL.** `(A;;GA;;;{sid})` still denied. Refutes the "mask mapping" hypothesis — `GA` gets mapped to `FILE_ALL_ACCESS = 0x001F01FF` at access-check time, so the mnemonic vs. explicit hex isn't the issue.
- 2026-04-21T00:33:00Z — **V3 (explicit `0x001F01FF` = FILE_ALL_ACCESS for session SID ACE): FAIL.** Also denied. Conclusively eliminates any "mask-too-narrow" hypothesis — even FILE_ALL_ACCESS on the restricting-SID ACE doesn't satisfy the second-pass check alone.
- 2026-04-21T00:33:00Z — **V5 (Everyone `WD` alone, no session-SID ACE): FAIL.** Rules out "WD implicitly satisfies the restricting-SID check." Confirms that the restricted-SID ACE is necessary — just not sufficient.
- 2026-04-21T00:34:00Z — **V4 (session-SID ACE AND Everyone `WD` ACE): PASS.** `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)(A;;GA;;;{sid})(A;;GA;;;WD)S:(ML;;NW;;;LW)` → `CreateFileW: SUCCESS`. This is the first passing variant. Deterministic (confirmed via `V1-rerun` / `V4-rerun` back-to-back). Order-independent (`V4b-reorder` with WD before session-SID also passes).
- 2026-04-21T00:34:00Z — **V8 (AU = Authenticated Users alone, no session-SID ACE): FAIL.** Confirms the "group SID alone" hypothesis is wrong.
- 2026-04-21T00:34:00Z — **V9 (`AU` + session-SID): PASS.** V10 (`BU` = Built-in Users + session-SID): **PASS.** V11 (literal user-SID `S-1-5-21-...-1002` + session-SID): **FAIL.** Pattern refines: the second ACE must be for a GROUP SID in the token's enabled groups (WD, AU, BU all matched via `SE_GROUP_MANDATORY` attrs), NOT the user SID itself and NOT the `OW` Owner Rights virtual SID.
- 2026-04-21T00:35:00Z — **Diagnostic token dump.** Added `PIPE_REPRO_DUMP_TOKEN=1` env-gated helper that prints `TokenGroups` and `TokenRestrictedSids`. Output for V2: TokenGroups = 25 entries including user SID, `WD`, `AU`, `BU`, multiple domain groups, and **logon SID `S-1-5-5-0-8459283` with attrs=0xc0000007 (SE_GROUP_LOGON_ID | SE_GROUP_ENABLED | ...).** TokenRestrictedSids = 1 entry (our session SID, attrs=0x07). This proves the session-SID made it into the token's RestrictedSids correctly and is not being silently dropped.
- 2026-04-21T00:35:00Z — **V14 (logon SID `S-1-5-5-0-8459283` + session-SID): PASS.** This is the narrow fix. The logon SID is unique per interactive-logon session, narrower than `WD` (Everyone) or `AU` (all authenticated users on the machine) — only processes in the SAME logon session have this SID in their token. When the interactive logon ends, this SID is gone.
- 2026-04-21T00:35:00Z — **V15 (logon SID alone, no session-SID ACE): FAIL.** Confirms both ACEs are required together.
- 2026-04-21T00:36:00Z — **Root cause located (empirical).** The second-pass (restricted-SID) access check in Windows 11 26200 requires NOT just an ACE whose SID matches a SID in RestrictedSids, but ALSO requires the DACL to contain an ACE for a group SID present in the token's enabled groups with `SE_GROUP_MANDATORY` attribute (`WD`, `AU`, `BU`, or the logon SID). The existing `(A;;GA;;;OW)` baseline ACE does NOT satisfy this — `OW` is the Owner Rights virtual SID (`S-1-3-4`), which is resolved against the object owner at access-check time and apparently is NOT considered equivalent to the user's enabled groups during the restricted-SID access check. Microsoft's documentation for `CreateRestrictedToken` with `WRITE_RESTRICTED` does not describe this co-requirement explicitly; it was discovered by systematic SDDL iteration. The cleanest narrow fix is to embed the LOGON SID (retrieved at runtime from `TokenGroups` by matching `SE_GROUP_LOGON_ID`) as an additional ACE; this is narrower than OW (unique per logon session).

- 2026-04-20T23:55:00Z — **Cycle 2 instrumentation landed as commit `da8e13d`.** Four `tracing::info!` sites added, all confined to the two files I was asked to touch (no control-flow change, no new `?`):
  1. `crates/nono/src/supervisor/socket_windows.rs::bind_low_integrity_with_session_sid` entry — logs `session_sid_present` + first-16-char prefix (`S-1-5-117-...`) when present, plus rendezvous `path`. Event: `"bind_low_integrity_with_session_sid entry"`.
  2. `crates/nono/src/supervisor/socket_windows.rs::build_capability_pipe_sddl` — logs the FULL resulting SDDL string (DACL is not secret) plus `session_sid_present`. Event: `"build_capability_pipe_sddl result"`. Refactored to compute `sddl` first, then log, then `Ok(sddl)` — no behavior change (tests `build_capability_pipe_sddl_none_matches_constant` and `build_capability_pipe_sddl_some_embeds_ace_before_sacl` both exercise byte-identical output path).
  3. `crates/nono-cli/src/exec_strategy_windows/supervisor.rs::start_capability_pipe_server` entry — logs `self.session_sid.is_some()` + `session_id`. Event: `"start_capability_pipe_server entry"`.
  4. `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` spawned-thread pre-bind — logs `session_sid.is_some()` + `rendezvous_path.display()` + `session_id`, immediately BEFORE the call to `bind_low_integrity_with_session_sid`. Event: `"capability pipe server thread pre-bind"`. Confirms the `move ||` closure captured the expected values.
  - Security constraint honored: full SID never logged anywhere — only first-16-char prefix at site 1. SDDL string is logged in full at site 2 per the instrumentation-scope spec (DACL structure is the whole point of the diagnostic).
  - Rebuilt release binary via `cargo build --release --bin nono`; build succeeded in 6m26s; binary timestamp `Apr 20 21:40` (verified via `ls -la target/release/nono.exe`).
  - Awaiting checkpoint response: user re-runs repro with `RUST_LOG=info` to produce a trace with the four new events, which will disambiguate C1 vs C2 vs C3.

## Eliminated

- **H1 (AppliedLabelsGuard)** — the guard does not touch the temp-dir rendezvous file nor the kernel pipe namespace. Scope of labeled paths is limited to user dirs that no part of pipe creation / connect traverses.
- **H3 (silent supervisor failure)** — the error message text is specific to `CreateFileW` returning a non-busy non-not-found error. If the pipe did not exist, the child would hit `ERROR_FILE_NOT_FOUND` and take the retry branch with `WaitNamedPipeW`, producing the distinct "Timed out waiting for Windows supervisor pipe" message. We see the "Access is denied" variant, so the pipe exists and the OS denied the open.

## Resolution

- root_cause: **The Windows 11 (26200) second-pass DACL access check for a `WRITE_RESTRICTED` token requires MORE than just an ACE matching a SID in the token's `RestrictedSids` list. Empirically (cycle 3), it also requires an ACE whose SID is a group SID in the token's enabled groups with `SE_GROUP_MANDATORY` — and specifically, `OW` (the Owner Rights virtual SID `S-1-3-4`) does NOT satisfy this co-requirement. The pre-cycle-3 fix added the session-SID ACE (cycle 2, commit `3c68377`), which was necessary but insufficient: the DACL had `(A;;GA;;;OW)` only on the "token groups" side, and Windows did not treat that as fulfilling the second-pass requirement for a token group ACE. Without such an ACE, CreateFileW from the WRITE_RESTRICTED child returns ERROR_ACCESS_DENIED even when the restricting-SID ACE is correctly present. Microsoft's `CreateRestrictedToken` documentation does not describe this co-requirement; it was identified by systematic SDDL iteration (13 variants) in the `crates/nono-cli/examples/pipe-repro.rs` harness.**
- fix: **Two-stage landing on `windows-squash`:**
  - **Stage 1 (cycle 2) — commit `3c68377`** `fix(supervisor): grant per-session restricting SID FILE_GENERIC_RW on capability pipe DACL`. Added the per-session restricting-SID ACE `(A;;0x0012019F;;;<session_sid>)` to `build_capability_pipe_sddl`. Necessary but insufficient on Windows 11 26200 (second-pass DACL check still failed).
  - **Stage 2 (cycle 3) — commit `938887f`** `fix(supervisor): append logon-SID ACE to capability pipe DACL for WRITE_RESTRICTED access`. Added a second ACE `(A;;0x0012019F;;;<logon_sid>)` where the logon SID is retrieved at runtime via the new `current_logon_sid()` helper which queries the current process's `TokenGroups` for the entry with `SE_GROUP_LOGON_ID`. The logon SID (`S-1-5-5-X-Y`) is unique per interactive logon session and is strictly narrower than `WD` Everyone, `AU` Authenticated Users, or `BU` Built-in Users — any of which would also work but grant unwanted access to processes outside the logon session. Logon SID lookup failure is fail-closed (returns `NonoError::SandboxInit`).
  - **Stage 3 (companion fix) — commit `e4c1bfa`** `fix(aipc-01): initialize Winsock before socket handle reconstruction`. With the pipe DACL fixed, the `aipc-demo` end-to-end re-run advanced through the supervisor-pipe handshake and surfaced a SECOND latent bug in Phase 18 Plan 18-04's child SDK: `reconstruct_socket_from_blob` called `WSASocketW(FROM_PROTOCOL_INFO)` without first calling `WSAStartup(2.2)`. Unit tests passed because they called `WSAStartup` themselves. Fix adds a `OnceLock<()>`-guarded `WSAStartup(MAKEWORD(2,2), ...)` before `WSASocketW`. Not strictly part of the supervisor-pipe DACL investigation, but landed in the same push because it was the immediate-next failure on the end-to-end UAT path after the DACL fix unblocked the pipe connect. Tracked separately in `18-HUMAN-UAT.md` Key-Decision B.
- verification: **PASSING end-to-end on live `nono run --profile claude-code -- .\target\release\aipc-demo.exe` (2026-04-20 UAT re-run).** After all three commits landed and `target/release/nono.exe` was rebuilt, the live flow advanced past the supervisor-pipe connect (the original symptom) AND past the Winsock-uninitialized panic, and successfully brokered 4 of 5 AIPC handle types end-to-end (Event, Mutex, Pipe, Socket — all received duplicated handles in the child process). The 5th broker type (JobObject) failed with a separate bug tracked as G-03 in the Phase 18 HUMAN-UAT carry-forward, which is unrelated to supervisor-pipe access-denied. Library-level regression test also PASSING: `cargo test --release --lib -p nono supervisor::socket::tests::capability_pipe_admits_restricted_token_child_with_session_sid` bind-spawns a real capability pipe via `SupervisorSocket::bind_low_integrity_with_session_sid`, constructs the identical production-shape `WRITE_RESTRICTED` token, impersonates it on the current thread via `ImpersonateLoggedOnUser`, and calls `SupervisorSocket::connect` — which executes the same `CreateFileW(pipe, GENERIC_READ | GENERIC_WRITE)` that failed in production. The test previously relied on SDDL-structure assertions only (false positive — passed with the broken production fix). Additionally verified via the `pipe-repro` harness: both `--sddl-template "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)(A;;0x0012019F;;;{sid})(A;;0x0012019F;;;S-1-5-5-0-<logon>)S:(ML;;NW;;;LW)"` (matching production shape) and the synthetic V4/V9/V10/V14 variants documented in Evidence PASS, while V1/V2/V3/V5/V11/V15 FAIL — reproducing the production bug in isolation and confirming the logon-SID ACE is the missing ingredient.
- files_changed:
  - `crates/nono/src/supervisor/socket_windows.rs` — `build_capability_pipe_sddl` now appends two ACEs (session-SID + logon-SID); added `current_logon_sid()` helper; replaced SDDL-structure regression test with an end-to-end impersonation + `CreateFileW` test; updated `build_capability_pipe_sddl_some_embeds_ace_before_sacl` test to assert the new structural shape. Reverted cycle-2 `tracing::info!` instrumentation.
  - `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — reverted cycle-2 `tracing::info!` instrumentation (both call sites in `start_capability_pipe_server` and the spawned-thread pre-bind).
  - `crates/nono-cli/examples/pipe-repro.rs` — NEW; standalone WRITE_RESTRICTED + SDDL-template + CreateFileW harness used for empirical iteration across 13 SDDL variants (V1..V15 in the Evidence block).
  - `crates/nono/src/supervisor/aipc_sdk.rs` — `reconstruct_socket_from_blob` now calls `OnceLock<()>`-guarded `WSAStartup(MAKEWORD(2,2), ...)` before `WSASocketW(FROM_PROTOCOL_INFO)` (commit `e4c1bfa`; companion fix, not strictly supervisor-pipe scope — surfaced by the same end-to-end UAT re-run).

## Follow-up

New Phase 18 issues discovered during the 2026-04-20 end-to-end UAT re-run (after supervisor-pipe was unblocked) that are OUTSIDE this debug session's scope and will be addressed by a future Phase 18.1 `/gsd-plan-phase 18.1 --gaps`:

- **G-02** — CONIN$ approval prompts render generic fields instead of the D-04 per-kind templates that Plan 18-02 was supposed to emit. Captured in `18-HUMAN-UAT.md` § Gaps.
- **G-03** — `OpenJobObjectW` fails with `os error 2` (ERROR_FILE_NOT_FOUND) on the JobObject broker path; investigation needed on whether broker should CREATE-if-not-exists or demo should pre-create. Captured in `18-HUMAN-UAT.md` § Gaps.
- **G-04** — After G-03 failure, supervisor returned `granted=true` with `grant=None` (empty ResourceGrant), a protocol violation. Child surfaced this as `SandboxInit("supervisor granted but returned no ResourceGrant")`. Captured in `18-HUMAN-UAT.md` § Gaps.

These are independent bugs on the broker-handler / prompt-formatter side, not regressions of the supervisor-pipe DACL fix landed here.
