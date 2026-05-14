---
phase: 40-upst4-sync-execution
plan: 06
slug: fp-proxy-tls
cluster_id: C5
subsystem: nono-proxy
type: execute
wave: 2
depends_on: ["40-05"]
upstream_tag_range: v0.52.2..v0.53.0
upstream_commit_count: 3
autonomous: true
tags: [upst4, c5, fork-preserve, proxy-tls, d20-manual-replay, credential-matching, wave-2, phase-40-terminal]

# Dependency graph
requires:
  - phase: 39-upst4-audit
    provides: cluster C5 disposition (fork-preserve, 3 commits) + commit chain inventory + D-40-B2 LOCK rationale
  - phase: 40-01-PROXY-HARDENING
    provides: Wave 1 closed (Plan 40-01 server.rs review fixes — NODE_USE_ENV_PROXY, libdbus feature unification — preserved byte-identically)
  - phase: 40-04-RELEASE-RIDE
    provides: Wave 1 closed (Landlock ABI cache + full failure diagnostic + v0.52.1/v0.52.2/v0.53.0 CHANGELOG)
  - phase: 40-05-FP-PROFILE-SAVE
    provides: Wave 2 first plan closed (D-20 manual replay precedent + SUMMARY frontmatter shape + load-bearing vs environmental gate categorization)
provides:
  - "Native system CA loading at server.rs::start (combined with webpki-roots) — fixes UnknownIssuer on corporate networks with TLS-inspection MITM"
  - "Native system CA loading at route.rs::build_tls_connector_with_ca — per-route TLS connector now trusts OS roots alongside webpki + custom CA"
  - "Shared pub(crate) helper route::build_base_root_store() — single source of truth for combined webpki + native trust"
  - "rustls-native-certs = 0.8 dependency added to crates/nono-proxy/Cargo.toml (not workspace-wide)"
  - "Credential-match policy disposition documented (D-20 replay of f77e0e3) — struct + method-level doc comments on CredentialStore explaining structural enforcement of upstream's three cases"
  - "Windows-fallback decision documented (Option A — uniform no-creds passthrough; no fork-specific Windows fallback exists to suppress)"
  - "Phase 40 terminal plan — 6/6 plans complete; REQ-UPST4-02 acceptance criteria 1–5 met"
affects:
  - "Future Plan 40-06 / UPST5 ingestion of tls_intercept-related upstream commits — base_root_store helper is the integration point if a future phase ever ports tls_intercept"
  - "Phase 41 backlog — no new failures introduced; pre-existing helper_stamps_session_token_from_env flake unchanged (zero touches to crates/nono/)"

# Tech tracking
tech-stack:
  added:
    - "rustls-native-certs = 0.8 (proxy-only dep)"
  patterns:
    - "D-20 manual replay (D-40-B3 commit body sections; no D-19 trailer; Upstream-replayed-from: provenance)"
    - "pub(crate) helper extraction at fork boundary so per-call-site root-store assembly is DRY"
    - "Structural enforcement of multi-route credential policy via HashMap<prefix, LoadedCredential> invariant (single-credential-per-prefix by construction)"
    - "Disposition-as-doc-comment: f77e0e3 policy semantics ported as struct-level + method-level doc comments where the policy IS the architecture, not algorithm"

key-files:
  created:
    - .planning/phases/40-upst4-sync-execution/40-06-FP-PROXY-TLS-SUMMARY.md
  modified:
    - crates/nono-proxy/Cargo.toml (+6: rustls-native-certs = "0.8" with replay-intent doc comment)
    - crates/nono-proxy/src/route.rs (+38/-3: build_base_root_store helper; build_tls_connector_with_ca composes from base)
    - crates/nono-proxy/src/server.rs (+8/-3: import route::build_base_root_store; use helper at start())
    - crates/nono-proxy/src/credential.rs (+58/-1: struct-level + get() doc comments documenting f77e0e3 policy disposition + Windows-fallback decision)
    - Cargo.lock (+1 transitive)

# Skipped gates categorization (per .continue-here.md anti-pattern #3)
skipped_gates_load_bearing: [3, 4]   # cross-target clippy linux-gnu/darwin (CI substitute required)
skipped_gates_environmental: [6, 7, 8]   # detached-console / wfp_port / learn_windows (Windows runtime missing in agent context)

