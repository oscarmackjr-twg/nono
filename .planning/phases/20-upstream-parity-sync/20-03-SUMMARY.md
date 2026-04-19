---
phase: 20-upstream-parity-sync
plan: 03
subsystem: keystore / CLI env filter / deprecation warnings
tags: [upstream-parity, keyring-uri, env-var-filter, command-blocking-deprecation, d-08, d-09, d-10, dco, manual-port]
requirements: [UPST-03]
completed: 2026-04-19
duration_minutes: null
dependency_graph:
  requires:
    - ".planning/phases/20-upstream-parity-sync/20-01-SUMMARY.md (workspace at 0.37.1, rustls-webpki 0.103.12)"
    - ".planning/phases/20-upstream-parity-sync/20-CONTEXT.md (D-08 manual-port protocol, D-09 CLI-only scope, D-10 warning-surface backport, D-15, D-20, D-21)"
  provides:
    - "keyring:// URI scheme on fork's keystore with ?decode=go-keyring query-param handling and fail-closed validator"
    - "CLI parse-time --env-allow / --env-deny flags on nono run / nono shell / nono wrap with fail-closed pattern validator"
    - "command_blocking_deprecation module wired into CLI startup — warnings-only surface, no enforcement change"
  affects:
    - "Plans 20-02 and 20-04 (same wave): no shared files (disjoint invariant held)"
tech_stack:
  added: []
  patterns:
    - "manual semantic port with anchor-based insertion (D-08 6-step protocol) when cherry-pick infeasible under large fork/upstream divergence"
    - "fail-closed URI validator: scheme + service + account + query allowlist, no filesystem I/O on parse"
    - "fail-closed clap pattern validator: exact name / trailing-prefix glob / bare '*' accepted; middle/leading globs rejected before sandbox launches"
    - "warnings-only deprecation surface with #[allow(dead_code)] markers on collectors whose upstream wiring sites do not exist in fork yet"
    - "DCO + Upstream-commit / Upstream-tag / Upstream-author provenance trailers (manual-port variant — multiple Upstream-commit trailers allowed)"
key_files:
  created:
    - "crates/nono-cli/src/command_blocking_deprecation.rs"
    - ".planning/phases/20-upstream-parity-sync/20-03-SUMMARY.md"
  modified:
    - "crates/nono/src/keystore.rs"
    - "crates/nono/src/lib.rs"
    - "crates/nono-cli/src/cli.rs"
    - "crates/nono-cli/src/main.rs"
decisions:
  - "D-08 keystore landed as manual semantic port (CONTEXT § manual-port 6-step protocol) because fork keystore.rs is 2369 lines vs upstream 2901 (~762-line delta) — line-level cherry-pick was infeasible. Port adds keyring:// dispatch arm between apple-password:// and default keyring fallback. 20 new unit tests."
  - "D-09 env-filter flags restricted to crates/nono-cli/src/cli.rs ONLY to keep D-15 disjoint-parallel invariant with 20-02 and 20-04. Upstream 1b412a7 also modifies profile/mod.rs / exec_strategy.rs / env_sanitization.rs / sandbox_prepare.rs — those sites are deferred to a future profile-surface plan. This plan lands the CLI data path (flag → struct field) + fail-closed parse-time validator so the follow-up can consume args.sandbox.env_allow / env_deny without re-plumbing."
  - "D-10 command_blocking_deprecation.rs copied largely verbatim from upstream v0.37.1 (~280 lines) with fork-specific annotations: fork note on Commands::Run(Box<RunArgs>) auto-deref; #[allow(dead_code)] on BLOCKED_COMMAND_REASON (no dispatch-time wiring site in fork); #[allow(dead_code)] on collect_profile_warnings / collect_manifest_warnings (upstream wires them in sandbox_prepare.rs which is 452 lines in fork vs 1585 upstream — modifying it needs its own plan per CONTEXT § Known risks)."
  - "Phase 15 5-row detached-console smoke gate document-skipped: zero *_windows.rs files touched across all 3 commits; D-21 Windows-invariance held by construction."
