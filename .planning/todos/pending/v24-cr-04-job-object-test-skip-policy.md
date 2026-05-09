---
title: "Decide silent-SKIP-as-PASS policy for broker_launch_assigns_child_to_job_object (CR-04 secondary from Phase 31 review)"
created: 2026-05-09
source: .planning/phases/31-broker-process-architecture-shell-01/31-REVIEW.md (CR-04 secondary)
target_milestone: v2.4
priority: low
---

# CR-04 (secondary): Job Object containment test silent-SKIP shape

`crates/nono-cli/src/exec_strategy_windows/launch.rs` test `broker_launch_assigns_child_to_job_object` (Plan 31-05 lifted from `#[ignore]`) silently returns when the broker artifact is absent at `target/x86_64-pc-windows-msvc/release/nono-shell-broker.exe` rather than failing. The test prints a SKIP diagnostic via `eprintln!` and exits cleanly. CR-04 flagged this as a false-PASS class for unaware CI runs.

**Verifier note (from `31-VERIFICATION.md`):** Phase 31 acceptance is unaffected — the field-test runner has the broker artifact built, so the assertion path executes. The default `cargo test -p nono-cli` for developers who haven't built the broker yet stays green via the SKIP path. This was Plan 31-05's intentional design.

**Decision required for v2.4 CI matrix expansion (Win10 22H2 / Win11 23H2 / Server 2022):**
- (a) **Accept the SKIP-as-PASS shape** because Plan 31-05 owns the runtime acceptance via field-test, and the SKIP keeps developer UX clean. CI matrix uses a wrapper that builds the broker before running the test.
- (b) **Add `#[ignore]` back** — the test shows as ignored (not passed) when the artifact is missing; CI uses `cargo test -- --ignored` to opt in.
- (c) **Convert SKIP to FAIL** — if the artifact is missing, `panic!`. Forces every cargo test invocation to pre-build the broker.

**Source:** Phase 31 code review report `31-REVIEW.md` finding CR-04 (secondary).
**Impact:** CI signal quality. Production code unaffected.
**Acceptance gate:** Pick one of (a)/(b)/(c); apply; verify the chosen behavior with a missing-artifact test.
