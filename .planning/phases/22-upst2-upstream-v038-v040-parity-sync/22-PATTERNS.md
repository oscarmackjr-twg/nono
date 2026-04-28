# Phase 22: UPST2 — Upstream v0.38–v0.40 Parity Sync — Pattern Map

**Mapped:** 2026-04-27
**Files analyzed:** 14 file groups across 5 plans (22-01..22-05)
**Analogs found:** 13 / 14 (one fork-only addition has no in-fork analog: Authenticode FFI; documented separately)

> Cross-cuts: every plan inherits the Phase 20 `Upstream-commit:` trailer convention (see commit `8cb85037` for a manual-port commit body, `af5c1249` / `f377a3e` / `ec73a8a` for clean cherry-picks). Verifier gates use `make ci`. Plan 22-05 follows the chronological cherry-pick choreography described in 22-CONTEXT.md § Specifics.

---

## File Classification

| New / Modified File | Plan | Role | Data Flow | Closest Analog | Match Quality |
|---------------------|------|------|-----------|----------------|---------------|
| `crates/nono-cli/src/profile/mod.rs` (new fields) | 22-01 | model (serde struct) | transform | existing `Profile`/`ProfileDeserialize` field shape (mod.rs:1175–1310) | exact |
| `crates/nono-cli/src/profile/builtin.rs` + `data/policy.json` (claude-no-keychain) | 22-01 | config / data file | transform | `claude-code` builtin in `policy.json:651–716` + tests in `builtin.rs:23–43` | exact |
| `crates/nono-cli/src/cli.rs` (clap conflicts_with for POLY-02) | 22-02 | CLI controller (clap) | request-response | existing `--rollback` / `--no-audit` already wired with `conflicts_with` (cli.rs:1555, 1602) | exact (no-op verification) |
| `crates/nono-cli/src/policy.rs` / `profile/mod.rs` (POLY-01 `override_deny` orphan check) | 22-02 | service (resolver) | transform | `policy::apply_deny_overrides` already implements the rejection (policy.rs:774–845; tests at 2689–2733) | exact |
| `crates/nono-cli/src/package_cmd.rs` (NEW) | 22-03 | CLI controller (subcommand tree) | request-response | `cli.rs::Commands::Audit(AuditArgs)` (cli.rs:680, 2030–2045) + `audit_commands.rs` dispatch | role-match |
| `crates/nono-cli/src/registry_client.rs` (NEW) | 22-03 | service (HTTP client) | streaming | `nono-proxy/src/server.rs::start` rustls/hyper client setup (server.rs:233–245) | role-match |
| `crates/nono-cli/src/package_cmd.rs` Windows install_dir | 22-03 | service (filesystem) | file-I/O | `config::user_config_dir` + `profile::resolve_user_config_dir` (config/mod.rs:100–110, profile/mod.rs:1769–1817) | exact |
| `crates/nono-cli/src/package_cmd.rs` Windows long-path `\\?\` | 22-03 | utility (path) | file-I/O | `crates/nono/src/sandbox/windows.rs::try_set_mandatory_label` `encode_wide` UTF-16 path pattern (windows.rs:514–542) | role-match |
| `crates/nono-proxy/src/oauth2.rs` (NEW) | 22-04 | service (auth client) | request-response | `nono-proxy/src/server.rs::start` rustls + `Zeroizing<String>` token + `nono::keystore::load_secret_by_ref` (keystore.rs:201–216) | role-match |
| `crates/nono-proxy/src/reverse.rs` HTTP loopback gating | 22-04 | middleware (network policy) | request-response | `profile::validate_upstream_url` loopback/unspecified gate (profile/mod.rs:598–633) | exact |
| `crates/nono-cli/src/{audit_integrity,audit_session,audit_ledger,audit_attestation}.rs` (NEW; flat layout per upstream 4f9552ec/02ee0bd1/6ecade2e — files at `crates/nono-cli/src/` top level, NOT a `audit/` sub-module directory) | 22-05 | service (ledger) | event-driven | `nono::undo::merkle::MerkleTree` (merkle.rs:36–93) for hash chain; `nono::trust::signing::sign_bytes_inner` (signing.rs:252–319) for DSSE | exact |
| `crates/nono-cli/src/audit_authenticode.rs` (NEW; fork-only) | 22-05 | utility (Windows FFI) | request-response | `crates/nono/src/sandbox/windows.rs::try_set_mandatory_label` (windows.rs:514–600) — `encode_wide`, `unsafe { ... }` blocks, `// SAFETY:` doc, RAII `_sd_guard`, `GetLastError` → `NonoError::LabelApplyFailed`-shaped errors | partial (no Authenticode-specific analog; see § No Analog Found) |
| `crates/nono-cli/src/cli.rs` (`prune` → `session cleanup` rename + hidden alias) | 22-05 | CLI controller (clap) | request-response | existing aliases: `Detach` `alias = "pause"` (cli.rs:738), `Attach` `alias = "resume"` (cli.rs:753); existing hidden command `OpenUrlHelper` `#[command(hide = true)]` (cli.rs:817) | role-match |
| `crates/nono-cli/src/session_commands.rs` + `session_commands_windows.rs` (rename plumbing) | 22-05 | service (session lifecycle) | event-driven | existing `auto_prune_if_needed` + `reject_if_sandboxed` + `AUTO_PRUNE_STALE_THRESHOLD` (session_commands.rs:20–120) — preserve byte-for-byte through rename per CLEAN-04 | exact |
| Windows-only test files (D-15: `package_integration.rs`, `audit_attestation.rs` Windows cases) | all plans | test (integration) | request-response | `crates/nono-cli/tests/wfp_port_integration.rs:1–40` `#![cfg(target_os = "windows")]` + `#![allow(clippy::unwrap_used)]` + admin-skipped pattern; sandbox/windows.rs::tests:1788–1830 cfg-gated unit tests | exact |
| Test fixture port surface (D-13/D-14) | 22-03/22-04/22-05 | test (inline + integration) | n/a | Phase 20 commits `8cb85037` (manual-port body), `af5c1249` (clean cherry-pick body) | exact |

---

## Pattern Assignments

### 1. Profile struct field additions (Plan 22-01 PROF-01..03) — `crates/nono-cli/src/profile/mod.rs`

**Role:** model · **Data flow:** transform (serde JSON → typed Rust)
**Analog:** existing `Profile` struct + `ProfileDeserialize` companion at `crates/nono-cli/src/profile/mod.rs:1175–1310`

