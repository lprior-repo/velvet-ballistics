# Verification Layers: vb-qi37.4

## Boundary
- Verus-owned kernel: Rust-local pure predicates for accepted-artifact proof schema, digest equality, and exact capability coverage in concrete Verus files.
- TLA+ temporal model: admission failure prevents acknowledgement, duplicate run ids reject without live state, live state appears only after durable admission/header persistence, and success eventually acknowledges under weak fairness.
- Theorem projection: none; Lean/Aeneas/Hax waived unless proof review escalates.
- Runtime shell: Fjall persistence, postcard bytes, journal append, shard mutation, CLI/API/IPC rendering.
- External systems excluded from formal proof: actual disk flush, process death, wall clock, and host filesystem behavior; covered by integration/recovery evidence.

## Layer Assignment
- PRE-001 -> integration + storage artifact store checks + `moon run :verify-deep`.
- PRE-002 -> fuzz/Bolero or cargo-fuzz for accepted artifact postcard bytes + integration malformed-envelope cases.
- PRE-003 / INV-003 -> Verus (`verification/verus/admission_artifact_model.rs`, `VERUS-GATE-004`) + Kani/integration for production extraction and all false flags.
- PRE-004 / INV-004 -> Verus (`verification/verus/capability_artifact_model.rs`, `VERUS-CAP-003`) + Kani/proptest for capability mismatch cases.
- PRE-005 / POST-004 / INV-005 -> TLA+ (`specs/admission_header_before_ack.tla`, `TLA-ACK-001`) + storage failure injection integration.
- PRE-006 / ERR-004 -> TLA+ (`TLA-STATE-002`) + `INT-DUPLICATE-014` integration/BDD duplicate run id test + mutation evidence.
- POST-001 / POST-005 / INV-002 -> Verus (`verification/verus/admission_artifact_model.rs`, `VERUS-DIGEST-005`) + recovery/integration event/header lookup.
- POST-002 / POST-003 / INV-001 -> TLA+ (`TLA-STATE-002`) + integration tests proving no run state after rejection/failure.
- ERR-006 -> TLA+ QueueFull failure abstraction + `KANI-ADMIT-006` + `INT-CAPACITY-015` capacity-exceeded diagnostic/state evidence.
- INV-006 / ERR-001..ERR-006 -> API/CLI/IPC diagnostic tests + mutation + static scan.
- INV-007 -> static scan/source lint + integration proving strict runtime starts from accepted binary artifact path.

## Verus Scope
- Capability target: `verification/verus/capability_artifact_model.rs`; proof surface: `exact_capability_match`, `exact_profile`, `accepted_certificate_preserves_profile`, `proof_exact_match_requires_name_and_action`, `proof_prefix_or_action_mismatch_denies`, `proof_exact_profile_requires_cardinality`, `proof_missing_or_excess_grants_deny`, `proof_certificate_preserves_required_capabilities`, `proof_non_empty_contract_not_erased`.
- Admission target: `verification/verus/admission_artifact_model.rs`; proof surface: `required_gate_count`, `proof_flags_complete`, `gate_schema_valid`, `digest_binding_valid`, `strict_admission_valid`, `proof_success_requires_runtime_gate_count`, `proof_wrong_gate_count_denies`, `proof_false_required_flag_denies`, `proof_success_preserves_digest_binding`, `proof_digest_mismatch_denies`.
- Trusted boundary: validated construction of `WorkflowDigest`, postcard deserialization, Fjall record retrieval, and production-to-model extraction.
- Shell exclusions: I/O, storage flush, shard mutation ordering, CLI/API/IPC rendering, and wall-clock timestamps.
- Evidence commands: `verus verification/verus/capability_artifact_model.rs` and `verus verification/verus/admission_artifact_model.rs`. `moon run :verify-proof` remains canonical rollup but is blocked by unrelated wrapper tooling per State 6.

## TLA+ Scope
- Module/model path: `specs/admission_header_before_ack.tla`.
- Config: `specs/admission_header_before_ack.cfg`.
- Variables: `state`, `code`, `ack`, `persisted`, `live_state`, `duplicate_run`.
- Actions: `Init`, `AdmissionReject`, `StorageFail`, `PersistHeader`, `Ack`, `TerminalStutter`, `Next`.
- Safety invariants: `TypeOK`, `FailurePreventsAck`, `DuplicateRejectsNoLiveState`, `AckRequiresPersistence`, `LiveStateRequiresPersistence`, `NoLiveStateBeforeDurableAdmission`.
- Temporal properties: `FailureEventuallyRejected`, `SuccessEventuallyAcked`.
- Fairness/deadlock stance: weak fairness on rejection/storage failure/persist/ack; cfg deadlock check enabled.
- Refinement boundary: runtime `handle_submit`, duplicate lookup, storage append/persist, `runs.insert`, and success return outcomes refine TLA actions by run id, duplicate status, persistence status, and admission error code.
- Evidence command: `tlc -config specs/admission_header_before_ack.cfg specs/admission_header_before_ack.tla`. `moon run :verify-proof` is canonical rollup but not the executable obligation while the wrapper is broken.

## Second-Ring Evidence
- Fuzz: malformed accepted-artifact envelope cannot panic or admit invalid bytes; command boundary `moon run :fuzz-smoke` or `moon run :verify-deep`.
- Miri: interpreter smoke for memory/UB-sensitive core paths via `moon run :miri` or `moon run :verify-deep`.
- Mutation: error-path assertion strength via `moon run :mutants-smoke` or `moon run :verify-deep`.
- Coverage: changed admission/durability surface represented by `moon run :coverage` or `moon run :verify-deep`.
- Supply chain/static scan: `moon run :supply-chain`, `moon run :lint-src`, and `moon ci` for final workspace gate.

## Waivers and Blockers
- BLOCKER-001: Production shell must demonstrate the runtime/storage gate-count source of truth extracts the same value proven by Verus (`required_gate_count() == 15`); the contract-level Verus obligation is no longer a placeholder.
- BLOCKER-002: Atomic accepted-run persistence boundary is owned by `vb-core-atomic-admission`; until closed, State 3 treats atomicity as a planned obligation.
- BLOCKER-003: Strict production runtime must use `StorageArtifactStore`, not dummy stores; owned by `vb-core-storage-artifact-store`.
