---
quick_id: 260417-wla
slug: fix-windows-createprocess-handle-uaf
type: quick
created: 2026-04-17
files_modified:
  - crates/nono-cli/src/exec_strategy_windows/launch.rs

must_haves:
  truths:
    - "nono run --allow-cwd -- <cmd> no longer fails with 'Failed to launch Windows child process (error=6)' on Windows"
    - "The RestrictedToken / OwnedHandle returned by create_restricted_token_with_sid / create_low_integrity_primary_token is bound to a named local that lives through the CreateProcess call, not a temporary that drops (closing the handle) before use"
    - "The redundant OwnedHandle(h_token) wrapper at launch.rs:841 is removed so the handle is closed exactly once (by its original owner)"
    - "cargo build -p nono-cli --release succeeds"
    - "cargo test -p nono-cli succeeds (no new failures)"
    - "make clippy passes (no new warnings on launch.rs)"
  artifacts:
    - path: "crates/nono-cli/src/exec_strategy_windows/launch.rs"
      provides: "Fixed spawn_windows_child token lifetime handling"
      contains: "spawn_windows_child"
---

<objective>
Fix a use-after-close bug in the Windows child process launcher. In `spawn_windows_child` (crates/nono-cli/src/exec_strategy_windows/launch.rs:834-841), the token handle is obtained via `?.h_token` / `?.raw()` on a temporary `RestrictedToken` or `OwnedHandle`. The temporary drops immediately after the raw HANDLE is extracted, which closes the handle via `CloseHandle`. The raw HANDLE is then wrapped in a **new** `OwnedHandle(h_token)` and later passed to `CreateProcessAsUserW`, which fails with `ERROR_INVALID_HANDLE (6)`. On function exit, the second `OwnedHandle` drops and double-closes the same HANDLE value.

**Trigger:** Any `nono run` invocation with at least one filesystem capability (e.g. `--allow-cwd`) takes the `should_use_low_integrity_windows_launch == true` branch, hits the bug, and fails to start the child.

**Discovered during Phase 13 UAT (2026-04-17).** Blocks 6 of 10 UAT items: P05-HV-1, P07-HV-1, P07-HV-3, P09-HV-1, P11-HV-1, P11-HV-3.

**Fix shape:**
1. Hoist the two potential holders into named `Option<>` bindings that outlive the `CreateProcess{AsUser}W` call.
2. Drop the redundant `let token = OwnedHandle(h_token);` wrapper — the original holder already owns the handle.
3. Replace the 4 `token.raw()` / `token.0.is_null()` references with the raw `HANDLE` / `h_token.is_null()` directly.
</objective>

<tasks>

<task type="auto">
  <name>Task 1: Apply token-lifetime fix in spawn_windows_child</name>
  <files>crates/nono-cli/src/exec_strategy_windows/launch.rs</files>
  <read_first>
    - crates/nono-cli/src/exec_strategy_windows/launch.rs (full function spawn_windows_child, ~lines 822-1030)
    - crates/nono-cli/src/exec_strategy_windows/restricted_token.rs (RestrictedToken Drop impl)
    - crates/nono-cli/src/exec_strategy_windows/mod.rs (OwnedHandle struct declaration, line 351)
  </read_first>
  <action>
In `crates/nono-cli/src/exec_strategy_windows/launch.rs`:

**1. Replace the token-acquisition block (approx lines 833-841).** Current code:

```rust
    // Create restricted token if session SID was generated during network enforcement setup
    let h_token = if let Some(ref sid) = config.session_sid {
        restricted_token::create_restricted_token_with_sid(sid)?.h_token
    } else if should_use_low_integrity_windows_launch(config.caps) {
        create_low_integrity_primary_token()?.raw()
    } else {
        std::ptr::null_mut() // Use current process token (CreateProcessW)
    };
    let token = OwnedHandle(h_token);
```

Replace with:

```rust
    // Bind each potential holder to a named local so its Drop does NOT run
    // until after CreateProcess{AsUser}W uses the raw HANDLE. Previously,
    // `?.h_token` / `?.raw()` returned a raw HANDLE from a temporary which
    // dropped (closing the handle) before it was passed to the Win32 API,
    // yielding ERROR_INVALID_HANDLE (6).
    let _restricted_holder: Option<restricted_token::RestrictedToken>;
    let _low_integrity_holder: Option<OwnedHandle>;
    let h_token: HANDLE = if let Some(ref sid) = config.session_sid {
        let holder = restricted_token::create_restricted_token_with_sid(sid)?;
        let raw = holder.h_token;
        _restricted_holder = Some(holder);
        _low_integrity_holder = None;
        raw
    } else if should_use_low_integrity_windows_launch(config.caps) {
        let holder = create_low_integrity_primary_token()?;
        let raw = holder.0;
        _low_integrity_holder = Some(holder);
        _restricted_holder = None;
        raw
    } else {
        _restricted_holder = None;
        _low_integrity_holder = None;
        std::ptr::null_mut() // Use current process token (CreateProcessW)
    };
    // NOTE: do NOT re-wrap h_token in a fresh OwnedHandle — the holder above
    // already owns the close. A second wrapper would double-close on Drop.
```

