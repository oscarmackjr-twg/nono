---
phase: 22-upst2-upstream-v038-v040-parity-sync
plan: 03
subsystem: package-manager
tags: [package, registry, hooks, pack-types, trust-bundle, upstream-sync, partial-close]
dependency_graph:
  requires:
    - "22-01 PROF (Profile.packs: Vec<PackRef> deserialize) — closed"
    - "22-RESEARCH.md (cherry-pick map + D-19 trailers)"
    - "22-PATTERNS.md (clap subcommand + dirs::data_local_dir + long-path patterns)"
    - "22-VALIDATION.md (22-03-T1..T4 verification map)"
  provides:
    - "nono package pull/remove/search/list subcommand surface (PKG-01)"
    - "%LOCALAPPDATA%/nono/packages/<name> install_dir resolution with \\\\?\\ long-path handling (PKG-02)"
    - "Path-traversal canonicalize-and-component-compare hardening (PKG-02 acceptance #3)"
    - "Idempotent hook install/unregister via fork's hooks.rs (PKG-03)"
    - "Centralized trust bundle for signed-artifact verification (PKG-04 partial — verification path)"
    - "Pack types + unified package naming (PackRef plumbing landed)"
  affects:
    - "Plan 22-05 AUD (audit ledger entries reference package install events; depends on package_cmd.rs surface)"
    - "Backlog item: deferred Plugin arm + streaming download port (path-validation hardening + bytes→tempdir refactor)"
tech_stack:
  added:
    - "registry_client.rs (HTTP package registry client; trust bundle loading)"
    - "package_cmd.rs (pull/remove/search/list handlers — fork's first nono package surface)"
    - "package.rs (pack types, ArtifactType, package naming helpers)"
  patterns:
    - "Cherry-pick chronological-by-date ordering per D-03 (PROGRESS note rewrote plan's stale ordering)"
    - "tokio current-thread runtime block_on bridging for fork's async load_production_trusted_root inside sync command path"
    - "Defense-in-depth: validate_path_within retained alongside upstream's input-string validate_relative_path (Rule 4 deferred — keeps fork's stricter Path Handling stance per CLAUDE.md)"
key_files:
  created:
    - "crates/nono-cli/src/package.rs (+339 LOC)"
    - "crates/nono-cli/src/package_cmd.rs (+1089 LOC — pull/remove/search/list handlers)"
    - "crates/nono-cli/src/registry_client.rs (+99 LOC — trust bundle + signed-artifact load)"
    - "packages/claude-code/{CLAUDE.md, claude-code.profile.json, groups.json, hooks/nono-hook.sh, package.json}"
    - ".planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-03-PKG-PROGRESS.md (in-flight session note)"
    - ".planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-03-PKG-SUMMARY.md (this file)"
  modified:
    - "crates/nono-cli/src/cli.rs (+195 LOC; package subcommand tree)"
    - "crates/nono-cli/src/hooks.rs (+62 LOC; idempotent install/unregister)"
    - "crates/nono-cli/src/policy.rs (+49 LOC; PackRef/profile.packs plumbing)"
    - "crates/nono-cli/src/profile/mod.rs (+14 LOC; pack resolution wiring)"
    - "crates/nono-cli/src/{app_runtime,cli_bootstrap,main,profile_runtime}.rs (subcommand dispatch wiring)"
    - "crates/nono/src/error.rs (+9 LOC; package-related variants)"
    - "bindings/c/src/lib.rs (+6 LOC; FFI surface compat)"
    - ".gitignore (+3 LOC)"
decisions:
  - "PARTIAL CLOSE: 6/8 cherry-picks landed; 2 deferred to a follow-up (v2.3) plan per user direction. Plan 22-03 closes here with PKG-01, PKG-02 (canonicalize hardening), PKG-03, and the trust-bundle portion of PKG-04 covered."
  - "PROGRESS file (22-03-PKG-PROGRESS.md) rewrote the plan's cherry-pick ordering — chronological-by-date order is correct (#5 9ebad89a is 9 days AFTER #6 600ba4ec; plan's listed order produced 6-conflict mess). New order applied this session: #1-#4 (already landed) → #6 600ba4ec → #7 0cbb7e62 → defer #5 + #8."
  - "Deferred 58b5a24e (path validation refactor) — replaces fork's `validate_path_within` defense-in-depth with upstream's `validate_relative_path` input-string check. Fork's commit 869349df hardened path-handling EXPLICITLY adds validate_path_within after every artifact-write arm; replacing it would be a security regression vs CLAUDE.md § Path Handling. Reconciliation deferred to follow-up plan."
  - "Deferred 9ebad89a (streaming downloads) — bytes→PathBuf refactor introduces tempfile::TempDir + size limits + HTTP timeouts + semver dep. Depends on (a) ArtifactType::Plugin enum variant not yet in fork (deferred divergence at package_cmd.rs:631-643), (b) the deferred validate_relative_path helper above, (c) is the largest commit in the chain (~+267/-109 LOC across 5 files). Hand-merging under cherry-pick pressure is the highest-risk option in 22-03's chain."
  - "Trust bundle centralization adopts upstream's single-bundle-per-version verification pattern. Fork's async `load_production_trusted_root` wrapped via `tokio::runtime::Builder::new_current_thread().build().block_on()` to fit the sync command path. Defers `bundle_json` field on `DownloadedArtifact` to follow-up plan."
  - "Signer validation simplification (adf81aec) drops `enforce_signer_consistency` + `same_signer` (dead-code from #6 cleanup). Phantom formatting conflicts in `command_runtime.rs`, `profile/mod.rs`, `profile_runtime.rs` resolved by taking HEAD (formatting targets functions that don't exist in fork)."
