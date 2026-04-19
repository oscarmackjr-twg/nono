# Session retention policy

## Retention rule

Only sessions with `Status: Exited` are ever pruned. Active sessions
(`Running` / `Paused`) are NEVER removed regardless of age. The default
age threshold is 30 days. Boundary is inclusive: a session exactly 30
days old qualifies for pruning.

Fail-closed semantics: if a session's `started_epoch` is malformed,
corrupt, or in the future (clock skew), the retention predicate returns
`false` and the record is not pruned.

## Automatic sweep on `nono ps`

At the start of `nono ps`, if more than 100 stale session files exist
on disk, nono spawns a background thread to prune them and emits:

```
info: pruning N stale session files (>30 days, exited)
```

to stderr. The `ps` table output itself is not delayed — the background
thread performs deletions independently.

The automatic sweep is a **structural no-op inside sandboxes**. When
`NONO_CAP_FILE` is set (i.e. the caller is a sandboxed agent), the
sweep returns immediately. `nono ps` remains a read-only inspection
command when invoked from inside nono; only the deletion side effect
is suppressed. This guard matches the canonical sandbox detection used
elsewhere in the CLI (see `reject_if_sandboxed`) and prevents a
sandboxed agent from causing the host supervisor to delete its own
session records (threat T-19-04-07, mitigated).

## Manual prune (`nono prune`)

| Flag                       | Behavior                                                                                                                                                                  |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--older-than <DURATION>`  | Remove exited sessions older than this duration. Default: `30d`. Accepts `30s`, `5m`, `1h`, `30d`. Raw integers (e.g. `30`) are rejected with a helpful error — specify a suffix. |
| `--all-exited`             | Ignore age and prune every `Exited` session. Escape hatch. Mutually exclusive with `--older-than`.                                                                        |
| `--dry-run`                | List candidates, delete nothing. Exit code 0; the reported count matches the real count.                                                                                   |
| `--keep <N>`               | Keep the N most recent candidates in addition to any other filter.                                                                                                        |

### Examples

```
# Preview sweep of anything older than 30 days (default)
nono prune --dry-run

# Sweep anything older than 12 hours
nono prune --older-than 12h

# Wipe every exited session regardless of age
nono prune --all-exited

# Remove all stale sessions but keep the 10 most recent
nono prune --keep 10
```

### Breaking change (v2.0 → Plan 19-04)

The pre-Plan-19-04 `--older-than <DAYS>` (integer days) was replaced by
`--older-than <DURATION>` with a require-suffix parser. Scripts passing
`--older-than 30` will now fail fast with a migration hint:

```
error: ambiguous duration '30' — please specify a suffix: 30s, 30m, 30h, 30d
```

This is intentional — silently treating `30` as `30 seconds` instead of
`30 days` would be a surprise nobody wants.

## Configuration knobs

Currently the 30-day retention and 100-file threshold are compile-time
constants in `crates/nono-cli/src/session_commands.rs` (and its Windows
mirror `session_commands_windows.rs`):

```rust
const AUTO_PRUNE_STALE_THRESHOLD: usize = 100;
const AUTO_PRUNE_RETENTION_SECS: u64 = 30 * 86_400;
```

Runtime configuration (e.g. an env var or config file override) is a
potential v2.2 polish item.

## Session file location

- Unix (Linux, macOS): `~/.nono/sessions/`
- Windows: `%USERPROFILE%\.nono\sessions\`

The directory is created with mode `0o700` on Unix when first written.
Each session contributes two files: `<session_id>.json` (the record)
and `<session_id>.events.ndjson` (the event log, if any). Both are
removed together by prune.

## One-time cleanup note

During the Plan 19-04 rollout on 2026-04-18, a one-shot
`nono prune --older-than 30d` was run on the windows-squash
development host to clear the backlog accumulated during v2.0 testing
(prior to the retention policy landing).

- Pre-cleanup baseline:  1392 files.
- Post-cleanup baseline: 49 files.
- Delta:                 1343 stale session files removed.

Exact invocation: `./target/release/nono prune --older-than 30d`.
A dry-run preceded the real run and the counts agreed.

This one-shot is a historical note — ongoing operation relies on the
automatic sweep in `nono ps` and explicit `nono prune` invocations
described above. No further manual cleanup should be necessary unless
`nono ps` is rarely run and the operator prefers to sweep proactively.
