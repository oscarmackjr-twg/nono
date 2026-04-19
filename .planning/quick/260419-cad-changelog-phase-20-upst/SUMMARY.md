---
slug: 260419-cad-changelog-phase-20-upst
completed: 2026-04-19
status: complete
type: docs
files_modified:
  - CHANGELOG.md
---

# Quick Summary: CHANGELOG entries for Phase 20 (UPST-01..04)

Added Phase 20 entries to `CHANGELOG.md [Unreleased]` across 6 subsections (new `### Security` subsection created; existing Features / Bug Fixes / Refactoring / Miscellaneous / Build subsections extended — or created inline where missing).

## Entries added

- **Security (1 entry):** rustls-webpki `0.103.10` → `0.103.12` clearing RUSTSEC-2026-0098 + RUSTSEC-2026-0099 (plan 20-01).
- **Features (5 entries):** `--allow-gpu` flag with 3-platform dispatch (plan 20-04 D-12/D-13); `keyring://` URI scheme (plan 20-03 D-08); `--env-allow`/`--env-deny` CLI filter flags (plan 20-03 D-09); GitLab ID tokens for trust signing (plan 20-04 D-11); `.claude.json` Claude Code hook symlink (plan 20-02 D-07).
- **Refactoring (1 entry):** Profile `extends` cycle guard end-to-end regression coverage (plan 20-02 D-06).
- **Miscellaneous (1 entry):** `command_blocking_deprecation` module backport wired into CLI startup as warnings-only surface (plan 20-03 D-10).
- **Build (1 entry):** Workspace crate version realignment `0.30.1` → `0.37.1` across `nono`, `nono-cli`, `nono-proxy`, `nono-ffi` (plan 20-01 UPST-01).

Each entry follows the existing `*(scope)*` convention and references upstream commit provenance.

## Verification

- `git diff --name-only HEAD^ HEAD` lists only `CHANGELOG.md` + the two quick-task files. No code files touched.
- `wc -l` on the `[Unreleased]` block: 9 new lines added across 6 subsections.
- Existing Phase 14/15 entries preserved verbatim.

## Commit

- Single DCO-signed commit — `docs(changelog): add Phase 20 entries for UPST-01..04 to [Unreleased]`

## Self-Check: PASSED
