---
quick_id: 260412-ajy
description: Architecture gaps and roadmap input for Windows safe-layer end state
date: 2026-04-12
status: completed
---

# Windows Safe-Layer Roadmap Input

## Purpose

This note is not a feature-parity checklist. It is a roadmap input for closing the remaining architecture gaps between the current Windows implementation and the full `nono` end state:

- a trustworthy safe layer for agentic software
- truthful product boundaries
- enforceable least-privilege defaults
- durable host-owned control of credentials, policy, execution, and audit

The target audience is a future roadmap/planning pass. The goal is to make the remaining work legible in terms of security properties, dependency order, and architectural end state.

---

## End-State Definition

Windows reaches the intended end state when all of the following are true:

1. `nono` on Windows has a real, enforceable sandbox primitive rather than a CLI-only validation and launcher discipline.
2. The library and CLI share a coherent trust model: what `support_info()`, docs, help text, and runtime behavior claim is exactly what is enforced.
3. Runtime capability expansion, trust verification, and network mediation work as host-controlled security boundaries, not just UX features.
4. The sandbox boundary remains fail-closed under common Windows escape shapes: symlinks, junctions, reparse points, UNC/device paths, ADS, search-order abuse, process tree expansion, and inherited handles.
5. Sensitive capabilities such as credentials, browser-open flows, proxy paths, and approval prompts stay outside the sandbox and cross the boundary only through audited, minimal, explicit channels.
6. The Windows backend is strong enough to be described as a platform-grade safe layer for agents, not merely “a Windows CLI port.”

---

## Current Position

The current branch is significantly stronger than upstream public messaging implies:

- Real Windows launch containment exists: Job Objects, low-integrity / restricted-token execution, environment hardening, path/argument validation, ConPTY support, WFP activation plumbing, named-pipe supervisor IPC, and ETW-based `learn`.
- Core supervised controls exist: session token validation, replay defense, low-integrity pipe ACLs, brokered handle passing, audit redaction.
- The main `nono run` path is already a meaningful Windows sandboxed execution flow.

But the implementation still falls short of the end state because several key security properties are incomplete, split across layers, or truthfully undocumented.

---

## Architecture Gap Summary

| Gap | Severity | Why It Matters |
|-----|----------|----------------|
| No irreversible Windows sandbox primitive in library layer | Critical | The Windows backend does not yet match the core `nono` design of a host-enforced primitive that embedders can rely on directly |
| Runtime trust interception absent | Critical | Windows cannot yet mediate sensitive file opens during execution, leaving a major hole in “safe layer” semantics |
| Runtime capability expansion only partially attached | High | Plumbing exists, but the backend still treats the feature as unsupported at the capability/model layer |
| Docs/help/tests/runtime boundary mismatch | High | Security software must be explicit and truthful about its real enforcement boundary |
| Network mediation model is still narrower than end-state trust model | High | WFP process policy exists, but broader host-owned network policy and credential/proxy semantics remain uneven |
| Session and interactive surface are internally inconsistent | Medium | Operational trust erodes when attach/detach/logging/shell behavior varies by stale docs or split implementations |
| Learn/audit semantics are still conservative and incomplete | Medium | Discovery and evidence are useful, but not yet strong enough to anchor policy with high confidence |
| Windows-specific threat model is not yet fully codified into product guarantees | Medium | There is implementation strength, but the security contract is not yet expressed as a stable architecture |

---

## Detailed Gaps

### Gap A: No true Windows sandbox primitive in the library layer

### Current state

`crates/nono/src/sandbox/windows.rs` validates capability shape and support, but it does not itself apply an irreversible OS-level sandbox primitive comparable to Linux Landlock or macOS Seatbelt.

### Why this is an end-state blocker

- The `nono` library promise is stronger than “the CLI launches children carefully.”
- Embedders cannot rely on `Sandbox::apply()` on Windows as a primitive in the same way they can on Linux/macOS.
- The trust boundary is therefore partly in product orchestration rather than in the platform primitive.

### End-state requirement

Windows must have a clearly-defined primitive layer with an auditable enforcement claim. If the primitive cannot be a single syscall-style “apply,” then the product must explicitly redefine the Windows primitive as a launcher-owned boundary and document that difference as a first-class architectural fact.

### Closure options

- Preferred: define and harden a Windows primitive composed of restricted token + integrity level + process mitigation policy + Job Object + network policy attachment + handle inheritance rules as one coherent enforcement unit.
- Acceptable fallback: explicitly split the Windows architecture into “library validation” and “CLI-enforced boundary,” then scope product claims accordingly. This is weaker than Unix and should be called out as such.

---

### Gap B: Runtime trust interception is not present

### Current state

Pre-exec trust scanning exists, but Windows runtime trust interception still fails closed as unsupported.

### Why this is an end-state blocker

