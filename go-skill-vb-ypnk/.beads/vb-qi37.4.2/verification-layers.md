<<<<<<< HEAD
# Verification Layers: vb-qi37.4.2

## Boundary

- **Verus-owned kernel**: Taint lattice laws (INV-001 to INV-006), StepBudget monotonicity (INV-008), RunFrame dimension immutability (INV-007), StepState transition matrix (INV-010), resource budget arithmetic (VB-CORE-RESOURCE-001 to VB-CORE-RESOURCE-003), EngineSignal Finished canonical form (INV-010).
- **TLA+ temporal model**: Journal ordering (INV-013), replay safety (VB-REPLAY-001 to VB-REPLAY-007), concurrency (VB-CONC-001 to VB-CONC-005), retry FSM (VB-REPLAY-004, VB-REPLAY-005), capability lifecycle (VB-REPLAY-006, VB-REPLAY-007).
- **Theorem projection**: None — all Rust-local pure properties are in Verus.
- **Runtime shell**: I/O (journal write, Fjall), async scheduling, IPC transport, UI rendering.
- **External systems excluded from formal proof**: Fjall compaction internals, OS scheduler, network transport, hardware memory model.

---

## Layer Assignment by Contract Clause

### Taint Lattice (INV-001 to INV-006)

| Clause | Layer | Tool | Evidence |
|--------|-------|------|----------|
| INV-001 (associative) | `verus` L4 | `verus verification/verus/taint_lattice.rs` | Verus proof fn |
| INV-002 (commutative) | `verus` L4 | `verus verification/verus/taint_lattice.rs` | Verus proof fn |
| INV-003 (idempotent) | `verus` L4 | `verus verification/verus/taint_lattice.rs` | Verus proof fn |
| INV-004 (identity Clean) | `verus` L4 | `verus verification/verus/taint_lattice.rs` | Verus proof fn |
| INV-005 (Secret no downgrade) | `verus` L4 + `kani` L3 | `verus` + `cargo kani --harness kani_taint_lattice` | Verus proof fn + Kani bounded check |
| INV-006 (DerivedFromSecret no downgrade) | `verus` L4 | `verus verification/verus/taint_lattice.rs` | Verus proof fn |
| VB-CORE-TAINT-006 (join in EvalExpr) | `kani` L3 | `cargo kani --harness kani_taint_propagation` | Kani bounded model check |
| All taint | `proptest` L1 | `cargo nextest run -p vb_core taint_` | Property tests |

### StepState Machine (INV-010, VB-CORE-STATE-001, VB-CORE-STATE-002, VB-CORE-STATE-003)

| Clause | Layer | Tool | Evidence |
|--------|-------|------|----------|
| INV-010 (canonical Finished) | `verus` L4 | `verus verification/verus/signals_invariant.rs` | Verus invariant proof |
| VB-CORE-STATE-001 (valid transitions) | `verus` L4 + `kani` L3 | `verus verification/verus/step_state_machine.rs` + `cargo kani --harness kani_step_state` | Verus proof + Kani bounded check |
| VB-CORE-STATE-002 (idempotent re-mark) | `kani` L3 | `cargo kani --harness kani_step_state` | Kani bounded check |
| VB-CORE-STATE-003 (invalid transition error) | `proptest` L1 + `unit` | `cargo nextest run -p vb_core step_state_invalid` | Unit tests |
| StepState all | `proptest` L1 | `cargo nextest run -p vb_core step_state` | Property tests |

### StepBudget (INV-008, VB-CORE-BUDGET-001, VB-CORE-BUDGET-002, VB-CORE-BUDGET-003)

| Clause | Layer | Tool | Evidence |
|--------|-------|------|----------|
| INV-008 (monotonic, no underflow) | `verus` L4 | `verus verification/verus/step_budget.rs` | Verus invariant proof |
| VB-CORE-BUDGET-001 (budget 0 → zero transitions) | `kani` L3 | `cargo kani --harness kani_step_budget_zero` | Kani bounded check |
| VB-CORE-BUDGET-002 (budget 1 → exactly one) | `kani` L3 | `cargo kani --harness kani_step_budget_one` | Kani bounded check |
| VB-CORE-BUDGET-003 (try_take never underflows) | `verus` L4 + `kani` L3 | `verus verification/verus/step_budget.rs` + `cargo kani --harness kani_step_budget` | Verus deductive + Kani bounded |
| StepBudget all | `proptest` L1 | `cargo nextest run -p vb_core step_budget` | Property tests |

