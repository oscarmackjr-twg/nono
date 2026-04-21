---
task: fix-v2-pr-job-object-newline
type: bug-fix
severity: high (runtime-breaking on Job Object name)
source: gemini-code-assist PR #725 review (comment 3119918103, launch.rs:37)
created: 2026-04-21
---

# Quick Task: Fix literal newline in Job Object name on v2.0-pr

## Problem

gemini-code-assist on PR #725 flagged `crates/nono-cli/src/exec_strategy_windows/launch.rs:37` on the v2.0-pr squash:

> The Job Object name contains a newline character because of the multi-line raw string literal. This will likely cause issues when trying to manage the job object or may lead to unexpected behavior.
>
> Suggested fix: `r"Local\nono-session-{}"`

On v2.0-pr the code reads:
```rust
let name = format!(
    r"Local
ono-session-{}",
    id
);
```

Because `r"..."` is a raw string that preserves literal characters between the quotes — **including the newline at the end of the first line** — the actual Job Object name becomes `"Local<NL>ono-session-<id>"` rather than `"Local\nono-session-<id>"`. The Windows kernel uses `\` as namespace separator; with a newline instead, the kernel may accept the name but the namespace parsing is semantically wrong.

## This fix is v2.0-pr-only

- **windows-squash**: already correct (single-line raw string landed in commit `7db6595` as part of Phase 17's "3 latent session-id mismatch bugs" fix).
- **v2.1-pr**: already correct (inherited via v2.1 squash tree state).
- **v2.0-pr**: still broken because v2.0-pr was squashed from the tree at `ffea633` (pre-Phase-17).

Therefore this quick task does NOT land a code change on windows-squash. It documents the v2.0-pr-branch surgical fix in the quick-task log for consistency with our gemini-response flow.

## Fix

Single line on v2.0-pr `crates/nono-cli/src/exec_strategy_windows/launch.rs`:

**Before (3 source lines, 1 logical string with embedded newline):**
```rust
let name = format!(
    r"Local
ono-session-{}",
    id
);
```

**After (1 source line, 1 logical string with backslash-n as Windows namespace separator):**
```rust
let name = format!(r"Local\nono-session-{}", id);
```

Raw string semantics in Rust: `r"Local\n..."` preserves `\` and `n` as two separate characters; `\` is Windows' Job Object namespace separator. Result: kernel sees `Local\nono-session-<id>` which is a valid `Local\`-namespaced name.

## Propagation

1. Edit v2.0-pr directly (no windows-squash change needed).
2. Amend v2.0-pr squash.
3. Force-push v2.0-pr.
4. Rebase v2.1-pr onto new v2.0-pr tip — should be a tree-identity rebase (v2.1-pr's end tree already matches, only the parent SHA needs updating).
5. Force-push v2.1-pr.
6. Reply + resolve PR #725 thread `PRRT_kwDORFb4ys58myFO` (comment 3119918103).

## Verification

- Structural: `grep -n 'r"Local' crates/nono-cli/src/exec_strategy_windows/launch.rs` shows the single-line form.
- `cargo build --workspace` exit 0.
- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` exit 0.