- A safe layer for agents must protect against access to newly-discovered sensitive instructions or files after launch.
- Pre-exec verification only protects the initial known surface.
- Long-running or tool-using agents can still discover sensitive paths during execution unless another runtime mediation channel exists.

### End-state requirement

Windows needs a durable story for runtime trust enforcement.

### Architectural options

- Kernel minifilter path: strongest parity with the desired trust model, highest cost.
- Narrower product boundary: explicitly state that Windows provides pre-exec instruction trust but not runtime file-open mediation, and compensate with stronger path deny policy plus launch-time validation. This may be acceptable for a “good Windows edition,” but it is not the full safe-layer end state.

### Dependency note

This is the largest single strategic decision in the roadmap. It should be decided early, even if implementation is deferred, because it determines the final security claims Windows is allowed to make.

---

### Gap C: Runtime capability expansion is only partially real

### Current state

The named-pipe/token/audit plumbing is present, but the backend still rejects `extensions_enabled()` and still classifies runtime capability expansion as unsupported in several places.

### Why this matters

- The architecture is split between “transport exists” and “feature is supported.”
- The product cannot yet present a coherent story for host-approved capability growth at runtime.
- Without full attachment, Windows cannot match the supervised approval model that is central to agent safety.

### End-state requirement

Capability expansion on Windows should be:

- authenticated
- replay-safe
- audited
- host-approved
- minimal in granted scope
- represented truthfully in `CapabilitySet` and `support_info()`

### Closure work

- Remove the remaining backend-level hard reject once the pipeline is complete.
- Ensure granted handles cannot exceed requested access.
- Verify denial remains fail-closed when console/approval backend is absent.
- Add end-to-end tests from child request to host approval to brokered access.

---

### Gap D: Product truth surface is inconsistent

### Current state

Current code, README, CLI help, setup output, and tests disagree about Windows support for `shell`, `wrap`, sessions, and related commands.

### Why this is a security issue

- Security posture depends on users knowing the real boundary.
- Overclaiming support is dangerous.
- Underclaiming support creates drift and makes regression review harder.

### End-state requirement

A single support contract must drive:

- `support_info()`
- CLI help
- setup output
- README/docs
- tests
- release notes

### Closure work

- Choose the authoritative support matrix source.
- Generate or validate docs/help/tests against that matrix.
- Refuse merges that change Windows surface area without updating the matrix.

---

### Gap E: Windows network control is still narrower than the full safe-layer model

### Current state

WFP activation and port-level policy are materially better than earlier phases, but Windows still has a narrower and more process-centric network story than the full host-owned agent safety model.

### Why this matters

The end state is not just “block or allow ports.” It is:

- safe credential handling
- explicit proxy ownership
- trustworthy outbound identity and mediation
- clean separation between host-side secrets and sandbox-side execution

### End-state requirement

Windows should support a coherent network security architecture where:

- host-owned secrets stay outside the sandbox
- proxy mode is explicit and verifiable
- localhost exceptions are minimal and auditable
- the network policy attached to a child is provably the one the CLI reported

### Closure work

- Tighten proxy-only mode into an end-to-end supported flow.
- Validate WFP attachment against descendant processes and relaunches.
- Expand integration testing around connect, bind, localhost, proxy, and teardown ordering.
- Ensure network policy claims in logs/inspect output reflect actual active backend state.

---

### Gap F: Session and interactive architecture is split

### Current state

There is real Windows implementation for session records and commands, but detached-session messaging, PTY support, and docs/help still show inconsistent availability.

### Why this matters

- Operators need stable lifecycle semantics for long-running agents.
- A safe layer should preserve control over agent sessions, not just initial launch.
- Inconsistency here makes both debugging and incident response weaker.

### End-state requirement

Windows needs one consistent session model that covers:

- interactive shell
- attach/detach
- stop/terminate
- logs/inspect/prune
- audit continuity
- rollback continuity

### Closure work

- Decide whether detached sessions are a first-class Windows capability in the end state or intentionally out of scope.
- Remove stale “unsupported” branches once support is real.
- Add recovery tests for supervisor restarts, stale rendezvous, pipe collision, and orphan cleanup.

---

### Gap G: Learn and audit are useful but not yet authoritative

### Current state

Windows ETW `learn` is implemented and valuable, but still conservative in file access classification and lightly integration-tested.

### Why this matters

- A safe layer for agents needs good evidence, not just enforcement.
- If learning is too coarse, policies become overbroad.
- If audit is incomplete, approval workflows become less trustworthy.

### End-state requirement

Windows evidence tooling should be good enough to:

- bootstrap minimal policies
- explain why a policy is needed
- support incident review
- prove approval and access history without leaking secrets

### Closure work

- Improve access-mode classification beyond blanket `readwrite` where feasible.
- Expand Windows-host integration coverage for ETW event mapping.
- Verify audit continuity across approval, network activation, session lifecycle, and teardown.

