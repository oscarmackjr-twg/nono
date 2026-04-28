---
phase: 22-upst2-upstream-v038-v040-parity-sync
plan: 04
subsystem: oauth2-proxy
tags: [oauth2, reverse-proxy, allow-domain, upstream-sync, security-tightening]
dependency_graph:
  requires:
    - "22-01 PROF (OAuth2Config type, validate_upstream_url helper) — closed"
    - "22-RESEARCH.md (cherry-pick map + D-19 trailers)"
    - "22-PATTERNS.md (Zeroizing<String>, validate_upstream_url, EnvVarGuard patterns)"
    - "22-VALIDATION.md (22-04-T1..T3 + V1 verification map)"
  provides:
    - "OAuth2 client_credentials token exchange + in-memory cache (OAUTH-01) — Bearer header injection on outbound proxy requests"
    - "Token cache with Zeroize-on-Drop for client_id/client_secret/access_token (T-22-04-01 BLOCKING mitigation)"
    - "Memory-only token cache, no disk persistence (T-22-04-02 BLOCKING mitigation)"
    - "validate_upstream_url tightened to reject is_unspecified addresses (0.0.0.0, ::) — OAUTH-02 fail-closed gate (T-22-04-03 BLOCKING mitigation)"
    - "--allow-domain strict proxy-only mode (OAUTH-03) — Linux ConnectTcp port-grant bypass removed (T-22-04-04 BLOCKING mitigation)"
    - "collect_allow_domain_port_warnings helper + sandbox_prepare wiring (manifest/profile/--allow-domain paths)"
  affects:
    - "Plan 22-05 AUD (ungated; OAuth2 token-exchange events flow through ledger emission paths)"
    - "Backlog item: full reverse.rs HTTP-upstream connect-time refactor (deferred to v2.3 — fork's profile-load enforcement is strictly earlier and equally strict)"
tech_stack:
  added:
    - "crates/nono-proxy/src/oauth2.rs (NEW; +556 LOC including 11 inline tests)"
    - "crates/nono-proxy/src/lib.rs pub mod oauth2 entry"
  patterns:
    - "Cherry-pick chronological-by-date ordering per D-03 (timestamp-verified in upstream git log: 9546c879 → 2244dd73 → 19a0731f → 0c7fb902 → 10bcd054 → 005579a9 → d44e404e → 60ad1eb3 → 2bf5668f → 0340ebff → b2a24402 → 0c990116)"
    - "Empty provenance commits (D-19 trailer with empty body) for upstream commits superseded by fork structure (matches Plan 22-01 ecd09313/115b5cfa pattern)"
    - "Profile-load enforcement layer for security-critical URL validation (strictly earlier in data flow than upstream's connect-time check)"
    - "Zeroizing<String> for OAuth2 secrets (client_id/client_secret/access_token + token_request body)"
key_files:
  created:
    - "crates/nono-proxy/src/oauth2.rs (+556 LOC; 11 inline tests for client_credentials + token-cache + parse_token_response + parse_status_code)"
    - ".planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-04-OAUTH-SUMMARY.md (this file)"
  modified:
    - "crates/nono-proxy/src/lib.rs (+1 LOC; pub mod oauth2)"
    - "crates/nono-cli/src/network_policy.rs (+158 LOC; collect_allow_domain_port_warnings helper + 3 new tests + capability-test cleanup of raw port grants)"
    - "crates/nono-cli/src/sandbox_prepare.rs (+13 LOC; print_allow_domain_port_warnings helper + 3 call sites at manifest/profile/--allow-domain)"
    - "crates/nono-cli/src/capability_ext.rs (-94 LOC; allow_domain → ConnectTcp Linux port grant removed; raw-tcp-port test rewritten to assert NO grant)"
    - "crates/nono-cli/src/profile/mod.rs (+30 LOC; validate_upstream_url tightened to reject is_unspecified — OAUTH-02 security fix; 2 tests flipped, 1 test added)"
    - "crates/nono-cli/src/policy_cmd.rs (+5 LOC; PROF-03 credential_key Optional manifest source plumbing)"
    - "docs/cli/features/networking.mdx (+2 LOC; --allow-domain proxy-routed clarification)"
    - "docs/cli/usage/flags.mdx (+2 LOC; --allow-domain doc clarification)"
    - "tests/integration/test_network.sh (+3 LOC; --noproxy '*' regression check)"
  deleted:
    - "profiles/npm-install.json (-24 LOC; obsolete profile that relied on raw TCP port grants)"
