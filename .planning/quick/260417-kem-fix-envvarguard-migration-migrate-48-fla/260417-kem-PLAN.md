---
phase: quick
plan: 260417-kem
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/config/mod.rs
  - crates/nono-cli/src/sandbox_state.rs
autonomous: true
must_haves:
  truths:
    - "All 48 clippy::disallowed_methods errors are resolved"
    - "Every env-mutating test uses EnvVarGuard for save/restore"
    - "Existing env mutex locks are preserved for mutual exclusion"
    - "All test assertions and logic remain unchanged"
  artifacts:
    - path: "crates/nono-cli/src/profile/mod.rs"
      provides: "8 tests migrated to EnvVarGuard (30 errors fixed)"
    - path: "crates/nono-cli/src/config/mod.rs"
      provides: "2 tests migrated to EnvVarGuard (12 errors fixed)"
    - path: "crates/nono-cli/src/sandbox_state.rs"
      provides: "1 test migrated to EnvVarGuard (6 errors fixed)"
  key_links:
    - from: "all three files"
      to: "crate::test_env::EnvVarGuard"
      via: "use import + set_all/remove calls"
      pattern: "EnvVarGuard::set_all"
---

<objective>
Migrate 48 `env::set_var` / `env::remove_var` calls in test code to use `crate::test_env::EnvVarGuard`, fixing clippy::disallowed_methods errors introduced by a partial revert.

Purpose: Restore CI compliance by eliminating direct env var mutation in tests.
Output: Three files modified, zero clippy errors from disallowed env methods.
</objective>

<execution_context>
@.planning/quick/260417-kem-fix-envvarguard-migration-migrate-48-fla/260417-kem-PLAN.md
</execution_context>

<context>
@crates/nono-cli/src/test_env.rs
@crates/nono-cli/src/profile/mod.rs
@crates/nono-cli/src/config/mod.rs
@crates/nono-cli/src/sandbox_state.rs

<interfaces>
From crates/nono-cli/src/test_env.rs:
```rust
pub static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub fn lock_env() -> std::sync::MutexGuard<'static, ()>;

pub struct EnvVarGuard { /* saves originals, restores on drop */ }
impl EnvVarGuard {
    #[must_use]
    pub fn set_all(vars: &[(&'static str, &str)]) -> Self;
    pub fn remove(&self, key: &str); // key must have been in set_all
}
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Migrate profile/mod.rs tests (30 errors, 8 tests)</name>
  <files>crates/nono-cli/src/profile/mod.rs</files>
  <action>
In the `#[cfg(test)] mod tests` block (lines ~1722-2009):

1. **Add import**: Add `use crate::test_env::EnvVarGuard;` alongside the existing `use std::env;` import at line 1726. Keep `use std::env;` since it is used for reading env vars in assertions. Remove `use std::sync::{Mutex, MutexGuard, OnceLock};` since it is only used by `env_lock()`.

2. **Remove `env_lock()` function** (lines 1770-1776) entirely. The tests will keep calling their existing `env_lock()` but we need to redirect. Actually -- the 4 non-Windows tests use the local `env_lock()` while the 2 Windows tests use `crate::config::test_env_lock()`. Replace the local `env_lock()` body to delegate to `crate::test_env::lock_env()` instead of maintaining its own OnceLock+Mutex. Simpler: just replace `env_lock()` with a one-liner: `fn env_lock() -> std::sync::MutexGuard<'static, ()> { crate::test_env::lock_env() }`. Keep the imports needed for `MutexGuard` (add it back as `use std::sync::MutexGuard;`).

3. **Migrate each test** using this pattern -- KEEP the `let _guard = env_lock();` (or `crate::config::test_env_lock()...`) line, then replace manual set_var/save/restore:

**test_expand_vars** (lines 1790-1809):
- Remove `let original_home = env::var("HOME").ok();`
- Replace `env::set_var("HOME", test_home());` with `let _env = EnvVarGuard::set_all(&[("HOME", test_home())]);`
- Remove the restore block (lines 1806-1808)

