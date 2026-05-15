---
slug: sigstore-tuf-rotation
quick_id: 260511-fpn
created: 2026-05-11
type: docs-workaround
status: completed
---

# Quick task: Document Sigstore TUF root rotation workaround + track upstream upgrade

## Problem

POC user runs `nono setup --refresh-trust-root` and gets:

```
[3/5] Refreshing Sigstore trusted root...
ERROR Setup error: Failed to fetch Sigstore trusted root from
  https://tuf-repo-cdn.sigstore.dev: TUF error: TUF repository load failed:
  Failed to verify trusted root metadata:
  Signature threshold of 3 not met for role root (0 valid signatures)
```

[1/5] and [2/5] succeed (binary install + sandbox support both green); only the TUF refresh
fails. The CDN fetch itself succeeded — the failure is at signature verification.

## Root cause

`sigstore-verify 0.6.5` (the fork's pinned dep, `crates/nono/Cargo.toml:38`) embeds a TUF
trust anchor that's now stale. Sigstore has rotated TUF root keys since the crate was
published; the TUF client fetches the current `root.json` from the CDN, attempts to validate
it against its embedded anchor, and gets `0 valid signatures` because every embedded key has
been rotated out. This is **correct fail-closed behavior** (per Phase 32 D-32-01 / D-32-03
fail-closed-on-bad-trust-chain) — but the rotation has stranded the embedded anchor.

**sigstore-verify 0.6.6** (released 2026-04-29) ships PR #69 "API for fetching / using the
trust root" which likely refreshes the embedded anchor. The fix-upstream is a dependency
upgrade in the `nono` crate; that's a multi-crate workspace change (sigstore-verify,
sigstore-sign, sigstore-bundle, etc. all bumped together) with downstream risk on the
keyless sign/verify flows — out of scope for a same-day POC unblock.

## Verify-is-offline invariant lets us work around this

Phase 32 D-32-15 codified: **`load_production_trusted_root()` reads the cache file via plain
JSON deserialization (`TrustedRoot::from_file()` at `crates/nono/src/trust/bundle.rs:114`),
NOT TUF re-verification.** Only an expiry gate fires (tlog keys' `valid_for.end` in the past
→ fail). The CLI never re-fetches; it trusts the cached bytes on disk.

That means: **dropping a known-good `trusted_root.json` into
`%USERPROFILE%\.nono\trust-root\trusted_root.json` unblocks `nono trust verify --issuer
... --identity ...` immediately**, with no code change.

The repo already has a known-good capture at
`crates/nono/tests/fixtures/trust-root-frozen.json` (commit `d9969978`,
captured 2026-05-10 from `sigstore/root-signing@main`, 6787 bytes). Maintainer-of-record
captured this 1 day ago against the same sigstore root rotation the POC user is hitting —
it's the closest thing to "TUF-verified, then pinned" data we have for the current root
state.

## Scope (this quick task)

This task addresses the POC-unblock half ONLY:

1. **Add a "Known Issue: Sigstore TUF root rotation" section** to
   `docs/cli/development/windows-poc-handoff.mdx`, placed inside the existing
   `## Sigstore Trust Root Setup (one-time per user)` section right after the
   `nono setup --refresh-trust-root` instructions.
2. **Document the manual workaround** using `Invoke-WebRequest` to fetch the frozen fixture
   from the GitHub raw URL into the user's cache path.
3. **Cite the upstream upgrade follow-up** as a tracked deferred item that v2.4+ will pick up
   (sigstore-verify 0.6.5 → 0.6.6 dependency bump).

**Track the upstream upgrade as a separate v2.4 candidate** — append to
`.planning/phases/32-sigstore-integration/deferred-items.md` as `P32-DEFER-005`.

## Out of scope

- **No `sigstore-verify` upgrade.** Multi-crate workspace bump with non-trivial blast radius
  (keyless sign + verify both touch `sigstore-verify`/`sigstore-sign`/`sigstore-bundle`).
  Tracked as P32-DEFER-005 for v2.4+.
- **No new CLI flag** (e.g., `nono setup --refresh-trust-root --from-file <PATH>`). The
  workaround is a manual file drop; that's sufficient for POC. If we want the flag long-term
  it's part of the P32-DEFER-005 work.
- **No build-system changes** to bundle the fixture into the MSI. Fetching from
  `raw.githubusercontent.com` is sufficient for POC use.
- **No code changes to `crates/nono/`, `crates/nono-cli/`, or `crates/nono-shell-broker/`.**
  This is a pure docs + tracking task.

## Files touched

1. `docs/cli/development/windows-poc-handoff.mdx` — add Known Issue subsection + workaround
   block (~30 lines).
2. `.planning/phases/32-sigstore-integration/deferred-items.md` — append P32-DEFER-005 entry
   (~25 lines).

## Acceptance

- [ ] `docs/cli/development/windows-poc-handoff.mdx` has a "Known Issue" callout in the
      Sigstore Trust Root Setup section with the failure signature, root cause one-liner, and
      the workaround command.
- [ ] Workaround command tested against the actual cache path resolution
      (`%USERPROFILE%\.nono\trust-root\trusted_root.json`) and matches the fork's
      `load_production_trusted_root()` lookup (`crates/nono/src/trust/bundle.rs:147-148`).
- [ ] `Invoke-WebRequest` example uses the raw-GitHub URL pinned to a specific commit sha
      (not `main`) so the fetched file is reproducible: pin to `281f71ab` (current HEAD).
- [ ] `.planning/phases/32-sigstore-integration/deferred-items.md` has a P32-DEFER-005 entry
      describing the sigstore-verify 0.6.5 → 0.6.6 upgrade as the long-term fix, with entry
      criteria (which crates need bumping, which tests cover the surface).

## POC user workaround command (verbatim — to ship in the doc)

```powershell
# Workaround for the sigstore-verify 0.6.5 TUF root rotation issue.
# Drops a maintainer-captured, known-good trusted_root.json into the cache.
# Verify subsequently runs offline against this file (per Phase 32 D-32-01
# verify-is-offline invariant).

$cacheDir = "$env:USERPROFILE\.nono\trust-root"
New-Item -ItemType Directory -Force -Path $cacheDir | Out-Null
Invoke-WebRequest -UseBasicParsing `
  -Uri "https://raw.githubusercontent.com/oscarmackjr-twg/nono/281f71ab/crates/nono/tests/fixtures/trust-root-frozen.json" `
  -OutFile "$cacheDir\trusted_root.json"

# Then verify works:
nono trust verify `
  --issuer https://token.actions.githubusercontent.com `
  --identity '<your-identity-regex>' `
  <bundle-file>
```
