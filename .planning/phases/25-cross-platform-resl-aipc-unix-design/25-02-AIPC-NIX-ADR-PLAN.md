---
phase: 25-cross-platform-resl-aipc-unix-design
plan: 02
type: design
wave: 1
depends_on: []
files_modified:
  - docs/architecture/aipc-unix-futures.md
  - .planning/PROJECT.md
autonomous: true
requirements:
  - AIPC-NIX-01
tags:
  - aipc
  - unix
  - adr
  - design
  - cross-platform
tdd: false
risk: low
threat_model_summary:
  - "No threat surface — design-only ADR; zero source-code changes; zero runtime behavior delta"

must_haves:
  truths:
    - "`docs/architecture/aipc-unix-futures.md` exists and contains a 6-row decision table (one row per HandleKind 0..5: File, Socket, Pipe, JobObject, Event, Mutex) with a Yes/No verdict + 1-paragraph rationale per row."
    - "ADR records the locked decision: HandleKinds 0–2 (File / Socket / Pipe) admit Unix backends via Unix-domain socket + `SCM_RIGHTS` file-descriptor passing; HandleKinds 3–5 (JobObject / Event / Mutex) are Windows-only by design — Linux equivalents (cgroup v2, eventfd, pthread mutex) don't broker the same way."
    - "For each 'No' verdict, the ADR names the alternate Unix mechanism users should reach for instead: JobObject → cgroup v2 (already shipping in Plan 25-01); Event → `pipe(2)` for one-shot signaling; Mutex → `flock(2)` for cross-process advisory file locks."
    - "PROJECT.md cross-links the ADR from § Upstream Parity Process (or the existing § that references `docs/architecture/`) — verified by `grep -n 'aipc-unix-futures' .planning/PROJECT.md` returning at least 1 match."
    - "ADR includes a 'Reversibility' section (1 paragraph) noting that the decision can be revisited if/when AIPC G-04 wire-protocol tightening lands in v2.4+, or if Linux gains primitives that broker the same way as Windows JobObject/Event/Mutex (none currently do)."
    - "ADR is decision-only: no API surface sketch, no implementation pseudocode, no RFC-style design diagrams; total length 250–400 lines."
    - "No source-code changes — `git diff --stat HEAD` after the plan executes shows ONLY `docs/architecture/aipc-unix-futures.md` (new file) and `.planning/PROJECT.md` (modified); zero `.rs` / `.toml` / `Cargo.lock` / `Makefile` deltas."
  artifacts:
    - path: "docs/architecture/aipc-unix-futures.md"
      provides: "AIPC Unix futures ADR (Decision-only): 6-row HandleKind decision table + per-kind rationale + alternate-mechanism mapping for the 3 No verdicts + Reversibility clause"
      contains: "AIPC Unix Futures"
      contains: "SCM_RIGHTS"
      contains: "JobObject"
      contains: "Reversibility"
      min_lines: 250
    - path: ".planning/PROJECT.md"
      provides: "cross-link from § Upstream Parity Process (or existing architecture-docs section) to the new ADR"
      contains: "aipc-unix-futures"
  key_links:
    - from: ".planning/PROJECT.md § Upstream Parity Process"
      to: "docs/architecture/aipc-unix-futures.md"
      via: "Markdown link from PROJECT.md to the ADR file"
      pattern: "docs/architecture/aipc-unix-futures\\.md"
    - from: "docs/architecture/aipc-unix-futures.md § References"
      to: ".planning/PROJECT.md (AIPC HandleKind discriminator pinning decision in key-decisions table)"
      via: "back-reference to the originating decision context"
      pattern: "PROJECT\\.md"
    - from: "docs/architecture/aipc-unix-futures.md § References"
      to: "Phase 23 RejectStage discussion"
      via: "back-reference to the AIPC enforcement-stage taxonomy that motivated the cross-platform question"
      pattern: "Phase 23"
---

<objective>
Deliver the AIPC Unix Futures ADR (REQ-AIPC-NIX-01) — a decision-only architecture record at `docs/architecture/aipc-unix-futures.md` documenting which AIPC `HandleKind` discriminants admit Unix backends and which are Windows-only by design.

