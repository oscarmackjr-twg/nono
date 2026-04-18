---
phase: quick/260410-nlt
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/nono-cli/src/learn_windows.rs
  - crates/nono-cli/src/learn_runtime.rs
autonomous: true
requirements:
  - UAT-GAP-1  # Admin gate ordering (Gap 1, test 1)
  - UAT-GAP-2  # LearnState::new dead code on Windows (Gap 2, test 2)
  - UAT-GAP-4  # Zone.Identifier ADS filter (Gap 4 partial, test 4)

must_haves:
  truths:
    - "Non-admin `nono learn` on Windows prints the admin error to stderr and exits non-zero WITHOUT showing the `Continue? [y/N]` warning prompt first"
    - "`cargo clippy -p nono-cli -- -D warnings` compiles cleanly on Windows with no dead_code warning on LearnState::new"
    - "classify_and_record_file_access drops any NTFS Alternate Data Stream suffix (`:Zone.Identifier`, etc.) from the NT path before classification, so no `...:Zone.Identifier` entries appear in learn output"
  artifacts:
    - path: crates/nono-cli/src/learn_runtime.rs
      provides: "Windows admin pre-check before warning prompt"
      contains: "is_admin_process"
    - path: crates/nono-cli/src/learn_windows.rs
      provides: "Single LearnState::new constructor; ADS-stripped classify_and_record_file_access"
      contains: "strip_ads_suffix"
  key_links:
    - from: crates/nono-cli/src/learn_runtime.rs
      to: crates/nono-cli/src/exec_strategy_windows/mod.rs
      via: "crate::exec_strategy::is_admin_process()"
      pattern: "is_admin_process"
    - from: crates/nono-cli/src/learn_windows.rs
      to: "classify_and_record_file_access path handling"
      via: "ADS suffix stripped before nt_to_win32"
      pattern: "strip_ads_suffix|:Zone\\.Identifier|\\.split\\(':'\\)"
---

<objective>
Close three UAT gaps from `.planning/phases/10-etw-based-learn-command/10-UAT.md`:

1. **Gap 1 (major):** Admin gate fires AFTER the "Continue? [y/N]" warning prompt on Windows. Non-admin users see the prompt before getting rejected. The admin check must happen first.
2. **Gap 2 (major):** `LearnState::new(root_pid, volume_map)` became dead code on Windows after the WR-03 fix switched `run_learn` to call `new_empty(volume_map)`. `-D warnings` fails the Windows build.
3. **Gap 4 partial (major):** NTFS Alternate Data Streams like `\Device\HarddiskVolume3\...\kernel32.dll:Zone.Identifier` pollute learn output. Every DLL ends up with a `:Zone.Identifier` sibling entry. Must be filtered at the file-classification boundary.

Purpose: Unblock Phase 10 UAT so it can move from "issue" to "pass" on tests 1, 2, and 4 (ADS noise).
Output: Updated `learn_runtime.rs` + `learn_windows.rs`; `make ci` clean; new unit tests for ADS stripping.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md
@.planning/phases/10-etw-based-learn-command/10-UAT.md
@crates/nono-cli/src/learn_windows.rs
@crates/nono-cli/src/learn_runtime.rs
@crates/nono-cli/src/learn.rs

<interfaces>
<!-- Key contracts the executor needs. Do NOT go exploring — these are the exact signatures. -->

