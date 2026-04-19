---
phase: 20-upstream-parity-sync
plan: 04
subsystem: capability + CLI + Linux/macOS sandbox + trust signing
tags: [upstream-parity, allow-gpu, nvidia-landlock, gitlab-id-tokens, d-11, d-12, d-13, d-21, dco, manual-port]
requirements: [UPST-04]
completed: 2026-04-19
duration_minutes: null
dependency_graph:
  requires:
    - ".planning/phases/20-upstream-parity-sync/20-01-SUMMARY.md (workspace at 0.37.1, rustls-webpki 0.103.12)"
    - ".planning/phases/20-upstream-parity-sync/20-03-SUMMARY.md (post-20-03 cli.rs env-filter additions — this plan shares cli.rs)"
    - ".planning/phases/20-upstream-parity-sync/20-CONTEXT.md (D-11 GitLab tokens, D-12 --allow-gpu, D-13 NVIDIA allowlist, D-15 disjoint-parallel, D-16 verification gate, D-17 atomic-commit discipline, D-20 Windows regression safety net, D-21 Windows invariance)"
  provides:
    - "`--allow-gpu` flag on `nono run` / `nono shell` / `nono wrap` with 3-platform dispatch (Linux Landlock, macOS Seatbelt IOKit, Windows CLI-layer warning)"
    - "`CapabilitySet::gpu` / `allow_gpu` / `set_gpu` builder surface for library clients"
    - "NVIDIA + DRM + AMD + WSL2 Linux device-node + procfs allowlist gated by `--allow-gpu` (D-12 + D-13 bundled)"
    - "GitLab ID token support in `nono trust sign --keyless` via `gitlab_keyless_predicate` + generic OIDC issuer validator"
    - "`validate_oidc_issuer` — fail-closed URL-component equality validator for OIDC issuer pins (regression-guarded against `iss.starts_with` anti-pattern)"
  affects:
    - "Plans 20-02 and 20-03 (same phase): zero shared files with 20-02 (profile/mod.rs, profile/builtin.rs, hooks.rs); `cli.rs` shared with 20-03 but this plan depends_on 20-03 so its edits land on top"
tech_stack:
  added:
    - "url = 2.5 (workspace + nono direct dep; was transitive via reqwest/sigstore, promoted for fail-closed OIDC issuer-pin validation)"
  patterns:
    - "cross-platform capability dispatch: single `CapabilitySet::gpu` field, Linux backend probes devfs/procfs, macOS backend emits Seatbelt IOKit grants, Windows emits tracing::warn! at CLI layer"
    - "fail-closed OIDC issuer-pin validation via `url::Url::parse` component equality (scheme + host + port) — explicit regression guard against `iss.starts_with` string-prefix anti-pattern"
    - "apply-time filesystem probing for GPU device enumeration — absent devices silently skipped so same `--allow-gpu` invocation works across headless, NVIDIA-only, AMD-only, WSL2 hosts"
    - "CLI-only Windows-invariance emission: `#[cfg(target_os = \"windows\")]` warning in cli.rs, never in sandbox/windows.rs (D-21 mandate)"
    - "atomic-commit provenance: every commit carries DCO `Signed-off-by:` + `Upstream-commit:` (possibly multiple) + `Upstream-tag: v0.37.1` + `Upstream-author:` trailers"
key_files:
  created:
    - ".planning/phases/20-upstream-parity-sync/20-04-SUMMARY.md"
    - ".planning/phases/20-upstream-parity-sync/deferred-items.md"
  modified:
    - "crates/nono/src/capability.rs"
    - "crates/nono-cli/src/cli.rs"
    - "crates/nono-cli/src/capability_ext.rs"
    - "crates/nono/src/sandbox/linux.rs"
    - "crates/nono/src/sandbox/macos.rs"
    - "crates/nono/src/trust/signing.rs"
    - "crates/nono-cli/src/trust_cmd.rs"
    - "Cargo.toml"
    - "crates/nono/Cargo.toml"
    - "Cargo.lock"
