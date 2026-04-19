# Phase 20 — Deferred Items (out of scope for current plans)

Items logged during Plan 20-04 execution that are outside the plan's scope
boundary and should be addressed in a future plan or phase.

## Plan 20-04 out-of-scope discoveries

### Pre-existing clippy `unwrap_used` violations in `crates/nono-cli/src/cli.rs` test module

Introduced by Plan 20-03 (commit `e6fde89`, 2026-04-19). The `parser_tests`
module (which lacks a `#[allow(clippy::unwrap_used)]` attribute) has two
test functions that use `.unwrap()` / `.unwrap_err()` directly:

- `crates/nono-cli/src/cli.rs:2646` — `cli.unwrap_err().to_string()` in
  `env_allow_malformed_pattern_fails_closed_at_parse` (Plan 20-03 test)
- `crates/nono-cli/src/cli.rs:2719` — `Cli::try_parse_from(...).unwrap()`
  in `env_allow_default_is_empty_vec` (Plan 20-03 test)

Confirmed pre-existing by `git stash && cargo clippy --workspace
--all-targets -- -D warnings -D clippy::unwrap_used` on post-Plan-20-03
HEAD (`c3297aa`) — same 2 errors, same line numbers.

**Why deferred:** Outside Plan 20-04's `files_modified` boundary (Plan
20-04 only owns the `--allow-gpu`-related edits on `cli.rs`, not the
Plan 20-03 env-filter tests). Plan 20-04's own new tests in
`parser_tests` use `.expect("…")` with meaningful messages, per
CLAUDE.md § Coding Standards, and introduce zero new clippy violations.

**Suggested fix:** One-line style PR: replace both `.unwrap[_err]()`
calls with `.expect("…")`. No functional change. Could land alongside
a future Phase 19 CLEAN-05 or as a follow-up housekeeping task.
