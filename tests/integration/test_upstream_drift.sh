#!/bin/bash
# Upstream Drift Check Tests
# Verifies the bash + PowerShell twin scripts produce identical JSON for known
# ranges, tag auto-detection picks v0.X tags (not fork v2.X), missing upstream
# remote exits 1 with actionable hint, --format table emits category-grouped
# output, and the read-only invariant holds (git status clean post-invocation).
#
# This suite does NOT require the nono binary. It tests
# scripts/check-upstream-drift.{sh,ps1}. It is NOT in tests/run_integration_tests.sh
# SUITES array (which builds nono via cargo first). Run standalone:
#   bash tests/integration/test_upstream_drift.sh

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/../lib/test_helpers.sh"
# test_helpers.sh sets `set -euo pipefail`; restore the looser mode so failed
# assertions don't abort the whole suite (we want to keep running through all
# checks and report a final summary).
set +e

echo ""
echo -e "${BLUE}=== Upstream Drift Tests ===${NC}"

FIXTURES="$SCRIPT_DIR/fixtures/upstream-drift"
SCRIPT_SH="$REPO_ROOT/scripts/check-upstream-drift.sh"
SCRIPT_PS1="$REPO_ROOT/scripts/check-upstream-drift.ps1"

# Local pass/fail helpers (test_helpers.sh's expect_* helpers focus on the nono
# binary; the drift test does plain shell assertions and benefits from compact
# pass/fail rendering).
pass() {
    echo -e "  ${GREEN}PASS${NC}: $1"
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
}
fail() {
    echo -e "  ${RED}FAIL${NC}: $1"
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_FAILED=$((TESTS_FAILED + 1))
}

# ===========================================================================
# Test 1: Golden-fixture diff (bash) for 3 ranges
# ===========================================================================
echo ""
echo "Golden-fixture diff (bash):"
for range in "v0.37.1__v0.40.1" "v0.39.0__v0.40.0" "v0.40.0__v0.40.1"; do
    from="${range%__*}"
    to="${range#*__}"
    expected="$FIXTURES/${range}.json"
    if [[ ! -f "$expected" ]]; then
        fail "fixture missing: $expected"
        continue
    fi
    actual=$(bash "$SCRIPT_SH" --from "$from" --to "$to" --format json 2>/dev/null)
    if diff <(echo "$actual") "$expected" >/dev/null 2>&1; then
        pass "bash fixture diff: $range"
    else
        fail "bash fixture diff: $range (run: bash $SCRIPT_SH --from $from --to $to --format json | diff - $expected)"
    fi
done

# ===========================================================================
# Test 2: Twin-parity diff (skip if no PS interpreter)
# ===========================================================================
echo ""
echo "Twin-parity diff (bash vs PowerShell):"
PS_RUNNER=""
if command -v pwsh >/dev/null 2>&1; then
    PS_RUNNER="pwsh"
elif command -v powershell.exe >/dev/null 2>&1; then
    PS_RUNNER="powershell.exe"
fi
if [[ -n "$PS_RUNNER" ]]; then
    for range in "v0.37.1__v0.40.1" "v0.39.0__v0.40.0" "v0.40.0__v0.40.1"; do
        from="${range%__*}"
        to="${range#*__}"
        expected="$FIXTURES/${range}.json"
        actual_ps=$("$PS_RUNNER" -NoProfile -File "$SCRIPT_PS1" -From "$from" -To "$to" -Format json 2>/dev/null)
        if diff <(echo "$actual_ps") "$expected" >/dev/null 2>&1; then
            pass "ps fixture diff ($PS_RUNNER): $range"
        else
            fail "ps fixture diff ($PS_RUNNER): $range"
        fi
    done
else
    echo "  SKIP: no PowerShell runner on PATH (pwsh / powershell.exe both absent)"
    TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
fi

# ===========================================================================
# Test 3: Tag auto-detect picks v0.X (not fork v1.X / v2.X)
# ===========================================================================
echo ""
echo "Tag auto-detect:"
auto_output=$(bash "$SCRIPT_SH" --format json 2>/dev/null)
if [[ -n "$auto_output" ]] \
   && echo "$auto_output" \
       | python3 -c "import sys,json; d=json.loads(sys.stdin.read()); sys.exit(0 if d['from'].startswith('v0.') else 1)" 2>/dev/null; then
    pass "auto-detected from-tag matches ^v0\\. (not fork v1./v2.)"
