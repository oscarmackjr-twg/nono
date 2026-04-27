# Phase 22: UPST2 — Upstream v0.38–v0.40 Parity Sync — Context

**Gathered:** 2026-04-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Port upstream `always-further/nono` v0.38–v0.40 cross-platform features into the fork so Windows users get the same `nono` commands, flags, and security guarantees as macOS users. Five plans land 18 requirements (PROF-01..04, POLY-01..03, PKG-01..04, OAUTH-01..03, AUD-01..04). AUD-05 splits into Phase 23; DRIFT-01..02 into Phase 24.

**In scope:**
- Profile struct alignment — deserialize `unsafe_macos_seatbelt_rules`, `packs`, `command_args`, `custom_credentials.oauth2`; ship `claude-no-keychain` builtin
- Policy tightening — `override_deny` requires matching grant; `--rollback` + `--no-audit` conflict; `.claude.lock` to `allow_file`
- Package manager + packs — `nono package pull/remove/search/list` with Windows `%LOCALAPPDATA%` + long-path handling
- OAuth2 proxy + reverse-proxy upstream gating — client-credentials token cache, loopback-only HTTP upstream gating, `--allow-domain` strict-proxy preservation
- Audit integrity + attestation — hash-chained Merkle-rooted ledger, DSSE/in-toto signing, `nono audit verify`, exec-identity recording, `prune` → `session cleanup` rename preserving v2.1 CLEAN-04 invariants

**Out of scope (route elsewhere or explicitly defer):**
- AUD-05 Windows AIPC broker audit-event retrofit — Phase 23 (may collapse to no-op if Plan 22-05 reveals upstream ledger covers AIPC HandleKinds cleanly)
- DRIFT-01 / DRIFT-02 (drift-check tooling + GSD upstream-sync template) — Phase 24 (linear after Phase 22 ships; not parallel)
- WR-01 reject-stage unification, AIPC G-04 wire-protocol compile-time tightening, cross-platform RESL Unix backends — v2.3+
- WR-02 EDR HUMAN-UAT — v3.0 (no EDR-instrumented runner)
- Upstream v0.41+ ingestion — v2.3 first quick task (Phase 22 caps at v0.38–v0.40)
- `unsafe_macos_seatbelt_rules` runtime application on Windows — macOS-only by design (deserialize-only on Windows per REQ-PROF-01)
- `claude-code integration package` carry-forward — follow upstream's removal; fork's `hooks.rs` is sufficient

</domain>

<decisions>
## Implementation Decisions

### Plan 22-05 Conflict Strategy (highest-risk plan; ~1.4k LOC across heavily-forked files)

- **D-01: Cherry-pick first, fallback per-commit.** Default approach for the 7-commit audit cluster (`4f9552ec` → `4ec61c29` → `02ee0bd1` → `7b7815f7` → `0b1822a9` → `6ecade2e` → `9db06336`). Try `git cherry-pick <sha>` per upstream commit; on conflict, resolve manually but keep the commit boundary. Matches Phase 20 D-01 hybrid pattern.
- **D-02: Soft fallback gate.** Fall back to read-upstream-replay for a single commit when conflicts span >50 lines OR >2 forked files OR semantic meaning is unclear. Soft because raw line count isn't always the right signal — a 10-line conflict in `supervised_runtime.rs`'s D-19 byte-identical region can be harder than a 200-line conflict in a new module. Planner judgment call. Matches Phase 20 D-18 spirit.
- **D-03: Strict upstream chronological order.** One fork commit per upstream SHA, exact chronological sequence. Each fork commit's `Upstream-commit:` trailer is trivially verifiable; D-20 Windows-regression gate fires after each commit; no reordering. No squashing to fewer logical commits.
- **D-04: CLEAN-04 invariants verified after each rename-touching commit.** REQ-AUD-04 renames `nono prune` → `nono session cleanup` and adds `nono audit cleanup` peer. Every commit that touches `prune`/`cleanup` code re-runs the v2.1 Phase 19 CLEAN-04 invariant suite (`auto_prune_is_noop_when_sandboxed`, `--older-than <DURATION>` require-suffix parser, `--all-exited` escape hatch, 100-file auto-sweep) before commit. Catches mid-plan regression early; matches Phase 20 D-20 per-plan discipline at finer granularity for this specific high-risk surface.

### Working Branch & Origin Push

- **D-05: Phase 22 / 23 / 24 commits land directly on `main`.** Pre-milestone fast-forward (commit `1ef30c63`) made `main` the integration branch. No `v2.2-integration` branch, no per-plan branches. Matches the v2.0/v2.1 `windows-squash` direct-on-branch pattern; minimum branch-juggling overhead.
- **D-06: Push `main` to origin NOW, before Phase 22 starts.** `git push origin main` advances `origin/main` from `063ebad6` to `8213da64` (447 commits). Publishes the 46 DCO-missing commits to the user's fork; per quick task 260424-mrg path C this is acceptable (DCO only matters when opening a PR to upstream `always-further/nono`). origin/main becomes the canonical baseline for Phase 22 cherry-picks.
- **D-07: Push to origin after each plan closes.** ~8 push events across v2.2 (one per plan boundary: 22-01, 22-02, 22-03, 22-04, 22-05, 23-01, 24-01, 24-02). Bounded blast radius if local host dies; no per-commit push churn. Mid-plan commits stay local until the plan's verifier passes.
- **D-08: Push v2.0 + v2.1 tags to origin alongside the initial main push.** `git push origin v2.0 v2.1`. Tags become reachable from origin/main; PROJECT.md / RETROSPECTIVE.md tag references become verifiable from a fresh clone.