### RunFrame (INV-007, PRE-001, POST-001)

| Clause | Layer | Tool | Evidence |
|--------|-------|------|----------|
| INV-007 (dimension immutability) | `verus` L4 | `verus verification/verus/run_frame_invariant.rs` | `VB-CORE-RUNFRAME-003` Verus representation invariant |
| PRE-001 (step_count > 0, first_step < step_count) | `verus` L4 | `verus verification/verus/run_frame_invariant.rs` | `VB-CORE-RUNFRAME-001` Verus preconditions |
| POST-001 (constructor initializes dimensions/default states/default taint) | `verus` L4 | `verus verification/verus/run_frame_invariant.rs` | `VB-CORE-RUNFRAME-002` Verus postconditions |
| PRE-001 | `kani` L3 | `cargo kani --harness kani_frame_construction` | Kani bounded check |
| PRE-002 (entry < node_count) | `kani` L3 | `cargo kani --harness kani_budget_compute` | Kani bounded check |

### Resource Budget Arithmetic (VB-CORE-RESOURCE-001 to VB-CORE-RESOURCE-004)

| Clause | Layer | Tool | Evidence |
|--------|-------|------|----------|
| VB-CORE-RESOURCE-001 (sequential saturating add is policy-bounded and non-wrapping) | `verus` L4 | `verus verification/verus/resource_budget.rs` | Verus arithmetic proof |
| VB-CORE-RESOURCE-002 (branch max safe) | `verus` L4 | `verus verification/verus/resource_budget.rs` | Verus arithmetic proof |
| VB-CORE-RESOURCE-003 (loop saturating multiply is policy-bounded and non-wrapping) | `verus` L4 | `verus verification/verus/resource_budget.rs` | Verus arithmetic proof |
| VB-CORE-RESOURCE-004 (budget ≤ policy) | `kani` L3 + `proptest` L1 | `cargo kani --harness kani_resource_budget_bounded` + `cargo nextest run -p vb_core resource_policy` | Kani bounded + property tests |
| Resource budget all | `proptest` L1 | `cargo nextest run -p vb_core resource_budget` | Property tests |

### EngineSignal (INV-010, VB-CORE-SIGNAL-001)

| Clause | Layer | Tool | Evidence |
|--------|-------|------|----------|
| INV-010 (Finished carries Taint) | `verus` L4 | `verus verification/verus/signals_invariant.rs` | Verus invariant proof |
| VB-CORE-SIGNAL-001 (canonical form) | `proptest` L1 + `fuzz` L2 | `cargo nextest run -p vb_core finish_signal` + `cargo fuzz run expr_eval` | Unit tests + fuzz |

### Numeric ID Index Safety (INV-009, VB-CORE-IDX-001, VB-CORE-IDX-002)

| Clause | Layer | Tool | Evidence |
|--------|-------|------|----------|
| INV-009 (checked access only) | `kani` L3 + `static-scan` L0 | `cargo kani --harness kani_index_access` + `cargo xtask forbidden-scan --pattern as_usize_index --crate vb_core` | Kani + forbidden-scan |
| VB-CORE-IDX-001 (bounds validated) | `kani` L3 | `cargo kani --harness kani_index_access` | Kani bounded check |
| VB-CORE-IDX-002 (no raw as_usize index) | `static-scan` L0 | `cargo xtask forbidden-scan --pattern as_usize_index --crate vb_core` | Forbidden pattern scan |

### IPC Decoder (INV-011, VB-IPC-DECODE-001 to VB-IPC-DECODE-003)