**test_expand_vars_xdg_state_home** (lines 1811-1849):
- Remove `let original_home` and `let original_state` saves
- Replace the two `env::set_var` calls with `let _env = EnvVarGuard::set_all(&[("HOME", test_home()), ("XDG_STATE_HOME", test_xdg_state_home())]);`
- Replace `env::remove_var("XDG_STATE_HOME");` with `_env.remove("XDG_STATE_HOME");`
- Remove the restore block (lines 1842-1848)

**test_expand_vars_xdg_cache_home** (lines 1851-1879):
- Remove `let original_home` and `let original_cache` saves
- Replace `env::set_var` calls with `let _env = EnvVarGuard::set_all(&[("HOME", test_home()), ("XDG_CACHE_HOME", test_xdg_cache_home())]);`
- Replace `env::remove_var("XDG_CACHE_HOME");` with `_env.remove("XDG_CACHE_HOME");`
- Remove the restore block (lines 1872-1878)

**test_expand_vars_xdg_runtime_dir** (lines 1881-1909):
- Remove `let original_runtime` save
- Replace `env::set_var("XDG_RUNTIME_DIR", test_xdg_runtime_dir());` with `let _env = EnvVarGuard::set_all(&[("XDG_RUNTIME_DIR", test_xdg_runtime_dir())]);`
- Replace `env::remove_var("XDG_RUNTIME_DIR");` with `_env.remove("XDG_RUNTIME_DIR");`
- Remove the restore block (lines 1906-1908)

**test_resolve_user_config_dir_uses_valid_absolute_xdg** (lines 1911-1923):
- Replace `env::set_var(...)` with `let _env = EnvVarGuard::set_all(&[("XDG_CONFIG_HOME", tmp.path().to_str().expect("utf8 path"))]);`
- Remove `env::remove_var("XDG_CONFIG_HOME");`
- Note: the guard restores on drop so explicit remove_var for cleanup is unnecessary

**test_resolve_user_config_dir_falls_back_on_relative_xdg** (lines 1925-1936):
- Replace `env::set_var(...)` with `let _env = EnvVarGuard::set_all(&[("XDG_CONFIG_HOME", "relative/path")]);`
- Remove `env::remove_var("XDG_CONFIG_HOME");`

**test_resolve_user_config_dir_uses_appdata** (Windows, lines 1938-1967):
- Keep `let _guard = crate::config::test_env_lock()...` line
- Remove all `original_*` saves
- The test needs APPDATA set and XDG_CONFIG_HOME unset. Use: `let _env = EnvVarGuard::set_all(&[("APPDATA", tmp.path().to_str().expect("utf8 path")), ("XDG_CONFIG_HOME", "placeholder")]);` then `_env.remove("XDG_CONFIG_HOME");`
- Remove the restore block (lines 1957-1966)

**test_expand_vars_uses_windows_home_and_appdata** (Windows, lines 1969-2009):
- Keep `let _guard = crate::config::test_env_lock()...` line
- Remove all `original_*` saves
- The test needs HOME unset, USERPROFILE and APPDATA set. Use: `let _env = EnvVarGuard::set_all(&[("HOME", "placeholder"), ("USERPROFILE", r"C:\Users\tester"), ("APPDATA", r"C:\Users\tester\AppData\Roaming")]);` then `_env.remove("HOME");`
- Remove the restore block (lines 1994-2008)

4. **Do NOT change**: Any test assertions, test logic, comments explaining XDG behavior, cfg attributes, or the `test_home()` / `test_xdg_*()` helper functions.
  </action>
  <verify>
    <automated>cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used 2>&1 | grep -c "disallowed_methods\|set_var\|remove_var" || echo "0 errors"</automated>
  </verify>
  <done>All 8 tests in profile/mod.rs use EnvVarGuard; zero disallowed_methods errors from this file; `cargo test -p nono-cli -- profile::tests` passes.</done>
</task>