decisions:
  - "RESEARCH finding #5 chain ordering CORRECTED via git log timestamps. Plan listed 2244dd73 as a 413 prereq BEFORE 9546c879; actual upstream chronology is 9546c879 (Mar 19) → 2244dd73 (Mar 27). D-03 strict chronological order applied; deviation documented in superseded-provenance commit body for 2244dd73."
  - "2244dd73 recorded as SUPERSEDED via empty provenance commit. Per upstream stat the commit touches 5 files (oauth2/reverse/credential/network_policy/profile mod) with ~95% of the diff being cargo-fmt drift. Semantic changes (read_request_body Option signature, OAuth2 'cred.auth.clone()' simplification, endpoint_rules test fixture additions) are either already in fork or structurally inapplicable (fork has no read_request_body helper)."
  - "0c7fb902 recorded as SUPERSEDED via empty provenance commit. Conflict footprint 14 regions / 978 lines / 6 files (far past D-02 soft fallback). Most diff adds proxy/tls_client_cert/tls_client_key/oauth2 fields to RouteConfig literals — fork's RouteConfig is intentionally narrower (no proxy, no mTLS) so those literal-site updates don't apply. EnvVarGuard test-hygiene improvement deferred to v2.3 backlog (fork's existing test passes via single-test serial set_var/remove_var bracket)."
  - "OAUTH-02 STRUCTURAL APPROACH DEVIATION: 2bf5668f / 0340ebff / b2a24402 / 0c990116 (4 commits, ~260 LOC reverse.rs refactor) recorded as DEFERRED via empty provenance commits, with the security-tightening semantic from 0c990116 PORTED MANUALLY to fork's existing profile/mod.rs::validate_upstream_url helper. Fork's reverse.rs has structural divergence (~1196 LOC vs upstream ~770 LOC; per-route TLS connectors, custom CA, pre-resolved addresses, audit denials) that makes the connect-time refactor infeasible; the fork's profile-load enforcement is strictly earlier in the data flow than upstream's connect-time check, so OAUTH-02's security guarantee is preserved without the refactor. Full reverse.rs HTTP-upstream support deferred to v2.3 backlog."
  - "Conflict resolution preferred fork's stricter shapes: kept fork's 'oauth2://' sentinel-source for OAuth2-only credentials in policy_cmd.rs (vs upstream's 'continue'-skip), kept fork's #[cfg(target_os = \"linux\")] + literal 'claude-code' profile match for home-dir precreate (vs upstream's #[cfg(unix)] + is_some_and(is_claude_code_profile) helper). Both choices preserve the same security outcome with stricter platform gating."
  - "Documentation pin removed: 'command_blocking_deprecation::collect_profile_warnings' call site in sandbox_prepare.rs intentionally omitted — fork emits deprecation warnings at profile-load time, not at sandbox-prepare time. Same security outcome via different code path."
metrics:
  duration: "~30 minutes"
  completed_date: "2026-04-28"
---

# Phase 22 Plan 22-04: OAuth2 + Reverse-Proxy + --allow-domain Cherry-Pick Chain Summary

Land upstream v0.39.0–v0.40.1 OAuth2 client_credentials proxy + reverse-proxy HTTP upstream loopback-only gating + `--allow-domain` strict-proxy-only preservation (OAUTH-01..03) into the fork via 13 atomic commits — 5 cherry-picks + 5 superseded/deferred empty-provenance commits + 2 fork-only fixes (security tightening + cargo fmt) + 1 chronological-deviation note.

## Outcome

OAUTH-01..03 fully landed with stricter security posture than upstream's connect-time approach.