**Imports / serde annotations** (mod.rs:1180–1222):
```rust
#[serde(default, deserialize_with = "deserialize_extends")]
pub extends: Option<Vec<String>>,
#[serde(default)]
pub meta: ProfileMeta,
#[serde(default)]
pub security: SecurityConfig,
#[serde(default, alias = "secrets")]
pub env_credentials: SecretsConfig,
#[serde(default, alias = "undo")]
pub rollback: RollbackConfig,
#[serde(default)]
pub open_urls: Option<OpenUrlConfig>,
#[serde(default)]
pub allow_launch_services: Option<bool>,
#[serde(default)]
pub capabilities: CapabilitiesConfig,
```

**Companion deserialize struct + impl From** (mod.rs:1225–1280):
```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileDeserialize {
    #[serde(rename = "$schema", default)]
    _schema: Option<String>,
    #[serde(default, deserialize_with = "deserialize_extends")]
    extends: Option<Vec<String>>,
    // ... matches Profile field-by-field ...
}

impl From<ProfileDeserialize> for Profile {
    fn from(raw: ProfileDeserialize) -> Self {
        Self { extends: raw.extends, meta: raw.meta, /* ... */ }
    }
}
```

**Apply pattern (PROF-01 macOS-only Seatbelt rules):** the cfg-gated apply pattern is the same shape as the existing AIPC apply path. Mirror `Profile::resolve_aipc_allowlist` (mod.rs:706–740) for any new `resolve_*` helper.

**Validation pattern (PROF-03 https-only `token_url`):** existing `validate_upstream_url` (mod.rs:598–633) is the *exact* shape REQ-PROF-03 needs:
```rust
fn validate_upstream_url(url: &str, service_name: &str) -> Result<()> {
    let parsed = url::Url::parse(url).map_err(|e| {
        NonoError::ProfileParse(format!("Invalid upstream URL ... '{}': {}", service_name, e))
    })?;
    match parsed.scheme() {
        "https" => Ok(()),
        "http" => {
            let is_loopback = match parsed.host() {
                Some(url::Host::Ipv4(ip)) => ip.is_loopback() || ip.is_unspecified(),
                Some(url::Host::Ipv6(ip)) => ip.is_loopback() || ip.is_unspecified(),
                Some(url::Host::Domain(domain)) => domain == "localhost",
                None => false,
            };
            if is_loopback { Ok(()) } else {
                Err(NonoError::ProfileParse(format!(
                    "Upstream URL ... must use HTTPS (HTTP only allowed for loopback): {}", url
                )))
            }
        }
        scheme => Err(NonoError::ProfileParse(format!(
            "Upstream URL ... must use HTTPS, got scheme '{}': {}", scheme, url))),
    }
}
```

**Coordination warning (Pitfall 5):** Plan 22-01 step 7 (`b1ecbc02`, +364 LOC mod.rs) hits fork drift +732/-414 — D-02 manual replay highly likely. Plan executor must coordinate with 22-02 to avoid `mod.rs` race.

---

### 2. `claude-no-keychain` builtin profile (Plan 22-01 PROF-04)

**Role:** config (data file) + thin Rust shim · **Data flow:** transform
**Analog:** `claude-code` entry in `crates/nono-cli/data/policy.json:651–716` + test fixture in `crates/nono-cli/src/profile/builtin.rs:23–43`

**`policy.json` builtin shape** (policy.json:651–716):
```json
"claude-code": {
  "extends": "default",
  "meta": {
    "name": "claude-code",
    "version": "1.0.0",
    "description": "Anthropic Claude Code CLI agent",
    "author": "nono-project"
  },
  "security": {
    "groups": [
      "claude_code_macos", "claude_code_linux", "user_caches_macos",
      "claude_cache_linux", "node_runtime", "rust_runtime", "python_runtime",
      "vscode_macos", "vscode_linux", "linux_sysfs_read", "nix_runtime",
      "git_config", "unlink_protection"
    ],
    "signal_mode": "isolated",
    "capability_elevation": false
  },
  "filesystem": {
    "allow": ["$HOME/.claude", "$HOME/.cache/claude"],
    "allow_file": ["$HOME/.claude.json", "$HOME/.claude.json.lock", "$HOME/.claude.lock"]
  },
  "network": { "block": false },
  "workdir": { "access": "readwrite" },
  "open_urls": { "allow_origins": ["https://claude.ai"], "allow_localhost": true },
  "capabilities": { "aipc": { /* ... */ } },
  "allow_launch_services": true,
  "hooks": { "claude-code": { "event": "PostToolUseFailure", /* ... */ } },
  "undo": { "exclude_patterns": ["node_modules", ".next", "__pycache__", "target"] },
  "interactive": true
}
```

**`builtin.rs` is a thin shim — no per-profile Rust code** (builtin.rs:1–16):
```rust
//! Built-in profiles compiled into the nono binary
//!
//! Profiles are defined declaratively in `policy.json` under the `profiles` key.
//! This module delegates to the policy resolver for loading and listing.
use super::Profile;

pub fn get_builtin(name: &str) -> Option<Profile> {
    crate::policy::get_policy_profile(name).ok().flatten()
}

pub fn list_builtin() -> Vec<String> {
    crate::policy::list_policy_profiles().unwrap_or_default()
}
```

**Test pattern** (builtin.rs:23–43):
```rust
#[test]
fn test_get_builtin_claude_code() {
    let profile = get_builtin("claude-code").expect("Profile not found");
    assert_eq!(profile.meta.name, "claude-code");
    assert!(!profile.network.block);
    assert!(profile.security.groups.contains(&"deny_credentials".to_string()));
    assert!(profile.filesystem.allow_file.contains(&"$HOME/.claude.lock".to_string()));
}
```

**Implication for PROF-04:** the `claude-no-keychain` work is mostly a `policy.json` JSON insert (clone the `claude-code` block, drop keychain-touching `allow_file` entries, reuse remaining shape). The `builtin.rs` Rust code needs only an additional cfg-gated `#[test]` mirroring the analog above. **Cross-plan dependency:** depends on POLY-03's `49925bbf` for the `.claude.lock` `allow_file` move (per CONTEXT.md Pitfall 5; CONTEXT.md cheat-sheet step 11 = `713b2e0f` follow-up).

---

### 3. clap `conflicts_with` for POLY-02 (Plan 22-02) — `crates/nono-cli/src/cli.rs`

**Role:** CLI controller · **Data flow:** request-response (parse-time)
**Analog:** the conflict is **already wired** in fork's `cli.rs` — the upstream change may be a no-op semantically.

**Existing conflict already in place** (cli.rs:1553–1604):
```rust
// ── Rollback ──────────────────────────────────────────────────────
/// Enable atomic rollback snapshots for the session
#[arg(long, conflicts_with = "no_rollback", help_heading = "ROLLBACK")]
pub rollback: bool,

/// Disable rollback entirely (no snapshots taken)
#[arg(long, conflicts_with = "rollback", help_heading = "ROLLBACK")]
pub no_rollback: bool,

// ── Options ────────────────────────────────────────────────────────
/// Disable the audit trail for this session
#[arg(long, conflicts_with = "rollback", help_heading = "OPTIONS")]
pub no_audit: bool,
```

