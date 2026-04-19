---
compared: upstream v0.36.0 (468ae00a41abc7871284dfe92f4f1582ec64704a) vs windows-squash HEAD (8f5927c)
date: 2026-04-19
status: complete
---

# Functional differences: upstream nono v0.36.0 vs Windows fork

## Summary (TL;DR)

The headline divergence is **Windows enforcement**: upstream v0.36.0 ships no
native Windows backend (its README states "Native Windows support in planning"
and the `crates/nono/src/sandbox/` tree contains only `linux.rs` and
`macos.rs`), while the fork adds a full Windows story — a 2,427-line
`sandbox/windows.rs`, a separate `nono-wfp-service` binary (~1,800 lines) for
kernel-level WFP activation, a Windows supervisor, restricted-token + Job
Object exec strategy, ConPTY pty proxy, ETW-based learn, and named-pipe
supervisor IPC (`socket_windows.rs`, 972 lines). The top-level CLI command
list is nearly identical (18 commands match by name), but the fork adds
resource-limit flags to `run` (`--cpu-percent`, `--memory`, `--timeout`,
`--max-processes`, all Phase 16 / v2.1) and reshapes `prune` (duration-suffix
`--older-than`, new `--all-exited`, auto-sweep on `ps`, Phase 19). The fork
branched from upstream around **v0.30.1** — crate versions are still pinned at
0.30.1 while upstream has shipped 0.31→0.36, so the fork is missing roughly
six upstream releases' worth of Unix-side features (`--allow-gpu`, mTLS,
`file://` + `keyring://` credential URIs, GitLab trust tokens, several macOS
Seatbelt fixes). `rust-version` is 1.77 (fork) vs 1.74 (upstream 0.36).

## Upstream v0.36.0 baseline

**Identity.** Tag commit `468ae00` "chore: release v0.36.0", authored by Luke
Hinds 2026-04-15. 255 tracked files. Workspace crates: `nono`, `nono-cli`,
`nono-proxy`, `bindings/c` — same four members the fork has.

**Platform claims.** From `README.md` at v0.36.0 (line ~95):

> **Platform support:** macOS, Linux, and WSL2.

and in the Key Features table:

> **Cross-platform** | Support for macOS, Linux, and WSL2. Native Windows
> support in planning.

**Sandbox backends present.** `git ls-tree -r --name-only v0.36.0 -- crates/nono/src/sandbox`:

```
crates/nono/src/sandbox/linux.rs
crates/nono/src/sandbox/macos.rs
crates/nono/src/sandbox/mod.rs
```

No `windows.rs`. `mod.rs` header states explicitly: "Linux: Landlock LSM /
macOS: Seatbelt sandbox" (see `crates/nono/src/sandbox/mod.rs:3-5` at v0.36.0).

**CLI commands (18 top-level).** From `crates/nono-cli/src/cli.rs` at v0.36.0
around lines 91–473:

`setup`, `run`, `shell`, `wrap`, `learn`, `why`, `rollback`, `audit`, `trust`,
`ps`, `stop`, `detach`, `attach`, `logs`, `inspect`, `prune`, `policy`,
`profile`, plus an internal `open-url-helper`. Session lifecycle
(`ps/stop/detach/attach/logs/inspect/prune`) is upstream-owned — not a fork
addition.

**Key features shipped at 0.36** (CHANGELOG sections visible at
`git show v0.36.0:CHANGELOG.md`):

- `keyring://service/account` credential URI scheme (0.36)
- `?decode=go-keyring` URI parameter (0.36)
- GitLab ID tokens for trust signing, proxy artifact strip (0.35)
- macOS auto-enable claude launch services + keychain refinements (0.34)
- mTLS client certificates, `file://` credentials, `tls_ca` (0.32/0.33)
- `--allow-gpu` flag, GPU access on Apple Silicon + Linux + WSL2 (0.31/0.33)
- Deprecated startup-only command blocking (0.33)

**Supervisor IPC.** `crates/nono/src/supervisor/` at v0.36.0 has
`mod.rs`, `socket.rs`, `types.rs` — Unix-domain-socket only.

**Core lib size signals.** `crates/nono/src/keystore.rs` = 2,901 lines;
`crates/nono-cli/src/sandbox_prepare.rs` = 1,585 lines; `session.rs` = 992;
`setup.rs` = 569.

## Windows fork additions (absent in upstream 0.36)