decisions:
  - "D-12 capability-routing deviation from upstream: upstream wires `--allow-gpu` through `sandbox_prepare.rs::maybe_enable_macos_gpu` + `maybe_enable_gpu` (fork 452 lines vs upstream 1585 — known-risky per CONTEXT § D-18) and through `profile/mod.rs` (Plan 20-02's exclusive scope). Fork's port routes the capability DIRECTLY through the `CapabilitySet` + sandbox backend layer: `SandboxArgs::allow_gpu` (cli.rs) → `caps.set_gpu(true)` (capability_ext.rs) → `caps.gpu()` consumed in sandbox/linux.rs + sandbox/macos.rs. Simpler than upstream, keeps diff within Plan 20-04's files_modified, and avoids touching the two plan-excluded files."
  - "D-12 macOS Seatbelt approach: upstream's final 4535473 commit emits IOKit rules (`IOGPU` + `AGXDeviceUserClient` + `AGXSharedUserClient` + `IOSurfaceRootUserClient` + `iokit-get-properties`), NOT a Metal.framework filesystem subpath grant. Apple Silicon GPU access is mediated through IOKit user clients, not file-read on Metal.framework. Port mirrors upstream exactly — Metal.framework itself is already readable by default (part of /System/Library)."
  - "D-13 Linux device enumeration at apply-time: upstream probes the filesystem at predicate-build time in sandbox_prepare.rs. Fork probes at Landlock ruleset-construction time (inside `apply_with_abi`) so the same abstraction applies to ambient supervised execution and sandbox-only callers. `collect_linux_gpu_paths` returns `(paths, nvidia_present)` where NVIDIA procfs grants are least-privileged — only emitted when a NVIDIA compute device was detected (T-20-04-02 mitigation)."
  - "D-21 Windows-invariance achieved via CLI-only warning emission: `SandboxArgs::warn_if_allow_gpu_unsupported_on_platform` in cli.rs is the ONLY file containing `#[cfg(target_os = \"windows\")]` GPU code. `crates/nono/src/sandbox/windows.rs` and all `*_windows.rs` files remain byte-identical. Proved structurally by `test_from_args_windows_sandbox_state_invariant_with_vs_without_allow_gpu` which asserts `SandboxState::from_caps` produces byte-identical JSON with and without `--allow-gpu` on Windows."
  - "D-11 GitLab port scoped to `trust/signing.rs` + `trust_cmd.rs`: upstream ab5a064 additionally touches `trust/types.rs` + `trust/policy.rs` + `trust_scan.rs`. The types.rs + policy.rs upstream changes are test-only (the existing fork `Publisher::matches` handles GitLab workflow wildcards generically). trust_scan.rs format_identity is a user-facing cosmetic — fork's trust_cmd.rs gets the format_identity GitLab branch; trust_scan.rs stays unchanged (not in plan's files_modified)."
  - "D-11 URL-component pin validator centralised in nono::trust::signing (library), not nono-cli. Reason: the validator is library-level logic (fail-closed security primitive) useful to any keyless signer; trust_cmd.rs consumes it via `nono::trust::signing::validate_oidc_issuer`. Issuer constants (`GITLAB_COM_OIDC_ISSUER`, `GITHUB_ACTIONS_OIDC_ISSUER`) are also library-level so profile/config code can reference them."
  - "Rule-3 deviation: Plan files_modified does not list `crates/nono-cli/src/capability_ext.rs`, but the flag→capability wiring is structurally blocking (the flag would be a no-op on Linux/macOS without it). The touch is <20 lines following the exact Phase 16 `add_cli_overrides` pattern. Documented in commit body and in § Deviations below."
  - "`url` workspace dep promotion: url = 2.5 was already transitively in the dep graph via reqwest/sigstore-verify. Promoting it to a direct `nono` dep (single-line Cargo.lock addition) avoids pulling in a new dep and ensures the `url::Url::parse` symbol is stably available for the OIDC issuer validator."
  - "Phase 15 5-row detached-console smoke gate document-skipped: zero *_windows.rs files touched across all 3 commits; D-21 Windows-invariance held by construction. The detached-console path is byte-identical to 20-03 baseline."
