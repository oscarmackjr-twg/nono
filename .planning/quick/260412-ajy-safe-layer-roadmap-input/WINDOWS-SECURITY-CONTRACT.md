# Windows Security Contract

## Status

Draft planning artifact.

This document defines the **current Windows security contract** for `nono`, the **trusted computing base** that implements that contract, and the **non-claims** the project must preserve until further milestones are complete.

This is not a marketing document. If this document conflicts with README/help/setup/tests, this document is the planning baseline and the other artifacts must be updated to match it.

---

## Purpose

The purpose of the Windows security contract is to answer four questions unambiguously:

1. What boundary does `nono` currently enforce on Windows?
2. Which components are trusted to enforce that boundary?
3. Which guarantees are strong enough to claim today?
4. Which guarantees are not yet true and therefore must not be implied?

---

## Contract Type

### Current contract

`nono` on Windows currently provides a **launcher-owned supervised sandbox boundary**, not a Unix-equivalent irreversible in-process sandbox primitive.

This means:

- The primary security boundary on Windows is established by the trusted launcher/supervisor path.
- The current Windows library backend validates capability shape and supported policy, but does not yet provide Linux/macOS-style primitive parity through `Sandbox::apply()` alone.
- Claims about Windows must therefore be phrased in terms of the **supervised execution boundary**, not in terms of a standalone library primitive.

### Non-claim

`nono` must not currently claim that Windows has full primitive-level parity with Linux Landlock or macOS Seatbelt.

---

## Security Boundary

### Boundary statement

For supported Windows execution paths, `nono` establishes a host-owned boundary around the launched child process and its descendants by combining:

- launch-time capability validation
- constrained token / low-integrity launch
- Job Object process containment
- controlled environment construction
- launch-path and argument validation
- supervised named-pipe control channels
- network policy attachment through the active Windows backend

### Boundary owner

The boundary is owned by the trusted `nono` launcher/supervisor and supporting helper components, not by an irreversible self-application primitive inside the child process.

### Boundary consequence

If the launcher/supervisor path is bypassed, the Windows contract does not claim equivalent enforcement.

---

## Trusted Computing Base

The current Windows trusted computing base consists of:

### TCB components

- `nono-cli` Windows launcher path
- `nono-cli` Windows supervised runtime
- `crates/nono` Windows policy compilation and validation layer
- Windows Job Object process containment
- Windows token / integrity-level launch configuration
- Windows named-pipe supervisor IPC
- the active Windows network backend used for the requested policy
- any required Windows helper/service component involved in enforcing the requested policy

### Conditionally trusted components

- WFP service/driver path, when Windows network policy depends on it
- approval backend path used for runtime capability grants
- future runtime trust mediation components, if introduced

### Not currently in TCB

- sandboxed child process logic
- user-authored agent code
- upstream documentation text
- any unsupported runtime trust interception mechanism

---

## Supported Security Guarantees

The following guarantees are within the current Windows contract for supported flows.

### G1: Fail-closed unsupported-shape handling

When a requested Windows capability shape is unsupported, `nono` must fail closed rather than silently widen or ignore the request.

### G2: Host-owned launch validation

`nono` validates launch paths, working directory expectations, and relevant path-bearing command arguments against the compiled Windows filesystem policy before child execution proceeds.

### G3: Process-tree containment for supported flows

For supported live Windows execution flows, the target child is launched under trusted process containment using a Job Object so lifecycle control is owned by the supervisor, not delegated to the child.

### G4: Reduced write authority through token/integrity controls

For Windows flows that require filesystem restriction, `nono` uses a lower-authority launch model rather than granting the child ambient write authority by default.

### G5: Controlled runtime capability channel

Where runtime capability expansion is attached, capability requests cross a host-owned control channel and are subject to token validation, replay defense, approval, and audit. Until fully attached, unsupported cases must remain fail-closed.

### G6: Protected control-channel secrets

Session tokens and approval-sensitive values must not be logged or exposed to the child through ordinary audit serialization or status surfaces.

### G7: Explicit network-policy activation

When Windows network restriction is claimed for a supported flow, it must be attached through the selected active backend or the command must fail closed with diagnostics.

### G8: Truthful support boundary

The Windows command surface claimed by the project must match the supported subset actually enforced by the code.

---

## Explicit Non-Guarantees

The following are not currently guaranteed and must not be implied.

### N1: Primitive-level parity with Linux/macOS

Windows does not currently guarantee a Linux/macOS-equivalent irreversible sandbox primitive through the library layer alone.

### N2: Runtime trust interception

Windows does not currently guarantee runtime mediation of sensitive file opens during child execution.

Pre-exec trust scanning may exist, but runtime file-open interception is not part of the current Windows contract.

### N3: Universal parity for every CLI command

Windows must not claim that every `nono` command has the same semantics, transport, or maturity as Linux/macOS.

### N4: Silent compensation for unsupported controls

Windows must not silently widen access in order to simulate unsupported file, network, trust, or session behavior.