**Other `conflicts_with` patterns in fork to mirror** (cli.rs:1022–1041):
```rust
#[arg(
    long = "block-net",
    alias = "net-block",
    conflicts_with = "allow_net",
    env = "NONO_BLOCK_NET",
    value_parser = clap::builder::BoolishValueParser::new(),
    /* ... */
)]
pub block_net: bool,

#[arg(
    long = "no-net-blocking",
    /* ... */
    conflicts_with_all = ["block_net", "network_profile", /* ... */],
)]
pub no_net_blocking: bool,
```

**[CONTRADICTION SURFACED]:** fork already enforces `--rollback`/`--no-audit` mutual exclusion via `conflicts_with` (cli.rs:1602). Upstream `5c301e8d` may have already been "ported by accident" in earlier work. Planner must (a) verify the upstream commit's exact `conflicts_with` shape (potentially `conflicts_with_all` covering both `--rollback` and `--no-rollback-prompt`), (b) reconcile with fork's existing wiring before cherry-pick, (c) audit fork rollback tests per CONTEXT.md Pitfall 4 even if the clap change is a no-op.

---

### 4. `override_deny` orphan rejection (Plan 22-02 POLY-01) — `crates/nono-cli/src/policy.rs`

**Role:** service (resolver) · **Data flow:** transform (validate-then-apply)
**Analog:** **`policy::apply_deny_overrides` already implements POLY-01's exact semantic** (policy.rs:774–845) — fail-closed when no user-intent grant covers the override path.

**Core rejection pattern** (policy.rs:806–845):
```rust
// Verify the override path is actually granted via explicit user intent
// (CLI flags or profile filesystem/policy config), not just covered by a
// system or group grant. Without this, a deny override under /tmp would
// silently pass because system_write_macos grants /var/folders, creating
// an unintended permission grant.
let mut grant_has_read = false;
let mut grant_has_write = false;
for cap in caps.fs_capabilities() {
    if !cap.source.is_user_intent() {
        continue;
    }
    let covers = if cap.is_file {
        cap.resolved == canonical
    } else {
        canonical.starts_with(&cap.resolved)
    };
    if covers {
        match cap.access {
            AccessMode::Read => grant_has_read = true,
            AccessMode::Write => grant_has_write = true,
            AccessMode::ReadWrite => { grant_has_read = true; grant_has_write = true; }
        }
    }
}
if !grant_has_read && !grant_has_write {
    return Err(NonoError::SandboxInit(format!(
        "override_deny '{}' has no matching grant. \
         Add a filesystem allow (--allow, --read, --write, or profile filesystem/policy) \
         for this path.",
        override_path.display(),
    )));
}
```

**Error variant pattern (existing `NonoError`)** — see `crates/nono/src/error.rs:80, 36, 42`:
```rust
#[error("Profile parse error: {0}")]
ProfileParse(String),
#[error("Sandbox initialization failed: {0}")]
SandboxInit(String),
#[error("Command '{command}' is blocked: {reason}")]
BlockedCommand { command: String, reason: String },
```

Fork uses `NonoError::SandboxInit` for the existing override-deny rejection. Upstream's `5c301e8d` may introduce a dedicated `NonoError::PolicyError { kind, ... }` variant — planner must reconcile: extend `NonoError` with a new `PolicyError` variant per CONTEXT.md Integration Points, OR keep the existing `SandboxInit`/`ProfileParse` shape if upstream's variant collides with fork's drift.

**Test pattern** (policy.rs:2689–2733):
```rust
#[test]
fn test_apply_deny_overrides_rejects_missing_grant() {
    // No grant added — override should fail
    let result = apply_deny_overrides(&overrides, &mut deny_paths, &mut caps);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no matching grant"));
}

#[test]
fn test_apply_deny_overrides_rejects_group_sourced_grant() {
    // Group-sourced grant must NOT satisfy override_deny
    let mut cap = FsCapability::new_dir(dir.path(), AccessMode::ReadWrite).expect("grant");
    cap.source = CapabilitySource::Group("system_write".to_string());
    caps.add_fs(cap);
    let result = apply_deny_overrides(&overrides, &mut deny_paths, &mut caps);
    assert!(result.is_err());
}
```

**[CONTRADICTION SURFACED]:** fork already encodes "user-intent-only grant" semantic, which is *stricter* than upstream's "any grant" check. Cherry-picking `b83da813` ("filter override_deny entries without grants") may **conflict structurally** with this richer check. Recommend: keep fork's user-intent semantic, port upstream's filtering logic on top (so both restrictions apply). Document deviation in commit body.

---

### 5. Package manager subcommand tree (Plan 22-03 PKG-01) — NEW `crates/nono-cli/src/package_cmd.rs` + `registry_client.rs`

**Role:** CLI controller (subcommand) + service (HTTP client) · **Data flow:** request-response + streaming

#### 5a. Subcommand wiring analog (`Audit`-style)

**Top-level `Commands` variant pattern** (cli.rs:680–702):
```rust
#[command(subcommand_help_heading = "COMMANDS", disable_help_subcommand = true)]
#[command(help_template = "\
{about}

\x1b[1mUSAGE\x1b[0m
  nono audit <command>

{all-args}
{after-help}")]
#[command(after_help = "\x1b[1mEXAMPLES\x1b[0m
  nono audit list                              # List all sessions
  nono audit list --today                      # List sessions from today
  nono audit show <id> --json                  # Export as JSON
")]
Audit(AuditArgs),
```

**Parent args + subcommand enum** (cli.rs:2030–2096):
```rust
#[derive(Parser, Debug)]
#[command(disable_help_flag = true)]
pub struct AuditArgs {
    #[command(subcommand)]
    pub command: AuditCommands,

    /// Print help
    #[arg(long, short = 'h', action = clap::ArgAction::Help, help_heading = "OPTIONS")]
    pub help: Option<bool>,
}

#[derive(Subcommand, Debug)]
pub enum AuditCommands {
    /// List all sandboxed sessions
    List(AuditListArgs),
    /// Show audit details for a session
    Show(AuditShowArgs),
}

#[derive(Parser, Debug)]
#[command(disable_help_flag = true)]
pub struct AuditListArgs {
    #[arg(long)]
    pub today: bool,
    #[arg(long, value_name = "DATE")]
    pub since: Option<String>,
    #[arg(long)]
    pub json: bool,
    #[arg(long, short = 'h', action = clap::ArgAction::Help, help_heading = "OPTIONS")]
    pub help: Option<bool>,
}
```

Apply this exact shape for `Package(PackageArgs)` → `PackageCommands { Pull, Remove, Search, List }`.

