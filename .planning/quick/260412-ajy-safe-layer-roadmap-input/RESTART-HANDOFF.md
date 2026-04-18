# Restart Handoff: Windows Safe-Layer Planning

## Purpose

This note is the restart point for continuing the Windows safe-layer planning work later.

If resuming this effort, start here, then open the linked artifacts in order.

---

## What Was Decided

### Decision 1: Planning frame

The Windows roadmap should be driven by **architecture and security-boundary gaps**, not by command-parity checklists.

### Decision 2: Current Windows contract

Windows currently has a **launcher-owned supervised sandbox boundary**, not full primitive-level parity with Linux/macOS.

### Decision 3: Canonical source of truth

The canonical support-matrix artifact is:

- `WINDOWS-SUPPORT-MATRIX.md`

This file is the planning source of truth for Windows support claims until replaced by a generated or schema-backed alternative.

### Decision 4: M1 approach

M1 should be executed as a truth-surface cleanup driven by:

- `WINDOWS-SECURITY-CONTRACT.md`
- `WINDOWS-SUPPORT-MATRIX.md`

### Decision 5: Current blocked surfaces

The following are currently treated as **Contract-Blocked** until M1 resolves them:

- `shell`
- `wrap`
- `ps`
- `stop`
- `attach`
- `detach`
- `logs`
- `inspect`
- `prune`

---

## Key Artifacts

Read these in this order:

1. [260412-ajy-SAFE-LAYER-ROADMAP-INPUT.md](/abs/path/C:/Users/omack/nono/.planning/quick/260412-ajy-safe-layer-roadmap-input/260412-ajy-SAFE-LAYER-ROADMAP-INPUT.md)
2. [WINDOWS-SAFE-LAYER-ROADMAP.md](/abs/path/C:/Users/omack/nono/.planning/quick/260412-ajy-safe-layer-roadmap-input/WINDOWS-SAFE-LAYER-ROADMAP.md)
3. [M0-FIRST-EXECUTABLE-PHASE-SET.md](/abs/path/C:/Users/omack/nono/.planning/quick/260412-ajy-safe-layer-roadmap-input/M0-FIRST-EXECUTABLE-PHASE-SET.md)
4. [WINDOWS-SECURITY-CONTRACT.md](/abs/path/C:/Users/omack/nono/.planning/quick/260412-ajy-safe-layer-roadmap-input/WINDOWS-SECURITY-CONTRACT.md)
5. [WINDOWS-SUPPORT-MATRIX.md](/abs/path/C:/Users/omack/nono/.planning/quick/260412-ajy-safe-layer-roadmap-input/WINDOWS-SUPPORT-MATRIX.md)
6. [M1-TRUTH-SURFACE-CLEANUP-PLAN.md](/abs/path/C:/Users/omack/nono/.planning/quick/260412-ajy-safe-layer-roadmap-input/M1-TRUTH-SURFACE-CLEANUP-PLAN.md)

---

## Current Status

Completed:

- architecture-gap assessment
- safe-layer roadmap input
- concrete phased roadmap
- M0 executable phase set
- initial Windows security contract draft
- canonical support matrix selection
- M1 truth-surface cleanup plan

Not yet done:

- executing M1 cleanup in code/docs/tests
- resolving contract-blocked surfaces into supported vs unsupported
- adding CI enforcement for support-matrix drift

---

## Recommended Next Step

Resume with **M1 Workstream 5: Contract-blocked resolution**.

Specifically:

1. Decide whether `shell` and `wrap` should be promoted or demoted.
2. Decide the Windows session/operator model for `ps` / `stop` / `attach` / `detach` / `logs` / `inspect` / `prune`.
3. Update `WINDOWS-SUPPORT-MATRIX.md` if those decisions change the current labels.
4. Execute the file edits in:
   - `README.md`
   - `crates/nono-cli/README.md`
   - `crates/nono-cli/src/cli.rs`
   - `crates/nono-cli/src/setup.rs`
   - `crates/nono-cli/tests/env_vars.rs`

---

## Resume Prompt

If restarting this later, use a prompt like:

`Resume Windows safe-layer planning from .planning/quick/260412-ajy-safe-layer-roadmap-input/RESTART-HANDOFF.md and execute M1 Workstream 5 plus the resulting truth-surface cleanup.`
