# Phase 9: WFP Port-Level + Proxy Filtering — Research

**Researched:** 2026-04-09
**Domain:** Windows Filtering Platform (WFP) port-level filtering, proxy credential injection
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

1. **IPC Protocol Version: No bump** — `WFP_RUNTIME_PROTOCOL_VERSION` stays at `1`. The fields `tcp_connect_ports`, `tcp_bind_ports`, and `localhost_ports` already exist in `WfpRuntimeActivationRequest` and the WFP service already parses and handles them. Populating them from the CLI is not a wire format change.

2. **`--proxy-only` Without Proxy Config: Hard Error** — If `--proxy-only` (i.e., ProxyOnly network mode) is set but `proxy.active` is false, fail with a clear error before touching WFP:
   ```
   Cannot use --proxy-only without a network profile or credential configuration.
   ```

3. **Unsupported Marker Removal: All Three at Once** — Remove all three `unsupported` markers from `compile_network_policy()` in a single plan: `PortConnectAllowlist`, `PortBindAllowlist`, `LocalhostPortAllowlist`. All three are handled by the same `build_policy_filter_specs()` path.

4. **SC5 Connection Test: Integration Test, Windows CI Only** — Write a `#[cfg(target_os = "windows")]` integration test that binds a loopback echo server, activates WFP with that port in the connect allowlist, and confirms `connect()` succeeds. Lives in `tests/` directory matching the existing `test_network_wfp.sh` pattern.

### Claude's Discretion

None captured.

### Deferred Ideas (OUT OF SCOPE)

None captured during this discussion.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PORT-01 | User can allow specific ports for outbound TCP connections on Windows using `--allow-port`; bind and connect allowlists operate independently; WFP permit filters have higher weight than the block-all filter. | `compile_network_policy()` already sets `tcp_connect_ports` and `tcp_bind_ports` correctly; only the unsupported markers block it. Filter weight logic confirmed: Permit=100/20, Block=0/10. |
| PROXY-01 | User can route sandboxed agent traffic through a local proxy via proxy mode with `HTTPS_PROXY` credential injection; a WFP loopback permit filter ensures the proxy port is reachable; all other outbound traffic is blocked. | `proxy_env_vars` plumbing through `execution_runtime.rs` is complete on Windows. `ProxyOnly` mode already adds proxy port to `localhost_ports`. The missing piece is a pre-flight guard when `proxy.active` is false. |
</phase_requirements>

---

## Summary

Phase 9 closes two feature gaps on Windows: port-granular WFP network policy (`PORT-01`) and proxy credential injection with `--proxy-only` enforcement (`PROXY-01`). Both gaps are almost entirely bridged by existing code — the work is primarily removal of three "unsupported" markers and addition of one pre-flight guard.

The library (`crates/nono/src/sandbox/windows.rs`) marks `PortConnectAllowlist`, `PortBindAllowlist`, and `LocalhostPortAllowlist` as unsupported in `compile_network_policy()`. This causes `prepare_network_enforcement()` in the CLI to bail with `UnsupportedPlatform` before WFP is touched. The underlying WFP service (`nono-wfp-service.rs`) already implements `build_policy_filter_specs()` with full per-port permit filter installation for all three port types. The `WfpRuntimeActivationRequest` wire format already carries all three port arrays. No service changes, no protocol bump, no new filter code.

For proxy: `proxy_env_vars` (which contains `HTTPS_PROXY` and `NONO_PROXY_TOKEN`) already flows from `start_proxy_runtime()` through `execution_runtime.rs` into `ExecConfig.env_vars` on Windows. `ProxyOnly` mode already adds the proxy port to `localhost_ports` in the WFP activation request. The only missing piece is a pre-flight guard in `prepare_proxy_launch_options()` (or `execution_runtime.rs`) that fails fast when `caps.network_mode()` is `ProxyOnly` but `proxy.active` is false.

**Primary recommendation:** Remove the three unsupported markers in `compile_network_policy()` (Plan 09-01), add the proxy pre-flight guard (Plan 09-02), and write a real TCP connection integration test for SC5 (Plan 09-03).

---

## Standard Stack

No new dependencies required. All work is within existing crates.