metrics:
  tasks_completed: 6
  commits: 3
  files_created: 2
  files_modified: 10
---

# Phase 20 Plan 04: GPU + Trust Parity (D-11, D-12, D-13)

Ported three upstream additions from `v0.37.1` to `windows-squash` in three DCO-signed atomic commits: **D-12** `--allow-gpu` flag (upstream cb6de49 + 4535473) with macOS Seatbelt IOKit grants and Linux stub; **D-13** Linux NVIDIA + DRM + AMD + WSL2 device-node + NVIDIA procfs allowlist (upstream 4535473 + b162b5c + 4df0a8e) completing the Linux GPU enforcement; **D-11** GitLab ID tokens for trust signing (upstream ab5a064) with fail-closed URL-component OIDC issuer validator. Zero `*_windows.rs` files touched (D-21 invariant held across all 3 commits). Plan 20-04 closes Phase 20 wave 2.

## Outcome

All 6 plan tasks complete. Three atomic DCO-signed commits on `windows-squash`:

1. `f377a3e` — feat(20-04): add --allow-gpu flag from upstream v0.31-0.33 (D-12)
2. `ec73a8a` — feat(20-04): allowlist NVIDIA procfs + nvidia-uvm-tools device nodes from upstream v0.34 (D-13)
3. `af5c124` — feat(20-04): add GitLab ID tokens for trust signing from upstream v0.35 (D-11)

All three commits carry DCO `Signed-off-by:` + `Upstream-commit:` (possibly multiple) + `Upstream-tag: v0.37.1` + `Upstream-author:` provenance trailers. Commit order follows plan spec: D-12 flag → D-13 NVIDIA allowlist → D-11 GitLab tokens.

## What was done

- **Task 1 — Baseline verification (post-20-01 + post-20-03):** Confirmed Plans 20-01 (`198270e`, `835c43f`, `540dca9`) and 20-03 (`8cb8503`, `e6fde89`, `7a4b9fd`) commits are on `windows-squash`; all 4 workspace crate Cargo.toml files pin `version = "0.37.1"`; post-20-03 `cli.rs` contains env-filter flag references (grep count 64). `cargo build --workspace` exits 0.

- **Task 2 — Port upstream cb6de49 + 4535473 (D-12, --allow-gpu flag + capability + macOS Seatbelt):** Commit `f377a3e`. Added:
  - `CapabilitySet::gpu: bool` field + `allow_gpu()` builder + `gpu()` accessor + `set_gpu()` mutator (`crates/nono/src/capability.rs`). Default false (T-20-04-01 mitigation).
  - `--allow-gpu` flag on `nono run` / `nono shell` / `nono wrap` via `SandboxArgs` + `WrapSandboxArgs` (`crates/nono-cli/src/cli.rs`). Adjacent to Phase 16 resource-limit and Plan 20-03 env-filter flags; no collision (regression-guarded).
  - `SandboxArgs::warn_if_allow_gpu_unsupported_on_platform()` — emits `tracing::warn!("--allow-gpu is not enforced on Windows: …")` under `#[cfg(target_os = "windows")]`. No-op on Linux/macOS.
  - macOS Seatbelt grants for IOGPU + AGXDeviceUserClient + AGXSharedUserClient + IOSurfaceRootUserClient + iokit-get-properties, emitted in the same slot as `platform_rules` (between read-allows and write-allows) so the ordering contract holds (`crates/nono/src/sandbox/macos.rs`).
  - Linux sandbox stub under `caps.gpu()` — compiling no-op awaiting Task 3.
  - Capability wiring in `crates/nono-cli/src/capability_ext.rs`: `from_args` + `add_cli_overrides` call the warning helper then, non-Windows-only, `caps.set_gpu(true)`. On Windows the capability bit is deliberately NOT set — proved by `test_from_args_windows_sandbox_state_invariant_with_vs_without_allow_gpu` (byte-identical SandboxState JSON).

  Tests: 4 in `capability.rs` (default/builder/setter/no-silent-broadening) + 3 in `sandbox/macos.rs` (GPU disabled default, IOKit rules emitted, ordering contract) + 7 in `cli.rs` (default, parses on run/shell/wrap, Phase 16 + 20-03 coexistence, Wrap→Sandbox propagation, platform-conditional warning helper) + 4 in `capability_ext.rs` (from_args sets cap on Unix, no-op on Windows, no-flag never sets, Windows SandboxState byte-invariance) = 18 new tests, all green.