**Dispatch site** (cli.rs:3124+):
```rust
Commands::Audit(args) => match args.command {
    AuditCommands::List(list_args) => /* dispatch */,
    AuditCommands::Show(show_args) => /* dispatch */,
}
```

#### 5b. HTTP client pattern (registry streaming)

**Existing rustls + Zeroizing setup** (`crates/nono-proxy/src/server.rs:233–245`):
```rust
let mut root_store = rustls::RootCertStore::empty();
root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
let tls_config = rustls::ClientConfig::builder_with_provider(Arc::new(
    rustls::crypto::ring::default_provider(),
))
.with_safe_default_protocol_versions()
.map_err(|e| ProxyError::Config(format!("TLS config error: {}", e)))?
.with_root_certificates(root_store)
.with_no_client_auth();
let tls_connector = tokio_rustls::TlsConnector::from(Arc::new(tls_config));
```

**Why this matters:** fork already has the *exact* rustls + ring + webpki-roots configuration upstream's `registry_client.rs` and `oauth2.rs` will need. Reuse — do NOT introduce reqwest, isahc, or alternative TLS providers. Per RESEARCH § Don't Hand-Roll.

---

### 6. Windows install_dir resolution + long-path handling (Plan 22-03 PKG-02)

**Role:** utility (path) · **Data flow:** file-I/O

#### 6a. Cross-platform `dirs` analog

**Already in fork** (`crates/nono-cli/src/config/mod.rs:98–110`):
```rust
/// Get the user config directory path
pub fn user_config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("nono"))
}

pub fn user_state_dir() -> Option<PathBuf> {
    dirs::state_dir()
        .or_else(dirs::data_local_dir)
        .map(|p| p.join("nono"))
}
```

**`%APPDATA%` Windows fallback with canonicalize** (`profile/mod.rs:1769–1817`):
```rust
pub(crate) fn resolve_user_config_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    if let Ok(raw) = std::env::var("APPDATA") {
        let path = PathBuf::from(&raw);
        if path.is_absolute() {
            match path.canonicalize() {
                Ok(canonical) => return Ok(canonical),
                Err(_) => return Ok(path),
            }
        }
    }

    if let Ok(raw) = std::env::var("XDG_CONFIG_HOME") {
        let path = PathBuf::from(&raw);
        if path.is_absolute() {
            match path.canonicalize() {
                Ok(canonical) => return Ok(canonical),
                Err(e) => {
                    tracing::warn!("Ignoring invalid XDG_CONFIG_HOME='{}' ... falling back", raw);
                }
            }
        }
    }

    let home = home_dir()?;
    let home_base = match home.canonicalize() {
        Ok(canonical) => canonical,
        Err(e) => {
            tracing::warn!("Failed to canonicalize HOME='{}' ({}) ...", home.display(), e);
            home
        }
    };
    Ok(home_base.join(".config"))
}
```

**Implication:** PKG-02's `%LOCALAPPDATA%\nono\packages\<name>` resolution should **mirror** this exact pattern: try env-var (`LOCALAPPDATA`), require absolute, canonicalize-with-fallback, fail to `dirs::data_local_dir()` if env is poisoned. **DO NOT** introduce direct `windows-sys::SHGetKnownFolderPath` — fork uses `dirs` crate exclusively for this surface.

#### 6b. Windows long-path / wide-string FFI pattern

**`encode_wide` UTF-16 conversion** (`crates/nono/src/sandbox/windows.rs:514–528`):
```rust
pub fn try_set_mandatory_label(path: &Path, mask: u32) -> Result<()> {
    use windows_sys::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_INVALID_FUNCTION, ERROR_NOT_SUPPORTED,
    };

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // ...
}
```

**Use for:** any path being passed to a `*W` Win32 API (path-traversal safety, long-path handling). Note: Rust `std::fs::canonicalize` on Windows already returns an extended-length `\\?\`-prefixed path, so no manual prefix concat is needed — see RESEARCH § Don't Hand-Roll.

---

### 7. OAuth2 client-credentials flow + token cache (Plan 22-04 OAUTH-01) — NEW `crates/nono-proxy/src/oauth2.rs`

**Role:** service (auth client) · **Data flow:** request-response

**Credential resolution analog** — `nono::keystore::load_secret_by_ref` (`crates/nono/src/keystore.rs:201–216`):
```rust
#[must_use = "loaded secret should be used or explicitly dropped"]
pub fn load_secret_by_ref(service: &str, credential_ref: &str) -> Result<Zeroizing<String>> {
    if credential_ref.starts_with(FILE_URI_PREFIX) {
        load_from_file(credential_ref)
    } else if credential_ref.starts_with(ENV_URI_PREFIX) {
        load_from_env(credential_ref)
    } else if credential_ref.starts_with(OP_URI_PREFIX) {
        load_from_op(credential_ref)
    } else if is_apple_password_uri(credential_ref) {
        load_from_apple_password(credential_ref)
    } else if is_keyring_uri(credential_ref) {
        load_from_keyring_uri(credential_ref)
    } else {
        load_single_secret(service, credential_ref)
    }
}
```

**Use for `client_secret: keyring://...`:** call `load_secret_by_ref("nono-oauth2", &client_secret_uri)` once at OAuth2 client init; the returned `Zeroizing<String>` auto-zeroes on Drop.

**Zeroizing import + token storage analog** (nono-proxy/src/server.rs:164, reverse.rs:30):
```rust
use zeroize::Zeroizing;

struct ProxyState {
    /* ... */
    session_token: Zeroizing<String>,
    /* ... */
}
```

**TLS connector reuse** — see § 5b above.

**`#[must_use]` discipline** (keystore.rs:201) — apply to `oauth2::TokenCache::get_token`, `oauth2::exchange_token`, etc. per CLAUDE.md § Coding Standards.

**Test fixture deviation:** RESEARCH § Pattern 3 calls out that upstream's `oauth2.rs` test code uses bare `.unwrap()`. Fork must wrap the test module with `#[cfg(test)] #[allow(clippy::unwrap_used)] mod tests { ... }` per CLAUDE.md.

---

### 8. Reverse-proxy HTTP upstream loopback gating (Plan 22-04 OAUTH-02) — `crates/nono-proxy/src/reverse.rs`

**Role:** middleware (network policy) · **Data flow:** request-response (allow / deny)
**Analog:** `crates/nono-cli/src/profile/mod.rs:598–633` `validate_upstream_url`