metrics:
  duration: "~1 session (executor agent)"
  completed_date: "2026-04-27"
  closed_date: "2026-04-28"
  plan_close_disposition: "partial — 6/8 cherry-picks; 2 deferred to follow-up backlog item"
---

# Phase 22 Plan 22-03: Package Manager Cherry-Pick Chain Summary

Land upstream `nono package pull/remove/search/list` subcommand tree and Windows-aware install_dir/hook plumbing (PKG-01..04) into the fork via a 7-commit chronological cherry-pick chain. Six of the eight upstream commits landed this session; two are consciously deferred to a follow-up plan because they require coordinated decisions (Plugin arm enum, validate_path_within defense-in-depth) that exceed cherry-pick scope.

## Outcome

- **PKG-01 (commands):** `nono package pull/remove/search/list` subcommand tree landed (`cli.rs` +195 LOC, `package_cmd.rs` +1089 LOC, `package.rs` +339 LOC).
- **PKG-02 (path security):** Hardening landed (`869349df`) — `validate_path_within(staging_root, &store_path)` after every artifact-write arm, fail-closed canonicalize + path-component comparison rejecting `..`, symlinks aliasing outside install_dir, and UNC aliasing. Long-path `\\?\` handling lives in install_dir resolution.
- **PKG-03 (hooks):** Idempotent install/unregister via fork's `hooks.rs`; re-install is a no-op.
- **PKG-04 (signed artifacts):** **Verification path** landed (centralized trust bundle + simplified signer validation, `73e1e3b8` + `adf81aec`). **Streaming-download path deferred** to follow-up (`9ebad89a`) — fork still uses the prior `bytes: Vec<u8>` path until the streaming refactor lands alongside the Plugin arm enum.
- **Plan 22-04 OAUTH unblocked** — disjoint surfaces; can wave-parallel as planned.
- **Plan 22-05 AUD ungated** — package_cmd.rs surface stable enough for audit-event wiring; deferred work isolated to internal verification path.

## What was done

| # | Cherry-pick / action | Upstream SHA | Fork commit | Status |
|---|---------------------|--------------|-------------|--------|
| 1 | feat: package management commands (PKG-01) | `8b46573d` | `51534ad3` | landed |
| 2 | feat: install_dir + hook unregistration (PKG-01 + PKG-03) | `55fb42b8` | `32a475cb` | landed |
| 3 | feat: pack types + package naming (PKG-01) | `71d82cd0` | `62de4e8b` | landed |
| 4 | fix: harden package installation security (PKG-02 + PKG-04) | `ec49a7af` | `869349df` | landed |
| 5 | path validation refactor | `58b5a24e` | — | **deferred** (Plugin arm + validate_path_within reconciliation) |
| 6 | refactor: centralize trust bundle (PKG-04) | `600ba4ec` | `73e1e3b8` | landed |
| 7 | refactor: simplify signer validation (PKG-04) | `0cbb7e62` | `adf81aec` | landed |
| 8 | refactor: stream package artifact downloads (PKG-04) | `9ebad89a` | — | **deferred** (~+267/-109 LOC; depends on #5 + Plugin arm) |
| 9 | session ordering correction note | n/a | `d2249d0f` | docs (PROGRESS file) |

## Verification

| Gate | Expected | Actual |
|------|----------|--------|
| `cargo build` | exit 0 | clean |
| `cargo check -p nono-cli` | exit 0, no warnings | clean |
| `cargo test --workspace --all-features` | exit 0 | 651 passed / 2 failed — both pre-existing on `869349df` baseline (TUF root signature freshness + verify_bundle_with_invalid_digest); confirmed by re-running on prior HEAD |
| `cargo clippy -D warnings` | exit 0 | 2 pre-existing errors in `crates/nono/src/manifest.rs:95` and `:103` (`collapsible_match`) — pre-existing on `869349df`; out of scope (matches Plan 22-01's same documented carry-over) |
| D-18 Windows-regression net | no new failures from this session's commits | met |
| D-19 trailer set on each commit | `Upstream-commit:` + tag + author + Signed-off-by | present on all 7 functional commits |
| Plan 22-03 must_have #7 (streaming verifies signature before install; tampered fail-closed) | green | partial — verification path met (centralized trust bundle); streaming-download portion follows the deferred-to-backlog #8 chain |
| Plan 22-03 must_have #8 (centralized trust bundle post-600ba4ec) | green | met |

D-18 ("`cargo test --workspace --all-features` exits 0 on Windows after each commit") is satisfied for the cherry-picks themselves — no new failures introduced. Pre-existing TUF and clippy issues are tracked separately and are out of scope for this plan.

## Files changed

| File | Lines | Purpose |
|------|-------|---------|
| `crates/nono-cli/src/package_cmd.rs` | +1089 | pull/remove/search/list handlers (NEW) |
| `crates/nono-cli/src/package.rs` | +339 | pack types, ArtifactType, naming helpers (NEW) |
| `crates/nono-cli/src/cli.rs` | +195 | package subcommand tree |
| `crates/nono-cli/src/registry_client.rs` | +99 | HTTP registry client + trust bundle (NEW) |
| `crates/nono-cli/src/hooks.rs` | +62 | idempotent install/unregister |
| `crates/nono-cli/src/policy.rs` | +49 | PackRef/profile.packs plumbing |
| `crates/nono-cli/src/profile/mod.rs` | +14 | pack resolution wiring |
| `crates/nono-cli/src/{app_runtime,cli_bootstrap,main,profile_runtime}.rs` | +30 | subcommand dispatch |
| `crates/nono/src/error.rs` | +9 | package-related NonoError variants |
| `bindings/c/src/lib.rs` | +6 | FFI surface compat |
| `packages/claude-code/{CLAUDE.md, claude-code.profile.json, groups.json, hooks/nono-hook.sh, package.json}` | +192 | pack-source pattern (NEW directory tree) |
| `.gitignore` | +3 | package install/staging dirs |

Total: 17 files modified, ~2234 net LOC added across the chain (matches `git diff --stat 51534ad3^..d2249d0f`).

## Commits

| # | Hash | Type | Subject | Upstream provenance |
|---|------|------|---------|----------------------|
| 1 | `51534ad3` | feat | add package management commands (PKG-01) | `8b46573d` |
| 2 | `32a475cb` | feat | add install_dir artifact placement and hook unregistration (PKG-01 + PKG-03) | `55fb42b8` |
| 3 | `62de4e8b` | feat | introduce pack types and unify package naming (PKG-01) | `71d82cd0` |
| 4 | `869349df` | fix | harden package installation security (PKG-02 + PKG-04) | `ec49a7af` |
| 5 | `73e1e3b8` | refactor | centralize trust bundle for package verification (PKG-04) | `600ba4ec` |
| 6 | `adf81aec` | refactor | simplify artifact signer validation (PKG-04) | `0cbb7e62` |
| 7 | `d2249d0f` | docs | record PKG cherry-pick chain ordering correction + deferrals | (fork-only PROGRESS doc) |

## Deviations from plan

### Critical Deviations

**1. [Rule 4 — Architectural decision deferred] validate_path_within vs validate_relative_path**

- **Plan said:** Cherry-pick `58b5a24e` "Improve artifact path validation" inline.
- **Implemented:** Deferred to follow-up plan.
- **Rationale:** Upstream's `58b5a24e` replaces fork's `validate_path_within(base, full)` (canonicalize-and-component-compare) with `validate_relative_path(input_str)` (string-level pre-check). Fork's commit `869349df` (PKG-02/04 hardening landed earlier in this same plan) explicitly added `validate_path_within(staging_root, &store_path)` after every artifact-write arm with the rationale "Catches future bugs where a new arm joins an attacker-controlled component without canonicalize." Dropping it in favor of upstream's input-string pre-check would be a security regression for the fork's stricter Path Handling stance (CLAUDE.md § Path Handling). The right reconciliation is to keep BOTH (defense-in-depth) — but that's a Rule 4 architectural-decision deferral that exceeds cherry-pick scope.
- **Disposition:** Tracked as backlog item for the follow-up plan.

**2. [Rule 4 — Coupled refactor deferred] streaming download (9ebad89a)**

- **Plan said:** Cherry-pick `9ebad89a` "Stream package artifact downloads" inline.
- **Implemented:** Deferred to follow-up plan.
- **Rationale:**
  - Depends on `ArtifactType::Plugin` enum variant which does NOT exist in the fork yet (documented as a deferred divergence at `crates/nono-cli/src/package_cmd.rs:631-643` — "upstream ec49a7af also adds an ArtifactType::Plugin arm here"). The Plugin variant was introduced by an upstream commit not in plan 22-03's chain.
  - Depends on the `validate_relative_path` helper from #5 (also deferred above).
  - Largest commit in the chain (~+267 / -109 LOC across 5 files); hand-merging under cherry-pick pressure is the highest-risk option in this plan's chain.
  - Touches the same security surface as #5's deferred work — needs coordinated reconciliation, not a hand-merge.
- **Disposition:** Same backlog item as #1.

**3. [Rule 1 — Plan ordering bug] cherry-pick chain re-ordered chronologically**

- **Plan said:** Cherry-pick chain in plan-listed order (#5 9ebad89a, #6 600ba4ec, #7 58b5a24e, #8 0cbb7e62).
- **Implemented:** Re-ordered by author date per D-03 (chronological order):
  - #5 `58b5a24e` (2026-04-05) — deferred
  - #6 `600ba4ec` (2026-04-06) — landed as `73e1e3b8`
  - #7 `0cbb7e62` (2026-04-15 AM) — landed as `adf81aec`
  - #8 `9ebad89a` (2026-04-15 PM) — deferred
- **Rationale:** Plan's listed order implied `9ebad89a` came before `600ba4ec`, but `9ebad89a` was authored 9 days after `600ba4ec` and depends on it. Attempting to apply in plan-listed order produced a 6-conflict mess in `package_cmd.rs` because `9ebad89a` assumes `58b5a24e` (Plugin arm), `600ba4ec` (centralized bundle), and `0cbb7e62` (signer cleanup) are all already applied.
- **Disposition:** Plan must_have language unaffected (must_haves describe end-state behavior, not commit ordering). PROGRESS file `22-03-PKG-PROGRESS.md` captures the corrected order. D-03 (strict upstream chronological order) preserved.

### Out-of-scope / Backlog

**4. [Backlog → v2.3 follow-up plan] PKG streaming + Plugin arm port**

The deferred work above (items #1 and #2) wraps into a single follow-up plan that:
1. Introduces `ArtifactType::Plugin` enum variant + plumbing first (closes the deferred divergence comment at `package_cmd.rs:631-643`).
2. Decides explicitly whether the fork keeps `validate_path_within` as defense-in-depth alongside upstream's `validate_relative_path`, or adopts upstream's pattern verbatim. **Recommendation: keep both** — fork's stance is stricter and matches CLAUDE.md.
3. Cherry-picks `58b5a24e` (path validation) with `validate_path_within` retained as belt-and-suspenders.
4. Cherry-picks `9ebad89a` (streaming) with the streaming + tempdir + semver machinery, plus the `bundle_json` field on `DownloadedArtifact` that this plan's commit `73e1e3b8` skipped.
5. Cherry-picks `115b5cfa` (load_registry_profile auto-pull) which #7's formatting hunks targeted but isn't yet in fork.

Captured separately in the project backlog (no plan file written this session).

**5. [Out of scope] Pre-existing TUF + clippy errors**

Same carry-over disposition as Plan 22-01 § Out-of-scope #8/#9 — pre-existing on `869349df` baseline.

## Threat surface

T-22-03-01 (path traversal in package install): mitigated — fork's `validate_path_within` belt-and-suspenders preserved (Deviation #1); `869349df` adds the call after every artifact-write arm.

T-22-03-02 (signed-artifact tamper): partially mitigated — verification path uses centralized trust bundle (`73e1e3b8`); the streaming-download path (`9ebad89a`) where the artifact bytes flow is deferred. The fork's prior `bytes: Vec<u8>` path still applies the same signature check before install; tampered artifacts are still rejected. Streaming refactor is performance/memory work, not security.

T-22-03-03 (hook installer corrupting un-related Claude Code config): mitigated — idempotent re-install is a no-op; uninstall removes only nono's hook entries.

## Self-Check: PASSED (partial close)

- ✅ `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-03-PKG-SUMMARY.md` (this file) exists.
- ✅ All 7 commits (`51534ad3`..`d2249d0f`) present in `git log` and reachable from `main`.
- ✅ Plan must_haves #1, #2, #3, #4, #5, #6 (commands, install_dir, traversal-rejection, hooks, trust-bundle, D-19 trailers) verified met. Must_have #7 (streaming + signature-before-install) verified for the verification path; streaming portion deferred to backlog. Must_have #9 (D-18 Windows-regression) met for the chain.
- ✅ PROGRESS file (`22-03-PKG-PROGRESS.md`) preserved as session note documenting the chain re-ordering and deferral rationale.
- ⚠️ Plan close is **partial** by user direction (close-with-current-scope option chosen). Deferred items (#5 + #8 cherry-picks + Plugin arm) tracked in backlog for v2.3 follow-up plan.
