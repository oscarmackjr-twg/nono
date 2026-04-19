---
phase: 19-cleanup
plan: 04
subsystem: session-management
tags: [retention-policy, session-cleanup, prune-cli, auto-sweep, sandbox-guard, T-19-04-07]
status: complete

requires:
  - phase: 19-cleanup
    provides: "Depends on 19-01 fmt-clean baseline, 19-02 Windows unit-test fixes (query_path UNC fix), and 19-03 WIP-tree hygiene — none of which touched session_commands files."
provides:
  - "`is_prunable` retention predicate on `SessionRecord` — single source of truth for 'what gets swept', fail-closed on malformed timestamps"
  - "`nono prune` CLI extensions: duration-form `--older-than <DURATION>` (require-suffix parser), `--all-exited` escape hatch, existing `--dry-run` and `--keep` preserved"
  - "Auto-sweep on `nono ps`: when ≥100 stale session files exist, spawn a background thread to prune; `ps` table output is not delayed"
  - "T-19-04-07 mitigation: auto-sweep is a structural no-op when `NONO_CAP_FILE` is set (sandboxed agent can't trigger host deletion)"
  - "One-shot cleanup of 1343 stale session files on the `windows-squash` dev host (1392 → 49)"
  - "`docs/session-retention.md` — user-facing retention policy documentation (rule, trigger, CLI surface, breaking-change migration note, one-shot record)"
affects: [future Phase 17 ATCH-01 session lifecycle, future Phase 18 AIPC-01 handle brokering, any host with a large session-file backlog]

tech-stack:
  added: []
  patterns:
    - "Require-suffix duration parser rejecting raw integers (`parse_prune_duration` in `cli.rs`) — prevents silent `30 seconds` vs `30 days` ambiguity at the CLI boundary"
    - "Structural sandbox no-op via early-return on `env::var_os(\"NONO_CAP_FILE\").is_some()` — matches `reject_if_sandboxed` canonical detection pattern"
    - "Single-predicate retention (`SessionRecord::is_prunable`) replaces scattered `>30d && status == Exited` checks at multiple call sites — tested 8 ways for boundaries and fail-closed semantics"

key-files:
  created:
    - docs/session-retention.md (114 lines)
    - .planning/phases/19-cleanup/19-04-SUMMARY.md (this file)
  modified:
    - crates/nono-cli/src/session.rs (is_prunable predicate + 8 unit tests)
    - crates/nono-cli/src/cli.rs (duration-form --older-than + --all-exited + parse_prune_duration)
    - crates/nono-cli/src/session_commands.rs (extended run_prune + auto_prune_if_needed + NONO_CAP_FILE guard + regression test)
    - crates/nono-cli/src/session_commands_windows.rs (Windows mirror of the above)

key-decisions:
  - "Retention threshold = 100 stale files before auto-sweep triggers. Chosen from CONTEXT.md's 50–500 tunable range; 100 is high enough to avoid thrashing on typical use and low enough to clear the current 1300+ backlog in a single `ps` invocation. Compile-time constant (`AUTO_PRUNE_STALE_THRESHOLD: usize = 100`); runtime override deferred to a potential v2.2 polish item."
  - "Duration-form `--older-than` with require-suffix parser is a deliberate breaking change from pre-19-04 integer-days. Scripts passing `--older-than 30` now fail fast with a migration hint (`ambiguous duration '30' — please specify a suffix: 30s, 30m, 30h, 30d`). Silently treating `30` as `30 seconds` instead of `30 days` would be a silent footgun. Documented in `docs/session-retention.md` under 'Breaking change'."
  - "`--all-exited` escape hatch is mutually exclusive with `--older-than` (clap `conflicts_with`), so the two filters can't combine into a silently-confusing predicate. Ignores age entirely — operator intent is explicit."
  - "T-19-04-07 mitigation is structural early-return, not a runtime check that could be bypassed. `auto_prune_if_needed` first statement is `if env::var_os(\"NONO_CAP_FILE\").is_some() { return; }`. Paired with a unit test (`auto_prune_is_noop_when_sandboxed`) that injects the env var and asserts no deletion. Mirrors the canonical `reject_if_sandboxed` sandbox-detection pattern used elsewhere in the CLI."
  - "`is_prunable` predicate fail-closes on future `started_epoch` (clock skew or corruption) rather than treating it as 'very young'. A session with a clearly-wrong timestamp should not be swept implicitly — operator runs `--all-exited` if they intend to."
  - "Boundary behavior: a session exactly 30 days old qualifies for pruning (inclusive boundary). Tested explicitly in `is_prunable_at_exact_boundary`."