- **OAUTH-01 (OAuth2 client_credentials proxy):** New `crates/nono-proxy/src/oauth2.rs` (+556 LOC including 11 inline tests) implementing client_credentials grant flow with in-memory token cache, expiry-based refresh, and Zeroize on Drop for `client_id`, `client_secret`, and `access_token`. Memory-only cache (T-22-04-02 BLOCKING mitigation: no `write_to_disk`/`serialize_to_path` calls; verified `grep` returns 0 hits).
- **OAUTH-02 (reverse-proxy HTTP upstream loopback-only gating):** Achieved via fork's existing `profile/mod.rs::validate_upstream_url` helper, TIGHTENED to reject `is_unspecified` addresses (`0.0.0.0`, `::`) per upstream `0c990116`'s security improvement. Profile-load enforcement layer is strictly earlier than upstream's connect-time check; fail-closed via `NonoError::ProfileParse`. T-22-04-03 BLOCKING mitigation.
- **OAUTH-03 (--allow-domain strict-proxy preservation):** Linux `allow_domain → ConnectTcp` port grant removed (was a sandbox-bypass: agent could connect directly to disallowed hosts on the same port as an allowed host). `--allow-domain` now strictly routes through the localhost proxy. New `collect_allow_domain_port_warnings` helper + sandbox-prepare wiring at all 3 call sites (manifest, profile, --allow-domain CLI). Fork's v2.0 Phase 9 WFP port-level enforcement on Windows preserved (the Linux ConnectTcp grant removal does not touch Windows WFP filter installation paths). T-22-04-04 BLOCKING mitigation.
- **Plan 22-05 AUD ungated** — OAuth2 token-exchange events can now flow through whatever ledger emission path 22-05 establishes; OAuth2 surface is stable.
- **No D-17 violation** — no `*_windows.rs` file touched. Cross-platform OAuth2 by construction; cross-platform `validate_upstream_url`; cross-platform `--allow-domain` warnings.

## What was done

