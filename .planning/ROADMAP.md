# Roadmap: nono Windows Parity & Quality

This roadmap tracks the path to full Windows/Unix parity and ongoing quality-of-life work for `nono`.

## Milestones

- ✅ **v1.0 Windows Alpha** — Phases 1–4 (shipped 2026-03-31; tag `v1.0`)
- ✅ **v2.0 Windows Gap Closure** — Phases 5–14 (shipped 2026-04-18; tag `v2.0` pending merge)
- ✅ **Phase 15 closure** — completed 2026-04-18; closed the v2.0 known-issue carry-forward
- 🚧 **v2.1 Resource Limits, Extended IPC, Attach-Streaming & Cleanup** — Phases 16–19 (scoped 2026-04-18)

## Phases

<details>
<summary>✅ v1.0 Windows Alpha (Phases 1–4) — SHIPPED 2026-03-31</summary>

- [x] Phase 1: Windows Control Foundation (3/3 plans) — completed 2026-04-04
- [x] Phase 2: Persistent Sessions — Detach/Attach (4/4 plans) — completed 2026-04-04
- [x] Phase 3: Network Sandboxing — WFP Integration (4/4 plans) — completed 2026-04-04
- [x] Phase 4: State Integrity & Deployment (3/3 plans) — completed 2026-04-05

</details>

<details>
<summary>✅ v2.0 Windows Gap Closure (Phases 5–14) — SHIPPED 2026-04-18 with carry-forward</summary>

- [x] Phase 5: Windows Detach Readiness Fix (1/1 plan) — completed 2026-04-05
- [x] Phase 6: WFP Enforcement Activation (2/2 plans) — completed 2026-04-06
- [x] Phase 7: Quick Wins (2/2 plans) — completed 2026-04-08
- [x] Phase 8: ConPTY Shell (1/1 plan, UAT-driven) — completed 2026-04-10
- [x] Phase 9: WFP Port-Level + Proxy Filtering (4/4 plans) — completed 2026-04-10
- [x] Phase 10: ETW-Based Learn Command (3/3 plans) — completed 2026-04-10
- [x] Phase 11: Runtime Capability Expansion — stretch (2/2 plans) — completed 2026-04-11
- [x] Phase 12: Milestone Bookkeeping Cleanup (3/3 plans) — completed 2026-04-11
- [x] Phase 13: v2.0 Human Verification UAT (1/1 plan) — resolved 2026-04-18 (3 pass, 7 waived; all terminal)
- [x] Phase 14: v2.0 Fix Pass (2/3 plans, 1 escalated) — complete-with-carry-forward 2026-04-18

Carry-forward → Phase 15: detached-console-grandchild `0xC0000142 STATUS_DLL_INIT_FAILED` bug. Affected UAT items P05-HV-1, P07-HV-3, P11-HV-1, P11-HV-3 waived as `v2.0-known-issue`. See `.planning/milestones/v2.0-ROADMAP.md` for the full v2.0 archive.

</details>

<details>
<summary>✅ Phase 15 closure (2026-04-18)</summary>

- [x] **Phase 15: Detached Console + ConPTY Architecture Investigation** — Delivered direction-b architectural pivot: gated PTY-disable + null-token + AppID WFP on the Windows detached path. 5-row smoke gate pass; 4 Phase 13 UAT items promoted to `pass`; Phase 14 carry-forward closed. Fix commits `802c958` + `2c414d8`; bookkeeping `0de3e77`, `eda3d6f`, `bfd3f94`, `034b4d3`, `83e3db0`. Security waivers scoped strictly to the detached path. Attach-streaming deferred to v2.1 ATCH-01.

</details>

### 🚧 v2.1 Resource Limits, Extended IPC, Attach-Streaming & Cleanup (scoped 2026-04-18)

**Goal:** Deliver Job Object resource limits (CPU / memory / timeout / process-count), extend the Phase 11 capability pipe to broker additional handle types, finish the Phase 15 attach-streaming gap with full ConPTY re-attach, and clean up accumulated v2.0 WIP.

**Requirements (13):** RESL-01..04, AIPC-01, ATCH-01, CLEAN-01..04, WSFG-01..03. See `.planning/REQUIREMENTS.md`.

