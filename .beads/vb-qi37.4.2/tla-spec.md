# TLA+ Temporal Model Plan: vb-qi37.4.2

## Non-applicability Rationale

The admission gate sequencing in `handle_submit_with_inputs_contracts_and_header_mode` is a **single atomic step function** — no branching on state, no concurrency, no inter-step temporal dependencies, no liveness requirements, and no state machine with multiple transition paths that could violate ordering.

The sequencing is fully determined by the linear execution of Rust statements:

```
build_admission → [? short-circuit on Err] → take_frame → journal RunSubmitted → journal RunAdmission → runs.insert
```

This is a **structural ordering invariant**, not a temporal/liveness property. It is verified by:
1. Code inspection (the `?` operator on line 86 short-circuits all subsequent steps)
2. Integration tests with `NeverPresentArtifactStore` + Strict policy verifying `active_run_count == 0` after rejection

TLA+ would add no value here because:
- There are no concurrent agents or processes
- There are no fairness conditions or eventual delivery requirements
- There are no state machine branches that could deadlock or livelock
- The model would be a 1:1 mirror of the Rust code with no abstraction benefit
- Model-checking would not reveal any property not already visible from the `?` propagation

## TLA+-Owned Clauses

None.

## Explicit Waiver

This bead waives TLA+ for temporal modeling. The verification relies entirely on:
1. Structural code inspection (the `?` short-circuit is deterministic Rust control flow)
2. Integration tests (`miri` + `cargo test`) with NeverPresentArtifactStore under Strict/Journaled policy
3. Unit tests for `admit_artifact_run` with NeverPresentStore under Strict/Journaled policy

**Owner**: vb-qi37.4.2  
**Reason**: Single atomic step function with no temporal/state-over-time behavior  
**Compensating Evidence**: Integration tests + Miri execution with strict store rejection