| Area | What the fork adds | Phase | File(s) |
|------|--------------------|-------|---------|
| Windows sandbox backend | Full `apply()` implementation — directory read / read-write grants, blocked-network mode, port-level WFP filter (connect / bind / localhost), Job Object signal/process/ipc isolation enforcement | Phases 1, 3, 6, 9 | `crates/nono/src/sandbox/windows.rs` (2,427 lines) |
| WFP service binary | User-mode Windows service that installs/tears-down WFP filters via named-pipe control plane; Event Log integration for orphan sweep | Phase 3, 6 | `crates/nono-cli/src/bin/nono-wfp-service.rs` (1,807 lines) + `windows_wfp_contract.rs` |
| Windows exec strategy | Restricted token + Job Object launcher with CreateProcessAsUser, handle brokering, supervisor IPC | Phases 1, 6, 11, 14, 15 | `crates/nono-cli/src/exec_strategy_windows/{mod.rs, launch.rs, network.rs, restricted_token.rs, supervisor.rs}` (~5,700 lines total) |
| Windows supervisor pipe | Named-pipe supervisor IPC mirroring the Unix-domain-socket design | Phase 11 | `crates/nono/src/supervisor/socket_windows.rs` (972 lines) |
| ConPTY pty proxy | Interactive shell/wrap backend using Windows Pseudo-Console | Phase 8 | `crates/nono-cli/src/pty_proxy_windows.rs` (133 lines) |
| ETW-based learn | Windows equivalent of strace/fs_usage tracing using ferrisetw + Microsoft-Windows-Kernel-File/Network providers | Phase 10 | `crates/nono-cli/src/learn_windows.rs` (1,140 lines) |
| Windows session commands | `ps/stop/detach/attach/logs/inspect/prune` backend using named pipes + file-based session records | Phase 2, 15, 19 | `crates/nono-cli/src/session_commands_windows.rs` (879 lines) |
| Trust intercept for Windows | Windows-specific trust verification hooks | Phase 4 | `crates/nono-cli/src/trust_intercept_windows.rs` (44 lines) |
| Resource-limit flags | `--cpu-percent`, `--memory`, `--timeout`, `--max-processes` on `nono run`, kernel-enforced on Windows via `JOB_OBJECT_CPU_RATE_CONTROL_ENABLE`, `JobMemoryLimit`, supervisor timer + `TerminateJobObject`, `ActiveProcessLimit` | Phase 16 (RESL-01..04) | `crates/nono-cli/src/cli.rs:1453-1504` |
| Prune durations + `--all-exited` | `--older-than` accepts `30s/5m/1h/30d` suffix (upstream = raw integer days), new `--all-exited` flag, conflicts-with handling | Phase 19 (CLEAN-04) | `crates/nono-cli/src/cli.rs:2018-2039` |
| Auto-prune on `ps` | `nono ps` kicks off a background sweep when >100 stale session files exist | Phase 19 (CLEAN-04) | `crates/nono-cli/src/session_commands.rs`, `docs/session-retention.md` |
| `NONO_CAP_FILE` sandbox guard | Auto-prune is a structural no-op when invoked from inside a sandboxed agent | Phase 19 (T-19-04-07) | `docs/session-retention.md`, fork session_commands |
| WFP port/network integration tests | Windows-specific test harness | Phase 9 | `crates/nono-cli/tests/wfp_port_integration.rs` (190 lines), `tests/learn_windows_integration.rs` (61 lines) |
| Session-retention docs | Explicit retention policy page | Phase 19 | `docs/session-retention.md` |
| Windows helper binaries | `windows-net-probe`, `test-connector` for WFP and handshake tests | Phase 6, 9 | `crates/nono-cli/src/bin/windows-net-probe.rs`, `test-connector.rs` |

**Windows-specific deps added to core lib Cargo.toml** (`crates/nono/Cargo.toml`):

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows-sys = { version = "0.59", features = [
  "Win32_Foundation", "Win32_Security", "Win32_Security_Authorization",
  "Win32_Storage_FileSystem", "Win32_System_IO", "Win32_System_Pipes",
  "Win32_System_SystemServices", "Win32_System_Threading",
  "Wdk_System_SystemServices", "Win32_System_SystemInformation",
] }
```

And for the CLI:

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows-service = "0.7"
windows-sys = { version = "0.59", features = [ ... WFP / Job / Event Log ... ] }
```

Upstream v0.36.0 has zero `target_os = "windows"` target blocks in either
Cargo.toml.

## Windows fork modifications (same surface, different behavior)

