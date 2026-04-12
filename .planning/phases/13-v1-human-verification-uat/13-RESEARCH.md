# Phase 13: v1.0 Human Verification UAT — Research

**Researched:** 2026-04-12
**Domain:** Windows live-host acceptance testing — supervisor lifecycle, WFP network enforcement, runtime capability expansion
**Confidence:** HIGH (all items sourced directly from upstream VERIFICATION.md files and the v1.0 milestone audit)

---

## Summary

Phase 13 is a pure testing phase: no code is written. Its job is to execute the 10 deferred human-verification items that are tracked across four upstream VERIFICATION.md files (phases 05, 07, 09, 11), record results in a `13-UAT.md` log, and promote each upstream VERIFICATION.md status from `human_needed` to `passed` (or open a gap issue if an item fails).

All 10 items were deferred from their originating phases precisely because they require a live Windows host, an attached console, and in some cases admin privileges or a running `nono-wfp-service`. Nothing new to implement — the codebase already contains the code that makes these items verifiable. The research task is to catalogue what exactly needs to happen, in what order, and what the host prerequisites are.

**Primary recommendation:** Treat Phase 13 as a single-wave, purely manual test pass. Write one plan that produces the `13-UAT.md` test script and walk the user through each numbered item in dependency order. The plan does NOT execute the tests autonomously — it hands off a ready-to-run script to the human tester.

---

## Project Constraints (from CLAUDE.md)

- **Security is non-negotiable.** No code changes that weaken sandboxing guarantees.
- **Commits require DCO sign-off** (`Signed-off-by: Name <email>`).
- **No `.unwrap()` / `.expect()` outside test modules** (enforced by clippy).
- **`make ci` must pass** before milestone archive (currently blocked by pre-existing `disallowed_methods` clippy errors — tracked as a blocker in STATE.md; Phase 13 itself does not introduce code changes so `make ci` status is a pre-condition, not something Phase 13 fixes).
- `nyquist_validation: false` in config.json — no automated test framework section needed.

---

## Complete Inventory of `human_needed` Items

This is the canonical list drawn from the four upstream VERIFICATION.md files. It is the definitive input to the planning step.

### Phase 05 Items (05-VERIFICATION.md)

Source: `.planning/phases/05-windows-detach-readiness-fix/05-VERIFICATION.md` — status `passed`, score 5/5, but two items listed under `Human Verification Required`. [VERIFIED: read file directly]

| Item ID | Test | Expected | Prerequisites |
|---------|------|----------|--------------|
| P05-HV-1 | `nono run --detach -- <long-running-cmd>`, then `nono attach <session-id>` | `nono run --detach` exits within ~2 seconds with "Started detached session" banner; `nono attach` connects via Named Pipe | Windows host, built `nono` binary, any long-running command (e.g., `ping -t 127.0.0.1`) |
| P05-HV-2 | Force `WaitNamedPipeW` to return an unexpected error code | `nono run --detach` returns `SandboxInit` error "Named pipe readiness probe failed with error N" | Windows host; requires either a debug build with injected failure or a simulator; may be impractical — document as "best-effort" |

**Note on P05-HV-2:** Triggering a specific Windows API error code without process manipulation or a custom test harness is not straightforward. The plan should specify that P05-HV-2 can be documented as "impractical without debug tooling — fail-closed path verified by code inspection in 05-VERIFICATION.md" and marked `waived`. [ASSUMED — judgment call; the VERIFICATION.md already states this reason]

### Phase 07 Items (07-VERIFICATION.md and 07-HUMAN-UAT.md)

Source: `.planning/phases/07-quick-wins/07-VERIFICATION.md` — status `passed`, score 5/5, three human-verification items in YAML frontmatter. A stub `07-HUMAN-UAT.md` exists with all three tests in `[pending]` state. [VERIFIED: read files directly]