**Exact loopback / unspecified IP class gate**:
```rust
let is_loopback = match parsed.host() {
    Some(url::Host::Ipv4(ip)) => ip.is_loopback() || ip.is_unspecified(),
    Some(url::Host::Ipv6(ip)) => ip.is_loopback() || ip.is_unspecified(),
    Some(url::Host::Domain(domain)) => domain == "localhost",
    None => false,
};

if is_loopback {
    Ok(())
} else {
    Err(NonoError::ProfileParse(format!(
        "Upstream URL ... must use HTTPS (HTTP only allowed for loopback): {}", url
    )))
}
```

**This shape is exactly what upstream `0c990116` enforces** (REQ-OAUTH-02 acceptance: `0.0.0.0` rejected, `192.168.x.x` rejected, `127.0.0.1` allowed). Reuse the helper or factor it out into a shared utility — do NOT reimplement in `reverse.rs`.

---

### 9. Audit ledger hash-chain + Merkle root (Plan 22-05 AUD-01) — NEW `crates/nono-cli/src/audit_integrity.rs` + `audit_session.rs`

**Role:** service (ledger) · **Data flow:** event-driven (append-only chain)
**Analog:** `crates/nono/src/undo/merkle.rs::MerkleTree` — already used by fork's snapshot system for cryptographic state commitment.

**Domain-separated leaf / internal hashing** (merkle.rs:17–34):
```rust
/// Domain separation prefixes per RFC 6962 to prevent second-preimage attacks.
/// Leaf and internal nodes use distinct prefixes so an attacker cannot substitute
/// a leaf node hash for an internal node hash or vice versa.
const LEAF_PREFIX: u8 = 0x00;
const INTERNAL_PREFIX: u8 = 0x01;

pub struct MerkleTree {
    root: ContentHash,
    leaf_count: usize,
}
```

**Tree construction** (merkle.rs:41–93):
```rust
pub fn from_manifest(files: &HashMap<PathBuf, FileState>) -> Result<Self> {
    if files.is_empty() {
        let empty_root: [u8; 32] = Sha256::digest(b"").into();
        return Ok(Self {
            root: ContentHash::from_bytes(empty_root),
            leaf_count: 0,
        });
    }

    let mut sorted_paths: Vec<&PathBuf> = files.keys().collect();
    sorted_paths.sort();

    let mut level: Vec<[u8; 32]> = sorted_paths
        .iter()
        .map(|path| {
            let file_state = &files[*path];
            compute_leaf_hash(path, &file_state.hash)
        })
        .collect();

    let leaf_count = level.len();

    while level.len() > 1 {
        let mut next_level = Vec::with_capacity(level.len().saturating_add(1) / 2);
        let mut i = 0;
        while i < level.len() {
            if i + 1 < level.len() {
                next_level.push(compute_internal_hash(&level[i], &level[i + 1]));
                i += 2;
            } else {
                next_level.push(level[i]);
                i += 1;
            }
        }
        level = next_level;
    }
    /* ... */
}
```

**Implication:** upstream's `audit_integrity::v3 → Alpha` migration (RESEARCH state-of-the-art) follows the same RFC 6962 domain-separation pattern. The fork's `MerkleTree` is **directly reusable** for AUD-01's hash chain — wrap it for the per-event append-only chain semantic; do NOT introduce a parallel Merkle implementation.

---

### 10. DSSE / in-toto attestation signing (Plan 22-05 AUD-02)

**Role:** service (signing) · **Data flow:** transform (bytes → DSSE bundle)
**Analog:** `crates/nono/src/trust/signing.rs::sign_bytes_inner` (signing.rs:322+)

**Public signing entry point** (signing.rs:252–265):
```rust
pub fn sign_bytes(
    content: &[u8],
    filename: &str,
    key_pair: &KeyPair,
    key_id: &str,
) -> Result<String> {
    sign_bytes_inner(
        content,
        filename,
        key_pair,
        key_id,
        dsse::NONO_PREDICATE_TYPE,
    )
}
```

**Error variant** — fork already has `NonoError::TrustSigning { path, reason }` (error.rs:181–182):
```rust
#[error("Signing failed for {path}: {reason}")]
TrustSigning { path: String, reason: String },
```

**Implication:** REQ-AUD-02 acceptance ("`nono audit verify` succeeds for untampered, fails for tampered" + "Verify surfaces signer key ID") reuses `sign_bytes` directly with a NEW predicate type constant (e.g., `NONO_AUDIT_PREDICATE_TYPE` in `crates/nono/src/trust/dsse.rs`). The DSSE bundle round-trip already has 4 inline tests in `signing.rs` (`sign_bytes_*`) — port the same shape for `sign_audit_merkle_root`.

---

### 11. Windows exec-identity (Authenticode + SHA-256 fallback) — NEW `crates/nono-cli/src/audit_authenticode.rs`

**Role:** utility (Windows FFI) · **Data flow:** request-response (file → signature status)
**Analog:** `crates/nono/src/sandbox/windows.rs::try_set_mandatory_label` (windows.rs:514–600) — same windows-sys 0.59 `unsafe { ... }` block + `// SAFETY:` doc + RAII guard + `GetLastError` → typed `NonoError` shape.

**`encode_wide` path → wide-string + RAII guard pattern** (windows.rs:514–554):
```rust
pub fn try_set_mandatory_label(path: &Path, mask: u32) -> Result<()> {
    use windows_sys::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_INVALID_FUNCTION, ERROR_NOT_SUPPORTED,
    };

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let sddl = format!("S:(ML;;0x{mask:X};;;LW)");
    let wide_sddl: Vec<u16> = sddl.encode_utf16().chain(std::iter::once(0)).collect();

    let mut security_descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    let ok = unsafe {
        // SAFETY: `wide_sddl` is a valid nul-terminated UTF-16 buffer; the
        // output pointer is a valid mutable out-pointer for the duration of
        // the call. On success, the returned SD must be freed with LocalFree
        // (handled by the OwnedSecurityDescriptor guard below).
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide_sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut security_descriptor,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        let hresult = unsafe {
            // SAFETY: GetLastError has no preconditions.
            windows_sys::Win32::Foundation::GetLastError()
        };
        return Err(NonoError::LabelApplyFailed {
            path: path.to_path_buf(),
            hresult,
            hint: format!("Failed to construct mandatory-label SDDL (mask=0x{mask:X})"),
        });
    }
    let _sd_guard = OwnedSecurityDescriptor(security_descriptor);
    /* ... */
}
```

**Apply to Authenticode:**
- Same `encode_wide` for `WINTRUST_FILE_INFO::pcwszFilePath`.
- Same `unsafe { /* SAFETY: ... */ }` block discipline for `WinVerifyTrust`.
- Same RAII `_close_guard` pattern for the WTD_STATEACTION_CLOSE second call (per RESEARCH § Code Examples Pattern 4).
- New error variant **on the CLI side** (NOT in `crates/nono/src/error.rs`) — Authenticode is fork-only; do NOT extend `NonoError`. Instead introduce a sibling `AuthenticodeStatus { Valid, Unsigned, InvalidSignature(i32), NotApplicable }` enum in the CLI module.