| Area | Upstream 0.36 behavior | Fork behavior | Rationale |
|------|------------------------|---------------|-----------|
| `nono prune --older-than` | `Option<u64>` raw integer days (`cli.rs:1614-1616` at v0.36.0) | `Option<Duration>` with required suffix: `30s/5m/1h/30d` (`cli.rs:2022-2027` fork) | Phase 19 CLEAN-04 — extend precision and disallow ambiguous integer inputs |
| `nono prune --all-exited` | Absent | New flag, `conflicts_with = "older_than"` (`cli.rs:2030-2032`) | Phase 19 CLEAN-04 — escape hatch for "just nuke everything exited" |
| `nono ps` | Pure read-only listing | Read-only listing + spawns background auto-prune when stale-file count exceeds threshold; suppressed inside sandboxed agents via `NONO_CAP_FILE` | Phase 19 — keeps the session dir from growing unbounded on Windows where retention is the user's problem |
| `nono run` flags | No resource-limit flags | Adds `--cpu-percent N`, `--memory SIZE`, `--timeout DURATION`, `--max-processes N`; on Unix these emit a "not enforced on this platform" warning | Phase 16 (RESL-01..04) — kernel-enforced on Windows via Job Object rate-control / memory / active-process limits + supervisor timer |
| `rollback_dest` help text | "By default, snapshots are stored in ~/.nono/rollbacks/" (`cli.rs:1112` at v0.36.0) | "By default, snapshots are stored in the platform's nono rollback directory" (`cli.rs:1411` fork) | Windows uses `%LOCALAPPDATA%\nono\rollbacks`, not `~/.nono/` — help text generalized |
| `rust-version` | workspace = `"1.74"` | workspace = `"1.77"` | Fork bumped in Phase 4 plan 02 to pull in safer windows-sys 0.59 service/WFP handle bindings |
| `sandbox_prepare.rs` | 1,585 lines (monolithic) | 452 lines (factored) | Large refactor during Windows bring-up — logic redistributed into `profile/`, `policy.rs`, `capability_ext.rs` |
| `keystore.rs` | 2,901 lines, supports `keyring://service/account` URI with `?decode=go-keyring` | 2,369 lines, does NOT recognize `keyring://` scheme or `?decode` query param | Fork forked at 0.30.1 before these 0.36 additions landed |
| `env://` / file:// / credential URIs | Full set including `keyring://` (0.36) and `tls_ca` + `file://` (0.33) in creds | Has `tls_ca` and `file://` (present in code) but missing `keyring://` URI + `?decode` | Some 0.33-era fixes were backported; 0.36 additions were not |
| Trust signing backends | Keyless (Sigstore), GitHub ID tokens, **GitLab ID tokens (0.35)** | No GitLab support | Forked before 0.35 |
| `nono run --allow-gpu` | Present (0.31 feature) with Apple Silicon / Linux / WSL2 support | Not found (`grep allow_gpu crates/nono-cli/src/cli.rs` → no matches) | Forked before 0.31 |

## Upstream 0.36 features the Windows fork lacks or stubs

The fork's crate versions are pinned at `0.30.1` (see
`crates/nono/Cargo.toml`, `crates/nono-cli/Cargo.toml`,
`crates/nono-proxy/Cargo.toml`), while upstream's are `0.36.0`. That means
the fork diverged from upstream around v0.30.1 and the following upstream
features shipped between **0.31 and 0.36** but are NOT integrated into
`windows-squash`:

| Area | Upstream release | Fork status | Notes |
|------|------------------|-------------|-------|
| `--allow-gpu` flag (macOS + Linux + WSL2 GPU passthrough) | 0.31–0.33 | Missing | No Windows analog needed, but the Unix story on this branch is behind |
| mTLS client certificate support (proxy upstream connections) | 0.32 | Uncertain — `tls_ca` field exists in code; full mTLS path not verified | Backport partial |
| `file://` credential URI | 0.33 | Uncertain — `file://` path code present in keystore/network_policy | Likely backported |
| `--allow-gpu` flag docs + `tls_ca` in network policy | 0.33 | Partial — `tls_ca` referenced | Partial backport |
| macOS keychain + Seatbelt refinements (deny Mach IPC services on modern macOS, allow atomic-write temp files) | 0.31, 0.33, 0.34 | Missing | macOS users on the fork will see staleness |
| Auto-enable Claude launch services on macOS | 0.34 | Missing | macOS-specific |
| GPU NVIDIA procfs grants, nvidia-uvm-tools in device allowlist | 0.34 | Missing | Linux GPU |
| GitLab ID tokens for trust signing | 0.35 | Missing | Trust command |
| Strip proxy artifacts / fix upstream connection handling (proxy) | 0.35 | Missing | Proxy robustness |
| `keyring://service/account` URI with custom service names | 0.36 | Missing | Credential ergonomics |
| `?decode=go-keyring` URI parameter | 0.36 | Missing | Credential compat |
| Proxy CONNECT log severity adjustment | 0.36 | Missing | Minor logging |