| Clause | Layer | Tool | Evidence |
|--------|-------|------|----------|
| INV-011 (reject before alloc) | `kani` L3 + `fuzz` L2 | `cargo kani --harness kani_ipc_header` + `cargo fuzz run ipc_decode` | Kani bounded + fuzz |
| VB-IPC-DECODE-001 (header_len ≥ 60) | `kani` L3 | `cargo kani --harness kani_ipc_header` | Kani bounded check |
| VB-IPC-DECODE-002 (payload_len ≤ MAX) | `kani` L3 | `cargo kani --harness kani_ipc_header_rejects_oversize` | Kani bounded check |
| VB-IPC-DECODE-003 (magic valid) | `kani` L3 | `cargo kani --harness kani_ipc_header` | Kani bounded check |
| IPC all | `fuzz` L2 | `cargo fuzz run ipc_decode` | Fuzz 24h corpus |

### Record Decoder (INV-012, VB-STORAGE-DECODE-001 to VB-STORAGE-DECODE-006)

| Clause | Layer | Tool | Evidence |
|--------|-------|------|----------|
| INV-012 (validate before alloc) | `kani` L3 + `fuzz` L2 | `cargo kani --harness kani_record_magic` + `cargo fuzz run record_decode` | Kani + fuzz |
| VB-STORAGE-DECODE-001 (magic) | `kani` L3 | `cargo kani --harness kani_record_magic` | Kani bounded check |
| VB-STORAGE-DECODE-002 (schema) | `kani` L3 | `cargo kani --harness kani_record_schema` | Kani bounded check |
| VB-STORAGE-DECODE-003 (kind) | `kani` L3 | `cargo kani --harness kani_record_kind` | Kani bounded check |
| VB-STORAGE-DECODE-004 (payload_len) | `kani` L3 | `cargo kani --harness kani_record_payload_len` | Kani bounded check |
| VB-STORAGE-DECODE-005 (CRC/BLAKE3) | `kani` L3 | `cargo kani --harness kani_record_crc` | Kani bounded check |
| VB-STORAGE-DECODE-006 (full pipeline) | `fuzz` L2 | `cargo fuzz run record_decode` | Fuzz corpus 24h |
| Storage all | `crash-lab` L2 | `crash-lab vb_storage_record_decode` | Fault injection |

### Journal Ordering (INV-013, VB-REPLAY-001 to VB-REPLAY-007)

| Clause | Layer | Tool | Evidence |
|--------|-------|------|----------|
| INV-013 (journal before dispatch) | `tla-plus` L3 | `tlc -config verification/tla/LifecycleJournal.cfg verification/tla/LifecycleJournal.tla` | TLC model check |
| VB-REPLAY-001 (journal entry validity) | `tla-plus` L3 | `tlc -config verification/tla/LifecycleJournal.cfg verification/tla/LifecycleJournal.tla` | TLC invariant |
| VB-REPLAY-002 (replay order) | `tla-plus` L3 | `tlc -config verification/tla/LifecycleJournal.cfg verification/tla/LifecycleJournal.tla` | TLC invariant |
| VB-REPLAY-003 (no duplicate replay) | `tla-plus` L3 | `tlc -config verification/tla/LifecycleJournal.cfg verification/tla/LifecycleJournal.tla` | TLC invariant |
| VB-REPLAY-004 (retry FSM valid) | `tla-plus` L3 | `tlc -config verification/tla/RetryFSM.cfg verification/tla/RetryFSM.tla` | TLC model check |
| VB-REPLAY-005 (max attempts respected) | `tla-plus` L3 | `tlc -config verification/tla/RetryFSM.cfg verification/tla/RetryFSM.tla` | TLC invariant |
| VB-REPLAY-006 (capability unique owner) | `tla-plus` L3 | `tlc -config verification/tla/CapabilityLifecycle.cfg verification/tla/CapabilityLifecycle.tla` | TLC invariant |
| VB-REPLAY-007 (capability valid access) | `tla-plus` L3 | `tlc -config verification/tla/CapabilityLifecycle.cfg verification/tla/CapabilityLifecycle.tla` | TLC invariant |
| Replay all | `proptest` L1 + `integration` | `cargo nextest run -p vb_runtime replay` | Integration tests |

