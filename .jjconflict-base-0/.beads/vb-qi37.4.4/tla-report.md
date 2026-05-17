# TLA+ Report

STATUS: APPROVED

## Obligation
- id: TLA-ERR-001
- target: `specs/admission_header_before_ack.tla`
- config: `specs/admission_header_before_ack.cfg`
- claim: admission/storage failure before acknowledgement terminates without Ack

## Model Shape
- variables: `state`, `code`, `ack`
- constants: `ErrorCodes`, `NoCode`
- actions: `AdmissionReject`, `StorageFail`, `Ack`, `TerminalStutter`
- spec: `Init /\ [][Next]_vars /\ WF_vars(AdmissionReject) /\ WF_vars(StorageFail)`
- invariants: `TypeOK`, `FailurePreventsAck`
- temporal property: `FailureEventuallyRejected`
- deadlock stance: `CHECK_DEADLOCK TRUE`; terminal states use explicit stutter
- symmetry: none

## Bounds
- `ErrorCodes = {HeaderPersistenceFailed, QueueFull}`
- `NoCode = NoCode`
- finite states: `{Pending, Rejected, Acked}`
- workers: 1 TLC worker
- state/action constraints: none

## Command Evidence
- `tlc -config specs/admission_header_before_ack.cfg specs/admission_header_before_ack.tla` -> PASS
- TLC 2.19, breadth-first, 1 worker.
- Initial states: 2 distinct states generated.
- Complete graph: 8 states generated, 4 distinct states found, 0 states left on queue.
- Diameter: 2.
- Temporal checking: completed; no error found.

## Refinement Map
- `AdmissionReject` / `StorageFail` refine runtime admission/header persistence failure classification before acknowledgement.
- `code` refines stable `RuntimeError` diagnostic codes exposed by typed APIs.
- `ack = TRUE` refines successful admission acknowledgement; `FailurePreventsAck` proves any classified failure remains unacknowledged.

## Limitations
- Bounded finite model covers two representative admission durability codes.
- No symmetry reduction used.
