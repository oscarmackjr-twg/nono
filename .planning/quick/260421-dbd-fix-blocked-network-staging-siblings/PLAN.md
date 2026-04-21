---
task: fix-blocked-network-staging-siblings
type: bug-fix
severity: medium (correctness on firewall-fallback path)
source: gemini-code-assist PR #725 review (comment 3119918138, network.rs:103)
created: 2026-04-21
---

# Quick Task: Stage sibling DLLs/manifests alongside the program binary

## Problem

gemini-code-assist on PR #725 flagged `stage_program_for_blocked_network_launch` in `crates/nono-cli/src/exec_strategy_windows/network.rs`:

> The staging logic for blocked-network enforcement only copies the main executable to a temporary directory. If the application depends on DLLs located in its original directory, it will fail to start.

**Why staging exists:** blocked-network enforcement on this fallback path uses a per-program Windows Firewall rule (`netsh advfirewall firewall add rule ... program=<path>`). Using the program's real path would permanently affect future launches outside nono. Staging a copy to a unique temp dir lets the rule be scoped to the sandboxed launch.

**The gap:** many Windows apps ship with sibling DLLs, manifests, or config files in the same directory. Copying only the `.exe` to a fresh temp dir means the loader fails to find the deps when the program starts.

## Context: this is the fallback path, not the primary backend

The blocked-network enforcement on Windows has two backends:
1. **Primary: WFP** (kernel-level, SID-based) — doesn't stage anything, doesn't have this gap.
2. **Fallback: temporary Windows Firewall rules** — this function, per-program path-based.

WFP is preferred per `classify_netsh_firewall_failure` ("The long-term Windows backend target is WFP"). This fix improves the fallback without reshaping the architecture.

## Fix

Add a sibling-copy pass after copying the program itself. Allowlist extensions that are commonly-loaded dependencies; skip everything else so we don't pull in unrelated content (e.g., if the program lives in `C:\Program Files\Something\` we don't accidentally copy gigabytes).

**Allowlist:** `.dll`, `.pdb`, `.manifest`, `.config`, `.xml`.
- `.dll` — runtime dependencies.
- `.pdb` — debug symbols (loader + stack tools look for them).
- `.manifest` — Windows activation contexts and side-by-side assembly manifests (e.g., `MyApp.exe.manifest`).
- `.config` — .NET runtime config (e.g., `MyApp.exe.config`).
- `.xml` — some WinRT / activation contexts live in `.xml`.

**Excluded intentionally:** resource files, data, docs, localization dirs, everything else. If a program has an unusual layout, it will still fail — we document this as a known limitation of the fallback path rather than trying to copy everything.

Fail-secure: if any copy fails, abort the whole stage (don't silently start the program with missing deps and hope for the best).

## Implementation

Single new helper `copy_program_siblings`, called from `stage_program_for_blocked_network_launch` after the main copy. ~25 lines net.

## Verification

- `cargo build --workspace` → exit 0
- `cargo fmt --all -- --check` → clean
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` → exit 0
- Optional smoke: run `nono wrap` (firewall-fallback mode) on a simple program with a sibling DLL and verify it launches. Documented as manual check; no automated test added in this pass.

## Non-goals

- Do NOT redesign the Windows Firewall fallback path. WFP is the primary; this path is maintained for legacy compatibility.
- Do NOT add a byte/count cap on the sibling copy. Extension allowlist is the gate; simpler to review.
- Do NOT switch to hard links or symlinks — they defeat the purpose of staging (Windows Firewall rule needs a distinct image path).
- Do NOT attempt `SetDllDirectory` or child-process environment tricks — we have no control over child-side code.

## Propagation

Standard flow (this function exists on all three branches with identical content):
1. Fix on windows-squash, commit.
2. Cherry-pick + amend onto v2.0-pr, force-push.
3. Rebase v2.1-pr onto updated v2.0-pr, force-push.
4. Reply + resolve PR #725 thread `PRRT_kwDORFb4ys58myFu`.
