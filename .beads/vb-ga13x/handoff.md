# vb-ga13x Handoff

## Status: FIXED (compilation), PARTIAL (proof timeout)

Both Kani fixture compilation errors from the DeepSeek run are fixed. One harness proves successfully; the other times out.

## Changes Made

1. **crates/vb_core/src/kani_idempotency_gates.rs**: Restored missing `attempt: kani::any()` field in `symbolic_ticket()` function. The fixture migration a8452f9e added `run_attempt` but accidentally removed `attempt`. The current ActionTicket struct has both fields.

2. **crates/vb_core/src/replay/kani_harnesses.rs**: Added missing `input_slots: Box::new([])` field to `WorkflowParts` struct literal in `make_minimal_plan()`. The production struct requires this field.

## Verification Results

### kani_action_ticket_has_valid_key (FIXED, PROVED)
- Command: `cargo kani -p vb_core --harness kani_action_ticket_has_valid_key`
- Exit code: 0
- Time: 506 seconds
- Result: VERIFICATION:- SUCCESSFUL
- Evidence: .beads/vb-ga13x/evidence/kani-ticket-compile.{stdout,stderr,json}
- This was the specific harness DeepSeek ran that failed to compile. It now compiles and proves.

### verify_replay_deterministic_for_same_input (COMPILES, TIMEOUT)
- Command: `cargo kani -p vb_core --harness verify_replay_deterministic_for_same_input`
- Exit code: 124 (timeout)
- Time: 840 seconds (hit 14-minute limit)
- Result: Proof in progress but not completed within budget
- Evidence: .beads/vb-ga13x/evidence/kani-replay-deterministic.{stdout,stderr,json}
- The harness now compiles (input_slots fix verified). The CBMC solver made significant progress (loop unwinding iteration 162+) but did not reach VERIFICATION:- SUCCESSFUL within the 840s budget. This is a proof complexity issue, not a compilation issue.

## Limitations

- The replay deterministic harness timeout may indicate the proof obligation is too large for bounded verification with the current CBMC solver and input. This is not a fixture bug but a proof complexity issue.
- The harness uses `kani::any()` for slot values, creating a very large symbolic state space. A future improvement could use bounded concrete values instead of fully symbolic execution.

## Remaining Work for Root

- Decide whether to accept the timeout as expected for this harness or rerun with a longer timeout.
- If longer timeout is needed, rerun: `python3 /tmp/vb-run-gate.py 3600 .beads/vb-ga13x/evidence/kani-replay-deterministic cargo kani -p vb_core --harness verify_replay_deterministic_for_same_input`
- The compilation fixes are complete; the ticket harness proves as expected.