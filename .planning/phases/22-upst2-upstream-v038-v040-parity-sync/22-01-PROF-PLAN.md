---
phase: 22-upst2-upstream-v038-v040-parity-sync
plan: 01
type: execute
wave: 0
depends_on: []
blocks: ["22-03", "22-04"]
files_modified:
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/profile/builtin.rs
  - crates/nono-cli/data/policy.json
  - crates/nono-cli/Cargo.toml
  - crates/nono-proxy/Cargo.toml
  - crates/nono/Cargo.toml
autonomous: true
requirements: ["PROF-01", "PROF-02", "PROF-03", "PROF-04"]

must_haves:
  truths:
    - "Profile struct deserializes `unsafe_macos_seatbelt_rules: Vec<String>` via serde with `#[serde(default)]`; runtime application is macOS-only (Windows is no-op deserialize per REQ-PROF-01)"
    - "Profile struct deserializes `packs: Vec<PackRef>` and `command_args: Vec<String>` via serde with `#[serde(default)]`"
    - "Profile struct deserializes `custom_credentials.oauth2: Option<OAuth2Config>`; `OAuth2Config::token_url` rejects http:// fail-closed via `NonoError::PolicyError { kind: InsecureTokenUrl, .. }`"
    - "`OAuth2Config::client_secret` resolves through `nono::keystore::load_secret` for `keyring://` URIs"
    - "`claude-no-keychain` builtin profile loads via `Profile::load_builtin(\"claude-no-keychain\")` and inherits `claude-code` `override_deny` entries"
    - "`claude-no-keychain` resolves cleanly under existing fork POLY-01-stricter posture (PATTERNS CONTRADICTION-A) — no orphan override_deny errors at load time"
    - "Every cherry-pick commit body contains `Upstream-commit: <sha>`, `Upstream-tag: v0.40.1` (or earlier where applicable), `Upstream-author: <name> <email>`, and `Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>` trailers per D-19"
    - "`cargo test --workspace --all-features` exits 0 on Windows host after every cherry-pick commit (D-18 per-commit safety net)"
    - "`origin/main` advanced from `063ebad6` to current HEAD; tags `v2.0` and `v2.1` reachable from origin (D-06 + D-08)"
  artifacts:
    - path: "crates/nono-cli/src/profile/mod.rs"
      provides: "Profile struct with PROF-01..03 fields; OAuth2Config type definition; serde defaults preserving backward compat"
    - path: "crates/nono-cli/src/profile/builtin.rs"
      provides: "claude-no-keychain builtin registration (PROF-04)"
    - path: "crates/nono-cli/data/policy.json"
      provides: "claude-no-keychain entry inheriting claude-code override_deny; allow_file move from D-04 fixup"
  key_links:
    - from: "ROADMAP § Phase 22 success criterion #1"
      to: "Profile struct PROF-01..04 field additions"
      via: "serde deserialize coverage on Windows"
      pattern: "unsafe_macos_seatbelt_rules|packs|command_args|custom_credentials\\.oauth2"
    - from: "Plan 22-04 OAuth2 client implementation"
      to: "Plan 22-01 OAuth2Config type definition (commit fbf5c06e)"
      via: "type-level dependency"
      pattern: "use.*OAuth2Config"
---

<objective>
Land upstream v0.38–v0.40 Profile struct field additions (PROF-01..04) into the fork via chronological cherry-pick of 9 upstream commits, plus the D-06 origin push gate at Plan 22-01 head and the D-07 plan-close push at the tail. This plan unblocks Plan 22-03 (PKG depends on `packs` deserialize) and Plan 22-04 (OAUTH depends on `OAuth2Config` type). Each commit carries the D-19 trailer set so downstream audit can trace provenance back to upstream `always-further/nono`.

Purpose: Windows users running `nono run --profile <profile-with-new-fields> -- <cmd>` parse `unsafe_macos_seatbelt_rules`, `packs`, `command_args`, `custom_credentials.oauth2` without error, with runtime application of `unsafe_macos_seatbelt_rules` remaining macOS-only by design (REQ-PROF-01 fail-secure on unsupported platforms).
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md
@.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-RESEARCH.md
@.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-PATTERNS.md
@.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-VALIDATION.md
@crates/nono-cli/src/profile/mod.rs
@crates/nono-cli/src/profile/builtin.rs
@crates/nono-cli/data/policy.json

<interfaces>
**Upstream cherry-pick chain (chronological per D-03):**

| Order | SHA | Upstream subject | REQ |
|-------|-----|------------------|-----|
| 1 | `14c644ce` | feat: add `unsafe_macos_seatbelt_rules` profile field | PROF-01 |
| 2 | `c14e4365` | chore: cargo fmt (REQUIRED interleave per RESEARCH finding) | — |
| 3 | `e3decf9d` | test follow-up to seatbelt rules | PROF-01 |
| 4 | `ecd09313` | fmt follow-up | — |
| 5 | `088bdad7` | feat(profile): introduce packs and command_args | PROF-02 |
| 6 | `115b5cfa` | feat(profile): load profiles from registry packs | PROF-02 |
| 7 | `fbf5c06e` | feat(config): OAuth2Config type | PROF-03 prereq |
| 8 | `b1ecbc02` | feat(profile): support OAuth2 auth in custom_credentials | PROF-03 |
| 9 | `3c8b6756` | feat(claude): add no-keychain profile | PROF-04 |
| 10 | `713b2e0f` | fix(policy): update tests and claude-no-kc for allow_file move | PROF-04 |

