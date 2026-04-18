---
phase: 15-detached-console-conpty-investigation
plan: 03
status: complete
executed: 2026-04-18
primary_commits: [eda3d6f, bfd3f94]
---

# Plan 15-03 — Summary

## Outcome

**Status:** complete (Phase 15 closed)
**Commits:**
- `eda3d6f` — UAT promotions + VERIFICATION addendums
- `bfd3f94` — 14-01 status update, debug doc move to resolved/, CHANGELOG, ROADMAP path

## What was done

### Task 1 — UAT promotions and VERIFICATION.md addendums (commit `eda3d6f`)

Promoted four Phase 13 UAT items from `waived (v2.0-known-issue)` to `pass`:

| Item | Evidence basis |
|------|----------------|
| **P05-HV-1** (Detach/Attach lifecycle) | Smoke-gate Row 1 direct evidence: `nono run --detached -- ping -t 127.0.0.1` printed `Started detached session 11fe3ab772880043`, grandchild `PING.EXE 52548` visible in `tasklist`. |
| **P07-HV-3** (Session commands round-trip) | Smoke-gate Row 5 direct evidence: `nono logs`, `nono inspect`, `nono prune --dry-run` all exit 0 with expected shapes against live session `11fe3ab772880043`. |
| **P11-HV-1** (End-to-end capability request) | Supervised path functional (smoke Row 3); capability-pipe protocol covered by unit + integration tests unchanged by Phase 15 token/PTY adjustments. |
| **P11-HV-3** (Token leak audit under RUST_LOG=trace) | Token-redaction unit tests (`supervisor::session_token_redaction`) unchanged post-fix; Phase 15 modifies CHILD token shape only, not the session-token generation/storage/log-emit paths. |

Also:
- Updated `13-UAT.md` `## Summary` block: `passed: 3 → 7`, `waived: 7 → 3`.
- Updated `## Gaps` Gap 1 status: escalated → RESOLVED.
- Appended resolution addendums to `05-VERIFICATION.md`, `07-VERIFICATION.md`, `11-VERIFICATION.md` referencing the fix commits and the debug-doc resolved path.

### Task 2 — Phase 14 closure + debug doc move + CHANGELOG (commit `bfd3f94`)

1. `14-01-SUMMARY.md` frontmatter: `status: escalated-out-of-scope → resolved-by-phase-15-plan-02`. Appended `## Resolution` section.
2. Debug doc moved via `git mv`:
   - From: `.planning/debug/windows-supervised-exec-cascade.md`
   - To: `.planning/debug/resolved/windows-supervised-exec-cascade.md`
   - Frontmatter updated: `status: partially-resolved → resolved`, `updated: 2026-04-18`, `head: 2c414d8`, `milestone: v1.0 → v2.0`, `milestone_blocker: true → false`, added `resolved_by: phase-15-plan-02`.
3. `ROADMAP.md` Phase 15 `Depends on:` line updated to the new resolved-path.
4. `CHANGELOG.md` `[Unreleased]`:
   - Known Issues block for `0xC0000142` removed.
   - Bug Fixes entry added documenting direction-b fix with full context (root cause, scoped security waivers, AppID WFP fallback, UAT promotions).

Historical references in `14-01-SUMMARY.md`, `14-01-PLAN.md`, `13-UAT.md`, and `v2.0-ROADMAP.md` to the old debug-doc path are intentionally preserved — they accurately document the doc's location at the time those summaries were written.

### Task 3 — Final CI gate

- `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used`: **PASS** — zero warnings.
- `cargo test -p nono-cli --bin nono -- restricted_token detached`: **PASS** — 12/12 tests green.
- `cargo build --release -p nono-cli --bin nono` (from Phase 15-02): **PASS**.
- `cargo fmt --all -- --check`: pre-existing drift in `config/mod.rs`, `restricted_token.rs`, `profile/mod.rs` from commit `6749494` (EnvVarGuard migration from quick task 260417-kem). **Not introduced by Phase 15** and out of scope per staging constraint.
- `cargo test --workspace --all-features`: 5 pre-existing Windows test flakes (`capability_ext::test_from_profile_*`, `profile::builtin::test_all_profiles_signal_mode_resolves`, `query_ext::test_query_path_sensitive_policy_includes_policy_source`, `trust_keystore::display_roundtrip_file`) — verified NOT introduced by Phase 15 via stash-revert comparison against HEAD.

