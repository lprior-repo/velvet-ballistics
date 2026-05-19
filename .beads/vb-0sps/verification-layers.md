# Verification Layers - vb-0sps State 3 Repair

## Boundary

- Verus-owned kernel after adapters exist: pure `ObservedRun`, `ObservedEvent`, `NormalizedError`, and comparison/refinement adapters.
- TLA+ temporal model: separate IR/generated machines, block/resume, event ordering, fail-closed unsupported behavior, bounded overflow/error states, and divergence sanity.
- Theorem projection: none currently required.
- Runtime shell: BDD test target, cargo commands, generated source compile/run behavior.
- External systems excluded from formal proof: rustc process details, filesystem temp dirs, timers beyond symbolic inputs, external action side effects.

## Layer Assignment

- PRE-001 -> focused BDD fixture construction obligation.
- PRE-002 -> focused BDD assertion around `validate_generated_subset`.
- PRE-003 -> BDD initial-state equality plus temporary Verus waiver until adapters exist.
- PRE-004 -> TLA+ supplied-resume-input obligation plus BDD resume scenarios.
- PRE-005 -> BDD unsupported fail-closed scenario plus TLA+ unsupported reject/source-emission obligation.
- POST-001 -> BDD exact terminal assertions plus temporary Verus waiver and proptest/focused single-field checks.
- POST-002 -> BDD typed error assertions plus temporary Verus waiver.
- POST-003 -> TLA+ suspension config plus BDD suspension scenarios.
- POST-004 -> TLA+ resume config plus BDD resume scenarios.
- POST-005 -> TLA+ journal field parity plus BDD/proptest field checks.
- POST-006 -> BDD fail-closed assertion plus TLA+ unsupported reject/source-emission obligation.
- POST-007 -> acceptance catalog test.
- INV-001 -> explicit BDD/manual oracle contract obligation.
- INV-002 -> temporary Verus waiver plus proptest/focused structured comparator checks; debug strings forbidden.
- INV-003 -> temporary Verus waiver plus BDD/proptest taint parity.
- INV-004 -> TLA+ legal StepState transition obligation for both machines.
- INV-005 -> TLA+ no-advance-past-suspension obligation.
- INV-006 -> TLA+ and BDD fail-closed unsupported obligation.
- INV-007 -> manual contract review; no performance/release obligations generated.

## Exact Focused Evidence Commands

- `cargo test -p velvet-ballastics-workspace-tests --test vb_0sps_generated_ir_parity_bdd`
- `cargo test -p velvet-ballastics-workspace-tests --test vb_hxm0_acceptance_catalog`
- `cargo test -p vb_codegen --all-features`
- `cargo test -p vb_core --all-features` only if BDD fixtures expose or alter core engine fixture helpers.
- TLA commands listed in `tla-spec.md` must be run by State 5 after proof artifacts are repaired. Timeout exit `124` is failure evidence, not waiver evidence.

## Verus Scope and Waiver

- Future target: concrete public/test-support comparison adapters, not private generated internals and not proof-only enums.
- Future spec functions: `spec_observed_run_equal`, `spec_normalized_error_equal`, `spec_event_sequence_equal`, `spec_initial_observation_equal`.
- Future proof functions: `proof_compare_observed_runs_sound`, `proof_normalized_error_mapping_total`, `proof_event_sequence_compare_ordered`, `proof_initial_inputs_preserved`.
- Invariants: symmetric equality, field-total comparison, no ignored slot/taint/step/event/error fields, mismatch returns a typed `ParityError`.
- Trusted boundary: validated construction of `ObservedRun` from public IR/generated observations.
- Shell exclusions: executing workflows, generated source compilation, filesystem, rustc, timers, action side effects.
- Waiver metadata: Owner `State 5 proof-writer plus State 6 contract-verification reviewer`; reason `no concrete adapter exec functions exist in State 3`; limitation `Rust-local adapter correctness is not formally proven`; expiry/follow-up `expires when adapters exist or before State 6 approval if adapters already exist`; compensating evidence `BDD exact assertions, proptest/focused single-field mismatch checks, static review forbidding debug-string equality, and TLA+ sequence/refinement proof`.

## TLA+ Scope

- Module path: `verification/tla/generated_ir_parity/GeneratedIrParity.tla`.
- Config paths: `GeneratedIrParity_success.cfg`, `GeneratedIrParity_suspension_resume.cfg`, `GeneratedIrParity_typed_error.cfg`, `GeneratedIrParity_unsupported_reject.cfg`, `GeneratedIrParity_divergence_sanity.cfg` under `verification/tla/generated_ir_parity/`.
- Variables: `ir_pc`, `gen_pc`, `ir_slots`, `gen_slots`, `ir_taints`, `gen_taints`, `ir_steps`, `gen_steps`, `ir_journal`, `gen_journal`, `ir_blocked`, `gen_blocked`, `resumeQueue`, `ir_terminal`, `gen_terminal`, `ir_error`, `gen_error`, `unsupported`, `sourceEmitted`.
- Actions: separate `Ir*`, `Gen*`, and `Env*` relations as listed in `tla-spec.md`; combined `Next` must not prove equality by construction.
- Safety invariants: observation refinement, terminal observable equality, blocked metadata equality, full journal prefix equality, no advance past suspension, unsupported no source emission, valid StepState transitions for both machines.
- Temporal properties: eventually matching terminal/blocked/error/reject; resume eventually progresses under supplied matching resume input.
- Fairness/deadlock stance: weak fairness on enabled machine/resume actions; no deadlock outside explicit terminal, blocked-without-resume, typed error, unsupported reject, or bounded overflow/error.
- Evidence commands: exact `timeout 120 tlc ...` split-config commands in `tla-spec.md`.

## Kani / Proptest / Static Layers

- Kani: optional scoped harness for generated runtime bounds only; no hardcoded Kani shapes, use `kani::Arbitrary` or safe bounded generators if added later.
- Proptest/focused cases: observed-run comparator and single-field mismatch cases; complements BDD and waiver compensation, not a substitute for Verus after adapters exist.
- Static scan: touched production/test-support code must remain free of `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/slicing/casts/arithmetic where repository policy applies.

## Performance / Release Provenance

- Non-goal for this bead. No performance, maxperf, PGO, generated-vs-IR ratio, release-provenance, or generated-mode readiness claim is contracted.

## Waivers

- `WAIVER-VERUS-ADAPTERS-001`: applies to `PRE-003`, `POST-001`, `POST-002`, `INV-002`, and `INV-003` until real adapter exec functions exist; metadata above.
- `THM-WAIVER-001`: Lean/Aeneas/Hax deferred per `lean-contract.md`.
- Whole-fleet verification waived for bead scope; focused commands only.
- No TLA+ waiver is granted by State 3.
