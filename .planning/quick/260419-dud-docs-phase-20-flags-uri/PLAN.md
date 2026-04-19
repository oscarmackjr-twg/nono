---
slug: 260419-dud-docs-phase-20-flags-uri
created: 2026-04-19
type: docs
files_modified:
  - docs/cli/usage/flags.mdx
  - docs/cli/features/credential-injection.mdx
  - docs/cli/features/trust.mdx
  - docs/cli/clients/claude-code.mdx
  - crates/nono/README.md
---

# Quick: Docs additions for Phase 20 user-facing surfaces

## Objective

Add user-facing documentation for the 5 Phase 20 additions that were silently landed without corresponding doc entries. Scope restricted to docs under `docs/cli/` + one factual fix to `crates/nono/README.md` (version pin drift from 0.1 to current 0.37.x line).

## Audit result

`grep -r "allow-gpu\|env-allow\|env-deny\|keyring://\|GitLab" docs/ crates/*/README.md` returned zero matches pre-edit.

## Changes

1. **docs/cli/usage/flags.mdx**
   - Add `--allow-gpu` flag entry near the existing `--allow-launch-services` section (macOS-specific flag pattern is the closest analog).
   - Add `--env-allow` / `--env-deny` pair under the Environment / credentials group, close to `--env-credential-map`.
   - Add rows to the env-var mapping table at the bottom (NONO_ALLOW_GPU, NONO_ENV_ALLOW, NONO_ENV_DENY).

2. **docs/cli/features/credential-injection.mdx**
   - Add `keyring://` URI scheme subsection after the existing "System keyring" / `op://` / `apple-password://` subsections. Document `?decode=go-keyring` and fail-closed validator semantics.

3. **docs/cli/features/trust.mdx**
   - Add GitLab ID tokens subsection mirroring existing GitHub coverage. Document `validate_oidc_issuer` component-equality validator.

4. **docs/cli/clients/claude-code.mdx**
   - Add a short "Token refresh" note documenting that `nono setup` wires a `.claude.json` symlink at hook install time, with canonicalized root-containment validation and Windows fail-open.

5. **crates/nono/README.md**
   - Update install snippet `nono = "0.1"` → `nono = "0.37"` (factual drift fix, not Phase 20 per se but landed during audit).

## Non-goals

- No edits to `crates/nono-cli/README.md` — its Usage section is a quickstart, not a comprehensive flag reference; adding `--allow-gpu` there would bloat it.
- No edits to `crates/nono-proxy/README.md` — Phase 20 changes do not affect proxy behavior.
- No changes to the CHANGELOG (landed as separate quick task `260419-cad`).

## Commit plan

Single DCO-signed commit — `docs(cli): document Phase 20 user-facing surfaces (--allow-gpu, --env-allow/--env-deny, keyring:// URI, GitLab trust, .claude.json symlink)`. All 5 files in one commit since they form a cohesive "Phase 20 doc catch-up" unit.

## Success criteria

- [ ] `grep -r "allow-gpu" docs/` returns at least one match in `flags.mdx`.
- [ ] `grep -r "keyring://" docs/` returns matches in `credential-injection.mdx`.
- [ ] `grep -r "GitLab" docs/cli/features/trust.mdx` returns at least one match.
- [ ] `crates/nono/README.md` install snippet references `"0.37"` (or later).
- [ ] Commit carries `Signed-off-by:` trailer.