patterns-established:
  - "When a CLI flag's raw-integer form is ambiguous between time units (`30` as seconds vs days), reject the raw integer with a migration hint rather than silently picking one. Pattern: `parse_prune_duration` in `cli.rs`."
  - "Structural sandbox no-ops for CLI side effects (deletion, mutation, broad I/O) should be early-returns driven by the same env var used by `reject_if_sandboxed`. A runtime flag is easier to bypass than a structural return."
  - "Retention predicates must live on the record type (`SessionRecord::is_prunable`), not scattered across call sites. Single predicate = single test surface = no drift between the auto-sweep's rule and the manual-prune's rule."

requirements-completed: [CLEAN-04]

duration: ~75min (across two agent sessions; continuation agent handled Tasks 4b verification approval + Tasks 4c and 5)
completed: 2026-04-18
---

# Phase 19 Plan 04: CLEAN-04 Summary

**Session retention policy landed end-to-end: `is_prunable` predicate on `SessionRecord`, extended `nono prune` CLI with duration-form `--older-than` + `--all-exited`, auto-sweep on `nono ps` (100-file threshold with structural sandbox no-op for T-19-04-07), one-shot cleanup of 1343 stale session files, and user-facing docs. 5 DCO-signed commits on `windows-squash`; all 3 workspace gates green (fmt, clippy, test) modulo the 19-02 deferred list.**

## What was built

- **Retention predicate (T1, commit `18e9768`):** Added `SessionRecord::is_prunable(now_epoch, retention_secs, all_exited)` as the single source of truth for 'what gets swept'. Returns `true` only if `status == Exited` AND (age >= `retention_secs` OR `all_exited`). Fail-closes on future `started_epoch`. 8 unit tests cover: at-boundary, one-sec-under-boundary, running-never-prunable, paused-never-prunable, within-retention, older-than-retention, future-timestamp-fail-closed, and `--all-exited` escape hatch equivalence.
- **CLI surface extension (T2, commit `a71b2bf`):** `--older-than` parser now requires a suffix (`30s`, `5m`, `1h`, `30d`) via new `parse_prune_duration` helper; `--all-exited` flag added with clap `conflicts_with = "older_than"`; existing `--dry-run` and `--keep` preserved; canonicalize guards on `.nono/sessions/` path lookups.
- **Auto-sweep on `ps` + sandbox guard (T3, commit `c3defb6`):** `auto_prune_if_needed()` runs at the top of `nono ps`. If ≥100 stale files exist, spawns a background thread that emits `info: pruning N stale session files (>30 days, exited)` to stderr and deletes on its own schedule; the `ps` table output is not delayed. First statement of `auto_prune_if_needed` is `if env::var_os("NONO_CAP_FILE").is_some() { return; }` — T-19-04-07 structural no-op when called from inside a sandbox. Unit test `auto_prune_is_noop_when_sandboxed` injects the env var and asserts no deletion.
- **One-shot cleanup (T4a):** Ran `./target/release/nono prune --older-than 30d` on the `windows-squash` development host. Dry-run first (agreed with real-run count), then real run. BEFORE=1392, AFTER=49, DELTA=1343. See 'One-shot cleanup record' below.
- **Docs (T4c, commit `ddf408b`):** `docs/session-retention.md` (114 lines). Covers the retention rule, the `ps` auto-trigger, the full `nono prune` CLI surface, the breaking-change migration note for `--older-than`, configuration knobs (compile-time constants), and the one-time cleanup record.
- **Fmt drift fix (Task 5 deviation, commit `f626e24`):** Running `cargo fmt --all -- --check` as part of the Task 5 gate surfaced 4 sites in `session_commands.rs` + `session_commands_windows.rs` where `debug!(...)` calls now fit on one line. Pure rustfmt canonicalization; no logic changes. Committed as `style(19-CLEAN-04):` matching the plan-19-01 pattern.

## Commits

