# Phase 9 Context: WFP Port-Level + Proxy Filtering

## Phase Summary

**Goal:** Wire up port-granular WFP network policy and route sandboxed agent traffic through a local proxy with credential injection on Windows.

**Requirements:** PORT-01, PROXY-01

**Success Criteria:**
1. `--allow-port <N>` creates a WFP permit filter for TCP connect to port N; reachable from inside sandbox.
2. `--proxy-only` routes all sandboxed outbound traffic through the configured proxy port; all other outbound TCP blocked.
3. Bind and connect port allowlists operate independently.
4. `HTTPS_PROXY` and `NONO_PROXY_TOKEN` injected via `ExecConfig.env_vars`.
5. WFP per-port permit filters carry higher weight than block-all; confirmed by a real TCP connection test.

---

## Decisions

### 1. IPC Protocol Version: No bump

`WFP_RUNTIME_PROTOCOL_VERSION` stays at `1`. The `tcp_connect_ports`, `tcp_bind_ports`, and `localhost_ports` fields already exist in `WfpRuntimeActivationRequest` and the WFP service already parses and handles them. Populating these fields from the CLI is not a wire format change — it's plumbing that was deferred. Bumping would require service reinstall on upgrade for no real benefit.

### 2. `--proxy-only` Without Proxy Config: Hard Error

If `--proxy-only` is set but `proxy.active` is false (no `--network-profile`, no credentials, no upstream proxy configured), fail with a clear error **before** touching WFP:

```
Cannot use --proxy-only without a network profile or credential configuration.
```

Rationale: fail-secure. `--proxy-only` implies traffic must flow through a proxy. If there's no proxy to allow through the loopback permit filter, there's nothing to allow — the user made a configuration mistake and must fix it.

### 3. Unsupported Marker Removal: All Three at Once

Remove all three `unsupported` markers from `compile_network_policy()` in a single plan:
- `PortConnectAllowlist`
- `PortBindAllowlist`
- `LocalhostPortAllowlist`

All three are handled by the same `build_policy_filter_specs()` path in `nono-wfp-service.rs`. No reason to ship two half-done patches. One plan removes all three markers and verifies each filter type works.

### 4. SC5 Connection Test: Integration Test, Windows CI Only

Write a `#[cfg(target_os = "windows")]` integration test in `tests/` that:
1. Binds a loopback echo server on a specific port
2. Activates WFP with that port in the connect allowlist
3. Confirms a real `connect()` succeeds

Lives in the existing Windows integration test harness (`tests/windows-test-harness.ps1` pattern). Does not need to run on non-Windows CI. This satisfies SC5's "real TCP connection test" requirement.

---

## Codebase Map (what to read)

### Core files for this phase

- `crates/nono/src/sandbox/windows.rs` — `compile_network_policy()`: remove `PortConnectAllowlist`, `PortBindAllowlist`, `LocalhostPortAllowlist` from `unsupported`
- `crates/nono/src/sandbox/mod.rs` (lines 290–396) — `WindowsNetworkPolicyMode`, `WindowsNetworkPolicy`, `WindowsUnsupportedNetworkIssueKind`
- `crates/nono-cli/src/exec_strategy_windows/network.rs` — `prepare_network_enforcement()`, `build_wfp_runtime_activation_request()`, `build_wfp_target_activation_request()`
- `crates/nono-cli/src/windows_wfp_contract.rs` — `WfpRuntimeActivationRequest` struct (fields already present, no changes needed)
- `crates/nono-cli/src/bin/nono-wfp-service.rs` — `build_policy_filter_specs()` (handles per-port filters; already complete), filter weight constants (Permit=100/20, Block=0/10)
- `crates/nono-cli/src/proxy_runtime.rs` — `start_proxy_runtime()`, proxy env var collection (`HTTPS_PROXY`, credential env vars)
- `crates/nono-cli/src/execution_runtime.rs` (lines 165–270) — `proxy_env_vars` plumbed into `ExecConfig.env_vars` on Windows

### Validation

- `crates/nono-cli/src/exec_strategy_windows/network.rs` lines 1499–1504: `prepare_network_enforcement()` currently bails on `!policy.is_fully_supported()` — this is the guard that must be removed after unsupported markers are cleared
- Filter weight path in `nono-wfp-service.rs` lines 1289–1298: Permit gets weight 100 (SID-scoped) or 20 (non-SID), Block gets 0 or 10 — permits already outrank block-all; no weight changes needed

---

## What Is Already Wired (Confirmed, Do Not Rebuild)

- `proxy_env_vars` (`HTTPS_PROXY` + credential env vars) flows through `execution_runtime.rs` → `ExecConfig.env_vars` → child env on Windows. This path is complete.
- `build_policy_filter_specs()` in `nono-wfp-service.rs` already installs per-port permit WFP filters for connect, bind, and localhost port lists.
- `build_wfp_runtime_activation_request()` already passes `tcp_connect_ports` through to the service request (line 451).
- `ProxyOnly` mode already adds the proxy port to `localhost_ports` in the activation request.
- Filter weights: permits already win over block-all — no changes needed.

---

## Plan Shape (Guidance for Planner)

Three plans are expected:

**Plan 09-01** — Remove unsupported markers, enable port-level WFP enforcement
- Remove `PortConnectAllowlist`, `PortBindAllowlist`, `LocalhostPortAllowlist` from `compile_network_policy()` unsupported list
- Add unit tests for `compile_network_policy()` with port-populated caps: verify `unsupported` is empty, `tcp_connect_ports` / `tcp_bind_ports` are set correctly
- Verify `prepare_network_enforcement()` no longer bails on port-only caps

**Plan 09-02** — `--proxy-only` guard + proxy env injection verification  
- Add pre-flight check: if `--proxy-only` requested but `proxy.active` is false, return hard error before WFP setup
- Verify (test) that `HTTPS_PROXY` and `NONO_PROXY_TOKEN` appear in `ExecConfig.env_vars` when proxy is active on Windows
- Ensure `ProxyOnly` mode on Windows activates the loopback port permit filter via `localhost_ports`

**Plan 09-03** — Integration test: real TCP connection through WFP allow-listed port
- Write `#[cfg(target_os = "windows")]` integration test: bind loopback echo server → activate WFP with port allow-listed → confirm `connect()` succeeds
- Confirm that non-allow-listed port connect is blocked (negative test)
- Runs in Windows CI only

---

## Deferred Ideas

None captured during this discussion.