From crates/nono-cli/src/learn_windows.rs (current state):
```rust
// Private constant — must be made pub(crate) so learn_runtime can use the exact same string.
const NON_ADMIN_ERROR: &str = "nono learn requires administrator privileges. \
    Run from an elevated prompt (right-click \u{2192} Run as administrator).";

impl LearnState {
    // Currently dead on Windows because run_learn calls new_empty() instead.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub fn new(root_pid: u32, volume_map: HashMap<String, String>) -> Self {
        let mut s = Self::new_empty(volume_map);
        s.tracked_pids.insert(root_pid);
        s
    }

    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub fn new_empty(volume_map: HashMap<String, String>) -> Self { /* ... */ }
}

pub(crate) fn classify_and_record_file_access(state: &mut LearnState, pid: u32, nt_path: &str) {
    if !state.is_tracked(pid) { return; }
    let Some(win32_path) = nt_to_win32(nt_path, &state.volume_map) else { return; };
    state.result.readwrite_paths.insert(win32_path);
}

pub fn run_learn(args: &LearnArgs) -> Result<LearnResult> {
    if !is_admin() {
        return Err(NonoError::LearnError(NON_ADMIN_ERROR.to_string()));
    }
    // ... uses LearnState::new_empty(volume_map) at ~line 398
}

fn is_admin() -> bool { crate::exec_strategy::is_admin_process() }
```

From crates/nono-cli/src/learn_runtime.rs (current state, called BEFORE learn::run_learn):
```rust
pub(crate) fn run_learn(args: LearnArgs, silent: bool) -> Result<()> {
    if !silent {
        // Prompt fires here — BEFORE the Windows admin gate in learn_windows::run_learn.
        eprintln!("WARNING: nono learn runs the command WITHOUT any sandbox restrictions.");
        eprint!("Continue? [y/N] ");
        // reads stdin ...
    }
    let result = learn::run_learn(&args)?; // dispatches to learn_windows::run_learn on Windows
    // ...
}
```

From crates/nono-cli/src/exec_strategy_windows/mod.rs:
```rust
pub(crate) fn is_admin_process() -> bool;
```

From crates/nono-cli/src/learn.rs (dispatch shim):
```rust
#[cfg(target_os = "windows")]
pub fn run_learn(args: &LearnArgs) -> Result<LearnResult> {
    crate::learn_windows::run_learn(args)
}
```