**Coordination caveat:** Plan 22-02 (POLY) wave-parallels with Plan 22-01 and also touches `crates/nono-cli/src/profile/mod.rs` and `crates/nono-cli/data/policy.json` (CONTEXT § Known Risks). Use one atomic commit per upstream SHA so 22-02's executor can rebase between commits without long-running parallel branches.

**D-02 fallback heuristic per cherry-pick:** If `git cherry-pick <sha>` produces conflict markers exceeding 50 lines OR spanning >2 forked files OR the semantic meaning is ambiguous against fork's current `profile/mod.rs` (already heavily forked: +732/-414 vs upstream v0.37.1 per RESEARCH baseline), `git cherry-pick --abort` and apply D-20 manual-port template.

**D-19 commit body template (cherry-pick path):**
```
feat(22-01): <one-line subject from upstream>

<2-3 line why-this-matters>

Upstream-commit: <sha>
Upstream-tag: v0.40.1
Upstream-author: <git log -1 <sha> --format='%an <%ae>'>
Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
```

**D-20 commit body template (manual-port fallback):**
```
feat(22-01): port <feature> from upstream <sha> (manual replay)

Read-upstream-and-replay over heavily-forked <file>. Cherry-pick aborted at
<count> conflict markers across <file-list>; replayed semantically.

Upstream-commit: <sha> (replayed manually)
Upstream-tag: v0.40.1
Upstream-author: <name> <email>
Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
```

**Pattern map analogs (PATTERNS.md):**
- Profile struct field additions follow existing `Profile` + `ProfileDeserialize` + `impl From<>` companion pattern with `#[serde(default)]` on every field — new fields slot in identically
- `claude-no-keychain` is mostly a `policy.json` insert plus a 20-line test in `builtin.rs` (built-in profiles are JSON-data-driven, not Rust-coded)
- `OAuth2Config::token_url` http:// rejection follows existing fork pattern of fail-closed `NonoError::PolicyError { kind, .. }` enum returns
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 0: Origin push gate (D-06 + D-08) — STOP-and-escalate if remote diverged</name>
  <files>(no files modified — git operations only)</files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § D-06, D-08, § Specifics "Initial origin push command sequence"
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-RESEARCH.md (origin push pending finding — was 447 commits, now 513)
    - .planning/quick/260424-mrg-merge-windows-squash-to-main/SUMMARY.md (path-C deferrals — DCO remediation NOT a Phase 22 gate)
  </read_first>
  <action>
    1. Verify the remote has not raced ahead:
       ```
       git fetch origin
       git log origin/main..main --oneline | wc -l   # expect ~513 commits ahead
       git log main..origin/main --oneline | wc -l   # MUST be 0; non-zero = STOP
       ```
       If `main..origin/main` returns non-zero, the remote has commits the local doesn't — STOP per CONTEXT STOP trigger #2 spirit. Do NOT push.

    2. Push main:
       ```
       git push origin main
       ```
       This advances `origin/main` to current local HEAD (commit `50ce13e1` or later if Wave 0 docs commits land before Task 0 runs).

    3. Push the v2.0 + v2.1 milestone tags per D-08:
       ```
       git push origin v2.0 v2.1
       ```
       After push, `git ls-remote --tags origin v2.0 v2.1` must show both tag SHAs reachable from origin.

    4. Confirm clean state:
       ```
       git status              # working tree clean
       git log -1 --format=%H  # capture HEAD SHA for SUMMARY traceability
       ```
  </action>
  <verify>
    <automated>git fetch origin &amp;&amp; test "$(git log main..origin/main --oneline | wc -l)" = "0" &amp;&amp; test "$(git log origin/main..main --oneline | wc -l)" = "0"</automated>
  </verify>
  <acceptance_criteria>
    - `git log main..origin/main --oneline | wc -l` returns `0` after push (origin not ahead of local).
    - `git log origin/main..main --oneline | wc -l` returns `0` after push (local not ahead of origin — push was complete).
    - `git ls-remote --tags origin v2.0 | grep -c refs/tags/v2.0` returns `1`.
    - `git ls-remote --tags origin v2.1 | grep -c refs/tags/v2.1` returns `1`.
    - SUMMARY records the post-push origin/main SHA for traceability.
  </acceptance_criteria>
  <done>
    `origin/main` is the canonical baseline for Phase 22 cherry-picks. v2.0 + v2.1 tags reachable from origin. PROJECT.md / RETROSPECTIVE.md tag references verifiable from a fresh clone.
  </done>
</task>