**`#[cfg(target_os = "windows")]` test gating** — see § 13 below.

---

### 12. `prune` → `session cleanup` rename + hidden alias (Plan 22-05 AUD-04)

**Role:** CLI controller · **Data flow:** request-response

#### 12a. Hidden + aliased subcommand pattern

**Existing alias in fork** (cli.rs:738–756):
```rust
/// Detach from a running sandboxed session and return to the shell
#[command(
    help_template = "...",
    alias = "pause",
    after_help = DETACH_AFTER_HELP
)]
Detach(DetachArgs),

/// Attach to a detached or running session from another terminal
#[command(
    help_template = "...",
    alias = "resume",
    after_help = ATTACH_AFTER_HELP
)]
Attach(AttachArgs),
```

**Existing hidden subcommand** (cli.rs:816–818):
```rust
/// Internal: open a URL via supervisor IPC
#[command(hide = true)]
OpenUrlHelper(OpenUrlHelperArgs),
```

**Combine for `prune` deprecation:**
```rust
/// Clean up old session files (deprecated; use `nono session cleanup`)
#[command(hide = true)]
Prune(PruneArgs),
```

The `Prune` variant stays in the `Commands` enum but is hidden from `--help`; `Session(SessionArgs)` becomes the documented command and dispatches `Cleanup(SessionCleanupArgs)` underneath. Per CONTEXT.md Pitfall the deprecation note must be surfaced at runtime — see existing `command_blocking_deprecation.rs` for fork's deprecation-notice pattern (Phase 20 D-10 backport).

#### 12b. `auto_prune_if_needed` byte-exact preservation (CLEAN-04 D-04)

**Critical invariant — preserve through rename** (`crates/nono-cli/src/session_commands.rs:32–58`):
```rust
const AUTO_PRUNE_STALE_THRESHOLD: usize = 100;
const AUTO_PRUNE_RETENTION_SECS: u64 = 30 * 86_400;

fn auto_prune_if_needed() {
    // T-19-04-07: refuse to delete host supervisor's sessions from
    // within a sandboxed process. NONO_CAP_FILE is the canonical
    // "I am running inside nono" signal (same check as reject_if_sandboxed).
    if std::env::var_os("NONO_CAP_FILE").is_some() {
        debug!("auto-prune skipped: running inside sandbox (NONO_CAP_FILE set)");
        return;
    }
    // ... rest of function
}

fn reject_if_sandboxed(command: &str) -> Result<()> {
    if std::env::var_os("NONO_CAP_FILE").is_some() {
        return Err(NonoError::ConfigParse(format!(
            "`nono {}` cannot be used inside a sandbox.",
            command
        )));
    }
    Ok(())
}
```

**Test invariant (preserve test name + body)** (session_commands.rs:708):
```rust
#[test]
fn auto_prune_is_noop_when_sandboxed() {
    let _guard = EnvVarGuard::set_all(&[("NONO_CAP_FILE", "/tmp/fake-cap-file")]);
    // ... auto_prune_if_needed() called; assert no side effects
}
```

**This test exists in BOTH `session_commands.rs:708` AND `session_commands_windows.rs:801`.** Per D-04, both must pass after EVERY 22-05 commit touching `session_commands*` — re-run `cargo test -p nono-cli --bin nono auto_prune_is_noop_when_sandboxed` per CONTEXT.md § Specifics gate.

---

### 13. `#[cfg(target_os = "windows")]` test gating (D-15 / D-18)

**Role:** test (integration) · **Data flow:** n/a
**Analog:** Phase 21 WSFG pattern at `crates/nono/src/sandbox/windows.rs:1788–1830` (76 unit tests) + `crates/nono-cli/tests/wfp_port_integration.rs:1–40` (integration test).

