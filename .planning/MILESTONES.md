# Milestones

## v2.1 — Resource Limits, Extended IPC, Attach-Streaming & Cleanup

**Status:** ✅ SHIPPED 2026-04-21
**Started:** 2026-04-18
**Shipped:** 2026-04-21
**Branch:** `windows-squash` (continuing from v2.0 + Phase 15)

**Goal:** Deliver Job Object resource limits (CPU / memory / timeout / process-count), extend the Phase 11 capability pipe to broker additional handle types end-to-end, land attach-streaming on detached Windows sessions, sync to upstream v0.37.1 (including the rustls-webpki security upgrade), enable single-file filesystem grants on Windows so the `claude-code` profile runs cleanly, and clean up v2.0 WIP.

**Phases:** 7 phases (Phases 16–21 plus decimal Phase 18.1).
**Plans shipped:** 25 plans.
**Requirements:** 13 — RESL-01..04, AIPC-01, ATCH-01, CLEAN-01..04, UPST-01..04, WSFG-01..03.

**Key accomplishments:**
- Job Object resource limits — CPU/memory/timeout/process-count caps with kernel enforcement (Phase 16).
- `nono attach` on detached Windows sessions now streams child stdout live via anonymous-pipe stdio; friendly multi-attach error (Phase 17).
- AIPC handle brokering for Socket / Pipe / Job Object / Event / Mutex over the Phase 11 capability pipe + `capabilities.aipc` profile-widening schema + containment-Job runtime guard (Phases 18 + 18.1).
- 5 HUMAN-UAT gaps (G-02..G-06) closed in Phase 18.1 with live dual-run widening proof on rebuilt binary.
- Cleanup workstream — fmt drift, 4 Windows test flakes (incl. UNC-prefix `query_path` production bug), 10 WIP items triaged, `is_prunable` + `nono prune --older-than`/`--all-exited` + auto-sweep on `nono ps`, 1343-file one-shot prune on dev host (Phase 19).
- Upstream parity sync to v0.37.1 — rustls-webpki 0.103.12 security upgrade (RUSTSEC-2026-0098/0099), `keyring://` URIs, env-var filtering, `--allow-gpu` with NVIDIA Linux allowlist, GitLab ID tokens for trust signing (Phase 20).
- Windows single-file filesystem grants via per-file Low-IL mandatory-label ACEs + `AppliedLabelsGuard` RAII lifecycle + ownership-skip pre-check; unblocks `claude-code` profile's `git_config` group on Windows (Phase 21).

**Notable in-flight finding:** Windows 11 26200's `WRITE_RESTRICTED` tokens require BOTH a restricting-SID ACE AND a logon-SID ACE in the pipe DACL for the second-pass access check to pass — MSDN-undocumented; discovered via 13-variant systematic SDDL iteration in `crates/nono-cli/examples/pipe-repro.rs`. Fix in commit `938887f`.

**Known deferred items at close:** 17 (5 UAT bookkeeping gaps, 3 verification human_needed flags, 9 stale quick-task index pointers to already-removed directories). See STATE.md `## Deferred Items` for the full table. None block release.

**Archive files:**
- `.planning/milestones/v2.1-ROADMAP.md`
- `.planning/milestones/v2.1-REQUIREMENTS.md`

Git tag: `v2.1`.

---

## v2.0 — Windows Gap Closure (a.k.a. "Windows Parity")

**Status:** ✅ SHIPPED 2026-04-18 (with v2.0-known-issue carry-forward to Phase 15)
**Started:** 2026-04-06
**Shipped:** 2026-04-18
**Branch:** `windows-squash` (committed; push/merge-to-main pending per user)

**Goal:** Close the 7 remaining feature gaps between Windows and Unix platforms — `nono wrap`, session log commands, interactive ConPTY shell, port-granular WFP policy, proxy credential injection, ETW-based learn, and runtime capability expansion (stretch) — so everyday CLI usage reaches cross-platform parity.

**Phases:** 10 phases (Phases 5–14; Phase 15 created as v2.1 follow-up for the carry-forward).
**Plans shipped:** 28 firm plans. Plan 14-01 escalated to Phase 15.

**Key accomplishments:**
- WFP promoted to primary enforced network backend with SID-based filtering (Phase 06).
- `nono wrap` on Windows with Direct strategy + help-text correction (Phases 07, 14-02).
- Interactive `nono shell` via ConPTY on Windows 10 build 17763+ (Phase 08).
- Port-granular WFP policy + proxy credential injection (Phase 09).
- `nono learn` on Windows via ETW with Win32-format paths (Phase 10).
- Runtime capability expansion over named pipe with constant-time token auth (Phase 11).
- Human Verification UAT resolved with terminal verdicts on all 10 items (Phase 13).

**Known deferred items at close:**
- Detached-supervisor + ConPTY + restricted-token `0xC0000142 STATUS_DLL_INIT_FAILED` on sandboxed console grandchildren. Carried forward to Phase 15 per explicit user shipping decision.
- Affected UAT legs waived as `v2.0-known-issue`: P05-HV-1, P07-HV-3, P11-HV-1, P11-HV-3.
- P09-HV-1 live end-to-end waived as `no-test-fixture` (no built-in network-profile-with-credentials ships out of the box).

**Archive files:**
- `.planning/milestones/v2.0-ROADMAP.md`
- `.planning/milestones/v2.0-REQUIREMENTS.md`
- `.planning/milestones/v2.0-MILESTONE-AUDIT.md`

Git tag: `v2.0` (see `git show v2.0` for tagger signature).

---

## v1.0 — Windows Alpha (shipped 2026-03-31)

**Status:** ✅ SHIPPED 2026-03-31
**Git tag:** `v1.0`

**Delivered:** Windows is a first-class nono release target with signed artifacts, WFP service packaging, and no preview language anywhere.

**Key accomplishments:**
- Authenticode signing pipeline (sign-windows-artifacts.ps1 + release.yml gate).
- WFP service packaging via WiX 4 ServiceInstall/ServiceControl in machine MSI.
- All preview language removed from runtime, docs, CI, and README.
- Formal Windows promotion criteria (21 gates, all checked).
- Supervisor parity (attach, detach, ps, stop) — Phases 1–2.
- Snapshot/rollback for Windows filesystems — Phase 4.
- MSI packaging and code signing automation — Phase 4.

**Phases:** 4 (Phases 1–4). Requirements: SUPV-01..05, NETW-01..03, STAT-01..02, DEPL-01..02 (12 total).

(An earlier draft of this entry referred to this milestone as "v1.0 — WIN-1706 Option 1: Windows Library/Runtime Alignment" and was never properly closed; the real shipped content is what the `v1.0` git tag points at from 2026-03-31. That earlier draft is superseded by this entry.)
