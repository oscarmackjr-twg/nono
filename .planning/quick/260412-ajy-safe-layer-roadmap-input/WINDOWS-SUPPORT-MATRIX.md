# Windows Support Matrix

## Status

Canonical planning artifact for the Windows command/support surface.

This file operationalizes `WINDOWS-SECURITY-CONTRACT.md`. It is the canonical source of truth for the Windows support surface until a generated or schema-backed alternative replaces it.

If any of the following disagree with this file, they must be updated to match it or this file must be changed first:

- `README.md`
- `crates/nono-cli/README.md`
- Windows CLI help text
- Windows setup output
- Windows-specific tests
- Windows support reporting text

---

## Status Labels

Use exactly these labels:

- **Supported**: claimed, implemented, and within the current Windows contract
- **Unsupported**: explicitly outside the current Windows contract
- **Conditional**: supported only under explicitly stated constraints
- **Contract-Blocked**: code may exist, but the command cannot be claimed until docs/help/tests/runtime are aligned and the contract ambiguity is resolved

---

## Surface Rules

1. A command is not `Supported` unless code, help, docs, setup output, and tests agree.
2. `Contract-Blocked` means no public/support claim should be made yet.
3. `Conditional` entries must state the condition in the notes column.
4. Unsupported commands must fail clearly and must not silently degrade into wider access.

---

## Current Matrix

| Surface | Status | Notes |
|---------|--------|-------|
| `run` | Supported | Supported on the current Windows policy subset via the launcher-owned supervised boundary |
| `run --dry-run` | Supported | Policy inspection / validation only |
| `setup --check-only` | Supported | Within current Windows readiness/reporting scope |
| `policy` / `profile` inspection surfaces | Supported | Not part of the live enforcement boundary, but supported as policy tooling |
| `learn` | Conditional | Supported on Windows via ETW with host/admin constraints; evidence quality remains coarser than desired end state |
| `wrap` | Contract-Blocked | Code/help indicate support, but README/setup/tests still describe it as unavailable |
| `shell` | Contract-Blocked | Code/help indicate support, but README/setup/tests still describe it as unavailable |
| `ps` | Contract-Blocked | Root help still marks session lifecycle as available while subcommand help/tests mark it unavailable; runtime support is inconsistent at the truth-surface level |
| `stop` | Contract-Blocked | Same as `ps` |
| `attach` | Contract-Blocked | Same as `ps`; detached-session model not yet contract-defined |
| `detach` | Contract-Blocked | Same as `ps`; detached-session model not yet contract-defined |
| `logs` | Contract-Blocked | Runtime implementation exists, but tests/help still describe it as unavailable |
| `inspect` | Contract-Blocked | Runtime implementation exists, but tests/help still describe it as unavailable |
| `prune` | Contract-Blocked | Runtime implementation exists, but tests/help still describe it as unavailable |
| runtime capability expansion | Conditional | Security plumbing exists; cannot be broadly claimed until backend and support classification are fully aligned |
| runtime trust interception | Unsupported | Not in the current Windows contract |

---

## Category View

### Category A: Supported supervised execution

| Surface | Status |
|---------|--------|
| `run` | Supported |
| `wrap` | Contract-Blocked |
| `shell` | Contract-Blocked |

### Category B: Supported validation / setup

| Surface | Status |
|---------|--------|
| `run --dry-run` | Supported |
| `setup --check-only` | Supported |
| policy/profile inspection | Supported |

### Category C: Session / operator control

| Surface | Status |
|---------|--------|
| `ps` | Contract-Blocked |
| `stop` | Contract-Blocked |
| `attach` | Contract-Blocked |
| `detach` | Contract-Blocked |
| `logs` | Contract-Blocked |
| `inspect` | Contract-Blocked |
| `prune` | Contract-Blocked |

### Category D: Trust / mediation

| Surface | Status |
|---------|--------|
| runtime capability expansion | Conditional |
| runtime trust interception | Unsupported |

---

## Required Downstream Alignment

The following downstream artifacts must reflect this matrix exactly:

| Artifact | Role |
|----------|------|
| `README.md` | top-level public support summary |
| `crates/nono-cli/README.md` | CLI-level public support summary |
| `crates/nono-cli/src/cli.rs` | root help and subcommand help |
| `crates/nono-cli/src/setup.rs` | setup/readiness messaging |
| `crates/nono-cli/tests/env_vars.rs` | Windows truth-surface assertions |

---

## Update Rule

Any PR that changes the Windows support surface must update:

1. this support matrix
2. the Windows security contract if the contract itself changes
3. all downstream truth-surface artifacts listed above

If only code changes but the support claim does not, tests should prove that the matrix remains correct.