<task type="auto">
  <name>Task 1: Cherry-pick `14c644ce` — feat: `unsafe_macos_seatbelt_rules` profile field (PROF-01)</name>
  <files>crates/nono-cli/src/profile/mod.rs</files>
  <read_first>
    - crates/nono-cli/src/profile/mod.rs (current `Profile` + `ProfileDeserialize` companion structs — observe `#[serde(default)]` pattern on every field per PATTERNS.md analog)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-RESEARCH.md § Plan 22-01 cheat-sheet (commit ordering verification)
    - `git show 14c644ce -- crates/nono-cli/src/profile/mod.rs` (read upstream's diff before cherry-pick to anticipate conflicts)
  </read_first>
  <action>
    1. Run the cherry-pick:
       ```
       git cherry-pick 14c644ce
       ```

    2. **D-02 fallback gate.** If conflict:
       ```
       git diff --name-only --diff-filter=U
       grep -c '<<<<<<<' crates/nono-cli/src/profile/mod.rs   # count conflict markers
       ```
       - If conflict markers ≤ 50 lines AND only `profile/mod.rs` is conflicted AND semantic meaning is clear: resolve in place by adding the new field with `#[serde(default)]` to BOTH `Profile` and `ProfileDeserialize` structs + the `impl From<ProfileDeserialize> for Profile` mapping. Then `git cherry-pick --continue`.
       - Else: `git cherry-pick --abort` and apply the D-20 manual-port template. Replay the diff: add `pub unsafe_macos_seatbelt_rules: Vec<String>` to `Profile` (after the `extends` field per upstream's structural placement) AND `#[serde(default)] pub unsafe_macos_seatbelt_rules: Vec<String>` to `ProfileDeserialize`, plus the `From` mapping. macOS-only runtime application is upstream's existing concern; Windows is deserialize-only by design (REQ-PROF-01).

    3. Verify the deserialize works on Windows:
       ```
       cargo build --workspace
       cargo test -p nono-cli profile::tests::
       ```

    4. Amend commit body to D-19 template if cherry-pick path used; if D-20 path, build the body from scratch:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-01): add unsafe_macos_seatbelt_rules profile field

       Deserialize-only field on Windows (REQ-PROF-01); runtime application
       remains macOS-only by design. Slots into existing Profile +
       ProfileDeserialize companion-struct pattern with #[serde(default)].

       Upstream-commit: 14c644ce
       Upstream-tag: v0.39.0
       Upstream-author: <capture from `git log -1 14c644ce --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
       Preflight: replace `<capture from ...>` with the literal value before commit. If the commit message contains `<capture from`, abort and re-substitute.
  </action>
  <verify>
    <automated>grep -E 'unsafe_macos_seatbelt_rules' crates/nono-cli/src/profile/mod.rs &amp;&amp; cargo build --workspace &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 14c644ce' &amp;&amp; git log -1 --format='%b' | grep -c '^Signed-off-by: '</automated>
  </verify>
  <acceptance_criteria>
    - `grep -E 'pub unsafe_macos_seatbelt_rules: Vec<String>' crates/nono-cli/src/profile/mod.rs` returns 2 hits (Profile + ProfileDeserialize).
    - `cargo build --workspace` exits 0.
    - `git log -1 --format=%B | grep '^Upstream-commit: 14c644ce'` returns 1 line.
    - `git log -1 --format=%B | grep -c '^Signed-off-by: '` returns `1`.
    - `git log -1 --format=%B | grep -c '<capture from'` returns `0` (no unsubstituted placeholders).
    - VALIDATION 22-01-T1 (`cargo test -p nono-cli profile::tests::deserialize_seatbelt_rules`) — added in Task 8 Wave 0 stub if not already present from upstream.
  </acceptance_criteria>
  <done>
    `unsafe_macos_seatbelt_rules` deserializes on Windows (no runtime application); commit landed on main with D-19 trailers.
  </done>
</task>

<task type="auto">
  <name>Task 2: Cherry-pick `c14e4365` — chore: cargo fmt (REQUIRED interleave)</name>
  <files>(varies — fmt-only diff; whatever upstream `c14e4365` touched)</files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-RESEARCH.md (hidden-dependency finding: c14e4365 must land between 14c644ce and e3decf9d to keep cargo fmt --check green)
    - `git show c14e4365 --stat` (confirm fmt-only)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick c14e4365
       ```
       Fmt-only diffs rarely conflict against fork drift. If conflict appears, prefer manual `cargo fmt --all` and substitute the result for upstream's diff (D-20 fallback applies trivially: replay = `cargo fmt --all`).

    2. Verify:
       ```
       cargo fmt --all -- --check
       ```

    3. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       chore(22-01): cargo fmt after seatbelt-rules field add

       Required interleave between upstream 14c644ce and e3decf9d so the test
       follow-up cherry-pick lands cleanly without fmt drift.

       Upstream-commit: c14e4365
       Upstream-tag: v0.39.0
       Upstream-author: <capture from `git log -1 c14e4365 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
  </action>
  <verify>
    <automated>cargo fmt --all -- --check &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: c14e4365'</automated>
  </verify>
  <acceptance_criteria>
    - `cargo fmt --all -- --check` exits 0.
    - `git log -1 --format=%B | grep '^Upstream-commit: c14e4365'` returns 1 line.
    - `git log -1 --format=%B | grep -c '^Signed-off-by: '` returns `1`.
    - `git log -1 --format=%B | grep -c '<capture from'` returns `0`.
  </acceptance_criteria>
  <done>
    Fmt drift cleared; chain ready for e3decf9d.
  </done>
</task>

<task type="auto">
  <name>Task 3: Cherry-pick `e3decf9d` and `ecd09313` — test/fmt follow-ups for seatbelt rules</name>
  <files>crates/nono-cli/src/profile/mod.rs (likely test module)</files>
  <read_first>
    - `git show e3decf9d --stat` and `git show ecd09313 --stat` to confirm test+fmt scope
  </read_first>
  <action>
    Cherry-pick each commit individually per D-03:
    ```
    git cherry-pick e3decf9d
    # resolve any conflicts per D-02 gate
    git commit --amend -s -m "$(cat <<'EOF'
    test(22-01): add seatbelt-rules profile deserialize coverage

    Test follow-up confirming PROF-01 deserialize works on all platforms;
    runtime application gated by macOS at the sandbox driver layer.

    Upstream-commit: e3decf9d
    Upstream-tag: v0.39.0
    Upstream-author: <capture from `git log -1 e3decf9d --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"

    git cherry-pick ecd09313
    git commit --amend -s -m "$(cat <<'EOF'
    chore(22-01): cargo fmt follow-up to seatbelt test

    Upstream-commit: ecd09313
    Upstream-tag: v0.39.0
    Upstream-author: <capture from `git log -1 ecd09313 --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"
    ```

    Per-commit Windows-regression spot check after each:
    ```
    cargo test --workspace --lib
    ```
    If red, STOP per CONTEXT STOP trigger #2.
  </action>
  <verify>
    <automated>git log -3 --format='%b' | grep -E '^Upstream-commit: (e3decf9d|ecd09313)' | wc -l &amp;&amp; cargo test --workspace --lib</automated>
  </verify>
  <acceptance_criteria>
    - `git log -3 --format=%B | grep -E '^Upstream-commit: (e3decf9d|ecd09313)' | wc -l` returns `2`.
    - `cargo test --workspace --lib` exits 0.
    - Each commit body has 1 Signed-off-by line and no `<capture from` placeholders.
  </acceptance_criteria>
  <done>
    PROF-01 fully landed (3 commits: 14c644ce, e3decf9d, ecd09313) plus required fmt interleave c14e4365.
  </done>
</task>

<task type="auto">
  <name>Task 4: Cherry-pick `088bdad7` and `115b5cfa` — packs + command_args (PROF-02)</name>
  <files>crates/nono-cli/src/profile/mod.rs</files>
  <read_first>
    - crates/nono-cli/src/profile/mod.rs (current Profile struct shape post-Task 1)
    - `git show 088bdad7 -- crates/nono-cli/src/profile/mod.rs` (PackRef type definition)
    - `git show 115b5cfa -- crates/nono-cli/src/profile/mod.rs` (registry pack loading hooks)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § Integration Points "Plan 22-01 PROF-02 (`packs`) wiring"
  </read_first>
  <action>
    1. Cherry-pick `088bdad7` (introduces `PackRef` type + `packs: Vec<PackRef>` + `command_args: Vec<String>`):
       ```
       git cherry-pick 088bdad7
       ```
       D-02 fallback if conflict markers > 50 OR > 2 files OR ambiguous. Manual replay: add fields with `#[serde(default)]` to Profile + ProfileDeserialize per established pattern.

    2. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-01): introduce packs and command_args for profiles

       Adds PackRef type, packs: Vec<PackRef>, and command_args: Vec<String>
       to Profile. Pack resolution short-circuits on Windows when registry
       client is unavailable per REQ-PROF-02 fail-secure semantics.

       Upstream-commit: 088bdad7
       Upstream-tag: v0.38.0
       Upstream-author: <capture from `git log -1 088bdad7 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    3. Cherry-pick `115b5cfa` (registry pack loading wiring):
       ```
       git cherry-pick 115b5cfa
       ```
       D-02 gate; manual replay if needed.

    4. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-01): load profiles from registry packs

       Wires registry pack resolution into Profile::resolve_aipc_allowlist
       end-to-end (existing Phase 18.1 pipeline). Windows short-circuits
       when registry client is unavailable.

       Upstream-commit: 115b5cfa
       Upstream-tag: v0.38.0
       Upstream-author: <capture from `git log -1 115b5cfa --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    5. Spot check:
       ```
       cargo test --workspace --lib
       cargo test -p nono-cli profile::tests::
       ```
  </action>
  <verify>
    <automated>grep -E '(packs: Vec|command_args: Vec)' crates/nono-cli/src/profile/mod.rs &amp;&amp; cargo build --workspace &amp;&amp; git log -2 --format='%b' | grep -E '^Upstream-commit: (088bdad7|115b5cfa)' | wc -l</automated>
  </verify>
  <acceptance_criteria>
    - `grep -E 'pub packs: Vec<PackRef>' crates/nono-cli/src/profile/mod.rs` returns ≥ 1 hit (Profile or ProfileDeserialize).
    - `grep -E 'pub command_args: Vec<String>' crates/nono-cli/src/profile/mod.rs` returns ≥ 1 hit.
    - `git log -2 --format=%B | grep -E '^Upstream-commit: (088bdad7|115b5cfa)' | wc -l` returns `2`.
    - `cargo build --workspace` exits 0; `cargo test --workspace --lib` exits 0.
  </acceptance_criteria>
  <done>
    PROF-02 fully landed (2 commits: 088bdad7, 115b5cfa). Profile struct now carries packs + command_args; registry pack loading wired.
  </done>
