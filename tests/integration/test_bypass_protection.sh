#!/bin/bash
# Bypass Protection Tests (legacy name: Override Deny Tests)
#
# Plan 34-04b (upstream f0abd413, v0.47.0, #594): the canonical name is
# `bypass_protection` (JSON key) / `--bypass-protection` (CLI flag). The
# legacy `override_deny` / `--override-deny` continues to work via
# serde alias + clap visible_alias for v2.3 backwards-compat. This test
# file exercises BOTH paths to verify the rename-acceptance contract.
#
# Verifies that bypass_protection (canonical) AND override_deny (legacy)
# in profiles, and --bypass-protection (canonical) AND --override-deny
# (legacy) CLI flags, correctly punch through deny groups while requiring
# explicit grants.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/test_helpers.sh"

echo ""
echo -e "${BLUE}=== Bypass Protection Tests (legacy: Override Deny) ===${NC}"

verify_nono_binary
if ! require_working_sandbox "override deny suite"; then
    print_summary
    exit 0
fi

# Create test fixtures
TMPDIR=$(setup_test_dir)
trap 'cleanup_test_dir "$TMPDIR"' EXIT

PROFILES_DIR="$TMPDIR/profiles"
mkdir -p "$PROFILES_DIR"

# Create a directory that mimics a sensitive path for testing.
# We use ~/.docker which is in deny_credentials.
DOCKER_DIR="$HOME/.docker"

echo ""
echo "Test directory: $TMPDIR"
echo ""

# =============================================================================
# CLI --override-deny
# =============================================================================

echo "--- CLI --override-deny ---"

if [[ -d "$DOCKER_DIR" ]]; then
    # override-deny with matching grant should succeed
    expect_success "CLI --override-deny with --allow succeeds (dry-run)" \
        "$NONO_BIN" run --allow "$DOCKER_DIR" --override-deny "$DOCKER_DIR" --dry-run -- echo ok

    # override-deny without grant should fail
    expect_failure "CLI --override-deny without grant fails" \
        "$NONO_BIN" run --override-deny "$DOCKER_DIR" --dry-run -- echo ok

    # override-deny with read-only grant should succeed
    expect_success "CLI --override-deny with --read succeeds (dry-run)" \
        "$NONO_BIN" run --read "$DOCKER_DIR" --override-deny "$DOCKER_DIR" --dry-run -- echo ok
else
    skip_test "CLI --override-deny with --allow" "~/.docker not found"
    skip_test "CLI --override-deny without grant" "~/.docker not found"
    skip_test "CLI --override-deny with --read" "~/.docker not found"
fi

# =============================================================================
# Profile override_deny
# =============================================================================

echo ""
echo "--- Profile override_deny ---"

if [[ -d "$DOCKER_DIR" ]]; then
    # Profile with override_deny and matching filesystem grant
    cat > "$PROFILES_DIR/docker-override.json" <<EOF
{
    "meta": { "name": "docker-override", "version": "1.0.0" },
    "extends": "default",
    "filesystem": {
        "allow": ["\$HOME/.docker"]
    },
    "policy": {
        "override_deny": ["\$HOME/.docker"]
    }
}
EOF

    expect_success "profile override_deny with filesystem grant succeeds (dry-run)" \
        "$NONO_BIN" run --profile "$PROFILES_DIR/docker-override.json" --dry-run -- echo ok

    expect_output_contains "profile override_deny shows .docker in capabilities" ".docker" \
        "$NONO_BIN" run --profile "$PROFILES_DIR/docker-override.json" --dry-run -- echo ok

    # Profile with override_deny but NO filesystem grant
    cat > "$PROFILES_DIR/docker-no-grant.json" <<EOF
{
    "meta": { "name": "docker-no-grant", "version": "1.0.0" },
    "extends": "default",
    "policy": {
        "override_deny": ["\$HOME/.docker"]
    }
}
EOF

    expect_failure "profile override_deny without grant fails" \
        "$NONO_BIN" run --profile "$PROFILES_DIR/docker-no-grant.json" --dry-run -- echo ok

    expect_output_contains "profile override_deny without grant mentions missing grant" \
        "no matching grant" \
        "$NONO_BIN" run --profile "$PROFILES_DIR/docker-no-grant.json" --dry-run -- echo ok

    # Profile with read-only grant (least privilege)
    cat > "$PROFILES_DIR/docker-readonly.json" <<EOF
{
    "meta": { "name": "docker-readonly", "version": "1.0.0" },
    "extends": "default",
    "filesystem": {
        "read": ["\$HOME/.docker"]
    },
    "policy": {
        "override_deny": ["\$HOME/.docker"]
    }
}
EOF

    expect_success "profile override_deny with read-only grant succeeds (dry-run)" \
        "$NONO_BIN" run --profile "$PROFILES_DIR/docker-readonly.json" --dry-run -- echo ok
