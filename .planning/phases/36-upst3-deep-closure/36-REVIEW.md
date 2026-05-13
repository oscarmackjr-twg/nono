---
phase: 36-upst3-deep-closure
reviewed: 2026-05-13T03:26:39Z
depth: standard
files_reviewed: 42
files_reviewed_list:
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/data/nono-profile.schema.json
  - crates/nono-cli/data/policy.json
  - crates/nono-cli/data/profile-authoring-guide.md
  - crates/nono-cli/src/capability_ext.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/command_runtime.rs
  - crates/nono-cli/src/deprecated_schema.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/launch_runtime.rs
  - crates/nono-cli/src/learn.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/policy.rs
  - crates/nono-cli/src/profile/builtin.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/profile_cmd.rs
  - crates/nono-cli/src/profile_runtime.rs
  - crates/nono-cli/src/profile_save_runtime.rs
  - crates/nono-cli/src/pty_proxy.rs
  - crates/nono-cli/src/query_ext.rs
  - crates/nono-cli/src/sandbox_log.rs
  - crates/nono-cli/src/sandbox_prepare.rs
  - crates/nono-cli/src/sandbox_state.rs
  - crates/nono-cli/src/startup_prompt.rs
  - crates/nono-cli/src/why_runtime.rs
  - crates/nono-cli/src/wiring.rs
  - crates/nono-cli/tests/builtin_profile_load.rs
  - crates/nono-cli/tests/profile_validate_strict.rs
  - crates/nono-cli/tests/yaml_merge_reversal.rs
  - crates/nono/src/diagnostic.rs
  - docs/cli/development/windows-filesystem-parity-contract.mdx
  - docs/cli/development/windows-preview-pilot.mdx
  - docs/cli/development/windows-preview-validation.mdx
  - docs/cli/development/windows-security-model.mdx
  - docs/cli/features/profile-authoring.mdx
  - docs/cli/features/profiles-groups.mdx
  - docs/cli/internals/wsl2-feature-matrix.mdx
  - docs/cli/usage/examples.mdx
  - docs/cli/usage/flags.mdx
  - docs/cli/usage/troubleshooting.mdx
  - scripts/lint-docs.sh
  - scripts/test-list-aliases.sh
findings:
  blocker: 4
  warning: 7
  info: 4
  total: 15
status: fixes_applied
fixes_applied_at: 2026-05-13T07:00:00Z
fixes_scope: critical_warning
fixes_summary:
  blocker_fixed: 4
  warning_fixed: 7
  info_fixed: 0
  info_deferred: 4
---

# Phase 36: Code Review Report

**Reviewed:** 2026-05-13T03:26:39Z
**Depth:** standard
**Files Reviewed:** 42
**Status:** fixes_applied (all 4 BLOCKERs + 7 WARNINGs landed; 4 INFOs deferred per fix-scope policy)

## Summary

Phase 36 ports `deprecated_schema`, canonical profile sections (`CommandsConfig`, `FilesystemConfig.{deny,bypass_protection}`), the `override_deny` → `bypass_protection` atomic rename across 17 files / 183 callsites, the `yaml_merge` directive, and ExecConfig surgical changes.

The atomic-rename mechanic is solid and the new canonical schema fields deserialise correctly with serde aliases preserving legacy JSON compat. The `wiring::validate_target_path` primitive correctly uses `Path::components()` iteration per CLAUDE.md § Path Handling, and the yaml_merge surface is well-tested for path-traversal rejection.

However, several adversarial findings block ship-readiness:

1. **`Profile::commands` is dead-letter** — the new canonical `commands.{allow,deny}` section deserialises into `Profile` but is **never wired** into `CapabilitySet` construction (`from_profile()` ignores it). A profile author who writes `"commands": {"deny": ["rm"]}` will have the rule silently dropped at runtime. This is a security-correctness BLOCKER.
2. **`cmd_validate` strict-mode error is incoherent** — the user-facing message refers to "legacy bypass_protection" instead of "legacy override_deny", and the `DeprecationCounter::emit_once("bypass_protection", "bypass_protection")` call is a silent no-op (the counter map only registers `override_deny`).
3. **User-facing diagnostic still cites the legacy field name** — `crates/nono/src/diagnostic.rs:1417` and line 1425 reference `policy.override_deny` in user guidance after the canonical rename.
4. **`tracing::warn!` macro unimported in `sandbox_prepare.rs`** — line 350 uses bare `warn!` but only `tracing::info` is imported; this is inside `#[cfg(target_os = "linux")]` so Linux builds emit a compile error.