### Core (existing)
| Crate | Version | Role in Phase |
|-------|---------|---------------|
| `nono` (library) | workspace | `compile_network_policy()` — remove unsupported markers |
| `nono-cli` | workspace | `prepare_network_enforcement()`, proxy pre-flight guard, env injection |
| `nono-wfp-service` (bin) | workspace | Already complete — no changes needed |
| `windows-sys` | 0.59 | WFP FFI — already in scope, no new feature flags needed |

**Installation:** No new dependencies. `[VERIFIED: codebase inspection]`

---

## Architecture Patterns

### How Port Filtering Flows (End-to-End)

```
CLI flags (--allow-port N, --allow-bind N)
    ↓  capability_ext.rs
CapabilitySet.add_localhost_port(N)          ← allow-port maps to localhost_port
CapabilitySet.add_tcp_bind_port(N)           ← allow-bind maps to tcp_bind_port
    ↓  execution_runtime.rs → prepare_network_enforcement()
Sandbox::windows_network_policy(caps)        ← calls compile_network_policy()
    ↓  compile_network_policy() [windows.rs]
WindowsNetworkPolicy {                        ← CURRENTLY marks these unsupported
    tcp_connect_ports: [N],
    tcp_bind_ports: [N],
    localhost_ports: [N],
    unsupported: [PortConnectAllowlist, PortBindAllowlist, LocalhostPortAllowlist],  ← REMOVE THESE
}
    ↓  build_wfp_runtime_activation_request()
WfpRuntimeActivationRequest {                 ← already passes tcp_connect_ports through (line 451)
    tcp_connect_ports: [N],
    tcp_bind_ports: [N],
    localhost_ports: [N],
}
    ↓  WFP service: build_policy_filter_specs()
PolicyFilterSpec { action: Permit, port: Remote(N), weight: 100 }  ← ALREADY COMPLETE
PolicyFilterSpec { action: Block, port: None, weight: 0 }
```

**Key insight:** `--allow-port` in the CLI maps to `add_localhost_port()`, not `add_tcp_connect_port()`. This adds the port to `localhost_ports` (loopback-only, both connect and bind). The `tcp_connect_ports` path is populated only from Linux-specific `--allow-proxy` domain parsing and profile network allow_domain on Linux. `--allow-bind` maps to `tcp_bind_port`. `[VERIFIED: codebase inspection of capability_ext.rs lines 363–365]`

### How Proxy Env Injection Flows

```
start_proxy_runtime() [proxy_runtime.rs]
    ↓
proxy_handle.env_vars()           ← contains HTTPS_PROXY=http://localhost:PORT
proxy_handle.credential_env_vars()  ← contains NONO_PROXY_TOKEN=...
    ↓
active_proxy.env_vars = [(key, value), ...]
    ↓
execution_runtime.rs lines 177–179
    env_vars.push((key, value))   ← merged with loaded_secrets
    ↓
ExecConfig { env_vars, ... }      ← Windows ExecConfig at line 261–269
    ↓
child process environment         ← injected at spawn time
```

`[VERIFIED: codebase inspection of proxy_runtime.rs lines 217–224 and execution_runtime.rs lines 165–179]`

### Pattern: Pre-Flight Guard Before WFP

The guard for `--proxy-only` without proxy config must fire **before** `prepare_network_enforcement()` is called. Two candidate insertion points:

1. **In `execution_runtime.rs`** after `start_proxy_runtime()` and before the Windows `ExecConfig` construction: check if `caps.network_mode()` is `ProxyOnly` but `proxy.active` was false.
2. **In `prepare_proxy_launch_options()` in `proxy_runtime.rs`**: detect when `has_proxy_flags()` is false but caps would result in ProxyOnly, and fail early.

Option 1 is simpler because `proxy.active` is already evaluated at that point and `caps` has been updated by `start_proxy_runtime()`. The guard reads: "if the caps are in ProxyOnly mode but no proxy started (proxy.active == false), return error."

`[VERIFIED: codebase inspection of execution_runtime.rs and proxy_runtime.rs]`

### Filter Weight Architecture (Confirmed, No Changes Needed)

```
With security descriptor (SID-scoped):
  Permit filters: weight = 100
  Block filters:  weight = 0

Without security descriptor:
  Permit filters: weight = 20
  Block filters:  weight = 10
```

