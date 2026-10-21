# vb-xx331 scoped Moon CI result

Source snapshot: compiler-guard candidate `cd6a8e8596259c20e0159492bbca041f5de152e7` integrated with base `856603b8` in described JJ integration child `vb-xx331: integrate compiler guard candidate for canonical CI`.

The default forced Moon graph resolves Kani baseline, heavy Kani baseline, and mutants-smoke. It was interrupted after 1m24s to avoid those fleet-wide lanes; Moon returned 0 but marked the run interrupted. This is not a passing result. The separate explicit §40 run omitted only those three lanes, used `CARGO_BUILD_JOBS=2`, Moon concurrency 2, and 300-second timeout. It exited 124 at 4m58s with summary 15 actions completed, 3 failed, 3 skipped.

## Failed gates

- `fmt`: Rustfmt differences in candidate-owned `crates/vb_compile/src/lib.rs` and `migration_compat.rs`; the formatter also cannot parse Flux token syntax in `crates/vb_storage/src/types/flux_sequence.rs` (`16#ffff#`). No formatting or source changes were made.
- `source-length`: 5 production files exceed 300 lines: `crates/vb_compile/src/proptest_finish_digest.rs` (344), `crates/vb_expr/src/expression.rs` (655), `crates/vb_expr/src/expression_bytecode.rs` (568), `crates/vb_runtime/src/error/equality.rs` (309), `crates/vb_storage/src/types/flux_sequence.rs` (318). No exception rows were added.
- `test`: nextest stopped on `vb_compile::digest_repeat_unit::test_repeat_different_set_output_changes_digest`: distinct Set output fields produced identical `WorkflowDigest`. At stop: 574/14,190 tests run; 573 passed, 1 failed, 35 skipped, 13,616 not run.

## Passed gates

`check`, `lint-src`, `feature-powerset`, `beads-server-mode`, `agent-cli-contract`, `workspace-assertions`, `hot-cold-forbidden-apis`, `nightly-feature-gate`, `panic-surface`, `test-integrity`, `unsafe-audit`, `ignored-fallible-results`, and `banned-token-gates` passed.

`doc-test`, `doc`, and `miri` were skipped after dependency failure. Coverage, bench-build, benchmark-regression-policy, and fuzz-smoke were not reached before timeout. Moon emitted remote cache upload errors and a warning that `crates/velvet_ballistics/tests/fixtures/fixtures` does not exist; these warnings did not change the reported outcomes of passed tasks.

No build-script, Moon configuration, or toolchain invocation repair was made. Failures found are source formatting/length and semantic test failures outside the assigned tooling-only scope. Raw logs and actual exit codes are beside this report. Do not claim full CI green or v1 acceptance.