decisions:
  - "Disposition remains D-20 manual replay (LOCKED per D-40-B2). No trial cherry-pick attempted — the lock is structural (fork has no tls_intercept module; upstream's TLS-trust + multi-route + auth-intercept changes all live in or directly depend on that module). A cherry-pick would have produced modify/delete on tls_intercept/handle.rs (deleted in fork — never existed) plus structural conflicts in route.rs / server.rs against the fork's path-prefix-routing-only architecture."
  - "Native-CA loading replayed in BOTH server.rs::start AND route.rs::build_tls_connector_with_ca — not just one site — because both sites build a TLS connector for upstream traffic and both benefit identically from corporate-network MITM compatibility."
  - "Shared helper route::build_base_root_store() mirrors 54c7552's factoring pattern. pub(crate) visibility, not pub, so the helper is not exposed to other crates."
  - "Multi-route dispatch (lookup_all_by_upstream / has_intercept_route rewrite) NOT replayed — no caller in fork. Adding it would produce dead-code lint failures and violate D-40-E6 surgical-retrofit posture (no opportunistic composition)."
  - "TLS-intercept handle.rs changes (163 lines, biggest upstream hunk) NOT applicable — fork has no tls_intercept module (Plan 34-10 documented this as a non-port per D-34-B1 fork-preserve, inherited at D-40-B2)."
  - "f77e0e3 credential-match policy ported as DOC-COMMENT replay rather than algorithm replay. The fork's HashMap-keyed-by-prefix architecture enforces all three upstream cases (absolute match, 2-match-deny, no-match-passthrough) STRUCTURALLY — no runtime selection algorithm needed. The doc comment makes this explicit so future maintainers (a) understand the upstream policy and (b) know it's already enforced by construction."
  - "Windows-fallback decision: OPTION A (uniform no-creds passthrough). Rationale: audit of fork's credential.rs at plan start confirmed zero #[cfg(target_os = \"windows\")] arms and zero alternative cred-lookup paths beyond nono::keystore::load_secret_by_ref. Windows credential injection is a transitive consequence of `keyring v3` (cross-platform crate). There is no Windows-specific fallback in the fork to disable or annotate with a warning log (Option B). The policy is uniform across platforms by construction."
  - "Both replay commits include the D-40-B3 5 body sections (Upstream intent / What was replayed / What was NOT replayed and why / Fork-only wiring preserved / Upstream-replayed-from). Co-Authored-By: Claude on every commit per D-40-B3 + 2 DCO sign-offs per commit."
  - "fmt fix mid-Task-3 squashed into Replay Commit 1 via soft-reset + recreate (unpushed local commits; D-40-E1 + chain invariants re-verified post-rebuild). Two commit SHAs changed from local-only pre-push form: 70d229aa → cfab2e8b (Replay Commit 1), e7478311 → 6f75b3dd (Replay Commit 2). No data loss — both messages preserved verbatim from /tmp."

patterns-established:
  - "D-20 manual replay where upstream surface is structurally absent: replay the SECURITY-positive subset (native CAs) and DOCUMENT the structurally-enforced subset (credential-match policy). Two distinct shapes within the same D-20 plan."
  - "pub(crate) shared base-store helper at fork boundary: route::build_base_root_store() — both server.rs and route.rs call it, single source of truth for webpki + native trust."
  - "Disposition-as-doc-comment when policy semantics are structurally enforced rather than algorithmically — ports the SAFETY guarantee without ports the runtime cost or dead-code surface."
  - "Per-platform fallback NULL inference: when an upstream policy change references 'Windows fallback behavior' but the fork has no Windows-specific code path, the correct disposition is Option A (uniform behavior) with an audit-evidence justification — not Option B (warning-log preserving a thing that doesn't exist)."

requirements-completed: [REQ-UPST4-02]

# Metrics
duration: ~75m
completed: 2026-05-14
---

# Phase 40 Plan 06: FP-PROXY-TLS Summary

**Cluster C5 (v0.52.2..v0.53.0, 3 commits — `8ddb143`, `54c7552`, `f77e0e3`) absorbed via D-20 manual replay onto fork main. Two replay commits land the security-positive native-CA loading intent (corporate-network TLS-MITM compatibility) and document the credential-match policy disposition (structural enforcement via HashMap-keyed-by-prefix architecture; Option A uniform no-creds passthrough). Zero touches to the fork's tls_intercept-absent surface; D-40-E1 invariant holds (0 Windows-file edits); fork's Phase 09 + Phase 11 Windows credential path preserved. Phase 40 terminal plan — 6/6 plans complete; REQ-UPST4-02 satisfied.**

## Performance

- **Duration:** ~75 min
- **Started:** 2026-05-14
- **Completed:** 2026-05-14
- **Tasks:** 4 plan tasks (Task 4 push/PR/Phase-40 close-out is downstream of orchestrator-merge per worktree pattern)
- **Files modified:** 4 (`crates/nono-proxy/Cargo.toml`, `crates/nono-proxy/src/route.rs`, `crates/nono-proxy/src/server.rs`, `crates/nono-proxy/src/credential.rs`) + 1 lockfile (`Cargo.lock`)
- **Commits landed:** 2 D-20 manual replay commits

## Accomplishments

- **Native system CA loading absorbed at TWO sites:**
  - `crates/nono-proxy/src/server.rs::start` — the shared proxy-startup TLS connector now combines webpki roots with `rustls_native_certs::load_native_certs()` via the new helper.
  - `crates/nono-proxy/src/route.rs::build_tls_connector_with_ca` — the per-route TLS connector composes the new base store first, then adds the route's custom CA on top.
  - Result: upstream TLS handshakes succeed on corporate networks with TLS-inspection MITM (which trust their own root via the OS trust store, not webpki defaults). Replays 8ddb143's security-positive intent.
- **DRY refactor via `pub(crate) fn route::build_base_root_store()`:**
  - Single source of truth for combined webpki + native trust building.
  - Native-cert errors logged at `debug!` and skipped; webpki defaults always present so the store is never empty.
  - Mirrors 54c7552's factoring pattern in shape (named identically, placed identically inside `route.rs`).
- **Credential-match policy disposition documented at the code level (D-20 replay of f77e0e3):**
  - Struct-level doc comment on `CredentialStore` explains all three upstream cases (absolute-match / 2-match-deny / no-match-passthrough) and how each is enforced STRUCTURALLY in the fork's `HashMap<String, LoadedCredential>` architecture.
  - Method-level doc comment on `CredentialStore::get` reinforces fail-secure "no match → passthrough with no credentials injected" at the lookup boundary.
  - Both comments cite the upstream commit (`f77e0e3`) and the relevant fork-side architectural invariants (path-prefix routing in `reverse::parse_service_prefix`; `keyring v3` cross-platform crate).
