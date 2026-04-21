---
task: fix-fwpmgetappid-inverted-check
status: complete
completed: 2026-04-21
source: gemini-code-assist PR #726 review (critical)
---

# Summary: Flip inverted status check on `FwpmGetAppIdFromFileName0`

## Outcome

Changed `if status == 0 {` → `if status != 0 {` in `get_app_id_blob` (`crates/nono-cli/src/bin/nono-wfp-service.rs:1202`). Added inline comment documenting the `DWORD` return convention. WFP errors from `FwpmGetAppIdFromFileName0` now surface through `format_windows_error` with the actual status code instead of falling through to the less-informative `blob.is_null()` branch.

## Audit performed

Swept all 8 `Fwpm*` status checks in the file; only 1201 was inverted. All others (`FwpmEngineOpen0`, `FwpmTransactionBegin0`, `FwpmTransactionCommit0`, `FwpmSubLayerAdd0`, `FwpmFilterAdd0`, `FwpmFilterDeleteByKey0` x2) correctly use `!= 0` or the appropriate acceptable-error variant.

## Verification

- `cargo fmt --all -- --check` → clean
- `cargo build --workspace` → exit 0

## PR thread

To resolve: thread on PR #726 (v2.1) for `nono-wfp-service.rs` inverted status check.
