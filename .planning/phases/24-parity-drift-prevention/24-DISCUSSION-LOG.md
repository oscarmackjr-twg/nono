# Phase 24: Parity-Drift Prevention — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in `24-CONTEXT.md` — this log preserves the alternatives considered.

**Date:** 2026-04-27
**Phase:** 24-parity-drift-prevention
**Areas discussed:** Script architecture, Output format + categorization, Diff-range strategy, Template location + shape

---

## Script architecture (DRIFT-01)

### Q: Architecture
| Option | Description | Selected |
|--------|-------------|----------|
| Twin .sh + .ps1 (Recommended) | Two scripts maintained as parallel implementations. Matches existing `scripts/` convention. | ✓ |
| One Rust CLI tool | Cargo crate, eliminates twin-drift but adds build step. | |
| Bash-only + WSL note for Windows | Single .sh, Windows uses WSL. Contradicts Windows-native parity stance. | |

### Q: Location
| Option | Description | Selected |
|--------|-------------|----------|
| `scripts/check-upstream-drift.{sh,ps1}` (Recommended) | Matches REQ-DRIFT-01 enforcement clause exactly. | ✓ |
| `tools/upstream-drift/` subdirectory | Future-proofing for hypothetical growth. | |
| Inside Cargo workspace as binary crate | Only viable if architecture chose Rust. | |

### Q: Make target
| Option | Description | Selected |
|--------|-------------|----------|
| Yes — `make check-upstream-drift` (Recommended) | Matches existing Makefile UX. | ✓ |
| No — invoke the script directly | Less Makefile surface. | |

### Q: CI gate
| Option | Description | Selected |
|--------|-------------|----------|
| No — script-only this phase (Recommended) | Keep Phase 24 scoped. CI is a follow-up. | ✓ |
| Yes — periodic GHA workflow | Adds workflow + token plumbing in scope. | |

---

## Output format + categorization

### Q: Format
| Option | Description | Selected |
|--------|-------------|----------|
| Both via `--format` flag (Recommended) | Default table, opt-in JSON. Covers acceptance #2. | ✓ |
| Markdown table only | PR-paste-friendly but loses CI consumability. | |
| Plain-text table only | Simplest. JSON consumers parse text. | |

### Q: Categorize
| Option | Description | Selected |
|--------|-------------|----------|
| File path heuristics (Recommended) | Lookup table at top of script. Deterministic. | ✓ |
| Subject-line keyword scan | Closer to 260424-upr SUMMARY's manual approach. | |
| Both (path-first, subject as fallback) | Most robust, most complex. | |

### Q: Multi-category display
| Option | Description | Selected |
|--------|-------------|----------|
| Listed under each matching category (Recommended) | Maximizes discoverability when scanning by category. | ✓ |
| Listed under primary (largest LOC) category only | Cleaner totals, might miss audit-touching commits. | |
| Listed under 'multi' / 'other' category | Adds a category not in REQ enumeration. | |

### Q: JSON depth
| Option | Description | Selected |
|--------|-------------|----------|
| Full stats (Recommended) | sha, subject, author, date, additions, deletions, files_changed, categories. | ✓ |
| Minimal (sha + subject + categories only) | Smaller, pushes work onto consumers. | |

---

## Diff-range strategy

### Q: Default range
| Option | Description | Selected |
|--------|-------------|----------|
| Last-synced-tag..latest-upstream-tag (Recommended) | Auto-detect, deterministic. | ✓ |
| `upstream/main..HEAD` (live tip) | Always-current but answer changes commit-by-commit. | |
| No default — require `--from`/`--to` | Most explicit, most friction. | |

### Q: Overrides
| Option | Description | Selected |
|--------|-------------|----------|
| Yes — always allow explicit `--from`/`--to` (Recommended) | Required for acceptance #1 reproducibility. | ✓ |
| No — keep CLI minimal | Forces single mode of thinking. | |

### Q: Missing remote
| Option | Description | Selected |
|--------|-------------|----------|
| Fail with actionable hint (Recommended) | Exit 1 with `git remote add upstream …` guidance. | ✓ |
| Auto-add upstream remote if missing | Mutates user's git config without consent. | |
| Use a configurable remote name (default `upstream`) | Adds a flag with low expected use. | |

### Q: Exclusions
| Option | Description | Selected |
|--------|-------------|----------|
| Filter out (Recommended) | Per REQ; commits report only cross-platform files. | ✓ |
| Show separately as 'fork-only' bucket | Adds category not in REQ enumeration. | |
| Skip exclusion entirely | Defeats the purpose. | |

---

## Template location + shape (DRIFT-02)

### Q: Template location
| Option | Description | Selected |
|--------|-------------|----------|
| `.planning/templates/upstream-sync-quick.md` (Recommended) | Matches REQ-DRIFT-02 default. Simplest. | ✓ |
| `.claude/skills/gsd-upstream-sync/SKILL.md` | Discoverable as slash command. Higher setup. | |
| Both — markdown + skill wrapper | Maximum surface, two things to maintain. | |

### Q: Template shape
| Option | Description | Selected |
|--------|-------------|----------|
| Fillable-blanks Markdown with placeholders (Recommended) | Maps directly to REQ-DRIFT-02 acceptance #1. | ✓ |
| Bash scaffold script that creates dir + file | Lower per-use friction, higher up-front cost. | |
| Long-form prose runbook | Reads like docs, not a template. | |

### Q: Drift wiring
| Option | Description | Selected |
|--------|-------------|----------|
| Reference, don't auto-include (Recommended) | Template doesn't depend on script being invoked first. | ✓ |
| Auto-include via scaffold script | Tightly couples template + script. | |
| Both — reference + optional auto-fill | Maximum flexibility, two paths to wire. | |

### Q: Doc anchor
| Option | Description | Selected |
|--------|-------------|----------|
| `PROJECT.md § Upstream Parity Process` (Recommended) | Single discoverable spot. Matches REQ enforcement. | ✓ |
| `PROJECT.md` only (paragraph, no new section) | Lighter touch but less discoverable. | |
| `PROJECT.md` + `docs/cli/development/upstream-drift.mdx` (Recommended pair) | Short pointer in PROJECT.md, long form in docs. | |

**Note:** D-15 in CONTEXT.md combines the two "recommended" picks above — short pointer in PROJECT.md `§ Upstream Parity Process`, with cross-link to `docs/cli/development/upstream-drift.mdx` (D-16) for the long-form runbook.

---

## Claude's Discretion

- Make-target dispatch logic (Windows vs Unix detection)
- Per-script CLI argument parsing implementation
- JSON schema exact field names beyond the listed ones
- Placeholder syntax in the template (`{{NAME}}` vs `<NAME>` vs other)
- `git log --numstat` vs `--shortstat` for diff stats
- Test approach for the scripts

## Deferred Ideas

- GitHub Actions weekly drift workflow (follow-up quick task)
- Cherry-pick automation / merge helper (future phase)
- Conflict resolver UX (separate phase, more sync experience needed)
- Rust rewrite of the twin scripts (v3.x consideration)
- Subject-line keyword categorization fallback (track false-positive rate first)
