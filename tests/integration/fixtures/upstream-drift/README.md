# Upstream-drift fixtures

Frozen JSON outputs of `scripts/check-upstream-drift.sh --format json` for known
commit ranges. These fixtures are the regression baseline for the bash +
PowerShell twin scripts; the integration test
(`tests/integration/test_upstream_drift.sh`) diffs both runtimes' output against
the same fixture file, byte-for-byte.

## Files

- `v0.37.1__v0.40.1.json` — large-range fixture; reproduces the 260424-upr
  SUMMARY's per-category inventory (profile / policy / package / proxy / audit /
  other). Acceptance #1 ground-truth for REQ-DRIFT-01.
- `v0.39.0__v0.40.0.json` — mid-range fixture covering the audit-integrity
  cluster.
- `v0.40.0__v0.40.1.json` — small-range fixture (2 filtered commits); regression
  guard for PowerShell 5.1's single-element-array unwrap behavior in
  `ConvertTo-Json`.

## Regeneration

Generated 2026-04-27 from `git rev-parse HEAD` = `0834aa66`. Regenerate with:

```bash
bash scripts/check-upstream-drift.sh --from v0.37.1 --to v0.40.1 --format json \
  > tests/integration/fixtures/upstream-drift/v0.37.1__v0.40.1.json
bash scripts/check-upstream-drift.sh --from v0.39.0 --to v0.40.0 --format json \
  > tests/integration/fixtures/upstream-drift/v0.39.0__v0.40.0.json
bash scripts/check-upstream-drift.sh --from v0.40.0 --to v0.40.1 --format json \
  > tests/integration/fixtures/upstream-drift/v0.40.0__v0.40.1.json
```

When upstream tags change or the script's lookup table is updated, regenerate,
run the integration test, and commit both the fixture changes and any related
script changes in a single commit.

## Test invocation

```bash
bash tests/integration/test_upstream_drift.sh
```

This test does NOT require the `nono` binary and is NOT included in
`tests/run_integration_tests.sh`'s `SUITES` array (which builds `nono` via cargo
first). Run it standalone.

## Informational delta vs SUMMARY.md headline

The 260424-upr SUMMARY.md headline reads "78 non-merge commits, ~9k insertions".
The script's canonical D-11 path filter
(`crates/{nono,nono-cli,nono-proxy}/src/` + `crates/nono/Cargo.toml`, excluding
`*_windows.rs` and `crates/nono-cli/src/exec_strategy_windows/`) produces 56
commits for the same `v0.37.1..v0.40.1` range. The 22-commit difference is
documented here so future readers don't read the discrepancy as a bug:

| Excluded by D-11 filter | Why | Listed in SUMMARY for |
|-------------------------|-----|-----------------------|
| docs-only commits | not cross-platform Rust code; live under `docs/` | narrative completeness |
| `Cargo.lock` dep bumps | not in path filter; lockfile churn isn't policy-meaningful drift | dep-bump audit trail |
| Crate `Cargo.toml` (other than `crates/nono/Cargo.toml`) | filter intentionally narrow per D-11 | release-version bumps |
| `.github/workflows/*.yml` | not cross-platform Rust code | CI history |
| `tests/integration/*.sh` | integration-test churn, not source drift | test-evolution record |
| claude-code package removal | upstream removed the integration package; not a feature-drift signal | removal record |

The script's category breakdown for a given range should agree with the
SUMMARY's per-release narrative blocks; the headline-count delta is
informational only. Open Question 1 in
`.planning/phases/24-parity-drift-prevention/24-RESEARCH.md` is the long-form
rationale.

## Cross-references

- Source script: `scripts/check-upstream-drift.sh` (and `.ps1` twin)
- Long-form runbook: `docs/cli/development/upstream-drift.mdx` (created in Plan
  24-02)
- Originating quick task:
  `.planning/quick/260424-upr-review-upstream-037-to-040/SUMMARY.md`
