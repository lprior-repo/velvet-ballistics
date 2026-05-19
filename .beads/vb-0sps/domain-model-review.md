# Domain Model Review - vb-0sps State 3 Repair

## Decision

STATUS: CONTRACT_MODEL_READY_FOR_REVIEW

The repaired model keeps the IR interpreter as oracle and generated runtime as candidate. It closes only a BDD evidence gap and does not claim generated release readiness.

## Type Model

- `ParityInput`: run id, initial PC, slot values, slot taints, value-store fixture, step budget, and optional resume input.
- `ObservedRun`: terminal/block/error status, final PC, executed count when public, slot snapshot, taint snapshot, step-state snapshot, normalized event list, optional suspension metadata.
- `ObservedEvent`: ordered event with kind, run, step, slot, value, taint, action ticket/id, retry, wait metadata, ask metadata, terminal data, typed error data.
- `ObservedSuspension`: kind (`Action`, `WaitUntil`, `WaitEvent`, `Ask`, `Budget`), step, resume PC, ticket/deadline/event/prompt/timeout fields.
- `NormalizedError`: typed class plus semantic fields from `CoreError`, generated `DriveError`, or `CodegenError` mapping.
- `ParityError`: typed mismatch taxonomy used by BDD assertions.

## Boundary Review

- Inside bead: contract for executable BDD evidence for `VB-BDD-CATALOG-007`, non-vacuous TLA+ obligation surface, explicit Verus waiver metadata until real adapters exist, and focused downstream gates.
- Outside bead: changing production runtime semantics, emitting generated Rust as a CLI feature, maxperf acceptance, benchmark ratios, PGO, and release gating.
- Oracle: public `vb_core` interpreter/runtime observations.
- Candidate: public `vb_codegen` accepted generated subset only.

## Observable Parity Fields

Required exact structured comparisons:

1. Terminal result: status, result `SlotValue`, result `Taint`.
2. Typed errors: variant/class and semantic fields, not display-only text.
3. Final PC: exact `StepIdx` or terminal PC convention documented by adapter.
4. Slots: every observed slot value and initialized/uninitialized state.
5. Taints: every observed slot taint plus terminal result taint.
6. Step states: every step state and every legal transition edge for both modes.
7. Suspensions: kind, step, resume PC, and metadata.
8. Resume metadata: action completion/failure payload, ask answer, wait/timer input, retry attempt.
9. Journal/events: ordered normalized sequence and all POST-005 fields.
10. Action tickets: run, step, sequence, action id, attempt, idempotency key where public.
11. Wait/ask scheduling: deadline/event/prompt/timeout/answer slots.
12. Unsupported subset: typed `UnsupportedIr { feature }` before source acceptance/emission/compile/run.

## Repair Adequacy Checks

- The TLA+ contract no longer permits parity-by-construction: IR and generated transitions must be independent and checked by `ObservationRefinesOracle`/parity invariants.
- The TLA+ contract no longer permits unreachable resume/source properties: bounded external resume/source-emission/reject alternatives are required.
- Bounded model claims are scenario-kernel claims only. Split configs may use small finite bounds, but the contract must not claim unproved generalization.
- Verus is not faked. Current Rust-local proofs are explicit waivers until real adapter exec functions exist.
- Canonical obligations now require exact rows for all previously missing clauses.

## Risk Review

- High risk: generated code is deferred; BDD must not be represented as release readiness.
- High risk: journal parity can be weakened by debug strings; contract forbids this.
- High risk: TLA+ can become vacuous if both sides update in one action; contract forbids this.
- Medium risk: generated error types may not match core names exactly; normalized mapping must be explicit and tested.
- Medium risk: Verus proof cannot bind before adapters exist; waiver expires when adapters exist or before State 6 if they already exist.
