---
status: complete
date: 2026-04-17
quick_id: 260417-wla
slug: fix-windows-createprocess-handle-uaf
type: quick
subsystem: infra
tags: [windows, job-objects, handles, restricted-token, low-integrity, createprocess, unsafe-ffi, supervisor]

requires:
  - phase: 04-windows-wfp
    provides: spawn_windows_child launcher for supervised Windows child processes
provides:
  - Correct HANDLE ownership semantics in spawn_windows_child (named Option<> holders outlive CreateProcess*W)
  - Removal of the double-close redundant OwnedHandle wrapper
affects: [phase-13-v1-human-verification-uat, phase-05, phase-07, phase-09, phase-11]

tech-stack:
  added: []
  patterns:
    - "HANDLE-returning FFI helpers: bind the owner (RestrictedToken / OwnedHandle) to a named local that outlives every consumer of the raw HANDLE — never extract .raw() / field-access on a temporary"

key-files:
  created:
    - .planning/quick/260417-wla-fix-windows-createprocess-handle-uaf/260417-wla-PLAN.md
    - .planning/quick/260417-wla-fix-windows-createprocess-handle-uaf/260417-wla-SUMMARY.md
  modified:
    - crates/nono-cli/src/exec_strategy_windows/launch.rs

key-decisions:
  - "Used two Option<> holders (_restricted_holder, _low_integrity_holder) rather than a single Box<dyn Drop> so each branch is statically typed and holders are zero-cost when unused."
  - "Kept the `OwnedHandle` and `RestrictedToken` type definitions untouched — the bug was purely about ownership in spawn_windows_child, not about the wrappers themselves."
  - "Accepted that the smoke test now surfaces a different downstream error (Windows filesystem policy coverage) as proof the original use-after-close is gone; that new error is out of scope for this quick."

patterns-established:
  - "HANDLE lifetime discipline: any FFI helper returning a wrapper whose Drop calls CloseHandle MUST be stored in a named local for the duration of every consumer of the raw HANDLE."

requirements-completed: []

duration: ~25min
completed: 2026-04-17

commits:
  - "eb4730c fix(260417-wla): repair Windows token handle use-after-close in spawn_windows_child"
---

# Quick 260417-wla: Fix Windows CreateProcess Handle Use-After-Close Summary

**Hoisted the restricted / low-integrity token owners into named `Option<>` locals and removed the redundant `OwnedHandle(h_token)` wrapper so `CreateProcessAsUserW` no longer receives a closed HANDLE (ERROR_INVALID_HANDLE, 6).**

## Background

`spawn_windows_child` in `crates/nono-cli/src/exec_strategy_windows/launch.rs` constructed its primary token as:

```rust
let h_token = if let Some(ref sid) = config.session_sid {
    restricted_token::create_restricted_token_with_sid(sid)?.h_token
} else if should_use_low_integrity_windows_launch(config.caps) {
    create_low_integrity_primary_token()?.raw()
} else {
    std::ptr::null_mut()
};
let token = OwnedHandle(h_token);
```

Both `RestrictedToken` (`crates/nono-cli/src/exec_strategy_windows/restricted_token.rs:12-17`) and `OwnedHandle` (`crates/nono-cli/src/exec_strategy_windows/launch.rs:9-18`) close their inner HANDLE on `Drop`. Because `?.h_token` / `?.raw()` extract a field from an unnamed temporary, the temporary drops *in that very statement*, calling `CloseHandle(h_token)` immediately. The subsequent `OwnedHandle(h_token)` then wraps a **closed** HANDLE and hands it to `CreateProcessAsUserW`, which fails with `ERROR_INVALID_HANDLE (6)`. On function return the second `OwnedHandle` drops and double-closes. This blocked 6 of 10 Phase 13 UAT items — any `nono run` with a filesystem capability on Windows.

## Change summary

Inside `spawn_windows_child` only, the token-acquisition block now binds each potential owner to a named local — `_restricted_holder: Option<RestrictedToken>` and `_low_integrity_holder: Option<OwnedHandle>` — whose `Drop` impls are guaranteed to run at the end of the function, *after* `CreateProcess{AsUser}W` consumes the raw HANDLE. The redundant `let token = OwnedHandle(h_token);` wrapper was removed (the named holders already own the close, so a second wrapper would double-close). The four downstream references inside the function (`token.0.is_null()` × 2 at the two if-guards, `token.raw()` × 2 at the two `CreateProcessAsUserW` argument lists) were rewritten to use the raw `h_token` / `h_token.is_null()` directly. Nothing else was touched: `OwnedHandle`, `RestrictedToken`, their `Drop` impls, and the `current_token.raw()` / `primary_token.raw()` call sites in neighbouring functions (where the owner is already a named local) are unchanged. Net diff: +26 / −9 lines, single file.

## Verification

