---
task: fix-open-supervisor-path-directory
type: bug-fix
severity: medium (capability broker fails opaquely on directory paths)
source: gemini-code-assist PR #725 review (comment 3119918153, supervisor.rs:902)
created: 2026-04-21
---

# Quick Task: Clear error when capability broker receives a directory path

## Problem

gemini-code-assist on PR #725:

> `open_windows_supervisor_path` uses default `OpenOptions`, which on Windows will fail to open directories. If a sandboxed child requests access to a directory via the capability pipe, the supervisor would fail to open it with an opaque platform-level error.

On Windows, `std::fs::OpenOptions::open()` wraps `CreateFileW` without `FILE_FLAG_BACKUP_SEMANTICS`. Directory opens return `ERROR_ACCESS_DENIED`, which propagates up as a generic `SandboxInit("Windows supervisor failed to open approved path … Access is denied")` — misleading because the issue isn't the ACL, it's the platform's requirement for the backup-semantics flag.

## Design decision

**Explicit rejection with clear error, not transparent directory support.**

Two options were considered:
1. (Chosen) Detect directory paths pre-open; reject with a clear capability-broker-boundary message.
2. Set `FILE_FLAG_BACKUP_SEMANTICS` via `OpenOptionsExt::custom_flags` and return directory handles.

Option 2 is rejected for this fix because:
- The cap-pipe protocol was designed to broker **file handles** (`HandleKind::File` + `DuplicateHandle` based on read/write access mask). Directory handles are semantically different: they enable enumeration + change-notification beyond the approved file's scope.
- Granting a directory handle silently would be capability-scope expansion (broker surface change). That needs a protocol revision, not a drive-by patch.
- If future work wants directory brokering, adding an `is_dir: bool` to the request keeps the distinction explicit.

The diagnostic improvement alone (option 1) is what gemini was flagging — "supervisor would fail" implied "would fail opaquely with a confusing error", not "should be made to succeed".

## Fix

Pre-open metadata check. If the target is a directory, return a clear capability-broker-boundary error. Otherwise proceed with the existing OpenOptions flow.

```rust
pub(super) fn open_windows_supervisor_path(
    path: &Path,
    access: &nono::AccessMode,
) -> Result<std::fs::File> {
    // std::fs::OpenOptions on Windows uses CreateFileW without
    // FILE_FLAG_BACKUP_SEMANTICS, so directory paths fail with an opaque
    // ERROR_ACCESS_DENIED. The capability broker contract is file-scoped
    // by design (cap-pipe brokers file handles via DuplicateHandle, not
    // directory enumeration handles). Reject directory paths up-front
    // with a clear boundary message rather than surfacing the platform
    // error. Future directory brokering (if ever needed) should add an
    // explicit `is_dir` discriminator to the request rather than
    // silently flipping this branch.
    if path.metadata().map(|m| m.is_dir()).unwrap_or(false) {
        return Err(NonoError::SandboxInit(format!(
            "Windows supervisor refused directory path {}: capability broker brokers file handles only",
            path.display()
        )));
    }

    let mut options = std::fs::OpenOptions::new();
    // ... (existing match on access mode) ...
}
```

## TOCTOU note

The metadata check and the subsequent open are not atomic — a symlink could be swapped between the two. The existing code has the same TOCTOU shape (no pre-open check). This fix doesn't make the race worse; the worst case is a directory check that says "file" followed by an open that now fails with `ACCESS_DENIED` — i.e., back to the original behavior.

Proper atomicity would require `CreateFileW` with `FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS` and a `GetFileInformationByHandle` post-open check. That's a substantial refactor. Not worth it for a diagnostic-quality improvement; the approved path has already passed the policy check anyway.

## Verification

- `cargo fmt --all -- --check` / `cargo build --workspace` / `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` all green.
- No existing tests of `open_windows_supervisor_path` (grep shows no test callers). Not adding a unit test for this — the behavior is a single-statement pre-check with obvious semantics.

## Propagation

Standard flow: windows-squash → v2.0-pr (cherry-pick + amend + force-push) → v2.1-pr (rebase + force-push) → reply + resolve PR thread `PRRT_kwDORFb4ys58myF5`.
