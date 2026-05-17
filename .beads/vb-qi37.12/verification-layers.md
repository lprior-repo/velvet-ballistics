# Verification Layers: vb-qi37.12

## Boundary
- Verus-owned kernel: classification lattice, diagnostic envelope preservation, recovery decode classification.
- TLA+ temporal model: persist/ack/recovery/runtime failure lifecycle.
- Theorem projection: none unless State 4 discovers a tiny kernel beyond Verus.
- Runtime shell: Fjall, filesystem process locks, postcard decode implementation, shard mutation, compiler validation execution.
- External systems excluded from formal proof: OS, database engine internals, scheduler, process death.

## Layer Assignment
- PRE-001 -> Verus classification model + focused static scan + traceability inventory.
- PRE-002 -> Verus classification model + mutation/static scan of typed discard exceptions.
- PRE-003 -> static scan + focused compile/runtime/storage API tests in later states.
- PRE-004 -> Verus decode classification + corruption-injection tests in later states.
- POST-001 / INV-001 -> TLA+ lifecycle model + journal/storage failure-injection tests + `moon ci` release gate later.
- POST-002 -> process lock failure-injection tests + static scan of best-effort metadata discard annotations.
- POST-003 / INV-002 -> runtime failure-injection tests + diagnostic envelope Verus obligation.
- POST-004 / INV-005 -> compiler negative tests + static scan for discarded validation errors.
- POST-005 / INV-003 -> Verus decode classification + storage recovery corruption tests.
- POST-006 / INV-004 -> mechanical ignored-result gate from `vb-qi37.12.4` + inventory review.

## Verus Scope
- Rust targets:
  - `.beads/vb-qi37.12/proof/verus/discard_classification.rs` for `DiscardClassification`.
  - `.beads/vb-qi37.12/proof/verus/diagnostic_envelope.rs` for `DiagnosticEnvelope`.
  - `.beads/vb-qi37.12/proof/verus/recovery_decode_class.rs` for `RecoveryDecodeClass`.
- Spec/proof functions: `spec_classified_sites_have_no_implicit_discard`, `proof_diagnostic_envelope_preserves_cause`, `proof_corrupt_decode_not_absent_success`.
- Invariants: no implicit discard classification; required diagnostic fields preserved; corrupt bytes not classified as successful absence.
- Trusted boundary: validated inventory input, abstract diagnostic envelope constructors, abstract decode result classifier.
- Shell exclusions: Fjall I/O, filesystem lock behavior, postcard internals, runtime scheduling, storage persistence side effects.
- Evidence commands:
  - `verus .beads/vb-qi37.12/proof/verus/discard_classification.rs`.
  - `verus .beads/vb-qi37.12/proof/verus/diagnostic_envelope.rs`.
  - `verus .beads/vb-qi37.12/proof/verus/recovery_decode_class.rs`.

## TLA+ Scope
- Module/model path: `.beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.tla`; config `.beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.cfg`; module `SilentDiscardLifecycle`.
- Variables: `op_state`, `persist_result`, `ack_state`, `runtime_state`, `recovery_input`, `diagnostic`, `discard_classification`.
- Actions: `StartMutation`, `PersistOk`, `PersistFail`, `AckSuccess`, `ReturnTypedError`, `DecodeCorrupt`, `HydrateFailClosed`, `EngineDriveFail`, `TerminalFailureWithCause`.
- Safety invariants: `NoAckAfterFailedRequiredPersist`, `NoUnclassifiedDiscard`, `CorruptionDoesNotHydrateEmptySuccess`, `DiagnosticCausePreserved`.
- Temporal properties: `PersistFailureEventuallyTypedError`, `RecoveryCorruptionEventuallyFailClosed`.
- Fairness/deadlock stance: weak fairness on typed-failure actions; check deadlock freedom under finite model bounds.
- Refinement boundary: Rust runtime/storage events map by operation, result, run id, record kind, and diagnostic cause token.
- Evidence command: `TMPDIR=target/tmp tlc -config .beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.cfg .beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.tla`.
- Deadlock evidence: State 5 repair removed the explicit TLA `Stutter` action from `Next`; stuttering is supplied only by `[Next]_vars`, and TLC deadlock checking is enabled.

## Second-Ring And Release Layers
- Static scan: exact raw inventory command is `/usr/bin/rg -n "let _ =|\\.ok\\(|Err\\(_\\)|Err\\([^)]*\\) =>|log::|tracing::" crates/vb_storage/src crates/vb_runtime/src crates/vb_compile/src crates/workspace_tests/src > .beads/vb-qi37.12/silent-discard-scan-report.full.raw.txt`; `silent-discard-scan-report.md` classifies every candidate and proves zero unclassified release-critical silent discards.
- Kani/proptest: later states should bound classification and decode state transitions once APIs exist.
- Miri/cargo-careful: later states should run storage/runtime focused checks when injected failures exercise memory/aliasing-sensitive paths.
- Fuzz/Bolero: target `vb_qi37_12_persisted_payload_decode` is wired and executed with absolute `TMPDIR`/GNU target repair; see `.beads/vb-qi37.12/proof/fuzz/persisted_payload_decode.md`.
- Mutation/coverage: tests for injected journal/storage/action/recovery failures must be mutation-resistant and covered.
- Release gate: because `release_critical=true`, final evidence must include the repo canonical gate `moon ci` after implementation.

## Waivers
- Lean/Aeneas/Hax waived at State 3 as recorded in `lean-contract.md`.
- Performance and assembly/IR are non-goals for this bead unless implementation introduces a speed or zero-cost claim.