else
    skip_test "profile override_deny with filesystem grant" "~/.docker not found"
    skip_test "profile override_deny shows .docker in capabilities" "~/.docker not found"
    skip_test "profile override_deny without grant fails" "~/.docker not found"
    skip_test "profile override_deny without grant mentions missing grant" "~/.docker not found"
    skip_test "profile override_deny with read-only grant" "~/.docker not found"
fi

# =============================================================================
# nono why with override_deny
# =============================================================================

echo ""
echo "--- nono why with override_deny ---"

if [[ -d "$DOCKER_DIR" ]]; then
    # Without override, ~/.docker should be denied
    expect_output_contains "nono why reports .docker denied without override" \
        "sensitive_path" \
        "$NONO_BIN" --silent why --json --path "$DOCKER_DIR" --op read

    # With profile override_deny, ~/.docker should be allowed
    expect_output_contains "nono why reports .docker allowed with profile override" \
        "\"status\": \"allowed\"" \
        "$NONO_BIN" --silent why --json --profile "$PROFILES_DIR/docker-override.json" \
            --path "$DOCKER_DIR" --op read

    # With read-only profile, write should be denied
    expect_output_contains "nono why reports .docker write denied with read-only profile" \
        "insufficient_access" \
        "$NONO_BIN" --silent why --json --profile "$PROFILES_DIR/docker-readonly.json" \
            --path "$DOCKER_DIR" --op write
else
    skip_test "nono why reports .docker denied without override" "~/.docker not found"
    skip_test "nono why reports .docker allowed with profile override" "~/.docker not found"
    skip_test "nono why reports .docker write denied with read-only profile" "~/.docker not found"
fi

# =============================================================================
# Override deny with profile inheritance
# =============================================================================

echo ""
echo "--- Profile Inheritance ---"

if [[ -d "$DOCKER_DIR" ]]; then
    # Child profile inherits override_deny from parent via user profiles directory.
    # The extends field resolves by name from ~/.config/nono/profiles/.
    USER_PROFILES_DIR="$HOME/.config/nono/profiles"
    CREATED_USER_PROFILES=0
    if [[ ! -d "$USER_PROFILES_DIR" ]]; then
        mkdir -p "$USER_PROFILES_DIR"
        CREATED_USER_PROFILES=1
    fi

    cat > "$USER_PROFILES_DIR/nono-test-docker-base.json" <<EOF
{
    "meta": { "name": "nono-test-docker-base", "version": "1.0.0" },
    "extends": "default",
    "filesystem": {
        "allow": ["\$HOME/.docker"]
    },
    "policy": {
        "override_deny": ["\$HOME/.docker"]
    }
}
EOF

    cat > "$USER_PROFILES_DIR/nono-test-docker-child.json" <<EOF
{
    "meta": { "name": "nono-test-docker-child", "version": "1.0.0" },
    "extends": "nono-test-docker-base",
    "filesystem": {
        "read": ["\$HOME/.config"]
    }
}
EOF

    expect_success "child profile inherits override_deny from parent (dry-run)" \
        "$NONO_BIN" run --profile nono-test-docker-child --dry-run -- echo ok

    expect_output_contains "child profile shows .docker from inherited override" ".docker" \
        "$NONO_BIN" run --profile nono-test-docker-child --dry-run -- echo ok

    # Cleanup
    rm -f "$USER_PROFILES_DIR/nono-test-docker-base.json" \
          "$USER_PROFILES_DIR/nono-test-docker-child.json"
    if [[ "$CREATED_USER_PROFILES" -eq 1 ]]; then
        rmdir "$USER_PROFILES_DIR" 2>/dev/null || true
    fi
else
    skip_test "child profile inherits override_deny from parent" "~/.docker not found"
    skip_test "child profile shows .docker from inherited override" "~/.docker not found"
fi

# =============================================================================
# Override deny does NOT bypass other deny groups
# =============================================================================

echo ""
echo "--- Override scope is targeted ---"

