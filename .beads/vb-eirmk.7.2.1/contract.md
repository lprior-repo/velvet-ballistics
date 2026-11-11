# Reduce replay step semantics parity

Verify that Reduce step replay via `replay_step_with_collect` produces the same result as the live engine for empty, single, and multi-item input lists with an accumulator expression `$total + 1`, initial value `10`, and input list bound to slot 8.

Expected results for input list sizes: empty → 10, single item → 11, multiple items → 13 (for 3 items).

The test must drive the workflow step by step until `ReplayAction::Finished`, compare the final output to the independently computed expected result, and assert that the source ListID, root list contents, and input slot taint are preserved. Use the actual public replay API; do not fabricate terminal pc, states, or evidence.

Production scope: new workspace test only. No source edits. Existing tests remain passing.