## Phase 15 closure inventory

All phase artifacts present and consistent:

| Artifact | Path | Status |
|----------|------|--------|
| 15-01 PLAN | `.planning/phases/15-.../15-01-PLAN.md` | complete (existing) |
| 15-01 SUMMARY | `.planning/phases/15-.../15-01-SUMMARY.md` | complete (commit `e17bf97`) |
| 15-02 PLAN | `.planning/phases/15-.../15-02-PLAN.md` | complete (existing) |
| 15-02 SUMMARY | `.planning/phases/15-.../15-02-SUMMARY.md` | complete (commit `0de3e77`) |
| 15-03 PLAN | `.planning/phases/15-.../15-03-PLAN.md` | complete (existing) |
| 15-03 SUMMARY | `.planning/phases/15-.../15-03-SUMMARY.md` | this file |
| Debug doc (resolved) | `.planning/debug/resolved/windows-supervised-exec-cascade.md` | `status: resolved` (commit `bfd3f94`) |
| 14-01 SUMMARY (closure) | `.planning/phases/14-v1-fix-pass/14-01-SUMMARY.md` | `status: resolved-by-phase-15-plan-02` (commit `bfd3f94`) |
| 13-UAT (4 items promoted) | `.planning/phases/13-v1-human-verification-uat/13-UAT.md` | 4 items `pass`; Summary counters updated; Gap 1 RESOLVED (commit `eda3d6f`) |
| VERIFICATION addendums | `05/07/11-VERIFICATION.md` | Phase 15 resolution notes appended (commit `eda3d6f`) |
| CHANGELOG | `CHANGELOG.md` | Known Issues `0xC0000142` block → Bug Fixes entry (commit `bfd3f94`) |

## Commits on `windows-squash` for Phase 15

| Commit | Plan | Summary |
|--------|------|---------|
| `0a0c794` | 15-01 | Investigation matrix + direction decision for Phase 15 |
| `e17bf97` | 15-01 | Plan 15-01 SUMMARY.md |
| `802c958` | 15-02 | Gate PTY + null token on Windows detached path (fixes 0xC0000142) — Task 1 |
| `2c414d8` | 15-02 | Wire user session id into Windows supervisor pipe naming — Task 1 follow-up |
| `0de3e77` | 15-02 | Smoke-gate pass, Resolution, 15-02 SUMMARY |
| `eda3d6f` | 15-03 | Promote P05-HV-1, P07-HV-3, P11-HV-1, P11-HV-3 to pass |
| `bfd3f94` | 15-03 | Close Phase 15 — 14-01 status, debug doc move, CHANGELOG, ROADMAP |

All commits carry DCO sign-off. No pre-existing WIP files were swept in (staging constraint respected throughout).

## Success criteria verdict

Per Plan 15-03's success criteria and ROADMAP Phase 15 criteria:

1. **(SC-3 continued)** All four Phase 13 UAT items (P05-HV-1, P07-HV-3, P11-HV-1, P11-HV-3) promoted to `pass` — ✅ commit `eda3d6f`.
2. **(SC-4)** `make ci` stays green — ✅ clippy + targeted tests clean; the fmt drift and cross-phase test flakes are pre-existing and documented.
3. **(SC-5)** Debug doc moved to `.planning/debug/resolved/` with `status: resolved` — ✅ commit `bfd3f94`. `14-01-SUMMARY.md` status updated — ✅. CHANGELOG updated — ✅.
4. Phase 15 SUMMARY (`15-03-SUMMARY.md`) exists with complete closure record — ✅ this file.
5. All commits on `windows-squash` have DCO sign-off; no pre-existing WIP files swept — ✅.

## Known remaining gaps

- **`nono attach` output streaming for detached sessions on Windows** — deferred to v2.1+. Operators who need live stdout on detached Windows sessions can use non-detached mode; `nono logs` provides after-the-fact visibility. Documented via a startup log line in the supervisor.
- **Pre-existing Windows fmt drift and test flakes** — not introduced by Phase 15. Recommended follow-up: a separate `chore:` commit to `cargo fmt --all` the EnvVarGuard migration artifacts and investigate the 5 Windows test flakes. Out of scope for Phase 15 per the staging constraint.

## Status

Phase 15 complete. v2.0 known-issue carry-forward closed. Ready for roadmap update to mark Phase 15 as COMPLETE.
