---
task: cmp-upstream-036-windows-parity
status: research-only
started: 2026-04-19
owner: research-agent
---

# Task: Compare upstream nono v0.36.0 vs Windows fork (windows-squash)

## Objective

Document functional differences between upstream `always-further/nono` at tag
`v0.36.0` and this repo's `windows-squash` HEAD (`8f5927c`, post-Phase-19 v2.1
cleanup).

## Scope

- CLI command/flag surface
- Sandbox backend coverage per platform
- Platform support claims and behaviors
- Major features present on one side but not the other
- Crate versioning and toolchain drift
- Upstream activity past 0.36 (context only)

## Out of scope

- Code formatting, commit message style, test organization
- Per-line diff review of refactors that preserve behavior
- Merge planning / upstream pull-down strategy

## Method

1. Read upstream surface at tag `v0.36.0` via `git show v0.36.0:<path>` and
   `git ls-tree v0.36.0`.
2. Read fork surface from the current working tree (HEAD = `8f5927c`).
3. Diff the two with `git diff v0.36.0..HEAD` on targeted paths.
4. Cross-check `upstream/main` for post-0.36 changes (context section only).

## Output

`COMPARISON.md` in this directory — single-file functional-differences report.
No source-code changes.
