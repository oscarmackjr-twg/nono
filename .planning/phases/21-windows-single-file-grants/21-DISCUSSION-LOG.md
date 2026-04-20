# Phase 21: Windows Single-File Filesystem Grants — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in `21-CONTEXT.md` — this log preserves the alternatives considered.

**Date:** 2026-04-20
**Phase:** 21-windows-single-file-grants
**Areas discussed:** Enforcement primitive, Access modes, WriteOnly scope, Test strategy

---

## Enforcement Primitive

### Question: What is the primary enforcement mechanism for a single-file grant on Windows?

| Option | Description | Selected |
|--------|-------------|----------|
| Per-file Low IL label (Recommended) | Apply `SYSTEM_MANDATORY_LABEL_ACE` at `SECURITY_MANDATORY_LOW_RID` directly to each granted file via `SetNamedSecurityInfoW(SE_FILE_OBJECT, LABEL_SECURITY_INFORMATION, ..)`. Mirrors the existing directory-label primitive; mask field encodes access mode. Caveat: persists on the user's real file after session ends unless reverted. | ✓ |
| Clone-to-runtime-dir | Copy source file(s) into the Low-IL-labeled runtime state dir, child reads the clone. Zero mutation of user's files; trivial cleanup. Caveat: write-mode grants have to sync back, which is racy. | |
| Hybrid: label for read, clone for write | Per-file label for read-only, clone-to-runtime-dir for write / read-write. Most defensible semantically but two code paths + two test matrices. | |
| DACL ACE for session SID | Add explicit `Allow(SessionSID, FILE_GENERIC_READ/WRITE)` ACE. Doesn't bypass Low IL integrity-check — would need to be paired with a label, reduces to a variant of option 1. | |

**User's choice:** Per-file Low IL label (Recommended)
**Notes:** Chosen as the clean extension of the existing directory-label primitive. Avoids the write-sync races of clone-to-runtime-dir and the mechanism-multiplication of the hybrid / DACL options.

---

### Question: Label cleanup on session end — do we revert the Low IL mandatory label on the user's real file after the session exits?

| Option | Description | Selected |
|--------|-------------|----------|
| Revert on session end (Recommended) | Supervisor records pre-grant label state, restores on child exit / detach-and-stop. Keeps user's files in original integrity state across sessions. Concurrent-session overlap: "last session out restores". | |
| Persist label permanently | Apply once, never revert. Simpler code, no lifecycle races. Downside: silently lowers integrity label on user's `.gitconfig` forever — visible to Defender/EDR, violates "never silently degrade" if unexpected. | |
| Idempotent apply, best-effort revert (Recommended-variant) | Apply if not already set; on session end, revert only if WE set it (no-op if file was already Low IL for unrelated reasons). Safer than persist, less invasive than unconditional revert. | ✓ |

**User's choice:** Revert on session end (Recommended) — captured in D-02 as idempotent apply + revert on session end, which is the best-of-both reading (revert when we set it, no-op otherwise).
**Notes:** The idempotent-variant language was folded into D-02 because it preserves the user intent (revert on session end) while handling the file-already-Low-IL edge case correctly.

---

### Question: Write-mode single-file grant where the target file does NOT exist yet — how do we handle it?

| Option | Description | Selected |
|--------|-------------|----------|
| Reject at compile time (Recommended) | Path resolution at `CapabilitySet` build time canonicalizes; missing file → resolution error → compile_filesystem_policy returns PathNotFound; launch fails closed. If user wants write-to-new-file, grant parent dir or touch the file first. | ✓ |
| Create empty placeholder + label | Supervisor creates a zero-byte file at grant time, labels it Low IL, child writes to it. Handles first-run-writes-config case but mutates user's filesystem as a side effect of sandboxing. | |
| Defer — missing file is a runtime capability request | Let child's attempted open fail, fall through to Phase 11 runtime capability expansion. Out of scope for v2.1 (Windows doesn't support runtime capability expansion). | |

**User's choice:** Reject at compile time (Recommended)
**Notes:** Least-surprise win. Nono doesn't create files on behalf of the user as a side effect of sandboxing.

---

### Question: When `SetNamedSecurityInfoW` fails at apply time — what surface do we show?

| Option | Description | Selected |
|--------|-------------|----------|
| Named error with path + cause (Recommended) | `NonoError` variant carrying exact path that failed, Win32 error code, actionable hint. Matches diagnostic-quality precedent from Phase 09 ERROR_PIPE_BUSY translation and Phase 17 friendly single-attach message. | ✓ |
| Generic UnsupportedPlatform | Reuse existing variant with formatted message. Less structured but zero new error-type surface. | |
| Named variant + telemetry event | (1) plus a `tracing::error!` structured event for support-bound debugging via `--log-file`. | |