Permit filters always outrank block-all filters within the NONO sublayer. No weight changes needed. `[VERIFIED: nono-wfp-service.rs lines 1289–1298]`

### Integration Test Pattern

The existing `tests/integration/test_network_wfp.sh` tests the block mode at the shell level. For SC5 (real TCP connection), the CONTEXT.md specifies a Rust `#[cfg(target_os = "windows")]` integration test. The correct home is in a `tests/` directory at the workspace root or within `crates/nono-cli/tests/`. Pattern:

```rust
#[cfg(target_os = "windows")]
#[test]
fn wfp_port_permit_allows_real_tcp_connection() {
    // 1. Bind a loopback echo server on a specific port (e.g., 19876)
    // 2. Build a CapabilitySet with Blocked mode + localhost_port(19876)
    // 3. Activate WFP with that caps set
    // 4. Confirm connect() to 127.0.0.1:19876 succeeds
    // 5. Confirm connect() to a non-allowlisted port fails (negative test)
}
```

Requires admin privileges (WFP filter installation). Must be tagged `#[cfg(target_os = "windows")]` and documented in CI configuration. `[ASSUMED]` — exact test file location and CI tagging convention needs planner to confirm against existing test harness.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Per-port WFP permit filters | Custom filter installation code | Existing `build_policy_filter_specs()` in `nono-wfp-service.rs` | Already complete — handles connect, bind, localhost port types |
| Proxy env var collection | Manual env var building | `proxy_handle.env_vars()` + `credential_env_vars()` | Already returns `HTTPS_PROXY` + credential vars |
| Port deduplication | Manual sort + dedup | `merge_dedup_ports()` in `proxy_runtime.rs` | Already handles combined port lists |
| Protocol versioning | New version bump | Version stays at `1` | Fields already in wire format |

---

## Common Pitfalls

### Pitfall 1: `--allow-port` Maps to `localhost_ports`, Not `tcp_connect_ports`

**What goes wrong:** Developer reads the CLI help for `--allow-port` and assumes it maps to `tcp_connect_ports`, then writes tests checking `tcp_connect_ports`.
**Why it happens:** The naming is non-obvious. `--allow-port` implies "allow TCP to port N" but the implementation maps it to `localhost_ports` (loopback-only, both connect and bind). This is documented in `capability_ext.rs` lines 362–365.
**How to avoid:** When removing the `LocalhostPortAllowlist` unsupported marker (not `PortConnectAllowlist`), unit tests must assert `policy.localhost_ports == [N]`, not `policy.tcp_connect_ports`.
**Warning signs:** Test for `--allow-port` verifying `tcp_connect_ports` would pass zero ports.

`[VERIFIED: capability_ext.rs lines 362–365]`

### Pitfall 2: Removing the `is_fully_supported()` Guard in `prepare_network_enforcement()`

**What goes wrong:** After removing the three unsupported markers from `compile_network_policy()`, the `is_fully_supported()` check in `prepare_network_enforcement()` (lines 1498–1505) would pass for port-populated caps. This is correct. BUT: if the plan removes the markers without also verifying the WFP service activation request actually carries the ports, the ports would be silently dropped.
**Why it happens:** The path `build_wfp_runtime_activation_request()` already passes `policy.tcp_connect_ports` through at line 451. This is confirmed complete. But `tcp_bind_ports` and `localhost_ports` are merged with `ProxyOnly` bind_ports — verify the non-ProxyOnly case also passes them.
**How to avoid:** Unit test `build_wfp_runtime_activation_request()` with port-populated policies in non-ProxyOnly mode to confirm all three port lists appear in the outgoing request.

`[VERIFIED: network.rs lines 428–436]`

### Pitfall 3: Pre-Flight Guard Fires on ProxyOnly Mode Set by `start_proxy_runtime()`