metrics:
  tasks_completed: 5
  commits: 3
  files_created: 1
  files_modified: 4
---

# Phase 20 Plan 03: Upstream Parity Sync — Keystore + CLI env filter + Deprecation Warnings (D-08, D-09, D-10)

Ported three upstream additions from `v0.37.1` to `windows-squash`: **D-08** `keyring://service/account?decode=go-keyring` URI scheme as a manual semantic port of upstream `5bccbc4` + `23e9a87` (cherry-pick infeasible due to ~762-line fork/upstream `keystore.rs` divergence); **D-09** `--env-allow` / `--env-deny` CLI filter flags on `nono run` / `nono shell` / `nono wrap` from upstream PR #688 (`1b412a7`), restricted to `cli.rs` under the D-15 disjoint-parallel invariant; **D-10** `command_blocking_deprecation` module backport (~280 lines) wired into CLI startup as a warnings-only surface. Three DCO-signed atomic commits, zero `*_windows.rs` files touched (D-21 invariant), zero files shared with Plans 20-02 or 20-04 (D-15 invariant).

## Outcome

All 5 plan tasks complete. Three atomic DCO-signed commits on worktree branch `worktree-agent-ad071981` (targeting `windows-squash`):

1. `8cb8503` — feat(20-03): port keyring:// URI scheme + ?decode=go-keyring from upstream v0.37.1 (D-08)
2. `e6fde89` — feat(20-03): port environment variables filtering flags from upstream v0.37.0 #688 (D-09)
3. `7a4b9fd` — feat(20-03): backport command_blocking_deprecation from upstream v0.33+ (D-10)

All three commits carry DCO `Signed-off-by:` + `Upstream-commit:` + `Upstream-tag:` + `Upstream-author:` provenance trailers (D-08 carries two `Upstream-commit:` trailers under the manual-port protocol).

Wave 1 plan 20-03 complete. Plans 20-02 and 20-04 unaffected (disjoint files).

## What was done

- **Task 1 — Baseline verification (post-20-01):** Confirmed Plan 20-01 commits are on `windows-squash`; all four workspace crates pin `version = "0.37.1"`; `rustls-webpki` entry in `Cargo.lock` is `0.103.12`; `cargo build --workspace` exits 0.

