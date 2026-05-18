# TLA+ Temporal Model Plan: vb-ahfl

## Boundary

- Temporal/workflow behavior: none required for the State 2 UI schema parity scope as written. The contract is static data shape, canonicalization, redaction, and bounds validation.
- Rust/core behavior excluded from TLA+ and handled by Verus/Kani/proptest/tests: metadata completeness, bounded collections, graph references, event ordering, redaction projection, and CLI/UI canonical equality.
- External systems abstracted: CLI JSON/JSONL emitters, Makepad rendering, runtime execution, wall-clock timestamp production.
- Non-applicability rationale: No accepted State 2 delivery-scope clause defines a lifecycle, scheduler, retry, lease, claim, distributed protocol, or concurrent process. `BLOCKER-SCOPE-001` is resolved for this artifact stack by explicit UI artifact schema parity scope. If the owner/orchestrator instead selects engine YAML-to-IR semantic evidence, State 2/3/4/5 must be regenerated and a new TLA+ plan must model YAML compile/admit/run/journal/replay lifecycle.

## TLA+-Owned Clauses

- WAIVED-TLA-001 -> INV-007/POST-008: UI model boundary is a static dependency/data-flow boundary in this scope, not a temporal protocol.

## Model Shape

- Module/model path: not created in State 3.
- Variables: not applicable for current scope.
- Init action: not applicable for current scope.
- Next/actions: not applicable for current scope.
- State constraints: not applicable for current scope.
- Symmetry sets: not applicable for current scope.
- Bounded model limits: not applicable for current scope.

## Properties If Scope Changes To Engine YAML-To-IR

- Safety invariants would include digest binding, no runtime YAML reparsing, accepted artifact admission before run, and journal event signature equality.
- Liveness would include valid strict YAML workflows eventually reaching completed/suspended/typed-failed terminal state under fair runtime steps.
- Fairness would include weak fairness for enabled runtime step execution and recovery replay.
- Deadlock freedom would require no non-terminal admitted run without an enabled action or typed suspension.
- Refinement would map Rust runtime events to model actions by run id, artifact digest, step idx, and journal seq.

## Evidence Command

- Current UI schema scope: no TLA+ command required; waiver must be reviewed against `SCOPE-001`.
- If scope changes to engine YAML-to-IR: State 3 must be regenerated with an exact model path and command after proof target discovery; do not invent one here.

## Waivers

- WAIVED-TLA-001
  - Clauses: INV-007, POST-008.
  - Owner: State 3 rust-contract, pending independent contract verification review.
  - Reason: Current State 2 scope has no temporal behavior; static dependency/data schema constraints are better covered by Verus, Kani/proptest, static scan, API compatibility, and tests.
  - Expiry: immediately if the owner/orchestrator rejects the accepted UI delivery scope, selects engine YAML-to-IR scope, or introduces an asynchronous artifact pipeline.
  - Compensating evidence: VERUS-META-001, VERUS-BOUNDS-001, VERUS-REDACT-001, VERUS-GRAPH-001, KANI-CANON-001, PROP-PARITY-001, STATIC-BOUNDARY-001, API-COMPAT-001, GATE-CI-001.
