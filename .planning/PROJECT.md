# nono - Windows Parity & Quality

## Current State

**Shipped:** v2.1 — Resource Limits, Extended IPC, Attach-Streaming & Cleanup (2026-04-21, tag `v2.1`).

v2.1 closed the live `nono run --profile claude-code` path on Windows end-to-end: per-file Low-IL mandatory-label filesystem grants (unblocking the profile's `git_config` group), AIPC handle brokering for Socket/Pipe/JobObject/Event/Mutex with `capabilities.aipc` profile widening wired end-to-end, anonymous-pipe-stdio `nono attach` on detached sessions, Job Object resource caps (CPU/memory/timeout/process-count) surfaced in `nono inspect`, upstream v0.37.1 parity sync (including RUSTSEC-2026-0098/0099 `rustls-webpki` fix), and a cleanup workstream that paid down v2.0 fmt/test/WIP/session-file debt.

## Next Milestone Goals

No milestone locked yet. Candidate focus areas surfaced during v2.1:

- **WR-01 reject-stage unification** (currently deferred to v2.2) — align all 5 AIPC HandleKinds on the same reject stage (BEFORE vs AFTER prompt) instead of the current Event/Mutex/JobObject-before + Pipe/Socket-after split. Product decision.
- **AIPC G-04 wire-protocol compile-time tightening** (D-09 / D-11, deferred to v2.2) — `Approved(ResourceGrant)` inline at the wire type so `(Approved, grant=None)` becomes a compile-time error; removes the child SDK demultiplexer defense-in-depth branch.
- **Cross-platform resource limits** — native Unix backends for RESL-01..04 (cgroup v2 `cpu.max` / `memory.max` / `pids.max`; macOS equivalents where practical).
- **WR-02 EDR telemetry item 3** (deferred to v3.0) — rerun HUMAN-UAT on an EDR-instrumented host.
- **Merge v2.0 + v2.1 to main** — `windows-squash` branch holds both milestones; publish PR after resolving any DCO/signoff items carried from v2.0 PR 555.

<details>
<summary>Previously Shipped</summary>

- **v2.1 Resource Limits, Extended IPC, Attach-Streaming & Cleanup** (2026-04-21, tag `v2.1`) — 7 phases (16–21 + 18.1), 25 plans, 13 requirements (RESL, AIPC, ATCH, CLEAN, UPST, WSFG).
- **v2.0 Windows Gap Closure** (2026-04-18, tag `v2.0`; closed 2026-04-18 with Phase 15) — 7 Windows feature gaps closed (`nono wrap`, session commands, ConPTY shell, port-level WFP, proxy credential injection, ETW `learn`, runtime capability expansion stretch). Phase 15 closed the detached-console-grandchild `0xC0000142` carry-forward via direction-b fix (gated PTY-disable + null-token + AppID WFP on detached path only).
- **v1.0 Windows Alpha** (2026-03-31, tag `v1.0`) — signed release artifacts, WFP service packaging, supervisor parity, snapshot/rollback, MSI packaging.

</details>

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
- ✔ **ATCH-01** — `nono attach <id>` on Windows detached sessions streams child stdout live, accepts stdin, supports clean detach (Ctrl-]d) + re-attach, and rejects a 2nd concurrent attach with a friendly busy error. Implemented via anonymous-pipe stdio at child spawn time bridged through the supervisor (no ConPTY on the detached path — preserves the Phase 15 `0xC0000142` fix structurally). Resize via `ResizePseudoConsole` explicitly downgraded to a documented limitation per D-07 (anonymous-pipe stdio is structurally exclusive of ConPTY). — v2.1 Phase 17.
- ✔ **AIPC-01** — Extended handle brokering on the Phase 11 capability pipe: Socket, Pipe, Job Object, Event, Mutex handles with `DuplicateHandle` MAP-DOWN semantics + access-mask validation + `capabilities.aipc` profile widening end-to-end (Profile threaded through `PreparedSandbox → LaunchPlan → execute_sandboxed → SupervisedRuntimeContext → WindowsSupervisorRuntime.resolved_aipc_allowlist`). Containment-Job runtime guard via `CompareObjectHandles`. Cross-platform child-side SDK with 5 `request_*` methods. — v2.1 Phases 18 + 18.1 (HUMAN-UAT item 3 WR-02 EDR deferred to v3.0).
- ✔ **CLEAN-01..04** — `cargo fmt --all` drift fix; 4 deterministic Windows test bugs fixed incl. UNC-prefix production bug in `query_path`; 10 WIP items triaged (6 backfilled, 2 reverted, 2 deleted); `is_prunable` retention predicate + `nono prune --older-than <DURATION>` + `--all-exited` + auto-sweep on `nono ps` (100-file threshold) + `NONO_CAP_FILE` structural no-op + one-shot cleanup of 1343 stale session files + `docs/session-retention.md`. — v2.1 Phase 19.
- ✔ **UPST-01** — `rustls-webpki` upgraded to 0.103.12 (clears RUSTSEC-2026-0098 + RUSTSEC-2026-0099); workspace crate versions bumped 0.30.1 → 0.37.1 across all 4 members. — v2.1 Phase 20.
- ✔ **UPST-02** — Upstream profile `extends` cycle guard + claude-code `.claude.json` symlink for token refresh. — v2.1 Phase 20.
- ✔ **UPST-03** — `keyring://service/account` URI + `?decode=go-keyring` + environment-variable filter flags + `command_blocking_deprecation` backport. — v2.1 Phase 20.
- ✔ **UPST-04** — `--allow-gpu` flag with 3-platform dispatch (Linux Landlock NVIDIA/DRM/AMD/WSL2 + NVIDIA procfs, macOS Seatbelt IOKit, Windows CLI-layer warning); GitLab ID tokens for trust signing with `validate_oidc_issuer` fail-closed validator. — v2.1 Phase 20.
- ✔ **WSFG-01** — `compile_filesystem_policy` emits rules for single-file Read/Write/ReadWrite + write-only-directory grants; `apply()` applies `SYSTEM_MANDATORY_LABEL_ACE` at `SECURITY_MANDATORY_LOW_RID` via `SetNamedSecurityInfoW` with mode-derived mask per D-01 encoding table. — v2.1 Phase 21.
- ✔ **WSFG-02** — `NonoError::LabelApplyFailed { path, hresult, hint }` + `AppliedLabelsGuard` RAII lifecycle wired into `prepare_live_windows_launch` (revert on `Drop`); ownership pre-check in `try_set_mandatory_label` skips system-owned paths (`C:\Windows`). — v2.1 Phase 21.
- ✔ **WSFG-03** — Phase 18 HUMAN-UAT Path B + Path C close-out; frontmatter transition achieved; live-CONIN$ pass verdicts folded into Phase 18.1 HUMAN-UAT items 1+2 pass via live dual-run. — v2.1 Phase 21 + 18.1.

