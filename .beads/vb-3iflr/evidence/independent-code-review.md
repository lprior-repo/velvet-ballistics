# Independent Code Review: vb-3iflr Benchmark Freshness

**Reviewer:** LOCAL5090 independent agent
**Subject:** DeepSeek production author commit `29a506a02` vs original main `da633fe0bbde9e414173d5f7aabe314a37f8752d`
**Scope:** xtask benchmark policy/budget/freshness/support/tests only
**Review date:** 2026-10-22

## Verdict: ACCEPT

No blocking defects found in production code. Two minor test-gate violations identified and fixed by reviewer (not production).

## Files Reviewed (blob hashes at review commit)

| File | Hash | Type |
|------|------|------|
| xtask/src/benchmark_freshness.rs | f8abe6e52b4d51fee1f9433322113c20425f9f82 | Production |
| xtask/src/benchmark_freshness_tests.rs | 4de38b5dfcd4f717a95f0eb8b199420e2701efd4 | Tests |
| xtask/src/benchmark_policy.rs | caf3bc34e9a91ef49da2379d40c411a349d2d2d8 | Production |
| xtask/src/benchmark_policy_tests.rs | 0cce53c1806d543e9f3dbf4243ee9de7743fa3ae | Tests |
| xtask/src/benchmark_budget.rs | b2327bf06aa7b73f99e232f40666c809a74ff8b0 | Production |
| xtask/src/benchmark_tests_support.rs | 2c6a8b9709eeab7990a9146b0fcc239f19618d94 | Test support |
| xtask/tests/benchmark_policy_cli.rs | 1b83d76ea6a51044419db85412507fa37c88f73c | CLI integration |
| xtask/src/main.rs | fdad0a0dcb6de640791564332360919b9d6f3b92 | Entry point |

## Production Code Assessment

### benchmark_freshness.rs — ACCEPT

**Architecture:** Conservative validation that evidence commits are ancestors of current HEAD and that all performance-relevant source bytes are identical between measured commit and actual working tree.

**Strengths:**
- Explicit `PERF_RELEVANT_PATHS` list (crates, xtask, .moon, Cargo.lock, toolchain files, scripts/perf-budget) is intentionally conservative.
- Correctly compares against working tree, not just HEAD — dirty/staged changes invalidate evidence.
- `resolve_commit` peels tags via `^{commit}` and rejects blob/tree objects.
- `changed_paths_vs_working_tree` uses `--no-renames` (move-out must surface as delete) and `-z` (lossless path decoding).
- Untracked file detection does NOT use `--exclude-standard` — ignored `.cargo/config` or ignored `include!`d sources still invalidate.
- Build output excluded only for genuine artifacts (`target`, `.git`, `.jj`, `.moon/cache`).

**Weaknesses (non-blocking):**
- None found.

### benchmark_policy.rs — ACCEPT

**Architecture:** Schema validation for benchmark evidence claims, threshold checking, and freshness binding.

**Strengths:**
- Requires explicit commit hash in every evidence claim.
- Validates required fields per budget specification.
- Threshold percent validation (0-10000).
- Rejects placeholder values (TBD, NA, etc.).
- Mode validation against deferred-mode lists.

**Weaknesses (non-blocking):**
- None found.

### benchmark_budget.rs — ACCEPT

**Architecture:** YAML budget file parsing with schema validation.

**Strengths:**
- Explicit field requirements.
- Validates milestone, modes, and benchmark entries.

**Weaknesses (non-blocking):**
- None found.

### main.rs / CLI — ACCEPT

Proper subcommand registration for `benchmark` family.

## Test Code Assessment

### Test Coverage — ACCEPT

29 unit tests + 6 evidence-gate tests + 5 public CLI integration tests = 40 total.

**Positive cases covered:**
- Clean same-head is fresh
- Evidence-only descendant commits are fresh
- Touched-then-reverted source is fresh
- Unrelated dirty files are fresh
- Ignored build output not flagged
- Untracked irrelevant files are fresh

**Negative cases covered:**
- Dirty working tree (same head) rejected
- Staged changes (same head) rejected
- Committed source change rejected
- Untracked relevant Rust rejected
- Untracked relevant config rejected
- Gitignored relevant config rejected
- Cargo.lock mutation rejected
- Toolchain mutation rejected
- Bench fixture mutation rejected
- Perf budget mutation rejected
- Xtask policy mutation rejected
- Script mutation rejected
- Non-ancestor commit rejected
- Missing commit rejected
- Non-commit blob rejected

**Commit format resolution:**
- Full 40-char SHA resolves
- Unique short prefix resolves

### Test Gate Violations (FIXED BY REVIEWER AND FOLLOW-UP AGENT)

Two files had 5 instances of `.err().context(...)?` pattern that violates DISCARD-003 (lossy fallible result handling). Fixed by replacing with explicit `match` destructuring returning `anyhow::Result`:

1. `benchmark_freshness_tests.rs:8` — `stale_rejection` helper
2. `benchmark_freshness_tests.rs:250` — `missing_commit_rejected` test
3. `benchmark_freshness_tests.rs:265` — `non_commit_blob_rejected` test
4. `benchmark_policy_tests.rs:88` — `rejects_non_hex_commit_format` test
5. `benchmark_policy_tests.rs:110` — `rejects_deferred_mode_in_claim` test

These fixes preserve exact expected error messages while satisfying the ignored-fallible-results gate.

Additionally, the original worker left 4 `.expect()`/`.expect_err()` calls in NEW test functions in `benchmark_policy_tests.rs`. Fixed by converting to explicit `match`/`?` patterns with `anyhow::Result<()>` return type:

1. `benchmark_policy_tests.rs:53` — `rejects_empty_evidence_for_budgeted_metrics`: `parse_budget(VALID_BUDGET).expect("valid budget")`
2. `benchmark_policy_tests.rs:55` — same test: `.expect_err("budgeted metrics require evidence")`
3. `benchmark_policy_tests.rs:62` — `rejects_deferred_generated_budget`: `parse_budget(&text).expect_err("generated budget is deferred")`
4. `benchmark_policy_tests.rs:79` — `rejects_placeholder_instruction_metadata`: `.expect_err("placeholder instruction metadata is rejected")`

All 4 functions now return `anyhow::Result<()>` and use explicit Err destructuring. Negative tests bail on unexpected Ok; positive parse uses `?`. Threshold and assertion behavior unchanged.

## Verification Commands Run

```bash
bash scripts/check-ignored-fallible-results.sh   # PASS
cargo test --manifest-path xtask/Cargo.toml benchmark  # 40/40 PASS
cargo fmt --manifest-path xtask/Cargo.toml --check     # PASS
cargo clippy --manifest-path xtask/Cargo.toml ...      # PASS (strict)
```

## Limitations

- Only reviewed xtask benchmark subsystem; broader Moon CI, Kani/Verus, and fuzz runs excluded per scope.
- No benchmark measurements taken; only policy/freshness/validation logic reviewed.
- Production contract/threshold values not changed by this bead — inherited from earlier work.

## Conclusion

Production code is correct, conservative, and well-architected. Test coverage is comprehensive across positive, negative, and edge cases. The two test-gate violations were minor style issues that have been corrected. No blocking defects found.