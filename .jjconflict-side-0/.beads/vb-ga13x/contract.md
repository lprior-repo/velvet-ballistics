# vb-ga13x: Repair Kani Fixture Compilation

**Type:** fixture-repair
**Scope:** crates/vb_core/src/kani_idempotency_gates.rs, crates/vb_core/src/replay/kani_harnesses.rs
**Delivery:** Rust contract -> implementation -> tests (Kani) -> review

## Problem
Independent DeepSeek run of `cargo kani -p vb_core --harness kani_action_ticket_has_valid_key` failed to compile due to two fixture issues:

1. `symbolic_ticket()` in `kani_idempotency_gates.rs` lost the `attempt` field when `run_attempt` was added in fixture migration a8452f9e. Current ActionTicket has BOTH `run_attempt` and `attempt` fields.
2. `make_minimal_plan()` in `replay/kani_harnesses.rs` is missing the required `input_slots` field in the `WorkflowParts` struct literal.

## Repair Specification

### Fixture 1: kani_idempotency_gates.rs
Restore `attempt: kani::any()` in `symbolic_ticket()`, preserving the existing `run_attempt: kani::any()`. The canonical key computation must use the same `run_attempt` variable as the ticket to maintain GOD RULE 1 compliance (no hardcoded shapes; the same variable flows into both the ticket and key calculation).

### Fixture 2: replay/kani_harnesses.rs
Add `input_slots: Box::new([])` to the `WorkflowParts` struct literal in `make_minimal_plan()`. This matches the current production struct definition which requires this field. No hardcoded dummy values; this is a minimal required field to compile against the current production interface.

## Verification
- Run `cargo kani -p vb_core --harness kani_action_ticket_has_valid_key` through the vb-run-gate helper.
- Run `cargo kani -p vb_core --harness verify_replay_deterministic_for_same_input` through the vb-run-gate helper.
- Both must compile and prove successfully (not just compile).

## Acceptance
Both named Kani harnesses prove successfully with no vacuous assumptions or hardcoded dummy data that violate GOD RULES.