### Active (v2.2+)

(No milestone locked. See "Next Milestone Goals" at top for candidate focus areas: WR-01 reject-stage unification, AIPC G-04 wire-protocol compile-time tightening, cross-platform RESL Unix backends, WR-02 EDR HUMAN-UAT item, merge `windows-squash` to main.)

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

## Context

Shipped v2.1 on 2026-04-21 on `windows-squash` branch. Tech stack: Rust 1.77 (Edition 2021) across a 4-crate workspace (`nono`, `nono-cli`, `nono-proxy`, `nono-ffi`). Key runtime deps: `tokio` 1, `hyper` 1, `landlock` 0.4, `windows-sys` 0.59, `sigstore-rs`, `rustls-webpki` 0.103.12 (post-RUSTSEC-2026-0098/0099 upgrade). Windows-specific: WFP network enforcement, ConPTY interactive shell (Win10 17763+), ETW for `nono learn`, Low-IL mandatory-label filesystem sandboxing. Cross-platform: Landlock (Linux), Seatbelt (macOS), capability builder API with per-platform compile-down.

Workspace-internal LOC is growing but contained: v2.1 added ~17k lines across code + docs with the bulk in AIPC scaffolding, child SDK, WSFG label primitives, and upstream-parity ports. Branch `windows-squash` holds both v2.0 and v2.1; merge-to-main is a candidate task for the next milestone.

