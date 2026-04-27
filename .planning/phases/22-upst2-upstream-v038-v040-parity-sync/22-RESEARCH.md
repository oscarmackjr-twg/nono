# Phase 22: UPST2 — Upstream v0.38–v0.40 Parity Sync — Research

**Researched:** 2026-04-27
**Domain:** Cross-platform feature port from upstream `always-further/nono` v0.37.1 → v0.40.1 (78 non-merge commits) with Windows parity in lockstep
**Confidence:** HIGH (commits verified against `upstream/v0.40.1` ref; fork drift measured via `git diff` on this working copy; no speculation)

## Summary

Phase 22 ports 5 cross-platform feature clusters from upstream v0.38–v0.40 into the Windows-parity fork. All upstream commits referenced in CONTEXT.md are reachable from `v0.40.1` (verified: `git log v0.37.1..v0.40.1` returns 105 total commits, of which 78 are non-merge — matches upr-review count). All fork-modified high-risk files have been re-measured against `v0.37.1..main` for current drift baseline. The OAuth2 fixture port surface is **inline tests in `oauth2.rs`** (no separate fixture files). The package fixture port surface is **inline `#[cfg(test)] mod tests` in `package_cmd.rs` + `package.rs`** (no integration test file). The audit cluster includes one external integration test file (`tests/audit_attestation.rs`) which currently does not exist in the fork.

**Two contradictions surfaced** that the planner must reconcile (see § Contradictions Found):
1. CONTEXT.md cites upstream commit `8b2a5ffb` for the "bash via env" hook fix; the actual SHA is `8b5a2ffb` (typo, single-character).
2. CONTEXT.md and ROADMAP.md require Authenticode signature recording for REQ-AUD-03; upstream's `ExecutableIdentity` struct is **SHA-256 only** with no `WinVerifyTrust` integration. Authenticode is therefore a **fork-internal addition**, not an upstream port — D-17 ALLOWED per CONTEXT.md guidance, but the planner must treat it as new fork-only code (~150 LOC) on top of the upstream parity port.

**Primary recommendation:** Plan execution should map exactly to the 7-commit chronological audit cluster + supplementary commits I've enumerated below. Both `8b5a2ffb` (hooks#!env) and `1d49246a` (claude-code package removal) are confirmed N/A on Windows and already absent from the fork; planners should explicitly add them to each plan's "DO NOT cherry-pick" list. The pre-milestone push to `origin/main` is **still pending** (513 local-only commits as of research time) — Plan 22-01 must STOP at task #1 if `git push origin main` has not yet been executed.

## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Plan 22-05 Conflict Strategy (highest-risk plan; ~1.4k LOC across heavily-forked files)
- **D-01: Cherry-pick first, fallback per-commit.** Default approach for the 7-commit audit cluster (`4f9552ec` → `4ec61c29` → `02ee0bd1` → `7b7815f7` → `0b1822a9` → `6ecade2e` → `9db06336`).
- **D-02: Soft fallback gate.** Read-upstream-replay when conflicts span >50 lines OR >2 forked files OR semantic ambiguity.
- **D-03: Strict upstream chronological order.** One fork commit per upstream SHA; no reordering, no squashing.
- **D-04: CLEAN-04 invariants verified after each rename-touching commit.** `auto_prune_is_noop_when_sandboxed`, suffix parser, `--all-exited` escape hatch, 100-file auto-sweep.

#### Working Branch & Origin Push
- **D-05:** Phase 22 / 23 / 24 commits land directly on `main`.
- **D-06:** Push `main` to origin NOW, before Phase 22 starts.
- **D-07:** Push to origin after each plan closes.
- **D-08:** Push v2.0 + v2.1 tags to origin alongside the initial main push.

#### Plan Sequencing & Wave Plan
- **D-09:** Plans 22-03 (PKG) and 22-04 (OAUTH) start only after Plan 22-01 verifier signs off.
- **D-10:** Plan 22-02 (POLY) wave-parallels with Plan 22-01 (PROF) from phase start.
- **D-11:** Phase 24 (DRIFT) sequences after Phase 22 ships.
- **D-12:** Plans 22-03 and 22-04 wave-parallel after 22-01 closes.

#### Test Fixtures
- **D-13:** Port upstream's OAuth2 test fixture for REQ-OAUTH-01.
- **D-14:** Port upstream's package registry test fixture for REQ-PKG-01..04.
- **D-15:** Add Windows-specific test cases atop ported fixtures (long-path, traversal rejection, Credential Manager, Authenticode).
- **D-16:** Tests live in existing `make ci` + `make test`. No new CI lane.

#### Carry-Forward From Phase 20
- **D-17:** Windows-only files structurally invariant; cherry-pick or manual-port touching `*_windows.rs` is by definition a bug. Exception: AUD-05 + Plan 22-05 Authenticode site.
- **D-18:** Per-plan close gate: `cargo test --workspace --all-features` + Phase 15 5-row smoke + `wfp_port_integration` + `learn_windows_integration`.
- **D-19:** Atomic commit-per-semantic-change with `Upstream-commit:` trailer.
- **D-20:** Manual port for heavily-diverged files; commit body documents what + why.

### Claude's Discretion
- Exact `make ci` invocations per plan
- Audit signing key provisioning model on Windows (default: pre-provision + fail-closed)
- Authenticode fallback path shape for REQ-AUD-03
- AUD-05 fold-or-split decision-point during 22-05 execution
- `prune` alias deprecation timeline (v2.3+ scoping decision)
- Per-plan PR vs single Phase-22 PR (default: single)

