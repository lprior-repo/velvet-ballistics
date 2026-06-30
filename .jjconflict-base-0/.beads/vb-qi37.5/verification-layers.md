# Verification Layers: vb-qi37.5

## Boundary

- Verus-owned kernel: pure action idempotency decision table, compile/validate parity abstraction, certificate idempotency summary soundness, replay tracker monotonicity.
- TLA+ temporal model: validation/compile/certificate/admission/retry/replay lifecycle and duplicate/stale completion ordering.
- Theorem projection: no Lean/Aeneas/Hax kernel at State 3; use Verus first.
- Runtime shell: storage submission, runtime admission, hydration/replay, action dispatch, diagnostics, CLI rendering.
- External systems excluded from formal proof: Fjall internals, OS/filesystem, wall-clock, terminal UI.

## Layer Assignment

- PRE-001 -> static-scan + unit/integration contract completeness evidence.
- PRE-002 -> Verus decision-table precondition + Kani decision enumeration.
- PRE-003 -> Verus/Kani key metadata obligation + proptest over contract combinations.
- PRE-004 -> integration/admission tests + static scan for default-empty certificate construction.
- POST-001 -> Verus + Kani + integration tests.
- POST-002 -> Kani cross-crate parity + proptest table enumeration.
- POST-003 -> Verus certificate-summary soundness + integration certificate assertions.
- POST-004 -> TLA+ admission lifecycle + runtime integration tests + executable cargo-fuzz target `admission_fuzz` for accepted-artifact decode/admission hostile bytes.
- POST-005 -> TLA+ replay lifecycle + storage replay tests.
- POST-006 -> TLA+ duplicate/stale completion lifecycle + required runtime realization obligation `TEST-COMPLETION-015` proving same-ticket/key same-digest collapse and stale/conflicting rejection outcomes.
- INV-001 -> Verus + Kani parity + proptest table enumeration.
- INV-002 -> Verus + Kani + unit tests.
- INV-003 -> Verus + Kani + unit tests.
- INV-004 -> contract-verification review required before implementation because State 2 found validate/compile disagreement.
- INV-005 -> Verus + integration certificate assertions.
- INV-006 -> integration admission tests + static schema/gate-count assertion.
- INV-007 -> TLA+ + Verus replay tracker monotonicity + storage replay tests.
- INV-008 -> unit/integration tests + mutation testing for diagnostics branch strength.

## Verus Scope

- Rust targets:
  - `vb_validate::idempotency_contract::is_statically_idempotent_contract`
  - `vb_compile::check_idempotency_gates`
  - `vb_storage::admission::VerificationProof` construction from action contracts
  - `vb_storage::recovery::types::ActionReplayTracker`
- Spec/proof functions to create later:
  - `spec_idempotency_decision(contract) -> Decision`
  - `proof_decision_total_deterministic`
  - `proof_compile_validate_decision_parity`
  - `proof_certificate_summary_sound`
  - `proof_resolved_action_monotonic`
- Invariants: total deterministic decision; static unsafe side-effect rejection; accepted certificate fields only summarize accepted contracts; resolved action/step cannot become unresolved.
- Trusted boundary: validated construction of `ActionContract`, action IDs, contract maps, certificate field extraction, and replay tracker state imported from durable journal.
- Shell exclusions: I/O, Fjall/Postcard persistence, CLI text, async/runtime scheduling, wall-clock, allocation strategy.
- Evidence command when Verus files exist: `moon run :verify-proof`.

## TLA+ Scope

- Module/model path: `specs/idempotency_gate/IdempotencyGate.tla`
- Config: `specs/idempotency_gate/IdempotencyGate.cfg`
- Variables: `actions`, `contracts`, `decision`, `certificate`, `artifact`, `admission`, `journal`, `resolved`, `tickets`, `completions`.
- Actions: `Init`, `Validate`, `Compile`, `EmitCertificate`, `AdmitArtifact`, `ScheduleAction`, `CompleteAction`, `RetryAction`, `ReplayJournal`, `Reject`.
- Safety invariants: `NoRejectedEffectScheduled`, `CertificateSound`, `AdmissionRequiresEvidence`, `ResolvedActionMonotonic`, `DuplicateCompletionSameDigestOnly`.
- Temporal properties: `EventuallyAdmittedOrRejected`, `EventuallyReplaySettles`.
- Fairness/deadlock stance: weak fairness on internal progress/reject actions; terminal states stutter; no deadlock under bounded TLC model.
- Refinement boundary: Rust events refine model actions by action ID, run ID, contract decision, proof evidence, ticket key, and digest.
- Evidence command: `tlc -config specs/idempotency_gate/IdempotencyGate.cfg specs/idempotency_gate/IdempotencyGate.tla` after model creation, or `moon run :verify-proof` if proof lane owns TLA+.

## Kani / Proptest / Fuzz / Miri / Mutation Scope

- Kani: existing files `crates/vb_core/src/kani_idempotency_gates.rs`, `crates/vb_validate/src/kani_idempotency_contract.rs`, and `crates/vb_compile/src/kani_idempotency_parity.rs` must be updated or confirmed to cover final contract semantics.
- Proptest: generate all combinations of `Idempotency`, `SideEffect`, `RetrySafety`, contract presence, and certificate evidence to assert parity and no default-empty accepted proof.
- Fuzz/parser-codec: accepted artifact Postcard decode plus admission must run `cargo fuzz run admission_fuzz -- -runs=1000` and reject malformed/missing idempotency evidence without panic or runnable admission.
- Duplicate completion realization: runtime/action tests must prove `ActionError::CompletionAlreadyRecorded` is reserved for exact same ticket/key plus same digest, while stale or conflicting ticket/key/digest attempts return `ActionError::NonIdempotentReplayBlocked` before any duplicate external effect.
- Miri: run admission/replay tests under interpreter for UB/aliasing regression around storage/replay data structures.
- Mutation: mutation run must kill branches that skip unsafe retry rejection, certificate idempotency population, and admission idempotency proof checks.

## Performance, API, Release Provenance

- Performance: no speed claim. Downstream may run `moon ci`, but no p99/throughput threshold is contracted here.
- API compatibility: public APIs touched by State 2 (`compile_workflow_with_contracts`, `validate_with_contracts`, `submit_artifact_with_contracts`, `admit_artifact_run`) require source compile and integration evidence; no semver release claim.
- Release provenance: no new dependency expected; if a dependency is introduced later, formal-verifier must add `cargo deny`/supply-chain evidence.

## Waivers

- Lean/Aeneas/Hax: waived until proof review finds a theorem kernel beyond Verus.
- Runtime concurrency model checking with Loom/Shuttle: waived for State 3 because State 2 did not identify shared-memory concurrency; TLA+ covers lifecycle ordering and Miri/integration cover runtime shell. Expiry before implementation if async/threaded action dispatch is modified.
