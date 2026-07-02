# Verification Layers: vb-engine-yaml

## Boundary

- Verus-owned kernel: numeric models, resource bounds, checked access, budgets, taint/capability/artifact lattices, recovery validity predicates, state transition preservation.
- TLA+ temporal model: strict admission, persist-before-ack, ingress/backpressure, run lifecycle, journal ordering, recovery/replay fail-closed behavior.
- Theorem projection: no Lean/Aeneas/Hax kernel required at State 3.
- Runtime shell: direct API, IPC, CLI, Fjall, Postcard, filesystem, Moon commands; verified by integration tests, fuzz, Miri/cargo-careful where applicable, static scans, mutation, coverage, and manual/operator evidence.
- External systems excluded from formal proof: OS, terminal, Fjall internals, hardware persistence guarantees beyond explicit flush/batch result contracts.

## Layer Assignment

- PRE-001 -> dependency-boundary static scan + contract scenarios + fuzz/parser evidence.
- PRE-002 -> Verus artifact/capability model + TLA+ admission lifecycle + Kani/proptest artifact constructors.
- PRE-003 -> TLA+ persist-before-ack + integration failure injection + storage record decode fuzz.
- PRE-004 -> Verus/Kani resource bounds + proptest/fuzz adversarial workflows + static scan for unbounded APIs.
- PRE-005 -> TLA+ recovery + Verus recovery validity + replay/recovery integration evidence.
- PRE-006 -> TLA+ ingress/backpressure protocol model + Loom/property for bounded IPC/direct queues + fuzz IPC frames.
- POST-001 -> parser/validator/compiler parity tests + fuzz YAML profile + mutation.
- POST-002 -> TLA+ admission + integration tests rejecting raw IR/dummy proof/legacy formats.
- POST-003 -> TLA+ persist-before-ack + Fjall failure injection + recovery-after-crash evidence.
- POST-004 -> dependency-boundary and banned-token scans.
- POST-005 -> TLA+ lifecycle + Verus step state/budget + Loom/property where concurrent orchestration exists.
- POST-006 -> TLA+ recovery + integration corrupt/mismatch recovery + storage decode fuzz.
- POST-007 -> TLA+ typed ingress/operator outcome model + CLI/operator golden/semantic tests + manual QA evidence.
- POST-008 -> `moon ci` + focused engine-scoped gates.
- INV-001 -> dependency-boundary scan + static-scan + contract scenarios.
- INV-002 -> Verus checked numeric model + static scan for runtime string lookup.
- INV-003 -> TLA+ artifact lifecycle + Verus artifact digest model + integration digest mismatch tests.
- INV-004 -> TLA+ persist-before-ack + storage failure injection.
- INV-005 -> TLA+ lifecycle + Verus sequence monotonicity + replay tests.
- INV-006 -> Verus/Kani resource bounds + proptest/fuzz + performance/resource stress evidence.
- INV-007 -> Verus gate lattice + integration missing-gate rejection + mutation.
- INV-008 -> TLA+ recovery + integration no-YAML-reparse evidence + static scan.
- INV-009 -> traceability/release evidence that UI/codegen parity are non-goals for this bead.

## Verus Scope

- Rust targets: `verification/verus/resource_budget.rs`, `verification/verus/budget_bounded.rs`, `verification/verus/budget_monotonic.rs`, `verification/verus/step_budget.rs`, `verification/verus/step_state_machine.rs`, `verification/verus/taint_lattice.rs`, `verification/verus/value_store_invariant.rs`, `verification/verus/recovery_verification.rs`, `verification/verus/capability_artifact_model.rs`.
- Spec/proof functions: file-local spec/proof surfaces owned by proof-writer; exact names must be confirmed when executing Verus.
- Invariants: checked numeric bounds, monotonic budgets/sequences, valid step transitions, taint join monotonicity, accepted artifact/capability gate validity, recovery record completeness.
- Trusted boundary: constructors that validate IDs, resource contracts, durable record decode, and accepted artifact proof envelope before runtime use.
- Shell exclusions: Fjall I/O, Postcard implementation internals, CLI rendering, IPC socket loop, wall-clock time, OS process execution.

## TLA+ Scope

- Existing model: `verification/tla/CapabilityLifecycle.tla` with `CapabilityLifecycleAll.cfg`.
- Planned models: `EngineYamlAdmission`, `EngineYamlRunLifecycle`, `EngineYamlRecovery`, `EngineYamlIngress`.
- Variables: artifact_state, durable_records, ack_state, run_state, seq, ingress_queue, proof_gates, capabilities, recovery_source.
- Actions: ValidateYamlCold, CompileNumericIr, VerifyGates, PersistBatch, AckAccepted, FailBeforeAck, SubmitDirect, SubmitIpc, RejectBackpressure, StartRun, Step, Suspend, AppendJournal, CompleteAction, Retry, Cancel, Finish, Fail, BeginRecovery, HydrateFromDurableRecords, DetectMismatch, FailClosedRecovery, Replay.
- Safety invariants: NoAckWithoutDurableAcceptedRecords, NoRawIrBypass, NoRuntimeYaml, SeqMonotonic, BoundedIngress, CapabilityGateRequired, FailClosedRecovery.
- Temporal properties: EventuallyAckOrFailBeforeAck, EventuallyTerminalOrSuspended, RecoveryEventuallyHydratesOrFailsClosed, IngressEventuallyAcceptedOrTypedRejected.
- Fairness/deadlock stance: weak fairness on enabled internal persist/dequeue/recovery/terminal actions; no deadlock except modeled terminal stutter.
- Refinement boundary: Rust events and durable records refine TLA+ actions by run id, artifact digest, sequence, command, durability result, gate result, recovery result.

## Theorem Scope

- None at State 3.
- Non-goal: theorem-proving I/O shells, async runtimes, Fjall internals, CLI rendering, or storage adapters.

## Exact Evidence Commands

- Canonical workspace gate: `moon ci`
- Existing TLA+ capability model: `tlc -config verification/tla/CapabilityLifecycleAll.cfg verification/tla/CapabilityLifecycle.tla`
- Required TLA+ ingress/backpressure model: `tlc -config verification/tla/EngineYamlIngress.cfg verification/tla/EngineYamlIngress.tla`
- Existing Verus files: `verus verification/verus/resource_budget.rs`, `verus verification/verus/step_budget.rs`, `verus verification/verus/step_state_machine.rs`, `verus verification/verus/taint_lattice.rs`, `verus verification/verus/recovery_verification.rs`, `verus verification/verus/capability_artifact_model.rs`
- JSONL validity: `python3 -m json.tool .beads/vb-engine-yaml/proof-obligations.jsonl` is not valid for multi-line JSONL; use `python3 -c 'import json,sys; [json.loads(line) for line in open(sys.argv[1]) if line.strip()]' .beads/vb-engine-yaml/proof-obligations.jsonl`

## Waivers

- LEAN-WAIVER-001 only; see `lean-contract.md`. No waiver for TLA+ lifecycle/admission/recovery obligations or Verus-owned pure invariants.