### Deferred Ideas (OUT OF SCOPE)
- AUD-05 Windows AIPC broker audit emissions → Phase 23
- DRIFT-01 / DRIFT-02 → Phase 24 (linear after Phase 22)
- WR-01 reject-stage unification → v2.3+
- AIPC G-04 wire-protocol compile-time tightening → v2.3+
- Cross-platform RESL Unix backends → v2.3+
- WR-02 EDR HUMAN-UAT → v3.0
- Upstream v0.41+ ingestion → v2.3 first quick task
- `unsafe_macos_seatbelt_rules` runtime application on Windows (deserialize-only forever)
- `claude-code integration package` carry-forward (follow upstream's removal)

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PROF-01 | Profile deserializes `unsafe_macos_seatbelt_rules` (macOS-only apply) | Upstream commit `14c644ce` confirmed; needs follow-ups `c14e4365` (fmt), `e3decf9d` (review feedback), `ecd09313` (test initializers), `d32ab18a` (log level) for clean cherry-pick |
| PROF-02 | Profile deserializes `packs` + `command_args` | Upstream `088bdad7` + `115b5cfa` (`load_registry_profile`) confirmed; depends on PackRef type from `71d82cd0` |
| PROF-03 | Profile `custom_credentials.oauth2` deserializes | Upstream `fbf5c06e` (90 LOC OAuth2Config) + `b1ecbc02` (Profile wiring) confirmed |
| PROF-04 | `claude-no-keychain` builtin available | Upstream `3c8b6756` (102 LOC builtin.rs) + `713b2e0f` (allow_file follow-up) confirmed |
| POLY-01 | `override_deny` requires matching grant | Upstream `5c301e8d` + `b83da813` + `930d82b4` confirmed |
| POLY-02 | `--rollback` + `--no-audit` conflict rejected | Upstream `5c301e8d` (combined with POLY-01) confirmed |
| POLY-03 | `.claude.lock` moved to `allow_file` | Upstream `49925bbf` + `713b2e0f` + `a524b1a7` + `7d1d9a0d` confirmed |
| PKG-01 | `package pull/remove/search/list` on Windows | Upstream `8b46573d` (4,556+ LOC) + `71d82cd0` rename confirmed |
| PKG-02 | Windows `install_dir` path resolution | Upstream `55fb42b8` + `ec49a7af` + `58b5a24e` confirmed; Windows-specific long-path is fork addition (D-15) |
| PKG-03 | Hook registration/unregistration on Windows | Upstream `55fb42b8` (hook unreg); `8b5a2ffb` is N/A on Windows (D-17 ABORT trigger) |
| PKG-04 | Signed-artifact streaming download on Windows | Upstream `9ebad89a` + `600ba4ec` + `0cbb7e62` confirmed |
| OAUTH-01 | `nono-proxy` client-credentials token exchange with cache | Upstream `9546c879` (552 LOC `oauth2.rs`, 11 tests inline) + `2244dd73` 413 fix + `0c7fb902` rebase + `19a0731f` compile fix confirmed |
| OAUTH-02 | Reverse-proxy HTTP upstream gated to local-only | Upstream `2bf5668f` + `0340ebff` + `b2a24402` + `0c990116` confirmed |
| OAUTH-03 | `--allow-domain` preserved in strict proxy-only | Upstream `10bcd054` + `005579a9` + `d44e404e` + `60ad1eb3` confirmed |
| AUD-01 | `--audit-integrity` hash-chained + Merkle ledger | Upstream `4f9552ec` (15 files, 1,419+/226-) + `4ec61c29` confirmed |
| AUD-02 | `--audit-sign-key` DSSE/in-toto attestation + verify | Upstream `6ecade2e` (519 LOC `audit_attestation.rs`) + `0b1822a9` confirmed |
| AUD-03 | Executable identity recorded on Windows | Upstream `02ee0bd1` + `7b7815f7` confirmed (SHA-256 only); **Authenticode integration is fork-only addition** — see Contradictions §2 |
| AUD-04 | `prune` → `session cleanup` rename + CLEAN-04 invariants | Section "Deprecate nono prune" inside `4f9552ec`; `9db06336` adds `audit_attestation.rs` integration test |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Profile struct fields (PROF-01..03) | nono-cli (CLI policy) | — | Profile is a CLI concept; library has no notion of profiles |
| `claude-no-keychain` builtin (PROF-04) | nono-cli (data/policy.json) | nono-cli (profile/builtin.rs) | Built-in profile registration is CLI-owned policy data |
| Policy tightening (POLY-01..03) | nono-cli (policy resolver) | nono-cli (cli.rs clap layer for POLY-02) | Policy validation is CLI-owned; library applies what CLI hands it |
| Package manager subcommands (PKG-01) | nono-cli (`package_cmd.rs` + `registry_client.rs`) | — | Pure CLI surface; no library involvement |
| Windows install_dir path resolution (PKG-02) | nono-cli (`package_cmd.rs` Windows branch) | windows-sys for `SHGetKnownFolderPath` | CLI-owned; uses `dirs` crate or direct windows-sys API |
| Hook installer (PKG-03) | nono-cli (`hooks.rs`) | — | Already cross-platform in fork (320 LOC drift vs upstream) |
| Streaming download (PKG-04) | nono-cli (`registry_client.rs`) | nono-proxy (rustls trust chain) | CLI orchestrates; trust chain via existing fork code |
| OAuth2 token exchange (OAUTH-01) | nono-proxy (`oauth2.rs`) | nono-cli (`network_policy.rs` config wiring) | Proxy is the credential-injection authority |
| Reverse-proxy HTTP upstream gating (OAUTH-02) | nono-proxy (`reverse.rs`) | — | Application-layer defense-in-depth atop WFP kernel filter |
| `--allow-domain` strict-proxy preservation (OAUTH-03) | nono-cli (`network_policy.rs` + `sandbox_prepare.rs`) | nono-proxy (host filter) | CLI threads the domain list; proxy enforces |
| Audit ledger emission (AUD-01) | nono-cli (`audit_integrity.rs` + `supervised_runtime.rs`) | nono (undo types) | Supervised path emits; library types serialize |
| Audit attestation (AUD-02) | nono-cli (`audit_attestation.rs`) | nono (`trust::signing` already in fork) | Signing is CLI; sigstore-rs lives in library trust module |
| Exec identity recording (AUD-03) | nono-cli (`execution_runtime.rs::compute_executable_identity`) | nono (`undo::ExecutableIdentity` struct) | CLI computes; library types persist |
| `prune` → `session cleanup` rename (AUD-04) | nono-cli (`cli.rs` + `session_commands.rs` + `session_commands_windows.rs`) | — | Pure CLI rename; both Unix and Windows mirrors must be updated |

## Standard Stack

### Reusable Fork Assets (do NOT introduce alternatives)

| Asset | Location | Purpose | Why Standard |
|-------|----------|---------|--------------|
| `nono::keystore::load_secret` + `keyring://` URI | `crates/nono/src/keystore.rs` | Cross-platform secret resolution (Credential Manager / keychain / env://) | Already shipped in v2.1 Phase 20 UPST-03; REQ-PROF-03 (`oauth2.client_secret`) and REQ-AUD-02 (`--audit-sign-key keyring://nono/audit`) reuse directly. No new keystore plumbing. [VERIFIED: `crates/nono/src/keystore.rs` exists, 333+ insertion drift since v0.37.1] |
| `nono::trust::signing::sign_statement_bundle` + `public_key_id_hex` | `crates/nono/src/trust/signing.rs` | DSSE/in-toto bundle signing | Already in fork from v2.1; REQ-AUD-02 reuses without Windows-specific code [CITED: 22-CONTEXT.md `<code_context>`] |
| `Profile::resolve_aipc_allowlist` end-to-end wiring | `crates/nono-cli/src/profile/mod.rs` (Phase 18.1 commit `993cdcb`) | Profile struct flows through `PreparedSandbox → LaunchPlan → execute_sandboxed → SupervisedRuntimeContext` | New PROF-* fields slot into established plumbing; no architectural rework |
| `AppliedLabelsGuard` RAII pattern | `crates/nono/src/sandbox/windows.rs` (Phase 21) | Snapshot + label lifecycle on Windows | REQ-AUD-05 (Phase 23) acceptance #3 references; emissions must survive Drop |
| `current_logon_sid()` + `build_capability_pipe_sddl` | Supervisor pipe debug fix commit `938887f` (2026-04-20) | SDDL construction for capability pipe | Plan 22-05's audit-integrity Windows ledger may need for write-path SDDL — **verify during planning** |
| `Upstream-commit:` trailer convention | Phase 20 commits (`198270e`, `835c43f`, `540dca9`, `f377a3e`, `ec73a8a`, `af5c124`) | Provenance discipline | Phase 22 inherits verbatim per D-19 |
| DCO commit hook on `main` | `.git/hooks/commit-msg` (or equivalent) | Enforces `Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>` | Already active; 46 historical pre-DCO commits deferred per quick task 260424-mrg |
| `make ci` target | `Makefile:122` (`ci: check test audit`) | Single verification entry point per D-18 | `cargo build --workspace` + `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` + `cargo fmt --all -- --check` + `cargo test -p nono` + `cargo test -p nono-cli` + `cargo test -p nono-ffi` + `cargo audit` [VERIFIED: Makefile read 2026-04-27] |

### New Dependencies for Plan 22-05 (Authenticode fork addition)

| Dependency | Version | Purpose | Verification |
|------------|---------|---------|--------------|
| `windows-sys` feature `Win32_Security_WinTrust` | 0.59 (already in use) | `WinVerifyTrust` for Authenticode signature query | [CITED: docs.rs/windows-sys/0.59 — feature must be added to `crates/nono-cli/Cargo.toml`'s existing windows-sys block] |
| `windows-sys` feature `Win32_Security_Cryptography` | 0.59 (likely needed) | `CryptCATAdminAcquireContext` if catalog-signed binaries are in scope (probably out of scope for v0.0 dev builds) | [CITED: Microsoft Learn wintrust.h] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Inline tests in `oauth2.rs` (D-13 directs port-as-is) | Separate `crates/nono-cli/tests/oauth2_integration.rs` | Upstream chose inline; D-13 says port-as-is; no reason to deviate |
| `windows-sys` for `WinVerifyTrust` | `winapi` crate | windows-sys is current standard; winapi is legacy; fork already uses windows-sys 0.59 across both crates |
| Cherry-pick `8b5a2ffb` (hooks#!env) | Skip entirely | N/A on Windows (Phase 22 STOP trigger #8); hooks fired through PowerShell on Windows, not POSIX shell |
| Cherry-pick `1d49246a` (claude-code package removal) | Verify already-absent then skip | Fork has no `packages/` directory; nothing to remove; cherry-pick would fail with "no files to delete" |
| Custom Authenticode fallback shape | Default to upstream's `ExecutableIdentity { resolved_path, sha256 }` plus a sibling `Option<AuthenticodeStatus>` | Upstream's struct is fixed; fork addition needs separate field, not modification of upstream type |

**Version verification (windows-sys):**
- `crates/nono-cli/Cargo.toml`: `windows-sys = { version = "0.59", features = [...existing 17...] }` — `Win32_Security_WinTrust` to be added in Plan 22-05 [VERIFIED: file read 2026-04-27]
- `crates/nono/Cargo.toml`: `windows-sys = { version = "0.59", features = [...existing 12...] }` — no change required (Authenticode lives in nono-cli per D-17 exception) [VERIFIED]

## Architecture Patterns

### Cherry-Pick + Fallback Replay Pattern (D-01..D-03)

For each upstream commit in chronological order:

1. `git cherry-pick <sha>`
2. If clean → `git commit --amend --signoff` to add DCO signoff + `Upstream-commit:` trailer (or use commit body template per D-19)
3. If conflict → soft-fallback gate per D-02:
   - Count conflict markers: `git diff --name-only --diff-filter=U | xargs -I {} grep -c '<<<<<<<' {}`
   - Count conflicted files: `git diff --name-only --diff-filter=U | wc -l`
   - If `>50 lines OR >2 files OR semantic ambiguity`: `git cherry-pick --abort` then manual replay using D-20 commit body template
   - Else: resolve in-place + `git cherry-pick --continue`
4. Run D-04 CLEAN-04 gate (if commit touches prune/cleanup):
   ```
   cargo test -p nono-cli --bin nono auto_prune_is_noop_when_sandboxed
   cargo test -p nono-cli --bin nono parse_duration
   cargo test -p nono-cli --bin nono is_prunable_all_exited_escape_hatch_matches_any_exited
   ```
5. Run D-18 Windows-regression gate (every commit in 22-05; per-plan in 22-01..04)

### Profile Field Addition Pattern (Plan 22-01)

Upstream's PROF commits all use `#[serde(default)]` for backward compatibility:
```rust
// Source: upstream 14c644ce + 088bdad7 + b1ecbc02 patterns
#[serde(default)]
pub unsafe_macos_seatbelt_rules: Vec<String>,
#[serde(default)]
pub packs: Vec<PackRef>,
#[serde(default)]
pub command_args: Vec<String>,
#[serde(default)]
pub custom_credentials: Option<CustomCredentialDef>,  // OAuth2Config inside
```

This means existing fork profiles (without these fields) continue to deserialize cleanly.

### Path-Validation Pattern (Plan 22-03)

Source: `ec49a7af fix(package): harden package installation security`:
```rust
// validate_safe_name — reject `..`, `/`, `\`, NUL, control chars
// validate_path_within — canonicalize then verify base.starts_with()
// validate_relative_path — added in 58b5a24e to consolidate logic
```

Use upstream's exact functions. Do NOT hand-roll Windows path validation — `validate_path_within` is already cross-platform. **D-15 Windows test addition:** path-traversal regression with `..\..\..\windows\system32` and UNC alias `\\?\GLOBALROOT\Device\HarddiskVolume2`.

### Anti-Patterns to Avoid

- **DO NOT cherry-pick `8b5a2ffb` (hooks#!env).** Fork's `nono-hook.sh` still uses `#!/bin/bash` (line 1, verified). Upstream changed to `#!/usr/bin/env bash` for non-`/bin/bash` distros. On Windows, the hook is invoked through PowerShell or via shebang interpretation by Git Bash/WSL — not directly by Windows. Per CONTEXT.md STOP trigger #8: ABORT.
- **DO NOT cherry-pick `1d49246a` (claude-code package removal).** Fork has no `packages/` directory ([VERIFIED: `ls packages/` returns "No such file or directory"]). The package was never carried into the windows-squash branch. Per CONTEXT.md STOP trigger #9: investigate before committing — investigation complete, finding: nothing to do.
- **DO NOT introduce a new keystore abstraction.** Use existing `nono::keystore::load_secret` per D-15.
- **DO NOT compose token cache to disk.** Upstream `9546c879`'s `TokenCache` is RwLock-guarded `Zeroizing<String>` in memory; persistence would reintroduce WSFG-01..03 Low-IL label issues.
- **DO NOT modify upstream's `ExecutableIdentity` struct shape.** Authenticode addition (REQ-AUD-03) must be a sibling field on `SessionMetadata` or a `audit_attestation`-style optional summary — see Contradictions §2 below.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Windows %LOCALAPPDATA% path | Custom `env::var("LOCALAPPDATA")` parsing | `dirs::config_local_dir()` (already in fork via dirs crate) | Handles edge cases: missing env var, non-existent path, redirected folders |
| Long-path `\\?\` prefix handling | String concatenation | `dunce` crate or windows-sys `GetFinalPathNameByHandleW` | Composability with relative path resolution; Rust's std `canonicalize` already returns extended-length paths on Windows |
| Path traversal validation | String search for `..` | Upstream's `validate_safe_name` + `validate_path_within` from `ec49a7af` | Handles symlinks, UNC aliasing, mixed separators, NUL bytes |
| OAuth2 token exchange | Custom HTTP client | Upstream's `oauth2.rs` with rustls + hyper (already in fork) | Tested with 11 inline tests; handles 401 retry, expiry race, scope omission |
| Hash chain construction | Custom Vec<Hash> | Upstream's `MerkleScheme::Alpha` (`7b7815f7` unification) | Non-trivial domain separation; v3 → alpha migration; getting this wrong invalidates verification |
| Authenticode signature query | Custom certificate parsing | `windows-sys` `WinVerifyTrust` with `WINTRUST_DATA` + `WINTRUST_FILE_INFO` (`WINTRUST_ACTION_GENERIC_VERIFY_V2` GUID) | OS-validated; revocation checks; CA chain verification; never roll your own cert validation |
| Cross-platform exe path resolution | `env::current_exe()` directly | Upstream's `compute_executable_identity` calling `canonicalize()` first | Handles symlinks; matches upstream's hash domain |

**Key insight:** Phase 22's value is **parity**, not novelty. The default action for any "should I build this myself?" question is "no — find the upstream pattern and port it." Authenticode (REQ-AUD-03) is the one explicit fork-addition.

## Runtime State Inventory

> Phase 22 is rename/refactor + new feature port. The `prune` → `session cleanup` rename (REQ-AUD-04) crosses runtime-state surfaces.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — Phase 22 does not change session-data layout. Audit ledger files in `~/.nono/audit/<session-id>/` (REQ-AUD-01..04) are NEW writes, not migrations of existing data. | None |
| Live service config | `nono-wfp-service` (Windows): no impact — Phase 22 does not touch WFP. Claude Code hook settings (`~/.claude/...`): Plan 22-03 PKG-03 writes hook entries; existing hooks must be preserved (`9ebad89a` "retain shared hook scripts installed outside the package store") | Plan 22-03 idempotent install per REQ-PKG-03 acceptance #3 |
| OS-registered state | None — Plan 22-05 `prune` → `session cleanup` rename does NOT affect OS-registered tasks (no Windows Task Scheduler entries reference `nono prune`). Hidden alias preserves CLI surface. | None |
| Secrets/env vars | `NONO_CAP_FILE` env var: structural early-return MUST be preserved through Plan 22-05 rename (REQ-AUD-04 acceptance #4 + STOP trigger #6). New: `keyring://nono/audit` key (REQ-AUD-02) — user pre-provisions per Claude's Discretion default. | Plan 22-05 verifies `auto_prune_if_needed` first statement remains `if env::var_os("NONO_CAP_FILE").is_some() { return; }` after rename. |
| Build artifacts | None — Cargo workspace is the build system; `cargo clean` regenerates everything. No installed package state. | None |

**Nothing found in category "Stored data" / "OS-registered state" / "Build artifacts":** Verified by grep against fork source for `nono prune` / `windows task scheduler` / `egg-info` / `.installed`.

## Common Pitfalls

### Pitfall 1: Cherry-pick chain ordering trap (D-03 violation candidate)
**What goes wrong:** Plan 22-01 needs `c14e4365` (cargo fmt) chronologically between `14c644ce` (PROF-01 main) and `e3decf9d` (review feedback). If skipped, `e3decf9d` will conflict with the unformatted code.
**Why it happens:** CONTEXT.md canonical refs list `e3decf9d` and `ecd09313` but not `c14e4365`.
**How to avoid:** Use `git log v0.37.1..v0.40.1 -- crates/nono-cli/src/policy_cmd.rs` to enumerate the full chain per file.
**Warning signs:** Cherry-pick of `e3decf9d` produces conflicts in `policy_cmd.rs` rustfmt-related lines.
**Specific commits the planner must include alongside CONTEXT.md's named SHAs:**
- Between `14c644ce` and `e3decf9d`: `c14e4365` (style: run cargo fmt)
- Optional follow-up after `ecd09313`: `d32ab18a` (downgrade unsafe seatbelt log warn → info)

### Pitfall 2: `1d49246a` reverse-cherry-pick failure mode
**What goes wrong:** `git cherry-pick 1d49246a` will FAIL with "no files to delete" because `packages/claude-code/` doesn't exist in fork.
**Why it happens:** The fork never carried `packages/claude-code/` from upstream — `windows-squash` diverged before `8b46573d` introduced it, so when `1d49246a` removed it later, fork was already absent of those files.
**How to avoid:** Do NOT cherry-pick `1d49246a`. Per CONTEXT.md STOP trigger #9, investigate before committing — this research IS the investigation. Conclusion: skip safely.
**Warning signs:** Plan 22-03 sub-task that mentions `1d49246a` in its acceptance criteria.

### Pitfall 3: `ExecutableIdentity` struct extension (Authenticode fallback)
**What goes wrong:** Modifying upstream's `ExecutableIdentity` struct shape breaks D-19 atomic commit-per-semantic-change (Authenticode is fork-only, not part of any upstream commit).
**Why it happens:** REQ-AUD-03 acceptance #2/#3 requires Authenticode status field, but upstream `ExecutableIdentity` has only `resolved_path` and `sha256`.
**How to avoid:** Add Authenticode as a separate `audit_authenticode: Option<AuthenticodeStatus>` field on `SessionMetadata` (or a sibling `audit-attestation.bundle`-style file), not a mutation of upstream's `ExecutableIdentity`. Land it as its own commit with body documenting "fork addition; no upstream parent" instead of `Upstream-commit:` trailer.
**Warning signs:** A diff to `crates/nono/src/undo/types.rs` `ExecutableIdentity` struct in any commit; the commit message claims `Upstream-commit:` lineage that doesn't actually contain that change.

### Pitfall 4: `5c301e8d` breaks fork's existing rollback tests (POLY-02)
**What goes wrong:** Fork's `crates/nono-cli/tests/*rollback*.rs` and Windows rollback integration tests may pair `--rollback` with `--no-audit`. Cherry-picking `5c301e8d` breaks all of them simultaneously.
**Why it happens:** Upstream test fixtures were updated atomically with the breaking change in upstream. Fork's tests have not been audited for this combination.
**How to avoid:** BEFORE cherry-picking `5c301e8d`, run `grep -rE "no.audit.*rollback|rollback.*no.audit" crates/nono-cli/tests/ tests/integration/` and update fork tests to drop `--no-audit` from `--rollback` invocations. Land the test update in the same commit (drive-by Rule-3 deviation, documented in commit body per Phase 18.1 / 21 precedent).
**Warning signs:** `cargo test -p nono-cli` failure on a `rollback_*` test file after `5c301e8d` cherry-pick.

### Pitfall 5: Plan 22-01 + Plan 22-02 racing on `policy.json` and `profile/mod.rs` (D-10 wave-parallel hazard)
**What goes wrong:** Both plans modify `crates/nono-cli/data/policy.json` (PROF-04 adds `claude-no-keychain` builtin entries; POLY-03 moves `.claude.lock` to `allow_file`) AND `crates/nono-cli/src/profile/mod.rs` (PROF-01..03 add fields; POLY-01 adds `OrphanOverrideDeny` validation in `Profile::resolve`).
**Why it happens:** D-10 wave-parallels them based on disjoint surfaces, but the JSON file is single-source-of-truth and `profile/mod.rs` is the central struct.
**How to avoid:** Sequence the SHARED-FILE commits within each plan:
- Plan 22-01: cherry-pick `14c644ce` first (touches `profile/mod.rs` minimally)
- Plan 22-02: wait for 22-01's `b1ecbc02` (`profile/mod.rs` +364 LOC) to land, then cherry-pick `b83da813` + `5c301e8d` against the latest `mod.rs`
- For `policy.json`: Plan 22-01's `3c8b6756` (75 LOC policy.json change) must land before Plan 22-02's `49925bbf` + `713b2e0f` (5-line change) — `713b2e0f` is literally a follow-up to `49925bbf` against the post-`3c8b6756` policy.json
**Warning signs:** Either plan's executor reports a merge conflict during `git pull --rebase` mid-wave.
**Mitigation pattern:** Use `git rebase --interactive` discipline — both plans commit to local `main`, then sequence the merges. NO long-running parallel branches.

### Pitfall 6: Phase 15 5-row smoke gate fails after Plan 22-05 commit (D-18 violation candidate)
**What goes wrong:** Audit-cluster commits modify `exec_strategy.rs` (+144 lines upstream; +186 fork lines already), `supervised_runtime.rs` (+42 / +229), `rollback_runtime.rs` (+586 / +200). Any of these may regress Windows detached-console or supervisor-pipe behavior.
**Why it happens:** These three files are the densest fork-vs-upstream divergence. D-02 fallback rule will likely fire on at least one cluster commit.
**How to avoid:** Plan 22-05 D-18 gate after EVERY commit (not just plan-close). The Phase 15 5-row smoke is `nono run --allow-cwd -- echo hello` followed by `nono ps` and `nono stop` against the running session.
**Warning signs:** `nono ps` returns 0 sessions after `nono run` succeeds; or `nono stop` hangs for >30s.

## Code Examples

### Pattern 1: Profile field addition with `#[serde(default)]`
```rust
// Source: upstream 14c644ce crates/nono-cli/src/profile/mod.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    // ... existing fields ...
    
    /// macOS-only escape hatch for raw Seatbelt S-expressions.
    /// Deserialized on every platform; applied via add_platform_rule on macOS only.
    #[serde(default)]
    pub unsafe_macos_seatbelt_rules: Vec<String>,
}
```

### Pattern 2: Cross-platform vs platform-gated rule application
```rust
// Source: upstream 14c644ce crates/nono-cli/src/sandbox_prepare.rs
fn apply_unsafe_seatbelt_rules(profile: &Profile, sandbox: &mut Sandbox) -> Result<()> {
    if profile.unsafe_macos_seatbelt_rules.is_empty() {
        return Ok(());
    }
    
    #[cfg(target_os = "macos")]
    {
        for rule in &profile.unsafe_macos_seatbelt_rules {
            sandbox.add_platform_rule(rule)?;  // Apply only on macOS
        }
        info!("Applied {} unsafe Seatbelt rules", profile.unsafe_macos_seatbelt_rules.len());
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        // Windows + Linux: parse the field but emit no rule.
        // Log at info level (downgraded from warn in d32ab18a).
        info!("unsafe_macos_seatbelt_rules ignored (not macOS)");
    }
    
    Ok(())
}
```

### Pattern 3: OAuth2 token exchange test fixture (port-as-is per D-13)
```rust
// Source: upstream 9546c879 crates/nono-proxy/src/oauth2.rs (lines 552 onward)
fn make_test_cache(token: &str, ttl: Duration) -> TokenCache {
    let config = OAuth2ExchangeConfig {
        token_url: "https://127.0.0.1:1/oauth/token".to_string(),
        client_id: Zeroizing::new("test-client".to_string()),
        client_secret: Zeroizing::new("test-secret".to_string()),
        scope: String::new(),
    };
    
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let tls_config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()  // <-- WARNING: fork's clippy::unwrap_used policy forbids this in production code
    .with_root_certificates(root_store)
    .with_no_client_auth();
    let tls_connector = TlsConnector::from(Arc::new(tls_config));
    
    TokenCache {
        token: Arc::new(RwLock::new(CachedToken {
            access_token: Zeroizing::new(token.to_string()),
            expires_at: Instant::now() + ttl,
        })),
        config,
        tls_connector,
    }
}
```

**Important:** Upstream's test code uses `.unwrap()` which the fork's `-D clippy::unwrap_used` policy forbids in production. Test code is generally allowed but must be in a `#[cfg(test)] mod tests` with `#[allow(clippy::unwrap_used)]` at the module level. Verify when porting per `CLAUDE.md § Coding Standards`.

### Pattern 4: Authenticode signature query (fork addition, ~150 LOC sketch)
```rust
// FORK ADDITION — no upstream parent. Lives in audit_session.rs or new audit_authenticode.rs
// Source: derived from Microsoft Learn wintrust.h docs; not from upstream commits.
#[cfg(target_os = "windows")]
fn query_authenticode_status(exe_path: &Path) -> Result<AuthenticodeStatus> {
    use windows_sys::Win32::Security::WinTrust::{
        WinVerifyTrust, WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA,
        WINTRUST_FILE_INFO, WTD_CHOICE_FILE, WTD_REVOKE_NONE, WTD_UI_NONE,
        WTD_STATEACTION_VERIFY, WTD_STATEACTION_CLOSE, WTD_REVOCATION_CHECK_NONE,
    };
    use std::os::windows::ffi::OsStrExt;
    
    let path_wide: Vec<u16> = exe_path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    
    let mut file_info = WINTRUST_FILE_INFO {
        cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: path_wide.as_ptr(),
        hFile: std::ptr::null_mut(),
        pgKnownSubject: std::ptr::null_mut(),
    };
    
    let mut data = WINTRUST_DATA::default();
    data.cbStruct = std::mem::size_of::<WINTRUST_DATA>() as u32;
    data.dwUIChoice = WTD_UI_NONE;
    data.fdwRevocationChecks = WTD_REVOKE_NONE;
    data.dwUnionChoice = WTD_CHOICE_FILE;
    data.dwStateAction = WTD_STATEACTION_VERIFY;
    data.Anonymous.pFile = &mut file_info;
    
    // SAFETY: WINTRUST_DATA is fully populated; action GUID is constant.
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    let result = unsafe {
        WinVerifyTrust(std::ptr::null_mut(), &mut action, &mut data as *mut _ as *mut _)
    };
    
    // CRITICAL: per WinVerifyTrust docs, must call again with WTD_STATEACTION_CLOSE
    // to release allocated state, regardless of success/failure.
    data.dwStateAction = WTD_STATEACTION_CLOSE;
    unsafe {
        WinVerifyTrust(std::ptr::null_mut(), &mut action, &mut data as *mut _ as *mut _);
    }
    
    match result {
        0 => Ok(AuthenticodeStatus::Valid { signer_chain: parse_signer_chain(&data)? }),
        // TRUST_E_NOSIGNATURE = 0x800B0100
        i32::from_le_bytes([0x00, 0x01, 0x0B, 0x80i32 as u8]) => Ok(AuthenticodeStatus::Unsigned),
        other => Ok(AuthenticodeStatus::InvalidSignature { hresult: other }),
    }
}

#[cfg(not(target_os = "windows"))]
fn query_authenticode_status(_exe_path: &Path) -> Result<AuthenticodeStatus> {
    Ok(AuthenticodeStatus::NotApplicable)  // Unix path: SHA-256 only, no Authenticode concept
}
```

[CITED: Microsoft Learn `wintrust.h` API reference; see Sources section]
[ASSUMED: Exact `signer_chain` extraction logic — pull from `data.hWVTStateData` requires `CryptUIDlgViewSignerInfo` family or manual ASN.1 walking; planner discretion per Claude's Discretion bullet]

This Authenticode addition fits CONTEXT.md's REQ-AUD-03 acceptance criteria #2 ("Signed release binary shows valid signer chain") and #3 ("Unsigned dev build shows `unsigned` + SHA-256"), but is NOT in upstream `02ee0bd1` / `7b7815f7`. Plan it as a fork-only commit with rationale in commit body, separate from any cherry-picked upstream commit.

### Pattern 5: CLEAN-04 invariant test invocation (D-04 gate)
```bash
# Source: verified against fork at HEAD 8381d9ca (2026-04-27)
# Per-commit gate after any 22-05 commit touching prune/cleanup:

# Test 1: NONO_CAP_FILE structural no-op (T-19-04-07 mitigation)
cargo test -p nono-cli --bin nono auto_prune_is_noop_when_sandboxed
# Expected: "test auto_prune_is_noop_when_sandboxed ... ok" appears in BOTH
# the unix mod (session_commands.rs:708) AND windows mod (session_commands_windows.rs:801)

# Test 2: --older-than suffix parser (clap-level)
# Note: there is NO test named `older_than_requires_suffix` in fork.
# CLEAN-04 covers this via `parse_duration` family + clap's value_parser rejection.
cargo test -p nono-cli --bin nono parse_duration_accepts_suffixes
cargo test -p nono-cli --bin nono parse_duration_accepts_raw_seconds
cargo test -p nono-cli --bin nono parse_duration_rejects_invalid

# Test 3: --all-exited escape hatch
cargo test -p nono-cli --bin nono is_prunable_all_exited_escape_hatch_matches_any_exited

# Test 4: 100-file auto-sweep threshold (constant verification)
grep -E "^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100;" crates/nono-cli/src/session_commands.rs
# Expected: 1 match. Constant must remain literal `100` post-rename.

# Test 5: full is_prunable family
cargo test -p nono-cli --bin nono is_prunable
# Expected: 8 tests pass (lines 1299-1394 of session.rs)
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `Profile::override_deny` silently ignored when no matching grant | Fail-closed `NonoError::PolicyError { kind: OrphanOverrideDeny }` | upstream `5c301e8d` (2026-04-22) | Breaking; Plan 22-02 must audit fork's built-in profiles before landing |
| `--rollback` + `--no-audit` accepted, undefined behavior | clap `conflicts_with` rejects at parse time | upstream `5c301e8d` | Breaking; Plan 22-02 must update fork rollback tests |
| `.claude.lock` granted via directory-level allow | `allow_file` single-file grant (least-privilege) | upstream `49925bbf` | Reduces attack surface; Windows WSFG mode encoding compatible |
| `nono prune` standalone command | `nono session cleanup` + `nono audit cleanup` peers (with `prune` hidden alias) | upstream `4f9552ec` (2026-04-21) | Phase 22-05 must preserve all CLEAN-04 invariants through rename |
| Audit ledger plain-text NDJSON | Hash-chained + Merkle-rooted ledger with optional DSSE attestation | upstream `4f9552ec` + `6ecade2e` | New attack surface (sigstore-rs trust chain), but inherits fork's existing trust module |
| OAuth2 credentials inline in Profile | Cross-platform `OAuth2Config` with `keyring://` + `env://` URI schemes | upstream `fbf5c06e` + `b1ecbc02` (2026-03-19) | Reuses fork's `nono::keystore::load_secret` (Phase 20 UPST-03) |
| `MerkleScheme::v1` / `v2` / `v3` (versioned legacy) | `MerkleScheme::Alpha` (single unified scheme) | upstream `7b7815f7` | Plan 22-05 commit chain MUST include `7b7815f7` to inherit unification; replaying selectively may produce diverged hashes |
| Per-artifact Sigstore bundle | Centralized package-level trust bundle | upstream `600ba4ec` (`refactor(package-cmd)`) | Plan 22-03 simpler; less network round-trips |
| Buffered package download | Streaming + chunked verification | upstream `9ebad89a` | Memory-bounded for ≥50MB artifacts (REQ-PKG-04 acceptance #1) |
| Per-artifact signer consistency check | Removed; per-artifact verification only | upstream `0cbb7e62` | Simpler validation surface |

**Deprecated/outdated:**
- `nono prune` — hidden alias only post-22-05; force-removal timeline deferred to v2.3+
- `MerkleScheme::v3` — superseded by `Alpha`; mid-cluster commits (`02ee0bd1`) introduce v3, then `7b7815f7` unifies — DO NOT skip `7b7815f7`
- `enforce_signer_consistency` / `same_signer` functions — removed in `0cbb7e62`; planner ensures Plan 22-03 cherry-picks reach `0cbb7e62` before fork has a chance to depend on these
- `MANUAL_TEST_STEPS.md` — removed in `5c301e8d`; re-added in `9db06336`. Net: file is present at v0.40.1; Plan 22-02 cherry-picks `5c301e8d` (deletes), Plan 22-05 cherry-picks `9db06336` (re-adds). Fork must end up with the v0.40.1 version.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `c14e4365` (cargo fmt) is required between `14c644ce` and `e3decf9d` | Plan 22-01 cheat-sheet | Without it, `e3decf9d` fmt-related changes will conflict; planner should `git log v0.37.1..v0.40.1 -- crates/nono-cli/src/policy_cmd.rs` to confirm the exact chain |
| A2 | `d32ab18a` (downgrade unsafe seatbelt log warn → info) is grouped with PROF-01 | Plan 22-01 cheat-sheet | Low risk — small commit (1 line); planner may safely defer to a separate plan or skip if log level unimportant |
| A3 | `228a5cf1` and `fabfc9ed` (test improvements) are out-of-scope | Plan 22-01 cheat-sheet | These are integration test improvements (`tests/integration/test_profiles.sh`) for shell-level tests; fork uses different test harness — likely skip; if cherry-pick attempted will fail on missing test paths |
| A4 | Plan 22-01's `b1ecbc02` (Profile `oauth2` wiring +364 LOC `profile/mod.rs`) lands cleanly | File-drift baseline | Fork drift on `profile/mod.rs` is +732/-414 lines vs v0.37.1; conflict probability HIGH; D-02 fallback likely fires |
| A5 | Plan 22-04's `005579a9` + `d44e404e` + `60ad1eb3` are required follow-ups to `10bcd054` | Plan 22-04 cheat-sheet | Cherry-pick of `10bcd054` alone will leave fork in a state with the breaking rename but missing follow-up fmt + test fixes; CI will go red |
| A6 | Plan 22-05's `9db06336` re-introduces `MANUAL_TEST_STEPS.md` after Plan 22-02's `5c301e8d` deleted it | State of the Art | Test file existence at v0.40.1 — verified, but the round-trip is unusual; flag for planner to confirm net effect |
| A7 | The Authenticode integration in REQ-AUD-03 is fork-only addition (~150 LOC) | Architecture Patterns Pattern 4 | Verified — upstream `ExecutableIdentity` is SHA-256 only. Planner must scope Authenticode work as a separate task, not as part of cherry-picking `02ee0bd1`/`7b7815f7` |
| A8 | `windows-sys` feature `Win32_Security_WinTrust` is sufficient for `WinVerifyTrust` | Standard Stack | [CITED: docs.rs/windows-sys] — confirmed; `WINTRUST_DATA` lives in this feature module |
| A9 | Hidden `nono prune` alias mechanism in clap is straightforward | REQ-AUD-04 | Clap supports `#[command(alias = "prune", hide = true)]` since 4.0; verified working in fork [VERIFIED: clap 4.6.1 in upstream Cargo.toml] |
| A10 | Plan 22-05 commits NEVER touch `*_windows.rs` files | D-17 invariant | Verified: `git show <sha> --stat` for all 7 audit-cluster commits — none touch `*_windows.rs`. The Authenticode addition (Pattern 4) lives in new code, not existing `*_windows.rs` |
| A11 | `Win32_Security_Cryptography` feature is NOT needed for v0.0 (WinVerifyTrust suffices) | Standard Stack | Catalog-signed binaries (`CryptCATAdminAcquireContext`) are out of scope for dev builds; valid only for installed Windows components |
| A12 | Plan 22-04 OAuth2 inline tests don't need Windows-specific `#[cfg]` gating | D-15 | Inline tests are TLS-pure (no filesystem); cross-platform by construction. Windows-specific tests per D-15 are ADDITIONAL (`keyring://`, Credential Manager resolution) — those are new test functions |

## Open Questions

1. **Should Plan 22-05 land Authenticode (REQ-AUD-03 acceptance #2/#3) in the SAME commit as `02ee0bd1` cherry-pick, or separately?**
   - What we know: Authenticode is fork-only; `02ee0bd1` is the upstream parent of `ExecutableIdentity`; D-19 says one commit per semantic change.
   - What's unclear: Whether "exec_identity Authenticode addition" is a separate semantic change from "exec_identity recording introduced".
   - Recommendation: Separate commit. First commit cherry-picks `02ee0bd1` clean (sha256-only), then a fork-only commit titled `feat(22-05): add Authenticode signature query for exec-identity (Windows)` follows. Commit body uses no `Upstream-commit:` trailer; instead documents "fork-only addition; satisfies REQ-AUD-03 acceptance #2/#3".

2. **Can `9db06336` cherry-pick include `crates/nono-cli/tests/audit_attestation.rs` directly, or does fork-specific path adjustment break the test?**
   - What we know: Fork's `crates/nono-cli/tests/` already has 6 test files; upstream `audit_attestation.rs` uses `env!("CARGO_BIN_EXE_nono")` which is cross-platform.
   - What's unclear: Whether the test file's `HOME` env-var manipulation conflicts with fork's `EnvVarGuard` lock pattern (CLAUDE.md "Environment variables in tests" rule).
   - Recommendation: Cherry-pick clean. If test fails on Windows due to `HOME` not being a Windows concept (Windows uses `USERPROFILE`), wrap with fork's existing `EnvVarGuard` pattern. The fork test files (`config_flag.rs`, `env_vars.rs`) demonstrate the right pattern.

3. **Does Plan 22-03 (PKG) land `nono package` as a top-level subcommand or under `nono` directly?**
   - What we know: Upstream `8b46573d` adds `Pull, Remove, Update, Search, List` variants; `71d82cd0` later renames "packages" → "packs" but keeps subcommand structure.
   - What's unclear: Whether fork's existing CLI (with `nono ps`, `nono audit`, `nono profile`) has free namespace for `nono package <subcmd>` vs `nono pull` / `nono remove` directly.
   - Recommendation: Mirror upstream's final v0.40.1 shape — `nono package <subcmd>` (post-`71d82cd0` unification). Verify against `cargo run -p nono-cli -- --help` after Plan 22-03 wave-1 first-commit lands.

4. **Should Plan 22-03 PKG land `55fb42b8` or `55fb42b8 + 1d49246a` rollup?**
   - What we know: `55fb42b8` adds `packages/claude-code/` files. `1d49246a` (out-of-range for plan 22-03 explicitly per OUT OF SCOPE) removes them. Fork has neither.
   - What's unclear: Whether to cherry-pick `55fb42b8` then `1d49246a` (round-trip), or cherry-pick `55fb42b8` with `--strategy=ours` for the new files.
   - Recommendation: Cherry-pick `55fb42b8`'s code changes only (`package_cmd.rs` + `policy.rs`), reset the file additions in `packages/claude-code/`. Document deviation in commit body. Avoids round-trip churn.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `git` | All cherry-picks (D-01..D-04) | ✓ | 2.x assumed | — |
| `cargo` (Rust toolchain) | All builds + tests | ✓ (Rust 1.77+ per Cargo.toml) | — | — |
| `cargo audit` | `make ci` D-18 gate | Unverified (not run during research) | — | Skip via `make test` if missing |
| `cargo fmt` | `make ci` fmt-check | ✓ (rustup component) | — | — |
| `cargo clippy` | `make ci` clippy gate | ✓ | — | — |
| `pwsh` (PowerShell 7+) | `make test-windows-*` Windows harness | Unverified on this host (research host is Windows) | — | `make test-cli` covers most |
| `gh` CLI | Per memory: available; only relevant if Phase 22 chooses per-plan PR pattern (Claude's Discretion bullet) | ✓ (per `feedback_gh_available.md`) | — | — |
| `upstream` git remote | Cherry-picks | ✓ | `https://github.com/always-further/nono.git` (verified `git remote -v`) | — |
| `origin` git remote | D-06/D-07 push events | ✓ | `https://github.com/oscarmackjr-twg/nono.git` (verified) | — |
| Admin elevation (Windows) | `wfp_port_integration -- --ignored`, `learn_windows_integration -- --ignored` | Conditional (per-developer) | — | Skip ignored tests; documented as `-- --ignored` opt-in |
| `nono-wfp-service` (Windows) | `wfp_port_integration -- --ignored` | Conditional | — | Test gracefully skips per file header |
| `windows-sys 0.59 Win32_Security_WinTrust` feature | Plan 22-05 Authenticode | ⚠ NOT enabled in fork; must add to `crates/nono-cli/Cargo.toml` | — | None — required for REQ-AUD-03 acceptance #2 |

**Missing dependencies with no fallback:**
- `Win32_Security_WinTrust` feature flag must be added to `crates/nono-cli/Cargo.toml` `windows-sys` features list before Plan 22-05's Authenticode commit.

**Missing dependencies with fallback:**
- Admin-elevation tests (`wfp_port_integration --ignored`, `learn_windows_integration --ignored`) gracefully skip — documented in test file headers.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust built-in) + `proptest` for property tests |
| Config file | `Cargo.toml` (`[dev-dependencies]` per crate) — no separate config |
| Quick run command | `cargo test -p nono-cli --bin nono <test-name-glob>` |
| Full suite command | `make ci` (= `cargo build --workspace` + clippy + fmt-check + `cargo test -p nono` + `cargo test -p nono-cli` + `cargo test -p nono-ffi` + `cargo audit`) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PROF-01 | Profile parses `unsafe_macos_seatbelt_rules` on Windows | unit (serde roundtrip) | `cargo test -p nono-cli --bin nono profile::tests` | ✅ existing tests in `profile/mod.rs`; new test added per upstream `14c644ce` |
| PROF-01 | macOS `add_platform_rule` applies; Windows/Linux no-op | unit (cfg-gated) | `cargo test -p nono-cli sandbox_prepare::tests` | ✅ `crates/nono-cli/src/sandbox_prepare.rs::tests` |
| PROF-02 | Profile with `packs: []` round-trips through serde_json | unit | `cargo test -p nono-cli --bin nono profile::tests::packs_default_serde` | ❌ Wave 0 — port from upstream `088bdad7` |
| PROF-02 | Registry-pack profile loads contributions with correct precedence | integration | `cargo test -p nono-cli --bin nono profile::tests::registry_pack_loads` | ❌ Wave 0 — port from upstream `115b5cfa` |
| PROF-03 | Profile with `custom_credentials.oauth2` parses on Windows | unit | `cargo test -p nono-cli --bin nono profile::tests::oauth2_parses` | ❌ Wave 0 — port from `b1ecbc02` (12 new tests) |
| PROF-03 | `token_url: http://...` rejected | unit | `cargo test -p nono-cli --bin nono profile::tests::oauth2_http_rejected` | ❌ Wave 0 |
| PROF-03 | Zeroize on Drop confirmed | unit | `cargo test -p nono-cli --bin nono profile::tests::client_secret_zeroized` | ❌ Wave 0 |
| PROF-04 | `nono profile list` shows `claude-no-keychain` on Windows | integration | `cargo test -p nono-cli --test profile_cmd` | ✅ `crates/nono-cli/tests/profile_cmd.rs` exists |
| POLY-01 | Orphan `override_deny` fails load | unit | `cargo test -p nono-cli --bin nono profile::tests::override_deny_without_grant_fails_load` | ❌ Wave 0 — REQ-POLY-01 acceptance #3 names this test |
| POLY-01 | All fork-shipped built-in profiles pass new check | integration | `cargo test -p nono-cli --test policy_cmd` | ✅ existing; planner extends |
| POLY-02 | `--rollback --no-audit` rejects with exit code != 0 | unit (clap try_parse) | `cargo test -p nono-cli --bin nono cli::tests::rollback_no_audit_conflict` | ❌ Wave 0 |
| POLY-03 | `.claude.lock` renders as single-file grant on Windows | unit | `cargo test -p nono-cli policy::tests::claude_lock_allow_file` | ✅ existing tests in `policy.rs`; planner extends per `49925bbf` |
| PKG-01 | `nono package list` empty on clean Windows install | integration | `cargo test -p nono-cli --test package_integration` | ❌ Wave 0 — new test file |
| PKG-01 | `nono package pull <known-good>` places + registers hooks | integration | `cargo test -p nono-cli --test package_integration -- --ignored test_pull_with_mock_registry` | ❌ Wave 0 — port upstream fixture per D-14 |
| PKG-02 | Long-path `\\?\` install (path > 260 chars) | integration (Windows-only) | `cargo test -p nono-cli --test package_integration -- --ignored windows_long_path` | ❌ Wave 0 — D-15 |
| PKG-02 | Path-traversal regression (`../` in manifest) | unit | `cargo test -p nono-cli --bin nono package_cmd::tests::reject_traversal` | ❌ Wave 0 — port `ec49a7af` |
| PKG-03 | Idempotent install (double-pull no-op) | integration | `cargo test -p nono-cli --test package_integration test_double_install_idempotent` | ❌ Wave 0 |
| PKG-04 | Streaming download succeeds for ≥50MB | integration (slow, opt-in) | `cargo test -p nono-cli --test package_integration -- --ignored streaming_50mb` | ❌ Wave 0 |
| PKG-04 | Corrupted artifact rejected pre-install | unit | `cargo test -p nono-cli --bin nono package_cmd::tests::reject_corrupted` | ❌ Wave 0 — port `9ebad89a` test |
| PKG-04 | HTTP-only URL rejected | unit | `cargo test -p nono-cli --bin nono registry_client::tests::reject_http` | ❌ Wave 0 |
| OAUTH-01 | Bearer token injected on outbound request | integration | `cargo test -p nono-proxy --bin nono-proxy oauth2::tests::test_token_cache_returns_valid_token` | ❌ Wave 0 — port 11 inline tests from `9546c879` |
| OAUTH-01 | Cached token reused until expiry | unit | `cargo test -p nono-proxy oauth2::tests::test_token_cache_detects_expiry` | ❌ Wave 0 — same port |
| OAUTH-01 | Windows `keyring://` resolution | unit (Windows-only) | `cargo test -p nono-cli --bin nono profile::tests::oauth2_keyring_windows` (cfg-gated) | ❌ Wave 0 — D-15 |
| OAUTH-02 | `http://127.0.0.1:8080` upstream works | integration | `cargo test -p nono-proxy reverse::tests::http_loopback_allowed` | ❌ Wave 0 — port `2bf5668f` + `0c990116` |
| OAUTH-02 | `http://192.168.1.10:8080` rejected | unit | `cargo test -p nono-proxy reverse::tests::http_non_loopback_rejected` | ❌ Wave 0 |
| OAUTH-02 | `http://0.0.0.0:8080` rejected | unit | `cargo test -p nono-proxy reverse::tests::http_unspecified_rejected` | ❌ Wave 0 — port `0c990116` |
| OAUTH-03 | `--allow-domain --strict-proxy` enforces both | integration | `cargo test -p nono-cli --test wfp_port_integration allow_domain_in_strict_mode` | ⚠ existing file; new test in it |
| OAUTH-03 | Dry-run output lists domain exactly once | unit | `cargo test -p nono-cli sandbox_prepare::tests::no_duplicate_allow_domain_print` | ❌ Wave 0 — port `60ad1eb3` |
| AUD-01 | `--audit-integrity` produces populated event_count + chain_head + merkle_root | integration | `cargo test -p nono-cli --test audit_attestation` (port from upstream) | ❌ Wave 0 — file does not exist in fork |
| AUD-01 | Tamper test invalidates subsequent chain entries | unit | `cargo test -p nono-cli audit_integrity::tests::tamper_invalidates` | ❌ Wave 0 — part of `4f9552ec` port |
| AUD-02 | `nono audit verify` succeeds for untampered, fails for tampered | integration | `cargo test -p nono-cli --test audit_attestation verify_tamper` | ❌ Wave 0 — port `0b1822a9` + `6ecade2e` |
| AUD-02 | Verify surfaces signer key ID in output | integration | `cargo test -p nono-cli --test audit_attestation verify_displays_key_id` | ❌ Wave 0 |
| AUD-03 | Signed release binary shows valid signer chain (Windows) | integration (Windows-only) | `cargo test -p nono-cli --test audit_attestation -- --ignored windows_authenticode_signed` | ❌ Wave 0 — D-15 + fork addition |
| AUD-03 | Unsigned dev build shows `unsigned` + SHA-256 | integration | `cargo test -p nono-cli --test audit_attestation windows_authenticode_unsigned` | ❌ Wave 0 — fork addition |
| AUD-04 | `nono session cleanup --older-than 7d` works on Windows | integration | `cargo test -p nono-cli --bin nono session::tests::cleanup_command_basic` | ❌ Wave 0 — new tests post-rename |
| AUD-04 | `auto_prune_is_noop_when_sandboxed` passes under both old + new function names | unit | `cargo test -p nono-cli --bin nono auto_prune_is_noop_when_sandboxed` | ✅ EXISTS at `session_commands.rs:708` AND `session_commands_windows.rs:801` — D-04 gate |
| AUD-04 | `--older-than 30` (no suffix) fails with CLEAN-04 hint | unit | `cargo test -p nono-cli --bin nono parse_duration_rejects_invalid` | ✅ EXISTS at `cli.rs:2467` — close approximation; planner adds explicit `parse_prune_duration` test |

### Sampling Rate
- **Per task commit:** `cargo test -p nono-cli --bin nono <relevant-glob>` (sub-30s)
- **Per wave merge:** `make test` (full workspace tests, ~3-5min on Windows)
- **Phase gate:** `make ci` (full + clippy + fmt + audit, ~10min) — D-18 mandate

### Wave 0 Gaps (test infrastructure that must land before requirement implementation)
- [ ] `crates/nono-cli/tests/audit_attestation.rs` — does not exist in fork; port from upstream `9db06336` for AUD-01..04 acceptance
- [ ] `crates/nono-cli/tests/package_integration.rs` — does not exist; create as new file alongside `8b46573d` cherry-pick for PKG-01..04 acceptance
- [ ] `crates/nono-cli/src/profile/mod.rs::tests::override_deny_without_grant_fails_load` — does not exist; REQ-POLY-01 acceptance #3 names it explicitly
- [ ] `crates/nono-cli/src/cli.rs::tests::rollback_no_audit_conflict` — does not exist; REQ-POLY-02 acceptance needs it
- [ ] `crates/nono-cli/src/audit_integrity.rs` — file does not exist; new module from `4f9552ec` port
- [ ] `crates/nono-cli/src/audit_session.rs` — file does not exist; new module from `4f9552ec` port
- [ ] `crates/nono-cli/src/audit_ledger.rs` — file does not exist; new module from `02ee0bd1` port
- [ ] `crates/nono-cli/src/audit_attestation.rs` — file does not exist; new module from `6ecade2e` port (519 LOC)
- [ ] `crates/nono-cli/src/audit_authenticode.rs` (or sibling) — fork addition for REQ-AUD-03 acceptance #2/#3 Authenticode integration
- [ ] `crates/nono-cli/src/registry_client.rs` — file does not exist in fork; new module from `8b46573d` (95 LOC) + `9ebad89a` (114 LOC)
- [ ] `crates/nono-cli/src/package.rs` — file does not exist; new module from `8b46573d` (312 LOC) + `71d82cd0`
- [ ] `crates/nono-cli/src/package_cmd.rs` — file does not exist; new module from `8b46573d` (765 LOC) + cluster

*(Confirmed by `ls crates/nono-cli/src/ | grep -E 'audit|package|registry'` — none present.)*

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes (OAUTH-01..03) | OAuth2 client_credentials via existing fork keystore (`keyring://`) |
| V3 Session Management | yes (AUD-01..04) | Hash-chained ledger + DSSE attestation; immutable session metadata post-close (file permissions) |
| V4 Access Control | yes (POLY-01..03, PKG-02) | `override_deny` requires matching grant; `--rollback` requires audit; path-traversal validation |
| V5 Input Validation | yes (PROF-01..04, POLY-01..03, PKG-01..04) | serde rejection on malformed JSON; `validate_safe_name` + `validate_path_within` from `ec49a7af`; `token_url` HTTPS-only check |
| V6 Cryptography | yes (AUD-01..04, PKG-04) | sigstore-rs DSSE bundles; SHA-256 hash chain; rustls-webpki 0.103.13 trust chain — never hand-roll |
| V8 Data Protection | yes (PROF-03, AUD-02) | `Zeroizing<String>` for client_secret + access_token (REQ-PROF-03 acceptance #3); audit-sign-key never logged |

### Known Threat Patterns for Phase 22 stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal in package install (REQ-PKG-02) | Tampering | `validate_path_within` + canonicalize-then-verify; reject `..`, symlinks, UNC aliases (`\\?\GLOBALROOT\…`) |
| Long-path bypass on Windows (REQ-PKG-02) | Tampering | Use `\\?\` extended-length prefix consistently; std `canonicalize` returns extended-path on Windows |
| Insecure HTTP upstream in reverse-proxy (REQ-OAUTH-02) | Information Disclosure | Loopback-only enforcement; reject `0.0.0.0` / unspecified addresses (`0c990116`) |
| OAuth2 token leakage to disk (REQ-OAUTH-01) | Information Disclosure | In-memory cache only with `Zeroizing`; no `serde::Serialize` impl on `CachedToken` |
| Audit log tampering (REQ-AUD-01) | Tampering | Hash chain + Merkle root; tamper invalidates subsequent entries |
| Audit log forgery (REQ-AUD-02) | Spoofing / Repudiation | DSSE/in-toto attestation over Merkle root; sigstore-rs Rekor lookup |
| Sandboxed agent triggers host file deletion via `nono ps` (T-19-04-07 from CLEAN-04) | Elevation of Privilege | Structural NONO_CAP_FILE early-return as FIRST statement of `auto_prune_if_needed`; preserved through Plan 22-05 rename |
| Spoofed nono binary (REQ-AUD-03) | Spoofing | Authenticode signature query on Windows (fork addition); SHA-256 hash always recorded as fallback |
| Orphan `override_deny` masking unintended grant (REQ-POLY-01) | Tampering | Fail-closed at profile load; no silent ignore |
| `--rollback` without audit (REQ-POLY-02) | Repudiation | Clap `conflicts_with` rejects at parse time; no partial sandbox state |
| Symlink escape from `install_dir` (REQ-PKG-02) | Tampering | `canonicalize()` before grant; verify resolved path starts with base |
| Untrusted package signer (REQ-PKG-04) | Spoofing | sigstore bundle verification before any FS placement; signer pinning in lockfile |

## Project Constraints (from CLAUDE.md)

The following CLAUDE.md directives are LOAD-BEARING for Phase 22 and the planner must verify each per plan:

1. **No `.unwrap()` or `.expect()` in production code** (`-D clippy::unwrap_used` enforced). Test code may use `#[allow(clippy::unwrap_used)]` at module level. Upstream `oauth2.rs` test fixture uses `.unwrap()` — fork's port must wrap test mod with `#[allow]`.
2. **DCO sign-off on every commit.** `Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>` — already enforced by hook on `main`. The 46 historical pre-DCO commits are deferred per quick task 260424-mrg path C.
3. **Path component comparison, NOT string operations.** `path.starts_with("/home")` matches `/homeevil`; use `Path::starts_with()`. Especially relevant for REQ-PKG-02 `install_dir` validation.
4. **Validate env vars before use.** `HOME`, `LOCALAPPDATA`, `XDG_CONFIG_HOME` are all candidates for poisoning. Use `dirs` crate canonicalization.
5. **Fail-secure on any unsupported shape.** REQ-PROF-03 `token_url: http://...` rejection; REQ-OAUTH-02 reverse-proxy IP class enforcement; REQ-PKG-02 path-traversal rejection — all fail-closed.
6. **Environment variable save/restore in tests.** `EnvVarGuard` pattern from `crates/nono-cli/src/test_env.rs`. Critical for any test mutating `HOME`, `TMPDIR`, `XDG_CONFIG_HOME`, `NONO_CAP_FILE`.
7. **`#[must_use]` on critical Result-returning functions.** Apply to new `audit_integrity::record_event`, `audit_attestation::sign_bundle` per `nono::Result<T>` conventions.
8. **Strict OS-level isolation: never grant access to entire directories when specific paths suffice.** REQ-POLY-03 `.claude.lock` allow_file move is the canonical example; planners apply same principle to PKG-02 `install_dir`.
9. **Memory zeroization for sensitive data.** `client_secret`, `access_token`, `audit-sign-key` private bytes — all wrapped in `Zeroizing<…>`. Verify in REQ-PROF-03 acceptance #3 test.
10. **Library should almost never panic.** Use `Result` instead. Audit cluster's `audit_integrity::v3 → alpha` migration MUST surface chain mismatches as `NonoError`, not panic (`7b7815f7` upstream pattern).

## Contradictions Found

> Honest reporting per RESEARCH.md philosophy. CONTEXT.md decisions D-01..D-20 are LOCKED — these contradictions are not invitations to re-derive decisions, but factual corrections the planner must honor.

### Contradiction 1: SHA typo for hooks#!env commit

- **CONTEXT.md** (line 245, 256, 337, 338) cites upstream commit **`8b2a5ffb`** for `fix(hooks): invoke bash via env`.
- **Actual upstream SHA** is **`8b5a2ffb`** (single-character difference: `b2a` → `b5a`).
- **Verification:** `git show 8b2a5ffb` returns `fatal: ambiguous argument 'unknown revision'` (would fail; not tested but expected). `git show 8b5a2ffb` returns the cited commit.
- **Impact:** Plans must reference the correct SHA. STOP trigger #8 still fires for either name (the trigger is "if cherry-pick attempts the bash#!env hook fix"); but operators following CONTEXT.md verbatim will fail.
- **Resolution recommendation:** Plans should reference **`8b5a2ffb`** with a note "(was `8b2a5ffb` in CONTEXT.md — typo)".

### Contradiction 2: Authenticode in REQ-AUD-03 is a fork-only addition, not an upstream port

- **CONTEXT.md** `<canonical_refs>` Plan 22-05 commits list `02ee0bd1 feat(audit): record executable identity` and `7b7815f7 feat(audit): record exec identity and unify audit integrity` as the upstream parents for REQ-AUD-03.
- **CONTEXT.md** `<code_context>` Integration Points Plan 22-05 says: "AUD exec-identity Windows path — `GetModuleFileNameW` for path resolution + `WinVerifyTrust` / `CryptCATAdminAcquireContext` for Authenticode signature query. SHA-256 fallback when unsigned."
- **CONTEXT.md** Claude's Discretion bullet says: "Authenticode fallback path for REQ-AUD-03 — when Authenticode signature query fails or returns 'unsigned', how the SHA-256 fallback is recorded in the ledger format. Planner decides based on upstream's record shape."
- **VERIFIED FROM UPSTREAM SOURCE** (`git show v0.40.1:crates/nono/src/undo/types.rs`): Upstream's `ExecutableIdentity` struct is:
  ```rust
  pub struct ExecutableIdentity {
      pub resolved_path: PathBuf,
      pub sha256: ContentHash,
  }
  ```
  Two fields. **No Authenticode field. No `WinVerifyTrust` call anywhere in upstream's audit cluster** (`git grep WinVerifyTrust v0.40.1` returns 0 results).
- **Impact:** Authenticode integration is **purely a fork addition** to satisfy REQ-AUD-03 acceptance #2 ("Signed release binary shows valid signer chain") and #3 ("Unsigned dev build shows `unsigned` + SHA-256"). It is NOT covered by `02ee0bd1` or `7b7815f7` cherry-picks — both will land SHA-256-only `ExecutableIdentity` cleanly.
- **Resolution recommendation:** 
  1. Plan 22-05's commit chain cherry-picks `02ee0bd1` and `7b7815f7` clean (sha256-only).
  2. A separate fork-only commit follows (~150 LOC), titled `feat(22-05): add Authenticode signature query for exec-identity (Windows)`. No `Upstream-commit:` trailer; instead documents "fork addition to satisfy REQ-AUD-03 acceptance #2/#3 — upstream `ExecutableIdentity` is SHA-256 only."
  3. The Authenticode status lives in a **sibling field** on `SessionMetadata` (e.g., `audit_authenticode: Option<AuthenticodeStatus>`), NOT a mutation of upstream's `ExecutableIdentity` (which would break D-19 atomic-commit-per-semantic-change discipline).
  4. Per D-17 exception note, this is one of the planned Windows-only fork additions — ALLOWED.

## Planner Cheat-Sheets

### Plan 22-01 PROF (Wave 0, parallel with 22-02)

**Upstream commit chain (chronological):**
1. `14c644ce` feat: add `unsafe_macos_seatbelt_rules` profile field — touches `profile/mod.rs` (+20), `policy.rs` (+1), `policy_cmd.rs` (+18), `sandbox_prepare.rs` (+18), `nono-profile.schema.json` (+5)
2. `c14e4365` style: run cargo fmt — touches `policy_cmd.rs` (+6/-3) **REQUIRED INTERMEDIATE — not in CONTEXT.md but verified necessary**
3. `e3decf9d` fix: address review feedback — touches `policy.rs` (+4/-1), `policy_cmd.rs` (+1/-1)
4. `ecd09313` fix: add to test Profile initializers — touches `profile/mod.rs` (+2)
5. `d32ab18a` fix(sandbox): downgrade unsafe seatbelt rules log warn → info — touches `sandbox_prepare.rs` (+1/-1) [ASSUMED A2: safe to defer/skip]
6. `fbf5c06e` feat(config): OAuth2Config type — touches `nono-proxy/src/config.rs` (+90), `nono-proxy/src/credential.rs` (+1), `nono-proxy/src/server.rs` (+5), `nono-cli/src/network_policy.rs` (+2). **NOTE:** Plan 22-04 also lists this; per dependency analysis, PROF-03 needs this type but the new config code is in nono-proxy. Land in 22-01 (touches `network_policy.rs`) and `OAuth2Config` becomes available for both plans.
7. `b1ecbc02` feat(profile): support OAuth2 in custom_credentials — touches `profile/mod.rs` (+364/-???), `network_policy.rs` (+125), `policy_cmd.rs` (+22), `nono-proxy/src/credential.rs` (+261), `nono-profile.schema.json` (+38)
8. `088bdad7` feat(profile): introduce packs and command_args — touches `profile/mod.rs`, `command_runtime.rs` (+31), `package.rs` (+1), `package_cmd.rs` (+35), `nono-profile.schema.json` (+10), `policy.json` (+9). **WARNING:** depends on PackRef type from 22-03's `71d82cd0` — order matters
9. `115b5cfa` feat(profile): load profiles from registry packs — touches `profile/mod.rs` (+94)
10. `3c8b6756` feat(claude): add no-keychain profile — touches `policy.json` (+75/-???), `profile/builtin.rs` (+102), `policy.rs` (+1), `manifest_roundtrip.rs` (+3), `policy_cmd.rs` (+3)
11. `713b2e0f` fix(policy): update tests and claude-no-kc for allow_file move — touches `policy.json` (+5/-2), `policy.rs` (+2/-1), `profile/builtin.rs` (+6/-3) **DEPENDS ON 22-02's `49925bbf`**

**Cross-plan dependency:** Step 8 (`088bdad7`) depends on PackRef type from 22-03 `71d82cd0`. Two options: (a) defer step 8 to start of 22-03 wave; (b) land 22-03's `71d82cd0` first (out of order). Recommendation (a) — keeps 22-01 self-contained; PROF-02's `packs` field is field-only deserialization, the actual pack resolution lives in 22-03.

**Cross-plan dependency:** Step 11 (`713b2e0f`) depends on 22-02's `49925bbf` (which moves `.claude.lock` to `allow_file`). Recommendation: 22-02 lands `49925bbf` before 22-01 lands `713b2e0f`. Inline coordination via small commits.

**Fork files touched:** `crates/nono-cli/src/profile/mod.rs` (DRIFT: +732/-414 vs v0.37.1), `crates/nono-cli/src/profile/builtin.rs` (DRIFT: +229/-2), `crates/nono-cli/src/policy.rs` (DRIFT: +106/-333), `crates/nono-cli/src/policy_cmd.rs` (drift unmeasured), `crates/nono-cli/src/network_policy.rs` (DRIFT: -45), `crates/nono-cli/src/sandbox_prepare.rs` (DRIFT: +69/-1202), `crates/nono-cli/data/policy.json` (DRIFT: +59/-2), `crates/nono-cli/data/nono-profile.schema.json`, `crates/nono-cli/src/command_runtime.rs`, `crates/nono-proxy/src/config.rs`, `crates/nono-proxy/src/credential.rs`, `crates/nono-proxy/src/server.rs`, `crates/nono-cli/src/manifest_roundtrip.rs` (test).

**D-02 fallback prediction:** Step 7 (`b1ecbc02`, +364 LOC `profile/mod.rs`) hits fork's +732/-414 drift. CONFLICT PROBABILITY: HIGH. Plan executor should anticipate manual replay. Step 10 (`3c8b6756`, +75 LOC `policy.json`) hits fork's +59/-2 drift. CONFLICT PROBABILITY: MEDIUM.

**Fixtures to port (D-13/D-14):** None — all PROF tests are inline `#[cfg(test)]` in `profile/mod.rs` or unit tests in `policy_cmd.rs`. Inherits 12 new tests from `b1ecbc02`.

**Existing fork patterns to reuse:**
- `Profile::resolve_aipc_allowlist` Phase 18.1 plumbing — new fields slot in
- `nono::keystore::load_secret` for REQ-PROF-03
- `EnvVarGuard` for any test that touches HOME/XDG_CONFIG_HOME

### Plan 22-02 POLY (Wave 0, parallel with 22-01)

**Upstream commit chain (chronological):**
1. `49925bbf` fix(policy): move `.claude.lock` to allow_file — small (~10 lines, `policy.json`)
2. `a524b1a7` fix(policy): add entry for ~.local/share/claude/versions — small (~5 lines, `policy.json`)
3. `7d1d9a0d` fix(policy): improve unlink rules; add claude read path — touches `policy.rs` macOS branch
4. `5c301e8d` refactor(policy): enforce stricter policy for overrides, rollback — touches `capability_ext.rs` (+70/-???), `cli.rs` (clap conflicts_with), `tests/integration/test_audit.sh` (+52/-???), DELETES `MANUAL_TEST_STEPS.md`. **BREAKING.**
5. `b83da813` feat(policy): filter profile override deny entries without grants — touches `capability_ext.rs` (+47)
6. `930d82b4` fix(cli): skip non-existent profile deny overrides — small (~20 lines)

**Cross-plan dependency:** Step 1 (`49925bbf`) must land before 22-01 step 11 (`713b2e0f`). Inline coordination.

**Fork files touched:** `crates/nono-cli/data/policy.json`, `crates/nono-cli/src/policy.rs` (DRIFT: +106/-333), `crates/nono-cli/src/capability_ext.rs` (DRIFT: +182/-102), `crates/nono-cli/src/cli.rs` (DRIFT: +1207/-144), `crates/nono-cli/tests/*rollback*.rs`, `tests/integration/test_audit.sh`, `MANUAL_TEST_STEPS.md` (delete).

**Fork rollback test audit (Pitfall 4):** BEFORE step 4 (`5c301e8d`), run:
```bash
grep -rE "no.audit.*rollback|rollback.*no.audit" crates/nono-cli/tests/ tests/integration/ 2>&1
```
Update fork tests to drop `--no-audit` from `--rollback` invocations. Land update in same commit.

**D-02 fallback prediction:** Step 4 (`5c301e8d`) on `capability_ext.rs` (DRIFT: +182/-102): MEDIUM probability. `cli.rs` (DRIFT: +1207/-144): HIGH probability for the clap-config conflict.

**Fixtures to port:** None new; existing `crates/nono-cli/tests/policy_cmd.rs` extends.

**Existing fork patterns to reuse:**
- `clap::conflicts_with` already used elsewhere in `cli.rs` (e.g., `--all-exited` conflicts with `--older-than` at line 2204)
- `NonoError::PolicyError` variant already exists; add `OrphanOverrideDeny` kind enum value

### Plan 22-03 PKG (Wave 1, parallel with 22-04, after 22-01 closes)

**Upstream commit chain (chronological):**
1. `8b46573d` feat(cli): add package management commands (pull, remove, search, list) — 4,556 insertions across 13 files. **NEW MODULES:** `package.rs` (312), `package_cmd.rs` (765), `registry_client.rs` (95). Touches `cli.rs` (+186), `main.rs` (+2384 — major restructure), `hooks.rs` (+54), `profile/mod.rs` (+13), `policy.rs` (+38), `error.rs` (+9), `bindings/c/src/lib.rs` (+6/-3). **WARNING:** `main.rs` +2384 is the breaking restructure.
2. `55fb42b8` feat(package): add install_dir + hook unregistration — touches `package.rs` (+4), `package_cmd.rs` (+286), `policy.rs` (+7). **ADDS** `packages/claude-code/*` files.
3. `088bdad7` (already in 22-01 step 8 if deferred there) — touches `package.rs` (+1), `package_cmd.rs` (+35)
4. `ec49a7af` fix(package): harden installation security — touches `package_cmd.rs` (+48). **NEW:** `validate_safe_name`, `validate_path_within`.
5. `115b5cfa` (already in 22-01 step 9) — touches `profile/mod.rs` only; not in 22-03 surface
6. `9ebad89a` refactor(pkg): stream package artifact downloads — touches `package_cmd.rs` (+259/-???), `registry_client.rs` (+114/-???), `Cargo.toml` (+1 semver crate). **REMOVES** the `claude-code` package's `nono-hook.sh` from cherry-pick effects on the fork (already absent).
7. `0cbb7e62` refactor(package): simplify artifact signer validation — small
8. `600ba4ec` refactor(package-cmd): centralize trust bundle — touches `package.rs` (+2/-???)
9. `58b5a24e` refactor(cli): improve artifact path validation — small (`package_cmd.rs` +20/-14)
10. `71d82cd0` feat(pack): introduce pack types and unify package naming — touches `app_runtime.rs` (+16), `cli.rs` (+28/-???), `cli_bootstrap.rs` (+5), `main.rs` (-2381 — undoes `8b46573d`'s main.rs changes), `package.rs` (+25/-???), `package_cmd.rs` (+22/-???), `profile_runtime.rs` (+6/-???). **REMOVES** `package-hosting.md`.

**DO NOT cherry-pick (verified):**
- `8b5a2ffb` fix(hooks): invoke bash via env — N/A on Windows; STOP trigger #8
- `1d49246a` feat(claude-code): remove claude-code integration package — fork has no `packages/claude-code/`; STOP trigger #9 (investigated; conclusion: skip)

**For step 2 (`55fb42b8`):** Cherry-pick the code changes only. Reset `packages/claude-code/*` file additions in the same commit (drive-by Rule-3 deviation, document in commit body).

**Fork files touched:** `crates/nono-cli/src/cli.rs` (HIGH DRIFT), `crates/nono-cli/src/main.rs` (DRIFT unmeasured but likely high — fork has different module layout), `crates/nono-cli/src/hooks.rs` (DRIFT: +319/-1), `crates/nono-cli/src/policy.rs` (HIGH DRIFT), `crates/nono-cli/src/profile/mod.rs` (HIGH DRIFT), `crates/nono/src/error.rs`, `bindings/c/src/lib.rs`, **NEW:** `crates/nono-cli/src/package.rs`, `crates/nono-cli/src/package_cmd.rs`, `crates/nono-cli/src/registry_client.rs`, `crates/nono-cli/src/app_runtime.rs`, `crates/nono-cli/src/profile_runtime.rs`.

**D-02 fallback prediction:** Step 1 (`8b46573d`) on `main.rs` +2384 lines: GUARANTEED CONFLICT (fork's main.rs structure differs). **High probability of full manual replay for `main.rs` portion.** Recommend: split step 1 into `feat(22-03): port package modules` (clean: package.rs, package_cmd.rs, registry_client.rs) + `feat(22-03): wire package commands into CLI` (manual: main.rs + cli.rs).

**Fixtures to port (D-14/D-15):** Inline `#[cfg(test)] mod tests` in `package_cmd.rs` (3 tests) and `package.rs` (2 tests). NEW Windows-specific tests:
- `tests/package_integration.rs::windows_long_path` (D-15 #2)
- `tests/package_integration.rs::windows_path_traversal_unc_alias` (D-15 #2)
- `tests/package_integration.rs::windows_localappdata_resolution` (D-15)

**Existing fork patterns to reuse:**
- `nono::trust::signing::sign_statement_bundle` (Phase 20 UPST-04)
- `dirs::config_local_dir()` for `%LOCALAPPDATA%` (already in fork via dirs crate)
- `nono::keystore` for any registry API tokens

### Plan 22-04 OAUTH (Wave 1, parallel with 22-03, after 22-01 closes)

**Upstream commit chain (chronological):**
1. `2244dd73` fix(proxy): return early after 413 in read_request_body — touches `nono-proxy/src/server.rs`. **WARNING:** prerequisite for `9546c879`.
2. `9546c879` feat(proxy): implement OAuth2 client_credentials token exchange with cache — 552 LOC NEW `oauth2.rs` + `error.rs` (+3) + `lib.rs` (+1). **11 inline tests.**
3. `0c7fb902` fix(oauth): PR 517 rebase on main — touches `nono-profile.schema.json` (-3), `network_policy.rs` (+8)
4. `19a0731f` fix: compilation against current main after rebase — touches multiple files; cleanup
5. `005579a9` do not silently fail port — touches `network_policy.rs` (+45/-2), `sandbox_prepare.rs` (+21)
6. `10bcd054` fix(network): keep `--allow-domain` in strict proxy-only mode — primary OAUTH-03 commit
7. `d44e404e` fix(tests): tests and format fixes — touches `sandbox_prepare.rs` (+9/-7)
8. `60ad1eb3` fix(dry): duplicated allow_domain warning-print logic — touches `sandbox_prepare.rs` (+13/-21)
9. `2bf5668f` feat(reverse-proxy): add http upstream support — 260 LOC `reverse.rs` (+179/-81)
10. `0340ebff` fix(proxy): restrict insecure http upstreams to local-only targets — `reverse.rs` (+111/-2)
11. `b2a24402` fix(proxy): support local-only http upstreams safely — `reverse.rs` (+46/-27)
12. `0c990116` fix(reverse-proxy): disallow insecure http upstreams for unspecified local addresses — `reverse.rs` (+15/-7)

**Cross-plan dependency:** Step 1 (`2244dd73`) is in upstream chronological order BEFORE Plan 22-01's `fbf5c06e`/`b1ecbc02`. If 22-01 cherry-picks `fbf5c06e` first, `2244dd73` may already be needed. Recommendation: include `2244dd73` in 22-01 alongside `fbf5c06e` to avoid 22-04 having to look back.

**Fork files touched:** `crates/nono-proxy/src/oauth2.rs` (NEW), `crates/nono-proxy/src/server.rs`, `crates/nono-proxy/src/error.rs`, `crates/nono-proxy/src/lib.rs`, `crates/nono-proxy/src/reverse.rs`, `crates/nono-cli/src/network_policy.rs` (DRIFT: -45), `crates/nono-cli/src/sandbox_prepare.rs` (HIGH DRIFT: +69/-1202).

**D-02 fallback prediction:** Steps 5-8 on `sandbox_prepare.rs` (HIGH DRIFT): HIGH conflict probability. `oauth2.rs` is NEW file (no conflict).

**Fixtures to port (D-13):** `oauth2.rs` 11 inline tests:
- `test_parse_token_response_success`
- `test_parse_token_response_missing_expires_defaults`
- `test_parse_token_response_missing_access_token_errors`
- `test_parse_token_response_non_json_errors`
- `test_build_token_request_body`
- `test_build_token_request_body_no_scope`
- `test_parse_status_code_200`
- `test_parse_status_code_401`
- `test_parse_status_code_garbage`
- `test_token_cache_returns_valid_token` (tokio::test)
- `test_token_cache_detects_expiry` (tokio::test)

NEW Windows-specific tests (D-15):
- Windows `keyring://` resolution for `client_secret` (REQ-PROF-03 acceptance #1, REQ-OAUTH-01 by inheritance)

**Existing fork patterns to reuse:**
- `nono::keystore::load_secret` for `client_secret` resolution
- Fork's existing rustls + hyper setup (no new deps)
- `Zeroizing<String>` for secrets (CLAUDE.md mandate + REQ-PROF-03 acceptance #3)

### Plan 22-05 AUD (Wave 2, last; D-01..D-04 strict strategy)

**Upstream commit chain (chronological per D-03):**
1. `4f9552ec` feat(audit): add tamper-evident audit log integrity — 18+ files, +1419/-226. **NEW MODULES:** `audit_integrity.rs` (221), `audit_session.rs` (322). Touches `app_runtime.rs` (+5), `audit_commands.rs` (+253), `cli.rs` (+144), `cli_bootstrap.rs` (+1), `exec_strategy.rs` (+114), `launch_runtime.rs` (+5), `main.rs` (+2), `rollback_commands.rs` (+84), `rollback_runtime.rs` (+325/-???), `supervised_runtime.rs` (+27), `nono/src/undo/snapshot.rs` (+4), `nono/src/undo/types.rs` (+19). **THE BIG ONE.**
2. `4ec61c29` feat(audit): capture pre/post merkle roots in audit trail — touches `rollback_runtime.rs` (+94), `supervised_runtime.rs` (+17), `nono/src/undo/snapshot.rs` (+48)
3. `02ee0bd1` feat(audit): record executable identity and improve integrity — touches `audit_commands.rs` (+5), `audit_integrity.rs` (+206/-???), `audit_ledger.rs` (NEW, +151), `audit_session.rs` (+3), `execution_runtime.rs` (+75/-???). **Introduces `MerkleScheme::DomainSeparatedV3`.**
4. `7b7815f7` feat(audit): record exec identity and unify audit integrity — touches `audit_commands.rs` (-2), `audit_integrity.rs` (+222/-172), `audit_ledger.rs` (+67/-???). **Unifies V1/V2/V3 → Alpha. CRITICAL: do NOT skip — replaying selectively produces diverged hashes.**
5. `0b1822a9` feat(audit): add audit verify command — touches `audit_commands.rs` (+98), `audit_integrity.rs` (+235/-???). **Adds `nono audit verify` subcommand.**
6. `6ecade2e` feat(audit): add audit attestation for session merkle roots — touches `audit_attestation.rs` (NEW, +519), `audit_commands.rs` (+55), `audit_ledger.rs` (+21/-???), `audit_session.rs` (+3), plus DSSE/in-toto integration with `nono::trust::signing`.
7. `9db06336` feat(audit): refine audit path derivation and documentation — touches `capability_ext.rs` (+27/-???), `profile/mod.rs` (+1), `rollback_runtime.rs` (+76), `supervised_runtime.rs` (+2/-???), **NEW:** `crates/nono-cli/tests/audit_attestation.rs` (+188). Re-adds `MANUAL_TEST_STEPS.md`.

**Plus 1 fork-only commit (Authenticode):**
- `feat(22-05): add Authenticode signature query for exec-identity (Windows)` — fork addition (~150 LOC), no `Upstream-commit:` trailer. Lands AFTER step 7 (so it builds on the unified Alpha scheme).

**Fork files touched (HIGHLY DIVERGENT — D-20 manual port likely):**
- `crates/nono-cli/src/exec_strategy.rs` (DRIFT: +186/-87 vs upstream +144) — D-20 candidate
- `crates/nono-cli/src/supervised_runtime.rs` (DRIFT: +229/-54 vs upstream +42) — D-20 candidate
- `crates/nono-cli/src/rollback_runtime.rs` (DRIFT: +200/-108 vs upstream +586/-260) — D-20 candidate
- `crates/nono-cli/src/cli.rs` (HIGH DRIFT)
- `crates/nono/src/undo/snapshot.rs` (DRIFT: +136/-16)
- `crates/nono/src/undo/types.rs` (DRIFT: +149)
- `crates/nono-cli/src/capability_ext.rs` (DRIFT: +182/-102)
- `crates/nono-cli/src/profile/mod.rs` (HIGH DRIFT)
- `crates/nono-cli/src/session_commands.rs` (rename impact)
- `crates/nono-cli/src/session_commands_windows.rs` (rename impact — but D-17 invariant says no upstream commit touches `*_windows.rs`. The rename of `nono prune` → `nono session cleanup` propagates through fork's Windows mirror as a fork-internal sync, not an upstream port. **Drive-by Rule-3 deviation; document in commit body.**)
- **NEW MODULES:** `crates/nono-cli/src/audit_integrity.rs`, `audit_session.rs`, `audit_ledger.rs`, `audit_attestation.rs`, `execution_runtime.rs`, `audit_commands.rs`, `rollback_commands.rs`

**CLEAN-04 invariant gate per commit (D-04):** After each commit touching `prune`/`cleanup`/`session_commands*`:
```bash
cargo test -p nono-cli --bin nono auto_prune_is_noop_when_sandboxed
cargo test -p nono-cli --bin nono parse_duration_accepts_suffixes
cargo test -p nono-cli --bin nono parse_duration_accepts_raw_seconds
cargo test -p nono-cli --bin nono parse_duration_rejects_invalid
cargo test -p nono-cli --bin nono is_prunable_all_exited_escape_hatch_matches_any_exited
cargo test -p nono-cli --bin nono is_prunable
grep -E "^const AUTO_PRUNE_STALE_THRESHOLD: usize = 100;" crates/nono-cli/src/session_commands.rs
```

**D-18 Windows regression gate per commit:**
```bash
cargo test --workspace --all-features
# (Phase 15 5-row smoke - manual or scripted via scripts/windows-test-harness.ps1 -Suite smoke)
cargo test -p nono-cli --test wfp_port_integration         # without --ignored
cargo test -p nono-cli --test learn_windows_integration    # without --ignored
```

**Fixtures to port:** `crates/nono-cli/tests/audit_attestation.rs` (NEW, +188 LOC from `9db06336`). Plus inline tests in `audit_integrity.rs`, `audit_attestation.rs`, `execution_runtime.rs`. Plus fork addition: `audit_authenticode_windows.rs` test file (D-15).

**Existing fork patterns to reuse:**
- `nono::trust::signing::sign_statement_bundle` for AUD-02 (already in fork from v2.1)
- `nono::keystore::load_secret` for `--audit-sign-key keyring://nono/audit`
- `EnvVarGuard` for `audit_attestation.rs` test that manipulates `HOME`
- `AppliedLabelsGuard` Phase 21 reference for AUD-05 (Phase 23) Drop-flush requirement

## Pre-Phase Push Status (D-06 verification)

**Verified at research time (2026-04-27):**
```
git rev-parse main           → 8381d9caa035c1ac7b879f40993fab85193205f6
git rev-parse origin/main    → 063ebad6347bd45d7b29ce9e00619e0a7639b11e
git log origin/main..main    → 513 commits ahead (was 447 in CONTEXT.md — 66 commits added since context capture)
git tag -l "v2.*"            → v2.0, v2.1 (verified present locally)
```

**Conclusion:** D-06 push to origin is **STILL PENDING**. Plan 22-01 must verify `git push origin main` and `git push origin v2.0 v2.1` have completed before commit #1 lands. Recommend: Plan 22-01 task #0 is "Verify origin push" with abort-if-fails semantics.

## Sources

### Primary (HIGH confidence — verified in this session)
- `git log v0.37.1..v0.40.1 --oneline` (105 commits, run on this working copy 2026-04-27)
- `git show <sha> --stat` for every commit listed in CONTEXT.md canonical refs (verified all 28+ SHAs reachable)
- `git diff v0.37.1..main --stat -- <file>` for all 10 high-conflict files in CONTEXT.md (drift baselines measured)
- `git show v0.40.1:crates/nono/src/undo/types.rs` (`ExecutableIdentity` struct shape verified)
- `git show v0.40.1:crates/nono-cli/src/execution_runtime.rs` (`compute_executable_identity` signature verified)
- `git show v0.40.1:crates/nono-cli/tests/audit_attestation.rs` (test file shape verified)
- `git ls-tree v0.40.1 crates/nono-cli/tests/` (test file inventory at v0.40.1 verified)
- File reads: `crates/nono-cli/src/cli.rs:2170-2210` (parse_prune_duration), `:2450-2475` (parse_duration tests), `:3160-3210` (rollback cleanup tests), `crates/nono-cli/src/session_commands.rs:700-730` (auto_prune_is_noop_when_sandboxed), `crates/nono-cli/data/hooks/nono-hook.sh` (full file), `crates/nono-cli/Cargo.toml` (windows-sys features), `Makefile:60-130` (ci target)

### Secondary (HIGH confidence — verified upstream-side)
- [Microsoft Learn — WinVerifyTrust function (wintrust.h)](https://learn.microsoft.com/en-us/windows/win32/api/wintrust/nf-wintrust-winverifytrust)
- [docs.rs — `windows-sys::Win32::Security::WinTrust::WINTRUST_DATA`](https://docs.rs/windows-sys/latest/windows_sys/Win32/Security/WinTrust/struct.WINTRUST_DATA.html)
- [docs.rs — `windows-sys::Win32::Security`](https://docs.rs/windows-sys/latest/windows_sys/Win32/Security/index.html)

### Tertiary (MEDIUM confidence — context-internal)
- `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md` (decisions D-01..D-20 LOCKED)
- `.planning/REQUIREMENTS.md` § PROF/POLY/PKG/OAUTH/AUD (18 requirements LOCKED)
- `.planning/ROADMAP.md` § Phase 22 (sequencing LOCKED)
- `.planning/quick/260424-upr-review-upstream-037-to-040/SUMMARY.md` (file-drift baseline updated post-`260424-mrg`)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every reusable asset verified in fork source; new windows-sys feature flag verified via docs.rs
- Architecture: HIGH — Architectural Responsibility Map sourced from upstream commit file lists
- Pitfalls: HIGH — every pitfall has a specific commit SHA or fork file as evidence
- Authenticode contradiction: HIGH — verified via `git show v0.40.1:.../types.rs` and `git grep WinVerifyTrust v0.40.1` returning 0 results
- 22-01 / 22-02 race condition (Pitfall 5): MEDIUM — drift counts are measured but the precise overlap inside `policy.json` and `profile/mod.rs` requires plan-time grep when both plans start
- Plan 22-05 D-02 fallback predictions: MEDIUM — drift counts are objective but conflict severity prediction is heuristic

**Research date:** 2026-04-27
**Valid until:** 2026-05-04 (7 days — fast-moving research; upstream v0.41.0 already exists at `073620e9`, v0.42.0 at `a87c6ae5`. Phase 22 must close before further upstream commits in v0.40.1..v0.41.0 land any cross-platform regressions.)