---

## Non-Negotiable Security Properties

Any roadmap toward the end state should preserve these properties at every phase:

1. Fail closed on unsupported enforcement shapes.
2. Never silently widen filesystem or network access to compensate for Windows limitations.
3. Never leak session tokens, approval secrets, or proxy credentials into child-visible logs or state.
4. Keep handle brokering minimal, typed, and auditable.
5. Keep docs and runtime claims synchronized.
6. Preserve least privilege even when UX features lag behind parity.

---

## Recommended Roadmap Themes

These are the architectural themes the later roadmap should organize around.

### Theme 1: Define the real Windows security boundary

Goal:
Decide whether Windows reaches parity through a true primitive layer, or through a product-defined launcher boundary with different claims than Unix.

Deliverables:

- explicit Windows security model document
- allowed product claims
- forbidden overclaims
- boundary diagram for library vs CLI vs helper service vs child

Why first:
Every later roadmap phase depends on what Windows is allowed to claim.

### Theme 2: Finish supervised capability control

Goal:
Take runtime capability expansion from “transport exists” to “supported feature.”

Deliverables:

- end-to-end capability grant flow
- approval UX behavior under console/no-console conditions
- invariant tests for token auth, replay defense, and grant narrowing

Why second:
This is the cleanest path to stronger host control without requiring kernel interception first.

### Theme 3: Decide the runtime trust story

Goal:
Choose and document the Windows answer for runtime trust enforcement.

Deliverables:

- go/no-go decision for minifilter approach
- product claim language
- compensating controls if minifilter is deferred

Why third:
This is the major fork between “full end state possible” and “Windows remains a narrower edition.”

### Theme 4: Harden and prove network mediation

Goal:
Make Windows network control credible as a host-owned safety boundary.

Deliverables:

- WFP attachment proof points
- proxy-only end-to-end validation
- stronger integration tests around process tree/network policy coupling

### Theme 5: Unify session and operator control plane

Goal:
Ensure lifecycle management is stable and truthfully documented.

Deliverables:

- one supported Windows session model
- aligned CLI help/docs/tests
- recovery and orphan-cleanup tests

### Theme 6: Raise evidence quality

Goal:
Make `learn`, audit, and inspection strong enough to support human and automated trust decisions.

Deliverables:

- richer ETW learn confidence
- audit completeness matrix
- review tooling for Windows supervised sessions

---

## Dependency-Ordered Closure Sequence

1. **Define the Windows security contract**
   This unblocks truthful docs, support matrix cleanup, and later roadmap claims.
2. **Clean up truth-surface drift**
   Align README, CLI help, setup output, tests, and `support_info()`.
3. **Finish runtime capability expansion**
   This is a high-value supervised control that does not require kernel interception.
4. **Harden network mediation and process-policy coupling**
   Close proof gaps around WFP attachment and host-owned proxy semantics.
5. **Unify session semantics**
   Make lifecycle/operator control coherent.
6. **Improve learn and audit evidence**
   Stronger policy generation and incident review.
7. **Resolve runtime trust interception strategy**
   Either implement the kernel-backed path or formally narrow the Windows end-state claim.

Note:
Step 7 is last in implementation cost, but not last in decision priority. The architectural decision should happen near the start, even if execution is deferred.

---

## Suggested Milestone Framing

The next roadmap should probably avoid “v2 parity” framing and instead use milestone language tied to security properties:

- **Milestone A: Truthful Windows Security Boundary**
- **Milestone B: Host-Approved Runtime Capability Control**
- **Milestone C: Credible Network Mediation**
- **Milestone D: Stable Session Control Plane**
- **Milestone E: High-Confidence Evidence and Audit**
- **Milestone F: Runtime Trust Enforcement or Explicit Product Narrowing**

This keeps the roadmap tied to the actual safe-layer end state rather than to isolated command gaps.

---

## Questions The Roadmap Must Answer

Before planning execution, the roadmap pass should explicitly answer:

1. Is Windows expected to achieve full primitive-level parity with Linux/macOS, or only full CLI-level parity?
2. Is runtime trust interception required for the product to claim “safe layer for agentic software” on Windows?
3. If not, what exact narrower Windows claim is acceptable?
4. What is the canonical source of truth for supported Windows features?
5. Which invariants must be proven by Windows-host integration tests before broad release?
6. Which helper components, if any, are allowed to become part of the trusted computing base on Windows?

---

## Recommended Immediate Next Step

Use this note as the input to a fresh roadmap artifact that is explicitly organized around:

- end-state security properties
- trusted computing base boundaries
- product claims
- dependency order
- proof obligations

Do not start with command-by-command parity. Start with the Windows security contract and the runtime trust decision, then derive the roadmap from that.
