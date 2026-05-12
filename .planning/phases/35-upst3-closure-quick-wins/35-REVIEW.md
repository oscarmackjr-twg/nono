---
phase: 35-upst3-closure-quick-wins
reviewed: 2026-05-12T15:00:00Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - crates/nono-cli/src/exec_strategy/env_sanitization.rs
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/exec_strategy_windows/mod.rs
  - crates/nono-cli/src/exec_strategy_windows/network.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/profile_cmd.rs
  - crates/nono-cli/src/profile_runtime.rs
  - crates/nono-cli/src/query_ext.rs
findings:
  critical: 0
  warning: 4
  info: 3
  total: 7
status: issues_found
---

# Phase 35: Code Review Report

**Reviewed:** 2026-05-12T15:00:00Z
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Summary

Phase 35 delivers three quick-win closures of UPST3 deferred items:

- **35-01** (REQ-PORT-CLOSURE-01): wires `allowed_env_vars` / `denied_env_vars`
  into the Windows execution path's `build_child_env` in
  `exec_strategy_windows/launch.rs`. Precedence (deny-before-allow,
  empty-allow strip-all, nono-injected-credentials bypass) is preserved and
  documented; the Windows-gated `test_should_skip_env_var_matches_windows_keys_case_insensitively`
  test locks the case-insensitive shape for the blocked-extras list.

- **35-02** (REQ-PORT-CLOSURE-06): pre-creates `~/.config/nono/profiles/`
  before `Sandbox::apply` locks the Landlock ruleset. Linux-only; macOS and
  Windows are compile-time no-ops. Idempotency is locked by a regression test
  with proper `EnvGuard` save/restore discipline.

