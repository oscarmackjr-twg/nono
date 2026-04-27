---
phase: 22-upst2-upstream-v038-v040-parity-sync
plan: 04
type: execute
wave: 1
depends_on: ["22-01"]
blocks: []
files_modified:
  - crates/nono-proxy/src/oauth2.rs
  - crates/nono-proxy/src/reverse.rs
  - crates/nono-proxy/src/server.rs
  - crates/nono-proxy/src/lib.rs
  - crates/nono-cli/src/network_policy.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-proxy/Cargo.toml
autonomous: true
requirements: ["OAUTH-01", "OAUTH-02", "OAUTH-03"]

must_haves:
  truths:
    - "`OAuth2Config` client-credentials flow exchanges + caches token in memory; expired token refreshes; cache uses Zeroize on Drop (OAUTH-01)"
    - "`Bearer <token>` header injected into outbound proxy requests when profile carries OAuth2 config; secret value never logged or written to disk"
    - "Reverse-proxy HTTP upstream gating: loopback (127.0.0.0/8, ::1) allowed; non-loopback rejected fail-closed via `validate_upstream_url` (OAUTH-02)"
    - "`--allow-domain <host>` works in strict-proxy-only mode without bypassing host-network restrictions (OAUTH-03)"
    - "11 inline OAuth2 tests from upstream `9546c879` ported and pass on Windows host"
    - "OAuth2 token cache is memory-only — no `write_to_disk` / `serialize_to_path` calls in `crates/nono-proxy/src/oauth2.rs` (T-22-04-02 BLOCKING mitigation; avoids Low-IL label issues from WSFG-01..03)"
    - "Every cherry-pick commit body contains D-19 trailers (Upstream-commit/tag/author + Signed-off-by)"
    - "`cargo test --workspace --all-features` exits 0 on Windows after each commit (D-18)"
  artifacts:
    - path: "crates/nono-proxy/src/oauth2.rs"
      provides: "OAuth2 client_credentials token exchange + cache (NEW, ~557 LOC including 11 inline tests)"
    - path: "crates/nono-proxy/src/reverse.rs"
      provides: "Reverse-proxy HTTP upstream loopback-only gating"
    - path: "crates/nono-cli/src/network_policy.rs"
      provides: "--allow-domain strict-proxy-only enforcement preserved"
  key_links:
    - from: "ROADMAP § Phase 22 success criterion #4"
      to: "OAuth2 Bearer header injection on outbound requests"
      via: "nono-proxy/src/oauth2.rs token-cache lookup at request hook"
      pattern: "Authorization.*Bearer"
    - from: "nono-proxy/src/oauth2.rs"
      to: "nono::keystore::load_secret"
      via: "keyring:// URI resolution for client_secret (cross-platform from v2.1 Phase 20 UPST-03)"
      pattern: "keyring|load_secret"
---

<objective>
Land upstream OAuth2 client-credentials proxy + reverse-proxy HTTP upstream loopback-only gating + `--allow-domain` strict-proxy preservation (OAUTH-01..03) into the fork. Wave 1 plan, parallel with Plan 22-03 per D-12. Depends on Plan 22-01 (OAuth2Config type from `fbf5c06e` already landed). Cross-platform by construction; no Windows-specific code paths needed beyond fork's existing keystore plumbing for client_secret resolution.

Purpose: A Windows user running `nono run --profile <with-oauth2> -- curl https://api.example.com` receives a `Bearer` token on the outbound request with the same token-cache + refresh semantics as macOS, while reverse-proxy HTTP upstream gating prevents loopback-only restrictions from being bypassed (REQ-OAUTH-01..03).
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
@crates/nono-proxy/src/lib.rs
@crates/nono-proxy/src/server.rs
@crates/nono-cli/src/network_policy.rs

<interfaces>
**Upstream cherry-pick chain (chronological per D-03):**

