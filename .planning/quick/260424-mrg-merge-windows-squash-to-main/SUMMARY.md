---
slug: mrg-merge-windows-squash-to-main
status: complete
type: git-operations
date: 2026-04-24
path_chosen: C (consolidation now, DCO deferred)
push_policy: stage-locally-do-not-push
executed: steps 1–5 + step 7 (step 6 cargo sanity skipped — fast-forward advances to already-validated commit)
---

# Merge `windows-squash` → `main` — local staging complete

Pre-milestone for v2.2. Fast-forward merge executed locally; no push performed. Phase 22 UPST2 cherry-picks can now target stable mainline once pushed.

## Outcome

| Ref | Before | After |
|-----|--------|-------|
| `main` | `063ebad6` (tracking `origin/main`) | `8213da64` (local ahead of origin by 447 commits) |
| `windows-squash` | `8213da64` | `8213da64` (unchanged) |
| `v2.0` | `8fb16105` (annotated, local-only) | `8fb16105` (unchanged; reachable from main) |
| `v2.1` | `36700006` (annotated, local-only) | `36700006` (unchanged; reachable from main) |
| `origin/main` | `063ebad6` | `063ebad6` (unchanged — nothing pushed) |
| Working tree | clean + untracked quick-task dir | clean + untracked quick-task dir |

## Steps executed

1. ✅ `git status --short` — only untracked quick-task dir (expected).
2. ✅ Pre-flight refs verified:
   - `main` = 063ebad6, `windows-squash` = 8213da64 (v2.2 roadmap tip).
   - `v2.0^{commit}` = 8fb16105, `v2.1^{commit}` = 36700006.
   - `git merge-base main windows-squash` = 063ebad6 (confirms fast-forward).
3. ✅ `git fetch origin` clean; `git log origin/main..main` empty (local main in sync).
4. ✅ `git checkout main` + `git merge --ff-only windows-squash` — fast-forward landed. Many `create mode` lines indicate the merge applied the expected ~447 commits' worth of file additions.
5. ✅ `git branch --contains v2.0` + `git branch --contains v2.1` — both show `main` (+ windows-squash + two unrelated worktree agent refs). Tag messages match expectations.
6. ⏭️ Skipped. Fast-forward advances to a commit that already passed v2.1 phase verification gates (clippy, fmt, build); no new source state introduced.
7. ✅ Stopped. No `git push` executed.

## What stays true after this task

- `main` is a local-only advance; `origin/main` is still at `063ebad6`.
- Tags `v2.0` and `v2.1` are still annotated-unsigned and local-only.
- `windows-squash` still exists (no reason to delete yet).
- 46 DCO-missing commits are preserved as-is on main (Path C — deferred).
- No history rewritten; all SHAs referenced in STATE.md / RETROSPECTIVE.md / phase SUMMARYs remain valid.

## Deferred (do when ready)

From PLAN.md § "Deferred follow-up":

```bash
git push origin main        # advances origin/main by 447 commits
git push origin v2.0 v2.1   # publishes the milestone tags
```

Before pushing, re-verify:
- `git fetch && git log origin/main..main | head` — confirms nothing raced in.
- OK with publishing the 46 DCO-missing commits to the fork? (Path C: yes; DCO only matters on upstream PRs.)

## Deferred items list

| Item | Trigger to address |
|------|--------------------|
| DCO signoff remediation on 46 commits | Only when opening a PR to `always-further/nono` that includes these commits. Use GitHub DCO-bot remediation (empty-message signoffs) — preserves tags and commit SHAs. |
| Push main + tags to `origin` | User's call; can be done any time. |
| Delete local `windows-squash` branch | Defer until `origin/main` is pushed AND no other work references the branch. |
| PR 555 disposition on upstream | Separate question — decide whether to close it (superseded) or reopen against consolidated main. `gh` CLI is not authenticated in this environment; check manually on github.com. |

## Next

Phase 22 UPST2 can begin planning (`/gsd-plan-phase 22`) now. The cherry-pick-per-commit pattern will land on `main` (locally or post-push). Phase 24 (drift-prevention) is independent and can also be planned in parallel.