- **35-03** (REQ-PORT-CLOSURE-07): replaces `format!("{:?}")` JSON emission
  in `profile_cmd::profile_to_json` and `diff_to_json` with
  `serde_json::to_value` so enum fields use the serde-driven snake_case
  representation; also adds `strip_verbatim_prefix` in `query_ext` to remove
  Windows NT verbatim prefixes (`\\?\`, `\\?\UNC\`, `\??\`) before deriving
  `suggested_flag` strings and before the sensitive-path check.

The diff is mechanically careful and matches the upstream commits it claims
to replay. Findings below cluster around two themes: (1) the new
`strip_verbatim_prefix` uses unanchored `String::replace` rather than
`strip_prefix`, which over-strips embedded substrings; (2) the
`is_dangerous_env_var` blocklist remains case-sensitive even when consumed
on Windows, so a parent that sets `ld_preload` (lowercase) bypasses the
linker-injection block. Neither is a new regression — both are pre-existing
weaknesses that Plan 35-01's wider Windows exposure now surfaces — but they
should be addressed before relying on the Windows env-filter as a security
boundary.

## Warnings

### WR-01: `strip_verbatim_prefix` uses unanchored `String::replace`, not `strip_prefix`

**File:** `crates/nono-cli/src/query_ext.rs:320-327`
**Issue:** `strip_verbatim_prefix` calls `raw.replace("\\\\?\\UNC\\", r"\\")`,
`raw.replace("\\\\?\\", "")`, and `raw.replace("\\??\\", "")` against the
full string. `String::replace` is unanchored — it strips every occurrence,
not just a leading prefix. Real Windows verbatim/device prefixes only appear
at the start of a canonicalized path; an embedded occurrence inside a
component (e.g., a filename literally containing `\\?\` after
`to_string_lossy()` lossy-conversion of unusual paths, or a path containing
backslashes from a Unicode replacement) would be silently mutated, distorting
the sensitive-path comparison and the `suggested_flag` UX.

The helper's own doc comment promises "only the well-known prefixes are
stripped." The implementation does not match that contract.

**Fix:** anchor to leading prefix only, matching `protected_paths::normalize_for_compare`'s pattern:
```rust
#[cfg(target_os = "windows")]
fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let raw = path.as_os_str().to_string_lossy();
    // Order matters: UNC prefix must be checked before the plain `\\?\` form.
    if let Some(rest) = raw.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = raw.strip_prefix(r"\\?\") {
        return PathBuf::from(rest);
    }
    if let Some(rest) = raw.strip_prefix(r"\??\") {
        return PathBuf::from(rest);
    }
    PathBuf::from(raw.into_owned())
}
```

### WR-02: `is_dangerous_env_var` blocklist is case-sensitive on Windows; bypassable via lowercase env names

**File:** `crates/nono-cli/src/exec_strategy/env_sanitization.rs:14-53`
**Issue:** Windows environment variable names are case-insensitive at OS
lookup time but case-preserving at iteration time. A parent process can set
`ld_preload` (or `Ld_Preload`, etc.) and `std::env::vars()` will return that
exact casing. `is_dangerous_env_var` does `key.starts_with("LD_")` /
`key == "BASH_ENV"` / `key.starts_with("OP_SESSION_")` — all case-sensitive.
On Windows, a malicious or compromised parent can therefore bypass the
linker-injection, shell-injection, interpreter-injection, **and 1Password
session-token** blocks by lowercasing the variable name.

Note that the sibling function `env_key_matches` (used by `should_skip_env_var`
for `config_env_vars` and `blocked_extra`) is already case-aware
(`eq_ignore_ascii_case` on Windows) — the inconsistency is precisely in the
hardcoded blocklist.

The Windows-gated test `test_should_skip_env_var_matches_windows_keys_case_insensitively`
exercises only the `blocked_extra` path, not `is_dangerous_env_var`, so the
gap is uncovered.

Severity is "warning" rather than "critical" because (a) the immediate
adversary model is "compromised parent shell," which is mostly outside
nono's stated threat model, and (b) on Windows the launcher additionally
sets `NoDefaultCurrentDirectoryInExePath=1` and a sanitized `PATH` that
limit some of the downstream injection surface. But the OP_* leak is a
direct credential exfiltration risk on Windows hosts where the parent is
already lightly hostile (Plan 35-01's exact threat surface).

**Fix:** make the comparison platform-aware:
```rust
pub(crate) fn is_dangerous_env_var(key: &str) -> bool {
    let starts_with_ci = |needle: &str| {
        if cfg!(target_os = "windows") {
            key.len() >= needle.len()
                && key[..needle.len()].eq_ignore_ascii_case(needle)
        } else {
            key.starts_with(needle)
        }
    };
    let eq_ci = |needle: &str| {
        if cfg!(target_os = "windows") {
            key.eq_ignore_ascii_case(needle)
        } else {
            key == needle
        }
    };
    starts_with_ci("LD_")
        || starts_with_ci("DYLD_")
        || eq_ci("BASH_ENV")
        // ... etc
}
```
Add a Windows-gated test asserting `is_dangerous_env_var("ld_preload")`,
`is_dangerous_env_var("op_session_personal")`, etc. all return true.

### WR-03: Duplicate copy of `validate_env_var_patterns` diverges silently

**File:** `crates/nono-cli/src/profile_runtime.rs:269-287`
**Issue:** `validate_env_var_patterns_local` is a byte-for-byte duplicate of
`exec_strategy::env_sanitization::validate_env_var_patterns` introduced
specifically to avoid crossing the D-34-E1 module boundary. The function's
own doc comment says "Kept in lock-step with the canonical helper via tests
in `exec_strategy/env_sanitization.rs`" — but those tests exercise the
canonical helper, not the local copy. If the canonical helper grows a new
validation rule (e.g., to forbid `=` in patterns), the local copy will
silently drift and `prepare_profile` will accept patterns the canonical
helper would reject. The Plan 35-01 wiring then surfaces those patterns at
`build_child_env` time on Windows, where the canonical helper IS the one
that matters.

**Fix:** either (a) lift `validate_env_var_patterns` to a parent module both
`exec_strategy` and `exec_strategy_windows` can import from, or (b) at
minimum add a property test asserting that for all `(patterns, field_name)`
the two functions return identical `Option<String>` results. Option (a) is
the structurally-correct fix; the D-34-E1 boundary is procedural, not a
hard invariant.

### WR-04: `expand_vars` failure silently drops `override_deny` entries

**File:** `crates/nono-cli/src/profile_runtime.rs:97-110`
**Issue:** In `collect_override_deny_paths`, profile-supplied `override_deny`
templates pass through `profile::expand_vars(template, workdir).ok().map(...)`.
A failed expansion (e.g., a `${EVIL_VAR}` reference to a variable that is not
set) yields `None`, which `filter_map` silently drops from the
override-deny list. The user's intent ("exempt this sensitive path from
deny") is silently dropped, with no warning emitted. The fail-secure default
applies (the path stays in the deny list), but the operator never learns
their override didn't take effect. This is the inverse of the CLAUDE.md
"explicit over implicit" rule: a security-relevant decision is being made
silently.

Note this is pre-existing code style (CLI-overlay path on line 79 has the
same shape via `unwrap_or_else(|_| path.to_path_buf())`), but it sits inside
files in the Plan 35 review scope.

**Fix:** match the CLI-override branch's behavior (fall back to the raw
template path) and emit a `tracing::warn!` so operators see expansion
failures:
```rust
profile::expand_vars(template, workdir)
    .inspect_err(|e| tracing::warn!(
        "Failed to expand override_deny template {template:?}: {e}; using raw"
    ))
    .unwrap_or_else(|_| PathBuf::from(template))