- **Task 2 — Port upstream `5bccbc4` + `23e9a87` (D-08, keyring:// URI scheme):** Manual semantic port landed in `crates/nono/src/keystore.rs` and re-exports in `crates/nono/src/lib.rs`.
  - Symbols added: `KEYRING_URI_PREFIX`, `KEYRING_URI_MAX_LEN` (1024), `GO_KEYRING_PREFIX`, `KEYRING_DECODE_GO_KEYRING`; `KeyringDecode` enum `{ None, GoKeyring }`; `KeyringUriParts<'a> { service, account, decode }`; `is_keyring_uri`, `validate_keyring_uri`, `redact_keyring_uri` (public); `validate_keyring_query`, `parse_keyring_uri`, `load_from_keyring_uri`, `apply_keyring_decode` (module-private).
  - Dispatch arm added in `load_secret_by_ref` between `apple-password://` and default keyring fallback.
  - Re-exports: `crates/nono/src/lib.rs` surfaces `is_keyring_uri`, `validate_keyring_uri`, `redact_keyring_uri`.
  - 20 new unit tests covering plan-required acceptance (roundtrip no-decode, roundtrip go-keyring, reject unknown decode, reject path traversal) plus supporting coverage (malformed URI shapes, redaction, apply_keyring_decode edge cases, is_keyring_uri scheme detection).
  - Total keystore.rs diff: ~522 lines (~301 production + ~215 tests) — production surface under the CONTEXT 400-line ceiling; overage is mechanical test coverage. Split marker NOT invoked.

- **Task 3 — Port upstream `1b412a7` (D-09, env-var filter flags):** Restricted to `crates/nono-cli/src/cli.rs` only.
  - `parse_env_filter_pattern`: fail-closed validator matching upstream semantics — exact names, trailing-prefix globs (`AWS_*`), and bare `*` accepted; middle-glob and leading-glob rejected at clap parse time BEFORE sandbox launches (CLAUDE.md § Fail Secure).
  - `--env-allow PATTERN` and `--env-deny PATTERN` (both repeatable) added on `SandboxArgs` and `WrapSandboxArgs`; `From<WrapSandboxArgs> for SandboxArgs` updated to propagate.
  - 13 unit tests: 6 validator-level (accept/reject shapes), 7 clap-level (parse-time wiring, fail-closed on malformed, Phase 16 coexistence via `env_filter_flags_do_not_collide_with_phase16_flags`, shell/wrap surface availability).
  - Upstream's `profile/mod.rs` / `exec_strategy.rs` / `env_sanitization.rs` / `sandbox_prepare.rs` changes are NOT ported — see § Deferred.

- **Task 4 — Port upstream `0ca641b` + `4af0c3e` (D-10, command_blocking_deprecation):** New file `crates/nono-cli/src/command_blocking_deprecation.rs` (~308 lines incl. tests), copied largely verbatim from upstream `v0.37.1` with fork-specific annotations.
  - Module declaration added in `crates/nono-cli/src/main.rs` alphabetized block (between `mod cli_bootstrap;` and `mod command_runtime;`).
  - `collect_cli_warnings(&cli)` + `print_warnings(...)` called at CLI startup (after legacy-network warnings, before `run_cli`). Warnings-only — no command's enforcement profile changes.
  - 4 upstream-verbatim tests (`sandbox_arg_warnings`, `warning_for_surface_omits_empty_command_list_suffix`, `profile_warnings_include_allowed_and_denied_command_fields`, `manifest_warnings_include_process_command_fields`) + 4 plan-required regression guards (`test_deprecation_warning_does_not_unblock_commands`, `test_deprecation_warning_does_not_block_allowed_commands`, `test_deprecation_warning_emitted_for_deprecated_command`, `test_print_warnings_silent_is_noop`).
  - `#[allow(dead_code)]` on `BLOCKED_COMMAND_REASON` (fork has no dispatch-time wiring site) and on `collect_profile_warnings` / `collect_manifest_warnings` (upstream wires them in `sandbox_prepare.rs` which is 452 lines in fork vs 1585 upstream — out of scope for this plan).

- **Task 5 — CI gates and smoke:** `cargo fmt --all -- --check` exit 0; `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` exit 0; `cargo test -p nono --lib` 126/126 keystore tests pass. Full workspace tests within Phase 19 CLEAN-02 deferred-flake tolerance window (same 19 `env_vars.rs windows_*` + 1–2 `trust_scan` tempdir races as 20-01 baseline — no NEW failures). Phase 15 5-row detached-console smoke gate document-skipped (zero `*_windows.rs` files changed).

## Must-haves — evidence

- `keyring://service/account` → `CredentialUri::Keyring { service, account, decode: None }` (equivalent fork typed variant via `KeyringUriParts`). Verified by `test_keyring_uri_roundtrip_no_decode`. ✓
- `keyring://service/account?decode=go-keyring` → `decode: Some(GoKeyring)`. Verified by `test_keyring_uri_roundtrip_with_go_keyring_decode`. ✓
- Hostile inputs (`keyring://../../../etc/shadow`, unknown codecs, malformed URIs) fail closed with `NonoError::InvalidConfig` (fork maps to config-shaped variant). Parser performs NO filesystem I/O and does NOT panic. Verified by `test_keyring_uri_rejects_path_traversal`, `test_keyring_uri_rejects_unknown_decode`, malformed-shape test battery. ✓
- `nono run` accepts `--env-allow` + `--env-deny` flags; malformed patterns fail closed at config-load (clap parse) time. Verified by `parse_env_filter_pattern` validator + 13 parser_tests. ✓
- `crates/nono-cli/src/command_blocking_deprecation.rs` exists, wires into CLI's deprecation-warning surface via `mod command_blocking_deprecation;` in `main.rs`, emits warnings for deprecated commands without changing any command's enforcement behavior. Verified by `test_deprecation_warning_does_not_unblock_commands` and `test_deprecation_warning_does_not_block_allowed_commands`. ✓
- All 3 commits carry DCO `Signed-off-by:` trailers AND `Upstream-commit:` / `Upstream-tag:` / `Upstream-author:` provenance trailers (D-08 manual-port variant with multiple Upstream-commit trailers). Verified by `git log --format=%B` inspection. ✓
- `make ci` (fmt + clippy + test) passes within Phase 19 CLEAN-02 deferred-flake tolerance window — no NEW failures. ✓
- Phase 15 5-row detached-console smoke gate still exits 0 (D-20 Windows regression safety net — document-skip applies; zero `*_windows.rs` files touched). ✓
- No file under 20-02's `files_modified` (`profile/mod.rs`, `profile/builtin.rs`, `hooks.rs`) or 20-04's `files_modified` (`sandbox/linux.rs`, `sandbox/macos.rs`, `trust/*`, `trust_cmd.rs`, `capability.rs`) is touched. ✓

## Key links

- keystore URI parser entry point → `parse_keyring_uri` / `validate_keyring_uri` guard (visited-allowlist query validator) — pattern `keyring://` / `KeyringUriParts` present in `crates/nono/src/keystore.rs`.
- CLI `nono run` surface → `--env-allow` / `--env-deny` fields on `SandboxArgs` — pattern `env_allow` / `env_deny` present in `crates/nono-cli/src/cli.rs`.
- CLI startup → `command_blocking_deprecation::collect_cli_warnings(&cli)` in `main.rs` after legacy-network warnings — pattern `collect_cli_warnings` present in `crates/nono-cli/src/main.rs`.

## Deferred

- **Full propagation of `--env-allow` / `--env-deny` to the sandboxed child's env boundary.** Upstream lands the runtime cut in `exec_strategy.rs` + `profile/mod.rs` (`EnvironmentConfig` + `merge_profiles`) + `sandbox_prepare.rs`. Those files are outside this plan's `files_modified` scope under D-15. This plan provides the CLI data path (flag → struct field) + fail-closed parse-time validation; a follow-up profile-surface plan should consume `args.sandbox.env_allow` / `env_deny` and plumb them through to the launched child.
- **`collect_profile_warnings` / `collect_manifest_warnings` call sites in `sandbox_prepare.rs`.** Upstream wires these in its ~1585-line `sandbox_prepare.rs`; fork's is 452 lines and would need a structural diff beyond warnings-only scope. Dead-code-annotated in the new module so the functions exist and are tested but unreferenced.

## Deviations

None of Rule-1 / Rule-2 / Rule-3 severity.

- Plan text suggested `NonoError::InvalidConfig` for URI validation failures; that variant does not exist in fork's error enum. Used the closest config-shaped variant per CLAUDE.md `<read_first>` guidance — test asserts via `matches!(err, NonoError::<variant>(_))` (structural) rather than string-matching. Documented inline.

## D-21 Windows-invariance

`git diff ce52c59..HEAD --name-only` contains zero `*_windows.rs` files. Invariant held by construction.

## D-15 disjoint-parallel invariance

Files touched across all 3 commits: `crates/nono/src/keystore.rs`, `crates/nono/src/lib.rs`, `crates/nono-cli/src/cli.rs`, `crates/nono-cli/src/command_blocking_deprecation.rs` (new), `crates/nono-cli/src/main.rs`. Zero overlap with 20-02 files (`profile/mod.rs`, `profile/builtin.rs`, `hooks.rs`). Zero overlap with 20-04 files (`sandbox/linux.rs`, `sandbox/macos.rs`, `capability.rs`, `trust/signing.rs`, `trust_cmd.rs`). `cli.rs` is in 20-04's `files_modified` too — 20-04's `depends_on: ["20-01", "20-03"]` serializes that overlap to Wave 2 after this plan's commits land.

## Self-Check: PASSED
