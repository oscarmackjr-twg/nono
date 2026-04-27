# Phase 22: UPST2 — Upstream v0.38–v0.40 Parity Sync — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-27
**Phase:** 22-upst2-upstream-v038-v040-parity-sync
**Areas discussed:** Plan 22-05 conflict strategy, Working branch & origin push, 22-01 unblock granularity, Test fixture strategy

---

## Plan 22-05 conflict strategy

### Q1: Default approach for the 7-commit audit cluster

| Option | Description | Selected |
|--------|-------------|----------|
| Cherry-pick first, fallback per-commit | Try `git cherry-pick <sha>` per commit; resolve in-place; fall back to read-upstream-replay only for individual commits exceeding threshold | ✓ |
| Manual port the entire cluster | Read upstream end-to-end; replay semantically; fork commits cite multiple upstream SHAs | |
| Hybrid — split by file impact | Cherry-pick additive-only commits atomically; manual-port the ones rewriting forked files | |

**User's choice:** Cherry-pick first, fallback per-commit
**Notes:** Matches Phase 20 D-01 hybrid pattern; preserves upstream-commit provenance maximally; lets each commit's blast radius surface independently.

### Q2: Fallback rule when cherry-pick conflicts arise

| Option | Description | Selected |
|--------|-------------|----------|
| Soft threshold + planner judgment | >50 lines OR >2 forked files OR semantic ambiguity → planner replays that single commit | ✓ |
| Hard line-count threshold | If `<<<<<<<` markers >100 lines across all conflicted files, fall back. Mechanical | |
| Always resolve in-place; never fall back | Cherry-pick + manual conflict resolution per commit, no fallback ever | |

**User's choice:** Soft threshold + planner judgment
**Notes:** Raw line count isn't always the right signal — a 10-line conflict in `supervised_runtime.rs` D-19 byte-identical region can be harder than a 200-line conflict in a new module.

### Q3: Sequencing within the cluster

| Option | Description | Selected |
|--------|-------------|----------|
| Strict upstream chronological order | One fork commit per upstream SHA in exact upstream history order | ✓ |
| Topological order (foundations first) | Reorder so module-additions land before wiring commits | |
| Squash to 2–3 logical commits | Bundle 7 upstream SHAs into 2–3 thematic fork commits; trailers list multiple SHAs | |

**User's choice:** Strict upstream chronological order
**Notes:** Makes `Upstream-commit:` trailer trivially verifiable; D-20 Windows-regression gate fires after each commit; no provenance loss.

### Q4: CLEAN-04 invariant gate strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Verify after each rename-touching commit | Per-commit invariant suite (`auto_prune_is_noop_when_sandboxed`, suffix-required `--older-than`, `--all-exited`, 100-file auto-sweep) | ✓ |
| Single end-of-plan verification | Run invariants once at plan close | |
| Dedicated `prune_cleanup_rename` task in Plan 22-05 | Carve rename out as a surgical task with explicit pre/post assertions | |

**User's choice:** Verify after each rename-touching commit
**Notes:** Catches mid-plan regression early; finer-grained version of Phase 20 D-20 per-plan discipline for this specific high-risk surface.

---

## Working branch & origin push

### Q1: Branch home for Phase 22/23/24 cherry-picks

| Option | Description | Selected |
|--------|-------------|----------|
| Directly on `main` | Cherry-picks land on `main` per-commit; no integration branch | ✓ |
| `v2.2-integration` branch off `main` | All v2.2 commits on integration branch; merge into main at milestone close | |
| Per-plan branches off `main` | Each plan gets its own branch; merge at plan close | |

**User's choice:** Directly on `main`
**Notes:** main IS the integration branch post-merge; no merge-back overhead; mirrors v2.0/v2.1 windows-squash direct pattern.

### Q2: Origin push timing — initial state

| Option | Description | Selected |
|--------|-------------|----------|
| Push now, before Phase 22 starts | `git push origin main` immediately; publishes 447-commit fast-forward + 46 DCO-missing commits | ✓ |
| Push at v2.2 milestone close | Hold all v2.2 work locally; push only after Phase 24 + audit done | |
| Push per-phase | Push at each phase close; mid-phase commits stay local | |