```

## Info

### IN-01: `is_env_var_allowed` / `is_env_var_denied` pattern matching is case-sensitive on Windows

**File:** `crates/nono-cli/src/exec_strategy/env_sanitization.rs:88-102`
**Issue:** `matches_env_var_patterns` does `key.starts_with(prefix)` and
`key == *pattern` with no case folding. On Windows, an operator who writes
`deny_vars: ["GH_TOKEN"]` in their profile gets a deny that misses a parent
environment containing `Gh_Token` or `gh_token`. The function is correctly
case-sensitive on Unix (canonical) but inconsistent with `env_key_matches`
elsewhere in the file.

This is consistent with the upstream port, so changing the semantics needs
upstream coordination. Worth noting in the Windows-parity documentation so
operators don't assume Windows-natural case-insensitive matching.

**Fix:** Either (a) document the case-sensitivity explicitly in the
profile-schema docs for `allow_vars` / `deny_vars`, or (b) introduce a
Windows-specific case-insensitive comparison and add a regression test.

### IN-02: `unsafe` impl `Drop` ordering comment for `_applied_labels` is correct but fragile

**File:** `crates/nono-cli/src/exec_strategy_windows/mod.rs:251-259`
**Issue:** `PreparedWindowsLaunch` relies on Rust's documented
"reverse-of-declaration drop order" so `_applied_labels` reverts mandatory-
label ACEs before `_network_enforcement` tears down WFP/firewall rules. The
field is named with a leading underscore (suppressing warnings) and the
ordering is comment-documented but not test-locked. A future refactor that
moves the fields alphabetically or runs `rustfmt`-like sorting tooling on
struct fields would silently reverse the cleanup order.

**Fix:** add a regression test (Phase 21 territory, not strictly Plan 35)
that mocks the two guards' `Drop` impls and asserts the call order; or wrap
the pair in a `LabelsThenNetwork` struct whose own `Drop` calls them in the
desired order, making the contract explicit. Not blocking.

### IN-03: `runtime_dirs` directory pre-creation ignores errors silently

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:880-887`
**Issue:** `let _ = std::fs::create_dir_all(dir);` swallows every error
during runtime-root pre-creation. If creation fails for a security-relevant
reason (ACL conflict, disk full, parent path missing), the subsequent
`CreateProcessW` will spawn the child with env vars pointing at directories
that don't exist. Some tools (npm, pip) handle missing config dirs
gracefully; others (cargo, gradle) treat missing parent dirs as fatal at
first write. The user sees a confusing child-side error rather than a clear
"failed to prepare runtime root" diagnostic.

This is consistent with the existing low-IL fallback path's best-effort
semantics, and the env-filter wiring in Plan 35-01 does not change it. Worth
noting for future hardening.

**Fix:** capture per-dir errors into a `tracing::warn!` and, if more than a
small fraction fail, return `Err(NonoError::SandboxInit(...))` rather than
proceeding with a known-broken runtime root.

---

_Reviewed: 2026-05-12T15:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