The fork does NOT drop any upstream-0.30 feature that was present at fork
time; all removals observed (e.g. `command_blocking_deprecation.rs` is
present in v0.36 but absent from fork's tree) are either drift from the
fork's pre-0.30 base or intentional refactors, not feature deletions.
Specifically `command_blocking_deprecation.rs` exists at v0.36.0 (190 lines)
and does not exist in the fork — this is because the upstream 0.33 deprecation
of startup-only command blocking never arrived in the fork.

## Platform support matrix

| Feature | Upstream 0.36 Linux | Upstream 0.36 macOS | Upstream 0.36 Windows | Fork Windows (windows-squash) |
|---------|---------------------|---------------------|-----------------------|-------------------------------|
| Sandbox backend | Landlock (ABI v1–v5) | Seatbelt profile | **Not supported** (README: "in planning"; no `sandbox/windows.rs`) | Restricted token + Job Object + WFP kernel filter (`sandbox/windows.rs`, 2,427 lines) |
| Network filter | Proxy + seccomp-notify + Landlock v4+ | Proxy + Seatbelt network rules | Not supported | Proxy + WFP kernel filter (connect / bind / localhost ports) via `nono-wfp-service` |
| Supervisor IPC | Unix domain sockets (`socket.rs`) | Unix domain sockets | Not supported | Named pipes (`socket_windows.rs`, 972 lines) |
| `nono shell` / `nono wrap` PTY | Unix PTY (`pty_proxy.rs`) | Unix PTY | Not supported | ConPTY via `pty_proxy_windows.rs` on Windows 10 17763+ |
| `nono learn` | strace | fs_usage + nettop | Not supported | ETW via ferrisetw, kernel-file + network providers |
| `nono ps / stop / detach / attach / logs / inspect / prune` | Present | Present | Not supported | Present (`session_commands_windows.rs`) + auto-prune on `ps` |
| Rollback snapshots | Full | Full | Not supported | Full (shared with Unix via `crates/nono/src/undo/`) |
| Resource limits (`--cpu-percent` / `--memory` / `--timeout` / `--max-processes`) | Flag absent in upstream; fork warns "not enforced" on Unix | Same | Not supported | Kernel-enforced via Job Object rate-control / `JobMemoryLimit` / `ActiveProcessLimit` / supervisor timer |
| Runtime capability elevation (`--capability-elevation`) | seccomp-notify + approval prompts | Present | Not supported | Named-pipe brokered prompts via Phase 11 pipe (extensions paused on Windows — see `sandbox/windows.rs` refusing `extensions_enabled`) |
| `--allow-gpu` | Present (0.31+, WSL2 in 0.33) | Present (Apple Silicon) | Not supported | Missing (fork forked before 0.31) |
| `keyring://` credential URI | Present (0.36) | Present | n/a | Missing (0.36 feature) |

## Commits that drove the divergence

Fork-only commits since v0.36.0: **163 non-merge commits** on this branch.
Selected phase narrative from `.planning/ROADMAP.md`:

- **Phase 1–4 (v1.0 Windows Alpha, shipped 2026-03-31):** Control foundation,
  persistent detach/attach sessions, WFP integration, state integrity +
  packaging.
- **Phase 5:** Windows detach readiness fix.
- **Phase 6:** WFP enforcement activation (kernel-level block mode).
- **Phase 7:** Quick wins.
- **Phase 8:** ConPTY shell (`pty_proxy_windows.rs`).
- **Phase 9:** WFP port-level + proxy filtering.
- **Phase 10:** ETW-based `nono learn` (`learn_windows.rs` + ferrisetw audit).
- **Phase 11:** Runtime capability expansion — Windows named-pipe broker
  (`socket_windows.rs`).
- **Phase 12:** Milestone bookkeeping cleanup.
- **Phase 13–15:** v2.0 UAT pass + detached-console ConPTY carry-forward fix.
- **Phase 16 (v2.1, shipped 2026-04-18):** Resource limits
  (`--cpu-percent/--memory/--timeout/--max-processes`).
- **Phase 17 ATCH-01, Phase 18 AIPC-01:** Attach-streaming and extended IPC
  (still pending per ROADMAP).
- **Phase 19 (just shipped today, 2026-04-18):** Cleanup — format drift fix
  (CLEAN-01), 5 Windows test-flake restorations including UNC-path fix in
  `query_path` (CLEAN-02), WIP triage (CLEAN-03, 10 items), session retention
  + prune duration-format + auto-prune on `ps` + `docs/session-retention.md`
  (CLEAN-04).

Top fork HEAD commits (from `git log v0.36.0..HEAD --oneline --no-merges`):

```
8f5927c docs(19): close phase 19 — all 4 plans complete, verifier passed 25/25
292c2e2 docs(19-04): complete CLEAN-04 plan — session retention + prune + auto-trigger + one-shot cleanup
c3defb6 feat(19-CLEAN-04): auto-prune stale sessions at top of `nono ps` (D-15)
a71b2bf feat(19-CLEAN-04): extend `nono prune` with --all-exited and duration-format --older-than
18e9768 feat(19-CLEAN-04): add is_prunable predicate + tests (D-14)
```

## Upstream activity past 0.36 (context only)

Selected `git log v0.36.0..upstream/main --oneline --no-merges`:

```
8b5a2ff fix(hooks): invoke bash via env
832be3e Update README with Claude Code Logins regression note
8df95e6 chore: release v0.37.1
1b412a7 feat: implements environment variables filtering #688
03cbd42 fix(macos): emit specific-op seatbelt rules for keychain DB allows
4038f4f chore: release v0.37.0
97f7294 fix(claude-code): enable token refresh via .claude.json symlink
37ebaba feat(claude-code): pre-create claude config lock directory
c1bc439 fix(profiles): prevent infinite recursion in profile extends check
f9623dc fix(sandbox): support claude-code profile extensions and simplify config
8876d89 chore: upgrade rustls-webpki to 0.103.12 to fix RUSTSEC-2026-0098 and -0099
98460a0 fix(sandbox): allow Unix domain socket connections in restricted network modes
9e24ce1 fix(learn): print profile JSON as fallback when save fails
```

Notable: **RUSTSEC-2026-0098/0099 rustls-webpki fix** (already cut in 0.37.0)
and **environment-variable filtering feature** (0.37.0) are both missing from
the fork. Neither touches the Windows surface — they'd be clean Unix-side
merge candidates. No post-0.36 upstream commit introduces anything named
"windows" in the subject line; upstream still has not shipped a Windows
backend.

## Notable caveats / open questions

1. **Partial backports are ambiguous.** `tls_ca` and some `file://` credential
   support are visible in fork code (`network_policy.rs:237-238`). Whether
   those are clean forward-ports of upstream 0.33 or independent reimplementations
   was not fully traced — extended investigation required if the user needs
   credential-surface parity.
2. **`nono-cli/Cargo.toml` version = `0.30.1`.** This is the clearest numerical
   signal of where the fork branched. The fork may want to either (a) rebase
   forward to 0.36 to absorb six minor releases, or (b) rename the fork
   package to diverge cleanly. Not a blocker to usability.
3. **Capability-elevation on Windows.** The Windows sandbox explicitly rejects
   `extensions_enabled` (`crates/nono/src/sandbox/windows.rs:76-79`) despite
   Phase 11 shipping the pipe infrastructure — Windows elevation is gated
   pending Phase 18 AIPC-01.
4. **`rustls-webpki` security upgrade missed.** Upstream 0.37 absorbs
   RUSTSEC-2026-0098/0099; the fork's `Cargo.lock` likely still has the
   vulnerable versions (not verified in this report). Worth a quick scan.
5. **macOS drift.** The fork has 62 lines of diff in `crates/nono/src/sandbox/macos.rs`
   vs 0.36 but is also missing 0.31–0.34 macOS Seatbelt refinements (Mach IPC
   denies, atomic-write temp file allow, keychain DB specific-op rules).
   Users running the fork on macOS get a somewhat older Seatbelt profile
   than upstream 0.36 users.
6. **Upstream has NO native Windows story.** README at v0.36.0: "Native
   Windows support in planning." Upstream/main as of this check has not
   changed that — the fork is genuinely filling a gap, not racing a parallel
   implementation. An upstream maintainer merging this work would inherit
   the entire Windows backend more or less as-is.
7. **CLI parity is higher than it looks.** All 18 top-level commands exist
   on both sides with the same names. Functional deltas are concentrated in
   (a) which commands are backed by a real implementation on Windows, and
   (b) the Phase 16 / Phase 19 flag additions to `run` and `prune`.
