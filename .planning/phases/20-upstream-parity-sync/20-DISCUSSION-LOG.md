# Phase 20: Upstream Parity Sync (UPST) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-19
**Phase:** 20-upstream-parity-sync
**Areas discussed:** Merge strategy & target version, Feature scope, Plan decomposition, Windows-protection & crate versioning

---

## Merge strategy & target version

### Q1 — Primary merge strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Hybrid per-feature | Cherry-pick clean commits, manual port diverged files, rebase for simple cases | ✓ |
| Rebase onto upstream/v0.37.1 | Clean history; heavy conflicts on Windows-diverged files | |
| Cherry-pick individual commits | Targeted; tedious across 6 minor releases | |
| Manual port (read-and-replay) | Slowest; zero risk of corrupting fork behavior | |

### Q2 — Target version

| Option | Description | Selected |
|--------|-------------|----------|
| 0.37.1 | Latest; includes RUSTSEC fix + env-var filtering + Seatbelt rules | ✓ |
| 0.36.0 | Baseline stated in COMPARISON.md; no security fix | |
| Split target | 0.37 for security, 0.36 for features | |

### Q3 — Security-fix sequencing

| Option | Description | Selected |
|--------|-------------|----------|
| Land security fix FIRST, own plan | Plan 20-01 = rustls-webpki only; shippable standalone | ✓ |
| Bundle with credential/keystore work | Higher blast radius; fewer commits | |
| Any order — planner decides | Defers the decision to dependency analysis | |

### Q4 — Branch strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Directly on windows-squash | Same branch as Phases 16–19; no proliferation | ✓ |
| Separate sync branch → PR | Isolates absorption; extra coordination | |
| Worktree + squash at end | Loses per-feature traceability | |

**Notes:** All 4 recommended options selected. The hybrid + 0.37.1 + security-first + on-branch combination lines up with the fork's Phase 16/19 discipline and lets the RUSTSEC fix ship inside v2.1 even if later plans drag.

---

## Feature scope

### Q1 — Must-land security / stability fixes (multi-select)

| Option | Selected |
|--------|----------|
| rustls-webpki 0.103.12 (RUSTSEC-2026-0098/0099) | ✓ |
| Profile extends infinite recursion fix | ✓ |
| Unix domain socket allow in restricted net modes | — |
| Claude-code token refresh via .claude.json symlink | ✓ |

### Q2 — Should-land credential / ergonomics items (multi-select)