**What goes wrong:** `start_proxy_runtime()` itself calls `caps.set_network_mode_mut(ProxyOnly { port, ... })` after starting the proxy. If the guard checks `caps.network_mode() == ProxyOnly` AFTER this call, it would fire on legitimate proxy use.
**Why it happens:** The caps mode is mutated by `start_proxy_runtime()` at line 212 in `proxy_runtime.rs`. The guard must check the original request (before proxy start), not the post-mutation state.
**How to avoid:** The guard must check whether ProxyOnly was requested as part of the original capabilities AND `proxy.active` was false — not whether caps are currently in ProxyOnly mode after `start_proxy_runtime()` runs. Check `proxy.active` before calling `start_proxy_runtime()`, or check the pre-mutation network mode.

`[VERIFIED: proxy_runtime.rs line 212 and execution_runtime.rs lines 165–179]`

### Pitfall 4: Integration Test Requires Admin Privileges

**What goes wrong:** The Rust integration test for SC5 requires admin privileges to install WFP filters. If run without elevation, `FwpmFilterAdd0` returns an access denied error, and the test fails with an opaque message.
**Why it happens:** WFP filter installation requires SE_MANAGE_VOLUME_PRIVILEGE or equivalent. The existing `test_network_wfp.sh` already checks for this with `net session`.
**How to avoid:** The integration test must either (a) skip gracefully when not elevated, or (b) be marked as requiring admin in the CI configuration and only run in the privileged test matrix. Document clearly.

`[ASSUMED: exact privilege check API in Rust on Windows needs verification at implementation time]`

### Pitfall 5: Existing Test `compile_network_policy_carries_port_filters_into_wfp_policy` Asserts `unsupported.len() == 3`

**What goes wrong:** `windows.rs` line 1554 currently asserts `policy.unsupported.len() == 3` as a passing test. After removing the markers, this test will fail.
**Why it happens:** The test was written to verify the current (incomplete) behavior — that ports populate both the port lists AND the unsupported list.
**How to avoid:** When removing the markers from `compile_network_policy()`, update this existing test to assert `unsupported` is empty and `is_fully_supported()` is true.

`[VERIFIED: windows.rs lines 1545–1560]`

---

## Code Examples

### Removing the Unsupported Markers (Plan 09-01)

Current state in `crates/nono/src/sandbox/windows.rs` `compile_network_policy()`:

```rust
// CURRENT (lines 306–320) — REMOVE ALL THREE BLOCKS:
if !caps.tcp_connect_ports().is_empty() {
    unsupported.push(crate::sandbox::WindowsUnsupportedNetworkIssue {
        kind: crate::sandbox::WindowsUnsupportedNetworkIssueKind::PortConnectAllowlist,
    });
}
if !caps.tcp_bind_ports().is_empty() {
    unsupported.push(crate::sandbox::WindowsUnsupportedNetworkIssue {
        kind: crate::sandbox::WindowsUnsupportedNetworkIssueKind::PortBindAllowlist,
    });
}
if !caps.localhost_ports().is_empty() {
    unsupported.push(crate::sandbox::WindowsUnsupportedNetworkIssue {
        kind: crate::sandbox::WindowsUnsupportedNetworkIssueKind::LocalhostPortAllowlist,
    });
}
```

After removal, the `unsupported` vec stays empty for port-populated caps. The enum variants `WindowsUnsupportedNetworkIssueKind::PortConnectAllowlist`, `PortBindAllowlist`, `LocalhostPortAllowlist` may become dead code — check if they can be removed or must be kept for future use. `[VERIFIED: windows.rs lines 306–320, mod.rs lines 323–328]`

### Unit Test Shape for Plan 09-01

```rust
// Source: pattern from existing test at windows.rs:1546
#[cfg(target_os = "windows")]
#[test]
fn compile_network_policy_with_connect_ports_is_fully_supported() {
    let mut caps = CapabilitySet::new().set_network_mode(NetworkMode::Blocked);
    caps.add_tcp_connect_port(443);
    let policy = compile_network_policy(&caps);
    assert!(policy.is_fully_supported(), "port connect should now be supported");
    assert_eq!(policy.tcp_connect_ports, vec![443]);
    assert!(policy.unsupported.is_empty());
}

#[cfg(target_os = "windows")]
#[test]
fn compile_network_policy_with_localhost_ports_is_fully_supported() {
    let mut caps = CapabilitySet::new().set_network_mode(NetworkMode::Blocked);
    caps.add_localhost_port(8080); // --allow-port maps here
    let policy = compile_network_policy(&caps);
    assert!(policy.is_fully_supported());
    assert_eq!(policy.localhost_ports, vec![8080]);
    assert!(policy.unsupported.is_empty());
}
```