**2. Update the 4 downstream references to `token` inside `spawn_windows_child`:**

- `if !token.0.is_null() {` (approx line 908) → `if !h_token.is_null() {`
- `token.raw(),` on the `CreateProcessAsUserW` argument (approx line 913) → `h_token,`
- `if !token.0.is_null() {` (approx line 957) → `if !h_token.is_null() {`
- `token.raw(),` on the `CreateProcessAsUserW` argument (approx line 961) → `h_token,`

**Do not touch:** lines 730 (`current_token.raw()`) or 780 (`primary_token.raw()`) — these are in different functions where the owner is already a named local.

**Do not change:** the `OwnedHandle` type definition, the `RestrictedToken` struct, or any Drop impls. The fix is purely in how `spawn_windows_child` captures ownership.
  </action>
  <verify>
    <automated>cargo build -p nono-cli --release 2>&1 | tail -5</automated>
    <automated>cargo test -p nono-cli --lib -- --test-threads=1 2>&1 | tail -20</automated>
    <automated>make clippy 2>&1 | tail -10</automated>
  </verify>
  <acceptance_criteria>
    - `cargo build -p nono-cli --release` exits 0
    - `cargo test -p nono-cli --lib` exits 0 (no new test failures)
    - `make clippy` exits 0 (no new warnings; pre-existing warnings on unrelated files are acceptable but must not have grown)
    - `crates/nono-cli/src/exec_strategy_windows/launch.rs` no longer contains the string `let token = OwnedHandle(h_token);`
    - `crates/nono-cli/src/exec_strategy_windows/launch.rs` no longer contains `?.h_token` or `?.raw()` on an unnamed temporary in `spawn_windows_child` — every token owner is a named local
    - `spawn_windows_child` contains either `_restricted_holder` or `_low_integrity_holder` (or equivalent named bindings for the same purpose)
    - Token-bearing `CreateProcess{AsUser}W` call sites reference `h_token` (the raw HANDLE) directly, not `token.raw()`
  </acceptance_criteria>
  <done>launch.rs compiles and tests pass; token-acquisition block uses named holders that outlive CreateProcess; no OwnedHandle(h_token) redundant wrapper; 4 downstream call-site updates done.</done>
</task>

<task type="auto">
  <name>Task 2: Smoke-test the fix with a real nono run invocation</name>
  <files>(no file edits — runtime verification only)</files>
  <read_first>
    - .planning/phases/13-v1-human-verification-uat/13-UAT.md (optional context)
  </read_first>
  <action>
This task is a smoke test, not a code change. Run:

```
./target/release/nono.exe run --allow-cwd -- cmd /c "echo hello"
```

Expected output (approximately):
- `hello` is printed to stdout
- Exit code is 0
- No `Failed to launch Windows child process (error=6)` error
- Banner shows capabilities including the CWD read permission

If the command still errors with `error=6`, the fix is incomplete — STOP and report.

If the command errors with a **different** Windows error code, record the new error but treat the original bug as fixed (the use-after-close is resolved; the new error is out of scope).

If the command prints `hello` cleanly, the fix is verified.

**No commit for this task** — it's a runtime check only. Record the observed output in the SUMMARY.md for Task 1.
  </action>
  <verify>
    <automated>./target/release/nono.exe run --allow-cwd -- cmd /c "echo hello" 2>&1 | tail -20</automated>
  </verify>
  <acceptance_criteria>
    - `nono run --allow-cwd -- cmd /c "echo hello"` prints `hello` and exits 0, OR
    - The command fails for a reason unrelated to the use-after-close (different error code, not error=6)
  </acceptance_criteria>
  <done>Smoke test run and result recorded in SUMMARY.md.</done>
</task>

</tasks>

<success_criteria>
1. `spawn_windows_child` no longer has any temporary-drop use-after-close on token handles
2. The redundant `OwnedHandle(h_token)` wrapper is removed (so no double-close)
3. `cargo build -p nono-cli --release` succeeds
4. `cargo test -p nono-cli --lib` passes
5. Smoke test: `nono run --allow-cwd -- cmd /c "echo hello"` prints `hello` and exits 0 (unblocks Phase 13 UAT)
6. No new clippy warnings introduced
</success_criteria>

<output>
After completion, create `.planning/quick/260417-wla-fix-windows-createprocess-handle-uaf/260417-wla-SUMMARY.md`
</output>
