---
task: fix-windows-system32-path-hijack
status: complete
completed: 2026-04-21
source: gemini-code-assist PR review (security-high)
---

# Summary: Fix Windows `Command::new` relative-path hijacking

## Outcome

10 call sites fixed across 5 files. All OS utilities (`icacls`, `netsh`, `sc`, `cmd`, `net`) now invoked via `system32_exe("<name>")` which resolves to `%SystemRoot%\System32\<name>.exe`.

## Files touched

| File | Call sites fixed | Helper placement |
|---|---|---|
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` | +19 lines | Added `pub(super) fn system32_exe` |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` | 1 (`icacls`) | Uses `super::system32_exe` |
| `crates/nono-cli/src/exec_strategy_windows/network.rs` | 3 (`netsh`, `sc`, `sc`) | Uses `super::system32_exe` |
| `crates/nono/src/sandbox/windows.rs` | 2 (`cmd`, `icacls` in tests) | Local `fn system32_exe` in `mod tests` |
| `crates/nono-cli/tests/env_vars.rs` | 3 (`icacls`, `netsh`, `netsh`) | Local `fn system32_exe` |
| `crates/nono-cli/tests/wfp_port_integration.rs` | 1 (`net`) | Local `fn system32_exe` |

Net diff: 6 files changed, +61 / -10.

## Verification

- `grep -rEn 'Command::new\("(icacls\|netsh\|sc\|cmd\|net)"\)' crates/` → **0 hits** (structural proof)
- `cargo fmt --all -- --check` → clean
- `cargo build --workspace` → exit 0
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` → exit 0

## Design notes

- `system32_exe` duplicated at 3 test-harness boundaries instead of extracted to a shared public API in the `nono` crate. Rationale: security-critical library should not grow public surface for a 6-line utility that test-side callers can own themselves.
- Fallback chain `SystemRoot` → `windir` → `C:\Windows` mirrors the existing pattern in `exec_strategy_windows/launch.rs:677` (`append_windows_runtime_env`) and `sandbox/windows.rs:3249` (test helper), preserving consistency.
- `PathBuf` import was already in-scope via `use super::*;` (mod.rs and sandbox/windows.rs tests) — no new `use` lines needed for the production crate changes.

## Why fix all 10 and not just the one flagged

CLAUDE.md § Security Considerations: "Fail Secure" + "Defense in Depth" + "SECURITY IS NON-NEGOTIABLE." Same hijacking hazard, same codebase — Rule 3. Test-code call sites run on dev hosts where a compromised cwd is plausible (project directory, extracted archives, etc.), so the same mitigation applies.

## Propagation to PR branches

This fix lands on `windows-squash` (source of truth). To propagate to the stacked PR branches:

1. Cherry-pick this commit onto `v2.0-pr` (where the bug lives — pre-squash scope included all the flagged files).
2. Since `v2.0-pr` is a single squash commit, cherry-pick + `git commit --amend --no-edit` to keep the stack single-commit, then `git push --force-with-lease origin v2.0-pr`.
3. Rebase `v2.1-pr` onto the updated `v2.0-pr`.

Alternative: if upstream has already started review, open a separate follow-up commit on `v2.0-pr` rather than amending (preserves review context).
