# Roadmap: Windows Safe-Layer End State

This roadmap turns `260412-ajy-SAFE-LAYER-ROADMAP-INPUT.md` into a concrete, dependency-ordered plan for closing the remaining architecture gaps between the current Windows implementation and the intended `nono` end state: a trustworthy safe layer for agentic software.

This is intentionally not a command-parity roadmap. It is organized around:

- security boundary definition
- trusted computing base shape
- host-controlled runtime mediation
- truthful product claims
- proof obligations

**Milestone goal:** Bring Windows from “strong supervised CLI port” to a Windows backend that can credibly claim safe-layer semantics, with explicit narrowing only where structural limits remain.

**Core value:** Every shipped Windows capability must be both enforced and truthfully described.

---

## Milestone Overview

| Milestone | Goal | Complexity | Depends On |
|-----------|------|------------|------------|
| M0 | Define the Windows security contract | M | None |
| M1 | Align truth surface to the contract | S | M0 |
| M2 | Finish host-approved runtime capability control | M | M0 |
| M3 | Harden and prove network mediation | M | M0 |
| M4 | Unify session and operator control plane | M | M1 |
| M5 | Raise learn/audit evidence quality | M | M1 |
| M6 | Establish the Windows primitive boundary | L | M0, M2, M3 |
| M7 | Resolve runtime trust enforcement | XL | M0, M5, M6 |

---

## Dependency Diagram

```text
M0 Security Contract
 ├──> M1 Truth Surface
 ├──> M2 Runtime Capability Control
 ├──> M3 Network Mediation
 └──> M6 Primitive Boundary

M1 Truth Surface ──> M4 Session Control Plane
M1 Truth Surface ──> M5 Evidence Quality

M2 Runtime Capability Control ──┐
M3 Network Mediation ───────────┼──> M6 Primitive Boundary
M5 Evidence Quality ────────────┘

M0 + M5 + M6 ──> M7 Runtime Trust Enforcement
```

Notes:

- M0 is the forcing function. Do not skip it.
- M7 contains the strategic fork: implement kernel-backed runtime trust interception, or formally narrow the Windows end-state claim.
- M6 is where Windows stops being “mostly a launcher architecture” and becomes a clearly-defined platform boundary.

---

## Milestones

- [ ] **M0: Security Contract** — Define the real Windows security boundary
- [ ] **M1: Truth Surface** — Make docs, help, tests, and runtime claims match
- [ ] **M2: Runtime Capability Control** — Finish supervised capability expansion
- [ ] **M3: Network Mediation** — Make WFP/proxy policy a credible host-owned boundary
- [ ] **M4: Session Control Plane** — Unify session/operator lifecycle semantics
- [ ] **M5: Evidence Quality** — Improve `learn`, audit, and inspection confidence
- [ ] **M6: Primitive Boundary** — Establish a coherent Windows primitive layer
- [ ] **M7: Runtime Trust Enforcement** — Implement or formally narrow runtime trust claims

---

## Milestone Details

### M0: Security Contract

**Goal:** Define the Windows security model precisely enough that engineering, docs, tests, and release claims can all agree on what Windows does and does not guarantee.

**Why first:** The remaining roadmap is blocked on whether Windows is targeting primitive-level parity, CLI-level parity, or an explicitly narrower but still trustworthy Windows edition.

**Requirements:** CONTRACT-01, TCB-01

**Plans:**

- [ ] M0-01 — Write `WINDOWS-SECURITY-CONTRACT.md`: define the trusted computing base, boundary diagram, enforcement claims, and explicit non-claims
- [ ] M0-02 — Decide runtime trust target: `KernelMinifilterRequired`, `DeferredWithNarrowedClaim`, or `OutOfScopeForWindowsEdition`
- [ ] M0-03 — Define the canonical support matrix source used by `support_info()`, CLI help, docs, setup output, and tests

**Success Criteria:**

1. There is one reviewed artifact that states the Windows security boundary and trusted computing base.
2. The project explicitly answers whether Windows is targeting full primitive-level parity or a narrower Windows-specific safe-layer claim.
3. Runtime trust interception has an architectural decision, even if implementation is deferred.
4. There is a canonical support-matrix source that later milestones can validate against.

---

### M1: Truth Surface

**Goal:** Eliminate boundary drift between runtime behavior and the text the project shows users and contributors.

**Why now:** Once the security contract exists, mismatched docs/help/tests become a release blocker rather than a cleanup item.

**Depends on:** M0

**Requirements:** TRUTH-01

**Plans:**

- [ ] M1-01 — Reconcile README, `crates/nono-cli/README.md`, setup output, and help templates with the M0 contract
- [ ] M1-02 — Align Windows-specific tests with the supported command surface and remove stale “unsupported” assertions
- [ ] M1-03 — Add CI validation that all Windows surface-area changes also update the support matrix and help/docs artifacts