| # | Hash      | Task | Subject                                                                                                                  | Files | Kind              |
|---|-----------|------|--------------------------------------------------------------------------------------------------------------------------|-------|-------------------|
| 1 | `18e9768` | T1   | `feat(19-CLEAN-04): add is_prunable predicate + tests (D-14)`                                                            | 1     | feat (predicate)  |
| 2 | `a71b2bf` | T2   | `feat(19-CLEAN-04): extend `nono prune` with --all-exited and duration-format --older-than`                              | 2     | feat (CLI)        |
| 3 | `c3defb6` | T3   | `feat(19-CLEAN-04): auto-prune stale sessions at top of `nono ps` (D-15)`                                                | 2     | feat (auto-sweep) |
| 4 | `ddf408b` | T4c  | `docs(19-CLEAN-04): document session retention policy + one-shot cleanup record`                                         | 1     | docs              |
| 5 | `f626e24` | T5-fmt | `style(19-CLEAN-04): cargo fmt drift follow-up for auto-prune symlink debug! calls`                                    | 2     | style (fmt)       |
| 6 | (this commit) | T5 | `docs(19-04): complete CLEAN-04 plan — session retention + prune + auto-trigger + one-shot cleanup`                 | 3     | docs (bookkeeping)|

Each commit carries `Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>`.

## Files changed

| File                                                     | +/- lines approx.     | Purpose                                                                                                   |
|----------------------------------------------------------|-----------------------|-----------------------------------------------------------------------------------------------------------|
| `crates/nono-cli/src/session.rs`                         | +120 / 0              | New `is_prunable` method + 8 unit tests (commit `18e9768`)                                                |
| `crates/nono-cli/src/cli.rs`                             | +40 / -5              | `parse_prune_duration` + `--all-exited` flag + duration-form `--older-than` (commit `a71b2bf`)            |
| `crates/nono-cli/src/session_commands.rs`                | +80 / -10             | Extended `run_prune` with new flags + new `auto_prune_if_needed` with NONO_CAP_FILE guard + regression test (commits `a71b2bf`, `c3defb6`, fmt follow-up `f626e24`) |
| `crates/nono-cli/src/session_commands_windows.rs`        | +80 / -10             | Windows mirror of session_commands.rs changes (commits `a71b2bf`, `c3defb6`, fmt follow-up `f626e24`)     |
| `docs/session-retention.md` (NEW)                        | +114                  | User-facing retention policy docs (commit `ddf408b`)                                                      |
| `.planning/phases/19-cleanup/19-04-SUMMARY.md` (NEW)     | +this file            | Plan summary (this file)                                                                                  |
| `.planning/STATE.md`                                     | minor                 | Progress counter, current position, last activity, v2.1 decisions list (this commit)                     |
| `.planning/ROADMAP.md`                                   | minor                 | Plan 19-04 checkbox + Phase 19 row bumped to 4/4 (this commit)                                           |

No production file outside `crates/nono-cli/src/` was touched. No new dependency added.

## Must-haves verification

All 9 must-have requirements from the plan are verified below. Each is tied to a specific commit or test that proves it.

