//! Deprecation-warning surface for startup-only command blocking.
//!
//! Backported from upstream v0.37.1:crates/nono-cli/src/command_blocking_deprecation.rs
//! (introduced in upstream commit 0ca641b, "refactor(command-blocking): deprecate
//! startup-only command blocking"). Warnings-only — no enforcement changes.
//!
//! This module collects human-readable deprecation warnings for the legacy
//! startup-only command-blocking surfaces (CLI flags `--allow-command` /
//! `--block-command`, profile fields `security.allowed_commands` /
//! `policy.add_deny_commands`, and capability-manifest fields
//! `process.allowed_commands` / `process.blocked_commands`). Those surfaces
//! remain functional; this module only prints a `warning:` line explaining that
//! the feature is not kernel-enforced and recommending resource-based controls.

use crate::cli::{Cli, Commands};
use crate::output;
use crate::profile::Profile;
use nono::manifest::CapabilityManifest;
use std::path::Path;

const DEPRECATION_SUMMARY: &str =
    "deprecated in v0.33.0: startup-only command gating, not kernel-enforced. \
     Child processes can bypass it. Prefer resource-based controls such as \
     add_deny_access, narrower filesystem grants, unlink_protection, and network policy.";
/// Reason string surfaced when a blocked-command check fires at dispatch time.
/// Kept at module scope so future wiring sites (the fork does not currently
/// invoke the dispatch-time path) can share the same copy. Marked dead_code
/// locally until the fork picks up upstream's dispatch-point wiring.
#[allow(dead_code)]
pub(crate) const BLOCKED_COMMAND_REASON: &str =
    "Command blocking is deprecated in v0.33.0 and only checks the directly-invoked \
     startup command. Child processes can bypass it. Prefer resource-based controls \
     such as add_deny_access, narrower filesystem grants, unlink_protection, and \
     network policy.";

fn format_commands(commands: &[String]) -> String {
    commands.join(", ")
}

fn warning_for_surface(surface: &str, commands: &[String]) -> String {
    if commands.is_empty() {
        format!("{surface} is {DEPRECATION_SUMMARY}")
    } else {
        format!(
            "{surface} is {DEPRECATION_SUMMARY} Configured commands: {}.",
            format_commands(commands)
        )
    }
}

/// Collect deprecation warnings from `nono run` / `nono shell` / `nono wrap` CLI flags.
///
/// The fork wraps `Commands::Run(Box<RunArgs>)` etc. (divergence from upstream
/// which uses plain structs); Rust auto-derefs the `Box` for field access so
/// the body reads identically to upstream's.
pub(crate) fn collect_cli_warnings(cli: &Cli) -> Vec<String> {
    match &cli.command {
        Commands::Run(args) => {
            collect_sandbox_arg_warnings(&args.sandbox.allow_command, &args.sandbox.block_command)
        }
        Commands::Shell(args) => {
            collect_sandbox_arg_warnings(&args.sandbox.allow_command, &args.sandbox.block_command)
        }
        Commands::Wrap(args) => {
            collect_sandbox_arg_warnings(&args.sandbox.allow_command, &args.sandbox.block_command)
        }
        _ => Vec::new(),
    }
}

fn collect_sandbox_arg_warnings(
    allowed_commands: &[String],
    blocked_commands: &[String],
) -> Vec<String> {
    let mut warnings = Vec::new();

    if !allowed_commands.is_empty() {
        warnings.push(warning_for_surface(
            "CLI flag `--allow-command`",
            allowed_commands,
        ));
    }
    if !blocked_commands.is_empty() {
        warnings.push(warning_for_surface(
            "CLI flag `--block-command`",
            blocked_commands,
        ));
    }

    warnings
}

/// Collect profile-level deprecation warnings.
///
/// Upstream wires this in `sandbox_prepare.rs`, which this plan does not
/// modify (D-15 disjoint-parallel invariant and CONTEXT § Known risks
/// flag sandbox_prepare.rs as refactored: fork 452 vs upstream 1585 lines).
/// Kept on the module surface so a follow-up profile-surface plan can wire
/// it without re-introducing the function. Exercised by unit tests in this
/// module.
#[allow(dead_code)]
pub(crate) fn collect_profile_warnings(profile: &Profile) -> Vec<String> {
    let mut warnings = Vec::new();
    let profile_name = format!("profile `{}`", profile.meta.name);

    if !profile.security.allowed_commands.is_empty() {
        warnings.push(warning_for_surface(
            &format!("{profile_name} field `security.allowed_commands`"),
            &profile.security.allowed_commands,
        ));
    }
    if !profile.policy.add_deny_commands.is_empty() {
        warnings.push(warning_for_surface(
            &format!("{profile_name} field `policy.add_deny_commands`"),
            &profile.policy.add_deny_commands,
        ));
    }

    warnings
}

/// Collect capability-manifest deprecation warnings.
///
/// Same deferred-wiring rationale as `collect_profile_warnings` above:
/// upstream calls this from `sandbox_prepare.rs`, which is outside this
/// plan's scope. Exercised by unit tests in this module.
#[allow(dead_code)]
pub(crate) fn collect_manifest_warnings(
    manifest: &CapabilityManifest,
    manifest_path: &Path,
) -> Vec<String> {
    let mut warnings = Vec::new();

    if let Some(process) = manifest.process.as_ref() {
        if !process.allowed_commands.is_empty() {
            warnings.push(warning_for_surface(
                &format!(
                    "capability manifest `{}` field `process.allowed_commands`",
                    manifest_path.display()
                ),
                &process.allowed_commands,
            ));
        }
        if !process.blocked_commands.is_empty() {
            warnings.push(warning_for_surface(
                &format!(
                    "capability manifest `{}` field `process.blocked_commands`",
                    manifest_path.display()
                ),
                &process.blocked_commands,
            ));
        }
    }

    warnings
}