Purpose: Phase 18 + 18.1 shipped AIPC handle brokering as a Windows-only subsystem (Job Objects, Events, Mutexes, Sockets, Pipes, Files routed across the supervisor IPC boundary by `HandleKind` u32 discriminant). Going into v2.4 cross-platform AIPC planning, the foundational question is not "how do we port AIPC to Unix?" but "which HandleKinds *can* be ported, which *cannot*, and what should Unix users reach for in the cases where they cannot?" Without this ADR locked in, every v2.4 Unix-AIPC discussion will re-litigate the same six rows of the decision table and three alternate-mechanism mappings. This plan freezes the decision now (v2.3) so v2.4 implementation can build against it instead of beside it.

Output:
- `docs/architecture/aipc-unix-futures.md` — NEW ADR file (250–400 lines), decision-only, 6-row HandleKind table, per-kind rationale, alternate-mechanism mapping for the 3 "No" verdicts, Reversibility clause, References.
- `.planning/PROJECT.md` — single cross-link line added under § Upstream Parity Process (or the existing § that references `docs/architecture/`); idempotent (skip if already present).

**ACCEPTANCE LOCK:** The ADR is decision-only. No API surface sketch. No implementation pseudocode. No RFC-style design diagrams. The total file length is 250–400 lines. Each HandleKind row gets exactly: a one-word verdict (Yes / No), a 3–5 sentence rationale, and (for "No" rows) one sentence pointing at the Unix alternate mechanism. Anything beyond that scope is **out of scope for v2.3** and deferred to v2.4+ implementation phases.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/REQUIREMENTS.md
@.planning/phases/25-cross-platform-resl-aipc-unix-design/25-CONTEXT.md
@.planning/phases/25-cross-platform-resl-aipc-unix-design/25-RESEARCH.md
@.planning/phases/25-cross-platform-resl-aipc-unix-design/25-01-RESL-NIX-PLAN.md

<interfaces>
<!--
  AIPC HandleKind u32 discriminant table — verbatim from Phase 18 + 18.1 wire protocol.
  This is the canonical mapping the ADR's decision table must mirror exactly. Discriminants
  are pinned via const assertions in `aipc_sdk.rs`; do not renumber.

  Per Phase 18.1 PROJECT.md key-decisions entry: discriminator values 0..5 are append-only;
  new HandleKinds get the next sequential discriminant (6, 7, ...) and never reuse holes.
-->

# HandleKind discriminants (Windows AIPC wire protocol, Phase 18 + 18.1):
#   0 = File         (HANDLE to a kernel File object)
#   1 = Socket       (SOCKET / WSA-compatible)
#   2 = Pipe         (Anonymous or named pipe HANDLE)
#   3 = JobObject    (HANDLE to a Job Object — process containment)
#   4 = Event        (HANDLE to a kernel Event — cross-process signaling)
#   5 = Mutex        (HANDLE to a kernel Mutex — cross-process locking)

# Unix mechanism mapping (locked decision — this ADR records it):
#   0 = File      → Yes; already cross-platform (FDs are FDs).
#   1 = Socket    → Yes; Unix-domain socket + SCM_RIGHTS ancillary message FD passing.
#   2 = Pipe      → Yes; Unix-domain socket + SCM_RIGHTS (passes anonymous-pipe FD).
#   3 = JobObject → No;  cgroup v2 (Plan 25-01) is the alternate, but it's a different shape — not handle-brokerable.
#   4 = Event     → No;  pipe(2) for one-shot signaling; eventfd is process-local and not brokerable equivalently.
#   5 = Mutex     → No;  flock(2) for cross-process advisory locks; pthread mutexes don't broker via FD.

# ADR file location convention: `docs/architecture/{topic}.md`.
# Note: the directory `docs/architecture/` does NOT yet exist in the fork — Task 1 mkdir -p first.
</interfaces>