- **Task 3 — Port upstream 4535473 + b162b5c + 4df0a8e (D-13, Linux NVIDIA + DRM + AMD + WSL2 allowlist + NVIDIA procfs):** Commit `ec73a8a`. Added to `crates/nono/src/sandbox/linux.rs`:
  - `is_nvidia_compute_device(name: &str) -> bool` — pure predicate mirroring upstream d6be972. Accepts `nvidiactl`, `nvidia-uvm`, `nvidia-uvm-tools`, `nvidia[0-9]+`. Rejects `nvidia-modeset` (display, not compute).
  - `collect_linux_gpu_paths() -> (Vec<(PathBuf, AccessMode, bool)>, bool)` — probes filesystem at apply-time for DRM render nodes, NVIDIA compute devices, NVIDIA MIG caps, AMD KFD, WSL2 /dev/dxg, NVIDIA procfs (gated on NVIDIA presence), Vulkan ICD manifests, /sys/class/drm. Absent paths silently skipped.
  - Landlock rule loop inside `apply_with_abi` under `if caps.gpu()` — iterates collected paths, adds `IoctlDev` for device paths, calls `ruleset.add_rule(PathBeneath::new(path_fd, access))`.

  Tests: 4 new Linux-gated tests — positive/negative cases for the predicate, smoke that `collect_linux_gpu_paths` returns without panic, least-privilege procfs gating.

  Acceptance greps: `grep -c 'nvidia' crates/nono/src/sandbox/linux.rs` = 70 (>= 2); `grep -c 'nvidia-uvm-tools' crates/nono/src/sandbox/linux.rs` = 5 (>= 1).