### Plan Sequencing & Wave Plan

- **D-09: Plans 22-03 (PKG) and 22-04 (OAUTH) start only after Plan 22-01 (PROF) verifier signs off.** No partial-22-01-state unblocking. Matches Phase 20 D-15 ("20-01 sequential; 20-02..04 parallel after 20-01"). Cleaner per-plan reset; less rebase churn.
- **D-10: Plan 22-02 (POLY) wave-parallels with Plan 22-01 (PROF) from phase start.** Disjoint surfaces (22-01: profile struct fields; 22-02: clap conflict, override_deny resolver, allow_file move). Matches ROADMAP-locked rationale ("22-02 independent of 22-01; can wave-parallel"). v2.0/v2.1 already validated wave-parallel discipline.
- **D-11: Phase 24 (DRIFT) sequences after Phase 22 ships.** Linear flow: Phase 22 → Phase 23 → Phase 24. Phase 24's first real use is against v0.41+ commit ranges (which don't exist until v2.3) — urgency is low, parallel-via-worktree adds management overhead for no near-term gain.
- **D-12: Plans 22-03 and 22-04 wave-parallel after 22-01 closes.** Disjoint surfaces (22-03: `package_cmd.rs` + `registry_client.rs` + hooks; 22-04: `nono-proxy/oauth2.rs` + `reverse.rs`). Maximum throughput; pattern validated in v2.1 Phase 18 + Phase 20.

**Sequencing diagram:**
```
Phase 22 start
  ├─ 22-01 (PROF) ──┐  Wave 0  (D-10: parallel)
  └─ 22-02 (POLY) ──┘
                    │
                    ↓ (22-01 closes — D-09 gate)
  ├─ 22-03 (PKG)  ──┐  Wave 1  (D-12: parallel)
  └─ 22-04 (OAUTH)──┘
                    │
                    ↓ (22-03 + 22-04 close)
  └─ 22-05 (AUD)            Wave 2 (last; D-01..D-04 strategy)
                    │
                    ↓ (Phase 22 ships — D-11 linear gate)
  └─ Phase 23 (AUD-05)
                    │
                    ↓
  └─ Phase 24 (DRIFT-01..02)
```

### Test Fixtures

- **D-13: Port upstream's OAuth2 test fixture for REQ-OAUTH-01 acceptance.** Upstream's `9546c879` (557 LOC `oauth2.rs`) ships with test infrastructure; port it as-is alongside the production code. Matches the `Upstream-commit:` provenance discipline — if upstream has fixture coverage, parity-stealing is a tax already paid. No fork-local OAuth2 mock; no public OAuth2 sandbox dependency.
- **D-14: Port upstream's package registry test fixture for REQ-PKG-01..04 acceptance.** Same logic as D-13. Upstream's `8b46573d` + `71d82cd0` + `9ebad89a` + `ec49a7af` ship registry + signed-artifact test fixtures; port them. Local-filesystem-only registry mock would bypass REQ-PKG-04 streaming-download path — insufficient on its own.
- **D-15: Add Windows-specific test cases atop ported fixtures.** Long-path `\\?\` prefix coverage (REQ-PKG-02 acceptance #2), path-traversal rejection (REQ-PKG-02 acceptance #3), Credential Manager `keyring://` resolution (REQ-PROF-03 acceptance #1, REQ-AUD-02 acceptance), Authenticode exec-identity recording (REQ-AUD-03 acceptance #2/#3). `#[cfg(target_os = "windows")]`-gated. Matches v2.1 Phase 21 WSFG Windows-only test pattern. Ported fixtures alone won't prove Windows parity; this gap is the milestone's reason to exist.
- **D-16: Tests live in existing `make ci` + `make test` (per-plan D-20 gate).** Tests land in their natural homes (`crates/nono-cli/tests/oauth2_*.rs`, `crates/nono-cli/tests/package_*.rs`, etc.) and run inside the existing Phase 20 D-20 Windows-regression safety-net gate. No new CI lane (`make test-windows-parity`). No `--ignored` gating unless a test genuinely needs admin/WFP-service infrastructure.

### Carry-Forward From Phase 20 (still binding)

