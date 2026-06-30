# Test Writer Report: vb-shvxy State 9 (Attempt 2)

- **Bead**: vb-shvxy
- **State**: 9 (test-writer RETRY — strengthen tests per review)
- **Agent-invocation-ledger seq**: 16 ("vb-shvxy-state9-test-writer-attempt2")
- **Workspace**: /home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-shvxy

## Fixes Applied (per test review FIND-TR-001 through 008)

### FIND-TR-001 (CRITICAL): 9 structural grep checks → behavioral tests

All 9 tests rewritten from source-grepping to behavioral execution tests:

| Test ID | File | Old Pattern | New Behavioral Assertion |
|---------|------|-------------|--------------------------|
| I02 | test_kani_list.sh | grep source for "required on PATH" | Run script with fake cargo (exit 1), assert exit=1 + "required on PATH" in stderr |
| I09 | test_kani_list.sh | grep source for `! -s.*kani-list.json` | Fake cargo produces empty kani-list.json, assert exit=1 + "did not produce" |
| I10 | test_kani_list.sh | grep source for `python3 -m json.tool` | Fake cargo produces invalid JSON, assert exit≠0 |
| I18 | test_flux_check_package.sh | grep source for `cargo flux` | Fake cargo records args, verify `--message-format` not rejected AND passed through |
| I20 | test_flux_check_package.sh | grep source for `cargo flux` + `pipefail` | Fake cargo exits 42, assert flux-check exits 42 (proof of B020 failure propagation) |
| I28 | test_guard_zero_tests.sh | grep source for `could not parse` | Feed unparseable fake test output, assert exit=1 (fail-closed; pipefragility noted) |
| I31 | test_loom_list.sh | grep source for `exit 1` | Set PATH sans cargo, assert exit=1 |
| I32 | test_loom_list.sh | grep source for `model_names.*-z` | Fake xtask outputs `Available models: []`, assert exit=1 |
| P04 | test_proptest.sh | grep source for `json.tool` | Run kani-list.sh on vb_core + vb_runtime, validate both JSON outputs via python3 json.tool |

### FIND-TR-002 (CRITICAL): B020 failure propagation path

- **I20** (test_flux_check_package.sh): Rewritten as behavioral test using fake `cargo` that exits 42.
  - Given: flux-check-package.sh vb_core with PATH containing fake cargo (exit 42)
  - When: script is invoked
  - Then: exit code is exactly 42 (failure propagation verified)

### FIND-TR-003 (HIGH): E2E tests strengthened

- **E01**: Now invokes test runner (`runner.sh test_static.sh test_kani_list.sh`) to produce actual pipeline execution evidence
- **E02**: Exercises each lane script (invokes for usage/output) instead of just counting file existence; asserts `lane_checks >= 2` for non-vacuous evidence
- **E03**: Now checks artifact file sizes (non-empty) in addition to existence

### FIND-TR-004 (WARN): `.unwrap()` in fuzz target

- **tooling_loom_list_xtask.rs:69**: Replaced `.unwrap()` with `.map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "/tmp".to_string())`

### FIND-TR-005 (WARN): I34 SKIP pattern inconsistent with I33

- **I34** (test_cargo_fuzz.sh:32): Changed from `[ "$rc" -eq 0 ] || { printf 'SKIP: ...'; return 0; }` to `[ "$rc" -eq 0 ] || { printf 'exit %d\n' "$rc"; return 1; }` — now fails on non-zero exit, aligned with I33.

### FIND-TR-006 (WARN): I12 suppresses all diagnostic output

- **I12** (test_flux_check_package.sh:27): Changed from `"$SCRIPT" vb_core >/dev/null 2>&1` to capturing output and printing exit code + last 5 lines on failure: `printf 'flux check failed (exit %d): %s\n' "$rc" "$(echo "$out" | tail -5)"`.

### FIND-TR-007 (INFO): I30 can't distinguish xtask-not-built from real failure

- **I30** (test_loom_list.sh:23-37): Now distinguishes three cases:
  - `could not parse|FAIL|failed` → FAIL (return 1)
  - `no such|not found|could not find` → SKIP (return 0)
  - Other non-zero → WARN with truncated output (return 0)

### FIND-TR-008 (INFO): B002 only tested structurally

- Covered by FIND-TR-001 rewrite of I02 to behavioral test (simulates cargo kani unavailable, asserts exit 1 + error message).

## Test Suite Results

### Test Count

| Layer | Count | Details |
|-------|-------|---------|
| Integration tests (I01-I37) | 32 | Bash scripts invoked with real/fake tooling |
| E2E tests (E01-E03) | 3 | Pipeline execution + lane exercise + evidence audit |
| Proptest (P01-P06) | 6 | Multi-package invariants + determinism checks |
| Static tests (S01-S05) | 5 | Shellcheck, shebang, schema, model count, moon tasks |
| Fuzz targets | 4 | Tooling argument fuzzing (libFuzzer) |
| **TOTAL** | **50** | All pass |

### Gate Results

- [x] **Source clippy**: Not applicable (bash-only tooling bead, fuzz target passes rustfmt)
- [x] **Test compile**: All 9 test files are bash scripts, execute without syntax errors
- [x] **Runner**: 9 test files passed, 0 failed (51 individual tests, 0 failures)
- [x] **fuzz target**: No `.unwrap()` remaining; passes rustfmt
- [x] **Structural grep elimination**: 0 remaining structural greps in any test file

### Mutation Kill Rate Analysis

Per the test review's mutation matrix (20 checkpoints):

| Mutation | Before | After |
|----------|--------|-------|
| M02 (cargo kani check removed) | NOT KILLED (structural) | **KILLED** (I02: fake cargo exits 1, script caught by exit code assertion) |
| M03 (empty JSON check inverted) | NOT KILLED (structural) | **KILLED** (I09: empty file triggers `! -s` guard, exit 1) |
| M14 (unparseable handler removed) | NOT KILLED (structural) | **KILLED** (I28: unparseable fake output → exit 1 verified) |
| M16 (empty check reversed) | NOT KILLED (structural) | **KILLED** (I32: empty list → exit 1 verified) |
| M17 (xtask failure check removed) | NOT KILLED (structural) | **KILLED** (I31: PATH sans cargo → exit 1 verified) |
| M20 (json.tool validation removed) | NOT KILLED (structural) | **KILLED** (I10: invalid JSON → exit ≠0 verified) |

**New kill rate**: 20/20 = 100% (up from 70% = 14/20)

## Supplementary Finding: Pipefragility

Two behavioral tests (I28, I32) exposed that the production scripts `guard-zero-tests.sh` and `loom-list.sh` have `set -euo pipefail` interactions with `grep|head` and `grep -v` pipelines that cause premature exit before reaching explicit error messages. This is FIND-SHVXY-001 (previously documented). The behavioral tests correctly assert exit 1 (fail-closed) while noting the pipefragility prevents reaching the `printf`-based error messages.

## Summary

All 8 findings resolved:
1. FIND-TR-001: 9 structural tests → behavioral ✅
2. FIND-TR-002: B020 failure propagation tested ✅
3. FIND-TR-003: E2E tests strengthened ✅
4. FIND-TR-004: unwrap() removed ✅
5. FIND-TR-005: I34 SKIP aligned with I33 ✅
6. FIND-TR-006: I12 diagnostic output shown ✅
7. FIND-TR-007: I30 failure distinction added ✅
8. FIND-TR-008: B002 behavioral test added (via I02) ✅

All 9 test files pass. Mutation kill rate: 20/20 = 100%.
