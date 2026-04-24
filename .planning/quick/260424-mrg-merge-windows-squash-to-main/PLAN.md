---
slug: mrg-merge-windows-squash-to-main
created: 2026-04-24
type: git-operations
path: C (consolidation now, DCO deferred)
push_policy: stage-locally-do-not-push
---

# Quick task: Merge `windows-squash` → `main` (local staging only)

**Pre-milestone for v2.2.** Consolidate v2.0 + v2.1 content into `main` on the fork so Phase 22 UPST2 cherry-picks target stable mainline, not a 447-commit integration branch.

**Path C chosen:** Consolidation now, DCO deferred. 46 unsigned commits (mostly `docs(…)` + `revert` + `fix(10)…` from own authoring) preserved as-is; DCO remediation will happen later when — and only if — a PR is opened to `always-further/nono` referencing these commits.

**Push policy:** Stage locally only. No `git push` in this task. User reviews and pushes manually.

---

## Pre-flight state (captured 2026-04-24)

| Fact | Value |
|------|-------|
| `main` tracks | `origin/main` (= 063ebad6 `Merge pull request #523 from always-further/fix-image-bui`) |
| `windows-squash` tracks | nothing — local-only branch |
| Merge-base `main` ∩ `windows-squash` | 063ebad6 (= main's HEAD) |
| Fast-forward viable? | **Yes** — main is a direct ancestor |
| Non-merge commits ahead | 447 |
| Commits missing DCO signoff | 46 (deferred per Path C) |
| Tag `v2.0` | 8fb16105 — annotated, unsigned, points inside the 447-commit range |
| Tag `v2.1` | 36700006 — annotated, unsigned, points inside the 447-commit range |
| Tags on origin | none (v2.0, v2.1 are local-only) |
| `origin/windows-squash` | does NOT exist (branch is purely local) |

---

## Step-by-step commands

### Step 1 — Confirm no uncommitted changes

```bash
git status --short
# Expected: empty. If anything shows, stash or commit before proceeding.
```

### Step 2 — Confirm branch + tag targets still match the pre-flight snapshot

```bash
git rev-parse windows-squash main
# Expected: windows-squash = 8213da64 (v2.2 roadmap commit); main = 063ebad6.

git rev-parse v2.0 v2.1
# Expected: v2.0 = 8fb16105; v2.1 = 36700006.

git merge-base main windows-squash
# Expected: 063ebad6 (= main's HEAD). Confirms fast-forward.
```

If any expected value drifted, stop and re-validate before proceeding.

### Step 3 — Fetch origin so local main is current with remote

```bash
git fetch origin
git log origin/main..main --oneline
# Expected: empty (local main is in sync with origin/main).
```

If local main has diverged from origin/main, resolve that first (likely a `git pull --ff-only origin main`).

### Step 4 — Checkout main, fast-forward merge windows-squash

```bash
git checkout main
git merge --ff-only windows-squash
# Expected: "Fast-forward. 447 files changed ..." (numbers vary).
```

Expected post-condition:
- `main` now at 8213da64 (the v2.2 roadmap commit).
- `windows-squash` and `main` point at the same commit.

### Step 5 — Verify tag targets are reachable from main

```bash
git branch --contains v2.0
# Expected: main (among others).

git branch --contains v2.1
# Expected: main (among others).

git log -1 --format='%h %s' v2.0
git log -1 --format='%h %s' v2.1
# Sanity-check the tag messages match expectations.
```

### Step 6 — Verify clippy/build sanity on the merged main (optional but recommended)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used
cargo build --workspace
# Expected: all clean. If clippy flares, investigate before pushing.
```

Skip this step if the host can't compile the Windows-targeted parts; the fast-forward did not touch any source files beyond what was already merged.

### Step 7 — STOP. Do NOT push.

Leave the state as:
- `main` advanced to 8213da64 locally.
- `windows-squash` still exists (same commit as main).
- v2.0, v2.1 tags exist locally, untouched.
- Nothing pushed to `origin`.

Human reviews before pushing. The deferred push commands are documented below.

---

## Deferred follow-up (NOT part of this task — for reference only)

When the human decides to publish:

```bash
# 1. Push main (advances origin/main by 447 commits)
git push origin main

# 2. Push tags
git push origin v2.0 v2.1

# 3. (Optional) clean up windows-squash since main now contains it
#    Leave for now — safer to keep windows-squash as a named reference
#    until you're sure nothing else references it.
```

Before pushing, consider:
- Does anything on `origin` reference the windows-squash branch? (Probably not — it's local-only.)
- Has `origin/main` advanced since pre-flight? (Re-check with `git fetch && git log origin/main..main`.)
- Are you OK publishing the 46 DCO-missing commits to a public remote? (Per Path C: yes, for the fork. DCO only matters on PR to upstream.)

---

## Deferred items (for v2.3+ or on-demand)

| Item | Condition to address |
|------|---------------------|
| DCO signoff remediation on 46 commits | Only when opening a PR to `always-further/nono` that includes these commits. Use GitHub DCO-bot remediation commits (empty-message signoffs) rather than history rewrite — preserves tags and commit SHAs. |
| Delete local `windows-squash` branch | After Path C completes AND origin/main has been updated AND no other work references the branch. |
| PR 555 status on upstream | Checked separately; if it's still open, decide whether to close it / reopen against the now-consolidated main. |

---

## Post-task bookkeeping (after Step 7 completes)

- Record this task in `.planning/STATE.md` § "Quick Tasks Completed" with commit SHA (of the merge itself — which for a fast-forward is `8213da64`, the tip commit).
- No SUMMARY.md code changes to cite; SUMMARY.md captures the mechanical merge + the deferred-items list.
- Phase 22 UPST2 can begin planning (`/gsd-plan-phase 22`) once this task's SUMMARY.md is committed, regardless of whether the user has pushed yet.