### Concurrency (INV-015, VB-CONC-001 to VB-CONC-005)

| Clause | Layer | Tool | Evidence |
|--------|-------|------|----------|
| INV-015 (single shard owner) | `tla-plus` L3 + `loom` L3 | `tlc -config verification/tla/ConcurrencyControl.cfg verification/tla/ConcurrencyControl.tla` + `cargo loom --test concurrency` | TLC + Loom interleaving |
| VB-CONC-001 (frame pool bounded) | `tla-plus` L3 | `tlc -config verification/tla/ConcurrencyControl.cfg verification/tla/ConcurrencyControl.tla` | TLC invariant |
| VB-CONC-002 (no cross-shard alias) | `tla-plus` L3 | `tlc -config verification/tla/ConcurrencyControl.cfg verification/tla/ConcurrencyControl.tla` | TLC invariant |
| VB-CONC-003 (no deadlock on locks) | `tla-plus` L3 + `loom` L3 | `tlc` + `cargo loom` | TLC + Loom |
| VB-CONC-004 (frame pool liveness) | `tla-plus` L3 | `tlc -config verification/tla/ConcurrencyControl.cfg verification/tla/ConcurrencyControl.tla` | TLC temporal property |
| VB-CONC-005 (lock no starvation) | `tla-plus` L3 + `loom` L3 | `tlc` + `cargo loom` | TLC + Loom |
| Concurrency all | `shuttle` L3 | `cargo shuttle test concurrency` | Shuttle schedule exploration |

### Idempotency Key Well-Formedness (INV-014)

| Clause | Layer | Tool | Evidence |
|--------|-------|------|----------|
| INV-014 (idempotency key well-formedness) | `proptest` L1 | `cargo nextest run -p vb_runtime idempotency_key_well_formed` | Property tests covering accepted/rejected key shapes |

### Expression Evaluator (VB-EXPR-001 to VB-EXPR-003)

| Clause | Layer | Tool | Evidence |
|--------|-------|------|----------|
| VB-EXPR-001 (AST/bytecode equivalence) | `differential` L1 + `proptest` L1 | `cargo nextest run -p vb_expr ast_bytecode_equiv` | Differential tests |
| VB-EXPR-002 (stack depth ≤ MAX_EXPR_STACK) | `kani` L3 | `cargo kani --harness kani_expr_stack` | Kani bounded check |
| VB-EXPR-003 (f64 support correct) | `proptest` L1 + `fuzz` L2 | `cargo nextest run -p vb_expr f64_` + `cargo fuzz run expr_eval` | Property + fuzz |

### WholeWorkflowBudget (POST-006, VB-CORE-RESOURCE-004)

| Clause | Layer | Tool | Evidence |
|--------|-------|------|----------|
| POST-006 (budget ≤ policy) | `kani` L3 | `cargo kani --harness kani_resource_budget_bounded` | Kani bounded check |
| VB-CORE-RESOURCE-004 (computed ≤ policy) | `kani` L3 + `proptest` L1 | `cargo kani --harness kani_resource_budget_bounded` + `cargo nextest run -p vb_core resource_policy` | Kani + property tests |

### UI Model Envelope (VB-UI-MODEL-envelope-001)

| Clause | Layer | Tool | Evidence |
|--------|-------|------|----------|
| Envelope schema valid | `proptest` L1 + `unit` | `cargo nextest run -p vb_ui_model envelope_` | Schema tests |
| Envelope JSON handling | `proptest` L1 | `cargo nextest run -p vb_ui_model serde_json_` | Property tests |
| Redaction correct | `unit` L1 | `cargo nextest run -p vb_ui_model redaction_` | Unit tests |

### Source Lint (Global Invariants)

