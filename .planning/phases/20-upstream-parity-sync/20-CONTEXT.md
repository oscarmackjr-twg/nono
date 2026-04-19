# Phase 20: Upstream Parity Sync (UPST) - Context

**Gathered:** 2026-04-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Back-port upstream `always-further/nono` functional changes (0.31 → 0.37.1) into the `windows-squash` fork to re-establish Unix/macOS parity and absorb the `rustls-webpki` RUSTSEC-2026-0098/0099 security upgrade — without regressing the Windows-specific work delivered in Phases 1–19.

**In scope:** selected upstream commits that land clean or are portable-with-manual-work into the fork; crate-version realignment from 0.30.1 → 0.37.1 across all workspace crates; a Windows-regression safety net so Phase 1–19 behavior is preserved on every plan.

**Out of scope (route elsewhere or explicitly defer):**
- Any upstream feature the user explicitly excluded in the scope discussion (see Deferred Ideas)
- New Windows capabilities (those are v2.2+)
- Unix native backends for RESL-01..04 (`--cpu-percent` / `--memory` / `--timeout` / `--max-processes`) — separate cross-platform milestone
- AIPC-01 (Phase 18) and ATCH-01 (Phase 17) — independent v2.1 work, not touched here
- Rename or re-publish of the fork crate name (`nono` → `nono-windows`) — separate strategic decision

</domain>

<decisions>
## Implementation Decisions

### Merge Strategy & Target Version

