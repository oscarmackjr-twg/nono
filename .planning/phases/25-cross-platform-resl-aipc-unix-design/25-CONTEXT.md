---
phase: 25-cross-platform-resl-aipc-unix-design
created: 2026-04-29
type: phase-context
---

# Phase 25 Context — Cross-Platform RESL + AIPC Unix Design

## Open questions resolved at scope-lock

The Plan 25-01 draft flagged two open questions for the executor; both are resolved here so they don't need re-discovery during execution.

### Q1 — Do `memory_kill` / `timeout_kill` fields exist on `SessionRecord` / `SandboxState`?

**Verified absent** (2026-04-29). Grep against `crates/nono-cli/src/inspect_cmd.rs` and `crates/nono-cli/src/sandbox_state.rs` returned **zero matches** for either field name. The fields are referenced in REQ-RESL-NIX-01 acceptance criterion 1 ("`nono inspect <id>` shows `memory_kill: true`") but do not yet exist on the wire.

**Resolution for Plan 25-01:** the plan MUST add these fields to the `SessionRecord` (or equivalent) struct as part of its enforcement work. Suggested placement: alongside the existing Phase 16 `Limits:` block surfaced in `nono inspect`. The new fields are populated by the supervisor watchdog (`memory_kill: true` when cgroup v2 reports OOM kill via `memory.events`, `timeout_kill: true` when the supervisor's `Instant` deadline fires and writes `cgroup.kill`). On Windows, the same fields can be wired from `JOB_OBJECT_LIMIT_VIOLATION_INFORMATION` in a follow-up — but Phase 25 owns Linux/macOS plumbing only.

**Scope guidance:** treat field addition as a Linux-side requirement of REQ-RESL-NIX-01 acceptance #1 + REQ-RESL-NIX-02 acceptance #1, not as a separate phase. If the field plumbing meaningfully expands plan scope (>2 file additions), surface as a deviation during execution rather than expanding upfront.

### Q2 — Do `NonoError::UnsupportedPlatform` and `NotSupportedOnPlatform` variants both exist?

**Only `UnsupportedPlatform(String)` exists** (verified at `crates/nono/src/error.rs:40`). There is no separate `NotSupportedOnPlatform` variant; Plan 25-01's prompt introduced that name as a placeholder.

**Resolution for Plan 25-01:** reuse the existing `UnsupportedPlatform(String)` variant for both fail-fast paths. Use these feature-string conventions as the in-string discriminator:

- Linux cgroup v1 / no-delegation detection: `NonoError::UnsupportedPlatform("cgroup_v2".to_string())` (or richer string with diagnostic hint, but the substring `cgroup_v2` MUST be present so error matchers can key on it).
- macOS `--cpu-percent` clap-time rejection: `NonoError::UnsupportedPlatform("cpu_percent_macos".to_string())` (substring `cpu_percent_macos` MUST be present).

**Do NOT add a new `NotSupportedOnPlatform` variant** — that would cascade `NonoError` signature changes through the workspace and trip D-19 byte-identical preservation checks during execution.

**Plan 25-01 frontmatter `must_haves.truths` adjustment:** any truth assertion referring to `NotSupportedOnPlatform { feature: "..." }` should be re-read as `UnsupportedPlatform("...")` containing the feature substring. Functionally equivalent; just narrower API surface.

## Reference acceptance shape

v2.1 Phase 16 (Windows RESL) is the reference acceptance shape for Plan 25-01:
- `.planning/milestones/v2.1-ROADMAP.md` § "Phase 16: Resource Limits" for the high-level shape.
- `.planning/phases/16-resource-limits/16-01-PLAN.md` for the detailed plan template (frontmatter shape, `must_haves.truths` style, task structure).
- `.planning/phases/16-resource-limits/16-02-PLAN.md` for the Phase 16 timeout watchdog pattern (supervisor-side `Instant` + `TerminateJobObject`); Plan 25-01's Linux watchdog mirrors this with `cgroup.kill`, macOS watchdog mirrors with `kill(pgrp, SIGKILL)`.

## Plan 25-02 has no open questions

All six AIPC HandleKind verdicts (File/Socket/Pipe = Yes; JobObject/Event/Mutex = No) and three alternate mechanisms (cgroup v2 / pipe(2) / flock(2)) were locked at scope-lock and recorded in the plan prompt verbatim. The ADR is decision-only with no API surface sketch, so there's no design surface for ambiguity.

## Cross-plan independence

Plans 25-01 and 25-02 are independent (`depends_on: []` on both). They can execute in parallel waves or sequentially. Plan 25-02 writes to `docs/architecture/` + `.planning/PROJECT.md` only — zero overlap with Plan 25-01's source-code touch list.

## Out of scope (re-confirmed at scope-lock)

- AIPC G-04 wire-protocol compile-time tightening — v2.4 backlog (pre-existing).
- Cross-platform RESL drift QA / docs pass — bundle with v2.4 ingestion.
- Privileged cgroup paths / `systemd-run` shell-out — explicitly rejected at scope-lock; do NOT re-introduce during execution.
- AIPC SDK source touches — Plan 25-02 is design-only.

## D-19 cross-phase byte-identical preservation

Plan 25-01's Linux/macOS code lives under `#[cfg(target_os = "linux")]` / `#[cfg(target_os = "macos")]` gates. Windows behavior MUST remain byte-identical. Verification grep at plan's verification gate: `git diff --stat HEAD~N HEAD -- crates/nono-cli/src/exec_strategy_windows/ crates/nono/src/sandbox/windows.rs` should be empty across all Plan 25-01 commits. (Exception: the four "not enforced on linux/macos" warnings being removed from `exec_strategy.rs` is a cross-platform file edit, but the removed lines are guarded `#[cfg(not(target_os = "windows"))]` already, so removing them does not affect Windows-side behavior.)

## D-21 Windows-invariance

Same as D-19 above — Plan 25-01 must not regress any Windows behavior. Plan 25-02 doesn't touch source at all, so D-21 is trivially satisfied.
