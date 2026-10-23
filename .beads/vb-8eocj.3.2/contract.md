# Repeat Check Counter Evidence Negative Probe (vb-8eocj.3.2)

Local negative IR probe on RTX5090 to expose the pre-fix repeat counter
evidence gap before the vb-8eocj.3.1 lowering fix is merged.

## What this proves

Two observational tests on the current unfixed codebase:
1. Baseline repeat workflow from source compiles and executes correctly
   (finishes with value 42, exactly 3 body executions, under 64-step budget).
2. Malformed-IR negative probe: compile the same workflow, mutate the RepeatCheck
   node's output slot from None to the counter state slot, reconstruct via
   CompiledWorkflow::try_from_parts, and observe whether the constructor or
   runtime rejects the mutated IR.

Pre-fix expectation: try_from_parts rejects the mutated shape (BackwardEdge or
similar validation failure) OR the runtime accepts it but the counter writes are
not captured as SlotWritten evidence because RepeatCheck has no declared output.

## Constraints

- Uses ONLY the proven engine API from vb_f2_source_slot_preservation.rs.
- No Runtime, FjallJournal, iter_nodes, submit_direct, or v3-invented APIs.
- Test file: crates/workspace_tests/tests/vb_f1_repeat_counter_reset_probe.rs
- No config editing, cargo config changes, or broad API exploration.
- No production code changes; pure observational probe.