if [[ -d "$DOCKER_DIR" ]] && [[ -d "$HOME/.ssh" ]]; then
    # Overriding .docker must NOT also unlock .ssh
    expect_output_contains "override_deny for .docker does not bypass .ssh deny" \
        "sensitive_path" \
        "$NONO_BIN" --silent why --json --profile "$PROFILES_DIR/docker-override.json" \
            --path "$HOME/.ssh" --op read
else
    skip_test "override_deny for .docker does not bypass .ssh deny" "~/.docker or ~/.ssh not found"
fi

# =============================================================================
# Warning output
# =============================================================================

echo ""
echo "--- Warning output ---"

if [[ -d "$DOCKER_DIR" ]]; then
    expect_output_contains "override_deny shows styled warning" \
        "warning:" \
        "$NONO_BIN" run --profile "$PROFILES_DIR/docker-override.json" --dry-run -- echo ok
else
    skip_test "override_deny shows styled warning" "~/.docker not found"
fi

# =============================================================================
# Required groups cannot be excluded
# =============================================================================

echo ""
echo "--- Required group protection ---"

cat > "$PROFILES_DIR/exclude-required.json" <<EOF
{
    "meta": { "name": "exclude-required", "version": "1.0.0" },
    "extends": "default",
    "policy": {
        "exclude_groups": ["deny_credentials"]
    }
}
EOF

expect_failure "excluding required deny_credentials group fails" \
    "$NONO_BIN" run --profile "$PROFILES_DIR/exclude-required.json" --dry-run -- echo ok

expect_output_contains "excluding required group mentions 'required'" \
    "required" \
    "$NONO_BIN" run --profile "$PROFILES_DIR/exclude-required.json" --dry-run -- echo ok

# =============================================================================
# Plan 34-04b canonical-name smoke tests (--bypass-protection + bypass_protection)
# =============================================================================
#
# Upstream f0abd413 renamed --override-deny -> --bypass-protection and the
# JSON key override_deny -> bypass_protection. The fork accepts both names
# (Option C: serde alias + clap visible_alias + one-time stderr deprecation
# warning when the legacy JSON key is observed). Verify both new names work
# at the CLI and JSON surfaces.

echo ""
echo "--- Plan 34-04b canonical-name smoke (--bypass-protection / bypass_protection) ---"

if [[ -d "$DOCKER_DIR" ]]; then
    # Canonical CLI flag --bypass-protection should behave identically to
    # legacy --override-deny.
    expect_success "CLI --bypass-protection with --allow succeeds (canonical name)" \
        "$NONO_BIN" run --allow "$DOCKER_DIR" --bypass-protection "$DOCKER_DIR" --dry-run -- echo ok

    expect_failure "CLI --bypass-protection without grant fails (canonical name)" \
        "$NONO_BIN" run --bypass-protection "$DOCKER_DIR" --dry-run -- echo ok
fi

# Canonical JSON key bypass_protection (in profile policy block) should
# deserialize identically to legacy override_deny via the serde alias on
# PolicyPatchConfig.
cat > "$PROFILES_DIR/canonical-bypass-protection.json" <<EOF
{
    "meta": { "name": "canonical-bypass-protection", "version": "1.0.0" },
    "extends": "default",
    "filesystem": {
        "allow_readwrite": ["$DOCKER_DIR"]
    },
    "policy": {
        "bypass_protection": ["$DOCKER_DIR"]
    }
}
EOF

if [[ -d "$DOCKER_DIR" ]]; then
    expect_success "Canonical JSON key bypass_protection deserializes via serde alias" \
        "$NONO_BIN" run --profile "$PROFILES_DIR/canonical-bypass-protection.json" --dry-run -- echo ok
fi

# Legacy JSON key override_deny should still load (backwards-compat) and
# emit a one-time stderr deprecation warning. We only smoke-test the load
# path here; the warning emission is unit-tested in
# crates/nono-cli/src/profile/mod.rs::canonical_schema_rename_tests.
cat > "$PROFILES_DIR/legacy-override-deny.json" <<EOF
{
    "meta": { "name": "legacy-override-deny", "version": "1.0.0" },
    "extends": "default",
    "filesystem": {
        "allow_readwrite": ["$DOCKER_DIR"]
    },
    "policy": {
        "override_deny": ["$DOCKER_DIR"]
    }
}
EOF

if [[ -d "$DOCKER_DIR" ]]; then
    expect_success "Legacy JSON key override_deny still loads (v2.3 backwards-compat)" \
        "$NONO_BIN" run --profile "$PROFILES_DIR/legacy-override-deny.json" --dry-run -- echo ok
fi

# =============================================================================
# Summary
# =============================================================================

print_summary