</task>

<task type="auto">
  <name>Task 5: Cherry-pick `fbf5c06e` — OAuth2Config type definition (PROF-03 prereq)</name>
  <files>crates/nono-cli/src/profile/mod.rs (or new module if upstream split it out)</files>
  <read_first>
    - `git show fbf5c06e --stat` and `git show fbf5c06e -- crates/nono-cli/src/profile/mod.rs` (where the OAuth2Config type is defined upstream)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-RESEARCH.md (Plan 22-04 dependency: OAuth2Config from fbf5c06e is required by Plan 22-04's 9546c879)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick fbf5c06e
       ```
       D-02 gate.

    2. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-01): introduce OAuth2Config type

       Type-level prerequisite for both PROF-03 (custom_credentials.oauth2)
       and Plan 22-04's nono-proxy/oauth2.rs client implementation. Lands
       in Plan 22-01 so 22-04 sees it from wave 1 onward.

       Upstream-commit: fbf5c06e
       Upstream-tag: v0.39.0
       Upstream-author: <capture from `git log -1 fbf5c06e --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    3. Spot check:
       ```
       cargo build --workspace
       grep -nE 'pub struct OAuth2Config' crates/nono-cli/src/
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: fbf5c06e' &amp;&amp; grep -rE 'pub struct OAuth2Config' crates/nono-cli/src/ | head -1</automated>
  </verify>
  <acceptance_criteria>
    - `grep -rE 'pub struct OAuth2Config' crates/nono-cli/src/` returns ≥ 1 hit.
    - `cargo build --workspace` exits 0.
    - `git log -1 --format=%B | grep '^Upstream-commit: fbf5c06e'` returns 1 line.
  </acceptance_criteria>
  <done>
    OAuth2Config type defined; Plan 22-04 unblocked at type level.
  </done>
</task>

<task type="auto">
  <name>Task 6: Cherry-pick `b1ecbc02` — custom_credentials.oauth2 (PROF-03)</name>
  <files>crates/nono-cli/src/profile/mod.rs</files>
  <read_first>
    - crates/nono-cli/src/profile/mod.rs (current `custom_credentials` field shape)
    - `git show b1ecbc02 -- crates/nono-cli/src/profile/mod.rs`
    - REQUIREMENTS.md § PROF-03 (acceptance #1 — `keyring://` resolution; acceptance — `token_url: http://` rejection fail-closed)
    - CLAUDE.md § Security Considerations "Permission Scope" — fail-secure on insecure schemes
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick b1ecbc02
       ```
       D-02 gate.

    2. **PROF-03 fail-secure verification.** Confirm upstream's `OAuth2Config::deserialize` (or equivalent validate hook) rejects `token_url: http://...` with a typed error. If upstream's behavior is permissive (allows http://), the fork MUST add the rejection — manual additional commit:
       ```
       # If b1ecbc02 doesn't already enforce https, add a fork-only commit:
       # Edit OAuth2Config validate to return Err(NonoError::PolicyError {
       #   kind: InsecureTokenUrl, ...
       # }) when token_url scheme != "https"
       ```
       The fork's existing `NonoError::PolicyError { kind, .. }` variant pattern (PATTERNS analog) hosts the new `InsecureTokenUrl` kind cleanly.

    3. Verify keystore integration: `OAuth2Config::client_secret` accepts `keyring://nono/oauth2-test` and resolves through `nono::keystore::load_secret` (already cross-platform from v2.1 Phase 20 UPST-03).

    4. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-01): support OAuth2 auth in custom_credentials

       Adds custom_credentials.oauth2: Option<OAuth2Config> to Profile.
       OAuth2Config::token_url validates https-only fail-closed
       (NonoError::PolicyError { kind: InsecureTokenUrl }) per CLAUDE.md
       fail-secure principle. client_secret resolves via existing
       nono::keystore::load_secret for keyring:// URIs.

       Upstream-commit: b1ecbc02
       Upstream-tag: v0.39.0
       Upstream-author: <capture from `git log -1 b1ecbc02 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    5. If a fork-only follow-up commit was needed for the http:// rejection, land it as a separate atomic commit immediately after, with subject `fix(22-01): reject http:// token_url in OAuth2Config (REQ-PROF-03 fail-secure)` and no `Upstream-commit:` trailer (it's a fork-only addition).
  </action>
  <verify>
    <automated>grep -E 'oauth2: Option<OAuth2Config>' crates/nono-cli/src/profile/mod.rs &amp;&amp; cargo test -p nono-cli profile::tests::oauth2_http_token_url_rejected 2&gt;/dev/null || cargo test -p nono-cli profile::tests:: 2&gt;&amp;1 | grep -E 'token_url|http_rejected|insecure' &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: b1ecbc02'</automated>
  </verify>
  <acceptance_criteria>
    - `grep -E 'oauth2: Option<OAuth2Config>' crates/nono-cli/src/profile/mod.rs` returns ≥ 1 hit.
    - A test exists verifying `token_url: http://...` is rejected (test name from VALIDATION 22-01-T3 — `oauth2_http_token_url_rejected` or equivalent).
    - `git log -1 --format=%B | grep '^Upstream-commit: b1ecbc02'` returns 1 line (or, if fork-only follow-up landed, `git log -2 --format=%B | grep -E '^Upstream-commit: b1ecbc02|fix.*http.*token_url'` returns 2 distinct trailer hits).
    - `cargo build --workspace` exits 0.
  </acceptance_criteria>
  <done>
    PROF-03 landed; OAuth2Config rejects insecure token_url scheme; client_secret resolves through nono::keystore.
  </done>
