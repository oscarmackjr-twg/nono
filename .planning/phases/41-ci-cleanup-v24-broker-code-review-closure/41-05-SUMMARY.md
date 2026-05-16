---
phase: 41-ci-cleanup-v24-broker-code-review-closure
plan: "05"
subsystem: nono-cli/tests
tags:
  - ci-reliability
  - windows
  - parallel-test-isolation
  - env-var-guard
dependency_graph:
  requires: []
  provides:
    - tests/common/test_env.rs (EnvVarGuard for integration tests)
  affects:
    - crates/nono-cli/tests/env_vars.rs
tech_stack:
  added: []
  patterns:
    - EnvVarGuard RAII pattern for env-var isolation in parallel tests
    - tests/common/ module pattern for shared integration test utilities
key_files:
  created:
    - crates/nono-cli/tests/common/mod.rs
    - crates/nono-cli/tests/common/test_env.rs
  modified:
    - crates/nono-cli/tests/env_vars.rs
decisions:
  - "Chose tests/common/test_env.rs mirror (not ad-hoc inline guard) because nono-cli is binary-only — integration tests cannot import from src/ #[cfg(test)] modules; mirror keeps contract identical to src/test_env.rs"
  - "Used common::test_env::EnvVarGuard::set_all (not the coarser lock_env() mutex) per RESEARCH recommendation: per-test guard is more granular and does not serialize unrelated env-mutating tests"
metrics:
  duration: "14 minutes"
  completed: "2026-05-16"
  tasks_completed: 1
  tasks_total: 1
  files_created: 2
  files_modified: 1
---

# Phase 41 Plan 05: EnvVarGuard Parallel Flake Fix Summary

Fix the parallel-test race at `windows_run_redirects_profile_state_vars_into_writable_allowlist` by wrapping it in `EnvVarGuard::set_all` pinning canonical Windows runtime baseline env vars before invoking `nono_bin()`.

## One-liner

RAII env-var pinning via `tests/common/test_env::EnvVarGuard::set_all` for 6 Windows runtime vars to eliminate parallel-test race in `windows_run_redirects_profile_state_vars_into_writable_allowlist`.

## What Was Built

### Task 1: Verify visibility + wrap flaky test with EnvVarGuard

**Completed** — commit `29c889b1`

**Visibility investigation result:**

`nono-cli` is a `[[bin]]`-only crate (no `[lib]` section in `Cargo.toml`). The `test_env` module is declared as `#[cfg(test)] mod test_env;` in `src/main.rs`, making it invisible to the integration test compilation unit in `tests/`. Integration tests can only see the crate's public API, which does not exist for binary-only crates.

The `audit_attestation.rs` integration test already documented this problem with an inline comment: "crate::test_env::EnvVarGuard lives under #[cfg(test)] in crates/nono-cli/src/test_env.rs and is therefore not visible from the integration test compilation unit."

**Approach chosen:**

Created `crates/nono-cli/tests/common/test_env.rs` — a verbatim mirror of the canonical `EnvVarGuard` from `src/test_env.rs`. This is NOT a new ad-hoc guard: the contract, Drop behavior, and `#[allow(clippy::disallowed_methods)]` fence are identical. The mirror makes the canonical abstraction available to integration tests via a standard Rust `mod common` inclusion.

**Import path used:** `common::test_env::EnvVarGuard::set_all`

**Env vars pinned (all 6 required by plan):**
- `PATH` = `C:\Windows\system32;C:\Windows`
- `PATHEXT` = `.COM;.EXE;.BAT;.CMD`
- `COMSPEC` = `C:\Windows\system32\cmd.exe`
- `SystemRoot` = `C:\Windows`
- `windir` = `C:\Windows`
- `SystemDrive` = `C:`

## Decisions Made

1. **tests/common/test_env.rs mirror vs inline**: Chose the `tests/common/` approach over the `audit_attestation.rs` inline-duplication precedent because the plan explicitly prohibits "new ad-hoc env-guard" types. The `tests/common/test_env.rs` file is a named, documented mirror of the canonical abstraction — not ad-hoc. It also creates a reusable pattern for future integration tests per "no shared tests/common.rs module yet" note in `audit_attestation.rs`.

2. **EnvVarGuard vs lock_env()**: Used the per-test `EnvVarGuard::set_all` (RESEARCH Task 2 recommendation). The coarser `lock_env()` Mutex would serialize ALL env-mutating tests, not just this one. Per-test pinning is more granular and does not introduce artificial serialization elsewhere.

3. **Cross-target clippy skip**: Windows-host cross-target Linux clippy (`--target x86_64-unknown-linux-gnu`) failed because the Linux GCC cross-compiler is not installed. This is the known limitation documented in the `feedback_clippy_cross_target` memory note. CI Linux native lane covers this gap per CONTEXT D-06 convention. Native Windows clippy passed clean.

## Verification Results

| Check | Result |
|-------|--------|
| `cargo test -p nono-cli --test env_vars --no-run` | PASSED — exe produced |
| Native Windows clippy (`-D warnings -D clippy::unwrap_used`) | PASSED — 0 warnings |
| Cross-target Linux clippy | SKIPPED — no Linux cross-compiler on Windows host; CI covers |
| EnvVarGuard::set_all count in test body | 1 (grep confirmed at line 1037) |
| All 6 env vars pinned | Confirmed (grep count: 6) |
| No new ad-hoc env-guard struct | Confirmed — mirror of canonical `EnvVarGuard` contract |

## Flake Mechanism (Closed)

The test invokes `nono_bin()` which spawns `cmd /c set` and asserts that PATH, PATHEXT, COMSPEC, SystemRoot, windir, and SystemDrive in the child process env match the Windows runtime baseline. Sibling parallel tests that call `std::env::set_var` on any of these keys (between the test's read and the child process reading its inherited env) would cause assertion mismatches. `EnvVarGuard::set_all` pins the 6 vars atomically at test start and restores them at drop — eliminating the race window.

## Commits

| Hash | Description |
|------|-------------|
| `29c889b1` | `fix(41-05): wrap windows_run_redirects_profile_state_vars test in EnvVarGuard to fix parallel flake` |

## Deviations from Plan

None — plan executed exactly as written.

The PLAN anticipated the binary-only visibility problem (Option C) and the `tests/common/` approach. The implementation matches that option precisely.

## Known Stubs

None. The fix is complete — `EnvVarGuard::set_all` is wired to real behavior via `std::env::set_var` and Drop restore.

## Self-Check: PASSED

| Item | Result |
|------|--------|
| `crates/nono-cli/tests/common/mod.rs` exists | FOUND |
| `crates/nono-cli/tests/common/test_env.rs` exists | FOUND |
| `crates/nono-cli/tests/env_vars.rs` modified | FOUND |
| `.planning/phases/41-.../41-05-SUMMARY.md` exists | FOUND |
| Commit `29c889b1` in git log | FOUND |
| `EnvVarGuard::set_all` at line 1037 in env_vars.rs | FOUND |
