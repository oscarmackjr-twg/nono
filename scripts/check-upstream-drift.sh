#!/usr/bin/env bash
# scripts/check-upstream-drift.sh
# Reports upstream commits the fork has not absorbed, grouped by file category.
# Read-only - does NOT modify git state.
#
# Usage:
#   ./scripts/check-upstream-drift.sh                              # auto-detect last-synced..latest-upstream
#   ./scripts/check-upstream-drift.sh --from v0.37.1 --to v0.40.1
#   ./scripts/check-upstream-drift.sh --format json                # JSON output (default: table)
#
# Path filter (D-11): Reports commits touching cross-platform Rust code under
# crates/{nono,nono-cli,nono-proxy}/src/ + crates/nono/Cargo.toml. Excludes
# *_windows.rs and crates/nono-cli/src/exec_strategy_windows/. Dep bumps in
# Cargo.lock and other crate Cargo.toml files are NOT reported. The 22-commit
# informational delta vs the 260424-upr SUMMARY headline is documented in
# tests/integration/fixtures/upstream-drift/README.md.

set -euo pipefail

# Best-effort UTF-8 locale to harden non-ASCII commit subject handling on
# Git-for-Windows MSYS bash. The "|| true" swallows the error on systems
# lacking C.UTF-8 (rare; MSYS2 supports it).
export LC_ALL=C.UTF-8 2>/dev/null || true

# ---------------------------------------------------------------------------
# CLI parsing
# ---------------------------------------------------------------------------

print_usage() {
    cat <<'USAGE'
Usage: scripts/check-upstream-drift.sh [--from <ref>] [--to <ref>] [--format table|json]

Reports upstream commits the fork has not absorbed, grouped by file category.
Read-only over .git; does NOT modify git state.

Options:
  --from <ref>      Start of commit range (default: highest v0.* tag merged into HEAD)
  --to <ref>        End of commit range (default: git describe --tags --abbrev=0 upstream/main)
  --format table    Human-readable grouped output (default)
  --format json     Single-line JSON for templates / CI consumers
  -h, --help        Show this message

Path filter (D-11): cross-platform Rust code under crates/{nono,nono-cli,nono-proxy}/src/
plus crates/nono/Cargo.toml. Excludes *_windows.rs and crates/nono-cli/src/exec_strategy_windows/.
Dep bumps in Cargo.lock and other crate Cargo.toml files are NOT reported.
USAGE
}

FROM_REF=""
TO_REF=""
FORMAT="table"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --from)   FROM_REF="$2"; shift 2 ;;
        --to)     TO_REF="$2"; shift 2 ;;
        --format) FORMAT="$2"; shift 2 ;;
        -h|--help) print_usage; exit 0 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

# Format validation (D-04)
case "$FORMAT" in
    table|json) ;;
    *) echo "Error: --format must be 'table' or 'json' (got: $FORMAT)" >&2; exit 2 ;;
esac

# Ref input validation (T-24-01 - V5 BLOCKING-eligible).
# Must match BEFORE any git invocation. The "--" separator on git invocations
# below provides defense in depth against any ref-vs-pathspec ambiguity.
validate_ref() {
    local r="$1"
    if [[ -n "$r" && ! "$r" =~ ^[A-Za-z0-9._/-]+$ ]]; then
        echo "Error: invalid ref '$r' (must match [A-Za-z0-9._/-]+)" >&2
        exit 2
    fi
}
validate_ref "$FROM_REF"
validate_ref "$TO_REF"

# ---------------------------------------------------------------------------
# Tag auto-detection (D-08, D-10; T-24-02 read-only invariant)
# ---------------------------------------------------------------------------
# Verify upstream remote exists before touching tags. Fail-closed per D-10:
# never auto-add the remote, never silently degrade.
if ! git remote get-url upstream >/dev/null 2>&1; then
    cat >&2 <<'EOF'
Error: 'upstream' remote not configured.
Add it with:
  git remote add upstream https://github.com/always-further/nono.git
EOF
    exit 1
fi

if [[ -z "$FROM_REF" ]]; then
    FROM_REF=$(git tag --list 'v0.*' --merged HEAD --sort=-v:refname | head -n1)
    if [[ -z "$FROM_REF" ]]; then
        echo "Error: no upstream-style tag (v0.*) reachable from HEAD; cannot auto-detect last-synced point. Use --from <ref>." >&2
        exit 1
    fi
fi