Existing test call sites in learn_windows.rs (lines ~799–933) that use the OLD `LearnState::new(pid, volume_map)` signature — these MUST be updated when the constructors merge:
```rust
let state = LearnState::new(1234, HashMap::new());
let mut state = LearnState::new(1234, HashMap::new());  // appears ~8 times
fn state_with_map(root_pid: u32) -> LearnState {
    let mut map = HashMap::new();
    // ...
    LearnState::new(root_pid, map)
}
```
After merge, these become `LearnState::new(HashMap::new())` + explicit `state.tracked_pids.insert(1234);` (or a small test helper).
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Fix admin gate ordering (Gap 1) and merge LearnState constructors (Gap 2)</name>
  <files>crates/nono-cli/src/learn_windows.rs, crates/nono-cli/src/learn_runtime.rs</files>
  <behavior>
    - Test (unit, learn_windows.rs): `LearnState::new(HashMap::new())` returns a state with empty `tracked_pids` (replaces old `new_empty` semantics; no root_pid parameter).
    - Test (unit, learn_windows.rs): After `LearnState::new(map); state.tracked_pids.insert(1234);`, `state.is_tracked(1234) == true`.
    - Behavior (learn_runtime.rs, Windows only): when `is_admin_process()` returns false, `run_learn` returns `Err(NonoError::LearnError(NON_ADMIN_ERROR))` BEFORE any `eprintln!("WARNING...")` or stdin read. On non-Windows platforms, behavior is unchanged.
    - Behavior: no `#[allow(dead_code)]` attribute is needed on `LearnState::new` because it now has real call sites (run_learn + tests) on Windows.
  </behavior>
  <action>
    **Step A — Merge LearnState constructors (learn_windows.rs):**

    1. Delete the current `pub fn new(root_pid: u32, volume_map: HashMap<String, String>) -> Self` at ~line 75–80.
    2. Rename `pub fn new_empty(volume_map: HashMap<String, String>) -> Self` at ~line 89 to `pub fn new(volume_map: HashMap<String, String>) -> Self`. Keep it `pub(crate)` visibility-wise (currently `pub` — preserve that).
    3. Remove the `#[cfg_attr(not(target_os = "windows"), allow(dead_code))]` attribute from the merged `new()` — it now has a real call site on Windows (run_learn) so the lint will not fire.
    4. Update the doc comment: delete references to "new_empty" and rewrite it as "Create a `LearnState` with an empty tracked-PID set. Callers must insert the root PID via `state.tracked_pids.insert(pid)` after spawning the child."
    5. Update the `run_learn` call site at ~line 398: `LearnState::new_empty(volume_map)` → `LearnState::new(volume_map)`.
    6. Update all unit test call sites (~lines 799, 806, 813, 820, 828, 837, 846, 855, 864–867, 933). The old `LearnState::new(1234, HashMap::new())` signature is gone. Replace each with:
       ```rust
       let mut state = LearnState::new(HashMap::new());
       state.tracked_pids.insert(1234);
       ```
       For the `state_with_map` helper, do the same: `let mut s = LearnState::new(map); s.tracked_pids.insert(root_pid); s`.
       Do NOT introduce a `new_with_pid` helper — keep the surface minimal. The explicit two-line pattern is fine for tests.

    **Step B — Expose admin constant (learn_windows.rs):**

    7. Change `const NON_ADMIN_ERROR: &str = ...` at line 46 from private `const` to `pub(crate) const NON_ADMIN_ERROR: &str = ...`. Do not change the string value — Phase 10 tests depend on exact substrings ("nono learn requires administrator privileges", "Run from an elevated prompt", "Run as administrator").
    8. Also expose the existing `fn is_admin() -> bool` helper at ~line 702 as `pub(crate) fn is_admin()` (it is already there for both Windows and non-Windows via cfg). This avoids callers reaching into `exec_strategy` directly and keeps the admin policy centralized in learn_windows.

    **Step C — Pre-check admin on Windows before prompt (learn_runtime.rs):**

    9. At the very top of `pub(crate) fn run_learn(args: LearnArgs, silent: bool) -> Result<()>` (before the `if !silent` block), insert:
       ```rust
       // D-02 / UAT Gap 1: on Windows, reject non-admin invocations immediately,
       // BEFORE the interactive warning prompt. This matches the phase 10 admin gate
       // contract — users should never see "Continue? [y/N]" if they cannot run learn
       // in the first place.
       #[cfg(target_os = "windows")]
       {
           if !crate::learn_windows::is_admin() {
               return Err(NonoError::LearnError(
                   crate::learn_windows::NON_ADMIN_ERROR.to_string(),
               ));
           }
       }
       ```
    10. Do NOT remove the admin check inside `learn_windows::run_learn` — keep it as defense-in-depth so library-mode callers (bypassing `learn_runtime`) still fail secure. Both checks use the same `NON_ADMIN_ERROR` constant, so there is no drift.

    **Step D — Verify no regressions:**

    11. Search `crates/nono-cli/src` for any remaining `LearnState::new_empty(` references. There should be zero after the rename. If any exist, update them.
    12. Run `cargo build -p nono-cli` and `cargo clippy -p nono-cli -- -D warnings` (Linux is fine for clippy verification — the cfg_attr gate is gone so the warning state is now platform-neutral).
    13. Run `cargo test -p nono-cli --lib learn_windows` to verify the updated LearnState tests compile and pass on Linux (they are compiled unconditionally since learn_windows.rs uses cfg_attr guards, not module-level cfg).
  </action>
  <verify>
    <automated>cargo clippy -p nono-cli -- -D warnings 2>&1 | tee /tmp/clippy.log; grep -q "LearnState::new" /tmp/clippy.log && echo "FAIL: dead_code on LearnState::new" && exit 1; cargo test -p nono-cli --lib learn</automated>
  </verify>
  <done>
    - `LearnState::new_empty` is gone; `LearnState::new(HashMap<String, String>)` is the sole constructor.
    - `NON_ADMIN_ERROR` and `is_admin` are `pub(crate)` in learn_windows.rs.
    - `learn_runtime::run_learn` returns the non-admin error on Windows before reaching the warning prompt (verified by code inspection — task 3 will do the runtime verification).
    - `cargo clippy -p nono-cli -- -D warnings` is clean; no `dead_code` warning mentions `LearnState::new`.
    - `cargo test -p nono-cli --lib learn` passes.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Filter NTFS Alternate Data Stream suffixes from learn output (Gap 4)</name>
  <files>crates/nono-cli/src/learn_windows.rs</files>
  <behavior>
    - Test: `strip_ads_suffix("\\Device\\HarddiskVolume3\\Windows\\System32\\kernel32.dll:Zone.Identifier")` returns `"\\Device\\HarddiskVolume3\\Windows\\System32\\kernel32.dll"`.
    - Test: `strip_ads_suffix("\\Device\\HarddiskVolume3\\Windows\\System32\\kernel32.dll")` returns the input unchanged (no ADS).
    - Test: `strip_ads_suffix("\\Device\\HarddiskVolume3\\foo.txt:customstream")` returns `"\\Device\\HarddiskVolume3\\foo.txt"` (not just Zone.Identifier — ANY ADS stream must be stripped).
    - Test: `strip_ads_suffix("\\Device\\NamedPipe\\chrome.1234")` returns input unchanged. `:1234` after `chrome.` is NOT an ADS — the colon-detection must only apply to the LAST path component AFTER the final backslash, and only when the substring before the colon contains a `.` (filename with extension) or is otherwise a recognizable filename. Safer rule: split on last `\`, then in the final segment, if there is a colon, truncate at the first colon. For `\Device\NamedPipe\chrome.1234` the final segment is `chrome.1234` which has no colon → unchanged. ✓
    - Test: `strip_ads_suffix("C:\\Users\\foo.txt:Zone.Identifier")` — but note this function operates on NT paths ONLY (before nt_to_win32). Still, test a drive-letter-style input returns `"C:\\Users\\foo.txt"` (first colon in final segment wins, but we must not strip `C:` itself — since `C:` is in the FIRST segment, it is preserved. The rule "last segment only" handles this.)
    - Test (end-to-end unit): after calling `classify_and_record_file_access(&mut state, pid, "\\Device\\HarddiskVolume3\\Windows\\System32\\kernel32.dll:Zone.Identifier")` with a volume map that resolves HarddiskVolume3 → C:, `state.result.readwrite_paths` contains `"C:\\Windows\\System32\\kernel32.dll"` and does NOT contain any string ending in `:Zone.Identifier`.
    - Test: the `nt_to_win32` conversion is NOT bypassed — ADS stripping happens BEFORE conversion so the path-classification + drive-letter resolution paths stay identical to before.
  </behavior>
  <action>
    **Step A — Add `strip_ads_suffix` helper (learn_windows.rs):**

    1. Add a private helper near `classify_and_record_file_access` (~line 260):
       ```rust
       /// Strip NTFS Alternate Data Stream suffixes from an NT path.
       ///
       /// NTFS supports named streams via the syntax `path\to\file.ext:streamname`.
       /// The most common case nono sees is `:Zone.Identifier`, which Windows attaches
       /// to every downloaded file as a Mark-of-the-Web marker. These streams are
       /// filesystem metadata, not distinct files — granting access to them is
       /// semantically wrong and pollutes learn output (UAT Gap 4).
       ///
       /// Rule: only the FINAL path segment (after the last backslash) is considered.
       /// If that segment contains a `:`, truncate at the first colon. This preserves
       /// drive-letter prefixes like `C:\...` (which live in the first segment) and
       /// `\Device\NamedPipe\chrome.1234`-style names that have no colon in the final
       /// segment.
       ///
       /// Returns a borrowed slice of the input — no allocation on the hot path.
       fn strip_ads_suffix(nt_path: &str) -> &str {
           // Find the final segment after the last backslash.
           let last_sep = nt_path.rfind('\\');
           let (prefix_end, segment_start) = match last_sep {
               Some(idx) => (idx + 1, idx + 1),
               None => (0, 0),
           };
           let segment = &nt_path[segment_start..];
           match segment.find(':') {
               Some(colon_idx) => &nt_path[..prefix_end + colon_idx],
               None => nt_path,
           }
       }
       ```

    2. Update `classify_and_record_file_access` at ~line 261. Insert the strip as the first line of the function body (before the tracked-PID guard is fine, but after is slightly cheaper; put it right before `nt_to_win32`):
       ```rust
       pub(crate) fn classify_and_record_file_access(state: &mut LearnState, pid: u32, nt_path: &str) {
           if !state.is_tracked(pid) {
               return;
           }
           // UAT Gap 4: strip NTFS ADS suffixes (e.g. `:Zone.Identifier`) before
           // classification. These are filesystem metadata streams, not files, and
           // would otherwise pollute learn output with a `:Zone.Identifier` entry
           // for every downloaded DLL.
           let nt_path = strip_ads_suffix(nt_path);
           let Some(win32_path) = nt_to_win32(nt_path, &state.volume_map) else {
               debug!(nt_path, "learn_windows: skipping non-drive NT path");
               return;
           };
           state.result.readwrite_paths.insert(win32_path);
       }
       ```

    **Step B — Add unit tests (learn_windows.rs, in the existing `#[cfg(test)] mod tests` block near the bottom):**

    3. Add these tests to the existing tests module (same module that already contains `state_with_map`, `classify_and_record_file_access` tests). Follow the existing helper pattern:
       ```rust
       #[test]
       fn strip_ads_suffix_removes_zone_identifier() {
           let input = "\\Device\\HarddiskVolume3\\Windows\\System32\\kernel32.dll:Zone.Identifier";
           assert_eq!(
               strip_ads_suffix(input),
               "\\Device\\HarddiskVolume3\\Windows\\System32\\kernel32.dll"
           );
       }

       #[test]
       fn strip_ads_suffix_passthrough_no_stream() {
           let input = "\\Device\\HarddiskVolume3\\Windows\\System32\\kernel32.dll";
           assert_eq!(strip_ads_suffix(input), input);
       }

       #[test]
       fn strip_ads_suffix_removes_arbitrary_stream_name() {
           let input = "\\Device\\HarddiskVolume3\\foo.txt:customstream";
           assert_eq!(strip_ads_suffix(input), "\\Device\\HarddiskVolume3\\foo.txt");
       }

       #[test]
       fn strip_ads_suffix_preserves_named_pipe_without_colon() {
           let input = "\\Device\\NamedPipe\\chrome.1234";
           assert_eq!(strip_ads_suffix(input), input);
       }

       #[test]
       fn strip_ads_suffix_only_considers_final_segment() {
           // A colon earlier in the path (hypothetical) must not split.
           let input = "\\Device\\HarddiskVolume3\\dir\\file.dll";
           assert_eq!(strip_ads_suffix(input), input);
       }

       #[test]
       fn classify_and_record_file_access_filters_zone_identifier() {
           let mut state = state_with_map(1234);
           classify_and_record_file_access(
               &mut state,
               1234,
               "\\Device\\HarddiskVolume3\\Windows\\System32\\kernel32.dll:Zone.Identifier",
           );
           // The clean path must be present.
           assert!(state
               .result
               .readwrite_paths
               .iter()
               .any(|p| p.ends_with("kernel32.dll")));
           // No :Zone.Identifier entries anywhere.
           assert!(
               !state
                   .result
                   .readwrite_paths
                   .iter()
                   .any(|p| p.contains(":Zone.Identifier")),
               "ADS suffix leaked into readwrite_paths: {:?}",
               state.result.readwrite_paths
           );
       }
       ```

    **Step C — Verify:**

    4. Run `cargo test -p nono-cli --lib learn_windows -- strip_ads_suffix classify_and_record_file_access_filters_zone_identifier`.
    5. Run full `cargo test -p nono-cli --lib learn_windows` to ensure no other tests regressed.
    6. Check that the debug log in `classify_and_record_file_access` now uses the stripped path, not the original (which is desirable — keeps logs consistent with output).
  </action>
  <verify>
    <automated>cargo test -p nono-cli --lib learn_windows:: 2>&1 | tail -40</automated>
  </verify>
  <done>
    - `strip_ads_suffix` function exists in learn_windows.rs and handles: Zone.Identifier, arbitrary stream names, passthrough, final-segment-only rule.
    - `classify_and_record_file_access` calls `strip_ads_suffix` before `nt_to_win32`.
    - Six new unit tests pass.
    - Full `cargo test -p nono-cli --lib learn_windows` is green.
  </done>
