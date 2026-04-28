---
phase: 22-upst2-upstream-v038-v040-parity-sync
plan: 03
status: in-progress (6/8 cherry-picks landed; 2 deferred to follow-up plan)
session_date: 2026-04-27
---

# Plan 22-03 PKG — Progress Note

This file is a session note documenting deviations from `22-03-PKG-PLAN.md`.
It exists because the plan's cherry-pick chain table (lines 75-84) had ordering
issues and two cherry-picks turned out to require fork-aware adaptation that
exceeded this session's scope.

## Plan ordering correction

The plan listed cherry-picks #5/#6/#7/#8 in the wrong order. Real chronological
order per D-03 (by author date):

| Plan # | SHA | Date | Subject |
|--------|-----|------|---------|
| 5 (was) | `9ebad89a` | 2026-04-15 21:47 | stream package artifact downloads |
| 6 (was) | `600ba4ec` | 2026-04-06 21:42 | centralize trust bundle |
| 7 (was) | `58b5a24e` | 2026-04-05 17:05 | path validation |
| 8 (was) | `0cbb7e62` | 2026-04-15 08:21 | simplify signer validation |

**Correct chronological order (after #1-#4 already landed):**

| New # | SHA | Date | Subject | Status this session |
|-------|-----|------|---------|---------------------|
| 5 | `58b5a24e` | 2026-04-05 | path validation | **Deferred** (see below) |
| 6 | `600ba4ec` | 2026-04-06 | centralize trust bundle | Landed as `73e1e3b8` |
| 7 | `0cbb7e62` | 2026-04-15 AM | simplify signer validation | Landed as `adf81aec` |
| 8 | `9ebad89a` | 2026-04-15 PM | streaming downloads | **Deferred** (see below) |

The plan's stated ordering implied `9ebad89a` came before `600ba4ec`, but
`9ebad89a` was authored 9 days *after* `600ba4ec` and depends on it.
Attempting to apply in plan-listed order produced a 6-conflict mess in
`package_cmd.rs` because `9ebad89a` assumes `58b5a24e` (Plugin arm),
`600ba4ec` (centralized bundle), and `0cbb7e62` (signer cleanup) are all
already applied.

## Deferred cherry-picks and rationale

### Deferred: `58b5a24e` "Improve artifact path validation"

**Material change:** Replaces `validate_path_within(base, full)` with a new
`validate_relative_path(path)` helper that pre-validates input strings;
also touches the `ArtifactType::Plugin` arm.

**Fork divergences blocking verbatim apply:**

1. **`ArtifactType::Plugin` does not exist in fork.** Documented as a
   deliberate deferred divergence at `crates/nono-cli/src/package_cmd.rs:631-643`
   ("upstream ec49a7af also adds an ArtifactType::Plugin arm here"). The
   Plugin variant was introduced by an upstream commit not in plan 22-03's
   chain.

2. **Fork's `validate_path_within` is defense-in-depth.** Prior commit
   `869349df` (PKG-02/04 hardening, just landed) explicitly added
   `validate_path_within(staging_root, &store_path)` calls *after every
   artifact-write arm* with the rationale: "Catches future bugs where a new
   arm joins an attacker-controlled component without canonicalize." Dropping
   it in favor of upstream's input-string `validate_relative_path` would be
   a security regression for the fork's stricter path-handling stance
   (CLAUDE.md § Path Handling).

**Net effect of skipping:** None — the function `validate_relative_path`
isn't needed by anything else in the fork yet, and the Plugin arm work is
already a deferred divergence. The fork keeps its stricter
`validate_path_within` defense-in-depth.

### Deferred: `9ebad89a` "Stream package artifact downloads"

**Material change:** Large refactor — replaces `bytes: Vec<u8>` with
`path: PathBuf` on `DownloadedArtifact`, introduces a `VerifiedDownloads`
wrapper holding a `tempfile::TempDir`, restructures download/install around
streaming-to-disk, adds `download_artifact_to_path` / `download_bundle`
methods to `RegistryClient`, adds size limits and HTTP timeouts, and adds
the `semver` crate dep.

**Why deferred:**

1. **Depends on `ArtifactType::Plugin`** (same fork divergence as #5).
   The streaming refactor's diff includes a Plugin arm rewrite.
2. **Depends on `validate_relative_path`** helper from #5 (which we skipped).
3. **Is the largest commit in the chain** (~+267 / -109 LOC across 5 files).
   Hand-merging it under cherry-pick pressure is the highest-risk option.
4. **Touches the same security surface as #5's deferred work** — fork's
   `validate_path_within` defense-in-depth needs to be reconciled with
   upstream's removal as a coordinated decision, not a hand-merge.

**Recommended follow-up plan:** Add a new plan (`22-XX-PKG-STREAMING.md`
or a v2.3 plan) that:

1. Introduces `ArtifactType::Plugin` enum variant + plumbing first
   (closes the deferred divergence comment at `package_cmd.rs:631-643`).
2. Decides explicitly whether the fork keeps `validate_path_within` as
   defense-in-depth alongside upstream's `validate_relative_path`, or
   adopts upstream's pattern verbatim. (Recommendation: keep both for
   defense-in-depth — fork's stance is stricter and matches CLAUDE.md.)
3. Cherry-picks `58b5a24e` (path validation) with `validate_path_within`
   retained as belt-and-suspenders.
4. Cherry-picks `9ebad89a` (streaming) with the streaming + tempdir +
   semver machinery, plus the `bundle_json` field on `DownloadedArtifact`
   that this Plan-22-03 work skipped (commit `73e1e3b8` notes this).
5. Cherry-picks `115b5cfa` (load_registry_profile auto-pull) which #7's
   formatting hunks targeted but isn't yet in fork.

## What this session delivered

- Cherry-pick #6 `600ba4ec` → `73e1e3b8 refactor(22-03): centralize trust bundle`
  - Adopts single-bundle-per-version verification pattern.
  - Wraps fork's async `load_production_trusted_root` in
    `tokio::runtime::Builder::new_current_thread().build().block_on()`.
  - Defers `bundle_json` field on `DownloadedArtifact` to follow-up plan.
- Cherry-pick #7 `0cbb7e62` → `adf81aec refactor(22-03): simplify artifact signer validation`
  - Removes `enforce_signer_consistency` + `same_signer` (dead-code from #6
    cleanup). Phantom formatting conflicts in `command_runtime.rs`,
    `profile/mod.rs`, `profile_runtime.rs` resolved by taking HEAD
    (formatting targets functions that don't exist in fork).

## Verification

- `cargo build`: clean
- `cargo check -p nono-cli`: clean (no warnings)
- `cargo test --workspace --all-features`: 651 passed, 2 failed
  - Both failures (`trust::bundle::tests::load_production_trusted_root_succeeds`
    and `verify_bundle_with_invalid_digest`) are **pre-existing on
    `869349df`** — TUF root signature freshness / network-dependent test
    issues unrelated to this session's cherry-picks. Confirmed by checking
    out prior HEAD and re-running both tests independently — both fail
    identically there.
- `cargo clippy -D warnings`: pre-existing errors in
  `crates/nono/src/manifest.rs:95` and `:103` (collapsible_match) —
  also confirmed pre-existing on `869349df`. Not a regression from
  this session's work.

D-18 ("`cargo test --workspace --all-features` exits 0 on Windows after
each commit") is satisfied for the *cherry-picks themselves* (no new
failures introduced); the pre-existing TUF and clippy issues are out of
scope for this plan and should be tracked separately.

## Cherry-pick chain status (after this session)

| # | SHA | Status | Fork commit |
|---|-----|--------|-------------|
| 1 | `8b46573d` | landed | `51534ad3` |
| 2 | `55fb42b8` | landed | `32a475cb` |
| 3 | `71d82cd0` | landed | `62de4e8b` |
| 4 | `ec49a7af` | landed | `869349df` |
| 5 | `58b5a24e` | **deferred** | (follow-up plan) |
| 6 | `600ba4ec` | landed | `73e1e3b8` |
| 7 | `0cbb7e62` | landed | `adf81aec` |
| 8 | `9ebad89a` | **deferred** | (follow-up plan) |

Plan 22-03 is **partially complete** — PKG-01 (commands), PKG-02 (security
hardening), PKG-03 (hooks), and the trust-bundle portion of PKG-04 are
landed. The streaming-download portion of PKG-04 and the Plugin-arm
adaptation depend on the deferred follow-up plan.
