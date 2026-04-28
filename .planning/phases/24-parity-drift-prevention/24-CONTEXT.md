---
phase: 24
phase_name: Parity-Drift Prevention
gathered: 2026-04-27
status: Ready for planning
requirements: ["DRIFT-01", "DRIFT-02"]
---

# Phase 24: Parity-Drift Prevention — Context

**Gathered:** 2026-04-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 24 delivers two pieces of tooling that together prevent the Windows-vs-macOS parity gap from re-opening as upstream ships v0.41+:

1. **`scripts/check-upstream-drift.{sh,ps1}`** (REQ-DRIFT-01) — A pair of twin scripts that report upstream commits the fork hasn't absorbed yet, grouped by file category (profile, policy, package, proxy, audit, other). Read-only inventory tool — no git mutations, no automation, no cherry-pick logic.

2. **`.planning/templates/upstream-sync-quick.md`** (REQ-DRIFT-02) — A reusable Markdown template for scaffolding upstream-sync quick tasks, with placeholders for commit ranges, cherry-pick chains, conflict-file inventories, and the Windows-specific retrofit checklist that Plan 22-03 had to invent ad-hoc.

**Out of scope (deferred):**
- Automated cherry-pick orchestration
- CI gate / GitHub Actions workflow that posts drift comments
- Conflict resolver / merge helper
- Replacing the twin-script pattern with a Rust binary

**Why this phase exists:** Without it, v0.41+ recreates the gap v2.2 just closed. The drift-check script + template make upstream absorption a weeks-scale quick task instead of a milestone-scale effort.

</domain>

<decisions>
## Implementation Decisions

### Script architecture (DRIFT-01)
- **D-01:** Twin `.sh` + `.ps1` scripts at `scripts/check-upstream-drift.sh` and `scripts/check-upstream-drift.ps1`, maintained in parallel. Matches the existing convention (e.g., `scripts/test-linux.sh` + `scripts/build-windows-msi.ps1`). No Rust crate. No WSL-only fallback for Windows.
- **D-02:** Add a `make check-upstream-drift` target that dispatches to the platform-appropriate script (mirrors `make build`/`make test`/`make ci` UX). Leaves direct invocation (`./scripts/check-upstream-drift.sh`) working as well.
- **D-03:** No CI integration in this phase. The script must be CI-consumable (acceptance #2: "structured output ... consumable by humans + CI") but the actual GHA workflow is deferred to a follow-up quick task once the JSON shape proves out in real use.

### Output format + categorization (DRIFT-01)
- **D-04:** Two output formats via `--format <table|json>` flag. Default = `table` (human-readable plain-text grouped output). `--format json` emits structured JSON for templates and CI. Markdown-table format is NOT included — the JSON consumer can re-render to markdown if needed.
- **D-05:** Categorize commits by **file path heuristics** using a small lookup table at the top of the script. Mappings (initial set, extensible):
  - `crates/nono-cli/src/profile/` or `profile.rs` → `profile`
  - `crates/nono-cli/src/package*` or `data/policy.json` (deny rules) → `package` / `policy` (split as appropriate)
  - `crates/nono-proxy/` → `proxy`
  - `crates/nono/src/audit/` or `audit_attestation*` → `audit`
  - Anything else under cross-platform paths → `other`
  No subject-line keyword scanning in v1 (deterministic + simpler; can add later if false-positive rate is high).
- **D-06:** Commits that touch MULTIPLE categories appear under each matching category in the table output. JSON output lists `categories: [...]` per commit. Header line shows `Total: N unique commits` to disambiguate from sum-of-rows.
- **D-07:** JSON output includes full diff stats per commit: `{sha, subject, author, date, additions, deletions, files_changed: [...], categories: [...]}`. Implemented via `git log --numstat`. Lets templates use stats for scope estimation (matches the `~9k insertions` headline pattern from 260424-upr SUMMARY).

### Diff-range strategy (DRIFT-01)
- **D-08:** Default range = **last-synced-tag..latest-upstream-tag**, auto-detected. Resolution logic:
  - "last-synced-tag" = highest local tag matching upstream's tag pattern (e.g., the fork's `v0.40.x` synced state, currently inferred from the `windows-squash` tag chain)
  - "latest-upstream-tag" = highest tag on `upstream/main` (e.g., `v0.40.1` today, `v0.41.0` after upstream tags it)
  Falls back to a clear error message if no upstream remote OR no tags can be resolved.
- **D-09:** `--from <ref> --to <ref>` flags ALWAYS override the default. This is required to satisfy acceptance #1 (reproduce the v0.37.1..v0.40.1 inventory deterministically).
- **D-10:** Missing `upstream` remote → exit 1 with actionable hint: `No 'upstream' remote configured. Add it with: git remote add upstream https://github.com/always-further/nono.git`. Do NOT auto-add (no silent git config mutation).
- **D-11:** Excluded paths (`*_windows.rs`, `exec_strategy_windows/`) are filtered OUT of commit reporting. A commit that touches BOTH excluded AND cross-platform files appears in the report, with only its cross-platform files listed in the per-commit detail. No separate "fork-only" bucket.

