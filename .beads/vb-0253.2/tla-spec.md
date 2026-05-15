# TLA+ Temporal Model Plan: vb-0253.2

## Boundary

- **Temporal/workflow behavior**: None — this is a pure facade-conversion refactor with no temporal, workflow, protocol, scheduler, retry, claim/lease, lifecycle, concurrent, or distributed behavior changes.
- **Rust/core behavior excluded from TLA+**: All behavior is handled by existing unit/integration tests in `tests.rs`, `client/tests.rs`, `server/impl_tests.rs`, and `frame/tests.rs`. The refactor is structural (re-export re-organization) only.
- **External systems abstracted**: `crossbeam_channel` is trusted runtime component for `MemoryIngress` bounded queue semantics.
- **Non-applicability rationale**: The vb_ipc crate's `MemoryIngress` queue uses `crossbeam_channel::bounded` which is the authoritative source of truth for channel capacity, FIFO ordering, disconnect semantics, and backpressure behavior. The facade conversion does not change any temporal state — it only removes duplicate struct/enum/function definitions from `lib.rs` and replaces them with re-exports from the canonical modules (`bounded.rs`, `ingress.rs`, `error.rs`, `codec.rs`). There is no workflow, no protocol state machine, no scheduler, no retry logic, no inter-agent coordination, no liveness condition, no fairness condition, and no deadlock possibility introduced by this refactor. The bounded-channel behavior is unchanged.

## TLA+-Owned Clauses

- **None** — explicit waiver: no temporal model applies to a pure facade-conversion refactor that removes duplicate definitions and adds module re-exports.

## Model Shape

- **Module/model path**: N/A
- **Variables**: N/A
- **Init action**: N/A
- **Next/actions**: N/A
- **State constraints**: N/A
- **Symmetry sets**: N/A
- **Bounded model limits**: N/A

## Properties

- **Safety invariants**: N/A
- **Liveness/eventuality**: N/A
- **Fairness assumptions**: N/A
- **Deadlock freedom**: N/A
- **Refinement to Rust/runtime behavior**: N/A

## Evidence Command

- **None** — no TLA+ model exists or is required. `moon run :verify-standard` is the evidence command for the behavioral contract via the existing test suite.

## Waivers

| Clause | Owner | Reason | Expiry | Compensating Evidence |
|---|---|---|---|---|
| Any TLA+ temporal model | vb-0253.2 agent | Facade refactor introduces no temporal behavior changes; all queue semantics are unchanged and exercised by existing tests | N/A | `cargo test -p vb_ipc` passes; moon ci verify-standard lane |
| TLA+ for MemoryIngress queue ordering/disconnect | vb-0253.2 agent | crossbeam_channel is trusted runtime; no new concurrent patterns introduced | N/A | Existing adversarial tests in tests.rs cover Full/Disconnected/Empty cases |
