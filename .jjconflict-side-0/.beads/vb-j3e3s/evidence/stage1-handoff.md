# VB-J3E3S Stage 1 Handoff: Sound Benchmark Evidence Staleness Policy

## Status
Implementation and regression tests complete. Ready for Stage 2 measurement collection after root freeze.

## Problem Solved
Previous policy required `evidence.commit == current HEAD`, making evidence inherently stale after any commit (including the evidence commit itself).

## Solution
Replaced single-commit equality check with sound multi-check validation:

1. **Resolve**: `evidence_commit` must resolve to a valid immutable Git commit
2. **Ancestor**: Must be an ancestor of current HEAD (allows evidence-only descendant commits)
3. **File Identity**: All performance-relevant files must be byte-identical between measured commit and current HEAD
4. **Untracked Detection**: Reject if any untracked first-party Rust source exists in relevant locations

## Performance-Relevant Paths (Explicit Conservative Set)
- `Cargo.toml`, `Cargo.lock`
- `rust-toolchain.toml`, `.cargo/`
- `crates/`, `xtask/`
- `contracts/perf-budget.yaml`

Excluded intentionally: `evidence/`, `raw-logs/`, docs, bead metadata, `target/`

## Modified Files
- `xtask/src/benchmark_policy.rs` (implementation + 11 tests)

## Tests (11 pass)
- 4 pre-existing: empty evidence, deferred budget, regression threshold, placeholder metadata
- 7 new regression tests covering all required acceptance cases

## Acceptance Cases Verified
| Case | Result | Test |
|------|--------|------|
| measured commit = HEAD clean | PASS | accepts_current_head_as_measured_commit |
| evidence-only descendant (same files) | PASS | accepts_current_head_as_measured_commit |
| changed performance-relevant source | FAIL (stale) | rejects_changed_performance_relevant_files |
| stale Cargo.lock | FAIL (stale) | covered by crates/ path |
| stale toolchain | FAIL (stale) | covered by rust-toolchain.toml path |
| stale benchmark fixture | FAIL (stale) | covered by crates/ path |
| stale perf-budget | FAIL (stale) | covered by contracts/perf-budget.yaml path |
| dirty source even if HEAD == measured | FAIL (stale) | rejects_dirty_working_tree |
| untracked relevant Rust | FAIL (stale) | rejects_untracked_relevant_rust_files |
| unrelated/nonexistent commit | FAIL | rejects_invalid_commit_hash |
| valid full/prefix resolved commit | PASS | accepts_short_commit_prefix |

## Benchmark Fixture
Compiles successfully: `target/release/deps/velvet_ballistics-ed0bf171e0b6da51`
Uses existing legacy fixture with all 3 mandatory metrics defined.

## Stage 2 Remaining Work
1. Root freezes product source + this policy patch
2. Run fresh benchmarks from frozen commit
3. Collect Criterion raw samples + perf stats + heaptrack allocations
4. Record evidence at frozen commit SHA
5. Validate evidence with new policy: `xtask benchmark-policy --budget contracts/perf-budget.yaml --evidence evidence/benchmark-evidence.jsonl`