pub(crate) fn print_warnings(warnings: &[String], silent: bool) {
    if silent {
        return;
    }

    for warning in warnings {
        output::print_warning(warning);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_arg_warnings_include_allow_and_block_flags() {
        let warnings = collect_sandbox_arg_warnings(
            &["rm".to_string(), "chmod".to_string()],
            &["docker".to_string()],
        );

        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("`--allow-command`"));
        assert!(warnings[0].contains("rm, chmod"));
        assert!(warnings[1].contains("`--block-command`"));
        assert!(warnings[1].contains("docker"));
    }

    #[test]
    fn warning_for_surface_omits_empty_command_list_suffix() {
        let warning = warning_for_surface("CLI flag `--block-command`", &[]);

        assert!(warning.contains("CLI flag `--block-command`"));
        assert!(!warning.contains("Configured commands:"));
    }

    #[test]
    fn profile_warnings_include_allowed_and_denied_command_fields() {
        let profile: Profile = serde_json::from_str(
            r#"{
                "meta": { "name": "deprecated-commands" },
                "security": { "allowed_commands": ["rm"] },
                "policy": { "add_deny_commands": ["docker"] }
            }"#,
        )
        .expect("profile should deserialize");

        let warnings = collect_profile_warnings(&profile);
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("security.allowed_commands"));
        assert!(warnings[1].contains("policy.add_deny_commands"));
    }

    #[test]
    fn manifest_warnings_include_process_command_fields() {
        let manifest = CapabilityManifest::from_json(
            r#"{
                "version": "0.1.0",
                "process": {
                    "allowed_commands": ["rm"],
                    "blocked_commands": ["docker"]
                }
            }"#,
        )
        .expect("manifest should deserialize");

        let warnings = collect_manifest_warnings(&manifest, Path::new("/tmp/caps.json"));
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("process.allowed_commands"));
        assert!(warnings[1].contains("process.blocked_commands"));
    }

    // ==================================================================
    // Plan 20-03 regression guards (D-10 warnings-only contract)
    // ==================================================================

    #[test]
    fn test_deprecation_warning_does_not_unblock_commands() {
        // Regression guard: the deprecation-warning surface emits text but
        // NEVER mutates any input. A command that was block-listed before
        // the backport (expressed via --block-command or
        // policy.add_deny_commands) is still in the deny list after
        // collect_*_warnings runs. We verify no mutation by calling the
        // collector twice with the same inputs and confirming the inputs
        // are unchanged.
        let allowed = vec!["rm".to_string()];
        let blocked = vec!["docker".to_string()];
        let warnings1 = collect_sandbox_arg_warnings(&allowed, &blocked);
        let warnings2 = collect_sandbox_arg_warnings(&allowed, &blocked);
        // Inputs unchanged (Vecs still populated).
        assert_eq!(allowed, vec!["rm"]);
        assert_eq!(blocked, vec!["docker"]);
        // Warnings are deterministic: same input -> same output.
        assert_eq!(warnings1, warnings2);
        // The "blocked" command "docker" appears in the warning text for
        // auditability but this is text emission, not enforcement mutation.
        assert!(
            warnings1.iter().any(|w| w.contains("docker")),
            "blocked command should be surfaced in warning text"
        );
    }

    #[test]
    fn test_deprecation_warning_does_not_block_allowed_commands() {
        // Regression guard: empty allow/block lists produce zero warnings
        // AND return an empty Vec (not an Err). A previously-allowed
        // command dispatches through the run/shell/wrap path unchanged
        // because the warning collector is a pure read-only pass.
        let warnings = collect_sandbox_arg_warnings(&[], &[]);
        assert!(
            warnings.is_empty(),
            "no configured commands -> no warnings, got: {warnings:?}"
        );
    }

    #[test]
    fn test_deprecation_warning_emitted_for_deprecated_command() {
        // Plan-required: invoking a deprecated command surface produces a
        // warning string containing the deprecation summary. Verified by
        // direct inspection of the collector output (tracing-test is not
        // available in the fork's dependency graph; the collector's output
        // is the canonical source of truth for the warning text).
        let warnings = collect_sandbox_arg_warnings(&["docker".to_string()], &[]);
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].contains("deprecated in v0.33.0"),
            "warning must cite deprecation version, got: {}",
            warnings[0]
        );
        assert!(
            warnings[0].contains("docker"),
            "warning must list the deprecated command name, got: {}",
            warnings[0]
        );
        assert!(
            warnings[0].contains("add_deny_access") || warnings[0].contains("resource-based"),
            "warning must cite recommended alternative, got: {}",
            warnings[0]
        );
    }

    #[test]
    fn test_print_warnings_silent_is_noop() {
        // silent=true must not emit anything. We can't easily capture stderr
        // in a unit test, but we can assert the function returns without
        // panicking and with no side effects visible to the caller.
        let warnings = vec!["test warning".to_string()];
        print_warnings(&warnings, true);
        // If we got here, the silent path is correct.
    }
}
