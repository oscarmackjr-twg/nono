---
phase: 26
slug: pkg-streaming-followup
status: partial
nyquist_compliant: partial
wave_0_complete: true
created: 2026-05-09
---

# Phase 26 — Validation Strategy

> Retroactive Nyquist audit. Phase 26 ships in two plans:
> - **Plan 26-01** (REQ-PKGS-02 + REQ-PKGS-03) — executed 2026-04-29 / closed 2026-05-01. Nyquist audit complete; 1 gap filled this audit.
> - **Plan 26-02** (REQ-PKGS-01 + REQ-PKGS-04) — executed 2026-05-09 on a Windows host under a "portable subset" directive (commits `9cb7770f..18eb3913`). 11 new unit tests covering streaming + size-cap + timeout + TempDir-cleanup invariants; auto-pull e2e tests deferred per host_blocker (require Sigstore-signed fixture packs + `run_nono` harness). Three plan-text deviations were accepted during merge — see "Plan 26-02 Acceptance Notes" below.
>
> Phase 26 is recorded as **PARTIAL** at the milestone level: production-code surface is fully landed for both plans, but auto-pull e2e coverage (REQ-PKGS-04 acceptance #1–#3) remains deferred to a Linux/macOS host pass. Will flip to `compliant` after a follow-up validation closes those e2e tests.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust built-in) |
| **Config file** | `crates/nono-cli/Cargo.toml` (test target gated by `[[bin]]` — nono-cli has NO library target) |
| **Test target flag** | `--bin nono` (required; `--lib` fails with "no library targets found") |
| **Quick run command** | `cargo test -p nono-cli --bin nono -- <test_name>` |
| **Full suite command** | `cargo test --workspace` |
| **Plan 26-01 module** | `crates/nono-cli/src/package_cmd.rs::tests` (line 1153) and `crates/nono-cli/src/package.rs::tests` (line 323) |
| **Estimated runtime** | ~6s targeted; ~3 min workspace-wide |

---

## Sampling Rate

- **After every task commit:** `cargo build --workspace` + targeted test for the touched requirement.
- **After every plan wave:** `cargo test -p nono-cli --bin nono` (full nono-cli surface, ~836 tests + new additions).
- **Before `/gsd-verify-work`:** `make ci` must be green modulo documented carry-overs (2 pre-existing `nono::manifest` `collapsible_match` clippy errors and 2 pre-existing TUF integration failures from `869349df` baseline; carried per Plan 22-03 § Out-of-scope #5).
- **Max feedback latency:** ~10s for targeted; ~3min for full.

---

## Per-Task Verification Map

### Plan 26-01 — PKG fork-architectural decisions (executed 2026-04-29)

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 26-01-01 | 01 | 1 | PKGS-02 | T-26-01-01 (Tampering — `..` traversal) | `validate_relative_path` rejects `..` Path components at input-string layer before any filesystem syscall | unit | `cargo test -p nono-cli --bin nono -- validate_relative_path_rejects_traversal` | ✅ | ✅ green |
| 26-01-01 | 01 | 1 | PKGS-02 | T-26-01-02 (Tampering — absolute path) | `validate_relative_path` rejects Unix `/foo` and (Windows-host) `C:\foo`, `\\server\share` shapes | unit | `cargo test -p nono-cli --bin nono -- validate_relative_path_rejects_absolute_path` | ✅ | ✅ green |
| 26-01-01 | 01 | 1 | PKGS-02 | T-26-01-03 (Tampering — symlink-traversal) | `validate_path_within` (canonicalize-and-component-compare, line 1043) rejects symlink-resolved escapes from `staging_root`; defense-in-depth posture preserved | unit | `cargo test -p nono-cli --bin nono -- validate_path_within_rejects_symlink_escape` | ✅ | ✅ green |
| 26-01-02 | 01 | 1 | PKGS-03 | T-26-01-04 (Tampering — unknown variant) | `ArtifactType::Plugin` round-trips JSON `"plugin"` via `#[serde(rename_all = "snake_case")]` | unit | `cargo test -p nono-cli --bin nono -- artifact_type_plugin_round_trips` | ✅ | ✅ green |
| 26-01-02 | 01 | 1 | PKGS-03 | T-26-01-04 (Tampering — unknown variant) | Unknown `artifact_type` JSON values (`"made_up_variant"`, `"PLUGIN"`, non-string) deserialize as `Err` (fail-closed; no silent coercion to default or to filename-fallback `Script`) | unit | `cargo test -p nono-cli --bin nono -- artifact_type_unknown_fails_closed` | ✅ | ✅ green |
| 26-01-03 | 01 | 1 | PKGS-03 | — | `ArtifactType::Plugin` match arms exhaustive across 1+ enum-discriminant site in `package_cmd.rs`; deferred-divergence comment removed | build-gate | `cargo build --workspace` (non-exhaustive match would surface here) | ✅ | ✅ green |
| 26-01-D19 | 01 | 1 | PKGS-02 + PKGS-03 | — | D-19 byte-identical preservation: `crates/nono/` untouched across the plan | grep-gate | `git diff --stat <baseline>..HEAD -- crates/nono/ \| wc -l` returns `0` | ✅ | ✅ green (verified at SUMMARY time) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

### Plan 26-02 — PKG streaming + auto-pull (executed 2026-05-09 on Windows host)

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 26-02-T2 | 02 | 2 | PKGS-01 | — | `semver` runtime dep added (matches upstream pin) | build-gate | `cargo build --workspace` (post-`9cb7770f`) | ✅ | ✅ green |
| 26-02-T3 | 02 | 2 | PKGS-01 | T-26-02-01 (Tampering — bytes corrupted mid-stream) | Streaming SHA-256 incremental in `RegistryClient::download_artifact_to_path`; mismatch rejects before install_artifacts copies bytes from TempDir to install_dir | unit | `cargo test -p nono-cli --bin nono -- registry_client::tests::download_artifact_to_path_computes_digest_of_streamed_bytes` | ✅ | ✅ green |
| 26-02-T3 | 02 | 2 | PKGS-01 | T-26-02-02 (DoS — memory bomb) | Content-Length pre-check + ureq `with_config().limit()` reader cap (defense-in-depth) at fixed-const ceilings (2 MiB JSON / 8 MiB bundle / 64 MiB artifact) — upstream-aligned | unit | `cargo test -p nono-cli --bin nono -- registry_client::tests::download_artifact_to_path_rejects_oversize_via_content_length` | ✅ | ✅ green |
| 26-02-T3 | 02 | 2 | PKGS-01 | T-26-02-03 (DoS — hung connection) | ureq Agent connect_timeout=10s | unit | `cargo test -p nono-cli --bin nono -- registry_client::tests::registry_client_connect_timeout_fires_within_bounded_window` | ✅ | ✅ green |
| 26-02-T3 | 02 | 2 | PKGS-01 | — | `enforce_content_length` boundary semantics: rejects oversize, passes at boundary, passes when header absent | unit | `cargo test -p nono-cli --bin nono -- registry_client::tests::enforce_content_length_` | ✅ | ✅ green (3 tests) |
| 26-02-T3 | 02 | 2 | PKGS-01 | T-26-02-07 (panic mid-stream) | `tempfile::TempDir` Drop fires unconditionally on panic; staged bytes cleaned up | unit | `cargo test -p nono-cli --bin nono -- registry_client::tests::tempdir_cleanup_runs_on_panic` | ✅ | ✅ green |
| 26-02-T3 | 02 | 2 | PKGS-01 | — | `RegistryClient::new` constructor smoke test | unit | `cargo test -p nono-cli --bin nono -- registry_client::tests::registry_client_constructor_succeeds` | ✅ | ✅ green |
| 26-02-T3 | 02 | 2 | PKGS-01 | — | `compare_versions` switched to `semver::Version` parsing; prerelease ordering honored | unit | `cargo test -p nono-cli --bin nono -- package_cmd::tests::compare_versions_honors_prerelease_ordering` | ✅ | ✅ green |
| 26-02-T3 | 02 | 2 | PKGS-01 | — | `remove_external_artifacts` retains shared hook scripts (Hook is the lone whitelist); still removes non-hook files | unit | `cargo test -p nono-cli --bin nono -- package_cmd::tests::remove_external_artifacts_` | ✅ | ✅ green (2 tests) |
| 26-02-T4 | 02 | 2 | PKGS-04 | — | `is_registry_ref` discriminator routes `namespace/name[@version]` shapes through `load_registry_profile` (auto-pull); idempotent (present `<install_dir>/package.json` short-circuits network) | grep-gate | `grep -nE 'load_registry_profile\|is_registry_ref' crates/nono-cli/src/profile/mod.rs` returns ≥4 lines | ✅ | ✅ green |
| 26-02-T4 | 02 | 2 | PKGS-04 (acceptance #1–#3) | T-26-02-05, T-26-02-08 | E2E auto-pull via `run_nono` harness with mock registry: registry-pack profile triggers idempotent fetch; offline + missing pack fails closed; auto-pull respects size cap | e2e (deferred) | (not implemented — host_blocker) | ❌ deferred | ⬜ deferred |
| 26-02-T6 | 02 | 2 | PKGS-01 acceptance #1 (Linux RSS) | — | 200MB streams at ~10MB peak RSS via `/proc/self/status` | unit (Linux-only) | `cargo test -p nono-cli --bin nono -- registry_client::tests::download_artifact_to_path_streams_under_bounded_rss` | ✅ (compiled-out on Windows) | ⬜ Linux-only — not exercised on this host |
| 26-02-T7 | 02 | 2 | — | — | D-19 byte-identical preservation: `crates/nono/` untouched | grep-gate | `git diff --stat 57be91a9..18eb3913 -- crates/nono/ \| wc -l` returns `0` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

### Plan 26-02 Acceptance Notes (operator-accepted deviations)

Three plan-text deviations were surfaced during merge and accepted by the operator (2026-05-09). Documented here so future audits don't re-flag them as gaps:

| Deviation | Plan-text expectation | What landed | Rationale (accepted) |
|-----------|----------------------|-------------|----------------------|
| **A.** No `NonoError::ArtifactTooLarge` variant + no `--max-size` CLI flag | Truth #3: `NonoError::ArtifactTooLarge { actual, max }` + `--max-size <bytes>` flag (default 500MB, configurable) | Size violations surface as `NonoError::RegistryError(String)`; size cap is fixed const (2/8/64 MiB JSON/bundle/artifact) — exact upstream `9ebad89a` shape | Plan Task 7 explicitly allowed both Path A (variant + provenance trailer) and Path B (upstream-aligned, no new variant). Path B chosen for D-19 strict preservation: `crates/nono/` byte-identical across all Plan 26-02 commits |
| **B.** `bundle_json` on `VerifiedDownloads` wrapper struct, not on `DownloadedArtifact` | Truth #11: `pub bundle_json` field on `DownloadedArtifact`; grep `'pub bundle_json' crates/nono-cli/src/package.rs` = 1, grep `'let bundle_json' crates/nono-cli/src/package_cmd.rs` = 0 | New `VerifiedDownloads` wrapper struct holds bundle_json + per-pull TempDir + signer identity; the `let bundle_json` line at L?? is the field-init binding inside the wrapper construction (not a free local var). Both literal grep gates fail | Plan-text drift. Upstream's actual `9ebad89a` diff places bundle_json on the wrapper because every artifact in a single pull shares one multi-subject bundle — putting it on `DownloadedArtifact` would duplicate the JSON across N artifacts in one pull. Structural intent of truth #11 (no longer a free local var; lives on a struct field that survives the call) is satisfied |
| **C.** No `mockito` dev-dep | Task 5: add `mockito = "1"` to `[dev-dependencies]`; Task 6 integration tests use mockito `Server::new()` | Task 5 skipped. Test fixtures use a 50-LOC std-only single-shot in-process TCP server inside `registry_client::tests` | Portable-subset constraint avoids a new dev-dep; std-only TCP server covers all in-process invariants. Auto-pull e2e tests (the ones that genuinely benefit from mockito's fixture model) are deferred per host_blocker — when those land on a Linux/macOS host, mockito can be added as part of that follow-up |

---

## Wave 0 Requirements

Existing test infrastructure (Rust `cargo test` + the `tempfile` crate already in `crates/nono-cli/Cargo.toml:73`) covers all Plan 26-01 requirements. No Wave 0 framework installation needed.

---

## Manual-Only Verifications

All Plan 26-01 phase behaviors have automated verification after this audit (the prior gap on truth #7 is now closed by `validate_path_within_rejects_symlink_escape`).

One privilege-conditional behavior:

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Symlink creation on Windows without Developer Mode / SeCreateSymbolicLink privilege | PKGS-02 truth #7 | `std::os::windows::fs::symlink_dir` returns `Err` on hosts lacking the privilege; the regression test early-returns in that case (NOT `#[ignore]`). On a privilege-missing host, validation of the canonicalize-and-compare layer falls back to manual review of `validate_path_within` invariants. | Run on a host with the symlink privilege (the current dev box satisfies this — verified 2026-05-09). The test prints `skipping symlink test` to stderr only when privilege is missing. |

Two host-conditional behaviors from Plan 26-02:

| Behavior | Requirement | Why Manual / Deferred | Test Instructions |
|----------|-------------|----------------------|-------------------|
| 200MB streams at ~10MB peak RSS | PKGS-01 acceptance #1 | `read_proc_self_rss_kb` reads `/proc/self/status` (Linux-specific). Test is `#[cfg(target_os = "linux")]` — compiles out cleanly on Windows/macOS | Run on a Linux host: `cargo test -p nono-cli --bin nono -- registry_client::tests::download_artifact_to_path_streams_under_bounded_rss` |
| Auto-pull e2e (REQ-PKGS-04 acceptance #1–#3) | PKGS-04 | Requires Sigstore-signed fixture packs + `run_nono` harness coordination. Plan was authored before Phase 27.1's `NONO_TEST_HOME` seam (landed 2026-05-05); test code does not yet use the seam. Production code (`load_registry_profile` + `is_registry_ref`) is in place | Author 3 e2e tests on a Linux/macOS host (or Windows with `NONO_TEST_HOME` plumbed into the harness): registry-pack profile triggers idempotent fetch; offline + missing pack fails closed; auto-pull respects size cap. Then re-run `/gsd-validate-phase 26` to flip nyquist_compliant to `compliant` |

---

## Validation Audit 2026-05-09

| Metric | Count |
|--------|-------|
| Requirements in Plan 26-01 scope | 2 (PKGS-02, PKGS-03) |
| Truths declared (must_haves.truths) | 12 |
| Truths with automated tests | 5 (#5, #6, #7, #8, #9) |
| Truths covered by build/grep/CI gates | 7 (#1, #2, #3, #4, #10, #11, #12) |
| Gaps found | 1 (truth #7) |
| Resolved | 1 (truth #7 — `validate_path_within_rejects_symlink_escape` added) |
| Escalated | 0 |
| Plan 26-02 scope | 2 reqs (PKGS-01, PKGS-04) — execution deferred to v2.4; out of scope for this audit |

---

## Validation Sign-Off

- [x] All Plan 26-01 tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (none — no framework install needed)
- [x] No watch-mode flags
- [x] Feedback latency < 10s targeted, < 3min full
- [ ] `nyquist_compliant: true` — set to `partial` because Plan 26-02 is unexecuted; will flip to `true` only after Plan 26-02 executes and a follow-up validation pass closes PKGS-01 + PKGS-04 truths.

**Approval:** approved 2026-05-09 (Plan 26-01 surface only)

---

## Validation Audit 2026-05-09 (re-audit)

Re-confirmed compliance at HEAD via fresh `cargo test -p nono-cli --bin nono -- validate_relative_path_rejects_traversal validate_relative_path_rejects_absolute_path validate_path_within_rejects_symlink_escape artifact_type_plugin_round_trips artifact_type_unknown_fails_closed`: **5 passed; 0 failed; 0 ignored** (~9 s targeted, includes build). All Per-Task Map runtime rows remain green.

Documentation drift corrected on one must-have grep gate (cosmetic only — runtime behavior was always satisfied):

| Row | Drift | Fix |
|-----|-------|-----|
| Plan must-have truth #2 | Documented `grep -c 'fn validate_path_within' crates/nono-cli/src/package_cmd.rs` = "exactly 1" (production fn definition only). At HEAD this returns 2 — production fn at line 1043 + new test fn name `validate_path_within_rejects_symlink_escape` at line 1210, which shares the `fn validate_path_within` substring. Same overlap pattern truth #1 already documents for `fn validate_relative_path` (3 matches: 1 production + 2 test fn names). | Production-fn count of 1 verifiable by line-by-line inspection: `crates/nono-cli/src/package_cmd.rs:1043` is the only `fn validate_path_within(base: &Path, full: &Path) -> Result<()>` definition. The line-1210 match is the regression test added during the original 2026-05-09 audit to close truth #7 — its function-name overlap with truth #2's grep pattern is benign. Defense-in-depth posture (truth #2 substance) is unchanged. |

| Metric | Count |
|--------|-------|
| Gaps found | 0 (runtime); 1 (cosmetic grep-gate drift on truth #2) |
| Resolved | 1 (cosmetic — documented inline above) |
| Escalated | 0 |
| New tests written | 0 |
| Existing tests verified | 5 (all green at HEAD) |
| `nyquist_compliant` status | unchanged: `partial` (Plan 26-02 still queued for v2.4; will flip to `true` only after Plan 26-02 executes and a follow-up validation pass closes PKGS-01 + PKGS-04 truths) |

---

## Validation Audit 2026-05-09 (third re-audit)

Re-confirmed compliance at HEAD. Re-ran the same five tests via `cargo test -p nono-cli --bin nono -- validate_relative_path_rejects_traversal validate_relative_path_rejects_absolute_path validate_path_within_rejects_symlink_escape artifact_type_plugin_round_trips artifact_type_unknown_fails_closed`: **5 passed; 0 failed; 0 ignored** (~0.55s build + ~0.00s test). All Per-Task Map runtime rows remain green.

Re-verified all five must-have grep gates at HEAD (same counts as the prior re-audit, including the documented truth #2 substring overlap with the test fn name):

| Gate | Pattern | HEAD count | Documented expectation |
|------|---------|-----------|------------------------|
| truth #1 | `fn validate_relative_path` in `package_cmd.rs` | 3 | 1 production + 2 test fn name overlaps (already documented in original audit) |
| truth #2 | `fn validate_path_within` in `package_cmd.rs` | 2 | 1 production + 1 test fn name overlap (documented in re-audit cosmetic-fix table above) |
| truth #3 | `    Plugin,` in `package.rs` | 1 | exactly 1 (variant body) |
| truth #4 | `ArtifactType::Plugin` in `package_cmd.rs` | 1 | at least 1 (the new arm) |
| truth #10 | `upstream ec49a7af also adds an ArtifactType::Plugin` | 0 | exactly 0 (deferred-divergence comment removed) |

Re-verified D-19 byte-identical preservation per Phase 26 commit (`git diff --stat <c>^..<c> -- crates/nono/ | wc -l` for each of `e5e1f2d7 dd7b28b3 797f3295 8ff89923 1f47d0ee da8bbefa`): all return `0`. Touches to `crates/nono/` in the chronological span between the first and last Phase 26 commits originate from Phase 27.1 / 28 / 29 commits sandwiched in the same range, not from this phase.

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 (no drift this pass) |
| Escalated | 0 |
| New tests written | 0 |
| Existing tests verified | 5 (all green at HEAD) |
| `nyquist_compliant` status | unchanged: `partial` (Plan 26-02 still queued for v2.4; will flip to `true` only after Plan 26-02 executes and a follow-up validation pass closes PKGS-01 + PKGS-04 truths) |

---

## Validation Audit 2026-05-09 (Plan 26-02 execution close)

Plan 26-02 executed on Windows host under "portable subset" directive. 6 commits landed on `main` (`9cb7770f..18eb3913`). Three operator-accepted plan-text deviations documented above ("Plan 26-02 Acceptance Notes"). Per-Task verification map populated above.

**Re-verified at HEAD (`18eb3913`):**

```
cargo test -p nono-cli --bin nono -- registry_client compare_versions remove_external_artifacts
→ 11 passed; 0 failed; 0 ignored (~10s; includes incremental build)

cargo test -p nono-cli --bin nono -- validate_relative_path_rejects_traversal validate_relative_path_rejects_absolute_path validate_path_within_rejects_symlink_escape artifact_type_plugin_round_trips artifact_type_unknown_fails_closed
→ 5 passed; 0 failed; 0 ignored (Plan 26-01 surface holds across the merge)
```

**D-19 preservation (Plan 26-02 commits):** `git diff --stat 57be91a9..18eb3913 -- crates/nono/ | wc -l` returns `0`. `crates/nono/` is byte-identical across the 6 commits — Path B chosen (no `NonoError::ArtifactTooLarge` variant) preserves the strict invariant.

**Carryovers (per Plan 22-03 § Out-of-scope #5 + Phase 23/28/29 precedent):**
- 2 pre-existing TUF integration failures in `nono::trust::bundle::tests` (`load_production_trusted_root_succeeds`, `verify_bundle_with_invalid_digest`) — TUF trust-root signature threshold environmental issue, not a code regression
- 2 pre-existing `nono::manifest::*::collapsible_match` clippy errors at lines 95, 103 — Phase 26 does not touch `crates/nono/`

| Metric | Count |
|--------|-------|
| Plan 26-02 truths declared | 13 |
| Truths covered by automated unit tests | 7 (digest streaming, oversize Content-Length, connect timeout, panic-safe TempDir cleanup, prerelease ordering, hook-retention pair, enforce_content_length boundary trio) |
| Truths covered by build/grep/CI gates | 4 (semver dep present, ureq timeouts, profile auto-pull surface present, D-19 preservation) |
| Truths covered by Linux-only test (cfg-gated, not exercised this run) | 1 (200MB RSS) |
| Truths deferred to e2e on Linux/macOS host | 1 (auto-pull happy/sad/cap-respect — REQ-PKGS-04 acceptance #1–#3) |
| Operator-accepted deviations (not gaps) | 3 (Path B, VerifiedDownloads wrapper, no mockito) |
| Gaps found | 0 |
| `nyquist_compliant` status | unchanged: `partial` (Plan 26-02 production code is in; auto-pull e2e tests deferred to Linux/macOS host pass — flip to `compliant` after that pass) |