**Inline unit test pattern** (sandbox/windows.rs:1788–1814):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccessMode, CapabilitySet, CapabilitySource, FsCapability, IpcMode, NetworkMode};
    use std::process::Command;
    use tempfile::tempdir;

    // Avoid relative-path `Command::new("<tool>")` for OS utilities — path-
    // hijack hazard on Windows. Resolve via `%SystemRoot%\System32\<tool>.exe`
    // with the same fallback chain used elsewhere in the codebase.
    fn system32_exe(name: &str) -> PathBuf {
        let system_root = std::env::var_os("SystemRoot")
            .or_else(|| std::env::var_os("windir"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
        system_root.join("System32").join(format!("{name}.exe"))
    }

    fn try_create_symlink_file(link: &Path, target: &Path) -> bool {
        match std::os::windows::fs::symlink_file(target, link) {
            Ok(()) => true,
            Err(err) => {
                eprintln!("skipping symlink escape test because symlink creation failed: {err}");
                false
            }
        }
    }
}
```

**Integration test pattern** (`crates/nono-cli/tests/wfp_port_integration.rs:1–40`):
```rust
//! Integration test: WFP port-level permit filter allows real TCP connections.
//!
//! Requires:
//! - Windows OS
//! - Administrator privileges (WFP filter installation) for the `#[ignore]`d test
//! - nono-wfp-service running (or test will skip gracefully)
//!
//! Run the policy-compilation test (no privileges required):
//!   cargo test -p nono-cli --test wfp_port_integration

#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]

use std::net::{TcpListener, TcpStream};
use std::time::Duration;

fn system32_exe(name: &str) -> std::path::PathBuf {
    let system_root = std::env::var_os("SystemRoot")
        .or_else(|| std::env::var_os("windir"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"));
    system_root.join("System32").join(format!("{name}.exe"))
}
```

**Apply to D-15 Windows-only tests:**
- `tests/package_integration.rs::windows_long_path` — file-level `#![cfg(target_os = "windows")]` + `#![allow(clippy::unwrap_used)]`
- `tests/package_integration.rs::windows_path_traversal_unc_alias`
- `tests/package_integration.rs::windows_localappdata_resolution`
- `tests/audit_attestation.rs::windows_authenticode_signed` (signed binary; gate `#[ignore]` because requires production-signed `nono.exe` on disk)
- `tests/audit_attestation.rs::windows_authenticode_unsigned` (no `#[ignore]`; dev build)

**Cross-platform inline tests** (no cfg gate) — keep in the same `mod tests { ... }` block; they parse JSON / call `serde_json::from_str` and behave identically on all platforms. Examples: PROF-01 `unsafe_macos_seatbelt_rules` deserialize round-trip, OAuth2 `validate_upstream_url` http-rejection.

---

### 14. Test fixture port surface (D-13 / D-14)

**Role:** test (port from upstream) · **Data flow:** n/a
**Analog:** Phase 20 commits with `Upstream-commit:` trailers — `8cb85037ac7b...` (manual port), `af5c1249c5...`, `f377a3eaf1...`, `ec73a8acfe...` (clean cherry-pick). All available via `git log --grep "Upstream-commit:"`.

**Manual-port commit body template** (from `8cb85037` body):
```
feat(20-03): port keyring:// URI scheme + ?decode=go-keyring from upstream v0.37.1 (D-08)

Manual port from upstream v0.37.1:crates/nono/src/keystore.rs (commits
5bccbc4 and 23e9a87) — cherry-pick was infeasible because fork's
keystore.rs had diverged from upstream: fork 2369 lines vs upstream
2901 lines (762-line delta) made line-level 3-way merge unreliable.
Manual semantic replay lands the keyring:// variant in fork's
credential-URI dispatch alongside the existing env:// / op:// /
apple-password:// / file:// schemes.

Symbols ported:
  - KEYRING_URI_PREFIX, KEYRING_URI_MAX_LEN (1024 bytes), ...
  - pub fn is_keyring_uri(&str) -> bool
  - pub fn validate_keyring_uri(&str) -> Result<()> — fail-closed parser
  ...

Upstream-commit: 5bccbc4...
Upstream-commit: 23e9a87...
Upstream-tag: v0.37.1
Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
```

**Apply to Phase 22 fixture ports:**
- D-13 (OAuth2 11 inline tests in `oauth2.rs`) — port AS-IS in same commit as production `9546c879`; document `#[allow(clippy::unwrap_used)]` add as drive-by Rule-3 deviation in commit body.
- D-14 (`audit_attestation.rs` test file from `9db06336` +188 LOC) — port AS-IS into `crates/nono-cli/tests/audit_attestation.rs`; verify `EnvVarGuard` wrap for `HOME` env-var manipulation per CLAUDE.md (RESEARCH Open Question #2).

**Cross-platform inline-test reuse** — `crates/nono-cli/tests/config_flag.rs:10–18`:
```rust
#[cfg(target_os = "windows")]
fn test_command() -> [&'static str; 4] {
    ["--", "cmd", "/c", "echo"]
}

#[cfg(not(target_os = "windows"))]
fn test_command() -> [&'static str; 2] {
    ["--", "echo"]
}

fn escaped_temp_dir() -> String {
    std::env::temp_dir()
        .display()
        .to_string()
        .replace('\\', "\\\\")
}
```

**Apply to all PKG/AUD integration tests** that need to invoke a child process — never assume POSIX shell.

---

## Shared Patterns

### Atomic commit + `Upstream-commit:` trailer (CLAUDE.md DCO + D-19)

**Source:** every Phase 20 commit (`af5c1249`, `ec73a8ac`, `f377a3ea`, `7a4b9fdc`, `e6fde898`, `8cb85037`, `f8ef9dd5`, `05c24a6b` — see `git log --grep "Upstream-commit:"`).
**Apply to:** every Phase 22 commit, both clean cherry-picks and manual ports.

```
feat(22-0X): <one-line change>

<2-3 line why-this-matters rationale>

Upstream-commit: <hash>
Upstream-tag: v0.40.1
Upstream-author: <name> <email>
Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
```

For fork-only commits (Authenticode addition; D-13/D-14 fixture ports if substantively diverged): omit the `Upstream-commit:` trailer; document "fork addition; no upstream parent" in the body. Phase 20 manual-port body in `8cb85037` is the canonical analog.

---

### `EnvVarGuard` save/restore (CLAUDE.md "Environment variables in tests")

**Source:** `crates/nono-cli/src/test_env.rs::EnvVarGuard` — used in `session_commands.rs:715`, `profile/mod.rs:2137+`, `profile/builtin.rs:389+`.
**Apply to:** every test that mutates `HOME`, `TMPDIR`, `XDG_CONFIG_HOME`, `LOCALAPPDATA`, `APPDATA`, `NONO_CAP_FILE`, `NONO_LOG`, `RUST_LOG`.

```rust
// session_commands.rs:715
let _guard = EnvVarGuard::set_all(&[("NONO_CAP_FILE", "/tmp/fake-cap-file")]);
// guard restores original env on Drop
```

Critical for: PROF-01..03 deserialize tests (touching `XDG_CONFIG_HOME`/`APPDATA`), PROF-03 `keyring://` resolution test (touching keyring service env), PKG-02 `LOCALAPPDATA` resolution test, AUD-01..04 `audit_attestation.rs` port (RESEARCH Open Question #2 explicit hazard), AUD-04 `auto_prune_is_noop_when_sandboxed` regression.

---

### `Zeroizing<String>` for secrets (CLAUDE.md § Memory zeroization)

**Source:** `crates/nono/src/keystore.rs` (every `load_*` returns `Zeroizing<String>`); `crates/nono-proxy/src/server.rs:164` (session token); `crates/nono-proxy/src/reverse.rs:30`.
**Apply to:** REQ-PROF-03 `client_secret` (acceptance #3), REQ-OAUTH-01 `access_token` token cache, REQ-AUD-02 `--audit-sign-key` private bytes.

```rust
use zeroize::Zeroizing;

// Hold the secret only as long as needed; auto-zeros on Drop.
let secret: Zeroizing<String> = nono::keystore::load_secret_by_ref("nono", &uri)?;
```

Never derive `Serialize` on a struct containing `Zeroizing<String>` — would defeat the on-disk-leak protection. Per RESEARCH § Anti-Patterns to Avoid, OAuth2 token cache must be in-memory-only (no `serde::Serialize` impl on `CachedToken`).

---

### Fail-closed error patterns (CLAUDE.md § Fail Secure)

**Source:** `crates/nono-cli/src/policy.rs:838–845` (override_deny rejection), `crates/nono-cli/src/profile/mod.rs:618–632` (HTTPS-only validation), `crates/nono/src/sandbox/windows.rs:548–552` (label-apply HRESULT propagation).
**Apply to:** every fault-handling site in Phase 22.

Three concrete error-construction shapes already in fork:

```rust
// (a) Profile parse / config error — most common
return Err(NonoError::ProfileParse(format!(
    "Upstream URL ... must use HTTPS, got scheme '{}': {}", scheme, url)));

// (b) SandboxInit (security-relevant rejection at policy resolution)
return Err(NonoError::SandboxInit(format!(
    "override_deny '{}' has no matching grant. \
     Add a filesystem allow ... for this path.",
    override_path.display())));

// (c) Structured Windows-FFI error with hresult + actionable hint
return Err(NonoError::LabelApplyFailed {
    path: path.to_path_buf(),
    hresult,
    hint: format!("Failed to construct mandatory-label SDDL (mask=0x{mask:X})"),
});
```

Reuse these shapes — do NOT introduce new variants for variants' sake. If POLY-01 / POLY-02 / OAUTH-01 errors fit one of (a)/(b)/(c), use it.

---

## No Analog Found

Files / surfaces with no close in-fork analog (planner uses RESEARCH.md § Code Examples Pattern 4 instead):

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/nono-cli/src/audit_authenticode.rs` (Authenticode FFI) | utility (Windows FFI) | request-response | Fork has no existing `WinVerifyTrust` / `WINTRUST_DATA` usage anywhere (`Grep("WinVerifyTrust")` returns 0 hits). Closest analog is `sandbox/windows.rs::try_set_mandatory_label` for the FFI *style* (encode_wide, unsafe-block + SAFETY comment, RAII guard, GetLastError → typed error) but the *call* itself is novel. Use RESEARCH § Code Examples Pattern 4 as the reference; cite Microsoft Learn `wintrust.h` API in the commit body. |

The `Win32_Security_WinTrust` `windows-sys` feature must be added to `crates/nono-cli/Cargo.toml` before this file compiles (RESEARCH § Environment Availability missing dependency).

---

## Contradictions Surfaced (fork vs upstream pattern divergences)

Three fork-vs-upstream pattern contradictions the planner must reconcile:

### CONTRADICTION-A: POLY-02 `--rollback` / `--no-audit` conflict already wired in fork

- **Upstream `5c301e8d`** introduces clap `conflicts_with` for `--rollback` + `--no-audit`.
- **Fork already has it** at `cli.rs:1602` (`#[arg(long, conflicts_with = "rollback", help_heading = "OPTIONS")]` on `pub no_audit: bool`).
- **Risk:** the upstream commit may add `conflicts_with_all` (broader) than fork's single-target `conflicts_with`, OR may touch `--rollback-prompt` which fork has at line 1559–1560 with no conflict.
- **Resolution:** plan executor diffs the *exact* upstream `cli.rs` change against fork's lines 1553–1604 before cherry-pick. If functionally identical, document "no-op cherry-pick — upstream parity already in place" in commit body.

### CONTRADICTION-B: POLY-01 `override_deny` is *stricter* in fork than upstream

- **Upstream** (`b83da813` "filter override_deny entries without grants" + `5c301e8d`): rejects when *any* grant is absent.
- **Fork** (`policy.rs:806–845`): rejects when no *user-intent* grant covers the path — group-sourced grants do NOT satisfy the check (test `test_apply_deny_overrides_rejects_group_sourced_grant` at policy.rs:2710).
- **Risk:** cherry-picking upstream's looser check would *regress* fork's stricter security posture.
- **Resolution:** keep fork's user-intent semantic as the outer gate; port upstream's filtering logic as a *secondary* check only if both apply additively. Document deviation in 22-02 plan SUMMARY.

### CONTRADICTION-C: AUD-03 Authenticode is fork-only, not in `02ee0bd1` / `7b7815f7`

This contradiction is also flagged in RESEARCH.md § Contradictions Found #2. Restated for pattern-mapping clarity:

- **CONTEXT.md** Plan 22-05 cherry-pick chain lists `02ee0bd1` + `7b7815f7` as the upstream parents for REQ-AUD-03 ("Executable identity recorded on Windows").
- **Upstream's `ExecutableIdentity` struct** is `{ resolved_path: PathBuf, sha256: ContentHash }` — SHA-256 only. `git grep WinVerifyTrust v0.40.1` returns 0 results.
- **REQ-AUD-03 acceptance #2/#3** require Authenticode signer-chain extraction AND unsigned/SHA-256 fallback recording — neither is in upstream.
- **Resolution:**
  1. Cherry-pick `02ee0bd1` + `7b7815f7` clean (sha256-only `ExecutableIdentity`).
  2. Land a fork-only commit `feat(22-05): add Authenticode signature query for exec-identity (Windows)` AFTER the chain — no `Upstream-commit:` trailer.
  3. Authenticode status lives as `audit_authenticode: Option<AuthenticodeStatus>` *sibling* on `SessionMetadata`, NOT a mutation of upstream's `ExecutableIdentity` (preserves D-19 atomic-commit-per-semantic-change discipline).
  4. Per D-17 exception, this is ALLOWED Windows-only fork addition. Use § 11 above for FFI pattern.

---

## Metadata

**Analog search scope:**
- `crates/nono/src/` (library — error.rs, keystore.rs, sandbox/windows.rs, undo/merkle.rs, trust/signing.rs)
- `crates/nono-cli/src/` (CLI — profile/, policy.rs, cli.rs, hooks.rs, session_commands*, config/, learn_windows.rs, pty_proxy_windows.rs)
- `crates/nono-cli/data/policy.json` (built-in profile data)
- `crates/nono-cli/tests/` (integration test analogs)
- `crates/nono-proxy/src/` (server.rs, reverse.rs, error.rs)
- Phase 20 commit history (Upstream-commit-trailered commits: `af5c1249`, `ec73a8ac`, `f377a3ea`, `7a4b9fdc`, `e6fde898`, `8cb85037`, `f8ef9dd5`, `05c24a6b`)

**Files scanned (read or grepped, with concrete excerpts extracted):**
- `crates/nono-cli/src/profile/mod.rs` (4943 lines — targeted reads at 595–633, 694–740, 1175–1280, 1404–1450, 1769–1817)
- `crates/nono-cli/src/profile/builtin.rs` (620 lines — read 1–100)
- `crates/nono-cli/src/policy.rs` (2818 lines — targeted reads at 774–845, 2689–2733)
- `crates/nono-cli/src/cli.rs` (3985 lines — targeted reads at 670–820, 1022–1041, 1553–1604, 2030–2096)
- `crates/nono-cli/src/session_commands.rs` (809 lines — read 1–130)
- `crates/nono-cli/src/hooks.rs` (582 lines — read 1–80)
- `crates/nono-cli/src/config/mod.rs` (read 95–115)
- `crates/nono-cli/data/policy.json` (951 lines — read 615–720)
- `crates/nono-cli/tests/config_flag.rs` (read 1–60)
- `crates/nono-cli/tests/wfp_port_integration.rs` (read 1–40)
- `crates/nono/src/error.rs` (245 lines — full read)
- `crates/nono/src/keystore.rs` (2883 lines — targeted read 196–216)
- `crates/nono/src/sandbox/windows.rs` (3275 lines — targeted reads at 1–80, 514–600, 1788–1830)
- `crates/nono/src/undo/merkle.rs` (235 lines — read 1–93)
- `crates/nono/src/trust/signing.rs` (targeted read 252–319)
- `crates/nono-proxy/src/server.rs` (919 lines — read 160–260)
- `crates/nono-proxy/src/reverse.rs` (1196 lines — read 1–100)

**Pattern extraction date:** 2026-04-27
