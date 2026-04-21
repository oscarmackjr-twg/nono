---
task: fix-sc-create-binpath-quoting
type: bug-fix
severity: high (Windows service registration breakage on paths with spaces)
source: gemini-code-assist PR #725 review (comment 3119918146, network.rs:223)
created: 2026-04-21
---

# Quick Task: Quote driver binPath value in `build_wfp_driver_create_args`

## Problem

gemini-code-assist on PR #725:

> The `binPath` argument for `sc create` is not quoted. If the path to the driver binary contains spaces, the command will be misinterpreted by the Service Control Manager.

Windows `sc create SERVICE binPath= VALUE` stores VALUE verbatim as the service's ImagePath registry entry. If VALUE contains spaces and no quotes, the Service Control Manager at boot time will split on whitespace using CreateProcess conventions — treating the first space-separated token as the image and the rest as arguments. E.g., `C:\Program Files\nono\driver.sys /foo` is stored and later parsed as program `C:\Program` with args `Files\nono\driver.sys /foo`. Service fails to start.

## Current state

```rust
// crates/nono-cli/src/exec_strategy_windows/network.rs:276
pub(super) fn build_wfp_driver_create_args(config: &WfpProbeConfig) -> Vec<String> {
    vec![
        "create".to_string(),
        config.backend_driver.to_string(),
        "binPath=".to_string(),
        config.backend_driver_binary_path.display().to_string(),   // ← no quotes
        ...
    ]
}
```

Compare `format_wfp_service_command` at line 214 — for the SERVICE binary path, quotes are already embedded. The DRIVER binary path was missed.

## Fix

Change line 281 from
```rust
config.backend_driver_binary_path.display().to_string(),
```
to
```rust
format!("\"{}\"", config.backend_driver_binary_path.display()),
```

The embedded `"` characters become part of the VALUE that `sc create` stores; SCM at boot time then parses the quoted image path correctly.

## Verification

- `cargo fmt --all -- --check` / `cargo build --workspace` / `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` all green.
- No existing tests assert the pre-fix shape (grep shows no test callers of `build_wfp_driver_create_args`). Not adding a unit test in this pass — behavior is a single-site string-format change with a clear invariant.

## Non-goals

- Do NOT refactor both `build_wfp_{service,driver}_create_args` into a shared helper. The service branch already quotes via `format_wfp_service_command`; DRY extraction is a separate concern.
- Do NOT change the `binPath= <space>` convention — that's required by `sc.exe` syntax.

## Propagation

Standard flow: windows-squash → v2.0-pr (cherry-pick + amend + force-push) → v2.1-pr (rebase + force-push) → reply + resolve PR thread `PRRT_kwDORFb4ys58myF1`.
