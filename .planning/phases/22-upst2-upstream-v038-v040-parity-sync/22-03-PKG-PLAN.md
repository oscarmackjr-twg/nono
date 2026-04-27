---
phase: 22-upst2-upstream-v038-v040-parity-sync
plan: 03
type: execute
wave: 1
depends_on: ["22-01"]
blocks: []
files_modified:
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/package_cmd.rs
  - crates/nono-cli/src/registry_client.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/hooks.rs
  - crates/nono-cli/data/policy.json
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/tests/package_integration.rs
autonomous: true
requirements: ["PKG-01", "PKG-02", "PKG-03", "PKG-04"]

must_haves:
  truths:
    - "`nono package pull <name>` downloads + installs to %LOCALAPPDATA%\\nono\\packages\\<name> on Windows; macOS/Linux use existing dirs-crate analog (PKG-01)"
    - "`nono package remove <name>` deletes installed package + unregisters its hooks idempotently (PKG-01)"
    - "`nono package search <query>` returns registry results; `nono package list` enumerates installed packages (PKG-01)"
    - "Long paths use \\\\?\\ prefix when path length > 260 chars (Windows MAX_PATH); package install handles names exceeding standard limits (PKG-02 acceptance #2)"
    - "Path traversal (`..`, symlinks aliasing outside install_dir, UNC aliasing) rejected fail-closed via canonicalization + path component comparison, NOT string starts_with (PKG-02 acceptance #3)"
    - "Hook installer registers via fork's existing hooks.rs Windows path; idempotent re-install is a no-op (PKG-03)"
    - "Streaming download verifies signed-artifact signature before install; tampered artifact rejected fail-closed (PKG-04)"
    - "Trust bundle resolution uses centralized fork pattern (post-600ba4ec)"
    - "Every cherry-pick commit body contains D-19 trailers (Upstream-commit/tag/author + Signed-off-by)"
    - "`cargo test --workspace --all-features` exits 0 on Windows after each commit (D-18)"
  artifacts:
    - path: "crates/nono-cli/src/package_cmd.rs"
      provides: "nono package pull/remove/search/list subcommand handlers (NEW)"
    - path: "crates/nono-cli/src/registry_client.rs"
      provides: "Registry HTTP client + signed-artifact verification (NEW)"
    - path: "crates/nono-cli/src/cli.rs"
      provides: "package subcommand tree (Pull/Remove/Search/List variants in clap enum)"
    - path: "crates/nono-cli/src/hooks.rs"
      provides: "Idempotent hook install/uninstall during package install/remove"
  key_links:
    - from: "ROADMAP § Phase 22 success criterion #3"
      to: "nono package CLI behavior + %LOCALAPPDATA% install path"
      via: "cross-platform package management with Windows long-path handling"
      pattern: "package_cmd|registry_client|LOCALAPPDATA|\\\\\\\\\\?\\\\"
---

<objective>
Land the upstream `nono package pull/remove/search/list` subcommand tree (PKG-01..04) into the fork. Adds a new package management subcommand surface, registry HTTP client with streaming + signed-artifact verification, and Windows-specific install_dir resolution under `%LOCALAPPDATA%\nono\packages\<name>` with `\\?\` long-path handling and path-traversal rejection. Wave 1 plan — runs after Plan 22-01 closes per D-09 because `Profile::packs: Vec<PackRef>` from PROF-02 is the type packages register against.

Purpose: A Windows user runs `nono package pull <name>` / `remove` / `search` / `list` and sees identical behavior to a macOS user on the same registry, with artifacts landing under `%LOCALAPPDATA%\nono\packages\<name>` and hooks registered through Claude Code (REQ-PKG-01..04).
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md
@.planning/REQUIREMENTS.md
@.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md
@.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-RESEARCH.md
@.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-PATTERNS.md
@.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-VALIDATION.md
@crates/nono-cli/src/cli.rs
@crates/nono-cli/src/hooks.rs
@crates/nono-cli/data/hooks/nono-hook.sh
@crates/nono-cli/data/policy.json

<interfaces>
**Upstream cherry-pick chain (chronological per D-03):**

| Order | SHA | Upstream subject | REQ |
|-------|-----|------------------|-----|
| 1 | `8b46573d` | feat(cli): add package management commands (~+2384 LOC main.rs restructure per RESEARCH) | PKG-01 |
| 2 | `55fb42b8` | feat(package): add install_dir artifact placement and hook unregistration | PKG-01 + PKG-03 |
| 3 | `71d82cd0` | feat(pack): introduce pack types and unify package naming | PKG-01 |
| 4 | `ec49a7af` | fix(package): harden package installation security | PKG-02 + PKG-04 |
| 5 | `9ebad89a` | refactor(pkg): stream package artifact downloads | PKG-04 |
| 6 | `600ba4ec` | refactor(package-cmd): centralize trust bundle | PKG-04 |
| 7 | `58b5a24e` | package refactor (supplementary) | PKG-* |
| 8 | `0cbb7e62` | package refactor (supplementary) | PKG-* |

**Fork-only Windows additions (D-15):**
- `%LOCALAPPDATA%\nono\packages\<name>` install_dir resolution via `dirs::data_local_dir()` (cross-platform; on Windows resolves to `%LOCALAPPDATA%`).
- `\\?\` long-path prefix when joined path exceeds Windows MAX_PATH (260 chars). Detect via path length check; prepend `\\?\` only when needed.
- `#[cfg(target_os = "windows")]`-gated tests for both behaviors.

