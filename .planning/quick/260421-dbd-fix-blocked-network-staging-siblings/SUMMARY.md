---
task: fix-blocked-network-staging-siblings
status: complete
completed: 2026-04-21
source: gemini-code-assist PR #725 review (comment 3119918138)
---

# Summary: Stage sibling DLLs/manifests alongside the program binary

## Outcome

Added `copy_program_siblings` helper to `stage_program_for_blocked_network_launch` in `crates/nono-cli/src/exec_strategy_windows/network.rs`. The fallback Windows Firewall enforcement path now stages extension-allowlisted siblings (.dll / .pdb / .manifest / .config / .xml) alongside the program binary, so programs that ship with bundled dependencies start correctly from the temp staging directory.

## File touched

- `crates/nono-cli/src/exec_strategy_windows/network.rs` — +54 / −0 net.

## Verification

- `cargo fmt --all -- --check` → clean
- `cargo build --workspace` → exit 0
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` → exit 0

## Design notes

- **Extension allowlist, not wholesale copy.** Blind `fs::copy` of the entire parent directory would pull in arbitrary amounts of unrelated data when the program lives in e.g. `C:\Program Files\SomeApp\`. The 5 allowed extensions cover Windows' standard startup dependencies without significant false positives.
- **Fail-secure.** Any copy error aborts the stage. The alternative — start the program without complete deps — would produce obscure runtime failures.
- **No cap on count/size.** A program with 200 DLL siblings is unusual but not pathological; the cost is temp-disk space during the session. Adding caps complicates the logic for a rare case.
- **Does not touch the WFP primary path.** WFP is SID-based and doesn't stage anything; this fix is purely for the Windows Firewall fallback.

## Propagation

1. Committed on windows-squash: `3f7d241 fix(windows/network-fallback): stage sibling DLLs + manifests alongside program binary`
2. Cherry-picked + amended onto v2.0-pr, force-pushed.
3. v2.1-pr rebased onto updated v2.0-pr, force-pushed.
4. Reply + resolve on PR #725 thread `PRRT_kwDORFb4ys58myFu`.