- **D-17 (= Phase 20 D-21): Windows-only files are structurally invariant.** Any cherry-pick or manual-port that touches `crates/nono/src/sandbox/windows.rs`, `crates/nono-cli/src/bin/nono-wfp-service.rs`, `crates/nono-cli/src/exec_strategy_windows/`, `crates/nono/src/supervisor/socket_windows.rs`, `crates/nono-cli/src/pty_proxy_windows.rs`, `crates/nono-cli/src/learn_windows.rs`, `crates/nono-cli/src/session_commands_windows.rs`, `crates/nono-cli/src/trust_intercept_windows.rs`, `crates/nono-cli/src/open_url_runtime_windows.rs`, or any `target_os = "windows"` block in a Cargo.toml is **by definition a cherry-pick bug** — abort and investigate. Document this in every plan's PLAN.md § Non-Goals. (Note: AUD-05 in Phase 23 is the ONE planned Windows-only addition — but that's a fork-internal retrofit, not an upstream port.)
- **D-18 (= Phase 20 D-20): Windows-regression safety net per plan.** Before each plan closes:
  - `cargo test --workspace --all-features` passes on Windows
  - Phase 15 5-row detached-console smoke gate passes (`nono run` → `nono ps` → `nono stop`)
  - `wfp_port_integration` test suite passes (admin + service available; documented-skipped otherwise)
  - `learn_windows_integration` test suite passes
  - Any plan that cannot meet this gate STOPs and surfaces the regression.
- **D-19 (= Phase 20 D-17): Atomic commit-per-semantic-change with `Upstream-commit:` trailer.** Commit body template:
  ```
  feat(22-0X): <one-line change>

  <2-3 line why-this-matters rationale>

  Upstream-commit: <hash>
  Upstream-tag: v0.40.1
  Upstream-author: <name> <email>
  Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
  ```
- **D-20 (= Phase 20 D-18): Manual port for heavily-diverged files.** Files in scope where fork drift is high (`keystore.rs`, `sandbox_prepare.rs`, `rollback_runtime.rs`, `supervised_runtime.rs`, `exec_strategy.rs`, `policy.rs`, `network_policy.rs`) are read-upstream-and-replay candidates per D-02 fallback rule. Each manual port's commit body must document what was ported and why straight cherry-pick was infeasible.

### Claude's Discretion

- **Exact `make ci` invocations per plan** — whether OAuth2 fixture port runs under `cargo test --all-features` or needs a dedicated `--test oauth2_integration` file. Planner decides based on existing test harness conventions.
- **Audit signing key provisioning model on Windows** — REQ-AUD-02 acceptance uses `keyring://nono/audit`. Whether the key is auto-generated on first `--audit-sign-key` use, requires user pre-provisioning, or has a `nono audit init` setup helper is a planner / implementer call. Default to "user pre-provisioning + fail-closed if missing" unless upstream's pattern says otherwise.
- **Authenticode fallback path for REQ-AUD-03** — when Authenticode signature query fails or returns "unsigned", how the SHA-256 fallback is recorded in the ledger format. Planner decides based on upstream's record shape.
- **AUD-05 fold-or-split decision-point** — ROADMAP defaults to keeping AUD-05 in Phase 23. During Plan 22-05 execution, if upstream's ledger event shape covers AIPC HandleKinds cleanly with no Windows-specific surface, planner may surface a fold-into-22-05 proposal — discuss before deciding.
- **`prune` alias deprecation timeline** — REQ-AUD-04 acceptance #3 says `nono prune` (hidden alias) "still works and surfaces a deprecation note". Whether the alias survives one release (v2.3), two releases (v2.4), or longer is a v2.3-milestone scoping decision, not a Phase 22 decision.
- **Per-plan PR vs single Phase-22 PR** — Project norm is one PR per phase (Phase 19 / 20 pattern). Default to Phase-22-as-one-PR unless a single plan's blast radius justifies its own PR. Each plan's commits must still be atomic.

### Folded Todos

None — the 3 pending todos in STATE.md (Phase 4 VSS vs Merkle, Phase 09 proxy E2E UAT, Phase 09 SC5 WFP TCP test) are pre-v2.2 carry-overs unrelated to Phase 22 scope.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 22 scope sources
- `.planning/ROADMAP.md` § Phase 22 — phase goal, locked plan sequencing, requirement mapping, success criteria
- `.planning/REQUIREMENTS.md` § PROF / POLY / PKG / OAUTH / AUD — 18 locked requirements with `What / Enforcement / Security / Acceptance / Maps to` shape
- `.planning/quick/260424-upr-review-upstream-037-to-040/SUMMARY.md` — authoritative 78-commit / ~9k-LOC upstream impact review (commit `4d61012d`); identifies merge-conflict surface, files-at-risk table, and recommended Phase 22 scope
- `.planning/quick/260424-mrg-merge-windows-squash-to-main/SUMMARY.md` — pre-milestone fast-forward state record; D-06 push timing rests on this artifact's path-C decision

### Project-level context
- `.planning/PROJECT.md` § Current Milestone: v2.2 — Core Value, target features, "Out of scope" deferral list, Key Decisions table
- `.planning/STATE.md` — current milestone position; v2.1 close-out state; carry-forward decisions through Phase 18.1, 19, 20, 21
- `.planning/PROJECT.md` § Constraints — Security ("fail secure on any unsupported shape"), Compatibility (Win10/11), Performance (zero startup latency)

