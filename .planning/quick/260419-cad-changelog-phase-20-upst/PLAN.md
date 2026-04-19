---
slug: 260419-cad-changelog-phase-20-upst
created: 2026-04-19
type: docs
files_modified:
  - CHANGELOG.md
---

# Quick: CHANGELOG entries for Phase 20 (UPST-01..04)

## Objective

Add Phase 20 entries to the `[Unreleased]` section of `CHANGELOG.md` so the rustls-webpki security upgrade + upstream parity ports land in the next release notes. Single-file edit, no code changes.

## Scope

Add entries under the existing `### Documentation`, `### Features`, and `### Security` (new) subsections:

- **Security:** rustls-webpki 0.103.10 → 0.103.12 (RUSTSEC-2026-0098, RUSTSEC-2026-0099) — Plan 20-01.
- **Features (macOS / Linux):** `--allow-gpu` flag with per-platform dispatch (Linux NVIDIA + DRM + AMD KFD + WSL2 DXG device nodes + NVIDIA procfs Landlock allowlist; macOS Seatbelt IOKit Metal/AGX grants; Windows CLI-layer `tracing::warn!` + byte-identical sandbox state — D-12/D-13) — Plan 20-04.
- **Features (credentials):** `keyring://service/account[?decode=go-keyring]` URI scheme on `crates/nono/src/keystore.rs` with fail-closed validator and path-traversal rejection — Plan 20-03.
- **Features (CLI):** `--env-allow`/`--env-deny` pattern flags on `nono run`/`nono shell`/`nono wrap` with parse-time fail-closed validator; patterns propagate via `SandboxArgs`/`WrapSandboxArgs` — Plan 20-03 (full profile-surface wiring deferred per SUMMARY).
- **Features (trust):** GitLab ID tokens for trust signing with `url::Url` component-equality issuer validator (fail-closed on prefix-match anti-pattern) — Plan 20-04.
- **Features (hooks):** `.claude.json` symlink wiring at Claude Code hook install time with canonicalized root-containment validation and Windows fail-open behavior on unprivileged symlink creation — Plan 20-02.
- **Refactor:** Profile `extends` cycle-guard end-to-end regression tests (self-reference + indirect cycle + linear chain) — Plan 20-02.
- **Build:** Workspace crate versions realigned `0.30.1` → `0.37.1` across `nono`, `nono-cli`, `nono-proxy`, `nono-ffi` (bindings/c reconciled from `0.1.0`) — Plan 20-01.
- **Miscellaneous:** `command_blocking_deprecation` module backported from upstream v0.33+ and wired into CLI startup as a warnings-only surface (no enforcement change) — Plan 20-03.

## Tasks

1. Append the above entries to `CHANGELOG.md [Unreleased]` in the appropriate subsections, preserving upstream naming convention `*(scope)*`.
2. Commit with DCO sign-off — `docs(changelog): add Phase 20 entries for UPST-01..04 to [Unreleased]`.

## Success criteria

- [ ] CHANGELOG.md `[Unreleased]` contains entries for all 4 Phase 20 plans.
- [ ] Entries reference the upstream provenance where applicable (commit/tag) but stay concise.
- [ ] Commit carries `Signed-off-by:` trailer.
- [ ] No code files changed.
