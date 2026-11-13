# Test-integrity heuristic review proposal

**Status: unapplied proposal for human review.** This receipt documents the exact remaining `check-test-integrity.sh` findings at source pin `55c7569d0071e9db27a72e0538730395d92f6ce9` against base `25d7d327574d4e5fee7d0260727932af72b2ff3d`. It does not modify the gate or its justification TSV and does not claim canonical acceptance.

## Reproduction

Command: `bash scripts/check-test-integrity.sh --base 25d7d327574d4e5fee7d0260727932af72b2ff3d`. Observed exit: 1. Complete stdout is preserved at `.evidence/r9/test-integrity-final.log`. The gate reported one `WeakenedAssertion` finding and seven `IgnoredOrSkippedTest` findings; all are enumerated below. The proposal adds hash-bound records for review, but remains unapplied pending human approval.

Focused registered-suite evidence from the source pin:

- `CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.evidence/r9/target" TMPDIR="$PWD/.evidence/r9/scratch" TMP="$PWD/.evidence/r9/scratch" TEMP="$PWD/.evidence/r9/scratch" cargo test -p velvet-ballistics-workspace-tests --test shipping_nested_data --test shipping_open_wait_reentry --test shipping_repeat_recovery --test shipping_together_recovery -- --test-threads=1` exited 0: 36/36 passed. Raw output: `.evidence/r9/integrity-finding-suites.log`.
- `CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.evidence/r9/target" TMPDIR="$PWD/.evidence/r9/scratch" TMP="$PWD/.evidence/r9/scratch" TEMP="$PWD/.evidence/r9/scratch" cargo test -p velvet-ballistics-workspace-tests --test shipping_foreach_recovery --test shipping_nested_effects --test shipping_nested_data --test shipping_open_wait_reentry -- --test-threads=1` exited 0: 42/42 passed. Raw output: `.evidence/r9/nested-foreach-r9-focused-rerun.log`.

## Finding-by-finding basis

1. `WeakenedAssertion|crates/workspace_tests/tests/shipping_nested_data/cells.rs|removed_exact=0 added_exact=0 added_weak=2`. The two aggregate `failures.is_empty()` assertions follow explicit per-cell checks. The 10 supported cells check exact expected leaf-success counts, nonterminal cut state, owned replay selection, Secret terminal taint, expected scalar/list terminal values, exact ordered list items, and Clean leaf result. The eight Collect-involving cells make the same checks, including exact `[3, 5]` order. Both loops assert the exact number of checked cells. The registered tests are `supported_owned_cells_recover_with_exact_evidence` and `collect_involving_owned_cells_recover_with_exact_evidence`; the implementation records each mismatch as a failure before the aggregate assertion. The checker identifies the aggregate `.is_empty()` syntax but not these exact preceding observations.
2. `IgnoredOrSkippedTest|crates/workspace_tests/tests/shipping_open_wait_reentry.rs|const SOURCE: &str = "version: velvet-ballistics/v1\nname: repeat_open_wait_reentry\nwhen: {manual: {}}\ninputs: {retained: {is: list, of: number, max: 1}}\nsteps:\n  - id: seed\n    set: {output: seed, value: '42'}\n  - id: event_id\n    set: {output: event_id, value: '1'}\n  - id: retry_loop\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: route_action\n          do: {action: '0', input: '$step.seed'}\n        - id: route\n          choose:\n            branches:\n              - when: '$step.route_action == 1'\n                steps:\n                  - id: gate\n                    wait: {event: '$step.event_id'}\n            otherwise: skip_wait\n        - id: skip_wait\n          set: {output: skip, value: 'true'}\n  - id: done\n    finish: {result: '$step.seed'}\n";`. `skip_wait` is a workflow step identifier and an `otherwise` route in the source fixture. The registered cold-recovery test `repeat_reentry_open_wait_survives_fjall_cold_recovery` ran and passed; it is not ignored or short-circuited.
3. `IgnoredOrSkippedTest|crates/workspace_tests/tests/shipping_repeat_recovery/ask.rs|.ok_or("ask repeat replay skipped")?`. This is the failure message used when expected replay evidence is absent. The registered `ask::repeat_ask_attempts_replay_counter_and_secret_result_taint` test ran and passed; the code propagates the error rather than skipping the test.
4. `IgnoredOrSkippedTest|crates/workspace_tests/tests/shipping_repeat_recovery/replay.rs|.ok_or("complete scalar replay skipped")?`. This likewise turns missing replay evidence into a test error. Registered test `replay::repeat_persisted_trace_replays_exact_counter_state_and_pc` ran and passed.
5. `IgnoredOrSkippedTest|crates/workspace_tests/tests/shipping_together_recovery/failure.rs|let skipped = live`. `skipped` names the expected runtime state for the unreachable workflow step in a real failure-policy scenario; it is not a test-skip mechanism.
6. `IgnoredOrSkippedTest|crates/workspace_tests/tests/shipping_together_recovery/failure.rs|"failure policy skips the unreachable step"`. This describes the product behavior under test. Registered test `failure::failing_branch_routes_to_handler_and_cold_recovery_rejects_wrong_terminal` ran and passed.
7. `IgnoredOrSkippedTest|crates/workspace_tests/tests/shipping_together_recovery/failure.rs|live.state.frame.pc() != skipped,`. This asserts the runtime program counter advanced past the unreachable step after failure routing.
8. `IgnoredOrSkippedTest|crates/workspace_tests/tests/shipping_together_recovery/support.rs|set: {output: skipped, value: '99'}`. `skipped` is a workflow output field in the failure-policy fixture, not test metadata. The `shipping_together_recovery` suite ran 13/13, including the failure-routing and recovery tests.