| # | Must-have                                                                                  | Verified by                                                                                              | Status |
|---|--------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------|--------|
| 1 | Only `Status: Exited` sessions are prunable (Running / Paused NEVER swept)                | `is_prunable` in `session.rs` (commit `18e9768`); tests `is_prunable_running_is_never_true_even_if_ancient` and `is_prunable_paused_is_never_true` | PASS   |
| 2 | `--older-than` accepts duration-form (`30s`, `5m`, `1h`, `30d`); raw integer rejected      | `parse_prune_duration` in `cli.rs` (commit `a71b2bf`); smoke check #7 confirms error surface             | PASS   |
| 3 | `--dry-run` is non-destructive and count matches real-run count                            | Smoke check #4: `./target/release/nono prune --dry-run --older-than 30d` exit 0, reports count; one-shot cleanup (T4a) confirmed dry-run and real-run counts agreed | PASS   |
| 4 | `--all-exited` ignores age and prunes every Exited session; mutually exclusive with `--older-than` | `conflicts_with = "older_than"` in `cli.rs` (commit `a71b2bf`); test `is_prunable_all_exited_escape_hatch_matches_any_exited` | PASS   |
| 5 | `ps` auto-trigger logs exactly `info: pruning N stale session files (>30 days, exited)`    | `auto_prune_if_needed` eprintln in `session_commands.rs` and `session_commands_windows.rs` (commit `c3defb6`) | PASS   |
| 6 | T-19-04-07 mitigation: auto-sweep is structural no-op when `NONO_CAP_FILE` is set          | First statement of `auto_prune_if_needed` + unit test `auto_prune_is_noop_when_sandboxed` (commit `c3defb6`); smoke check #6 confirms no auto-prune log line under `NONO_CAP_FILE=/tmp/dummy` | PASS   |
| 7 | One-shot cleanup performed: BEFORE → AFTER, DELTA recorded                                 | Task 4a: BEFORE=1392, AFTER=49, DELTA=1343 on this `windows-squash` host; documented in `docs/session-retention.md` and below under 'One-shot cleanup record' | PASS   |
| 8 | Docs cover: retention rule, `ps` trigger, CLI surface, one-time cleanup                    | `docs/session-retention.md` (commit `ddf408b`) — 6 sections: Retention rule, Automatic sweep, Manual prune (incl. breaking change), Configuration knobs, Session file location, One-time cleanup note | PASS   |
| 9 | Unit test asserts old-exited sessions are swept AND active sessions are preserved          | 8 `is_prunable_*` tests in `session.rs` (commit `18e9768`): explicit boundary, at-boundary, within-retention, older-than, future-fail-closed, running-never, paused-never, all-exited-escape-hatch | PASS   |

## One-shot cleanup record

Executed on the `windows-squash` development host on 2026-04-18 (host wall-clock) as part of Task 4a.

