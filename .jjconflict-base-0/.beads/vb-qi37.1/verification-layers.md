# Verification Layers: vb-qi37.1

## Boundary
- Verus-owned kernel: `verification/verus/recovery_verification.rs` plus any added recovery-specific Verus surfaces for unsupported-state rejection, no silent empty-frame success, and digest-mode truthfulness.
- TLA+ temporal model: planned `verification/tla/RecoveryHydration.tla` for crash/restart and hydration lifecycle.
- Theorem projection: none for State 3.
- Runtime shell: `vb_storage` Fjall journal access, snapshot decode, runtime frame mutation, CLI/integration evidence.
- External systems excluded from formal proof: OS crash semantics, Fjall internals, wall-clock scheduling.

## Layer assignment
- PRE-001 -> TLA+ model bounds + integration crash tests.
- PRE-002 -> TLA+ `NoMixedRunRecovery` + proptest mixed-run event streams.
- PRE-003 -> TLA+ `SnapshotThenTailOnly` + unit tests for snapshot run/sequence rejection.
- PRE-004 -> direct Verus digest-input precondition obligation `VERUS-PRE-004` + unit tests for production-visible workflow-source and compiled-IR digest mismatch variants.
- PRE-005 -> Verus unsupported-state obligation + runtime boundary tests.
- POST-001 -> unit/integration tests for non-empty summaries + proptest event folds.
- POST-002 -> TLA+ completeness/unsupported invariant + integration crash restart evidence.
- POST-003 -> Verus no-empty-frame obligation + runtime/storage tests.
- POST-004 -> TLA+ snapshot-tail ordering + integration snapshot+journal replay.
- POST-005 -> Verus/runtime boundary obligation + test for `UnsupportedFullRecoveryHydration`.
- POST-006 -> Verus digest-mode obligation + unit/fault-injection digest tests for workflow-source and compiled-IR mismatch only; action ABI and policy digest families are waived optional downstream lanes until production exposes inputs/lookups/checks.
- POST-007 -> integration + BDD/fault-injection crash-before-ack/crash-after-ack evidence.
- INV-001 -> TLA+ monotonic sequence + proptest corrupt ordering.
- INV-002 -> Verus/proptest no fabricated slots/taint.
- INV-003 -> Verus unsupported reject lattice.
- INV-004 -> TLA+ no-YAML-recovery abstraction + integration evidence over accepted artifacts.
- INV-005 -> non-vacuous Verus typed-error propagation/refinement model + static scan + fault-injection typed error propagation. A proof whose postcondition merely restates the antecedent is invalid evidence.
- INV-006 -> TLA+ terminal consistency + unit tests.
- ERR-001 -> typed-error scenario + integration/manual evidence for journal failure propagation.
- ERR-002 -> Verus digest algebra + typed-error scenario for workflow source digest mismatch.
- ERR-003 -> Verus digest algebra + typed-error scenario for compiled IR digest mismatch.
- ERR-004 -> waiver for this bead; downstream Verus/static/test evidence required only after production action ABI digest surface exists.
- ERR-005 -> waiver for this bead; downstream Verus/static/test evidence required only after production policy digest surface exists.
- ERR-006 -> TLA+ mixed-run/order invariants + proptest typed-error scenario for replay divergence.
- ERR-007 -> TLA+ no-data fail-closed branch + typed-error scenario for no recovery data.
- ERR-008 -> fuzz/property/manual evidence for corrupt snapshot decoding typed error.
- ERR-009 -> TLA+ terminal consistency + typed-error scenario for terminal mismatch.
- ERR-010 -> Verus/Kani/proptest dimension bound obligation + typed-error scenario for overflow.
- ERR-011 -> Verus unsupported-state reject obligation + runtime typed-error scenario.
- ERR-012 -> Verus summary-only boundary obligation + runtime typed-error scenario.

## Verus scope
- Rust target: `vb_runtime::recovery::reject_unsupported_live_frame_state`, `vb_runtime::recovery::RuntimeRecoveryBoundary::hydrate_run_frame`, `vb_storage::recovery::verify_digests`.
- Existing proof target: `verification/verus/recovery_verification.rs`.
- Spec/proof functions: `spec_reject_unsupported`, `proof_reject_unsupported_slot_taint_alone`, `proof_reject_unsupported_pending_actions_no_bypass`, `proof_workflow_source_mismatch_detected`, `proof_compiled_ir_mismatch_detected`, and a non-vacuous `proof_typed_recovery_error_not_silently_discarded`; additional no-empty-frame proof surface may be required.
- Invariants: unsupported flags imply rejection; summary-only cannot hydrate; workflow-source and compiled-IR mismatches cannot pass in their requested digest modes; typed errors are preserved or explicitly refined across pure recovery/runtime decision boundaries.
- Trusted boundary: mapping from concrete Rust events/frame seeds into Verus spec records.
- Shell exclusions: Fjall I/O, runtime allocation, byte decoding, crash mechanics.
- Evidence command: `verus verification/verus/recovery_verification.rs`.

## TLA+ scope
- Module/model path: planned `verification/tla/RecoveryHydration.tla`.
- Variables: runs, headers, journal, snapshots, digests, pc, slots, taint, step states, pending actions, waits, asks, retries, collect state, terminal, hydration, errors, ack state.
- Actions: Init, PersistHeader, AppendEvent, PersistSnapshot, AcknowledgeRun, Crash, Restart, VerifyDigest, RecoverSummary, RecoverFrameSeed, HydrateRuntimeFrame, FailClosed.
- Safety invariants: JournalSeqMonotonic, NoMixedRunRecovery, NoSilentEmptyFrame, NoFabricatedSlotOrTaint, UnsupportedRejectsHydration, TerminalConsistent.
- Temporal properties: RestartEventuallyRecoversOrFailsClosed.
- Fairness/deadlock stance: weak fairness on recovery actions; deadlock freedom required for non-terminal states.
- Refinement boundary: Rust journal/snapshot/recovery products refine model events and states by run id and sequence.
- Evidence command after proof-writer creates model: `tlc -config verification/tla/RecoveryHydration.cfg verification/tla/RecoveryHydration.tla`.