- [x] **Phase 16: Resource Limits (RESL)** — CPU %, memory cap, wall-clock timeout, process count via `JOB_OBJECT_CPU_RATE_CONTROL_ENABLE`, `JobMemoryLimit`, supervisor-timer + `TerminateJobObject` (kernel JOB_TIME deliberately NOT used since it tracks CPU not wall-clock), `ActiveProcessLimit`. CLI flags: `--cpu-percent`, `--memory`, `--timeout`, `--max-processes`. Cross-platform: Unix accepts flags with a "not enforced on this platform" warning pending cross-platform follow-up milestone. `nono inspect` surfaces active caps via a `Limits:` block. **Completed 2026-04-18.**

  **Depends on:** v2.0 Named Job Object infrastructure (Phase 01 / Phase 06).

  **Plans:** 16-01 (CLI flags + Windows enforcement for CPU/memory/processes) + 16-02 (wall-clock timeout timer + observability). Both complete.

- [x] **Phase 17: Attach-Streaming (ATCH)** — Anonymous-pipe stdio bridges in the Windows detached supervisor: `nono attach <id>` now streams child stdout live, accepts stdin from the attach client, supports clean detach (Ctrl-]d) + re-attach, and a second attach client receives a friendly `Session <id> is already attached` error. Resize on detached sessions is **explicitly downgraded to a documented limitation per D-07** (anonymous-pipe stdio is structurally exclusive of ConPTY; preserves the Phase 15 `0xC0000142` fix). Smoke gate executed 2026-04-19 with pragmatic-PASS verdict (G-01 PASS, G-02 PARTIAL PASS, G-03 PASS, G-04 Row 3 PASS + Row 4 environmental + Rows 1/2/5 structurally PASS); 4 deferred items routed to `17-HUMAN-UAT.md`. Surfaced 3 latent pre-Phase-17 Windows session-id bugs which were fixed in commit `7db6595` (corrupted job-name format string in `create_process_containment` + 2 `self.session_id` → `self.user_session_id` fixes in `start_logging`/`start_data_pipe_server`). Verifier passed 13/13 must-haves with `status: human_needed` (deferred items, not gaps). **Completed 2026-04-19.**

  **Depends on:** Phase 15 attach-pipe naming fix (commit `2c414d8`).

  **Resolved (in CONTEXT.md):** D-01 anonymous-pipes-only on detached path; D-07 resize downgraded to documented limitation; D-21 Windows-invariance — zero changes outside `*_windows.rs` files.

  **Plans:** 2 plans.
  - [x] 17-01-PLAN.md — implementation: DetachedStdioPipes + STARTUPINFOW wiring + start_logging/start_data_pipe_server pipe branches + run_attach friendly busy-error + unit/integration tests (complete 2026-04-19, 9 commits `1e38381`..`ecfeba7`; D-02 + D-21 invariance held)
  - [x] 17-02-PLAN.md — manual smoke gate G-01..G-04 + REQUIREMENTS.md ATCH-01 acceptance #3 downgrade + CHANGELOG [Unreleased] entry + docs/cli/features/session-lifecycle.mdx no-resize note + 13-UAT.md P17-HV-1..4 rows (complete 2026-04-19, commit `ab88cf5`; pragmatic-PASS verdict per user)