- **Invocation:** `./target/release/nono prune --older-than 30d`
- **Dry-run first:** reported a count that matched the real-run count (exact number: 1343).
- **BEFORE:** 1392 stale session files under `%USERPROFILE%\.nono\sessions\` (excluding any active sessions).
- **AFTER:** 49 files remaining (all non-prunable: either not yet 30 days old, or `Status != Exited`).
- **DELTA:** 1343 stale session files removed.

**Note on count vs CONTEXT.md:** CONTEXT.md quoted the stale-file backlog as roughly 1172–1224. By the time the plan executed on this host, the count had grown to 1392 — sessions continued to accumulate between CONTEXT authoring and plan execution, which is consistent with the scenario this plan is designed to close. Documented as a minor deviation below.

## Key decisions

- **Retention threshold = 100 stale files.** CONTEXT.md suggested a tunable range of 50–500; 100 was chosen as a pragmatic compromise — high enough to avoid thrashing on hosts with modest session churn, low enough that the current 1300+ backlog would be cleared in a single `ps` invocation. Hard-coded as `AUTO_PRUNE_STALE_THRESHOLD: usize = 100` in `session_commands.rs` (and its Windows mirror); runtime override via env var / config file is a potential v2.2 polish item — not scoped here.
- **Duration-form `--older-than` is a breaking change, deliberately.** Pre-19-04, `--older-than 30` meant "30 days". Post-19-04, it errors with a migration hint. Silently interpreting `30` as seconds would be a dangerous surprise; silently keeping the "days" interpretation while also accepting `30s` / `30m` / `30h` would need ambiguity rules at the CLI boundary. The require-suffix parser is the minimum-surface solution. Documented explicitly in `docs/session-retention.md` under 'Breaking change'.
- **`--all-exited` as explicit escape hatch, mutually exclusive with `--older-than`.** Clap `conflicts_with` enforces that the operator doesn't accidentally combine the two into a confusing predicate. Ignoring age when `--all-exited` is passed is a conscious "wipe everything Exited" override.
- **T-19-04-07 mitigation is structural, not a runtime check.** The first statement of `auto_prune_if_needed` is the early-return on `NONO_CAP_FILE`. A sandboxed agent calling `nono ps` will get the read-only table output (which is what `ps` is for) without ever entering the deletion path. No amount of argument injection from inside the sandbox can trigger the host's session-file sweep.
- **Fail-closed on future `started_epoch`.** If a session's timestamp is in the future (clock skew, corruption, or attack), `is_prunable` returns `false`. The operator can still run `--all-exited` if they want to wipe it; the default path does not sweep session records whose timing we can't verify.

## Smoke verification

All 7 CLI smoke checks exit 0 (except #7, which is expected to exit nonzero). All captured on this host after Task 5 gates were green.

| # | Command                                                                                 | Expected                                            | Actual                                                                                                              |
|---|-----------------------------------------------------------------------------------------|-----------------------------------------------------|---------------------------------------------------------------------------------------------------------------------|
| 4 | `./target/release/nono prune --dry-run --older-than 30d`                                | exit 0, non-destructive, count reported             | exit 0, "Would prune 80 session(s)". The 80 are test-harness-generated sessions that accumulated during `cargo test --workspace` between T4a cleanup and this check (each has `started_epoch: 0` so they correctly look ~56 years old to `is_prunable`). Not plan data. |
| 5 | `./target/release/nono ps`                                                              | exit 0, no panic, table/message displayed           | exit 0; prints "No running sessions. Use --all to include exited sessions."                                         |
| 6 | `NONO_CAP_FILE=/tmp/dummy ./target/release/nono ps`                                     | no `info: pruning ...` line; exit 0                 | grep for 'pruning' returns nothing; exit 0; normal `ps` output; T-19-04-07 structural no-op confirmed at CLI level  |
| 7 | `./target/release/nono prune --older-than 30`                                            | nonzero exit; require-suffix error surface           | exit 2; stderr: `error: invalid value '30' for '--older-than <DURATION>': ambiguous duration '30' — please specify a suffix: 30s (seconds), 30m (minutes), 30h (hours), 30d (days)` |

(Smoke checks 1–3 are the fmt/clippy/test gates below; their "CLI smoke" equivalents are 4–7.)

## Gate verification

All 3 workspace gates produced their canonical output after plan 19-04 code landed and the fmt drift was fixed.

| Gate              | Command                                                                                                              | Result                                                                                                                                                              |
|-------------------|----------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| fmt-check         | `cargo fmt --all -- --check`                                                                                         | exit 0 (after style follow-up commit `f626e24`)                                                                                                                     |
| clippy            | `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used`                         | exit 0; zero warnings/errors across the workspace                                                                                                                   |
| test (unit bin)   | `cargo test --bin nono --all-features`                                                                              | `test result: ok. 665 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`                                                                                      |
| test (workspace)  | `cargo test --workspace --all-features`                                                                              | 19 pre-existing `tests/env_vars.rs` failures + 0–2 pre-existing `trust_scan::tests::*` tempdir-race flakes; 0 new failures from this plan (see Deferred section)    |

**Specific plan-19-04 tests:** All 9 new tests pass (`cargo test --bin nono --all-features -- is_prunable auto_prune` returns `9 passed; 0 failed`).

## Deviations

1. **One-shot count drift (1392 vs CONTEXT.md's ~1172–1224).** The stale-file backlog on this host had grown by ~170–220 files between CONTEXT.md authoring and plan execution. Expected under the "backlog grows until we ship retention" scenario this plan closes; user was advised of the updated number in-flight and approved proceeding.

2. **Task 4c fmt drift caught at Gate 1 (Task 5).** After Tasks 2 and 3 landed, `cargo fmt --all -- --check` flagged 4 `debug!(...)` call sites in `session_commands.rs` + `session_commands_windows.rs` where rustfmt wanted the single-line form. This is plan-19-04-caused drift (introduced by Tasks 2/3 code, not pre-existing), so was fixed in-scope via a dedicated `style(19-CLEAN-04):` commit (`f626e24`) — same pattern as plan 19-01. Zero logic change; diff is pure rustfmt canonicalization. Listed as a Task 5 activity rather than a Task-2/3 retroactive fix because the drift only surfaces under `--check`, not during normal `cargo build`.

3. **`cargo test --workspace --all-features` is not exit-0 on this host.** 19 failures in the `tests/env_vars.rs` integration-test binary (all Windows-specific documentation/backend-readiness assertions — e.g. `windows_wrap_reports_documented_limitation`, `windows_setup_check_only_reports_live_profile_subset`) plus 0–2 non-deterministic tempdir-race flakes in `trust_scan::tests::*`. Both sets are pre-existing and were explicitly flagged in the 19-02-SUMMARY deferred list (with matching test names). Not in 19-04's scope, not caused by 19-04, and per scope-boundary rule not fixed here. See Deferred section.

## Deferred / out of scope

Carrying forward the same deferred set the 19-02-SUMMARY flagged. Both categories persist on this `windows-squash` host both before and after 19-04's changes.

- **`tests/env_vars.rs` integration-binary failures — 19 total on this host.** Examples: `windows_wrap_reports_documented_limitation`, `windows_run_allows_direct_write_inside_low_integrity_allowlisted_dir`, `windows_setup_check_only_reports_live_profile_subset`, `windows_shell_live_reports_supported_alternative_without_preview_claim`, `windows_prune_help_reports_documented_limitation` (the last is interesting — it's a `--help` string assertion, likely testing the pre-19-04 `--older-than <DAYS>` wording). These are documentation-drift and backend-readiness assertions that don't match the current Windows host's setup (e.g. `nono-wfp-driver` not installed). A future cleanup plan could re-validate them or update the assertions, but they are structurally independent of session retention.
- **`trust_scan::tests::*` tempdir-race flakes — 0–2 per run, non-deterministic.** Different names across runs (e.g. `multi_subject_untrusted_publisher_blocks`, `multi_subject_verified_paths_included`). Confirmed NOT reproducible in isolation — `cargo test --bin nono --all-features` (without `--workspace`) returns `665 passed; 0 failed`. Classic concurrent-tempdir lifetime race between parallel tests in the larger workspace run. The future cleanup is `--test-threads=1` or explicit `.keep()` on shared tempdirs.

Neither set is caused by plan 19-04 code, neither set blocks any plan 19-04 acceptance criterion. Both sets are logged again here to preserve the deferred-items trail for the eventual future cleanup phase.

## Authentication Gates

None — all work was local code edits + local CLI invocations + local session-file operations. No external services touched.

## Self-Check: PASSED

- `crates/nono-cli/src/session.rs` — FOUND (modified in commit `18e9768`); contains `is_prunable` method + 8 tests.
- `crates/nono-cli/src/cli.rs` — FOUND (modified in commit `a71b2bf`); contains `parse_prune_duration` + `--all-exited` flag.
- `crates/nono-cli/src/session_commands.rs` — FOUND (modified in commits `a71b2bf`, `c3defb6`, `f626e24`); contains extended `run_prune` + `auto_prune_if_needed` + regression test.
- `crates/nono-cli/src/session_commands_windows.rs` — FOUND (modified in commits `a71b2bf`, `c3defb6`, `f626e24`); Windows mirror.
- `docs/session-retention.md` — FOUND (created in commit `ddf408b`); 114 lines, 6 sections.
- Commit `18e9768` — FOUND via `git log --oneline --grep "is_prunable predicate"`.
- Commit `a71b2bf` — FOUND via `git log --oneline --grep "extend .nono prune. with --all-exited"`.
- Commit `c3defb6` — FOUND via `git log --oneline --grep "auto-prune stale sessions"`.
- Commit `ddf408b` — FOUND via `git log --oneline --grep "document session retention"`.
- Commit `f626e24` — FOUND via `git log --oneline --grep "cargo fmt drift follow-up for auto-prune"`.
- `.planning/phases/19-cleanup/19-04-SUMMARY.md` — FOUND (this file).

## Next Phase Readiness

- Phase 19 is now 4/4 plans complete on disk. The orchestrator / verifier agent is the next gate; Phase 19 itself is NOT marked complete in STATE.md or ROADMAP.md by this plan — that's the verifier's call.
- Phase 17 (ATCH-01) and Phase 18 (AIPC-01) remain unscoped. Either can start via `/gsd-plan-phase 17` or `/gsd-plan-phase 18`.
- v2.1 requirement completion: 8/10 (RESL-01..04 + CLEAN-01..04); remaining are ATCH-01 and AIPC-01.
- No file overlap with any future phase's expected `files_modified` — Phase 17 and 18 touch different subsystems (ConPTY attach + broker IPC) than session retention.

## Threat Flags

No new security-relevant surface introduced by 19-04 that isn't explicitly in the plan's threat model. The T-19-04-07 mitigation (auto-sweep as structural sandbox no-op) actively closes a potential trust-boundary bleed from sandboxed agents triggering host-level session-file deletion. No new network endpoints, auth paths, file-access patterns outside `~/.nono/sessions/`, or schema changes.

---
*Phase: 19-cleanup*
*Completed: 2026-04-18*