| Option | Selected |
|--------|----------|
| keyring:// URI scheme + ?decode=go-keyring (0.36) | ✓ |
| Environment variables filtering (0.37 #688) | ✓ |
| Learn: print profile JSON fallback on save failure (0.37) | — |
| command_blocking_deprecation (0.33) | ✓ |

### Q3 — Unix/macOS-specific refinements (multi-select)

| Option | Selected |
|--------|----------|
| macOS Seatbelt keychain specific-op rules (0.37) | — |
| macOS Mach IPC denies + atomic-write (0.31, 0.33) | — |
| macOS claude-code launch services + keychain (0.34) | — |
| GitLab ID tokens for trust signing (0.35) | ✓ |

### Q4 — Remaining items (GPU / proxy / hooks, multi-select)

| Option | Selected |
|--------|----------|
| --allow-gpu flag (macOS / Linux / WSL2, 0.31–0.33) | ✓ |
| NVIDIA procfs + nvidia-uvm-tools device allowlist (0.34) | ✓ |
| Proxy: strip artifacts + CONNECT log severity (0.35, 0.36) | — |
| Hooks: invoke bash via env (0.37.1) | — |

**Notes:** 9 of 16 considered items selected. All 3 macOS-specific Seatbelt / launch-services refinements explicitly deferred (macOS parity is a lower priority on this Windows-focused fork). Proxy fixes deferred because `nono-proxy` has fork-specific Windows credential-injection work that needs careful cross-check. Unix domain socket sandbox fix deferred as a future backport candidate.

---

## Plan decomposition

### Q1 — Number of plans

| Option | Selected |
|--------|----------|
| 4 plans, feature-grouped | ✓ |
| 3 plans, broader bundling | |
| 6 plans, one per feature area | |
| 2 plans, minimal split | |

### Q2 — Parallelization model

| Option | Selected |
|--------|----------|
| 20-01 sequential, 02–04 parallel | ✓ |
| All plans parallel after 20-01 | |
| Strictly sequential | |
| Claude's discretion | |

### Q3 — Verification gate per plan

| Option | Selected |
|--------|----------|
| make ci + targeted smoke | ✓ |
| make ci only | |
| make ci + Windows-regression smoke | |

### Q4 — Commit discipline

| Option | Selected |
|--------|----------|
| Multiple atomic commits per plan, one per semantic change | ✓ |
| One commit per plan | |
| Per-upstream-commit preservation where feasible | |

**Notes:** Verification-gate choice was "make ci + targeted smoke"; the Windows-regression smoke layer was NOT selected as part of the per-plan gate question, but it returned as D-20 via the Windows-protection area Q3 ("Phase 15 5-row smoke + existing Windows integration tests"). Both layers coexist in the CONTEXT.md — `make ci + targeted smoke` is the per-change gate (D-16); the Phase 15 5-row + wfp/learn integration suite is the Windows-regression safety net (D-20). The planner should treat both as required.

Per-upstream-commit preservation was not the primary choice but is recommended in CONTEXT.md Specifics as the commit-body template, because it's compatible with "multiple atomic commits per plan" — every clean cherry-pick gets `Upstream-commit: <hash>` provenance in the body.

---

## Windows-protection & crate versioning

### Q1 — Handling files with heavy fork divergence

| Option | Selected |
|--------|----------|
| Manual port per-file, document rationale | ✓ |
| Take upstream wholesale, re-integrate fork changes | |
| Skip heavily diverged files, port only what's clean | |

### Q2 — Fork crate-version bump

| Option | Selected |
|--------|----------|
| Bump all crates to 0.37.1 | ✓ |
| Keep fork at 0.30.1 with +windows marker | |
| Independent fork versioning | |
| Defer to Phase 21 / v2.2 | |

### Q3 — Windows-regression safety net

| Option | Selected |
|--------|----------|
| Phase 15 5-row smoke + existing Windows integration tests | ✓ |
| Workspace tests only | |
| Add a new Phase 20 Windows-parity audit plan | |

### Q4 — Upstream change touching windows-only file path

| Option | Selected |
|--------|----------|
| Upstream can't touch windows-only files; skip cleanly | ✓ |
| Windows files are review-required for every port | |
| Trust merge tool; resolve conflicts case-by-case | |

**Notes:** All 4 Recommended options selected. The "invariant" framing of Q4 (upstream has no Windows backend, so any Windows-file conflict is by definition a cherry-pick bug) is codified as D-21 in CONTEXT.md and must appear in every plan's PLAN.md § Non-Goals.

---

## Claude's Discretion

Explicit "you decide" areas deferred to the planner:

- Exact test-command invocations inside `make ci`
- Order of the three parallel plans (20-02, 20-03, 20-04) if a real dependency surfaces
- Commit-body provenance template format (recommended format provided in CONTEXT.md Specifics)
- Per-plan PR vs single Phase-20 PR (default: single PR matching Phase 19 pattern)
- Whether to split 20-03 further if `keyring://` manual port exceeds ~400 lines of diff

---

## Deferred Ideas

Upstream features explicitly excluded from Phase 20:
- Unix domain socket allow in restricted net modes (commit `98460a0`)
- Learn: print profile JSON fallback (commit `9e24ce1`)
- macOS Seatbelt keychain specific-op rules (commit `03cbd42`)
- macOS Mach IPC denies + atomic-write temp-file allow (0.31, 0.33)
- macOS claude-code launch services + keychain refinements (0.34)
- Proxy: strip artifacts + CONNECT log severity (0.35, 0.36)
- Hooks: invoke bash via env (commit `8b5a2ff`)

Cross-cutting items not in Phase 20:
- Fork crate renaming (`nono` → `nono-windows`)
- Retroactive REQUIREMENTS.md update to add UPST-01..04 requirement IDs
- Scheduled re-sync cadence for upstream minor releases past 0.37.1