**Success Criteria:**

1. README, CLI help, setup output, and tests describe the same Windows boundary.
2. No Windows command is simultaneously described as both supported and unsupported in-tree.
3. `support_info()` and CLI messaging reflect the same supported subset.
4. CI catches future drift automatically.

---

### M2: Runtime Capability Control

**Goal:** Complete runtime capability expansion as a supported, host-approved feature rather than a partially-wired transport.

**Depends on:** M0

**Requirements:** CAPEXP-01

**Plans:**

- [ ] M2-01 — Complete the child-to-supervisor capability request path and remove remaining backend-level “unsupported” gates once invariants are met
- [ ] M2-02 — Harden approval semantics: no-console behavior, deny-all fallback, request narrowing, and brokered-handle lifecycle validation
- [ ] M2-03 — Add Windows-host integration coverage for request, grant, deny, replay rejection, and token mismatch flows

**Success Criteria:**

1. A Windows child can request additional capabilities through the supported supervisor path.
2. Requests are authenticated, replay-safe, minimally granted, and audited.
3. The feature remains fail-closed when approval UX or control channels are unavailable.
4. `CapabilitySet`, `support_info()`, and supervised feature classification no longer disagree about whether capability expansion is supported.

---

### M3: Network Mediation

**Goal:** Make Windows network control credible as a host-owned security boundary, not just a process-launch accessory.

**Depends on:** M0

**Requirements:** NETSAFE-01

**Plans:**

- [ ] M3-01 — Normalize the Windows network model in product terms: allow-all, blocked, proxy-only, per-port rules, localhost exceptions, and helper-service prerequisites
- [ ] M3-02 — Harden WFP activation proof: verify policy attachment to the target process tree, descendant processes, teardown ordering, and relaunch behavior
- [ ] M3-03 — Finish proxy-only as an end-to-end flow: env injection, localhost permit rules, explicit host-owned credential separation, and inspect/log visibility
- [ ] M3-04 — Add Windows-host integration tests for blocked, proxy-only, connect allowlist, bind allowlist, localhost allowlist, and failure paths

**Success Criteria:**

1. The network policy reported to the user matches the policy actually enforced by the active backend.
2. Proxy-only mode is fully supported or explicitly documented as unsupported by contract; there is no ambiguous middle state.
3. Child and descendant process networking obey the same intended policy.
4. Unsupported or unattachable network policy requests fail closed with actionable diagnostics.

---

### M4: Session Control Plane

**Goal:** Unify the Windows session/operator lifecycle into one supported model.

**Depends on:** M1

**Requirements:** SESSION-01

**Plans:**

- [ ] M4-01 — Decide the supported Windows session model: fully interactive only, detachable sessions, or a split supported subset with explicit rationale
- [ ] M4-02 — Align `ps`, `stop`, `attach`, `detach`, `logs`, `inspect`, and `prune` with that model and remove stale “unsupported” branches
- [ ] M4-03 — Add recovery coverage for stale rendezvous, orphaned jobs, crashed supervisors, log continuity, and cleanup behavior

**Success Criteria:**

1. Operators have one clear answer for how Windows session lifecycle works.
2. Session commands either work end-to-end or are intentionally absent by contract.
3. Stale pipe, orphaned job, and supervisor-restart scenarios have deterministic behavior.
4. Help/docs/tests all match the actual Windows session model.

---

### M5: Evidence Quality

**Goal:** Make Windows evidence tooling strong enough to support policy generation, human review, and incident analysis.

**Depends on:** M1

**Requirements:** EVIDENCE-01

**Plans:**

- [ ] M5-01 — Improve ETW `learn` fidelity: better file access classification, path normalization edge cases, and event confidence notes
- [ ] M5-02 — Expand Windows-host integration coverage for ETW file/network events and error handling under non-admin/admin conditions
- [ ] M5-03 — Define an audit completeness matrix covering capability approval, network activation, session lifecycle, shell/wrap execution, and teardown
- [ ] M5-04 — Improve inspection outputs so they expose actual backend state and evidence confidence, not just static configuration

**Success Criteria:**

1. `learn` output is no longer blanket-`readwrite` where a better classification is feasible.
2. Windows-host tests validate ETW mapping on representative cases.
3. Audit outputs cover the security-relevant control paths that Windows claims to support.
4. Inspection surfaces can be used to answer “what was enforced?” and “what was approved?” without reading code.

---

### M6: Primitive Boundary

**Goal:** Establish a coherent Windows primitive layer so the Windows backend is no longer just “CLI validation plus careful launch.”

**Depends on:** M0, M2, M3

**Requirements:** PRIMITIVE-01

**Plans:**

