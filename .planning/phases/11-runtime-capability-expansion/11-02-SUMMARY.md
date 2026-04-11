---
phase: 11-runtime-capability-expansion
plan: 02
subsystem: windows-supervisor-interactive-approval
tags: [windows, supervisor, terminal-approval, conin, session-token, audit-redaction, trust-01]
requires:
  - crates/nono-cli/src/terminal_approval.rs (existing)
  - 11-01 capability pipe server + session_token plumbing
provides:
  - Windows CONIN$ branch for TerminalApproval::request_capability
  - Fail-secure denial when no console is attached on Windows
  - Live wiring of TerminalApproval into the Windows capability pipe
    server thread via Arc<dyn ApprovalBackend + Send + Sync>
  - Regression tests for session_token redaction in serialized
    AuditEntry (both valid-token and wrong-token paths)
affects:
  - Windows supervised run: interactive [nono] Grant access? prompt
    now appears on the supervisor's console when the sandboxed child
    posts a RequestCapability message.
  - Windows SupervisorConfig API: approval_backend field is now an
    owned Arc instead of a borrowed &dyn. Single call site updated.
tech-stack:
  added: []
  patterns:
    - Shared Arc<TerminalApproval> used by both the Unix &dyn and
      Windows Arc<dyn +Send+Sync> paths of SupervisorConfig
    - cfg(unix) / cfg(target_os = "windows") branch inside
      request_capability with fail-secure denial on open failure
key-files:
  created:
    - .planning/phases/11-runtime-capability-expansion/deferred-items.md
  modified:
    - crates/nono-cli/src/terminal_approval.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/src/exec_strategy_windows/supervisor.rs
    - crates/nono-cli/src/supervised_runtime.rs
  deleted:
    - crates/nono-cli/src/terminal_approval_windows.rs