</task>

<task type="auto">
  <name>Task 3: CI gate + integration sanity (make ci)</name>
  <files></files>
  <action>
    1. Run `make ci` from the repo root. This runs clippy (`-D warnings -D clippy::unwrap_used`), fmt-check, and the full workspace tests. This is the single authoritative gate per CLAUDE.md.
    2. If `cargo fmt --all --check` fails, run `cargo fmt --all` and verify the only changes are to the files touched in tasks 1 and 2.
    3. If any unrelated pre-existing Windows compile error surfaces (per UAT test 3, `policy.rs` and `trust_keystore.rs` have pre-existing Windows compile errors predating phase 10), DO NOT fix them — note them in the summary as "blocked by prior phase, out of scope". The gate for this quick plan is that the files touched compile and clippy-clean on Linux (the host make ci runs on) and that unit tests for `learn_windows` pass.
    4. Confirm by inspection (grep) that no `new_empty(` call sites remain anywhere under `crates/` and no `:Zone.Identifier` string appears in sample learn output from any test fixture.
  </action>
  <verify>
    <automated>make ci 2>&1 | tail -60</automated>
  </verify>
  <done>
    - `make ci` passes on the development host.
    - No `LearnState::new_empty` references remain in the workspace.
    - Summary notes whether any pre-existing Windows-only compile errors blocked additional Windows verification (they are out of scope for this quick fix).
  </done>