### N5: Child-owned approval or secret custody

Windows must not imply that approval decisions, approval credentials, or proxy credentials are safely delegated to the child itself.

---

## Supported Surface Categories

The supported Windows surface should be described by categories, not only by individual commands.

### Category A: Supported supervised execution

Supported:

- `run` on the supported Windows policy subset
- `wrap` only if current runtime behavior and docs are aligned to claim it as supported
- `shell` only if current runtime behavior and docs are aligned to claim it as supported

Contract rule:

- A command is supported only if help/docs/tests/runtime all agree and its enforcement path is within this contract.

### Category B: Supported policy validation and dry-run

Supported:

- support reporting
- dry-run policy inspection
- setup and readiness diagnostics within the Windows contract

### Category C: Supported supervised control channels

Conditionally supported:

- runtime capability requests when fully attached
- session lifecycle operations only within the explicitly supported Windows session model

### Category D: Unsupported or not-yet-claimed trust features

Not currently claimed:

- runtime trust interception
- any feature that depends on it for correctness

---

## Contract for Library vs CLI

### Library contract

On Windows today, the library layer is a **policy compiler and support validator**. It is not yet the sole source of the Windows sandbox boundary.

### CLI contract

On Windows today, the CLI/supervisor path is the primary source of the live execution security boundary for supported flows.

### Required implication

Any project text that talks about Windows “sandboxing” must make clear whether it refers to:

- policy validation only
- supervised launcher enforcement
- a future primitive layer not yet implemented

---

## Contract for Runtime Capability Expansion

Runtime capability expansion on Windows is only part of the contract when all of the following are true:

1. The child uses the supported supervisor channel.
2. The request includes a valid session token.
3. Replay is rejected.
4. Approval is host-controlled.
5. Granted access is no broader than the approved request.
6. The audit trail redacts sensitive request secrets.

If any of these conditions are not met, the feature must fail closed and must not be claimed as supported.

---

## Contract for Network Mediation

Windows network restriction is only part of the contract when:

1. the requested policy is in the supported Windows subset
2. the active backend is available and successfully attached
3. the user-visible status matches the active backend and requested policy

If the backend cannot enforce the requested policy, the launch must fail closed.

### Required truth rule

Proxy-only, port allowlist, and localhost exception claims must reflect actual enforced behavior, not intended future behavior.

---

## Contract for Sessions and Operator Control

Windows session commands are in contract only if they belong to the explicitly supported Windows session model.

This model must answer:

- whether detached sessions are supported
- whether interactive attach/detach is supported
- whether logs/inspect/prune are supported for Windows sessions
- how stale supervisor state is handled

Until the model is explicit and aligned across artifacts, no ambiguous session claim should ship.

---

## Contract for Trust

### Current trust claim

Windows may claim:

- pre-exec trust verification of configured instruction/material inputs, where implemented

Windows may not currently claim:

- runtime file-open mediation during execution

### Strategic fork

The project must later decide one of two contract evolutions:

1. **Full-trust path:** add runtime trust mediation and extend the contract
2. **Narrowed Windows path:** keep trust verification pre-exec only and explicitly state that Windows is a narrower safe-layer edition

---

## Non-Negotiable Invariants

The following invariants apply to every Windows milestone:

1. Unsupported requests fail closed.
2. Sensitive secrets are not written to child-visible logs or audit output.
3. Handle brokering remains minimal and auditable.
4. Support claims are synchronized across docs/help/tests/runtime.
5. The child never becomes the owner of host-only approval or secret material.
6. New helper components are explicitly added to the TCB, never implicitly assumed.

---

## Source of Truth Rule

This contract should become the parent artifact for the Windows support matrix.

Downstream artifacts that must conform to it:

- `README.md`
- `crates/nono-cli/README.md`
- Windows help templates
- setup output
- Windows-specific tests
- support reporting text

If downstream artifacts disagree with this contract, they are wrong unless this contract is updated first.

---

## Open Decisions

These decisions remain open at the contract level:

1. Is a launcher-owned Windows boundary acceptable as the final Windows edition, or only as an interim stage?
2. Is runtime trust interception mandatory for Windows to claim the full safe-layer end state?
3. If runtime trust interception is deferred, what exact narrowed Windows claim is allowed?
4. Which helper services and drivers are acceptable members of the final Windows trusted computing base?
5. What is the canonical support-matrix artifact that operationalizes this contract?

---

## Immediate Follow-On Work

This contract is the baseline input for:

- **M1 Truth Surface**
  Align docs/help/setup/tests to this contract.
- **M2 Runtime Capability Control**
  Finish and prove the host-approved capability channel.
- **M3 Network Mediation**
  Prove the active backend matches the claimed enforcement.
- **M6 Primitive Boundary**
  Decide whether Windows remains launcher-owned or grows into a true primitive layer.
- **M7 Runtime Trust Enforcement**
  Resolve the runtime trust fork.
