# Phase 10: ETW-Based Learn Command - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-10
**Phase:** 10-etw-based-learn-command
**Areas discussed:** ETW library, Admin privilege UX, Process tree scope, FileIo event → access mode mapping

---

## ETW Library

| Option | Description | Selected |
|--------|-------------|----------|
| ferrisetw | Higher-level Rust API; handles session lifecycle, callback dispatch, schema decoding; one new dependency; community crate (last release 2023) — audit required | ✓ |
| windows-sys direct bindings | Raw Win32 ETW calls; no new dep; ~200–300 lines of unsafe boilerplate | |
| windows-sys + add ETW features | Same as above but explicitly frames it as adding feature flags only | |

**User's choice:** ferrisetw
**Notes:** Audit required before committing. Prior research already recommended this approach.

---

## Admin Privilege UX

| Option | Description | Selected |
|--------|-------------|----------|
| Upfront check + runas hint | IsUserAnAdmin() before ETW; immediate error with "Run as administrator" hint; exit non-zero | ✓ |
| Attempt ETW, surface ACCESS_DENIED | Let StartTrace/OpenTrace fail; translate error to user message | |
| Upfront check, no runas hint | Detect early, minimal error message only | |

**User's choice:** Upfront check + runas hint
**Notes:** Fail fast, actionable error message with platform-specific guidance.

---

## Process Tree Scope

| Option | Description | Selected |
|--------|-------------|----------|
| Full process tree | Track direct child + all descendants via CreateProcess/ExitProcess events; mirrors strace -f | ✓ |
| Direct child only | Filter to single spawned PID; simpler but incomplete for build tools | |
| Direct child + --follow-forks flag | Default to direct child, opt-in flag for tree; unnecessary complexity | |

**User's choice:** Full process tree
**Notes:** Essential for cargo, npm, make and similar tools that fork subprocesses.

---

## FileIo Event → Access Mode Mapping

| Option | Description | Selected |
|--------|-------------|----------|
| Map by CreateFile DesiredAccess flags | GENERIC_READ → read, GENERIC_WRITE/DELETE → write, both → readwrite; captures intent at open time | ✓ |
| Map by observed event types | FileIo/Read seen → read; FileIo/Write seen → write; both → readwrite; more events to process | |
| All files as readwrite (conservative) | Treat everything as readwrite; simple; over-grants | |

**User's choice:** DesiredAccess flag-based mapping
**Notes:** FileIo/Create DesiredAccess captures caller intent; FileIo/Read+Write events alone are too noisy.

---

## Claude's Discretion

- NT → Win32 path conversion mechanics (QueryDosDevice volume map at startup)
- ferrisetw session naming
- ETW consumer buffer sizing and timeout
- Network event subtypes to capture
- Post-exit event drain strategy

## Deferred Ideas

None.
