---
task: fix-open-supervisor-path-directory
status: complete
completed: 2026-04-21
source: gemini-code-assist PR #725 review (comment 3119918153)
---

# Summary: Clear error when capability broker receives a directory path

## Outcome

Added a pre-open `metadata().is_dir()` check to `open_windows_supervisor_path`. Directory paths now return a clear capability-broker-boundary error rather than surfacing Windows' opaque `ERROR_ACCESS_DENIED` from `CreateFileW` without `FILE_FLAG_BACKUP_SEMANTICS`.

Deliberately chose clear rejection over transparent directory support: the cap-pipe protocol is file-scoped by design, and silently granting directory handles would expand capability scope beyond the existing broker contract. Future directory brokering (if ever needed) should require an explicit `is_dir` discriminator on `CapabilityRequest`.

## File touched

- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — added 13 lines (metadata check + comment) in `open_windows_supervisor_path`.

## Verification

- `cargo fmt --all -- --check` → clean
- `cargo build --workspace` → exit 0

## PR thread

To resolve: `PRRT_kwDORFb4ys58myF5` (comment 3119918153).
