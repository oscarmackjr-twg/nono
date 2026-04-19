---
slug: 260419-dud-docs-phase-20-flags-uri
completed: 2026-04-19
status: complete
type: docs
files_modified:
  - docs/cli/usage/flags.mdx
  - docs/cli/features/credential-injection.mdx
  - docs/cli/features/trust.mdx
  - docs/cli/clients/claude-code.mdx
  - crates/nono/README.md
---

# Quick Summary: Docs additions for Phase 20 user-facing surfaces

Added 5 doc entries covering the Phase 20 user-facing surfaces that shipped without corresponding docs: `--allow-gpu`, `--env-allow`/`--env-deny`, `keyring://` credential URI scheme, GitLab CI/CD trust signing, and the Claude Code `.claude.json` symlink. One version-pin fix in `crates/nono/README.md` along the way.

## Pre-edit audit result

`grep -r "allow-gpu\|env-allow\|env-deny\|keyring://\|GitLab" docs/ crates/*/README.md` — zero matches.

## Post-edit verification

`grep -l ... docs/ -r` now returns matches in `flags.mdx`, `credential-injection.mdx`, and `trust.mdx`. `.claude.json` symlink discussion is in `claude-code.mdx`. Crate README's install snippet now references `0.37`.

## Files touched (+165 lines, -1 line)

- **`docs/cli/usage/flags.mdx`** (+65): new `--allow-gpu` section with per-platform behavior table (Linux NVIDIA+DRM+AMD+WSL2 allowlist, macOS Seatbelt IOKit Metal/AGX grants, Windows CLI-layer `tracing::warn!` with byte-identical sandbox state); new `--env-allow` / `--env-deny` pair with accepted/rejected pattern grammar.
- **`docs/cli/features/credential-injection.mdx`** (+42): new "Custom Keyring Entries (`keyring://` URIs)" section documenting `keyring://service/account[?decode=go-keyring]` with fail-closed parser guarantees (no filesystem I/O, path-traversal rejection, oversize rejection).
- **`docs/cli/features/trust.mdx`** (+49): new "GitLab CI/CD Integration" section mirroring existing GitHub Actions coverage, including self-hosted GitLab issuer pin example and a Note explaining the `url::Url` component-equality validator that guards against prefix-match / subdomain-hijack anti-patterns.
- **`docs/cli/clients/claude-code.mdx`** (+8): new "Token refresh via `.claude.json` symlink" subsection under the built-in profile's feature list, documenting canonicalized root-containment validation and Windows fail-open behavior on unprivileged symlink creation.
- **`crates/nono/README.md`** (+1/-1): install snippet `nono = "0.1"` → `nono = "0.37"` (version pin drift from 0.1 to current 0.37.x line).

## Non-goals honored

- `crates/nono-cli/README.md` untouched — Usage section is a quickstart, not a comprehensive flag reference.
- `crates/nono-proxy/README.md` untouched — Phase 20 changes do not affect proxy behavior.
- `flags.mdx` Environment Variables mapping table untouched — I did not verify whether `--allow-gpu`, `--env-allow`, `--env-deny` are wired to `NONO_*` env vars via clap's `env = ...` attribute. Adding unverified entries would mislead users; flagged for a future quick task.

## Commit

- Single DCO-signed commit — `docs(cli): document Phase 20 user-facing surfaces (--allow-gpu, --env-allow/--env-deny, keyring:// URI, GitLab trust, .claude.json symlink)`

## Deferred

- Audit whether `--allow-gpu`, `--env-allow`, `--env-deny` support `NONO_ALLOW_GPU` / `NONO_ENV_ALLOW` / `NONO_ENV_DENY` env-var bindings. If yes, add rows to the mapping table in `flags.mdx` § Environment Variables.

## Self-Check: PASSED