</task>

<task type="auto">
  <name>Task 7: Cherry-pick `3c8b6756` — claude-no-keychain builtin profile (PROF-04)</name>
  <files>
    crates/nono-cli/src/profile/builtin.rs
    crates/nono-cli/data/policy.json
  </files>
  <read_first>
    - crates/nono-cli/src/profile/builtin.rs (existing `claude-code` builtin definition — analog per PATTERNS)
    - crates/nono-cli/data/policy.json (existing `claude-code` entry — claude-no-keychain inherits override_deny)
    - `git show 3c8b6756 --stat`
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-PATTERNS.md (CONTRADICTION-A reconciliation: fork POLY-01 is already stricter — `claude-no-keychain` must resolve cleanly under fork posture)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 3c8b6756
       ```
       D-02 gate. Note: `policy.json` is touched by Plan 22-02 — coordinate per CONTEXT § Known Risks. If 22-02 has already amended `policy.json` between Task 7 cherry-pick and now, manually rebase (atomic small commits).

    2. **CONTRADICTION-A reconciliation:** Run `cargo test -p nono-cli profile::builtin::tests::all_builtins_resolve` (or equivalent). The fork already enforces stricter POLY-01 than upstream — `claude-no-keychain` must NOT regress to upstream's looser posture. If the test fails because the inherited `claude-code` `override_deny` entries surface as orphans, STOP per CONTEXT STOP trigger #7 and re-discuss with Plan 22-02 executor.

    3. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-01): add claude-no-keychain builtin profile

       New claude-no-keychain builtin inherits claude-code override_deny
       entries minus the keychain-specific grants. Resolves cleanly under
       fork's existing POLY-01-stricter posture (PATTERNS CONTRADICTION-A).

       Upstream-commit: 3c8b6756
       Upstream-tag: v0.38.0
       Upstream-author: <capture from `git log -1 3c8b6756 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
  </action>
  <verify>
    <automated>grep -E 'claude-no-keychain' crates/nono-cli/data/policy.json &amp;&amp; grep -E 'claude.no.keychain|claude_no_keychain' crates/nono-cli/src/profile/builtin.rs &amp;&amp; cargo test -p nono-cli profile::builtin:: &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 3c8b6756'</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c 'claude-no-keychain' crates/nono-cli/data/policy.json` returns ≥ 1.
    - `cargo test -p nono-cli profile::builtin::tests::claude_no_keychain_loads` passes (test name from VALIDATION 22-01-T4).
    - `cargo test -p nono-cli profile::builtin::tests::all_builtins_resolve` passes (fork POLY-01 posture preserved).
    - `git log -1 --format=%B | grep '^Upstream-commit: 3c8b6756'` returns 1 line.
  </acceptance_criteria>
  <done>
    PROF-04 landed; claude-no-keychain builtin loads cleanly under fork's existing POLY-01-stricter posture.
  </done>
