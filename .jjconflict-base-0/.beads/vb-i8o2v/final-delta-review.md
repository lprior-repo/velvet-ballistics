# Final benchmark test relocation disposition

Base: `da633fe0bbde9e414173d5f7aabe314a37f8752d`
Finding: `DeletedTestDeclaration|xtask/src/benchmark_policy.rs|removed_declarations=6 added_declarations=0`

The old inline tests `rejects_empty_evidence_for_budgeted_metrics`, `rejects_deferred_generated_budget`, `detects_integer_regression_threshold`, and `rejects_placeholder_instruction_metadata` retain their named oracles in `xtask/src/benchmark_policy_tests.rs`. Error handling now returns a test failure through `Result` instead of panicking helper calls. The 103/104 versus 100 at 3% threshold boundary is unchanged.

`rejects_stale_benchmark_commit` is replaced by actual commit-object, ancestry, dirty working-tree, staged, source/dependency/toolchain/config/script/budget, and untracked/ignored-input rejection cases. `accepts_current_benchmark_commit_prefix` is superseded by resolved commit identity: public evidence accepts full commit records; the internal resolver separately checks a unique prefix resolves to its full commit. Evidence-only descendants are explicitly accepted only when performance inputs remain identical.

The benchmark filter executes 40 tests: 6 existing evidence-gate tests, 6 policy unit tests, 23 freshness unit tests, and 5 public CLI tests. They are not 40 integration tests and the shared fixture file has no tests.

Actual public CLI test names:
- `benchmark_policy_clean_same_head_passes_end_to_end`
- `benchmark_policy_evidence_only_descendant_passes_end_to_end`
- `benchmark_policy_dirty_source_same_head_rejected_end_to_end`
- `benchmark_policy_staged_source_rejected_end_to_end`
- `benchmark_policy_missing_commit_rejected_end_to_end`

Actual freshness unit test names:
- `clean_same_head_is_fresh`
- `evidence_only_descendant_is_fresh`
- `dirty_working_tree_same_head_rejected`
- `staged_change_same_head_rejected`
- `committed_source_change_rejected`
- `touched_then_reverted_source_is_fresh`
- `unrelated_dirty_file_is_fresh`
- `untracked_relevant_rust_rejected`
- `untracked_relevant_config_rejected`
- `gitignored_relevant_config_rejected`
- `ignored_build_output_not_flagged`
- `untracked_irrelevant_file_is_fresh`
- `cargo_lock_mutation_rejected`
- `toolchain_mutation_rejected`
- `bench_fixture_mutation_rejected`
- `perf_budget_mutation_rejected`
- `xtask_policy_mutation_rejected`
- `script_mutation_rejected`
- `non_ancestor_commit_rejected`
- `missing_commit_rejected`
- `non_commit_blob_rejected`
- `resolve_commit_full_sha`
- `resolve_commit_unique_prefix`

Independent source review: `.beads/vb-3iflr/evidence/independent-code-review.md`; independent test review: `independent-test-suite-review.md`. Their earlier global fmt/full-xtask claims are limited by raw results: xtask-specific formatting passed; the old author base lacks the final root HJ formatting; unrelated `integration_gates` alias tests fail because xtask is outside the root workspace. Final combined gates and the actual benchmark filter are recorded separately. No fabricated CLI names or blanket full-xtask success is accepted.

The coordinator checked the local review diff: five `.err()` conversions and four panic helpers were replaced with explicit failure-preserving `Result` handling; negative error-text assertions and test declarations remain. No production freshness policy or thresholds changed in this repair.
