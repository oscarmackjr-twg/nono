---
phase: quick
plan: 260417-kem
subsystem: nono-cli/tests
tags: [clippy, env-safety, test-infrastructure]
key-files:
  modified:
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/config/mod.rs
    - crates/nono-cli/src/sandbox_state.rs
decisions: []
metrics:
  duration: 399s
  completed: "2026-04-17T18:55:00Z"
  tasks: 3
  files: 3
---

# Quick Task 260417-kem: Fix EnvVarGuard Migration Summary

Migrated 48 raw `env::set_var`/`env::remove_var` calls across 11 tests to `EnvVarGuard::set_all`/`remove`, restoring CI clippy compliance.

## What Changed

### Task 1: profile/mod.rs (8 tests, 30 errors fixed)
- Replaced all manual save/restore blocks with `EnvVarGuard::set_all`
- Simplified local `env_lock()` to delegate to `crate::test_env::lock_env()`
- Removed unused `std::env`, `OnceLock`, `Mutex` imports; kept `MutexGuard` for `env_lock()` return type
- Tests migrated: `test_expand_vars`, `test_expand_vars_xdg_state_home`, `test_expand_vars_xdg_cache_home`, `test_expand_vars_xdg_runtime_dir`, `test_resolve_user_config_dir_uses_valid_absolute_xdg`, `test_resolve_user_config_dir_falls_back_on_relative_xdg`, `test_resolve_user_config_dir_uses_appdata`, `test_expand_vars_uses_windows_home_and_appdata`

### Task 2: config/mod.rs (2 tests, 12 errors fixed)
- Migrated `test_validated_home_falls_back_to_userprofile` and `test_validated_home_ignores_non_absolute_home_when_userprofile_exists`
- Used `set_all` with placeholder + `remove` pattern for HOME-unset tests
- Preserved existing `test_env_lock()` mutex calls

### Task 3: sandbox_state.rs (1 test, 6 errors fixed)
- Migrated `test_validate_cap_file_path_accepts_windows_runtime_temp_dir`
- Replaced `var_os` save + `match` restore with `EnvVarGuard::set_all` for TMP/TEMP

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed unused `std::env` import in profile/mod.rs**
- **Found during:** Task 1
- **Issue:** After removing all `env::set_var`/`env::remove_var` calls, the `use std::env;` import became unused, producing a compiler warning
- **Fix:** Removed the unused import
- **Files modified:** crates/nono-cli/src/profile/mod.rs
- **Commit:** 68d9374

## Verification

- `cargo clippy --all-targets --all-features -- -D warnings -D clippy::unwrap_used`: 0 disallowed_methods errors
- `cargo test -p nono-cli -- profile::tests config::tests sandbox_state::tests`: all tests pass
- `grep env::set_var/env::remove_var` across all three files: zero code matches (1 comment only)

## Commits

| Task | Commit  | Description                                      |
|------|---------|--------------------------------------------------|
| 1    | 68d9374 | Migrate 8 profile/mod.rs tests to EnvVarGuard    |
| 2    | 41aad1d | Migrate 2 config/mod.rs tests to EnvVarGuard     |
| 3    | 6749494 | Migrate 1 sandbox_state.rs test to EnvVarGuard   |

## Self-Check: PASSED

All three modified files exist and all three commits verified in git log.