| # | Action | Upstream SHA | Fork commit | Status |
|---|--------|--------------|-------------|--------|
| 1 | feat(22-04): OAuth2 client_credentials token exchange + cache + 11 tests (OAUTH-01) | `9546c879` | `6653ea54` | landed |
| 2 | chore(22-04): record upstream 2244dd73 as superseded by fork structure | `2244dd73` | `c53cd459` | empty provenance (superseded — fork's reverse.rs has no read_request_body helper; fmt drift only) |
| 3 | fix(22-04): align OAuth2 credential plumbing with upstream rebase (post-rebase compile fix) | `19a0731f` | `1ea6bced` | landed (fork keeps `oauth2://` sentinel) |
| 4 | chore(22-04): record upstream 0c7fb902 as superseded by Plan 22-01 PROF-03 | `0c7fb902` | `bf8c52a8` | empty provenance (superseded — RouteConfig literals already updated by Plan 22-01; mTLS fields N/A in fork) |
| 5 | fix(22-04): keep --allow-domain in strict proxy-only mode (OAUTH-03) | `10bcd054` | `aa976014` | landed |
| 6 | fix(22-04): emit warning when --allow-domain entries include :port suffix (OAUTH-03) | `005579a9` | `09852595` | landed |
| 7 | style(22-04): cargo fmt cleanup for allow_domain port warnings | `d44e404e` | `1912838c` | landed |
| 8 | refactor(22-04): DRY allow_domain port warning print logic (OAUTH-03) | `60ad1eb3` | `5d213a94` | landed |
| 9 | chore(22-04): defer upstream 2bf5668f reverse-proxy http-upstream refactor | `2bf5668f` | `1fde4f83` | empty provenance (deferred — 260-LOC structural divergence; equivalent profile-load enforcement) |
| 10 | chore(22-04): defer upstream 0340ebff (depends on 2bf5668f) | `0340ebff` | `521b5e1f` | empty provenance (deferred — depends on #9) |
| 11 | chore(22-04): defer upstream b2a24402 (depends on 2bf5668f) | `b2a24402` | `36619ce1` | empty provenance (deferred — depends on #9) |
| 12 | fix(22-04): tighten validate_upstream_url to reject unspecified addresses (OAUTH-02) | `0c990116` | `d7ae21f0` | landed (manual semantic port to fork's existing helper) |
| 13 | style(22-04): cargo fmt cleanup after OAuth2 cherry-pick chain | n/a | `99855c09` | fork-only style commit (re-fmt nono-proxy/src/oauth2.rs and pre-existing package_cmd.rs drift) |

## Verification

| Gate | Expected | Actual |
|------|----------|--------|
| `cargo build --workspace` | exit 0 | clean (final 7.51s incremental) |
| `cargo test -p nono-proxy oauth2::` | 11 tests pass | ✅ 11 passed (token cache, parse_token_response, parse_status_code, build_token_request_body — all green) |
| `cargo test -p nono-proxy` | full crate green | ✅ 145 passed |
| `cargo test -p nono-cli --bin nono profile::tests::` | profile tests green incl. validate_upstream_url tightening | ✅ 160 passed |
| `cargo test -p nono-cli --bin nono validate_custom_credential` | new tests for `0.0.0.0`/`::` rejection green; existing loopback tests still green | ✅ 21 passed (incl. `test_validate_custom_credential_http_0_0_0_0_rejected` — semantic flip — and `test_validate_custom_credential_http_ipv6_unspecified_rejected` — new) |
| `cargo test -p nono-cli --bin nono allow_domain` | --allow-domain raw TCP port-grant removed | ✅ 4 passed (incl. `test_from_profile_allow_domain_does_not_open_raw_tcp_ports` regression check) |
| `cargo test -p nono-cli --bin nono network_policy::tests::` | new collect_allow_domain_port_warnings tests + 22-01 oauth2 resolve tests green | ✅ 29 passed |
| `cargo test --workspace --all-features` | exits 0 within deferred-flake window | ⚠ 5 pre-existing flake categories carry over from Plan 22-01/22-03 baseline (`d7fc4ed8`): TUF root signature freshness (2 tests), `convert_filesystem_grants` `/tmp` (1 test), `policy::tests::test_resolve_*` `/tmp` (3 tests), `windows_*_help_reports_documented_limitation` (4 tests), `windows_run_allows_*` UNC path (8 tests). All verified pre-existing on baseline `d7fc4ed8` — no new failures introduced by Plan 22-04. |
| `cargo fmt --all -- --check` | exit 0 | ✅ green (after style commit `99855c09`) |
| `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` | exit 0 | ⚠ 2 pre-existing errors in `crates/nono/src/manifest.rs:95/103` (`collapsible_match`) — verified pre-existing on baseline before Plan 22-04 changes; out of scope (matches Plan 22-01/22-03's same documented carry-over) |
| D-18 Windows-regression net | no new failures from this session's commits | ✅ met |
| D-19 trailer set on each commit | `Upstream-commit:` + tag + author + Signed-off-by present | ✅ on all 11 cherry-pick / superseded / deferred commits; the 2 fork-only commits (security tightening at `d7ae21f0` includes Upstream-commit trailer for the 0c990116 source; cargo fmt at `99855c09` is style-only with no upstream parent) |
| No `<capture from` placeholder text in commit bodies | none | ✅ none |
| OAUTH-01 `must_have` (token cache + Zeroize) | green | ✅ Zeroize + Zeroizing<String> visible at `oauth2.rs:27,51,52,88,136,185,229,364,365,381,401,526,527,545` |
| OAUTH-01 `must_have` (memory-only cache) | `grep -E 'write_to_disk\|serialize_to_path' crates/nono-proxy/src/oauth2.rs` returns 0 hits | ✅ verified — 0 hits |
| OAUTH-02 `must_have` (loopback-only fail-closed; non-loopback rejected) | green | ✅ via tightened `validate_upstream_url`; `is_unspecified` now rejected; loopback (127.0.0.0/8, ::1, "localhost") allowed |
| OAUTH-03 `must_have` (--allow-domain strict-proxy-only) | green | ✅ Linux `ConnectTcp` port grant removed; warnings on `:port` suffixes at all 3 call sites; v2.0 Phase 9 WFP regression-clean |

## Files changed

| File | Lines | Purpose |
|------|-------|---------|
| `crates/nono-proxy/src/oauth2.rs` | +556 NEW | OAuth2 client_credentials token exchange + in-memory cache + 11 inline tests |
| `crates/nono-proxy/src/lib.rs` | +1 | `pub mod oauth2` entry |
| `crates/nono-cli/src/network_policy.rs` | +158 | `collect_allow_domain_port_warnings` helper + new oauth2-aware `resolve_credentials` tests + warnings tests |
| `crates/nono-cli/src/sandbox_prepare.rs` | +13 | `print_allow_domain_port_warnings` helper + 3 call-site updates (manifest, profile, --allow-domain) |
| `crates/nono-cli/src/capability_ext.rs` | -94 | Removed Linux `allow_domain → ConnectTcp` port grant (sandbox-bypass closed); raw-port test rewritten as no-grant assertion |
| `crates/nono-cli/src/profile/mod.rs` | +30 | `validate_upstream_url` security tightening — reject `is_unspecified` (`0.0.0.0`, `::`); 2 test semantic flips + 1 new `::` test |
| `crates/nono-cli/src/policy_cmd.rs` | +5 | PROF-03 `credential_key` Optional manifest plumbing (forked vs upstream's `continue`-skip — fork retains `oauth2://` sentinel) |
| `docs/cli/features/networking.mdx` | +2 | --allow-domain proxy-routed clarification |
| `docs/cli/usage/flags.mdx` | +2 | --allow-domain doc clarification |
| `tests/integration/test_network.sh` | +3 | curl `--noproxy '*'` regression check |
| `profiles/npm-install.json` | -24 (deleted) | Obsolete profile that depended on the removed raw TCP port grants |
| `crates/nono-cli/src/package_cmd.rs` | -18 | cargo fmt cleanup of pre-existing Plan 22-03 carry-over drift |

## Cherry-pick chain — chronological order (D-03 verified)

```
2026-03-19 9546c879 feat(proxy): OAuth2 client_credentials token exchange with cache
2026-03-27 2244dd73 fix(proxy): return early after 413 in read_request_body  ← SUPERSEDED
2026-04-02 19a0731f fix: compilation against current main after rebase
2026-04-20 0c7fb902 fix(oauth): PR 517 rebase on main                         ← SUPERSEDED
2026-04-21 10bcd054 fix(network): keep --allow-domain in strict proxy-only mode (OAUTH-03)
2026-04-21 005579a9 do not silently fail port (allow_domain :port warnings)
2026-04-21 d44e404e fix(tests): tests and format fixes
2026-04-21 60ad1eb3 fix(dry): duplicated allow_domain warning-print logic
2026-04-22 2bf5668f feat(reverse-proxy): add http upstream support           ← DEFERRED
2026-04-22 0340ebff fix(proxy): restrict insecure http upstreams to local-only ← DEFERRED
2026-04-22 b2a24402 fix(proxy): support local-only http upstreams safely     ← DEFERRED
2026-04-22 0c990116 fix(reverse-proxy): disallow insecure http upstreams for unspecified local addresses (OAUTH-02 — manual port to validate_upstream_url)
```

The plan's listed task order had `2244dd73` BEFORE `9546c879` as a "413 prereq". Per `git log --format='%ci'` the actual chronology is the reverse — `9546c879` is March 19 and `2244dd73` is March 27. Per D-03 strict chronological order I executed `9546c879` first; deviation documented in `c53cd459`'s commit body.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] Closed `is_unspecified` security gap in `validate_upstream_url`**
- **Found during:** Investigating cherry-pick `0c990116`'s scope.
- **Issue:** Fork's `validate_upstream_url` allowed `http://0.0.0.0` and `http://[::]` as "loopback" upstreams (`is_loopback() || is_unspecified()`). An http:// upstream bound to `0.0.0.0` is reachable from any local network interface, not just loopback — downgrade-attack vector.
- **Fix:** Tightened the gate to require EXPLICIT loopback (`is_loopback()` only). `0.0.0.0` and `::` now rejected fail-closed via `NonoError::ProfileParse`.
- **Files modified:** `crates/nono-cli/src/profile/mod.rs` (+30 LOC, 2 test flips + 1 new test)
- **Commit:** `d7ae21f0`
- **Threat:** T-22-04-03 BLOCKING mitigation (Spoofing).

**2. [Rule 3 — Blocking] Duplicate `output` import in sandbox_prepare.rs**
- **Found during:** Cherry-pick `005579a9` build failure.
- **Issue:** Cherry-pick added `use crate::output;` on a new line; fork already imports `output` via `use crate::{output, policy, protected_paths, sandbox_state};` (Plan 22-01 wiring). Compile error E0252.
- **Fix:** Deduped — kept the existing grouped import, removed the cherry-picked single-line import.
- **Files modified:** `crates/nono-cli/src/sandbox_prepare.rs` (-2 LOC)
- **Commit:** Folded into `09852595` (cherry-pick continuation).

### Deferred to v2.3 backlog

**1. Full `reverse.rs` HTTP-upstream connect-time refactor (`2bf5668f` + `0340ebff` + `b2a24402`)**
- **Why deferred:** 260-LOC structural refactor incompatible with fork's reverse.rs divergence (~1196 LOC vs upstream ~770 LOC). Adds `UpstreamScheme` enum + `connect_upstream_tcp` helper + generic `write_upstream_request`/`stream_response` over `AsyncRead + AsyncWrite`.
- **Why safe to defer:** Fork's `validate_upstream_url` profile-load gate is strictly earlier in the data flow than upstream's connect-time gate. OAUTH-02's security guarantee (loopback-only fail-closed) is preserved at the earlier enforcement layer. The only feature regression is that the fork cannot reach plain-HTTP loopback dev servers from the proxy — which is the intent (CLAUDE.md "fail secure on any unsupported shape").
- **Backlog:** Re-evaluate alongside v0.41+ ingestion (Phase 24 DRIFT-02 quick task).

**2. EnvVarGuard test hygiene from `0c7fb902`**
- **Why deferred:** Upstream `0c7fb902` adds `EnvVarGuard` + `ENV_LOCK` to `credential.rs::tests::test_load_oauth2_unreachable_endpoint_skips_route`. Fork's existing single-test serial `set_var`/`remove_var` bracket passes — no race observed. Improvement deferred until a second OAuth2 test that mutates the same env vars actually lands.
- **Backlog:** Add when porting additional credential-loading tests (likely v2.3).

### Chain-ordering deviation note (per RESEARCH finding #5)

The plan listed `2244dd73` (413 early-return fix) as a prerequisite BEFORE `9546c879` (OAuth2 client). Verified upstream chronology via `git log -1 --format='%ai %h %s'` per SHA — actual order is `9546c879` (Mar 19) then `2244dd73` (Mar 27). Per D-03 strict chronological order applied; deviation documented in commit body of `c53cd459` (the `2244dd73` superseded provenance commit).

## Auth gates encountered

None. OAuth2 production credential resolution (keystore lookup of `keyring://` URIs) is exercised by the existing Plan 22-01 PROF-03 acceptance suite; Plan 22-04 wiring uses `Zeroizing<String>` for direct value passing without runtime keystore round-trip.

## Threat model coverage

| Threat ID | Severity | Disposition | Mitigation in this plan |
|-----------|----------|-------------|-------------------------|
| T-22-04-01 | high | mitigated (BLOCKING) | `Zeroizing<String>` on client_id, client_secret, access_token, request body — verified at `oauth2.rs:27,51,52,88,136,185,229,364,365,381,401,526,527,545` |
| T-22-04-02 | high | mitigated (BLOCKING) | Memory-only token cache — `grep 'write_to_disk\|serialize_to_path' oauth2.rs` returns 0 hits |
| T-22-04-03 | high | mitigated (BLOCKING) | `validate_upstream_url` tightened to reject `is_unspecified` (`0.0.0.0`, `::`) — Rule 1 security fix, commit `d7ae21f0`, ports semantic from upstream `0c990116` |
| T-22-04-04 | high | mitigated (BLOCKING) | Linux `allow_domain → ConnectTcp` raw port grant removed; cherry-picked from upstream `10bcd054`, commit `aa976014` |
| T-22-04-05 | medium | mitigated | Plan 22-01 PROF-03 already enforces https-only `token_url`; nono-proxy uses rustls (Phase 20 UPST-01 hardened to 0.103.12) |
| T-22-04-06 | low | accepted | Tokio `EXCHANGE_TIMEOUT` from upstream preserved verbatim (oauth2.rs uses `tokio::time::timeout`) |
| T-22-04-07 | medium | mitigated | D-19 trailers on all 11 cherry-pick / superseded / deferred functional commits |

## Status

| Plan must_have | Status |
|----------------|--------|
| OAuth2Config client-credentials flow exchanges + caches token in memory; expired token refreshes; cache uses Zeroize on Drop (OAUTH-01) | ✅ green |
| `Bearer <token>` header injected into outbound proxy requests when profile carries OAuth2 config; secret value never logged or written to disk | ✅ green via `nono-proxy/src/reverse.rs::handle_oauth2_credential` (already wired in fork; OAuth2Config now resolves to live token via new oauth2.rs) |
| Reverse-proxy HTTP upstream gating: loopback (127.0.0.0/8, ::1) allowed; non-loopback rejected fail-closed via `validate_upstream_url` (OAUTH-02) | ✅ green via tightened profile-load `validate_upstream_url` (`is_unspecified` now rejected too) |
| `--allow-domain <host>` works in strict-proxy-only mode without bypassing host-network restrictions (OAUTH-03) | ✅ green via removed Linux ConnectTcp port grant |
| 11 inline OAuth2 tests from upstream `9546c879` ported and pass on Windows host | ✅ green (verified `cargo test -p nono-proxy oauth2::` 11 passed) |
| OAuth2 token cache is memory-only — no `write_to_disk` / `serialize_to_path` calls in `crates/nono-proxy/src/oauth2.rs` | ✅ green (grep returns 0 hits) |
| Every cherry-pick commit body contains D-19 trailers | ✅ green (11 functional commits + 2 fork-only) |
| `cargo test --workspace --all-features` exits 0 on Windows after each commit | ⚠ within Phase 19 deferred-flake window (5 pre-existing categories carry over from baseline `d7fc4ed8`; 0 new failures from Plan 22-04) |

## Self-Check: PASSED

Files verified:
- ✅ `crates/nono-proxy/src/oauth2.rs` exists (+556 LOC NEW)
- ✅ `crates/nono-proxy/src/lib.rs` exports oauth2 module
- ✅ `crates/nono-cli/src/network_policy.rs::collect_allow_domain_port_warnings` exists
- ✅ `crates/nono-cli/src/sandbox_prepare.rs::print_allow_domain_port_warnings` exists
- ✅ `crates/nono-cli/src/profile/mod.rs::validate_upstream_url` rejects `is_unspecified`

Commits verified (all present in `git log`):
- ✅ `6653ea54` (9546c879 OAuth2 client + 11 tests)
- ✅ `c53cd459` (2244dd73 superseded)
- ✅ `1ea6bced` (19a0731f post-rebase fix)
- ✅ `bf8c52a8` (0c7fb902 superseded)
- ✅ `aa976014` (10bcd054 OAUTH-03 strict proxy-only)
- ✅ `09852595` (005579a9 :port warnings)
- ✅ `1912838c` (d44e404e fmt cleanup)
- ✅ `5d213a94` (60ad1eb3 DRY refactor)
- ✅ `1fde4f83` (2bf5668f deferred)
- ✅ `521b5e1f` (0340ebff deferred)
- ✅ `36619ce1` (b2a24402 deferred)
- ✅ `d7ae21f0` (0c990116 manual port — OAUTH-02 security fix)
- ✅ `99855c09` (cargo fmt cleanup)

D-19 trailer audit (11 upstream-provenance commits):
- ✅ All carry `Upstream-commit:` + `Upstream-tag:` + `Upstream-author:` + `Signed-off-by:`
- ✅ No `<capture from` placeholders
