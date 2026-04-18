---
phase: 14-v1-fix-pass
plan: 03
completed: 2026-04-18T02:45:00Z
status: complete
---

## What built

Finished the v1.0 Windows Parity milestone UAT bookkeeping — corrected the two
Phase 13 runbook defects that rendered P05-HV-1 and P09-HV-1 un-runnable, ran
the 2nd-pass UAT on all 6 target items, and promoted the four upstream
`*-VERIFICATION.md` files per the Phase 13 Task 3 outcome-handling matrix.

## Tasks

### Task 1 — Runbook corrections (commit `647e0a5`)

Two surgical edits to `13-UAT.md`:
- **P05-HV-1 command:** `nono run --detach -- ...` → `nono run --detached -- ...`
  (`--detach` is not a valid `nono run` flag; `detach` is a separate subcommand).
- **P09-HV-1 command + prereqs:** replaced the nonexistent `--proxy-only` with
  the real flags `--network-profile PROFILE` + `--credential SERVICE` +
  `--upstream-proxy HOST:PORT` (see `crates/nono-cli/src/cli.rs:938-1010`).
  Expanded the prereqs block to explicitly document the admin-shell +
  `nono setup --install-wfp-service` + `nono setup --start-wfp-service` chain.

### Task 2 — 2nd-pass UAT (user-executed on admin Win11 host, 2026-04-18)

Results (full evidence in `13-UAT.md` § Summary and per-item `notes:`):

| Item      | 1st-pass | 2nd-pass | Driver |
|-----------|----------|----------|--------|
| P05-HV-1  | blocked  | waived (v1.0-known-issue) | Bug #3 residual — Phase 15 |
| P07-HV-2  | fail     | **pass**                  | 14-02 fix (commit `8e200f8`) |
| P07-HV-3  | blocked  | waived (v1.0-known-issue) | Prereq P05-HV-1 |
| P09-HV-1  | blocked  | waived (no-test-fixture)  | Runbook typo fixed; live E2E blocked on missing built-in network profile |
| P11-HV-1  | blocked  | waived (v1.0-known-issue) | Supervised detached path = Bug #3 |
| P11-HV-3  | blocked  | waived (v1.0-known-issue) | Supervised detached path = Bug #3 |

Summary count moved from `passed 2 / issues 1 / blocked 5 / waived 2` to
`passed 3 / issues 0 / blocked 0 / waived 7`. All 10 items now have terminal
verdicts (no `blocked`, no `pending`).

### Task 3 — Upstream VERIFICATION.md promotion (commit `6500bb1`)

Applied the Phase 13 outcome-handling matrix:

| File                                                                    | Before         | After  | Notes added |
|-------------------------------------------------------------------------|----------------|--------|-------------|
| `.planning/phases/05-windows-detach-readiness-fix/05-VERIFICATION.md`   | `passed`       | `passed` | P05-HV-1 v1.0-known-issue addendum |
| `.planning/phases/07-quick-wins/07-VERIFICATION.md`                     | `passed`       | `passed` | P07-HV-2 `fail→pass` verdict + P07-HV-3 addendum |
| `.planning/phases/09-wfp-port-level-proxy-filtering/09-VERIFICATION.md` | `human_needed` | **`passed`** | P09-HV-1 waiver (no-test-fixture), P09-HV-2 pass |
| `.planning/phases/11-runtime-capability-expansion/11-VERIFICATION.md`   | `human_needed` | **`passed`** | P11-HV-1/HV-3 v1.0-known-issue, P11-HV-2 waived |

Both `human_needed` promotions now explicitly document v1.0-known-issue
carry-forwards inline so the phase history remains honest.

## Verification

| Check | Result |
|-------|--------|
| Runbook contains `--detached` (not `--detach`) for P05-HV-1 | ✓ (13-UAT.md line 77) |
| Runbook contains `--network-profile`/`--credential`/`--upstream-proxy` for P09-HV-1 | ✓ (13-UAT.md line 170-172) |
| No UAT item in `blocked` state | ✓ (Summary: `blocked: 0`) |
| No UAT item in `pending` state | ✓ |
| 09-VERIFICATION.md status = passed | ✓ |
| 11-VERIFICATION.md status = passed | ✓ |
| v1.0-known-issue items documented in CHANGELOG.md | ✓ (commit `c00d709`) |
| Phase 15 created for the carry-forward bug | ✓ (commit `dc71474`) |

## Decisions

- **v1.0 ships with the detached-console-grandchild bug as a documented known
  issue.** Per the user's explicit decision 2026-04-18. Non-detached mode is
  fully functional; users who need sandboxed console processes on Windows can
  use `nono run -- <cmd>` directly. GUI apps in detached mode work. The 4
  affected UAT items (P05-HV-1, P07-HV-3, P11-HV-1, P11-HV-3) are explicitly
  waived as `v1.0-known-issue` and carry forward to Phase 15.
- **P09-HV-1 is a `no-test-fixture` waiver, not a code defect.** The corrected
  runbook is verified — live end-to-end execution just needs a
  network-profile-with-credentials that isn't shipped out of the box. Users
  with configured network profiles can verify via the runbook.
- **Upstream promotions preserve honesty.** Neither 09 nor 11 became a clean
  unconditional `passed` — both got promoted with documented waivers inline so
  a future reader sees exactly which UAT legs ran live and which were waived.

## Execution notes

- Task 1 executed inline (trivial text edits).
- Task 2 executed by the human tester on their admin Win11 host. Paste of the
  PowerShell output was captured in the orchestrator session and applied
  verbatim to `13-UAT.md`.
- Task 3 executed inline using the verdicts from Task 2.
- Commits: `647e0a5` (Task 1), `14a9ef0` (Task 2 = 13-UAT.md update),
  `6500bb1` (Task 3 = upstream promotions).
- No code changes; documentation only.

## Follow-up

- **Phase 15** — already created. Run `/gsd-plan-phase 15` to scope the
  detached-console + ConPTY architecture investigation.
- **Milestone archive** — v1.0 Windows Parity is now eligible for archive
  with documented known-issue carry-forward items. User decision pending on
  when to actually archive.
- **Stale `.nono-<hex>.json` files** — 55 files in the project root CWD
  produce debug noise on every invocation. Non-blocking. Noted in 13-UAT.md
  Gaps § "Additional observations" — worth a gsd-note or a one-line cleanup
  follow-up.