| Order | SHA | Upstream subject | REQ |
|-------|-----|------------------|-----|
| 1 | `9546c879` | feat(proxy): implement OAuth2 client_credentials token exchange with cache (557 LOC NEW oauth2.rs + 11 inline tests) | OAUTH-01 |
| 2 | `0c7fb902` | OAuth2 rebase | OAUTH-01 |
| 3 | `19a0731f` | OAuth2 rebase | OAUTH-01 |
| 4 | `2244dd73` | 413 early-return fix (per RESEARCH: prereq for 9546c879 — should land BEFORE per planner discretion) | OAUTH-01 |
| 5 | `2bf5668f` | feat(reverse-proxy): add http upstream support | OAUTH-02 |
| 6 | `0340ebff` | HTTP upstream loopback-only gating | OAUTH-02 |
| 7 | `b2a24402` | HTTP upstream loopback-only gating cont. | OAUTH-02 |
| 8 | `0c990116` | HTTP upstream loopback-only gating cont. | OAUTH-02 |
| 9 | `10bcd054` | fix(network): keep `--allow-domain` in strict proxy-only mode | OAUTH-03 |
| 10 | `005579a9` | port/dry-run/test fix around `--allow-domain` | OAUTH-03 |
| 11 | `d44e404e` | port/dry-run/test fix around `--allow-domain` | OAUTH-03 |
| 12 | `60ad1eb3` | port/dry-run/test fix around `--allow-domain` | OAUTH-03 |

**RESEARCH finding #5 — chain ordering correction:** `2244dd73` (413 early-return fix) is a PREREQUISITE for `9546c879`. CONTEXT cites them in order `9546c879, 0c7fb902, 19a0731f, 2244dd73` but the correct chronological build order requires `2244dd73` BEFORE `9546c879`. Per D-03 strict chronological order, verify the actual `git log` order:
```
git log --reverse --oneline 9546c879~5..2244dd73
```
Land in true chronological order; document any deviation from CONTEXT's listing in the commit body.

**Pattern-map analogs (PATTERNS.md):**
- OAuth2 client_credentials flow uses fork's existing hyper client (already in nono-proxy) + `nono::keystore::load_secret` for `keyring://` URIs (cross-platform, shipped v2.1 Phase 20 UPST-03).
- `Zeroize` on Drop pattern: existing in fork (verify in `crates/nono/src/keystore.rs` and elsewhere).
- Reverse-proxy loopback gating: fork already has `validate_upstream_url` helper that classifies IP into loopback/private/public — reuse verbatim per PATTERNS finding (already cross-platform).
- `--allow-domain` strict-proxy mode: fork's `network_policy.rs` already enforces this; cherry-picks should preserve, not regress.

**No Windows-specific test additions (D-15) needed beyond keystore-backed `client_secret`:** OAuth2 is cross-platform by construction. Windows path is `keyring://` resolution, which Plan 22-01 PROF-03 already covers. Add ONE Windows-gated keystore round-trip test if not present from upstream.

**RESEARCH finding #6 — no external test fixture files:** OAuth2 ships 11 inline tests in `oauth2.rs`. D-13 fixture port = port these inline tests verbatim along with the production code.

**D-19 commit body template:** Same as prior plans.

**D-20 commit body template:** Same as prior plans.

**Coordination caveat:** Plan 22-03 PKG wave-parallels with 22-04. Disjoint surfaces (22-03: nono-cli/src/package_cmd.rs; 22-04: nono-proxy/src/oauth2.rs + reverse.rs). No expected shared edits; safe to run in parallel.

