---
phase: 21-windows-single-file-grants
plan: 02
subsystem: sandbox/windows

tags: [windows, filesystem, sandbox, wsfg, mandatory-label, ffi, ntfs, low-integrity, setnamedsecurityinfow, error-surface]

# Dependency graph
requires:
  - plan: 21-01
    provides: WSFG-01, WSFG-02 requirement IDs exist in .planning/REQUIREMENTS.md
provides:
  - nono::try_set_mandatory_label(path, mask) -> Result<()> (crate-root Windows-cfg-gated pub fn)
  - nono::label_mask_for_access_mode(AccessMode) -> u32 (crate-root Windows-cfg-gated pub fn)
  - nono::low_integrity_label_and_mask(path) -> Option<(u32, u32)> (crate-root Windows-cfg-gated pub fn)
  - NonoError::LabelApplyFailed { path, hresult, hint } (cross-platform variant in crates/nono/src/error.rs)
  - sandbox::windows module promoted to `pub mod windows;` in sandbox/mod.rs (enables crate-root re-exports)
affects: [plan-21-03 (policy compile-site will call try_set_mandatory_label in compile_filesystem_policy), plan-21-04 (AppliedLabelsGuard RAII lifecycle in nono-cli/exec_strategy_windows will apply+revert via these helpers), plan-21-05 (Phase 18 HUMAN-UAT re-run consumes the end-to-end primitive), bindings/c (nono-ffi map_error gained ErrSandboxInit arm for LabelApplyFailed)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SDDL-constructed mandatory-label ACE: build `S:(ML;;0x{mask:X};;;LW)` string, pass through ConvertStringSecurityDescriptorToSecurityDescriptorW to get a SECURITY_DESCRIPTOR with the SACL populated, then call SetNamedSecurityInfoW(SE_FILE_OBJECT, LABEL_SECURITY_INFORMATION, ..). Avoids hand-rolling a SYSTEM_MANDATORY_LABEL_ACE byte layout."
    - "OwnedSecurityDescriptor RAII guard (existing) wraps PSECURITY_DESCRIPTOR + LocalFree Drop — reused verbatim for both the SDDL-parse and the GetNamedSecurityInfoW readback paths."
    - "Fail-closed Windows error surface: NonoError::LabelApplyFailed { path, hresult, hint } carries Win32 raw code in `0x{:08X}` hex form + actionable hint mapped from ERROR_ACCESS_DENIED / ERROR_INVALID_FUNCTION / ERROR_NOT_SUPPORTED (NTFS/ReFS hint) and ERROR_FILE_NOT_FOUND (existing-file hint). Unknown codes fall through to a raw-hex support-triage hint."
    - "TDD gate sequence for Plan 21-02: two RED commits (test(21-02):, separate tests for error variant and mask helper), two GREEN commits (feat(21-02):). Each RED proves failure mode before GREEN closes it."
    - "Crate-root Windows-cfg-gated re-export convention extended: `#[cfg(target_os = \"windows\")] pub use sandbox::windows::{ ... };` block sits alongside the existing `pub use sandbox::{ Windows* types };` block. Uses the explicit submodule path because the three helpers live in sandbox::windows, not in the sandbox::{} facade."

key-files:
  created:
    - .planning/phases/21-windows-single-file-grants/21-02-SUMMARY.md — this file
    - .planning/phases/21-windows-single-file-grants/deferred-items.md — logs pre-existing trust::bundle TUF failures (out-of-scope)
  modified:
    - crates/nono/src/error.rs — added NonoError::LabelApplyFailed variant (cross-platform, not cfg-gated) + 2 unit tests (tests module with `#[allow(clippy::unwrap_used)]` for expect_err usage)
    - crates/nono/src/sandbox/windows.rs — extended FFI import block with SetNamedSecurityInfoW + ConvertStringSecurityDescriptorToSecurityDescriptorW + SDDL_REVISION_1 + GetSecurityDescriptorSacl + three SYSTEM_MANDATORY_LABEL_NO_*_UP mask constants; added 3 pub fns (label_mask_for_access_mode, try_set_mandatory_label, low_integrity_label_and_mask); added 3 mask-encoder unit tests
    - crates/nono/src/sandbox/mod.rs — promoted `mod windows;` to `pub mod windows;` (line 18)
    - crates/nono/src/lib.rs — added `#[cfg(target_os = "windows")] pub use sandbox::windows::{ label_mask_for_access_mode, low_integrity_label_and_mask, try_set_mandatory_label };` block immediately after the existing Windows types re-export block
    - bindings/c/src/lib.rs — added `NonoError::LabelApplyFailed { .. } => NonoErrorCode::ErrSandboxInit` arm to map_error's exhaustive match (Rule-3 auto-fix required for workspace compile)

key-decisions:
  - "SYSTEM_MANDATORY_LABEL_NO_{READ,WRITE,EXECUTE}_UP constants are imported from windows-sys 0.59 at the module path `Win32::System::SystemServices` (NOT `Win32::Security` as the plan hypothesized). Verified in the installed registry at `~/.cargo/registry/src/index.crates.io-*/windows-sys-0.59.0/src/Windows/Win32/System/SystemServices/mod.rs:2333-2335`. Chosen path: `use` import (not inlined const) — the constants are first-class exports of the feature-gated `Win32_System_SystemServices` feature which is already enabled in crates/nono/Cargo.toml."
  - "SetNamedSecurityInfoW signature in windows-sys 0.59: `fn SetNamedSecurityInfoW(pobjectname: PCWSTR, objecttype: SE_OBJECT_TYPE, securityinfo: OBJECT_SECURITY_INFORMATION, psidowner: PSID, psidgroup: PSID, pdacl: *const ACL, psacl: *const ACL) -> WIN32_ERROR`. Object name is `*const u16` (PCWSTR) — NOT `*mut u16`. psacl is `*const ACL`. Plan 21-04's clear_mandatory_label will inherit these const-pointer shapes. The `as *mut u16` cast suggested as a fallback in the plan was NOT needed and was omitted."
  - "GetSecurityDescriptorSacl lives in `Win32::Security` (NOT `Win32::Security::Authorization`). Verified in windows-sys 0.59 at `src/Windows/Win32/Security/mod.rs:95`. Out-pointers `lpbsaclpresent` / `lpbsacldefaulted` are `BOOL` (i32), matching the plan's `let mut sacl_present: i32 = 0;` / `let mut sacl_defaulted: i32 = 0;` shapes."
  - "ConvertStringSecurityDescriptorToSecurityDescriptorW's revision parameter uses the exported constant `SDDL_REVISION_1` (u32 = 1) from `Win32::Security::Authorization` — imported symbolically rather than as a magic `1` literal."
  - "SDDL encoding uses `str::encode_utf16()` rather than `OsStr::new(&sddl).encode_wide()`. The SDDL format string `S:(ML;;0x{mask:X};;;LW)` is ASCII-only, so encode_utf16 produces the same wide form with less ceremony (avoids the unused `std::ffi::OsStr` import)."
  - "NonoError::LabelApplyFailed is NOT cfg-gated to Windows (unlike the Linux-only Landlock variant). Rationale: the variant propagates through cross-platform Sandbox::apply() code paths via the shared Result<T> alias, and match sites in downstream code (incl. nono-ffi's map_error, which MUST be exhaustive per its doc comment) compile on all platforms. The Landlock variant is cfg-gated only because its error type (landlock::RulesetError) is itself cfg-gated."
  - "nono-ffi's map_error match in bindings/c/src/lib.rs is documented as exhaustive: 'Every NonoError variant is matched explicitly so the compiler will flag new variants that need a mapping, instead of silently falling through to ErrUnknown.' Added arm `NonoError::LabelApplyFailed { .. } => NonoErrorCode::ErrSandboxInit` — semantically the closest existing code, matching the Sandbox::apply() call path that would surface this error in practice."
  - "Single atomic GREEN commit for Task 2 (7637694): ships the windows.rs primitives, the mod.rs `pub mod` promotion, the lib.rs re-export block, and the nono-ffi match arm together. Splitting these would break intermediate commits (the lib.rs re-export requires `pub mod windows;`; the workspace build requires the ffi arm). TDD structure is preserved: the RED test commit (853683a) sits separately, and the GREEN commit makes it pass."
  - "Out-of-scope failures in `trust::bundle::tests::{load_production_trusted_root_succeeds, verify_bundle_with_invalid_digest}` (TUF signature threshold errors) are PRE-EXISTING on the `windows-squash` branch — verified by checking out the pre-plan `windows.rs` and reproducing the same assertion failure. Logged to `deferred-items.md`; not fixed in Plan 21-02."

requirements-completed: [WSFG-01-primitive, WSFG-02-error-surface]
# Note: WSFG-01 has three parts (primitive, compile-site integration, RAII lifecycle). Plan 21-02 closes the primitive only;
# Plans 21-03 and 21-04 close the remaining parts. WSFG-02 (error surface) is fully closed here.

# Metrics
duration: 11min
completed: 2026-04-20
started: 2026-04-20T19:00:06Z
finished: 2026-04-20T19:11:27Z
---

# Phase 21 Plan 21-02: Windows Mandatory-Label Enforcement Primitive Summary

**Landed the production FFI primitive `nono::try_set_mandatory_label` + mode-encoder `nono::label_mask_for_access_mode` + mask-returning reader `nono::low_integrity_label_and_mask` at the crate root, all three declared `pub` and re-exported from `crates/nono/src/lib.rs` under `#[cfg(target_os = "windows")]`. Introduces `NonoError::LabelApplyFailed { path, hresult, hint }` for fail-closed error reporting. No compile-site integration (Plan 21-03) or lifecycle guard (Plan 21-04) yet.**

## Performance

- **Duration:** ~11 min
- **Started:** 2026-04-20T19:00:06Z
- **Completed:** 2026-04-20T19:11:27Z
- **Tasks:** 2 (Task 1: error variant; Task 2: 6-edit primitive + re-export)
- **Commits:** 4 DCO-signed (2 RED, 2 GREEN)
- **Files modified:** 5 (error.rs, sandbox/windows.rs, sandbox/mod.rs, lib.rs, bindings/c/src/lib.rs)
- **Files created:** 2 (21-02-SUMMARY.md, deferred-items.md)

## Accomplishments

### Task 1 — NonoError::LabelApplyFailed variant

Appended a new cross-platform variant to `NonoError` in `crates/nono/src/error.rs`:

```rust
#[error("Failed to apply integrity label to {path}: {hint} (HRESULT: 0x{hresult:08X})")]
LabelApplyFailed {
    path: PathBuf,
    hresult: u32,
    hint: String,
},
```

Display format carries all three fields — verified by two unit tests in a new `#[cfg(test)] mod tests` block at the bottom of `error.rs`. The `#[allow(clippy::unwrap_used)]` on the tests module enables `expect_err("must error")` in the propagation test.

**TDD gates:**
- RED commit `1a545e1` — tests reference `NonoError::LabelApplyFailed` which doesn't exist yet. `cargo test -p nono --lib error::tests` fails with `error[E0599]: no variant named 'LabelApplyFailed' found`.
- GREEN commit `d19aaaa` — adds the variant; both tests pass.

### Task 2 — Production label-apply primitive + crate-root publish

Six coordinated edits across three files (`windows.rs`, `mod.rs`, `lib.rs`) plus a Rule-3 auto-fix in `bindings/c/src/lib.rs`, shipped as one atomic GREEN commit.

**windows.rs — FFI imports extended:**

```rust
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, GetNamedSecurityInfoW,
    SetNamedSecurityInfoW, SDDL_REVISION_1, SE_FILE_OBJECT,
};
use windows_sys::Win32::Security::{
    GetAce, GetSecurityDescriptorSacl, GetSidSubAuthority, GetSidSubAuthorityCount,
    ACE_HEADER, ACL, LABEL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    SYSTEM_MANDATORY_LABEL_ACE,
};
use windows_sys::Win32::System::SystemServices::{
    SECURITY_MANDATORY_LOW_RID, SYSTEM_MANDATORY_LABEL_ACE_TYPE,
    SYSTEM_MANDATORY_LABEL_NO_EXECUTE_UP, SYSTEM_MANDATORY_LABEL_NO_READ_UP,
    SYSTEM_MANDATORY_LABEL_NO_WRITE_UP,
};
```

**windows.rs — three new `pub fn` helpers:**

1. `label_mask_for_access_mode(mode: crate::AccessMode) -> u32` — pure mapping per CONTEXT.md D-01 (Read → NO_WRITE_UP | NO_EXECUTE_UP; Write → NO_READ_UP | NO_EXECUTE_UP; ReadWrite → NO_EXECUTE_UP).
2. `try_set_mandatory_label(path: &Path, mask: u32) -> Result<()>` — SDDL-constructed SACL via `ConvertStringSecurityDescriptorToSecurityDescriptorW`, then `SetNamedSecurityInfoW(SE_FILE_OBJECT, LABEL_SECURITY_INFORMATION, ..)`. Fail-closed with `NonoError::LabelApplyFailed` carrying Win32 code + actionable hint.
3. `low_integrity_label_and_mask(path: &Path) -> Option<(u32, u32)>` — reads back mandatory-label ACE, returns (rid, mask). Mirrors the existing `low_integrity_label_rid` function but extends it to also return the Mask field for integration-test assertions.

Every unsafe block carries a `// SAFETY:` comment citing preconditions (nul-terminated UTF-16 buffer, valid out-pointer, AceType already checked, RAII guard owns SD lifetime, etc.).

**sandbox/mod.rs — module promoted to `pub`:**

```diff
 #[cfg(target_os = "windows")]
-mod windows;
+pub mod windows;
```

One-line change. Required for the lib.rs re-export to compile (else rustc E0603 "module 'windows' is private").

**lib.rs — new Windows-cfg-gated re-export block:**

```rust
#[cfg(target_os = "windows")]
pub use sandbox::windows::{
    label_mask_for_access_mode, low_integrity_label_and_mask, try_set_mandatory_label,
};
```

Downstream `nono-cli` code (Plan 21-04) can now write `use nono::{try_set_mandatory_label, label_mask_for_access_mode, low_integrity_label_and_mask};` — no `pub(crate)` barrier, no private-module barrier.

**bindings/c/src/lib.rs — Rule-3 auto-fix (non-exhaustive match closure):**

The nono-ffi `map_error` match is documented exhaustive — a new NonoError variant breaks the workspace build. Added:

```rust
nono::NonoError::LabelApplyFailed { .. } => NonoErrorCode::ErrSandboxInit,
```

Semantically ErrSandboxInit is the closest existing code — the error originates inside `Sandbox::apply()`-like enforcement paths.

**TDD gates:**
- RED commit `853683a` — three tests reference `label_mask_for_access_mode` and the three mask constants which aren't in scope yet. `cargo test -p nono --lib sandbox::windows::tests::label_mask_for_access_mode` fails with `error[E0425]: cannot find function 'label_mask_for_access_mode'` + 3 "cannot find value" errors for the mask constants.
- GREEN commit `7637694` — adds all imports + the three helpers + the pub-mod promotion + the lib.rs re-export + the ffi match arm; all 3 mask tests pass, workspace compiles clean.

## Task Commits

Each task was committed atomically with DCO sign-off on branch `windows-squash`:

1. **Task 1 RED** — `1a545e1` `test(21-02): add failing tests for NonoError::LabelApplyFailed variant`
2. **Task 1 GREEN** — `d19aaaa` `feat(21-02): add NonoError::LabelApplyFailed variant for Windows label failures`
3. **Task 2 RED** — `853683a` `test(21-02): add failing tests for label_mask_for_access_mode`
4. **Task 2 GREEN** — `7637694` `feat(21-02): add Windows mandatory-label primitives + publish at crate root`

All four commits include `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` trailer per repo DCO requirements.

## Files Created/Modified

### Modified

- `crates/nono/src/error.rs` (+49 lines) — added `LabelApplyFailed` variant + `#[cfg(test)] mod tests` block with two unit tests; variant is cross-platform (no `#[cfg(target_os = ...)]` gate)
- `crates/nono/src/sandbox/windows.rs` (+236 lines net, roughly) — added 8 new imports to the FFI import block; added 3 `pub fn` helpers (label_mask_for_access_mode, try_set_mandatory_label, low_integrity_label_and_mask); added 3 new unit tests
- `crates/nono/src/sandbox/mod.rs` (+1 / -1) — `mod windows;` → `pub mod windows;` (line 18)
- `crates/nono/src/lib.rs` (+4) — new `#[cfg(target_os = "windows")] pub use sandbox::windows::{ ... };` block (3 helpers)
- `bindings/c/src/lib.rs` (+1) — one new match arm in `map_error` for `NonoError::LabelApplyFailed`

### Created

- `.planning/phases/21-windows-single-file-grants/21-02-SUMMARY.md` — this file
- `.planning/phases/21-windows-single-file-grants/deferred-items.md` — logs pre-existing `trust::bundle::tests` TUF failures as out-of-scope for Plan 21-02

### Unchanged (verified per acceptance criteria)

- `crates/nono/src/sandbox/windows.rs::try_set_low_integrity_label` at line 1356+ (the test-only `icacls` shell-out helper — stays as-is)
- `crates/nono/src/sandbox/windows.rs::compile_filesystem_policy` (Plan 21-03 owns this — still pushes `SingleFileGrant` / `WriteOnlyDirectoryGrant` to `unsupported` today)
- `crates/nono/src/sandbox/windows.rs::low_integrity_label_rid` (existing reader — unchanged; `low_integrity_label_and_mask` is a sibling, not a replacement)
- `crates/nono/src/capability.rs` (zero diff since plan start — D-21 Windows-invariance held)
- `crates/nono/src/sandbox/linux.rs`, `crates/nono/src/sandbox/macos.rs` (zero diff since plan start — D-21 Windows-invariance held)

## Decisions Made

- **windows-sys 0.59 module paths recorded (plan deliverable #1, #2, #3):**
  - **Mask constants:** `SYSTEM_MANDATORY_LABEL_NO_{WRITE,READ,EXECUTE}_UP` live under `windows_sys::Win32::System::SystemServices` (verified in registry at `windows-sys-0.59.0/src/Windows/Win32/System/SystemServices/mod.rs:2333-2335`). Values: `NO_WRITE_UP=1`, `NO_READ_UP=2`, `NO_EXECUTE_UP=4`. **Chosen path: `use` import, not inlined const** — the feature flag `Win32_System_SystemServices` is already enabled in `crates/nono/Cargo.toml` (line 52). Plan 21-04's `clear_mandatory_label` helper should inherit this import path.
  - **SetNamedSecurityInfoW signature:** `fn SetNamedSecurityInfoW(pobjectname: PCWSTR, objecttype: SE_OBJECT_TYPE, securityinfo: OBJECT_SECURITY_INFORMATION, psidowner: PSID, psidgroup: PSID, pdacl: *const ACL, psacl: *const ACL) -> WIN32_ERROR`. Object name is `*const u16` (PCWSTR), NOT `*mut u16`. SACL is `*const ACL`, NOT `*mut ACL`. The speculative `as *mut u16` cast mentioned in the plan was NOT needed; Plan 21-04 should NOT add one.
  - **GetSecurityDescriptorSacl location:** `windows_sys::Win32::Security::GetSecurityDescriptorSacl` (NOT `Win32::Security::Authorization`). Verified at `windows-sys-0.59.0/src/Windows/Win32/Security/mod.rs:95`. BOOL out-pointers are `i32`. Plan 21-04's revert path may skip this function entirely (it can clear the SACL by constructing a new SD without the ML ACE and passing `psacl = null`), but if it does call it, the import path is now locked.
  - **SDDL_REVISION_1 location:** `windows_sys::Win32::Security::Authorization::SDDL_REVISION_1` (u32 = 1). Imported symbolically rather than as a magic `1` literal in the call.

- **sandbox::windows `mod` → `pub mod` promotion (plan deliverable #4):** Confirmed the pre-plan baseline was the private form (`mod windows;` at line 18 of `sandbox/mod.rs`). Promotion executed in Edit 6a at commit `7637694`. No rustc errors surfaced during the promotion itself. The re-export in `lib.rs` (Edit 6b, same commit) resolves cleanly under `#[cfg(target_os = "windows")]` without E0603.

- **Unexpected HRESULTs surfaced in the hint-mapping branch (plan deliverable #5):** None encountered during unit-test execution (the mask-encoder tests are pure Rust, no FFI call). The hint-mapping logic was written per spec (ERROR_ACCESS_DENIED / ERROR_INVALID_FUNCTION / ERROR_NOT_SUPPORTED → NTFS/ReFS hint; ERROR_FILE_NOT_FOUND → existing-file hint; fallthrough → raw hex). The live-FFI path isn't exercised yet — Plan 21-05's HUMAN-UAT re-run will be the first real HRESULT-surface test. If an unexpected code appears there, fold back into Plan 21-02's hint mapping then.

- **Single atomic commit for Task 2 GREEN:** The plan required Edit 6a (mod.rs promotion) + Edit 6b (lib.rs re-export) to ship together because lib.rs's `pub use sandbox::windows::{...}` would fail with E0603 without the `pub mod` promotion. I extended this to also include Edits 1-5 (FFI imports, three helpers, tests) in the same commit — splitting them would leave an intermediate commit that fails `cargo check -p nono` (tests reference symbols defined in the next commit). The TDD structure is preserved because the RED commit (`853683a`) sits ahead of the atomic GREEN.

- **Rule-3 auto-fix in bindings/c/src/lib.rs:** Adding `NonoError::LabelApplyFailed` breaks the documented-exhaustive `map_error` match in nono-ffi — the workspace refuses to compile without an arm for the new variant. This is a Rule-3 blocking-issue auto-fix (plan actions can't complete without it), not an expansion of scope. Chose `ErrSandboxInit` as the mapped error code because label-apply failures originate inside `Sandbox::apply()`-like paths — the closest existing semantic bucket.

- **Out-of-scope `trust::bundle::tests` failures left alone:** `cargo test -p nono --lib` surfaces two pre-existing TUF-signature-threshold failures. Verified they reproduce on `1a545e1^` (pre-plan baseline) — NOT introduced by Plan 21-02. Logged to `deferred-items.md` per the executor's scope-boundary rule.

## Deviations from Plan

**None substantive.** Minor adjustments from plan text:

1. **Rule-3 auto-fix (nono-ffi match closure):** The plan's `<action>` blocks did not mention bindings/c/src/lib.rs, but the nono-ffi `map_error` is documented exhaustive and breaks the workspace build without an arm for the new variant. Added one line mapping `LabelApplyFailed` → `ErrSandboxInit`. This is a standard Rule-3 blocking-fix, not a plan deviation proper. Tracked here for auditability.

2. **Plan's speculative `as *mut u16` cast omitted:** The plan said "if the signature is `*const u16`, drop the `as *mut u16` cast." windows-sys 0.59's signature is in fact `*const u16` (PCWSTR), so the cast was omitted. The plan's conditional was followed exactly.

3. **Plan's speculative local `const` fallback for mask constants omitted:** The plan said "if windows-sys 0.59 does not export these three constants from Win32::Security, fall back to ... local `const`." The constants ARE exported (under `Win32::System::SystemServices`, not `Win32::Security`), so the `use` import path was used and no local `const` fallback was written. The plan's conditional was followed exactly; the constants table in CONTEXT.md D-01 is preserved verbatim through the exported constants.

4. **SDDL wide-string encoding:** The plan's skeleton used `OsStr::new(&sddl).encode_wide()`. I used `sddl.encode_utf16()` instead — the SDDL format string is ASCII-only, so both produce identical output, but `encode_utf16` is simpler and avoids an unused `std::ffi::OsStr` import.

5. **Formatter reflow on error.rs tests:** `cargo fmt --all` reformatted the `assert!(msg.contains(...), "...");` calls to multi-line shape per repo defaults. Content unchanged. The rewritten shape is what shipped in commit `d19aaaa`.

## Issues Encountered

1. **Pre-existing trust::bundle TUF failures** (out of scope): See `deferred-items.md`. Not fixed in Plan 21-02 per scope-boundary rule.

2. **cargo test positional-arg limitation:** `cargo test` accepts only ONE positional `TESTNAME` argument (it's a substring filter). My initial invocation `cargo test -p nono --lib a::test_1 a::test_2` failed with `unexpected argument ... found`. Worked around by using a common substring prefix (e.g., `label_mask_for_access_mode` matches all 3 mask tests). Noted for future executions; not a blocker.

3. **PreToolUse READ-BEFORE-EDIT hook reminders (7 occurrences):** Fired on every Edit call despite every target file having been Read via the Read tool at session start. All edits completed successfully per the tool output. Not a blocker; same hook-tuning follow-up note from Plan 21-01.

## User Setup Required

None. The helpers are published symbols; downstream plans (21-03 and 21-04) consume them directly via the crate-root `nono::` path. No environment variables, no external services, no admin rights, no keystore entries. The production enforcement path itself requires no Windows admin rights either — labeling a file one owns is a user-mode operation.

## Verification Commands Re-Run Post-Commit

```bash
$ grep -c "pub fn try_set_mandatory_label" crates/nono/src/sandbox/windows.rs
1

$ grep -c "pub fn label_mask_for_access_mode" crates/nono/src/sandbox/windows.rs
1

$ grep -c "pub fn low_integrity_label_and_mask" crates/nono/src/sandbox/windows.rs
1

$ grep -c "^pub mod windows;" crates/nono/src/sandbox/mod.rs
1

$ grep -c "^mod windows;" crates/nono/src/sandbox/mod.rs
0

$ grep -c "pub use sandbox::windows::" crates/nono/src/lib.rs
1

$ grep -c "SetNamedSecurityInfoW" crates/nono/src/sandbox/windows.rs
7   # import + call site + 5 documentation comment mentions

$ grep -c "SYSTEM_MANDATORY_LABEL_NO_WRITE_UP" crates/nono/src/sandbox/windows.rs
3   # import + Read arm of label_mask_for_access_mode + one test

$ grep -c "SYSTEM_MANDATORY_LABEL_NO_READ_UP" crates/nono/src/sandbox/windows.rs
3   # import + Write arm + one test

$ grep -c "SYSTEM_MANDATORY_LABEL_NO_EXECUTE_UP" crates/nono/src/sandbox/windows.rs
7   # import + all 3 mode arms + 3 tests

$ grep -c "NonoError::LabelApplyFailed" crates/nono/src/sandbox/windows.rs
5   # three error sites in try_set_mandatory_label + sites in test imports

$ grep -c "// SAFETY:" crates/nono/src/sandbox/windows.rs
24  # existing 14 + ~10 new in try_set_mandatory_label + low_integrity_label_and_mask

$ grep -c "LabelApplyFailed" crates/nono/src/error.rs
4   # variant definition + 2 test construction sites + matches! test site

$ grep -c "HRESULT: 0x{hresult:08X}" crates/nono/src/error.rs
1   # Display format attribute

$ cargo test -p nono --lib sandbox::windows::tests::label_mask_for_access_mode 2>&1 | tail -6
running 3 tests
test sandbox::windows::tests::label_mask_for_access_mode_read_denies_write_and_execute_up ... ok
test sandbox::windows::tests::label_mask_for_access_mode_read_write_denies_only_execute_up ... ok
test sandbox::windows::tests::label_mask_for_access_mode_write_denies_read_and_execute_up ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 634 filtered out; finished in 0.00s

$ cargo test -p nono --lib error::tests::label_apply_failed 2>&1 | tail -5
running 2 tests
test error::tests::label_apply_failed_is_propagatable_via_result_alias ... ok
test error::tests::label_apply_failed_display_includes_path_hresult_and_hint ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 635 filtered out; finished in 0.00s

$ cargo build --workspace 2>&1 | tail -1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 39.53s

$ cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used 2>&1 | tail -1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.36s

$ cargo fmt --all -- --check && echo "FMT OK"
FMT OK

$ git diff 1a545e1^ -- crates/nono/src/capability.rs crates/nono/src/sandbox/linux.rs crates/nono/src/sandbox/macos.rs | wc -l
0   # D-21 Windows-invariance held: zero diff on cross-platform files since plan started

$ git log --oneline -4
7637694 feat(21-02): add Windows mandatory-label primitives + publish at crate root
853683a test(21-02): add failing tests for label_mask_for_access_mode
d19aaaa feat(21-02): add NonoError::LabelApplyFailed variant for Windows label failures
1a545e1 test(21-02): add failing tests for NonoError::LabelApplyFailed variant
```

## TDD Gate Compliance

Plan 21-02 is not itself a `type: tdd` plan, but each of its two tasks carried `tdd="true"`. Both gate sequences fully observed:

**Task 1 gate sequence:**
- RED: `1a545e1` (`test(21-02): add failing tests for NonoError::LabelApplyFailed variant`) — tests fail to compile (E0599 variant not found)
- GREEN: `d19aaaa` (`feat(21-02): add NonoError::LabelApplyFailed variant for Windows label failures`) — tests pass

**Task 2 gate sequence:**
- RED: `853683a` (`test(21-02): add failing tests for label_mask_for_access_mode`) — tests fail to compile (E0425 function not found + 3 value-not-found errors for mask constants)
- GREEN: `7637694` (`feat(21-02): add Windows mandatory-label primitives + publish at crate root`) — tests pass

Both RED commits ship the expected compile-failure signal; both GREEN commits are verified with `cargo test -p nono --lib` on Windows.

No REFACTOR phase was needed — the initial GREEN implementations matched the plan's specification without follow-up cleanup.

## Self-Check: PASSED

- [x] `NonoError::LabelApplyFailed { path: PathBuf, hresult: u32, hint: String }` exists in `crates/nono/src/error.rs` with `#[error(..)]` Display format including all three fields — verified via `grep -c "HRESULT: 0x{hresult:08X}" = 1`
- [x] Two unit tests in `error.rs` pass (Display format, propagation via Result<T>)
- [x] `label_mask_for_access_mode(AccessMode) -> u32` exists as `pub fn` in `sandbox/windows.rs` — verified via `grep -c "pub fn label_mask_for_access_mode" = 1`
- [x] Three unit tests in `sandbox::windows::tests` pass (one per AccessMode variant)
- [x] `try_set_mandatory_label(path, mask) -> Result<()>` exists as `pub fn`, wraps SetNamedSecurityInfoW, returns NonoError::LabelApplyFailed on non-zero with hint mapping — verified via `grep -c "pub fn try_set_mandatory_label" = 1` and `grep -c "NonoError::LabelApplyFailed" = 5`
- [x] `low_integrity_label_and_mask(path) -> Option<(u32, u32)>` exists as `pub fn` — verified via `grep -c "pub fn low_integrity_label_and_mask" = 1`
- [x] `crates/nono/src/sandbox/mod.rs` declares `pub mod windows;` — verified via `grep -c "^pub mod windows;" = 1` and `grep -c "^mod windows;" = 0`
- [x] `crates/nono/src/lib.rs` re-exports all three helpers — verified via `grep -c "pub use sandbox::windows::" = 1` and the block contains all three function names
- [x] Existing `compile_filesystem_policy`, `low_integrity_label_rid`, `try_set_low_integrity_label` helpers unchanged — verified by inspection (the test-only `icacls` shell-out still grep-matches at the expected line; no diff in `compile_filesystem_policy`'s unsupported-push sites)
- [x] Every unsafe block in new code carries `// SAFETY:` — verified via `grep -c "// SAFETY:" = 24` (up from 14 pre-plan)
- [x] No `.unwrap()` or `.expect()` added to production code — only `expect_err` in test modules (permitted under `#[allow(clippy::unwrap_used)]` at test-module scope)
- [x] clippy with `-D warnings -D clippy::unwrap_used` green on workspace — `cargo clippy --workspace --all-targets` exits 0
- [x] `cargo fmt --all -- --check` green
- [x] `cargo build --workspace` green on Windows
- [x] D-21 Windows-invariance held — zero diff vs pre-plan baseline on `capability.rs`, `linux.rs`, `macos.rs`
- [x] Four DCO-signed commits on `windows-squash` (`1a545e1`, `d19aaaa`, `853683a`, `7637694`) — verified via `git log --show-signature` / commit message trailers
- [x] SUMMARY.md created at `.planning/phases/21-windows-single-file-grants/21-02-SUMMARY.md` with all five plan deliverables recorded (constant-import path, SetNamedSecurityInfoW signature, GetSecurityDescriptorSacl module path, pub-mod promotion confirmation, unexpected-HRESULT note)

## Next Phase Readiness

- **Plan 21-03** (policy compile-site integration) can now import `nono::try_set_mandatory_label` + `nono::label_mask_for_access_mode` from the crate root and wire them into `compile_filesystem_policy` — the two `unsupported.push(...)` sites at `crates/nono/src/sandbox/windows.rs:568` and `:573` become `rules.push(...)` sites instead. The primitive is ready.

- **Plan 21-04** (RAII `AppliedLabelsGuard` in `nono-cli/exec_strategy_windows/`) can import the same three helpers plus the reader `nono::low_integrity_label_and_mask` for per-session revert accounting. The reader returns `(rid, mask)` so the guard can record exactly what was applied and reverse it via a SACL clear (or a re-apply of the pre-session mask if one existed).

- **Plan 21-05** (Phase 18 HUMAN-UAT re-run) depends on 21-03 + 21-04 landing first. Plan 21-02 is the primitive layer; end-to-end UAT validates the full stack.

- **windows-sys 0.59 module paths are locked for downstream plans:** Plan 21-04's `clear_mandatory_label` helper should inherit exactly the import paths recorded in the Decisions section above — no re-research needed. Plan 21-03 does not need any of these imports directly (it only calls the published crate-root functions), but benefits from the `bindings/c` ffi arm being in place (no cascading workspace-build breakage when 21-03's changes ship).

---
*Phase: 21-windows-single-file-grants*
*Plan: 21-02*
*Completed: 2026-04-20*
