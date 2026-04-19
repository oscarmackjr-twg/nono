---
phase: 17-attach-streaming
status: ready-for-planning
gathered: 2026-04-19
---

# Phase 17: Attach-Streaming (ATCH-01) — Context

**Gathered:** 2026-04-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Closes the v2.1 attach-streaming gap on Windows detached sessions. After this phase:
- `nono attach <id>` against a detached Windows session streams child stdout live, writes stdin to the child, and cleanly detaches via Ctrl-]d.
- The Phase 15 `0xC0000142` fix (PTY-disable + null-token + AppID WFP on detached path) is preserved with **zero regressions** to the 5-row Phase 15 smoke matrix.
- Detached child output is captured to the per-session log file from spawn time, so `nono logs` works whether or not anyone ever attaches.

**Out of scope (intentionally):**
- ConPTY mid-run hand-off on the detached path. Phase 15 evidence (`STATUS_DLL_INIT_FAILED`) and REQUIREMENTS.md ATCH-01 design unknown both say this is infeasible. Anonymous pipes are the structural solution.
- Native terminal resize on detached sessions (no `ResizePseudoConsole` without ConPTY). Documented limitation; users who need full TUI fidelity use `nono shell` or non-detached `nono run`.
- Multi-client attach (broadcast / shared sessions). Single-attach is preserved; second client gets a friendly busy error.
- Cross-platform changes. Phase 17 is Windows-only — Unix detached sessions already stream via the existing socket path.

</domain>

<decisions>
## Implementation Decisions

### Stream Architecture

- **D-01:** **Anonymous pipes only** on the Windows detached path. The supervisor sets `STARTUPINFOW.hStdInput / hStdOutput / hStdError` to anonymous pipes (`CreatePipe`) when spawning the detached child. No ConPTY allocation on the detached path — preserves the Phase 15 `0xC0000142` fix structurally. Locked: no flag, no opt-in path, no mid-run ConPTY hand-off.
- **D-02:** Non-detached path is **unchanged**. `nono shell` and non-detached `nono run` continue to allocate ConPTY via `pty_proxy_windows::open_pty()`. The Phase 15 `should_allocate_pty` gate (`supervised_runtime.rs:88-94`) stays as the single decision point.

### Pipe Lifecycle

- **D-03:** **Pipes created at child spawn**, not lazily on first attach. `spawn_windows_child` (in `exec_strategy_windows/launch.rs`) creates inheritable anonymous pipe pairs and binds them to `STARTUPINFOW`. Supervisor side keeps the supervisor-end handles (stdin write, stdout read, stderr read) for the lifetime of the child.
- **D-04:** Supervisor's `start_logging` worker (`exec_strategy_windows/supervisor.rs:463-546`) is extended so the `pty_output_read == 0` early-return is replaced with a "read from anonymous-pipe stdout/stderr → write to log file (always) AND mirror to active attach client (when present)" branch. The existing PTY branch is untouched.
- **D-05:** Supervisor's `start_data_pipe` (`supervisor.rs:760-837`) is extended in mirror fashion: the `pty_input_write == 0` early-return is replaced with a "read from `\\.\pipe\nono-data-<id>` → write to anonymous-pipe stdin" branch. Pipe naming and `active_attachment: Mutex<Option<SendableHandle>>` semantics unchanged.

### Resize

- **D-06:** **No resize on detached sessions.** Child sees whatever pipe-default geometry the OS reports (typically 80×24 or "no console"). The supervisor startup `tracing::info!` line (currently at `supervisor.rs:476-479`) is updated to communicate the new state: streaming is supported, resize is not. Document the gap in `docs/cli/attach.md` (or equivalent — to be confirmed in research) so users reach for `nono shell` / non-detached `nono run` when they need vim/htop.
- **D-07:** REQUIREMENTS.md acceptance criterion #3 (`Terminal resize ... propagates to the child via ResizePseudoConsole`) is **explicitly downgraded** to "documented limitation on detached path". Plan must record this deviation in its `<acceptance_criteria>` block and `<deferred_ideas>` section. Resize lives in a future phase if/when there is a credible mid-run ConPTY architecture (likely never on Windows).

### Multi-Attach Policy

- **D-08:** **Single-attach, second client rejected with a friendly error.** Existing `active_attachment: Arc<Mutex<Option<SendableHandle>>>` semantics preserved (the named pipe accepts one connection at a time). When a second `nono attach <id>` is run while another client holds the pipe, surface a clean message like: `Session <id> is already attached. Use 'nono detach <id>' to release the existing client first.` Implement by translating `ERROR_PIPE_BUSY` (or our connect failure) at the client side in `session_commands_windows::run_attach` (`session_commands_windows.rs:391-400`).

### Smoke-Gate Scope (must all PASS for ship)

