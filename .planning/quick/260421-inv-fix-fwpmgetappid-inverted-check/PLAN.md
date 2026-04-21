---
task: fix-fwpmgetappid-inverted-check
type: bug-fix
severity: critical (success-path errors, failure-path may flow invalid data downstream)
source: gemini-code-assist PR #726 review (v2.1 PR, nono-wfp-service.rs:1201)
created: 2026-04-21
---

# Quick Task: Flip inverted status check on `FwpmGetAppIdFromFileName0`

## Problem

gemini-code-assist on PR #726 (v2.1):

> The error check for `FwpmGetAppIdFromFileName0` is inverted. This function returns `ERROR_SUCCESS` (0) on success and a non-zero error code on failure. The current code will return an error when the function succeeds and proceed with a null or invalid blob when it fails.
>
> Suggested change: `if status != 0 {`

Current code at `crates/nono-cli/src/bin/nono-wfp-service.rs:1201`:

```rust
let status = unsafe { FwpmGetAppIdFromFileName0(path_wide.as_ptr(), &mut blob) };
if status == 0 {                              // ← WRONG: errors on success
    return Err(format_windows_error(status, ...));
}
if blob.is_null() { return Err(...); }       // ← catches most failures incidentally
Ok(WfpAppIdBlob(blob))
```

`FwpmGetAppIdFromFileName0` follows the standard Win32 `DWORD` return convention: `ERROR_SUCCESS = 0` on success, non-zero on failure. The current code inverts this.

In practice the `blob.is_null()` branch below catches most failures (since WFP nulls the out-param on failure), but:
1. The error message from the `status == 0` branch is unreachable, so genuine WFP failures surface as the less-informative "null app id blob" error instead of `format_windows_error(status, ...)`.
2. The success path errors out — meaning `get_app_id_blob` would NEVER return `Ok` in a well-formed call. This contradicts the fact that the WFP service has tested successfully in UAT, which strongly suggests the function isn't exercised on the happy path in the current test matrix, OR the calls return non-zero for other reasons and the null-check silently skips to the wrong branch.
3. Any edge where WFP returns non-zero but leaves `blob` non-null (unusual but not guaranteed to be impossible per Microsoft docs) would flow through to `Ok(WfpAppIdBlob(blob))` with an invalid blob.

## Audit of related call sites

Ran a sweep of all other `Fwpm*` status checks in this file:

| Line | Function | Check | Verdict |
|---|---|---|---|
| 334 | `FwpmFilterDeleteByKey0` | `== 0 \|\| == FWP_E_FILTER_NOT_FOUND` | ✅ correct |
| 857 | `FwpmTransactionBegin0` | `!= 0` | ✅ correct |
| 872 | `FwpmTransactionCommit0` | `!= 0` | ✅ correct |
| 1111 | `FwpmEngineOpen0` | `!= 0` | ✅ correct |
| 1144 | `FwpmSubLayerAdd0` | `!= 0 && != FWP_E_ALREADY_EXISTS` | ✅ correct |
| **1201** | **`FwpmGetAppIdFromFileName0`** | **`== 0`** | **❌ inverted (this fix)** |
| 1323 | `FwpmFilterAdd0` | `!= 0` | ✅ correct |
| 1339 | `FwpmFilterDeleteByKey0` | `!= 0` | ✅ correct |

Only line 1201 is inverted. All other WFP calls follow the convention correctly.

## Fix

Single-character change: `if status == 0 {` → `if status != 0 {`.

## Verification

- `cargo fmt --all -- --check` / `cargo build --workspace` / `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` all green.
- No tests cover `get_app_id_blob` (grep confirms). Behavior change is a single-boolean flip with a clear invariant; relying on live WFP smoke (the 5-row Phase 15 gate and Phase 09's `wfp_port_permit_allows_real_tcp_connection` integration test) for regression coverage.

## Propagation

Standard flow — windows-squash → v2.0-pr (cherry-pick + amend + force-push) → v2.1-pr (rebase + force-push) → reply + resolve PR #726 thread.