**D-17 caveat:** Upstream `8b5a2ffb` (correct SHA per RESEARCH finding #2 — was `8b2a5ffb` typo in CONTEXT) "fix(hooks): invoke bash via env" is N/A on Windows. If a cherry-pick attempt picks it up, ABORT (D-17 violation candidate). Verify fork's `nono-cli/data/hooks/nono-hook.sh` already handles the relevant path correctly.

**D-17 caveat (continued):** Upstream `1d49246a` "claude-code integration package removal" is followed by the fork. Verify fork's `hooks.sh` doesn't reference removed surfaces before Plan 22-03 lands its hook installer per CONTEXT § Known Risks.

**No external test fixture files (RESEARCH finding #6):** Package ships inline tests in `package_cmd.rs`/`package.rs` upstream. D-14 fixture port = port these inline tests verbatim where they fit fork's structure.

**D-19 commit body template (cherry-pick path):** Same as Plan 22-01.

**D-20 commit body template (manual-port fallback):** Same as Plan 22-01.

**RESEARCH finding for 8b46573d:** This commit restructures `main.rs` with a +2384 LOC delta. D-02 fallback is HIGH probability. Plan ahead for manual-port: fork's `main.rs` + clap `Cli` enum may diverge significantly from upstream's pre-8b46573d state due to fork's Phase 9/15/18.1/19/20/21 additions.

**Pattern map analogs (PATTERNS.md):**
- Package subcommand tree follows existing clap subcommand pattern (`nono session`, `nono audit`).
- Streaming HTTP client follows existing hyper client usage in `nono-proxy/src/server.rs`.
- Long-path `\\?\` prefix follows existing usage in `crates/nono-cli/src/learn_windows.rs` or `pty_proxy_windows.rs`.
- `dirs::data_local_dir()` for `%LOCALAPPDATA%` resolution: idiomatic cross-platform pattern.
- Path canonicalization + component comparison per CLAUDE.md § Path Handling — fork already enforces this elsewhere.

**Coordination caveat:** Plan 22-04 OAUTH wave-parallels with 22-03. They touch disjoint surfaces (22-03: package_cmd.rs, registry_client.rs, hooks.rs; 22-04: nono-proxy/oauth2.rs, reverse.rs). No expected coordination needed.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: D-17 preflight — verify fork's hooks.sh + 1d49246a removal hygiene</name>
  <files>(read-only audit — no files modified)</files>
  <read_first>
    - crates/nono-cli/data/hooks/nono-hook.sh (full content)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § Known Risks "claude-code integration package removal"
  </read_first>
  <action>
    1. Verify fork's `nono-hook.sh` does NOT reference `claude-code integration package` (removed upstream `1d49246a`):
       ```
       grep -nE 'claude-code.*integration.*package|integration.package' crates/nono-cli/data/hooks/nono-hook.sh
       ```
       If hits returned: STOP per CONTEXT STOP trigger #9. Investigate before proceeding.

    2. Verify fork's `nono-hook.sh` does NOT use bare `bash` invocation that upstream `8b5a2ffb` (corrected SHA per RESEARCH finding #2) replaced with `env bash`:
       ```
       grep -nE '^#!/bin/bash|^#!bash|/bin/bash ' crates/nono-cli/data/hooks/nono-hook.sh
       ```
       Note: fork uses Windows hook strategy that doesn't depend on `bash` shebang per CONTEXT § Known Risks. Hits here = expected absence; if present, fork's hook strategy needs review.

    3. Inventory fork's existing `hooks.rs`:
       ```
       grep -nE 'pub fn (install|uninstall|register|unregister)' crates/nono-cli/src/hooks.rs
       ```
       Confirm fork already has hook install/uninstall pattern that Plan 22-03 Task 4 can extend.

    4. Record findings in preflight note (for SUMMARY).
  </action>
  <verify>
    <automated>! grep -qE 'claude-code.*integration.*package' crates/nono-cli/data/hooks/nono-hook.sh &amp;&amp; grep -nE 'pub fn (install|uninstall|register|unregister)' crates/nono-cli/src/hooks.rs</automated>
  </verify>
  <acceptance_criteria>
    - `grep -nE 'claude-code.*integration.*package' crates/nono-cli/data/hooks/nono-hook.sh` returns 0 hits.
    - Fork's `hooks.rs` has install/uninstall functions (or equivalent) that 22-03 Task 4 can reuse.
    - Preflight note recorded for SUMMARY.
  </acceptance_criteria>
  <done>
    Fork's hook surface is clean of upstream-removed references; D-17 risk for 8b5a2ffb / 1d49246a documented.
  </done>
</task>

<task type="auto">
  <name>Task 2: Cherry-pick `8b46573d` — package management commands subcommand tree (PKG-01)</name>
  <files>
    crates/nono-cli/src/cli.rs
    crates/nono-cli/src/main.rs
    crates/nono-cli/src/package_cmd.rs (NEW)
    crates/nono-cli/src/registry_client.rs (NEW — may be split per upstream)
    crates/nono-cli/Cargo.toml (likely adds dirs crate dep + http client dep if not already in fork)
  </files>
  <read_first>
    - `git show 8b46573d --stat` (anticipate the +2384 LOC restructure)
    - `git show 8b46573d -- crates/nono-cli/src/main.rs` (read full upstream diff before cherry-pick)
    - crates/nono-cli/src/cli.rs (current `Cli` enum shape — count subcommands; package will be a new variant)
    - crates/nono-cli/src/main.rs (current command dispatch shape)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-PATTERNS.md "Package manager subcommand tree" analog
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-RESEARCH.md (D-02 fallback prediction for 8b46573d)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 8b46573d
       ```

    2. **D-02 fallback gate (HIGH PROBABILITY for this commit per RESEARCH).** main.rs has +2384 upstream LOC delta and fork has Phase 9/15/18.1/19/20/21 additions interleaved.
       ```
       git diff --name-only --diff-filter=U
       grep -c '<<<<<<<' crates/nono-cli/src/main.rs crates/nono-cli/src/cli.rs 2>/dev/null
       ```
       Manual-port (D-20) is the expected path:
       - `git cherry-pick --abort`
       - Manually create new file `crates/nono-cli/src/package_cmd.rs` containing upstream's package command surface (read from `git show 8b46573d -- crates/nono-cli/src/package_cmd.rs` or wherever upstream split it).
       - Manually add `Package(PackageCmd)` variant to fork's `Cli` enum in `cli.rs`.
       - Manually add `Commands::Package(args) => package_cmd::run(args).await,` (or sync equivalent) to `main.rs` dispatch.
       - Manually create `registry_client.rs` if upstream split it out.
       - **Reset upstream's `packages/claude-code/*` files (which fork doesn't carry) to deleted in same commit per RESEARCH open question #3.**

    3. **Cargo.toml additions:** check if fork already has these dependencies — if not, add only what's needed:
       - `dirs` (for `data_local_dir()`)
       - `reqwest` or `hyper`/`hyper-tls` for HTTP client (likely already in fork via nono-proxy; verify before adding to nono-cli)
       - `tar` / `flate2` / `zip` for archive extraction (verify upstream's choice)

    4. Verify build:
       ```
       cargo build --workspace
       cargo run -p nono-cli -- package --help   # should print package subcommand help
       ```

    5. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-03): add package management commands (PKG-01)

       Adds nono package pull/remove/search/list subcommand tree. Manual-port
       fallback applied per D-02/D-20: main.rs +2384 LOC restructure conflicts
       with fork's Phase 9/15/18.1/19/20/21 additions. Replayed upstream's
       package_cmd.rs + registry_client.rs surface; preserved fork's existing
       command dispatch shape. Reset upstream's packages/claude-code/* files
       (not in fork).

       Upstream-commit: 8b46573d (replayed manually)
       Upstream-tag: v0.38.0
       Upstream-author: <capture from `git log -1 8b46573d --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
       (If cherry-pick path was clean, drop `(replayed manually)` and use D-19 template instead.)
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; cargo run -p nono-cli -- package --help 2&gt;&amp;1 | grep -E 'pull|remove|search|list' &amp;&amp; test -f crates/nono-cli/src/package_cmd.rs &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 8b46573d'</automated>
  </verify>
  <acceptance_criteria>
    - `crates/nono-cli/src/package_cmd.rs` exists.
    - `cargo run -p nono-cli -- package --help` lists `pull`, `remove`, `search`, `list` subcommands.
    - `cargo build --workspace` exits 0.
    - `git log -1 --format=%B | grep '^Upstream-commit: 8b46573d'` returns 1 line.
    - No `<capture from` placeholders.
  </acceptance_criteria>
  <done>
    Package subcommand tree scaffolded; nono package CLI surface available.
  </done>
</task>

<task type="auto">
  <name>Task 3: Cherry-pick `55fb42b8` — install_dir artifact placement + hook unregistration (PKG-01 + PKG-03)</name>
  <files>
    crates/nono-cli/src/package_cmd.rs
    crates/nono-cli/src/hooks.rs
  </files>
  <read_first>
    - `git show 55fb42b8 --stat` and full diff
    - crates/nono-cli/src/hooks.rs (Task 1 inventory)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 55fb42b8
       ```
       D-02 gate.

    2. **Windows install_dir resolution.** Verify (or add) the install_dir resolves correctly:
       ```rust
       // package_cmd.rs install_dir() should resolve to:
       // - Windows: %LOCALAPPDATA%\nono\packages\<name> (via dirs::data_local_dir())
       // - macOS:   ~/Library/Application Support/nono/packages/<name>
       // - Linux:   ~/.local/share/nono/packages/<name>
       ```
       If upstream uses a different convention, deviate to match `dirs::data_local_dir()` per REQ-PKG-02 acceptance #1.

    3. **Hook unregistration on remove.** Verify `package remove <name>` calls `hooks::unregister(<package>)` idempotently:
       ```
       grep -E 'unregister|uninstall' crates/nono-cli/src/package_cmd.rs
       ```

    4. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-03): add install_dir artifact placement and hook unregistration

       Resolves package install dir via dirs::data_local_dir() for cross-platform
       %LOCALAPPDATA%/Application Support/.local/share consistency. nono package
       remove <name> unregisters hooks idempotently (REQ-PKG-03 acceptance).

       Upstream-commit: 55fb42b8
       Upstream-tag: v0.38.0
       Upstream-author: <capture from `git log -1 55fb42b8 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    5. Verify:
       ```
       cargo build --workspace
       cargo test -p nono-cli package_cmd:: 2>&1 | tail
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; grep -E 'data_local_dir|LOCALAPPDATA' crates/nono-cli/src/package_cmd.rs &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 55fb42b8'</automated>
  </verify>
  <acceptance_criteria>
    - `grep -E 'data_local_dir' crates/nono-cli/src/package_cmd.rs` returns ≥ 1 hit (Windows install_dir uses dirs crate).
    - Hook unregistration code path exists in `package_cmd.rs::remove` flow.
    - `cargo build --workspace` exits 0.
    - `git log -1 --format=%B | grep '^Upstream-commit: 55fb42b8'` returns 1 line.
  </acceptance_criteria>
  <done>
    install_dir resolves correctly cross-platform; hook unregistration idempotent.
  </done>
