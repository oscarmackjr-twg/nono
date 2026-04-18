# M1 Truth-Surface Cleanup Plan

This plan uses:

- `WINDOWS-SECURITY-CONTRACT.md`
- `WINDOWS-SUPPORT-MATRIX.md`

to turn M1 into a bounded cleanup pass.

Goal:
Make every Windows-facing truth surface match the current contract and canonical support matrix.

---

## M1 Goal

By the end of M1:

1. docs, help, setup output, and tests all describe the same Windows boundary
2. every Windows command is either explicitly supported, explicitly unsupported, or explicitly contract-blocked
3. no user-visible artifact overclaims primitive-level Windows parity

---

## Planning Choice

The support matrix selected for M0.2 is:

- `WINDOWS-SUPPORT-MATRIX.md`

This file is the canonical source of truth for M1.

---

## Workstreams

### Workstream 1: Public docs alignment

**Files:**

- [README.md](/abs/path/C:/Users/omack/nono/README.md)
- [crates/nono-cli/README.md](/abs/path/C:/Users/omack/nono/crates/nono-cli/README.md)

**Current mismatch:**

- Both READMEs still say live Windows `shell`/`wrap` are intentionally unavailable.
- Top-level README still describes Windows in broad terms that may imply more stable support than the contract currently allows.
- README examples and feature table still implicitly reflect Unix-first capabilities.

**Planned changes:**

- Rewrite Windows support summary to match the contract and support matrix.
- Remove primitive-parity language from Windows-facing summaries.
- Replace stale `shell`/`wrap` availability claims with matrix-aligned wording.
- Add explicit note that Windows currently uses a launcher-owned supervised boundary.

**Exit criteria:**

1. README text matches the current matrix exactly.
2. No README implies full primitive-level parity on Windows.

---

### Workstream 2: CLI help alignment

**Files:**

- [crates/nono-cli/src/cli.rs](/abs/path/C:/Users/omack/nono/crates/nono-cli/src/cli.rs)

**Current mismatch:**

- Root help lists `ps`, `stop`, `attach`, `detach`, `logs`, `inspect`, `prune` as available.
- Several Windows subcommand helps still describe those same surfaces as intentionally unavailable.
- `shell`/`wrap` help currently claims support while README/setup/tests still disagree.

**Planned changes:**

- Make root help consistent with subcommand help and support matrix.
- For every contract-blocked surface, choose one of:
  - mark as unavailable everywhere
  - or promote to supported everywhere if that is the chosen truth
- Remove mixed messaging within the help templates.

**Exit criteria:**

1. Root help and subcommand help no longer disagree.
2. Every Windows help surface matches the support matrix label.

---

### Workstream 3: Setup output alignment

**Files:**

- [crates/nono-cli/src/setup.rs](/abs/path/C:/Users/omack/nono/crates/nono-cli/src/setup.rs)

**Current mismatch:**

- Setup output still says live Windows `shell` and `wrap` are unavailable.
- Setup output does not explicitly anchor itself to the support matrix or contract wording.

**Planned changes:**

- Update Windows setup/readiness output to summarize the matrix accurately.
- Point users to the current Windows command surface without contradicting help/docs.
- Remove stale statements about `shell`/`wrap` if the matrix changes them.

**Exit criteria:**

1. Setup output matches README/help/tests on Windows surface claims.
2. Setup output no longer contains stale availability text.

---

### Workstream 4: Test alignment

**Files:**

- [crates/nono-cli/tests/env_vars.rs](/abs/path/C:/Users/omack/nono/crates/nono-cli/tests/env_vars.rs)

**Current mismatch:**

- Tests still assert that `shell`, `wrap`, `logs`, `inspect`, and `prune` are unavailable on Windows.
- Tests also assert unsupported session-management descriptions that conflict with root help and runtime code.

**Planned changes:**

- Replace stale availability assertions with matrix-driven assertions.
- Separate “unsupported by contract” from “implemented but contract-blocked.”
- Add a focused Windows truth-surface test block keyed to the support matrix.

**Exit criteria:**

1. Tests validate the intended Windows support surface rather than stale text.
2. A truth-surface regression fails whenever help/docs/setup drift from the matrix.

---

### Workstream 5: Contract-blocked resolution

**Surfaces:**

- `shell`
- `wrap`
- `ps`
- `stop`
- `attach`
- `detach`
- `logs`
- `inspect`
- `prune`

**Current issue:**

These are neither cleanly supported nor cleanly unsupported at the truth-surface level.

**Planned approach:**

For each surface, pick one resolution:

1. **Promote to Supported**
   Use only if code, tests, and contract can all support the claim immediately.
2. **Demote to Unsupported**
   Use if the session model or enforcement story is still too ambiguous.
3. **Keep Contract-Blocked**
   Only temporarily, while M1 is in progress. M1 should aim to eliminate contract-blocked status from user-visible artifacts.

**Recommendation:**

- Resolve `shell` and `wrap` first, because they directly affect the top-level Windows support story.
- Resolve the session-control surfaces together, because they share one operator model.

**Exit criteria:**

1. M1 ends with minimal or zero `Contract-Blocked` surfaces in public-facing artifacts.
2. Any remaining blocked surfaces are hidden from public support claims and clearly marked as pending contract resolution.

---

## Execution Order

1. Update `WINDOWS-SUPPORT-MATRIX.md` only if M1 chooses to promote/demote any contract-blocked surface.
2. Align CLI help in `cli.rs`.
3. Align setup output in `setup.rs`.
4. Align public docs in both READMEs.
5. Align Windows truth-surface tests.
6. Run a final matrix-to-artifact review pass.

---

## Exit Criteria

M1 is complete when:

1. `README.md`, `crates/nono-cli/README.md`, `cli.rs`, `setup.rs`, and Windows truth-surface tests all match `WINDOWS-SUPPORT-MATRIX.md`.
2. No artifact claims full primitive-level Windows parity.
3. No command is simultaneously described as both supported and unsupported.
4. The project can point contributors to one support matrix and one contract document for Windows truth.

---

## Suggested Follow-On

Once M1 is complete:

- M2 can proceed with runtime capability control using a stable truth surface
- M4 can define the final session/operator model without carrying stale text debt
- future CI can validate matrix alignment automatically
