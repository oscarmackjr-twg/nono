# Phase 18: Extended IPC (AIPC-01) — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in `18-CONTEXT.md` — this log preserves the alternatives considered.

**Date:** 2026-04-19
**Phase:** 18-extended-ipc
**Areas discussed:** Wire-protocol shape, Per-handle-type approval UX, Access-mask allowlist defaults, CLI/SDK surface

---

## Gray-Area Selection

User selected **all 4** offered areas.

| Option | Description | Selected |
|--------|-------------|----------|
| Wire-protocol shape | Tagged enum vs new variants vs backward-compat single message | ✓ |
| Per-handle-type approval UX | Single template vs five distinct vs structured table | ✓ |
| Access-mask allowlist defaults | Hard-coded + override vs pure-profile vs hard-coded only | ✓ |
| CLI/SDK surface | SDK-only vs hybrid vs CLI-only declarative | ✓ |

---

## Wire-Protocol Shape

| Option | Description | Selected |
|--------|-------------|----------|
| Tagged enum on CapabilityRequest (Recommended) | `kind: HandleKind` + `target: HandleTarget` enum on existing CapabilityRequest. Deprecate `path: PathBuf` to `Option<PathBuf>` for one release. Single dispatch site, one audit shape, one validator path. Breaks Phase 11 wire compat (acceptable — wire is internal). | ✓ |
| New SupervisorMessage variants per type | Add `RequestSocket(SocketRequest)`, `RequestPipe(PipeRequest)`, etc. Most additive; preserves wire compat. Cost: 5x dispatch arms, 5x audit shapes, 5x validators. | |
| Backward-compat single Request + optional handle_type | `handle_type: Option<HandleKind>` defaults to File. Awkward middle ground; path/target both optional means more validation surface. | |

**User's choice:** Tagged enum on CapabilityRequest.
**Notes:** Locks the protocol cleanly. Makes the constant-time discriminator check (D-03) a single concentrated step. Single audit-entry shape preserves Phase 11's `AuditEntry` unchanged. Wire-break is intentional and documented; SDK consumer surface stays stable via thin shim.

---

## Per-Handle-Type Approval UX

| Option | Description | Selected |
|--------|-------------|----------|
| Single template, per-type field labels (Recommended) | One render function `format_capability_prompt(kind, target, access_mask, reason) -> String`. Consistent shape; one sanitizer call site; easy to test; easy to extend. | ✓ |
| Five distinct prompt strings | Per-type hand-tuned prompts. Friendlier per type. Cost: 5 sanitizer call sites, 5 test string sets, harder for users to learn shape. | |
| Structured table format | Multi-line table. Most readable for complex types. Breaks Phase 11's single-line prompt UX; two formats coexist. | |

**User's choice:** Single template, per-type field labels.
**Notes:** Consistent template shape `[nono] Grant <kind> access? <type-specific-fields> access=<mask> reason="<r>" [y/N]` for all 6 types (File + 5 new). One call site for `sanitize_for_terminal()`. Future handle types extend the same template trivially.

---

## Access-Mask Allowlist Defaults

| Option | Description | Selected |
|--------|-------------|----------|
| Hard-coded supervisor defaults + profile override (Recommended) | Conservative defaults baked in (socket connect-only, pipe single-direction, JobObject query-only, event/mutex sync+modify). Profile.toml `[capabilities.aipc]` widens. Default-deny applies to anything not in either set. | ✓ |
| Pure-profile (no defaults) | Most explicit/auditable. Cost: every project needs profile setup; no "just works"; built-in profiles need editing. | |
| Hard-coded only | Simplest. Cost: rigid; no per-profile customization; profile-override can be added later additively. | |

**User's choice:** Hard-coded supervisor defaults + profile override.
**Notes:** Built-in profiles (claude-code, codex, opencode, openclaw, swival) get tuned `[capabilities.aipc]` blocks per-need. Defaults review-able in one place: `crates/nono/src/supervisor/policy.rs`. Profile override is widening-only — narrowing requires future mechanism (e.g. `[capabilities.aipc.deny]` block, deferred to v2.2+).

---

## CLI/SDK Surface

| Option | Description | Selected |
|--------|-------------|----------|
| SDK-only — extend Phase 11 pattern (Recommended) | Child SDK gains 5 new request methods. CONIN$ prompt fires per request. Zero new CLI flags. Matches Phase 11 lock. | ✓ |
| Hybrid — SDK runtime + optional CLI pre-approval | Keep SDK runtime + add `--allow-socket :8080`, `--allow-pipe`, etc. Pre-approved handles brokered at child-spawn time (no prompt). Useful for unattended/CI. Cost: 5 new flags, fail-closed validators, profile-merge logic. | |
| CLI-only declarative | All handles pre-declared on `nono run`. No runtime SDK requests. Most predictable. Cost: regression vs Phase 11 ergonomics. | |

**User's choice:** SDK-only — extend Phase 11 pattern.
**Notes:** Matches Phase 11's locked behavior; least surface area; least security review burden. CLI pre-approval captured in deferred ideas as a v2.2 candidate if user demand surfaces.

---

## Cross-Platform Behavior (D-09)

Per REQUIREMENTS.md line 163: "Unix builds either reject `--request-handle` at parse time or degrade gracefully." Since D-08 is SDK-only (no CLI flags), there's no parse-time rejection surface. The discussion converged on:
- SDK methods exist on all platforms (compile cross-platform)
- Brokering paths are `#[cfg(target_os = "windows")]`
- Non-Windows call returns `NonoError::PlatformNotSupported` at runtime with a message pointing to SCM_RIGHTS as the future Unix equivalent

This was Claude's discretion within the Phase 18 scope (D-09 in CONTEXT.md), not surfaced as a separate gray area but worth recording.

---

## Claude's Discretion

User did not explicitly say "you decide" on any structural decision. The following implementation details are Claude's discretion (recorded in `18-CONTEXT.md § Claude's Discretion` for the planner):

- File layout for the new policy module (`policy.rs` vs `aipc.rs`).
- Internal helper structure for the 5 brokers (5 functions vs 1 dispatcher).
- `request_pipe` direction handling (enum arg vs two methods).
- `SocketProtocol` enum vs string.
- Test scaffolding split (per-handle-type unit + integration; both granted + denied paths).
- Audit `duration_ms` granularity (unchanged from Phase 11).

---

## Deferred Ideas

Captured in `18-CONTEXT.md § Deferred Ideas`:

- CLI pre-approval flags (`--allow-socket`, etc.) — v2.2 candidate.
- Sibling-to-sibling handle brokering — REQUIREMENTS.md `Out of Scope (v2.1)`.
- Linux/macOS SCM_RIGHTS handle brokering — separate cross-platform requirement; future milestone.
- Profile narrowing (`[capabilities.aipc.deny]`) — v2.1 lock is widening-only.
- Bind-to-privileged-port (port < 1024) — explicitly denied in v2.1.
- `JOB_OBJECT_TERMINATE` and `JOB_OBJECT_SET_*` access masks — excluded from default; profile opt-in possible.
- Removal of deprecated `path: PathBuf` field — future phase after one deprecation release.
- `ApprovalBackend::request_capability` trait refactor — preserved unchanged for backward compat.
