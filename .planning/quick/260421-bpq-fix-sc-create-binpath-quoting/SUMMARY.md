---
task: fix-sc-create-binpath-quoting
status: complete
completed: 2026-04-21
source: gemini-code-assist PR #725 review (comment 3119918146)
---

# Summary: Quote driver binPath value in `build_wfp_driver_create_args`

## Outcome

Embedded literal `"` characters around `config.backend_driver_binary_path` in `build_wfp_driver_create_args` so the SCM stores a quoted ImagePath. Matches the existing treatment of the service-binary side in `format_wfp_service_command`.

## File touched

- `crates/nono-cli/src/exec_strategy_windows/network.rs` — single-line format-string change (plus inline comment).

## Verification

- `cargo fmt --all -- --check` → clean
- `cargo build --workspace` → exit 0

## PR thread

To resolve: `PRRT_kwDORFb4ys58myF1` (comment 3119918146).
