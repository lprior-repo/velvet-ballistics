# QA Report

STATUS: PASS

## Bead
- `vb-qi37.4.3`
- Current state: State 8 rerun green after State 13 refactor/rebase repair

## Command Evidence

### `moon ci` (canonical QA gate)
```
Tasks: 19 completed (2 cached), 0 failed
Time: 52s 711ms
```
- `velvet-ballastics:test`: 8015/8015 nextest tests passed
- `velvet-ballastics:lint-src`: PASS
- `velvet-ballastics:fmt`: PASS
- `velvet-ballastics:check`: PASS
- `velvet-ballastics:feature-powerset`: PASS
- `velvet-ballastics:source-length`: PASS
- `velvet-ballastics:nightly-feature-gate`: PASS
- `velvet-ballastics:nightly-feature-cargo-probe`: PASS
- `velvet-ballastics:hardened-build`: PASS
- `velvet-ballastics:maxperf`: PASS
- `velvet-ballastics:maxperf-native`: PASS
- `velvet-ballastics:coverage`: PASS
- `velvet-ballastics:doc`: PASS
- `velvet-ballastics:doc-test`: PASS
- `velvet-ballastics:miri`: PASS
- `velvet-ballastics:mutants-smoke`: 1 mutant tested, 1 caught
- `velvet-ballastics:bench-build`: PASS
- `velvet-ballastics:fuzz-smoke`: PASS
- `velvet-ballastics:agent-cli-contract`: cached

Output: `/home/lewis/.local/share/opencode/tool-output/tool_e1a0e953600105TFc0VD4L4qQz`

### `jj status`
```
Working copy (@): ssplpxsu 60c4af54
Parent commit (@-): lxwyustn c9939431 main | landing: merge landable vb-jkrk wave3 qi37.16.3
```
- Bead artifacts: 27 files added/modified in `.beads/vb-qi37.4.3/`
- Split files: 69 Rust façade/chunk files created (all <=300 lines)
- Scoped source: journal.rs, runtime.rs, shard/impl_.rs, shard/lifecycle.rs, shard/tests.rs, admission_evidence_integration.rs

## Artifact Inspection

| File | STATUS |
|------|--------|
| `STATE.md` | State 8 green; highest completed state 13; next gate States 9-14 |
| `moon-report.md` | `moon ci` PASS after rebase repair |
| `regression-diff.md` | No `BLOCK_RELEASE`/`BLOCK_LOCAL`; downstream State 8 green |
| `architectural-drift-review.md` | `STATUS: REFACTORED`; line-count blocker removed |
| `delivery-scope.jsonl` | Scoped crates: vb_runtime, vb_storage, velvet_ballastics |

## QA Gates

| Gate | Result | Evidence |
|------|--------|----------|
| `moon ci` | PASS | 19 completed, 0 failed |
| Test suite | PASS | 8015/8015 passed |
| Lint | PASS | `lint-src`, `fmt`, `check` all green |
| Feature powerset | PASS | All feature combos compile |
| Source length | PASS | Split files <=300 lines |
| Formal verification | PASS | `formal-verification-report.md` APPROVED |
| Black-hat review | PASS | `black-hat-review.md` APPROVED |
| Red-queen | PASS | `red-queen-report.md` APPROVED |
| Test plan | PASS | `test-suite-review.md` APPROVED |

## Blockers
- None. All QA gates pass.

## Next Gate
- Continue downstream States 9-14 after refactor/rebase repair confirmed green.

## Verification
- QA executed: 2026-05-12
- Workspace: `/home/lewis/src/Velvet-ballistics-vb-qi37-4-3-go`
- Forbidden checkout not touched: `/home/lewis/src/Velvet-ballistics`