</task>

<task type="auto">
  <name>Task 8: Cherry-pick `713b2e0f` — allow_file move test fixup</name>
  <files>(varies — likely tests + policy.json)</files>
  <read_first>
    - `git show 713b2e0f --stat`
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md (Plan 22-02 POLY-03 also moves `.claude.lock` to allow_file — coordinate)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 713b2e0f
       ```

    2. **Coordination check:** This commit fixes tests for the upstream `allow_file` move. Plan 22-02 POLY-03 lands the `.claude.lock` allow_file move itself. If 22-02 hasn't landed POLY-03 yet, the test fixups may be premature. Two acceptable paths:
       - (a) Land 713b2e0f now, fix any failing tests inline if 22-02 hasn't shipped POLY-03 (rebase later when 22-02's POLY-03 commit lands).
       - (b) Defer 713b2e0f until after 22-02 POLY-03 lands (preserves chronological order for the policy.json/test surface).
       Choose (b) when wave-parallel coordination requires; document the deferral in SUMMARY.

    3. Amend commit body (only if cherry-picked now):
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       fix(22-01): update tests and claude-no-keychain for allow_file move

       Test fixup follow-up to upstream's .claude.lock allow_file relocation
       (Plan 22-02 POLY-03 lands the move itself). Tests previously asserting
       allow_dir-style behavior updated to allow_file expectations.

       Upstream-commit: 713b2e0f
       Upstream-tag: v0.39.0
       Upstream-author: <capture from `git log -1 713b2e0f --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
  </action>
  <verify>
    <automated>cargo test --workspace --lib &amp;&amp; (git log -1 --format='%b' | grep -E '^Upstream-commit: 713b2e0f' || echo "deferred to post-22-02-POLY-03")</automated>
  </verify>
  <acceptance_criteria>
    - Either: `git log -1 --format=%B | grep '^Upstream-commit: 713b2e0f'` returns 1 line AND `cargo test --workspace --lib` exits 0.
    - Or: SUMMARY documents the deferral until Plan 22-02 POLY-03 lands.
  </acceptance_criteria>
  <done>
    Either landed cleanly, or deferred to post-22-02-POLY-03 with SUMMARY entry.
  </done>
</task>

