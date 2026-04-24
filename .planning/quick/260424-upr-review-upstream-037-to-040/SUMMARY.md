---
slug: upr-review-upstream-037-to-040
status: complete
type: research-only
date: 2026-04-24
range: v0.37.1..v0.40.1
tag_head_upstream: v0.40.1 (79154fe0)
fork_baseline: v2.1 shipped (windows-squash) — UPST-01..04 = v0.37.1 parity
---

# Upstream v0.37.1 → v0.40.1 review — Windows-native impact

## Headline

**78 non-merge commits, ~9k insertions / ~600 deletions.** Zero `*_windows.rs` files touched upstream. All impact flows in through cross-platform files the fork has also modified — so the risk is merge conflicts and missing parity in Windows paths, not direct Windows regressions.

Five feature groups dominate. In priority order for Windows follow-up:

| # | Feature group                               | Upstream LOC | Windows impact                                                                              |
|---|---------------------------------------------|--------------|---------------------------------------------------------------------------------------------|
| 1 | Audit integrity + attestation               | ~1,400       | **High.** Supervisor event recording + DSSE/in-toto signing; Windows supervisor must match. |
| 2 | Package manager / packs                     | ~1,500 new   | **Medium.** New subcommands (pull/remove/search/list) + hook registration; Windows CI gap.  |
| 3 | OAuth2 proxy credential injection           | ~900         | **Medium.** `nono-proxy` is cross-platform; Windows proxy-cred flow affected.               |
| 4 | `override_deny` + `--rollback` fail-closed  | ~200         | **Low–Medium.** Behavioral CLI contract change; Windows rollback tests need updating.       |
| 5 | Env-var filtering (#688)                    | ~600         | **None — already ported** in v2.1 Phase 20 UPST (commit `e6fde898`).                        |

---

## Commit inventory by release

### v0.37.1 → v0.38.0 (24 commits, 03d32203)

Theme: **Package manager v1.** Introduces signed-artifact pull/remove/search/list, packs-as-profile-source, artifact-path hardening. Upstream also removed the `claude-code` integration package and CLAUDE.md/AGENTS.md.

- `8b46573d` feat(cli): add package management commands (pull, remove, search, list)
- `55fb42b8` feat(package): add install_dir artifact placement and hook unregistration
- `71d82cd0` feat(pack): introduce pack types and unify package naming
- `088bdad7` feat(profile): introduce packs and command_args for profiles
- `115b5cfa` feat(profile): load profiles from registry packs
- `ec49a7af` fix(package): harden package installation security
- `600ba4ec`, `58b5a24e`, `0cbb7e62`, `9ebad89a` — package refactors
- `1d49246a` feat(claude-code): **remove claude-code integration package** (breaking)
- `91c384ff` Remove CLAUDE.md and AGENTS.md
- `8b2a5ffb` fix(hooks): invoke bash via env  (**Windows hook strategy differs — N/A**)
- `ec2a2866` feat(trust): prefer CI_CONFIG_REF_URI (GitLab) — **already ported** (Phase 20 UPST-04, `af5c1249`)
- `c07c66ac` docs(cli): nix install
- `d5b1ee59` fix(trust): `TrustedRoot` compilation fix
- `f0de169b`, `58f0a4cd`, `832be3e5`, `4a1b6c87`, `1b412a74` (env vars — **already ported**, Phase 20 UPST)

### v0.38.0 → v0.39.0 (23 commits, 6a284447)

Theme: **OAuth2 credential injection + profile polish.** New `OAuth2Config` type, client-credentials token exchange with cache, `unsafe_macos_seatbelt_rules` escape hatch, `--allow-domain` strict-proxy-only behavior fixes.

- `fbf5c06e` feat(config): `OAuth2Config` for client_credentials flow
- `9546c879` feat(proxy): client_credentials token exchange with cache (557 LOC new `oauth2.rs`)
- `b1ecbc02` feat(profile): OAuth2 auth in `custom_credentials`
- `0c7fb902`, `19a0731f`, `2244dd73` — OAuth2 rebase + 413 early-return fix
- `14c644ce` feat: `unsafe_macos_seatbelt_rules` profile field (**profile struct change → Windows must deserialize**)
- `e3decf9d`, `ecd09313`, `c14e4365` — `unsafe_macos_seatbelt_rules` test/fmt follow-ups
- `10bcd054` fix(network): keep `--allow-domain` in strict proxy-only mode  (**WFP interaction**)
- `005579a9`, `d44e404e`, `60ad1eb3` — port/dry-run/test fixes around `--allow-domain`
- `3c8b6756` feat(claude): `claude-no-keychain` profile (**new builtin**)
- `b83da813` feat(policy): filter profile override-deny entries without grants (foreshadows `5c301e8d`)
- `cd1230f3` docs(agents+claude)
- `a524b1a7` fix(policy): `~/.local/share/claude/versions` allow-list entry
- `bf2e0969`, `5c4e2aea`, `4a7a5a7c`, `5987358e` — dep bumps (`clap` 4.6.1, `tokio` 1.52.1, `semver` 1.0.28, `actions/cache` 5.0.5)

### v0.39.0 → v0.40.0 (28 commits, eedc83d8)

Theme: **Audit integrity + attestation + reverse-proxy HTTP upstreams.** The largest and most architecturally significant block in the range.

- `4f9552ec` feat(audit): **tamper-evident audit log integrity** — `--audit-integrity` flag, hash-chain + Merkle root + `event_count`/`chain_head` in session metadata. Supervisor records capability decisions + URL opens. Adds `nono audit cleanup`. Deprecates `nono prune` → `nono session cleanup`. (1,419 +, 226 − across 21 files.)
- `6ecade2e` feat(audit): **attestation** — `--audit-sign-key`, DSSE/in-toto bundle, `nono audit verify --public-key-file`, new `audit_attestation.rs` module, refactors `nono::trust::signing` to export `public_key_id_hex` + `sign_statement_bundle`.
- `0b1822a9` feat(audit): `nono audit verify` command (integrity checks)
- `02ee0bd1` / `7b7815f7` feat(audit): record executable identity (signature/hash of nono binary) in audit trail
- `4ec61c29` feat(audit): capture pre/post Merkle roots in audit trail
- `9db06336` feat(audit): refine audit path derivation
- `2bf5668f` feat(reverse-proxy): add HTTP upstream support
- `0340ebff`, `b2a24402`, `0c990116` fix(proxy): insecure-HTTP upstreams restricted to local-only targets (loopback + bind-local only)
- `af19de1f` feat(rollback): refine snapshot exclusion and path tracking
- `5c301e8d` refactor(policy): **override_deny now requires matching grant** (was silently ignored); **`--no-audit` + `--rollback` now conflicts** → CLI rejects. Removes MANUAL_TEST_STEPS.md.
- `49925bbf` fix(policy): move `.claude.lock` to `allow_file` for least-privilege
- `713b2e0f` fix(policy): tests + `claude-no-kc` for `allow_file` move
- `87b20b6a`, `3b57b3b3` chore(deps): rustls-webpki 0.103.12 → 0.103.13 (v2.1 shipped 0.103.12)
- `930d82b4` fix(cli): skip non-existent profile deny overrides
- `d32ab18a` fix(sandbox): downgrade unsafe-seatbelt-rules log warn→info
- `024ca6f3` chore(cli): informational path/policy messages
- `c8b8aa9a`, `2a846d2a`, `fdc92f8d` docker/test-env moves (no code impact)
- `7a124e8d`, `8f5f15ff`, `c4131794` docs(SECURITY.md + agents)
- `228a5cf1`, `fabfc9ed` test(profiles) accuracy + missing codex coverage

### v0.40.0 → v0.40.1 (3 commits, 79154fe0)

Minor:
- `7d1d9a0d` fix(policy): improve unlink rules; add claude read path
- `94efbeb1` chore: gitignore + hiring badge

---

## Windows-native plan impact

### What the review confirms is safe

- **No `*_windows.rs`, `exec_strategy_windows/*`, `pty_proxy_windows.rs`, `trust_intercept_windows.rs`, `session_commands_windows.rs`, `windows_wfp_contract.rs`, `learn_windows.rs`, or `open_url_runtime_windows.rs` touched upstream.** All Windows-specific code in the fork is untouched by the v0.38–v0.40 range.
- Env-var filtering (`1b412a74`) is in-range but **already ported** via Phase 20 UPST-03 (`e6fde898`). Fork has `exec_strategy/env_sanitization.rs` referenced from `exec_strategy.rs` + `exec_strategy_windows/mod.rs`.
- GitLab CI_CONFIG_REF_URI (`ec2a2866`) already ported via UPST-04 (`af5c1249`).
- `--allow-gpu` already ported via UPST-04 (`ec73a8ac`).
- rustls-webpki 0.103.13 is a patch bump; no RUSTSEC-level urgency vs v2.1's shipped 0.103.12.

### High-impact Windows parity gaps introduced

1. **Audit integrity + attestation wiring (v0.40.0 — biggest risk of merge pain)**

   Upstream expanded `exec_strategy.rs` (+144), `supervised_runtime.rs` (+42), `rollback_runtime.rs` (+586), `app_runtime.rs` (+21), `launch_runtime.rs` (+21) to emit capability-decision and URL-open events to a hash-chained + Merkle-rooted ledger. Fork has significantly diverged in those same files (windows-squash carries AIPC brokering + resource limits + attach-streaming changes).

   **Windows tasks required:**
   - Windows supervisor (`exec_strategy_windows/supervisor.rs`) must emit the same capability-decision events over its capability pipe or audit sink. The AIPC broker paths (File/Socket/Pipe/JobObject/Event/Mutex) each need an audit emission.
   - `--audit-integrity` + `--audit-sign-key` flags need Windows CI matrix coverage. Signing uses `nono::trust::signing` (cross-platform — sigstore-rs) so no new Windows code path, but tests must run on `windows-latest`.
   - `nono audit verify` must work on Windows paths (long-path + UNC).
   - Executable-identity recording (`02ee0bd1`, `7b7815f7`): Windows uses a different exe-path resolution (GetModuleFileNameW + Authenticode signature vs. procfs). The audit ledger's "exec identity" record needs a Windows code path.

2. **Audit cleanup + `prune` → `session cleanup` rename**

   v2.1 Phase 19 (CLEAN-04) shipped `is_prunable` + `--older-than` parser + 100-file auto-sweep threshold + `NONO_CAP_FILE` structural no-op. Upstream v0.40.0 deprecates `nono prune`, renames to `nono session cleanup`, and adds `nono audit cleanup` as a peer. **These CLEAN-04 invariants must be preserved through the rename.** Regression tests in fork: `cleanup_*` suite — need mapping to both new subcommand paths.

3. **`--rollback` + `--no-audit` now conflicts (breaking CLI)**

   Upstream `5c301e8d` makes `--rollback` structurally require audit to be active. Fork's Windows rollback flow on `windows-squash` may have tests that pair `--rollback` with explicit `--no-audit` — these will break on port. **Search target:** `crates/nono-cli/tests/*rollback*.rs` + `tests/integration/*windows*.sh` for `--no-audit`.

4. **`override_deny` requires matching grant (breaking policy contract)**

   Upstream `5c301e8d` turns a previously-silent ignore into a fail-closed error. This aligns with nono's fail-secure philosophy but will break profiles that had orphan `override_deny` entries. **Windows profiles to audit:** `claude-code`, `claude-no-keychain` (new), any WSFG-test profiles.

5. **`unsafe_macos_seatbelt_rules` profile field (macOS-only escape hatch)**

   `14c644ce` adds `Vec<String>` field to `Profile` struct. On Windows this is a no-op applied via `add_platform_rule` (macOS-gated), but **the Profile struct must deserialize it without erroring** — otherwise every cross-platform profile breaks on Windows parse. Also surfaces in `policy show` JSON output → snapshot tests.

6. **Package manager / packs — new Windows CLI surface**

   `8b46573d`, `55fb42b8`, `71d82cd0`, `088bdad7`, `115b5cfa`, `ec49a7af`: `nono package pull/remove/search/list` + `install_dir` artifact placement + hook (un)registration + packs as profile source. ~1,500 LOC of new cross-platform code. **Windows-specific concerns:**
   - `install_dir` on Windows: LOCALAPPDATA resolution, long-path (`\\?\`) prefixing, Authenticode on downloaded artifacts.
   - Hook unregistration: fork's hook installer writes Claude Code settings via `hooks.rs`; interacts with Windows shell integration.
   - Signed-artifact streaming download (`9ebad89a`): Windows TLS trust chain (schannel vs. rustls — fork uses rustls + rustls-webpki so OK).

7. **OAuth2 proxy credential injection**

   `9546c879` (557 LOC new `oauth2.rs` in `nono-proxy`) + `b1ecbc02` (Profile `custom_credentials.oauth2`). Cross-platform by construction, but Windows proxy-cred injection path (Phase 9 WFP + Phase 11 expansion) must accept the new config shape. The token cache persists to disk — Windows path resolution must not hit the Low-IL label issues covered by WSFG-01..03.

8. **Reverse-proxy HTTP upstream restrictions**

   `2bf5668f`, `0340ebff`, `b2a24402`, `0c990116`: HTTP upstreams restricted to local-only targets (loopback + bind-local only; unspecified addresses disallowed). Fork's WFP port-level filtering (Phase 9) already enforces kernel-level port gates — upstream's proxy-level check is an additional defense-in-depth layer that should compose cleanly.

9. **`claude-code integration package removed` (`1d49246a`)**

   Upstream removed the integration package as a first-class feature. Fork's `nono-cli/data/hooks/nono-hook.sh` + `hooks.rs` flow may still depend on the package. **Impact likely low** — Windows hook install uses PowerShell/MSI — but the feature's removal upstream means merging will delete files the fork references unless we carry-forward.

10. **`--allow-domain` strict proxy-only mode (`10bcd054`)**

    Previously `--allow-domain` silently dropped in strict proxy-only mode. Now preserved. Windows WFP + proxy interaction (Phase 9) should already honor this (WFP gates by IP, proxy gates by host) — spot-check in `net_filter_windows` test fixtures.

### Low-impact / informational

- `d32ab18a` — `unsafe_macos_seatbelt_rules` log level (macOS only).
- `d44e404e`, `60ad1eb3` — fmt + dry-run cosmetic.
- `024ca6f3` — message tone.
- All `chore(deps)` bumps — safe.
- `91c384ff` (remove CLAUDE.md/AGENTS.md) — fork carries its own CLAUDE.md.
- `c4131794`, `7a124e8d`, `8f5f15ff` — docs only.

---

## Files with highest merge-conflict likelihood on next sync

Fork has modified all of these on `windows-squash`; upstream has also modified them in-range:

| File                                                | Upstream Δ    | Fork notes                                                         |
|-----------------------------------------------------|---------------|--------------------------------------------------------------------|
| `crates/nono-cli/src/profile/mod.rs`                | +635/-~30     | v2.1 AIPC profile widening (Phase 18.1-03), RESL field.            |
| `crates/nono-cli/src/rollback_runtime.rs`           | +586/-~260    | v2.1 snapshot+label lifecycle (Phase 21 `AppliedLabelsGuard`).     |
| `crates/nono-cli/src/policy.rs`                     | +162/-~40     | v2.1 `never_grant` + group expansion + WSFG mode encoding.         |
| `crates/nono-cli/src/network_policy.rs`             | +171/-~10     | v2.0 Phase 9 WFP port-level.                                       |
| `crates/nono-cli/src/exec_strategy.rs`              | +144/-~30     | v2.0 Direct/Monitor/Supervised branching; AIPC wiring.             |
| `crates/nono-cli/src/profile/builtin.rs`            | +104/-~5      | Fork has Windows-specific builtin profile gating.                  |
| `crates/nono-cli/src/rollback_commands.rs`          | +96/-~10      | v2.1 rollback UI on Windows.                                       |
| `crates/nono-cli/src/sandbox_prepare.rs`            | +62/-~10      | v2.1 `--allow-gpu` 3-platform dispatch (Phase 20 UPST-04).         |
| `crates/nono-cli/src/supervised_runtime.rs`         | +42/-~10      | v2.1 `SupervisedRuntimeContext.loaded_profile` + AIPC allowlist.   |
| `crates/nono/src/undo/snapshot.rs`, `undo/types.rs` | +149/-~10     | ObjectStore clone_or_copy + Merkle wiring on Windows.              |
| `crates/nono/src/trust/signing.rs`                  | +17           | v2.1 Phase 20 UPST-04 already ported adjacent changes.             |

**Recommendation:** Upstream-parity work will need a cherry-pick-per-commit strategy (as Phase 20 used — `Upstream-commit:` trailer) rather than a bulk merge. The audit-integrity cluster (`4f9552ec` → `4ec61c29` → `02ee0bd1` → `7b7815f7` → `0b1822a9` → `6ecade2e` → `9db06336`) should be sequenced together as a single plan.

---

## Recommended v2.2 scope additions

The v2.1 shipping note in `PROJECT.md § Active (v2.2+)` lists four candidates: WR-01 reject-stage unification, AIPC G-04 compile-time tightening, cross-platform RESL Unix backends, WR-02 EDR UAT, merge `windows-squash` to main. This review suggests **three more**:

### Proposed Phase 22 — Upstream v0.38–v0.40 Parity Sync (UPST2)

Port the non-dependent feature groups first; audit-integrity last. Estimated 5 plans, 2 waves:

- **22-01 Profile struct alignment** — deserialize `unsafe_macos_seatbelt_rules` + `packs` + `command_args` + `oauth2.custom_credentials`. Add `claude-no-keychain` builtin. Verify Windows profile parse.
- **22-02 Policy tightening** — `override_deny` requires matching grant; `--rollback` + `--no-audit` conflict; `.claude.lock` allow_file move. Fork's Windows rollback tests updated.
- **22-03 Package manager + packs** — port `package` subcommand tree; Windows `install_dir` + hook unregistration; signed-artifact streaming. Decide fate of `claude-code integration package` removal.
- **22-04 OAuth2 proxy + reverse-proxy upstreams** — port `OAuth2Config`, `oauth2.rs`, `reverse.rs` HTTP-upstream gating. Windows WFP port-level interaction spot-check.
- **22-05 Audit integrity + attestation** — port the 7-commit audit cluster. Windows supervisor emits capability-decision + URL-open events to ledger; exec-identity recording uses Windows Authenticode path; CI matrix adds `windows-latest` for `--audit-integrity` + `--audit-sign-key`. **Must preserve CLEAN-04 invariants through `prune` → `session cleanup` rename.**

### Proposed Phase 23 — Windows audit-event coverage retrofit (if 22-05 reveals gaps)

Only create if 22-05 discovers AIPC broker paths or WFP activation events not covered by upstream's supervisor-record shape. The v2.1 AIPC brokering is rich in security-relevant decisions that would benefit from inclusion in the tamper-evident ledger.

### Sequencing note

Merge `windows-squash` to `main` **before** starting Phase 22. Two reasons:
1. Audit-integrity cluster rewrites `rollback_runtime.rs` and `supervised_runtime.rs` — resolving those conflicts inside an unreleased integration branch is expensive compared to doing it on mainline.
2. Upstream CI on `main` gives parity-sync work a stable baseline to cherry-pick onto.

---

## Data & commands used

```
git fetch upstream --tags
git log --oneline v0.37.1..v0.40.1 --no-merges            # 78 commits
git diff --stat v0.37.1..v0.40.1                          # 58 files, +8942/-611
git diff --name-only v0.37.1..v0.40.1 | grep -iE '(windows|wfp|conpty|etw)'  # 3 cross-platform hits only; 0 *_windows.rs
git log --oneline v0.37.1..v0.40.1 --no-merges -- <fork-modified-paths>
```

Already-ported commits confirmed via `git log --grep=...` on `windows-squash`:
- `1b412a74` env-var filtering → fork `e6fde898` (Phase 20 UPST-03)
- `ec2a2866` GitLab CI_CONFIG_REF_URI → fork `af5c1249` (Phase 20 UPST-04)
- `--allow-gpu` (adjacent to range, pre-v0.37.1) → fork `ec73a8ac` (Phase 20 UPST-04)
