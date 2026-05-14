# Verification Layers: vb-gvmt

## Boundary
- Verus-owned kernel: pure generated execution model over slots, taints, journal capacity, resume payload validation, and deterministic taint checks.
- TLA+ temporal model: generated lifecycle and journal ordering parity for action/ask suspension/resume and finish.
- Theorem projection: none initially; Lean waiver candidate only.
- Runtime shell: `vb_codegen` emission, generated source compile, generated-vs-IR harness, action/ask fixture adapters.
- External systems excluded from formal proof: OS process execution, Fjall persistence, IPC framing, real action side effects.

## Layer Assignment
- PRE-001 -> generated subset validation tests + static scan.
- PRE-002 -> Verus capacity model + Kani/proptest bounded generated fixtures.
- PRE-003 -> static scan + compile-fail/trybuild generated-source fixtures.
- PRE-004 -> Fowler taint violation scenario + Verus taint monotonicity + mutation testing.
- PRE-005 -> Verus pure resume transition + Fowler invalid resume tests; TLA+ covers valid lifecycle ordering only in this revision.
- POST-001 -> generated-vs-IR semantic parity harness + proptest fixtures.
- POST-002 -> Fowler finish taint scenario + Verus slot/taint read invariant.
- POST-003 -> TLA+ action schedule ordering + Fowler exact journal scenario.
- POST-004 -> TLA+ action resume ordering + Verus no-mutation-before-validation + Fowler exact journal scenario.
- POST-005 -> TLA+ ask resume ordering + Fowler exact answer slot scenario.
- POST-006 -> TLA+ journal ordering + generated-vs-IR journal comparator.
- POST-007 -> Fowler `TaintViolation` scenario + Verus taint monotonicity + mutation testing.
- POST-008 -> Fowler step-budget scenario + Kani/proptest bounded budget exploration.
- POST-009 -> semantic parity tests replacing source-count-only acceptance.
- INV-001 -> Verus + proptest.
- INV-002 -> Verus + static scan + Kani.
- INV-003 -> Verus capacity arithmetic + Rust overflow pre-mutation tests; TLA+ covers modeled capacity error transitions only in this revision.
- INV-004 -> Verus taint lattice + Fowler tainted result scenarios.
- INV-005 -> Verus resume transition + Kani/Rust invalid-resume preservation tests; TLA+ invalid-resume preservation is not claimed in this revision.
- INV-006 -> TLA+ trace parity abstraction + generated-vs-IR tests.
- INV-007 -> compile-fail/unsupported primitive tests.

## Verus Scope
- Rust proof target: `.beads/vb-gvmt/proofs/generated_semantics_verus.rs`.
- Spec/proof functions covered:
  - `spec_join_taint_monotonic`
  - `proof_slot_write_preserves_parallel_taint`
  - `proof_checked_slot_access_no_panic`
  - `proof_journal_append_capacity_or_error`
  - `proof_resume_validates_identity_before_mutation`
  - `proof_no_contract_tainted_input_clean_output_rejected`
- Trusted boundary: validated `CompiledWorkflow`, bounded generated frame constructors, runtime-equivalent abstraction from generated concrete frame into proof model.
- Shell exclusions: emitted Rust string formatting, action executor I/O, ask external input, file/process execution, persistence.
- Exact passing command: `/home/lewis/.local/bin/verus --crate-type=lib .beads/vb-gvmt/proofs/generated_semantics_verus.rs`.
- Evidence: `verification results:: 6 verified, 0 errors`.
- Tooling caveat: bare `verus` resolved to a broken wrapper (`did not find a valid verusroot`), so the evidence command intentionally pins `/home/lewis/.local/bin/verus`.

## TLA+ Scope
- Module/model path: `.beads/vb-gvmt/specs/GeneratedParity.tla` with `.beads/vb-gvmt/specs/GeneratedParity.cfg`.
- Variables: `scenario`, `phase`, `pending`, `budget`, `genTrace`, `irTrace`, `terminal`, `error`.
- Actions: `Init`, `DeterministicSlotWrite`, `DoSuspend`, `DoResumeValid`, `AskSuspend`, `AskResumeValid`, `FinishRun`, `BudgetExhaust`, `JournalCapacityFail`, `TerminalOrErrorStutter`.
- Claimed safety invariants: `TraceParity`, `JournalBounded`, `ActionScheduleBeforeComplete`, `SlotWrittenBeforeActionCompleted`, `AskAnswerBeforeAdvance`, `RunFinishedLast`, `NoPendingWhenTerminal`.
- Non-claimed placeholders in the current executable model: `SlotTaintParallel`, `JournalAppendOnly`, `NoMutationOnInvalidResume`, and `NoDropOnJournalFull` are trivial `TRUE` definitions in `GeneratedParity.tla`; they are retained only as traceability markers, are intentionally omitted from `GeneratedParity.cfg`, and must not be cited as TLA+ proof evidence.
- Temporal properties: `EventuallyTerminalOrSuspended`, `ScheduledEventuallyCompletable`, `AskEventuallyAnswerable`.
- Fairness/deadlock stance: weak fairness on continuously enabled valid resume; no unexpected deadlock except terminal or explicit external suspension.
- Refinement boundary: normalized generated observations equal normalized IR observations for modeled fixtures.
- Exact passing command: `tlc -config .beads/vb-gvmt/specs/GeneratedParity.cfg .beads/vb-gvmt/specs/GeneratedParity.tla`.
- Evidence: TLC2 2.19 reported `Model checking completed. No error has been found.`, 17 states generated, 13 distinct states, depth 4.

## Test and Dynamic Verification Scope
- Fowler scenarios in `martin-fowler-tests.md` are the minimum executable behavior contract.
- Generated semantic parity harness must compare exact values/errors/events, including `SlotWritten`, `ActionScheduled`, `ActionCompleted`, `RunFinished`, and `AskAnswered` if runtime has it.
- Property tests should generate small supported workflows over SetConst, Copy, EvalExpr, Do, Ask, AskResume, Jump, Choose, and Finish, bounded by slot/step/journal capacities.
- Mutation testing should kill mutants that remove taint propagation, reorder journal events, skip resume identity validation, skip budget checks, or downgrade errors.

## Static and Compile Layers
- Static scan: generated and codegen source must contain no unsafe, unwrap, expect, panic, todo, unimplemented, dbg, unchecked indexing/slicing/casts/arithmetic, JSON, YAML runtime, HTTP.
- Compile-fail/trybuild: malformed generated constructs and unsupported primitives must fail with typed compile/validation errors, not silently emit wrong code.
- Repository canonical gate is `moon ci`; latest direct run passed with 19 completed tasks (1 cached) and nextest 8276/8276 tests passed.

## Waivers / Blockers
- DEFERRED-MUTATION-001: scoped cargo-mutants run produced 35/35 unviable mutants, so mutation adequacy is not proven by this session.
- WAIVER-CANDIDATE-001: Lean not used unless reviewer requires theorem kernel beyond Verus.
- WAIVER-CANDIDATE-002: distinct `AskAnswered` journal event no longer requires waiver in generated tests; generated journal emits and tests assert `AskAnswered` where applicable.