| Item ID | Test | Expected | Prerequisites |
|---------|------|----------|--------------|
| P07-HV-1 | `nono wrap -- cmd.exe /c exit 42` then `echo %ERRORLEVEL%` | Output is `42`; no `unreachable!()` panic | Windows host, built nono binary, no admin required |
| P07-HV-2 | `nono setup --check-only` | Output contains `'nono wrap' is available on Windows with Job Object + WFP enforcement (no exec-replace, unlike Unix)` and does NOT contain `remain intentionally unavailable` | Windows host, built nono binary, no admin required |
| P07-HV-3 | `nono logs <id>`, `nono inspect <id>`, `nono prune --dry-run` against a real session | Each reads from `~/.config/nono/sessions/` and returns real data; no `UnsupportedPlatform` error | Windows host, at least one completed session record (dependency: P05-HV-1 or any other `nono run` that completes) |

### Phase 09 Items (09-VERIFICATION.md)

Source: `.planning/phases/09-wfp-port-level-proxy-filtering/09-VERIFICATION.md` — status `human_needed`, score 6/7. [VERIFIED: read file directly]

| Item ID | Test | Expected | Prerequisites |
|---------|------|----------|--------------|
| P09-HV-1 | `nono run -- <cmd-that-prints-env>` with `--proxy-only` and a configured network profile | Sandboxed child environment contains `HTTPS_PROXY=http://localhost:<port>` and `NONO_PROXY_TOKEN=<token>` | Windows host, admin (for WFP), `nono-wfp-service` running, a network profile with proxy credentials configured |
| P09-HV-2 | `cargo test -p nono-cli --test wfp_port_integration -- --ignored` | `wfp_port_permit_allows_real_tcp_connection` passes: TCP connect to the ephemeral allowed port succeeds; TCP connect to the blocked port fails | Windows host, **admin privileges**, `nono-wfp-service` running |

