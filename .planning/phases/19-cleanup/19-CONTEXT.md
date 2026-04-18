# Phase 19: Cleanup (CLEAN) - Context

**Gathered:** 2026-04-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Pay down v2.0/v2.1 milestone debt that accumulated during feature work — formatting drift, pre-existing test failures, on-disk WIP files, and stale session records. **No new product capabilities.** Phase 19 is the receiver for the deferrals explicitly logged by Plans 16-01 SUMMARY and 16-02 SUMMARY (CLEAN-01 fmt drift, CLEAN-02 5 test flakes, CLEAN-03 disk-resident WIP, CLEAN-04 1172 stale session files).

Out of scope (route to other phases):
- Any new `nono` subcommand other than `prune` (already in scope per CLEAN-04)
- Refactors of the modules touched by failing tests (capability_ext, profile, query_ext, trust_keystore) beyond what's needed to make the tests pass
- Migration of session-file format

</domain>

<decisions>
## Implementation Decisions

### Scope of Phase 19

- **D-01:** Phase 19 keeps all 4 CLEAN items as separate plans (CLEAN-01, CLEAN-02, CLEAN-03, CLEAN-04). Single phase, 4 plans.
- **D-02:** Plans are parallelizable — they touch disjoint files (fmt drift on 3 specific files, test fixes in nono-cli unit tests, planning-dir WIP, session file housekeeping). Planner should mark them as Wave 1 (all parallel) unless a dependency surfaces during planning.

### CLEAN-01 — fmt drift

- **D-03:** Run `cargo fmt --all` on the 3 drifted files identified in Plan 16-02 SUMMARY § Deferred for follow-up: `crates/nono-cli/src/config/mod.rs`, `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs`, `crates/nono-cli/src/profile/mod.rs`. These pre-date Phase 16 (commit `6749494` aftermath).
- **D-04:** Verification: `cargo fmt --all -- --check` passes clean on the entire workspace at end of plan.

### CLEAN-02 — 5 pre-existing Windows test flakes

- **D-05:** **Diagnose AND fix all 5.** Restore `cargo test --workspace --all-features` to 0 failures on Windows. Failing tests are a noise floor that masks real regressions; this debt must be paid in full, not deferred.
- **D-06:** The 5 tests (per Plan 16-02 SUMMARY):
  - `capability_ext::test_from_profile_allow_file_rejects_directory_when_exact_dir_unsupported`
  - `capability_ext::test_from_profile_filesystem_read_accepts_file_paths`
  - `profile::builtin::test_all_profiles_signal_mode_resolves`
  - `query_ext::test_query_path_sensitive_policy_includes_policy_source`
  - `trust_keystore::display_roundtrip_file`
- **D-07:** REQUIREMENTS.md hint: "Likely env-var isolation bugs." First investigation step: re-read CLAUDE.md § "Environment variables in tests" rule (save/restore HOME / TMPDIR / XDG_CONFIG_HOME). Most likely root cause is parallel-test env-var contamination matching the existing pattern Phase 11 narrowly avoided.
- **D-08:** If a test failure proves to be a genuine bug requiring an API change (not a test-isolation fix), STOP and surface it for re-discussion before proceeding — don't silently expand scope.
- **D-09:** Each fix lands as `fix(19-CLEAN-02): restore <test_name>` with a one-line root-cause statement in the commit body. Atomic per test or per shared-fix-cluster (e.g., if all 5 share one env-var bug, one commit is fine).

### CLEAN-03 — disk-resident WIP triage (10 items)

