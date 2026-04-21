---
task: fix-windows-system32-path-hijack
type: security-fix
severity: security-high
source: gemini-code-assist PR review (upstream always-further/nono PR)
created: 2026-04-21
---

# Quick Task: Fix Windows `Command::new` relative-path hijacking

## Problem

gemini-code-assist flagged `crates/nono-cli/src/exec_strategy_windows/launch.rs:927`:

> The icacls command is invoked using a relative path. On Windows, this is a security risk as it allows for path hijacking if a malicious executable with the same name is placed in the application directory or the current working directory. Use an absolute path derived from the SystemRoot environment variable (e.g., C:\Windows\System32\icacls.exe).

A full grep reveals 10 call sites (not just the flagged one) with the same pattern:

| File | Line | Command | Context |
|---|---|---|---|
| crates/nono-cli/src/exec_strategy_windows/launch.rs | 927 | `icacls` | prod — flagged |
| crates/nono-cli/src/exec_strategy_windows/network.rs | 37 | `netsh` | prod |
| crates/nono-cli/src/exec_strategy_windows/network.rs | 165 | `sc` | prod |
| crates/nono-cli/src/exec_strategy_windows/network.rs | 177 | `sc` | prod |
| crates/nono/src/sandbox/windows.rs | 1806 | `cmd` | `#[cfg(test)] mod tests` |
| crates/nono/src/sandbox/windows.rs | 1833 | `icacls` | `#[cfg(test)] mod tests` |
| crates/nono-cli/tests/env_vars.rs | 36 | `icacls` | integration test |
| crates/nono-cli/tests/env_vars.rs | 69 | `netsh` | integration test |
| crates/nono-cli/tests/env_vars.rs | 87 | `netsh` | integration test |
| crates/nono-cli/tests/wfp_port_integration.rs | 38 | `net` | integration test |

## Fix

Introduce `system32_exe(name: &str) -> PathBuf` helper that resolves `%SystemRoot%\System32\<name>.exe`, with a fallback chain already used elsewhere in the codebase:

```rust
let system_root = std::env::var_os("SystemRoot")
    .or_else(|| std::env::var_os("windir"))
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
system_root.join("System32").join(format!("{name}.exe"))
```

Placement:
- `pub(super) fn system32_exe` in `crates/nono-cli/src/exec_strategy_windows/mod.rs` — shared by `launch.rs` + `network.rs` (both are submodules).
- Private `fn system32_exe` in `crates/nono/src/sandbox/windows.rs` tests module.
- Private `fn system32_exe` in each of `crates/nono-cli/tests/env_vars.rs` + `crates/nono-cli/tests/wfp_port_integration.rs` — local duplication acceptable for a 6-line helper at a test-harness boundary.

Replace each `Command::new("X")` with `Command::new(system32_exe("X"))`.

Why not pull the existing inline pattern (e.g., `command_runtime.rs:50`) into a new `pub` function in the `nono` crate: Adding public API to the security-critical library for a single-purpose helper would outlive the need. Per-boundary duplication is the lighter touch.

## Verification

- `cargo build --workspace` passes on Windows
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` passes
- `cargo fmt --all -- --check` passes
- `grep -rEn 'Command::new\("(icacls|netsh|sc|net|cmd)"\)' crates/` returns 0 hits
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` exposes `system32_exe` at `pub(super)` scope; launch.rs + network.rs call `super::system32_exe(...)`
- `crates/nono/src/sandbox/windows.rs::tests` has one local `system32_exe` used by 2 call sites

## Non-goals

- Do NOT expose `system32_exe` as public API on the `nono` crate — test boundaries keep their own copies.
- Do NOT refactor the existing `SystemRoot` inline resolutions in `command_runtime.rs` / `launch.rs::append_windows_runtime_env` / `labels_guard.rs` / `sandbox/windows.rs:3249` — those are unrelated env-var setups, not command-invocation hazards.
- Do NOT change command semantics (args, stdout/stderr handling). Only replace the first argument of `Command::new`.