- [ ] M6-01 — Define the Windows primitive composition: token model, integrity model, Job Object policy, process mitigation policy, handle inheritance rules, and network attachment semantics
- [ ] M6-02 — Decide whether `Sandbox::apply()` on Windows becomes a real primitive application path or whether Windows formally adopts a launcher-owned primitive with narrower library guarantees
- [ ] M6-03 — Implement the chosen primitive boundary and encode its guarantees in `support_info()` and library-facing docs
- [ ] M6-04 — Add adversarial validation for common Windows escape shapes: symlink/junction/reparse, UNC/device paths, ADS, search-order abuse, inherited handles, descendant processes

**Success Criteria:**

1. Windows has a clearly-defined primitive boundary with auditable guarantees.
2. The library and CLI no longer imply different security models.
3. Adversarial validation demonstrates fail-closed behavior on common Windows escape shapes.
4. The project can state, without ambiguity, whether Windows has reached primitive-level parity or a narrower but explicit contract.

---

### M7: Runtime Trust Enforcement

**Goal:** Resolve the largest remaining gap between Windows and the full safe-layer end state: runtime mediation of sensitive file opens during execution.

**Depends on:** M0, M5, M6

**Requirements:** TRUST-01

**Paths:**

- **Path A: Full implementation**
  - signed kernel minifilter project
  - driver/service/user-mode coordination
  - policy and audit integration into the Windows control plane
- **Path B: Explicit product narrowing**
  - formalize that Windows provides pre-exec trust verification only
  - define compensating controls and exact marketing/product wording
  - prove the narrowed Windows claim is still internally coherent

**Plans:**

- [ ] M7-01 — Final architecture decision: implement minifilter or formalize the narrowed claim
- [ ] M7-02A — If implementing: create the kernel-driver project, signing path, IPC contract, policy model, and audit surface
- [ ] M7-03A — If implementing: integrate runtime trust mediation into Windows supervised execution and add end-to-end tests
- [ ] M7-02B — If narrowing: update all product claims, support matrices, and trust docs to state the exact Windows trust boundary and compensating controls

**Success Criteria:**

1. Windows runtime trust behavior is no longer ambiguous.
2. If full implementation is chosen, runtime file-open mediation exists and is audited.
3. If narrowing is chosen, the narrowed Windows claim is explicit everywhere and supported by compensating controls.
4. The project can make a truthful statement about whether Windows has reached the full safe-layer end state or a consciously narrower edition.

---

## Release Gates

These gates should block any claim that Windows has reached the safe-layer end state.

### Gate 1: Contract Gate

Must be true:

1. M0 and M1 are complete.
2. There is no truth-surface drift in-tree.

### Gate 2: Capability Gate

Must be true:

1. M2 is complete.
2. Runtime capability control is either fully supported or fully documented as out of scope.

### Gate 3: Network Gate

Must be true:

1. M3 is complete.
2. Windows-host tests prove enforcement and fail-closed behavior for all claimed network modes.

### Gate 4: Primitive Gate

Must be true:

1. M6 is complete.
2. The Windows primitive boundary is documented and adversarially tested.

### Gate 5: Trust Gate

Must be true:

1. M7 is resolved.
2. Product claims reflect the actual trust model with no ambiguity.

---

## Suggested Execution Order

### Wave 1: Define and Align

- M0 Security Contract
- M1 Truth Surface

### Wave 2: Strengthen Host Control

- M2 Runtime Capability Control
- M3 Network Mediation

### Wave 3: Operational Coherence

- M4 Session Control Plane
- M5 Evidence Quality

### Wave 4: Boundary Completion

- M6 Primitive Boundary

### Wave 5: Final Trust Resolution

- M7 Runtime Trust Enforcement

---

## Open Decisions

These must be answered before or during M0:

1. Is Windows required to reach primitive-level parity, or is a narrower launcher-owned Windows boundary acceptable?
2. Is runtime trust interception mandatory for the Windows edition to claim “safe layer for agentic software”?
3. If runtime trust interception is deferred, what exact Windows claim is still acceptable?
4. Which components are allowed to become part of the Windows trusted computing base: helper service, WFP service, driver, future minifilter?
5. Which Windows-host integration tests are mandatory before broad release?

---

## Progress Table

| Milestone | Plans Complete | Status | Completed |
|-----------|----------------|--------|-----------|
| M0. Security Contract | 0/3 | Not Started | — |
| M1. Truth Surface | 0/3 | Not Started | — |
| M2. Runtime Capability Control | 0/3 | Not Started | — |
| M3. Network Mediation | 0/4 | Not Started | — |
| M4. Session Control Plane | 0/3 | Not Started | — |
| M5. Evidence Quality | 0/4 | Not Started | — |
| M6. Primitive Boundary | 0/4 | Not Started | — |
| M7. Runtime Trust Enforcement | 0/4 | Not Started | — |