### Proxy Pre-Flight Guard Shape (Plan 09-02)

```rust
// In execution_runtime.rs, after start_proxy_runtime() call but before ExecConfig construction.
// The proxy.active field is evaluated by prepare_proxy_launch_options() before start_proxy_runtime().
// Source: execution_runtime.rs lines 162-168, proxy_runtime.rs lines 52-78

// If the network mode that was requested requires a proxy but none was configured:
if matches!(
    prepared_caps_before_proxy_mutation.network_mode(),
    nono::NetworkMode::ProxyOnly { .. }
) && !proxy.active {
    return Err(NonoError::SandboxInit(
        "Cannot use --proxy-only without a network profile or credential configuration."
            .to_string(),
    ));
}
```

Note: The exact insertion point depends on whether the guard is in `execution_runtime.rs` (after `start_proxy_runtime`) or in `proxy_runtime.rs` (in `prepare_proxy_launch_options`). The caps mutation happens inside `start_proxy_runtime`, so the check must use the pre-mutation mode OR check `proxy.active` directly.

### `WINDOWS_SUPPORTED_DETAILS` String Update

After removing the markers, update the `WINDOWS_SUPPORTED_DETAILS` constant in `windows.rs` line 28–35 to remove "port-level network filtering" from the unsupported list:

```rust
// Current (line 33): "port-level network filtering, runtime capability expansion..."
// After: remove "port-level network filtering" from that list
const WINDOWS_SUPPORTED_DETAILS: &str =
    "Windows sandbox enforcement supports directory read and directory read-write grants, \
     blocked network mode, port-level network filtering (connect, bind, and localhost ports), \
     and default signal/process/ipc modes. Single-file grants, write-only directory grants, \
     runtime capability expansion, and platform-specific rules are not in the supported subset. ...";
```

`[VERIFIED: windows.rs lines 28–35]`

---

## Runtime State Inventory

Not applicable — this phase involves code changes only. No stored data, live service config, OS-registered state, secrets, or build artifacts carry state that must be migrated. WFP filters are installed and removed per-session at runtime; no persistent named state exists that needs renaming.

**Nothing found in any category — verified by codebase inspection.**

---

## Open Questions (RESOLVED)

1. **Dead Code from Removed Enum Variants** — **RESOLVED:** Plan 09-01 keeps the `WindowsUnsupportedNetworkIssueKind::PortConnectAllowlist`, `PortBindAllowlist`, and `LocalhostPortAllowlist` variants in place. Plan 09-01 Task 1 explicitly documents: "Do NOT remove the enum variants... They are referenced in exhaustive match arms (`label()` and `description()`) and removing them would break those match blocks. The variants remain valid for documentation/diagnostic purposes even if nothing currently constructs them." No `#[allow(dead_code)]` is required because the variants are still referenced by the match arms. CLAUDE.md compliance preserved.

2. **Integration Test Location and CI Privilege** — **RESOLVED:** Plan 09-03 places the test at `crates/nono-cli/tests/wfp_port_integration.rs` with `#![cfg(target_os = "windows")]` guard. The admin-requiring real-connection test is marked `#[ignore]` so standard CI runs skip it, while elevated Windows runs can invoke it with `cargo test -- --ignored`. A second test in the same file verifies the policy compilation path without any privilege requirement and runs in all Windows CI.

3. **`--proxy-only` as CLI Concept vs. `has_proxy_flags()`** — **RESOLVED:** No new CLI flag is introduced. Plan 09-02 implements the guard by checking `caps.network_mode() == ProxyOnly` (which is set by the profile path at `capability_ext.rs:535` when `profile.network.has_proxy_flags()` is true) AND `proxy.active == false` (computed in `proxy_runtime.rs:52-78`). "--proxy-only" in CONTEXT.md success criteria refers to ProxyOnly network mode generally, not a new flag.

---

## Environment Availability

Step 2.6: SKIPPED — this phase makes no use of external tools beyond the Rust toolchain already in use. WFP is a Windows kernel facility, not an external dependency that needs probing.

---

## Validation Architecture