decisions:
  - Unify terminal_approval.rs across platforms rather than maintaining a
    separate Windows stub module. This removes a bifurcation that pre-dated
    Phase 11 and made the TerminalApproval wiring discoverable on Windows.
  - Model the Windows SupervisorConfig.approval_backend as an owned
    Arc<dyn ApprovalBackend + Send + Sync> (not a borrow). Required to move
    the backend into the 'static-lifetime capability pipe thread, and safe
    because ApprovalBackend is already defined as Send + Sync in the library.
  - Do NOT delete WindowsSupervisorDenyAllApprovalBackend. It remains as the
    compile-time fallback for code paths that build a SupervisorConfig
    without a real interactive backend (SC #4 preservation).
  - Add both handle_redacts_token_in_serialized_audit and
    handle_redacts_token_on_mismatch_audit even though 11-01 already had
    handle_redacts_token_in_audit_entry_json — the plan explicitly named
    both, and the mismatch path is a distinct redaction code path worth
    covering with its own regression.
metrics:
  duration: ~45m
  completed: 2026-04-11
  tasks: 2
  commits: 2
---

# Phase 11 Plan 02: Windows Interactive Approval + Token Redaction Tests

TerminalApproval now opens `\\.\CONIN$` on Windows when stderr is a
terminal, falls back to a fail-secure denial when no console is attached,
and is plumbed through `SupervisorConfig` into the Windows capability pipe
server thread as the live approval backend. Regression tests prove that
the per-session authentication token never survives into a serialized
`AuditEntry` on either the valid-token or mismatch path.

## What changed

### Library (`crates/nono`)

No library changes. All work is in `nono-cli`.

### CLI (`crates/nono-cli`)

**`terminal_approval.rs` (unified across platforms)**

- The file formerly compiled only on non-Windows; Windows used a separate
  `terminal_approval_windows.rs` stub that returned `Denied` for every
  request. Both are now collapsed into a single cross-platform file.
- `request_capability` body is shared up to the terminal-device open.
  At that point a cfg branch picks between `/dev/tty` (unix) and
  `\\.\CONIN$` (target_os = "windows"). On a Windows host where the open
  of `\\.\CONIN$` fails (no attached console), the backend returns
  `ApprovalDecision::Denied { reason: "No console available for
  interactive approval" }` and logs a warning. This closes T-11-10 and
  satisfies SC #2's fail-secure requirement.
- Two new unit tests:
  - `sanitize_for_terminal_strips_ansi` — cross-platform regression
    guard that the sanitization helper still strips `\x1b[31m...\x1b[0m`.
  - `windows_no_console_denies_gracefully` (gated
    `#[cfg(target_os = "windows")]`) — under `cargo test` stderr is
    captured, so the early `is_terminal()` guard fires and the test
    asserts the denial reason mentions "terminal" / "console" / "tty".
- `main.rs` no longer routes `terminal_approval` through a
  `#[path = "terminal_approval_windows.rs"]` redirect; it is a single
  unified module.

**`exec_strategy_windows/mod.rs`**

- `SupervisorConfig.approval_backend` changes from
  `&'a dyn ApprovalBackend` to
  `Arc<dyn ApprovalBackend + Send + Sync>`. The backend can now be
  moved into a `'static` background thread. There is only one call site
  (`supervised_runtime.rs`).
- `WindowsSupervisorDenyAllApprovalBackend` and its `ApprovalBackend`
  implementation remain defined in the same file as the compile-time
  fallback (SC #4).
- Call site that logs the backend name via
  `supervisor.approval_backend.backend_name()` was updated to
  `.as_ref().backend_name()` to work through the `Arc`.

**`exec_strategy_windows/supervisor.rs`**

- `WindowsSupervisorRuntime` gains an `approval_backend` field
  (`Arc<dyn ApprovalBackend + Send + Sync>`), initialized from
  `supervisor.approval_backend.clone()` in `initialize()`.
- `#[derive(Debug)]` removed from `WindowsSupervisorRuntime` (the new
  `dyn ApprovalBackend` trait object is not `Debug`; no caller relied
  on the derive).
- `start_capability_pipe_server` clones `self.approval_backend` into
  the thread closure as `backend`, and passes `backend.as_ref()` to
  `handle_windows_supervisor_message` in place of the previous local
  `WindowsSupervisorDenyAllApprovalBackend::default()` stand-in. The
  TODO comment referencing plan 11-02 was removed and replaced with a
  reference to where the backend is plumbed from.
- `capability_handler_tests` module gains two new tests:
  - `handle_redacts_token_in_serialized_audit` — valid token, serialize
    the resulting `AuditEntry` to JSON, assert the token hex string is
    absent and `audit_log[0].request.session_token == ""`.
  - `handle_redacts_token_on_mismatch_audit` — wrong token, assert the
    backend was not consulted (`calls() == 0`), the decision is denied,
    the audit entry's `session_token` is empty, and the wrong token
    string does not appear in the serialized `AuditEntry` JSON.

**`supervised_runtime.rs`**

- `let approval_backend = TerminalApproval;` replaced by
  `let approval_backend: Arc<TerminalApproval> = Arc::new(TerminalApproval);`.
- The non-Windows `SupervisorConfig` uses `approval_backend.as_ref()`
  for its `&dyn ApprovalBackend` field (Unix API is unchanged).
- The Windows `SupervisorConfig` uses
  `approval_backend.clone() as Arc<dyn ApprovalBackend + Send + Sync>`
  to satisfy the new owned-Arc field. A comment documents that
  `TerminalApproval` on Windows now drives the interactive prompt via
  `\\.\CONIN$` (plan 11-02 Task 1).

## Verification

```
cargo check -p nono-cli                                     # OK
cargo test -p nono-cli --bin nono terminal_approval         # 15 passed
cargo test -p nono-cli --bin nono capability_handler_tests  # 7 passed
cargo test -p nono --lib supervisor                         # 16 passed
cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used  # clean
cargo fmt --all -- --check                                   # clean
```

Acceptance grep audit:

- `grep -n "CONIN\\$" crates/nono-cli/src/terminal_approval.rs` → 3 matches
  (cfg branch, docstring, test).
- `grep -nE "#\[cfg\(unix\)\]" crates/nono-cli/src/terminal_approval.rs`
  → present in the `/dev/tty` open branch.
- `grep -n "/dev/tty" crates/nono-cli/src/terminal_approval.rs` → still
  present, Unix branch preserved.
- `grep -n "TerminalApproval"
  crates/nono-cli/src/supervised_runtime.rs` → matches on lines that set
  up the shared `Arc<TerminalApproval>` and a doc comment inside the
  `#[cfg(target_os = "windows")]` block that references `TerminalApproval`
  as the Windows live approval UX.
- `grep -n "TerminalApproval"
  crates/nono-cli/src/exec_strategy_windows/supervisor.rs` → doc
  comments inside the `approval_backend` field and the
  `start_capability_pipe_server` thread closure that document the
  backend being plumbed in.
- `grep -n "WindowsSupervisorDenyAllApprovalBackend"
  crates/nono-cli/src/exec_strategy_windows/mod.rs` → struct definition
  and `impl ApprovalBackend` block both present (SC #4 preserved).
- `grep -n
  "handle_redacts_token_in_serialized_audit\|handle_redacts_token_on_mismatch_audit"
  crates/nono-cli/src/exec_strategy_windows/supervisor.rs` → both tests
  present.
- `grep -nE
  "(tracing::.*|println|eprintln|format).*session_token"` across the
  three files touched → **zero** matches.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] Separate `terminal_approval_windows.rs` stub
forced an alternative unification strategy**

- **Found during:** Task 1 test discovery.
- **Issue:** `main.rs` routed `mod terminal_approval;` through a
  `[path = "terminal_approval_windows.rs"]` redirect on Windows. The
  Windows file was a hollow stub returning `Denied` for every request
  and shipping none of the sanitize helpers. Adding a cfg branch to
  `terminal_approval.rs` had no effect on Windows because that file was
  not compiled there.
- **Fix:** Collapse both files into a single cross-platform
  `terminal_approval.rs` and drop the redirect in `main.rs`. Delete the
  stub. The unified file now compiles on all targets and exposes the
  full ANSI sanitization, prompt flow, and unit tests on Windows too.
- **Files modified:** `crates/nono-cli/src/main.rs`,
  `crates/nono-cli/src/terminal_approval_windows.rs` (deleted).
- **Commit:** 8b82609
- **Why auto-fixed:** The plan's acceptance criterion
  ("`grep CONIN\\$ terminal_approval.rs`") and its test wiring both
  assumed a single file. The pre-existing bifurcation blocked Task 1's
  `<automated>` verification from even finding the new tests. Scope was
  contained to the approval backend module.

**2. [Rule 2 — Critical] `#[derive(Debug)]` on `WindowsSupervisorRuntime`
incompatible with `dyn ApprovalBackend + Send + Sync`**

- **Found during:** Task 2 `cargo check`.
- **Issue:** The `ApprovalBackend` trait does not implement `Debug`,
  so adding an `Arc<dyn ApprovalBackend + Send + Sync>` field to a
  struct that derives `Debug` triggers `E0277`.
- **Fix:** Drop the `#[derive(Debug)]` on `WindowsSupervisorRuntime`.
  No caller uses `{:?}` on the struct, so this is safe.
- **Files modified:**
  `crates/nono-cli/src/exec_strategy_windows/supervisor.rs`
- **Commit:** ac62932

### Planned but adjusted

- The plan asked that the Windows no-console test detach the console
  and exercise the `CONIN$` open failure path directly. Under
  `cargo test` stdio is captured, so the `is_terminal()` guard fires
  before we reach the `CONIN$` open; the test still proves the
  fail-secure contract (Denied with a reason mentioning the missing
  interactive device) and therefore satisfies the plan's `OR` assertion.
- `SupervisorConfig.approval_backend` was promoted from
  `&'a dyn ApprovalBackend` to `Arc<dyn ApprovalBackend + Send + Sync>`
  directly rather than adding a second field. Only one call site
  (`supervised_runtime.rs`) had to change, and this avoids a confusing
  API where two different approval fields are present on the same
  config.

### Auth gates

None.

## Deferred Issues

Four pre-existing Windows host test failures observed during the full
`cargo test -p nono-cli --bin nono` run. All four are present on the
base commit before any plan 11-02 changes (verified by `git stash` +
re-run). Logged to
`.planning/phases/11-runtime-capability-expansion/deferred-items.md`.
They are orthogonal to runtime capability expansion and out of scope
per GSD scope rules:

1. `query_ext::tests::test_query_path_sensitive_policy_includes_policy_source`
2. `capability_ext::tests::test_from_profile_allow_file_rejects_directory_when_exact_dir_unsupported`
3. `capability_ext::tests::test_from_profile_filesystem_read_accepts_file_paths`
4. `profile::builtin::tests::test_all_profiles_signal_mode_resolves`

## Human verification on a Windows host

Not performed in this execution. The Windows host is where this
executor is running, but a true end-to-end supervised run that
spawns a child, posts a `RequestCapability` to the capability pipe,
and observes an interactive `[nono] Grant access? [y/N]` prompt on
the supervisor's console requires a running child that imports the
`nono-cli` library bindings and writes to the rendezvous pipe. Adding
this harness is out of scope for plan 11-02 (it is an integration-test
level artifact). The functional contract is covered by the existing
unit tests plus the new `capability_handler_tests` suite: any live
request with a valid token now flows through `TerminalApproval`, and
`TerminalApproval` is proven to fail-secure when no console is
attached.

SC #2 ("supervisor presents the request to the user for interactive
approval; no capability is granted silently") is structurally
satisfied: the capability pipe thread holds an
`Arc<TerminalApproval>` and passes it to
`handle_windows_supervisor_message`, which is proven by test to call
`approval_backend.request_capability()` exactly once per valid-token
request. Grep audits confirm no alternative approval path exists in
the Windows capability pipe thread.

## Self-Check

Created files:

```
[ -f .planning/phases/11-runtime-capability-expansion/11-02-SUMMARY.md ]    → FOUND
[ -f .planning/phases/11-runtime-capability-expansion/deferred-items.md ]   → FOUND
```

Commits on `windows-squash`:

```
git log --oneline -3
ac62932 feat(11-02): wire TerminalApproval into Windows capability pipe server
8b82609 feat(11-02): add Windows CONIN$ branch to TerminalApproval
8453bfa docs(11): add research and plans for runtime capability expansion
```

Acceptance grep matches all satisfied (see Verification section above).

## Self-Check: PASSED