**User's choice:** Push now, before Phase 22 starts
**Notes:** origin/main becomes canonical baseline; per quick task 260424-mrg path C, publishing the 46 DCO-missing commits is acceptable (DCO only matters for upstream PRs).

### Q3: Origin push cadence — subsequent commits

| Option | Description | Selected |
|--------|-------------|----------|
| Push after every commit | Every cherry-pick pushed immediately | |
| Push after each plan closes | ~8 push events across v2.2 (one per plan boundary) | ✓ |
| Push after each phase closes | 3 push events for v2.2 (one per phase) | |

**User's choice:** Push after each plan closes
**Notes:** Bounded blast radius if local host dies; clear "this plan is published" signal without per-commit churn.

### Q4: Push v2.0 + v2.1 tags to origin

| Option | Description | Selected |
|--------|-------------|----------|
| Push v2.0 + v2.1 tags now | `git push origin v2.0 v2.1` alongside initial main push | ✓ |
| Hold tags local-only | Defer to v3.0 or later "go fully public" decision | |

**User's choice:** Push v2.0 + v2.1 tags now
**Notes:** Tags become reachable from origin/main; PROJECT.md / RETROSPECTIVE.md tag references become verifiable from a fresh clone.

---

## 22-01 unblock granularity

### Q1: When can Plans 22-03 and 22-04 start?

| Option | Description | Selected |
|--------|-------------|----------|
| After full Plan 22-01 close (verifier passed) | All 4 PROF reqs + claude-no-keychain land + verifier signoff | ✓ |
| After per-REQ commit lands (fine-grained) | 22-03 starts when PROF-02 commit lands; 22-04 starts when PROF-03 commit lands | |
| After Wave 1 of 22-01 (the deserialize commits) | Compromise: deserialize commits gate 22-03/22-04 but builtin lands later | |

**User's choice:** After full Plan 22-01 close (verifier passed)
**Notes:** Matches Phase 20 D-15; cleaner per-plan reset; less rebase churn.

### Q2: Plan 22-02 timing relative to 22-01

| Option | Description | Selected |
|--------|-------------|----------|
| Wave-parallel with 22-01 | Both run simultaneously from phase start (disjoint surfaces) | ✓ |
| Sequential after 22-01 | 22-02 starts alongside 22-03/22-04 wave | |
| Sequential before 22-01 | 22-02 first (smallest), then 22-01 | |

**User's choice:** Wave-parallel with 22-01
**Notes:** Matches ROADMAP-locked rationale ("22-02 independent of 22-01; can wave-parallel"); v2.0/v2.1 already validated wave-parallel discipline.

### Q3: Phase 24 (drift-prevention) timing

| Option | Description | Selected |
|--------|-------------|----------|
| Sequence after Phase 22 ships | Linear: Phase 22 → Phase 23 → Phase 24 | ✓ |
| Parallel with Phase 22 in a worktree | Spin up nono-phase24 worktree; run independently | |
| Parallel with Phase 23 only | Phase 22 first, then 23 + 24 in parallel after 22-05 closes | |

**User's choice:** Sequence after Phase 22 ships
**Notes:** Phase 24's first real use is against v0.41+ (which doesn't exist until v2.3); urgency is low; clean linear flow keeps mental load low.

### Q4: 22-03 + 22-04 parallel or sequential after 22-01 closes?

| Option | Description | Selected |
|--------|-------------|----------|
| Wave-parallel (22-03 + 22-04 simultaneously) | Both start after 22-01 closes; disjoint surfaces | ✓ |
| Sequence 22-03 first, then 22-04 | Bigger surface first | |
| Sequence 22-04 first, then 22-03 | Smaller surface first | |

**User's choice:** Wave-parallel
**Notes:** Surfaces are genuinely disjoint (22-03: package_cmd; 22-04: nono-proxy/oauth2); pattern validated in v2.1 Phase 18 + Phase 20.