The yaml_merge port and rename mechanics are otherwise clean. The strict-mode test passes incidentally because `parse_profile_file` emits the correct `override_deny` warning before `cmd_validate`'s buggy emit-once runs — the assertion holds but for the wrong reason.

## Critical Issues

### CR-01: `Profile::commands` (canonical `CommandsConfig`) is dead-letter — security policy silently dropped

**File:** `crates/nono-cli/src/capability_ext.rs:555-595` (and the absence of any `profile.commands.*` reference in this file)
**Issue:** Plan 36-01b adds `Profile::commands: CommandsConfig` with `allow: Vec<String>` and `deny: Vec<String>` fields, plumbs them through `From<ProfileDeserialize>`, includes them in `merge_profiles` and exposes them in the schema. **But `CapabilitySet::from_profile()` never reads `profile.commands.{allow,deny}` and never calls `caps.add_allowed_command` / `caps.add_blocked_command` for those entries.** A profile author who writes:

```json
{
  "meta": {"name": "test"},
  "commands": {"deny": ["rm", "dd"]}
}
```

…will have those denies completely ignored at runtime. `from_profile()` processes `profile.policy.add_deny_commands` (line 555) and `profile.security.allowed_commands` (line 593) but the new canonical section is dead code. The schema, the docs, and the merge logic all advertise this surface; runtime enforcement silently no-ops.

This is a security-correctness BLOCKER: a profile that reads as "deny rm" actually allows `rm` and silently succeeds. CLAUDE.md § Fail Secure mandates that policy intent is honored.

**Fix:** In `capability_ext.rs::from_profile()`, after the existing `for cmd in &profile.security.allowed_commands { ... }` and `for cmd in &profile.policy.add_deny_commands { ... }` loops, add:

```rust
// Plan 36-01b: canonical commands section per upstream f0abd413.
for cmd in &profile.commands.allow {
    caps.add_allowed_command(cmd.clone());
}
for cmd in &profile.commands.deny {
    caps.add_blocked_command(cmd);
}
```

Add a regression test asserting that `profile.commands.deny` entries appear in `caps.blocked_commands()`.

---

### CR-02: `cmd_validate` strict-mode error message references wrong key names (and emit_once is a silent no-op)

**File:** `crates/nono-cli/src/profile_cmd.rs:2243-2260`
**Issue:** Three connected bugs in the strict-mode legacy-key handling:

1. **Line 2243** calls `counter.emit_once("bypass_protection", "bypass_protection")`. The `DeprecationCounter::keys` map (defined in `deprecated_schema.rs:160-162`) only contains the legacy key `"override_deny"`. Calling `emit_once` with `"bypass_protection"` finds no matching map entry → wait-free no-op. No warning is emitted via this code path. The intent was clearly `counter.emit_once("override_deny", "bypass_protection")`.

2. **Lines 2247-2251** (strict-mode error message) say:
   ```
   "legacy key `bypass_protection` rejected by --strict; use canonical `bypass_protection` instead"
   ```
   This message refers to "legacy bypass_protection" — but `bypass_protection` IS the canonical name. The legacy key is `override_deny`. The message is logically incoherent.

3. **Lines 2256-2258** (non-strict warning message): same bug — refers to "legacy key bypass_protection" instead of "legacy key override_deny".

The integration test `test_profile_validate_strict_rejects_legacy_override_deny` happens to pass because `parse_profile_file` (called via `load_profile_from_path` at line 2269) invokes `detect_legacy_override_deny_key` which DOES emit the correct `override_deny` WARN via the global counter. So stderr ends up containing both "override_deny" (from the parse-time detection) and "bypass_protection" (from the strict-mode error). The test asserts both substrings appear — and they do, just not from the code path the test is targeting.

**Fix:** In `profile_cmd.rs:2243-2260`:

```rust
counter.emit_once("override_deny", "bypass_protection");
if args.strict {
    errors.push(format!(
        "legacy key `override_deny` rejected by --strict; \
         use canonical `bypass_protection` instead (found {} \
         path(s))",
        canonical.bypass_protection.len()
    ));
} else {
    warnings.push(
        "legacy key `override_deny` found; migrate to canonical \
         `bypass_protection`"
            .to_string(),
    );
}
```

After fixing, add a unit test that calls `cmd_validate` with `args.strict = true` against an in-memory fixture and asserts the error string contains exactly `legacy key `override_deny`` (not `bypass_protection`). The current integration test should be supplemented with a check on the `errors` vec contents to prevent regressing the message.

---

### CR-03: User-facing diagnostic still cites the legacy `policy.override_deny` name after canonical rename

**File:** `crates/nono/src/diagnostic.rs:1417, 1425`
**Issue:** Plan 36-01c's atomic rename commit message claims all 17 files were updated, but two user-facing references to `policy.override_deny` remain in `crates/nono/src/diagnostic.rs`:

- Line 1417 (user-visible stderr message): `"[nono] {}{} permanently restricted — override via a user profile with policy.override_deny."`
- Line 1425 (doc comment that surfaces in API docs and IDE hover): `"profile with `policy.override_deny`."`

The commit summary explicitly lists 17 fork-side source files but `crates/nono/src/diagnostic.rs` is not among them. The library `nono` is also not in the per-file diff stats. This is a missed rename — users hitting a denied path will be told to use the deprecated field name, which works (via serde alias) but contradicts the docs and the new strict mode that will fail-close on profiles that follow this guidance.

**Fix:**
```rust
// Line 1417
lines.push(format!(
    "[nono] {}{} permanently restricted — override via a user profile with policy.bypass_protection.",
    count_prefix, verb,
));
// Line 1425
/// profile with `policy.bypass_protection`.
```

---

### CR-04: `tracing::warn!` macro used without import in `sandbox_prepare.rs` (Linux-only compile error)

**File:** `crates/nono-cli/src/sandbox_prepare.rs:350`
**Issue:** Inside the `#[cfg(target_os = "linux")]` block (line 332-357), line 350 uses the bare `warn!` macro:

```rust
if let Err(e) = result {
    if e.kind() != std::io::ErrorKind::AlreadyExists {
        warn!("Failed to pre-create {}: {}", path.display(), e);
    }
}
```

But the top of the file only imports `use tracing::info;` (line 17). The `warn!` macro is not in scope. Line 377 in the same file correctly uses `tracing::warn!(...)` (fully qualified). This is inconsistent and will fail to compile on Linux.

Worth noting per CLAUDE.md feedback `feedback_clippy_cross_target.md`: Windows-host clippy will NOT catch this because the entire block is gated by `#[cfg(target_os = "linux")]`. The atomic-rename commit (e168dd6b) was validated on Windows host only ("Cross-target Linux/macOS clippy skipped"). This regression would surface in Linux CI but is invisible from the Windows worktree.

**Fix:** Either change line 350 to `tracing::warn!(...)` for consistency with line 377, OR add `warn` to the import:
```rust
use tracing::{info, warn};
```

---

## Warnings

### WR-01: Schema permits both legacy `override_deny` AND canonical `bypass_protection` keys simultaneously — semantics undefined

**File:** `crates/nono-cli/data/nono-profile.schema.json:350-359`
**Issue:** The schema declares both `bypass_protection` (line 350-354) and `override_deny` (line 355-359) as separate JSON properties on `PolicyPatchConfig`. The schema does not mark them mutually exclusive (no `oneOf` / `not` constraint). The Rust struct uses `#[serde(default, alias = "override_deny")]` which means serde's behavior when BOTH keys appear in the same JSON is non-deterministic (typically "last wins" but may differ between serde_json versions).

A profile like:
```json
{"policy": {"override_deny": ["/a"], "bypass_protection": ["/b"]}}
```

will silently drop one of the two lists depending on JSON key ordering. This is a security-relevant fail-open condition: an author migrating from legacy to canonical may leave both in place during transition and lose one set of grants.