## Test and shell evidence scope
- Focused storage tests: `cargo test -p vb_storage --test recovery_integration`.
- Focused storage module tests: `cargo test -p vb_storage recovery::`.
- Runtime recovery boundary tests: `cargo test -p vb_runtime recovery`.
- Workspace recovery contract tests: `cargo test -p velvet-ballistics-workspace vb_qi37_1_1`.
- Canonical gate after implementation: `moon ci`.

## Waivers and blockers
- No State 3 waiver for TLA+ ownership. The executable command is `tlc -config verification/tla/RecoveryHydration.cfg verification/tla/RecoveryHydration.tla`; State 5 owns any model repair needed to satisfy the clause-specific obligations.
- No State 3 waiver for required Verus-owned clauses. The executable command is `verus verification/verus/recovery_verification.rs`; State 5 owns adding or repairing named proof surfaces for POST-003, INV-002, INV-003, INV-005, workflow/IR digest, and typed dimension/error algebra.
- WAIVER `ERR-004` / `PO-021`: owner `rust-contract State 3 repair`; reason `DigestCheck::Full production code currently has no action ABI digest input, lookup, or comparison path, so a required Verus proof would certify an abstract model not linked to production`; limitation `does not prove action ABI drift detection`; compensating evidence `POST-006 remains required for workflow-source and compiled-IR digest families; STATIC-SCAN-001 remains required for recovery source quality`; expiry/follow-up `promote to required in the bead that adds action ABI digest fields or lookup/check behavior`.
- WAIVER `ERR-005` / `PO-022`: owner `rust-contract State 3 repair`; reason `DigestCheck::Full production code currently has no policy digest input, lookup, or comparison path, so a required Verus proof would certify an abstract model not linked to production`; limitation `does not prove policy drift detection`; compensating evidence `POST-006 remains required for workflow-source and compiled-IR digest families; STATIC-SCAN-001 remains required for recovery source quality`; expiry/follow-up `promote to required in the bead that adds policy digest fields or lookup/check behavior`.
- WAIVER `PO-033`: owner `proof-planner State 4, ratified by State 3 repair`; reason `Kani would duplicate TLA+ bounded protocol coverage and Verus Rust-local invariant coverage for this bead`; limitation `no separate bounded Rust state-machine harness`; compensating evidence `PO-001 through PO-017 where applicable plus PO-031/PO-032`; expiry/follow-up `promote if implementation adds numeric/indexing state transitions not covered by Verus or TLA+`.
- WAIVER `PO-034`: owner `proof-planner State 4, ratified by State 3 repair`; reason `Flux annotation surface is not present and Verus is the selected refinement lane`; limitation `no independent Flux refinement check`; compensating evidence `VERUS-PRE-005, VERUS-INV-002, VERUS-INV-003`; expiry/follow-up `promote if Verus cannot express the needed refinement or Flux annotations are introduced`.
- WAIVER `PO-035`: owner `proof-planner State 4, ratified by State 3 repair`; reason `scoped recovery work is persisted crash/restart ordering with unsafe forbidden and no new concurrency primitive risk`; limitation `no Loom interleaving or Miri UB lane`; compensating evidence `TLA-POST-007, STATIC-SCAN-001, moon ci`; expiry/follow-up `promote if implementation adds unsafe, raw pointers, atomics, locks, spawned tasks, or cancellation-sensitive concurrency`.
- WAIVER `PO-036`: owner `proof-planner State 4 attempt 4, ratified by State 3 schema repair`; reason `no distinct fuzz, theorem-kernel, or dependency-specific trigger remains after TLA+/Verus/proptest/integration assignment and empty dependency scope`; limitation `no fuzz target, theorem-kernel proof, or dependency-specific audit/vet lane is planned for vb-qi37.1; residual malformed byte-level parser/security, theorem-extraction, and supply-chain risks are outside this bead unless later implementation changes introduce them`; compensating evidence `PO-001, PO-005, PO-011, PO-014, PO-031, PO-032`; expiry/follow-up `promote if proof-reviewer, black-hat review, or implementation changes introduce byte-level parser/security risk, theorem-owned proof gaps, or dependency file changes`.

## State 3 repair completion evidence
- Isolation: all edits are confined to `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1/.beads/vb-qi37.1/`; production code is not edited.
- JSONL evidence command required after repair: `jq -c . .beads/vb-qi37.1/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-qi37.1/traceability-matrix.jsonl >/dev/null && jq -c . .beads/vb-qi37.1/proof-obligations.planned.jsonl >/dev/null`.

## State 3 schema repair completion evidence
- `PRE-004` has a direct `proof-obligations.jsonl` row: `VERUS-PRE-004`.
- Contract-time waiver rows in `proof-obligations.jsonl` and `proof-obligations.planned.jsonl` retain `status: "planned"` while preserving `required:false` and explicit waiver metadata.
- `PO-036` waiver metadata includes owner, reason, limitation, compensating evidence, and follow-up trigger.
- Validation passed with valid JSONL for `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, and `traceability-matrix.jsonl`; schema helper reported `obligations 31 planned 37 trace 30 clauses 30`, `missing []`, `nonplanned_obs []`, `nonplanned_plan []`, and `bad_waivers []`.