### Coding & security standards (apply to every Phase 22 plan)
- `CLAUDE.md` § Coding Standards — no `.unwrap()`, DCO sign-off, `#[must_use]` on critical Results, env-var save/restore in tests
- `CLAUDE.md` § Security Considerations — fail-secure on any unsupported shape, path component comparison (not string ops), validate env vars before use, escape Seatbelt profile data. Directly relevant: REQ-PROF-03 `token_url: http://…` rejection (fail-closed); REQ-OAUTH-02 reverse-proxy IP class enforcement (fail-closed on unknown class); REQ-PKG-02 path-traversal rejection (`..`, symlinks, UNC aliasing).
- `CLAUDE.md` § Platform-Specific Notes — Linux Landlock allow-list constraint (`unsafe_macos_seatbelt_rules` is a no-op there per REQ-PROF-01); macOS Seatbelt DSL escaping (relevant only if upstream-ported macOS profile changes leak in, but Phase 22 is intentionally cross-platform-features-only).

### Pattern reference — prior phases that establish patterns Phase 22 inherits
- `.planning/phases/20-upstream-parity-sync/20-CONTEXT.md` — nearest analog (4-plan upstream-parity phase). D-01 / D-15 / D-17 / D-18 / D-20 / D-21 directly inform Phase 22's D-01..D-04, D-09..D-12, D-17..D-20.
- `.planning/phases/20-upstream-parity-sync/20-VERIFICATION.md` — phase-close verifier pass pattern Phase 22 should follow
- `.planning/phases/19-cleanup/19-CONTEXT.md` — 4-plan parallel-wave phase pattern; CLEAN-04 invariants documented (relevant to Plan 22-05 D-04)
- `.planning/phases/18.1-extended-ipc-gaps/18.1-CONTEXT.md` — TDD discipline + grep-invariant verification + drive-by/Rule-3 deviation handling
- `.planning/phases/21-windows-single-file-grants/21-CONTEXT.md` — Windows-only test pattern (`#[cfg(target_os = "windows")]`); D-15 Windows-specific tests inherit this

### Upstream source (git-resolvable from `upstream` remote at `https://github.com/always-further/nono.git`)
- Tag `v0.40.1` (`79154fe0`) — Phase 22 target landing version
- Tag `v0.40.0` (`eedc83d8`) — audit integrity + attestation cluster lands
- Tag `v0.39.0` (`6a284447`) — OAuth2 + `unsafe_macos_seatbelt_rules` lands
- Tag `v0.38.0` (`03d32203`) — Package manager + `claude-no-keychain` lands
- Tag `v0.37.1` — Phase 20 last-synced point

### Plan 22-01 (PROF) primary upstream commits
- `14c644ce` feat: add `unsafe_macos_seatbelt_rules` profile field
- `e3decf9d`, `ecd09313` — `unsafe_macos_seatbelt_rules` test/fmt follow-ups
- `088bdad7` feat(profile): introduce packs and command_args for profiles
- `115b5cfa` feat(profile): load profiles from registry packs
- `fbf5c06e` feat(config): OAuth2Config type
- `b1ecbc02` feat(profile): support OAuth2 auth in custom_credentials
- `3c8b6756` feat(claude): add no-keychain profile
- `713b2e0f` fix(policy): update tests and claude-no-kc for allow_file move

### Plan 22-02 (POLY) primary upstream commits
- `5c301e8d` refactor(policy): enforce stricter policy for overrides, rollback (the breaking-CLI commit; orphan `override_deny` fail-closed + `--rollback`/`--no-audit` clap conflict)
- `b83da813` feat(policy): filter profile override_deny entries without grants
- `930d82b4` fix(cli): skip non-existent profile deny overrides
- `49925bbf` fix(policy): move `.claude.lock` to allow_file
- `a524b1a7`, `7d1d9a0d` — supplementary policy adjustments

### Plan 22-03 (PKG) primary upstream commits
- `8b46573d` feat(cli): add package management commands
- `55fb42b8` feat(package): add install_dir artifact placement and hook unregistration
- `71d82cd0` feat(pack): introduce pack types and unify package naming
- `ec49a7af` fix(package): harden package installation security
- `9ebad89a` refactor(pkg): stream package artifact downloads
- `600ba4ec` refactor(package-cmd): centralize trust bundle
- `58b5a24e`, `0cbb7e62` — package refactors

### Plan 22-04 (OAUTH) primary upstream commits
- `9546c879` feat(proxy): implement OAuth2 client_credentials token exchange with cache (557 LOC new `oauth2.rs`)
- `0c7fb902`, `19a0731f`, `2244dd73` — OAuth2 rebase + 413 early-return fix
- `2bf5668f` feat(reverse-proxy): add http upstream support
- `0340ebff`, `b2a24402`, `0c990116` — HTTP upstream loopback-only gating
- `10bcd054` fix(network): keep `--allow-domain` in strict proxy-only mode
- `005579a9`, `d44e404e`, `60ad1eb3` — port/dry-run/test fixes around `--allow-domain`

