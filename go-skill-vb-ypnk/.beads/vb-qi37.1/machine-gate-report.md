# Machine Gate Report: vb-qi37.1

STATUS: PASS

## Passed Gates

- `moon run :fmt`: exit 0; task completed.
- `moon run :lint-src`: exit 0; task completed.
- `moon run :check`: exit 0; tasks `agent-cli-contract`, `beads-server-mode`, `nightly-feature-gate`, and `check` completed.
- `moon run :source-length`: exit 0; task completed.
- `moon run :test`: exit 0; `8358 tests run: 8358 passed (1 slow), 6 skipped`.
- `moon run :bench-build`: exit 0; task completed.
- `rtk cargo test -p velvet-ballastics-workspace-tests --test vb_qi37_1_1_red_recovery_contract_test`: exit 0; 19 passed.
- `rtk cargo test -p vb_storage recovery::tests::`: exit 0; 77 passed.
- `rtk cargo test -p vb_runtime recovery::tests::`: exit 0; 9 passed.
- `PROPTEST_CASES=1000 rtk cargo test -p velvet-ballastics-workspace-tests --test vb_qi37_1_1_red_recovery_contract_test proptest`: exit 0; 3 passed.
- `mkdir -p target/tmp && TMPDIR=target/tmp verus verification/verus/recovery_verification.rs`: exit 0; `verification results:: 17 verified, 0 errors`.
- `JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp TMPDIR=target/tmp tlc -metadir target/tmp/tlc-review-rerun-metadir-2 -config verification/tla/RecoveryHydration.cfg verification/tla/RecoveryHydration.tla`: exit 0; model checking completed with no error.

## Blocked Rollups

- `moon ci`: exit 1 before task execution because Git in this jj workspace cannot resolve `main`; classified as environment rollup blocker, not a failed bead-local check.
- `moon run :verify-proof`: exit 2 because `scripts/rust-verification-gauntlet.sh` begins with Rust doc-comment syntax and is not a valid shell script in this workspace; exact Verus and TLC commands above satisfy the scoped proof obligations.