**Plan 22-01 dependency satisfied:** `OAuth2Config` type from `fbf5c06e` is in fork after Plan 22-01 Task 5. Plan 22-04 imports it; if 22-01 hasn't shipped Task 5 yet, this plan blocks per D-09.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Verify Plan 22-01 OAuth2Config dependency landed</name>
  <files>(read-only verification — no files modified)</files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-01-PROF-PLAN.md § Task 5 (cherry-pick `fbf5c06e`)
    - crates/nono-cli/src/profile/mod.rs (current state)
  </read_first>
  <action>
    1. Verify `OAuth2Config` type is defined in fork:
       ```
       grep -rE 'pub struct OAuth2Config' crates/nono-cli/src/ crates/nono-proxy/src/ crates/nono/src/
       ```
       Must return ≥ 1 hit. If 0 hits, Plan 22-01 Task 5 hasn't landed — STOP per D-09 dependency.

    2. Verify the latest Plan 22-01 commit landed:
       ```
       git log --grep='Upstream-commit: fbf5c06e' --oneline
       ```
       Must return ≥ 1 hit.

    3. Verify Plan 22-01 D-18 gate passed (look for plan-close push):
       ```
       git log --oneline | grep -E '22-01.*close|22-01.*push'
       ```
       Or verify the Plan 22-01 SUMMARY exists.

    4. Document baseline in preflight note (for SUMMARY).
  </action>
  <verify>
    <automated>grep -rE 'pub struct OAuth2Config' crates/ &amp;&amp; git log --grep='Upstream-commit: fbf5c06e' --oneline | head -1</automated>
  </verify>
  <acceptance_criteria>
    - `grep -rE 'pub struct OAuth2Config' crates/` returns ≥ 1 hit.
    - `git log --grep='Upstream-commit: fbf5c06e'` returns ≥ 1 commit.
    - Preflight note records Plan 22-01 baseline status.
  </acceptance_criteria>
  <done>
    Plan 22-01 dependency verified; Plan 22-04 may proceed.
  </done>
</task>