<task type="auto">
  <name>Task 9: D-18 Windows-regression gate (BLOCKING — final per-plan close)</name>
  <files>(read-only verification)</files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § D-18
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-VALIDATION.md (per-task verification map for 22-01-T1..T4 + V1)
    - crates/nono-cli/tests/wfp_port_integration.rs
    - crates/nono-cli/tests/learn_windows_integration.rs
  </read_first>
  <action>
    1. Workspace test:
       ```
       cargo test --workspace --all-features
       ```
       Expected: exit 0 within Phase 19 deferred-flake tolerance (`tests/env_vars.rs` up to 19 failures, `trust_scan::tests::*` 1–3 failures). NEW failures = STOP per CONTEXT STOP trigger #2.

    2. Phase 15 5-row detached-console smoke gate (manual, on Windows host):
       ```
       nono run --profile default -- powershell -Command "Write-Host 'row1'; Write-Host 'row2'; Write-Host 'row3'; Write-Host 'row4'; Write-Host 'row5'; Start-Sleep 30"
       nono ps
       nono stop <session-id>
       ```
       Expected: 5 rows visible; ps lists session; stop returns 0.

    3. WFP port integration (admin + nono-wfp-service available):
       ```
       cargo test -p nono-cli --test wfp_port_integration -- --ignored
       ```
       Documented-skip if not available.

    4. ETW learn smoke:
       ```
       cargo test -p nono-cli --test learn_windows_integration
       ```

    5. VALIDATION.md gate: confirm 22-01-T1..T4 + V1 are green per the per-task verification map.

    6. If ANY new regression: STOP per CONTEXT STOP trigger #4. Revert offending commits (`git reset --hard <pre-task-1-SHA>`) and re-scope before any further Plan 22-01 work.
  </action>
  <verify>
    <automated>cargo test --workspace --all-features &amp;&amp; cargo test -p nono-cli --test learn_windows_integration &amp;&amp; cargo fmt --all -- --check &amp;&amp; cargo clippy --workspace -- -D warnings -D clippy::unwrap_used</automated>
  </verify>
  <acceptance_criteria>
    - `cargo test --workspace --all-features` exits 0 within deferred-flake window.
    - Phase 15 5-row smoke gate passes (or documented-skip with rationale).
    - `wfp_port_integration --ignored` passes or documented-skipped.
    - `learn_windows_integration` exits 0.
    - `cargo fmt --all -- --check` exits 0.
    - `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` exits 0.
    - VALIDATION.md 22-01-T1..T4 + V1 status updated to green.
  </acceptance_criteria>
  <done>
    D-18 Windows-regression safety net cleared for Plan 22-01. Wave 1 (Plans 22-03, 22-04) may now begin per D-09.
  </done>
</task>

<task type="auto">
  <name>Task 10: D-07 plan-close push to origin</name>
  <files>(no files modified — git push only)</files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § D-07 "Push to origin after each plan closes"
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § Specifics "Per-plan close push pattern"
  </read_first>
  <action>
    1. Capture pre-push state:
       ```
       git log --oneline origin/main..main
       ```
       Expected: ~9 commits (8 cherry-picks + amendments, possibly more if D-02 fallbacks landed extra fork-only commits).

    2. Push:
       ```
       git push origin main
       ```

    3. Capture post-push origin/main SHA for SUMMARY traceability:
       ```
       git ls-remote origin main
       ```
  </action>
  <verify>
    <automated>git fetch origin &amp;&amp; test "$(git log origin/main..main --oneline | wc -l)" = "0"</automated>
  </verify>
  <acceptance_criteria>
    - `git log origin/main..main --oneline | wc -l` returns `0` (origin/main caught up).
    - SUMMARY records the post-push origin/main SHA.
  </acceptance_criteria>
  <done>
    Plan 22-01 commits published to origin. D-07 satisfied.
  </done>
</task>

</tasks>

<non_goals>
**Windows-invariance (D-17):** No Windows-only file (`*_windows.rs` or `target_os="windows"` block) is touched in this plan. Any cherry-pick that surfaces such a touch is a BUG — abort per D-17.

**Plan 22-02 POLY scope:** Orphan `override_deny` rejection (POLY-01), `--rollback`/`--no-audit` clap conflict (POLY-02), `.claude.lock` allow_file move (POLY-03) all live in Plan 22-02. Plan 22-01's `713b2e0f` cherry-pick fixes only the test-side fallout, not the policy code itself.

**Plan 22-04 OAUTH scope:** OAuth2 client implementation (`nono-proxy/src/oauth2.rs`), reverse-proxy HTTP upstream gating (`reverse.rs`), `--allow-domain` strict-proxy preservation all live in Plan 22-04. Plan 22-01 only lands the type definition (`fbf5c06e`) and the profile-side deserialize wiring.

**Plan 22-03 PKG scope:** Package manager subcommand tree, registry client, hook installer all live in Plan 22-03. Plan 22-01 only lands the `packs: Vec<PackRef>` deserialize wiring (`088bdad7` + `115b5cfa`).

**No new requirement IDs:** Phase 22 requirement IDs are locked in REQUIREMENTS.md per Phase 21 close. Plan 22-01 covers PROF-01..04 only.