### Plan 22-05 (AUD) primary upstream commits — strict chronological per D-03
- `4f9552ec` feat(audit): add tamper-evident audit log integrity (1,419+ / 226− across 21 files; `--audit-integrity`, hash-chain + Merkle root, `nono prune` → `nono session cleanup` deprecation, `nono audit cleanup`)
- `4ec61c29` feat(audit): capture pre/post merkle roots
- `02ee0bd1` feat(audit): record executable identity
- `7b7815f7` feat(audit): record exec identity and unify audit integrity
- `0b1822a9` feat(audit): add audit verify command
- `6ecade2e` feat(audit): add audit attestation
- `9db06336` feat(audit): refine audit path derivation

### Files at HIGH merge-conflict risk per upr-review (relevant to D-02 + D-20)
- `crates/nono-cli/src/profile/mod.rs` (upstream +635/-30; fork has v2.1 AIPC profile widening + RESL field)
- `crates/nono-cli/src/rollback_runtime.rs` (upstream +586; fork has v2.1 AppliedLabelsGuard snapshot+label lifecycle)
- `crates/nono-cli/src/policy.rs` (upstream +162; fork has v2.1 never_grant + group expansion + WSFG mode encoding)
- `crates/nono-cli/src/network_policy.rs` (upstream +171; fork has v2.0 Phase 9 WFP port-level)
- `crates/nono-cli/src/exec_strategy.rs` (upstream +144; fork has v2.0 Direct/Monitor/Supervised branching + AIPC wiring)
- `crates/nono-cli/src/profile/builtin.rs` (upstream +104; fork has Windows-specific builtin profile gating)
- `crates/nono-cli/src/sandbox_prepare.rs` (upstream +62; fork has v2.1 `--allow-gpu` 3-platform dispatch from Phase 20 UPST-04)
- `crates/nono-cli/src/supervised_runtime.rs` (upstream +42; fork has v2.1 SupervisedRuntimeContext.loaded_profile + AIPC allowlist)
- `crates/nono/src/undo/snapshot.rs`, `undo/types.rs` (upstream +149; fork has ObjectStore clone_or_copy + Merkle wiring on Windows)

### Windows regression-test sources (for D-18 gate)
- `crates/nono-cli/tests/wfp_port_integration.rs` — Windows WFP port-filter smoke (run with `-- --ignored` under admin + nono-wfp-service)
- `crates/nono-cli/tests/learn_windows_integration.rs` — ETW learn smoke
- `.planning/phases/15-detached-console-conpty-investigation/` — Phase 15 5-row detached-console smoke gate pattern
- `crates/nono/src/sandbox/windows.rs::tests` (76 tests post-Phase-21) — single-file grant + label semantics regression suite (relevant to D-04 if any audit-cluster commit touches `sandbox/windows.rs`, which would itself be a D-17 violation)

### Pre-milestone artifact (gates Phase 22)
- Quick task `260424-mrg-merge-windows-squash-to-main` — pre-milestone fast-forward COMPLETE locally (commit `1ef30c63`); D-06 says push to origin BEFORE Phase 22 starts. Path C deferrals (DCO remediation, windows-squash branch deletion, PR 555 disposition) remain deferred and do NOT gate Phase 22.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`upstream` git remote** — already configured (`https://github.com/always-further/nono.git`). `git fetch upstream` + `git log v0.37.1..v0.40.1` works today. No setup step needed.
- **`Upstream-commit:` trailer convention** — established in Phase 20 (commits `198270e`, `835c43f`, `540dca9`, `f377a3e`, `ec73a8a`, `af5c124`, etc.). Phase 22 commits inherit verbatim.
- **DCO commit hook on `main`** — enforces `Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>` on every commit. (46 historical commits pre-DCO-hook are deferred per quick task 260424-mrg path C.)
- **`make ci` target** — single entry point for the verification gate (D-18): `cargo build --workspace` + `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` + `cargo fmt --all -- --check` + `cargo test --workspace --all-features`. Was green at v2.1 close.
- **`nono::keystore::load_secret` / `keyring://` URI scheme** — already shipped in v2.1 Phase 20 UPST-03. REQ-PROF-03 (`oauth2.client_secret`) and REQ-AUD-02 (`--audit-sign-key keyring://nono/audit`) reuse this directly; no new keystore plumbing.
- **`nono::trust::signing::sign_statement_bundle` + `public_key_id_hex`** — already exists in fork from v2.1. REQ-AUD-02 (DSSE attestation) uses the existing trust-signing module; no Windows-specific code path.
- **`Profile::resolve_aipc_allowlist` end-to-end wiring** — landed in Phase 18.1 Plan 18.1-03 (commit `993cdcb`). Profile struct is now genuinely "loaded → flowed → enforced" on Windows. New PROF-* fields slot into this established plumbing.
- **`AppliedLabelsGuard` RAII pattern** — landed in Phase 21 (`crates/nono/src/sandbox/windows.rs`). REQ-AUD-05 acceptance #3 ("emissions survive AppliedLabelsGuard cleanup path") references this directly.
- **`current_logon_sid()` helper + `build_capability_pipe_sddl`** — landed in supervisor-pipe debug fix (commit `938887f`, 2026-04-20). Plan 22-05's audit-integrity Windows ledger may need this for write-path SDDL — verify during planning.