else
    fail "auto-detect failed or returned a non-v0.X from-tag (output prefix: ${auto_output:0:200})"
fi

# ===========================================================================
# Test 4: Missing upstream remote -> exit 1 + hint
# ===========================================================================
echo ""
echo "Missing upstream remote:"
TMP_REPO=$(mktemp -d)
TMP_REPO_TRAP_SET=1
cleanup_tmp_repo() {
    if [[ "${TMP_REPO_TRAP_SET:-0}" -eq 1 && -n "${TMP_REPO:-}" && -d "$TMP_REPO" ]]; then
        rm -rf "$TMP_REPO"
    fi
}
trap cleanup_tmp_repo EXIT

(
    set +e
    cd "$TMP_REPO"
    git init -q
    git config user.email "test@example.invalid"
    git config user.name "Test"
    git commit --allow-empty -q -m init
    cp "$SCRIPT_SH" check-upstream-drift.sh
    chmod +x check-upstream-drift.sh
    bash check-upstream-drift.sh --from v0.40.0 --to v0.40.1 --format json 2>err.log
    rc=$?
    if [[ $rc -ne 0 ]] && grep -F 'git remote add upstream' err.log >/dev/null 2>&1; then
        echo "OK"
        exit 0
    else
        echo "BAD rc=$rc err=$(cat err.log 2>/dev/null | tr -d '\r' | head -c 200)"
        exit 1
    fi
)
remote_test_rc=$?
if [[ $remote_test_rc -eq 0 ]]; then
    pass "missing-upstream exits non-zero with actionable hint"
else
    fail "missing-upstream test failed"
fi

# ===========================================================================
# Test 5: --format table produces grouped output
# ===========================================================================
echo ""
echo "Table format:"
table_out=$(bash "$SCRIPT_SH" --from v0.37.1 --to v0.40.1 --format table 2>/dev/null)
if echo "$table_out" | head -1 | grep -qE '^Upstream drift: v0\.37\.1\.\.v0\.40\.1$'; then
    pass "table header line"
else
    fail "table header line missing (got: $(echo "$table_out" | head -1))"
fi
if echo "$table_out" | head -2 | tail -1 | grep -qE '^Total: [0-9]+ unique commits$'; then
    pass "table Total line"
else
    fail "table Total line missing"
fi
if echo "$table_out" | grep -qE '^## (profile|policy|package|proxy|audit|other) \([0-9]+ commits\)$'; then
    pass "table category section header present"
else
    fail "no table category section header found"
fi

# ===========================================================================
# Test 6: Read-only invariant (T-24-02)
# ===========================================================================
echo ""
echo "Read-only invariant:"
status_before=$(git -C "$REPO_ROOT" status --porcelain 2>/dev/null)
bash "$SCRIPT_SH" --from v0.40.0 --to v0.40.1 --format json > /dev/null 2>&1
status_after=$(git -C "$REPO_ROOT" status --porcelain 2>/dev/null)
if [[ "$status_before" == "$status_after" ]]; then
    pass "git status unchanged after script invocation"
else
    fail "git status diverged after invocation (script wrote something - security violation)"
fi

# ===========================================================================
# Test 7: Ref injection rejection (T-24-01)
# ===========================================================================
echo ""
echo "Ref injection rejection:"
if bash "$SCRIPT_SH" --from "v0.40.0; touch /tmp/nono-drift-pwn-$$" --to v0.40.1 >/dev/null 2>&1; then
    fail "injected ref accepted (security regression)"
else
    pass "injected ref rejected (T-24-01 mitigation)"
fi
# Defense-in-depth: even if injection somehow ran, the touched file should not
# exist; clean up just in case.
rm -f "/tmp/nono-drift-pwn-$$" 2>/dev/null || true

# ===========================================================================
# Summary
# ===========================================================================
echo ""
print_summary
