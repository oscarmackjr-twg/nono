---
task: fix-v2-pr-job-object-newline
status: complete
completed: 2026-04-21
source: gemini-code-assist PR #725 review (comment 3119918103)
---

# Summary: Fix literal newline in Job Object name on v2.0-pr

## Outcome

Collapsed the two-line raw string `r"Local<NL>ono-session-{}"` to single-line `r"Local\nono-session-{}"` on v2.0-pr's launch.rs. Windows Job Object kernel now receives a properly-separated `Local\nono-session-<id>` name.

## Where this fix landed

- **v2.0-pr**: amended squash commit (`5db8776` → `2d858bf`), force-pushed.
- **v2.1-pr**: rebased onto new v2.0-pr tip (`e49d419` → `0390a08`). Tree-identity rebase — v2.1-pr already had the fix in its tree state via the earlier Phase 17 commit.
- **windows-squash**: NOT touched. The fix has been on windows-squash since Phase 17's session-id-mismatch fix (`7db6595`). This quick-task commit is docs-only.

## Verification

- `cargo build --workspace` on v2.0-pr post-amend → exit 0
- Structural: `grep -n 'r"Local' crates/nono-cli/src/exec_strategy_windows/launch.rs` on v2.0-pr shows the single-line form.
- Rebase of v2.1-pr onto updated v2.0-pr succeeded cleanly with no conflicts (expected — same end-tree).

## PR thread

[Reply posted](https://github.com/always-further/nono/pull/725#discussion_r3120809042) on thread `PRRT_kwDORFb4ys58myFR`, marked resolved.
