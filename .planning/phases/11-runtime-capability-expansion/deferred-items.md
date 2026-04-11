# Phase 11 Deferred Items

Pre-existing test failures on Windows host (observed during plan 11-02 execution, verified present on base commit `8b82609` before any 11-02 changes). Out of scope for plan 11-02; logged here for future triage.

## Pre-existing Windows host test failures (not caused by 11-02)

All four of these tests fail on the `windows-squash` branch *before* any plan 11-02 code changes were applied — confirmed by running `git stash` + the failing test on the base commit.

1. `query_ext::tests::test_query_path_sensitive_policy_includes_policy_source`
   - **Error:** `assertion \`left == right\` failed, left: "path_not_granted", right: "sensitive_path"`
   - **Location:** `crates/nono-cli/src/query_ext.rs:468`
   - **Likely cause:** Sensitive-path policy resolution interacts with Windows path canonicalization differently than the test expected.

2. `capability_ext::tests::test_from_profile_allow_file_rejects_directory_when_exact_dir_unsupported`
   - **Likely cause:** Windows filesystem semantics for directory vs file resolution.

3. `capability_ext::tests::test_from_profile_filesystem_read_accepts_file_paths`
   - **Likely cause:** Same area as #2.

4. `profile::builtin::tests::test_all_profiles_signal_mode_resolves`
   - **Error:** `Environment variable 'XDG_CONFIG_HOME' validation failed: must be an absolute path, got: /home/nono-test/.config`
   - **Location:** `crates/nono-cli/src/profile/builtin.rs:373`
   - **Likely cause:** Test uses a POSIX-style absolute path that Windows does not recognize as absolute. Needs `cfg(unix)` gate or a Windows-appropriate fixture path.

## Action

These are Windows host test-harness bugs that predate phase 11 and are orthogonal to runtime capability expansion. Left as-is per GSD scope rules.
