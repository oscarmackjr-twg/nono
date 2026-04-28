//! Plan 22-05b Task 3 — `nono prune` deprecation-alias regression.
//!
//! Verifies AUD-04 acceptance #3:
//! - `nono prune --help` output contains the word "deprecated" (case-insensitive)
//! - `nono prune` invocation emits a stderr deprecation note
//!
//! The alias delegates to the unchanged `session_commands::run_prune` worker
//! per Decision 2 LOCKED reframe, so the v2.1 Phase 19 CLEAN-04 invariants
//! (auto_prune_is_noop_when_sandboxed; NONO_CAP_FILE early-return as the
//! first statement of `auto_prune_if_needed`) are preserved trivially.

#[test]
fn prune_alias_surfaces_deprecation_note_in_help() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_nono"))
        .args(["prune", "--help"])
        .output()
        .expect("nono binary should be present");
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.to_lowercase().contains("deprecat"),
        "expected deprecation note in `nono prune --help`, got:\n{combined}"
    );
}

#[test]
fn prune_alias_emits_stderr_deprecation_note_on_invocation() {
    // `--dry-run` is a safe, fast invocation that exercises the dispatcher
    // path without touching real session state.
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_nono"))
        .args(["prune", "--dry-run"])
        .output()
        .expect("nono binary should be present");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("deprecat"),
        "expected stderr deprecation note from `nono prune --dry-run`, got stderr:\n{stderr}"
    );
}

#[test]
fn prune_alias_is_hidden_from_top_level_help() {
    // Plan 22-05b Task 3: `#[command(hide = true)]` removes `prune` from the
    // top-level `nono --help` listing while still allowing `nono prune` and
    // `nono prune --help` to function (verified by the two tests above).
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_nono"))
        .args(["--help"])
        .output()
        .expect("nono binary should be present");
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // `session` MUST appear in the top-level help (the renamed entry point)
    assert!(
        combined.contains("session"),
        "expected `session` subcommand in `nono --help`, got:\n{combined}"
    );

    // `prune` MUST NOT appear in the top-level help description block.
    // The `#[command(hide = true)]` clap attribute hides the `Prune`
    // variant from the rendered subcommand listing. (Substring match is
    // tolerant — the legacy ROOT_HELP_TEMPLATE may still mention `prune`
    // in non-listing examples; we only assert it is absent from the
    // commands listing portion. Use a coarse "Clean up old session files"
    // substring — the description text only renders when the variant is
    // visible.)
    let lc = combined.to_lowercase();
    let prune_listed = lc.contains("clean up old session files");
    assert!(
        !prune_listed,
        "expected hidden `prune` description absent from `nono --help`, got:\n{combined}"
    );
}