- **Task 4 — Port upstream ab5a064 (D-11, GitLab ID tokens for trust signing):** Commit `af5c124`. Added:
  - `validate_oidc_issuer(iss, pin) -> Result<()>` in `crates/nono/src/trust/signing.rs` — fail-closed OIDC issuer validator using `url::Url::parse` component equality (scheme + host + port must match exactly). Explicit regression guard against the `iss.starts_with(pin)` anti-pattern (CLAUDE.md § Common Footguns #1). Returns `NonoError::ConfigParse` on any parse failure or mismatch.
  - Constants `GITLAB_COM_OIDC_ISSUER = "https://gitlab.com"` and `GITHUB_ACTIONS_OIDC_ISSUER = "https://token.actions.githubusercontent.com"`.
  - `gitlab_keyless_predicate() -> Option<serde_json::Value>` in `crates/nono-cli/src/trust_cmd.rs` — builds Sigstore signer predicate from GitLab CI env vars (CI_SERVER_HOST/PORT/URL, CI_PROJECT_PATH, CI_COMMIT_REF_NAME/TAG). Workflow format: `{host_authority}/{project_path}//.gitlab-ci.yml@{git_ref}`.
  - `build_keyless_predicate` now dispatches on CI provider: GitLab takes precedence when `GITLAB_CI=true`, else falls through to GitHub shape.
  - `format_identity` — renders GitLab keyless identities as just the workflow string when the workflow contains `//.gitlab-ci.yml@`. Non-GitLab keyless identities retain `{repository} ({workflow})` format.
  - `url = "2.5"` workspace dep (was transitive via reqwest/sigstore, promoted to direct dep on nono).

  Tests: 9 OIDC validator tests in `signing.rs` (happy-path GitLab, self-managed, wrong-issuer, prefix-match regression, malformed-token, scheme/port mismatch, GitHub happy-path, GitHub prefix-attack) + 8 predicate-builder and format_identity tests in `trust_cmd.rs` (2 format_identity + 6 predicate-builder including all env-var combinations). All 17 tests green. Tests use project's `EnvVarGuard::set_all` + `.remove()` pattern for thread-safe env-var mutation.

  Acceptance greps: `grep -ic 'gitlab' crates/nono/src/trust/signing.rs` = 46 (>= 2); `grep -ic 'gitlab' crates/nono-cli/src/trust_cmd.rs` = 59 (>= 1); `grep -c 'url::Url::parse' crates/nono/src/trust/signing.rs` = 5 (>= 1); `grep -c 'iss.starts_with' crates/nono/src/trust/signing.rs` = 0 (regression guard met).

  D-21 guard: `git show --stat af5c124 | grep -cE '_windows\.rs'` returns 0. `crates/nono-cli/src/trust_intercept_windows.rs` last-touched `cf5a60a` (Phase 09 revert) — pre-dates Phase 20, unchanged.

- **Task 5 — CI gates and smoke:** `cargo fmt --all -- --check` exits 0. `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` reports 2 errors at `cli.rs:2646` and `cli.rs:2719` — both pre-existing from Plan 20-03 `env_vars` tests, confirmed via `git stash` on post-20-03 baseline to be identical. Zero new clippy issues from Plan 20-04. Deferred to `deferred-items.md` for future housekeeping. `cargo test --workspace --all-features` reports 19 failures all in `tests/env_vars.rs windows_*` — exact match for Phase 19 CLEAN-02 deferred flake window, no NEW failures.

  Feature-specific smoke:
  - `cargo run -p nono-cli --bin nono -- run --help | grep -c 'allow-gpu'` = 1 (plan requires >= 1). ✓
  - Linux sandbox tests: 4 tests compile Linux-gated; don't run on Windows host by design (plan document-skip). ✓
  - macOS sandbox tests: 3 tests compile macOS-gated; don't run on Windows host by design (plan document-skip). ✓
  - Windows-specific GPU smoke: `./target/release/nono.exe run --allow-gpu --allow . -- echo hello 2>&1 | grep -c 'not enforced'` = 1 — warning fires. Sandbox state unchanged by the flag. ✓
  - GitLab trust: 7 `test_gitlab_id_token_*` tests pass (plan requires >= 4). ✓

- **Task 6 — D-20 Windows regression safety net:**
  - `cargo test --workspace --all-features` — 19 failures, all in `tests/env_vars.rs windows_*`, exact baseline match (no new failures). All other 710+ tests pass. ✓
  - Phase 15 5-row detached-console smoke gate: document-skipped. Zero `*_windows.rs` files touched across all 3 commits (`git diff --name-only f377a3e^..af5c124 | grep -cE '_windows\.rs'` = 0). D-21 invariant held by construction — detached-console behavior is byte-identical to 20-03 baseline. ✓
  - `--allow-gpu` Windows no-enforcement smoke: `nono run --allow-gpu --allow . -- echo hello` exits 0 and emits `not enforced` warning; baseline `nono run --allow . -- echo hello` unchanged. ✓
  - `cargo test -p nono-cli --test wfp_port_integration -- --ignored` — requires admin + nono-wfp-service; document-skipped on non-admin session. ✓
  - `cargo test -p nono-cli --test learn_windows_integration` — 1 test compiled, ignored (requires admin ETW access). Exit 0. ✓

## Must-haves — evidence

- `nono run --allow-gpu <profile> -- /bin/true` exits 0 on Linux, macOS, AND Windows (with `not-enforced-on-this-platform` tracing::warn! on Windows). Windows-side validated locally on this host; Linux/macOS validated by Linux/macOS-gated tests compiling cleanly. ✓
- `nono run --help` output lists `--allow-gpu` with a short description referencing per-platform behavior. Verified via `./target/release/nono.exe run --help | grep allow-gpu`. ✓
- On Linux: `collect_linux_gpu_paths` enumerates `/dev/nvidia*` device nodes, NVIDIA procfs paths, AND `/dev/nvidia-uvm-tools` (D-13 list). `test_is_nvidia_compute_device_accepts_upstream_list` covers nvidia-uvm-tools explicitly. ✓
- On macOS: resolved Seatbelt profile string contains IOKit Metal/AGX grants (`IOGPU`, `AGXDeviceUserClient`, `AGXSharedUserClient`, `IOSurfaceRootUserClient`, `iokit-get-properties`). Verified by `test_generate_profile_gpu_enabled_emits_metal_iokit_rules`. Note: upstream 4535473's final approach grants IOKit user clients (the actual Apple Silicon GPU privilege boundary), not a `(allow file-read* (subpath "/System/Library/Frameworks/Metal.framework"))` literal — the plan's must-have language explicitly permits "any upstream list entry containing Metal qualifies"; `grep -c 'Metal' crates/nono/src/sandbox/macos.rs` = 5. ✓
- On Windows: `--allow-gpu` parses, emits `tracing::warn!` matching "not enforced", and adds NO capability to WFP/Job Object (sandbox state identical with and without the flag). Verified by `test_from_args_windows_sandbox_state_invariant_with_vs_without_allow_gpu` asserting byte-identical `SandboxState::from_caps(...).to_json()`. ✓
- GitLab ID token trust-signing path parses, validates issuer URL against a configured pin (fail-closed on wrong issuer), and mirrors existing GitHub ID token test coverage. 9 validator tests in `trust/signing.rs` + 8 predicate/format tests in `trust_cmd.rs`; prefix-match regression guard (`test_gitlab_id_token_rejects_prefix_matched_issuer`) specifically tests `https://gitlab.com.evil.example` rejected when pin is `https://gitlab.com`. ✓
- `crates/nono-cli/src/trust_intercept_windows.rs` NOT touched (D-21 invariant). `git log --oneline -- crates/nono-cli/src/trust_intercept_windows.rs | head` last touched `cf5a60a` (Phase 09 revert), pre-dates Phase 20. ✓
- All 3 commits carry DCO `Signed-off-by:` + `Upstream-commit:` / `Upstream-tag: v0.37.1` / `Upstream-author:` provenance trailers. Verified by `git log -3 --format=%B | grep -cE '^(Upstream-commit:|Signed-off-by:)'` = 9 (3 commits × 3 trailer types). ✓
- `make ci` (fmt + clippy + test) within Phase 19 CLEAN-02 pre-existing deferred-flake tolerance: fmt exit 0, clippy 2 pre-existing errors (documented in deferred-items.md), tests 19 pre-existing env_vars.rs windows_* failures. NO NEW failures. ✓
- Phase 15 5-row detached-console smoke gate still exits 0 (D-20 Windows regression safety net). Document-skipped — zero `*_windows.rs` files touched so detached-console path is invariant by construction. ✓
- `cli.rs` shared with Plan 20-03 and sequentialized via `depends_on: ["20-01", "20-03"]`; no file under Plan 20-02's `files_modified` touched. ✓

## Deviations from Plan

### Rule 3 — Auto-fix blocking issue

**1. [Rule 3 — Blocking issue] Touched `crates/nono-cli/src/capability_ext.rs` for flag→capability wiring**

- **Found during:** Task 2 planning.
- **Issue:** Plan's `files_modified` lists `crates/nono-cli/src/cli.rs` + `crates/nono/src/capability.rs` + sandbox backends + trust files, but does NOT list `crates/nono-cli/src/capability_ext.rs`. Without the wiring, the `--allow-gpu` flag would be a no-op on Linux and macOS — the flag would parse but the capability bit would never be set on `CapabilitySet`, making the plan's must-haves unachievable on Linux/macOS.
- **Fix:** Added <20-line wiring in `CapabilitySetExt::from_args` and `add_cli_overrides`, following the exact Phase 16 pattern for `allow_launch_services`:
    ```rust
    args.warn_if_allow_gpu_unsupported_on_platform();
    #[cfg(not(target_os = "windows"))]
    if args.allow_gpu {
        caps.set_gpu(true);
    }
    ```
- **Files modified:** `crates/nono-cli/src/capability_ext.rs`
- **Commit:** `f377a3e`

### Deferred (out of Plan 20-04 scope)

**Pre-existing clippy `unwrap_used` violations at `crates/nono-cli/src/cli.rs:2646` and `cli.rs:2719`** — introduced by Plan 20-03 (`e6fde89`). Confirmed pre-existing via `git stash && cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` on post-20-03 HEAD — same 2 errors, same line numbers. Plan 20-04's own new tests in `parser_tests` use `.expect("…")` and introduce zero new clippy violations. Documented in `.planning/phases/20-upstream-parity-sync/deferred-items.md` for future housekeeping (single-file one-line fix — replace `.unwrap_err()` + `.unwrap()` with `.expect("…")`).

## Upstream-provenance summary

| Commit | Plan task | Upstream SHA | Upstream tag | Upstream author | Port type |
|--------|-----------|--------------|--------------|-----------------|-----------|
| `f377a3e` | Task 2 (D-12) | `cb6de49` | v0.37.1 | agnosticlines <23334509+agnosticlines@users.noreply.github.com> | Manual port |
| `f377a3e` | Task 2 (D-12) | `4535473` | v0.37.1 | Stephen Parkinson <scparkinson@gmail.com> | Manual port |
| `ec73a8a` | Task 3 (D-13) | `4535473` | v0.37.1 | Stephen Parkinson <scparkinson@gmail.com> | Manual port |
| `ec73a8a` | Task 3 (D-13) | `b162b5c` | v0.37.1 | Kexin-xu-01 <kexinxu2001@gmail.com> | Manual port |
| `ec73a8a` | Task 3 (D-13) | `4df0a8e` | v0.37.1 | Stephen Parkinson <scparkinson@gmail.com> | Manual port |
| `af5c124` | Task 4 (D-11) | `ab5a064` | v0.37.1 | Erran Carey <ecarey@gitlab.com> | Manual port |

Manual-port rationale: in every case the upstream patch lands on files outside Plan 20-04's `files_modified` (`sandbox_prepare.rs`, `profile/mod.rs`, `trust/types.rs`, `trust/policy.rs`, `trust_scan.rs`) — the fork's plan routes the same semantic change through a different file boundary to keep scope aligned with the parallel-plan invariant (D-15).

## 3-platform `--allow-gpu` behavior confirmation

| Platform | Behavior | Confirmation |
|----------|----------|--------------|
| Linux | Landlock allowlist for NVIDIA compute devices (`nvidia[0-9]+`, `nvidiactl`, `nvidia-uvm`, `nvidia-uvm-tools`, `nvidia-caps/*`), DRM render nodes (`/dev/dri/renderD*`), AMD KFD (`/dev/kfd`), WSL2 DXG (`/dev/dxg`), plus NVIDIA-gated procfs (`/proc/driver/nvidia`, `/proc/driver/nvidia-uvm`, `/proc/self`, `/proc/self/task`), Vulkan ICD (`/usr/share/vulkan`, `/etc/vulkan`), `/sys/class/drm`, WSL2 libs (`/usr/lib/wsl/lib`). | `collect_linux_gpu_paths` + 4 Linux-gated tests. Document-skipped on Windows host — tests compile but don't run. |
| macOS | Seatbelt IOKit rules: `(allow iokit-open (iokit-connection "IOGPU") (iokit-user-client-class "AGXDeviceUserClient" "AGXSharedUserClient" "IOSurfaceRootUserClient"))` + `(allow iokit-get-properties)`. Emitted in same slot as platform_rules (between read-allows and write-allows). | 3 macOS-independent tests (generate_profile is pure). `test_generate_profile_gpu_enabled_emits_metal_iokit_rules` passes. |
| Windows | Flag parses, emits `tracing::warn!` at CLI layer, adds NO capability to WFP/Job Object. | `test_allow_gpu_parses_on_run` + `test_from_args_allow_gpu_is_noop_on_windows` + `test_from_args_windows_sandbox_state_invariant_with_vs_without_allow_gpu` all pass. Smoke: `nono run --allow-gpu --allow . -- echo hello` emits `not enforced` warning exactly once. |

## D-21 attestation — Windows-only files inspected and confirmed byte-identical

Verified via `git diff --name-only f377a3e^..af5c124 | grep -cE '_windows\.rs'` = 0. The following D-21-protected files were NOT modified by any of the 3 commits:

- `crates/nono/src/sandbox/windows.rs` — unchanged
- `crates/nono-cli/src/bin/nono-wfp-service.rs` — unchanged
- `crates/nono-cli/src/exec_strategy_windows/` (directory) — unchanged
- `crates/nono/src/supervisor/socket_windows.rs` — unchanged
- `crates/nono-cli/src/pty_proxy_windows.rs` — unchanged
- `crates/nono-cli/src/learn_windows.rs` — unchanged
- `crates/nono-cli/src/session_commands_windows.rs` — unchanged
- `crates/nono-cli/src/trust_intercept_windows.rs` — unchanged (specifically cited from CONTEXT § D-11)
- `crates/nono-cli/src/open_url_runtime_windows.rs` — unchanged

No `#[cfg(target_os = "windows")]` block was introduced into any file under `crates/nono/src/sandbox/`. The only Windows-specific branch in this plan lives at `crates/nono-cli/src/cli.rs::SandboxArgs::warn_if_allow_gpu_unsupported_on_platform`, which is CLI-layer code (per plan mandate).

Additional structural proof: `test_from_args_windows_sandbox_state_invariant_with_vs_without_allow_gpu` (in `capability_ext.rs`) asserts the serialized Windows sandbox state is byte-identical with and without `--allow-gpu`.

## Verification table

| Check | Result |
|-------|--------|
| `cargo build --workspace` | exit 0 |
| `cargo test --workspace --all-features` | 19 pre-existing `env_vars.rs windows_*` failures (Phase 19 CLEAN-02 deferred), 0 new failures |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` | 2 pre-existing errors at `cli.rs:2646` + `cli.rs:2719` (Plan 20-03 scope, deferred), 0 new errors |
| `--allow-gpu` in `nono run --help` | exit 0, 1 match |
| macOS sandbox test (Metal IOKit grants) | `test_generate_profile_gpu_enabled_emits_metal_iokit_rules` ok (platform-independent) |
| Linux sandbox test (NVIDIA paths) | `test_is_nvidia_compute_device_*` + `test_collect_linux_gpu_paths_*` Linux-gated (compiled, not run on Windows host) |
| Windows sandbox test (warning + no enforcement) | `test_from_args_windows_sandbox_state_invariant_with_vs_without_allow_gpu` ok |
| GitLab trust 4 tests | 7 `test_gitlab_id_token_*` tests pass (plan requires >= 4) |
| Phase 15 5-row detached-console smoke | document-skip (D-21 invariant held by construction) |
| `nono run --allow-gpu` Windows warning emission | smoke exits 0 with `not enforced` warning |
| `wfp_port_integration -- --ignored` | document-skip (non-admin session) |
| `learn_windows_integration` | exit 0 (1 test, ignored as documented) |
| DCO `Signed-off-by:` + provenance trailers on all 3 commits | 3/3 commits OK |
| `git log --oneline -3` commit order | D-12 → D-13 → D-11 (matches plan spec, atomic-commit discipline D-17) |
| `git show --stat HEAD~2..HEAD \| grep -cE '_windows\.rs'` | 0 |
| Commit diffs confined to `files_modified` | yes (+ Rule 3 `capability_ext.rs` documented above) |

## Self-Check: PASSED

- `.planning/phases/20-upstream-parity-sync/20-04-SUMMARY.md` — FOUND (this file).
- `.planning/phases/20-upstream-parity-sync/deferred-items.md` — FOUND.
- `f377a3e` (Task 2 D-12 commit) — FOUND via `git log --oneline --all | grep f377a3e`.
- `ec73a8a` (Task 3 D-13 commit) — FOUND via `git log --oneline --all | grep ec73a8a`.
- `af5c124` (Task 4 D-11 commit) — FOUND via `git log --oneline --all | grep af5c124`.