- **D-01: Hybrid per-feature merge strategy.** Pick the right tool per upstream change: cherry-pick atomic commits when the touched file has low fork drift (e.g., `8876d89` rustls-webpki bump); manual port when the fork has heavily refactored the file (keystore.rs: fork 2369 vs upstream 2901; sandbox_prepare.rs: fork 452 vs upstream 1585; macos.rs: 62-line fork diff); rebase/copy for additive upstream-only paths. No monolithic rebase onto `upstream/v0.37.1`.
- **D-02: Target upstream `v0.37.1`.** Includes the rustls-webpki 0.103.12 RUSTSEC-2026-0098/0099 fix (commit `8876d89`), 0.37.0 env-var filtering (#688, commit `1b412a7`), and the post-0.36 claude-code + Seatbelt refinements. No split target — one landing point simplifies Cargo.lock and transitive deps.
- **D-03: rustls-webpki RUSTSEC fix lands first in its own plan (20-01), shippable-standalone.** Atomic, revertible, low-risk. Other plans rebase on the post-20-01 Cargo.lock. This way the CVE fix can ship inside v2.1 even if later plans drag.
- **D-04: Commits land directly on `windows-squash`.** Same milestone branch as Phases 16–19. Each plan commits atomically; phase closes with a verifier pass like Phase 19. No separate upstream-sync branch, no worktree+squash.

### Feature Scope — IN for Phase 20 (9 items)

**Must-land (security / stability):**
- **D-05:** rustls-webpki 0.103.12 — RUSTSEC-2026-0098/0099 fix. Upstream commit `8876d89`.
- **D-06:** Profile `extends` infinite-recursion fix. Upstream commit `c1bc439`.
- **D-07:** Claude-code token refresh via `.claude.json` symlink. Upstream commit `97f7294`.

**Should-land (credentials / ergonomics):**
- **D-08:** `keyring://service/account` credential URI scheme + `?decode=go-keyring` URI parameter (upstream 0.36). Manual-port candidate — keystore.rs diverges heavily.
- **D-09:** Environment variables filtering (upstream 0.37.0 #688, commit `1b412a7`). User-facing, documented.
- **D-10:** `command_blocking_deprecation.rs` backport (upstream 0.33, ~190 lines). Brings the fork's deprecation surface in line with upstream's stated API story.

**Misc parity:**
- **D-11:** GitLab ID tokens for trust signing (upstream 0.35). Adds GitLab trust-signing path alongside existing GitHub ID tokens.
- **D-12:** `--allow-gpu` flag (upstream 0.31–0.33). Apple Silicon + Linux + WSL2 GPU passthrough. No Windows analog needed.
- **D-13:** NVIDIA procfs + `nvidia-uvm-tools` device allowlist (upstream 0.34). Linux-only GPU device nodes in the sandbox allowlist. Bundled with --allow-gpu (pointless without it).

### Plan Decomposition

- **D-14: 4 plans, feature-grouped:**
  - **20-01 Security** — rustls-webpki upgrade (D-05). Shippable standalone, lands first.
  - **20-02 Profile fixes** — profile `extends` recursion fix + claude-code token refresh (D-06, D-07).
  - **20-03 Credentials & environment** — keyring:// URI + env-var filtering + command_blocking_deprecation (D-08, D-09, D-10).
  - **20-04 GPU + trust** — `--allow-gpu` + NVIDIA procfs + GitLab ID tokens (D-11, D-12, D-13).
- **D-15: 20-01 sequential; 20-02, 20-03, 20-04 parallel after 20-01.** Plan 20-01 regenerates `Cargo.lock`; downstream plans rebase on that result. After 20-01 lands, the other three touch disjoint files and should run as Wave 1 parallel (Phase 19 pattern).
- **D-16: Verification gate per plan = `make ci` + feature-specific targeted smoke.** `make ci` (build + clippy `-D warnings` + `-D clippy::unwrap_used` + fmt --check + workspace tests) MUST pass before each atomic commit. Plus per-feature smoke: e.g., keyring:// URI round-trip test; `--allow-gpu` flag parse test; GitLab trust token happy-path. Matches Phase 16/19 discipline.
- **D-17: Multiple atomic commits per plan, one per semantic change.** A plan may land 2–6 DCO-signed commits, each an atomic logical change. Example: 20-03 commits = `feat(20-03): port keyring:// URI scheme from upstream 0.36`, `feat(20-03): port env-var filtering from upstream 0.37 #688`, `feat(20-03): backport command_blocking_deprecation from upstream 0.33`. Matches Phase 19 pattern.

### Windows Protection & Crate Versioning

- **D-18: Manual port per-file for heavily-diverged files.** Files where `git diff v0.37.1 -- <path>` shows large fork drift (keystore.rs, sandbox_prepare.rs, macos.rs, nono-proxy internals) get a read-upstream-and-replay treatment. Each manual port's commit body must document what was ported and why straight cherry-pick was infeasible.
- **D-19: Bump all workspace crates to `0.37.1`.** `nono`, `nono-cli`, `nono-proxy`, `nono-ffi` all move 0.30.1 → 0.37.1 to match upstream. Signals parity; simplifies future upstream sync. Landed as part of Plan 20-01 alongside the `Cargo.lock` regeneration.
- **D-20: Windows-regression safety net per plan (non-negotiable):**
  - `cargo test --workspace --all-features` passes on Windows
  - Phase 15 5-row detached-console smoke gate passes (nono run → nono ps → nono stop, 5-row verification on the detached path)
  - `wfp_port_integration` test suite passes (`cargo test -p nono-cli --test wfp_port_integration -- --ignored` where admin+service are available; otherwise documented-skipped with rationale)
  - `learn_windows_integration` test suite passes
  - Any plan that cannot meet this gate STOPs and surfaces the regression before proceeding.
- **D-21: Windows-only files are structurally invariant.** Upstream has **no** Windows backend (README v0.36.0: "Native Windows support in planning"; upstream/main has not changed this). Any upstream commit that touches `crates/nono/src/sandbox/windows.rs`, `crates/nono-cli/src/bin/nono-wfp-service.rs`, `crates/nono-cli/src/exec_strategy_windows/`, `crates/nono/src/supervisor/socket_windows.rs`, `crates/nono-cli/src/pty_proxy_windows.rs`, `crates/nono-cli/src/learn_windows.rs`, `crates/nono-cli/src/session_commands_windows.rs`, `crates/nono-cli/src/trust_intercept_windows.rs`, `crates/nono/Cargo.toml`'s `target_os = "windows"` block, or `crates/nono-cli/Cargo.toml`'s `target_os = "windows"` block is **by definition a cherry-pick bug** — abort and investigate. Document this invariant in every plan's PLAN.md § Non-Goals.

### Claude's Discretion

- **Exact test-command invocations inside `make ci`** — e.g., whether plan 20-03's keyring:// round-trip smoke runs under `cargo test --all-features` or needs a dedicated `--test keyring_uri_integration` file. Planner decides based on existing test harness conventions.
- **Order of the three parallel plans (20-02, 20-03, 20-04).** D-15 says they run in parallel; if a real dependency surfaces during planning (e.g., claude-code token refresh shares a file with keyring:// work), planner can re-order or sequentialize with a note.
- **Commit-body provenance format for cherry-picks.** D-17 says atomic commits; Recommended that clean cherry-picks include `Upstream-commit: <hash>` and `Co-Authored-By:` the upstream author, but exact template is planner's call.
- **Per-plan PR vs single Phase-20 PR.** Project norm on `windows-squash` is one PR per phase (Phase 19 pattern). Default to that unless a single plan's blast radius justifies its own PR.
- **Whether to split 20-03 further if `keyring://` manual port balloons.** If the manual port of keyring:// URI into fork's 2369-line keystore.rs exceeds ~400 lines of diff, planner may split 20-03 into 20-03a (keyring) and 20-03b (env-filter + deprecation backport). Document if split.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 20 scope sources
- `.planning/ROADMAP.md` § Phase 20 — phase goal, open questions, recommended target
- `.planning/quick/260419-cmp-upstream-036-windows-parity/COMPARISON.md` — authoritative functional-diff report between upstream v0.36.0 and fork HEAD `8f5927c` (commit `7180f23`)
- `.planning/quick/260419-cmp-upstream-036-windows-parity/PLAN.md` — research task brief; explains method and scope of the COMPARISON report

### Project-level context
- `.planning/PROJECT.md` § Core Value — "Windows security must be as structurally impossible and feature-complete as Unix platforms"; Phase 20 protects that by guarding Windows work during upstream absorption
- `.planning/REQUIREMENTS.md` § v2.1 — note: Phase 20 is scoped OUTSIDE the original RESL/AIPC/ATCH/CLEAN requirement set. Planner should surface whether Phase 20 warrants a new requirement ID (e.g., UPST-01..04 matching the 4 plans) or whether the ROADMAP phase entry suffices.
- `.planning/STATE.md` — milestone position + carryforward state; Phase 19 is the most recent closed phase (`6597fbf`)

### Coding & security standards (apply to every Phase 20 plan)
- `CLAUDE.md` § Coding Standards — no `.unwrap()`, DCO sign-off, `#[must_use]` on critical Results, env-var save/restore in tests
- `CLAUDE.md` § Security Considerations — fail-secure on any unsupported shape, path component comparison (not string ops), validate env vars before use. Directly relevant: rustls-webpki upgrade MUST fail closed if the new version rejects a cert the old one accepted; env-var filtering feature must validate filter patterns before applying.
- `CLAUDE.md` § Platform-Specific Notes — Linux Landlock allow-list-only constraint; macOS Seatbelt DSL escaping rules (relevant if any macOS Seatbelt profile changes leak in from the upstream ports).

### Upstream source (git-resolvable from `upstream` remote at `https://github.com/always-further/nono.git`)
- Tag `v0.37.1` — target landing version
- Tag `v0.37.0` — rustls-webpki fix landed here (commit `8876d89`)
- Tag `v0.36.0` — keyring:// URI landed here
- Commit `8876d89` — `chore: upgrade rustls-webpki to 0.103.12 to fix RUSTSEC-2026-0098 and -0099` (Plan 20-01 primary)
- Commit `c1bc439` — `fix(profiles): prevent infinite recursion in profile extends check` (Plan 20-02)
- Commit `97f7294` — `fix(claude-code): enable token refresh via .claude.json symlink` (Plan 20-02)
- Commit `1b412a7` — `feat: implements environment variables filtering #688` (Plan 20-03)

### Pattern reference — parallel-plan phase structure
- `.planning/phases/19-cleanup/19-CONTEXT.md` — nearest analog: 4-plan phase with Wave 1 parallel after a structural prerequisite (Phase 19 had no prereq; Phase 20 has 20-01 as the prereq)
- `.planning/phases/19-cleanup/19-VERIFICATION.md` — the verifier-pass pattern Phase 20 should follow to close
- `.planning/phases/16-resource-limits/16-02-SUMMARY.md` — example of "deferred for follow-up" commit discipline; relevant for items D-05..D-13 commit bodies

### Windows regression-test sources (for D-20 gate)
- `crates/nono-cli/tests/wfp_port_integration.rs` — Windows WFP port-filter smoke (run with `-- --ignored` under admin + service)
- `crates/nono-cli/tests/learn_windows_integration.rs` — ETW learn smoke
- `.planning/milestones/v2.0-ROADMAP.md` (archive) Phase 15 smoke gate — the 5-row detached-console verification pattern

### Fork files at risk of heavy manual-port work (from D-18)
- `crates/nono/src/keystore.rs` — fork 2369 lines vs upstream 2901; keyring:// URI work lands here
- `crates/nono-cli/src/sandbox_prepare.rs` — fork 452 vs upstream 1585; fork refactored heavily; any upstream change touching this file needs careful replay
- `crates/nono/src/sandbox/macos.rs` — 62-line fork diff; macOS Seatbelt refinements (not in Phase 20 scope) would land here
- `crates/nono-proxy/` — fork has Windows credential-injection additions; upstream proxy CONNECT log severity change would need cross-check (not in Phase 20 scope per user selection, but a fork guard is needed if future phases pick it up)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`upstream` git remote** — already configured (`https://github.com/always-further/nono.git`). `git fetch upstream` + `git log v0.37.1` works today. No setup step needed.
- **Tag `v0.37.1` resolvable** — accessible via `git show v0.37.1:<path>` and `git ls-tree v0.37.1` for per-file inspection (see COMPARISON.md for the pattern applied to v0.36.0).
- **DCO commit hook on `windows-squash`** — already enforces `Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>` on every commit. Phase 20 atomic commits inherit this automatically.
- **`make ci` target** — single entry point for the verification gate (D-16): `cargo build --workspace` + `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` + `cargo fmt --all -- --check` + `cargo test --workspace --all-features`. Already green on fork HEAD after Phase 19.
- **Cargo.lock regeneration pattern** — Phase 4 bumped MSRV and regenerated lock; same flow works for 20-01 rustls-webpki + crate-version bumps.

### Established Patterns

- **Atomic commits per task / per semantic change** — Phase 16 and Phase 19 precedent. Phase 20 inherits (D-17).
- **4-plan phase with Wave 1 parallelization** — Phase 19 shipped 4 plans all Wave 1 parallel; Phase 20 follows with 20-01 as the prereq then 20-02..04 parallel (D-15).
- **Verifier agent closes the phase** — Phase 19 closed via `/gsd-verify-phase 19` after all 4 plans complete (commit `6597fbf`). Phase 20 should follow the same close-out pattern.
- **Windows-only files use `#[cfg(target_os = "windows")]` or live under `*_windows.rs` suffixes** — these are structurally unreachable from upstream commits; D-21 invariant rests on this pattern.
- **Upstream-provenance in commit bodies** — not yet a fork norm, but Phase 20 is a natural place to establish `Upstream-commit: <hash>` + `Co-Authored-By: <upstream-author> <email>` as the pattern for clean cherry-picks.

### Integration Points

- **Plan 20-01 Cargo.lock regeneration is the gating event.** After 20-01 lands, plans 20-02/03/04 MUST rebase onto post-20-01 HEAD and confirm `cargo build --workspace` green before their feature work begins.
- **keyring:// URI work (D-08, Plan 20-03) lands in `crates/nono/src/keystore.rs`** — the most heavily diverged file in scope. Manual-port per D-18 with explicit rationale in commit body.
- **`env-var filtering` (D-09, Plan 20-03) lands across `crates/nono-cli/src/cli.rs` + `crates/nono-cli/src/profile/`** — verify no conflict with Phase 19 CLEAN-02's UNC-prefix `query_path` fix or Phase 16's `nono run` flag surface.
- **`--allow-gpu` (D-12, Plan 20-04) lands in `crates/nono-cli/src/cli.rs` near the existing `nono run` flag block** — Phase 16 added 4 flags (`--cpu-percent` / `--memory` / `--timeout` / `--max-processes`); `--allow-gpu` goes in the same neighborhood. Verify no flag-name collision.
- **GitLab trust tokens (D-11, Plan 20-04) lands in `crates/nono/src/trust/` + `crates/nono-cli/src/trust.rs`** — check `trust_intercept_windows.rs` (44 lines, Phase 4) is not on the cherry-pick path; it shouldn't be.
- **Crate-version bumps (D-19) land in 4 Cargo.tomls**: `Cargo.toml` (workspace), `crates/nono/Cargo.toml`, `crates/nono-cli/Cargo.toml`, `crates/nono-proxy/Cargo.toml`, `bindings/c/Cargo.toml` (nono-ffi). All bump 0.30.1 → 0.37.1 in Plan 20-01.

### Known risks / "be careful here"

- **`crates/nono-cli/src/sandbox_prepare.rs` refactor gap** — fork is 452 lines; upstream 0.37.1 is ≥1585 lines. Any upstream commit that claims to touch this file needs a careful read to decide if the fix applies to the fork's factored structure or not. If in doubt, STOP and surface.
- **macOS Seatbelt DSL escaping** — none of the in-scope features for Phase 20 touch Seatbelt directly (the macOS refinements were explicitly deferred). But if `command_blocking_deprecation` (D-10) references Seatbelt, re-read `CLAUDE.md` § Platform-Specific Notes before changing any profile string.
- **`make ci` currently green after Phase 19** — any red state introduced by a Phase 20 plan must be diagnosed before moving on. Don't chain red into red.
- **Phase 19 CLEAN-02 deferred issues** — `tests/env_vars.rs` (19 failures) and `trust_scan::tests::*` (1–3 failures) are pre-existing Windows-host flakes outside D-06 scope. They remain deferred; Phase 20 plans should NOT try to fix them, but should also not let them mask NEW Phase 20 regressions.

</code_context>

<specifics>
## Specific Ideas

- **Plan 20-01 smoke gate concrete commands:** `cargo update -p rustls-webpki` + `cargo build --workspace` + `cargo test --workspace` + `cargo audit` (if audit tool available) to confirm RUSTSEC-2026-0098 and -0099 are both cleared. Commit body explicitly cites the two RUSTSEC IDs.
- **Plan 20-03 manual-port discipline for keystore.rs:** open two terminals — one on `v0.37.1:crates/nono/src/keystore.rs`, one on fork HEAD. Identify the keyring:// parser block upstream, locate its natural home in the fork's factored keystore.rs, replay semantically, add the `?decode=go-keyring` query-param handling. Write a unit test that round-trips `keyring://service/account` → `KeyringUri { service, account }`.
- **Plan 20-04 `--allow-gpu` integration:** flag parses to a new `CapabilitySet::gpu` or equivalent; sandbox layers apply it per-platform (macOS Seatbelt grants, Linux device-file allowlist including the D-13 NVIDIA procfs + nvidia-uvm-tools). Windows accepts the flag with a "not-enforced-on-this-platform" warning (matches the Phase 16 pattern for RESL-01..04 on Unix).
- **Commit-body template for clean cherry-picks (D-17 + Claude's Discretion):**
  ```
  feat(20-0X): <one-line change>

  <2-3 line why-this-matters rationale>

  Upstream-commit: <hash>
  Upstream-tag: v0.37.1
  Upstream-author: <name> <email>
  Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
  ```
- **STOP triggers for Phase 20 (escalate before proceeding):**
  1. A cherry-pick or manual port touches a `*_windows.rs` file or any `target_os = "windows"` block → D-21 violation.
  2. `make ci` goes red on a plan and root cause isn't obvious within ~30 minutes → re-discuss before patching around it.
  3. A manual port diff exceeds ~400 lines → D-14 plan split may be needed.
  4. The Phase 15 5-row smoke gate fails on any plan → the port regressed Windows behavior; revert and re-scope.
  5. `cargo audit` still flags RUSTSEC-2026-0098/0099 after 20-01 → the upgrade didn't reach the transitive closure; investigate before closing 20-01.

</specifics>

<deferred>
## Deferred Ideas

### Upstream features explicitly excluded from Phase 20 scope
These were considered during discussion and left out of Phase 20. Capture as candidates for a future back-port phase (e.g., Phase 21 / v2.2):

- **Unix domain socket allow in restricted net modes** — upstream commit `98460a0`. Sandbox correctness fix for Linux/macOS. Not in Phase 20 scope.
- **Learn: print profile JSON fallback when save fails** — upstream commit `9e24ce1`. UX polish for `nono learn`. Not in Phase 20 scope.
- **macOS Seatbelt keychain specific-op rules** — upstream commit `03cbd42`. Would land in `sandbox/macos.rs` (62-line fork diff). Not in Phase 20 scope.
- **macOS Mach IPC denies + atomic-write temp-file allow** — upstream 0.31 / 0.33 Seatbelt refinements. Fork macOS users on the slightly-older profile. Not in Phase 20 scope.
- **macOS claude-code launch services + keychain refinements** — upstream 0.34. macOS-only setup-flow polish. Not in Phase 20 scope.
- **Proxy: strip artifacts + CONNECT log severity** — upstream 0.35, 0.36. Touches `nono-proxy`, which has the fork's Windows credential-injection story. Defer until a proxy-focused phase can cross-check conflicts.
- **Hooks: invoke bash via env (0.37.1)** — upstream commit `8b5a2ff`. Small shebang portability fix. Low-risk; easy future add.

### Cross-cutting items not in Phase 20
- **Fork crate renaming (`nono` → `nono-windows`)** — considered in D-19 options, rejected in favor of bumping version to 0.37.1. Rename is a strategic branding decision, not a Phase 20 decision.
- **Retroactive update of `REQUIREMENTS.md` to add UPST-01..04 requirement IDs matching Phase 20 plans** — planner call; either add as part of Plan 20-01 or leave to the phase-close bookkeeping pass.
- **Re-sync from upstream past 0.37.1** — once Phase 20 lands, the fork is at upstream-0.37.1 feature level. Future minor-release sync is a recurring chore; not a Phase 20 item but worth scheduling as a cadence (e.g., per upstream minor release).

</deferred>

---

*Phase: 20-upstream-parity-sync*
*Context gathered: 2026-04-19*