| Clause | Layer | Tool | Evidence |
|--------|-------|------|----------|
| `no_unsafe_in_first_party` | `static-scan` L0 | `cargo clippy --workspace -- -D warnings` | Clippy deny |
| `no_panic_in_first_party` | `static-scan` L0 | `cargo clippy --workspace -- -D warnings` | Clippy deny |
| `no_unchecked_indexing` | `static-scan` L0 | `cargo xtask forbidden-scan --pattern unchecked_index` | Forbidden scan |
| `no_unchecked_arithmetic` | `static-scan` L0 | `cargo xtask forbidden-scan --pattern unchecked_arith` | Forbidden scan |
| `no_unchecked_casts` | `static-scan` L0 | `cargo xtask forbidden-scan --pattern unchecked_cast` | Forbidden scan |
| `no_runtime_yaml/json/http` | `static-scan` L0 | `cargo xtask hotpath-scan` | Hotpath scan |
| `no_hot_path_allocation` | `static-scan` L0 | `cargo xtask hotpath-scan` | Hotpath scan |

### Performance (Non-goal for this bead)

Performance verification is out of scope for vb-qi37.4.2 contract. If a future bead introduces a performance regression, the appropriate evidence is `cargo bench --bench <name>` with p99 acceptance threshold.

---

## Verus Scope

- **Rust targets**: `vb_core::value::join_taint`, `vb_core::value::FiniteF64`, `vb_core::frame::RunFrame`, `vb_core::frame::StepState`, `vb_core::budget::WholeWorkflowBudget`, `vb_core::signals::EngineSignal`, `vb_core::budget::StepBudget`
- **Spec/proof functions**: `spec_join_taint`, `proof_join_associative`, `proof_join_commutative`, `proof_join_idempotent`, `proof_join_identity`, `proof_no_downgrade_Secret`, `proof_no_downgrade_DerivedFromSecret`, `proof_step_budget_monotonic`, `proof_frame_dimension_immutable`, `proof_step_state_valid_transition`, `proof_engine_signal_finished_taint`
- **Invariants**: Taint lattice laws, StepBudget remaining ≥ 0, RunFrame dimension immutability, EngineSignal Finished carries Taint
- **Trusted boundary**: Validated constructors (`new`, `try_take` returning Result) are the only entry points; all other functions call through validated constructors
- **Shell exclusions**: I/O, async scheduling, storage, wall-clock time, FFI

---

## TLA+ Scope

- **Modules/Models**: `LifecycleJournal`, `ConcurrencyControl`, `RetryFSM`, `CapabilityLifecycle`, `ResumeStateMachine`
- **Variables**: `journal: Seq(JournalEntry)`, `dispatched: Set(ActionId)`, `shards: [ShardId -> Procs]`, `framePool: [ShardId -> Set(RunFrame)]`, `globalLock: [ResourceId -> MachineId ∨ Nil]`, `retryState: [ActionId -> State]`, `capabilities: Set(CapabilityId)`
- **Actions**: `WriteJournal`, `DispatchAction`, `CompleteAction`, `ReplayEntry`, `AcquireFrame`, `ReleaseFrame`, `AcquireLock`, `ReleaseLock`, `AttemptRetry`, `ExhaustRetries`
- **Safety invariants**: `JournalBeforeDispatch`, `MonotonicSequence`, `SingleShardOwner`, `NoCrossShardAlias`, `FramePoolBounded`, `MaxAttemptsRespected`, `CapabilityUniqueOwner`
- **Temporal properties**: `EventuallyAllJournaled`, `NoOrphanDispatch`, `EventuallyReplayComplete`, `NoStarvation`, `NoDeadlockOnLocks`, `EventuallyExhaustedOrDone`
- **Fairness**: Weak fairness on all action operators under enabled preconditions
- **Refinement boundary**: Rust `JournalWriter` appends in program order → TLA+ `journal` sequence. Rust shard frames → TLA+ `framePool[shard]`. Rust action dispatch → TLA+ `DispatchAction`.
- **Evidence command**: `tlc -config verification/tla/LifecycleJournal.cfg verification/tla/LifecycleJournal.tla`

---

## Theorem Scope

- **Theorem modules**: None — all Rust-local pure clauses are in Verus
- **Non-goals**: Lean/Aeneas/Hax projection for this bead

---

## Waivers