- **D-10:** **Per-file judgment policy** — inspect each WIP item, classify into one of three actions, document the decision in 19-03-SUMMARY.md as a per-file table. Three actions:
  - **Commit alive** — file represents real work-in-progress with a clear destination (e.g., 11-PLAN.md modifications already in-flight, 10-UAT.md if it's a real test artifact)
  - **Delete dead** — file is a debug crumb or one-shot artifact (`host.nono_binary.commit`, `query`)
  - **Archive uncertain** — file has documentation value but no active home; move to `.planning/archive/` or equivalent
- **D-11:** **Stray root-file default:** `host.nono_binary.commit` and `query` → delete now AND add their patterns (or specific names if no clean pattern) to `.gitignore` to prevent recurrence. These are clearly debug crumbs; no inspection needed.
- **D-12:** WIP inventory to triage (from `git status` at session start):
  - `M  .planning/phases/11-runtime-capability-expansion/11-01-PLAN.md`
  - `M  .planning/phases/11-runtime-capability-expansion/11-02-PLAN.md`
  - `?? .planning/phases/10-etw-based-learn-command/10-RESEARCH.md`
  - `?? .planning/phases/10-etw-based-learn-command/10-UAT.md`
  - `?? .planning/phases/12-milestone-bookkeeping-cleanup/12-02-PLAN.md`
  - `?? .planning/quick/260410-nlt-fix-three-uat-gaps-in-phase-10-etw-learn/260410-nlt-PLAN.md`
  - `?? .planning/quick/260412-ajy-safe-layer-roadmap-input/`
  - `?? .planning/v1.0-INTEGRATION-REPORT.md`
  - `?? host.nono_binary.commit`
  - `?? query`
- **D-13:** Each "commit alive" is its own atomic commit (`docs(19-CLEAN-03): commit <file> — <one-line reason it's alive>`). Each "delete dead" / "archive uncertain" gets a single grouped commit per category.

### CLEAN-04 — session-file retention

- **D-14:** **Retention rule:** age-based, 30 days, applied to sessions with `Status: Exited` only. Active (non-Exited) sessions are NEVER pruned regardless of age.
- **D-15:** **Trigger:** lightweight check at the start of `nono ps`. If >100 stale sessions detected, prune in background and log the count to stderr (`info: pruned 1027 stale session files (>30 days, exited)`). Operators see it in their normal workflow without it being intrusive.
- **D-16:** Add explicit `nono prune` CLI command for operators who want manual control. Flags:
  - `--older-than <duration>` (default 30d, accepts the same duration parser as `--timeout` from Plan 16-01)
  - `--dry-run` (list what would be pruned, take no action)
  - `--all-exited` (ignore age, prune every Exited session — escape hatch)
- **D-17:** **Initial cleanup:** the existing 1172 stale session files MUST be pruned by this plan as a one-shot action. Don't rely on the next `nono ps` invocation to clean up the backlog — make it a deterministic step in the plan with a smoke that asserts the directory shrinks.
- **D-18:** Document the policy in `docs/session-retention.md` (or in the existing CLI reference doc — planner picks the right home). Include: rule, trigger, configuration knobs, and the one-time cleanup that ran during 19-CLEAN-04.

### Claude's Discretion

- **Branch / PR strategy** — single PR for all 4 plans vs per-plan PR. Planner picks based on git log size and merge-train norms in the project. Default assumption: stay on `windows-squash` (already the active milestone branch), one PR at the end.
- **CLEAN-02 test-fix grouping** — planner decides whether to commit per-test or per-root-cause-cluster. If all 5 share one env-var fix, one atomic commit is acceptable per D-09.
- **CLEAN-04 prune trigger UX** — exact wording of the `nono ps` log line, exact threshold (100 was suggested but planner can tune to 50–500 based on what feels right). Backgrounding mechanism (separate thread vs deferred join) is an implementation choice.
- **CLEAN-04 docs location** — `docs/session-retention.md` vs inline in an existing doc; pick whichever is closer to the rest of session-related docs.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 19 scope sources
- `.planning/REQUIREMENTS.md` § Active (CLEAN-01..04) — the four cleanup requirements with acceptance hints
- `.planning/ROADMAP.md` § Phase 19 — phase goal, deliverables, "likely 4 plans, can parallelize" guidance

### Receiver dependencies (deferrals from Phase 16)
- `.planning/phases/16-resource-limits/16-02-SUMMARY.md` § Deferred for follow-up — explicit list of CLEAN-01 (3 files), CLEAN-02 (5 tests with names), CLEAN-03 (10 WIP items)
- `.planning/phases/16-resource-limits/16-01-SUMMARY.md` — context on what Plan 16-01 left clean vs deferred

### Coding standards (apply to CLEAN-02 fixes)
- `CLAUDE.md` § Coding Standards — no `.unwrap()`, no broad `#[allow(dead_code)]`, DCO sign-off, env-var save/restore in tests (the most likely root cause of the 5 flakes)
- `CLAUDE.md` § Security Considerations — path handling component comparison, fail-secure on config load

### Project-level context
- `.planning/PROJECT.md` § Active (v2.1) — confirms CLEAN-01..04 are scheduled here, not deferred to a later milestone
- `.planning/PROJECT.md` § Core Value — "Windows security must be as structurally impossible and feature-complete as Unix platforms" — restoring `cargo test --workspace` to green directly serves this on the Windows test surface

### Existing artifacts CLEAN-03 will inspect (do NOT delete without judgment)
- `.planning/phases/10-etw-based-learn-command/10-RESEARCH.md` — appears to be unfinished research from the v2.0 LEARN-01 implementation
- `.planning/phases/10-etw-based-learn-command/10-UAT.md` — appears to be a UAT artifact; may belong with the resolved 10 phase
- `.planning/phases/11-runtime-capability-expansion/11-01-PLAN.md` (M) and `11-02-PLAN.md` (M) — modified plan files; modifications may represent real corrections worth preserving
- `.planning/phases/12-milestone-bookkeeping-cleanup/12-02-PLAN.md` — looks like a draft of an unmerged plan
- `.planning/quick/260410-nlt-*/` — quick task from 2026-04-10, never completed (no SUMMARY.md)
- `.planning/quick/260412-ajy-*/` — quick task from 2026-04-12, never completed
- `.planning/v1.0-INTEGRATION-REPORT.md` — historical artifact from v1.0 milestone close

### Existing CLI surface that CLEAN-04 extends
- `crates/nono-cli/src/session_commands.rs` and `session_commands_windows.rs` — `nono ps`, `nono inspect`, `nono prune`, `nono logs` live here. CLEAN-04 adds `nono prune` flags and the auto-prune trigger inside `run_ps`.
- `crates/nono-cli/src/session.rs` — `SessionRecord` schema, `load_session` / list functions; the prune logic reads `started_epoch` + `status` here.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`SessionRecord` (`crates/nono-cli/src/session.rs`)** — already has `started_epoch: u64` and `status: SessionStatus`. CLEAN-04 prune predicate reads these directly: `record.status == SessionStatus::Exited && (now_epoch - record.started_epoch) > retention_secs`.
- **`parse_duration` helper from Plan 16-01** — accepts `30d`, `5m`, `1h` etc. Reuse for `nono prune --older-than 30d`. Already covers the corner cases (overflow, zero-rejection).
- **Existing `nono prune` command (CLEAN-04 extends, not creates)** — `session_commands.rs` already has a `run_prune` function based on the SESS-03 work in v2.0 Phase 07. CLEAN-04 adds flags and the auto-trigger; the load/delete primitives exist.
- **Test pattern for env-var isolation** — `crates/nono/src/sandbox/macos.rs` and `crates/nono-cli/src/profile/mod.rs` already have `EnvVarGuard` patterns from the commit `6749494` migration. The 5 failing tests (CLEAN-02) almost certainly need to adopt this same guard.

### Established Patterns

- **DCO sign-off on every commit** — `Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>`. Enforced by hook on `windows-squash`. CLEAN plans must honor this.
- **Atomic commits per task** — established by Phase 16. Apply same discipline: one logical change per commit, files staged explicitly (no `git add -A`).
- **`#[cfg(target_os = "windows")]` test gating** — when fixing CLEAN-02 tests, check whether the test runs cross-platform or only on Windows. Some fixes may need `#[cfg(unix)]` vs `#[cfg(windows)]` divergence.

### Integration Points

- **`run_ps` in `session_commands.rs`** — CLEAN-04's auto-prune trigger goes at the top of this function (count stale sessions, conditionally invoke prune). Background thread or deferred join is an implementation choice.
- **`.gitignore` at repo root** — CLEAN-03's stray-file ignore patterns land here.
- **`docs/`** — CLEAN-04's retention policy documentation goes here (verify this directory exists and has analogous session docs before placing).

</code_context>

<specifics>
## Specific Ideas

- **CLEAN-02 first investigation move:** re-read CLAUDE.md § "Environment variables in tests" and grep the 5 failing test bodies for `env::set_var`, `HOME`, `TMPDIR`, `XDG_CONFIG_HOME`. If any of them touch env vars without a guard, that's almost certainly the root cause. This is the cheapest first hypothesis.
- **CLEAN-03 stray-file delete pattern:** `host.nono_binary.commit` looks like a debug log of a binary's commit hash; `query` looks like accidentally captured stdout. Both are clearly transient. Don't dignify them with inspection — delete + ignore.
- **CLEAN-04 specific sub-30d examples:** an exited session from 4 days ago STAYS (within 30d window). An exited session from 45 days ago PRUNES. An active session from 90 days ago STAYS (status guard wins).
- **CLEAN-04 prune log format:** `info: pruned 1027 stale session files (>30 days, exited)` printed to stderr at start of `nono ps` if count > 100. Don't surface in normal `ps` table output.

</specifics>

<deferred>
## Deferred Ideas

- **Background daemon for prune** — `nono` does not have a persistent daemon today. Adding one for retention is out of scope; the `nono ps` trigger is sufficient for v2.1.
- **Session file format migration** — if any of the existing 1172 sessions have an old schema, the prune just deletes them; no migration step. Real schema migration would be a separate phase.
- **CLEAN-02 deeper refactors** — if a failing test reveals a real API design problem (not a test-isolation bug), STOP and route to its own phase rather than fixing in CLEAN-02 (per D-08).
- **Cross-platform retention default tuning** — 30 days was picked as a reasonable Windows default; Linux/macOS may want different values once the retention infrastructure exists. Tracked as a potential v2.2 polish item, not a v2.1 must-have.

</deferred>

---

*Phase: 19-cleanup*
*Context gathered: 2026-04-18*