- [x] **Phase 18: Extended IPC (AIPC)** — Broker socket, named-pipe, Job Object, event, and mutex handles over the Phase 11 capability pipe. Each handle type validated server-side against access-mask allowlist; profile widening via `capabilities.aipc` JSON block; containment-Job runtime guard prevents the supervisor-own-Job footgun structurally. Completed 2026-04-19 with all 4 plans shipped; AIPC-01 feature-complete end-to-end (supervisor brokers + child SDK).

  **Depends on:** Phase 11 capability pipe protocol, Phase 16 (Job Object handle brokering benefits from RESL work landing first).

  **Plans:** 4 plans (Plans 18-01..18-03 sequential by risk axis Wave 1 → Wave 3, each builds on prior's protocol/policy scaffolding; Plan 18-04 lands the child-side SDK surface in Wave 4 closing the CONTEXT.md D-08 + D-09 coverage gap). All complete.
  - [x] 18-01-PLAN.md — Protocol skeleton (HandleKind/HandleTarget enums + extended CapabilityRequest, GrantedResourceKind, ResourceTransferKind, ResourceGrant) + cross-platform policy.rs (per-type access-mask constants + mask_is_allowed validator) + Event/Mutex brokers (lowest-risk handle types — pure DuplicateHandle with fixed mask) + constant-time discriminator validation in handle_windows_supervisor_message + format_capability_prompt File/Event/Mutex templates + 4 integration tests + 2 token-leak tests (complete 2026-04-19, 6 commits 51b1c12..3ea2017)
  - [x] 18-02-PLAN.md — Pipe + Socket brokers (medium-risk; Pipe MAPs DOWN from PIPE_ACCESS_DUPLEX via dwOptions=0 + direction-mapped GENERIC_READ/WRITE; Socket uses WSADuplicateSocketW + WSAPROTOCOL_INFOW blob serialization, role-based validation, privileged-port unconditional reject) + Win32_Networking_WinSock feature in nono crate + bind_aipc_pipe helper reusing Phase 11 SDDL + format_capability_prompt Pipe/Socket templates + 4 integration tests + 2 token-leak tests (complete 2026-04-19, 3 commits 39c5a82/834e534/0a1f2ee)
  - [x] 18-03-PLAN.md — Job Object broker + handle_job_object_request with containment_job runtime guard via CompareObjectHandles (D-05 footnote — refuses supervisor's own Job regardless of profile widening) + profile schema integration (CapabilitiesConfig + AipcConfig + 5 from_token parsers + Profile::resolve_aipc_allowlist + JSON schema $defs/CapabilitiesConfig + 5 built-in profiles updated) + AipcResolvedAllowlist plumbed through dispatcher (replaces hard-coded defaults from 18-01/18-02) + parameterized audit-redaction test over all 6 HandleKind shapes + new Windows-only integration test exercising end-to-end broker round-trip for all 5 new handle types via BrokerTargetProcess::current() (complete 2026-04-19, 4 commits 71270a1/611ea54/3750c1b/e29fd1c)
  - [x] 18-04-PLAN.md — Child-side SDK surface (CONTEXT.md D-08 + D-09): 5 cross-platform request_* methods (request_socket / request_pipe / request_job_object / request_event / request_mutex) on the existing Phase 11 SupervisorSocket transport — each method builds a CapabilityRequest with the appropriate HandleKind + HandleTarget from 18-01's protocol skeleton, posts over the existing capability pipe, demultiplexes SupervisorResponse::Decision into a typed Result. Windows arms return RawSocket / RawHandle (with WSAPROTOCOL_INFOW blob reconstruction via WSASocketW(FROM_PROTOCOL_INFO) for sockets); non-Windows arms return NonoError::UnsupportedPlatform with the exact D-09-locked message. Plans 18-01..18-03 byte-identical preservation (no file outside aipc_sdk.rs and the mod.rs registration block touched). 11 new tests (2 message-integrity + 4 Windows loopback + 5 Windows real-broker smoke). Complete 2026-04-19, 3 commits 4303c61/cfafdf3/53c5066.

- [ ] **Phase 18.1: Extended IPC Gap Closure (AIPC-01 follow-up)** — Close the 5 gaps surfaced during Phase 18 HUMAN-UAT re-run on 2026-04-20 after the supervisor-pipe DACL fix (`938887f`) unblocked the end-to-end `nono run --profile claude-code -- aipc-demo.exe` flow. G-01 was resolved by the debug-session fix chain (3c68377/938887f/e4c1bfa); G-02..G-06 require targeted code + test work. Scope: route CONIN$ prompts through `format_capability_prompt` per D-04 (G-02 — **closed 2026-04-21 by Plan 18.1-01**); audit the JobObject broker's CREATE-vs-OPEN posture + fix the semantic (G-03); fix broker handlers to flip `ApprovalDecision::Approved` → `Denied { reason }` when the internal kernel-object operation fails post-approval, and tighten the wire-protocol `Approved` shape so an empty `ResourceGrant` becomes a compile-time error (G-04); add scripted deny-stage integration tests verifying the WR-01 reject-BEFORE-prompt vs reject-AFTER-prompt invariant (G-05); thread `Profile` through the dispatcher initialization so `resolved_aipc_allowlist` is no longer seeded with `default()` and add an end-to-end profile-widening test (G-06). This is the remaining gate to the v2.1 milestone tag.

  **Requirements:** AIPC-01 (follow-through on acceptance criteria 1–3 for the 5 new handle types end-to-end). See `.planning/REQUIREMENTS.md`.

  **Depends on:** Phase 18 (18-01..18-04 all shipped), Phase 21 (Windows single-file grants — unblocked the live `claude-code` path that surfaced these gaps).

  **Source:** `.planning/phases/18-extended-ipc/18-HUMAN-UAT.md § Gaps` (G-02..G-06 with reproduction + candidate root causes) + `.planning/debug/resolved/supervisor-pipe-access-denied.md` (G-01 resolution context).

  **Plans:** 4 plans (planned 2026-04-21). Wave structure: Wave 1 (parallel-eligible: 18.1-01 disjoint file only), Waves 2–4 sequential on `supervisor.rs`.
  - [x] 18.1-01-PLAN.md — Wave 1 (disjoint — `terminal_approval.rs` only). G-02 route AIPC approval prompts through D-04 per-kind templates. Replaced the inline `Path:/Access:/Reason:` block in `TerminalApproval::request_capability` with a testable `build_prompt_text` helper that dispatches through `format_capability_prompt`; removed 5 `#[allow(dead_code)]` attributes on per-kind format helpers + `format_capability_prompt` itself; added 6 new tests (5 AIPC HandleKinds with full-string `assert_eq!` + 1 File-kind legacy `assert!.contains` preservation). 31/31 terminal_approval tests PASS; clippy/fmt/build clean; D-19 cross-phase byte-identical preservation verified (zero diff on any file outside `terminal_approval.rs`). TDD cycle: 2 commits `1960239` (RED) + `70984ac` (GREEN). Completed 2026-04-21.
  - [x] 18.1-02-PLAN.md — Wave 2 (supervisor.rs). G-03 `OpenJobObjectW` → `CreateJobObjectW(null, wide)` CREATE-if-not-exists parity with Event/Mutex/Pipe/Socket landed at `handle_job_object_request` line ~1675; containment-Job CompareObjectHandles D-06 guard preserved byte-identical. G-04 dispatcher flow-control rewrite: `let grant = if decision.is_granted()` → `let (decision, grant) = if decision.is_granted()` with `Err` arm flipping decision to `ApprovalDecision::Denied { reason: "broker failed: {e}" }` — `(Approved, grant=None)` now structurally unreachable via single-site tuple construction (audit_log push + send_response both see the same pair). 5 new `dispatcher_flips_approved_to_denied_on_*_broker_failure` tests (one per HandleKind: Event/Mutex/Pipe/Socket/JobObject) use deterministic input-validation injection (empty-name trips `validate_aipc_object_name`; empty-host trips `host.is_empty()`). TDD cycle: `e6ac4bb` (G-03 fix) + `9f81a39` (G-04 RED — 5 failing tests with "expected Denied, got Granted") + `3493dd8` (G-04 GREEN). 28/28 capability_handler_tests PASS (23 pre-existing + 5 new); 5/5 integration PASS; 60/60 nono lib supervisor PASS (D-19 byte-identical proof); 31/31 terminal_approval PASS (Wave 1 regression guard); clippy/fmt/build clean. D-19 verified — only supervisor.rs diffed across all 3 commits. D-21 preserved. G-04 D-09 (compile-time wire-protocol tightening `Approved(ResourceGrant)`) and D-11 (child SDK demultiplexer branch removal) deferred to v2.2 — would cascade into Plan 18-04 aipc_sdk.rs + 23 capability_handler_tests. Completed 2026-04-21.
  - [x] 18.1-03-PLAN.md — Wave 3 (supervisor.rs + mod.rs + supervised_runtime.rs + execution_runtime.rs + sandbox_prepare.rs + 3 cross-file plumbing: launch_runtime.rs + command_runtime.rs + main.rs). G-06 thread `Profile` through end-to-end: `SupervisorConfig.aipc_allowlist` field added; `WindowsSupervisorRuntime::initialize` reads `supervisor.aipc_allowlist.clone()` into `self.resolved_aipc_allowlist` (replaces Plan 18-03 `AipcResolvedAllowlist::default()` seed); `SupervisedRuntimeContext.loaded_profile: Option<&'a Profile>` plumbed end-to-end from `PreparedSandbox → LaunchPlan → execute_sandboxed → SupervisedRuntimeContext`; `execute_supervised_runtime` resolves `profile.resolve_aipc_allowlist()?` (or `default()`) and writes into the Windows `SupervisorConfig` literal. Plan 18-03 Deferred Issue #1 RESOLVED. 3 new end-to-end tests (`profile_widening_for_pipe_readwrite_reaches_backend`, `default_allowlist_rejects_pipe_readwrite_after_prompt`, `profile_widening_for_socket_bind_reaches_backend`) verify the live wiring exercises `Profile::resolve_aipc_allowlist` via `serde_json::from_str::<Profile>(...)` with minimal JSON `{"meta":{"name":"..."},"capabilities":{"aipc":{...}}}`. 31/31 capability_handler_tests PASS (28 pre-existing + 3 new); 5/5 aipc_handle_brokering_integration PASS; 4/4 profile AIPC tests PASS; 751/751 full nono-cli bin suite PASS; clippy/fmt/build clean. D-19 verified (nono lib + terminal_approval.rs + profile/ + data/ all 0-diff). D-21 preserved. Commits: `8170df2` (task 1 refactor) + `993cdcb` (task 2 feat) + `d88c20d` (task 3 test). Completed 2026-04-21.
  - [ ] 18.1-04-PLAN.md — Wave 4 (supervisor.rs test module). G-05 WR-01 reject-stage invariant verification: 5 new `wr01_*` tests empirically verify Event/Mutex/JobObject reject BEFORE prompt (backend.calls() == 0, pre-broker mask gate) and Pipe/Socket reject AFTER prompt (backend.calls() == 1, G-04-wrapped Denied). Socket privileged-port stage empirically verified. Module-level docstring records the verdict matrix per CONTEXT D-14; WR-01 fix itself deferred to v2.2 product decision.

- [x] **Phase 19: Cleanup (CLEAN)** — `cargo fmt --all` for drifted files from commit `6749494`; diagnose 5 pre-existing Windows test flakes; triage disk-resident WIP (10-*, 11-*, 12-*, quick tasks, INTEGRATION-REPORT); prune 1172 stale session files + document retention policy. (complete 2026-04-19; verifier passed 25/25 must-haves, commit `6597fbf`)

  **Depends on:** Nothing; can run in parallel with the feature phases. Recommended to run last so it catches any drift introduced by the feature phases too.

  **Plans:** 4 plans (all Wave 1, parallel — disjoint files_modified).
  - [x] 19-01-PLAN.md — CLEAN-01 fmt drift fix on 3 files from commit 6749494 (complete 2026-04-18, commit `c87b10b`)
  - [x] 19-02-PLAN.md — CLEAN-02 restore 5 pre-existing Windows test flakes — fixed as 4 distinct deterministic Windows platform bugs (JSON-escape, non-absolute Unix-shaped XDG env guard, UNC-prefix production bug in `query_path`, Unix-only path literal vs absolute-path debug_assert). Hypothesis D-07 (parallel env contamination) contradicted; deviation D-08 tripped and user-approved option C for the production fix (complete-with-deviation 2026-04-18, commits `400f8c9`, `8412fda`, `a449454`, `4db849d`)
  - [x] 19-03-PLAN.md — CLEAN-03 triage 10 disk-resident WIP items (per-file disposition) — 6 backfilled alive, 2 reverted to HEAD, 2 deleted, 2 new `.gitignore` patterns for WFP-service debug crumbs (complete 2026-04-18, commits `a208761`, `a4100aa`, `db4547b`, `0391e37`, `d49fda8`, `d6bf88f`)
  - [x] 19-04-PLAN.md — CLEAN-04 `is_prunable` retention predicate + `nono prune` CLI extensions (duration-form `--older-than`, `--all-exited`) + auto-sweep on `nono ps` (100-file threshold) + T-19-04-07 `NONO_CAP_FILE` structural no-op + one-shot cleanup on this host (1392 → 49, delta 1343) + `docs/session-retention.md` (complete 2026-04-18, commits `18e9768`, `a71b2bf`, `c3defb6`, `ddf408b`, `f626e24`)

- [x] **Phase 20: Upstream Parity Sync (UPST)** — Track and back-port functional changes from upstream `always-further/nono` since the fork branched. Quick-task research (commit `7180f23`, `.planning/quick/260419-cmp-upstream-036-windows-parity/COMPARISON.md`) established the fork is pinned at crate version `0.30.1` while upstream has shipped 0.31–0.37.1 — the fork is missing upstream work in keyring URIs, `--allow-gpu`, GitLab trust tokens, macOS Seatbelt refinements, Cargo version realignment, and (critically) the rustls-webpki RUSTSEC-2026-0098/0099 security upgrade landed upstream in 0.37. This phase re-establishes the Unix/macOS parity baseline without regressing the Windows-specific work in Phases 01–19. **Completed 2026-04-19** (verifier passed 38/38 must-haves).

  **Depends on:** Nothing structural; should land before v2.1 ships so the security upgrade is in the release.

  **Open questions** (for `/gsd-plan-phase 20` or `/gsd-discuss-phase 20`):
  - Rebase onto `upstream/v0.37.1` vs cherry-pick individual changes vs manual port? Rebase is cleanest but risks losing Windows-specific conflict resolution history.
  - Target tag — 0.36.0 (user's stated baseline) or 0.37.1 (latest, includes the security fix)? Recommend 0.37.1.
  - Scope boundary — do we also refresh crate versions in `Cargo.toml` to match upstream 0.37.1, or keep them on fork-internal versioning?

  **Plans:** TBD during `/gsd-plan-phase 20` — likely grouped by upstream version range (0.31–0.33, 0.34–0.35, 0.36–0.37) plus a dedicated plan for the rustls-webpki security upgrade.

- [x] **Phase 21: Windows Single-File Filesystem Grants** — Extended the Windows filesystem sandbox backend in `crates/nono/src/sandbox/windows.rs` to enforce single-file capability grants (read / write / read-write) via per-file Low-IL mandatory labels (`SetNamedSecurityInfoW` + `SYSTEM_MANDATORY_LABEL_ACE` with mode-derived mask per D-01). Closes `WindowsUnsupportedIssueKind::SingleFileGrant` + `WriteOnlyDirectoryGrant`. RAII `AppliedLabelsGuard` owns the apply→revert lifecycle wired into `prepare_live_windows_launch`. Silent-degradation regression test + per-mode mask integration tests + `git_config` motivator regression test land in Plan 21-05 (76 passing `sandbox::windows` tests). Inline Rule-1+Rule-3 fix `da25619` adds ownership pre-check to `try_set_mandatory_label` (skip when path not owned by current user — Low-IL is subtractive; system paths like `C:\Windows` are already readable to Low-IL subjects through OS ACLs). **Completed-with-issues 2026-04-20.**

  **Unblocks (at library level):** Phase 18 HUMAN-UAT Path B + Path C structural blocker resolved — `claude-code` profile's `git_config` 5 single-file grants now compile to rules + labels cleanly.

  **Carry-forward (not a Phase 21 blocker):** live-CONIN$ end-to-end verification deferred — a NEW supervisor control pipe `ERROR_ACCESS_DENIED` (HRESULT 0x5) regression surfaced on the first-ever end-to-end `claude-code → supervised → aipc-demo` run. Captured as **G-01 in `.planning/phases/18-extended-ipc/18-HUMAN-UAT.md § Gaps`** with reproduction command + 3 candidate hypotheses (AppliedLabelsGuard `.cache\claude` side-effect, Phase 11 `CAPABILITY_PIPE_SDDL` DACL gap for Low-IL subjects, silent supervisor startup failure). Routed to a dedicated `/gsd-debug` session.

  **Requirements:** WSFG-01 ✓, WSFG-02 ✓, WSFG-03 closed-with-deviation (frontmatter transition achieved; live-CONIN$ `pass` verdicts deferred). See `.planning/REQUIREMENTS.md`.

  **Depends on:** Nothing structural; isolated to the Windows filesystem backend.

  **Plans (5/5 complete):**
  - [x] 21-01 — WSFG requirements bookkeeping (`d8dbe2c` + `6df4f4b`)
  - [x] 21-02 — Mandatory-label enforcement primitive + `NonoError::LabelApplyFailed` (`1a545e1`/`d19aaaa`/`853683a`/`7637694`)
  - [x] 21-03 — `compile_filesystem_policy` rule emission + `apply()` label-apply loop (`2054903`/`a59e978`/`5526068`/`8c47a6b`)
  - [x] 21-04 — `AppliedLabelsGuard` RAII lifecycle wired into `prepare_live_windows_launch` (`3ad4f64`/`bedf679`)
  - [x] 21-05 — 5 new Windows-gated tests + Phase 18 HUMAN-UAT transition blocked → complete-with-issues + inline ownership-skip fix (`2e8dd82`/`da25619`/`a474594`)

## Progress Table

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Windows Control Foundation | v1.0 | 3/3 | Complete | 2026-04-04 |
| 2. Persistent Sessions | v1.0 | 4/4 | Complete | 2026-04-04 |
| 3. Network Sandboxing | v1.0 | 4/4 | Complete | 2026-04-04 |
| 4. State Integrity & Deployment | v1.0 | 3/3 | Complete | 2026-04-05 |
| 5. Windows Detach Readiness Fix | v2.0 | 1/1 | Complete | 2026-04-05 |
| 6. WFP Enforcement Activation | v2.0 | 2/2 | Complete | 2026-04-06 |
| 7. Quick Wins | v2.0 | 2/2 | Complete | 2026-04-08 |
| 8. ConPTY Shell | v2.0 | 1/1 | Complete | 2026-04-10 |
| 9. WFP Port-Level + Proxy Filtering | v2.0 | 4/4 | Complete | 2026-04-10 |
| 10. ETW-Based Learn Command | v2.0 | 3/3 | Complete | 2026-04-10 |
| 11. Runtime Capability Expansion | v2.0 | 2/2 | Complete | 2026-04-11 |
| 12. Milestone Bookkeeping Cleanup | v2.0 | 3/3 | Complete | 2026-04-11 |
| 13. Human Verification UAT | v2.0 | 1/1 | Resolved (2nd-pass 2026-04-18 — 3 pass, 7 waived incl. 4 v2.0-known-issue) | 2026-04-18 |
| 14. Fix Pass | v2.0 | 2/3 | Complete with carry-forward (14-02 done; 14-03 done; 14-01 escalated to Phase 15) | 2026-04-18 |
| 15. Detached Console + ConPTY Architecture Investigation | post-v2.0 closure | 3/3 | Complete (direction-b fix; 5-row smoke gate pass; 4 UAT items promoted; carry-forward closed) | 2026-04-18 |
| 16. Resource Limits (RESL-01..04) | v2.1 | 2/2 | Complete (RESL-01..04 shipped: CPU/memory/processes kernel-enforced, timeout via supervisor timer, `nono inspect` Limits block) | 2026-04-18 |
| 17. Attach-Streaming (ATCH-01) | v2.1 | 2/2 | Complete (anonymous-pipe stdio + supervisor pipe-source/sink bridges + ERROR_PIPE_BUSY friendly translation + 3 latent session-id mismatch fixes via debug `7db6595`; 4 deferred-by-design HUMAN-UAT items per pragmatic-PASS verdict) | 2026-04-19 |
| 18. Extended IPC (AIPC-01) | v2.1 | 4/4 | Complete (18-01 wire-protocol skeleton + Event/Mutex brokers 6 commits `51b1c12`..`3ea2017`; 18-02 Pipe + Socket brokers + Win32_Networking_WinSock feature + privileged-port deny 3 commits `39c5a82`..`0a1f2ee`; 18-03 Job Object broker + containment-Job CompareObjectHandles runtime guard + capabilities.aipc profile schema + AipcResolvedAllowlist + parameterized audit-redaction + Windows-only integration suite 4 commits `71270a1`..`e29fd1c`; 18-04 child-side SDK 5 cross-platform `request_*` methods + send_capability_request helper + reconstruct_socket_from_blob + message-integrity snapshot + 5 Windows real-broker smoke tests 3 commits `4303c61`/`cfafdf3`/`53c5066`. 108/108 plan-targeted tests pass; Plans 18-01..18-03 byte-identical preservation verified; AIPC-01 feature-complete end-to-end. **HUMAN-UAT Path B/C blocked on Phase 21** — 4 live-CONIN$ tests cannot run while `claude-code` profile's `git_config` group trips `WindowsUnsupportedIssueKind::SingleFileGrant`; Path A tests remain unaffected. Retry after Phase 21 ships.) | 2026-04-19 |
| 18.1. Extended IPC Gap Closure | v2.1 | 3/4 | In progress (18.1-01 G-02 CONIN$ prompt routing COMPLETE 2026-04-21 — commits `1960239` RED + `70984ac` GREEN; new `build_prompt_text` dispatcher in `terminal_approval.rs` routes AIPC kinds through `format_capability_prompt` D-04 per-kind templates + preserves Phase 11 File legacy block byte-identical. 18.1-02 G-03 JobObject CreateJobObjectW + G-04 broker-failure flip COMPLETE 2026-04-21 on windows-squash — 3 commits `e6ac4bb` G-03 fix + `9f81a39` G-04 RED + `3493dd8` G-04 GREEN; `handle_job_object_request` now calls `CreateJobObjectW(null_mut, wide)` (create-or-open) restoring parity with Event/Mutex/Pipe/Socket; dispatcher flow-control rewrite makes `(ApprovalDecision::Approved, grant=None)` structurally unreachable via single-site `(decision, grant)` tuple construction + Err-arm flip to `Denied { reason: "broker failed: {e}" }`; 5 new `dispatcher_flips_approved_to_denied_on_*_broker_failure` tests (one per HandleKind). 18.1-03 G-06 profile widening end-to-end wiring COMPLETE 2026-04-21 on windows-squash — 3 commits `8170df2` task 1 refactor + `993cdcb` task 2 feat + `d88c20d` task 3 test; `SupervisorConfig.aipc_allowlist` field added; `WindowsSupervisorRuntime::initialize` reads `supervisor.aipc_allowlist.clone()` (replaces Plan 18-03 `AipcResolvedAllowlist::default()` seed — Plan 18-03 Deferred Issue #1 RESOLVED); `SupervisedRuntimeContext.loaded_profile` plumbed end-to-end from `PreparedSandbox → LaunchPlan → execute_sandboxed → SupervisedRuntimeContext`; `execute_supervised_runtime` resolves `profile.resolve_aipc_allowlist()?` and writes into Windows SupervisorConfig literal; 3 new end-to-end tests (`profile_widening_for_pipe_readwrite_reaches_backend`, `default_allowlist_rejects_pipe_readwrite_after_prompt`, `profile_widening_for_socket_bind_reaches_backend`). 31/31 capability_handler_tests + 5/5 aipc_handle_brokering_integration + 4/4 profile AIPC tests + 751/751 full nono-cli bin suite PASS; clippy/fmt/build clean; D-19 verified — nono lib + terminal_approval.rs + profile/ + data/ all 0-diff. Remaining: 18.1-04 G-05 WR-01 reject-stage verification. 18.1-04 sequential on `supervisor.rs`. Remaining gate to v2.1 milestone tag.) | — |
| 19. Cleanup (CLEAN-01..04) | v2.1 | 4/4 | Complete (19-01 CLEAN-01 fmt drift; 19-02 CLEAN-02 5 test flakes + query_path UNC prod fix complete-with-deviation; 19-03 CLEAN-03 10-item WIP triage; 19-04 CLEAN-04 retention + prune + auto-sweep + T-19-04-07 mitigation + 1343-file one-shot cleanup + docs; verifier passed 25/25 must-haves, commit `6597fbf`) | 2026-04-19 |
| 20. Upstream Parity Sync (UPST) | v2.1 | 4/4 | Complete (Wave 0+1+2 2026-04-19: 20-01 rustls-webpki 0.103.12 + workspace 0.37.1; 20-02 profile extends cycle guard + claude.json symlink; 20-03 keyring:// URI + env-var filter flags + command_blocking_deprecation; 20-04 --allow-gpu + NVIDIA Linux allowlist + GitLab ID tokens. Clippy follow-up `4f08f3f` fixes 2 pre-existing 20-03 unwrap_used violations. Verifier passed 38/38 must-haves, UPST-01..04 all traced; D-21 Windows-invariance held across 11 feat/fix commits — zero `*_windows.rs` touched) | 2026-04-19 |
| 21. Windows Single-File Filesystem Grants | v2.1 | 5/5 | Complete-with-issues (all 5 plans shipped 2026-04-20 on `windows-squash`; WSFG-01 + WSFG-02 fully closed, WSFG-03 closed-with-deviation. 21-01 bookkeeping; 21-02 enforcement primitive + NonoError::LabelApplyFailed; 21-03 compile_filesystem_policy rule emission + apply() label-apply loop; 21-04 AppliedLabelsGuard RAII lifecycle in prepare_live_windows_launch; 21-05 5 new Windows-gated tests (silent-degradation regression + per-mode Write/ReadWrite mask + git_config motivator + end-to-end 5-file apply; sandbox::windows 71 → 76 tests) commits `2e8dd82`+`da25619`+`a474594`. Inline Rule-1+Rule-3 fix `da25619` added ownership pre-check to try_set_mandatory_label closing the C:\Windows ERROR_ACCESS_DENIED that tore down claude-code sandbox bring-up. Task 2 HUMAN-UAT re-run surfaced a NEW supervisor control pipe ERROR_ACCESS_DENIED (HRESULT 0x5) regression on live `nono run --profile claude-code -- aipc-demo.exe` — first observable now that the full claude-code → supervised → aipc-demo flow runs end-to-end post-Phase-21. Captured as G-01 in 18-HUMAN-UAT.md with reproduction + 3 candidate hypotheses (AppliedLabelsGuard .cache/claude side-effect, Phase 11 CAPABILITY_PIPE_SDDL DACL gap for Low-IL, silent supervisor startup failure) for dedicated /gsd-debug session. 18-HUMAN-UAT.md frontmatter transitioned blocked → complete-with-issues; 4 items [blocked] → [issue: supervisor control pipe access denied — pending /gsd-debug investigation]; Summary block blocked 4 → 0, issues 0 → 4. Library goal shipped clean — regression blocks only live-CONIN$ UAT, not the primitive itself. D-21 Windows-invariance preserved) | 2026-04-20 |