- **WAIVER-VRF-01**: clause `VB-CORE-TAINT-006`; waived layer `verus`; reason: modeling the full expression AST and object/list evaluator in Verus is outside this bead's proof-kernel boundary; limitation: waiver covers evaluator plumbing only, not the taint lattice laws; compensating evidence: required Kani L3 `kani_taint_propagation` plus proptest `taint_property_join`; owner: `proof-planner/formal-verifier`; expiry/follow-up: expires when evaluator AST proof surface is introduced or if Kani/proptest evidence is absent in State 11.
- **WAIVER-VRF-02**: clause `VB-CORE-IDX-002`; waived layer `verus`; reason: forbidden raw-index pattern is a static source property rather than a semantic Rust function contract; limitation: does not waive indexed access behavior in touched APIs; compensating evidence: required `cargo xtask forbidden-scan --pattern as_usize_index --crate vb_core`; owner: `formal-verifier`; expiry/follow-up: expires if the static-scan task is missing, failing, or stops covering `vb_core` hot paths.
- **WAIVER-VRF-03**: clause `REL-DEPENDENCY-SUPPLY-CHAIN`; waived layer `release-provenance` for this bead only; reason: no dependency files, feature flags, build scripts, vendored code, or dependency policy files are in scope; limitation: does not waive State 11 dependency evidence if such files changed; compensating evidence: delivery-scope dependency-change field plus State 11 regression diff; owner: `go-skill/formal-verifier`; expiry/follow-up: expires immediately on any dependency-scope diff.
=======
# Verification Layers

## Boundary

- Verus-owned kernel: capability exact-cardinality and decoded accepted-envelope gate/durable/status predicates using existing State5 verification artifacts.
- TLA+ temporal model: admission denial and no-allocation safety using `verification/tla/CapabilityLifecycle.tla`.
- Theorem projection: waived; no Lean kernel required at State 3.
- Runtime shell: storage loading, postcard decoding, runtime constructor wiring, CLI/IPC diagnostics.
- External systems excluded from formal proof: Fjall durability internals and wall-clock certificate source.

## Layer Assignment

- PRE-001 -> cargo test integration + fuzz/parser negative inputs + static scan.
- PRE-002 -> TLA+ gate mismatch + Verus accepted-envelope predicate + storage/runtime integration tests.
- PRE-003 -> digest-mismatch integration/domain test; Kani is waived at State3 because no harness exists and no source/proof writes are allowed in this repair.
- PRE-004 -> Verus accepted-envelope predicate + stale/non-durable integration tests.
- PRE-005 -> Verus capability model + TLA+ exact profile + proptest capability profiles.
- PRE-006 -> integration/Bdd constructor path tests + static scan for strict/journaled dummy-store use.
- POST-001 -> integration tests proving no runtime YAML/JSON parse on valid accepted artifact.
- POST-002 -> typed error integration/API/CLI/IPC assertions; mutation is waived at State3 until diagnostic tests exist.
- POST-003 / INV-005 -> TLA+ no allocation on denial + failure-injection integration tests.
- POST-004 / INV-007 -> diagnostics tests for rejected digest and semantic cause.
- INV-001 -> shared gate-count contract proof plus integration assertion storage/runtime agree on `15` or agreed canonical constant.
- INV-002 -> static scan + constructor integration tests.
- INV-003 -> explicit malformed-byte/error scenarios plus decoded Verus predicate; fuzz is waived at State3 because no accepted-envelope fuzz target exists and this repair cannot write tests/proofs.
- INV-004 -> static scan/API integration for accepted-artifact path.
- INV-006 -> Verus + proptest + TLA+.

## Verus Scope

- Existing Rust targets: `verification/verus/capability_artifact_model.rs` and `verification/verus/accepted_envelope_model.rs`.
- Existing spec/proof functions: `exact_capability_match`, `exact_profile`, `accepted_certificate_preserves_profile`, `proof_exact_profile_requires_cardinality`, `proof_missing_or_excess_grants_deny`, `proof_certificate_preserves_required_capabilities`, plus accepted-envelope predicates/proofs for schema support, canonical gate 15, durable flag, non-stale evidence, accepted proof flags, and fail-closed unsupported status denial.
- Trusted boundary: decoded `AcceptedArtifact` values and validated digest/capability inputs.
- Shell exclusions: Fjall I/O, postcard decode implementation, wall-clock source, CLI/IPC transport.