- **G-01** Live ping streaming: `nono run --detached --allow-cwd -- ping -t 127.0.0.1` followed by `nono attach <id>` shows live ping replies in the attach client. (REQUIREMENTS.md acceptance #1)
- **G-02** Bidirectional cmd.exe: `nono run --detached --allow-cwd -- cmd.exe` followed by `nono attach <id>` accepts stdin commands and returns stdout output. (REQUIREMENTS.md acceptance #2)
- **G-03** Detach sequence: Ctrl-]d disconnects the attach client without killing the detached child; subsequent `nono attach <id>` reconnects to the still-live session and resumes streaming. (REQUIREMENTS.md acceptance #4 + the existing Phase 13 detach UX)
- **G-04** Phase 15 5-row matrix unchanged: all 5 rows from `15-02-SUMMARY.md § 4-row smoke-gate matrix (+ Row 5)` still PASS — no regression in detached banner, fast-exit, non-detached, network-blocked, or `logs/inspect/prune` shapes. (REQUIREMENTS.md acceptance #5)

### Claude's Discretion

- Internal helper structure: how `start_logging` and `start_data_pipe` are refactored to share the new pipe-bridging code with their existing PTY branches. Choose readability over abstraction; minimum viable structural change is fine.
- Buffer sizes for the anonymous-pipe ↔ named-pipe bridge. The existing PTY paths use `[u8; 4096]` — match that unless research surfaces a reason to differ.
- Whether to split stderr into its own log/relay channel or merge it into stdout (the non-detached PTY path merges them; matching that for consistency is recommended).
- Test scaffolding: integration tests can spawn `cmd /c "echo X"` style commands. Live `ping -t` verification is a manual smoke step, not a CI test (matching Phase 15's smoke-gate structure).
- Whether the multi-attach friendly error is added in this phase or filed as a follow-up "polish" task. Recommend including it — it's a 1-screen change at the point of failure.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & roadmap
- `.planning/REQUIREMENTS.md` § ATCH-01 (lines 173-197) — full ATCH-01 spec including the design-unknown the planner must address (re-attach-vs-pipe choice — locked here as **pipes only** per D-01).
- `.planning/ROADMAP.md` § "Phase 17: Attach-Streaming (ATCH)" (lines 61-67) — phase scope statement and the "investigation + implementation + smoke gate" plan-count guidance.
- `.planning/PROJECT.md` § Active (v2.1) → ATCH-01 (line 61) — milestone-level statement.

### Phase 15 prior art (load-bearing — DO NOT regress)
- `.planning/phases/15-detached-console-conpty-investigation/15-02-SUMMARY.md` — direction-b implementation, the 4-row smoke matrix + Row 5, and the security-acceptance verdict table (LI / kernel-network waivers scoped to the detached path).
- `.planning/phases/15-detached-console-conpty-investigation/15-03-SUMMARY.md` — Phase 15 closure record + the explicit "`nono attach` output streaming for detached sessions on Windows — deferred to v2.1+" gap (line 102) that this phase closes.
- `.planning/debug/resolved/windows-supervised-exec-cascade.md` — the full investigation that produced the `0xC0000142` root-cause and direction-b. Phase 17 must not invalidate any conclusion in this doc.

### Implementation surfaces (downstream files)
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` § `start_logging` (lines 463-546) and § `start_data_pipe` (lines 760-837) — extension points for D-04 / D-05.
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `spawn_windows_child` and the `is_windows_detached_launch()` helper. Pipe creation + `STARTUPINFOW` wiring lives here.
- `crates/nono-cli/src/supervised_runtime.rs` § `should_allocate_pty` (lines 88-94) — the Phase 15 PTY gate. Read-only here; D-01 keeps this unchanged.
- `crates/nono-cli/src/pty_proxy_windows.rs` (133 lines) — `PtyPair` and `open_pty()`; reusable shape, not extended in this phase.
- `crates/nono-cli/src/session_commands_windows.rs` § `run_attach` (lines 363-460) — client-side scrollback + bidirectional pipe + Ctrl-]d. D-08 friendly-busy error attaches at lines 391-400.

### Project standards
- `CLAUDE.md` — workspace coding standards (no `unwrap`, `NonoError` propagation, `#[cfg(target_os="windows")]` discipline, EnvVarGuard pattern for tests, DCO sign-off on every commit).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`pty_proxy_windows::open_pty()`** — already creates anonymous pipes via `CreatePipe` as part of building the `PtyPair` (`pty_proxy_windows.rs:42-61`). The pipe-creation half can be lifted into a shared helper used by both the ConPTY path (existing) and the new pipe-only path (this phase) — but only if the abstraction is small. If extracting feels forced, just inline `CreatePipe` calls in `launch.rs`.
- **`SendableHandle`** wrapper + `active_attachment: Arc<Mutex<Option<SendableHandle>>>` — battle-tested under Phase 15. New pipe-bridging threads must use the same primitive so multi-attach behavior stays identical.
- **`ManuallyDrop<File::from_raw_handle(...)>`** pattern in `start_logging` (`supervisor.rs:508-509`) — preserved for the new pipe branch so handle ownership matches the existing PTY branch.
- **`session::session_log_path`** + log-file open dance (`supervisor.rs:484-506`) — works the same regardless of source (PTY or pipe). Reuse verbatim.
- **`create_secure_pipe`** for `\\.\pipe\nono-session-<id>` and `\\.\pipe\nono-data-<id>` (called at `supervisor.rs:559` and `:787`) — already SDDL'd `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)`. Phase 17 inherits this; no security wiring changes.

### Established Patterns

- Phase 15's **`should_allocate_pty` single-gate-point** convention: any cross-platform-affecting decision lives in one helper with `cfg!(target_os = "windows")` + cross-platform unit tests using the `EnvVarGuard` + `lock_env` pattern. Phase 17 must keep that gate untouched and add a *parallel* "should-create-pipes" decision — likely just inverting `should_allocate_pty` on Windows (if not allocating PTY, allocate pipes). Don't add a second cross-platform gate; the Windows detached branch is structurally distinct.
- **`tracing::info!` startup log line** at `supervisor.rs:476-479` is the user-visible documentation surface for behavior on the detached path. Update this line as part of this phase so `RUST_LOG=info` users immediately know what changed.
- **Bidirectional pipe bridging** via two threads (one for read, one for write) is the established pattern in `start_data_pipe` (`supervisor.rs:801-833`). Mirror it for the new pipe→pipe bridge.

### Integration Points

- **CLI**: no new CLI flags. Phase 17 does not change `nono attach` invocation; it only makes the existing command work on detached Windows sessions.
- **Session record**: `SessionRecord` shape unchanged. `attachment: SessionAttachment::Detached` already differentiates; no new fields needed.
- **Logging / observability**: the existing `start_logging` worker becomes the always-on log writer for detached sessions too — no new `nono logs` plumbing.
- **Security model**: zero changes. SDDL on the named pipes is unchanged; no new tokens, no new WFP wiring. The Job Object + filesystem sandbox + AppID-WFP set is exactly what Phase 15 shipped.
- **Cross-platform**: zero changes outside `*_windows.rs` files. `D-21`-style Windows-invariance discipline (per Phase 20 convention) means `crates/nono/src/sandbox/` is byte-identical, no `target_os = "linux"` / `target_os = "macos"` branches touched.

</code_context>

<specifics>
## Specific Ideas

- **The "no output at all → big improvement" framing from REQUIREMENTS.md** is the right benchmark. Anonymous pipes give us live stdout streaming, bidirectional stdin, scrollback replay, and clean detach — that's a complete user-visible win even without resize.
- **Buffer reuse**: 4096-byte buffers on both sides of the bridge match the existing `start_logging` PTY worker. Keeping the size identical avoids surprising back-pressure differences between PTY and pipe sessions.
- **Stderr handling**: the non-detached PTY path collapses stderr into stdout (single stream from ConPTY). For consistency, the detached path should merge stderr into the same log/relay channel so `nono attach`'s output is visually identical between PTY and pipe sessions. Implement either by passing the same handle for `hStdOutput` and `hStdError`, or by spawning a second reader thread that writes into the same log/relay sink.

</specifics>

<deferred>
## Deferred Ideas

- **Native terminal resize on detached sessions** (`ResizePseudoConsole`) — explicitly not feasible without ConPTY, which is structurally blocked by Phase 15's `0xC0000142` fix. Future work would require a fundamentally new architecture (kernel mini-filter? ConPTY hand-off via a console-proxy process? Win11-only API?). Document as v3.0 candidate; do NOT add to v2.1 backlog as it currently stands.
- **Multi-client attach (broadcast / shared sessions)** — interesting for pair-programming and observability but adds a second pipe, a fan-out thread, and lifecycle complexity. Single-attach with a friendly busy error covers the v2.1 use case. File as a v2.2 candidate if user demand surfaces.
- **Force-takeover semantics** (2nd attach boots the 1st) — useful when terminal sessions die without sending Ctrl-]d. Possible follow-up after we see how often the friendly busy error gets hit. Not in v2.1 scope.
- **Best-effort `COLUMNS`/`LINES` env-var injection at spawn** — marginal value; most Windows tools don't honor them. If user feedback after Phase 17 ships indicates `ls` / `less` users care, add as a quick task.
- **REQUIREMENTS.md acceptance criterion #3 (resize via `ResizePseudoConsole`)** — downgraded to "documented limitation" per D-07. The criterion as stated is structurally infeasible on the detached path; Phase 17 plan must record the downgrade and update REQUIREMENTS.md (or add a § note) as part of the closeout commit.

</deferred>

---

*Phase: 17-attach-streaming*
*Context gathered: 2026-04-19*
