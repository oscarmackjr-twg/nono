---
milestone: v2.5
milestone_name: Backlog Drain + UPST5
status: active
created: 2026-05-15
---

# Requirements — v2.5 Backlog Drain + UPST5

**Defined:** 2026-05-15
**Core Value:** Drain the host-blocked v2.4 carry-forwards via Windows-coded + CI-executed Linux backends, clear the pre-existing CI red that has been masking real regressions, absorb upstream `v0.53.0..+` (first sync where Windows is touched), and resolve the v24 Windows broker code-review backlog — leaving the fork in a state where every cross-platform CI lane is green and Phase 38 + v2.6 work can proceed on a quiet baseline.

**Context:** v2.4 shipped 2026-05-15 with 5 phases (35, 36, 36.5, 39, 40) on Windows host and 5 host-blocked requirements re-anchored to v2.5 (RESL-NIX-01..03 + PKGS-04 → Phase 37; AAHX-HOST-01 → Phase 38 optional). User has no Linux host; macOS access in a couple of days but macOS parity deprioritized for this milestone. Phase 37 Linux work will be coded on the Windows host and verified via GitHub Actions Linux runners (same pattern that landed Phase 35-02 Landlock profiles-dir fix). Pre-existing CI red surfaced in `.planning/PHASE-41-TRACKER.md` on 2026-05-14: 33 Linux/macOS Clippy errors + 5 Windows CI job failures (Build, Integration, Regression, Security, Packaging) all confirmed pre-existing on baseline `a72736bb`. UPST5 reaches upstream `v0.53.0..+`, the first window where the `windows-touch` column actually fires per D-39-C3 (2 known Windows-touching commits in `v0.54.0~5^2`).

**Scope shape:** ~5 phases. Phase 37 is the carry-forward (already plan-drafted in v2.3 Plan 25-01 + Plan 26-02; macOS `setrlimit` portion dropped from scope). Phase 41 is mechanical CI cleanup. Phases 42 + 43 mirror Phase 33+34 / 39+40 shape. v24 broker code-review todos fold into the appropriate phase. 13 in-milestone requirements across 5 categories (RESL-NIX carry-forward, PKGS carry-forward, CI-CLEAN new, UPST5 new, BROKER-CR new).

**Out of scope (explicit deferrals to v2.6 or later):**

- **REQ-AAHX-HOST-01** (Phase 38 optional) — REQ-AAH-01 native re-validation on Linux/macOS. Depends on Phase 37 native UAT; carry to v2.6 when native Linux UAT is available.
- **Phase 35 + 36 human-verify backlog** — 11 UAT items + 7 verification items host-blocked at v2.4 close; carry to v2.6.
- **macOS `setrlimit` portion of Plan 25-01** — `RLIMIT_AS` / `RLIMIT_NPROC` Seatbelt backend. Mac host available in a couple of days but macOS parity deprioritized this milestone; carry to v2.6+.
- **v2.5-FU-1** (audit-bundle shim removal — v2.5 candidate from Phase 27.2) — re-defer to v2.6 unless a phase has slack.
- **v2.5-FU-2** (cmd_verify v2 JSON schema — v2.5 candidate from Phase 27.2) — re-defer to v2.6.
- **AIPC G-04 wire-protocol compile-time tightening** — v3.0 (cascades into 23 pre-existing tests + child SDK demultiplexer).
- **WR-02 EDR HUMAN-UAT** — v3.0 (EDR-instrumented runner required).
- **P32-DEFER-005** (sigstore-verify 0.6.5 → 0.6.6) — defer to v2.6.

---

## RESL-NIX — Linux RESL backends (Phase 37 — carry-forward from v2.3)

Context: Plan 25-01 design committed in v2.3 (`3ed80d38`) but execution deferred to Linux/macOS host. v2.4 re-anchored to v2.5 Phase 37. macOS `setrlimit` portion dropped from scope per "macOS deprioritized". Linux cgroup v2 backend will be coded on Windows host + verified via GitHub Actions Linux runners. Closes the 3-year Linux silent-no-op for `--memory` / `--cpu-percent` / `--max-processes`.