**Pre-existing Phase 19 deferred flakes:** `tests/env_vars.rs` and `trust_scan::tests::*` failures are documented-deferred. Plan 22-01 must NOT attempt fixes but also MUST NOT let them mask new regressions (Task 9 handles this distinction).
</non_goals>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Profile JSON file → Profile struct | Untrusted user-supplied profile JSON crosses into `serde_json::from_str` → `ProfileDeserialize::deserialize`. Validation happens at deserialize boundary. |
| OAuth2 token endpoint URL → outbound request | `OAuth2Config::token_url` is a network destination. Insecure scheme (http://) creates eavesdropping + downgrade attack surface for client_secret + access_token. |
| Keystore → process memory | `OAuth2Config::client_secret` resolved via `nono::keystore::load_secret` for `keyring://` URIs. Crosses OS-keystore → process-memory boundary. |
| Builtin profile registration → policy resolver | `claude-no-keychain` inherits `claude-code` override_deny. Inheritance must not bypass POLY-01 enforcement. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation |
|-----------|----------|-----------|----------|-------------|------------|
| T-22-01-01 | Tampering | Malformed `unsafe_macos_seatbelt_rules` deserialize | medium | mitigate | serde `Result` propagation; fail at parse time with clear error. Test: `profile::tests::malformed_seatbelt_rules_returns_err`. |
| T-22-01-02 | Information Disclosure | OAuth2 `token_url: http://...` accepted, leaks `client_secret` + `access_token` to MITM | **high** | mitigate (BLOCKING) | Fail-closed at `OAuth2Config::deserialize` with `NonoError::PolicyError { kind: InsecureTokenUrl }`. Test: `profile::tests::oauth2_http_token_url_rejected`. |
| T-22-01-03 | Information Disclosure | `client_secret` leaks via debug logs / panic messages | medium | mitigate | OAuth2Config field uses `Zeroize` on Drop (verify upstream b1ecbc02 carries this; fork-only follow-up if not). `client_secret` Display impl redacts. |
| T-22-01-04 | Elevation of Privilege | `claude-no-keychain` builtin inherits `claude-code` override_deny + bypasses POLY-01 (which is in 22-02, not yet landed) | medium | mitigate | Task 7 verifies builtin resolves cleanly under fork's existing POLY-01-stricter posture (CONTRADICTION-A); failure = STOP per STOP trigger #7. |
| T-22-01-05 | Denial of Service | Pack registry resolution hangs / fails on Windows when registry client unavailable | low | accept | REQ-PROF-02 requires short-circuit; Task 4 wires it. Failure mode = empty pack list (no AIPC allowlist additions), not crash. |
| T-22-01-06 | Repudiation | Cherry-pick provenance lost (no Upstream-commit: trailer) | medium | mitigate | D-19 commit body template enforced on every commit. Acceptance criteria grep for trailers on every cherry-pick task. |
| T-22-01-07 | Tampering | `keyring://` URI in client_secret allows path-traversal style attack on keystore | low | accept | nono::keystore::load_secret already validates URI shape (Phase 20 UPST-03). Reuses existing cross-platform validation. |

**BLOCKING threat:** T-22-01-02 (high severity) — Plan 22-01 cannot close until `oauth2_http_token_url_rejected` test is green.
</threat_model>

<verification>
Per-plan verification gate (D-16 + D-18):

- `cargo build --workspace` exits 0.
- `cargo test --workspace --all-features` exits 0 within Phase 19 deferred-flake tolerance.
- `cargo fmt --all -- --check` exits 0.
- `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` exits 0.
- Phase 15 5-row detached-console smoke gate passes on Windows host.
- `cargo test -p nono-cli --test wfp_port_integration -- --ignored` passes (admin + service) or documented-skip.
- `cargo test -p nono-cli --test learn_windows_integration` exits 0.
- VALIDATION.md 22-01-T1..T4 + V1 marked green.
- All cherry-pick commits carry `Upstream-commit:`, `Upstream-tag:`, `Upstream-author:`, and `Signed-off-by:` trailers per D-19.
- `git log --oneline origin/main..main` shows the expected commit sequence post-Task 0 push (zero commits ahead) and post-Task 10 push (zero commits ahead).
- No `<capture from` placeholder text in any commit body.
</verification>

<success_criteria>
- 9–10 atomic commits on `main` (8 upstream cherry-picks + optional fork-only T-22-01-02 follow-up + 713b2e0f or its deferral).
- Profile struct carries `unsafe_macos_seatbelt_rules`, `packs`, `command_args`, `custom_credentials.oauth2` fields with `#[serde(default)]`.
- OAuth2Config rejects `http://` token_url fail-closed.
- claude-no-keychain builtin loads cleanly under fork's POLY-01-stricter posture.
- `make ci` green or matches Phase 19 deferred window.
- `origin/main` advanced to plan-close HEAD; v2.0 + v2.1 tags reachable from origin.
- Plan SUMMARY records all 11 tasks' outcomes, ~10 commit hashes, and the post-push origin/main SHA.
- Plans 22-03 and 22-04 unblocked per D-09.
</success_criteria>

<output>
After completion, create `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-01-SUMMARY.md` using the standard summary template. Required sections: Outcome, What was done (one bullet per task), Verification table (copy from `<verification>` with actual results), Files changed table, Commits (10-row table with hashes + upstream provenance), Status, Deferred (713b2e0f if deferred, T-22-01-02 fork-only follow-up if added).
</output>