<task type="auto">
  <name>Task 2: Migrate config/mod.rs tests (12 errors, 2 tests)</name>
  <files>crates/nono-cli/src/config/mod.rs</files>
  <action>
In the `#[cfg(test)] mod tests` block (lines ~192-336):

1. **Add import**: Add `use crate::test_env::EnvVarGuard;` in the test module's imports (near line 193).

2. **Migrate each test**:

**test_validated_home_falls_back_to_userprofile** (Windows, lines 288-311):
- Keep `let _guard = test_env_lock().lock().expect("env lock");`
- Remove `let original_home` and `let original_userprofile` saves
- The test needs HOME unset and USERPROFILE set. Use: `let _env = EnvVarGuard::set_all(&[("HOME", "placeholder"), ("USERPROFILE", r"C:\Users\tester")]);` then `_env.remove("HOME");`
- Remove the restore block (lines 301-310)

**test_validated_home_ignores_non_absolute_home_when_userprofile_exists** (Windows, lines 313-336):
- Keep `let _guard = test_env_lock().lock().expect("env lock");`
- Remove `let original_home` and `let original_userprofile` saves
- Both vars are set (not removed). Use: `let _env = EnvVarGuard::set_all(&[("HOME", "/home/user"), ("USERPROFILE", r"C:\Users\tester")]);`
- Remove the restore block (lines 326-335)

3. **Do NOT change**: The `test_env_lock()` function definition (lines 112-118), any non-test code, or any assertion logic.
  </action>
  <verify>
    <automated>cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used 2>&1 | grep -c "config/mod.rs.*disallowed" || echo "0 errors"</automated>
  </verify>
  <done>Both Windows tests in config/mod.rs use EnvVarGuard; zero disallowed_methods errors from this file.</done>
</task>

<task type="auto">
  <name>Task 3: Migrate sandbox_state.rs test (6 errors, 1 test)</name>
  <files>crates/nono-cli/src/sandbox_state.rs</files>
  <action>
In the `#[cfg(test)] mod tests` block:

1. **Add import**: Add `use crate::test_env::EnvVarGuard;` alongside the existing `use crate::test_env::lock_env;` (line 354).

2. **Migrate test_validate_cap_file_path_accepts_windows_runtime_temp_dir** (Windows, lines 391-419):
- Keep `let _guard = lock_env();`
- Remove `let old_tmp = std::env::var_os("TMP");` and `let old_temp = std::env::var_os("TEMP");`
- Replace the two `std::env::set_var` calls with: `let _env = EnvVarGuard::set_all(&[("TMP", dir.path().to_str().expect("utf8 path")), ("TEMP", dir.path().to_str().expect("utf8 path"))]);`
- Remove the restore block (lines 411-418, the two `match old_tmp/old_temp` blocks)

3. **Do NOT change**: Any assertion logic, the tempdir setup, or the `validate_cap_file_path` call.
  </action>
  <verify>
    <automated>cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used 2>&1 | grep -c "sandbox_state.*disallowed" || echo "0 errors"</automated>
  </verify>
  <done>The Windows test in sandbox_state.rs uses EnvVarGuard; zero disallowed_methods errors from this file.</done>
</task>

</tasks>

<verification>
```bash
# Full clippy check - must pass with zero warnings
cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used

# Run all affected tests (on Windows)
cargo test -p nono-cli -- profile::tests config::tests sandbox_state::tests

# Verify no remaining raw env::set_var/remove_var in test code (excluding test_env.rs itself)
grep -rn "env::set_var\|env::remove_var" crates/nono-cli/src/profile/mod.rs crates/nono-cli/src/config/mod.rs crates/nono-cli/src/sandbox_state.rs
# Expected: zero matches
```
</verification>

<success_criteria>
- `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` passes with zero errors
- `cargo test -p nono-cli` passes (all tests green)
- No raw `env::set_var` or `env::remove_var` calls remain in the three migrated files
- All env mutex locks preserved for mutual exclusion
</success_criteria>

<output>
Commit message: fix(cli): migrate 48 env var mutations in tests to EnvVarGuard
</output>