<source_audit_summary>
- REQ-AIPC-NIX-01 (REQUIREMENTS.md lines 71–73 — ADR at docs/architecture/aipc-unix-futures.md, design-only, no code, cross-linked from PROJECT.md and CONTEXT D-04 footnote) → Tasks 1, 2
- HandleKind discriminant table (0..5) — verbatim from Phase 18 + 18.1 wire protocol; pinned via const assertions in aipc_sdk.rs → Task 1 decision table rows
- Locked verdicts (File/Socket/Pipe = Yes; JobObject/Event/Mutex = No) — frozen by user in plan-time decision, recorded by this ADR → Task 1 verdict column
- Alternate mechanisms (cgroup v2 / pipe(2) / flock(2)) — locked at plan-time → Task 1 alternate-mechanism mapping section
- PROJECT.md cross-link convention (§ Upstream Parity Process pattern from Phase 24-02 D-15) → Task 2
- Reversibility clause referring to AIPC G-04 wire-protocol tightening (deferred to v2.4 backlog, pre-existing) → Task 1 Reversibility section
</source_audit_summary>
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Draft the AIPC Unix Futures ADR</name>
  <files>docs/architecture/aipc-unix-futures.md</files>
  <read_first>
    - .planning/REQUIREMENTS.md § AIPC-NIX-01 (lines 67–75) — ADR location, design-only constraint, cross-link target
    - .planning/PROJECT.md — locate the AIPC HandleKind discriminator pinning entry in the key-decisions table (anchor for the ADR's References section back-link)
    - .planning/phases/25-cross-platform-resl-aipc-unix-design/25-RESEARCH.md (if present) — any cross-platform AIPC notes the user collected before plan-time
  </read_first>
  <action>
The directory `docs/architecture/` does NOT exist in the fork. Run `mkdir -p docs/architecture` before writing the file.

Create `docs/architecture/aipc-unix-futures.md` as a decision-only ADR. Total length target: 250–400 lines. Do NOT exceed 400 lines. The ADR is a *record* of a decision already made by the user at plan-time; it is not a design exploration document.

**Required ADR structure (in order):**

```markdown
# AIPC Unix Futures

**Status:** Accepted
**Date:** 2026-04-29
**Phase:** 25 (v2.3 Cross-Platform RESL + AIPC Unix Design)
**Requirement:** REQ-AIPC-NIX-01

## Context

(1–2 paragraphs, ~150–250 words total.)

Phase 18 + 18.1 shipped AIPC (Agent IPC) handle brokering as a Windows-only subsystem. Sandboxed agents request access to Windows kernel objects — Files, Sockets, Pipes, Job Objects, Events, Mutexes — via a u32 `HandleKind` discriminant carried over the supervisor IPC channel. The supervisor brokers the actual `HANDLE` across the trust boundary using `DuplicateHandle` with reduced rights.

Going into v2.4 cross-platform AIPC planning, the foundational question is which `HandleKind` discriminants admit Unix backends and which do not. Two of the six are obvious in either direction (File trivially yes, JobObject obviously no), but the middle four — Socket, Pipe, Event, Mutex — each require a deliberate verdict because Unix has *something* in the neighborhood for each, but only Sockets and Pipes admit a *handle-brokering* shape (FD passing via Unix-domain sockets with `SCM_RIGHTS`).

This ADR records the locked decision so v2.4 implementation phases can build *against* it rather than re-litigate it.

## Decision Table

| Discriminant | HandleKind  | Unix backend? | Mechanism / Alternate                                                  |
|--------------|-------------|---------------|------------------------------------------------------------------------|
| 0            | File        | Yes           | Already cross-platform; FDs are FDs                                    |
| 1            | Socket      | Yes           | Unix-domain socket + `SCM_RIGHTS` ancillary FD passing                 |
| 2            | Pipe        | Yes           | Unix-domain socket + `SCM_RIGHTS` (passes anonymous-pipe FD)           |
| 3            | JobObject   | No            | Alternate: cgroup v2 (Plan 25-01) — different shape, not brokerable    |
| 4            | Event       | No            | Alternate: `pipe(2)` for one-shot signaling                            |
| 5            | Mutex       | No            | Alternate: `flock(2)` for cross-process advisory locks                 |

(The table mirrors the discriminant ordering pinned by const assertions in `crates/nono/src/supervisor/aipc_sdk.rs`. Discriminants are append-only — see PROJECT.md § Key Decisions, AIPC HandleKind discriminator pinning entry.)

## Per-HandleKind Rationale

### HandleKind 0: File — Yes

(3–5 sentences explaining why File is trivially cross-platform: file descriptors and Windows file HANDLEs both represent kernel-mediated access to filesystem objects; on Unix, FD passing is the established primitive; Windows already brokers File HANDLEs via `DuplicateHandle`; the AIPC abstraction maps cleanly. No new Unix mechanism is needed beyond what already exists in the broader SCM_RIGHTS-based Socket/Pipe brokers.)

### HandleKind 1: Socket — Yes

(3–5 sentences. Sockets are the classic SCM_RIGHTS use case on Unix; passing a SOCKET across the supervisor IPC boundary on Linux/macOS is a Unix-domain socket sendmsg with `cmsg(SCM_RIGHTS)`; the receiver's recvmsg yields a usable FD. The AIPC wire protocol's discriminant + payload shape maps onto sendmsg/recvmsg cleanly. The fork already uses tokio's UnixStream in cli_bootstrap; the broker layer would extend this with the ancillary-message dance.)

### HandleKind 2: Pipe — Yes

(3–5 sentences. Anonymous pipes on Unix are a `pipe(2)` pair of FDs; passing one end across a Unix-domain socket via SCM_RIGHTS is identical to the Socket case. Named pipes on Linux are FIFOs, also FD-based, also brokerable. The asymmetry to call out: Windows distinguishes anonymous pipe HANDLEs from named pipe HANDLEs at the WinAPI level, but on Unix both reduce to FDs over SCM_RIGHTS — the AIPC HandleKind=2 discriminant covers both shapes and the Unix backend collapses them.)

### HandleKind 3: JobObject — No (Windows-only by design)

(3–5 sentences. Job Objects are a Windows-specific process-containment primitive — they enforce per-process-tree resource limits (memory, CPU, handle count, UI restrictions) at kernel level via `AssignProcessToJobObject`. Linux's nearest equivalent — cgroup v2 — is conceptually similar (kernel-enforced resource limits over a process group) but is *not handle-brokerable*: cgroups are referenced by filesystem path under `/sys/fs/cgroup/`, not by FD/HANDLE that can be duplicated across a trust boundary. The supervisor model requires the broker to *hand* a child a constrained reference; cgroups require the broker to *write* the child's PID into a `cgroup.procs` file, which is a different control flow entirely. macOS has no equivalent at all (no cgroup analog; sandbox profiles fill a different role).)

**Alternate Unix mechanism:** cgroup v2 — already shipping in Phase 25 Plan 25-01. Not handle-brokerable, but achieves the equivalent process-containment outcome via a different shape (path-based control rather than HANDLE-based brokering).

### HandleKind 4: Event — No (Windows-only by design)

(3–5 sentences. Windows kernel Events are a cross-process signaling primitive — one process signals (`SetEvent`), another process waits (`WaitForSingleObject`), and the kernel mediates. The closest Unix primitive — `eventfd(2)` — is process-local and not brokerable across `fork()` boundaries in the way Windows Events are; while an eventfd FD *can* technically be passed via SCM_RIGHTS, the receiver doesn't gain the same multi-waiter cross-process semantics that `WaitForMultipleObjects` provides on Windows. The right Unix idiom for cross-process one-shot signaling is a `pipe(2)`: writer closes its end (or writes a byte), reader's `read()` returns. This is handle-brokerable — Pipe is HandleKind 2 already — so users wanting Event-like semantics get them via the Pipe broker plus a one-byte protocol.)

**Alternate Unix mechanism:** `pipe(2)` for one-shot signaling, brokered via the existing Pipe (HandleKind 2) backend.

### HandleKind 5: Mutex — No (Windows-only by design)

(3–5 sentences. Windows kernel Mutexes are cross-process locks — `WaitForSingleObject` acquires, `ReleaseMutex` releases, the kernel mediates ownership and recursive-acquisition semantics. POSIX has two related primitives: pthread mutexes (process-local unless allocated in shared memory with `PTHREAD_PROCESS_SHARED`, and even then not brokerable via FD) and `flock(2)` advisory file locks (cross-process, FD-based, brokerable via SCM_RIGHTS through the File HandleKind). Process-shared pthread mutexes don't fit the AIPC broker model because the lock state lives in shared memory rather than in a kernel object referenced by HANDLE/FD. `flock(2)` does fit — the lock is associated with the open file description, which is what an FD is — and so cross-process locking on Unix is achieved via the existing File (HandleKind 0) broker plus an `flock(LOCK_EX)` call, not via a new Mutex HandleKind.)

**Alternate Unix mechanism:** `flock(2)` advisory file locks on a broker-passed File FD (HandleKind 0).

## Alternate Mechanisms (Summary)

For the three "No" verdicts, Unix users reach for the following primitives instead. None of these require new AIPC HandleKind discriminants — they ride on existing primitives or sit outside the broker channel entirely.

| Windows HandleKind | Unix alternate    | Brokerable via AIPC? | Phase / Plan reference |
|--------------------|-------------------|----------------------|------------------------|
| JobObject (3)      | cgroup v2         | No (path-based)      | Phase 25 Plan 25-01    |
| Event (4)          | `pipe(2)` + byte  | Yes, via HandleKind 2 (Pipe) | This ADR (no new code) |
| Mutex (5)          | `flock(2)` on FD  | Yes, via HandleKind 0 (File) | This ADR (no new code) |

The implication for v2.4+ implementation: a Unix AIPC backend needs *only three* HandleKind handlers (File, Socket, Pipe), not six. JobObject/Event/Mutex requests from a sandboxed agent on Unix will return a structured "not supported on this platform; use {alternate}" diagnostic — not a silent failure, and not a cross-platform-shimmed mock.

## Reversibility

(1 paragraph, ~100 words.)

This decision can be revisited if and when AIPC G-04 (wire-protocol compile-time tightening, currently deferred to the v2.4 backlog) lands — G-04 may reshape the discriminant table in ways that affect Unix backend feasibility. The decision should also be revisited if Linux gains a primitive that brokers JobObject/Event/Mutex shapes the way Windows does today (none currently do; cgroup v2 is the closest and remains path-based). Until either of those holds, the verdicts above are stable. Re-opening this ADR requires updating both this file's Status field (Accepted → Superseded) and the cross-link in `.planning/PROJECT.md` § Upstream Parity Process.

## References

- `.planning/PROJECT.md` § Key Decisions — AIPC HandleKind discriminator pinning entry (Phase 18.1 origin)
- `.planning/REQUIREMENTS.md` § AIPC-NIX-01 — this ADR's source requirement
- Phase 18 + 18.1 SUMMARY files — original AIPC handle-brokering implementation context
- Phase 23 RejectStage discussion — AIPC enforcement-stage taxonomy that motivated the cross-platform question
- Phase 25 Plan 25-01 — Linux RESL via cgroup v2 (the alternate mechanism for HandleKind 3)
- AIPC G-04 (deferred, v2.4 backlog) — wire-protocol compile-time tightening; potential reversibility trigger
- `crates/nono/src/supervisor/aipc_sdk.rs` — discriminant pinning via const assertions (read-only reference; no changes in this plan)
```

**Style guidance:**
- Each per-HandleKind subsection: 3–5 sentences. Hard cap. No diagrams. No code blocks larger than a 6-row Markdown table.
- Voice: declarative ADR, present tense for the decision ("HandleKind 0 admits a Unix backend because..."), past tense for the originating context ("Phase 18 shipped AIPC as Windows-only because...").
- Do NOT speculate about implementation. The ADR is silent on *how* the Socket/Pipe brokers will be implemented in v2.4 — that is explicitly out of scope.
- Do NOT include "future work" sections beyond Reversibility. The "What about ...?" questions belong in v2.4 design phases, not this ADR.

**Verification (Task 1 only):**
- File exists at `docs/architecture/aipc-unix-futures.md`.
- Decision table has exactly 6 rows: `grep -E '^\| (File|Socket|Pipe|JobObject|Event|Mutex) \|' docs/architecture/aipc-unix-futures.md | wc -l` returns 6.
- Total length 250–400 lines: `wc -l docs/architecture/aipc-unix-futures.md` is in `[250, 400]`.
- Required sections present: `grep -cE '^## (Context|Decision Table|Per-HandleKind Rationale|Alternate Mechanisms|Reversibility|References)$' docs/architecture/aipc-unix-futures.md` returns 6.
  </action>
  <verify>
    <automated>test -f docs/architecture/aipc-unix-futures.md && [ "$(grep -cE '^\| (File|Socket|Pipe|JobObject|Event|Mutex) \|' docs/architecture/aipc-unix-futures.md)" = "6" ] && [ "$(wc -l < docs/architecture/aipc-unix-futures.md)" -ge 250 ] && [ "$(wc -l < docs/architecture/aipc-unix-futures.md)" -le 400 ] && [ "$(grep -cE '^## (Context|Decision Table|Per-HandleKind Rationale|Alternate Mechanisms|Reversibility|References)$' docs/architecture/aipc-unix-futures.md)" = "6" ]</automated>
  </verify>
  <done>ADR file exists at the locked path, has exactly 6 HandleKind rows, length is 250–400 lines, all six required H2 sections present (Context, Decision Table, Per-HandleKind Rationale, Alternate Mechanisms, Reversibility, References), Status field reads "Accepted".</done>
</task>

<task type="auto" tdd="false">
  <name>Task 2: Cross-link the ADR from PROJECT.md</name>
  <files>.planning/PROJECT.md</files>
  <read_first>
    - .planning/PROJECT.md — locate `## Upstream Parity Process` section (added in Phase 24-02). If it does not exist (e.g., Phase 24 hasn't merged), fall back to the existing § that references `docs/architecture/` or the AIPC HandleKind discriminator pinning entry in the key-decisions table.
  </read_first>
  <action>
Add a single cross-link line to `.planning/PROJECT.md` pointing at the new ADR. Idempotent: if the link `docs/architecture/aipc-unix-futures` is already present anywhere in PROJECT.md (`grep -q 'aipc-unix-futures' .planning/PROJECT.md`), skip the edit and exit Task 2 cleanly.

**Preferred insertion point:** under `## Upstream Parity Process` (added by Phase 24-02), append a new bullet at the end of the section:

```markdown
- **AIPC Unix futures** — see [docs/architecture/aipc-unix-futures.md](../docs/architecture/aipc-unix-futures.md) for the locked decision on which AIPC HandleKinds admit Unix backends (File / Socket / Pipe = yes via SCM_RIGHTS; JobObject / Event / Mutex = Windows-only by design).
```

**Fallback insertion point** (if `## Upstream Parity Process` doesn't exist yet): under the existing § that documents AIPC HandleKind discriminator pinning in the key-decisions table, append a single line:

```markdown
> Cross-platform implications: see [docs/architecture/aipc-unix-futures.md](../docs/architecture/aipc-unix-futures.md) (Phase 25 Plan 25-02 ADR).
```

Use the exact link text shown above — no rewording. The link target uses a relative path (`../docs/architecture/...`) because PROJECT.md lives at `.planning/PROJECT.md` and the ADR lives at `docs/architecture/...`; relative resolution from `.planning/` requires the `../` prefix.

Do NOT modify any other section of PROJECT.md. Do NOT update the key-decisions table beyond the single cross-link line. Do NOT touch ROADMAP.md or STATE.md (those are out of scope for this design-only plan).
  </action>
  <verify>
    <automated>grep -q 'aipc-unix-futures' .planning/PROJECT.md</automated>
  </verify>
  <done>PROJECT.md contains at least one match for `aipc-unix-futures`; the cross-link uses a relative path (`../docs/architecture/aipc-unix-futures.md`); no other PROJECT.md sections were modified (verified via `git diff --stat .planning/PROJECT.md` showing a small additive change, typically 1–2 lines).</done>
</task>

<task type="auto" tdd="false">
  <name>Task 3: Verify zero source-code changes and ADR shape</name>
  <files>(read-only verification — no files modified)</files>
  <action>
Final acceptance gate before commit. This task does NOT modify files; it runs verification commands and asserts the plan's must-haves are met.

**Verification commands (all must pass):**

1. ADR exists and has the locked structural shape:
   ```bash
   test -f docs/architecture/aipc-unix-futures.md
   test "$(grep -cE '^\| (File|Socket|Pipe|JobObject|Event|Mutex) \|' docs/architecture/aipc-unix-futures.md)" = "6"
   ```

2. ADR length is 250–400 lines (decision-only, not implementation):
   ```bash
   LINES=$(wc -l < docs/architecture/aipc-unix-futures.md)
   [ "$LINES" -ge 250 ] && [ "$LINES" -le 400 ]
   ```

3. ADR has all required H2 sections:
   ```bash
   test "$(grep -cE '^## (Context|Decision Table|Per-HandleKind Rationale|Alternate Mechanisms|Reversibility|References)$' docs/architecture/aipc-unix-futures.md)" = "6"
   ```

4. ADR status is Accepted:
   ```bash
   grep -qE '^\*\*Status:\*\* Accepted$' docs/architecture/aipc-unix-futures.md
   ```

5. PROJECT.md cross-links the ADR:
   ```bash
   grep -q 'aipc-unix-futures' .planning/PROJECT.md
   ```

6. **CRITICAL — zero source-code changes.** `git diff --stat HEAD` must show only `docs/architecture/aipc-unix-futures.md` (new) and `.planning/PROJECT.md` (modified). No `.rs`, `.toml`, `Cargo.lock`, `Makefile`, `.sh`, `.ps1`, or `.mdx` deltas:
   ```bash
   CHANGED=$(git diff --stat HEAD --name-only)
   echo "$CHANGED" | grep -qE '\.(rs|toml|lock|sh|ps1|mdx)$' && { echo "ERROR: source-code change detected; this plan is design-only" >&2; exit 1; } || true
   echo "$CHANGED" | grep -q '^Cargo\.lock$' && { echo "ERROR: Cargo.lock changed; design-only plan" >&2; exit 1; } || true
   echo "$CHANGED" | grep -q '^Makefile$' && { echo "ERROR: Makefile changed; design-only plan" >&2; exit 1; } || true
   ```

7. Decision row sanity (verdicts match the locked decision):
   ```bash
   grep -E '^\| 0 .* File .* Yes' docs/architecture/aipc-unix-futures.md
   grep -E '^\| 1 .* Socket .* Yes' docs/architecture/aipc-unix-futures.md
   grep -E '^\| 2 .* Pipe .* Yes' docs/architecture/aipc-unix-futures.md
   grep -E '^\| 3 .* JobObject .* No' docs/architecture/aipc-unix-futures.md
   grep -E '^\| 4 .* Event .* No' docs/architecture/aipc-unix-futures.md
   grep -E '^\| 5 .* Mutex .* No' docs/architecture/aipc-unix-futures.md
   ```

If any check fails, do NOT commit. Surface the failure to the maintainer with the specific check that failed and the actual vs. expected output. Re-running this task after fixing the underlying issue is safe (idempotent, read-only).
  </action>
  <verify>
    <automated>test -f docs/architecture/aipc-unix-futures.md && grep -q 'aipc-unix-futures' .planning/PROJECT.md && [ "$(grep -cE '^\| (File|Socket|Pipe|JobObject|Event|Mutex) \|' docs/architecture/aipc-unix-futures.md)" = "6" ] && ! git diff --stat HEAD --name-only | grep -qE '\.(rs|toml|sh|ps1|mdx)$|^Cargo\.lock$|^Makefile$'</automated>
  </verify>
  <done>All seven verification commands pass: ADR file exists with 6-row decision table, length in [250, 400], all H2 sections present, Status is Accepted, PROJECT.md cross-links the ADR, and `git diff --stat HEAD` shows zero source-code deltas (only docs/architecture/aipc-unix-futures.md new + .planning/PROJECT.md modified).</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

This plan is **design-only** and produces zero runtime behavior delta. There are no new trust boundaries introduced, no new attack surface, no new data flow.

| Boundary | Description |
|----------|-------------|
| (none)   | This plan modifies only documentation files. No process boundary, no IPC channel, no parsed input is added or changed. |

## STRIDE Threat Register

| Threat ID  | Category       | Component                    | Disposition | Mitigation Plan |
|------------|----------------|------------------------------|-------------|-----------------|
| T-25-02-01 | Information Disclosure | `docs/architecture/aipc-unix-futures.md` (public ADR) | accept | The ADR documents the AIPC HandleKind discriminant table and the locked Unix-backend decision. The discriminant values 0..5 are already public via the open-source `aipc_sdk.rs` const assertions; the Unix-backend decision is a roadmap statement, not a vulnerability disclosure. No PII, no secrets, no exploit detail. Accepting. |
| T-25-02-02 | Tampering      | `.planning/PROJECT.md` cross-link | mitigate | The cross-link is a single-line additive edit. Task 2 verifies idempotency (skip if link already present) and Task 3 verifies the diff shape (no other PROJECT.md sections modified). Reviewer signs off via the standard PR flow. |

(Threat register intentionally minimal — design-only ADR with zero runtime delta has nearly nothing to threat-model.)
</threat_model>

<risks>
## Top Risks

1. **Decision creeping into design.** The ADR is decision-only by ACCEPTANCE LOCK. The risk is that during drafting, the executor expands per-HandleKind rationales into API surface sketches, pseudocode, or RFC-style design diagrams. Mitigated by: explicit scope cap in `<objective>` and Task 1 action ("3–5 sentences. Hard cap."); verification in Task 3 caps total file length at 400 lines; reviewer enforces "no API sketch, no pseudocode, no diagrams" rule on PR.

2. **ADR getting orphaned by missing parent directory.** The `docs/architecture/` directory does not exist in the fork (verified at plan-time). The risk is that the file write fails silently or lands in the wrong path. Mitigated by: Task 1 action explicitly directs `mkdir -p docs/architecture` before write; Task 3 verification asserts the file exists at the locked path before allowing commit.

3. **PROJECT.md cross-link wording drifting from the ADR title.** The ADR title is "AIPC Unix Futures" and the link text in PROJECT.md must match for greppability. The risk is that the executor paraphrases the link text, breaking back-reference greps. Mitigated by: Task 2 specifies the exact link text verbatim ("AIPC Unix futures — see ..."); idempotency check prevents double-adding if a previous run committed a different wording.
</risks>

<verification>
## Phase-level Verification Gates

1. **ADR file exists at the locked path:** `docs/architecture/aipc-unix-futures.md` is a regular file.
2. **PROJECT.md cross-link visible:** `grep -n 'aipc-unix-futures' .planning/PROJECT.md` returns at least 1 match.
3. **Decision table has exactly 6 rows:** one per HandleKind 0..5 (File, Socket, Pipe, JobObject, Event, Mutex).
4. **Verdicts match the locked decision:** rows 0/1/2 = Yes; rows 3/4/5 = No.
5. **Required ADR sections present:** Context, Decision Table, Per-HandleKind Rationale, Alternate Mechanisms, Reversibility, References (six H2 headings).
6. **Status field is Accepted:** `grep -qE '^\*\*Status:\*\* Accepted$' docs/architecture/aipc-unix-futures.md`.
7. **ADR length is in [250, 400] lines:** decision-only, not implementation.
8. **Zero source-code changes:** `git diff --stat HEAD --name-only` shows ONLY `docs/architecture/aipc-unix-futures.md` and `.planning/PROJECT.md`. No `.rs`, `.toml`, `Cargo.lock`, `Makefile`, `.sh`, `.ps1`, or `.mdx` deltas.
</verification>

<success_criteria>
- [ ] `docs/architecture/aipc-unix-futures.md` exists, 250–400 lines, all six required H2 sections present.
- [ ] Decision table has exactly 6 rows with verdicts matching the locked decision (File/Socket/Pipe = Yes; JobObject/Event/Mutex = No).
- [ ] Each "No" row names its alternate Unix mechanism (cgroup v2 / `pipe(2)` / `flock(2)`).
- [ ] Reversibility section references AIPC G-04 (deferred to v2.4 backlog) as the trigger for revisiting the decision.
- [ ] References section back-links to `.planning/PROJECT.md` (AIPC HandleKind discriminator pinning entry) and Phase 23 RejectStage discussion.
- [ ] `.planning/PROJECT.md` cross-links the ADR via a single-line additive edit (idempotent if re-run).
- [ ] `git diff --stat HEAD` shows zero source-code changes — only `docs/architecture/aipc-unix-futures.md` (new) and `.planning/PROJECT.md` (modified).
- [ ] REQ-AIPC-NIX-01 is satisfied per REQUIREMENTS.md acceptance criteria (ADR at locked path, design-only, cross-linked from PROJECT.md).
</success_criteria>

<out_of_scope>
Explicit deferrals (do NOT include in this plan; do NOT let scope creep pull these in):

- **API surface sketch** for SCM_RIGHTS-based Socket/Pipe brokers (e.g., function signatures, error types, sendmsg/recvmsg call shape) — deferred to v2.4+ implementation phases.
- **Implementation pseudocode** for any HandleKind handler on Unix — deferred to v2.4+ implementation phases.
- **AIPC G-04** (wire-protocol compile-time tightening) — already deferred to v2.4 backlog, pre-existing.
- **Any source code touching** `crates/nono/src/supervisor/aipc_sdk.rs` or `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — this plan is documentation-only.
- **Updates to ROADMAP.md, STATE.md, REQUIREMENTS.md** — REQ-AIPC-NIX-01 is satisfied by ADR existence + cross-link; no requirement-table edits needed (Active → Validated transition happens at milestone close, not in this plan).
- **Diagrams, sequence diagrams, architecture diagrams** of any kind — the ADR is decision-only, not a design document.
- **Discussion of macOS-specific alternatives** beyond a single sentence in the JobObject rationale (macOS has no cgroup analog) — macOS deserves its own future ADR if/when AIPC-on-macOS becomes a priority; not in scope for v2.3.
</out_of_scope>

<output>
After completion, create `.planning/phases/25-cross-platform-resl-aipc-unix-design/25-02-SUMMARY.md` documenting:
- ADR file path + line count + section list
- PROJECT.md cross-link line + insertion point used (preferred vs fallback)
- Verification command outputs (all 7 from Task 3)
- `git diff --stat HEAD` output proving zero source-code changes
- Any deviations from the locked verdicts or alternate-mechanism mappings (should be none — flag loudly if any)
</output>