if [[ -z "$TO_REF" ]]; then
    # 2>/dev/null permitted ONLY here because we explicitly catch the failure
    # and substitute a clearer error message below.
    if ! TO_REF=$(git describe --tags --abbrev=0 upstream/main 2>/dev/null); then
        echo "Error: cannot resolve latest upstream tag (git describe --tags --abbrev=0 upstream/main failed). Is upstream/main fetched? Use --to <ref>." >&2
        exit 1
    fi
fi

# ---------------------------------------------------------------------------
# Path filter (D-11)
# ---------------------------------------------------------------------------
GITLOG_PATHS=(
    'crates/nono/src/'
    'crates/nono-cli/src/'
    'crates/nono-proxy/src/'
    'crates/nono/Cargo.toml'
    ':(exclude)*_windows.rs'
    ':(exclude)crates/nono-cli/src/exec_strategy_windows/'
)

# ---------------------------------------------------------------------------
# Categorization lookup table (D-05; ORDER IS LOAD-BEARING)
# ---------------------------------------------------------------------------
# First-match-wins. Audit must match before any generic crates/nono/src/*
# fallback (no such fallback exists today, but the comment locks the invariant
# for future maintainers). No subject-line keyword scanning per D-05; path
# prefix is the ONLY signal.
categorize_file() {
    local f="$1"
    case "$f" in
        crates/nono-cli/src/profile/*|crates/nono-cli/src/profile.rs|crates/nono-cli/data/profile-authoring-guide.md)
            echo profile ;;
        crates/nono-cli/src/policy.rs|crates/nono-cli/data/policy.json)
            echo policy ;;
        crates/nono-cli/src/package*|crates/nono-cli/src/package_cmd.rs|crates/nono/src/package*)
            echo package ;;
        crates/nono-proxy/*)
            echo proxy ;;
        crates/nono/src/audit/*|crates/nono/src/audit_attestation*|crates/nono-cli/src/audit*)
            echo audit ;;
        *)
            echo other ;;
    esac
}

# ---------------------------------------------------------------------------
# JSON escape helper (T-24-04)
# ---------------------------------------------------------------------------
# Order matters: backslash MUST be substituted FIRST. Scope: covers the
# printable-ASCII + tab/newline subset that real git-formatted text emits.
# Does NOT escape control chars 0x00-0x1F or non-BMP unicode to \uXXXX.
emit_json_string() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    s="${s//$'\n'/\\n}"
    s="${s//$'\r'/\\r}"
    s="${s//$'\t'/\\t}"
    printf '"%s"' "$s"
}

# ---------------------------------------------------------------------------
# Drive git log; consume per-commit blocks; build per-commit JSON.
# Use process substitution (Pitfall 8) so accumulator state survives the loop.
# ---------------------------------------------------------------------------
declare -a COMMITS_JSON=()
declare -a COMMITS_SHAS=()
declare -a COMMITS_SUBJECTS=()
declare -a COMMITS_CATS=()
declare -A BY_CAT=([profile]=0 [policy]=0 [package]=0 [proxy]=0 [audit]=0 [other]=0)

SHA=""; AUTH=""; DATE=""; SUBJ=""
ADDS=0; DELS=0
declare -a FILES=()

finalize_commit() {
    local sha="$1" auth="$2" date="$3" subj="$4"
    local adds="$5" dels="$6"

    # Compute categories for this commit (deduplicated, fixed-order iteration
    # over the 6 known categories so JSON output is deterministic across the
    # twin). NO sort -u: that introduces a subshell; explicit fixed-order
    # iteration is cheaper AND keeps emission deterministic.
    local f c
    local -A CAT_SET=()
    for f in "${FILES[@]}"; do
        c=$(categorize_file "$f")
        CAT_SET[$c]=1
    done
    local -a CATS=()
    for c in audit other package policy profile proxy; do
        [[ -n "${CAT_SET[$c]:-}" ]] && CATS+=("$c")
    done

    # Update by_category aggregate. Multi-category commits double-count by
    # design (D-06; the total_unique_commits header line disambiguates).
    for c in "${CATS[@]}"; do
        BY_CAT[$c]=$((BY_CAT[$c] + 1))
    done

    # Build files_changed JSON array
    local files_json="["
    local i=0
    for f in "${FILES[@]}"; do
        [[ $i -gt 0 ]] && files_json+=','
        files_json+=$(emit_json_string "$f")
        i=$((i + 1))
    done
    files_json+="]"

    # Build categories JSON array (sorted lex per the audit-first iteration
    # order above).
    local cats_json="["
    i=0
    for c in "${CATS[@]}"; do
        [[ $i -gt 0 ]] && cats_json+=','
        cats_json+=$(emit_json_string "$c")
        i=$((i + 1))
    done
    cats_json+="]"

    # Build per-commit object. Field order is locked: sha, subject, author,
    # date, additions, deletions, files_changed, categories.
    local obj
    obj=$(printf '{"sha":%s,"subject":%s,"author":%s,"date":%s,"additions":%d,"deletions":%d,"files_changed":%s,"categories":%s}' \
        "$(emit_json_string "$sha")" \
        "$(emit_json_string "$subj")" \
        "$(emit_json_string "$auth")" \
        "$(emit_json_string "$date")" \
        "$adds" "$dels" \
        "$files_json" \
        "$cats_json")

    COMMITS_JSON+=("$obj")
    COMMITS_SHAS+=("$sha")
    COMMITS_SUBJECTS+=("$subj")
    COMMITS_CATS+=("${CATS[*]}")
}

while IFS=$'\t' read -r c1 c2 c3 c4 c5; do
    if [[ "$c1" == "COMMIT" ]]; then
        if [[ -n "$SHA" ]]; then
            finalize_commit "$SHA" "$AUTH" "$DATE" "$SUBJ" "$ADDS" "$DELS"
        fi
        SHA="$c2"; AUTH="$c3"; DATE="$c4"; SUBJ="$c5"
        ADDS=0; DELS=0; FILES=()
    elif [[ -z "$c1" ]]; then
        : # blank line between commits
    else
        # numstat row: c1=adds, c2=dels, c3=filename (or "old => new")
        if [[ "$c1" != "-" ]]; then
            ADDS=$((ADDS + c1))
        fi
        if [[ "$c2" != "-" ]]; then
            DELS=$((DELS + c2))
        fi
        local_f="$c3"
        if [[ "$local_f" == *' => '* ]]; then
            local_f="${local_f##* => }"
        fi
        FILES+=("$local_f")
    fi
done < <(git log --no-merges --numstat \
    --format='COMMIT%x09%H%x09%an%x09%aI%x09%s' \
    "${FROM_REF}..${TO_REF}" -- "${GITLOG_PATHS[@]}")

# Finalize the trailing commit if any
if [[ -n "$SHA" ]]; then
    finalize_commit "$SHA" "$AUTH" "$DATE" "$SUBJ" "$ADDS" "$DELS"
fi

TOTAL=${#COMMITS_JSON[@]}

# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------

emit_json() {
    # Outer key order locked: range, from, to, total_unique_commits,
    # by_category, commits. by_category key order is the SUMMARY.md narrative
    # order: profile, policy, package, proxy, audit, other.
    printf '{"range":"%s..%s","from":"%s","to":"%s","total_unique_commits":%d,"by_category":{"profile":%d,"policy":%d,"package":%d,"proxy":%d,"audit":%d,"other":%d},"commits":[' \
        "$FROM_REF" "$TO_REF" "$FROM_REF" "$TO_REF" "$TOTAL" \
        "${BY_CAT[profile]}" "${BY_CAT[policy]}" "${BY_CAT[package]}" \
        "${BY_CAT[proxy]}" "${BY_CAT[audit]}" "${BY_CAT[other]}"
    local i
    for ((i=0; i<TOTAL; i++)); do
        [[ $i -gt 0 ]] && printf ','
        printf '%s' "${COMMITS_JSON[$i]}"
    done
    printf ']}\n'
}

emit_table() {
    # Header + per-category grouped output (D-06). The SAME commit appears
    # under EACH matching category, so sum-of-rows can exceed
    # total_unique_commits when commits span multiple categories.
    printf 'Upstream drift: %s..%s\n' "$FROM_REF" "$TO_REF"
    printf 'Total: %d unique commits\n' "$TOTAL"
    local cat count i
    # Fixed category section order matches SUMMARY.md narrative order.
    for cat in profile policy package proxy audit other; do
        count="${BY_CAT[$cat]:-0}"
        [[ $count -eq 0 ]] && continue
        printf '\n## %s (%d commits)\n' "$cat" "$count"
        for ((i=0; i<TOTAL; i++)); do
            if [[ " ${COMMITS_CATS[$i]} " == *" $cat "* ]]; then
                printf '  %.8s  %s\n' "${COMMITS_SHAS[$i]}" "${COMMITS_SUBJECTS[$i]}"
            fi
        done
    done
}

case "$FORMAT" in
    json) emit_json ;;
    table) emit_table ;;
esac