<task type="auto">
  <name>Task 2: Cherry-pick `2244dd73` — 413 early-return fix (OAUTH-01 prereq)</name>
  <files>crates/nono-proxy/src/server.rs</files>
  <read_first>
    - `git show 2244dd73 --stat` and full diff
    - `git log --reverse --oneline 9546c879~5..2244dd73` (verify chronological order — RESEARCH finding #5)
    - crates/nono-proxy/src/server.rs (current request-handling shape)
  </read_first>
  <action>
    1. Verify chronological order: `2244dd73` is older than `9546c879` per RESEARCH finding #5.
       ```
       git log -1 --format=%ci 2244dd73
       git log -1 --format=%ci 9546c879
       ```

    2. Cherry-pick:
       ```
       git cherry-pick 2244dd73
       ```
       D-02 gate.

    3. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       fix(22-04): early-return 413 for oversized requests (OAUTH-01 prereq)

       Prerequisite for OAuth2 token-exchange POST: nono-proxy must early-return
       413 Payload Too Large when request body exceeds the configured limit,
       so the OAuth2 client doesn't observe truncated/partial responses on
       large authorization-code flows. Lands BEFORE 9546c879 per chronological
       order (research finding #5; CONTEXT listed in different order).

       Upstream-commit: 2244dd73
       Upstream-tag: v0.39.0
       Upstream-author: <capture from `git log -1 2244dd73 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 2244dd73'</automated>
  </verify>
  <acceptance_criteria>
    - `cargo build --workspace` exits 0.
    - `git log -1 --format=%B | grep '^Upstream-commit: 2244dd73'` returns 1 line.
  </acceptance_criteria>
  <done>
    413 early-return prerequisite landed; OAuth2 cherry-pick chain ready.
  </done>
</task>

<task type="auto">
  <name>Task 3: Cherry-pick `9546c879` — OAuth2 client_credentials + token cache + 11 inline tests (OAUTH-01)</name>
  <files>
    crates/nono-proxy/src/oauth2.rs (NEW)
    crates/nono-proxy/src/lib.rs
    crates/nono-proxy/Cargo.toml
  </files>
  <read_first>
    - `git show 9546c879 --stat` (anticipate +557 LOC NEW file)
    - `git show 9546c879 -- crates/nono-proxy/src/oauth2.rs` (full upstream file content)
    - REQUIREMENTS.md § OAUTH-01 (acceptance: client_credentials flow, token cache, refresh on expiry, Zeroize on Drop)
    - CLAUDE.md § Memory ("Use the zeroize crate for sensitive data (keys/passwords) in memory")
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-PATTERNS.md (OAuth2 client analogs — keystore::load_secret, Zeroize on Drop)
  </read_first>
  <action>
    1. Cherry-pick:
       ```
       git cherry-pick 9546c879
       ```
       D-02 gate. New file `oauth2.rs` is unlikely to conflict (no fork drift on a non-existent file). The lib.rs `pub mod oauth2;` line MAY conflict with fork's existing `pub mod` ordering.

    2. **Verify Zeroize usage.** Token cache MUST zeroize secrets on Drop per CLAUDE.md § Memory:
       ```
       grep -E 'Zeroize|ZeroizeOnDrop' crates/nono-proxy/src/oauth2.rs
       ```
       If 0 hits: upstream may not use Zeroize. Add a fork-only follow-up commit wrapping `client_secret` and `access_token` fields in `zeroize::Zeroizing<String>` or deriving `ZeroizeOnDrop`. Test with `cargo test -p nono-proxy oauth2::tests::zeroize_on_drop`.

    3. **Verify keystore integration.** `client_secret: keyring://...` resolves through `nono::keystore::load_secret`:
       ```
       grep -E 'keyring|load_secret' crates/nono-proxy/src/oauth2.rs
       ```
       Cross-platform pattern (already in fork from v2.1 Phase 20 UPST-03).

    4. **Verify the 11 inline tests are ported (D-13).** The 557 LOC includes inline `#[cfg(test)] mod tests`:
       ```
       grep -cE '^[[:space:]]*#\[(test|tokio::test)\]' crates/nono-proxy/src/oauth2.rs
       ```
       Should return ~11.

    5. Amend commit body:
       ```
       git commit --amend -s -m "$(cat <<'EOF'
       feat(22-04): implement OAuth2 client_credentials token exchange with cache (OAUTH-01)

       New crates/nono-proxy/src/oauth2.rs (~557 LOC) implementing
       client_credentials grant flow with in-memory token cache, expiry-based
       refresh, and Zeroize on Drop for client_secret + access_token. Ports
       11 inline tests verbatim (D-13). client_secret resolves via existing
       nono::keystore::load_secret for keyring:// URIs (cross-platform).

       Upstream-commit: 9546c879
       Upstream-tag: v0.39.0
       Upstream-author: <capture from `git log -1 9546c879 --format='%an <%ae>'`>
       Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
       EOF
       )"
       ```

    6. Verify:
       ```
       cargo build --workspace
       cargo test -p nono-proxy oauth2::
       ```
  </action>
  <verify>
    <automated>cargo build --workspace &amp;&amp; cargo test -p nono-proxy oauth2:: &amp;&amp; grep -E 'Zeroize' crates/nono-proxy/src/oauth2.rs &amp;&amp; git log -1 --format='%b' | grep -E '^Upstream-commit: 9546c879'</automated>
  </verify>
  <acceptance_criteria>
    - `crates/nono-proxy/src/oauth2.rs` exists.
    - `grep -E 'Zeroize|ZeroizeOnDrop' crates/nono-proxy/src/oauth2.rs` returns ≥ 1 hit (or fork-only follow-up commit lands one).
    - `cargo test -p nono-proxy oauth2::` exits 0 (all 11 inline tests pass).
    - `cargo build --workspace` exits 0.
    - `git log -1 --format=%B | grep '^Upstream-commit: 9546c879'` returns 1 line.
  </acceptance_criteria>
  <done>
    OAuth2 client + 11 inline tests landed.
  </done>
</task>

<task type="auto">
  <name>Task 4: Cherry-pick `0c7fb902` and `19a0731f` — OAuth2 rebase fixups</name>
  <files>crates/nono-proxy/src/oauth2.rs</files>
  <read_first>
    - `git show 0c7fb902 --stat` and `git show 19a0731f --stat`
  </read_first>
  <action>
    Cherry-pick each in chronological order per D-03:
    ```
    git cherry-pick 0c7fb902
    # D-02 gate
    git commit --amend -s -m "$(cat <<'EOF'
    fix(22-04): <subject from upstream 0c7fb902>

    OAuth2 rebase fixup; preserves token-cache + refresh semantics.

    Upstream-commit: 0c7fb902
    Upstream-tag: <git describe --tags 0c7fb902>
    Upstream-author: <capture from `git log -1 0c7fb902 --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"

    git cherry-pick 19a0731f
    git commit --amend -s -m "$(cat <<'EOF'
    fix(22-04): <subject from upstream 19a0731f>

    Upstream-commit: 19a0731f
    Upstream-tag: <git describe --tags 19a0731f>
    Upstream-author: <capture from `git log -1 19a0731f --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"
    ```

    Per-commit spot check: `cargo test -p nono-proxy oauth2::`. STOP if red.
  </action>
  <verify>
    <automated>git log -2 --format='%b' | grep -E '^Upstream-commit: (0c7fb902|19a0731f)' | wc -l &amp;&amp; cargo test -p nono-proxy oauth2::</automated>
  </verify>
  <acceptance_criteria>
    - `git log -2 --format=%B | grep -E '^Upstream-commit: (0c7fb902|19a0731f)' | wc -l` returns `2`.
    - `cargo test -p nono-proxy oauth2::` exits 0.
  </acceptance_criteria>
  <done>
    OAuth2 client fully landed and stable.
  </done>
</task>

<task type="auto">
  <name>Task 5: Cherry-pick `2bf5668f`, `0340ebff`, `b2a24402`, `0c990116` — reverse-proxy HTTP upstream loopback-only gating (OAUTH-02)</name>
  <files>
    crates/nono-proxy/src/reverse.rs
    crates/nono-proxy/src/lib.rs
  </files>
  <read_first>
    - `git show 2bf5668f --stat`, `0340ebff`, `b2a24402`, `0c990116` (each)
    - REQUIREMENTS.md § OAUTH-02 (loopback-only fail-closed; non-loopback rejected)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-PATTERNS.md (existing `validate_upstream_url` helper for IP-class enforcement)
    - crates/nono-proxy/src/reverse.rs (current state)
  </read_first>
  <action>
    Cherry-pick each in chronological order per D-03 (verify via `git log -1 --format=%ci <sha>`):
    ```
    git cherry-pick 2bf5668f
    # D-02 gate; preserve fork's existing validate_upstream_url helper if upstream conflicts
    git commit --amend -s -m "$(cat <<'EOF'
    feat(22-04): add HTTP upstream support to reverse-proxy

    Allows reverse-proxy to target HTTP (not just HTTPS) upstreams.
    Subsequent commits gate this to loopback-only per OAUTH-02 fail-secure.

    Upstream-commit: 2bf5668f
    Upstream-tag: <git describe --tags 2bf5668f>
    Upstream-author: <capture from `git log -1 2bf5668f --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"

    git cherry-pick 0340ebff
    git commit --amend -s -m "$(cat <<'EOF'
    feat(22-04): gate HTTP upstream to loopback-only (OAUTH-02)

    Non-loopback HTTP upstream rejected fail-closed via validate_upstream_url
    IP-class check. Loopback (127.0.0.0/8, ::1) allowed for local dev/test.

    Upstream-commit: 0340ebff
    Upstream-tag: <git describe --tags 0340ebff>
    Upstream-author: <capture from `git log -1 0340ebff --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"

    git cherry-pick b2a24402
    git commit --amend -s -m "$(cat <<'EOF'
    fix(22-04): <subject from upstream b2a24402>

    Upstream-commit: b2a24402
    Upstream-tag: <git describe --tags b2a24402>
    Upstream-author: <capture from `git log -1 b2a24402 --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"

    git cherry-pick 0c990116
    git commit --amend -s -m "$(cat <<'EOF'
    fix(22-04): <subject from upstream 0c990116>

    Upstream-commit: 0c990116
    Upstream-tag: <git describe --tags 0c990116>
    Upstream-author: <capture from `git log -1 0c990116 --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"
    ```

    Per-commit spot check `cargo test -p nono-proxy reverse::`. STOP if red.

    **Verify OAUTH-02 acceptance test exists or add it:**
    ```rust
    // crates/nono-proxy/src/reverse.rs tests module:
    #[test]
    fn http_upstream_loopback_only() {
        assert!(validate_upstream_url("http://127.0.0.1:8080").is_ok());
        assert!(validate_upstream_url("http://[::1]:8080").is_ok());
        assert!(validate_upstream_url("http://example.com:8080").is_err());
        assert!(validate_upstream_url("http://10.0.0.1:8080").is_err());
    }
    ```
  </action>
  <verify>
    <automated>git log -4 --format='%b' | grep -E '^Upstream-commit: (2bf5668f|0340ebff|b2a24402|0c990116)' | wc -l &amp;&amp; cargo test -p nono-proxy reverse::</automated>
  </verify>
  <acceptance_criteria>
    - `git log -4 --format=%B | grep -E '^Upstream-commit: (2bf5668f|0340ebff|b2a24402|0c990116)' | wc -l` returns `4`.
    - `cargo test -p nono-proxy reverse::tests::http_upstream_loopback_only` (or equivalent) exits 0.
    - `cargo build --workspace` exits 0.
  </acceptance_criteria>
  <done>
    OAUTH-02 reverse-proxy HTTP upstream loopback-only gating landed.
  </done>
</task>

<task type="auto">
  <name>Task 6: Cherry-pick `10bcd054`, `005579a9`, `d44e404e`, `60ad1eb3` — `--allow-domain` strict-proxy preservation (OAUTH-03)</name>
  <files>
    crates/nono-cli/src/network_policy.rs
    crates/nono-cli/src/cli.rs
  </files>
  <read_first>
    - `git show 10bcd054 --stat`, `005579a9`, `d44e404e`, `60ad1eb3` (each)
    - REQUIREMENTS.md § OAUTH-03 (--allow-domain in strict-proxy-only mode)
    - crates/nono-cli/src/network_policy.rs (current state — fork has v2.0 Phase 9 WFP port-level additions)
  </read_first>
  <action>
    Cherry-pick each in chronological order per D-03:
    ```
    git cherry-pick 10bcd054
    # D-02 gate (network_policy.rs is in HIGH-conflict file list per CONTEXT § Files at HIGH merge-conflict risk)
    # Preserve fork's v2.0 Phase 9 WFP port-level enforcement during conflict resolution
    git commit --amend -s -m "$(cat <<'EOF'
    fix(22-04): keep --allow-domain in strict proxy-only mode (OAUTH-03)

    --allow-domain <host> entries continue to flow through the strict-proxy
    pipeline; no host-network bypass introduced. Preserves fork's v2.0 Phase 9
    WFP port-level enforcement on Windows.

    Upstream-commit: 10bcd054
    Upstream-tag: <git describe --tags 10bcd054>
    Upstream-author: <capture from `git log -1 10bcd054 --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"

    git cherry-pick 005579a9
    git commit --amend -s -m "$(cat <<'EOF'
    test(22-04): <subject from upstream 005579a9>

    Upstream-commit: 005579a9
    Upstream-tag: <git describe --tags 005579a9>
    Upstream-author: <capture from `git log -1 005579a9 --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"

    git cherry-pick d44e404e
    git commit --amend -s -m "$(cat <<'EOF'
    fix(22-04): <subject from upstream d44e404e>

    Upstream-commit: d44e404e
    Upstream-tag: <git describe --tags d44e404e>
    Upstream-author: <capture from `git log -1 d44e404e --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"

    git cherry-pick 60ad1eb3
    git commit --amend -s -m "$(cat <<'EOF'
    fix(22-04): <subject from upstream 60ad1eb3>

    Upstream-commit: 60ad1eb3
    Upstream-tag: <git describe --tags 60ad1eb3>
    Upstream-author: <capture from `git log -1 60ad1eb3 --format='%an <%ae>'`>
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    EOF
    )"
    ```

    Per-commit spot check `cargo test --workspace --lib` + `cargo test -p nono-cli network_policy::`. STOP if red.
  </action>
  <verify>
    <automated>git log -4 --format='%b' | grep -E '^Upstream-commit: (10bcd054|005579a9|d44e404e|60ad1eb3)' | wc -l &amp;&amp; cargo test -p nono-cli network_policy::</automated>
  </verify>
  <acceptance_criteria>
    - `git log -4 --format=%B | grep -E '^Upstream-commit: (10bcd054|005579a9|d44e404e|60ad1eb3)' | wc -l` returns `4`.
    - `cargo test -p nono-cli network_policy::` exits 0 (existing fork v2.0 tests + new --allow-domain tests).
    - `cargo build --workspace` exits 0.
  </acceptance_criteria>
  <done>
    OAUTH-03 `--allow-domain` strict-proxy preservation landed. Fork's v2.0 Phase 9 WFP enforcement preserved.
  </done>
</task>

<task type="auto">
  <name>Task 7: D-18 Windows-regression gate (BLOCKING — final per-plan close)</name>
  <files>(read-only verification)</files>
  <read_first>
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-CONTEXT.md § D-18
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-VALIDATION.md (per-task verification map for 22-04-T1..T3 + V1)
  </read_first>
  <action>
    1. `cargo test --workspace --all-features`
    2. Phase 15 5-row detached-console smoke gate
    3. `cargo test -p nono-cli --test wfp_port_integration -- --ignored` (admin + service available)
    4. `cargo test -p nono-cli --test learn_windows_integration`
    5. `cargo test -p nono-proxy oauth2::` (all 11 ported inline tests pass)
    6. `cargo test -p nono-proxy reverse::` (loopback-only gating tests pass)
    7. `cargo test -p nono-cli network_policy::` (--allow-domain strict-proxy tests pass; fork v2.0 Phase 9 WFP regression tests still green)
    8. VALIDATION.md gate: 22-04-T1..T3 + V1 green.

    If new regression: STOP per CONTEXT STOP trigger #4.
  </action>
  <verify>
    <automated>cargo test --workspace --all-features &amp;&amp; cargo test -p nono-cli --test learn_windows_integration &amp;&amp; cargo test -p nono-proxy oauth2:: &amp;&amp; cargo test -p nono-proxy reverse:: &amp;&amp; cargo fmt --all -- --check &amp;&amp; cargo clippy --workspace -- -D warnings -D clippy::unwrap_used</automated>
  </verify>
  <acceptance_criteria>
    - `cargo test --workspace --all-features` exits 0 within deferred-flake window.
    - Phase 15 5-row smoke gate passes (or documented-skip).
    - `wfp_port_integration --ignored` passes or documented-skipped.
    - `learn_windows_integration` exits 0.
    - `oauth2::` and `reverse::` tests pass.
    - `network_policy::` (--allow-domain + WFP regression) tests pass.
    - `cargo fmt --all -- --check` exits 0.
    - `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` exits 0.
    - VALIDATION.md 22-04-T1..T3 + V1 status updated to green.
  </acceptance_criteria>
  <done>
    D-18 Windows-regression safety net cleared for Plan 22-04.
  </done>
</task>

<task type="auto">
  <name>Task 8: D-07 plan-close push to origin</name>
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
    Plan 22-04 commits published to origin.
  </done>
</task>

</tasks>

<non_goals>
**Windows-invariance (D-17):** No Windows-only file (`*_windows.rs`) is touched. nono-proxy is cross-platform; network_policy.rs is cross-platform with platform-specific WFP wiring living in `wfp_proxy.rs` / Phase 9 sub-modules (untouched).

**Plan 22-03 PKG scope:** Disjoint surface (package_cmd.rs, registry_client.rs); no shared edits.

**Plan 22-05 AUD scope:** Audit ledger, Authenticode, prune→cleanup rename all live in 22-05. OAUTH plan does not touch audit subsystem beyond ensuring OAuth2 token exchange events flow through whatever ledger emission path 22-05 establishes.

**No new requirement IDs:** OAUTH-01..03 covered.

**OAuth2 disk persistence:** Token cache is memory-only with Zeroize on Drop (CONTEXT § Integration Points). NO disk persistence — avoids Low-IL label issues from WSFG-01..03.

**Public OAuth2 sandbox:** No public OAuth2 endpoint dependency in tests. All 11 inline tests use mock or local servers.
</non_goals>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Profile JSON `oauth2.client_secret: keyring://...` → process memory | OS keystore → memory; uses fork's vetted `nono::keystore::load_secret`. |
| OAuth2 token endpoint → process memory (token cache) | Network response crosses into in-memory cache. Cache must zeroize on Drop. |
| Process memory → outbound HTTP request | Bearer token attached to outbound proxy request. Logging or error paths must NOT leak token. |
| Reverse-proxy upstream URL → outbound network | User-configured upstream URL crosses into outbound request. Non-loopback HTTP = downgrade attack vector. |
| `--allow-domain <host>` arg → network policy resolver | User CLI input becomes allow-listed network egress. Must NOT bypass strict-proxy mode. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation |
|-----------|----------|-----------|----------|-------------|------------|
| T-22-04-01 | Information Disclosure | OAuth2 `client_secret` leaks via debug logs / panic / Display impl | **high** | mitigate (BLOCKING) | Wrap secrets in `zeroize::Zeroizing<String>` or derive `ZeroizeOnDrop`; Display impl redacts. Test: `oauth2::tests::secret_redacted_on_display`. Acceptance criterion in Task 3 verifies Zeroize usage. |
| T-22-04-02 | Information Disclosure | Token cache persists to disk, leaks via filesystem snapshot | **high** | mitigate (BLOCKING) | Cache memory-only; no disk write. Code path: in-memory `HashMap<TokenKey, CachedToken>`. Acceptance criterion verifies no `write_to_disk` / `serialize_to_path` calls in oauth2.rs. |
| T-22-04-03 | Spoofing | Reverse-proxy HTTP upstream redirected to attacker-controlled non-loopback host | **high** | mitigate (BLOCKING) | OAUTH-02: validate_upstream_url IP-class check; non-loopback HTTP rejected fail-closed. Test: `reverse::tests::http_upstream_loopback_only`. |
| T-22-04-04 | Elevation of Privilege | `--allow-domain` bypasses host-network restrictions, exposes sandboxed agent to non-allow-listed network | **high** | mitigate (BLOCKING) | OAUTH-03: --allow-domain stays in strict-proxy-only mode; cherry-pick 10bcd054 enforces. Fork's v2.0 Phase 9 WFP port-level enforcement preserved. |
| T-22-04-05 | Tampering | OAuth2 token endpoint response tampered (TLS MITM) | medium | mitigate | Plan 22-01 PROF-03 already enforces https-only token_url. nono-proxy uses rustls (Phase 20 UPST-01 hardened to 0.103.12). |
| T-22-04-06 | Denial of Service | Token endpoint hangs; OAuth2 client blocks indefinitely | low | accept | Upstream's client uses tokio timeout per `9546c879`; fork preserves. |
| T-22-04-07 | Repudiation | Cherry-pick provenance lost | medium | mitigate | D-19 trailers enforced on all 12 commits. |

**BLOCKING threats:** T-22-04-01, T-22-04-02, T-22-04-03, T-22-04-04 (high severity) — Plan 22-04 cannot close until all four are mitigated and verified.
</threat_model>

<verification>
- `cargo build --workspace` exits 0.
- `cargo test --workspace --all-features` exits 0 within deferred-flake tolerance.
- `cargo fmt --all -- --check` exits 0.
- `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` exits 0.
- Phase 15 5-row smoke gate passes.
- 11 inline OAuth2 tests + reverse-proxy loopback tests + --allow-domain tests all green.
- VALIDATION.md 22-04-T1..T3 + V1 marked green.
- All cherry-pick commits carry D-19 trailers.
- `git log origin/main..main` shows zero commits ahead post-Task 8.
- No `<capture from` placeholders in any commit body.
- `grep -E 'Zeroize|ZeroizeOnDrop' crates/nono-proxy/src/oauth2.rs` returns ≥ 1 hit.
- `grep -E 'write_to_disk|serialize_to_path' crates/nono-proxy/src/oauth2.rs` returns 0 hits.
</verification>

<success_criteria>
- 12 atomic upstream cherry-pick commits + optional fork-only Zeroize follow-up.
- OAuth2 client_credentials flow exchanges + caches token in memory; refresh on expiry; Zeroize on Drop.
- Bearer header injected on outbound requests when profile carries OAuth2 config.
- Reverse-proxy HTTP upstream loopback-only fail-closed; non-loopback rejected.
- `--allow-domain` works in strict-proxy-only mode; v2.0 Phase 9 WFP port-level enforcement preserved.
- `make ci` green or matches Phase 19 deferred window.
- `origin/main` advanced to plan-close HEAD.
- Plan SUMMARY records all 8 tasks' outcomes, ~13 commit hashes (12 upstream + maybe Zeroize follow-up), and OAuth2 chain ordering deviation from CONTEXT (per RESEARCH finding #5).
</success_criteria>

<output>
Create `.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-04-SUMMARY.md` per standard summary template. Required sections: Outcome, What was done (one bullet per task), Verification table, Files changed table, Commits (~13-row table with hashes + upstream provenance), Status, Deferred (Zeroize follow-up if added; chain-ordering deviation note).
</output>