### REQ-RESL-NIX-01 — Linux cgroup v2 memory cap (`--memory`)

- **What:** Linux backend for `--memory <BYTES>` via cgroup v2 `memory.max` + `cgroup.kill` (kernel-enforced OOM-style termination on cap breach). Mirrors v2.1 Phase 16 RESL-02 Windows `JobMemoryLimit` semantics.
- **Enforcement:** Linux cgroup v2 (kernel). Requires unified cgroup hierarchy + `cgroup_no_v1` boot flag OR distro defaulting to v2-only (Fedora 31+, Ubuntu 21.10+, Debian 11+).
- **Security:** Closes silent-no-op vulnerability. Today on Linux, `nono run --memory 100M -- mem_hog` returns success and leaks memory to the host; v2.5 makes the limit kernel-enforced.
- **Acceptance:**
  1. `nono run --memory 100M -- python -c 'a = bytearray(200_000_000)'` exits non-zero on Linux (cap breach killed by kernel OOM).
  2. `nono inspect <id>` Limits block shows `memory: 100M (cgroup v2 memory.max)` on Linux.
  3. Fail-closed on cgroup v1 hosts: `NonoError::UnsupportedKernelFeature` with hint pointing to `cgroup_no_v1` boot flag.
  4. GitHub Actions Linux runner (Ubuntu 24.04, cgroup v2 default) executes the integration test in CI.
- **Maps to:** Plan 25-01 (v2.3 design ADR shipped, execution carry-forward).

### REQ-RESL-NIX-02 — Linux cgroup v2 CPU cap (`--cpu-percent`)