`nyquist_validation` is `false` in `.planning/config.json`. This section is omitted per configuration.

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V4 Access Control | yes | WFP sublayer weight ensures permit > block; no capability escalation path |
| V5 Input Validation | yes | Port numbers are `u16` — range validation is structural. WFP request validation in `nono-wfp-service.rs` already validates `network_mode` string and `protocol_version`. |
| V6 Cryptography | no | No crypto in this phase |
| V2 Authentication | no | No auth changes |
| V3 Session Management | no | Session lifecycle unchanged |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Port allowlist bypass via high-weight external filter | Tampering | NONO sublayer owns its filters (deterministic GUID keys); no external filters in the sublayer can override within it. Block-all is within the same sublayer so weight ordering is deterministic. |
| Proxy env var leakage | Information Disclosure | `NONO_PROXY_TOKEN` is injected only into child env, not logged. `zeroize` crate handles credential clearing in `keystore.rs`. |
| `--proxy-only` with no proxy → unprotected traffic | Elevation of Privilege | Locked decision: hard error before WFP is touched. Fail-secure by design. |
| Port 0 in allowlist (placeholder before proxy starts) | Tampering | `start_proxy_runtime()` replaces port 0 with the real bound port via `set_network_mode_mut()`. WFP is only called after this mutation. The pre-flight guard on `proxy.active` ensures port 0 never reaches WFP in an invalid configuration. |

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `unsupported` markers block port filtering | Remove markers, enable full WFP per-port enforcement | This phase | Port-granular network policy on Windows reaches parity with Landlock V4 on Linux |
| Proxy env vars present in code but unverified on Windows | Verified path and guard added | This phase | PROXY-01 closure |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Integration test for SC5 should live in `crates/nono-cli/tests/wfp_port_integration.rs` | Architecture Patterns, Open Questions | Low — location is a planner decision; functional outcome unchanged |
| A2 | Exact privilege check API in Rust for Windows admin detection needs verification at implementation time | Common Pitfalls (Pitfall 4) | Low — alternative is a graceful skip/ignore if WFP returns access denied |
| A3 | `WindowsUnsupportedNetworkIssueKind` enum variants may become dead code after marker removal | Open Questions | Low — if used in match arms elsewhere, compiler will report; CLAUDE.md forbids dead_code allows so it will be caught |

---

## Sources

### Primary (HIGH confidence — codebase inspection)
- `crates/nono/src/sandbox/windows.rs` lines 295–350 — `compile_network_policy()` implementation and existing test at line 1546
- `crates/nono/src/sandbox/mod.rs` lines 290–456 — `WindowsNetworkPolicy`, `WindowsNetworkPolicyMode`, `WindowsUnsupportedNetworkIssueKind` types
- `crates/nono-cli/src/exec_strategy_windows/network.rs` lines 420–505, 1494–1518 — `build_wfp_runtime_activation_request()` and `prepare_network_enforcement()`
- `crates/nono-cli/src/windows_wfp_contract.rs` — `WfpRuntimeActivationRequest` struct (all three port arrays confirmed present)
- `crates/nono-cli/src/bin/nono-wfp-service.rs` lines 1009–1094, 1289–1298 — `build_policy_filter_specs()` and filter weight constants
- `crates/nono-cli/src/proxy_runtime.rs` lines 182–232 — `start_proxy_runtime()`, env var collection
- `crates/nono-cli/src/execution_runtime.rs` lines 155–270 — `proxy_env_vars` plumbed into `ExecConfig.env_vars` on Windows
- `crates/nono-cli/src/capability_ext.rs` lines 362–365 — `--allow-port` maps to `add_localhost_port()`

### Secondary (MEDIUM confidence)
- `.planning/phases/09-wfp-port-level-proxy-filtering/09-CONTEXT.md` — user locked decisions and codebase map

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; all work in existing crates verified by inspection
- Architecture: HIGH — end-to-end data flow verified by reading all relevant source files
- Pitfalls: HIGH — all pitfalls derived from actual code state, not assumptions
- Integration test shape: MEDIUM — location and CI privilege handling contain assumed details

**Research date:** 2026-04-09
**Valid until:** 2026-05-09 (stable codebase; no fast-moving ecosystem dependencies)