No finding represents an `#[ignore]`, early `return`, removed test, or missing assertion. The original gate still runs unchanged. Every test named above was observed in the raw Cargo test logs linked above.

## Hash bindings and proposed disposition

Base: `25d7d327574d4e5fee7d0260727932af72b2ff3d`. Source/test candidate: `55c7569d0071e9db27a72e0538730395d92f6ce9` (tree `284b4485ce849b91e46dabce7aef4444695b00d0`). The affected test files were absent at base; their candidate Git blob IDs are:

| Path | Candidate blob |
| --- | --- |
| `crates/workspace_tests/tests/shipping_nested_data/cells.rs` | `b9beb219f3ad5834ea8e9cb2da0480e43ce56e57` |
| `crates/workspace_tests/tests/shipping_open_wait_reentry.rs` | `d4f17c7287233a01f43c6069fac204ada06cc9a5` |
| `crates/workspace_tests/tests/shipping_repeat_recovery/ask.rs` | `fbb3e0c4de19b201d3b5b2d9ec41c00a4996df2c` |
| `crates/workspace_tests/tests/shipping_repeat_recovery/replay.rs` | `f53773db30bee7e8292b4f8677a5a7000c30ec08` |
| `crates/workspace_tests/tests/shipping_together_recovery/failure.rs` | `dfb4afac094380836e3b8bbbc9d8c5a078c7a9e3` |
| `crates/workspace_tests/tests/shipping_together_recovery/support.rs` | `e0bd9bd0ebcc9c8a5bdcf0790cb68b15ba61a788` |

The unapplied TSV rows are at `.evidence/r9/test-integrity-justifications-proposal.tsv`, with an unapplied patch at `.evidence/r9/test-integrity-justifications-proposal.patch`. They use the parser's exact 11-column layout and bind the base, absent old blob, current file blob, exact finding text, registered replacement-test blobs, this review receipt blob, and bead `vb-eirmk.75.6.4`. The existing `scripts/test-integrity-justifications.tsv` was not edited; its preimage SHA-256 and Git blob are recorded in `.evidence/r9/test-integrity-preimage.txt`. Any addition remains subject to human approval.
