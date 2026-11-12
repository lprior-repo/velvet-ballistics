# TLA+ Temporal Model Plan: vb-qi37.4

## Boundary
- Temporal/workflow behavior: strict/journaled admission lifecycle from pending submit through rejection or acknowledgement.
- Rust/core behavior excluded from TLA+ and handled by Verus/Kani/tests: postcard decoding, proof-flag field checks, capability set matching, digest byte equality, and error enum mapping.
- External systems abstracted: Fjall persistence is reduced to `StorageFail` versus persisted-success transition; accepted artifact store is reduced to valid/invalid artifact availability.

## Executable TLA+ Model
- Module/model path: `specs/admission_header_before_ack.tla`
- Config path: `specs/admission_header_before_ack.cfg`
- Variables: `state`, `code`, `ack`, `persisted`, `live_state`, `duplicate_run`
- Init action: `Init`
- Next/actions: `AdmissionReject`, `StorageFail`, `PersistHeader`, `Ack`, `TerminalStutter`, `Next`
- State constraints: finite states `{Pending, Persisted, Rejected, Acked}`, boolean `duplicate_run`, and finite error-code set from cfg.
- Symmetry sets: none required for current one-run abstraction.
- Bounded model limits: cfg constants `ErrorCodes = {HeaderPersistenceFailed, QueueFull}` and `NoCode`; this explicitly includes queue/capacity failure in the failure-before-ack model.

## TLA+-Owned Clauses
- PRE-005: strict admission success requires durable boundary before acknowledgement.
- PRE-006: duplicate run id rejects before acknowledgement or live state mutation.
- POST-002: live run state is inserted only after `PersistHeader` and `Ack`.
- POST-003: rejected artifacts, duplicate runs, and capacity/storage failures do not create live state.
- POST-004: failure before persistence prevents acknowledgement and reaches rejected terminal state.
- INV-005: failure-prevents-ack safety.
- ERR-004: duplicate run id is modeled as `duplicate_run = TRUE` and can only reject.
- ERR-005: header/admission persistence failures remain rejection, never success.
- ERR-006: active run capacity/queue-full failure is modeled by `QueueFull \in ErrorCodes`; it cannot acknowledge or create live state.

## Properties
- Safety invariants: `TypeOK`, `FailurePreventsAck`, `DuplicateRejectsNoLiveState`, `AckRequiresPersistence`, `LiveStateRequiresPersistence`, `NoLiveStateBeforeDurableAdmission`.
- Liveness/eventuality: `FailureEventuallyRejected`, `SuccessEventuallyAcked`.
- Fairness assumptions: weak fairness on `AdmissionReject`, `StorageFail`, `PersistHeader`, and `Ack` from `Spec`.
- Deadlock freedom: `CHECK_DEADLOCK TRUE` in cfg.
- Refinement to Rust/runtime behavior: `Pending` refines pre-`handle_submit` pending state; `AdmissionReject` refines duplicate-run, artifact, capability, and capacity rejection; `StorageFail` refines failed `append_journal_event`/`append_strict`/`persist_strict`; `PersistHeader` refines the durable accepted-run boundary; `Ack` refines externally visible success and `runs.insert`/live-state visibility only after `persisted = TRUE`.

## Evidence Command
- Direct executable command: `tlc -config specs/admission_header_before_ack.cfg specs/admission_header_before_ack.tla`
- Expected evidence: TLC reports no errors for `TypeOK`, `FailurePreventsAck`, `DuplicateRejectsNoLiveState`, `AckRequiresPersistence`, `LiveStateRequiresPersistence`, `NoLiveStateBeforeDurableAdmission`, `FailureEventuallyRejected`, `SuccessEventuallyAcked`, and deadlock checking under `admission_header_before_ack.cfg`.
- Canonical Moon wrapper note: `moon run :verify-proof` remains desired canonical rollup evidence, but State 6 found it blocked by unrelated shell-invalid wrapper tooling. Contract obligations therefore name the direct executable TLC command for TLA proof evidence until that tooling is repaired.

## Waivers
- No TLA+ waiver for persistence-before-ack. TLA+ is required because this is temporal lifecycle behavior.