### Template location + shape (DRIFT-02)
- **D-12:** Template lives at `.planning/templates/upstream-sync-quick.md` (matches REQ-DRIFT-02's stated default). The `.planning/templates/` directory will be created as part of this phase. No GSD skill wrapper in v1 — single Markdown file, copied by maintainer into a quick-task dir.
- **D-13:** Template shape = **fillable-blanks Markdown with placeholders**. Maintainer copies to `.planning/quick/YYMMDD-xxx-upstream-sync-vX.Y/PLAN.md` and fills in `{{from_tag}}`, `{{to_tag}}`, `{{commit_count}}`, etc. Pre-populated sections:
  - Frontmatter scaffold (slug, status, type, date, range)
  - Headline + commit inventory by release section
  - **D-19 cherry-pick trailer template** (Upstream-commit/Upstream-tag/Upstream-author/Co-Authored-By/Signed-off-by × 2)
  - **Conflict-file inventory section** with the canonical conflict patterns the fork has seen (async-runtime wrapping for `load_production_trusted_root`, `validate_path_within` defense-in-depth retention, deferred enum variants like `ArtifactType::Plugin`, etc.)
  - **Windows-specific retrofit checklist** (per-feature: does Windows path exist? if not, add it; if added, gate behind `#[cfg(target_os = "windows")]`)
  - **Fork-divergence catalog** entries from prior upstream syncs (Phase 22-03 PROGRESS note's deferred-divergence patterns)
- **D-14:** Template references `make check-upstream-drift` output but does NOT auto-include it. Template section: `## Drift inventory\n\nRun \`make check-upstream-drift > drift.json\` and paste relevant entries here.` Maintainer pastes + curates the output. Clean separation: template doesn't depend on the script being invoked first.
- **D-15:** Reference from `PROJECT.md § Upstream Parity Process` (new section). Section content:
  - Pointer to `make check-upstream-drift` for inventory
  - Pointer to `.planning/templates/upstream-sync-quick.md` for scaffolding
  - Brief workflow: run drift check → copy template → fill placeholders → execute as quick task
  - Cross-link to `docs/cli/development/upstream-drift.mdx` for the long-form runbook (if it ends up larger than fits in PROJECT.md)

### Documentation
- **D-16:** REQ-DRIFT-01 acceptance #3 says `docs/cli/development/upstream-drift.md` but the existing convention in `docs/cli/development/` is **`.mdx`** (every file in that directory uses `.mdx`). Use `.mdx` for consistency. Document covers: what the script does, when to run it, how to interpret output, how to use it with the template.

### Plan structure (Claude's Discretion)
- **D-17:** Plan split is the planner's call. Roadmap suggests 1–2 plans; given the small scope (one twin-script + one template + one PROJECT.md edit + one docs file), a single combined plan or a 2-plan split (24-01 DRIFT-01 script + 24-02 DRIFT-02 template+docs) are both viable. Planner picks based on dependency analysis.

### Claude's Discretion
- Exact Make-target dispatch logic (detect Windows vs Unix and invoke the right script — likely `MAKE_RUNNER` env var or shell heuristic in the Makefile)
- Per-script CLI argument parsing implementation (getopt vs manual parsing in bash; `param()` block in PowerShell)
- JSON schema exact field names beyond the listed ones (e.g., `committer` vs `author` precedence, ISO-8601 date format, etc.)
- Exact placeholder syntax in the template (`{{NAME}}` vs `<NAME>` vs other)
- Whether to use `git log --numstat` or `git log --shortstat` for diff stats (planner picks based on parse complexity)
- Test approach for the scripts (likely sample fixtures + grep-based assertions, but specific shape is up to executor)

### Folded Todos
None — no pending todos matched Phase 24 scope.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements + scope
- `.planning/REQUIREMENTS.md` § REQ-DRIFT-01 / REQ-DRIFT-02 — Locked acceptance criteria for both deliverables (lines 267–287)
- `.planning/ROADMAP.md` § "Phase 24: Parity-Drift Prevention" (lines 124–147) — Phase goal, dependencies, plans-TBD note, success criteria
- `.planning/PROJECT.md` — Will be modified by this phase (D-15 adds new "Upstream Parity Process" section)

### Seed data the script must reproduce
- `.planning/quick/260424-upr-review-upstream-037-to-040/SUMMARY.md` — The 78-commit, 5-feature-group inventory that DRIFT-01 acceptance #1 must reproduce when run against `v0.37.1..v0.40.1`. Categorization shape (profile/policy/package/proxy/audit/other) traces back here.
- `.planning/quick/260424-upr-review-upstream-037-to-040/PLAN.md` — Companion plan for the seed data quick task

### Fork-divergence patterns the template must encode
- `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-03-PKG-PROGRESS.md` — Captures the deferred-divergence pattern (`ArtifactType::Plugin` not in fork) and `validate_path_within` defense-in-depth retention. Template's "fork-divergence catalog" section seeds from here.
- Recent fork commits showing D-19 cherry-pick trailer format: `73e1e3b8`, `adf81aec`, `869349df` (run `git log --pretty=fuller` on these to see the full trailer block)

### Existing convention models
- `scripts/test-linux.sh` and `scripts/build-windows-msi.ps1` — Reference style for the twin-script pattern (D-01)
- `Makefile` — Existing target style for D-02 (`make build`, `make test`, `make ci`)
- `docs/cli/development/*.mdx` — Existing file convention (D-16 uses `.mdx` not `.md`)

### Project standards
- `CLAUDE.md` — Tech stack constraints (no Python, Rust + Bash + PowerShell only), security non-negotiables, conventions

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable assets
- **`scripts/` directory pattern**: All twin-style scripts live here flat. New twin (`check-upstream-drift.sh`/`.ps1`) drops in alongside `test-linux.sh` and `build-windows-msi.ps1`. No new directory needed.
- **`Makefile`**: Has the dispatch pattern for `make ci` (chains clippy + fmt + tests). Adding `make check-upstream-drift` follows the same shape.
- **`.planning/quick/` directory**: Existing pattern for ad-hoc quick tasks. Template will be copied INTO this dir as new sub-directories.

### Established patterns
- **Bash scripts**: Use `#!/usr/bin/env bash`, no `set -euo pipefail` is universal but commonly present. Plain shell + standard POSIX utilities (`git`, `awk`, `sed`, `grep`).
- **PowerShell scripts**: Use `param()` blocks for CLI args with `[Parameter(Mandatory=...)]` annotations. No external modules required by existing scripts.
- **Quick task naming**: `YYMMDD-xxx-slug` format (e.g., `260424-upr-review-upstream-037-to-040`). Template's example slug uses `YYMMDD-xxx-upstream-sync-vX.Y` shape.
- **Doc convention in `docs/cli/development/`**: All `.mdx`, not `.md`. Mintlify/Docusaurus-style.

### Integration points
- `Makefile` — D-02 adds one new target
- `PROJECT.md` — D-15 adds one new section (`## Upstream Parity Process`)
- `docs/cli/development/upstream-drift.mdx` — D-16 creates one new doc file
- `.planning/templates/upstream-sync-quick.md` — D-12 creates the templates directory + the template file

### Fork-only state worth referencing in the template
- `upstream` remote already configured (`https://github.com/always-further/nono.git` per `git remote -v`)
- D-19 trailer format established and consistently used since Plan 22-01 — template should encode the exact line shape rather than describing it abstractly

</code_context>

<specifics>
## Specific Ideas

- The script's category mapping should be a **single, top-of-file lookup table** so it's trivial to add categories when the fork's surface area grows (e.g., `crates/nono/src/snapshot/` → `snapshot` category if rollback work expands). Keep it data, not code.
- The template's "fork-divergence catalog" section should explicitly include the `validate_path_within` retention pattern — it's the kind of decision that's easy to silently drop in a sync if the maintainer doesn't know the history.
- The PROJECT.md "Upstream Parity Process" section should be **short** (workflow only), with the long-form explanation in `docs/cli/development/upstream-drift.mdx`. PROJECT.md is read often; deep details live in docs.
- Acceptance #1 for DRIFT-01 ("reproduce 260424-upr inventory") implies the script's test should `diff` the script's `--format json` output against a frozen fixture derived from the SUMMARY.md. Treat the SUMMARY.md table as the ground truth; if the script's categorization differs, either the script's lookup table or the SUMMARY's categorization needs to be reconciled.

</specifics>

<deferred>
## Deferred Ideas

- **GitHub Actions weekly drift workflow** — Periodic GHA workflow that runs `check-upstream-drift` and posts an issue/comment if drift exceeds N commits. Defer to a follow-up quick task once the JSON output shape proves out in real use (D-03 rationale).
- **Cherry-pick automation / merge helper** — Tooling that walks the drift inventory and runs `git cherry-pick` per commit with auto-trailer-injection. Belongs in a future phase if upstream syncs become frequent enough to justify it.
- **Conflict resolver UX** — Interactive tool for handling the conflict patterns the template documents. Belongs in a separate phase, depends on more sync experience.
- **Rust rewrite** — If the twin scripts drift in behavior, consider a single Rust binary in v3.x. Not now — twin maintenance is acceptable for a tool that changes rarely.
- **Subject-line keyword categorization fallback** — D-05 uses path heuristics only. If false-positive rate is high in practice, add subject-line scan as a fallback. Track the rate during the next sync, decide then.

</deferred>

---

*Phase: 24-parity-drift-prevention*
*Context gathered: 2026-04-27*
