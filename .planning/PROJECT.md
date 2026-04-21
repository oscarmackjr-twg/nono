# nono - Windows Gap Closure

## Current State (post-v2.0)

**Shipped:** v2.0 Windows Gap Closure (2026-04-18, with one documented known-issue carry-forward). All 7 feature gaps closed in code; live-UAT carry-forward is the detached-supervisor + ConPTY + restricted-token `0xC0000142` interaction on sandboxed console grandchildren. See `.planning/milestones/v2.0-ROADMAP.md`.

**Active (candidate v2.1):** Phase 15 — detached console + ConPTY architecture investigation. Unblocks the 4 v2.0-known-issue UAT items (P05-HV-1, P07-HV-3, P11-HV-1, P11-HV-3).

## Previously Shipped

- v1.0 Windows Alpha (2026-03-31, tag `v1.0`) — signed release artifacts, WFP service packaging, supervisor parity, snapshot/rollback, MSI packaging.

---

## What This Is

nono is a capability-based sandboxing system for running untrusted AI agents with OS-enforced isolation. This project focuses on bringing the Windows implementation to full cross-platform parity with Linux and macOS, covering supervisor lifecycle, kernel-level network enforcement, interactive shell hosting, path discovery, and developer tooling.

## Core Value

Windows security must be as structurally impossible and feature-complete as Unix platforms, ensuring the dangerous bits are kernel-enforced without compromising the supervisor-led security model.

## Requirements

### Validated

- ✔ Landlock sandbox (Linux) — core library
- ✔ Seatbelt sandbox (macOS) — core library
- ✔ Windows capability subset enforcement (WFP network + Low Integrity filesystem)
- ✔ CLI capability builder (`--allow`, `--read`, `--block-net`, profile-backed policy)
- ✔ Built-in profiles (claude-code, codex, opencode, openclaw, swival)
- ✔ Windows alignment (WIN-1706): Library/CLI contract unified
- ✔ Windows release automation (signed .exe, machine MSI, user MSI, zip)
- ✔ C FFI bindings (nono-ffi)
- ✔ Windows CI lanes (build, smoke, integration, security, parity-regression, packaging)
- ✔ Supervisor parity (attach, detach, ps, stop) — v1.0 Phases 1–2
- ✔ WFP promotion to primary enforced network backend — v1.0 Phase 06
- ✔ Snapshot/rollback for Windows filesystems — v1.0 Phase 4
- ✔ MSI packaging and code signing automation — v1.0 Phase 4
- ✔ **WRAP-01** — `nono wrap` on Windows (Direct strategy + Job Object + WFP + canonical help text) — v2.0 Phases 07, 14-02
- ✔ **SESS-01/02/03** — `nono logs`, `nono inspect`, `nono prune` on Windows session records — v2.0 Phase 07 (SESS-03 live UAT waived as v2.0-known-issue)
- ✔ **SHELL-01** — `nono shell` interactive ConPTY on Windows 10 17763+ — v2.0 Phase 08
- ✔ **PORT-01** — port-level WFP allowlists (`--allow-port`, bind/connect) — v2.0 Phase 09
- ✔ **PROXY-01** — proxy credential injection via `--network-profile` / `--credential` / `--upstream-proxy` (runbook corrected in Phase 14-03) — v2.0 Phase 09; live UAT waived as `no-test-fixture`
- ✔ **LEARN-01** — `nono learn` on Windows via ETW — v2.0 Phase 10
- ✔ **TRUST-01** *(stretch)* — runtime capability expansion over named pipe — v2.0 Phase 11 (live supervised UAT waived as v2.0-known-issue)

### Active (v2.1 candidate)

- [ ] Detached-supervisor + ConPTY + restricted-token architecture fix so sandboxed console grandchildren stop failing DLL init with `STATUS_DLL_INIT_FAILED (0xC0000142)` — Phase 15. Unblocks 4 v2.0-known-issue UAT items.
- [ ] **RESL-01/02** — CPU and memory limits on Windows Job Objects (deferred from v2.0).
- [ ] **AIPC-01** — secure handle brokering via Named Pipe IPC (deferred from v2.0).

### Out of Scope

- Gap 6b (runtime trust interception via kernel minifilter) — requires signed kernel driver; deferred to v3.0.
- Full feature parity for experimental Unix features not yet stabilized.
- Job Object nesting; global kernel walk (documented in v2.0-REQUIREMENTS.md archive).

## Context

- Windows parity is the current "honesty gap" in the product; users expect the same CLI experience across all supported OSs.
- The technically challenging core of this milestone is the Supervisor IPC (named pipes) and WFP driver/service orchestration.
- Previous work (PRs 530, 555, 583) has laid the foundation for native Windows functionality.
- Dark factory rules apply: fail closed, no silent fallback, no broadening claims beyond enforcement.

## Constraints

- **Security**: Fail secure on any unsupported shape â€” never silently degrade.
- **Compatibility**: Must support Windows 10/11 (modern Job Objects and WFP).
- **Performance**: Zero startup latency must be maintained for the Windows backend.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Supervisor Parity as Priority | Essential for "attach/detach" workflow used by long-running agents. | ✔ Good — attach/detach/ps/stop shipped in v1.0; v2.0 extended with `nono shell`, `nono wrap`, session commands |
| WFP over Temporary Firewall | Kernel-level enforcement is the "nono way"; temporary rules are a stopgap. | ✔ Complete — Phase 06 wired SID end-to-end, removed driver gate, cleaned duplicate activation path |
| Intentional `shell`/`wrap` omission | Lack of credible enforcement model on Windows; avoiding security over-claims. | ↶ Reversed in v2.0 — both now shipped with Job Object + WFP + ConPTY enforcement |
| Named Job Objects | Agent lifecycle management with atomic stop/list. | ✔ Good — v1.0 foundation |
| WRITE_RESTRICTED token | Narrow the restricting-SID access-check gate to writes only so DLL loads and console init aren't blocked. | ✔ Good — fixes Bug #2 (`STATUS_ACCESS_DENIED`); residual Bug #3 on detached console grandchildren is the v2.0-known-issue |
| Ship v2.0 with detached-console-grandchild bug as a documented known issue | Three fix directions attempted in Phase 14 plan 14-01 all failed the user smoke gate; real fix requires PTY + detached-supervisor architecture work which is its own investigation phase. Non-detached mode fully functional. | ⚠️ Revisit — close in Phase 15 (candidate v2.1) |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? â†’ Move to Out of Scope with reason
2. Requirements validated? â†’ Move to Validated with phase reference
3. New requirements emerged? â†’ Add to Active
4. Decisions to log? â†’ Add to Key Decisions
5. "What This Is" still accurate? â†’ Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check â€” still the right priority?
3. Audit Out of Scope â€” reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-04-18 — Milestone v2.0 Windows Gap Closure shipped (with detached-console-grandchild known-issue carry-forward to Phase 15)*