**Fix:** Either:
1. Add a schema-level `oneOf` constraint requiring exactly one of the two keys be present, OR
2. Add an explicit serde validator (custom `Deserialize` impl on `PolicyPatchConfig`) that errors when both keys are present, with the message: "use only `bypass_protection`; `override_deny` is the legacy alias and cannot coexist with the canonical key."

The second option is strongly preferred per CLAUDE.md § Fail Secure.

---

### WR-02: `cleanup_stale_state_files` cannot parse the file-naming scheme it's supposed to clean up

**File:** `crates/nono-cli/src/sandbox_state.rs:315-326`
**Issue:** `cleanup_stale_state_files` reads files matching `.nono-*.json` and tries to parse the `*` portion as `u32` to recover a PID:

```rust
let pid_str = file_name
    .trim_start_matches(".nono-")
    .trim_end_matches(".json");
let pid = match pid_str.parse::<u32>() {
    Ok(p) => p,
    Err(_) => {
        debug!("Skipping state file with invalid PID: {}", file_name);
        continue;
    }
};
```

But `execution_runtime.rs:81-91` (`next_capability_state_file_path`) names new state files with **random hex** (8 bytes → 16 hex chars), not a PID:

```rust
let bytes: [u8; 8] = rng.random();
let suffix = bytes.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
std::env::temp_dir().join(format!(".nono-{suffix}.json"))
```

For example, a file named `.nono-abcd1234567890ab.json` cannot parse as `u32` (16-digit hex would overflow even if it were decimal). The cleanup function silently skips ALL current-format state files, accumulating stale files in `/tmp` indefinitely.

This isn't a Phase 36 regression (the random-suffix scheme appears unrelated to the rename), but the file is in scope and the bug is clear-cut. Phase 36 documentation mentions `cleanup_stale_state_files` as part of the sandbox state lifecycle.

**Fix:** Either:
1. Restore PID-based naming so the cleanup logic works, OR
2. Rewrite cleanup to use mtime/atime (skip files older than e.g. 7 days regardless of name).

Option 2 is more robust since it doesn't depend on the file naming scheme.

---

### WR-03: `EnvGuard` in profile_runtime tests does not acquire `lock_env()` — parallel-test flakes

**File:** `crates/nono-cli/src/profile_runtime.rs:300-328` (test infra), `:342-363` (test that uses it)
**Issue:** `test_pre_create_landlock_profiles_dir_idempotent` (line 341) creates an `EnvGuard` for `XDG_CONFIG_HOME` but does NOT acquire `crate::test_env::lock_env()` first. CLAUDE.md § Environment variables in tests mandates:

> Rust runs unit tests in parallel within the same process, so an unrestored env var causes flaky failures in unrelated tests... Always use save/restore pattern and keep the modified window as short as possible.

While `EnvGuard::Drop` correctly restores the env var, the modification window is unprotected: another test reading `XDG_CONFIG_HOME` during this test's runtime will see the tempdir value. Other tests in this codebase (e.g. `profile_save_runtime.rs::tests`, `policy.rs::tests::test_expand_path`) consistently acquire the lock before env mutation.

**Fix:** Add `let _guard = crate::test_env::lock_env();` at the start of `test_pre_create_landlock_profiles_dir_idempotent` before the `EnvGuard::set` call. Apply the same pattern to any other test in this file that mutates env vars without locking.

---

### WR-04: `scripts/lint-docs.sh` emits invalid JSON output due to naive escaping

**File:** `scripts/lint-docs.sh:148-149`
**Issue:** The `--format json` output path hand-rolls JSON escaping:

```bash
local escaped_text="${DRIFT_TEXT[$i]//\"/\\\"}"
drift_arr+="{\"file\":\"${DRIFT_FILES[$i]}\",\"line\":${DRIFT_LINES[$i]},\"text\":\"${escaped_text}\"}"
```