</task>

<task type="auto">
  <name>Task 4: Cherry-pick `71d82cd0` — pack types + unified package naming (PKG-01)</name>
  <files>
    crates/nono-cli/src/package_cmd.rs
    crates/nono-cli/src/profile/mod.rs (PackRef type — coordinates with Plan 22-01 PROF-02)
  </files>
  <read_first>
    - `git show 71d82cd0 --stat` and full diff
    - crates/nono-cli/src/profile/mod.rs (PackRef type from Plan 22-01 Task 4 — already landed at this point per Wave dependency)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 71d82cd0
       ```
       D-02 gate.

    2. **Coordinate with Plan 22-01 PROF-02 PackRef:** This commit unifies pack types. Plan 22-01 Task 4 already landed `088bdad7`/`115b5cfa`. Verify the PackRef type definition is consistent — if upstream's `71d82cd0` adjusts PackRef shape, ensure profile/mod.rs's reference still compiles.

    3. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-03): introduce pack types and unify package naming

       Aligns PackRef type usage between profile (Plan 22-01) and package
       (Plan 22-03) so profiles can declare packs and the package manager
       can resolve them through the same naming convention.

       Upstream-commit: 71d82cd0
       Upstream-tag: v0.38.0
       Upstream-author: <capture from `git log -1 71d82cd0 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    4. Verify:
       ```
       cargo build --workspace
       cargo test -p nono-cli profile::tests::deserialize_packs_and_command_args   # 22-01-T2 still green
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; cargo test -p nono-cli profile:: &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 71d82cd0'</automated>
  </verify>
  <acceptance_criteria>
    - `cargo build --workspace` exits 0.
    - Plan 22-01 PROF-02 tests still green (no PackRef ABI break).
    - `git log -1 --format=%B | grep '^Upstream-commit: 71d82cd0'` returns 1 line.
  </acceptance_criteria>
  <done>
    Pack types unified between profile and package surfaces.
  </done>
</task>

<task type="auto">
  <name>Task 5: Cherry-pick `ec49a7af` — harden package installation security (PKG-02 + PKG-04)</name>
  <files>
    crates/nono-cli/src/package_cmd.rs
    crates/nono-cli/src/registry_client.rs
  </files>
  <read_first>
    - `git show ec49a7af --stat` and full diff
    - REQUIREMENTS.md § PKG-02 (path traversal rejection: `..`, symlinks, UNC aliasing)
    - REQUIREMENTS.md § PKG-04 (signed-artifact verification)
    - CLAUDE.md § Path Handling (path component comparison, NOT string starts_with; canonicalize at boundary)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick ec49a7af
       ```
       D-02 gate.

    2. **PKG-02 path traversal verification.** Confirm upstream's hardening uses path component comparison:
       ```
       grep -E 'starts_with|components|canonicalize' crates/nono-cli/src/package_cmd.rs
       ```
       Per CLAUDE.md § Common Footguns #1 ("string `starts_with()` on paths is a vulnerability"): if upstream uses `&str::starts_with` on a path, this is a fork-only fix (replace with `Path::starts_with` on `Path::components()`).

    3. **PKG-04 signed-artifact verification.** Confirm signature verification happens BEFORE extraction:
       ```
       grep -nE 'verify|signature' crates/nono-cli/src/package_cmd.rs crates/nono-cli/src/registry_client.rs
       ```
       Trace the install flow: download → verify → extract. Tampered artifact must reject before any filesystem write.

    4. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       fix(22-03): harden package installation security (PKG-02, PKG-04)

       Path-traversal rejection via canonicalize + Path::components comparison
       (NOT string starts_with — CLAUDE.md § Common Footguns #1). Signed-artifact
       verification gates extraction so tampered archives reject before any
       filesystem write.

       Upstream-commit: ec49a7af
       Upstream-tag: v0.38.0
       Upstream-author: <capture from `git log -1 ec49a7af --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    5. If upstream uses string `starts_with` (security footgun), add a fork-only follow-up commit:
       ```
       fix(22-03): replace string starts_with with Path::components comparison

       Per CLAUDE.md § Common Footguns #1, string starts_with on paths is
       a vulnerability (e.g., "/home" matches "/homeevil"). Replace with
       Path::components() iteration for path-traversal rejection.

       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; cargo test -p nono-cli package_cmd:: 2&gt;&amp;1 | grep -E 'path_traversal|signed_artifact' &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: ec49a7af'</automated>
  </verify>
  <acceptance_criteria>
    - `grep -E 'canonicalize' crates/nono-cli/src/package_cmd.rs` returns ≥ 1 hit.
    - `grep -E 'verify.*signature|verify_signature' crates/nono-cli/src/registry_client.rs crates/nono-cli/src/package_cmd.rs` returns ≥ 1 hit.
    - **Path-component-comparison invariant (CLAUDE.md § Common Footguns #1):** Every callsite of `.starts_with(` in `crates/nono-cli/src/package_cmd.rs` must satisfy ONE of: (a) operates on `Path` (not `&str`/`String`) — verify with `grep -E 'Path::new\(.*\)\.starts_with\(' crates/nono-cli/src/package_cmd.rs`; (b) operates on a non-path string (e.g., URL scheme prefix check) AND has an inline `// SAFETY: not a path comparison — <reason>` comment within 3 lines; (c) returns 0 hits (no `.starts_with(` calls at all). Verify via: `grep -E '\.starts_with\(' crates/nono-cli/src/package_cmd.rs` then audit each hit per the rules above.
    - `cargo build --workspace` exits 0.
    - `git log -1 --format=%B | grep '^Upstream-commit: ec49a7af'` returns 1 line.
  </acceptance_criteria>
  <done>
    PKG-02 path-traversal hardening + PKG-04 signed-artifact verification landed.
  </done>
</task>

<task type="auto">
  <name>Task 6: Cherry-pick `9ebad89a` — stream package artifact downloads (PKG-04)</name>
  <files>
    crates/nono-cli/src/registry_client.rs
    crates/nono-cli/src/package_cmd.rs
  </files>
  <read_first>
    - `git show 9ebad89a --stat` and full diff
    - REQUIREMENTS.md § PKG-04 (streaming download required for large packages)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 9ebad89a
       ```
       D-02 gate.

    2. **Verify streaming, not buffered.** Streaming download must NOT load the entire artifact into memory:
       ```
       grep -nE 'AsyncRead|stream|Bytes::|BodyExt' crates/nono-cli/src/registry_client.rs
       ```
       If upstream uses `bytes()` (buffered) instead of `into_data_stream()` or equivalent, that's a memory-bomb risk — fork should preserve streaming.

    3. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       refactor(22-03): stream package artifact downloads

       Replaces buffered download with streaming read; large packages no longer
       load entirely into memory before write-to-disk. Streaming preserves
       PKG-04 acceptance for arbitrary-size artifacts.

       Upstream-commit: 9ebad89a
       Upstream-tag: v0.38.0
       Upstream-author: <capture from `git log -1 9ebad89a --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 9ebad89a'</automated>
  </verify>
  <acceptance_criteria>
    - `grep -E 'stream|AsyncRead|chunk' crates/nono-cli/src/registry_client.rs` returns ≥ 1 hit indicating streaming I/O.
    - `cargo build --workspace` exits 0.
    - `git log -1 --format=%B | grep '^Upstream-commit: 9ebad89a'` returns 1 line.
  </acceptance_criteria>
  <done>
    PKG-04 streaming download landed.
  </done>
</task>

<task type="auto">
  <name>Task 7: Cherry-pick `600ba4ec`, `58b5a24e`, `0cbb7e62` — trust bundle centralization + supplementary refactors</name>
  <files>(varies — read upstream stat per commit)</files>
  <read_first>
    - `git show 600ba4ec --stat`, `git show 58b5a24e --stat`, `git show 0cbb7e62 --stat`
  </read_first>
  <action>
    Cherry-pick each in chronological order per D-03; one atomic commit per upstream SHA:
    ```
    git cherry-pick 600ba4ec
    # D-02 gate
    git commit --amend -s -m "$(cat <<'EOF'
    refactor(22-03): centralize trust bundle (PKG-04)

    <2-3 line context>

    Upstream-commit: 600ba4ec
    Upstream-tag: v0.38.0
    Upstream-author: <capture from `git log -1 600ba4ec --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"

    git cherry-pick 58b5a24e
    git commit --amend -s -m "$(cat <<'EOF'
    refactor(22-03): <subject from upstream 58b5a24e>

    Upstream-commit: 58b5a24e
    Upstream-tag: <git describe --tags 58b5a24e>
    Upstream-author: <capture from `git log -1 58b5a24e --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"

    git cherry-pick 0cbb7e62
    git commit --amend -s -m "$(cat <<'EOF'
    refactor(22-03): <subject from upstream 0cbb7e62>

    Upstream-commit: 0cbb7e62
    Upstream-tag: <git describe --tags 0cbb7e62>
    Upstream-author: <capture from `git log -1 0cbb7e62 --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"
    ```

    Per-commit spot check: `cargo test --workspace --lib`. STOP if red.
  </action>
  <verify>
    <automated>git log -3 --format='%b' | grep -E '^Upstream-commit: (600ba4ec|58b5a24e|0cbb7e62)' | wc -l &amp;&amp; cargo build --workspace</automated>
  </verify>
  <acceptance_criteria>
    - `git log -3 --format=%B | grep -E '^Upstream-commit: (600ba4ec|58b5a24e|0cbb7e62)' | wc -l` returns `3`.
    - `cargo build --workspace` exits 0.
    - Each commit body has 1 Signed-off-by line.
  </acceptance_criteria>
  <done>
    Trust bundle centralized + supplementary refactors landed. Full upstream PKG cluster complete.
  </done>
</task>

<task type="auto">
  <name>Task 8: Add Windows-specific tests (D-15) — long-path + path-traversal + Credential Manager keyring</name>
  <files>
    crates/nono-cli/tests/package_integration.rs (NEW)
  </files>
  <read_first>
    - REQUIREMENTS.md § PKG-02 acceptance #2 (long-path), #3 (path-traversal)
    - REQUIREMENTS.md § PROF-03 acceptance #1 (Credential Manager `keyring://` resolution — coordinates with Plan 22-01)
    - .planning/phases/21-windows-single-file-grants/21-CONTEXT.md (Windows-only test pattern: `#[cfg(target_os = "windows")]`)
    - crates/nono/src/sandbox/windows.rs::tests (76 tests — pattern reference)
  </read_first>
  <action>
    1. Create `crates/nono-cli/tests/package_integration.rs` with `#[cfg(target_os = "windows")]`-gated tests:

       ```rust
       //! Windows-specific package integration tests (REQ-PKG-02 acceptance #2, #3)

       #![cfg(target_os = "windows")]

       use std::path::PathBuf;

       #[test]
       fn install_dir_uses_localappdata_on_windows() {
           let install_dir = nono_cli::package_cmd::resolve_install_dir("test-pkg")
               .expect("install_dir resolution should succeed");
           let local_appdata = std::env::var("LOCALAPPDATA").expect("LOCALAPPDATA must be set on Windows");
           assert!(
               install_dir.starts_with(PathBuf::from(local_appdata).join("nono").join("packages")),
               "install_dir must be under %LOCALAPPDATA%\\nono\\packages, got: {:?}",
               install_dir
           );
       }

       #[test]
       fn long_path_prefix_applied_when_exceeds_max_path() {
           // Construct a name that pushes the joined path > 260 chars
           let long_name = "a".repeat(280);
           let resolved = nono_cli::package_cmd::resolve_install_dir(&long_name)
               .expect("long-name install_dir resolution should succeed");
           let s = resolved.to_string_lossy();
           assert!(
               s.starts_with(r"\\?\") || s.len() <= 260,
               "expected \\\\?\\ prefix on long path or path within MAX_PATH, got: {} chars: {}",
               s.len(),
               s
           );
       }

       #[test]
       fn path_traversal_rejected_dotdot() {
           // ".." in artifact path must be rejected by canonicalize-and-prefix-check
           let result = nono_cli::package_cmd::install_artifact_safe(
               "../../../etc/passwd",
               &PathBuf::from(r"C:\test\install_dir"),
           );
           assert!(result.is_err(), "expected path-traversal rejection for ../../../etc/passwd");
       }

       #[test]
       fn path_traversal_rejected_unc_alias() {
           // \\?\C:\... is canonical; a UNC alias attempting to redirect must reject
           let result = nono_cli::package_cmd::install_artifact_safe(
               r"\\?\GLOBALROOT\Device\HarddiskVolume1\Windows",
               &PathBuf::from(r"C:\test\install_dir"),
           );
           assert!(result.is_err(), "expected UNC alias rejection");
       }
       ```

       Note: function names (`resolve_install_dir`, `install_artifact_safe`) must match upstream's actual API. Adjust per Task 2-7 outcome.

    2. Verify tests compile:
       ```
       cargo test -p nono-cli --test package_integration --no-run
       ```

    3. Run on Windows host:
       ```
       cargo test -p nono-cli --test package_integration
       ```

    4. Commit (fork-only — no Upstream-commit trailer):
       ```
       git add crates/nono-cli/tests/package_integration.rs
       git commit -s -m "$(cat <<'EOF'
       test(22-03): add Windows-specific package integration tests (D-15)

       Covers REQ-PKG-02 acceptance #2 (long-path \\?\ prefix) and #3
       (path-traversal rejection: .., UNC aliasing). Inherits Phase 21 WSFG
       Windows-only test pattern (#[cfg(target_os = "windows")]).

       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
  </action>
  <verify>
    <automated>cargo test -p nono-cli --test package_integration --no-run &amp;&amp; cargo test -p nono-cli --test package_integration 2&gt;&amp;1 | tail</automated>
  </verify>
  <acceptance_criteria>
    - `crates/nono-cli/tests/package_integration.rs` exists and compiles.
    - Windows-only tests pass on Windows host (or documented-skip on non-Windows).
    - Commit has Signed-off-by trailer.
  </acceptance_criteria>
  <done>
    D-15 Windows-specific PKG tests landed.
  </done>
</task>

<task type="auto">
  <name>Task 9: D-18 Windows-regression gate (BLOCKING — final per-plan close)</name>
  <files>(read-only verification)</files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § D-18
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-VALIDATION.md (per-task verification map for 22-03-T1..T4 + V1)
  </read_first>
  <action>
    1. `cargo test --workspace --all-features`
    2. Phase 15 5-row detached-console smoke gate
    3. `cargo test -p nono-cli --test wfp_port_integration -- --ignored` (admin + service)
    4. `cargo test -p nono-cli --test learn_windows_integration`
    5. `cargo test -p nono-cli --test package_integration` (new from Task 8)
    6. VALIDATION.md gate: 22-03-T1..T4 + V1 green.

    If new regression: STOP per CONTEXT STOP trigger #4. Revert and re-scope.
  </action>
  <verify>
    <automated>cargo test --workspace --all-features &amp;&amp; cargo test -p nono-cli --test learn_windows_integration &amp;&amp; cargo test -p nono-cli --test package_integration &amp;&amp; cargo fmt --all -- --check &amp;&amp; cargo clippy --workspace -- -D warnings -D clippy::unwrap_used</automated>
  </verify>
  <acceptance_criteria>
    - `cargo test --workspace --all-features` exits 0 within deferred-flake window.
    - Phase 15 5-row smoke gate passes (or documented-skip).
    - `wfp_port_integration --ignored` passes or documented-skipped.
    - `learn_windows_integration` exits 0.
    - `package_integration` exits 0 (or documented-skip on non-Windows).
    - `cargo fmt --all -- --check` exits 0.
    - `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` exits 0.
    - VALIDATION.md 22-03-T1..T4 + V1 status updated to green.
  </acceptance_criteria>
  <done>
    D-18 Windows-regression safety net cleared for Plan 22-03.
  </done>
</task>

<task type="auto">
  <name>Task 10: D-07 plan-close push to origin</name>
  <files>(no files modified)</files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § D-07
  </read_first>
  <action>
    ```
    git fetch origin
    git log --oneline origin/main..main
    git push origin main
    git ls-remote origin main
    ```
  </action>
  <verify>
    <automated>git fetch origin &amp;&amp; test "$(git log origin/main..main --oneline | wc -l)" = "0"</automated>
  </verify>
  <acceptance_criteria>
    - `git log origin/main..main --oneline | wc -l` returns `0` after push.
    - SUMMARY records the post-push origin/main SHA.
  </acceptance_criteria>
  <done>
    Plan 22-03 commits published to origin.
  </done>
</task>

</tasks>

<non_goals>
**Windows-invariance (D-17):** No Windows-only file (`*_windows.rs`) is touched. `package_cmd.rs` and `registry_client.rs` are cross-platform with `cfg(target_os = "windows")` gates only inside path-resolution functions. Hook installer reuses fork's existing `hooks.rs` Windows path (already established).

**Plan 22-04 OAUTH scope:** OAuth2 client, reverse-proxy gating, `--allow-domain` strict-proxy preservation all live in Plan 22-04. Disjoint surface from PKG.

**Upstream `8b5a2ffb` (corrected SHA per RESEARCH finding #2 — was `8b2a5ffb` typo):** `fix(hooks): invoke bash via env` is N/A on Windows. ABORT if cherry-pick attempts to land it (D-17 violation candidate).

**Upstream `1d49246a`:** "claude-code integration package" removal already followed by fork. Plan 22-03 Task 1 verifies hygiene.

**Test fixture port (D-14):** Upstream ships inline tests in `package_cmd.rs`/`package.rs`; D-14 satisfied by porting them inline rather than as separate fixture files (RESEARCH finding #6).
</non_goals>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Registry HTTP endpoint → local registry_client | Untrusted network response crosses into the package install pipeline. Tampered response = Tampering threat (T-22-03-01). |
| Package archive → filesystem extraction | Path entries inside the archive cross into local filesystem writes. Path-traversal = Elevation of Privilege (T-22-03-02). |
| LOCALAPPDATA env var → install_dir | OS-supplied env var crosses into path construction. Untrusted-env path = follows CLAUDE.md "validate env vars before use". |
| Hook installer → Claude Code config | Modifies user's Claude Code configuration; idempotent + scoped to package. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation |
|-----------|----------|-----------|----------|-------------|------------|
| T-22-03-01 | Tampering | Registry returns tampered artifact | **high** | mitigate (BLOCKING) | PKG-04: signed-artifact verification BEFORE extraction. Trust bundle centralized via 600ba4ec. Test: `package_cmd::tests::signed_artifact_verify` + `signed_artifact_tampered_rejected`. |
| T-22-03-02 | Elevation of Privilege | Archive entry path traversal (`../../etc/passwd`) | **high** | mitigate (BLOCKING) | PKG-02: canonicalize + Path::components comparison (NOT string starts_with per CLAUDE.md § Common Footguns #1). Test: `package_integration::path_traversal_rejected_dotdot`. |
| T-22-03-03 | Elevation of Privilege | UNC aliasing redirects install_dir to system area (e.g., `\\?\GLOBALROOT\...`) | **high** | mitigate (BLOCKING) | PKG-02 acceptance #3: explicit UNC alias rejection. Test: `package_integration::path_traversal_rejected_unc_alias`. |
| T-22-03-04 | Denial of Service | Long path on Windows triggers MAX_PATH error mid-install, leaves filesystem inconsistent | medium | mitigate | PKG-02 acceptance #2: `\\?\` long-path prefix when joined path > 260. Test: `package_integration::long_path_prefix_applied`. |
| T-22-03-05 | Information Disclosure | Streaming download buffered to memory creates memory-bomb risk | medium | mitigate | PKG-04: streaming download via 9ebad89a. Acceptance criterion in Task 6 verifies streaming, not buffered. |
| T-22-03-06 | Tampering | Hook re-install creates duplicate entries | low | mitigate | PKG-03: idempotent install per acceptance criterion. Test: `package_cmd::tests::hook_install_idempotent`. |
| T-22-03-07 | Repudiation | Cherry-pick provenance lost | medium | mitigate | D-19 trailers enforced. |
| T-22-03-08 | Tampering | Trust bundle compromise via stale CA roots in fork | medium | accept | Trust bundle is centralized via 600ba4ec; refresh follows fork's existing trust-update cadence. |
| T-22-03-09 | Elevation of Privilege | Hook installer runs in fork's existing `hooks.rs` Windows path; if that path has elevated privileges, package install inherits them | low | accept | Existing fork posture; not introduced by 22-03. |

**BLOCKING threats:** T-22-03-01, T-22-03-02, T-22-03-03 (high severity) — Plan 22-03 cannot close until these are mitigated and verified.
</threat_model>

<verification>
- `cargo build --workspace` exits 0.
- `cargo test --workspace --all-features` exits 0 within deferred-flake tolerance.
- `cargo fmt --all -- --check` exits 0.
- `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` exits 0.
- Phase 15 5-row smoke gate passes.
- `package_integration` Windows tests pass.
- VALIDATION.md 22-03-T1..T4 + V1 marked green.
- `nono package --help` lists pull/remove/search/list subcommands.
- All cherry-pick commits carry D-19 trailers; fork-only commits (T-22-03 tests + path-component fix if added) carry Signed-off-by only.
- `git log origin/main..main` shows zero commits ahead post-Task 10.
- No `<capture from` placeholders.
</verification>

<success_criteria>
- 8–9 atomic commits on `main` (8 upstream cherry-picks + 1 D-15 Windows-test commit + optional fork-only path-component fix if upstream used string starts_with).
- `nono package pull/remove/search/list` subcommand tree functional cross-platform.
- Windows install_dir resolves to `%LOCALAPPDATA%\nono\packages\<name>`; long-path `\\?\` prefix applied when needed.
- Path-traversal (`..`, symlinks, UNC aliasing) rejected fail-closed.
- Streaming download verifies signed artifact before extraction.
- Hook install idempotent.
- `make ci` green or matches Phase 19 deferred window.
- `origin/main` advanced to plan-close HEAD.
- Plan SUMMARY records all 10 tasks' outcomes, ~9 commit hashes, D-17 hygiene findings from Task 1, and any fork-only follow-ups.
</success_criteria>

<output>
Create `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-03-SUMMARY.md` per standard summary template. Required sections: Outcome, What was done (one bullet per task), Verification table, Files changed table, Commits (~9-row table with hashes + upstream provenance), Status, Deferred (any 8b5a2ffb / 1d49246a hygiene notes from Task 1).
</output>