**Note on P09-HV-1:** The exact command to print the child environment on Windows is `cmd.exe /c set` or PowerShell `Get-ChildItem Env:`. The proxy profile setup must be done beforehand. This is the most complex item logistically — the tester needs a working proxy config. [VERIFIED: code path analysis in 09-VERIFICATION.md Truth #5]

**Note on P09-HV-2:** After Phase 12, plan 12-02 replaced the hardcoded ports 19876/19877 with ephemeral binding (`:0`). The test should no longer panic on port collision. [VERIFIED: 12-02-PLAN.md task 2]

### Phase 11 Items (11-VERIFICATION.md)

Source: `.planning/phases/11-runtime-capability-expansion/11-VERIFICATION.md` — status `human_needed`, score 4/4 structural. [VERIFIED: read file directly]

| Item ID | Test | Expected | Prerequisites |
|---------|------|----------|--------------|
| P11-HV-1 | End-to-end supervised run: a child that posts a `CapabilityRequest` over the rendezvous pipe | `[nono] Grant access? [y/N]` prompt appears on supervisor console; replying `y` brokers the handle; replying `N` returns Denied | Windows host with attached console, built nono binary, a test child binary that speaks the capability pipe protocol (sends `CapabilityRequest` with `NONO_SESSION_TOKEN` + `NONO_SUPERVISOR_PIPE`); admin NOT required |
| P11-HV-2 | A process spawned at Low Integrity opens the named pipe created by `SupervisorSocket::bind_low_integrity` | Low Integrity child can open the pipe and round-trip a `CapabilityRequest`; no access-denied on the pipe | Windows host; requires a mechanism to spawn a child at Low Integrity (e.g., `icacls` + `CreateProcess` with a restricted token or a test wrapper) |
| P11-HV-3 | Live run with `RUST_LOG=trace` while triggering several `CapabilityRequest` round-trips | Zero log lines contain the `NONO_SESSION_TOKEN` value | Windows host with attached console; `RUST_LOG=trace` set in environment; token value known in advance to grep for |

**Note on P11-HV-1:** The "test child binary" requirement is the key blocker. There is no pre-built test client in the repo. The tester needs to either write a small Rust binary or use a PowerShell script that opens the named pipe (`\\.\pipe\nono-cap-<session>`) and sends a JSON `CapabilityRequest`. [ASSUMED — sourced from 11-VERIFICATION.md description; no test client binary found in codebase]

**Note on P11-HV-2:** Spawning a child at Low Integrity on Windows requires either `WinSaferCreateLevel` + `SaferComputeTokenFromLevel` or duplicating a process token and setting its integrity label. This is non-trivial to do interactively. The SDDL `S:(ML;;NW;;;LW)` on the pipe is verified by code inspection; a full LI-cross-boundary test might be `waived` in favor of documenting the SDDL evidence. [ASSUMED — judgment call based on Windows security API complexity]

**Note on P11-HV-3:** This is the most automatable of the three: set `RUST_LOG=trace`, run a supervised session that performs a capability round-trip, capture output to a file, grep for the token value. [VERIFIED: 11-VERIFICATION.md description]

---

## Item Count Summary

| Phase | Human-Verification Items | Status in Source VERIFICATION.md |
|-------|--------------------------|----------------------------------|
| 05 | 2 (P05-HV-1, P05-HV-2) | `passed` (HV items listed but not blocking phase closure) |
| 07 | 3 (P07-HV-1, P07-HV-2, P07-HV-3) | `passed` (stub UAT at `[pending]`) |
| 09 | 2 (P09-HV-1, P09-HV-2) | `human_needed` (6/7 score) |
| 11 | 3 (P11-HV-1, P11-HV-2, P11-HV-3) | `human_needed` (4/4 structural) |
| **Total** | **10** | — |

This matches the ROADMAP description "10 live-host verification items" and the v1.0-MILESTONE-AUDIT.md tech_debt count. [VERIFIED: cross-referenced all three sources]

---

## Host Prerequisites

### Hard Requirements (items will fail without these)

| Prerequisite | Required By | How to Verify |
|-------------|------------|--------------|
| Windows 10 or 11 host (build 17763+) | All items | `winver` — must show 1809 or later |
| Built `nono` binary (`cargo build -p nono-cli --release`) | All items | `nono --version` |
| `nono-wfp-service` installed and running | P09-HV-1, P09-HV-2 | `sc query nono-wfp-service` shows RUNNING |
| Administrator privileges (elevated prompt) | P09-HV-1, P09-HV-2 | `net session` returns without error |
| At least one completed nono session record | P07-HV-3 | `nono ps` or `ls ~/.config/nono/sessions/` |
| Network profile with proxy credentials configured | P09-HV-1 | `nono setup --check-only` shows proxy configured |

### Soft Requirements (affect specific items)

| Prerequisite | Required By | Fallback |
|-------------|------------|---------|
| Test child binary for capability pipe | P11-HV-1 | Write a minimal PowerShell script that opens the named pipe and sends JSON |
| Mechanism to spawn Low Integrity process | P11-HV-2 | Document SDDL evidence and waive |
| `RUST_LOG=trace` + token capture | P11-HV-3 | Tester manually inspects terminal output |
| Any long-running command to detach from | P05-HV-1 | `ping -t 127.0.0.1` on Windows runs indefinitely |

---

## Dependency Order

Some items must run before others because they create state needed by later tests.

```
Wave 1 (no dependencies — run first):
  P05-HV-1  nono run --detach + nono attach cycle
  P07-HV-2  nono setup --check-only help text
  P09-HV-2  cargo test WFP integration (admin-gated)

Wave 2 (depends on P05-HV-1 creating a session):
  P07-HV-3  nono logs / inspect / prune (needs a session record)

Wave 3 (depends on supervisor being up, no other dependency):
  P07-HV-1  nono wrap exit code
  P09-HV-1  HTTPS_PROXY / NONO_PROXY_TOKEN in child env (needs proxy config)

Wave 4 (complex setup — save for last):
  P11-HV-1  End-to-end supervised + CONIN$ approval prompt
  P11-HV-3  RUST_LOG=trace token-leak audit (can run same session as P11-HV-1)
  P11-HV-2  Low Integrity child connectivity (may be waived)
  P05-HV-2  WaitNamedPipeW fail-closed path (likely waived)
```

---

## UAT Log Format

Based on the established format from `08-UAT.md` and `10-UAT.md` (the most complete examples in this project): [VERIFIED: read both files directly]

```yaml
---
status: [pending|partial|complete|failed]
phase: 13-v1-human-verification-uat
source: [05-VERIFICATION.md, 07-VERIFICATION.md, 09-VERIFICATION.md, 11-VERIFICATION.md]
host:
  windows_build: "Windows 11 Enterprise 10.0.XXXXX"
  admin: [yes|no]
  nono_binary_commit: [git sha]
  wfp_service_running: [yes|no]
started: [ISO timestamp]
updated: [ISO timestamp]
---
```

Each test entry in the body uses the same format as 08-UAT.md and 10-UAT.md:

```markdown
### N. <item name>
expected: <exact expected behavior from source VERIFICATION.md>
command: <exact command to run>
result: [pass|fail|waived|blocked]
notes: <observed output, or reason for waive/block>
```

---

## Outcome Handling per Item

The plan must specify what happens in each result case:

| Result | Action Required |
|--------|----------------|
| `pass` | Update upstream VERIFICATION.md `status` from `human_needed` to `passed`; close the item in `13-UAT.md` |
| `fail` | File a gap in `13-UAT.md` Gaps section; keep upstream VERIFICATION.md at `human_needed`; create a follow-up quick plan if fixable in <1 day |
| `waived` | Document rationale in `13-UAT.md`; update upstream VERIFICATION.md with a `waived` note; treat as `passed` for milestone archive purposes |
| `blocked` | Document blocker; does NOT update upstream VERIFICATION.md; escalate |

---

## Upstream VERIFICATION.md Update Protocol

When Phase 13 completes (or when an item passes):

- **Phase 05 VERIFICATION.md:** Already `status: passed` — add a `human_verification_completed` block in frontmatter noting which items passed/waived.
- **Phase 07 VERIFICATION.md:** Already `status: passed` — same approach; also update `07-HUMAN-UAT.md` to close the `[pending]` items.
- **Phase 09 VERIFICATION.md:** Status is `human_needed`. On all-pass, change to `status: passed` and `score: 7/7`. Update ROADMAP.md Phase 09 row if needed.
- **Phase 11 VERIFICATION.md:** Status is `human_needed`. On all-pass, change to `status: passed` and add human verification completion date.

---

## Automation Classification

Each item's degree of human involvement:

| Item | Can Claude Code Assist? | Human-Only Element |
|------|------------------------|-------------------|
| P05-HV-1 | No — Claude cannot launch live Windows processes | Console observation of detach banner + nono attach session |
| P05-HV-2 | No | Requires process manipulation or debug tooling |
| P07-HV-1 | No | `echo %ERRORLEVEL%` output on Windows console |
| P07-HV-2 | No | Reading `nono setup` binary output on Windows |
| P07-HV-3 | No | Session record creation + command output |
| P09-HV-1 | No | Proxy credential setup + child env inspection |
| P09-HV-2 | Partial — Claude can verify the test file structure; execution requires admin + service | Admin elevation + running WFP service |
| P11-HV-1 | Partial — Claude can write a test child script | Interactive console prompt reply |
| P11-HV-2 | No | Spawning a Low Integrity process |
| P11-HV-3 | Partial — Claude can define the grep pattern | Running trace and capturing output |

---

## Known Risks and Blockers

### Pre-existing CI Failure (CRITICAL PRE-CONDITION)

STATE.md documents a blocker: 48 `clippy::disallowed_methods` errors in `profile/mod.rs`, `config/mod.rs`, and `sandbox_state.rs` (from revert commit `cf5a60a`). This blocks `make ci`. [VERIFIED: STATE.md Blockers section]

**Impact on Phase 13:** Phase 13 itself makes no code changes, so it does not worsen or fix this. However, the milestone cannot be archived until `make ci` passes. The planner must note this as a hard dependency: Phase 13 UAT can run in parallel with the `EnvVarGuard` migration quick plan, but the milestone archive step requires both.

### `nono-wfp-service` Service Status

Items P09-HV-1 and P09-HV-2 require `nono-wfp-service` to be installed and running. This service is only installable via the machine MSI (requires admin). If the tester's environment does not have it, these items are blocked. [VERIFIED: 09-VERIFICATION.md Human Verification Required section]

### Test Child Binary for P11-HV-1

The capability pipe protocol requires a client that sends a JSON `CapabilityRequest` with `session_token` and the correct message framing. No pre-built test client exists in the repository. The plan must include instructions for writing a minimal test client (either a PowerShell script using `[System.IO.Pipes.NamedPipeClientStream]` or a small inline Rust binary). [ASSUMED — no test client found after reviewing repository structure]

### P05-HV-2 and P11-HV-2 Practicality

Both items require unusual Windows API manipulation (injecting API errors, spawning Low Integrity processes) that is not achievable through normal CLI use. Both will likely be `waived` with documented rationale. The plan should pre-authorize waivers for these two items to avoid blocking the UAT. [ASSUMED — judgment call based on item descriptions]

---

## Architecture Patterns

### Pattern 1: UAT as a Human-Executed Script

Phase 13 produces a `13-UAT.md` that functions as a test runbook. The plan is not "Claude executes the tests" — it is "Claude writes the runbook, human executes it, human records results."

This matches the pattern used by Phase 08 (`08-UAT.md` as the sole artifact, with the human recording pass/fail for each criterion). [VERIFIED: 08-UAT.md reviewed directly]

### Pattern 2: Upstream VERIFICATION.md Promotion

Phase 13 is a "closer" phase. Its primary output beyond the UAT log is promoting `human_needed` → `passed` in two VERIFICATION.md files (09, 11) and closing the pending items in the 07 HUMAN-UAT.md. This is a documentation-heavy, no-code phase.

### Pattern 3: Waiver as First-Class Outcome

The project already uses "blocked by prior-phase" as a first-class outcome in `04-UAT.md`. Phase 13 should treat `waived` similarly — a documented, intentional deferral with explicit rationale, not a failure.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Test client for capability pipe | A full Rust binary with complex error handling | A 20-line PowerShell script using `System.IO.Pipes.NamedPipeClientStream` and `System.Text.Json` |
| Low Integrity process spawning | A custom Rust binary | Document that SDDL evidence suffices and waive P11-HV-2 |
| Proxy configuration | A full proxy server | Any existing local proxy (e.g., mitmproxy, Fiddler, or even a simple netcat listener on the configured port) |

---

## Common Pitfalls

### Pitfall 1: Running Tests Out of Order

**What goes wrong:** P07-HV-3 runs before any session exists, returns "no sessions found" and is marked fail.
**Why it happens:** Session commands need a pre-existing session record. The tester does not read the dependency order.
**How to avoid:** The plan must explicitly list Wave 1 items (which create sessions) before Wave 2 items (which consume them).

### Pitfall 2: Forgetting to Elevate Before WFP Tests

**What goes wrong:** P09-HV-2 runs in a non-elevated prompt; the test fails with an access-denied error at WFP filter install time rather than the expected TCP-level enforcement behavior.
**How to avoid:** The plan must include a pre-flight checklist: `net session` succeeds → admin confirmed before running any WFP item.

### Pitfall 3: Updating Upstream VERIFICATION.md Before All Items Pass

**What goes wrong:** The planner updates Phase 09 VERIFICATION.md to `passed` after P09-HV-2 passes but P09-HV-1 is still blocked on proxy config. The audit then shows a false-completed state.
**How to avoid:** Only update upstream VERIFICATION.md after the full UAT log is complete and all pass/waive verdicts are final.

### Pitfall 4: `nono-wfp-service` Not Running Silently Fails

**What goes wrong:** P09-HV-1 runs the `nono run --proxy-only` command but `nono-wfp-service` is not running; the command fails with a generic "Failed to connect" error that does not look like a WFP issue, and the tester records it as a nono code bug.
**How to avoid:** Pre-flight check: `sc query nono-wfp-service` before any WFP item.

### Pitfall 5: Session Token Grep Pattern Too Broad

**What goes wrong:** P11-HV-3 greps for a substring of the token that happens to appear in other log lines (e.g., a hex string used elsewhere), producing false positives.
**How to avoid:** The plan must specify that the tester captures the full token value from `NONO_SESSION_TOKEN` env var output and greps for that exact 64-character hex string.

---

## Code Examples

No code is written in Phase 13. The following examples are from existing codebase for reference when writing the test runbook.

### Capability Pipe Named Pipe Path Format

From `11-VERIFICATION.md` Key Link table: [VERIFIED: 11-VERIFICATION.md]

```
\\.\pipe\nono-cap-<session_id>
```

The session token is available in `NONO_SESSION_TOKEN` env var; the pipe path in `NONO_SUPERVISOR_PIPE`.

### PowerShell Pipe Client Sketch (for P11-HV-1)

This is what the plan should instruct the tester to create: [ASSUMED — standard PowerShell named pipe API]

```powershell
# Read env vars set by nono for the supervised child
$token = $env:NONO_SESSION_TOKEN
$pipePath = $env:NONO_SUPERVISOR_PIPE  # e.g., \\.\pipe\nono-cap-<id>

$pipe = [System.IO.Pipes.NamedPipeClientStream]::new(".", $pipePath.Replace("\\.\pipe\",""), [System.IO.Pipes.PipeDirection]::InOut)
$pipe.Connect(5000)

$request = '{"session_token":"' + $token + '","capability":{"fs":{"path":"C:\\Temp","access":"read"}}}'
$bytes = [System.Text.Encoding]::UTF8.GetBytes($request + "`n")
$pipe.Write($bytes, 0, $bytes.Length)
$pipe.Flush()

$reader = [System.IO.StreamReader]::new($pipe)
$response = $reader.ReadLine()
Write-Output "Response: $response"
$pipe.Dispose()
```

### `nono run --detach` Expected Console Output (for P05-HV-1)

Based on Phase 05 code: the detach flow exits with a banner message within ~2 seconds. The exact string is not fixed in a test, but the VERIFICATION.md says "Started detached session" banner. [VERIFIED: 05-VERIFICATION.md Truth #1 description]

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | P05-HV-2 (fail-closed path) will be waived as impractical | Item Inventory, Risks | If the user wants it exercised, the plan needs a debug build or process injection approach |
| A2 | P11-HV-2 (Low Integrity child) will be waived in favor of SDDL evidence | Item Inventory, Risks | If the user wants a live LI-cross-boundary test, needs a custom test harness |
| A3 | No pre-built capability pipe test client exists in the repo | Item Inventory P11-HV-1 | If one exists somewhere (scripts/, tests/), the PowerShell script is unnecessary |
| A4 | PowerShell `NamedPipeClientStream` is usable for P11-HV-1 | Code Examples | If the message framing is more complex (length-prefixed binary, not newline-delimited JSON), the sketch above needs adjustment — check `crates/nono/src/supervisor/socket_windows.rs` message framing |
| A5 | The Phase 12 `EnvVarGuard` migration blocker is addressed by a separate quick plan | Pre-conditions | If it is not fixed before Phase 13 archive, the milestone cannot close |

---

## Open Questions

1. **Message framing for capability pipe (P11-HV-1)**
   - What we know: The pipe exists at `\\.\pipe\nono-cap-<session_id>`. Requests use `CapabilityRequest` JSON. The pipe server is started by `start_capability_pipe_server` in `supervisor.rs`.
   - What's unclear: Whether messages are newline-delimited or length-prefixed. The PowerShell sketch assumes newline-delimited.
   - Recommendation: The plan should instruct the planner to read `crates/nono/src/supervisor/socket_windows.rs` (specifically the `read_message` / `write_message` functions) to confirm framing before finalizing the test client script.

2. **`nono-wfp-service` installation for P09 items**
   - What we know: The machine MSI installs the service. Admin is required.
   - What's unclear: Whether the MSI is available locally on the tester's machine or needs to be built from the current branch.
   - Recommendation: The plan should include a pre-flight step that verifies service status and provides the `cargo build -p nono-wfp-service` or MSI install path if it is not running.

3. **Proxy configuration for P09-HV-1**
   - What we know: `HTTPS_PROXY` and `NONO_PROXY_TOKEN` must be injected by the supervisor into the child. The injection path is wired (09-VERIFICATION.md Truth #5 code path confirmed).
   - What's unclear: The exact CLI incantation to configure a network profile with proxy credentials. What does `--proxy-only` take as arguments? Does the tester need a real upstream proxy or does any TCP listener on the proxy port suffice?
   - Recommendation: The plan should note that a minimal test is possible with `netcat` or `python -m http.server` as the "proxy" — the UAT only needs to confirm the env vars appear in the child, not that the proxy successfully intercepts traffic.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Windows 10/11 host | All items | Assumed (platform is win32 per env) | Windows 11 Enterprise 10.0.26200 | None — phase is Windows-only |
| `nono` binary | All items | Must build | Current branch HEAD | `cargo build -p nono-cli --release` |
| `nono-wfp-service` | P09-HV-1, P09-HV-2 | Unknown — not verified | — | Build from source or MSI |
| Admin elevation | P09-HV-1, P09-HV-2 | Unknown — depends on user's session | — | P09 items blocked if no admin |
| `cargo` (for test run) | P09-HV-2 | Assumed present (Rust workspace) | — | None |

**Missing dependencies with no fallback:**
- Admin privileges for P09 items — if unavailable, those items are blocked.
- `nono-wfp-service` for P09 items — if not running, those items are blocked.

**Note:** The current working environment (this Claude Code session) is the Windows development machine. The user's environment has admin if they are the owner of the machine (`omack`). [ASSUMED — cannot verify elevation from this session]

---

## Sources

### Primary (HIGH confidence)
- `.planning/phases/05-windows-detach-readiness-fix/05-VERIFICATION.md` — P05 human verification items (exact wording)
- `.planning/phases/07-quick-wins/07-VERIFICATION.md` — P07 human verification items (YAML frontmatter)
- `.planning/phases/07-quick-wins/07-HUMAN-UAT.md` — P07 stub UAT, all items `[pending]`
- `.planning/phases/09-wfp-port-level-proxy-filtering/09-VERIFICATION.md` — P09 human verification items (status `human_needed`, 6/7)
- `.planning/phases/11-runtime-capability-expansion/11-VERIFICATION.md` — P11 human verification items (status `human_needed`, 4/4 structural)
- `.planning/v1.0-MILESTONE-AUDIT.md` — tech_debt frontmatter (canonical count of 10 items across 4 phases)
- `.planning/phases/08-conpty-shell/08-UAT.md` — UAT log format reference
- `.planning/phases/10-etw-based-learn-command/10-UAT.md` — UAT log format reference (more detailed)
- `.planning/STATE.md` — blocker: pre-existing clippy errors blocking `make ci`

### Secondary (MEDIUM confidence)
- `.planning/phases/12-milestone-bookkeeping-cleanup/12-02-PLAN.md` — confirms ephemeral port fix applied to `wfp_port_integration.rs`

### Tertiary (LOW confidence / ASSUMED)
- PowerShell `NamedPipeClientStream` sketch — standard Windows API, but message framing unconfirmed
- Waiver recommendations for P05-HV-2 and P11-HV-2 — judgment calls

---

## Metadata

**Confidence breakdown:**
- Item inventory (10 items, their descriptions, and prerequisites): HIGH — read directly from source files
- Dependency order: HIGH — based on obvious data dependencies between items
- Host prerequisites: HIGH for hard requirements; MEDIUM for soft requirements
- PowerShell test client sketch: MEDIUM — standard API but framing unconfirmed
- Waiver recommendations: LOW — judgment calls tagged [ASSUMED]

**Research date:** 2026-04-12
**Valid until:** Stable — this research describes static planning artifacts, not evolving library APIs