**User's choice:** Named error with path + cause (Recommended)
**Notes:** Supports the existing diagnostic quality bar. Exact variant shape (new `LabelApplyFailed` vs extension of `UnsupportedPlatform`) left to Claude's discretion during planning.

---

## Access Modes

### Question: Which access modes do we support for single-file grants in this phase?

| Option | Description | Selected |
|--------|-------------|----------|
| All three (Recommended) | Read / Write / ReadWrite — full parity with directory-scope grants. Label mask field encodes the choice cleanly. | ✓ |
| Read-only only (scope-narrow) | Address git_config motivator + unblock Phase 18 UAT only; defer write / read-write to a follow-up. Smaller blast radius; leaves `SingleFileGrant` partially enforceable. | |
| Read + ReadWrite, no write-only | Skip write-only single-file case. Symmetric with WriteOnlyDirectoryGrant being unsupported today. | |

**User's choice:** All three (Recommended)
**Notes:** The label mask field encodes the mode at zero extra implementation cost. No reason to artificially limit the surface.

---

## WriteOnly Scope

### Question: Do we also close `WindowsUnsupportedIssueKind::WriteOnlyDirectoryGrant` in this phase?

| Option | Description | Selected |
|--------|-------------|----------|
| Close both in this phase (Recommended) | Same primitive works at directory level (label a dir Low IL with `NO_READ_UP` mask). Retires both `WindowsUnsupportedIssueKind` variants. One phase, one researcher pass, one test matrix. | ✓ |
| Scope to single-file only (defer) | Land single-file grants only; leave `WriteOnlyDirectoryGrant` unsupported for a follow-up phase. Narrower scope — smaller phase, faster to ship. | |
| Close both AND retire the enum | (1) plus remove the `WindowsUnsupportedIssue` struct + unsupported_messages plumbing entirely. Cleaner end state but larger diff + risk of regressing fail-closed invariant. | |

**User's choice:** Close both in this phase (Recommended)
**Notes:** Enum retirement explicitly deferred to a follow-up cleanup phase — keep the enum shape as a reserved home for future unsupported Windows shapes. D-06 captures this.

---

## Test Strategy

### Question: How do we test the single-file / write-only enforcement end-to-end?

| Option | Description | Selected |
|--------|-------------|----------|
| Layered: unit + Windows integration + UAT re-run (Recommended) | (a) Cross-platform unit tests on `compile_filesystem_policy`; (b) Windows-only integration tests that apply label + read back via `GetNamedSecurityInfoW`; (c) Phase 18 AIPC UAT cookbook Path B/C re-run as phase close-out. Matches test-layering precedent from Phases 17/18/19. | ✓ |
| Unit + integration only, skip live UAT | Cover policy compilation + label-apply verification; leave Phase 18 UAT as separate quick-task. Phase 18 UAT stays blocked until that quick-task runs. | |
| Add a silent-degradation regression test | (1) plus a dedicated regression test: grant single file X, assert child CANNOT access sibling Y. Targets roadmap's "do not silently degrade" invariant. | |

**User's choice:** Layered: unit + Windows integration + UAT re-run (Recommended)
**Notes:** The silent-degradation regression test is folded INTO option (b) per the `<specifics>` block in CONTEXT.md — asserting `low_integrity_label_rid(parent_dir) == unchanged` after a single-file grant is applied. The roadmap's "do not silently degrade" invariant is covered by this assertion.

---

## Claude's Discretion

- Exact `NonoError` variant shape for label-apply failure (new `LabelApplyFailed` vs extension of `UnsupportedPlatform`) — planner decides during research.
- Exact module placement of revert-on-exit registration (sandbox/windows.rs vs exec_strategy_windows/supervisor.rs) — planner decides based on supervisor lifecycle-state ownership.
- Plan decomposition / wave parallelization — planner decides. Candidate shape is in CONTEXT.md `<decisions>` § Claude's Discretion.

## Deferred Ideas

- Full deletion of `WindowsUnsupportedIssueKind` enum + unsupported plumbing (reserved for follow-up cleanup phase).
- Windows runtime capability expansion for filesystem (`--trust`) — Phase 11 stretch, out of v2.1.
- Refcount / lease semantics for concurrent sessions sharing a labeled file — accept "last session out restores" for v2.1.
- Create-empty-and-label for write-mode grants to non-existent files — rejected on least-surprise; revisit if motivator surfaces.
- Kernel minifilter driver — v3.0 per STATE.md Gap 6b.
