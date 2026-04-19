# Phase 17: Attach-Streaming (ATCH-01) — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in `17-CONTEXT.md` — this log preserves the alternatives considered.

**Date:** 2026-04-19
**Phase:** 17-attach-streaming
**Areas discussed:** Stream architecture, Pipe lifecycle (launch-time vs lazy), Resize handling, Smoke gate scope (+ multi-attach policy clarifier)

---

## Gray-Area Selection

User selected **all 4** offered areas.

| Option | Description | Selected |
|--------|-------------|----------|
| Stream architecture | Anonymous pipes vs ConPTY mid-run hand-off vs hybrid behind a flag | ✓ |
| Launch-time vs lazy IPC | Pipes at child spawn vs lazily on first attach | ✓ |
| Resize handling | Full ResizePseudoConsole vs no-resize fallback vs both | ✓ |
| Smoke gate scope | Phase 15 5-row matrix unchanged + N attach-specific rows | ✓ |

---

## Stream Architecture

| Option | Description | Selected |
|--------|-------------|----------|
| Anonymous pipes only (Recommended) | Spawn child with STARTUPINFOW std handles set to anonymous pipes; supervisor bridges pipes ↔ named pipe ↔ attach client. Zero ConPTY risk. No native resize. | ✓ |
| Pipes by default + ConPTY behind --interactive-attach flag | Default to pipes; opt-in flag triggers experimental ConPTY hand-off on first attach. High risk of re-tripping 0xC0000142; adds a feature surface that may never stabilize. | |
| ConPTY mid-run hand-off (no fallback) | Lazy ConPTY allocation on first attach + console re-parenting. Highest fidelity if it works; Phase 15 evidence says it won't. Fails REQUIREMENTS.md's fallback guidance. | |

**User's choice:** Anonymous pipes only.
**Notes:** Locks the architecture before research begins. ConPTY mid-run hand-off is structurally off the table for the detached path; the Phase 15 `0xC0000142` evidence is conclusive.

---

## Pipe Lifecycle

| Option | Description | Selected |
|--------|-------------|----------|
| At child spawn (Recommended) | Pipes created in `spawn_windows_child` and bound to STARTUPINFOW. Supervisor's `start_logging` reads stdout pipe → log file (always) AND mirrors to attach client. | ✓ |
| Lazy (first attach allocates) | Child spawns with stdout/stderr → log file directly. First attach triggers a ConPTY-style hand-off. Re-introduces Phase 15 mid-run handle redirection problem on a different path. | |

**User's choice:** At child spawn.
**Notes:** Keeps `nono logs` working for detached sessions whether or not anyone ever attaches. Mirrors the existing non-detached supervisor's log/attach split structurally.

---

## Resize Handling

| Option | Description | Selected |
|--------|-------------|----------|
| No resize, document the gap (Recommended) | Child sees fixed 80×24. Document in `docs/cli/attach.md` and the supervisor startup `tracing::info!`. REQUIREMENTS.md acceptance #3 becomes a documented partial-pass. | ✓ |
| Inject WINDOW_BUFFER_SIZE_RECORD via supervisor on resize | Attach client sends resize events; supervisor has no API to forward them since the child has no console. Would require attaching child to a console (Phase 15 territory). | |
| Best-effort env-var hint at spawn | Set COLUMNS/LINES from the launching terminal at run-time. Static, won't reflow on attach-time resize. Marginal value. | |

**User's choice:** No resize, document the gap.
**Notes:** Forces an explicit downgrade of REQUIREMENTS.md acceptance criterion #3. Plan must record this deviation and surface a note in CHANGELOG / `docs/cli/attach.md`.

---

## Smoke Gate Scope

| Option | Description | Selected |
|--------|-------------|----------|
| Live ping streaming | `nono run --detached -- ping -t 127.0.0.1` + `nono attach` shows live replies. Acceptance #1. | ✓ |
| Bidirectional cmd.exe | `nono run --detached -- cmd.exe` + `nono attach` accepts stdin and returns stdout. Acceptance #2. | ✓ |
| Detach sequence cleanly unparents | Ctrl-]d disconnects without killing child; subsequent attach reconnects. Acceptance #4. | ✓ |
| Phase 15 5-row matrix unchanged | All 5 rows from `15-02-SUMMARY.md` still PASS. Acceptance #5. | ✓ |

**User's choice:** All four (multi-select).
**Notes:** Acceptance criterion #3 (resize) intentionally absent — folded into the documented-limitation track per the Resize decision above.

---

## Multi-Attach Policy (clarifier)

| Option | Description | Selected |
|--------|-------------|----------|
| Reject 2nd with clear error (Recommended) | Translate `ERROR_PIPE_BUSY` into a friendly "session is already attached, run `nono detach` first". Zero new code; matches Unix supervisor behavior. | ✓ |
| Boot the previous client | 2nd attach forcibly disconnects 1st. Friendlier when terminals die without Ctrl-]d. Adds handshake + race risk. | |
| Read-only broadcast for additional clients | 1st client read+write; subsequent clients stdout-only. Useful for pair-programming. Large surface for v2.1. | |

**User's choice:** Reject 2nd with clear error.
**Notes:** Force-takeover and broadcast are interesting follow-ups; deferred to v2.2 candidate list if user demand surfaces.

---

## Claude's Discretion

User did not explicitly say "you decide" on any structural decision. The following implementation details are Claude's discretion (recorded in `17-CONTEXT.md § Claude's Discretion` for the planner):

- Internal helper structure for sharing pipe-bridging code between PTY and pipe branches.
- Buffer sizes (default to 4096 to match PTY path).
- Whether stderr gets its own log/relay channel or merges into stdout (recommend merge for consistency).
- Test scaffolding (integration vs manual smoke gate split — match Phase 15 structure).
- Whether the multi-attach friendly error lands in this phase or as a follow-up.

---

## Deferred Ideas

Captured in `17-CONTEXT.md § Deferred Ideas`:

- Native terminal resize on detached sessions (v3.0 candidate; structurally blocked).
- Multi-client attach (broadcast / shared) — v2.2 candidate.
- Force-takeover (2nd attach boots 1st) — v2.2 candidate.
- Best-effort `COLUMNS`/`LINES` env-var injection at spawn — quick task if demand surfaces.
- REQUIREMENTS.md acceptance #3 downgrade to "documented limitation" — handled in Phase 17 closeout.
