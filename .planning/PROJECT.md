# nono - Windows Parity & Quality

## Current Milestone: v2.1 — Resource Limits, Extended IPC, Attach-Streaming & Cleanup

**Goal:** Deliver Job Object resource limits (CPU / memory / timeout / process-count), extend the Phase 11 capability pipe to broker additional handle types, finish the Phase 15 attach-streaming gap with full ConPTY re-attach on detached Windows sessions, and clean up accumulated v2.0 WIP.

**Target features:**
- Resource limits on Windows Job Objects (CPU %, memory cap, wall-clock timeout, process count)
- Extended IPC handle brokering (socket / pipe / Job Object / event / mutex handles)
- Full ConPTY re-attach on detached Windows sessions (read + write + resize) — closes Phase 15's deferred attach-streaming
- Cleanup workstream: fmt drift, Windows test flakes, WIP triage, session-file housekeeping

## Previously Shipped

- v2.0 Windows Gap Closure (2026-04-18, tag pending on merge; closed 2026-04-18 with Phase 15) — 7 Windows feature gaps closed (`nono wrap`, session commands, ConPTY shell, port-level WFP, proxy credential injection, ETW `learn`, runtime capability expansion stretch). Phase 15 closed the detached-console-grandchild `0xC0000142` carry-forward via direction-b fix (gated PTY-disable + null-token + AppID WFP on detached path only).
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
- ✔ **TRUST-01** *(stretch)* — runtime capability expansion over named pipe — v2.0 Phase 11 (live supervised UAT promoted to pass by Phase 15 direction-b fix)
- ✔ **DETACHED-FIX-01** — detached-supervisor + ConPTY + restricted-token architecture fix (direction-b: gated PTY-disable + null-token + AppID WFP on the Windows detached path). Unblocks 4 Phase 13 UAT items (P05-HV-1, P07-HV-3, P11-HV-1, P11-HV-3) — all promoted to `pass`. v2.1 Phase 15 (the Phase 15 carrier moved into the v2.1 milestone bucket on scoping day 2026-04-18).
- ✔ **RESL-01** — CPU percentage cap on Windows Job Object (`--cpu-percent`) via `JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP`. Validated in Phase 16: Resource Limits.
- ✔ **RESL-02** — Memory cap on Windows Job Object (`--memory`) via `JobMemoryLimit` with `KILL_ON_JOB_CLOSE` preserved. Validated in Phase 16: Resource Limits.
- ✔ **RESL-03** — Wall-clock timeout (`--timeout`) via supervisor-side `Instant` deadline + `TerminateJobObject` (kernel `JOB_TIME` deliberately not used since it tracks CPU not wall-clock). Validated in Phase 16: Resource Limits.
- ✔ **RESL-04** — Process count cap (`--max-processes`) via `ActiveProcessLimit`. Validated in Phase 16: Resource Limits. `nono inspect` surfaces all four caps via the new `Limits:` block.
- ✔ **ATCH-01** — `nono attach <id>` on Windows detached sessions streams child stdout live, accepts stdin, supports clean detach (Ctrl-]d) + re-attach, and rejects a 2nd concurrent attach with a friendly busy error. Implemented via anonymous-pipe stdio at child spawn time bridged through the supervisor (no ConPTY on the detached path — preserves the Phase 15 `0xC0000142` fix structurally). Resize via `ResizePseudoConsole` explicitly downgraded to a documented limitation per D-07 (anonymous-pipe stdio is structurally exclusive of ConPTY). Validated in Phase 17 with pragmatic-PASS verdict on the manual smoke gate; 4 deferred-by-design HUMAN-UAT items routed to `/gsd-verify-work` for later closure.

### Active (v2.1)

- [ ] **AIPC-01** — extended handle brokering on the Phase 11 capability pipe: socket handles, named-pipe handles, Job Object handles, event handles, mutex handles. Each with the correct `DuplicateHandle` inheritance/security semantics and access-mask validation.
- [ ] **CLEAN-01** — `cargo fmt --all` the 3 pre-existing drifted files from commit `6749494` (EnvVarGuard migration); restore CI `fmt --check` to clean.
- [ ] **CLEAN-02** — diagnose and fix 5 pre-existing Windows test flakes in `capability_ext`, `profile::builtin`, `query_ext`, `trust_keystore`. Likely env-var isolation bugs.
- [ ] **CLEAN-03** — triage disk-resident WIP: `10-RESEARCH.md`/`10-UAT.md`, `11-01/02-PLAN.md` modifications, `12-02-PLAN.md`, `.planning/quick/260410-nlt-*`, `.planning/quick/260412-ajy-*`, `.planning/v1.0-INTEGRATION-REPORT.md`, stray root files (`host.nono_binary.commit`, `query`). Commit alive work, remove dead artifacts.
- [ ] **CLEAN-04** — session-file housekeeping: prune the 1172 stale session records accumulated during v2.0 testing; document the retention policy so this doesn't re-accumulate.

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
| Ship v2.0 with detached-console-grandchild bug as a documented known issue | Three fix directions attempted in Phase 14 plan 14-01 all failed the user smoke gate; real fix requires PTY + detached-supervisor architecture work which is its own investigation phase. Non-detached mode fully functional. | ✔ Resolved by Phase 15 (direction-b: gated PTY-disable + null-token + AppID WFP) on 2026-04-18 |
| Direction-b scoped waivers for detached Windows path (Phase 15) | The only empirically-working configuration is null token + no PTY. Non-detached keeps WRITE_RESTRICTED + session-SID + ConPTY unchanged. Low-IL isolation waived on detached path (Job Object + filesystem sandbox remain primary); per-session-SID WFP replaced by AppID WFP on detached path (still kernel-enforced; requires nono-wfp-service). | ✔ Good — waivers documented in commit `802c958` body; scope strictly detached-only |

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
*Last updated: 2026-04-19 — Phase 17 complete (ATCH-01 shipped: anonymous-pipe stdio for `nono attach` on Windows detached sessions; resize on detached path explicitly downgraded per D-07). v2.1 active phases remaining: AIPC-01.*