This only escapes double-quote characters. It does NOT escape:
- Backslashes (`\` → `\\`)
- Control characters (`\b`, `\f`, `\n`, `\r`, `\t` → `\b`, `\f`, `\n`, `\r`, `\t`)
- Forward slashes (optional but recommended)
- Already-backslash-escaped quotes (`\"` becomes `\\"`, producing invalid JSON)

A doc line containing e.g. `<code class="foo\"bar">` will produce invalid JSON. CI consumers parsing the output will fail unpredictably.

**Fix:** Pipe through `jq -Rn '. | tostring'` or use Python's `json.dumps` for the JSON path:

```bash
emit_json() {
    python3 -c '
import json, sys
files = sys.argv[1::3]
lines = sys.argv[2::3]
texts = sys.argv[3::3]
drift = [{"file": f, "line": int(l), "text": t} for f, l, t in zip(files, lines, texts)]
print(json.dumps({"status": "drift" if drift else "clean", "drift_count": len(drift), "drift": drift}))
' "$@" "${DRIFT_FILES[@]}" "${DRIFT_LINES[@]}" "${DRIFT_TEXT[@]}"
}
```

Or use `jq` if available. The script already requires `set -euo pipefail` so a dependency on `jq` or `python3` is acceptable for a CI script.

`scripts/test-list-aliases.sh:150-170` has the same bug, though it's less likely to bite because the data fields are file paths and line numbers.

---

### WR-05: `LegacyPolicyPatch` rejects canonical-key documents — sound, but undocumented coupling with `cmd_validate`

**File:** `crates/nono-cli/src/deprecated_schema.rs:67-86`, used by `crates/nono-cli/src/profile_cmd.rs:2237-2240`
**Issue:** `LegacyPolicyPatch` is `#[serde(deny_unknown_fields)]` and only declares `override_deny`. This means a profile JSON containing the canonical `bypass_protection` (or any other policy field like `add_deny_access`) will FAIL to deserialise into `LegacyPolicyPatch`.

In `cmd_validate`, the call:
```rust
if let Ok(patch) = serde_json::from_value::<LegacyPolicyPatch>(policy_val.clone()) {
    if patch.has_legacy_keys() { ... }
}
```

silently swallows the error. This is intentional per the doc comment — but a profile with BOTH `bypass_protection` AND `override_deny` will also fail to deserialize (because `bypass_protection` is unknown in this struct), and `--strict` will silently NOT detect the legacy key.

Verified test gap: `test_profile_validate_strict_rejects_legacy_override_deny` only tests profiles with `override_deny` alone, not profiles with both keys.

**Fix:** Either:
1. Remove `deny_unknown_fields` from `LegacyPolicyPatch` (it's a detection-only struct; unknown fields don't affect detection), OR
2. Switch to direct JSON inspection: `policy_val.get("override_deny").is_some()`. This sidesteps serde's whole-struct-deny semantics. The `raw_profile_has_legacy_override_deny_key` helper already does this for the parse path; cmd_validate could use the same approach.

Option 2 is preferred — it eliminates the brittle coupling between `LegacyPolicyPatch`'s schema and the actual validation logic.

---

### WR-06: `wiring::apply_yaml_merge` does not check that `source` and `target` differ — can corrupt source

**File:** `crates/nono-cli/src/wiring.rs:237-291`
**Issue:** `apply_yaml_merge` validates that both paths are within `profile_dir` but doesn't check that `source != target`. If a user specifies the same file:

```yaml
yaml_merge:
  target: profile.yaml
  source: profile.yaml
```

The flow is:
1. Read `target_raw` from `profile.yaml`.
2. Read `source_raw` from `profile.yaml` (same file).
3. Parse and merge.
4. Atomically write merged YAML back to `profile.yaml`.

The merge function `merge_yaml_values` with overlapping base+overlay where both contain the same data: overlay wins, so the result is byte-equivalent for mappings (modulo serializer formatting). But scalars/sequences fully replace — and the YAML round-trip is not formatting-stable: ordering, comments, anchors, and quoting will be lost. The user thinks they're declaring a no-op but the file's formatting is silently mangled.

A more severe variant: a user's overlay accidentally has the same file as both source and target via symlinks or `../` shenanigans. The pre-canonicalize check at line 243-244 catches the path-equality case only after `canonicalize` — let me verify... Yes, since both are canonicalized through the same `validate_target_path`, they'd be equal at the canonical PathBuf level. But the function doesn't compare them.

**Fix:** After canonicalization, check:
```rust
if canonical_target == canonical_source {
    return Err(NonoError::ProfileParse(format!(
        "yaml_merge: source and target must differ, both resolve to '{}'",
        canonical_target.display()
    )));
}
```

---

### WR-07: `validate_target_path` re-implements `Path::starts_with` semantics — risk of drift

**File:** `crates/nono-cli/src/wiring.rs:82-120`
**Issue:** The function documents that it uses component iteration "for explicit clarity and to satisfy the grep acceptance criterion". The comment at line 100-102 acknowledges:

```
This is path-safe: Path::starts_with() on PathBuf uses component comparison, not
byte comparison, which is equivalent to the manual loop below.
```

So the manual loop is functionally equivalent to `canonical.starts_with(&canonical_profile_dir)`. The manual approach has two risks:

1. The hand-rolled comparison may drift from `Path::starts_with`'s semantics if Rust's `Path` adds new component types (already happened with `Prefix` for Windows UNC).
2. The justification ("satisfy the grep acceptance criterion") encodes a process check into the production code, which is a smell. The criterion's intent — "use component comparison, not byte comparison" — is satisfied equally well by `starts_with`.

This isn't strictly a bug but it's anti-pattern. The function passes its security tests (path traversal, symlink escape, UNC alias rejected). Just keep the implementation consistent with the rest of the codebase, which uses `starts_with` elsewhere (e.g. `policy.rs:1045`, `protected_paths`, `query_ext.rs`).

**Fix:** Replace the manual component loop (lines 103-117) with:
```rust
if !canonical.starts_with(&canonical_profile_dir) {
    return Err(NonoError::ProfileParse(format!(
        "yaml_merge target '{}' is outside the allowed directory '{}'",
        target.display(),
        profile_dir.display()
    )));
}
```

If the grep acceptance criterion is needed, add a comment that cites `Path::starts_with`'s docs explaining component-based comparison. Don't encode the grep-bait into structural code.

---

## Info

### IN-01: Duplicate `tracing::{debug, info, warn}` import in `learn.rs`

**File:** `crates/nono-cli/src/learn.rs:33-36`
**Issue:**
```rust
#[cfg(target_os = "linux")]
use tracing::{debug, info, warn};
#[cfg(target_os = "macos")]
use tracing::{debug, info, warn};
```

Both branches import the same items. Either consolidate or use `cfg(any(target_os = "linux", target_os = "macos"))`. Minor — both are mutually exclusive cfgs so only one ever compiles.

**Fix:** Consolidate:
```rust
#[cfg(any(target_os = "linux", target_os = "macos"))]
use tracing::{debug, info, warn};
```

---

### IN-02: `select_exec_strategy` accepts 5 args but uses none — misleading API

**File:** `crates/nono-cli/src/launch_runtime.rs:497-512`
**Issue:** The function takes 5 `bool` parameters and returns `ExecStrategy::Supervised` unconditionally. The body is:

```rust
let _ = (rollback, proxy_active, capability_elevation, trust_interception_active, detached_start);
exec_strategy::ExecStrategy::Supervised
```

This is documented behavior (all strategies converge on Supervised per main.rs:346-389 tests). But the function signature suggests conditional logic exists. Future maintainers may mistake this for live code and try to "fix" the unused args.

**Fix:** Add a one-line comment above the function body explaining the convergence:
```rust
// All exec strategies converge on Supervised (D-... in Phase ...).
// Args are accepted for API stability; the parameter pattern is preserved
// so future plans that re-introduce strategy variants don't need a
// callsite-wide refactor.
let _ = (...);
exec_strategy::ExecStrategy::Supervised
```

Or remove the unused parameters entirely if the call sites can adapt.

---

### IN-03: `serde_yaml_ng = "=0.10.0"` exact-version pin lacks renovate config

**File:** `crates/nono-cli/Cargo.toml:65-66`
**Issue:** The dependency is pinned to an exact version per upstream `242d4917` (the comment notes this). The pin is documented but no `renovate.json` / dependabot config in the repo references this lock, so automated updates may silently break the pin. If renovate or dependabot is configured at the org level, they may try to bump this dependency and break the wire compatibility documented in 36-02 PATTERNS.md.

**Fix:** Either:
1. Add a `[package.metadata.dependabot.ignore]` entry for `serde_yaml_ng`, OR
2. Add an inline `# DO NOT BUMP without re-running yaml_merge tests against upstream's pin` comment.

A simple `# version-locked to upstream 242d4917; see Plan 36-02 PATTERNS.md` would suffice as documentation.

---

### IN-04: `sandbox_log.rs::parse_event_message` uses `rsplit_once('(')` which mishandles process names containing `(`

**File:** `crates/nono-cli/src/sandbox_log.rs:380`
**Issue:** The function parses lines like `"processname(PID) deny(N) operation target"` by splitting on the last `(`. For most process names this is correct. For exotic process names that contain `(` (e.g. a binary literally named `proc(test)`), the split would produce the wrong PID.

This is an edge case — Unix process names rarely contain parens — but the function is security-relevant (sandbox log attribution). A malicious actor cannot generally control process names without already having shell access, but a defense-in-depth scan would catch this.

**Fix:** Use a regex match for `\(\d+\)` at the END of the process_and_pid segment, or parse the trailing `(PID` more carefully:
```rust
let (process_name, pid_str) = match process_and_pid.rsplit_once('(') {
    Some((name, pid)) if pid.chars().all(|c| c.is_ascii_digit()) => (name, pid),
    _ => return None,
};
```

The `pid.chars().all(is_ascii_digit)` guard ensures we got a real PID segment.

---

_Reviewed: 2026-05-13T03:26:39Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

---

## Fixes Applied

**Applied:** 2026-05-13
**Scope:** All BLOCKER (CR-*) + WARNING (WR-*) findings. INFO (IN-*) findings deferred per fix-scope policy.
**Build verification (release mode):**
- `cargo build --workspace --release` — clean
- `cargo test --workspace --release --lib` — 148 nono-proxy tests + nono lib tests pass; 0 failed
- `cargo test --workspace --release --bins` — 974 main bin tests + 13 shell-broker tests + 18 wfp-service tests pass; 0 failed
- `cargo test --workspace --release --tests` — integration tests all pass; 0 failed
- `cargo clippy --workspace --release --all-targets -- -D warnings -D clippy::unwrap_used` — clean (exit 0)

CR-04 Linux-only block (sandbox_prepare.rs:350) could not be cross-compile-verified on this Windows host (the `x86_64-linux-gnu-gcc` linker is absent; documented in MEMORY.md feedback_clippy_cross_target). Fix is correct by inspection — uses fully-qualified `tracing::warn!`, matching the existing pattern at line 377 in the same file. Linux CI will exercise the block.

### BLOCKER fixes

| ID | Commit | Files | Notes |
|----|--------|-------|-------|
| CR-01 | `b0e5e4c3` | `crates/nono-cli/src/capability_ext.rs` | Wired `profile.commands.{allow,deny}` (canonical CommandsConfig) into `CapabilitySet::from_profile`. Added regression test `test_from_profile_commands_allow_deny_canonical_section` asserting both `commands.allow` and `commands.deny` propagate to the resulting CapabilitySet. Closes fail-open security regression — profiles that said "deny rm" were silently no-ops. |
| CR-02 | `f454cce1` | `crates/nono-cli/src/profile_cmd.rs` | Fixed `cmd_validate` strict-mode legacy-key block: `counter.emit_once("override_deny", "bypass_protection")` (was `("bypass_protection", "bypass_protection")`, a silent no-op); error and warning messages now correctly reference `override_deny` as the legacy key and `bypass_protection` as the canonical replacement (were incoherent before). |
| CR-03 | `d132263c` | `crates/nono/src/diagnostic.rs` | Renamed lingering `policy.override_deny` → `policy.bypass_protection` at lines 1417 (user-facing stderr message on permanently-restricted paths) and 1425 (doc comment for `is_denial_policy_blocked`). Closes atomic-rename miss in commit e168dd6b — the library crate was not in the per-file diff stats. |
| CR-04 | `1640c1ae` | `crates/nono-cli/src/sandbox_prepare.rs` | Replaced bare `warn!` macro (not in scope; file only imports `tracing::info`) with fully-qualified `tracing::warn!` in the Linux-gated `#[cfg(target_os = "linux")]` pre-create block at line 350. Matches the convention at line 377 in the same file. Closes Linux compile error invisible from Windows-host clippy. |

### WARNING fixes

| ID | Commit | Files | Notes |
|----|--------|-------|-------|
| WR-01 | `a6644451` | `crates/nono-cli/src/profile/mod.rs`, `crates/nono-cli/data/nono-profile.schema.json` | Added raw-JSON pre-check `raw_profile_has_both_bypass_and_override_keys` that returns `NonoError::ProfileParse` when both `policy.bypass_protection` AND `policy.override_deny` appear simultaneously (serde's "last key wins" alias semantics is non-deterministic and silently fail-opens during migration). Added schema-level `not: { required: [...] }` constraint. Three regression tests cover both-set (rejects), canonical-only (accepts), legacy-only (accepts). |
| WR-02 | `a3e8ffe3` | `crates/nono-cli/src/sandbox_state.rs` | Switched `cleanup_stale_state_files` from PID-parsing (which never matched the random-hex naming scheme used by `execution_runtime::next_capability_state_file_path`) to mtime-based with a 7-day retention window. Removed now-unused `is_process_running` helper. |
| WR-03 | `a65cb9dc` | `crates/nono-cli/src/profile_runtime.rs` | Added `let _env_lock = crate::test_env::lock_env();` at the start of `test_pre_create_landlock_profiles_dir_idempotent` before any env-var mutation, matching the convention used in policy.rs / profile_save_runtime / session / trust_cmd tests per CLAUDE.md § Environment variables in tests. |
| WR-04 | `6c8ce9b5` | `scripts/lint-docs.sh` | Replaced hand-rolled JSON escape (only handled double-quotes) in `emit_json` with delegation to `python3 -c '...json.dumps...'` (or `jq` as a fallback). Files / lines / texts are streamed as null-separated arrays via stdin. Fails loudly when neither python3 nor jq is on PATH rather than silently emitting malformed JSON. `scripts/test-list-aliases.sh` left untouched — its JSON output has no user-controlled text field. |
| WR-05 | `68321b9b` | `crates/nono-cli/src/profile_cmd.rs` | Decoupled `cmd_validate`'s legacy-key detection from `LegacyPolicyPatch`'s `#[serde(deny_unknown_fields)]` schema. Direct JSON inspection (`policy_val.get("override_deny")`) extracts the legacy field robustly when sibling canonical fields coexist. Then synthesises a single-field `{"override_deny": [...]}` object and feeds it to `LegacyPolicyPatch::rewrite` so the canonical-form contract stays centralised in `deprecated_schema`. |
| WR-06 | `d69fac8d` | `crates/nono-cli/src/wiring.rs` | After canonicalization, `apply_yaml_merge` now compares `canonical_target == canonical_source` and returns `NonoError::ProfileParse` when they are equal. Closes silent formatting corruption (comments, anchors, quoting style lost on a YAML self-merge round-trip). Symlink/`..` aliases also caught by the canonicalize-then-compare. Added regression test `apply_yaml_merge_rejects_self_merge`. |
| WR-07 | `8f6f39a2` | `crates/nono-cli/src/wiring.rs`, `crates/nono-cli/tests/yaml_merge_reversal.rs` | Replaced manual `components().zip()` loop in `validate_target_path` with `Path::starts_with()` (which itself uses component-based comparison per stdlib docs). Removed the "grep acceptance criterion" process-check encoded into structural code. Path-validation suite (traversal, symlink escape, UNC alias, valid target, yaml_merge integration) all continue to pass. |

### Deferred INFOs

Per fix-scope policy, INFO findings are not auto-applied:

- IN-01: Duplicate `tracing::{debug, info, warn}` import branches in `learn.rs` — cosmetic; both cfgs mutually exclusive.
- IN-02: `select_exec_strategy` accepts 5 args but uses none — by-design convergence, comment polish only.
- IN-03: `serde_yaml_ng = "=0.10.0"` exact-version pin lacks renovate config — documentation-only.
- IN-04: `sandbox_log.rs::parse_event_message` `rsplit_once('(')` edge case — exotic process names with `(`.

Each is recommended for handling in a future quick-task rather than the Phase 36 closure.

---

_Fixes applied: 2026-05-13_
_Fixer: Claude (gsd-code-fixer)_
_Scope: critical_warning (4 CR + 7 WR)_