</task>

</tasks>

<verification>
- `cargo clippy -p nono-cli -- -D warnings` emits no `dead_code` warning naming `LearnState::new`.
- `cargo test -p nono-cli --lib learn` passes, including the six new tests from task 2.
- Code inspection of `learn_runtime::run_learn` shows the Windows admin gate is the FIRST statement inside the function body, before `if !silent`.
- `grep -R "LearnState::new_empty" crates/` returns nothing.
- `grep -R "strip_ads_suffix" crates/` returns the definition plus `classify_and_record_file_access` call site plus tests.
</verification>

<success_criteria>
- Gap 1 closed: non-admin learn invocation rejects without showing the warning prompt (verified by code inspection of learn_runtime.rs — the admin check is the first statement and returns Err immediately).
- Gap 2 closed: `LearnState::new` has exactly one definition and has real call sites on Windows; `clippy -D warnings` is clean.
- Gap 4 (ADS portion) closed: ADS suffixes are stripped at `classify_and_record_file_access` boundary; unit tests prove Zone.Identifier and arbitrary stream names are filtered.
- `make ci` passes.
- No changes outside `crates/nono-cli/src/learn_windows.rs` and `crates/nono-cli/src/learn_runtime.rs`.
</success_criteria>

<output>
After completion, create `.planning/quick/260410-nlt-fix-three-uat-gaps-in-phase-10-etw-learn/260410-nlt-SUMMARY.md` covering:
- Which UAT gaps closed (1, 2, 4-partial) with reference to `10-UAT.md` line numbers.
- Any residual Windows-only verification that requires a Windows host + admin shell (task 3 note).
- Suggested follow-up: update `10-UAT.md` to mark tests 1, 2, and the ADS portion of test 4 as `pass` once verified on a Windows host.
</output>