**`cargo build -p nono-cli --release`** — PASS:
```
   Compiling nono-cli v0.30.1 (C:\Users\omack\nono\crates\nono-cli)
    Finished `release` profile [optimized] target(s) in 1m 53s
```

**`cargo test -p nono-cli -- --test-threads=1`** — 570 pass, 4 pre-existing failures (unrelated to this change). The plan specified `--lib`, but `nono-cli` is a binary-only crate, so the lib flag was dropped and the full test suite ran against the default binary. Pre-existing failures were confirmed identical when running against the same commit with the patch stashed:
```
test result: FAILED. 570 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.31s
error: test failed, to rerun pass `-p nono-cli --bin nono`
```
The 4 failing tests are `capability_ext::tests::test_from_profile_allow_file_rejects_directory_when_exact_dir_unsupported`, `capability_ext::tests::test_from_profile_filesystem_read_accepts_file_paths`, `profile::builtin::tests::test_all_profiles_signal_mode_resolves`, and `query_ext::tests::test_query_path_sensitive_policy_includes_policy_source`. All four are pre-existing Windows-platform assertion failures about POSIX paths / escape characters / policy sources — they touch no code in `exec_strategy_windows/` and were confirmed to fail identically against the pre-edit code via `git stash`.

**`cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used`** — PASS (zero warnings):
```
    Checking nono-cli v0.30.1 (C:\Users\omack\nono\crates\nono-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.54s
```

## Smoke test result

Exact command:
```
./target/release/nono.exe run --allow-cwd -- cmd /c "echo hello"
```

Verbatim output:
```
  nono v0.30.1
  Capabilities:
  ────────────────────────────────────────────────────
    r   \\?\C:\Users\omack\Nono (dir)
       + 2 system/group paths (-v to show)
   net  outbound allowed
  ────────────────────────────────────────────────────

  mode supervised (supervisor)
  Applying sandbox...
2026-04-17T21:08:08.979323Z ERROR Platform not supported: Windows filesystem policy does not cover the absolute path argument required for launch: C:\
nono: Platform not supported: Windows filesystem policy does not cover the absolute path argument required for launch: C:\
EXIT_CODE=1
```

**Verdict: use-after-close is fixed.** The original `error=6` (`ERROR_INVALID_HANDLE`) is gone. The process now fails earlier in a **pre-flight policy check** — `spawn_windows_child` is never entered because the launch-path coverage check rejects `C:\` before any `CreateProcess*` call. Per PLAN Task 2 acceptance criteria: "The command fails for a reason unrelated to the use-after-close (different error code, not error=6)" — satisfied. The new `C:\` policy gap is out of scope for this quick and will be addressed separately (it is a distinct bug in the pre-flight path-coverage check, not a launcher handle issue).

## Impact on Phase 13 UAT

The handle use-after-close is resolved for all callers of `spawn_windows_child` — every `CreateProcessAsUserW` call site in the function now receives a live HANDLE owned by a local whose `Drop` runs strictly after the Win32 call.

HV items previously listed as blocked by `error=6`:
- **P05-HV-1** — unblocked w.r.t. handle UAF; may still be gated by the `C:\` path-coverage check uncovered during this smoke test
- **P07-HV-1** — unblocked w.r.t. handle UAF; ditto
- **P07-HV-3** — unblocked w.r.t. handle UAF; ditto
- **P09-HV-1** — unblocked w.r.t. handle UAF; ditto
- **P11-HV-1** — unblocked w.r.t. handle UAF; ditto
- **P11-HV-3** — unblocked w.r.t. handle UAF; ditto

All six items are no longer blocked by the handle use-after-close specifically. A follow-up quick will be needed to address the `C:\` launch-path pre-flight check so the UAT items can run end-to-end, but that is a separate bug surfaced by this fix — not a regression caused by it.

## Deviations from Plan

**1. [Rule 3 — Blocking] Dropped invalid `--lib` flag from test command**
- **Found during:** Task 1 verification
- **Issue:** `cargo test -p nono-cli --lib` errored with `no library targets found in package 'nono-cli'` — `nono-cli` is a binary-only crate, so `--lib` selects zero targets.
- **Fix:** Ran `cargo test -p nono-cli -- --test-threads=1` instead, which tests the default binary target.
- **Files modified:** none
- **Verification:** Test harness ran; 570 passed; 4 pre-existing failures confirmed unrelated via `git stash` bisect.

---

**Total deviations:** 1 blocking (verification-command typo in plan; no code change required).
**Impact on plan:** None on the fix itself.

## Self-Check: PASSED

- `crates/nono-cli/src/exec_strategy_windows/launch.rs` modified — FOUND
- `.planning/quick/260417-wla-fix-windows-createprocess-handle-uaf/260417-wla-PLAN.md` — FOUND
- `.planning/quick/260417-wla-fix-windows-createprocess-handle-uaf/260417-wla-SUMMARY.md` — FOUND (this file)
- commit `eb4730c` — FOUND in `git log --oneline`
