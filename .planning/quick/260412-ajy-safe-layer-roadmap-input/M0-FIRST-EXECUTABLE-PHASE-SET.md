# M0 First Executable Phase Set

This phase set takes **M0: Security Contract** from roadmap intent to executable planning. It is intentionally narrow: the goal is to define and ratify the Windows security contract before more implementation work expands the surface area further.

Primary output:

- `WINDOWS-SECURITY-CONTRACT.md`

Secondary outputs:

- canonical Windows support matrix source
- initial CI enforcement for truth-surface drift

---

## Phase Set Goal

Produce and ratify the first authoritative Windows security contract for `nono`, so that:

1. engineering knows the exact Windows boundary
2. docs/help/tests can be aligned to one source of truth
3. later roadmap milestones inherit explicit guarantees and non-claims

---

## Scope

In scope:

- trusted computing base definition
- enforcement boundary definition
- library vs CLI contract on Windows
- current supported surface vs unsupported surface
- explicit guarantees and explicit non-guarantees
- runtime trust decision framing
- support-matrix ownership

Out of scope:

- implementing new Windows features
- rewriting the helper/service architecture
- deciding final wording for marketing pages
- kernel minifilter implementation

---

## Phase Breakdown

### Phase M0.1: Contract Draft

**Goal:** Write the first complete Windows contract artifact.

**Inputs:**

- `260412-ajy-SAFE-LAYER-ROADMAP-INPUT.md`
- `WINDOWS-SAFE-LAYER-ROADMAP.md`
- current Windows code and help/docs surface

**Tasks:**

- [ ] Draft `WINDOWS-SECURITY-CONTRACT.md`
- [ ] Define Windows trusted computing base
- [ ] Define current guarantees and non-claims
- [ ] Define current supported command surface
- [ ] Define contract-level open decisions

**Exit Criteria:**

1. The contract can be read without reading code.
2. The contract clearly separates current state from end-state aspirations.
3. The contract names the biggest architectural fork: runtime trust interception.

---

### Phase M0.2: Support Matrix Source

**Goal:** Make the contract operational by choosing a single support-matrix source.

**Tasks:**

- [ ] Decide where the authoritative Windows support matrix lives
- [ ] Define which artifacts must derive from or validate against it
- [ ] Record update rules for future Windows surface changes

**Exit Criteria:**

1. There is one canonical support matrix source.
2. README/help/setup/tests are all listed as downstream consumers.
3. Future roadmap milestones can reference this source directly.

---

### Phase M0.3: Contract Review and Ratification

**Goal:** Convert the draft into a reviewed contract suitable for driving roadmap work.

**Tasks:**

- [ ] Review contract against current code surface
- [ ] Resolve obvious truth-surface mismatches captured by the contract
- [ ] Record unresolved strategic decisions rather than leaving them implicit
- [ ] Mark the contract as the source artifact for M1

**Exit Criteria:**

1. The contract is accepted as the planning baseline for M1 through M7.
2. Unresolved issues are explicit decision items, not hidden assumptions.
3. The roadmap can now refer to contract sections rather than broad architectural prose.

---

## Deliverables

| Deliverable | Purpose |
|-------------|---------|
| `WINDOWS-SECURITY-CONTRACT.md` | Primary contract artifact |
| support matrix decision note | Defines the source of truth for Windows claims |
| contract review notes | Captures unresolved strategic decisions |

---

## Required Decisions

These decisions should be made during M0, even if implementation happens later:

1. Is Windows targeting primitive-level parity or a narrower launcher-owned boundary?
2. Is runtime trust interception required for the Windows edition to claim the full safe-layer end state?
3. If runtime trust interception is deferred, what exact narrower Windows claim is acceptable?
4. Which helper components are in the Windows trusted computing base?
5. What is the canonical support-matrix source?

---

## Recommended Sequencing

1. Complete M0.1 first.
2. Immediately follow with M0.2 so the contract has operational force.
3. Finish with M0.3 review before starting M1.

Do not start truth-surface cleanup work from memory or ad hoc judgement. Use the contract text as the baseline.

---

## Hand-off to M1

M1 should begin only after:

1. `WINDOWS-SECURITY-CONTRACT.md` exists
2. the contract names the current Windows supported surface
3. the support matrix source is chosen
4. unresolved architectural forks are captured explicitly

At that point M1 can become largely mechanical: align README, CLI help, setup output, and tests to the contract.