---

## Test fixture strategy

### Q1: OAuth2 token endpoint for REQ-OAUTH-01

| Option | Description | Selected |
|--------|-------------|----------|
| Port upstream's existing test fixture | Reuse upstream's `9546c879` test infrastructure | ✓ |
| Build a fork-local OAuth2 mock | Stand up tokio-based test server; independent of upstream | |
| Use a public OAuth2 sandbox (gated to CI) | Use real third-party (Auth0/Keycloak) sandbox | |

**User's choice:** Port upstream's existing test fixture
**Notes:** Matches `Upstream-commit:` provenance discipline; if upstream has fixture coverage, parity-stealing is a tax already paid.

### Q2: Deterministic registry for REQ-PKG-01

| Option | Description | Selected |
|--------|-------------|----------|
| Port upstream's existing registry test fixture | Reuse upstream's package + signed-artifact + streaming fixtures | ✓ |
| Local filesystem registry (no HTTP server) | Mock as tempdir + static manifest; bypasses streaming codepath | |
| Build a fork-local hyper test server with real signed artifacts | Independent of upstream; better Windows-specific control | |

**User's choice:** Port upstream's existing registry test fixture
**Notes:** Consistent with D-13 logic.

### Q3: Windows-specific test cases atop ported fixtures

| Option | Description | Selected |
|--------|-------------|----------|
| Add Windows-only test cases atop ported fixtures | `#[cfg(windows)]`-gated long-path/traversal/Credential-Manager/Authenticode tests | ✓ |
| Trust ported fixtures; add Windows tests only when a gap surfaces | Reactive coverage | |
| Add ALL acceptance-criteria-driven tests (Windows + cross-platform) | Treat each REQ-* acceptance line as a test contract | |

**User's choice:** Add Windows-only test cases atop ported fixtures
**Notes:** Windows parity is the milestone goal; ported fixtures alone won't prove parity; matches v2.1 Phase 21 WSFG Windows-only test pattern.

### Q4: CI lane for new tests

| Option | Description | Selected |
|--------|-------------|----------|
| Existing `make ci` + `make test` (per-plan gate) | Tests in natural homes; reuse Phase 20 D-20 safety net | ✓ |
| Dedicated `windows-parity` test target | Carve out `make test-windows-parity` for fast iteration | |
| Gated under `--ignored` like wfp_port_integration | Tests only run with `cargo test -- --ignored` | |

**User's choice:** Existing `make ci` + `make test` (per-plan gate)
**Notes:** Don't add CI infra mid-milestone; reuse existing gate; matches Phase 20/21 discipline.

---

## Claude's Discretion (deferred to planner)

- Exact `make ci` invocations per plan
- Audit signing key provisioning model on Windows (auto-gen vs user pre-provision vs `nono audit init` helper)
- Authenticode fallback path shape for REQ-AUD-03 (record format when signature unavailable)
- AUD-05 fold-or-split decision-point during 22-05 execution (default: keep as Phase 23)
- `prune` alias deprecation timeline (one release? v2.3? v2.4? — v2.3 milestone scoping decision)
- Per-plan PR vs single Phase-22 PR (default: single Phase-22 PR per Phase 19/20 norm)

## Deferred Ideas

- **AUD-05** — Phase 23 (may collapse into 22-05)
- **DRIFT-01..02** — Phase 24 (linear after Phase 22)
- **Upstream v0.41+ ingestion** — v2.3 first quick task
- **WR-01 reject-stage unification** — v2.3+
- **AIPC G-04 wire-protocol tightening** — v2.3+
- **Cross-platform RESL Unix backends** — v2.3+
- **WR-02 EDR HUMAN-UAT** — v3.0
- **`prune` alias deprecation timeline** — v2.3 scoping
- **DCO signoff remediation** — only when opening upstream PR
- **Delete `windows-squash` branch** — after origin/main pushed + no references
- **PR 555 disposition on upstream** — separate question
