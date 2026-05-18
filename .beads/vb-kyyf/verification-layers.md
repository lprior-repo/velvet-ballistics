# vb-kyyf Verification Layers

## Boundary
- Verus-owned kernel: pure normalization/comparison, journal signature checks, semantic-difference rejection.
- TLA+ temporal model: replay/recovery, side-effect non-reexecution, digest mismatch and corrupt-evidence stable failure, generated/IR parity state transitions.
- Runtime shell: storage/runtime/codegen/CLI public surfaces exercised by BDD tests and scoped cargo tests.
- External systems excluded from formal proof: concrete Fjall engine internals, filesystem scheduling, wall-clock time, process ids, compiler internals.

## Layer Assignment
- PRE-001 -> BDD + acceptance catalog + static review: accepted artifact or generated-subset fixture required.
- PRE-002 -> BDD + proptest-style fixture isolation checks: no shared store/run mutable state.
- PRE-003, INV-001 -> BDD + mutation/static review: public surface must be named and invoked.
- PRE-004, INV-002 -> Verus + Kani/proptest: normalization admits only allowed cold metadata.
- PRE-005, INV-005, POST-003 -> TLA+ + BDD + mutation: non-replay-safe side effects not re-executed.
- POST-001 -> BDD + Verus comparison kernel: isolated identical runs match.
- POST-002, INV-003 -> TLA+ + storage tests: persisted replay is reproducible and sequence well-formed.
- POST-004, INV-004 -> TLA+ + storage recovery tests + fuzz adjacency: corrupt/digest mismatch fails stably.
- POST-005, INV-006 -> BDD + codegen tests + Kani/proptest where pure comparison applies: generated/IR semantic parity for supported subset.
- POST-006, ERR-008 -> BDD + codegen subset rejection tests: unsupported generated subset fails closed.
- POST-006, INV-007, ERR-009 -> BDD runner/catalog tests: evidence artifact is present and traceable.
- Release gate -> `moon ci` after scoped tests pass; classify unrelated global failures separately.

## Verus Scope
- Proposed Rust target: normalized observation module introduced by implementation state, likely under `crates/workspace_tests` for test support or a reusable non-runtime evidence crate if one exists.
- Spec functions:
  - `spec_allowed_metadata_delta`
  - `spec_normalized_observation_eq`
  - `spec_journal_signature_well_formed`
- Proof functions:
  - `proof_normalization_rejects_semantic_delta`
  - `proof_normalized_equality_is_stable`
  - `proof_journal_signature_monotonic_contiguous`
- Invariants:
  - allowed metadata whitelist is exhaustive.
  - semantic fields are preserved exactly.
  - journal sequence signature has no gaps/duplicates.
- Trusted boundary: construction of raw public observations from CLI/storage/runtime shell; Verus only proves pure normalization/comparison once raw observations are validated into abstract values.
- Shell exclusions: CLI execution, Fjall I/O, filesystem paths, wall-clock time, generated code compilation, runtime action dispatch.
- Planned command after proof artifacts exist: `moon run :verify-proof` or a narrower Verus command selected by proof-planner when the proof file path exists.

## TLA+ Scope
- Module/model path: `verification/tla/VbKyyfReplayDeterminism.tla`
- Config path: `verification/tla/VbKyyfReplayDeterminism.cfg`
- Variables: `runs`, `store`, `observations`, `actionClass`, `sideEffectDispatches`, `status`, `generatedMode`.
- Actions: `Init`, `RunOnce`, `PersistEvidence`, `DropAndReopen`, `ReplayFromEvidence`, `ObserveViaPublicSurface`, `CorruptRecord`, `DetectDigestMismatch`, `ScheduleAction`, `CompleteAction`, `AttemptUnsafeReplay`, `CompareGeneratedAndIr`, `FailClosedUnsupportedGeneratedSubset`.
- Safety invariants: `JournalSequenceWellFormed`, `ReplayIsReproducible`, `DigestMismatchNeverContinues`, `BadEvidenceFailsStably`, `NoUnsafeSideEffectReexecution`, `GeneratedIrObservationParity`.
- Temporal properties: eventual terminal/blocked/failed observation under weak fairness; unsupported generated subset eventually fail-closes.
- Fairness/deadlock stance: weak fairness on replay/observe/digest detection; no unexpected deadlocks outside terminal states.
- Evidence command: `tlc -config verification/tla/VbKyyfReplayDeterminism.cfg verification/tla/VbKyyfReplayDeterminism.tla`.

## BDD and Test Evidence Scope
- `cargo test -p vb_storage --test replay_resume`
- `cargo test -p vb_storage --test recovery_bdd_tests`
- `cargo test -p vb_storage --test recovery_integration`
- `cargo test -p workspace_tests --test vb_kyyf_cross_run_determinism`
- `cargo test -p workspace_tests --test vb_hxm0_acceptance_catalog`
- `cargo test -p vb_codegen`
- `moon ci` for canonical release gate when scoped evidence is green.

## Fuzz/Property Scope
- Reuse existing fuzz adjacency for replay/decode/recovery input spaces where available: `fuzz/src/bin/replay_events.rs`, `fuzz/src/bin/recovery_decode.rs`, `fuzz/src/bin/recover_runtime_frame_seed_contract.rs`.
- Exact fuzz commands are deferred to proof-planner/formal-verifier because the current manifest does not prove runnable target names or timeout policy.

## Theorem Scope
- None required. Lean/Aeneas/Hax is waived in `lean-contract.md` unless reviewer escalates a small normalization uniqueness theorem.

## Static and Release Scope
- Static source scan: no production `unsafe`, `unwrap`, `expect`, `panic`, `todo`, unchecked indexing/slicing/casts/arithmetic introduced by later implementation.
- API compatibility: no public runtime/storage/codegen API breaking changes expected; if implementation changes public APIs, downstream must add API compatibility evidence.
- Performance: non-goal; no speed claim.

## Waivers
- WAIVE-THM-001: theorem kernel non-applicable; Verus+TLA+ cover relevant math.
- WAIVE-PERF-001: performance evidence non-applicable because bead adds correctness BDD/evidence contracts and makes no throughput claim.