## TLA+ Scope

- Module/model path: `verification/tla/CapabilityLifecycle.tla`.
- Variables: `gate_count`, `required_count`, `grant_count`, `contracts_present`, `legacy_path`, `admission`, `run_allocated`, `journaled`, `drive_state`.
- Actions: `DenyGateMismatch`, `DenyCapabilityProfile`, `DenyLegacyBypass`, `AcceptAdmission`, `DriveDoWithoutContracts`, `DriveDoWithContracts`.
- Safety invariants: `ExactProfileRequired`, `ExcessGrantDenied`, `NoAdmissionOnGateMismatch`, `NoRunAllocatedOnDeniedAdmission`, `NoLegacyBypassForProtectedSubmit`.
- Temporal properties: none claimed; safety-only model.
- Fairness/deadlock stance: no fairness; deadlock check disabled by existing cfg.
- Refinement boundary: runtime admission errors refine denied states; run insertion/drive/accepted events refine allocated/admitted states.
- Evidence command: TLC commands listed in `tla-spec.md`.

## Second-Ring / Runtime Evidence

- Parser/codec: malformed-byte integration/error scenarios must cover raw `WorkflowParts`, YAML/JSON bytes, truncated postcard, wrong schema, and random bytes. Fuzz target is waived until a later state creates `fuzz/fuzz_targets/accepted_artifact_envelope.rs`.
- Kani/proptest: Kani digest harness and proptest feature lane are not execution passes at State3. Contract rows stay `status: planned`; later implementation/formal-verifier must either run exact executable targets when they exist or record WAIVED/DEFERRED results in downstream evidence artifacts.
- Static scan: strict/journaled production paths must not construct `AlwaysPresentArtifactStore`.
- Mutation: not an execution pass at State3. Contract row stays `status: planned`; later test/formal-verifier states must run a bounded cargo-mutants target after diagnostic tests exist or record WAIVED/DEFERRED in downstream evidence artifacts.
- Performance: non-goal. No speed claim is made.
- Release provenance/API compatibility: non-goal for this bead unless public error variants change.

## Waivers

- Lean/Aeneas/Hax waived; see `lean-contract.md`.
- TLA+ liveness waived because admission gate scope is fail-closed safety, not eventual scheduling.
- Kani digest downstream waiver policy: owner `formal-verifier-or-landing`; reason `verification/kani/digest_admission_harness.rs` is absent and State3 repair may not write proof code`; expiry `before formal-verifier can claim Kani digest evidence`; compensating evidence `ERR-005 scenario, integration/domain digest mismatch tests, and TLA+/Verus no-allocation/gate predicate coverage`.
- Fuzz accepted-envelope downstream waiver policy: owner `formal-verifier-or-landing`; reason `fuzz/fuzz_targets/accepted_artifact_envelope.rs` is absent and this repair may not write fuzz/test code`; expiry `before any hostile-byte fuzz claim`; compensating evidence `ERR-002/ERR-003 scenarios and decoded Verus predicate evidence`.
- Proptest invalid-space waiver: owner `rust-contract`; reason `no confirmed proptest feature/target in State3 artifact scope`; expiry `before State8 test implementation if generator target is added`; compensating evidence `explicit Fowler scenarios per invalid class plus exact capability Verus/TLA+`.
- Mutation diagnostic downstream waiver policy: owner `formal-verifier-or-landing`; reason `diagnostic tests do not yet exist and this repair may not write tests`; expiry `after diagnostic tests are implemented or before mutation evidence is claimed`; compensating evidence `ERR-001..ERR-008 exact expected scenarios and traceability rows`.
- Canonical CI downstream deferral policy: owner `formal-verifier-or-landing`; reason `State3 contract repair cannot execute implementation-wide CI as proof approval evidence`; expiry `formal-verifier/landing state`; compensating evidence `JSONL validation and existing State5 TLA+/Verus PASS evidence for proof artifacts`.
>>>>>>> origin/go-skill-p0-vb-qi37-4-2
