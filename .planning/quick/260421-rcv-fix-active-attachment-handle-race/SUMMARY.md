---
task: fix-active-attachment-handle-race
status: complete
completed: 2026-04-21
source: gemini-code-assist PR #725 review (high)
---

# Summary: Fix race in `start_logging` WriteFile to `active_attachment`

## Outcome

Race between `start_logging` (reads `active_attachment`, releases lock, calls raw `WriteFile`) and `start_data_pipe_server` (acquires same mutex to clear on disconnect, releases lock, drops `File` wrapper → `CloseHandle(h_pipe)`) could result in the logging thread writing child stdout bytes into a recycled HANDLE referring to an unrelated kernel object. Fixed by holding the mutex across the WriteFile call — the pipe-sink thread cannot drop its File wrapper without re-acquiring the same lock.

## File touched

| File | Function | Change |
|---|---|---|
| `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` | `start_logging` | Collapsed the old "snapshot-then-release" pattern into a single lock scope that spans the WriteFile call. Kept raw FFI WriteFile to preserve the existing ERROR_NO_DATA / ERROR_BROKEN_PIPE tolerance (bridge thread must not die on pipe errors). |

Net diff: 1 file, +13 / −9 lines, net +4 (mostly the expanded SAFETY comment documenting the lock-held invariant).

## Verification

- `cargo fmt --all -- --check` → clean
- `cargo build --workspace` → exit 0
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` → exit 0
- Structural check: `grep -n 'attachment_handle = {' supervisor.rs` → 0 hits (old two-step pattern removed)
- Existing `crates/nono-cli/tests/attach_streaming_integration.rs` unchanged; integration coverage remains.

## Design notes

- Chose "hold the lock" (gemini's first suggestion) over `DuplicateHandle` (second suggestion). The `DuplicateHandle` approach requires:
  - A second owned HANDLE on the logging-thread side
  - Lifecycle management (who closes the dup, when, on reattach)
  - A separate mechanism to detect when the original pipe has disconnected
  That's substantially more state for the same correctness gain. Holding the lock is ~8 lines and obviously correct.
- Pipe-sink thread only holds the lock for the brief swap-in (line ~989) / swap-out (line ~1017) of the `Option`. Contention is microsecond-scale — the new logging-thread hold is bounded by a single 4KiB WriteFile on a local named pipe whose reader is another nono process draining continuously.
- Worst-case WriteFile stall: attach client stuck not reading, buffer full. WriteFile would block. But Windows named pipes structurally return `ERROR_BROKEN_PIPE` fast when the reader has disconnected, so the stall is bounded by the client's own misbehavior, not unbounded.
- No deadlock possible: pipe-sink's code under the lock is straight-line (set the Option) and acquires no other locks.

## Why fix only this race (not the adjacent bug)

While reading the surrounding code I noticed a separate latent issue in `start_data_pipe_server` at line 997: `File::from_raw_handle(h_pipe as _)` takes ownership of `h_pipe` without `ManuallyDrop`. When `file` drops at the end of the `if connected` block, `CloseHandle(h_pipe)` fires. The next loop iteration's `ConnectNamedPipe(h_pipe)` would then operate on a closed handle, preventing re-attachment after the first client disconnects.

Flagged to user as a separate follow-up; NOT fixed here. The gemini comment is specifically about the write-side race, and mixing two unrelated lifecycle fixes into one commit would muddy the review. The File-drop bug also only manifests on the SECOND attach attempt — it hasn't fired in v2.1 HUMAN-UAT (which exercised attach once) so it's not a release blocker.

## Propagation to PR branches

Hazardous code exists on BOTH `v2.0-pr` and `v2.1-pr` (pattern predates Phase 17 — Phase 02 / Persistent Sessions introduced the active_attachment mirror).

Plan per prior fix:
1. Cherry-pick + amend onto `v2.0-pr` (strip `.planning/quick/` from stage), force-push.
2. Rebase `v2.1-pr` onto updated `v2.0-pr`, force-push.
3. Reply + resolve the gemini thread on PR #725.