- **What:** Linux backend for `--cpu-percent <PCT>` via cgroup v2 `cpu.max <quota> <period>`. Mirrors v2.1 Phase 16 RESL-01 Windows `JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP` semantics.
- **Enforcement:** Linux cgroup v2 (kernel). Default `cpu.max` period is 100ms; quota = `percent * period / 100`.
- **Security:** Closes silent-no-op for CPU throttling on Linux. Today `nono run --cpu-percent 25 -- yes >/dev/null` runs at full core.
- **Acceptance:**
  1. `nono run --cpu-percent 25 -- yes >/dev/null` averages ~25% CPU under sampling (measured via `top -b -n 5 -d 1 -p <pid>` aggregated over 5s window).
  2. `nono inspect <id>` Limits block shows `cpu_percent: 25 (cgroup v2 cpu.max 25000 100000)` on Linux.
  3. Fail-closed on cgroup v1 hosts (shares `--memory`'s `NonoError::UnsupportedKernelFeature` path).
  4. GitHub Actions Linux runner verifies the throttle is enforced.
- **Maps to:** Plan 25-01 (carry-forward).

### REQ-RESL-NIX-03 — Linux cgroup v2 process count cap (`--max-processes`)

- **What:** Linux backend for `--max-processes <N>` via cgroup v2 `pids.max`. Mirrors v2.1 Phase 16 RESL-04 Windows `ActiveProcessLimit` semantics.
- **Enforcement:** Linux cgroup v2 (kernel). `pids.max <N>` rejects `fork()` / `clone()` with `EAGAIN` once the count hits N.
- **Security:** Closes silent-no-op for fork-bomb protection on Linux.
- **Acceptance:**
  1. `nono run --max-processes 5 -- bash -c ':(){ :|:& };:'` is contained at ~5 processes (verified via `pgrep -c -P <pid>` snapshot mid-run).
  2. `nono inspect <id>` Limits block shows `max_processes: 5 (cgroup v2 pids.max)` on Linux.
  3. Fail-closed on cgroup v1 hosts (shares the `NonoError::UnsupportedKernelFeature` path).
  4. GitHub Actions Linux runner verifies the fork-bomb is contained.
- **Maps to:** Plan 25-01 (carry-forward).

---

## PKGS — Package manager streaming + auto-pull (Phase 37 — carry-forward from v2.3)

Context: Plan 26-02 (REQ-PKGS-01 streaming + REQ-PKGS-04 auto-pull) committed in v2.3 but execution deferred to Linux/macOS host (Windows-host integration tests hit Phase 27 `dirs::home_dir()` blocker, now closed by Phase 27.1 `NONO_TEST_HOME` seam). REQ-PKGS-01 retroactively closed at v2.4 close via v2.3 Phase 26-02 fork-arch ship. REQ-PKGS-04 remains for v2.5.

### REQ-PKGS-04 — `load_registry_profile` auto-pull on `--profile` reference

- **What:** Port upstream `115b5cfa` auto-pull behavior: when `--profile <name>` references a profile not present locally and the name matches a known registry entry, automatically pull the signed artifact before resolving the profile. Mirrors `cargo install <crate>` UX.
- **Enforcement:** Cross-platform (Linux + macOS + Windows). Reuses v2.3 Phase 26-01 signed-artifact verification + fork's `validate_path_within` belt-and-suspenders.
- **Security:** Auto-pull only fires when the requested name matches a registered registry entry (no implicit network for arbitrary names). Signed-artifact verification still gates installation.
- **Acceptance:**
  1. `nono run --profile claude-code-edge -- cmd` (where `claude-code-edge` is in the registry but not installed locally) auto-pulls the signed artifact, verifies its signature, installs it, and runs the command.
  2. `nono run --profile unknown-name -- cmd` fails closed with the existing "profile not found" error (no implicit network for non-registry names).
  3. Signed-artifact verification failure aborts the auto-pull with the existing security error.
  4. `--no-auto-pull` flag (new) skips auto-pull and falls back to the legacy "profile not found" error.
  5. GitHub Actions Linux runner verifies the e2e flow.
- **Maps to:** Plan 26-02 (carry-forward; v2.3 plan committed, execution deferred).

---

## CI-CLEAN — Pre-existing CI red cleanup (Phase 41 — new)

Context: `.planning/PHASE-41-TRACKER.md` filed 2026-05-14 documents pre-existing CI red on baseline `a72736bb`: 33 Linux/macOS Clippy errors + 5 Windows CI job failures (Build, Integration, Regression, Security, Packaging). v2.4 used a baseline-aware CI gate (flag only `success → failure` transitions vs baseline) which is correct for upstream-sync phases but accumulates drift. v2.5 resets the baseline so subsequent phases get clean gates.

### REQ-CI-01 — Linux/macOS Clippy lints resolved

- **What:** Resolve the 33 Linux/macOS Clippy errors enumerated in `.planning/PHASE-41-TRACKER.md`: (a) `nono::CapabilityRequest::path` deprecated API migration at 14 sites in `crates/nono-cli/src/exec_strategy.rs`; (b) ~14 dead-code orphans in `audit_ledger.rs`, `audit_integrity.rs`, `exec_identity.rs`, `exec_strategy.rs`, `exec_strategy/env_sanitization.rs`, `exec_strategy/supervisor_linux.rs`, `launch_runtime.rs`, `protected_paths.rs`, `pty_proxy.rs`, `rollback_session.rs`, `session.rs`; (c) `std::env::set_var/remove_var` → `EnvVarGuard` migration at 2 sites in `crates/nono-cli/src/test_env.rs:343,344`; (d) unreachable expression at `crates/nono-cli/src/exec_strategy.rs:1930`; (e) sundry / fields-never-read residuals.
- **Enforcement:** Cross-platform lint cleanup. Cross-target clippy required (`cargo clippy --workspace --target x86_64-unknown-linux-gnu`) per memory `feedback_clippy_cross_target`.
- **Security:** Indirect — clean clippy baseline makes regression detection reliable. Dead-code removal also surfaces hidden API drift (e.g. `record_capability_decision` orphan after Phase 23 wired the AIPC variant).
- **Acceptance:**
  1. `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` exits 0 on Linux from Windows host.
  2. `cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` exits 0 for macOS target from Windows host (if cross-toolchain available; otherwise rely on CI macOS runner).
  3. GitHub Actions Linux Clippy + macOS Clippy jobs green on the head of Phase 41.
  4. No `#[allow(dead_code)]` added — orphans either deleted or wired (per CLAUDE.md "lazy use of dead code" rule).
- **Maps to:** PHASE-41-TRACKER error categorization table.

### REQ-CI-02 — Windows CI jobs green

- **What:** Resolve the 5 Windows CI job failures (Build, Integration, Regression, Security, Packaging) per the Windows error categorization in `.planning/PHASE-41-TRACKER.md`.
- **Enforcement:** Windows host + Windows CI runners. Some failures may require code fixes; others may be CI-infra config (artifact paths, expired secrets, MSI signing cert refresh).
- **Security:** Windows Packaging green required for v2.5 release-quality (signed MSI + signed zip).
- **Acceptance:**
  1. GitHub Actions Windows Build job green on the head of Phase 41.
  2. Windows Integration + Windows Regression + Windows Security + Windows Packaging jobs green.
  3. No `[ignored]` test markers added without an issue link justifying the deferral.
- **Maps to:** PHASE-41-TRACKER Windows-side findings (filed 2026-05-14 follow-up).

### REQ-CI-03 — Baseline-aware gate reset

- **What:** After REQ-CI-01 + REQ-CI-02 close, reset the baseline-aware CI gate (used by Phase 34 + 40 UPST sync phases) so the new baseline is the head of Phase 41 (green on all lanes). Update `.planning/templates/upstream-sync-quick.md` and any plan templates referencing the old baseline.
- **Enforcement:** Process / tooling.
- **Security:** Subsequent v2.5 phases (Phase 42 + 43 UPST5) inherit a clean baseline — `success → failure` transitions become unambiguously real regressions instead of background drift.
- **Acceptance:**
  1. Baseline SHA in `.planning/templates/upstream-sync-quick.md` updated to the Phase 41 close SHA.
  2. Phase 34 + 40 SUMMARY frontmatter convention (`skipped_gates_load_bearing` vs `_environmental`) documented at top of Phase 41 SUMMARY with the new baseline expectations.
  3. STATE.md `## Deferred Items` cleared of v24 CR-A class entries that were waiting on a clean baseline.
- **Maps to:** Phase 40 process-hardening anti-pattern #3 (baseline distinction).

---

## UPST5 — Upstream v0.53.0..+ parity sync (Phases 42 + 43 — new)

Context: Mirror of Phase 33 (audit) + Phase 34 (execution) and Phase 39 + 40. Per Phase 33 ADR `upstream-parity-strategy.md` Option A `continue` (Accepted, re-confirmed by D-39-C4 at v2.4 close). First UPST sync where the `windows-touch` column actually fires: 2 known Windows-touching commits (`5d821c12` + `0748cced`) land in `v0.54.0~5^2`, with D-39-C3 conservative-default fork-preserve disposition until reviewed.

### REQ-UPST5-01 — Upstream v0.53.0..+ divergence audit

- **What:** Mirror Phase 33 / 39 shape. Produce `DIVERGENCE-LEDGER.md` for upstream `v0.53.0..+` (anchor TBD at audit start — typically the latest tag ≥ v0.54.0 at audit-open time). Themed clusters with cross-platform commit counts, per-cluster dispositions (will-sync / fork-preserve / won't-sync), `windows-touch` column. ADR review confirming or amending Phase 33 Option A `continue` strategy.
- **Enforcement:** Process. Output is `DIVERGENCE-LEDGER.md` + per-cluster cluster files in `.planning/phases/42-*/`.
- **Security:** Audit-only; no code change. Outputs gate Phase 43 cherry-pick selection.
- **Acceptance:**
  1. `DIVERGENCE-LEDGER.md` enumerates every upstream commit in `v0.53.0..<anchor>` that touches a fork-shared file (`crates/nono/`, `crates/nono-cli/` excluding `_windows.rs`/`exec_strategy_windows/`, `crates/nono-proxy/`).
  2. Per-cluster disposition + Windows-touch column + rationale.
  3. `5d821c12` + `0748cced` Windows-touching commits explicitly handled (will-sync vs fork-preserve decision).
  4. ADR review section confirming or amending the Phase 33 ADR.
  5. Empirical cross-check: spot-check 3 fork-shared files for upstream paths the ledger missed.
- **Maps to:** Phase 33 audit shape. Sets up Phase 43.

### REQ-UPST5-02 — Upstream v0.53.0..+ sync execution

- **What:** Mirror Phase 34 / 40 shape. Cherry-pick + D-20 manual replay per UPST5 audit dispositions. D-19 6-line trailer convention + Windows-only-files invariant (D-34-E1 / D-40-E1) inherited. Cluster ordering minimizes intra-wave file overlap. Baseline-aware CI gate vs the post-Phase-41 green baseline (REQ-CI-03).
- **Enforcement:** Cross-platform. Cherry-picks on Windows host; CI lanes verify Linux/macOS/Windows green.
- **Security:** D-19 invariant (cross-platform byte-identity preserved when cherry-picking upstream commits) holds. Windows-only-files invariant prevents accidental fork-Windows-code regression in upstream-sync commits.
- **Acceptance:**
  1. Every audit `will-sync` cluster has a corresponding plan in Phase 43 with cherry-picks + D-19 trailers.
  2. Every `fork-preserve` cluster has a documented "preserve fork because X" rationale at the SUMMARY level.
  3. `windows-touch` cluster (5d821c12 + 0748cced) handled per audit disposition; if `will-sync`, Windows CI green post-merge.
  4. Baseline-aware CI gate: zero `success → failure` transitions vs Phase 41 close SHA on every Wave 1+ head commit.
  5. PR umbrella to upstream holds all Phase 43 plan contribution sections.
- **Maps to:** Phase 34 + 40 execution shape.

---

## BROKER-CR — v24 Windows broker code-review backlog (folded into appropriate phase)

Context: 4 code-review todos drafted in `.planning/todos/pending/` during v2.4 close. Small Windows-host follow-ups. Folded into the appropriate v2.5 phase by the roadmapper.

### REQ-BROKER-CR-01 — Broker FFI not-found mapping

- **What:** Map broker FFI "not found" errors to a clear `NonoError` variant + diagnostic. Currently a raw `HRESULT` propagates.
- **Enforcement:** Windows broker (`crates/nono-shell-broker/`).
- **Acceptance:** Per `.planning/todos/pending/v24-cr-01-broker-not-found-ffi-mapping.md`.

### REQ-BROKER-CR-02 — Broker null-handle validation

- **What:** Validate broker-side handle arguments are non-null before crossing the FFI boundary. Closes a defense-in-depth gap.
- **Enforcement:** Windows broker.
- **Acceptance:** Per `.planning/todos/pending/v24-cr-02-broker-null-handle-validation.md`.

### REQ-BROKER-CR-03 — Broker empty-handle-list path

- **What:** Handle the empty-handle-list edge case in the broker handle-list dispatch path.
- **Enforcement:** Windows broker.
- **Acceptance:** Per `.planning/todos/pending/v24-cr-03-broker-empty-handle-list-path.md`.

### REQ-BROKER-CR-04 — Job-object test skip policy

- **What:** Clarify the Job-object test skip policy on platforms without Job Object support. Currently silent skip; should be explicit `#[ignore]` with a clear reason string.
- **Enforcement:** Windows test harness.
- **Acceptance:** Per `.planning/todos/pending/v24-cr-04-job-object-test-skip-policy.md`.

---

## Traceability

| REQ-ID | Phase | Status |
|---|---|---|
| REQ-RESL-NIX-01 | Phase 37 | not_started |
| REQ-RESL-NIX-02 | Phase 37 | not_started |
| REQ-RESL-NIX-03 | Phase 37 | not_started |
| REQ-PKGS-04 | Phase 37 | not_started |
| REQ-CI-01 | Phase 41 | not_started |
| REQ-CI-02 | Phase 41 | not_started |
| REQ-CI-03 | Phase 41 | not_started |
| REQ-UPST5-01 | Phase 42 | not_started |
| REQ-UPST5-02 | Phase 43 | not_started |
| REQ-BROKER-CR-01 | TBD (Phase 41 or 43) | not_started |
| REQ-BROKER-CR-02 | TBD (Phase 41 or 43) | not_started |
| REQ-BROKER-CR-03 | TBD (Phase 41 or 43) | not_started |
| REQ-BROKER-CR-04 | TBD (Phase 41 or 43) | not_started |

Phase assignments finalized in `.planning/ROADMAP.md` (filled by `gsd-roadmapper`).