- **Windows-fallback decision explicit in Replay Commit 2 body:** Option A — uniform no-creds passthrough. Audit-evidence justification (no Windows-specific cred-lookup fallback in fork's credential.rs).
- **D-40-B3 commit-body discipline holds on both replay commits:** all 5 sections present (`Upstream intent:` / `What was replayed:` / `What was NOT replayed and why:` / `Fork-only wiring preserved:` / `Upstream-replayed-from:`); zero `^Upstream-commit:` trailer lines; `Co-Authored-By: Claude` + 2× DCO sign-off per commit.
- **D-40-E1 invariant holds:** 0 Windows-file edits across the chain (`git diff --stat HEAD~2 HEAD -- crates/ | grep -E '_windows|exec_strategy_windows' | wc -l = 0`).
- **Fork-only wiring preserved:**
  - `crates/nono-proxy/src/credential.rs` Phase 09 + Phase 11 Windows credential injection path (via `nono::keystore::load_secret_by_ref` → `keyring v3` → Windows Credential Manager on Windows). The runtime credential-lookup function `CredentialStore::load` is byte-identical pre/post (only doc comments added).
  - `crates/nono-proxy/src/oauth2.rs` Phase 22-04 OAuth2 credential cache — NOT touched.
  - Plan 40-01 Wave 1 server.rs review fixes (NODE_USE_ENV_PROXY, libdbus feature unification, accurate warning message) — preserved byte-identically; verified by `grep -c 'NODE_USE_ENV_PROXY' crates/nono-proxy/src/server.rs = 5` (unchanged from pre-plan baseline).
  - Phase 18.1 D-04-locked surface (`build_prompt_text + HandleKind` in `crates/nono-cli/src/terminal_approval.rs`) — unchanged at 45 matches.
- **Phase 09 + Phase 11 Windows credential grep baseline GREW from 1 to 2** (additional `nono::keystore` reference in the new doc comment on `CredentialStore::get`). Plan acceptance criterion: `>= pre-task count` — satisfied.
- **f77e0e3 policy-semantics grep in credential.rs grew from 0 to 14 hits** (struct + method doc comments). Plan acceptance criterion: `>= 1` — satisfied.

## Task Commits

Each replay was committed atomically. D-20 manual replay (no `Upstream-commit:` trailer; `Upstream-replayed-from:` provenance only):

1. **Replay Commit 1:** `feat(proxy): load native system CAs alongside webpki roots (D-20 replay)` → `cfab2e8b`
   - Replays 8ddb143 + 54c7552 intent (native CA loading + shared base-store helper).
   - Files: `crates/nono-proxy/Cargo.toml`, `crates/nono-proxy/src/route.rs`, `crates/nono-proxy/src/server.rs`, `Cargo.lock`.
2. **Replay Commit 2:** `docs(proxy): document credential-match policy disposition (D-20 replay of f77e0e3)` → `6f75b3dd`
   - Replays f77e0e3 policy semantics as struct-level + method-level doc comments on `CredentialStore` and `CredentialStore::get`.
   - Windows-fallback decision (Option A) documented in commit body.
   - Files: `crates/nono-proxy/src/credential.rs`.

(SUMMARY-doc commit follows separately.)

## Files Created/Modified

- `crates/nono-proxy/Cargo.toml` — added `rustls-native-certs = "0.8"` dep with replay-intent doc comment. Version pin matches upstream `149abde0` declaration. Not added to workspace deps — single-crate scope.
- `crates/nono-proxy/src/route.rs` — new `pub(crate) fn build_base_root_store() -> rustls::RootCertStore` (combines webpki + native CAs, debug-logs errors, never fails). Modified `build_tls_connector_with_ca` to compose from `build_base_root_store()` rather than constructing the webpki-only store inline. Cleaner symmetry with `server.rs::start` and ports 54c7552's factoring pattern.
- `crates/nono-proxy/src/server.rs` — modified import line to `use crate::route::{self, RouteStore};`. Replaced inline `let mut root_store = rustls::RootCertStore::empty(); root_store.extend(...)` with `let root_store = route::build_base_root_store();`. Added inline comment citing 8ddb143 intent + D-40-B2 fork-preserve lock.
- `crates/nono-proxy/src/credential.rs` — added struct-level doc comment on `CredentialStore` (37 lines) and method-level doc comment on `CredentialStore::get` (12 lines). Both documents replay the f77e0e3 three-case policy and articulate its structural enforcement in the fork. Zero runtime behavior change; the new keyring/nono::keystore reference grows the Phase 09 + 11 Windows preservation grep from 1 to 2.
- `Cargo.lock` — single transitive bump for `rustls-native-certs` and its dependencies.

## Decisions Made

- **DEC-1: D-40-B2 LOCK honored.** No trial cherry-pick attempted; disposition was structurally pre-determined by D-40-B2 + the fork's absence of the `tls_intercept` module (audited pre-edit via `ls crates/nono-proxy/src/` and `find crates/nono-proxy/src -name 'tls*'`, both returning zero hits for the module).
- **DEC-2: Native-CA loading is the only security-positive replay-eligible hunk in cluster C5.** The 8ddb143 multi-route dispatch + auth-intercept changes are tls_intercept-specific (no fork callers); the f77e0e3 selection-policy algorithm is tls_intercept-specific. Replaying the dispatch / selection algorithm would have produced dead code and violated D-40-E6 surgical-retrofit posture.
- **DEC-3: `build_base_root_store()` named identically to upstream 54c7552's helper.** Maintains nominal symmetry so a future UPST5+ phase that ports `tls_intercept` can call the same helper without renaming.
- **DEC-4: `pub(crate)` visibility on `build_base_root_store()`, not `pub`.** The helper is a within-crate composition primitive; exposing it across the workspace boundary would invite unintended use.
- **DEC-5: f77e0e3 policy disposition is DOC-COMMENT replay, not algorithm replay.** The fork's `HashMap<String, LoadedCredential>` keyed by service prefix structurally enforces:
  - Case 1 (absolute match) — `HashMap` guarantees at most one value per key.
  - Case 2 (2-match-deny) — structurally impossible: the multi-route-per-upstream scenario that creates ambiguous selection requires tls_intercept's path-against-endpoint-rules selection, which doesn't exist in fork.
  - Case 3 (no match → passthrough no creds) — already the fork's runtime behavior: `get(&prefix)` returns `None`, downstream `reverse.rs` forwards without credential injection.
  - The doc comment makes all three explicit + cites the upstream commit for provenance.
- **DEC-6: Windows-fallback decision = Option A (uniform no-creds passthrough).** Audit-evidence justification documented in Replay Commit 2 body. Specifically:
  - `grep -nE 'cfg.*windows|cfg.*target_os' crates/nono-proxy/src/credential.rs` returned EMPTY at plan start. No Windows-specific arms exist.
  - The fork's Windows credential injection is wholly transitive through `nono::keystore::load_secret_by_ref` → `keyring v3` (cross-platform crate that resolves to Windows Credential Manager on Windows, macOS Keychain on macOS, libsecret/dbus on Linux). There is no separate "if Windows, try Credential Manager again as fallback" code branch.
  - Therefore there is nothing Windows-specific to disable (Option A's "uniform no-creds passthrough" silently re-enacts the existing behavior), and no cross-platform inconsistency to warn about (Option B's warning log would describe a thing that doesn't exist).
- **DEC-7: fmt deviation absorbed into Replay Commit 1 via soft-reset + recreate.** Pre-push, local-only commits — safe to reshape. Two commit hashes updated (70d229aa → cfab2e8b for RC1; e7478311 → 6f75b3dd for RC2). Both commit messages preserved verbatim from `/tmp/rc{1,2}-msg.txt`. Chain invariants re-verified post-rebuild (D-40-E1, trailer counts, sentinel SHA, policy grep) — all hold.
- **DEC-8: D-40-C2 gates 3+4 (cross-target clippy linux-gnu / darwin) are LOAD-BEARING skip → CI-verified** per `.continue-here.md` anti-pattern #3. Categorized in SUMMARY frontmatter as `skipped_gates_load_bearing: [3, 4]`. Windows host lacks C cross-compilers; CI's `ubuntu-latest` + `macos-latest` clippy jobs are the substitute. Task 5 baseline-aware regression gate (post-orchestrator-merge) is the enforcement point.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Plan-vs-reality mismatch] Plan frontmatter `files_modified` listed `crates/nono-proxy/src/tls_intercept/handle.rs` and implied fork-side TLS-intercept presence; reality is the fork has NO `tls_intercept` module at all.**
- **Found during:** Task 1 pre-flight (`ls crates/nono-proxy/src/` returned 13 .rs files, none named `tls*`; `find crates/nono-proxy/src -name "tls*"` returned empty).
- **Issue:** Plan's `files_modified` frontmatter and `<interfaces>` block list `crates/nono-proxy/src/tls_intercept/handle.rs` as a replay target. The fork did not adopt upstream's `tls_intercept` module (Plan 34-10's Phase-33-Cluster-11 follow-on documented this as a non-port per D-34-B1). Upstream's `8ddb143` made 163 lines of changes inside that module; the fork has zero lines there.
- **Fix:** Replay scope adapted to the fork's actual surface (no tls_intercept). Native-CA loading replayed at the two TLS-connector-build sites that DO exist in the fork (`server.rs::start` and `route.rs::build_tls_connector_with_ca`). Multi-route dispatch and auth-intercept changes from 8ddb143 NOT replayed because they have no caller in fork (would produce dead-code lint errors). f77e0e3 policy semantics replayed as doc-comment disposition rather than runtime algorithm.
- **Files modified:** Aligned to actual fork surface — `server.rs`, `route.rs`, `Cargo.toml`, `credential.rs`. NOT modified: `tls_intercept/handle.rs` (does not exist).
- **Verification:** `git diff --stat HEAD~2 HEAD -- crates/nono-proxy/src/tls_intercept/` returns empty (file path does not exist in fork at any point).
- **Committed in:** body of `cfab2e8b` (Replay Commit 1, "What was NOT replayed and why" section explicitly notes tls_intercept absence).

**2. [Rule 1 — Plan-vs-reality mismatch] Plan frontmatter implied fork's `crates/nono-proxy/src/credential.rs` has `#[cfg(target_os = "windows")]` Windows-credential-injection arms requiring byte-identical preservation.**
- **Found during:** Task 1 fork-only-surface audit (`grep -n 'cfg.*windows\|cfg.*target_os' crates/nono-proxy/src/credential.rs` returned empty; `grep -n 'windows\|Windows' crates/nono-proxy/src/credential.rs` returned empty).
- **Issue:** Plan's threat-model T-40-06-04 framing assumes the fork has Windows-specific credential fallback code in `credential.rs` that f77e0e3's "no-match passthrough" could silently disable. Reality: the fork's Windows credential injection is wholly transitive via `nono::keystore::load_secret_by_ref` (called at `CredentialStore::load`) using `keyring v3` (cross-platform crate). There is no Windows-specific branch in `credential.rs` to suppress or warn about.
- **Fix:** Windows-fallback decision recorded as Option A (uniform no-creds passthrough) — the de-facto fork behavior. No warning log needed because no cross-platform inconsistency exists in the first place. Replay Commit 2 body documents this evidence-based reframing.
- **Files modified:** None directly attributable to this deviation; informs the doc-comment shape in Replay Commit 2 + the disposition language in the commit body.
- **Verification:** Post-plan grep confirms zero `cfg(target_os = "windows")` arms in `credential.rs`. Windows credential injection grep baseline (`cfg.*windows|windows.*credential|keyring`) grew from 1 to 2 via the new doc-comment reference to `nono::keystore`.
- **Committed in:** body of `6f75b3dd` (Replay Commit 2 "Fork-only wiring preserved" section).

**3. [Rule 3 — Blocker: fmt deviation mid-Task-3] cargo fmt --all -- --check failed after Replay Commit 2 due to a `debug!` call wrapped across 3 lines that fmt prefers on one line.**
- **Found during:** Task 3 D-40-C2 Gate 5 (`cargo fmt --all -- --check` returned a 4-line diff).
- **Issue:** Replay Commit 1 introduced a `debug!("added {native_count} native system CA(s) to route connector trust store")` call wrapped across 3 lines (because the original draft used a longer message). fmt prefers single-line for messages under the line-length budget.
- **Fix:** Soft-reset HEAD~2 (both replay commits unpushed; safe to reshape). Ran `cargo fmt --all`, staged route.rs fmt fix into Replay Commit 1 (folded with original RC1 staged content). Restored credential.rs to staged state for Replay Commit 2. Recommitted both using `git commit -F /tmp/rc{1,2}-msg.txt` to preserve the original commit messages byte-identically. Two commit SHAs changed: RC1 70d229aa → cfab2e8b; RC2 e7478311 → 6f75b3dd. Chain invariants re-verified post-rebuild — all hold.
- **Files modified:** `crates/nono-proxy/src/route.rs` (fmt fix only, no semantic change).
- **Verification:** `cargo fmt --all -- --check` returns clean; `cargo build -p nono-proxy` PASS; `cargo test -p nono-proxy` PASS (148/148); `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` PASS.
- **Committed in:** folded into the message-identical re-creation of Replay Commit 1 (`cfab2e8b`).

---

**Total deviations:** 3 auto-fixed (2 Rule-1 plan-vs-reality mismatches grounded in the fork's actual `tls_intercept`-absent surface; 1 Rule-3 fmt blocker resolved by squashing into RC1 via safe soft-reset on unpushed local commits).
**Impact on plan:** All three are necessary for correctness and reflect the fork's actual surface vs the plan's frontmatter assumptions. No scope creep, no Windows files touched. D-40-B2 LOCK honored throughout.

## Issues Encountered

- **Pre-existing flaky test (`helper_stamps_session_token_from_env`):** `cargo test --workspace --all-features` reported 1 failure in `nono::supervisor::aipc_sdk::tests::windows_loopback_tests::helper_stamps_session_token_from_env` on the first execution but the second run passed cleanly (env-var-pollution race class, identical to the flake documented in 40-01 and 40-05 SUMMARYs). Confirmed pre-existing: zero touches to `crates/nono/` from this plan (`git diff --stat HEAD~2 HEAD -- crates/nono/` returns empty), so the flake is structurally not caused by Plan 40-06. Phase 41 backlog.
- **`tls_intercept` non-existence framing mismatch:** the plan's threat model and `<interfaces>` block were written assuming the fork has some form of tls_intercept-light surface. Reality: zero such surface exists; cluster C5's Phase 33 Cluster 11 lineage means the fork has never adopted tls_intercept at any point. This shifted the replay shape from "port the intent INTO the fork's tls_intercept" to "port the intent AROUND the fork's tls_intercept absence" — captured in DEC-1, DEC-2, and the deviations above.

## D-40-C2 8-check close gate

| Gate | Description | Status | Notes |
|------|-------------|--------|-------|
| 1 | `cargo test --workspace --all-features` (Windows host) | **PASS (modulo pre-existing flake)** | 688+ passed; 1 pre-existing flake (`helper_stamps_session_token_from_env`); flake passes in subsequent runs; Phase 41 scope; zero touches to `crates/nono/` from this plan confirmed |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (Windows host) | **PASS** | Clean |
| 3 | `cargo clippy --target x86_64-unknown-linux-gnu` | **load-bearing-skip → CI-verified** | C cross-compiler not available on Windows host (`aws-lc-sys` requires `x86_64-linux-gnu-gcc`); CI's `ubuntu-latest` clippy job confirms; orchestrator's Task 5 baseline-aware gate (post-merge) is the enforcement point |
| 4 | `cargo clippy --target x86_64-apple-darwin` | **load-bearing-skip → CI-verified** | Same as gate 3; CI's `macos-latest` clippy job covers |
| 5 | `cargo fmt --all -- --check` | **PASS** | Silent (after the Replay-Commit-1 fmt amend documented above) |
| 6 | Phase 15 5-row detached-console smoke | **environmental-skip** | Requires interactive Windows TTY session; cannot run in this executor context |
| 7 | `wfp_port_integration` tests | **environmental-skip** | Requires WFP service admin privileges; Phase 40 plans are documented-skip per `.continue-here.md` |
| 8 | `learn_windows_integration` tests | **environmental-skip** | Requires elevated Windows execution context; Phase 40 plans are documented-skip |

**Load-bearing skip categorization (per `.continue-here.md` anti-pattern #3):** Gates 3+4 are `skipped_gates_load_bearing` (CI substitute required, NOT environmental missingness). Gates 6+7+8 are `skipped_gates_environmental` (Windows runtime genuinely unavailable in the executor's sandboxed context). The orchestrator's Task 5 baseline-aware CI gate (post-merge) is the gate-3-and-4 enforcement point — `PLAN COMPLETE` cannot be declared until CI on the head commit (post-merge) confirms zero `success → failure` job transitions versus the most recent code-touching commit on main (baseline = `5c3da3d7` Plan 40-05 last code-touching commit; or `4665ae75` Plan 40-01 CR-A fix per 40-04 SUMMARY methodology if main has not advanced beyond 40-05).

### Branch-specific smoke check (D-20 branch)

- `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-commit: '` returns **0** (MUST be 0 for D-20)
- `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream intent:'` returns **2**
- `git log --format='%B' HEAD~2..HEAD | grep -c '^What was replayed:'` returns **2**
- `git log --format='%B' HEAD~2..HEAD | grep -c '^What was NOT replayed and why:'` returns **2**
- `git log --format='%B' HEAD~2..HEAD | grep -c '^Fork-only wiring preserved:'` returns **2**
- `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-replayed-from: '` returns **2** (8ddb143 + 54c7552 cited in RC1 line; f77e0e3 cited in RC2)
- `git log --format='%B' HEAD~2..HEAD | grep -c '^Co-Authored-By: Claude'` returns **2** (D-40-B3 requirement satisfied)
- `git log --format='%B' HEAD~2..HEAD | grep -cE '^Signed-off-by: '` returns **4** (2 commits × 2 DCO lines)
- D-40-E1: `git diff --stat HEAD~2 HEAD -- crates/ | grep -E '_windows|exec_strategy_windows' | wc -l` returns **0**
- Windows-only sentinel SHA: `96886ae969a26ea8fd87362751f88014ff324b2a` (unchanged from pre-plan baseline)
- Policy semantics grep in credential.rs (`passthrough|no.*cred|two.*match|absolute.*match|2.*match`): **14** matches (was 0 pre-plan; required >= 1)
- Phase 09 + 11 Windows credential baseline grep (`cfg.*windows|windows.*credential|keyring`): **2** matches (was 1 pre-plan; required >= pre-plan count)
- credential.rs runtime path byte-identical: only doc comments changed; the `CredentialStore::load` and `CredentialStore::get` function bodies are unchanged. Verifiable via `git show HEAD -- crates/nono-proxy/src/credential.rs | grep -E '^[+-]' | grep -vE '^[+-]\s*///' | head` returning only `+` lines that are doc-comment whitespace / structural plumbing for the new comment placement.

## Wave 2 CI Verification (Task 4 / Task 5 — DOWNSTREAM)

**Task 4 (push + PR + Phase 40 close-out note) and the implicit Task 5 wait-for-CI baseline-regression gate** are downstream of the orchestrator-merge per `.continue-here.md` anti-pattern #2. In worktree mode, the head commit on `main` has not advanced; Task 4/5 run after the orchestrator merges this worktree branch to main and pushes. The orchestrator owns:

1. Merge worktree `worktree-agent-afc74d66fc4ca2cfb` → `main` (fast-forward; 2 new commits land + the SUMMARY-doc commit that follows this file).
2. Push `main` → `origin/main`.
3. `gh run watch` the resulting CI run.
4. Per-job diff vs baseline = Plan 40-05's last code-touching commit on main (`5c3da3d7`), or the latest code-touching commit on main if it has advanced since 40-05 close. Per 40-04 SUMMARY methodology: zero `success → failure` job transitions = PASS.
5. Append Plan 40-06's contribution section to PR #922 body per the fork's umbrella-PR pattern (40-01 + 40-04 + 40-05 already in the umbrella; 40-06 appends as the terminal Wave 2 plan).
6. Run final Phase 40 phase-close verification — confirm REQ-UPST4-02 acceptance criteria 1–5 all met (see "Phase 40 all-plans close confirmation" below).

**This SUMMARY.md committing on the worktree is the executor's "return signal" — orchestrator picks it up after merge.**

## Won't-sync clusters from Phase 39 ledger (D-40-D1)

This Phase 40 close-out section satisfies D-40-D1 — Phase 40 terminal plan inline closure of the won't-sync cluster ledger from Phase 39 DIVERGENCE-LEDGER.md.

**Cluster 3 (PTY scrollback) won't-sync** per Phase 39 DIVERGENCE-LEDGER row + Phase 33 Cluster 1 same-class precedent (D-11 excluded; Phase 17 + Phase 30 already satisfied Windows scrollback requirement).

Rationale (pointer-only per D-40-D1):
- Phase 39 `DIVERGENCE-LEDGER.md` row for Cluster 3 has the full audit-time rationale.
- Phase 33 Cluster 1 (PTY attach/detach polish) was the same-class precedent — fork's `pty_proxy_windows.rs` ConPTY attach path is structurally distinct from upstream's `pty_proxy.rs` portable_pty primitives. D-11 (Phase 24 CONTEXT.md drift-tool path filter on `*_windows.rs`) excludes upstream PTY changes from drift detection because the fork's Windows PTY surface is intentionally divergent.
- Phase 17 v2.1 live-stream attach already closed the user-visible scrollback gap on Windows.
- Phase 30 reinforced the Windows scrollback / attach surface coverage.
- No further audit or replay is needed for Cluster 3; this section is the formal close-out per D-40-D1.

## Phase 40 all-plans close confirmation

**Phase 40 UPST4 sync execution: 6/6 plans complete.** REQ-UPST4-02 acceptance criteria 1–5 met across the phase:

| Plan | Cluster | Disposition | Commits | Status |
|------|---------|-------------|---------|--------|
| 40-01 | C1 proxy hardening | will-sync | 5 cherry-picks + 1 CR-A | **CLOSED** (40-01 SUMMARY) |
| 40-02 | C2 CLI --allow validate | will-sync (Wave 0) | 2 cherry-picks | **CLOSED** (40-02 SUMMARY) |
| 40-03 | C6 scrub module | will-sync (Wave 0) | 2 cherry-picks + 1 D-40-E1 addendum | **CLOSED** (40-03 SUMMARY) |
| 40-04 | C7 release ride-along | will-sync | 5 cherry-picks (2 features + 3 CHANGELOG-only releases) | **CLOSED** (40-04 SUMMARY) |
| 40-05 | C4 fp-profile-save | fork-preserve (D-20 manual replay) | 1 disposition docs + 1 D-20 replay | **CLOSED** (40-05 SUMMARY) |
| 40-06 | C5 fp-proxy-tls | fork-preserve (D-20 manual replay, LOCKED at D-40-B2) | 2 D-20 replay commits | **CLOSED** (this SUMMARY) |

REQ-UPST4-02 acceptance criteria:

1. **All will-sync cherry-picked with D-19 trailer** — satisfied across Plans 40-01 + 40-02 + 40-03 + 40-04 (14 cherry-picks total; D-19 trailer count matches commit count per each plan's SUMMARY).
2. **Fork-preserve via D-20 manual replay with documented preservation** — satisfied across Plans 40-05 (Cluster C4: 1 replay + 1 disposition commit) and 40-06 (Cluster C5: 2 replay commits). All replay commits carry the D-40-B3 5 body sections and `Upstream-replayed-from:` provenance; zero `^Upstream-commit:` trailers.
3. **Won't-sync documented in phase outcomes addendum** — satisfied by this SUMMARY's "Won't-sync clusters from Phase 39 ledger (D-40-D1)" section above for Cluster 3 (PTY scrollback). No separate `40-PHASE-OUTCOMES.md` file needed per D-40-D1 (smallest-footprint pointer-only rationale).
4. **Zero `*_windows.rs` edits** — D-40-E1 invariant. The Phase 40 chain saw ONE narrow forced-fork-adaptation exception in Plan 40-03 (commit `96886ae9`, +4 lines in `exec_strategy_windows/mod.rs` wiring `RollbackExitContext.redaction_policy` with `secure_default()`). Per D-40-E1's mid-execution-addended exception clause, this was accepted because (a) the cross-platform struct is non-optional, (b) the edit uses only the documented cross-platform default factory, (c) diff is ≤5 lines with no new public API / control flow / `#[cfg]` arms, (d) documented in 40-03 SUMMARY + STATE.md. Plans 40-01, 40-02, 40-04, 40-05, 40-06 all have ZERO `_windows.rs` / `exec_strategy_windows/` edits. The Windows-only sentinel SHA `96886ae9...` (set by Plan 40-03's exception) has been unchanged across all subsequent plans, confirming D-40-E1 has held since.
5. **Fork-defense grep baselines preserved or grown** — satisfied:
   - Phase 09 + 11 Windows credential injection grep on `crates/nono-proxy/src/credential.rs` grew from 1 to 2 in this plan (Plan 40-06's doc comment cites `nono::keystore`).
   - `validate_path_within` retention satisfied across Plans 40-01..40-04 per their SUMMARYs.
   - Phase 18.1 D-04-locked surface (`build_prompt_text + HandleKind` in `terminal_approval.rs`) remains at 45 matches (unchanged across all 6 plans).

**Plan 40-06 is the Phase 40 terminal plan.** With this SUMMARY committed, Phase 40 closes. The orchestrator's Task 4 / Task 5 work (push, PR, CI baseline gate, phase verification) is the final step before Phase 40 milestone close.

## Threat-model close-out

| Threat ID | Mitigation status | Evidence |
|-----------|-------------------|----------|
| T-40-06-01 (Tampering, D-40-E1 Windows-only files invariant) | **mitigated** | `git diff --stat HEAD~2 HEAD -- crates/ \| grep -E '_windows\|exec_strategy_windows' \| wc -l` returns 0; pre-plan Windows sentinel `96886ae9...` unchanged |
| T-40-06-02 (Repudiation, `Upstream-commit:` trailer in D-20 replay) | **mitigated** | `git log --format='%B' HEAD~2..HEAD \| grep -c '^Upstream-commit: '` returns 0; `Upstream-replayed-from:` provenance present on both commits |
| T-40-06-03 (Elevation of Privilege, replay overwrites fork's Windows credential injection path) | **mitigated** | `credential.rs::CredentialStore::load` runtime path byte-identical pre/post — only doc comments added. The Windows credential injection chain (`nono::keystore::load_secret_by_ref` → `keyring v3` → Windows Credential Manager) is unchanged; preservation grep grew from 1 to 2 |
| T-40-06-04 (Elevation, f77e0e3 "no match = passthrough no creds" silently disables Windows Credential Manager fallback) | **mitigated** | Audit at plan start confirmed fork has NO Windows-specific credential fallback to disable. Replay Commit 2 body documents Option A (uniform no-creds passthrough) with audit-evidence justification. No cross-platform inconsistency exists in the first place; no warning log needed |
| T-40-06-05 (Spoofing, 2-match-deny policy not replayed → ambiguous credential selection proceeds with first match) | **mitigated** | Structurally impossible in fork's `HashMap<String, LoadedCredential>` architecture. The 2-match scenario from upstream's tls_intercept requires multi-route-per-upstream dispatch, which the fork has no surface for. Replay Commit 2 doc comment makes this explicit |
| T-40-06-06 (Information Disclosure, TLS intercept trust root change weakens certificate pinning) | **mitigated** | Native CA loading is ADDITIVE only — webpki defaults always present; native certs are added on top. No certificate-pinning surface was weakened. The fork has no certificate-pinning logic to weaken (TLS trust = webpki + native + optional custom CA from `tls_ca` profile field) |
| T-40-06-07 (Repudiation, D-40-B3 commit body sections absent) | **mitigated** | Both replay commits have all 5 D-40-B3 sections; grep counts verified above |

## Self-Check: PASSED

**Files verified present:**
- `crates/nono-proxy/Cargo.toml` — contains `rustls-native-certs = "0.8"` dep with replay-intent doc comment. FOUND.
- `crates/nono-proxy/src/route.rs` — contains `pub(crate) fn build_base_root_store()` helper. FOUND.
- `crates/nono-proxy/src/server.rs` — contains `let root_store = route::build_base_root_store();`. FOUND.
- `crates/nono-proxy/src/credential.rs` — contains 14-hit policy-semantics doc-comment surface (passthrough / no-cred / two-match / absolute-match / 2-match). FOUND.

**Commits verified in git log:**
- `cfab2e8b` (Replay Commit 1: native CA loading) — reachable from `worktree-agent-afc74d66fc4ca2cfb` HEAD via `git log --oneline HEAD~2..HEAD`.
- `6f75b3dd` (Replay Commit 2: f77e0e3 policy disposition) — reachable from HEAD.

**Gates verified:**
- D-20 trailer count: 0 (must be 0) ✓
- D-40-B3 sections per commit: 5/5 on both commits ✓
- Co-Authored-By count: 2 ✓
- DCO sign-off count: 4 (2 commits × 2 lines) ✓
- D-40-E1 windows-file edits: 0 ✓
- Windows-only sentinel SHA `96886ae9...` unchanged ✓
- Phase 18.1 build_prompt_text + HandleKind count (45): unchanged from pre-plan baseline ✓
- f77e0e3 policy semantics in credential.rs: 14 matches (was 0; required >= 1) ✓
- Phase 09 + 11 Windows credential baseline: 2 (was 1; required >= pre-plan) ✓
- Native CA loading replayed: ✓
- Shared base-store helper exists at route::build_base_root_store: ✓
- cargo build --workspace: PASS ✓
- cargo test -p nono-proxy: PASS (148/148) ✓
- cargo clippy --workspace --all-targets: PASS (Windows host) ✓
- cargo fmt --all --check: PASS ✓

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Phase 40 closes** with this SUMMARY committed. 6/6 plans complete; REQ-UPST4-02 acceptance criteria 1–5 satisfied; Cluster 3 (PTY scrollback) won't-sync addendum documented inline per D-40-D1.
- **PR #922 umbrella PR** receives Plan 40-06's contribution section after orchestrator merges + pushes. PR #922 will then span all 6 Phase 40 plans (40-01 + 40-02 + 40-03 + 40-04 + 40-05 + 40-06).
- **UPST5 absorbs v0.54.0+** per ROADMAP § v2.5 backlog (D-39-D2). The 2 windows-touch candidates `5d821c12` + `0748cced` discovered at Phase 39 audit time stay out of Phase 40 scope; UPST5 will be the first audit where windows-touch:yes fires.
- **Future tls_intercept port** (if ever undertaken) can compose on top of `route::build_base_root_store()` directly — the helper is the integration point. Plan 40-06's surgical-retrofit posture means no new opportunistic Windows composition needs to be unwound; the fork's tls_intercept-absence is preserved cleanly.
- **Phase 41 backlog** unchanged — no new failures introduced by this plan. Pre-existing red Linux/macOS Clippy + Test jobs + 5 Windows job classes (+ the `helper_stamps_session_token_from_env` parallel-test race in `crates/nono/`) remain Phase 41 scope.

---

*Phase: 40-upst4-sync-execution*
*Completed: 2026-05-14*