Feedback/observations from v2.1 shipping:
- AIPC handle brokering surfaced the need for end-to-end Profile wiring (Plan 18.1-03 closed the gap).
- Windows 11 26200 empirical finding: `WRITE_RESTRICTED` pipes need a logon-SID co-requirement ACE — MSDN-undocumented. Harness (`examples/pipe-repro.rs`) is now available for future SDDL investigations.
- WR-01 reject-stage asymmetry (Event/Mutex/JobObject reject BEFORE prompt, Pipe/Socket reject AFTER prompt) is a product decision, not a bug. Locked by `wr01_*` regression tests; unification deferred to v2.2.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Supervisor Parity as Priority | Essential for "attach/detach" workflow used by long-running agents. | ✔ Good — attach/detach/ps/stop shipped in v1.0; v2.0 extended with `nono shell`, `nono wrap`, session commands; v2.1 added live-stream attach on detached path (Phase 17) |
| WFP over Temporary Firewall | Kernel-level enforcement is the "nono way"; temporary rules are a stopgap. | ✔ Complete — Phase 06 wired SID end-to-end, removed driver gate, cleaned duplicate activation path |
| Intentional `shell`/`wrap` omission | Lack of credible enforcement model on Windows; avoiding security over-claims. | ↶ Reversed in v2.0 — both now shipped with Job Object + WFP + ConPTY enforcement |
| Named Job Objects | Agent lifecycle management with atomic stop/list. | ✔ Good — v1.0 foundation; v2.1 Phase 16 extended with CPU/memory/timeout/process-count caps |
| WRITE_RESTRICTED token | Narrow the restricting-SID access-check gate to writes only so DLL loads and console init aren't blocked. | ✔ Good — fixes Bug #2 (`STATUS_ACCESS_DENIED`); residual Bug #3 on detached console grandchildren resolved by Phase 15 |
| Ship v2.0 with detached-console-grandchild bug as a documented known issue | Three fix directions attempted in Phase 14 plan 14-01 all failed the user smoke gate; real fix requires PTY + detached-supervisor architecture work which is its own investigation phase. Non-detached mode fully functional. | ✔ Resolved by Phase 15 (direction-b: gated PTY-disable + null-token + AppID WFP) on 2026-04-18 |
| Direction-b scoped waivers for detached Windows path (Phase 15) | The only empirically-working configuration is null token + no PTY. Non-detached keeps WRITE_RESTRICTED + session-SID + ConPTY unchanged. Low-IL isolation waived on detached path (Job Object + filesystem sandbox remain primary); per-session-SID WFP replaced by AppID WFP on detached path (still kernel-enforced; requires nono-wfp-service). | ✔ Good — waivers documented in commit `802c958` body; scope strictly detached-only |
| Phase 17 ATCH-01 anonymous-pipe stdio over ConPTY on detached path (D-07 resize downgrade) | ConPTY at detached-launch time trips Phase 15's `0xC0000142`; anonymous pipes preserve the fix structurally. Resize via `ResizePseudoConsole` is therefore unreachable; downgraded to documented limitation. | ✔ Good — `nono attach` streams live output + stdin; users needing full TUI fidelity use `nono shell` or non-detached `nono run` |
| AIPC `HandleKind` discriminators 0..=5 PINNED (Phase 18) | Wire-format stability lock so future cross-platform handle brokering doesn't need a migration. | ✔ Good — File=0, Socket=1, Pipe=2, JobObject=3, Event=4, Mutex=5 locked |
| AIPC access-mask MAP DOWN, not DUPLICATE_SAME_ACCESS (Phase 18) | `broker_*_to_process` pass `dwOptions=0` + explicit mask so child handle is the validated subset, not supervisor source's full ALL_ACCESS. | ✔ Good — T-18-01-11 mitigation; access-mask validation happens server-side |
| AIPC broker-failure flip via flow-control enforcement, not type-level (G-04 / Plan 18.1-02) | `Approved(ResourceGrant)` compile-time tightening would cascade into 23 pre-existing tests + child SDK demultiplexer; single-site `(decision, grant)` tuple construction keeps the shape illegal at the flow-control boundary. | ⚠️ Revisit v2.2 — D-09 + D-11 wire-protocol compile-time tightening deferred |
| AIPC privileged-port unconditional deny (Phase 18) | Socket broker rejects `port <= 1023` BEFORE any profile-widening check; cannot be widened by `capabilities.aipc`. | ✔ Good — structural; CONTEXT.md D-05 footnote |
| WRITE_RESTRICTED capability pipe requires logon-SID co-requirement ACE (Phase 21 debug) | Windows 11 26200's second-pass DACL access check requires BOTH a restricting-SID ACE AND a `SE_GROUP_MANDATORY` group-SID ACE. `OW` Owner Rights does NOT satisfy the co-requirement. | ✔ Empirical — fix in `build_capability_pipe_sddl` via `current_logon_sid()` helper (commit `938887f`); MSDN-undocumented; harness at `crates/nono-cli/examples/pipe-repro.rs` |
| CLEAN-04 auto-sweep threshold = 100 stale files + `NONO_CAP_FILE` structural no-op | Prevents sandboxed agent calling `nono ps` from triggering host-side session-file deletion. `--older-than` require-suffix parser rejects ambiguous bare integers. | ✔ Good — one-shot cleanup from 1392 to 49 stale files on dev host |
| Phase 20 `--allow-gpu` capability-routing deviation from upstream | Fork routes through `CapabilitySet` + sandbox backend layer, not upstream's `sandbox_prepare.rs::maybe_enable_*gpu` (fork 452 LOC vs upstream 1585 LOC — cherry-pick conflicts would dominate). D-21 Windows-invariance held (zero `*_windows.rs` touched). | ✔ Good — manual port preserved commit provenance via `Upstream-commit:` trailer |
| Phase 21 Low-IL ownership pre-check in `try_set_mandatory_label` (commit `da25619`) | Low-IL integrity is subtractive; Medium-IL system paths (e.g. `C:\Windows`) are already readable to Low-IL subjects through OS ACLs, so labeling them is unnecessary AND trips `ERROR_ACCESS_DENIED` for unprivileged users. | ✔ Good — inline comment preserved in source for future readers |
| WR-01 reject-stage asymmetry accepted as product decision (Plan 18.1-04, CONTEXT D-14) | Event/Mutex/JobObject reject BEFORE prompt (pre-broker mask gate); Pipe/Socket reject AFTER prompt (G-04-wrapped; direction/role/host checks post-approval). Locked by `wr01_*` regression tests. | ⚠️ Revisit v2.2 — stage unification requires product decision, not bug fix |

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
*Last updated: 2026-04-21 after v2.1 milestone. All 13 v2.1 requirements closed (RESL-01..04, ATCH-01, AIPC-01, CLEAN-01..04, UPST-01..04, WSFG-01..03). Deferred to v2.2+: WR-01 reject-stage unification, AIPC G-04 wire-protocol compile-time tightening, cross-platform RESL Unix backends, WR-02 EDR HUMAN-UAT item, merge `windows-squash` to main.*