### Established Patterns
- **Atomic commits per semantic change with `Upstream-commit:` trailer** — Phase 20 precedent. D-19 inherits.
- **4–5-plan phase with multi-wave parallelization** — Phase 19 (4 plans Wave 1) and Phase 20 (1 sequential + 3 parallel) precedent. D-09..D-12 wave plan inherits.
- **Verifier agent closes the phase** — Phase 19 (`6597fbf`), Phase 20 (similar), Phase 21 closed via `/gsd-verify-phase`. Phase 22 follows the same close-out pattern.
- **`#[cfg(target_os = "windows")]` test gating** — Phase 21 WSFG pattern (76 tests under `sandbox::windows::tests`). D-15 inherits.
- **Drive-by Rule-3 deviation handling** — Phase 18 / 18.1 / 21 precedent: deviations land in the same commit with explicit rationale in commit body or plan SUMMARY.

### Integration Points
- **Plan 22-01 PROF-02 (`packs`) wiring** — `Profile` struct already routes through `PreparedSandbox → LaunchPlan → execute_sandboxed → SupervisedRuntimeContext` end-to-end (Phase 18.1 Plan 18.1-03). New `packs: Vec<PackRef>` field slots in via `#[serde(default)]`; pack resolution short-circuits on Windows if registry client unavailable per REQ-PROF-02.
- **Plan 22-01 PROF-03 (`oauth2`) wiring** — `custom_credentials.oauth2: OAuth2Config` deserializes via existing serde plumbing. `client_secret: keyring://...` resolves through `nono::keystore::load_secret` (already cross-platform).
- **Plan 22-01 PROF-04 (`claude-no-keychain` builtin)** — registers in `crates/nono-cli/data/policy.json` + `crates/nono-cli/src/profile/builtin.rs` via existing builtin-profile mechanism. Inherits `claude-code` `override_deny` entries — must pass POLY-01 audit (Plan 22-02) for orphan-override-deny check.
- **Plan 22-02 POLY-01 enforcement site** — `Profile::resolve` (`crates/nono-cli/src/profile/mod.rs`). Cross-platform check; new `NonoError::PolicyError { kind: OrphanOverrideDeny, ... }` variant; existing fork built-in profiles must pass the audit before this lands.
- **Plan 22-02 POLY-02 enforcement site** — clap layer, `crates/nono-cli/src/cli.rs`. `conflicts_with` attribute on `--rollback` and `--no-audit`. Windows rollback integration tests (`crates/nono-cli/tests/rollback*`) need updating to not pair the flags.
- **Plan 22-03 PKG Windows install_dir resolution** — `%LOCALAPPDATA%\nono\packages\<name>` via `dirs` crate or direct `windows-sys` `SHGetKnownFolderPath`. Long-path `\\?\` prefix handling per REQ-PKG-02 acceptance #2. Path-traversal validation via existing canonicalization.
- **Plan 22-03 PKG hook installer** — fork's existing `hooks.rs` Windows path; idempotent install per REQ-PKG-03 acceptance. (Upstream's `8b2a5ffb` "invoke bash via env" hooks change is N/A on Windows — D-17 violation if cherry-picked.)
- **Plan 22-04 OAUTH cross-platform code path** — `nono-proxy/src/oauth2.rs` is cross-platform by construction. Windows proxy-credentials path already handles new config shapes via Phase 9. Token cache memory-only with Zeroize; no disk persistence (avoids Low-IL label issues from WSFG-01..03).
- **Plan 22-05 AUD `--audit-integrity` flag wiring** — clap layer + `WindowsSupervisorRuntime` event emission. Phase 23 retrofits ledger emissions into the 5 AIPC `handle_*_request` functions in `exec_strategy_windows/supervisor.rs` (separate scope; AUD-05 retrofit).
- **Plan 22-05 AUD exec-identity Windows path** — `GetModuleFileNameW` for path resolution + `WinVerifyTrust` / `CryptCATAdminAcquireContext` for Authenticode signature query. SHA-256 fallback when unsigned. New Windows code path; D-17 ALLOWED here (this is the fork's planned addition, not an upstream commit touching `*_windows.rs`).
- **Plan 22-05 AUD `prune` → `session cleanup` rename** — touches `crates/nono-cli/src/cli.rs` + `crates/nono-cli/src/session_commands_windows.rs` + `auto_prune_if_needed` function. CLEAN-04 invariant gate per D-04. Hidden-alias deprecation in `cli.rs` clap subcommand definitions.

### Known Risks / "Be Careful Here"
- **`exec_strategy.rs` + `supervised_runtime.rs` + `rollback_runtime.rs` are the audit-cluster minefield** — fork drift is maximal here (Phase 21 `AppliedLabelsGuard`, Phase 18.1 `loaded_profile` threading, Phase 20 `--allow-gpu` dispatch all touched these). Plan 22-05 D-02 fallback rule will likely fire here. Read upstream's pre-/post-cluster diffs side-by-side before committing.
- **`profile/mod.rs` is shared between Plan 22-01 + Plan 22-02** — both wave-parallel plans touch this file. D-10 says wave-parallel; if both plans add fields/methods to the same struct, plan executors must coordinate via small atomic commits + rebase rather than long-running parallel branches.
- **`policy.json` is shared between Plan 22-01 PROF-04 and Plan 22-02 POLY-01/02/03** — claude-no-keychain registration + override_deny strictness + `.claude.lock` allow_file move all touch the same JSON file. Same coordination caveat as `profile/mod.rs`.
- **CLEAN-04 invariants are tested in `crates/nono-cli/tests/` not the binary itself** — D-04 verification needs `cargo test --test 'cleanup_*' --test 'prune_*'` patterns; planner identifies the exact test names during Plan 22-05 research.
- **Upstream `8b2a5ffb` (`fix(hooks): invoke bash via env`)** — N/A on Windows per upr-review SUMMARY; if a cherry-pick attempt picks it up, ABORT (D-17 violation candidate — not a `*_windows.rs` touch but a fork-vs-upstream hook strategy divergence).
- **`claude-code integration package` removal (`1d49246a`)** — upstream removed it; fork follows. If fork's `nono-cli/data/hooks/nono-hook.sh` references the removed package, plan execution must verify no dangling references before Plan 22-03 lands.
- **`87b20b6a` + `3b57b3b3` (`rustls-webpki` 0.103.12 → 0.103.13)** — upr-review marks "no RUSTSEC urgency vs v2.1's shipped 0.103.12". Likely cherry-pickable in 22-01 alongside Cargo.toml minor updates; not a milestone-critical bump. Planner discretion.

</code_context>

<specifics>
## Specific Ideas

- **Initial origin push command sequence** (D-06 + D-08):
  ```
  git fetch origin                            # confirm nothing raced in
  git log origin/main..main | head -5         # confirm local-ahead state
  git push origin main                        # 447 commits + DCO-missing 46 commits
  git push origin v2.0 v2.1                   # historical milestone tags
  git status                                  # confirm clean
  ```
  Run BEFORE invoking `/gsd-plan-phase 22`.

- **Per-plan close push pattern** (D-07):
  ```
  # After verifier signs off on a plan:
  git log --oneline origin/main..main
  git push origin main
  ```
  Plan SUMMARY documents the post-push origin/main SHA for traceability.

- **Plan 22-05 cherry-pick choreography** (D-01 + D-02 + D-03):
  ```
  for sha in 4f9552ec 4ec61c29 02ee0bd1 7b7815f7 0b1822a9 6ecade2e 9db06336; do
    git cherry-pick $sha
    if [ $? -ne 0 ]; then
      # D-02 soft fallback: planner judgment
      # 1. count conflict markers: grep -c '<<<<<<<' on each conflicted file
      # 2. count conflicted files: git diff --name-only --diff-filter=U | wc -l
      # 3. if >50 lines OR >2 files OR semantic ambiguity → git cherry-pick --abort + manual replay
      # 4. otherwise resolve in-place + git cherry-pick --continue
    fi
    # D-04 CLEAN-04 gate (only if commit touches prune/cleanup):
    cargo test --test '*' auto_prune_is_noop_when_sandboxed older_than_requires_suffix all_exited_escape_hatch
    # D-18 Windows-regression gate:
    cargo test --workspace --all-features
  done
  ```

- **D-02 fallback commit body template** (manual port):
  ```
  feat(22-0X): port <feature> from upstream <range>

  Read-upstream-and-replay over heavily-forked <file>. Cherry-pick aborted at
  <line-count> conflict markers across <file-list>; replayed semantically.

  Upstream-commit: <hash> (replayed manually)
  Upstream-tag: v0.40.1
  Upstream-author: <name> <email>
  Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
  ```

- **Plan 22-01 sub-task ordering** (planner discretion within plan, but recommended):
  1. Add `unsafe_macos_seatbelt_rules: Vec<String>` (PROF-01) — smallest deserialize-only change; canary for cross-platform parse
  2. Add `packs: Vec<PackRef>` + `command_args: Vec<String>` (PROF-02) — depends on PackRef type from `71d82cd0`
  3. Add `custom_credentials.oauth2: Option<OAuth2Config>` (PROF-03) — requires OAuth2Config from `fbf5c06e`
  4. Register `claude-no-keychain` builtin (PROF-04) — depends on policy.json shape post-`713b2e0f`
  5. Verifier pass on full Plan 22-01 → unblocks Plan 22-03 / 22-04 wave (D-09)

- **Test fixture porting commit pattern** (D-13 + D-14):
  Each test fixture port is its own commit alongside the production code commit it covers. Example for Plan 22-04:
  ```
  feat(22-04): port OAuth2 client_credentials proxy from upstream
  feat(22-04): port OAuth2 token-exchange test fixture from upstream
  feat(22-04): add Windows-specific keyring:// resolution test for OAUTH-01
  ```

- **STOP triggers for Phase 22** (escalate before proceeding; inherits Phase 20 STOP triggers + adds new ones):
  1. A cherry-pick or manual port touches a `*_windows.rs` file or any `target_os = "windows"` block → D-17 violation. (Exception: AUD-05 in Phase 23 + new audit Authenticode code in Plan 22-05's exec-identity site, which is a fork-internal addition not an upstream port.)
  2. `make ci` goes red on a plan and root cause isn't obvious within ~30 minutes → re-discuss before patching around it.
  3. A manual port diff exceeds ~400 lines → consider whether the plan should split (Plan 22-05 is the most likely candidate).
  4. Phase 15 5-row smoke gate fails on any plan → the port regressed Windows behavior; revert and re-scope.
  5. CLEAN-04 invariant test fails after a Plan 22-05 commit → the rename or audit-cluster commit broke v2.1 Phase 19 contract; revert that commit and re-discuss.
  6. `auto_prune_is_noop_when_sandboxed` test fails post-rename — sandboxed agent file-deletion vector reopened; ABSOLUTE STOP.
  7. Plans 22-01 / 22-02 wave-parallel produces conflicting commits on `profile/mod.rs` or `policy.json` → coordinate inline or sequentialize for that surface; do not let parallel plans race on shared files.
  8. Cherry-pick lands `8b2a5ffb` (`fix(hooks): invoke bash via env`) — N/A for Windows hook strategy; ABORT.
  9. Cherry-pick lands `1d49246a` (`claude-code integration package` removal) but fork still references the removed surface — investigate before committing.

</specifics>

<deferred>
## Deferred Ideas

These came up during scope discussion or were intentionally split out of Phase 22:

### Phase 23 candidates (default home)
- **AUD-05 — Windows AIPC broker audit-event emissions** (REQ-AUD-05). Wires ledger emissions into the 5 `handle_*_request` paths (File, Socket, Pipe, JobObject, Event, Mutex) in `exec_strategy_windows/supervisor.rs`. Default home is Phase 23; may collapse into Plan 22-05 if upstream's ledger event shape covers AIPC HandleKinds cleanly with no Windows-specific surface (decision-point during 22-05 execution per Claude's Discretion).

### Phase 24 candidates (default home; sequence after Phase 22 ships per D-11)
- **DRIFT-01 — `scripts/check-upstream-drift` tooling.** PowerShell + Bash twin reporting commits in `upstream/main..HEAD` touching cross-platform files.
- **DRIFT-02 — GSD upstream-sync quick-task template.** `.planning/templates/upstream-sync-quick.md` scaffolding; PROJECT.md reference.

### v2.3+ (deferred per PROJECT.md "Out of Scope")
- **Upstream v0.41+ ingestion** — first quick task of v2.3, using DRIFT-02 template.
- **WR-01 reject-stage unification** — AIPC HandleKinds BEFORE/AFTER prompt alignment (Windows-internal consistency, not Windows-vs-macOS gap).
- **AIPC G-04 wire-protocol compile-time tightening** — `Approved(ResourceGrant)` inline at the wire type.
- **Cross-platform RESL Unix backends** — cgroup v2 / rlimit ports of Windows Job Object caps.
- **WR-02 EDR HUMAN-UAT** — v3.0 (no EDR-instrumented runner).
- **`prune` → `session cleanup` alias deprecation timeline** — REQ-AUD-04 says hidden alias survives one release; v2.3 milestone scoping decides force-fail point (v2.3? v2.4? longer?).

### Phase 22 internal decisions deferred to planner discretion (Claude's Discretion section above)
- AUD-05 fold-or-split decision-point during 22-05 execution
- Audit signing key provisioning model on Windows (auto-gen vs pre-provision)
- Authenticode fallback path shape for REQ-AUD-03
- Per-plan PR vs single Phase-22 PR
- Exact `make ci` invocation strings per plan

### Reviewed Todos (not folded)
- **Discuss Phase 4 filesystem strategy (VSS vs Merkle Trees)** — pre-v2.0 carry-over; settled by v2.1 Phase 21 WSFG work; does not affect Phase 22.
- **Phase 09 human verification: proxy E2E** — UAT-only item; not a Phase 22 implementation gap.
- **Phase 09 human verification: SC5 WFP TCP test** — UAT-only item; not a Phase 22 implementation gap.

### Pre-milestone deferred items (from quick task 260424-mrg path C; do NOT gate Phase 22)
- DCO signoff remediation on the 46 historical commits — only when opening a PR to upstream `always-further/nono`.
- Delete local `windows-squash` branch — defer until origin/main is pushed AND no other work references the branch.
- PR 555 disposition on upstream — separate question; `gh` CLI now available per memory (verify on github.com when ready).

</deferred>

---

*Phase: 22-upst2-upstream-v038-v040-parity-sync*
*Context gathered: 2026-04-27*
